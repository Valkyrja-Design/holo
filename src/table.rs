use super::{gc::GC, value::Value};
use std::{collections::HashMap, fmt::Debug};
use std::{
    hash::{Hash, Hasher},
    ptr::NonNull,
};

// SAFETY: the pointer must refer to a valid and live `str`.
// That's the responsibility of the GC. It should also be
// immutable, GC should remove it if the corresponding mem
// is freed
#[derive(Clone, Copy)]
struct StrKey(NonNull<str>);

impl Hash for StrKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: the pointer must refer to a valid and live `str`.
        // That's the responsibility of the GC
        unsafe { self.0.as_ref().hash(state) }
    }
}

impl PartialEq for StrKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: the pointer must refer to a valid and live `str`.
        // That's the responsibility of the GC
        unsafe { self.0.as_ref() == other.0.as_ref() }
    }
}

impl Eq for StrKey {}

pub struct StringInternTable(HashMap<StrKey, *mut String>);

impl StringInternTable {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn intern_slice(&mut self, value: &str, gc: &mut GC) -> *mut String {
        // only uses the `value` for comparison purposes
        let key = StrKey(NonNull::from(value));
        self.intern_inner(key, || gc.alloc_string_ptr(value.to_string()))
    }

    pub fn intern_owned(&mut self, value: String, gc: &mut GC) -> *mut String {
        // only uses the `value` for comparison purposes
        let key = StrKey(NonNull::from(value.as_str()));
        self.intern_inner(key, || gc.alloc_string_ptr(value))
    }

    fn intern_inner<F>(&mut self, key: StrKey, alloc: F) -> *mut String
    where
        F: FnOnce() -> *mut String,
    {
        if let Some(&handle) = self.0.get(&key) {
            return handle;
        }

        let handle = alloc();
        self.insert_handle(handle)
    }

    fn insert_handle(&mut self, handle: *mut String) -> *mut String {
        // SAFETY: the GC makes sure that the handle is valid
        unsafe {
            let key = StrKey(NonNull::from((*handle).as_str()));
            self.0.insert(key, handle);
            handle
        }
    }
}

impl Debug for StringInternTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_set();

        for &handle in self.0.values() {
            // SAFETY: This is only called while the VM is running and
            // the GC makes sure that all pointers in the table are alive and valid
            unsafe {
                dbg.entry(&(*handle).as_str());
            }
        }

        dbg.finish()
    }
}

pub struct StringTable(HashMap<StrKey, Value>);

impl StringTable {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, key: NonNull<str>, value: Value) -> Option<Value> {
        self.0.insert(StrKey(key), value)
    }

    pub fn get(&self, key: NonNull<str>) -> Option<&Value> {
        self.0.get(&StrKey(key))
    }
}

impl Debug for StringTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_set();

        for value in self.0.values() {
            dbg.entry(value);
        }

        dbg.finish()
    }
}
