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
                chunk::OpCode::Return => {
                    println!("{:#?}", self.stack.pop().unwrap());

                    return InterpretResult::Ok;
                }
                chunk::OpCode::Negate => {
                    if let Some(value) = self.stack.last_mut() {
                        *value = -*value;
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                chunk::OpCode::Add => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left += right;
                }
                chunk::OpCode::Sub => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left -= right;
                }
                chunk::OpCode::Mult => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    *left *= right;
                }
                chunk::OpCode::Divide => {
                    if self.stack.len() < 2 {
                        return InterpretResult::RuntimeError;
                    }

                    let right = self.stack.pop().unwrap();
                    let left = self.stack.last_mut().unwrap();

                    if right == 0.0 {
                        return InterpretResult::RuntimeError;
                    }

                    *left /= right;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_chunk() {
        let mut chunk = chunk::Chunk::default();

        for _ in 0..256 {
            let idx = chunk.add_constant(1.23) as u8;

            chunk.write_opcode(chunk::OpCode::Constant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 256..512 {
            let idx = chunk.add_constant(125.25);

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
            let idx = chunk.add_constant(1.23) as u8;

            chunk.write_opcode(chunk::OpCode::Constant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 2..4 {
            let idx = chunk.add_constant(125.25);

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

        let idx = chunk.add_constant(125.25);

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
