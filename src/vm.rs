use std::ptr::NonNull;

use super::{
    chunk::{Chunk, OpCode},
    gc,
    object::Object,
    table::{StringInternTable, StringTable},
    value::Value,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    gc: gc::GC,
    str_intern_table: StringInternTable,
    globals: StringTable,
}

impl VM {
    pub fn new(chunk: Chunk, gc: gc::GC, str_intern_table: StringInternTable) -> Self {
        VM {
            chunk, // Store the reference
            ip: 0,
            stack: vec![],
            gc,
            str_intern_table,
            globals: StringTable::new(),
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
                        return self.runtime_error("Operand to '-' must be a bool");
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

                    println!("{:#?}", self.stack.pop().unwrap());
                }
                OpCode::Pop => {
                    if self.stack.is_empty() {
                        return InterpretResult::RuntimeError;
                    }

                    self.stack.pop();
                }
                OpCode::DefineGlobal => {
                    // IMP: lookout for GC here
                    let name_val = self.read_constant();

                    if self.define_global(name_val) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::DefineGlobalLong => {
                    // IMP: lookout for GC here
                    let name_val = self.read_constant_long();

                    if self.define_global(name_val) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetGlobal => {
                    let name_val = self.read_constant();

                    if self.get_global(name_val) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetGlobalLong => {
                    let name_val = self.read_constant_long();

                    if self.get_global(name_val) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetGlobal => {
                    let name_val = self.read_constant();

                    if self.set_global(name_val) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetGlobalLong => {
                    let name_val = self.read_constant_long();

                    if self.set_global(name_val) != InterpretResult::Ok {
                        return InterpretResult::RuntimeError;
                    }
                }
            }
        }
    }

    fn define_global(&mut self, name_val: Value) -> InterpretResult {
        if self.stack.len() < 1 {
            return InterpretResult::RuntimeError;
        }

        let name = self.extract_string_key(name_val);

        let initializer = self.stack.pop().unwrap();
        self.globals.insert(name, initializer);

        InterpretResult::Ok
    }

    fn get_global(&mut self, name_val: Value) -> InterpretResult {
        let name = self.extract_string_key(name_val);

        match self.globals.get(name) {
            Some(value) => {
                self.stack.push(*value);
                InterpretResult::Ok
            }
            None => {
                // SAFETY: we only ever use GC allocated pointers which are
                // made sure to be valid by the GC
                let s = unsafe { name.as_ref() };
                self.runtime_error(format!("Undefined variable '{}'", s).as_str())
            }
        }
    }

    fn set_global(&mut self, name_val: Value) -> InterpretResult {
        if self.stack.len() < 1 {
            return InterpretResult::RuntimeError;
        }

        let name = self.extract_string_key(name_val);
        let to = self.stack.pop().unwrap();

        match self.globals.insert(name, to) {
            Some(_) => {
                self.stack.push(to);
                InterpretResult::Ok
            }
            None => {
                // SAFETY: we only ever use GC allocated pointers which are
                // made sure to be valid by the GC
                let s = unsafe { name.as_ref() };
                self.runtime_error(format!("Undefined variable '{}'", s).as_str())
            }
        }
    }

    fn extract_string_key(&self, value: Value) -> NonNull<str> {
        match value {
            // SAFETY: we only ever use GC allocated pointers which are
            // made sure to be valid by the GC
            Value::Object(handle) => unsafe {
                match &*handle {
                    Object::Str(s) => NonNull::from(s.as_str()),
                    _ => unreachable!(),
                }
            },
            _ => unreachable!(),
        }
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

    fn runtime_error(&self, err: &str) -> InterpretResult {
        let instr = self.ip - 1;
        let line = self.chunk.get_line_of(instr);

        eprintln!("[line {line}] Runtime error: {err}");
        InterpretResult::RuntimeError
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_chunk() {
        let mut chunk = Chunk::default();

        for _ in 0..256 {
            let idx = chunk.add_constant(Value::Number(1.23)) as u8;

            chunk.write_opcode(OpCode::Constant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 256..512 {
            let idx = chunk.add_constant(Value::Number(125.25));

            chunk.write_opcode(OpCode::ConstantLong, 1);
            chunk.write_as_24bit_int(idx, 1);
        }

        chunk.write_opcode(OpCode::Print, 1);
        chunk.write_opcode(OpCode::Return, 2);

        let gc = gc::GC::new();
        let str_intern_table = StringInternTable::new();
        let mut vm = VM::new(chunk, gc, str_intern_table);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }

    #[test]
    fn arithmetic() {
        let mut chunk = Chunk::default();

        for _ in 0..2 {
            let idx = chunk.add_constant(Value::Number(1.23)) as u8;

            chunk.write_opcode(OpCode::Constant, 7);
            chunk.write_byte(idx, 7);
        }

        for _ in 2..4 {
            let idx = chunk.add_constant(Value::Number(125.25));

            chunk.write_opcode(OpCode::ConstantLong, 7);
            chunk.write_as_24bit_int(idx, 7);
        }

        chunk.write_opcode(OpCode::Negate, 7);
        chunk.write_opcode(OpCode::Add, 7);
        chunk.write_opcode(OpCode::Sub, 7);
        chunk.write_opcode(OpCode::Divide, 7);

        chunk.write_opcode(OpCode::Print, 7);
        chunk.write_opcode(OpCode::Return, 8);

        let gc = gc::GC::new();
        let str_intern_table = StringInternTable::new();
        let mut vm = VM::new(chunk, gc, str_intern_table);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }

    #[test]
    fn benchmark_negation() {
        let mut chunk = Chunk::new();
        const INSTR_COUNT: usize = 100000000;

        let idx = chunk.add_constant(Value::Number(125.25));

        chunk.write_opcode(OpCode::ConstantLong, 3);
        chunk.write_as_24bit_int(idx, 3);

        for _ in 0..INSTR_COUNT {
            chunk.write_opcode(OpCode::Negate, 3);
        }

        chunk.write_opcode(OpCode::Print, 3);
        chunk.write_opcode(OpCode::Return, 8);

        let gc = gc::GC::new();
        let str_intern_table = StringInternTable::new();
        let mut vm = VM::new(chunk, gc, str_intern_table);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }
}
