//! The raw-pointer boundary: turning C pointers into Rust slices.
//!
//! C hands us `(pointer, length)` pairs and NUL-terminated strings; Rust code wants
//! slices, which carry their length and are bounds-checked on every index.  These four
//! functions are the *only* places the ported `lib/` code converts a C pointer into a
//! slice.  Everything after that conversion is safe Rust: an out-of-range index panics
//! (which becomes a Pintos kernel panic) instead of silently corrupting memory.
//!
//! Each function checks what it can (null pointers) and documents what it cannot (that
//! the memory really is valid for the length), so the proof obligation sits at the C
//! entry point that calls it, in one `// SAFETY:` comment.
use core::ffi::{c_char, CStr};
use core::slice;

/// Views `len` elements starting at `ptr` as a slice.
///
/// A zero-length view never touches `ptr`, so C callers may pass NULL together with 0.
///
/// # Safety
/// If `len > 0`, `ptr` must point to `len` initialized elements that stay valid and are
/// not mutated for the lifetime `'a`.
pub unsafe fn slice_from_raw<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if len == 0 {
        return &[];
    }
    assert!(!ptr.is_null(), "C passed a null pointer with a non-zero length");
    // SAFETY: non-null, and the caller vouches for `len` valid elements.
    unsafe { slice::from_raw_parts(ptr, len) }
}

/// Mutable version of [`slice_from_raw`].
///
/// # Safety
/// If `len > 0`, `ptr` must point to `len` initialized elements that are valid and not
/// accessed through any other pointer for the lifetime `'a`.
pub unsafe fn slice_from_raw_mut<'a, T>(ptr: *mut T, len: usize) -> &'a mut [T] {
    if len == 0 {
        return &mut [];
    }
    assert!(!ptr.is_null(), "C passed a null pointer with a non-zero length");
    // SAFETY: non-null, and the caller vouches for exclusive access to `len` elements.
    unsafe { slice::from_raw_parts_mut(ptr, len) }
}

/// The bytes of the NUL-terminated string at `ptr`, without the terminator.
///
/// # Safety
/// `ptr` must point to a NUL-terminated string that stays valid for the lifetime `'a`.
pub unsafe fn c_str_bytes<'a>(ptr: *const c_char) -> &'a [u8] {
    assert!(!ptr.is_null(), "C passed a null string");
    // SAFETY: non-null, and the caller vouches that a terminator exists.
    unsafe { CStr::from_ptr(ptr) }.to_bytes()
}

/// Like [`c_str_bytes`] but never reads more than `max_len` bytes, so it also works on
/// strings that are not terminated (the `%.5s` printf case).
///
/// # Safety
/// `ptr` must be readable up to the first NUL or `max_len` bytes, whichever comes first.
pub unsafe fn c_str_bytes_bounded<'a>(ptr: *const u8, max_len: usize) -> &'a [u8] {
    assert!(!ptr.is_null(), "C passed a null string");
    let mut len = 0;
    // SAFETY: each read is below `max_len` and stops at the first NUL, per the contract.
    while len < max_len && unsafe { ptr.add(len).read() } != 0 {
        len += 1;
    }
    // SAFETY: the `len` bytes just read are valid.
    unsafe { slice::from_raw_parts(ptr, len) }
}
