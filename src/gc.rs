//! Garbage collector implementation for the virtual machine.
//!
//! This module implements a simple mark-and-sweep garbage collector that manages
//! memory for all heap-allocated objects in the runtime. The collector uses
//! a mark-and-sweep algorithm with automatic threshold adjustment.

use crate::native::NativeFunc;
use crate::sizeof::Sizeof;
use crate::value::BoundMethod;
use crate::value::{Class, ClassInstance, Closure, Function, Upvalue, Value};
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
    classes: Vec<*mut Class>,
    class_instances: Vec<*mut ClassInstance>,
    bound_methods: Vec<*mut BoundMethod>,

    // "black" GC pointers that have had their references traced
    marked_strings: HashSet<*mut String>,
    marked_functions: HashSet<*mut Function>,
    marked_closures: HashSet<*mut Closure>,
    marked_natives: HashSet<*mut NativeFunc>,
    marked_upvalues: HashSet<*mut Upvalue>,
    marked_classes: HashSet<*mut Class>,
    marked_class_instances: HashSet<*mut ClassInstance>,
    marked_bound_methods: HashSet<*mut BoundMethod>,

    // Currently "gray" GC pointers that have not had their references traced
    worklist_functions: Vec<*mut Function>,
    worklist_closures: Vec<*mut Closure>,
    worklist_upvalues: Vec<*mut Upvalue>,
    worklist_classes: Vec<*mut Class>,
    worklist_class_instances: Vec<*mut ClassInstance>,
    worklist_bound_methods: Vec<*mut BoundMethod>,
}

macro_rules! impl_alloc_methods {
    ($(($method:ident, $ptr_method:ident, $field:ident, $type:ty, $value_variant:ident)),*) => {
        $(
            /// Allocates an object and returns a Value wrapping it.
            pub fn $method(&mut self, obj: $type) -> Value {
                let ptr = self.$ptr_method(obj);
                Value::$value_variant(ptr)
            }

            /// Allocates an object and returns a raw pointer to it.
            pub fn $ptr_method(&mut self, obj: $type) -> *mut $type {
                self.bytes_allocated += obj.sizeof();

                let ptr = Box::into_raw(Box::new(obj));
                self.$field.push(ptr);
                ptr
            }
        )*
    };
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
            classes: Vec::new(),
            class_instances: Vec::new(),
            bound_methods: Vec::new(),
            marked_strings: HashSet::new(),
            marked_functions: HashSet::new(),
            marked_closures: HashSet::new(),
            marked_natives: HashSet::new(),
            marked_upvalues: HashSet::new(),
            marked_classes: HashSet::new(),
            marked_class_instances: HashSet::new(),
            marked_bound_methods: HashSet::new(),
            worklist_functions: Vec::new(),
            worklist_closures: Vec::new(),
            worklist_upvalues: Vec::new(),
            worklist_classes: Vec::new(),
            worklist_class_instances: Vec::new(),
            worklist_bound_methods: Vec::new(),
        }
    }

    impl_alloc_methods!(
        (alloc_string, alloc_string_ptr, strings, String, String),
        (
            alloc_function,
            alloc_function_ptr,
            functions,
            Function,
            Function
        ),
        (alloc_closure, alloc_closure_ptr, closures, Closure, Closure),
        (
            alloc_native,
            alloc_native_ptr,
            natives,
            NativeFunc,
            NativeFunc
        ),
        (alloc_upvalue, alloc_upvalue_ptr, upvalues, Upvalue, Upvalue),
        (alloc_class, alloc_class_ptr, classes, Class, Class),
        (
            alloc_class_instance,
            alloc_class_instance_ptr,
            class_instances,
            ClassInstance,
            ClassInstance
        ),
        (
            alloc_bound_method,
            alloc_bound_method_ptr,
            bound_methods,
            BoundMethod,
            BoundMethod
        )
    );

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
            Value::Class(ptr) => {
                if self.marked_classes.contains(&ptr) {
                    return;
                }
                self.mark_class(ptr)
            }
            Value::ClassInstance(ptr) => {
                if self.marked_class_instances.contains(&ptr) {
                    return;
                }
                self.mark_class_instance(ptr)
            }
            Value::BoundMethod(ptr) => {
                if self.marked_bound_methods.contains(&ptr) {
                    return;
                }
                self.mark_bound_method(ptr)
            }
            Value::Nil | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    /// Marks a string pointer as reachable
    pub fn mark_string(&mut self, ptr: *mut String) {
        self.marked_strings.insert(ptr);
    }

    /// Marks a function pointer as reachable
    fn mark_function(&mut self, ptr: *mut Function) {
        self.marked_functions.insert(ptr);
        self.worklist_functions.push(ptr);
    }

    /// Marks a closure pointer as reachable
    pub fn mark_closure(&mut self, ptr: *mut Closure) {
        self.marked_closures.insert(ptr);
        self.worklist_closures.push(ptr);
    }

    /// Marks a native function pointer as reachable
    fn mark_native(&mut self, ptr: *mut NativeFunc) {
        self.marked_natives.insert(ptr);
    }

    /// Marks an upvalue pointer as reachable
    pub fn mark_upvalue(&mut self, ptr: *mut Upvalue) {
        self.marked_upvalues.insert(ptr);
        self.worklist_upvalues.push(ptr);
    }

    /// Marks a class pointer as reachable
    pub fn mark_class(&mut self, ptr: *mut Class) {
        self.marked_classes.insert(ptr);
        self.worklist_classes.push(ptr);
    }

    /// Marks a class instance pointer as reachable
    pub fn mark_class_instance(&mut self, ptr: *mut ClassInstance) {
        self.marked_class_instances.insert(ptr);
        self.worklist_class_instances.push(ptr);
    }

    /// Marks a bound method pointer as reachable
    pub fn mark_bound_method(&mut self, ptr: *mut BoundMethod) {
        self.marked_bound_methods.insert(ptr);
        self.worklist_bound_methods.push(ptr);
    }

    /// Traces all values that are reachable from the roots
    pub fn trace_references(&mut self) {
        // FIXME: Not very efficient, but works for now
        while !self.worklist_closures.is_empty()
            || !self.worklist_functions.is_empty()
            || !self.worklist_upvalues.is_empty()
            || !self.worklist_classes.is_empty()
            || !self.worklist_class_instances.is_empty()
            || !self.worklist_bound_methods.is_empty()
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

            while let Some(ptr) = self.worklist_classes.pop() {
                // Mark the methods
                unsafe {
                    for (_k, v) in &(*ptr).methods {
                        if !self.marked_closures.contains(v) {
                            self.mark_closure(*v);
                        }
                    }
                }
            }

            while let Some(ptr) = self.worklist_class_instances.pop() {
                // Mark the parent class and all fields
                unsafe {
                    if !self.marked_classes.contains(&(*ptr).class) {
                        self.mark_class((*ptr).class);
                    }

                    for (_k, v) in &(*ptr).fields {
                        self.mark_value(*v);
                    }
                }
            }

            while let Some(ptr) = self.worklist_bound_methods.pop() {
                // Mark the receiver and the method
                unsafe {
                    if !self.marked_class_instances.contains(&(*ptr).receiver) {
                        self.mark_class_instance((*ptr).receiver);
                    }

                    if !self.marked_closures.contains(&(*ptr).method) {
                        self.mark_closure((*ptr).method);
                    }
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
        self.marked_classes.clear();
        self.marked_class_instances.clear();
        self.marked_bound_methods.clear();
    }

    /// Frees all unmarked pointers
    pub fn sweep(&mut self) {
        macro_rules! sweep_objects {
            ($(($field:ident, $marked_set:ident)),*) => {
                $(
                    self.$field.retain(|&ptr| {
                        if self.$marked_set.contains(&ptr) {
                            true
                        } else {
                            unsafe {
                                self.bytes_allocated -= (&*ptr).sizeof();
                                let _ = Box::from_raw(ptr);
                            }
                            false
                        }
                    });
                )*
            };
        }

        sweep_objects!(
            (strings, marked_strings),
            (functions, marked_functions),
            (closures, marked_closures),
            (natives, marked_natives),
            (upvalues, marked_upvalues),
            (classes, marked_classes),
            (class_instances, marked_class_instances),
            (bound_methods, marked_bound_methods)
        );

        // Set the next GC threshold
        self.next_gc = (self.bytes_allocated as f64 * GC_THRESHOLD_GROWTH_FACTOR) as usize;
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
        macro_rules! free_all {
            ($($field:ident),*) => {
                $(
                    for &ptr in &self.$field {
                        unsafe {
                            let _ = Box::from_raw(ptr);
                        }
                    }
                )*
            };
        }

        free_all!(
            bound_methods,
            class_instances,
            classes,
            upvalues,
            natives,
            closures,
            functions,
            strings
        );
    }
}
