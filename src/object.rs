#[derive(Debug)]
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

pub type ObjRef = *mut Object;
