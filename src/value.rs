use super::object;
use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Object(object::ObjRef),
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => f.write_str("nil"),
            Self::Bool(value) => f.write_str(&format!("{}", value)),
            Self::Number(value) => f.write_str(&format!("{}", value)),
            Self::Object(obj) => {
                unsafe {
                    // SAFETY: `ObjRef`s on which `Debug` is attempted to be
                    // called are always alive -- made sure by the GC
                    (**obj).fmt(f)
                }
            }
        }
    }
}
