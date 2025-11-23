use std::{
    fs::{self, File},
    io,
    sync::{
        OnceLock,
        atomic::{AtomicU32, AtomicU64, Ordering as AtomicOrdering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};

use atomic_wait::{wait, wake_all};
use xxhash_rust::xxh3::xxh3_64;

use crate::{
    TARGET_DIR,
    abi::{
        str::Str,
        time::{AtomicDuration, AtomicInstant},
    },
    hotpatch::update_fn_table,
    lock::HotpatchLock,
    os::Module,
};

#[repr(C)]
pub struct Watcher {
    last_update: AtomicInstant,
    library_modified: AtomicDuration,
    library_hash: AtomicU64,
    library_name: Str<'static>,
    update_lock: AtomicU32,
}

impl Watcher {
    const POLL_MS: u64 = 100;

    pub fn get() -> Option<&'static Watcher> {
        *WATCHER.get_or_init(|| {
            Self::new()
                .inspect_err(|e| log::error!("error initializing Watcher: {e}"))
                .ok()
        })
    }

    pub fn poll(&'static self) {
        let last_update = self.last_update.load(AtomicOrdering::Relaxed);

        if last_update.elapsed() < Duration::from_millis(Self::POLL_MS) {
            return;
        }

        if self
            .update_lock
            .compare_exchange(0, 1, AtomicOrdering::Acquire, AtomicOrdering::Relaxed)
            .is_err()
        {
            wait(&self.update_lock, 0);
            return;
        }

        struct LockGuard<'a>(&'a AtomicU32);
        impl Drop for LockGuard<'_> {
            fn drop(&mut self) {
                self.0.store(0, AtomicOrdering::Relaxed);
                wake_all(self.0);
            }
        }

        let _lock_guard = LockGuard(&self.update_lock);

        let _ = self.update();

        self.last_update
            .store(Instant::now(), AtomicOrdering::Relaxed);
    }

    fn new() -> io::Result<&'static Watcher> {
        log::trace!("allocating a new Watcher");

        let current_library = Module::current().ok_or(io::ErrorKind::NotFound)?;
        log::debug!("current library path is {:?}", current_library.file_path());

        let loaded_library = current_library.loaded_library().map_err(io::Error::other)?;

        if unsafe {
            loaded_library
                .get::<extern "C" fn(&'static Watcher)>(b"__libhotpatch_init_watcher")
                .is_err()
        } {
            return Err(io::Error::other("not all exports are available"));
        }

        let time_modified = File::open(current_library.file_path())?
            .metadata()?
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap();

        if let Err(e) = fs::create_dir(format!("{TARGET_DIR}/.hotpatch"))
            && e.kind() != io::ErrorKind::AlreadyExists
        {
            return Err(e);
        }

        let bytes = fs::read(current_library.file_path())?;
        let hash = xxh3_64(&bytes);

        let library_name = current_library
            .file_name()
            .to_str()
            .ok_or(io::ErrorKind::InvalidFilename)?;

        let watcher = Box::new(Watcher {
            last_update: AtomicInstant::now(),
            update_lock: AtomicU32::new(0),
            library_modified: AtomicDuration::new(time_modified),
            library_name: Str::new(Box::leak(library_name.into())),
            library_hash: AtomicU64::new(hash),
        });

        Ok(Box::leak(watcher))
    }

    fn update(&'static self) -> io::Result<()> {
        let hotpatch_library_path = format!("{TARGET_DIR}/{}", self.library_name);

        let hotpatch_library = File::open(&hotpatch_library_path)?;

        let hotpatch_library_modified = hotpatch_library
            .metadata()?
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let library_last_modified = self.library_modified.load(AtomicOrdering::Relaxed);

        if hotpatch_library_modified.as_millis() == library_last_modified.as_millis() {
            return Ok(());
        }

        log::trace!("Watcher is updating...");

        let bytes = fs::read(&hotpatch_library_path)?;
        let hotpatch_library_hash = xxh3_64(&bytes);

        if hotpatch_library_hash == self.library_hash.load(AtomicOrdering::Relaxed) {
            log::trace!("file hash matched, no update required");

            self.library_modified
                .store(hotpatch_library_modified, AtomicOrdering::Relaxed);

            return Ok(());
        }

        self.hotpatch_library(&hotpatch_library_path)
            .inspect_err(|e| log::error!("error hot-patching library: {e}"))?;

        self.library_modified
            .store(hotpatch_library_modified, AtomicOrdering::Relaxed);

        self.library_hash
            .store(hotpatch_library_hash, AtomicOrdering::Relaxed);

        Ok(())
    }

    fn hotpatch_library(&'static self, hotpatch_library_path: &str) -> io::Result<()> {
        log::info!("hot-patching library {}", self.library_name);

        log::debug!("acquiring file lock");
        let _file_lock = HotpatchLock::new()?;

        let temp_dir = tempfile::tempdir_in(format!("{TARGET_DIR}/.hotpatch"))?;
        let temp_path = temp_dir.path().join(self.library_name.as_str());

        log::debug!("using temporary path {temp_path:?}");
        fs::copy(hotpatch_library_path, &temp_path)?;

        log::debug!("loading library {temp_path:?}");

        #[cfg(not(unix))]
        let lib = unsafe { libloading::Library::new(&temp_path).map_err(io::Error::other)? };

        #[cfg(unix)]
        let lib = unsafe {
            libloading::os::unix::Library::open(
                Some(&temp_path),
                libc::RTLD_LOCAL | libc::RTLD_LAZY | libc::RTLD_NODELETE,
            )
            .map(libloading::Library::from)
            .map_err(io::Error::other)?
        };

        let init_watcher = unsafe {
            lib.get::<extern "C" fn(&'static Watcher)>(b"__libhotpatch_init_watcher")
                .map_err(io::Error::other)?
        };

        log::debug!("calling __libhotpatch_init_watcher");
        init_watcher(self);

        log::debug!("patching function table");
        update_fn_table(lib, temp_dir)?;

        Ok(())
    }
}

static WATCHER: OnceLock<Option<&Watcher>> = OnceLock::new();

#[unsafe(no_mangle)]
extern "C" fn __libhotpatch_init_watcher(watcher: &'static Watcher) {
    let _ = WATCHER.get_or_init(|| Some(watcher));
}
