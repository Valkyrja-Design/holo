use super::value;

#[derive(Clone, Copy)]
pub enum OpCode {
    Constant,
    ConstantLong, // stores index as 24 bit integer
    Nil,
    True,
    False,
    Return,
    Negate,
    Add,
    Sub,
    Mult,
    Divide,
    Ternary,
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => OpCode::Constant,
            1 => OpCode::ConstantLong,
            2 => OpCode::Nil,
            3 => OpCode::True,
            4 => OpCode::False,
            5 => OpCode::Return,
            6 => OpCode::Negate,
            7 => OpCode::Add,
            8 => OpCode::Sub,
            9 => OpCode::Mult,
            10 => OpCode::Divide,
            11 => OpCode::Ternary,
            12 => OpCode::Not,
            13 => OpCode::Equal,
            14 => OpCode::NotEqual,
            15 => OpCode::Greater,
            16 => OpCode::GreaterEqual,
            17 => OpCode::Less,
            18 => OpCode::LessEqual,
            _ => panic!("invalid opcode!"),
        }
    }
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> u8 {
        match value {
            OpCode::Constant => 0,
            OpCode::ConstantLong => 1,
            OpCode::Nil => 2,
            OpCode::True => 3,
            OpCode::False => 4,
            OpCode::Return => 5,
            OpCode::Negate => 6,
            OpCode::Add => 7,
            OpCode::Sub => 8,
            OpCode::Mult => 9,
            OpCode::Divide => 10,
            OpCode::Ternary => 11,
            OpCode::Not => 12,
            OpCode::Equal => 13,
            OpCode::NotEqual => 14,
            OpCode::Greater => 15,
            OpCode::GreaterEqual => 16,
            OpCode::Less => 17,
            OpCode::LessEqual => 18,
        }
    }
}

#[derive(Debug)]
pub struct LineInfo {
    byte_idx: usize,
    line: usize,
}

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<value::Value>,
    pub line_info: Vec<LineInfo>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: vec![],
            constants: vec![],
            line_info: vec![],
        }
    }

    pub fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);

        if let Some(LineInfo {
            byte_idx: _,
            line: prev_line,
        }) = self.line_info.last()
        {
            if *prev_line != line {
                self.line_info.push(LineInfo {
                    byte_idx: self.code.len() - 1,
                    line,
                });
            }
        } else {
            self.line_info.push(LineInfo { byte_idx: 0, line });
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8], lines: &[usize]) {
        for (byte, line) in bytes.iter().zip(lines.iter()) {
            self.write_byte(*byte, *line);
        }
    }

    pub fn write_opcode(&mut self, opcode: OpCode, line: usize) {
        self.write_byte(opcode.into(), line);
    }

    pub fn write_as_24bit_int(&mut self, mut value: usize, line: usize) {
        const MASK: usize = (1usize << 8) - 1;
        let mut bytes: [u8; 3] = [0; 3];

        bytes[2] = (value & MASK) as u8;
        value >>= 8;

        bytes[1] = (value & MASK) as u8;
        value >>= 8;

        bytes[0] = (value & MASK) as u8;

        self.write_bytes(&bytes, &[line; 3]);
    }

    pub fn add_constant(&mut self, value: value::Value) -> usize {
        self.constants.push(value);

        self.constants.len() - 1
    }

    pub fn read_as_24bit_int(bytes: &[u8]) -> usize {
        let a = bytes[0] as usize;
        let b = bytes[1] as usize;
        let c = bytes[2] as usize;

        (a << 16) + (b << 8) + c
    }

    pub fn get_line_of(&self, byte_idx: usize) -> usize {
        let high = self.line_info.partition_point(|x| x.byte_idx <= byte_idx);

        self.line_info[high - 1].line
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk::new()
    }
}
