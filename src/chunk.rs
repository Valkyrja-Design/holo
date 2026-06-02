use crate::value::Value;

/// Represents an opcode in `Holo`'s instruction set
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum OpCode {
    /// CONSTANT <index: u8>
    /// Produces a value stored at `index` in the chunk's constant table
    Constant,
    /// CONSTANT <index: u24>
    /// Produces a value stored at `index` in the chunk's constant table
    ConstantLong,
    /// Produces a `nil` value
    Nil,
    /// Produces a `true` value
    True,
    /// Produces a `false` value
    False,
    /// Returns from the current function. The return value is the top value on the stack
    Return,
    Negate,
    Add,
    Sub,
    Mult,
    Divide,
    /// Evaluates a ternary expression (`condition ? true_value : false_value`)
    Ternary,
    /// Logical negation
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    /// Print the top value on the stack to standard output
    Print,
    /// Pop the top value off the stack
    Pop,
    /// DEFINE_GLOBAL <index: u8>
    /// Define a global variable
    DefineGlobal,
    /// DEFINE_GLOBAL <index: u24>
    /// Define a global variable
    DefineGlobalLong,
    /// GET_GLOBAL <index: u8>
    GetGlobal,
    /// GET_GLOBAL <index: u24>
    GetGlobalLong,
    /// SET_GLOBAL <index: u8>
    SetGlobal,
    /// SET_GLOBAL <index: u24>
    SetGlobalLong,
    /// GET_LOCAL <index: u8>
    GetLocal,
    /// GET_LOCAL <index: u24>
    GetLocalLong,
    /// SET_LOCAL <index: u8>
    SetLocal,
    /// SET_LOCAL <index: u24>
    SetLocalLong,
    /// POP_N <count: u8>
    PopN,
    /// POP_N <count: u24>
    PopNLong,
    /// JUMP_IF_FALSE <offset: u16>
    /// Jumps forward by the given offset if the top value on the stack is false
    JumpIfFalse,
    /// JUMP_IF_TRUE <offset: u16>
    /// Jumps forward by the given offset if the top value on the stack is true
    JumpIfTrue,
    /// JUMP <offset: u16>
    /// Jumps forward by the given offset
    Jump,
    /// LOOP <offset: u16>
    /// Jumps backward by the given offset
    Loop,
    /// CALL <arg_count: u8>
    /// Calls a function with the given number of arguments
    Call,
    /// CLOSURE <index: u8> [(<is_local: u8>, <index: u8>); upvalue_count]
    /// Produces a new closure object. The closure will capture variable from the surrounding scope
    /// as specified in the variadic arguments. `is_local` indicates the variable being captured is
    /// a local variable in the same scope as the closure and `index` is the variable's index in the
    /// stack, otherwise it is an upvalue captured from an outer scope and `index` is the variable's
    /// index in the upvalue vector of the enclosing function
    Closure,
    /// CLOSURE <index: u24>
    /// Produces a new closure object
    ClosureLong,
    /// GET_UPVALUE <index: u8>
    /// Pushes the upvalue at the given index onto the stack
    GetUpvalue,
    /// GET_UPVALUE <index: u24>
    /// Pushes the upvalue at the given index onto the stack
    GetUpvalueLong,
    /// SET_UPVALUE <index: u8>
    /// Sets the upvalue at the given index to the top value on the stack
    SetUpvalue,
    /// SET_UPVALUE <index: u24>
    /// Sets the upvalue at the given index to the top value on the stack
    SetUpvalueLong,
    /// CLOSE_UPVALUE
    /// Closes the upvalue pointing to the local at the top of the stack
    CloseUpvalue,
    /// CLASS <index: u8>
    /// Produces a new class object. The class's name is stored at `index` in the chunk's constant
    /// table
    Class,
    /// GET_PROPERTY <index: u8>
    /// Gets a property from the object at the top of the stack. The property's name is stored at
    /// `index` in the chunk's constant table
    GetProperty,
    /// SET_PROPERTY <index: u8>
    /// Sets a property on the object at the top of the stack. The property's name is stored at
    /// `index` in the chunk's constant table
    SetProperty,
    /// METHOD <index: u8>
    /// Defines a method of a class. The method's name is stored at `index` in the chunk's constant
    /// table with the method closure at the top of the stack and the class object right below it
    Method,
    /// INVOKE <index: u8> <arg_count: u8>
    /// Invokes a method on the object at the top of the stack. The method's name is stored at
    /// `index` in the chunk's constant table. The class instance on which the method is invoked
    /// lies below the first argument
    Invoke,
    /// INHERIT
    /// Inherit methods from the superclass. The subclass lies at the top of the stack with the
    /// superclass right below it
    Inherit,
    /// GET_SUPER <index: u8>
    /// Gets a method from the superclass. The method's name is stored at `index` in the chunk's
    /// constant table. The superclass lies at the top of the stack with the subclass instance
    /// right below it
    GetSuper,
    /// SUPER_INVOKE <index: u8> <arg_count: u8>
    /// Invokes a method on the superclass. The method's name is stored at `index` in the chunk's
    /// constant table. The superclass lies at the top of the stack with the subclass instance
    /// right below it. Accesses using `this` in the superclass method will resolve to the
    /// subclass instance
    SuperInvoke,
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Constant,
            1 => Self::ConstantLong,
            2 => Self::Nil,
            3 => Self::True,
            4 => Self::False,
            5 => Self::Return,
            6 => Self::Negate,
            7 => Self::Add,
            8 => Self::Sub,
            9 => Self::Mult,
            10 => Self::Divide,
            11 => Self::Ternary,
            12 => Self::Not,
            13 => Self::Equal,
            14 => Self::NotEqual,
            15 => Self::Greater,
            16 => Self::GreaterEqual,
            17 => Self::Less,
            18 => Self::LessEqual,
            19 => Self::Print,
            20 => Self::Pop,
            21 => Self::DefineGlobal,
            22 => Self::DefineGlobalLong,
            23 => Self::GetGlobal,
            24 => Self::GetGlobalLong,
            25 => Self::SetGlobal,
            26 => Self::SetGlobalLong,
            27 => Self::GetLocal,
            28 => Self::GetLocalLong,
            29 => Self::SetLocal,
            30 => Self::SetLocalLong,
            31 => Self::PopN,
            32 => Self::PopNLong,
            33 => Self::JumpIfFalse,
            34 => Self::JumpIfTrue,
            35 => Self::Jump,
            36 => Self::Loop,
            37 => Self::Call,
            38 => Self::Closure,
            39 => Self::ClosureLong,
            40 => Self::GetUpvalue,
            41 => Self::GetUpvalueLong,
            42 => Self::SetUpvalue,
            43 => Self::SetUpvalueLong,
            44 => Self::CloseUpvalue,
            45 => Self::Class,
            46 => Self::GetProperty,
            47 => Self::SetProperty,
            48 => Self::Method,
            49 => Self::Invoke,
            50 => Self::Inherit,
            51 => Self::GetSuper,
            52 => Self::SuperInvoke,
            _ => unreachable!("invalid opcode!"),
        }
    }
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> u8 {
        value as u8
    }
}

/// Represents line information for a specific bytecode.
#[derive(Debug)]
pub struct LineInfo {
    byte_idx: usize,
    line: usize,
}

/// Represents a piece of compiled bytecode, associated constants and line information.
#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
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

    pub fn write_int24(&mut self, mut value: usize, line: usize) {
        const MASK: usize = (1usize << 8) - 1;
        let mut bytes: [u8; 3] = [0; 3];

        bytes[2] = (value & MASK) as u8;
        value >>= 8;

        bytes[1] = (value & MASK) as u8;
        value >>= 8;

        bytes[0] = (value & MASK) as u8;

        self.write_bytes(&bytes, &[line; 3]);
    }

    pub fn write_int16(&mut self, value: usize, line: usize) {
        const MASK: usize = (1usize << 8) - 1;
        let mut bytes: [u8; 2] = [0; 2];

        bytes[1] = (value & MASK) as u8;
        bytes[0] = ((value >> 8) & MASK) as u8;

        self.write_bytes(&bytes, &[line; 2]);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn read_int24(bytes: &[u8]) -> usize {
        let a = bytes[0] as usize;
        let b = bytes[1] as usize;
        let c = bytes[2] as usize;

        (a << 16) | (b << 8) | c
    }

    pub fn read_int16(bytes: &[u8]) -> usize {
        let a = bytes[0] as usize;
        let b = bytes[1] as usize;

        (a << 8) | b
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int24_roundtrip() {
        let value = 0x123456;
        let mut bytes = vec![0u8; 3];

        bytes[0] = ((value >> 16) & 0xFF) as u8;
        bytes[1] = ((value >> 8) & 0xFF) as u8;
        bytes[2] = (value & 0xFF) as u8;

        let result = Chunk::read_int24(&bytes);
        assert_eq!(result, value);
    }

    #[test]
    fn test_int16_roundtrip() {
        let value = 0x1234;
        let mut bytes = vec![0u8; 2];

        bytes[0] = ((value >> 8) & 0xFF) as u8;
        bytes[1] = (value & 0xFF) as u8;

        let result = Chunk::read_int16(&bytes);
        assert_eq!(result, value);
    }

    #[test]
    fn test_line_info() {
        let mut chunk = Chunk::new();
        chunk.write_byte(0, 1);
        chunk.write_byte(1, 1);
        chunk.write_byte(2, 2);
        chunk.write_byte(3, 3);

        assert_eq!(chunk.get_line_of(0), 1);
        assert_eq!(chunk.get_line_of(1), 1);
        assert_eq!(chunk.get_line_of(2), 2);
        assert_eq!(chunk.get_line_of(3), 3);
    }
}
