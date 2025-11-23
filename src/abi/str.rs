use core::slice;
use std::{
    alloc::{Layout, handle_alloc_error},
    ffi::c_void,
    fmt,
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

use crate::os::{aligned_alloc, free};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Str<'a> {
    ptr: NonNull<u8>,
    len: usize,
    marker: PhantomData<&'a ()>,
}

#[repr(transparent)]
pub struct BoxedStr(Str<'static>);

impl<'a> Str<'a> {
    pub const fn new(str: &'a str) -> Self {
        Self {
            ptr: NonNull::from_ref(str.as_bytes()).cast::<u8>(),
            len: str.len(),
            marker: PhantomData,
        }
    }

    pub const fn as_str(&self) -> &'a str {
        // SAFETY: `Str` can only be created from a valid `str`.
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.ptr.as_ptr(), self.len)) }
    }
}

impl BoxedStr {
    pub fn new<S: AsRef<str>>(str: S) -> Self {
        let str = str.as_ref();
        let len = str.len();

        let layout = Layout::for_value(str);
        let ptr = aligned_alloc(layout.size(), layout.align()) as *mut u8;

        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };

        // SAFETY: copying to a brand new allocation that is properly sized.
        unsafe {
            ptr.copy_from_nonoverlapping(NonNull::from_ref(str.as_bytes()).cast::<u8>(), len);
        }

        Self(Str {
            ptr,
            len,
            marker: PhantomData,
        })
    }
}

impl<'a> AsRef<str> for Str<'a> {
    fn as_ref(&self) -> &'a str {
        self.as_str()
    }
}

impl AsRef<str> for BoxedStr {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for BoxedStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl Drop for BoxedStr {
    fn drop(&mut self) {
        // SAFETY: this pointer could have only been allocated with the corresponding `malloc`.
        unsafe {
            free(self.0.ptr.as_ptr() as *mut c_void);
        }
    }
}

impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl<'a> From<Str<'a>> for &'a str {
    fn from(value: Str<'a>) -> Self {
        value.as_str()
    }
}

impl fmt::Display for Str<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

unsafe impl Send for Str<'_> {}

unsafe impl Sync for Str<'_> {}

unsafe impl Send for BoxedStr {}

unsafe impl Sync for BoxedStr {}
