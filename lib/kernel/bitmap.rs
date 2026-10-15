//! Bitmaps: Rust replacement for `lib/kernel/bitmap.c`.
//!
//! Exports the exact C ABI declared in `lib/kernel/bitmap.h` (`struct bitmap` layout
//! included, since `palloc` and friends build bitmaps in caller-provided buffers), so
//! no C caller changes.  Unlike the C version, every bit index is bounds-checked, and
//! single-bit updates are `Relaxed` atomic read-modify-writes, which keeps the C code's
//! guarantee that `bitmap_mark/reset/flip` are not torn by an interrupt handler.
use core::ffi::c_void;
use core::slice;
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use crate::sys;

/// Same element type as the C code (`unsigned long`; 32 bits on i686).
type Elem = usize;
const ELEM_BITS: usize = Elem::BITS as usize;
const ELEM_BYTES: usize = core::mem::size_of::<Elem>();

/// `BITMAP_ERROR` from `bitmap.h`.
const BITMAP_ERROR: usize = usize::MAX;

/// C-compatible `struct bitmap`.
#[repr(C)]
pub struct Bitmap {
    bit_cnt: usize,
    bits: *mut Elem,
}

const fn elem_cnt(bit_cnt: usize) -> usize {
    bit_cnt.div_ceil(ELEM_BITS)
}

const fn byte_cnt(bit_cnt: usize) -> usize {
    ELEM_BYTES * elem_cnt(bit_cnt)
}

impl Bitmap {
    /// The storage as a slice of atomics, so shared `&Bitmap` can still mutate bits safely.
    fn words(&self) -> &[AtomicUsize] {
        let n = elem_cnt(self.bit_cnt);
        if n == 0 {
            return &[];
        }
        // SAFETY: `bits` points to `n` initialized elements (invariant established by
        // bitmap_create / bitmap_create_in_buf); AtomicUsize has the layout of usize.
        unsafe { slice::from_raw_parts(self.bits.cast::<AtomicUsize>(), n) }
    }

    fn mask(bit: usize) -> Elem {
        1 << (bit % ELEM_BITS)
    }

    fn test(&self, bit: usize) -> bool {
        assert!(bit < self.bit_cnt, "bitmap index out of range");
        self.words()[bit / ELEM_BITS].load(Relaxed) & Self::mask(bit) != 0
    }

    fn set(&self, bit: usize, value: bool) {
        assert!(bit < self.bit_cnt, "bitmap index out of range");
        let (w, m) = (&self.words()[bit / ELEM_BITS], Self::mask(bit));
        if value {
            w.fetch_or(m, Relaxed);
        } else {
            w.fetch_and(!m, Relaxed);
        }
    }

    fn flip(&self, bit: usize) {
        assert!(bit < self.bit_cnt, "bitmap index out of range");
        self.words()[bit / ELEM_BITS].fetch_xor(Self::mask(bit), Relaxed);
    }

    /// Validates `[start, start + cnt)` against the bitmap size.
    fn check_range(&self, start: usize, cnt: usize) {
        assert!(start <= self.bit_cnt, "bitmap range start out of range");
        assert!(
            start.checked_add(cnt).is_some_and(|end| end <= self.bit_cnt),
            "bitmap range out of range"
        );
    }

    fn set_multiple(&self, start: usize, cnt: usize, value: bool) {
        self.check_range(start, cnt);
        for i in start..start + cnt {
            self.set(i, value);
        }
    }

    fn contains(&self, start: usize, cnt: usize, value: bool) -> bool {
        self.check_range(start, cnt);
        (start..start + cnt).any(|i| self.test(i) == value)
    }

    fn count(&self, start: usize, cnt: usize, value: bool) -> usize {
        self.check_range(start, cnt);
        (start..start + cnt).filter(|&i| self.test(i) == value).count()
    }

    fn scan(&self, start: usize, cnt: usize, value: bool) -> usize {
        assert!(start <= self.bit_cnt, "bitmap range start out of range");
        if cnt <= self.bit_cnt {
            let last = self.bit_cnt - cnt;
            for i in start..=last {
                if !self.contains(i, cnt, !value) {
                    return i;
                }
            }
        }
        BITMAP_ERROR
    }
}

// ---- C ABI ---------------------------------------------------------------------------
//
// SAFETY contract for every function taking `*mut Bitmap` / `*const Bitmap`: the caller
// passes a bitmap made by bitmap_create / bitmap_create_in_buf that has not been
// destroyed -- the same contract as the C implementation.

/// # Safety
/// `b` must be a live bitmap.
unsafe fn bm<'a>(b: *const Bitmap) -> &'a Bitmap {
    assert!(!b.is_null(), "null bitmap");
    unsafe { &*b }
}

#[no_mangle]
pub extern "C" fn bitmap_create(bit_cnt: usize) -> *mut Bitmap {
    // SAFETY: plain kernel malloc/free; the new bitmap is fully initialized before return.
    unsafe {
        let b = sys::malloc(core::mem::size_of::<Bitmap>()).cast::<Bitmap>();
        if b.is_null() {
            return b;
        }
        let bits = sys::malloc(byte_cnt(bit_cnt)).cast::<Elem>();
        if bits.is_null() && bit_cnt != 0 {
            sys::free(b.cast());
            return core::ptr::null_mut();
        }
        b.write(Bitmap { bit_cnt, bits });
        bm(b).set_multiple(0, bit_cnt, false);
        b
    }
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_create_in_buf(
    bit_cnt: usize,
    block: *mut c_void,
    block_size: usize,
) -> *mut Bitmap {
    assert!(block_size >= bitmap_buf_size(bit_cnt));
    let b = block.cast::<Bitmap>();
    // SAFETY: the caller provides `block_size >= bitmap_buf_size(bit_cnt)` writable bytes,
    // suitably aligned for a Bitmap (same contract as the C version).
    unsafe {
        b.write(Bitmap { bit_cnt, bits: b.add(1).cast::<Elem>() });
        bm(b).set_multiple(0, bit_cnt, false);
    }
    b
}

#[no_mangle]
pub extern "C" fn bitmap_buf_size(bit_cnt: usize) -> usize {
    core::mem::size_of::<Bitmap>() + byte_cnt(bit_cnt)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_destroy(b: *mut Bitmap) {
    if !b.is_null() {
        // SAFETY: created by bitmap_create, so both blocks came from malloc.
        unsafe {
            sys::free((*b).bits.cast());
            sys::free(b.cast());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_size(b: *const Bitmap) -> usize {
    unsafe { bm(b) }.bit_cnt
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_set(b: *mut Bitmap, idx: usize, value: bool) {
    unsafe { bm(b) }.set(idx, value)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_mark(b: *mut Bitmap, idx: usize) {
    unsafe { bm(b) }.set(idx, true)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_reset(b: *mut Bitmap, idx: usize) {
    unsafe { bm(b) }.set(idx, false)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_flip(b: *mut Bitmap, idx: usize) {
    unsafe { bm(b) }.flip(idx)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_test(b: *const Bitmap, idx: usize) -> bool {
    unsafe { bm(b) }.test(idx)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_set_all(b: *mut Bitmap, value: bool) {
    let b = unsafe { bm(b) };
    b.set_multiple(0, b.bit_cnt, value)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_set_multiple(b: *mut Bitmap, start: usize, cnt: usize, value: bool) {
    unsafe { bm(b) }.set_multiple(start, cnt, value)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_count(b: *const Bitmap, start: usize, cnt: usize, value: bool) -> usize {
    unsafe { bm(b) }.count(start, cnt, value)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_contains(b: *const Bitmap, start: usize, cnt: usize, value: bool) -> bool {
    unsafe { bm(b) }.contains(start, cnt, value)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_any(b: *const Bitmap, start: usize, cnt: usize) -> bool {
    unsafe { bm(b) }.contains(start, cnt, true)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_none(b: *const Bitmap, start: usize, cnt: usize) -> bool {
    !unsafe { bm(b) }.contains(start, cnt, true)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_all(b: *const Bitmap, start: usize, cnt: usize) -> bool {
    !unsafe { bm(b) }.contains(start, cnt, false)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_scan(b: *const Bitmap, start: usize, cnt: usize, value: bool) -> usize {
    unsafe { bm(b) }.scan(start, cnt, value)
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_scan_and_flip(b: *mut Bitmap, start: usize, cnt: usize, value: bool) -> usize {
    let b = unsafe { bm(b) };
    let idx = b.scan(start, cnt, value);
    if idx != BITMAP_ERROR {
        b.set_multiple(idx, cnt, !value);
    }
    idx
}

#[no_mangle]
pub unsafe extern "C" fn bitmap_dump(b: *const Bitmap) {
    let b = unsafe { bm(b) };
    // SAFETY: `bits` is valid for byte_cnt(bit_cnt) bytes.
    unsafe { sys::hex_dump(0, b.bits.cast(), byte_cnt(b.bit_cnt), false) }
}

// File input and output (only in kernels that have a file system).
#[cfg(feature = "filesys")]
mod file_io {
    use super::*;
    use crate::sys::{file_read_at, file_write_at, File, Off};

    /// Mask of the bits actually used in the last element.
    fn last_mask(b: &Bitmap) -> Elem {
        match b.bit_cnt % ELEM_BITS {
            0 => Elem::MAX,
            n => (1 << n) - 1,
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn bitmap_file_size(b: *const Bitmap) -> usize {
        byte_cnt(unsafe { bm(b) }.bit_cnt)
    }

    #[no_mangle]
    pub unsafe extern "C" fn bitmap_read(b: *mut Bitmap, file: *mut File) -> bool {
        let b = unsafe { bm(b) };
        if b.bit_cnt == 0 {
            return true;
        }
        let size = byte_cnt(b.bit_cnt) as Off;
        // SAFETY: `bits` has room for `size` bytes.
        let success = unsafe { file_read_at(file, b.bits.cast(), size, 0) } == size;
        b.words()[elem_cnt(b.bit_cnt) - 1].fetch_and(last_mask(b), Relaxed);
        success
    }

    #[no_mangle]
    pub unsafe extern "C" fn bitmap_write(b: *const Bitmap, file: *mut File) -> bool {
        let b = unsafe { bm(b) };
        let size = byte_cnt(b.bit_cnt) as Off;
        // SAFETY: `bits` holds `size` readable bytes.
        unsafe { file_write_at(file, b.bits.cast(), size, 0) == size }
    }
}
