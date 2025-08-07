use super::chunk::Chunk;
use super::native::NativeFunc;
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
        let mut upvalues = Vec::with_capacity(upvalue_count);

        for _ in 0..upvalue_count {
            upvalues.push(std::ptr::null_mut());
        }

        Self { function, upvalues }
    }

    pub fn function(&self) -> &Function {
        unsafe {
            // SAFETY: Closure function pointers are allocated by GC and remain valid
            // for the lifetime of the GC which outlives all Closure references
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
}

impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(ptr) => unsafe {
                // SAFETY: String pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
                ptr.as_ref().map(|s| s.as_str())
            },
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&Function> {
        match self {
            Self::Function(ptr) => unsafe {
                // SAFETY: Function pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
                ptr.as_ref()
            },
            _ => None,
        }
    }

    pub fn as_function_mut(&self) -> Option<&mut Function> {
        match self {
            Self::Function(ptr) => unsafe {
                // SAFETY: Function pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
                ptr.as_mut()
            },
            _ => None,
        }
    }

    pub fn as_closure(&self) -> Option<&Closure> {
        match self {
            Self::Closure(ptr) => unsafe {
                // SAFETY: Closure pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
                ptr.as_ref()
            },
            _ => None,
        }
    }

    pub fn as_native_func(&self) -> Option<&NativeFunc> {
        match self {
            Self::NativeFunc(ptr) => unsafe {
                // SAFETY: NativeFunc pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
                ptr.as_ref()
            },
            _ => None,
        }
    }

    pub fn as_upvalue(&self) -> Option<&Upvalue> {
        match self {
            Self::Upvalue(ptr) => unsafe {
                // SAFETY: Upvalue pointers are allocated by GC and remain valid
                // for the lifetime of the GC which outlives all Value references
                ptr.as_ref()
            },
            _ => None,
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => f.write_str("nil"),
            Self::Bool(value) => f.write_str(&format!("{}", value)),
            Self::Number(value) => f.write_str(&format!("{}", value)),
            Self::String(ptr) => unsafe {
                // SAFETY: String pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(s) = ptr.as_ref() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "<invalid string>")
                }
            },
            Self::Function(ptr) => unsafe {
                // SAFETY: Function pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(func) = ptr.as_ref() {
                    write!(f, "<fn {}>", func.name)
                } else {
                    write!(f, "<invalid function>")
                }
            },
            Self::Closure(ptr) => unsafe {
                // SAFETY: Closure pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(closure) = ptr.as_ref() {
                    write!(f, "<closure {}>", closure.name())
                } else {
                    write!(f, "<invalid closure>")
                }
            },
            Self::NativeFunc(ptr) => unsafe {
                // SAFETY: NativeFunc pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(native) = ptr.as_ref() {
                    write!(f, "<native fn {}>", native.name)
                } else {
                    write!(f, "<invalid native fn>")
                }
            },
            Self::Upvalue(ptr) => unsafe {
                // SAFETY: Upvalue pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(upvalue) = ptr.as_ref() {
                    write!(f, "<upvalue {:p}>", upvalue.location)
                } else {
                    write!(f, "<invalid upvalue>")
                }
            },
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => f.write_str("nil"),
            Self::Bool(value) => f.write_str(&format!("{}", value)),
            Self::Number(value) => f.write_str(&format!("{}", value)),
            Self::String(ptr) => unsafe {
                // SAFETY: String pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(s) = ptr.as_ref() {
                    write!(f, "{}", s)
                } else {
                    write!(f, "<invalid string>")
                }
            },
            Self::Function(ptr) => unsafe {
                // SAFETY: Function pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(func) = ptr.as_ref() {
                    write!(f, "<fn {}>", func.name)
                } else {
                    write!(f, "<invalid function>")
                }
            },
            Self::Closure(ptr) => unsafe {
                // SAFETY: Closure pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(closure) = ptr.as_ref() {
                    write!(f, "<fn {}>", closure.name())
                } else {
                    write!(f, "<invalid closure>")
                }
            },
            Self::NativeFunc(ptr) => unsafe {
                // SAFETY: NativeFunc pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(native) = ptr.as_ref() {
                    write!(f, "<native fn {}>", native.name)
                } else {
                    write!(f, "<invalid native fn>")
                }
            },
            Self::Upvalue(ptr) => unsafe {
                // SAFETY: Upvalue pointers are allocated by GC and guaranteed to be valid
                // as long as the GC is alive, which outlives all Value instances
                if let Some(upvalue) = ptr.as_ref() {
                    write!(f, "<upvalue {:p}>", upvalue.location)
                } else {
                    write!(f, "<invalid upvalue>")
                }
            },
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Nil
    }
}
