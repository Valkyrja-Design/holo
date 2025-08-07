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
        OpCode::Constant => instr_with_const8(chunk, "CONSTANT", offset),
        OpCode::ConstantLong => instr_with_const24(chunk, "CONSTANT_LONG", offset),
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
        OpCode::DefineGlobal => unary_instr8(chunk, "DEFINE_GLOBAL", offset),
        OpCode::DefineGlobalLong => unary_instr24(chunk, "DEFINE_GLOBAL_LONG", offset),
        OpCode::GetGlobal => unary_instr8(chunk, "GET_GLOBAL", offset),
        OpCode::GetGlobalLong => unary_instr24(chunk, "GET_GLOBAL_LONG", offset),
        OpCode::SetGlobal => unary_instr8(chunk, "SET_GLOBAL", offset),
        OpCode::SetGlobalLong => unary_instr24(chunk, "SET_GLOBAL_LONG", offset),
        OpCode::GetLocal => unary_instr8(chunk, "GET_LOCAL", offset),
        OpCode::GetLocalLong => unary_instr24(chunk, "GET_LOCAL_LONG", offset),
        OpCode::SetLocal => unary_instr8(chunk, "SET_LOCAL", offset),
        OpCode::SetLocalLong => unary_instr24(chunk, "SET_LOCAL_LONG", offset),
        OpCode::PopN => unary_instr8(chunk, "POP_N", offset),
        OpCode::PopNLong => unary_instr24(chunk, "POP_N_LONG", offset),
        OpCode::JumpIfFalse => unary_instr16(chunk, "JUMP_IF_FALSE", offset),
        OpCode::JumpIfTrue => unary_instr16(chunk, "JUMP_IF_TRUE", offset),
        OpCode::Jump => unary_instr16(chunk, "JUMP", offset),
        OpCode::Loop => unary_instr16(chunk, "LOOP", offset),
    }
}

fn instr_with_const8(chunk: &Chunk, name: &str, offset: usize) -> usize {
    let idx = chunk.code[offset + 1];

    println!("{} {:#?}", name, chunk.constants[idx as usize]);
    offset + 2
}

fn instr_with_const24(chunk: &Chunk, name: &str, offset: usize) -> usize {
    let idx = Chunk::read_as_24bit_int(&chunk.code[offset + 1..offset + 4]);

    println!("{} {:#?}", name, chunk.constants[idx]);
    offset + 4
}

fn simple_instr(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}

fn unary_instr8(chunk: &Chunk, name: &str, offset: usize) -> usize {
    let op = chunk.code[offset + 1];

    println!("{} {}", name, op);
    offset + 2
}

fn unary_instr16(chunk: &Chunk, name: &str, offset: usize) -> usize {
    let op: usize = Chunk::read_as_16bit_int(&chunk.code[offset + 1..offset + 3]);

    println!("{} {}", name, op);
    offset + 3
}

fn unary_instr24(chunk: &Chunk, name: &str, offset: usize) -> usize {
    let op: usize = Chunk::read_as_16bit_int(&chunk.code[offset + 1..offset + 4]);

    println!("{} {}", name, op);
    offset + 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_chunk() {
        let mut chunk = Chunk::default();

        // constants
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

        // arithmetic
        chunk.write_opcode(OpCode::Negate, 3);
        chunk.write_opcode(OpCode::Add, 4);
        chunk.write_opcode(OpCode::Sub, 5);
        chunk.write_opcode(OpCode::Divide, 6);
        chunk.write_opcode(OpCode::Mult, 6);

        // Nil, True, False literals
        chunk.write_opcode(OpCode::Nil, 7);
        chunk.write_opcode(OpCode::True, 7);
        chunk.write_opcode(OpCode::False, 7);

        // logical operations
        chunk.write_opcode(OpCode::Not, 8);
        chunk.write_opcode(OpCode::Equal, 8);
        chunk.write_opcode(OpCode::NotEqual, 8);
        chunk.write_opcode(OpCode::Greater, 8);
        chunk.write_opcode(OpCode::GreaterEqual, 8);
        chunk.write_opcode(OpCode::Less, 8);
        chunk.write_opcode(OpCode::LessEqual, 8);

        chunk.write_opcode(OpCode::Print, 9);
        chunk.write_opcode(OpCode::Pop, 9);
        chunk.write_opcode(OpCode::Ternary, 9);

        // global variable operations
        chunk.write_opcode(OpCode::DefineGlobal, 10);
        chunk.write_byte(5, 10);

        chunk.write_opcode(OpCode::DefineGlobalLong, 10);
        chunk.write_as_24bit_int(500, 10);

        chunk.write_opcode(OpCode::GetGlobal, 11);
        chunk.write_byte(5, 11);

        chunk.write_opcode(OpCode::GetGlobalLong, 11);
        chunk.write_as_24bit_int(500, 11);

        chunk.write_opcode(OpCode::SetGlobal, 12);
        chunk.write_byte(5, 12);

        chunk.write_opcode(OpCode::SetGlobalLong, 12);
        chunk.write_as_24bit_int(500, 12);

        // local variable operations
        chunk.write_opcode(OpCode::GetLocal, 13);
        chunk.write_byte(1, 13);

        chunk.write_opcode(OpCode::GetLocalLong, 13);
        chunk.write_as_24bit_int(256, 13);

        chunk.write_opcode(OpCode::SetLocal, 14);
        chunk.write_byte(2, 14);

        chunk.write_opcode(OpCode::SetLocalLong, 14);
        chunk.write_as_24bit_int(257, 14);

        // stack manipulation
        chunk.write_opcode(OpCode::PopN, 15);
        chunk.write_byte(3, 15);

        chunk.write_opcode(OpCode::PopNLong, 15);
        chunk.write_as_24bit_int(300, 15);

        // control flow
        chunk.write_opcode(OpCode::Return, 7);

        chunk.write_opcode(OpCode::Jump, 8);
        chunk.write_as_16bit_int(125, 9);

        chunk.write_opcode(OpCode::JumpIfFalse, 8);
        chunk.write_as_16bit_int(250, 9);

        chunk.write_opcode(OpCode::JumpIfTrue, 8);
        chunk.write_as_16bit_int(375, 9);

        disassemble(&chunk, "simple test chunk");
    }
}
