use super::native::NativeFunc;
use super::value::{Closure, Function, Upvalue, Value};

#[derive(Debug)]
pub struct GC {
    strings: Vec<*mut String>,
    functions: Vec<*mut Function>,
    closures: Vec<*mut Closure>,
    natives: Vec<*mut NativeFunc>,
    upvalues: Vec<*mut Upvalue>,
}

impl GC {
    pub fn new() -> Self {
        GC {
            strings: Vec::new(),
            functions: Vec::new(),
            closures: Vec::new(),
            natives: Vec::new(),
            upvalues: Vec::new(),
        }
    }

    pub fn alloc_string(&mut self, s: String) -> Value {
        let ptr = Box::into_raw(Box::new(s));
        self.strings.push(ptr);
        Value::String(ptr)
    }

    pub fn alloc_function(&mut self, f: Function) -> Value {
        let ptr = Box::into_raw(Box::new(f));
        self.functions.push(ptr);
        Value::Function(ptr)
    }

    pub fn alloc_closure(&mut self, c: Closure) -> Value {
        let ptr = Box::into_raw(Box::new(c));
        self.closures.push(ptr);
        Value::Closure(ptr)
    }

    pub fn alloc_native(&mut self, n: NativeFunc) -> Value {
        let ptr = Box::into_raw(Box::new(n));
        self.natives.push(ptr);
        Value::NativeFunc(ptr)
    }

    pub fn alloc_upvalue(&mut self, u: Upvalue) -> Value {
        let ptr = Box::into_raw(Box::new(u));
        self.upvalues.push(ptr);
        Value::Upvalue(ptr)
    }

    // Raw pointer allocation methods for cases needing direct pointers
    pub fn alloc_string_ptr(&mut self, s: String) -> *mut String {
        let ptr = Box::into_raw(Box::new(s));
        self.strings.push(ptr);
        ptr
    }

    pub fn alloc_function_ptr(&mut self, f: Function) -> *mut Function {
        let ptr = Box::into_raw(Box::new(f));
        self.functions.push(ptr);
        ptr
    }

    pub fn alloc_closure_ptr(&mut self, c: Closure) -> *mut Closure {
        let ptr = Box::into_raw(Box::new(c));
        self.closures.push(ptr);
        ptr
    }

    pub fn alloc_native_ptr(&mut self, n: NativeFunc) -> *mut NativeFunc {
        let ptr = Box::into_raw(Box::new(n));
        self.natives.push(ptr);
        ptr
    }

    pub fn alloc_upvalue_ptr(&mut self, u: Upvalue) -> *mut Upvalue {
        let ptr = Box::into_raw(Box::new(u));
        self.upvalues.push(ptr);
        ptr
    }
}

impl Drop for GC {
    fn drop(&mut self) {
        // Convert raw pointers back to Box to properly drop them. The GC
        // should be the only owner of these pointers, so this is safe.
        for &ptr in &self.upvalues {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
        for &ptr in &self.natives {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
        for &ptr in &self.closures {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
        for &ptr in &self.functions {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
        for &ptr in &self.strings {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}
