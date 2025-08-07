use super::native::NativeFunc;
use super::sizeof::Sizeof;
use super::value::{Closure, Function, Upvalue, Value};
use log::debug;
use std::collections::HashSet;

static GC_DEFAULT_THRESHOLD: usize = 1024 * 1024; // 1 MB
static GC_THRESHOLD_GROWTH_FACTOR: f64 = 2.0;

#[derive(Debug)]
pub struct GC {
    bytes_allocated: usize,
    next_gc: usize,

    strings: Vec<*mut String>,
    functions: Vec<*mut Function>,
    closures: Vec<*mut Closure>,
    natives: Vec<*mut NativeFunc>,
    upvalues: Vec<*mut Upvalue>,

    // "black" GC pointers that have had their references traced
    marked_strings: HashSet<*mut String>,
    marked_functions: HashSet<*mut Function>,
    marked_closures: HashSet<*mut Closure>,
    marked_natives: HashSet<*mut NativeFunc>,
    marked_upvalues: HashSet<*mut Upvalue>,

    // Currently "gray" GC pointers that have not had their references traced
    worklist_functions: Vec<*mut Function>,
    worklist_closures: Vec<*mut Closure>,
    worklist_upvalues: Vec<*mut Upvalue>,
}

impl GC {
    pub fn new() -> Self {
        GC {
            bytes_allocated: 0,
            next_gc: GC_DEFAULT_THRESHOLD,
            strings: Vec::new(),
            functions: Vec::new(),
            closures: Vec::new(),
            natives: Vec::new(),
            upvalues: Vec::new(),
            marked_strings: HashSet::new(),
            marked_functions: HashSet::new(),
            marked_closures: HashSet::new(),
            marked_natives: HashSet::new(),
            marked_upvalues: HashSet::new(),
            worklist_functions: Vec::new(),
            worklist_closures: Vec::new(),
            worklist_upvalues: Vec::new(),
        }
    }

    pub fn alloc_string(&mut self, s: String) -> Value {
        self.bytes_allocated += s.sizeof();

        let ptr = Box::into_raw(Box::new(s));
        debug!("Allocating string {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.strings.push(ptr);
        Value::String(ptr)
    }

    pub fn alloc_function(&mut self, f: Function) -> Value {
        self.bytes_allocated += f.sizeof();

        let ptr = Box::into_raw(Box::new(f));
        debug!("Allocating function {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.functions.push(ptr);
        Value::Function(ptr)
    }

    pub fn alloc_closure(&mut self, c: Closure) -> Value {
        self.bytes_allocated += c.sizeof();

        let ptr = Box::into_raw(Box::new(c));
        debug!("Allocating closure {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.closures.push(ptr);
        Value::Closure(ptr)
    }

    pub fn alloc_native(&mut self, n: NativeFunc) -> Value {
        self.bytes_allocated += n.sizeof();

        let ptr = Box::into_raw(Box::new(n));
        debug!("Allocating native {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.natives.push(ptr);
        Value::NativeFunc(ptr)
    }

    pub fn alloc_upvalue(&mut self, u: Upvalue) -> Value {
        self.bytes_allocated += u.sizeof();

        let ptr = Box::into_raw(Box::new(u));
        debug!("Allocating upvalue {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.upvalues.push(ptr);
        Value::Upvalue(ptr)
    }

    // Raw pointer allocation methods for cases needing direct pointers
    pub fn alloc_string_ptr(&mut self, s: String) -> *mut String {
        self.bytes_allocated += s.sizeof();

        let ptr = Box::into_raw(Box::new(s));
        debug!("Allocating string {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.strings.push(ptr);
        ptr
    }

    pub fn alloc_function_ptr(&mut self, f: Function) -> *mut Function {
        self.bytes_allocated += f.sizeof();

        let ptr = Box::into_raw(Box::new(f));
        debug!("Allocating function {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.functions.push(ptr);
        ptr
    }

    pub fn alloc_closure_ptr(&mut self, c: Closure) -> *mut Closure {
        self.bytes_allocated += c.sizeof();

        let ptr = Box::into_raw(Box::new(c));
        debug!("Allocating closure {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.closures.push(ptr);
        ptr
    }

    pub fn alloc_native_ptr(&mut self, n: NativeFunc) -> *mut NativeFunc {
        self.bytes_allocated += n.sizeof();

        let ptr = Box::into_raw(Box::new(n));
        debug!("Allocating native {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.natives.push(ptr);
        ptr
    }

    pub fn alloc_upvalue_ptr(&mut self, u: Upvalue) -> *mut Upvalue {
        self.bytes_allocated += u.sizeof();

        let ptr = Box::into_raw(Box::new(u));
        debug!("Allocating upvalue {:?} at {:?}", unsafe { &*ptr }, ptr);

        self.upvalues.push(ptr);
        ptr
    }

    /// Marks a value as reachable
    pub fn mark_value(&mut self, v: Value) {
        match v {
            Value::String(ptr) => self.mark_string(ptr),
            Value::Function(ptr) => {
                if self.marked_functions.contains(&ptr) {
                    return;
                }
                self.mark_function(ptr)
            }
            Value::Closure(ptr) => {
                if self.marked_closures.contains(&ptr) {
                    return;
                }
                self.mark_closure(ptr)
            }
            Value::NativeFunc(ptr) => self.mark_native(ptr),
            Value::Upvalue(ptr) => {
                if self.marked_upvalues.contains(&ptr) {
                    return;
                }
                self.mark_upvalue(ptr)
            }
            Value::Nil | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    /// Marks a string pointer as reachable
    pub fn mark_string(&mut self, ptr: *mut String) {
        debug!("Marking string {:?} at {:?}", unsafe { &*ptr }, ptr);
        self.marked_strings.insert(ptr);
    }

    /// Marks a function pointer as reachable
    fn mark_function(&mut self, ptr: *mut Function) {
        debug!("Marking function {:?} at {:?}", unsafe { &*ptr }, ptr);
        self.marked_functions.insert(ptr);
        self.worklist_functions.push(ptr);
    }

    /// Marks a closure pointer as reachable
    pub fn mark_closure(&mut self, ptr: *mut Closure) {
        debug!("Marking closure {:?} at {:?}", unsafe { &*ptr }, ptr);
        self.marked_closures.insert(ptr);
        self.worklist_closures.push(ptr);
    }

    /// Marks a native function pointer as reachable
    fn mark_native(&mut self, ptr: *mut NativeFunc) {
        debug!(
            "Marking native function {:?} at {:?}",
            unsafe { &*ptr },
            ptr
        );
        self.marked_natives.insert(ptr);
    }

    /// Marks an upvalue pointer as reachable
    pub fn mark_upvalue(&mut self, ptr: *mut Upvalue) {
        debug!("Marking upvalue {:?} at {:?}", unsafe { &*ptr }, ptr);
        self.marked_upvalues.insert(ptr);
        self.worklist_upvalues.push(ptr);
    }

    /// Traces all values that are reachable from the roots
    pub fn trace_references(&mut self) {
        // FIXME: Not very efficient, but works for now
        while !self.worklist_closures.is_empty()
            || !self.worklist_functions.is_empty()
            || !self.worklist_upvalues.is_empty()
        {
            while let Some(ptr) = self.worklist_functions.pop() {
                // Mark the constants in the function's chunk
                unsafe {
                    let chunk = &(*ptr).chunk;

                    for constant in &chunk.constants {
                        self.mark_value(*constant);
                    }
                }
            }

            while let Some(ptr) = self.worklist_closures.pop() {
                // Mark the inner function and all upvalues
                unsafe {
                    if !self.marked_functions.contains(&(*ptr).function) {
                        self.mark_function((*ptr).function);
                    }

                    for &upvalue in &(*ptr).upvalues {
                        if !self.marked_upvalues.contains(&upvalue) {
                            self.mark_upvalue(upvalue);
                        }
                    }
                }
            }

            while let Some(ptr) = self.worklist_upvalues.pop() {
                unsafe {
                    // FIXME: Use the `closed` field instead?
                    self.mark_value(*((*ptr).location));
                }
            }
        }
    }

    /// Clears all marks
    pub fn clear_marks(&mut self) {
        self.marked_strings.clear();
        self.marked_functions.clear();
        self.marked_closures.clear();
        self.marked_natives.clear();
        self.marked_upvalues.clear();
    }

    /// Frees all unmarked pointers
    pub fn sweep(&mut self) {
        let prev_bytes_allocated = self.bytes_allocated;

        self.strings.retain(|&ptr| {
            if self.marked_strings.contains(&ptr) {
                true
            } else {
                debug!("Freeing string at {:?}", ptr);

                self.bytes_allocated -= unsafe { &*ptr }.sizeof();
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
                false
            }
        });

        self.functions.retain(|&ptr| {
            if self.marked_functions.contains(&ptr) {
                true
            } else {
                debug!("Freeing function at {:?}", ptr);

                self.bytes_allocated -= unsafe { &*ptr }.sizeof();
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
                false
            }
        });

        self.closures.retain(|&ptr| {
            if self.marked_closures.contains(&ptr) {
                true
            } else {
                debug!("Freeing closure at {:?}", ptr);

                self.bytes_allocated -= unsafe { &*ptr }.sizeof();
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
                false
            }
        });

        self.natives.retain(|&ptr| {
            if self.marked_natives.contains(&ptr) {
                true
            } else {
                debug!("Freeing native at {:?}", ptr);

                self.bytes_allocated -= unsafe { &*ptr }.sizeof();
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
                false
            }
        });

        self.upvalues.retain(|&ptr| {
            if self.marked_upvalues.contains(&ptr) {
                true
            } else {
                debug!("Freeing upvalue at {:?}", ptr);

                self.bytes_allocated -= unsafe { &*ptr }.sizeof();
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
                false
            }
        });

        // Set the next GC threshold
        self.next_gc = (self.bytes_allocated as f64 * GC_THRESHOLD_GROWTH_FACTOR) as usize;

        debug!(
            "GC freed {} bytes, {} remaining",
            prev_bytes_allocated - self.bytes_allocated,
            self.bytes_allocated
        );
        debug!("Next GC threshold: {}", self.next_gc);
    }

    /// Returns true if the given string is marked
    pub fn is_string_marked(&self, ptr: *mut String) -> bool {
        self.marked_strings.contains(&ptr)
    }

    /// Returns true if a garbage collection should be triggered
    pub fn should_collect(&self) -> bool {
        self.bytes_allocated > self.next_gc
    }
}

impl Drop for GC {
    fn drop(&mut self) {
        // Convert raw pointers back to Box to properly drop them. The GC
        // should be the only owner of these pointers, so this is safe
        for &ptr in &self.upvalues {
            debug!("Freeing upvalue at {:?}", ptr);
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        for &ptr in &self.natives {
            debug!("Freeing native at {:?}", ptr);
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        for &ptr in &self.closures {
            debug!("Freeing closure at {:?}", ptr);
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        for &ptr in &self.functions {
            debug!("Freeing function at {:?}", ptr);
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        for &ptr in &self.strings {
            debug!("Freeing string at {:?}", ptr);
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}
