use core::slice;
use std::{
    alloc::{Layout, handle_alloc_error},
    ffi::c_void,
    mem,
    ops::Deref,
    ptr::{self, NonNull},
};

use crate::os::{aligned_alloc, free};

#[repr(transparent)]
pub struct Box<T>(NonNull<T>);

#[repr(C)]
pub struct BoxedSlice<T> {
    ptr: NonNull<T>,
    len: usize,
}

impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        let ptr = aligned_alloc(layout.size(), layout.align()) as *mut T;

        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };

        // SAFETY: moving the value in into a brand new allocation that is properly sized.
        unsafe {
            ptr.write(value);
        }

        Self(ptr)
    }

    #[inline]
    pub fn into_raw(self) -> *mut T {
        let ptr = self.0.as_ptr();
        mem::forget(self);
        ptr
    }

    /// # Safety:
    ///
    /// `ptr` must be derived from a previous call to [`Box::into_raw`] and this method must not
    /// be called more than once on such pointer.
    #[inline]
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        unsafe { Self(NonNull::new_unchecked(ptr)) }
    }
}

impl<T: Copy> BoxedSlice<T> {
    pub fn new(value: &[T]) -> Self {
        let len = value.len();

        let layout = Layout::for_value(value);
        let ptr = aligned_alloc(layout.size(), layout.align()) as *mut T;

        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };

        // SAFETY: copying the values into a brand new allocation that is properly sized.
        unsafe {
            ptr::copy_nonoverlapping(value.as_ptr(), ptr.as_ptr(), len);
        }

        Self { ptr, len }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T> Deref for BoxedSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // SAFETY: slice is initialized and valid for up to `self.len`.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        // SAFETY: this pointer could have only been allocated with the corresponding `malloc`.
        unsafe {
            ptr::drop_in_place(self.0.as_ptr());
            free(self.0.as_ptr() as *mut c_void);
        }
    }
}

impl<T> Drop for BoxedSlice<T> {
    fn drop(&mut self) {
        // SAFETY: this pointer could have only been allocated with the corresponding `malloc`.
        unsafe {
            ptr::drop_in_place(NonNull::slice_from_raw_parts(self.ptr, self.len).as_ptr());
            free(self.ptr.as_ptr() as *mut c_void);
        }
    }
}

unsafe impl<T: Send> Send for Box<T> {}

unsafe impl<T: Sync> Sync for Box<T> {}

unsafe impl<T: Send> Send for BoxedSlice<T> {}

unsafe impl<T: Sync> Sync for BoxedSlice<T> {}
