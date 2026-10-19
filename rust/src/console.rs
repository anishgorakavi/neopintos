//! Safe console output.
//!
//! The console itself (serial port + VGA screen, serialized by a lock) is the C code in
//! `lib/kernel/console.c`, `devices/serial.c` and `devices/vga.c`.  Rust talks to it
//! through exactly one function, [`write_bytes`], which takes a slice, so a caller can
//! never pass a length that does not match the buffer.  All Rust printing (`kprint!`,
//! `hex_dump`, panic messages) goes through here and therefore needs no `unsafe`.
use core::fmt;

use crate::sys;

/// Writes `bytes` to the console, holding the console lock so output does not interleave
/// with other threads'.
pub fn write_bytes(bytes: &[u8]) {
    // SAFETY: `putbuf` reads exactly `bytes.len()` bytes starting at `bytes.as_ptr()`, which
    // is the extent of the slice.
    unsafe { sys::putbuf(bytes.as_ptr().cast(), bytes.len()) };
}

/// `core::fmt` sink for the console, so `write!(Console, ...)` works.
pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        write_bytes(text.as_bytes());
        Ok(())
    }
}
