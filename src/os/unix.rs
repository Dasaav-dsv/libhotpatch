use std::{
    ffi::{CStr, OsStr, c_void},
    mem,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

use libc::{Dl_info, dladdr};

#[inline(never)]
pub fn current_module_path() -> Option<PathBuf> {
    // SAFETY: POD C type that is safe to zero initialize.
    let mut info = unsafe { mem::zeroed::<Dl_info>() };

    // SAFETY: return value is checked.
    let res = unsafe { dladdr(current_module_path as *const c_void, &mut info) };

    (res != 0).then(|| {
        // SAFETY: `dli_fname` may not be NULL when `dladdr` succeeds.
        let c_str = unsafe { CStr::from_ptr(info.dli_fname) };
        let os_str = OsStr::from_bytes(c_str.to_bytes()).to_owned();
        os_str.into()
    })
}

#[inline]
pub fn aligned_alloc(size: usize, align: usize) -> *mut c_void {
    unsafe { libc::aligned_alloc(align, size) }
}

#[inline]
pub unsafe fn free(ptr: *mut c_void) {
    unsafe {
        libc::free(ptr);
    }
}
