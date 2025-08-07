use super::{chunk, disassembler, value};

pub enum InterpretResult {
    InterpretOk,
    InterpretCompilerError,
    InterpretRuntimeError,
}

pub struct VM {
    source: String,
    chunk: chunk::Chunk,
    ip: usize,
    stack: Vec<value::Value>,
}

impl VM {
    fn new(source: String) -> Self {
        VM {
            source,
            chunk: chunk::Chunk::default(),
            ip: 0,
            stack: vec![],
        }
    }

    fn interpret(&mut self) -> InterpretResult {
        loop {
            match self.read_opcode() {
                chunk::OpCode::OpConstant => {
                    let constant = self.read_constant();

                    self.stack.push(constant);
                    // println!("{:#?}", self.read_constant());
                }
                chunk::OpCode::OpConstantLong => {
                    let constant = self.read_constant_long();

                    self.stack.push(constant);
                    // println!("{:#?}", self.read_constant_long());
                }
                chunk::OpCode::OpReturn => {
                    println!("{:#?}", self.stack.pop().unwrap());

                    return InterpretResult::InterpretOk;
                }
                chunk::OpCode::OpNegate => {
                    if let Some(value) = self.stack.last_mut() {
                        *value = -*value;
                    } else {
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                chunk::OpCode::OpAdd => {
                    let right = self.stack.pop();
                    let left = self.stack.last_mut();

                    if left.is_none() || right.is_none() {
                        return InterpretResult::InterpretRuntimeError;
                    }

                    let left = left.unwrap();
                    let right = right.unwrap();

                    *left += right;
                }
                chunk::OpCode::OpSub => {
                    let right = self.stack.pop();
                    let left = self.stack.last_mut();

                    if left.is_none() || right.is_none() {
                        return InterpretResult::InterpretRuntimeError;
                    }

                    let left = left.unwrap();
                    let right = right.unwrap();

                    *left -= right;
                }
                chunk::OpCode::OpMult => {
                    let right = self.stack.pop();
                    let left = self.stack.last_mut();

                    if left.is_none() || right.is_none() {
                        return InterpretResult::InterpretRuntimeError;
                    }

                    let left = left.unwrap();
                    let right = right.unwrap();

                    *left *= right;
                }
                chunk::OpCode::OpDivide => {
                    let right = self.stack.pop();
                    let left = self.stack.last_mut();

                    if left.is_none() || right.is_none() {
                        return InterpretResult::InterpretRuntimeError;
                    }

                    let left = left.unwrap();
                    let right = right.unwrap();

                    if right == 0.0 {
                        return InterpretResult::InterpretRuntimeError;
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

    fn print_stack(&self) {
        println!("{:#?}", self.stack);
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

            chunk.write_opcode(chunk::OpCode::OpConstant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 256..512 {
            let idx = chunk.add_constant(125.25);

            chunk.write_opcode(chunk::OpCode::OpConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(chunk::OpCode::OpReturn, 3);
    }

    #[test]
    fn arithmetic() {
        let mut chunk = chunk::Chunk::default();

        for _ in 0..2 {
            let idx = chunk.add_constant(1.23) as u8;

            chunk.write_opcode(chunk::OpCode::OpConstant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 2..4 {
            let idx = chunk.add_constant(125.25);

            chunk.write_opcode(chunk::OpCode::OpConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(chunk::OpCode::OpNegate, 3);
        chunk.write_opcode(chunk::OpCode::OpAdd, 4);
        chunk.write_opcode(chunk::OpCode::OpSub, 5);
        chunk.write_opcode(chunk::OpCode::OpDivide, 6);

        chunk.write_opcode(chunk::OpCode::OpReturn, 7);
    }

    #[test]
    fn benchmark_negation() {
        let mut chunk = chunk::Chunk::new();
        const INSTR_COUNT: usize = 1000000000;

        let idx = chunk.add_constant(125.25);

        chunk.write_opcode(chunk::OpCode::OpConstantLong, 2);
        chunk.write_as_24bit_int(idx, 2);

        for _ in 0..INSTR_COUNT {
            chunk.write_opcode(chunk::OpCode::OpNegate, 3);
        }

        chunk.write_opcode(chunk::OpCode::OpReturn, 3);
    }
}
