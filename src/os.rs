use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use libloading::Library;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

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
