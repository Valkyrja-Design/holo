use super::chunk::Chunk;
use super::native::NativeFunc;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub arity: u8,
    pub upvalue_count: usize,
    pub chunk: Chunk,
}

#[derive(Debug)]
pub struct Upvalue {
    pub location: *mut Value,
    pub closed: Value,
}

impl Upvalue {
    pub fn new(location: *mut Value, closed: Value) -> Self {
        Self { location, closed }
    }
}

#[derive(Debug)]
pub struct Closure {
    pub function: *mut Function,
    pub upvalues: Vec<*mut Upvalue>,
}

impl Closure {
    pub fn new(function: *mut Function, upvalue_count: usize) -> Self {
        Self {
            function,
            upvalues: Vec::with_capacity(upvalue_count),
        }
    }

    pub fn function(&self) -> &Function {
        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            &*self.function
        }
    }

    pub fn chunk(&self) -> &Chunk {
        &self.function().chunk
    }

    pub fn arity(&self) -> u8 {
        self.function().arity
    }

    pub fn name(&self) -> &str {
        &self.function().name
    }
}

#[derive(Debug)]
pub struct Class {
    pub name: String,
}

impl Class {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug)]
pub struct ClassInstance {
    pub class: *mut Class,
    // FIXME: Might want to make it a hashmap over `NonNull<str>`
    pub fields: HashMap<String, Value>,
}

impl ClassInstance {
    pub fn new(class: *mut Class) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }

    pub fn get_field(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(*mut String),
    Function(*mut Function),
    Closure(*mut Closure),
    NativeFunc(*mut NativeFunc),
    Upvalue(*mut Upvalue),
    Class(*mut Class),
    ClassInstance(*mut ClassInstance),
}

impl Value {
    // SAFETY: GC guarantees that all pointers are valid
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(ptr) => unsafe { Some((**ptr).as_str()) },
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&Function> {
        match self {
            Self::Function(ptr) => unsafe { Some(&**ptr) },
            _ => None,
        }
    }

    pub fn as_function_mut(&self) -> Option<&mut Function> {
        match self {
            Self::Function(ptr) => unsafe { Some(&mut **ptr) },
            _ => None,
        }
    }

    pub fn as_closure(&self) -> Option<&Closure> {
        match self {
            Self::Closure(ptr) => unsafe { Some(&**ptr) },
            _ => None,
        }
    }

    pub fn as_native_func(&self) -> Option<&NativeFunc> {
        match self {
            Self::NativeFunc(ptr) => unsafe { Some(&**ptr) },
            _ => None,
        }
    }

    pub fn as_upvalue(&self) -> Option<&Upvalue> {
        match self {
            Self::Upvalue(ptr) => unsafe { Some(&**ptr) },
            _ => None,
        }
    }

    pub fn as_class(&self) -> Option<&Class> {
        match self {
            Self::Class(ptr) => unsafe { Some(&**ptr) },
            _ => None,
        }
    }

    pub fn as_class_instance(&self) -> Option<&ClassInstance> {
        match self {
            Self::ClassInstance(ptr) => unsafe { Some(&**ptr) },
            _ => None,
        }
    }

    pub fn as_class_instance_mut(&self) -> Option<&mut ClassInstance> {
        match self {
            Self::ClassInstance(ptr) => unsafe { Some(&mut **ptr) },
            _ => None,
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            match self {
                Self::Nil => f.write_str("nil"),
                Self::Bool(value) => f.write_str(&format!("{}", value)),
                Self::Number(value) => f.write_str(&format!("{}", value)),
                Self::String(ptr) => {
                    write!(f, "\"{}\"", (**ptr))
                }
                Self::Function(ptr) => {
                    write!(f, "<fn {}>", (**ptr).name)
                }
                Self::Closure(ptr) => {
                    write!(f, "<closure {}>", (**ptr).name())
                }
                Self::NativeFunc(ptr) => {
                    write!(f, "<native fn {}>", (**ptr).name)
                }
                Self::Upvalue(ptr) => {
                    write!(f, "<upvalue {:p}>", (**ptr).location)
                }
                Self::Class(ptr) => {
                    write!(f, "<class {}>", (**ptr).name)
                }
                Self::ClassInstance(ptr) => {
                    write!(f, "<instance of {}>", (*(**ptr).class).name)
                }
            }
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            // SAFETY: GC guarantees that all pointers are valid
            match self {
                Self::Nil => f.write_str("nil"),
                Self::Bool(value) => f.write_str(&format!("{}", value)),
                Self::Number(value) => f.write_str(&format!("{}", value)),
                Self::String(ptr) => {
                    write!(f, "{}", &**ptr)
                }
                Self::Function(ptr) => {
                    write!(f, "<fn {}>", (**ptr).name)
                }
                Self::Closure(ptr) => {
                    write!(f, "<fn {}>", (**ptr).name())
                }
                Self::NativeFunc(ptr) => {
                    write!(f, "<native fn {}>", (**ptr).name)
                }
                Self::Upvalue(ptr) => {
                    write!(f, "<upvalue {:p}>", (**ptr).location)
                }
                Self::Class(ptr) => {
                    write!(f, "<class {}>", (**ptr).name)
                }
                Self::ClassInstance(ptr) => {
                    write!(f, "<instance of {}>", (*(**ptr).class).name)
                }
            }
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Nil
    }
}
