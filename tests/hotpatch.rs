use std::{env, fs, path::Path, process::Command};

use libloading::{Library, library_filename};

#[test]
fn patch_test_lib() {
    build_test_lib("v1");

    let new_lib_dir = Path::new("target/debug/.tmp");
    fs::create_dir_all(new_lib_dir).unwrap();

    let old_lib_path = Path::new("target/debug").join(library_filename("test_library"));

    let new_lib_path = new_lib_dir.join(library_filename("test_library"));

    fs::rename(&old_lib_path, &new_lib_path).unwrap();

    let test_lib = unsafe { Library::new(new_lib_path).unwrap() };

    let test_lib_version = unsafe {
        test_lib
            .get::<extern "C" fn() -> u32>(b"test_lib_version")
            .unwrap()
    };

    assert_eq!(test_lib_version(), 1);

    build_test_lib("v2");

    assert_eq!(test_lib_version(), 2);

    build_test_lib("v3");

    assert_eq!(test_lib_version(), 3);

    #[cfg(unix)]
    std::mem::forget(test_lib);
}

fn build_test_lib(version: &str) {
    let cargo_build_test_lib = Command::new(env!("CARGO"))
        .current_dir("tests/test-library")
        .args(["build", "-F", version, "--no-default-features"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .success();

    assert!(cargo_build_test_lib, "failed to build test library");
}
