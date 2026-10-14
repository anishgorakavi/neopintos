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
