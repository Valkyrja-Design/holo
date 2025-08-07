use super::chunk::Chunk;
use std::fmt::Debug;

pub enum FunctionKind {
    Script,
    Function,
}

pub struct Function {
    pub name: String,
    pub arity: u8,
    pub chunk: Chunk,
}

pub enum Object {
    Str(String),
    Func(Function),
}

impl Object {
    pub fn is_string(&self) -> bool {
        match self {
            Self::Str(_) => true,
            _ => false,
        }
    }

    pub fn is_function(&self) -> bool {
        match self {
            Self::Func(_) => true,
            _ => false,
        }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s) => f.write_str(s),
            Self::Func(func) => f.write_str(&format!("<fn {}>", func.name)),
        }
    }
}

pub type ObjRef = *mut Object;
