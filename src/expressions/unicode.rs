//! Character types and Unicode 16.0-oriented properties (doc/RE).

extern crate alloc;

use super::encoding::Encoding;
use super::syntax::Options;
use super::ucd16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctype {
    Word,
    Digit,
    Space,
    Xdigit,
    Alnum,
    Alpha,
    Ascii,
    Blank,
    Cntrl,
    Graph,
    Lower,
    Print,
    Punct,
    Upper,
    Hiragana,
    Katakana,
    Newline,
    /// Unicode general category coarse: C L M N P S Z
    Gc(u8),
}

pub fn is_ascii_word(cp: u32) -> bool {
    matches!(cp, 0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a | 0x5f)
}

pub fn is_ascii_digit(cp: u32) -> bool {
    (0x30..=0x39).contains(&cp)
}

pub fn is_ascii_space(cp: u32) -> bool {
    matches!(cp, 0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x20)
}

pub fn is_xdigit(cp: u32) -> bool {
    is_ascii_digit(cp) || (0x41..=0x46).contains(&cp) || (0x61..=0x66).contains(&cp)
}

fn ch(cp: u32) -> Option<char> {
    char::from_u32(cp)
}

/// Unicode word: Letter | Mark | Number | Connector_Punctuation (doc/RE).
pub fn is_unicode_word(cp: u32) -> bool {
    if is_ascii_word(cp) {
        return true;
    }
    let n = ucd16::gc_name(ucd16::gc(cp));
    matches!(n.as_bytes().first().copied(), Some(b'L' | b'M' | b'N')) || n == "Pc"
}

fn is_mark(cp: u32) -> bool {
    matches!(ucd16::gc(cp), 6 | 7 | 8)
}

pub fn is_unicode_space(cp: u32) -> bool {
    matches!(
        cp,
        0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x20 | 0x85 | 0xa0 | 0x1680 | 0x2000..=0x200a
            | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000
    )
}

pub fn is_decimal_number(cp: u32) -> bool {
    ucd16::gc(cp) == 9
}

pub fn is_hiragana(cp: u32) -> bool {
    matches!(cp, 0x3041..=0x3096 | 0x3099..=0x309f | 0x1b001..=0x1b11f)
}

pub fn is_katakana(cp: u32) -> bool {
    matches!(cp, 0x30a0..=0x30ff | 0x31f0..=0x31ff | 0xff65..=0xff9f | 0x32d0..=0x32fe)
}

pub fn is_gc(cp: u32, letter: u8) -> bool {
    ucd16::gc_name(ucd16::gc(cp)).as_bytes().first().copied() == Some(letter)
}

pub fn is_word(enc: Encoding, opt: Options, cp: u32) -> bool {
    if opt.contains(Options::WORD_IS_ASCII) || opt.contains(Options::POSIX_IS_ASCII) || !enc.is_unicode()
    {
        if !enc.is_unicode() && cp > 0x7f {
            return true;
        }
        return is_ascii_word(cp);
    }
    is_unicode_word(cp)
}

pub fn is_digit(enc: Encoding, opt: Options, cp: u32) -> bool {
    if opt.contains(Options::DIGIT_IS_ASCII) || opt.contains(Options::POSIX_IS_ASCII) || !enc.is_unicode()
    {
        return is_ascii_digit(cp);
    }
    is_decimal_number(cp)
}

pub fn is_space(enc: Encoding, opt: Options, cp: u32) -> bool {
    if opt.contains(Options::SPACE_IS_ASCII) || opt.contains(Options::POSIX_IS_ASCII) || !enc.is_unicode()
    {
        return is_ascii_space(cp);
    }
    is_unicode_space(cp)
}

pub fn posix(name: &str, enc: Encoding, opt: Options, cp: u32) -> bool {
    let ascii = opt.contains(Options::POSIX_IS_ASCII) || !enc.is_unicode();
    match name {
        "alnum" => {
            if ascii {
                is_ascii_word(cp) && cp != 0x5f
            } else {
                ch(cp).map(|c| c.is_alphabetic()).unwrap_or(false) || is_decimal_number(cp)
            }
        }
        "alpha" => {
            if ascii {
                matches!(cp, 0x41..=0x5a | 0x61..=0x7a)
            } else {
                ch(cp).map(|c| c.is_alphabetic()).unwrap_or(false)
            }
        }
        "ascii" => cp <= 0x7f,
        "blank" => cp == 0x09 || cp == 0x20 || (!ascii && matches!(cp, 0x2000..=0x200a | 0x3000 | 0xa0)),
        "cntrl" => {
            if ascii {
                cp <= 0x1f || cp == 0x7f
            } else {
                cp <= 0x1f || (0x7f..=0x9f).contains(&cp)
            }
        }
        "digit" => is_digit(enc, opt, cp),
        "graph" => {
            if ascii {
                (0x21..=0x7e).contains(&cp)
            } else {
                !is_unicode_space(cp) && !posix("cntrl", enc, opt, cp)
            }
        }
        "lower" => {
            if ascii {
                (0x61..=0x7a).contains(&cp)
            } else {
                ch(cp).map(|c| c.is_lowercase()).unwrap_or(false)
            }
        }
        "print" => posix("graph", enc, opt, cp) || posix("space", enc, opt, cp),
        "punct" => {
            if ascii {
                matches!(cp, 0x21..=0x2f | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e)
            } else {
                is_gc(cp, b'P') || is_gc(cp, b'S')
            }
        }
        "space" => is_space(enc, opt, cp),
        "upper" => {
            if ascii {
                (0x41..=0x5a).contains(&cp)
            } else {
                ch(cp).map(|c| c.is_uppercase()).unwrap_or(false)
            }
        }
        "xdigit" => is_xdigit(cp),
        "word" => is_word(enc, opt, cp),
        "hiragana" => is_hiragana(cp),
        "katakana" => is_katakana(cp),
        _ => false,
    }
}

pub fn property(name: &str, enc: Encoding, opt: Options, cp: u32) -> bool {
    let n = name;
    if n.len() == 1 {
        return is_gc(cp, n.as_bytes()[0]);
    }
    if n.len() == 2 {
        if let Some(hit) = gc_detail(cp, n) {
            return hit;
        }
    }
    match n {
        "Alnum" | "alnum" => posix("alnum", enc, opt, cp),
        "Alpha" | "alpha" => posix("alpha", enc, opt, cp),
        "Blank" | "blank" => posix("blank", enc, opt, cp),
        "Cntrl" | "cntrl" => posix("cntrl", enc, opt, cp),
        "Digit" | "digit" => posix("digit", enc, opt, cp),
        "Graph" | "graph" => posix("graph", enc, opt, cp),
        "Lower" | "lower" => posix("lower", enc, opt, cp),
        "Print" | "print" => posix("print", enc, opt, cp),
        "Punct" | "punct" => posix("punct", enc, opt, cp),
        "Space" | "space" => posix("space", enc, opt, cp),
        "Upper" | "upper" => posix("upper", enc, opt, cp),
        "XDigit" | "xdigit" => posix("xdigit", enc, opt, cp),
        "Word" | "word" => posix("word", enc, opt, cp),
        "ASCII" | "ascii" => posix("ascii", enc, opt, cp),
        "Hiragana" | "hiragana" | "Hira" => is_hiragana(cp),
        "Katakana" | "katakana" | "Kana" => is_katakana(cp),
        "Letter" => is_gc(cp, b'L'),
        "Mark" => is_gc(cp, b'M'),
        "Number" => is_gc(cp, b'N'),
        "Punctuation" => is_gc(cp, b'P'),
        "Symbol" => is_gc(cp, b'S'),
        "Separator" => is_gc(cp, b'Z'),
        "Other" => is_gc(cp, b'C'),
        "Any" => true,
        "Assigned" => ucd16::gc(cp) != 0,
        "Latin" | "Latn" => is_latin(cp),
        "Greek" | "Grek" => matches!(cp, 0x0370..=0x03ff | 0x1f00..=0x1fff),
        "Cyrillic" | "Cyrl" => matches!(cp, 0x0400..=0x04ff | 0x0500..=0x052f | 0x2de0..=0x2dff | 0xa640..=0xa69f),
        "Han" | "Hani" => matches!(cp, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2a6df),
        "Hangul" | "Hang" => matches!(cp, 0x1100..=0x11ff | 0x3130..=0x318f | 0xac00..=0xd7af),
        "Common" | "Zyyy" => !is_latin(cp) && !is_hiragana(cp) && !is_katakana(cp),
        _ => posix(n, enc, opt, cp),
    }
}

/// Unicode 16.0 general-category detail from committed UCD tables.
fn gc_detail(cp: u32, name: &str) -> Option<bool> {
    const GCS: &[&str] = &[
        "Lu", "Ll", "Lt", "Lm", "Lo", "Mn", "Mc", "Me", "Nd", "Nl", "No", "Pc", "Pd", "Ps",
        "Pe", "Pi", "Pf", "Po", "Sm", "Sc", "Sk", "So", "Zs", "Zl", "Zp", "Cc", "Cf", "Cs",
        "Co", "Cn",
    ];
    if GCS.contains(&name) {
        Some(ucd16::gc_eq(cp, name))
    } else {
        None
    }
}

fn is_latin(cp: u32) -> bool {
    matches!(
        cp,
        0x0041..=0x005a
            | 0x0061..=0x007a
            | 0x00c0..=0x00d6
            | 0x00d8..=0x00f6
            | 0x00f8..=0x024f
            | 0x1e00..=0x1eff
            | 0x2c60..=0x2c7f
            | 0xa720..=0xa7ff
            | 0xab30..=0xab6f
    )
}

/// Extended grapheme cluster boundary (UAX #29 subset): break except CR+LF and Hangul.
pub fn grapheme_break(prev: Option<u32>, cur: u32) -> bool {
    match prev {
        None => true,
        Some(0x0d) if cur == 0x0a => false,
        Some(_) if cur == 0x0a || cur == 0x0d => true,
        Some(_) if is_mark(cur) => false,
        Some(p) if (0x1100..=0x11ff).contains(&p) && (0x1100..=0x11ff).contains(&cur) => false,
        _ => true,
    }
}

/// User-defined Unicode property: name -> inclusive ranges.
#[derive(Clone, Debug)]
pub struct UserProperty {
    pub name: alloc::string::String,
    pub ranges: alloc::vec::Vec<(u32, u32)>,
}

impl UserProperty {
    pub fn contains(&self, cp: u32) -> bool {
        self.ranges.iter().any(|&(a, b)| (a..=b).contains(&cp))
    }
}
