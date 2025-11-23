#![doc = include_str!("../README.md")]

mod abi;
mod hotpatch;
mod lock;
mod os;
mod watcher;

// Crate proc macro reexports:
#[doc(hidden)]
pub use hotpatch::HOTPATCH_FN;
#[doc(hidden)]
pub use hotpatch::LibraryHandle;
#[doc(hidden)]
pub use watcher::Watcher;
#[doc(hidden)]
pub use xxhash_rust::xxh3::Xxh3;

// Proc macro reexports for `linkme`:
#[doc(hidden)]
pub use linkme;
#[doc(hidden)]
pub use linkme::distributed_slice;

// Proc macro reexports for `rmp-serde`:
#[cfg(feature = "checked")]
#[doc(hidden)]
pub use rmp_serde;
#[cfg(feature = "checked")]
#[doc(hidden)]
pub use abi::boxed::BoxedSlice;

pub use libhotpatch_macros::hotpatch;

pub(crate) static TARGET_DIR: &str = env!("LIBHOTPATCH_TARGET_DIR");

pub fn is_hotpatched() -> bool {
    lock::HotpatchLock::is_locked()
}
