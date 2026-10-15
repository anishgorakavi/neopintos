//! Runtime glue: console output, panic handler and global allocator.
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::{c_char, c_void};
use core::fmt::{self, Write};

use crate::sys;

/// `core::fmt` sink that writes to the Pintos console (`putbuf`, which takes the console lock).
pub struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // SAFETY: putbuf reads exactly `s.len()` bytes from a valid slice.
        unsafe { sys::putbuf(s.as_ptr().cast(), s.len()) };
        Ok(())
    }
}

#[macro_export]
macro_rules! kprintln {
    ($($arg:tt)*) => {{
        use core::fmt::Write as _;
        let _ = writeln!($crate::rt::Console, $($arg)*);
    }};
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {{
        use core::fmt::Write as _;
        let _ = write!($crate::rt::Console, $($arg)*);
    }};
}
/// Fixed-size, always NUL-terminated buffer; excess output is truncated.
struct CBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> CBuf<N> {
    fn new() -> Self {
        Self { buf: [0; N], len: 0 }
    }
    fn as_ptr(&self) -> *const c_char {
        self.buf.as_ptr().cast()
    }
}

impl<const N: usize> Write for CBuf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let room = N - 1 - self.len;
        let n = s.len().min(room);
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

/// A Rust panic becomes an ordinary Pintos kernel panic (message, backtrace, power off).
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut file = CBuf::<96>::new();
    let mut msg = CBuf::<192>::new();
    let line = match info.location() {
        Some(loc) => {
            let _ = write!(file, "{}", loc.file());
            loc.line() as i32
        }
        None => 0,
    };
    let _ = write!(msg, "{}", info.message());
    // SAFETY: both buffers are NUL-terminated and outlive the call; "%s" consumes one char*.
    unsafe {
        sys::debug_panic(file.as_ptr(), line, c"rust".as_ptr(), c"%s".as_ptr(), msg.as_ptr())
    }
}

/// Global allocator backed by the Pintos kernel `malloc`/`free`, so `alloc` (Box, Vec,
/// ...) works.  Must not be used from interrupt context (malloc takes a lock).
pub struct KernelAlloc;

/// Alignment `malloc` is guaranteed to give (its arena header is 12 bytes).
const MALLOC_ALIGN: usize = 4;

unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MALLOC_ALIGN {
            return unsafe { sys::malloc(layout.size()) }.cast();
        }
        // Over-aligned: over-allocate and stash the original pointer just below the result.
        let word = core::mem::size_of::<*mut c_void>();
        let Some(total) = layout.size().checked_add(layout.align() + word) else {
            return core::ptr::null_mut();
        };
        let raw = unsafe { sys::malloc(total) }.cast::<u8>();
        if raw.is_null() {
            return raw;
        }
        let aligned = ((raw as usize + word + layout.align() - 1) & !(layout.align() - 1)) as *mut u8;
        // SAFETY: `aligned - word >= raw` and `aligned + size <= raw + total`.
        unsafe { aligned.sub(word).cast::<*mut u8>().write_unaligned(raw) };
        aligned
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.align() <= MALLOC_ALIGN {
            return unsafe { sys::free(ptr.cast()) };
        }
        let word = core::mem::size_of::<*mut c_void>();
        // SAFETY: `alloc` stored the original pointer in the word just below `ptr`.
        let raw = unsafe { ptr.sub(word).cast::<*mut u8>().read_unaligned() };
        unsafe { sys::free(raw.cast()) };
    }
}

#[global_allocator]
static ALLOCATOR: KernelAlloc = KernelAlloc;
