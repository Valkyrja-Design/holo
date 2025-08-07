use super::chunk;

pub fn disassemble(chunk: &chunk::Chunk, chunk_name: &str) {
    println!("== {} ==", chunk_name);

    let mut offset: usize = 0;

    while offset < chunk.code.len() {
        offset = disassemble_instr(chunk, offset);
    }
}

pub fn disassemble_instr(chunk: &chunk::Chunk, offset: usize) -> usize {
    print!("{:04} {:04} ", offset, chunk.get_line_of(offset));

    let instr = chunk.code[offset];

    match chunk::OpCode::from(instr) {
        chunk::OpCode::OpConstant => const_instr(chunk, offset),
        chunk::OpCode::OpConstantLong => const_long_instr(chunk, offset),
        chunk::OpCode::OpReturn => simple_instr("OP_RETURN", offset),
        chunk::OpCode::OpNegate => simple_instr("OP_NEGATE", offset),
        chunk::OpCode::OpAdd => simple_instr("OP_ADD", offset),
        chunk::OpCode::OpSub => simple_instr("OP_SUB", offset),
        chunk::OpCode::OpMult => simple_instr("OP_MULT", offset),
        chunk::OpCode::OpDivide => simple_instr("OP_DIVIDE", offset),
    }
}

fn const_instr(chunk: &chunk::Chunk, offset: usize) -> usize {
    let idx = chunk.code[offset + 1];

    println!("OP_CONSTANT {}", chunk.constants[idx as usize]);

    offset + 2
}

fn const_long_instr(chunk: &chunk::Chunk, offset: usize) -> usize {
    let idx = chunk::Chunk::read_as_24bit_int(&chunk.code[offset + 1..offset + 4]);

    println!("OP_CONSTANT_LONG {}", chunk.constants[idx]);

    offset + 4
}

fn simple_instr(name: &str, offset: usize) -> usize {
    println!("{}", name);

    offset + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_chunk() {
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

        disassemble(&chunk, "simple test chunk");
    }
}
