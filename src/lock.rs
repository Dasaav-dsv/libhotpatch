use std::{
    convert::identity,
    fs::{self, File},
    io,
};

use crate::TARGET_DIR;

pub struct HotpatchLock(File);

impl HotpatchLock {
    pub fn new() -> io::Result<Self> {
        let f = File::create(hotpatch_lock_path())?;
        f.lock()?;

        Ok(HotpatchLock(f))
    }

    pub fn is_locked() -> bool {
        fs::exists(hotpatch_lock_path()).is_ok_and(identity)
    }
}

impl Drop for HotpatchLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(hotpatch_lock_path());
        let _ = self.0.unlock();
    }
}

fn hotpatch_lock_path() -> String {
    let pid = std::process::id();
    format!("{TARGET_DIR}/.hotpatch/{pid}.lock")
}
