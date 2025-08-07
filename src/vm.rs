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
                    self.push(constant);
                }
                OpCode::ConstantLong => {
                    let constant = self.read_constant_long();
                    self.push(constant);
                }
                OpCode::Nil => {
                    self.push(Value::Nil);
                }
                OpCode::True => {
                    self.push(Value::Bool(true));
                }
                OpCode::False => {
                    self.push(Value::Bool(false));
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
                    self.stack.truncate(self.current_frame.stack_start - 1);
                    self.push(ret);
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
                    self.binary_number_op(|l, r| *l -= r, "Operands to '-' must be numbers")?
                }
                OpCode::Mult => {
                    self.binary_number_op(|l, r| *l *= r, "Operands to '*' must be numbers")?
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
                    if self.stack.len() < 2 {
                        return None;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left > right);
                }
                OpCode::GreaterEqual => {
                    if self.stack.len() < 2 {
                        return None;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left >= right);
                }
                OpCode::Less => {
                    if self.stack.len() < 2 {
                        return None;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left < right);
                }
                OpCode::LessEqual => {
                    if self.stack.len() < 2 {
                        return None;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left <= right);
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

                    self.get_local(index);
                }
                OpCode::GetLocalLong => {
                    let index = self.read_int24();

                    self.get_local(index);
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
                        // SAFETY: Closure pointers are allocated by GC and remain valid
                        // for the lifetime of the GC which outlives all Closure references
                        &mut *closure_ptr
                    };

                    // Push the closure first so that it can be captured by upvalues
                    self.push(Value::Closure(closure_ptr));

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
                        self.push(*(*upvalue).location);
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
            }
        }
    }

    fn call_value(&mut self, arg_count: u8) -> Option<()> {
        if self.stack.len() < (arg_count as usize) + 1 {
            return None;
        }

        let callee = self.stack[self.stack.len() - (arg_count as usize) - 1];

        match callee {
            Value::Closure(closure) => {
                let arity = unsafe {
                    // SAFETY: Closure pointers are allocated by GC and remain valid
                    // for the lifetime of the GC which outlives all Value references
                    (*closure).arity()
                };

                self.call(closure, arity, arg_count)
            }
            Value::NativeFunc(native) => {
                let args = &self.stack[self.stack.len() - (arg_count as usize)..];
                let ret = unsafe {
                    // SAFETY: NativeFunc pointers are allocated by GC and remain valid
                    // for the lifetime of the GC which outlives all Value references
                    (*native).call(args)
                };

                match ret {
                    Ok(value) => {
                        self.stack
                            .truncate(self.stack.len() - (arg_count as usize) - 1);
                        self.push(value);
                        Some(())
                    }
                    Err(err) => {
                        self.runtime_error(&err);
                        None
                    }
                }
            }
            _ => {
                self.runtime_error("Can only call functions and classes");
                None
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
            stack_start: self.stack.len() - (arg_count as usize),
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
                self.push(*value);
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
                self.push(to);
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

    fn get_local(&mut self, index: usize) {
        // Index is relative to the current frame
        let abs_index = self.current_frame.stack_start + index;
        self.push(self.stack[abs_index]);
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
                // SAFETY: String pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
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
            (Value::Number(_), Value::Number(0.0)) => {
                self.runtime_error("Division by 0");
                None
            }
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
                // SAFETY: Upvalue pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references

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
            // SAFETY: Closure function pointers are allocated by GC and remain valid
            // for the lifetime of the GC which outlives all Closure references
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
            // SAFETY: Closure pointers are allocated by GC and remain valid
            // for the lifetime of the GC which outlives all Closure references
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

        for frame in rev_frame_iter {
            let function = unsafe {
                // SAFETY: Closure function pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Closure references
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
