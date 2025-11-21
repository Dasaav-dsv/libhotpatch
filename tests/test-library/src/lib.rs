use std::sync::Once;

#[unsafe(no_mangle)]
extern "C" fn test_lib_version() -> u32 {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        env_logger::init();
    });
    unsafe { test_lib_version_hotpatch() }
}

#[libhotpatch::hotpatch]
unsafe fn test_lib_version_hotpatch() -> u32 {
    #[cfg(feature = "v1")]
    return 1;
    #[cfg(feature = "v2")]
    return 2;
    #[cfg(feature = "v3")]
    return 3;
}
