use crate::disassembler::{disassemble, disassemble_instr};

use super::{
    chunk::{Chunk, OpCode},
    gc,
    object::{Function, Object},
    table::StringInternTable,
    value::Value,
};
use std::io::Write;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

#[derive(Clone, Copy)]
pub struct CallFrame {
    function: *mut Object, // current function being executed
    ip: usize,             // instruction pointer
    stack_start: usize,    // index of the first element of the stack for this frame
}

pub struct VM<'a, T: Write, U: Write> {
    call_stack: Vec<CallFrame>,
    current_frame: CallFrame,
    stack: Vec<Value>,
    gc: gc::GC,
    str_intern_table: StringInternTable,
    globals: Vec<Option<Value>>, // None means the variable is undefined
    global_var_names: Vec<String>,
    output_stream: &'a mut T,
    err_stream: &'a mut U,
}

impl<'a, T: Write, U: Write> VM<'a, T, U> {
    pub fn new(
        main_func_ptr: *mut Object,
        gc: gc::GC,
        str_intern_table: StringInternTable,
        global_var_names: Vec<String>,
        globals: Vec<Option<Value>>,
        output_stream: &'a mut T,
        err_stream: &'a mut U,
    ) -> Self {
        VM {
            call_stack: vec![CallFrame {
                function: main_func_ptr,
                ip: 0,
                stack_start: 0,
            }],
            current_frame: CallFrame {
                function: main_func_ptr,
                ip: 0,
                stack_start: 0,
            },
            stack: Vec::new(),
            gc,
            str_intern_table,
            globals,
            global_var_names,
            output_stream,
            err_stream,
        }
    }

    pub fn run(&mut self) -> InterpretResult {
        loop {
            match self.read_opcode() {
                OpCode::Constant => {
                    let constant = self.read_constant();
                    self.stack.push(constant);
                }
                OpCode::ConstantLong => {
                    let constant = self.read_constant_long();
                    self.stack.push(constant);
                }
                OpCode::Nil => {
                    self.stack.push(Value::Nil);
                }
                OpCode::True => {
                    self.stack.push(Value::Bool(true));
                }
                OpCode::False => {
                    self.stack.push(Value::Bool(false));
                }
                OpCode::Return => {
                    // pop off the return value
                    let ret = self.stack.pop().unwrap();

                    // pop off the current frame
                    self.call_stack.pop();

                    // if the call stack is empty, we're done
                    // (we added an implicit return for the main function)
                    if self.call_stack.is_empty() {
                        return InterpretResult::Ok;
                    }

                    // otherwise, pop off the arguments and the callee from the stack,
                    // push the return value and set the current frame to the top of the call stack
                    self.stack.truncate(self.current_frame.stack_start - 1);
                    self.stack.push(ret);
                    self.current_frame = self.call_stack.last().unwrap().clone();
                }
                OpCode::Negate => match self.stack.last_mut() {
                    Some(Value::Number(value)) => *value = -*value,
                    Some(_) => {
                        return self.runtime_error("Operand to '-' must be a number");
                    }
                    _ => {
                        return InterpretResult::RuntimeError;
                    }
                },
                OpCode::Not => match self.stack.last_mut() {
                    Some(Value::Bool(value)) => *value = !*value,
                    Some(_) => {
                        return self.runtime_error("Operand to '!' must be a bool");
                    }
                    _ => {
                        return InterpretResult::RuntimeError;
                    }
                },
                OpCode::Add => {
                    if self.binary_add() == InterpretResult::Ok {
                        continue;
                    }

                    return InterpretResult::RuntimeError;
                }
                OpCode::Sub => {
                    if self.binary_number_op(|l, r| *l -= r, "Operands to '-' must be numbers")
                        == InterpretResult::Ok
                    {
                        continue;
                    }

                    return InterpretResult::RuntimeError;
                }
                OpCode::Mult => {
                    if self.binary_number_op(|l, r| *l *= r, "Operands to '*' must be numbers")
                        == InterpretResult::Ok
                    {
                        continue;
                    }

                    return InterpretResult::RuntimeError;
                }
                OpCode::Divide => {
                    if self.binary_divide() == InterpretResult::Ok {
                        continue;
                    }

                    return InterpretResult::RuntimeError;
                }
                OpCode::Equal => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left == right);
                }
                OpCode::NotEqual => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left != right);
                }
                OpCode::Greater => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left > right);
                }
                OpCode::GreaterEqual => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left >= right);
                }
                OpCode::Less => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left < right);
                }
                OpCode::LessEqual => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = Value::Bool(*left <= right);
                }
                OpCode::Ternary => {
                    if self.stack.len() < 3 {
                        return InterpretResult::RuntimeError;
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
                            return self
                                .runtime_error("Expected a boolean as ternary operator predicate");
                        }
                    }
                }
                OpCode::Print => {
                    if self.stack.is_empty() {
                        return InterpretResult::RuntimeError;
                    }

                    let _ = writeln!(self.output_stream, "{:#?}", self.stack.pop().unwrap());
                }
                OpCode::Pop => {
                    if self.stack.is_empty() {
                        return InterpretResult::RuntimeError;
                    }

                    self.stack.pop();
                }
                OpCode::DefineGlobal => {
                    // IMP: lookout for GC here
                    let index: usize = self.read_int8();

                    if self.define_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::DefineGlobalLong => {
                    // IMP: lookout for GC here
                    let index = self.read_int24();

                    if self.define_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetGlobal => {
                    let index = self.read_int8();

                    if self.get_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetGlobalLong => {
                    let index = self.read_int24();

                    if self.get_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetGlobal => {
                    let index = self.read_int8();

                    if self.set_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetGlobalLong => {
                    let index = self.read_int24();

                    if self.set_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
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

                    if self.set_local(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetLocalLong => {
                    let index = self.read_int24();

                    if self.set_local(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
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
                            return self.runtime_error("Expected `bool` as condition");
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
                            return self.runtime_error("Expected `bool` as condition");
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

                    if self.call_value(arg_count) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
            }
        }
    }

    fn call_value(&mut self, arg_count: u8) -> InterpretResult {
        if self.stack.len() < (arg_count as usize) + 1 {
            return InterpretResult::RuntimeError;
        }

        let callee = self.stack[self.stack.len() - (arg_count as usize) - 1];

        match callee {
            Value::Object(func_ptr) => {
                let function = unsafe {
                    // SAFETY: we only ever use GC allocated pointers which are
                    // made sure to be valid by the GC
                    match &*func_ptr {
                        Object::Func(func) => func,
                        Object::NativeFunc(native_func) => {
                            let args = &self.stack[self.stack.len() - (arg_count as usize)..];
                            let ret = native_func.call(args);

                            match ret {
                                Ok(value) => {
                                    self.stack.truncate(self.stack.len() - (arg_count as usize) - 1);
                                    self.stack.push(value);
                                    return InterpretResult::Ok;
                                }
                                Err(err) => {
                                    return self.runtime_error(&err);
                                }
                            }
                        }
                        _ => return self.runtime_error("Can only call functions and classes"),
                    }
                };

                self.call(func_ptr, function.arity, arg_count)
            }
            _ => self.runtime_error("Can only call functions and classes"),
        }
    }

    fn call(&mut self, function: *mut Object, arity: u8, arg_count: u8) -> InterpretResult {
        if arity != arg_count {
            return self.runtime_error(&format!(
                "Incorrect number of arguments: expected {}, got {}",
                arity, arg_count
            ));
        }

        // before setting the current frame to the new call frame
        // we need to write back the current ip to the current frame on the call stack
        self.call_stack.last_mut().unwrap().ip = self.current_frame.ip;
        self.call_stack.push(CallFrame {
            function,
            ip: 0,
            stack_start: self.stack.len() - (arg_count as usize),
        });

        // set the current frame to the top of the call stack
        self.current_frame = self.call_stack.last().unwrap().clone();
        InterpretResult::Ok
    }

    fn define_global(&mut self, index: usize) -> InterpretResult {
        if self.stack.len() < 1 {
            return InterpretResult::RuntimeError;
        }

        let initializer = self.stack.pop().unwrap();

        // don't care what the current value is
        match self.globals.get_mut(index) {
            Some(value) => {
                *value = Some(initializer);
                InterpretResult::Ok
            }
            _ => unreachable!(),
        }
    }

    fn get_global(&mut self, index: usize) -> InterpretResult {
        match self.globals.get(index) {
            Some(Some(value)) => {
                self.stack.push(*value);
                InterpretResult::Ok
            }
            _ => self.runtime_error(
                format!("Undefined variable '{}'", self.global_var_names[index]).as_str(),
            ),
        }
    }

    fn set_global(&mut self, index: usize) -> InterpretResult {
        if self.stack.len() < 1 {
            return InterpretResult::RuntimeError;
        }

        let to = self.stack.pop().unwrap();

        match self.globals.get_mut(index) {
            Some(Some(value)) => {
                *value = to;
                self.stack.push(to);
                InterpretResult::Ok
            }
            _ => self.runtime_error(
                format!("Undefined variable '{}'", self.global_var_names[index]).as_str(),
            ),
        }
    }

    fn get_local(&mut self, index: usize) {
        // index is relative to the current frame
        let abs_index = self.current_frame.stack_start + index;
        self.stack.push(self.stack[abs_index]);
    }

    fn set_local(&mut self, index: usize) -> InterpretResult {
        if self.stack.len() < 1 {
            return InterpretResult::RuntimeError;
        }

        // index is relative to the current frame
        let abs_index = self.current_frame.stack_start + index;
        self.stack[abs_index] = *self.stack.last().unwrap();
        InterpretResult::Ok
    }

    fn binary_number_op<F>(&mut self, op: F, err: &str) -> InterpretResult
    where
        F: FnOnce(&mut f64, f64),
    {
        if self.stack.len() < 2 {
            return InterpretResult::RuntimeError;
        }

        let right = self.stack.pop().unwrap();

        match (self.stack.last_mut().unwrap(), right) {
            (Value::Number(left), Value::Number(right)) => {
                op(left, right);
                InterpretResult::Ok
            }
            _ => self.runtime_error(err),
        }
    }

    fn binary_add(&mut self) -> InterpretResult {
        if self.stack.len() < 2 {
            return InterpretResult::RuntimeError;
        }

        let right = self.stack.pop().unwrap();
        let left = self.stack.last_mut().unwrap();

        match (left, right) {
            (Value::Number(left), Value::Number(right)) => {
                *left += right;
                InterpretResult::Ok
            }
            (Value::Object(left), Value::Object(right)) => unsafe {
                // SAFETY: we only ever use GC allocated pointers which are
                // made sure to be valid by the GC
                match (&**left, &*right) {
                    (Object::Str(l_str), Object::Str(r_str)) => {
                        let mut concatenated_str: String =
                            String::with_capacity(l_str.len() + r_str.len());
                        concatenated_str.push_str(l_str);
                        concatenated_str.push_str(r_str);

                        *left = self
                            .str_intern_table
                            .intern_owned(concatenated_str, &mut self.gc);
                        InterpretResult::Ok
                    }
                    _ => self.runtime_error("Operands to '+' must be two numbers or strings"),
                }
            },
            _ => self.runtime_error("Operands to '+' must be two numbers or strings"),
        }
    }

    fn binary_divide(&mut self) -> InterpretResult {
        if self.stack.len() < 2 {
            return InterpretResult::RuntimeError;
        }

        let right = self.stack.pop().unwrap();

        match (self.stack.last_mut().unwrap(), right) {
            (Value::Number(_), Value::Number(0.0)) => self.runtime_error("Division by 0"),
            (Value::Number(left), Value::Number(right)) => {
                *left /= right;
                InterpretResult::Ok
            }
            _ => self.runtime_error("Operands to '/' must be numbers"),
        }
    }

    fn chunk(&self) -> &Chunk {
        unsafe {
            // SAFETY: `self.function` is always a valid pointer to a function
            // object, the memory is managed by the GC
            match &*self.current_frame.function {
                Object::Func(Function { chunk, .. }) => chunk,
                _ => unreachable!(),
            }
        }
    }

    fn ip(&self) -> usize {
        self.current_frame.ip
    }

    fn ip_as_mut(&mut self) -> &mut usize {
        &mut self.current_frame.ip
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

    fn runtime_error(&mut self, err: &str) -> InterpretResult {
        // we have to write back the current ip to the current call frame on the call stack
        self.call_stack.last_mut().unwrap().ip = self.current_frame.ip;

        let _ = writeln!(self.err_stream, "Runtime error: {err}");
        let rev_frame_iter = self.call_stack.iter().rev();

        for frame in rev_frame_iter {
            let function = unsafe {
                // SAFETY: we only ever use GC allocated pointers which are
                // made sure to be valid by the GC
                match &*frame.function {
                    Object::Func(func) => func,
                    _ => unreachable!(),
                }
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

        InterpretResult::RuntimeError
    }
}
