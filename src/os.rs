use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use libloading::Library;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::{aligned_alloc, free};
#[cfg(windows)]
pub use windows::{aligned_alloc, free};

#[derive(Debug)]
pub struct Module {
    path: PathBuf,
    name: OsString,
}

impl Module {
    pub fn current() -> Option<Self> {
        #[cfg(unix)]
        let path = unix::current_module_path();
        #[cfg(windows)]
        let path = windows::current_module_path();

        let path = path?;
        let name = path.file_name()?.to_owned();

        Some(Self { path, name })
    }

    pub fn file_path(&self) -> &Path {
        &self.path
    }

    pub fn file_name(&self) -> &OsStr {
        &self.name
    }

    pub fn loaded_library(&self) -> Result<Library, libloading::Error> {
        unsafe { Library::new(self.file_path()) }
    }
}
