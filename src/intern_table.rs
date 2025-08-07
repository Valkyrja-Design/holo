use super::{
    gc,
    object::{ObjRef, Object},
};
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
struct InternKey(NonNull<str>);

impl Hash for InternKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: the pointer must refer to a valid and live `str`.
        // That's the responsibility of the GC
        unsafe { self.0.as_ref().hash(state) }
    }
}

impl PartialEq for InternKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: the pointer must refer to a valid and live `str`.
        // That's the responsibility of the GC
        unsafe { self.0.as_ref() == other.0.as_ref() }
    }
}

impl Eq for InternKey {}

pub struct StringInternTable {
    table: HashMap<InternKey, ObjRef>,
}

impl StringInternTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn intern_slice(&mut self, value: &str, gc: &mut gc::GC) -> ObjRef {
        // only uses the `value` for comparison purposes
        let key = InternKey(NonNull::from(value));
        self.intern_inner(key, || gc.alloc(Object::Str(value.to_owned())))
    }

    pub fn intern_owned(&mut self, value: String, gc: &mut gc::GC) -> ObjRef {
        // only uses the `value` for comparison purposes
        let key = InternKey(NonNull::from(value.as_str()));
        self.intern_inner(key, || gc.alloc(Object::Str(value)))
    }

    fn intern_inner<F>(&mut self, key: InternKey, alloc: F) -> ObjRef
    where
        F: FnOnce() -> ObjRef,
    {
        if let Some(&handle) = self.table.get(&key) {
            return handle;
        }

        let handle = alloc();
        self.insert_handle(handle)
    }

    fn insert_handle(&mut self, handle: ObjRef) -> ObjRef {
        unsafe {
            match &*handle {
                Object::Str(s) => {
                    let key = InternKey(NonNull::from(s.as_str()));
                    self.table.insert(key, handle);

                    handle
                }
                _ => unreachable!(),
            }
        }
    }
}

impl Debug for StringInternTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_set();

        for &handle in self.table.values() {
            // SAFETY: This is only called while the VM is running and
            // the GC makes sure `ObjRef`s in the table are alive and valid
            unsafe {
                if let Object::Str(ref s) = *handle {
                    dbg.entry(s);
                }
            }
        }

        dbg.finish()
    }
}
