use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use windows_sys::{
    Win32::{
        Foundation::{HANDLE, MAX_PATH},
        System::LibraryLoader::{
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            GetModuleFileNameW, GetModuleHandleExW,
        },
    },
    core::PCWSTR,
};

#[inline(never)]
pub fn current_module_path() -> Option<PathBuf> {
    let mut module_handle = HANDLE::default();

    // SAFETY: GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS flag is used, return value is checked.
    let res = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            current_module_path as PCWSTR,
            &mut module_handle,
        )
    };

    if res == 0 {
        return None;
    }

    let mut path = vec![0u16; MAX_PATH as usize];

    loop {
        // SAFETY: the `path` buffer uses the correct length, return value is checked.
        let res =
            unsafe { GetModuleFileNameW(module_handle, path.as_mut_ptr(), path.len() as u32) };

        if res == 0 {
            return None;
        } else if res != path.len() as u32 {
            let os_str = OsString::from_wide(&path[..res as usize]);
            return Some(os_str.into());
        }

        path.resize(path.len() * 2, 0);
    }
}
