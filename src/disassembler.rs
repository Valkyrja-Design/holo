use super::chunk::{Chunk, OpCode};

pub fn disassemble(chunk: &Chunk, chunk_name: &str) {
    println!("== {} ==", chunk_name);

    let mut offset: usize = 0;

    while offset < chunk.code.len() {
        offset = disassemble_instr(chunk, offset);
    }
}

pub fn disassemble_instr(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} {:04} ", offset, chunk.get_line_of(offset));

    let instr = chunk.code[offset];

    match OpCode::from(instr) {
        OpCode::Constant => const_instr(chunk, offset),
        OpCode::ConstantLong => const_long_instr(chunk, offset),
        OpCode::Nil => simple_instr("NIL", offset),
        OpCode::True => simple_instr("TRUE", offset),
        OpCode::False => simple_instr("FALSE", offset),
        OpCode::Return => simple_instr("RETURN", offset),
        OpCode::Negate => simple_instr("NEGATE", offset),
        OpCode::Add => simple_instr("ADD", offset),
        OpCode::Sub => simple_instr("SUB", offset),
        OpCode::Mult => simple_instr("MULT", offset),
        OpCode::Divide => simple_instr("DIVIDE", offset),
        OpCode::Ternary => simple_instr("TERNARY", offset),
        OpCode::Not => simple_instr("NOT", offset),
        OpCode::Equal => simple_instr("EQUAL", offset),
        OpCode::NotEqual => simple_instr("NOT_EQUAL", offset),
        OpCode::Greater => simple_instr("GREATER", offset),
        OpCode::GreaterEqual => simple_instr("GREATER_EQUAL", offset),
        OpCode::Less => simple_instr("LESS", offset),
        OpCode::LessEqual => simple_instr("LESS_EQUAL", offset),
        OpCode::Print => simple_instr("PRINT", offset),
        OpCode::Pop => simple_instr("POP", offset),
    }
}

fn const_instr(chunk: &Chunk, offset: usize) -> usize {
    let idx = chunk.code[offset + 1];

    println!("CONSTANT {:#?}", chunk.constants[idx as usize]);

    offset + 2
}

fn const_long_instr(chunk: &Chunk, offset: usize) -> usize {
    let idx = Chunk::read_as_24bit_int(&chunk.code[offset + 1..offset + 4]);

    println!("CONSTANT_LONG {:#?}", chunk.constants[idx]);

    offset + 4
}

fn simple_instr(name: &str, offset: usize) -> usize {
    println!("{}", name);

    offset + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn simple_chunk() {
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

        disassemble(&chunk, "simple test chunk");
    }
}
