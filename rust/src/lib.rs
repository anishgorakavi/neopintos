//! Rust components of the Pintos kernel.
//!
//! This crate is a `no_std` static library that is linked into `kernel.o` next to the
//! original C objects.  C stays in charge of booting, interrupts, context switching and
//! the device drivers; Rust takes over one subsystem at a time behind the *same* C
//! symbols and ABI, so every milestone boots and passes the same tests as stock Pintos.
//!
//! Layout:
//!  * [`sys`]  - the FFI boundary: declarations of the C kernel functions Rust calls.
//!  * [`rt`]   - runtime glue: panic handler, global allocator, `kprintln!`.
//!  * [`stdio`], [`stdlib`], [`random`] - the printf family, `atoi`/`qsort`/`bsearch` and the
//!    PRNG, replacing `lib/{stdio,stdlib,random}.c` in the kernel (user programs keep the C).
//!  * [`bitmap`], [`list`] - each replaces one C source file and lives next to it in
//!    `lib/kernel/`, wired in with `#[path]`.
//!  * `selftest` - in-kernel checks for the above.
#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

#[macro_use]
pub mod rt;
pub mod sys;
pub mod ffi;
pub mod console;

#[path = "../../lib/stdio.rs"]
pub mod stdio;
#[path = "../../lib/stdlib.rs"]
pub mod stdlib;
#[path = "../../lib/random.rs"]
pub mod random;
#[path = "../../lib/kernel/bitmap.rs"]
pub mod bitmap;
#[path = "../../lib/kernel/list.rs"]
pub mod list;
mod selftest;
mod selftest_printf;

#[path = "../../threads/mod.rs"]
pub mod threads;

/// Called once from `main()` after `malloc_init()`.  Silent by design: stock Pintos
/// boot output must not change.
#[no_mangle]
pub extern "C" fn rust_init() {}

/// Kernel command-line action `rust-selftest`: exercises the Rust components in the
/// running kernel and prints one `(rust-selftest) ...` line per check.
#[no_mangle]
pub extern "C" fn rust_selftest(_argv: *mut *mut core::ffi::c_char) {
    selftest::run();
}
