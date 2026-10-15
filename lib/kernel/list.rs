//! Doubly linked lists: Rust replacement for `lib/kernel/list.c`.
//!
//! Exports the exact C ABI declared in `lib/kernel/list.h`; `struct list` and
//! `struct list_elem` are intrusive and embedded in C structs, so their layout is fixed
//! (`#[repr(C)]`) and the `list_entry`/`LIST_INITIALIZER` macros stay in the header.
//!
//! As in C, a list has a head and a tail sentinel: the head's `prev` and the tail's
//! `next` are null, and the other two links point toward each other.  The C `ASSERT`s are
//! kept as `assert!`s (they were active in the stock kernel too).
use core::ffi::c_void;
use core::ptr::{self, null_mut};

/// C-compatible `struct list_elem`.
#[repr(C)]
pub struct ListElem {
    pub(crate) prev: *mut ListElem,
    pub(crate) next: *mut ListElem,
}

/// C-compatible `struct list`.
#[repr(C)]
pub struct List {
    pub(crate) head: ListElem,
    pub(crate) tail: ListElem,
}

/// `list_less_func`: true if A is less than B.  `Option` because C passes NULL to be
/// checked by an assertion, which a plain `extern "C" fn` type could not represent.
pub type LessFunc = Option<unsafe extern "C" fn(*const ListElem, *const ListElem, *mut c_void) -> bool>;

// The sentinel predicates read the links through raw pointers: elements are shared with C.
unsafe fn is_head(e: *const ListElem) -> bool {
    // SAFETY: the caller guarantees `e` is null or valid.
    !e.is_null() && unsafe { (*e).prev.is_null() && !(*e).next.is_null() }
}

unsafe fn is_interior(e: *const ListElem) -> bool {
    // SAFETY: as above.
    !e.is_null() && unsafe { !(*e).prev.is_null() && !(*e).next.is_null() }
}

unsafe fn is_tail(e: *const ListElem) -> bool {
    // SAFETY: as above.
    !e.is_null() && unsafe { !(*e).prev.is_null() && (*e).next.is_null() }
}

/// Calls the comparison function.  A NULL `less` panics (C would fault on the call).
unsafe fn lt(less: LessFunc, a: *const ListElem, b: *const ListElem, aux: *mut c_void) -> bool {
    let f = less.expect("list comparison function is NULL");
    // SAFETY: caller guarantees a, b are valid elements.
    unsafe { f(a, b, aux) }
}

#[no_mangle]
pub unsafe extern "C" fn list_init(list: *mut List) {
    assert!(!list.is_null());
    // SAFETY: `list` is non-null and points to writable storage for a `struct list`.
    unsafe {
        (*list).head.prev = null_mut();
        (*list).head.next = &raw mut (*list).tail;
        (*list).tail.prev = &raw mut (*list).head;
        (*list).tail.next = null_mut();
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_begin(list: *mut List) -> *mut ListElem {
    assert!(!list.is_null());
    // SAFETY: `list` is a valid initialized list.
    unsafe { (*list).head.next }
}

#[no_mangle]
pub unsafe extern "C" fn list_next(elem: *mut ListElem) -> *mut ListElem {
    // SAFETY: `elem` is a valid element (checked non-tail below).
    unsafe {
        assert!(is_head(elem) || is_interior(elem));
        (*elem).next
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_end(list: *mut List) -> *mut ListElem {
    assert!(!list.is_null());
    // SAFETY: only forms a pointer to the tail field.
    unsafe { &raw mut (*list).tail }
}

#[no_mangle]
pub unsafe extern "C" fn list_rbegin(list: *mut List) -> *mut ListElem {
    assert!(!list.is_null());
    // SAFETY: `list` is a valid initialized list.
    unsafe { (*list).tail.prev }
}

#[no_mangle]
pub unsafe extern "C" fn list_prev(elem: *mut ListElem) -> *mut ListElem {
    // SAFETY: `elem` is a valid element (checked non-head below).
    unsafe {
        assert!(is_interior(elem) || is_tail(elem));
        (*elem).prev
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_rend(list: *mut List) -> *mut ListElem {
    assert!(!list.is_null());
    // SAFETY: only forms a pointer to the head field.
    unsafe { &raw mut (*list).head }
}

#[no_mangle]
pub unsafe extern "C" fn list_head(list: *mut List) -> *mut ListElem {
    // SAFETY: as list_rend.
    unsafe { list_rend(list) }
}

#[no_mangle]
pub unsafe extern "C" fn list_tail(list: *mut List) -> *mut ListElem {
    // SAFETY: as list_end.
    unsafe { list_end(list) }
}

#[no_mangle]
pub unsafe extern "C" fn list_insert(before: *mut ListElem, elem: *mut ListElem) {
    // SAFETY: `before` is an interior element or tail of a valid list, `elem` is writable.
    unsafe {
        assert!(is_interior(before) || is_tail(before));
        assert!(!elem.is_null());
        (*elem).prev = (*before).prev;
        (*elem).next = before;
        (*(*before).prev).next = elem;
        (*before).prev = elem;
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_splice(before: *mut ListElem, first: *mut ListElem, last: *mut ListElem) {
    // SAFETY: FIRST..LAST (exclusive) is a range of interior elements of one valid list.
    unsafe {
        assert!(is_interior(before) || is_tail(before));
        if first == last {
            return;
        }
        let last = list_prev(last);
        assert!(is_interior(first));
        assert!(is_interior(last));

        // Cleanly remove FIRST...LAST from its current list.
        (*(*first).prev).next = (*last).next;
        (*(*last).next).prev = (*first).prev;

        // Splice FIRST...LAST into the new list.
        (*first).prev = (*before).prev;
        (*last).next = before;
        (*(*before).prev).next = first;
        (*before).prev = last;
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_push_front(list: *mut List, elem: *mut ListElem) {
    // SAFETY: forwards valid arguments.
    unsafe { list_insert(list_begin(list), elem) }
}

#[no_mangle]
pub unsafe extern "C" fn list_push_back(list: *mut List, elem: *mut ListElem) {
    // SAFETY: forwards valid arguments.
    unsafe { list_insert(list_end(list), elem) }
}

#[no_mangle]
pub unsafe extern "C" fn list_remove(elem: *mut ListElem) -> *mut ListElem {
    // SAFETY: `elem` is an interior element of a valid list.
    unsafe {
        assert!(is_interior(elem));
        (*(*elem).prev).next = (*elem).next;
        (*(*elem).next).prev = (*elem).prev;
        (*elem).next
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_pop_front(list: *mut List) -> *mut ListElem {
    // SAFETY: the list is valid; `list_front` asserts it is non-empty.
    unsafe {
        let front = list_front(list);
        list_remove(front);
        front
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_pop_back(list: *mut List) -> *mut ListElem {
    // SAFETY: the list is valid; `list_back` asserts it is non-empty.
    unsafe {
        let back = list_back(list);
        list_remove(back);
        back
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_front(list: *mut List) -> *mut ListElem {
    // SAFETY: the list is valid.
    unsafe {
        assert!(!list_empty(list));
        (*list).head.next
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_back(list: *mut List) -> *mut ListElem {
    // SAFETY: the list is valid.
    unsafe {
        assert!(!list_empty(list));
        (*list).tail.prev
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_size(list: *mut List) -> usize {
    // SAFETY: the list is valid.
    unsafe {
        let mut cnt = 0;
        let mut e = list_begin(list);
        while e != list_end(list) {
            cnt += 1;
            e = list_next(e);
        }
        cnt
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_empty(list: *mut List) -> bool {
    // SAFETY: the list is valid.
    unsafe { list_begin(list) == list_end(list) }
}

#[no_mangle]
pub unsafe extern "C" fn list_reverse(list: *mut List) {
    // SAFETY: the list is valid; every swap stays within its elements.
    unsafe {
        if !list_empty(list) {
            let mut e = list_begin(list);
            while e != list_end(list) {
                // Swap links, then step "forward" via the (now swapped) prev link.
                ptr::swap(&raw mut (*e).prev, &raw mut (*e).next);
                e = (*e).prev;
            }
            ptr::swap(&raw mut (*list).head.next, &raw mut (*list).tail.prev);
            ptr::swap(&raw mut (*(*list).head.next).prev, &raw mut (*(*list).tail.prev).next);
        }
    }
}

/// True only if elements A through B (exclusive) are in order according to LESS.
unsafe fn is_sorted(mut a: *mut ListElem, b: *mut ListElem, less: LessFunc, aux: *mut c_void) -> bool {
    // SAFETY: A..B is a valid range of one list.
    unsafe {
        if a != b {
            loop {
                a = list_next(a);
                if a == b {
                    break;
                }
                if lt(less, a, list_prev(a), aux) {
                    return false;
                }
            }
        }
    }
    true
}

/// Finds the end (exclusive) of the nondecreasing run starting at A and ending not after B.
unsafe fn find_end_of_run(mut a: *mut ListElem, b: *mut ListElem, less: LessFunc, aux: *mut c_void) -> *mut ListElem {
    assert!(!a.is_null());
    assert!(!b.is_null());
    assert!(less.is_some());
    assert!(a != b);
    // SAFETY: A..B is a valid non-empty range of one list.
    unsafe {
        loop {
            a = list_next(a);
            if a == b || lt(less, a, list_prev(a), aux) {
                return a;
            }
        }
    }
}

/// Merges the sorted ranges A0..A1B0 and A1B0..B1 into one sorted range ending at B1.
unsafe fn inplace_merge(
    mut a0: *mut ListElem,
    mut a1b0: *mut ListElem,
    b1: *mut ListElem,
    less: LessFunc,
    aux: *mut c_void,
) {
    assert!(!a0.is_null());
    assert!(!a1b0.is_null());
    assert!(!b1.is_null());
    assert!(less.is_some());
    // SAFETY: both ranges are valid, non-empty and adjacent in one list.
    unsafe {
        assert!(is_sorted(a0, a1b0, less, aux));
        assert!(is_sorted(a1b0, b1, less, aux));

        while a0 != a1b0 && a1b0 != b1 {
            if !lt(less, a1b0, a0, aux) {
                a0 = list_next(a0);
            } else {
                a1b0 = list_next(a1b0);
                list_splice(a0, list_prev(a1b0), a1b0);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_sort(list: *mut List, less: LessFunc, aux: *mut c_void) {
    assert!(!list.is_null());
    assert!(less.is_some());
    // SAFETY: the list is valid and `less` is non-null.
    unsafe {
        // Merge adjacent nondecreasing runs, pass after pass, until one run is left.
        loop {
            let mut output_run_cnt = 0usize;
            let mut a0 = list_begin(list);
            while a0 != list_end(list) {
                output_run_cnt += 1;
                let a1b0 = find_end_of_run(a0, list_end(list), less, aux);
                if a1b0 == list_end(list) {
                    break;
                }
                let b1 = find_end_of_run(a1b0, list_end(list), less, aux);
                inplace_merge(a0, a1b0, b1, less, aux);
                a0 = b1;
            }
            if output_run_cnt <= 1 {
                break;
            }
        }
        assert!(is_sorted(list_begin(list), list_end(list), less, aux));
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_insert_ordered(list: *mut List, elem: *mut ListElem, less: LessFunc, aux: *mut c_void) {
    assert!(!list.is_null());
    assert!(!elem.is_null());
    assert!(less.is_some());
    // SAFETY: the list is valid and sorted by `less`.
    unsafe {
        let mut e = list_begin(list);
        while e != list_end(list) {
            if lt(less, elem, e, aux) {
                break;
            }
            e = list_next(e);
        }
        list_insert(e, elem);
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_unique(list: *mut List, duplicates: *mut List, less: LessFunc, aux: *mut c_void) {
    assert!(!list.is_null());
    assert!(less.is_some());
    // SAFETY: the lists are valid.
    unsafe {
        if list_empty(list) {
            return;
        }
        let mut elem = list_begin(list);
        loop {
            let next = list_next(elem);
            if next == list_end(list) {
                break;
            }
            if !lt(less, elem, next, aux) && !lt(less, next, elem, aux) {
                list_remove(next);
                if !duplicates.is_null() {
                    list_push_back(duplicates, next);
                }
            } else {
                elem = next;
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_max(list: *mut List, less: LessFunc, aux: *mut c_void) -> *mut ListElem {
    // SAFETY: the list is valid.  A NULL `less` panics on first use (lists with 2+ elements).
    unsafe {
        let mut max = list_begin(list);
        if max != list_end(list) {
            let mut e = list_next(max);
            while e != list_end(list) {
                if lt(less, max, e, aux) {
                    max = e;
                }
                e = list_next(e);
            }
        }
        max
    }
}

#[no_mangle]
pub unsafe extern "C" fn list_min(list: *mut List, less: LessFunc, aux: *mut c_void) -> *mut ListElem {
    // SAFETY: as list_max.
    unsafe {
        let mut min = list_begin(list);
        if min != list_end(list) {
            let mut e = list_next(min);
            while e != list_end(list) {
                if lt(less, e, min, aux) {
                    min = e;
                }
                e = list_next(e);
            }
        }
        min
    }
}
