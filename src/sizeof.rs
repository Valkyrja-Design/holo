use crate::chunk::Chunk;
use crate::native::NativeFunc;
use crate::value::{Closure, Function, Upvalue, Value};

pub trait Sizeof {
    // Returns the estimated size of the object in bytes
    fn sizeof(&self) -> usize;
}

impl Sizeof for String {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<String>() + self.capacity()
    }
}

impl<T> Sizeof for Vec<T> {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<Vec<T>>() + self.capacity() * std::mem::size_of::<T>()
    }
}

impl Sizeof for Chunk {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<Chunk>()
            + self.code.sizeof()
            + self.constants.sizeof()
            + self.line_info.sizeof()
    }
}

impl Sizeof for Function {
    fn sizeof(&self) -> usize {
        self.name.sizeof()
            + std::mem::size_of::<u8>()
            + std::mem::size_of::<u32>()
            + self.chunk.sizeof()
    }
}

impl Sizeof for Closure {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<*mut Function>() + self.upvalues.sizeof()
    }
}

impl Sizeof for NativeFunc {
    fn sizeof(&self) -> usize {
        self.name.sizeof()
            + std::mem::size_of::<u8>()
            + std::mem::size_of::<fn(&[Value]) -> Result<Value, String>>()
    }
}
impl Sizeof for Upvalue {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<Upvalue>()
    }
}
