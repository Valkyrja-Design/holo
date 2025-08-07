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
        chunk::OpCode::Constant => const_instr(chunk, offset),
        chunk::OpCode::ConstantLong => const_long_instr(chunk, offset),
        chunk::OpCode::Return => simple_instr("RETURN", offset),
        chunk::OpCode::Negate => simple_instr("NEGATE", offset),
        chunk::OpCode::Add => simple_instr("ADD", offset),
        chunk::OpCode::Sub => simple_instr("SUB", offset),
        chunk::OpCode::Mult => simple_instr("MULT", offset),
        chunk::OpCode::Divide => simple_instr("DIVIDE", offset),
        chunk::OpCode::Ternary => offset,
    }
}

fn const_instr(chunk: &chunk::Chunk, offset: usize) -> usize {
    let idx = chunk.code[offset + 1];

    println!("CONSTANT {}", chunk.constants[idx as usize]);

    offset + 2
}

fn const_long_instr(chunk: &chunk::Chunk, offset: usize) -> usize {
    let idx = chunk::Chunk::read_as_24bit_int(&chunk.code[offset + 1..offset + 4]);

    println!("CONSTANT_LONG {}", chunk.constants[idx]);

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

        disassemble(&chunk, "simple test chunk");
    }
}
