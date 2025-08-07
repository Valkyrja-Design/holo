use super::chunk;

pub struct Diassembler<'a> {
    chunk: &'a chunk::Chunk,
    chunk_name: &'a str,
}

impl<'a> Diassembler<'a> {
    pub fn new(chunk: &'a chunk::Chunk, chunk_name: &'a str) -> Diassembler<'a> {
        Diassembler { chunk, chunk_name }
    }

    pub fn disassemble(&self) {
        println!("== {} ==", self.chunk_name);

        let mut offset: usize = 0;

        while offset < self.chunk.code.len() {
            offset = self.disassemble_instr(offset);
        }
    }

    pub fn disassemble_instr(&self, offset: usize) -> usize {
        print!("{:04} {:04} ", offset, self.chunk.get_line_of(offset));

        let instr = self.chunk.code[offset];

        match chunk::OpCode::from(instr) {
            chunk::OpCode::OpConstant => self.const_instr(offset),
            chunk::OpCode::OpConstantLong => self.const_long_instr(offset),
            chunk::OpCode::OpReturn => self.simple_instr("OP_RETURN", offset),
            _ => {
                println!("Unknown opcode {}", instr);
                offset + 1
            }
        }
    }

    fn const_instr(&self, offset: usize) -> usize {
        let idx = self.chunk.code[offset + 1];

        println!("OP_CONSTANT {}", self.chunk.constants[idx as usize]);

        offset + 2
    }

    fn const_long_instr(&self, offset: usize) -> usize {
        let idx = chunk::Chunk::read_as_24bit_int(&self.chunk.code[offset + 1..offset + 4]);

        println!("OP_CONSTANT_LONG {}", self.chunk.constants[idx]);

        offset + 4
    }

    fn simple_instr(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);

        offset + 1
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

        let disassembler = Diassembler::new(&chunk, "simple test chunk");

        disassembler.disassemble();
    }
}
