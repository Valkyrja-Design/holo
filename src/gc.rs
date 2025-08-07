use super::object::{ObjRef, Object};
use std::mem::{self, ManuallyDrop};

#[derive(Debug)]
pub struct GC {
    objects: Vec<mem::ManuallyDrop<Box<Object>>>,
}

impl GC {
    pub fn new() -> Self {
        GC {
            objects: Vec::new(),
        }
    }

    pub fn alloc(&mut self, obj: Object) -> ObjRef {
        let mut boxed = Box::new(obj);
        let ptr = boxed.as_mut() as ObjRef;

        self.objects.push(ManuallyDrop::new(boxed));
        ptr
    }
}

impl Drop for GC {
    fn drop(&mut self) {
        for obj in &mut self.objects {
            unsafe {
                // SAFETY: for now all objects will be dropped at the end of the execution
                ManuallyDrop::drop(obj);
            }
        }
    }
}
