//! Optional Oniguruma-shaped C ABI (`onig_new` / `regex_t`).
//!
//! Pure Rust `extern "C"` wrappers. No libonig, no C toolchain, no `onig-sys`.
//! Encoding and syntax are integer ids, not C pointers.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_int, c_uint};
use core::ptr;
use core::slice;

use super::{Encoding, MatchParam, Options, Regex, Region, Syntax};

pub const ONIG_NORMAL: c_int = 0;
pub const ONIG_MISMATCH: c_int = -1;
pub const ONIGERR_INVALID_ARGUMENT: c_int = -6;
pub const ONIGERR_MEMORY: c_int = -5;
pub const ONIGERR_COMPILE: c_int = -100;

pub const ONIG_ENCODING_ASCII: c_uint = 1;
pub const ONIG_ENCODING_UTF8: c_uint = 2;
pub const ONIG_ENCODING_UTF16_BE: c_uint = 3;
pub const ONIG_ENCODING_UTF16_LE: c_uint = 4;
pub const ONIG_ENCODING_UTF32_BE: c_uint = 5;
pub const ONIG_ENCODING_UTF32_LE: c_uint = 6;
pub const ONIG_ENCODING_ISO_8859_1: c_uint = 7;
pub const ONIG_ENCODING_EUC_JP: c_uint = 8;
pub const ONIG_ENCODING_SJIS: c_uint = 9;
pub const ONIG_ENCODING_BIG5: c_uint = 10;
pub const ONIG_ENCODING_GB18030: c_uint = 11;
pub const ONIG_ENCODING_KOI8_R: c_uint = 12;
pub const ONIG_ENCODING_CP1251: c_uint = 13;

pub const ONIG_SYNTAX_ONIGURUMA: c_uint = 0;
pub const ONIG_SYNTAX_POSIX_BASIC: c_uint = 1;
pub const ONIG_SYNTAX_POSIX_EXTENDED: c_uint = 2;
pub const ONIG_SYNTAX_PERL: c_uint = 3;
pub const ONIG_SYNTAX_PERL_NG: c_uint = 4;
pub const ONIG_SYNTAX_JAVA: c_uint = 5;
pub const ONIG_SYNTAX_PYTHON: c_uint = 6;
pub const ONIG_SYNTAX_GNU_REGEX: c_uint = 7;
pub const ONIG_SYNTAX_EMACS: c_uint = 8;
pub const ONIG_SYNTAX_GREP: c_uint = 9;
pub const ONIG_SYNTAX_ASIS: c_uint = 10;

/// Oniguruma `OnigErrorInfo` (pattern snapshot on compile failure).
#[repr(C)]
pub struct OnigErrorInfo {
    pub enc: c_uint,
    pub par: *const u8,
    pub par_end: *const u8,
}

/// Oniguruma `OnigRegion` (byte offsets). `history` is Rust-side only.
#[repr(C)]
pub struct OnigRegion {
    pub allocated: c_int,
    pub num_regs: c_int,
    pub beg: *mut c_int,
    pub end: *mut c_int,
}

fn enc_from_id(id: c_uint) -> Encoding {
    match id {
        ONIG_ENCODING_ASCII => Encoding::ASCII,
        ONIG_ENCODING_UTF16_BE => Encoding::UTF16_BE,
        ONIG_ENCODING_UTF16_LE => Encoding::UTF16_LE,
        ONIG_ENCODING_UTF32_BE => Encoding::UTF32_BE,
        ONIG_ENCODING_UTF32_LE => Encoding::UTF32_LE,
        ONIG_ENCODING_ISO_8859_1 => Encoding::ISO_8859_1,
        ONIG_ENCODING_EUC_JP => Encoding::EUC_JP,
        ONIG_ENCODING_SJIS => Encoding::SJIS,
        ONIG_ENCODING_BIG5 => Encoding::BIG5,
        ONIG_ENCODING_GB18030 => Encoding::GB18030,
        ONIG_ENCODING_KOI8_R => Encoding::KOI8_R,
        ONIG_ENCODING_CP1251 => Encoding::CP1251,
        _ => Encoding::UTF8,
    }
}

fn syntax_from_id(id: c_uint) -> Syntax {
    match id {
        ONIG_SYNTAX_POSIX_BASIC => Syntax::posix_basic(),
        ONIG_SYNTAX_POSIX_EXTENDED => Syntax::posix_extended(),
        ONIG_SYNTAX_PERL => Syntax::perl(),
        ONIG_SYNTAX_PERL_NG => Syntax::perl_ng(),
        ONIG_SYNTAX_JAVA => Syntax::java(),
        ONIG_SYNTAX_PYTHON => Syntax::python(),
        ONIG_SYNTAX_GNU_REGEX => Syntax::gnu_regex(),
        ONIG_SYNTAX_EMACS => Syntax::emacs(),
        ONIG_SYNTAX_GREP => Syntax::grep(),
        ONIG_SYNTAX_ASIS => Syntax::ASIS,
        _ => Syntax::ONIGURUMA,
    }
}

unsafe fn ptr_span<'a>(a: *const u8, b: *const u8) -> Option<&'a [u8]> {
    if a.is_null() || b.is_null() {
        return None;
    }
    let n = b.offset_from(a);
    if n < 0 {
        return None;
    }
    Some(slice::from_raw_parts(a, n as usize))
}

unsafe fn fill_region(dst: *mut OnigRegion, m: &Region) {
    if dst.is_null() {
        return;
    }
    onig_region_clear(dst);
    let n = m.captures.len();
    let mut beg = Vec::with_capacity(n);
    let mut end = Vec::with_capacity(n);
    for cap in &m.captures {
        match cap {
            Some(r) => {
                beg.push(r.start as c_int);
                end.push(r.end as c_int);
            }
            None => {
                beg.push(-1);
                end.push(-1);
            }
        }
    }
    let beg = beg.into_boxed_slice();
    let end = end.into_boxed_slice();
    let r = &mut *dst;
    r.num_regs = n as c_int;
    r.allocated = n as c_int;
    r.beg = Box::into_raw(beg) as *mut c_int;
    r.end = Box::into_raw(end) as *mut c_int;
}

/// No-op (`onig_initialize`). Encodings are values, not process-global tables.
#[no_mangle]
pub unsafe extern "C" fn onig_initialize(_encodings: *mut c_uint, _n: c_int) -> c_int {
    ONIG_NORMAL
}

/// No-op (`onig_end`).
#[no_mangle]
pub unsafe extern "C" fn onig_end() -> c_int {
    ONIG_NORMAL
}

/// Compile a pattern into a heap `regex_t` (`Box<Regex>`).
#[no_mangle]
pub unsafe extern "C" fn onig_new(
    reg: *mut *mut Regex,
    pattern: *const u8,
    pattern_end: *const u8,
    option: c_uint,
    enc: c_uint,
    syntax: c_uint,
    einfo: *mut OnigErrorInfo,
) -> c_int {
    if reg.is_null() {
        return ONIGERR_INVALID_ARGUMENT;
    }
    let Some(pat) = ptr_span(pattern, pattern_end) else {
        return ONIGERR_INVALID_ARGUMENT;
    };
    match Regex::new(pat, Options(option), enc_from_id(enc), syntax_from_id(syntax)) {
        Ok(r) => {
            *reg = Box::into_raw(Box::new(r));
            ONIG_NORMAL
        }
        Err(_) => {
            if !einfo.is_null() {
                (*einfo).enc = enc;
                (*einfo).par = pattern;
                (*einfo).par_end = pattern_end;
            }
            ONIGERR_COMPILE
        }
    }
}

/// Free a regex from `onig_new`.
#[no_mangle]
pub unsafe extern "C" fn onig_free(reg: *mut Regex) {
    if !reg.is_null() {
        drop(Box::from_raw(reg));
    }
}

/// Search (`onig_search`). Returns match start offset, `ONIG_MISMATCH`, or an error.
#[no_mangle]
pub unsafe extern "C" fn onig_search(
    reg: *const Regex,
    str: *const u8,
    end: *const u8,
    start: *const u8,
    range: *const u8,
    region: *mut OnigRegion,
    _option: c_uint,
) -> c_int {
    if reg.is_null() {
        return ONIGERR_INVALID_ARGUMENT;
    }
    let Some(hay) = ptr_span(str, end) else {
        return ONIGERR_INVALID_ARGUMENT;
    };
    let st = if start.is_null() {
        0
    } else {
        start.offset_from(str)
    };
    let rg = if range.is_null() {
        hay.len() as isize
    } else {
        range.offset_from(str)
    };
    if st < 0 || rg < 0 {
        return ONIGERR_INVALID_ARGUMENT;
    }
    match (*reg).search_range_param(hay, st as usize, rg as usize, &MatchParam::default()) {
        Ok(Some(m)) => {
            fill_region(region, &m);
            m.range().start as c_int
        }
        Ok(None) => ONIG_MISMATCH,
        Err(_) => ONIGERR_MEMORY,
    }
}

/// Match only at `at` (`onig_match`).
#[no_mangle]
pub unsafe extern "C" fn onig_match(
    reg: *const Regex,
    str: *const u8,
    end: *const u8,
    at: *const u8,
    region: *mut OnigRegion,
    _option: c_uint,
) -> c_int {
    if reg.is_null() {
        return ONIGERR_INVALID_ARGUMENT;
    }
    let Some(hay) = ptr_span(str, end) else {
        return ONIGERR_INVALID_ARGUMENT;
    };
    let pos = if at.is_null() {
        0
    } else {
        at.offset_from(str)
    };
    if pos < 0 {
        return ONIGERR_INVALID_ARGUMENT;
    }
    match (*reg).find_at(hay, pos as usize) {
        Ok(Some(m)) => {
            fill_region(region, &m);
            (m.range().end - m.range().start) as c_int
        }
        Ok(None) => ONIG_MISMATCH,
        Err(_) => ONIGERR_MEMORY,
    }
}

#[no_mangle]
pub unsafe extern "C" fn onig_region_new() -> *mut OnigRegion {
    Box::into_raw(Box::new(OnigRegion {
        allocated: 0,
        num_regs: 0,
        beg: ptr::null_mut(),
        end: ptr::null_mut(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn onig_region_free(region: *mut OnigRegion, free_self: c_int) {
    if region.is_null() {
        return;
    }
    onig_region_clear(region);
    if free_self != 0 {
        drop(Box::from_raw(region));
    }
}

#[no_mangle]
pub unsafe extern "C" fn onig_region_clear(region: *mut OnigRegion) {
    if region.is_null() {
        return;
    }
    let r = &mut *region;
    if !r.beg.is_null() && r.allocated > 0 {
        let n = r.allocated as usize;
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(r.beg, n)));
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(r.end, n)));
    }
    r.beg = ptr::null_mut();
    r.end = ptr::null_mut();
    r.allocated = 0;
    r.num_regs = 0;
}
