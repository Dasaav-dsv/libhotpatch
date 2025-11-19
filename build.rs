use std::{env, path::PathBuf};

fn main() {
    // Set by Cargo to the output directory of this build script.
    // Assume it is in the "target" directory of the top level crate.
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // "debug", "release", etc.
    let profile = env::var("PROFILE").unwrap();
    // E.g. "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc".
    let target = env::var("TARGET").unwrap();

    // Iterate the components of OUT_DIR in reverse, attempting to match "{target}/{profile}"
    // or "target/{profile}" (literal "target" directory).
    let mut out_dir_components = out_dir.components().rev().collect::<Vec<_>>();

    let target_dir_pos = out_dir_components
        .windows(2)
        .position(|w| {
            (w[1].as_os_str() == "target" || w[1].as_os_str() == &*target)
                && w[0].as_os_str() == &*profile
        })
        .expect("failed to match \"target\" directory pattern");

    // Reverse and build the real target directory path.
    let target_dir = out_dir_components
        .drain(target_dir_pos..)
        .rev()
        .collect::<PathBuf>();

    println!(
        "cargo:rustc-env=LIBHOTPATCH_TARGET_DIR={}",
        target_dir.display()
    );

    println!("cargo:rerun-if-changed-env=OUT_DIR");
    println!("cargo:rerun-if-changed-env=PROFILE");
    println!("cargo:rerun-if-changed-env=TARGET");
}
