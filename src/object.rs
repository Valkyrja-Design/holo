use std::fmt::Debug;

pub enum Object {
    Str(String),
}

impl Object {
    pub fn is_string(&self) -> bool {
        match self {
            Self::Str(_) => true,
            _ => false,
        }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s) => f.write_str(s),
            _ => f.write_str(""),
        }
    }
}

pub type ObjRef = *mut Object;
