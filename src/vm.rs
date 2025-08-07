use super::{
    chunk::{Chunk, OpCode},
    gc,
    interntable::StringInternTable,
    object::{ObjRef, Object},
    value::Value,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM<'a> {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    gc: &'a mut gc::GC,
    str_intern_table: StringInternTable,
}

impl<'a> VM<'a> {
    pub fn new(chunk: Chunk, gc: &'a mut gc::GC, str_intern_table: StringInternTable) -> Self {
        VM {
            chunk, // Store the reference
            ip: 0,
            stack: vec![],
            gc,
            str_intern_table,
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
                    println!("{:#?}", self.stack.pop().unwrap());

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
            }
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
                println!("{:#?}", self.str_intern_table);

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

            chunk.write_opcode(OpCode::ConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(OpCode::Return, 3);

        let mut gc = gc::GC::new();
        let str_intern_table = StringInternTable::new();
        let mut vm = VM::new(chunk, &mut gc, str_intern_table);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }

    #[test]
    fn arithmetic() {
        let mut chunk = Chunk::default();

        for _ in 0..2 {
            let idx = chunk.add_constant(Value::Number(1.23)) as u8;

            chunk.write_opcode(OpCode::Constant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 2..4 {
            let idx = chunk.add_constant(Value::Number(125.25));

            chunk.write_opcode(OpCode::ConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(OpCode::Negate, 3);
        chunk.write_opcode(OpCode::Add, 4);
        chunk.write_opcode(OpCode::Sub, 5);
        chunk.write_opcode(OpCode::Divide, 6);

        chunk.write_opcode(OpCode::Return, 7);

        let mut gc = gc::GC::new();
        let str_intern_table = StringInternTable::new();
        let mut vm = VM::new(chunk, &mut gc, str_intern_table);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }

    #[test]
    fn benchmark_negation() {
        let mut chunk = Chunk::new();
        const INSTR_COUNT: usize = 1000000000;

        let idx = chunk.add_constant(Value::Number(125.25));

        chunk.write_opcode(OpCode::ConstantLong, 2);
        chunk.write_as_24bit_int(idx, 2);

        for _ in 0..INSTR_COUNT {
            chunk.write_opcode(OpCode::Negate, 3);
        }

        chunk.write_opcode(OpCode::Return, 3);

        let mut gc = gc::GC::new();
        let str_intern_table = StringInternTable::new();
        let mut vm = VM::new(chunk, &mut gc, str_intern_table);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }
}
