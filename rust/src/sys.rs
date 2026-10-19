//! FFI boundary: the C kernel functions Rust code calls.
//!
//! Everything declared here is `unsafe` to call; safe wrappers live in the modules that
//! use them.  Signatures mirror the C headers named in the comments.
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    // lib/debug.h
    pub fn debug_panic(
        file: *const c_char,
        line: c_int,
        function: *const c_char,
        message: *const c_char,
        ...
    ) -> !;
    // lib/stdio.h
    pub fn putbuf(buffer: *const c_char, n: usize);
    pub fn vprintf(format: *const c_char, args: core::ffi::VaList<'_>) -> c_int;
    // threads/malloc.h
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
}

// lib/stdio.h
extern "C" {
    pub fn hex_dump(ofs: usize, buf: *const c_void, size: usize, ascii: bool);
}

// filesys/file.h (only in kernels built with FILESYS)
#[cfg(feature = "filesys")]
pub type Off = i32;
#[cfg(feature = "filesys")]
#[repr(C)]
pub struct File {
    _opaque: [u8; 0],
}
#[cfg(feature = "filesys")]
extern "C" {
    pub fn file_read_at(file: *mut File, buf: *mut c_void, size: Off, start: Off) -> Off;
    pub fn file_write_at(file: *mut File, buf: *const c_void, size: Off, start: Off) -> Off;
}
