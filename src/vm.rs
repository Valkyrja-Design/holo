use crate::value::{BoundMethod, Class, ClassInstance};

use super::{
    chunk::{Chunk, OpCode},
    gc,
    table::StringInternTable,
    value::{Closure, Function, Upvalue, Value},
};
use log::debug;
use std::io::Write;

#[derive(Clone, Copy)]
struct CallFrame {
    closure: *mut Closure, // Current closure being executed
    ip: usize,             // Instruction pointer
    stack_start: usize,    // Index of the first element of the stack for this frame
}

struct OpenUpvalue {
    stack_index: usize,
    upvalue: *mut Upvalue,
}

static VEC_SIZE: usize = 1024; // Default vec size for `VM::stack` and `VM::open_upvalues`
static STACK_TRACE_SIZE: usize = 10; // Number of frames to print in a stack trace

pub struct VM<'a, T: Write, U: Write> {
    call_stack: Vec<CallFrame>,
    current_frame: CallFrame,
    stack: Vec<Value>,
    open_upvalues: Vec<OpenUpvalue>,
    gc: gc::GC,
    str_intern_table: StringInternTable,
    globals: Vec<Option<Value>>, // None means the variable is undefined
    global_var_names: Vec<String>,
    output_stream: &'a mut T,
    err_stream: &'a mut U,
}

impl<'a, T: Write, U: Write> VM<'a, T, U> {
    pub fn new(
        main_closure: *mut Closure,
        gc: gc::GC,
        str_intern_table: StringInternTable,
        global_var_names: Vec<String>,
        globals: Vec<Option<Value>>,
        output_stream: &'a mut T,
        err_stream: &'a mut U,
    ) -> Self {
        VM {
            call_stack: vec![CallFrame {
                closure: main_closure,
                ip: 0,
                stack_start: 0,
            }],
            current_frame: CallFrame {
                closure: main_closure,
                ip: 0,
                stack_start: 0,
            },
            stack: Vec::with_capacity(VEC_SIZE),
            open_upvalues: Vec::with_capacity(VEC_SIZE),
            gc,
            str_intern_table,
            globals,
            global_var_names,
            output_stream,
            err_stream,
        }
    }

    pub fn run(&mut self) -> Option<()> {
        loop {
            match self.read_opcode() {
                OpCode::Constant => {
                    let constant = self.read_constant();
                    self.push(constant)?;
                }
                OpCode::ConstantLong => {
                    let constant = self.read_constant_long();
                    self.push(constant)?;
                }
                OpCode::Nil => {
                    self.push(Value::Nil)?;
                }
                OpCode::True => {
                    self.push(Value::Bool(true))?;
                }
                OpCode::False => {
                    self.push(Value::Bool(false))?;
                }
                OpCode::Return => {
                    // Pop off the return value
                    let ret = self.stack.pop().unwrap();

                    // Pop off the current frame
                    self.call_stack.pop();

                    // If the call stack is empty, we're done
                    // (we added an implicit return for the main function)
                    if self.call_stack.is_empty() {
                        return Some(());
                    }

                    // Close upvalues for the current frame
                    self.close_upvalues(self.current_frame.stack_start);

                    // Otherwise, pop off the arguments and the callee from the stack,
                    // push the return value and set the current frame to the top of the call stack
                    self.stack.truncate(self.current_frame.stack_start);
                    self.push(ret)?;
                    self.current_frame = self.call_stack.last().unwrap().clone();
                }
                OpCode::Negate => match self.stack.last_mut() {
                    Some(Value::Number(value)) => *value = -*value,
                    Some(_) => {
                        self.runtime_error("Operand to '-' must be a number");
                        return None;
                    }
                    _ => {
                        return None;
                    }
                },
                OpCode::Not => match self.stack.last_mut() {
                    Some(Value::Bool(value)) => *value = !*value,
                    Some(_) => {
                        self.runtime_error("Operand to '!' must be a bool");
                        return None;
                    }
                    _ => {
                        return None;
                    }
                },
                OpCode::Add => self.binary_add()?,
                OpCode::Sub => {
                    self.binary_number_op(|l, r| *l -= r, "Operands to '-' must be numbers")?;
                }
                OpCode::Mult => {
                    self.binary_number_op(|l, r| *l *= r, "Operands to '*' must be numbers")?;
                }
                OpCode::Divide => self.binary_divide()?,
                OpCode::Equal => {
                    if self.stack.len() < 2 {
                        return None;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left == right);
                }
                OpCode::NotEqual => {
                    if self.stack.len() < 2 {
                        return None;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left != right);
                }
                OpCode::Greater => {
                    self.binary_number_ordering_op(
                        |l, r| l > r,
                        "Operands to '>' must be numbers",
                    )?;
                }
                OpCode::GreaterEqual => {
                    self.binary_number_ordering_op(
                        |l, r| l >= r,
                        "Operands to '>=' must be numbers",
                    )?;
                }
                OpCode::Less => {
                    self.binary_number_ordering_op(
                        |l, r| l < r,
                        "Operands to '<' must be numbers",
                    )?;
                }
                OpCode::LessEqual => {
                    self.binary_number_ordering_op(
                        |l, r| l <= r,
                        "Operands to '<=' must be numbers",
                    )?;
                }
                OpCode::Ternary => {
                    if self.stack.len() < 3 {
                        return None;
                    }

                    let else_value = self.stack.pop().unwrap();
                    let then_value = self.stack.pop().unwrap();
                    let predicate = self.stack.last_mut().unwrap();

                    match predicate {
                        Value::Bool(value) => {
                            if *value {
                                *predicate = then_value;
                            } else {
                                *predicate = else_value;
                            }
                        }
                        _ => {
                            self.runtime_error("Expected a boolean as ternary operator predicate");
                            return None;
                        }
                    }
                }
                OpCode::Print => {
                    if self.stack.is_empty() {
                        return None;
                    }

                    let _ = writeln!(self.output_stream, "{}", self.stack.pop().unwrap());
                }
                OpCode::Pop => {
                    if self.stack.is_empty() {
                        return None;
                    }

                    self.stack.pop();
                }
                OpCode::DefineGlobal => {
                    // IMP: Lookout for GC here
                    let index: usize = self.read_int8();

                    self.define_global(index)?
                }
                OpCode::DefineGlobalLong => {
                    // IMP: Lookout for GC here
                    let index = self.read_int24();

                    self.define_global(index)?
                }
                OpCode::GetGlobal => {
                    let index = self.read_int8();

                    self.get_global(index)?
                }
                OpCode::GetGlobalLong => {
                    let index = self.read_int24();

                    self.get_global(index)?
                }
                OpCode::SetGlobal => {
                    let index = self.read_int8();

                    self.set_global(index)?
                }
                OpCode::SetGlobalLong => {
                    let index = self.read_int24();

                    self.set_global(index)?
                }
                OpCode::GetLocal => {
                    let index = self.read_int8();

                    self.get_local(index)?
                }
                OpCode::GetLocalLong => {
                    let index = self.read_int24();

                    self.get_local(index)?
                }
                OpCode::SetLocal => {
                    let index = self.read_int8();

                    self.set_local(index)?
                }
                OpCode::SetLocalLong => {
                    let index = self.read_int24();

                    self.set_local(index)?
                }
                OpCode::PopN => {
                    let n = self.read_int8();

                    self.stack.truncate(self.stack.len() - n);
                }
                OpCode::PopNLong => {
                    let n = self.read_int24();

                    self.stack.truncate(self.stack.len() - n);
                }
                OpCode::JumpIfFalse => {
                    let jump_offset = self.read_int16();

                    match self.stack.last() {
                        Some(Value::Bool(value)) => {
                            if !*value {
                                *self.ip_as_mut() += jump_offset;
                            }
                        }
                        Some(_) => {
                            self.runtime_error("Expected `bool` as condition");
                            return None;
                        }
                        _ => unreachable!("No value in the stack"),
                    }
                }
                OpCode::JumpIfTrue => {
                    let jump_offset = self.read_int16();

                    match self.stack.last() {
                        Some(Value::Bool(value)) => {
                            if *value {
                                *self.ip_as_mut() += jump_offset;
                            }
                        }
                        Some(_) => {
                            self.runtime_error("Expected `bool` as condition");
                            return None;
                        }
                        _ => unreachable!("No value in the stack"),
                    }
                }
                OpCode::Jump => {
                    let jump_offset = self.read_int16();

                    *self.ip_as_mut() += jump_offset;
                }
                OpCode::Loop => {
                    let jump_offset = self.read_int16();

                    *self.ip_as_mut() -= jump_offset;
                }
                OpCode::Call => {
                    let arg_count = self.read_int8() as u8;

                    self.call_value(arg_count)?
                }
                OpCode::Closure => {
                    let constant = self.read_constant();
                    let func = constant
                        .as_function_mut()
                        .expect("Closure constant must be a function");
                    let upvalue_count = func.upvalue_count;
                    let closure = Closure::new(func as *mut Function, upvalue_count);
                    let closure_ptr = self.gc.alloc_closure_ptr(closure);
                    let closure = unsafe {
                        // SAFETY: GC guarantees that all pointers are valid
                        &mut *closure_ptr
                    };

                    // Push the closure first so that it can be captured by upvalues
                    self.push(Value::Closure(closure_ptr))?;

                    // Attempt to trigger a garbage collection cycle
                    self.attempt_gc();

                    // Initialize the upvalues
                    for _ in 0..upvalue_count {
                        let is_local = self.read_byte() == 1;
                        let index = self.read_byte() as usize;

                        let upvalue = if is_local {
                            self.capture_local(index)
                        } else {
                            self.upvalues()[index]
                        };

                        closure.upvalues.push(upvalue);
                    }
                }
                OpCode::ClosureLong => {
                    // TODO
                }
                OpCode::GetUpvalue => {
                    let index = self.read_byte() as usize;
                    let upvalue = self.upvalues()[index];

                    unsafe {
                        // SAFETY: Upvalue pointers are allocated by GC and remain valid
                        // for the lifetime of the GC which outlives all Value references
                        self.push(*(*upvalue).location)?;
                    }
                }
                OpCode::GetUpvalueLong => {
                    // TODO
                }
                OpCode::SetUpvalue => {
                    let index = self.read_byte() as usize;
                    let upvalue = self.upvalues()[index];

                    unsafe {
                        // SAFETY: Upvalue pointers are allocated by GC and remain valid
                        // for the lifetime of the GC which outlives all Value references
                        *(*upvalue).location = *self.stack.last().unwrap()
                    }
                }
                OpCode::SetUpvalueLong => {
                    // TODO
                }
                OpCode::CloseUpvalue => {
                    // Close over the local at the top of the stack
                    self.close_upvalues(self.stack.len() - 1);
                    self.stack.pop();
                }
                OpCode::Class => {
                    let name = self.read_constant();
                    let name = name.as_string().expect("Class name must be a string");

                    let class = self.gc.alloc_class_ptr(Class::new(name.to_string()));

                    self.push(Value::Class(class))?;

                    // Attempt to trigger a garbage collection cycle
                    self.attempt_gc();
                }
                OpCode::GetProperty => {
                    let name = self.read_constant();
                    let name = name.as_string().expect("Property name must be a string");

                    // Get the field from the instance
                    let instance = self.stack.last().unwrap().as_class_instance();
                    if instance.is_none() {
                        self.runtime_error("Property must be accessed on a class instance");
                        return None;
                    }

                    let instance = instance.unwrap();
                    let field = instance.fields.get(name);

                    if field.is_some() {
                        *self.stack.last_mut().unwrap() = *field.unwrap();
                    } else {
                        // Bind the method to the instance
                        self.bind_method(instance.class, name)?;
                    }
                }
                OpCode::SetProperty => {
                    let name = self.read_constant();
                    let name = name.as_string().expect("Property name must be a string");

                    // Set the field on the instance
                    let value = self.stack.pop().unwrap();
                    let instance = self.stack.last().unwrap().as_class_instance_mut();

                    if instance.is_none() {
                        self.runtime_error("Property must be accessed on a class instance");
                        return None;
                    }

                    let instance = instance.unwrap();

                    instance.fields.insert(name.to_string(), value);
                    *self.stack.last_mut().unwrap() = value;
                }
                OpCode::Method => {
                    self.define_method()?;
                }
                OpCode::Invoke => {
                    self.invoke_method()?;
                }
                OpCode::Inherit => {
                    let subclass = self.stack.pop().unwrap();
                    let subclass = subclass.as_class_mut();
                    let superclass = self.stack.last().unwrap().as_class();
                    // Leave the subclass on the stack

                    if superclass.is_none() {
                        self.runtime_error("Superclass must be a class");
                        return None;
                    }

                    let superclass = superclass.unwrap();
                    let subclass = subclass.unwrap();

                    // Copy all methods from the superclass to the subclass
                    for (name, method) in superclass.methods.iter() {
                        subclass.methods.insert(name.clone(), *method);
                    }
                }
                OpCode::GetSuper => {
                    let superclass = self.stack.pop().unwrap();
                    let method_name = self.read_constant();
                    let method_name = method_name
                        .as_string()
                        .expect("Method name must be a string");

                    // `this` is at the top of the stack and will be the receiver for
                    // the bound method
                    self.bind_method(superclass.as_class_ptr().unwrap(), method_name)?;
                }
                OpCode::SuperInvoke => {
                    self.invoke_super_method()?;
                }
            }
        }
    }

    fn call_value(&mut self, arg_count: u8) -> Option<()> {
        if self.stack.len() < (arg_count as usize) + 1 {
            return None;
        }

        let callee = self.stack[self.stack.len() - (arg_count as usize) - 1];

        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            match callee {
                Value::Closure(closure) => {
                    let arity = (*closure).arity();

                    self.call(closure, arity, arg_count)
                }
                Value::NativeFunc(native) => {
                    let args = &self.stack[self.stack.len() - (arg_count as usize)..];
                    let ret = (*native).call(args);

                    match ret {
                        Ok(value) => {
                            self.stack
                                .truncate(self.stack.len() - (arg_count as usize) - 1);
                            self.push(value)?;
                            Some(())
                        }
                        Err(err) => {
                            self.runtime_error(&err);
                            None
                        }
                    }
                }
                Value::Class(class) => {
                    let instance = self.gc.alloc_class_instance_ptr(ClassInstance::new(class));
                    let len = self.stack.len();

                    self.stack[len - (arg_count as usize) - 1] = Value::ClassInstance(instance);
                    // Attempt to trigger a garbage collection cycle
                    self.attempt_gc();

                    // Find the initializer for this class if it exists
                    let initializer = (*class).methods.get("init");

                    if let Some(init) = initializer {
                        let arity = (**init).arity();

                        // Call the initializer with the instance as the receiver
                        self.call(*init, arity, arg_count)?;
                    } else if arg_count != 0 {
                        self.runtime_error(
                            format!(
                                "Expected 0 arguments for class initializer, got {}",
                                arg_count
                            )
                            .as_str(),
                        );
                        return None;
                    }

                    Some(())
                }
                Value::BoundMethod(bound_method) => {
                    let arity = (*(*bound_method).method).arity();
                    let len = self.stack.len();

                    // We reserved the first slot of the locals for the receiver. To utilize that we'll overwrite
                    // the callee with the receiver
                    self.stack[len - (arg_count as usize) - 1] =
                        Value::ClassInstance((*bound_method).receiver);
                    self.call((*bound_method).method, arity, arg_count)
                }
                _ => {
                    self.runtime_error("Can only call functions and classes");
                    None
                }
            }
        }
    }

    fn call(&mut self, closure: *mut Closure, arity: u8, arg_count: u8) -> Option<()> {
        if arity != arg_count {
            self.runtime_error(&format!(
                "Incorrect number of arguments: expected {}, got {}",
                arity, arg_count
            ));
            return None;
        }

        // Before setting the current frame to the new call frame we need to
        // write back the current ip to the current frame on the call stack
        self.call_stack.last_mut().unwrap().ip = self.current_frame.ip;
        self.call_stack.push(CallFrame {
            closure,
            ip: 0,
            stack_start: self.stack.len() - (arg_count as usize) - 1,
        });

        // Set the current frame to the top of the call stack
        self.current_frame = self.call_stack.last().unwrap().clone();
        Some(())
    }

    fn define_global(&mut self, index: usize) -> Option<()> {
        if self.stack.len() < 1 {
            return None;
        }

        let initializer = self.stack.pop().unwrap();

        // Don't care what the current value is
        match self.globals.get_mut(index) {
            Some(value) => {
                *value = Some(initializer);
                Some(())
            }
            _ => unreachable!("No global variable at index {}", index),
        }
    }

    fn get_global(&mut self, index: usize) -> Option<()> {
        match self.globals.get(index) {
            Some(Some(value)) => {
                self.push(*value)?;
                Some(())
            }
            _ => {
                self.runtime_error(
                    format!("Undefined variable '{}'", self.global_var_names[index]).as_str(),
                );
                None
            }
        }
    }

    fn set_global(&mut self, index: usize) -> Option<()> {
        if self.stack.len() < 1 {
            return None;
        }

        let to = self.stack.pop().unwrap();

        match self.globals.get_mut(index) {
            Some(Some(value)) => {
                *value = to;
                self.push(to)?;
                Some(())
            }
            _ => {
                self.runtime_error(
                    format!("Undefined variable '{}'", self.global_var_names[index]).as_str(),
                );
                None
            }
        }
    }

    fn get_local(&mut self, index: usize) -> Option<()> {
        // Index is relative to the current frame
        let abs_index = self.current_frame.stack_start + index;
        self.push(self.stack[abs_index])
    }

    fn set_local(&mut self, index: usize) -> Option<()> {
        if self.stack.len() < 1 {
            return None;
        }

        // Index is relative to the current frame
        let abs_index = self.current_frame.stack_start + index;
        self.stack[abs_index] = *self.stack.last().unwrap();
        Some(())
    }

    fn define_method(&mut self) -> Option<()> {
        let method_name = self.read_constant();
        let method_name = method_name
            .as_string()
            .expect("Method name must be a string");

        // The method's closure is at the top of the stack with the parent class right below it
        let method_closure = self
            .stack
            .pop()
            .unwrap()
            .as_closure_ptr()
            .expect("Expected a closure object");

        let class = self
            .stack
            .last()
            .unwrap()
            .as_class_mut()
            .expect("Expected a class object to define a method on");

        // Add the method to the class
        class
            .methods
            .insert(method_name.to_string(), method_closure);

        Some(())
    }

    fn invoke_method(&mut self) -> Option<()> {
        let method_name = self.read_constant();
        let method_name = method_name
            .as_string()
            .expect("Method name must be a string");
        let arg_count = self.read_int8() as u8;
        let len = self.stack.len();
        let instance = self.stack[len - (arg_count as usize) - 1].as_class_instance_ptr();

        if let Some(instance) = instance {
            // First check if this is a field access
            let field = unsafe { (*instance).fields.get(method_name) };

            if let Some(field) = field {
                self.stack[len - (arg_count as usize) - 1] = *field;
                return self.call_value(arg_count);
            }

            return self.invoke_from_class(unsafe { (*instance).class }, method_name, arg_count);
        }

        self.runtime_error("Can only call methods on class instances");
        None
    }

    fn invoke_super_method(&mut self) -> Option<()> {
        let method_name = self.read_constant();
        let method_name = method_name
            .as_string()
            .expect("Method name must be a string");
        let arg_count = self.read_int8() as u8;
        let superclass = self.stack.pop().unwrap().as_class_ptr().unwrap();

        self.invoke_from_class(superclass, method_name, arg_count)
    }

    fn invoke_from_class(
        &mut self,
        class: *mut Class,
        method_name: &str,
        arg_count: u8,
    ) -> Option<()> {
        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            let method = (*class).methods.get(method_name);

            if let Some(method) = method {
                return self.call(*method, (**method).arity(), arg_count);
            }

            self.runtime_error(&format!("Undefined method '{}'", method_name));
            None
        }
    }

    fn bind_method(&mut self, class: *mut Class, method_name: &str) -> Option<()> {
        let method = unsafe { (*class).methods.get(method_name) };

        if method.is_none() {
            self.runtime_error(&format!("Undefined property '{}'", method_name));
            return None;
        }

        // Bind the method to the instance
        let bound_method = self.gc.alloc_bound_method(BoundMethod::new(
            self.stack.last().unwrap().as_class_instance_ptr().unwrap(),
            *method.unwrap(),
        ));

        *self.stack.last_mut().unwrap() = bound_method;

        // Attempt to trigger a garbage collection cycle
        self.attempt_gc();
        Some(())
    }

    fn binary_number_op<F>(&mut self, op: F, err: &str) -> Option<()>
    where
        F: FnOnce(&mut f64, f64),
    {
        if self.stack.len() < 2 {
            return None;
        }

        let right = self.stack.pop().unwrap();
        let left = self.stack.last_mut().unwrap();

        match (left, right) {
            (Value::Number(left), Value::Number(right)) => {
                op(left, right);
                Some(())
            }
            _ => {
                self.runtime_error(err);
                None
            }
        }
    }

    fn binary_number_ordering_op<F>(&mut self, op: F, err: &str) -> Option<()>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        if self.stack.len() < 2 {
            return None;
        }

        let right = self.stack.pop().unwrap();
        let left = self.stack.last_mut().unwrap();

        match (&left, right) {
            (Value::Number(l), Value::Number(r)) => {
                *left = Value::Bool(op(*l, r));
                Some(())
            }
            _ => {
                self.runtime_error(err);
                None
            }
        }
    }

    fn binary_add(&mut self) -> Option<()> {
        if self.stack.len() < 2 {
            return None;
        }

        let right = self.stack.pop().unwrap();
        let left = self.stack.last_mut().unwrap();

        match (left, right) {
            (Value::Number(left), Value::Number(right)) => {
                *left += right;
                Some(())
            }
            (Value::String(left), Value::String(right)) => unsafe {
                // SAFETY: GC guarantees that all pointers are valid
                let mut concatenated_str: String =
                    String::with_capacity((**left).len() + (*right).len());
                concatenated_str.push_str(&**left);
                concatenated_str.push_str(&*right);

                *left = self
                    .str_intern_table
                    .intern_owned(concatenated_str, &mut self.gc);

                // Attempt to trigger a garbage collection cycle
                self.attempt_gc();

                Some(())
            },
            _ => {
                self.runtime_error("Operands to '+' must be two numbers or strings");
                None
            }
        }
    }

    fn binary_divide(&mut self) -> Option<()> {
        if self.stack.len() < 2 {
            return None;
        }

        let right = self.stack.pop().unwrap();
        let left = self.stack.last_mut().unwrap();

        match (left, right) {
            (Value::Number(left), Value::Number(right)) => {
                *left /= right;
                Some(())
            }
            _ => {
                self.runtime_error("Operands to '/' must be numbers");
                None
            }
        }
    }

    /// Captures the local at the given index for the current frame
    fn capture_local(&mut self, index: usize) -> *mut Upvalue {
        let abs_index = self.current_frame.stack_start + index;
        let location = &mut self.stack[abs_index] as *mut Value;

        // Search for an existing upvalue for this local, our `open_upvalues` array
        // is sorted by stack index, so we can use binary search
        match self
            .open_upvalues
            .binary_search_by_key(&abs_index, |probe| probe.stack_index)
        {
            Ok(index) => self.open_upvalues[index].upvalue,
            Err(index) => {
                let upvalue = self
                    .gc
                    .alloc_upvalue_ptr(Upvalue::new(location, Value::default()));

                self.open_upvalues.insert(
                    index,
                    OpenUpvalue {
                        stack_index: abs_index,
                        upvalue,
                    },
                );

                // Attempt to trigger a garbage collection cycle
                self.attempt_gc();

                upvalue
            }
        }
    }

    /// Closes all open upvalues that are above the given stack index
    fn close_upvalues(&mut self, stack_index: usize) {
        // Find the first open value that needs to be closed
        let pos = self
            .open_upvalues
            .partition_point(|upvalue| upvalue.stack_index < stack_index);

        // Close them
        for upvalue in self.open_upvalues.drain(pos..) {
            unsafe {
                // SAFETY: GC guarantees that all pointers are valid

                // Move the stack value to the upvalue's closed field
                // and set the upvalue's location to the closed field
                let upvalue = &mut *upvalue.upvalue;

                upvalue.closed = *upvalue.location;
                upvalue.location = &mut upvalue.closed as *mut Value;
            }
        }
    }

    /// Attempts to trigger a garbage collection cycle
    fn attempt_gc(&mut self) {
        if self.gc.should_collect() {
            self.collect_garbage();
        }
    }

    /// Do a garbage collection cycle
    fn collect_garbage(&mut self) {
        // Log the start of the garbage collection cycle for debugging
        debug!("-- Start of garbage collection cycle --");

        // Clear previous previous garbage collection cycle's marks
        self.gc.clear_marks();

        // Mark all values that are reachable from the call stack
        for frame in &self.call_stack {
            self.gc.mark_closure(frame.closure);
        }

        // Mark all values that are reachable from the open upvalues
        for open_upvalue in &self.open_upvalues {
            self.gc.mark_upvalue(open_upvalue.upvalue);
        }

        // Mark all values that are reachable from the stack
        for value in &self.stack {
            self.gc.mark_value(*value);
        }

        // Mark all values that are reachable from the globals
        for global in &self.globals {
            if let Some(value) = global {
                self.gc.mark_value(*value);
            }
        }

        // Mark all values that are reachable from the roots
        self.gc.trace_references();

        // Clear all interned strings that are not marked
        self.str_intern_table.clear_unmarked(&mut self.gc);

        // Sweep all values that are not reachable
        self.gc.sweep();

        // Log the end of the garbage collection cycle for debugging
        debug!("-- End of garbage collection cycle --");
    }

    // TRY: Stash the current frame's chunk in a local variable
    fn chunk(&self) -> &Chunk {
        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            &(*self.current_frame.closure).chunk()
        }
    }

    fn ip(&self) -> usize {
        self.current_frame.ip
    }

    fn ip_as_mut(&mut self) -> &mut usize {
        &mut self.current_frame.ip
    }

    fn upvalues(&self) -> &Vec<*mut Upvalue> {
        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            &(*self.current_frame.closure).upvalues
        }
    }

    fn push(&mut self, value: Value) -> Option<()> {
        if self.stack.len() >= VEC_SIZE {
            self.runtime_error(
                format!("Stack overflow: maximum stack size is {}", VEC_SIZE).as_str(),
            );
            return None;
        }

        self.stack.push(value);
        Some(())
    }

    fn read_opcode(&mut self) -> OpCode {
        OpCode::from(self.read_byte())
    }

    fn read_byte(&mut self) -> u8 {
        let ip = self.ip();
        let byte = self.chunk().code[ip];

        *self.ip_as_mut() += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        let idx = self.read_byte() as usize;

        self.chunk().constants[idx]
    }

    fn read_constant_long(&mut self) -> Value {
        let ip = self.ip();
        let idx = Chunk::read_as_24bit_int(&self.chunk().code[ip..ip + 3]);

        *self.ip_as_mut() += 3;
        self.chunk().constants[idx]
    }

    fn read_int8(&mut self) -> usize {
        self.read_byte() as usize
    }

    fn read_int16(&mut self) -> usize {
        let ip = self.ip();
        let ret = Chunk::read_as_16bit_int(&self.chunk().code[ip..ip + 2]);

        *self.ip_as_mut() += 2;
        ret
    }
    fn read_int24(&mut self) -> usize {
        let ip = self.ip();
        let ret = Chunk::read_as_24bit_int(&self.chunk().code[ip..ip + 3]);

        *self.ip_as_mut() += 3;
        ret
    }

    fn runtime_error(&mut self, err: &str) {
        // We have to write back the current ip to the current call frame on the call stack
        self.call_stack.last_mut().unwrap().ip = self.current_frame.ip;

        let _ = writeln!(self.err_stream, "Runtime error: {err}");
        let rev_frame_iter = self.call_stack.iter().rev();

        for frame in rev_frame_iter.take(STACK_TRACE_SIZE) {
            let function = unsafe {
                // SAFETY: GC guarantees that all pointers are valid
                (*frame.closure).function()
            };

            let instr = frame.ip - 1;
            let line = function.chunk.get_line_of(instr);
            let function_name = if function.name == "<main>" {
                "<main>".to_string()
            } else {
                format!("{}()", function.name)
            };

            let _ = writeln!(self.err_stream, "[line {}] in {}", line, function_name);
        }
    }
}
