//! In-kernel self-test for the Rust runtime and for every Rust-ported component.
//! Run with:  pintos -- -q rust-selftest
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;


static mut CHECKS: u32 = 0;

/// Records one passed check (single-threaded boot context; panics on failure).
pub fn check(cond: bool, what: &str) {
    assert!(cond, "rust-selftest failed: {}", what);
    // SAFETY: the self-test runs from the single kernel main thread.
    unsafe { CHECKS += 1 };
    kprintln!("(rust-selftest) ok: {}", what);
}

pub fn run() {
    kprintln!("(rust-selftest) begin");
    alloc_tests();
    bitmap_tests();
    list_tests();
    list_edge_tests();
    stdio_tests();
    stdlib_tests();
    random_tests();
    // SAFETY: as above.
    let n = unsafe { CHECKS };
    kprintln!("(rust-selftest) end: {} checks passed", n);
}

fn alloc_tests() {
    let b = Box::new(0xdead_beef_u32);
    check(*b == 0xdead_beef, "Box round-trip");

    let mut v: Vec<u32> = Vec::new();
    for i in 0..5000 {
        v.push(i * 3);
    }
    check(v.iter().enumerate().all(|(i, &x)| x == i as u32 * 3), "Vec growth across many pages");
    drop(v);

    #[repr(align(64))]
    struct Aligned([u8; 100]);
    let a = Box::new(Aligned([7; 100]));
    check((&*a as *const Aligned as usize) % 64 == 0 && a.0[99] == 7, "over-aligned allocation");

    let mut s = String::new();
    for i in 0..50 {
        s.push_str("ab");
        s.push((b'0' + (i % 10) as u8) as char);
    }
    check(s.len() == 150, "String building");
}

/// Differential test of the Rust bitmap (called through its C ABI) against a `Vec<bool>` model.
fn bitmap_tests() {
    use crate::bitmap::*;

    struct Rng(u32);
    impl Rng {
        fn next(&mut self, n: usize) -> usize {
            self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
            ((self.0 >> 8) as usize) % n
        }
    }

    let mut rng = Rng(12345);
    let mut all_ok = true;
    for &nbits in &[0usize, 1, 31, 32, 33, 70, 200] {
        // SAFETY: b comes from bitmap_create and is destroyed exactly once below.
        unsafe {
            let b = bitmap_create(nbits);
            all_ok &= !b.is_null() && bitmap_size(b) == nbits && bitmap_none(b, 0, nbits);
            let mut model = alloc::vec![false; nbits];
            for _ in 0..400 {
                if nbits == 0 {
                    break;
                }
                let (start, cnt) = {
                    let s = rng.next(nbits + 1);
                    (s, rng.next(nbits - s + 1))
                };
                let val = rng.next(2) == 1;
                match rng.next(6) {
                    0 => {
                        let i = rng.next(nbits);
                        bitmap_set(b, i, val);
                        model[i] = val;
                    }
                    1 => {
                        let i = rng.next(nbits);
                        bitmap_flip(b, i);
                        model[i] = !model[i];
                    }
                    2 => {
                        bitmap_set_multiple(b, start, cnt, val);
                        model[start..start + cnt].iter_mut().for_each(|x| *x = val);
                    }
                    3 => {
                        let want = model[start..start + cnt].iter().filter(|&&x| x == val).count();
                        all_ok &= bitmap_count(b, start, cnt, val) == want;
                        all_ok &= bitmap_contains(b, start, cnt, val) == (want > 0);
                        all_ok &= bitmap_any(b, start, cnt) == model[start..start + cnt].iter().any(|&x| x);
                        all_ok &= bitmap_all(b, start, cnt) == model[start..start + cnt].iter().all(|&x| x);
                    }
                    4 => {
                        let want = (start..=nbits.saturating_sub(cnt))
                            .find(|&i| cnt <= nbits && model[i..i + cnt].iter().all(|&x| x == val))
                            .unwrap_or(usize::MAX);
                        all_ok &= bitmap_scan(b, start, cnt, val) == want;
                    }
                    _ => {
                        let want = (start..=nbits.saturating_sub(cnt))
                            .find(|&i| cnt <= nbits && model[i..i + cnt].iter().all(|&x| x == val))
                            .unwrap_or(usize::MAX);
                        let got = bitmap_scan_and_flip(b, start, cnt, val);
                        all_ok &= got == want;
                        if want != usize::MAX {
                            model[want..want + cnt].iter_mut().for_each(|x| *x = !val);
                        }
                    }
                }
            }
            all_ok &= (0..nbits).all(|i| bitmap_test(b, i) == model[i]);
            bitmap_set_all(b, true);
            all_ok &= bitmap_all(b, 0, nbits) && bitmap_count(b, 0, nbits, true) == nbits;
            bitmap_destroy(b);
        }
    }
    check(all_ok, "bitmap matches reference model (random ops, 7 sizes)");

    // create_in_buf: struct + bits live in a caller buffer, as palloc/free-map use it.
    let sz = bitmap_buf_size(100);
    let mut buf = alloc::vec![0usize; sz.div_ceil(4)];
    // SAFETY: buf is large enough and word-aligned.
    let ok = unsafe {
        let b = bitmap_create_in_buf(100, buf.as_mut_ptr().cast(), sz);
        bitmap_mark(b, 99);
        bitmap_scan_and_flip(b, 0, 99, false) == 0 && bitmap_all(b, 0, 100) && bitmap_size(b) == 100
    };
    check(ok, "bitmap_create_in_buf");
}

/// Differential test of the Rust list (through its C ABI) against a `Vec` model.
fn list_tests() {
    use crate::list::*;
    use core::ffi::c_void;

    // Elements embed a ListElem exactly as C structs do; `elem` is first so casts are free.
    #[repr(C)]
    struct Node {
        elem: ListElem,
        key: u32,
        id: u32,
    }
    unsafe extern "C" fn less(a: *const ListElem, b: *const ListElem, _aux: *mut c_void) -> bool {
        // SAFETY: every element in these lists is the first field of a Node.
        unsafe { (*a.cast::<Node>()).key < (*b.cast::<Node>()).key }
    }

    // SAFETY (whole fn): nodes live in `nodes` (never reallocated after the lists are
    // built) and the lists are only touched through the list_* API.
    unsafe {
        let mut seed = 987654321u32;
        let mut rand = |n: u32| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) % n
        };
        let mut ok = true;
        for &n in &[0usize, 1, 2, 3, 10, 57] {
            let mut nodes: Vec<Node> = (0..n)
                .map(|i| Node { elem: core::mem::zeroed(), key: rand(8), id: i as u32 })
                .collect();
            let mut list: List = core::mem::zeroed();
            let mut dups: List = core::mem::zeroed();
            list_init(&mut list);
            list_init(&mut dups);
            ok &= list_empty(&mut list) && list_size(&mut list) == 0;
            for nd in nodes.iter_mut() {
                list_push_back(&mut list, &mut nd.elem);
            }
            let ids = |l: &mut List, rev: bool| -> Vec<u32> {
                let mut v = Vec::new();
                let (mut e, end) = if rev { (list_rbegin(l), list_rend(l)) } else { (list_begin(l), list_end(l)) };
                while e != end {
                    v.push((*e.cast::<Node>()).id);
                    e = if rev { list_prev(e) } else { list_next(e) };
                }
                v
            };
            let mut model: Vec<(u32, u32)> = nodes.iter().map(|x| (x.key, x.id)).collect();
            ok &= list_size(&mut list) == n;

            list_reverse(&mut list);
            model.reverse();
            ok &= ids(&mut list, false) == model.iter().map(|m| m.1).collect::<Vec<_>>();
            ok &= ids(&mut list, true) == model.iter().rev().map(|m| m.1).collect::<Vec<_>>();

            if n > 0 {
                let mx = list_max(&mut list, Some(less), core::ptr::null_mut()).cast::<Node>();
                let mn = list_min(&mut list, Some(less), core::ptr::null_mut()).cast::<Node>();
                let mut mxi = 0;
                let mut mni = 0;
                for i in 1..n {
                    if model[mxi].0 < model[i].0 { mxi = i; }
                    if model[i].0 < model[mni].0 { mni = i; }
                }
                ok &= (*mx).id == model[mxi].1 && (*mn).id == model[mni].1;
            }

            list_sort(&mut list, Some(less), core::ptr::null_mut());
            // Stable insertion sort: std's sort uses a 4 KB stack buffer, a whole kernel stack.
            for i in 1..model.len() {
                let mut j = i;
                while j > 0 && model[j].0 < model[j - 1].0 {
                    model.swap(j, j - 1);
                    j -= 1;
                }
            }
            ok &= ids(&mut list, false) == model.iter().map(|m| m.1).collect::<Vec<_>>();

            list_unique(&mut list, &mut dups, Some(less), core::ptr::null_mut());
            let mut kept: Vec<(u32, u32)> = Vec::new();
            let mut dropped: Vec<u32> = Vec::new();
            for m in &model {
                if kept.last().map_or(false, |k| k.0 == m.0) { dropped.push(m.1) } else { kept.push(*m) }
            }
            ok &= ids(&mut list, false) == kept.iter().map(|m| m.1).collect::<Vec<_>>();
            ok &= ids(&mut dups, false) == dropped;

            // Pop everything off both ends, then re-insert in order.
            let mut popped = 0;
            while !list_empty(&mut list) {
                let e = if popped % 2 == 0 { list_pop_front(&mut list) } else { list_pop_back(&mut list) };
                list_insert_ordered(&mut dups, e, Some(less), core::ptr::null_mut());
                popped += 1;
            }
            ok &= popped == kept.len() && list_size(&mut dups) == n;
            let all = ids(&mut dups, false);
            ok &= all.windows(2).all(|w| nodes[w[0] as usize].key <= nodes[w[1] as usize].key);
        }
        check(ok, "list matches reference model (push/pop/reverse/sort/unique/ordered insert)");

        // Splice a range between two lists.
        let mut nodes: Vec<Node> = (0..6).map(|i| Node { elem: core::mem::zeroed(), key: i, id: i }).collect();
        let mut a: List = core::mem::zeroed();
        let mut b: List = core::mem::zeroed();
        list_init(&mut a);
        list_init(&mut b);
        for nd in nodes.iter_mut() {
            list_push_back(&mut a, &mut nd.elem);
        }
        let first = list_next(list_begin(&mut a));
        let last = list_prev(list_end(&mut a));
        list_splice(list_end(&mut b), first, last);
        check(list_size(&mut a) == 2 && list_size(&mut b) == 4, "list_splice moves a range between lists");
    }
}

/// Edge cases for the Rust list: empty/singleton lists, duplicate keys and stability,
/// splice boundaries, and the structural invariants after every mutation.
#[inline(never)]
fn list_edge_tests() {
    use crate::list::*;
    use core::ffi::c_void;
    use core::ptr::null_mut;

    #[repr(C)]
    struct Node {
        elem: ListElem,
        key: u32,
        id: u32,
    }
    unsafe extern "C" fn less(a: *const ListElem, b: *const ListElem, _: *mut c_void) -> bool {
        // SAFETY: every element here is the first field of a Node.
        unsafe { (*a.cast::<Node>()).key < (*b.cast::<Node>()).key }
    }
    fn mk(keys: &[u32]) -> Vec<Node> {
        // SAFETY: an all-zero ListElem is a valid (unlinked) value.
        keys.iter().enumerate().map(|(i, &k)| Node { elem: unsafe { core::mem::zeroed() }, key: k, id: i as u32 }).collect()
    }
    // Structural invariants: sentinel nulls, prev/next symmetry, forward count == backward
    // count.  Returns the number of interior elements.
    unsafe fn sound(l: *mut List) -> Option<usize> {
        // SAFETY: caller passes an initialized list.
        unsafe {
            let (head, tail) = (&raw mut (*l).head, &raw mut (*l).tail);
            if !(*head).prev.is_null() || !(*tail).next.is_null() {
                return None;
            }
            let (mut fwd, mut e) = (0usize, head);
            while e != tail {
                let nx = (*e).next;
                if nx.is_null() || (*nx).prev != e || fwd > 10_000 {
                    return None;
                }
                e = nx;
                fwd += 1;
            }
            let (mut bwd, mut e) = (0usize, tail);
            while e != head {
                e = (*e).prev;
                if e.is_null() || bwd > 10_000 {
                    return None;
                }
                bwd += 1;
            }
            (fwd == bwd).then(|| fwd - 1) // both walks also count the sentinels' link
        }
    }
    unsafe fn keys(l: *mut List) -> Vec<(u32, u32)> {
        // SAFETY: caller passes an initialized list of Nodes.
        unsafe {
            let mut v = Vec::new();
            let mut e = list_begin(l);
            while e != list_end(l) {
                v.push(((*e.cast::<Node>()).key, (*e.cast::<Node>()).id));
                e = list_next(e);
            }
            v
        }
    }
    let mut ok = true;
    // SAFETY (whole fn): nodes outlive their lists and are linked only via the list API.
    unsafe {
        let nul = null_mut();
        // Empty list.
        let mut l: List = core::mem::zeroed();
        list_init(&mut l);
        ok &= sound(&mut l) == Some(0);
        ok &= list_begin(&mut l) == list_end(&mut l) && list_rbegin(&mut l) == list_rend(&mut l);
        ok &= list_head(&mut l) == list_rend(&mut l) && list_tail(&mut l) == list_end(&mut l);
        ok &= list_max(&mut l, Some(less), nul) == list_end(&mut l);
        ok &= list_min(&mut l, Some(less), nul) == list_end(&mut l);
        list_sort(&mut l, Some(less), nul);
        list_reverse(&mut l);
        list_unique(&mut l, nul.cast(), Some(less), nul);
        ok &= sound(&mut l) == Some(0) && list_empty(&mut l);
        check(ok, "list: empty-list operations");

        // Singleton.
        let mut one = mk(&[5]);
        list_push_front(&mut l, &mut one[0].elem);
        ok &= sound(&mut l) == Some(1) && list_front(&mut l) == list_back(&mut l);
        list_sort(&mut l, Some(less), nul);
        list_reverse(&mut l);
        list_unique(&mut l, nul.cast(), Some(less), nul);
        ok &= sound(&mut l) == Some(1);
        ok &= list_max(&mut l, Some(less), nul) == list_begin(&mut l) && list_min(&mut l, Some(less), nul) == list_begin(&mut l);
        ok &= list_next(list_begin(&mut l)) == list_end(&mut l) && list_prev(list_end(&mut l)) == list_begin(&mut l);
        ok &= list_pop_back(&mut l) == &raw mut one[0].elem && list_empty(&mut l) && sound(&mut l) == Some(0);
        check(ok, "list: singleton operations");

        // Stability and duplicates, across run shapes (sorted, reversed, all-equal, random).
        for shape in 0..4u32 {
            for &n in &[2usize, 5, 16, 33, 100, 200] {
                let ks: Vec<u32> = (0..n as u32)
                    .map(|i| match shape {
                        0 => i / 3,
                        1 => (n as u32 - i) / 3,
                        2 => 7,
                        _ => i.wrapping_mul(2654435761) >> 29,
                    })
                    .collect();
                let mut nodes = mk(&ks);
                list_init(&mut l);
                for nd in nodes.iter_mut() {
                    list_push_back(&mut l, &mut nd.elem);
                }
                // Expected: stable insertion sort of (key, id).
                let mut want: Vec<(u32, u32)> = nodes.iter().map(|x| (x.key, x.id)).collect();
                for i in 1..want.len() {
                    let mut j = i;
                    while j > 0 && want[j].0 < want[j - 1].0 {
                        want.swap(j, j - 1);
                        j -= 1;
                    }
                }
                list_sort(&mut l, Some(less), nul);
                ok &= sound(&mut l) == Some(n) && keys(&mut l) == want;
                // Sorting again must be a no-op.
                list_sort(&mut l, Some(less), nul);
                ok &= keys(&mut l) == want;
                // max/min return the earliest of equal extremes.
                let mx = (*list_max(&mut l, Some(less), nul).cast::<Node>()).id;
                let mn = (*list_min(&mut l, Some(less), nul).cast::<Node>()).id;
                ok &= mn == want[0].1;
                let top = want.last().unwrap().0;
                ok &= mx == want.iter().find(|w| w.0 == top).unwrap().1;
                // unique keeps the first of each equal run.
                let mut dups: List = core::mem::zeroed();
                list_init(&mut dups);
                list_unique(&mut l, &mut dups, Some(less), nul);
                let mut kept = Vec::new();
                for w in &want {
                    if kept.last().map_or(true, |k: &(u32, u32)| k.0 != w.0) {
                        kept.push(*w);
                    }
                }
                ok &= keys(&mut l) == kept && sound(&mut l) == Some(kept.len()) && sound(&mut dups) == Some(n - kept.len());
            }
        }
        check(ok, "list: stable sort, duplicates, min/max ties, unique over 24 shapes");

        // insert_ordered places an element after existing equal keys.
        let mut nodes = mk(&[1, 3, 3, 5, 3]);
        list_init(&mut l);
        for nd in nodes.iter_mut().take(4) {
            list_push_back(&mut l, &mut nd.elem);
        }
        list_insert_ordered(&mut l, &mut nodes[4].elem, Some(less), nul);
        ok &= keys(&mut l) == [(1, 0), (3, 1), (3, 2), (3, 4), (5, 3)] && sound(&mut l) == Some(5);
        let mut lo = mk(&[0]);
        let mut hi = mk(&[9]);
        list_insert_ordered(&mut l, &mut lo[0].elem, Some(less), nul);
        list_insert_ordered(&mut l, &mut hi[0].elem, Some(less), nul);
        ok &= list_front(&mut l) == &raw mut lo[0].elem && list_back(&mut l) == &raw mut hi[0].elem && sound(&mut l) == Some(7);
        check(ok, "list: insert_ordered at front, back and among equal keys");

        // Splice boundaries.
        let mut nodes = mk(&[0, 1, 2, 3, 4, 5]);
        let (mut a, mut b): (List, List) = (core::mem::zeroed(), core::mem::zeroed());
        list_init(&mut a);
        list_init(&mut b);
        for nd in nodes.iter_mut() {
            list_push_back(&mut a, &mut nd.elem);
        }
        let ids = |l: *mut List| keys(l).into_iter().map(|k| k.1).collect::<Vec<_>>();
        // Empty range is a no-op (first == last), even with BEFORE inside the range.
        let e2 = &raw mut nodes[2].elem;
        list_splice(&raw mut nodes[4].elem, e2, e2);
        ok &= ids(&mut a) == [0, 1, 2, 3, 4, 5] && sound(&mut a) == Some(6);
        // Single element to an empty list's tail.
        list_splice(list_end(&mut b), e2, list_next(e2));
        ok &= ids(&mut a) == [0, 1, 3, 4, 5] && ids(&mut b) == [2] && sound(&mut a) == Some(5) && sound(&mut b) == Some(1);
        // Range ending at the source tail, inserted at the destination front.
        list_splice(list_begin(&mut b), &raw mut nodes[4].elem, list_end(&mut a));
        ok &= ids(&mut a) == [0, 1, 3] && ids(&mut b) == [4, 5, 2];
        // Range starting at the source's first element: whole list to another's tail.
        list_splice(list_end(&mut b), list_begin(&mut a), list_end(&mut a));
        ok &= list_empty(&mut a) && ids(&mut b) == [4, 5, 2, 0, 1, 3] && sound(&mut a) == Some(0) && sound(&mut b) == Some(6);
        // Within one list: move [4,5] to before 3 (reordering).
        list_splice(&raw mut nodes[3].elem, &raw mut nodes[4].elem, &raw mut nodes[2].elem);
        ok &= ids(&mut b) == [2, 0, 1, 4, 5, 3] && sound(&mut b) == Some(6);
        check(ok, "list: splice boundaries (empty, single, tail, whole list, same list)");

        // Reverse twice is identity; invariants hold after each pop/remove/insert.
        let before = ids(&mut b);
        list_reverse(&mut b);
        ok &= sound(&mut b) == Some(6) && ids(&mut b) == before.iter().rev().copied().collect::<Vec<_>>();
        list_reverse(&mut b);
        ok &= ids(&mut b) == before;
        let next = list_remove(list_begin(&mut b));
        ok &= next == list_begin(&mut b) && sound(&mut b) == Some(5);
        list_pop_back(&mut b);
        list_push_front(&mut b, &mut nodes[2].elem); // node 2 was removed above
        ok &= sound(&mut b) == Some(5) && list_size(&mut b) == 5;
        check(ok, "list: reverse involution and invariants after remove/pop/push");
    }
}

fn stdio_tests() {
    use crate::stdio::snprintf;
    use core::ffi::c_char;

    let bad = crate::selftest_printf::first_mismatch();
    check(bad.is_none(), "printf matches the C implementation on all fixtures");
    kprintln!("(rust-selftest) printf fixtures: {}", crate::selftest_printf::COUNT);

    // Truncation: at most size-1 chars plus NUL, return value is the untruncated length.
    let mut small = [0xffu8; 8];
    // SAFETY: format and arguments match; the buffer size is passed.
    let written = unsafe { snprintf(small.as_mut_ptr().cast::<c_char>(), 8, c"%s".as_ptr(), c"truncated-string".as_ptr()) };
    check(written == 16 && &small == b"truncat\0", "snprintf truncation");
    let mut one = [0xffu8; 2];
    // SAFETY: as above.
    let written = unsafe { snprintf(one.as_mut_ptr().cast::<c_char>(), 1, c"%d".as_ptr(), 12345i32) };
    check(written == 5 && one == [0, 0xff], "snprintf size 1");
    // SAFETY: size 0 must not write at all.
    let written = unsafe { snprintf(one.as_mut_ptr().cast::<c_char>(), 0, c"%d".as_ptr(), 1i32) };
    check(written == 1 && one == [0, 0xff], "snprintf size 0");
}

fn stdlib_tests() {
    use crate::stdlib::*;
    use core::ffi::{c_int, c_void};

    // SAFETY: all strings are NUL-terminated literals.
    let ok = unsafe {
        atoi(c"42".as_ptr()) == 42
            && atoi(c"  \t-17x".as_ptr()) == -17
            && atoi(c"+5".as_ptr()) == 5
            && atoi(c"".as_ptr()) == 0
            && atoi(c"-2147483648".as_ptr()) == i32::MIN
            && atoi(c"2147483647".as_ptr()) == i32::MAX
    };
    check(ok, "atoi");

    extern "C" fn cmp(a: *const c_void, b: *const c_void) -> c_int {
        // SAFETY: only called on i32 elements below.
        let (a, b) = unsafe { (*a.cast::<i32>(), *b.cast::<i32>()) };
        a.cmp(&b) as c_int
    }
    let mut seed = 99u32;
    let mut all_ok = true;
    for cnt in [0usize, 1, 2, 3, 10, 257] {
        let mut v: Vec<i32> = (0..cnt)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 16) % 50) as i32 - 25
            })
            .collect();
        let mut want = v.clone();
        want.sort_unstable(); // (stable `sort` keeps a 4 KB scratch buffer on the stack)
        // SAFETY: v holds `cnt` i32s and cmp reads i32s.
        unsafe { qsort(v.as_mut_ptr().cast(), cnt, 4, Some(cmp)) };
        all_ok &= v == want;
        for key in -26..=26i32 {
            // SAFETY: as above.
            let hit = unsafe { bsearch((&key as *const i32).cast(), v.as_ptr().cast(), cnt, 4, Some(cmp)) };
            all_ok &= if want.contains(&key) {
                // SAFETY: a non-null result points into `v`.
                !hit.is_null() && unsafe { *hit.cast::<i32>() } == key
            } else {
                hit.is_null()
            };
        }
    }
    check(all_ok, "qsort + bsearch match std (6 sizes, with duplicates)");

    // Elements wider than a word, to exercise the swap path.
    let mut recs: Vec<[u32; 3]> = (0..40u32).rev().map(|i| [i, i * 7, !i]).collect();
    extern "C" fn cmp_rec(a: *const c_void, b: *const c_void, _aux: *mut c_void) -> c_int {
        // SAFETY: elements are [u32; 3].
        let (a, b) = unsafe { (*a.cast::<[u32; 3]>(), *b.cast::<[u32; 3]>()) };
        a[0].cmp(&b[0]) as c_int
    }
    // SAFETY: recs holds 40 records of 12 bytes.
    unsafe { sort(recs.as_mut_ptr().cast(), 40, 12, Some(cmp_rec), core::ptr::null_mut()) };
    check(recs.iter().enumerate().all(|(i, r)| *r == [i as u32, i as u32 * 7, !(i as u32)]), "sort with aux, 12-byte records");
}

fn random_tests() {
    use crate::random::Rc4;
    // Expected bytes come from running the original lib/random.c with random_init(12345).
    let mut generator = Rc4::new();
    generator.seed(12345);
    let mut b = [0u8; 20];
    generator.fill(&mut b);
    let want = [0x05, 0x36, 0x74, 0xc5, 0xbc, 0x6b, 0x84, 0xee, 0x36, 0xc6, 0x8a, 0xb8, 0xe6, 0x15, 0xa4, 0x0d];
    check(b[..16] == want, "PRNG byte stream matches the C implementation");
    check(u32::from_le_bytes(b[16..20].try_into().unwrap()) == 3025151672, "PRNG ulong matches the C implementation");
}
