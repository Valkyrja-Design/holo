use super::{
    chunk::{Chunk, OpCode},
    gc,
    object::Object,
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

pub struct VM<'a, T: Write, U: Write> {
    chunk: Chunk,
    ip: usize,
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
        chunk: Chunk,
        gc: gc::GC,
        str_intern_table: StringInternTable,
        global_var_names: Vec<String>,
        output_stream: &'a mut T,
        err_stream: &'a mut U,
    ) -> Self {
        VM {
            chunk, // Store the reference
            ip: 0,
            stack: vec![],
            gc,
            str_intern_table,
            globals: vec![None; global_var_names.len()],
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
                    return InterpretResult::Ok;
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

                    writeln!(self.output_stream, "{:#?}", self.stack.pop().unwrap());
                }
                OpCode::Pop => {
                    if self.stack.is_empty() {
                        return InterpretResult::RuntimeError;
                    }

                    self.stack.pop();
                }
                OpCode::DefineGlobal => {
                    // IMP: lookout for GC here
                    let index: usize = self.read_int();

                    if self.define_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::DefineGlobalLong => {
                    // IMP: lookout for GC here
                    let index = self.read_int_long();

                    if self.define_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetGlobal => {
                    let index = self.read_int();

                    if self.get_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetGlobalLong => {
                    let index = self.read_int_long();

                    if self.get_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetGlobal => {
                    let index = self.read_int();

                    if self.set_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetGlobalLong => {
                    let index = self.read_int_long();

                    if self.set_global(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetLocal => {
                    let index = self.read_int();

                    self.get_local(index);
                }
                OpCode::GetLocalLong => {
                    let index = self.read_int_long();

                    self.get_local(index);
                }
                OpCode::SetLocal => {
                    let index = self.read_int();

                    if self.set_local(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetLocalLong => {
                    let index = self.read_int_long();

                    if self.set_local(index) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::PopN => {
                    let n = self.read_int();

                    self.stack.truncate(self.stack.len() - n);
                }
                OpCode::PopNLong => {
                    let n = self.read_int_long();

                    self.stack.truncate(self.stack.len() - n);
                }
            }
        }
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
        self.stack.push(self.stack[index]);
    }

    fn set_local(&mut self, index: usize) -> InterpretResult {
        if self.stack.len() < 1 {
            return InterpretResult::RuntimeError;
        }

        self.stack[index] = *self.stack.last().unwrap();
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

    fn read_opcode(&mut self) -> OpCode {
        OpCode::from(self.read_byte())
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];

        self.ip += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        let idx = self.read_byte() as usize;

        self.chunk.constants[idx]
    }

    fn read_constant_long(&mut self) -> Value {
        let idx = Chunk::read_as_24bit_int(&self.chunk.code[self.ip..self.ip + 3]);

        self.ip += 3;

        self.chunk.constants[idx]
    }

    fn read_int(&mut self) -> usize {
        self.read_byte() as usize
    }

    fn read_int_long(&mut self) -> usize {
        let idx = Chunk::read_as_24bit_int(&self.chunk.code[self.ip..self.ip + 3]);
        self.ip += 3;

        idx
    }

    fn runtime_error(&mut self, err: &str) -> InterpretResult {
        let instr = self.ip - 1;
        let line = self.chunk.get_line_of(instr);

        let _ = writeln!(self.err_stream, "[line {line}] Runtime error: {err}");
        InterpretResult::RuntimeError
    }
}
