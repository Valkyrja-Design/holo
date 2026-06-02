use crate::gc::GC;
use std::{collections::HashMap, fmt::Debug};
use std::{
    hash::{Hash, Hasher},
    ptr::NonNull,
};

/// A wrapper around a raw pointer to a string for use as a hash map key.
///
/// # Safety Invariants
/// - The pointer must refer to a valid and live `str` throughout its lifetime
/// - The GC is responsible for ensuring the pointer remains valid
/// - The string data must be immutable once interned
/// - The user must remove this key when the corresponding memory is freed
///   (by calling `StringInternTable::clear_unmarked`)
#[derive(Clone, Copy)]
struct StrKey(NonNull<str>);

impl StrKey {
    unsafe fn new(ptr: NonNull<str>) -> Self {
        Self(ptr)
    }

    unsafe fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl Hash for StrKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.as_str().hash(state) }
    }
}

impl PartialEq for StrKey {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.as_str() == other.as_str() }
    }
}

impl Eq for StrKey {}

/// A string interning table that deduplicates strings in memory.
///
/// This table maintains a collection of unique strings, ensuring that
/// identical strings share the same memory location. The table works
/// in conjunction with the garbage collector to manage string lifetimes.
pub struct StringInternTable(HashMap<StrKey, *mut String>);

impl StringInternTable {
    /// Creates a new empty string interning table.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn intern_slice(&mut self, value: &str, gc: &mut GC) -> *mut String {
        // Only uses the `value` for comparison purposes
        let key = unsafe { StrKey::new(NonNull::from(value)) };
        self.intern_inner(key, || gc.alloc_string_ptr(value.to_string()))
    }

    pub fn intern_owned(&mut self, value: String, gc: &mut GC) -> *mut String {
        // Only uses the `value` for comparison purposes
        let key = unsafe { StrKey::new(NonNull::from(value.as_str())) };
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
        // SAFETY: the GC makes sure that the handle is valid.
        unsafe {
            let key = StrKey::new(NonNull::from((*handle).as_str()));
            self.0.insert(key, handle);
            handle
        }
    }

    /// Clears all unmarked interned strings
    pub fn clear_unmarked(&mut self, gc: &mut GC) {
        self.0.retain(|_, &mut handle| gc.is_string_marked(handle));
    }

    pub fn contains(&self, value: &str) -> bool {
        let key = unsafe { StrKey::new(NonNull::from(value)) };
        self.0.contains_key(&key)
    }
}

impl Debug for StringInternTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_set();

        for &handle in self.0.values() {
            // SAFETY: This is only called while the VM is running and the GC makes sure that all
            // pointers in the table are alive and valid
            unsafe {
                dbg.entry(&(*handle).as_str());
            }
        }

        dbg.finish()
    }
}

impl Default for StringInternTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let mut gc = GC::new();
        let mut table = StringInternTable::new();

        let s1 = table.intern_slice("hello", &mut gc);
        let s2 = table.intern_slice("hello", &mut gc);
        let s3 = table.intern_slice("world", &mut gc);

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);

        let s4 = table.intern_owned("hello".to_string(), &mut gc);
        let s5 = table.intern_owned("hello".to_string(), &mut gc);
        let s6 = table.intern_owned("world".to_string(), &mut gc);

        assert_eq!(s4, s5);
        assert_ne!(s4, s6);

        let s7 = table.intern_slice("", &mut gc);
        let s8 = table.intern_slice("", &mut gc);
        assert_eq!(s7, s8);

        assert_eq!(table.0.len(), 3);
        assert!(table.contains("hello"));
        assert!(table.contains("world"));
    }

    #[test]
    fn test_clear_unmarked() {
        let mut gc = GC::new();
        let mut table = StringInternTable::new();

        let s1 = table.intern_slice("keep", &mut gc);
        let _s2 = table.intern_slice("remove", &mut gc);

        // Mark only s1
        gc.mark_string(s1);

        // Clear unmarked strings
        table.clear_unmarked(&mut gc);

        // Only "keep" should remain
        assert!(table.contains("keep"));
        assert!(!table.contains("remove"));
    }
}
