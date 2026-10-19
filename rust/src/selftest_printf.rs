//! Differential fixtures for lib/stdio.rs.  GENERATED: each expectation is the output of
//! the original C `lib/stdio.c` (compiled on the host) for the same format and arguments.
use core::ffi::{c_char, c_void};

use crate::stdio::snprintf;

pub const COUNT: usize = 59;

/// Runs every fixture; returns the index of the first mismatch.
pub fn first_mismatch() -> Option<usize> {
    let mut buffer = [0u8; 512];
    let buffer_ptr = buffer.as_mut_ptr().cast::<c_char>();
    let mut fixture_index = 0;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"hello".as_ptr()) };
    if written != 5 || &buffer[..written as usize] != b"hello" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"100%%".as_ptr()) };
    if written != 4 || &buffer[..written as usize] != b"100%" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%d".as_ptr(), 0i32) };
    if written != 1 || &buffer[..written as usize] != b"0" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%d".as_ptr(), -2147483648i32) };
    if written != 11 || &buffer[..written as usize] != b"-2147483648" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%d".as_ptr(), 2147483647i32) };
    if written != 10 || &buffer[..written as usize] != b"2147483647" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%5d|%-5d|%05d".as_ptr(), 42i32, 42i32, 42i32) };
    if written != 17 || &buffer[..written as usize] != b"   42|42   |00042" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%+d % d %+d".as_ptr(), 5i32, 5i32, -5i32) };
    if written != 8 || &buffer[..written as usize] != b"+5  5 -5" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"% d".as_ptr(), -3i32) };
    if written != 2 || &buffer[..written as usize] != b"-3" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%i".as_ptr(), -17i32) };
    if written != 3 || &buffer[..written as usize] != b"-17" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%u".as_ptr(), 4294967295u32) };
    if written != 10 || &buffer[..written as usize] != b"4294967295" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%x %X %o".as_ptr(), 255u32, 255u32, 255u32) };
    if written != 9 || &buffer[..written as usize] != b"ff FF 377" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%#x %#X %#o".as_ptr(), 255u32, 255u32, 255u32) };
    if written != 14 || &buffer[..written as usize] != b"0xff 0XFF 0377" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%#x %#o %#X".as_ptr(), 0u32, 0u32, 0u32) };
    if written != 5 || &buffer[..written as usize] != b"0 0 0" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%#.0o|%.0d|%.0u".as_ptr(), 0u32, 0i32, 0u32) };
    if written != 3 || &buffer[..written as usize] != b"0||" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%.5d|%8.3d|%-8.3d|".as_ptr(), 42i32, 42i32, 42i32) };
    if written != 24 || &buffer[..written as usize] != b"00042|     042|042     |" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%08.3d".as_ptr(), 42i32) };
    if written != 8 || &buffer[..written as usize] != b"     042" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%'d %'u".as_ptr(), 1234567i32, 1000u32) };
    if written != 15 || &buffer[..written as usize] != b"1,234,567 1,000" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%'x".as_ptr(), 305419896u32) };
    if written != 9 || &buffer[..written as usize] != b"1234,5678" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%'d".as_ptr(), -123456789i32) };
    if written != 12 || &buffer[..written as usize] != b"-123,456,789" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%*d|%-*d|%.*d".as_ptr(), 6i32, 7i32, 6i32, 7i32, 4i32, 7i32) };
    if written != 18 || &buffer[..written as usize] != b"     7|7     |0007" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%*d".as_ptr(), -6i32, 7i32) };
    if written != 6 || &buffer[..written as usize] != b"7     " || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%.*d".as_ptr(), -3i32, 7i32) };
    if written != 1 || &buffer[..written as usize] != b"7" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%hhd %hd %hhu %hu".as_ptr(), 300i32, 70000i32, 300u32, 70000u32) };
    if written != 15 || &buffer[..written as usize] != b"44 4464 44 4464" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%hhx".as_ptr(), 511u32) };
    if written != 2 || &buffer[..written as usize] != b"ff" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%lld %llu".as_ptr(), -9223372036854775808i64, 18446744073709551615u64) };
    if written != 41 || &buffer[..written as usize] != b"-9223372036854775808 18446744073709551615" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%llx".as_ptr(), 1311768467463790320u64) };
    if written != 16 || &buffer[..written as usize] != b"123456789abcdef0" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%jd %ju".as_ptr(), -5000000000i64, 5000000000u64) };
    if written != 22 || &buffer[..written as usize] != b"-5000000000 5000000000" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%ld %lu %lx".as_ptr(), -1i32, 4000000000u32, 11259375u32) };
    if written != 20 || &buffer[..written as usize] != b"-1 4000000000 abcdef" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%zd %zu %zx".as_ptr(), -1i32, 4000000000u32, 255u32) };
    if written != 16 || &buffer[..written as usize] != b"-1 4000000000 ff" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%td %tu %tx".as_ptr(), -1i32, -1i32, -1i32) };
    if written != 22 || &buffer[..written as usize] != b"-1 4294967295 ffffffff" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%zu".as_ptr(), 4294967295u32) };
    if written != 10 || &buffer[..written as usize] != b"4294967295" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%'llu".as_ptr(), 1000000000000u64) };
    if written != 17 || &buffer[..written as usize] != b"1,000,000,000,000" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%#llo".as_ptr(), 8u64) };
    if written != 3 || &buffer[..written as usize] != b"010" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%c%c%c".as_ptr(), 104i32, 105i32, 33i32) };
    if written != 3 || &buffer[..written as usize] != b"hi!" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%5c|%-5c|".as_ptr(), 65i32, 66i32) };
    if written != 12 || &buffer[..written as usize] != b"    A|B    |" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%s".as_ptr(), c"abc".as_ptr()) };
    if written != 3 || &buffer[..written as usize] != b"abc" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%s".as_ptr(), core::ptr::null::<c_char>()) };
    if written != 6 || &buffer[..written as usize] != b"(null)" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%10s|%-10s|".as_ptr(), c"hi".as_ptr(), c"hi".as_ptr()) };
    if written != 22 || &buffer[..written as usize] != b"        hi|hi        |" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%.2s|%.0s|%5.1s|".as_ptr(), c"hello".as_ptr(), c"hello".as_ptr(), c"hello".as_ptr()) };
    if written != 10 || &buffer[..written as usize] != b"he||    h|" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%*s".as_ptr(), -6i32, c"ab".as_ptr()) };
    if written != 6 || &buffer[..written as usize] != b"ab    " || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%p".as_ptr(), 0usize as *const c_void) };
    if written != 1 || &buffer[..written as usize] != b"0" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%p".as_ptr(), 0xc0100000usize as *const c_void) };
    if written != 10 || &buffer[..written as usize] != b"0xc0100000" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%10p|%-10p|".as_ptr(), 0x1234usize as *const c_void, 0x1234usize as *const c_void) };
    if written != 22 || &buffer[..written as usize] != b"    0x1234|    0x1234|" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%f".as_ptr(), 0i32) };
    if written != 19 || &buffer[..written as usize] != b"<<no %f in kernel>>" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%n".as_ptr(), 0i32) };
    if written != 19 || &buffer[..written as usize] != b"<<no %n in kernel>>" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%e %g".as_ptr(), 0i32, 0i32) };
    if written != 39 || &buffer[..written as usize] != b"<<no %e in kernel>> <<no %g in kernel>>" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%q".as_ptr(), 0i32) };
    if written != 20 || &buffer[..written as usize] != b"<<no %q conversion>>" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%y!".as_ptr()) };
    if written != 21 || &buffer[..written as usize] != b"<<no %y conversion>>!" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%-+ #0d".as_ptr(), 7i32) };
    if written != 2 || &buffer[..written as usize] != b"+7" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%+x".as_ptr(), 7u32) };
    if written != 1 || &buffer[..written as usize] != b"7" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%- 5d|".as_ptr(), 3i32) };
    if written != 6 || &buffer[..written as usize] != b" 3   |" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%0-5d|".as_ptr(), 3i32) };
    if written != 6 || &buffer[..written as usize] != b"3    |" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%+ d".as_ptr(), 3i32) };
    if written != 2 || &buffer[..written as usize] != b"+3" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%.70d".as_ptr(), 5i32) };
    if written != 63 || &buffer[..written as usize] != b"000000000000000000000000000000000000000000000000000000000000005" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%#.70o".as_ptr(), 5u32) };
    if written != 63 || &buffer[..written as usize] != b"000000000000000000000000000000000000000000000000000000000000005" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%70d".as_ptr(), 5i32) };
    if written != 70 || &buffer[..written as usize] != b"                                                                     5" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%-70d|".as_ptr(), 5i32) };
    if written != 71 || &buffer[..written as usize] != b"5                                                                     |" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"a%db%sc%xd".as_ptr(), 1i32, c"two".as_ptr(), 3u32) };
    if written != 9 || &buffer[..written as usize] != b"a1btwoc3d" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    // SAFETY: arguments match the format; buffer is 512 bytes.
    let written = unsafe { snprintf(buffer_ptr, 512, c"%lu%%".as_ptr(), 5u32) };
    if written != 2 || &buffer[..written as usize] != b"5%" || buffer[written as usize] != 0 {
        return Some(fixture_index);
    }
    fixture_index += 1;
    let _ = fixture_index;
    None
}
