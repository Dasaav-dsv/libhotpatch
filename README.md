# libhotpatch

Live code reloading and hot-patching for shared libraries (.dll and .so).

## Disclaimer

This library strives to be memory safe, but it can only *attempt to* safeguard you from mistakes. The convenience of easier iteration and development comes at a cost. Do not use in production code.

## Usage

1. Add it to the Cargo.toml of a **C shared library**:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
libhotpatch = { version = "0.1.6", git = "https://github.com/Dasaav-dsv/libhotpatch.git" }
```

2. Annotate functions with `#[hotpatch]` (*NOTE:* the functions *must* also be marked `unsafe`):

```rs
use libhotpatch::hotpatch;

// SAFETY: the layout of any types passed as arguments to this function MUST NOT CHANGE.
// Any `static` variables defined in or outside its scope *must not escape* the scope of a
// function marked `#[hotpatch]`.
#[hotpatch]
unsafe fn present_frame(dt: f32) {
    // The body of this function will be updated live when your crate is rebuilt.
}
```

3. Build your library and **move the artifacts outside the build folder**. Otherwise, rebuilding will be blocked when the shared library is loaded into a process.

4. Have a process load your library **located outside the build folder**. Whenever you **rebuild**, the `#[hotpatch]` functions are **updated**.

*NOTE:* use `libhotpatch::is_hotpatched` in blocking entry points (like `DllMain`) to exit early instead of repeating their logic when the library is reloaded.

## Safety and usage tips

Patched functions behave as if called with the arguments from the **original build** of the shared library. Therefore, you *must not change the arguments, their types or their layouts* in `#[hotpatch]` function signatures and at their callsites.

Consider the lifetime of any static items to be restricted to the scope of `#[hotpatch]` functions that access them, including any outgoing function calls. In general, statics are reset to their initial state. Persistent static state can be achieved by accessing a static outside of `#[hotpatch]` scope, and passing it down as an argument (with a `'static` lifetime).

A `#[hotpatch]` function must not be marked `const`, `extern "Rust"`, use `Self`, use non-lifetime generic or `impl Trait` parameters. It *must be* marked `unsafe`.

`libhotpatch` uses the `log` crate to emit trace, debug and error logs. You can use a logging implementation compatible with `log` to capture them.

## Conditional attribute configuration

Your crate can define a feature and use [`cfg_attr`](https://doc.rust-lang.org/reference/conditional-compilation.html) to conditionally enable `#[hotpatch]` on functions.

In Cargo.toml:

```toml
[features]
hotpatch = []
```

In a source file:

```rs
#[cfg_attr(feature = "hotpatch", libhotpatch::hotpatch)]
unsafe fn present_frame(dt: f32) {
    // The body of this function will be updated live when your crate is rebuilt.
}
```

## License
Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
