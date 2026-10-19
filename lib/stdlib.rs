//! `atoi`, `qsort`, `bsearch` and friends: Rust replacement for `lib/stdlib.c` in the kernel.
//!
//! # Design
//!
//! C hands `qsort` a bare `void *array` plus an element count and size.  The entry points
//! at the bottom of this file convert that once into a `&mut [u8]` covering exactly
//! `count * element_size` bytes ([`ffi::slice_from_raw_mut`]).  After that the heap sort
//! and binary search are ordinary safe Rust: every access to an element goes through
//! [`element`] / [`swap_elements`], which are bounds-checked slice operations, so a bug in
//! the index math panics instead of scribbling over kernel memory.
//!
//! The comparison function is the only thing that leaves Rust.  It is a C function pointer
//! that receives pointers to two elements, which we derive from element slices.
//!
//! `sort` is the same in-place heap sort as the C version (O(n log n) time, O(1) space,
//! no allocation, safe on a 4 KB kernel stack), so the order of equal elements matches.
use core::ffi::{c_char, c_int, c_void};

use crate::ffi;

type CompareFn = extern "C" fn(*const c_void, *const c_void) -> c_int;
type CompareWithAuxFn = extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int;

/// Compares two elements, returning a `strcmp()`-style result: negative if the first is
/// smaller, zero if equal, positive if larger.
type ElementComparator<'a> = dyn FnMut(&[u8], &[u8]) -> c_int + 'a;

// ---------------------------------------------------------------------------------------
// atoi
// ---------------------------------------------------------------------------------------

fn is_c_whitespace(ch: u8) -> bool {
    matches!(ch, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Parses an optionally signed decimal integer at the start of `text`, after any leading
/// white space; stops at the first non-digit.  Overflow wraps, as C's did in practice.
pub fn parse_int(text: &[u8]) -> i32 {
    let mut remaining = text.iter().copied().skip_while(|&ch| is_c_whitespace(ch)).peekable();
    let is_negative = matches!(remaining.peek(), Some(b'-'));
    if matches!(remaining.peek(), Some(b'-' | b'+')) {
        remaining.next();
    }

    // Accumulate as a *negative* number: the negative range of an `int` is one larger than
    // the positive range, so this is the only way to represent `INT_MIN` without overflow.
    let mut negative_value = 0i32;
    for digit_char in remaining.take_while(u8::is_ascii_digit) {
        let digit = (digit_char - b'0') as i32;
        negative_value = negative_value.wrapping_mul(10).wrapping_sub(digit);
    }
    if is_negative {
        negative_value
    } else {
        negative_value.wrapping_neg()
    }
}

/// Converts the decimal integer in the C string `text` to an `int`.
///
/// # Safety
/// `text` is a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn atoi(text: *const c_char) -> c_int {
    // SAFETY: caller passes a valid C string.
    parse_int(unsafe { ffi::c_str_bytes(text) })
}

// ---------------------------------------------------------------------------------------
// Element access (all bounds-checked)
// ---------------------------------------------------------------------------------------

/// Element number `index` of an array whose elements are `element_size` bytes each.
fn element(array: &[u8], element_size: usize, index: usize) -> &[u8] {
    &array[index * element_size..(index + 1) * element_size]
}

/// Exchanges two different elements.
fn swap_elements(array: &mut [u8], element_size: usize, first: usize, second: usize) {
    let (lower, higher) = (first.min(second), first.max(second));
    if lower == higher {
        return;
    }
    // Split between the two so each element is a separate `&mut` slice.
    let (head, tail) = array.split_at_mut(higher * element_size);
    let lower_element = &mut head[lower * element_size..(lower + 1) * element_size];
    let higher_element = &mut tail[..element_size];
    lower_element.swap_with_slice(higher_element);
}

// ---------------------------------------------------------------------------------------
// Heap sort
// ---------------------------------------------------------------------------------------

/// Restores the max-heap property below `root` in the first `heap_len` elements, by
/// repeatedly swapping the root's value with its larger child ("floating it down").
///
/// Elements form an implicit binary tree: the children of index `i` are `2i + 1`, `2i + 2`.
fn sift_down(array: &mut [u8], element_size: usize, mut root: usize, heap_len: usize, compare: &mut ElementComparator) {
    loop {
        let left_child = 2 * root + 1;
        let right_child = 2 * root + 2;

        // Find the largest of the root and its (up to two) children.
        let mut largest = root;
        if left_child < heap_len && compare(element(array, element_size, left_child), element(array, element_size, largest)) > 0 {
            largest = left_child;
        }
        if right_child < heap_len && compare(element(array, element_size, right_child), element(array, element_size, largest)) > 0 {
            largest = right_child;
        }
        if largest == root {
            return; // heap property holds
        }
        swap_elements(array, element_size, root, largest);
        root = largest;
    }
}

/// Sorts `array` (consecutive elements of `element_size` bytes) in ascending order.
fn heap_sort(array: &mut [u8], element_size: usize, compare: &mut ElementComparator) {
    let element_count = array.len() / element_size;

    // Phase 1: arrange the elements into a max-heap, from the last parent up to the root.
    for parent in (0..element_count / 2).rev() {
        sift_down(array, element_size, parent, element_count, compare);
    }
    // Phase 2: repeatedly move the largest element (the root) to the end of the unsorted
    // part, shrink the heap by one, and repair it.
    for unsorted_end in (1..element_count).rev() {
        swap_elements(array, element_size, 0, unsorted_end);
        sift_down(array, element_size, 0, unsorted_end, compare);
    }
}

// ---------------------------------------------------------------------------------------
// Binary search
// ---------------------------------------------------------------------------------------

/// Finds an element equal to `key` in the sorted `array`; returns its index.
fn binary_search_index(array: &[u8], element_size: usize, key: &[u8], compare: &mut ElementComparator) -> Option<usize> {
    let mut low = 0; // candidates are in low..high
    let mut high = array.len() / element_size;
    while low < high {
        let middle = low + (high - low) / 2;
        let order = compare(key, element(array, element_size, middle));
        if order < 0 {
            high = middle;
        } else if order > 0 {
            low = middle + 1;
        } else {
            return Some(middle);
        }
    }
    None
}

// ---------------------------------------------------------------------------------------
// C ABI entry points (declared in lib/stdlib.h)
// ---------------------------------------------------------------------------------------

/// Total size in bytes of `count` elements of `element_size`, rejecting overflow.
fn array_len(count: usize, element_size: usize) -> usize {
    assert!(element_size > 0, "element size must be non-zero");
    count.checked_mul(element_size).expect("array size overflows")
}

/// Sorts `count` elements of `element_size` bytes with a `strcmp()`-style `compare`.
///
/// # Safety
/// `array` is valid for reads and writes of `count * element_size` bytes.
#[no_mangle]
pub unsafe extern "C" fn qsort(array: *mut c_void, count: usize, element_size: usize, compare: Option<CompareFn>) {
    let compare = compare.expect("qsort: null compare function");
    // SAFETY: caller passes a valid array of that extent.
    let bytes = unsafe { ffi::slice_from_raw_mut(array.cast::<u8>(), array_len(count, element_size)) };
    heap_sort(bytes, element_size, &mut |a, b| compare(a.as_ptr().cast(), b.as_ptr().cast()));
}

/// Like [`qsort`], passing `aux` to every `compare` call.
///
/// # Safety
/// As [`qsort`].
#[no_mangle]
pub unsafe extern "C" fn sort(array: *mut c_void, count: usize, element_size: usize, compare: Option<CompareWithAuxFn>, aux: *mut c_void) {
    let compare = compare.expect("sort: null compare function");
    // SAFETY: caller passes a valid array of that extent.
    let bytes = unsafe { ffi::slice_from_raw_mut(array.cast::<u8>(), array_len(count, element_size)) };
    heap_sort(bytes, element_size, &mut |a, b| compare(a.as_ptr().cast(), b.as_ptr().cast(), aux));
}

/// Searches the sorted array for `key`; returns a pointer to a matching element or NULL.
///
/// # Safety
/// `array` is valid for reads of `count * element_size` bytes and sorted by `compare`;
/// `key` points to an element-sized value.
#[no_mangle]
pub unsafe extern "C" fn bsearch(key: *const c_void, array: *const c_void, count: usize, element_size: usize, compare: Option<CompareFn>) -> *mut c_void {
    let compare = compare.expect("bsearch: null compare function");
    // SAFETY: caller passes a valid array and key.
    let (bytes, key_bytes) = unsafe {
        (ffi::slice_from_raw(array.cast::<u8>(), array_len(count, element_size)), ffi::slice_from_raw(key.cast::<u8>(), element_size))
    };
    let found = binary_search_index(bytes, element_size, key_bytes, &mut |a, b| compare(a.as_ptr().cast(), b.as_ptr().cast()));
    found.map_or(core::ptr::null_mut(), |index| element(bytes, element_size, index).as_ptr().cast_mut().cast())
}

/// Like [`bsearch`], passing `aux` to every `compare` call.
///
/// # Safety
/// As [`bsearch`].
#[no_mangle]
pub unsafe extern "C" fn binary_search(
    key: *const c_void,
    array: *const c_void,
    count: usize,
    element_size: usize,
    compare: Option<CompareWithAuxFn>,
    aux: *mut c_void,
) -> *mut c_void {
    let compare = compare.expect("binary_search: null compare function");
    // SAFETY: caller passes a valid array and key.
    let (bytes, key_bytes) = unsafe {
        (ffi::slice_from_raw(array.cast::<u8>(), array_len(count, element_size)), ffi::slice_from_raw(key.cast::<u8>(), element_size))
    };
    let found = binary_search_index(bytes, element_size, key_bytes, &mut |a, b| compare(a.as_ptr().cast(), b.as_ptr().cast(), aux));
    found.map_or(core::ptr::null_mut(), |index| element(bytes, element_size, index).as_ptr().cast_mut().cast())
}
