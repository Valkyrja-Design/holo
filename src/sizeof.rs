use std::collections::HashMap;

use crate::chunk::Chunk;
use crate::native::NativeFunc;
use crate::value::{BoundMethod, Class, ClassInstance, Closure, Function, Upvalue, Value};

/// Estimates the memory footprint of a type in bytes.
///
/// This trait is primarily used by the garbage collector to track memory usage
/// and determine when to trigger collection cycles. The size includes both
/// the stack-allocated portion of the type and any heap-allocated data it owns.
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
        // FIXME: This severely underestimates the memory size
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
            + std::mem::size_of::<usize>()
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

impl Sizeof for Class {
    fn sizeof(&self) -> usize {
        self.name.sizeof()
    }
}

impl Sizeof for Value {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<Value>()
    }
}

impl<K, V> Sizeof for HashMap<K, V>
where
    K: Sizeof,
    V: Sizeof,
{
    fn sizeof(&self) -> usize {
        // FIXME: This severely underestimates the memory size
        std::mem::size_of::<HashMap<K, V>>()
            + self.capacity() * (std::mem::size_of::<K>() + std::mem::size_of::<V>())
    }
}

impl Sizeof for ClassInstance {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<*mut Class>() + self.fields.sizeof()
    }
}

impl Sizeof for BoundMethod {
    fn sizeof(&self) -> usize {
        std::mem::size_of::<*mut ClassInstance>() + std::mem::size_of::<*mut Closure>()
    }
}
