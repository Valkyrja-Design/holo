use super::{chunk, value};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM<'a> {
    chunk: &'a chunk::Chunk,
    ip: usize,
    stack: Vec<value::Value>,
}

impl<'a> VM<'a> {
    pub fn new(chunk: &'a chunk::Chunk) -> Self {
        VM {
            chunk, // Store the reference
            ip: 0,
            stack: vec![],
        }
    }

    pub fn run(&mut self) -> InterpretResult {
        loop {
            match self.read_opcode() {
                chunk::OpCode::Constant => {
                    let constant = self.read_constant();
                    self.stack.push(constant);
                }
                chunk::OpCode::ConstantLong => {
                    let constant = self.read_constant_long();
                    self.stack.push(constant);
                }
                chunk::OpCode::Nil => {
                    self.stack.push(value::Value::Nil);
                }
                chunk::OpCode::True => {
                    self.stack.push(value::Value::Bool(true));
                }
                chunk::OpCode::False => {
                    self.stack.push(value::Value::Bool(false));
                }
                chunk::OpCode::Return => {
                    println!("{:#?}", self.stack.pop().unwrap());

                    return InterpretResult::Ok;
                }
                chunk::OpCode::Negate => match self.stack.last_mut() {
                    Some(value::Value::Number(value)) => *value = -*value,
                    Some(_) => {
                        self.runtime_error("Operand to '-' must be a number");
                        return InterpretResult::RuntimeError;
                    }
                    _ => {
                        return InterpretResult::RuntimeError;
                    }
                },
                chunk::OpCode::Not => match self.stack.last_mut() {
                    Some(value::Value::Bool(value)) => *value = !*value,
                    Some(_) => {
                        self.runtime_error("Operand to '-' must be a bool");
                        return InterpretResult::RuntimeError;
                    }
                    _ => {
                        return InterpretResult::RuntimeError;
                    }
                },
                chunk::OpCode::Add => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    if let value::Value::Number(left) = left {
                        if let value::Value::Number(right) = right {
                            *left += right;
                            continue;
                        }
                    }

                    self.runtime_error("Operands to '+' must be numbers");
                    return InterpretResult::RuntimeError;
                }
                chunk::OpCode::Sub => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    if let value::Value::Number(left) = left {
                        if let value::Value::Number(right) = right {
                            *left -= right;
                            continue;
                        }
                    }

                    self.runtime_error("Operands to '-' must be numbers");
                    return InterpretResult::RuntimeError;
                }
                chunk::OpCode::Mult => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    if let value::Value::Number(left) = left {
                        if let value::Value::Number(right) = right {
                            *left *= right;
                            continue;
                        }
                    }

                    self.runtime_error("Operands to '*' must be numbers");
                    return InterpretResult::RuntimeError;
                }
                chunk::OpCode::Divide => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    if let value::Value::Number(left) = left {
                        match right {
                            value::Value::Number(0.0) => {
                                self.runtime_error("Division by 0");
                                continue;
                            }
                            value::Value::Number(right) => {
                                *left /= right;
                                continue;
                            }
                            _ => (),
                        }
                    }

                    self.runtime_error("Operands to '/' must be numbers");
                    return InterpretResult::RuntimeError;
                }
                chunk::OpCode::Equal => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = value::Value::Bool(*left == right);
                }
                chunk::OpCode::NotEqual => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = value::Value::Bool(*left != right);
                }
                chunk::OpCode::Greater => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = value::Value::Bool(*left > right);
                }
                chunk::OpCode::GreaterEqual => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = value::Value::Bool(*left >= right);
                }
                chunk::OpCode::Less => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = value::Value::Bool(*left < right);
                }
                chunk::OpCode::LessEqual => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left = value::Value::Bool(*left <= right);
                }
                chunk::OpCode::Ternary => {
                    if self.stack.len() < 3 {
                        return InterpretResult::RuntimeError;
                    }

                    let else_value = self.stack.pop().unwrap();
                    let then_value = self.stack.pop().unwrap();
                    let predicate = self.stack.last_mut().unwrap();

                    match predicate {
                        value::Value::Bool(value) => {
                            if *value {
                                *predicate = then_value;
                            } else {
                                *predicate = else_value;
                            }
                        }
                        _ => {
                            self.runtime_error("Expected a boolean as ternary operator predicate");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
            }
        }
    }

    fn read_opcode(&mut self) -> chunk::OpCode {
        chunk::OpCode::from(self.read_byte())
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];

        self.ip += 1;

        byte
    }

    fn read_constant(&mut self) -> value::Value {
        let idx = self.read_byte() as usize;

        self.chunk.constants[idx]
    }

    fn read_constant_long(&mut self) -> value::Value {
        let idx = chunk::Chunk::read_as_24bit_int(&self.chunk.code[self.ip..self.ip + 3]);

        self.ip += 3;

        self.chunk.constants[idx]
    }

    fn runtime_error(&self, err: &str) {
        let instr = self.ip - 1;
        let line = self.chunk.get_line_of(instr);

        eprintln!("[line {line}] Runtime error: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_chunk() {
        let mut chunk = chunk::Chunk::default();

        for _ in 0..256 {
            let idx = chunk.add_constant(value::Value::Number(1.23)) as u8;

            chunk.write_opcode(chunk::OpCode::Constant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 256..512 {
            let idx = chunk.add_constant(value::Value::Number(125.25));

            chunk.write_opcode(chunk::OpCode::ConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(chunk::OpCode::Return, 3);

        let mut vm = VM::new(&chunk);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }

    #[test]
    fn arithmetic() {
        let mut chunk = chunk::Chunk::default();

        for _ in 0..2 {
            let idx = chunk.add_constant(value::Value::Number(1.23)) as u8;

            chunk.write_opcode(chunk::OpCode::Constant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 2..4 {
            let idx = chunk.add_constant(value::Value::Number(125.25));

            chunk.write_opcode(chunk::OpCode::ConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(chunk::OpCode::Negate, 3);
        chunk.write_opcode(chunk::OpCode::Add, 4);
        chunk.write_opcode(chunk::OpCode::Sub, 5);
        chunk.write_opcode(chunk::OpCode::Divide, 6);

        chunk.write_opcode(chunk::OpCode::Return, 7);

        let mut vm = VM::new(&chunk);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }

    #[test]
    fn benchmark_negation() {
        let mut chunk = chunk::Chunk::new();
        const INSTR_COUNT: usize = 1000000000;

        let idx = chunk.add_constant(value::Value::Number(125.25));

        chunk.write_opcode(chunk::OpCode::ConstantLong, 2);
        chunk.write_as_24bit_int(idx, 2);

        for _ in 0..INSTR_COUNT {
            chunk.write_opcode(chunk::OpCode::Negate, 3);
        }

        chunk.write_opcode(chunk::OpCode::Return, 3);

        let mut vm = VM::new(&chunk);

        assert_eq!(vm.run(), InterpretResult::Ok);
    }
}
