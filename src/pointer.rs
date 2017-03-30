
extern crate std;

use std::os::raw::c_void;

#[derive(Copy, Clone, Debug)]
pub struct Pointer {
    pointer: usize,
}

impl Pointer {
    pub fn new() -> Pointer {
        Pointer { pointer: 0 }
    }

    pub fn set(&mut self, ptr: *mut c_void) -> Self {
        unsafe {
            self.pointer = std::mem::transmute::<*mut c_void, usize>(ptr);
        }

        *self
    }

    pub fn as_ptr(&self) -> *mut c_void {
        unsafe { std::mem::transmute::<usize, *mut c_void>(self.pointer) }
    }
}

unsafe impl std::marker::Send for Pointer {}