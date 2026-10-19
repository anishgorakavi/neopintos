//! RC4-based pseudo-random numbers: Rust replacement for `lib/random.c` in the kernel.
//!
//! RC4 is a stream cipher; we use it only because it is tiny and its output is plenty
//! random for tests and scheduling jitter (not for security).  It keeps a permutation of
//! the byte values 0..=255 and two indices into it:
//!
//!  * **init**: start from the identity permutation and shuffle it using the seed bytes;
//!  * **next byte**: advance the indices, swap two entries of the permutation, and read
//!    the byte at the position given by the sum of the swapped entries.
//!
//! The byte stream for a given seed is identical to the C version, so a test that
//! passes `-rs=<seed>` behaves the same.  Like the C code, the generator is global,
//! unsynchronized, and not for use from interrupt handlers.
use core::ffi::c_void;

use crate::ffi;

/// The RC4 generator state.
pub struct Rc4 {
    permutation: [u8; 256],
    index_i: u8,
    index_j: u8,
    seeded: bool,
}

impl Rc4 {
    pub const fn new() -> Self {
        Self { permutation: [0; 256], index_i: 0, index_j: 0, seeded: false }
    }

    /// Seeds the generator.  Like the C version, only the first call has any effect.
    pub fn seed(&mut self, seed: u32) {
        if self.seeded {
            return;
        }
        let seed_bytes = seed.to_le_bytes();
        for (position, entry) in self.permutation.iter_mut().enumerate() {
            *entry = position as u8;
        }
        // Key-scheduling: mix each seed byte into the permutation.
        let mut swap_with = 0u8;
        for position in 0..256 {
            let seed_byte = seed_bytes[position % seed_bytes.len()];
            swap_with = swap_with.wrapping_add(self.permutation[position]).wrapping_add(seed_byte);
            self.permutation.swap(position, swap_with as usize);
        }
        self.index_i = 0;
        self.index_j = 0;
        self.seeded = true;
    }

    fn next_byte(&mut self) -> u8 {
        self.index_i = self.index_i.wrapping_add(1);
        self.index_j = self.index_j.wrapping_add(self.permutation[self.index_i as usize]);
        self.permutation.swap(self.index_i as usize, self.index_j as usize);

        let sum = self.permutation[self.index_i as usize].wrapping_add(self.permutation[self.index_j as usize]);
        self.permutation[sum as usize]
    }

    /// Fills `buffer` with random bytes, seeding with 0 first if nobody has seeded yet.
    pub fn fill(&mut self, buffer: &mut [u8]) {
        self.seed(0);
        for byte in buffer {
            *byte = self.next_byte();
        }
    }
}

static mut GLOBAL_GENERATOR: Rc4 = Rc4::new();

/// The kernel-wide generator behind the C functions below.
fn global_generator() -> &'static mut Rc4 {
    // SAFETY: same contract as the C globals this replaces: the generator is only driven
    // from kernel threads (never an interrupt handler), and no caller keeps the reference
    // across a call that could reach this function again.
    unsafe { &mut *(&raw mut GLOBAL_GENERATOR) }
}

/// Seeds the PRNG; calls after the first are ignored.
#[no_mangle]
pub extern "C" fn random_init(seed: u32) {
    global_generator().seed(seed);
}

/// Writes `size` random bytes into `buffer`.
///
/// # Safety
/// `buffer` is writable for `size` bytes.
#[no_mangle]
pub unsafe extern "C" fn random_bytes(buffer: *mut c_void, size: usize) {
    // SAFETY: caller passes a writable buffer of that size.
    let bytes = unsafe { ffi::slice_from_raw_mut(buffer.cast::<u8>(), size) };
    global_generator().fill(bytes);
}

/// Returns a pseudo-random `unsigned long`; use `random_ulong() % n` for a value in `0..n`.
#[no_mangle]
pub extern "C" fn random_ulong() -> usize {
    let mut bytes = [0u8; core::mem::size_of::<usize>()];
    global_generator().fill(&mut bytes);
    usize::from_le_bytes(bytes)
}
