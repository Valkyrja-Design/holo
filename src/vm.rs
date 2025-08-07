use super::{chunk, disassembler, value};

pub enum InterpretResult {
    InterpretOk,
    InterpretCompilerError,
    InterpretRuntimeError,
}

pub struct VM<'a> {
    chunk: &'a chunk::Chunk,
    ip: usize,
    stack: Vec<value::Value>,
    disassembler: disassembler::Diassembler<'a>,
}

impl<'a> VM<'a> {
    fn new(chunk: &'a mut chunk::Chunk) -> Self {
        VM {
            chunk,
            ip: 0,
            stack: vec![],
            disassembler: disassembler::Diassembler::new(chunk, "some chunk"),
        }
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            match self.read_opcode() {
                chunk::OpCode::OpConstant => {
                    println!("{:#?}", self.read_constant());
                }
                chunk::OpCode::OpConstantLong => {
                    println!("{:#?}", self.read_constant_long());
                }
                chunk::OpCode::OpReturn => {
                    return InterpretResult::InterpretOk;
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

            chunk.write_opcode(chunk::OpCode::OpConstant, 1);
            chunk.write_byte(idx, 1);
        }

        for _ in 256..512 {
            let idx = chunk.add_constant(125.25);

            chunk.write_opcode(chunk::OpCode::OpConstantLong, 2);
            chunk.write_as_24bit_int(idx, 2);
        }

        chunk.write_opcode(chunk::OpCode::OpReturn, 3);
        
        let mut vm = VM::new(&mut chunk);

        vm.run();
    }
}