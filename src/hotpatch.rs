#[cfg(unix)]
use std::ffi::c_void;
use std::{
    cmp::Ordering,
    fs, io, mem, ptr,
    sync::{
        LazyLock,
        atomic::{AtomicPtr, AtomicU64, Ordering as AtomicOrdering, fence},
    },
};

use libloading::Library;
use tempfile::TempDir;

use crate::abi::{
    boxed::{Box as AbiBox, BoxedSlice},
    str::{BoxedStr, Str},
};

#[linkme::distributed_slice]
pub static HOTPATCH_FN: [(AtomicPtr<()>, LibraryHandle, fn() -> (u128, &'static str))] = [..];

#[repr(C)]
pub struct LibraryHandle {
    ptr: AtomicPtr<LibraryPayload>,
}

#[repr(C)]
struct LibraryPayload {
    refcount: AtomicU64,

    #[cfg(unix)]
    lib_handle: *mut c_void,
    #[cfg(windows)]
    lib_handle: isize,

    temp_path: BoxedStr,
}

impl LibraryHandle {
    pub const fn null() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn replace(&self, mut new: Self) -> Self {
        let new_ptr = mem::replace(&mut new.ptr, AtomicPtr::new(ptr::null_mut())).into_inner();
        let old_ptr = self.ptr.swap(new_ptr, AtomicOrdering::Relaxed);

        Self {
            ptr: AtomicPtr::new(old_ptr),
        }
    }
}

impl LibraryPayload {
    pub fn make_handle(lib: Library, dir: TempDir) -> LibraryHandle {
        let payload = AbiBox::new(Self {
            refcount: AtomicU64::new(1),

            #[cfg(unix)]
            lib_handle: libloading::os::unix::Library::from(lib).into_raw(),
            #[cfg(windows)]
            lib_handle: libloading::os::windows::Library::from(lib).into_raw(),

            temp_path: BoxedStr::new(dir.path().to_string_lossy()),
        });

        LibraryHandle {
            ptr: AtomicPtr::new(AbiBox::into_raw(payload)),
        }
    }
}

impl Clone for LibraryHandle {
    fn clone(&self) -> Self {
        let payload_ptr = self.ptr.load(AtomicOrdering::Relaxed);

        if !payload_ptr.is_null() {
            // SAFETY: pointer is not null and points to an initialized `LibraryPayload`,
            // since there was an open handle to it (self).
            unsafe {
                let _ = (*payload_ptr)
                    .refcount
                    .fetch_add(1, AtomicOrdering::Relaxed);
            }
        }

        Self {
            ptr: AtomicPtr::new(payload_ptr),
        }
    }
}

impl Drop for LibraryHandle {
    fn drop(&mut self) {
        let payload_ptr = self.ptr.load(AtomicOrdering::Relaxed);

        if payload_ptr.is_null() {
            return;
        }

        // SAFETY: pointer is not null and points to an initialized `LibraryPayload`,
        // since there was an open handle to it (self).
        if unsafe {
            (*payload_ptr)
                .refcount
                .fetch_sub(1, AtomicOrdering::Release)
                != 1
        } {
            return;
        }

        self.ptr.store(ptr::null_mut(), AtomicOrdering::Release);
        fence(AtomicOrdering::Acquire);

        log::debug!("dropping library payload");

        // SAFETY: pointer is obtained from `Box::into_raw`.
        let payload = unsafe { AbiBox::from_raw(payload_ptr) };
        drop(payload);
    }
}

impl Drop for LibraryPayload {
    fn drop(&mut self) {
        // SAFETY: handle is obtained from `Library::into_raw`.
        unsafe {
            #[cfg(unix)]
            let _ = libloading::os::unix::Library::from_raw(self.lib_handle);
            #[cfg(windows)]
            let _ = libloading::os::windows::Library::from_raw(self.lib_handle);
        }
        let _ = fs::remove_dir(&*self.temp_path);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HotpatchFn {
    fn_ptr: &'static AtomicPtr<()>,
    handle: &'static LibraryHandle,
    hash: u128,
    name: Str<'static>,
}

pub fn update_fn_table(hotpatch_library: Library, dir: TempDir) -> io::Result<()> {
    static CACHED_HOTPATCH_FN: LazyLock<BoxedSlice<HotpatchFn>> = LazyLock::new(build_fn_table);

    let fn_table = unsafe {
        hotpatch_library
            .get::<extern "C" fn() -> BoxedSlice<HotpatchFn>>(b"__libhotpatch_fn_table")
            .map(|getter| getter())
            .map_err(io::Error::other)
    };

    let handle = LibraryPayload::make_handle(hotpatch_library, dir);
    let fn_table = fn_table?;

    let mut my_fns = CACHED_HOTPATCH_FN.iter().fuse().peekable();
    let mut new_fns = fn_table.iter().fuse().peekable();

    while let Some(&my_fn) = my_fns.peek()
        && let Some(&new_fn) = new_fns.peek()
    {
        match my_fn.hash.cmp(&new_fn.hash) {
            Ordering::Less => {
                log::warn!("skipping {}, it may have been removed", my_fn.name);
                let _ = my_fns.next();
            }
            Ordering::Greater => {
                log::debug!("skipping {}, it may be new", new_fn.name);
                let _ = new_fns.next();
            }
            Ordering::Equal => {
                log::debug!("updating {}", my_fn.name);

                let _ = my_fns.next();
                let _ = new_fns.next();

                let new_ptr = new_fn.fn_ptr.load(AtomicOrdering::Relaxed);
                my_fn.fn_ptr.store(new_ptr, AtomicOrdering::Relaxed);

                let _ = my_fn.handle.replace(handle.clone());
            }
        }
    }

    for skipped in my_fns {
        log::warn!("skipping {}, it may have been removed", skipped.name);
    }

    for skipped in new_fns {
        log::debug!("skipping {}, it may be new", skipped.name);
    }

    Ok(())
}

fn build_fn_table() -> BoxedSlice<HotpatchFn> {
    let mut hotpatch_fns = HOTPATCH_FN
        .iter()
        .map(|(fn_ptr, handle, type_of)| {
            let (hash, name) = type_of();
            HotpatchFn {
                fn_ptr,
                handle,
                hash,
                name: Str::new(name),
            }
        })
        .collect::<Vec<_>>();

    hotpatch_fns.sort_by(|a, b| a.hash.cmp(&b.hash));

    BoxedSlice::new(&hotpatch_fns)
}

#[unsafe(no_mangle)]
extern "C" fn __libhotpatch_fn_table() -> BoxedSlice<HotpatchFn> {
    build_fn_table()
}
