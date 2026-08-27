//! Per-regex character encoding (Oniguruma headline: encoding is a value).
//!
//! This is a value type with the `OnigEncodingType` method set (mbc length,
//! code in/out, case-fold, newline), not a stringly enum. Further encodings
//! add variants behind the same methods.

extern crate alloc;

use super::encoding_cjk as cjk;
use super::error::{Error, ErrorKind};

/// Built-in encodings plus a hook for user encodings (OnigEncodingType).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {
    kind: EncKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncKind {
    Ascii,
    Utf8,
    Utf16Be,
    Utf16Le,
    Utf32Be,
    Utf32Le,
    Iso8859(u8),
    Koi8R,
    Cp1251,
    EucJp,
    EucTw,
    EucKr,
    EucCn,
    Sjis,
    Big5,
    Gb18030,
}

impl Encoding {
    pub const ASCII: Self = Self { kind: EncKind::Ascii };
    pub const UTF8: Self = Self { kind: EncKind::Utf8 };
    pub const UTF16_BE: Self = Self { kind: EncKind::Utf16Be };
    pub const UTF16_LE: Self = Self { kind: EncKind::Utf16Le };
    pub const UTF32_BE: Self = Self { kind: EncKind::Utf32Be };
    pub const UTF32_LE: Self = Self { kind: EncKind::Utf32Le };
    pub const ISO_8859_1: Self = Self { kind: EncKind::Iso8859(1) };
    pub const ISO_8859_2: Self = Self { kind: EncKind::Iso8859(2) };
    pub const ISO_8859_3: Self = Self { kind: EncKind::Iso8859(3) };
    pub const ISO_8859_4: Self = Self { kind: EncKind::Iso8859(4) };
    pub const ISO_8859_5: Self = Self { kind: EncKind::Iso8859(5) };
    pub const ISO_8859_6: Self = Self { kind: EncKind::Iso8859(6) };
    pub const ISO_8859_7: Self = Self { kind: EncKind::Iso8859(7) };
    pub const ISO_8859_8: Self = Self { kind: EncKind::Iso8859(8) };
    pub const ISO_8859_9: Self = Self { kind: EncKind::Iso8859(9) };
    pub const ISO_8859_10: Self = Self { kind: EncKind::Iso8859(10) };
    pub const ISO_8859_11: Self = Self { kind: EncKind::Iso8859(11) };
    pub const ISO_8859_13: Self = Self { kind: EncKind::Iso8859(13) };
    pub const ISO_8859_14: Self = Self { kind: EncKind::Iso8859(14) };
    pub const ISO_8859_15: Self = Self { kind: EncKind::Iso8859(15) };
    pub const ISO_8859_16: Self = Self { kind: EncKind::Iso8859(16) };
    pub const KOI8_R: Self = Self { kind: EncKind::Koi8R };
    pub const CP1251: Self = Self { kind: EncKind::Cp1251 };
    pub const EUC_JP: Self = Self { kind: EncKind::EucJp };
    pub const EUC_TW: Self = Self { kind: EncKind::EucTw };
    pub const EUC_KR: Self = Self { kind: EncKind::EucKr };
    pub const EUC_CN: Self = Self { kind: EncKind::EucCn };
    pub const SJIS: Self = Self { kind: EncKind::Sjis };
    pub const BIG5: Self = Self { kind: EncKind::Big5 };
    pub const GB18030: Self = Self { kind: EncKind::Gb18030 };

    pub fn name(self) -> &'static str {
        match self.kind {
            EncKind::Ascii => "ASCII",
            EncKind::Utf8 => "UTF-8",
            EncKind::Utf16Be => "UTF-16BE",
            EncKind::Utf16Le => "UTF-16LE",
            EncKind::Utf32Be => "UTF-32BE",
            EncKind::Utf32Le => "UTF-32LE",
            EncKind::Iso8859(n) => match n {
                1 => "ISO-8859-1",
                2 => "ISO-8859-2",
                3 => "ISO-8859-3",
                4 => "ISO-8859-4",
                5 => "ISO-8859-5",
                6 => "ISO-8859-6",
                7 => "ISO-8859-7",
                8 => "ISO-8859-8",
                9 => "ISO-8859-9",
                10 => "ISO-8859-10",
                11 => "ISO-8859-11",
                13 => "ISO-8859-13",
                14 => "ISO-8859-14",
                15 => "ISO-8859-15",
                16 => "ISO-8859-16",
                _ => "ISO-8859",
            },
            EncKind::Koi8R => "KOI8-R",
            EncKind::Cp1251 => "CP1251",
            EncKind::EucJp => "EUC-JP",
            EncKind::EucTw => "EUC-TW",
            EncKind::EucKr => "EUC-KR",
            EncKind::EucCn => "EUC-CN",
            EncKind::Sjis => "Shift_JIS",
            EncKind::Big5 => "Big5",
            EncKind::Gb18030 => "GB18030",
        }
    }

    pub fn is_unicode(self) -> bool {
        matches!(
            self.kind,
            EncKind::Utf8
                | EncKind::Utf16Be
                | EncKind::Utf16Le
                | EncKind::Utf32Be
                | EncKind::Utf32Le
        )
    }

    /// True when a byte below 0x80 is always a whole character equal to its
    /// own codepoint, so a decode can be skipped entirely.
    #[inline(always)]
    pub(crate) fn ascii_transparent(self) -> bool {
        matches!(self.kind, EncKind::Ascii | EncKind::Utf8)
    }

    pub fn min_len(self) -> usize {
        match self.kind {
            EncKind::Utf16Be | EncKind::Utf16Le => 2,
            EncKind::Utf32Be | EncKind::Utf32Le => 4,
            _ => 1,
        }
    }

    pub fn max_len(self) -> usize {
        match self.kind {
            EncKind::Utf8 => 4,
            EncKind::Utf16Be | EncKind::Utf16Le => 4,
            EncKind::Utf32Be | EncKind::Utf32Le => 4,
            EncKind::EucJp | EncKind::EucTw => 3,
            EncKind::Gb18030 => 4,
            EncKind::Sjis | EncKind::Big5 | EncKind::EucKr | EncKind::EucCn => 2,
            _ => 1,
        }
    }

    /// Byte length of the character starting at `p[0]`.
    pub fn mbc_len(self, p: &[u8]) -> Result<usize, Error> {
        super::count::tick_mbc_len();
        if p.is_empty() {
            return Err(Error::kind_msg(
                ErrorKind::InvalidEncoding,
                "empty mbc",
            ));
        }
        let n = match self.kind {
            EncKind::Ascii | EncKind::Iso8859(_) | EncKind::Koi8R | EncKind::Cp1251 => 1,
            EncKind::Utf8 => utf8_len(p[0])?,
            EncKind::Utf16Be | EncKind::Utf16Le => utf16_len(self.kind, p)?,
            EncKind::Utf32Be | EncKind::Utf32Le => {
                if p.len() < 4 {
                    return Err(enc_err("truncated UTF-32"));
                }
                4
            }
            EncKind::Sjis => sjis_len(p),
            EncKind::Big5 => big5_len(p),
            EncKind::Gb18030 => gb18030_len(p)?,
            EncKind::EucJp | EncKind::EucTw => euc_jp_len(p),
            EncKind::EucKr | EncKind::EucCn => euc_kr_len(p),
        };
        if n > p.len() {
            return Err(enc_err("truncated character"));
        }
        Ok(n)
    }

    pub fn mbc_to_code(self, p: &[u8]) -> Result<u32, Error> {
        let n = self.mbc_len(p)?;
        self.decode_len(p, n)
    }

    pub(crate) fn decode_len(self, p: &[u8], n: usize) -> Result<u32, Error> {
        super::count::tick_mbc_to_code();
        let b = &p[..n];
        Ok(match self.kind {
            EncKind::Ascii => u32::from(b[0]),
            EncKind::Utf8 => utf8_to_code(b)?,
            EncKind::Utf16Be => utf16_to_code(b, true)?,
            EncKind::Utf16Le => utf16_to_code(b, false)?,
            EncKind::Utf32Be => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            EncKind::Utf32Le => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            EncKind::Iso8859(_) | EncKind::Koi8R | EncKind::Cp1251 => u32::from(b[0]),
            EncKind::Sjis => sjis_to_unicode(b),
            EncKind::Big5 => big5_to_unicode(b),
            EncKind::Gb18030 => gb18030_to_unicode(b)?,
            EncKind::EucJp => eucjp_to_unicode(b),
            EncKind::EucCn => gb18030_to_unicode(b).unwrap_or_else(|_| pack_mbc(b)),
            EncKind::EucTw | EncKind::EucKr => pack_mbc(b),
        })
    }

    pub fn code_to_mbc(self, code: u32, out: &mut [u8]) -> Result<usize, Error> {
        match self.kind {
            EncKind::Utf8 => code_to_utf8(code, out),
            EncKind::Ascii | EncKind::Iso8859(_) | EncKind::Koi8R | EncKind::Cp1251 => {
                if code > 0xff || out.is_empty() {
                    return Err(enc_err("code out of range"));
                }
                out[0] = code as u8;
                Ok(1)
            }
            EncKind::Utf32Be => {
                if out.len() < 4 {
                    return Err(enc_err("short buffer"));
                }
                out[..4].copy_from_slice(&code.to_be_bytes());
                Ok(4)
            }
            EncKind::Utf32Le => {
                if out.len() < 4 {
                    return Err(enc_err("short buffer"));
                }
                out[..4].copy_from_slice(&code.to_le_bytes());
                Ok(4)
            }
            EncKind::Utf16Be => code_to_utf16(code, out, true),
            EncKind::Utf16Le => code_to_utf16(code, out, false),
            EncKind::Sjis => unicode_to_sjis(code, out),
            EncKind::Big5 => unicode_to_big5(code, out),
            EncKind::Gb18030 => unicode_to_gb18030(code, out),
            EncKind::EucJp => unicode_to_eucjp(code, out),
            EncKind::EucCn => unicode_to_euccn(code, out),
            EncKind::EucTw | EncKind::EucKr => unpack_mbc(code, out),
        }
    }

    pub fn is_newline(self, p: &[u8]) -> bool {
        if p.is_empty() {
            return false;
        }
        match self.kind {
            EncKind::Utf16Be => p.len() >= 2 && p[0] == 0 && p[1] == b'\n',
            EncKind::Utf16Le => p.len() >= 2 && p[0] == b'\n' && p[1] == 0,
            EncKind::Utf32Be => p.len() >= 4 && p[0] == 0 && p[1] == 0 && p[2] == 0 && p[3] == b'\n',
            EncKind::Utf32Le => p.len() >= 4 && p[0] == b'\n' && p[1] == 0 && p[2] == 0 && p[3] == 0,
            _ => p[0] == b'\n',
        }
    }

    /// Case-fold one character; writes folded bytes, returns byte length.
    pub fn case_fold(self, ascii_only: bool, p: &[u8], to: &mut [u8]) -> Result<usize, Error> {
        let code = self.mbc_to_code(p)?;
        let folded = if ascii_only || !self.is_unicode() {
            if code < 0x80 {
                fold_ascii(code)
            } else {
                code
            }
        } else {
            fold_unicode(code)
        };
        self.code_to_mbc(folded, to)
    }

    /// Left-adjust `pos` to a character boundary in `hay[start..]`.
    pub fn left_adjust(self, hay: &[u8], start: usize, pos: usize) -> usize {
        if pos <= start || pos > hay.len() {
            return pos.min(hay.len());
        }
        match self.kind {
            EncKind::Utf8 => {
                let mut i = pos;
                while i > start && hay[i] & 0xc0 == 0x80 {
                    i -= 1;
                }
                i
            }
            EncKind::Utf16Be | EncKind::Utf16Le => start + ((pos - start) & !1),
            EncKind::Utf32Be | EncKind::Utf32Le => start + ((pos - start) & !3),
            _ => {
                let mut i = start;
                let mut last = start;
                while i < pos {
                    last = i;
                    match self.mbc_len(&hay[i..]) {
                        Ok(n) if n > 0 => i += n,
                        _ => return pos,
                    }
                }
                if i == pos {
                    pos
                } else {
                    last
                }
            }
        }
    }

    pub fn prev_char_start(self, hay: &[u8], start: usize, pos: usize) -> Option<usize> {
        if pos <= start {
            return None;
        }
        let adj = self.left_adjust(hay, start, pos.saturating_sub(1));
        if adj < pos {
            Some(adj)
        } else {
            None
        }
    }
}

fn enc_err(msg: &str) -> Error {
    Error::kind_msg(ErrorKind::InvalidEncoding, msg)
}

fn utf8_len(b0: u8) -> Result<usize, Error> {
    match b0 {
        0x00..=0x7f => Ok(1),
        0xc2..=0xdf => Ok(2),
        0xe0..=0xef => Ok(3),
        0xf0..=0xf4 => Ok(4),
        _ => Err(enc_err("invalid UTF-8 lead")),
    }
}

fn utf8_to_code(b: &[u8]) -> Result<u32, Error> {
    if let [c] = b {
        if *c < 0x80 {
            return Ok(u32::from(*c));
        }
    }
    super::count::tick_utf8_str();
    core::str::from_utf8(b)
        .map_err(|_| enc_err("invalid UTF-8"))?
        .chars()
        .next()
        .map(|c| c as u32)
        .ok_or_else(|| enc_err("empty UTF-8"))
}

fn code_to_utf8(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    let c = char::from_u32(code).ok_or_else(|| enc_err("invalid code point"))?;
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    if out.len() < s.len() {
        return Err(enc_err("short buffer"));
    }
    out[..s.len()].copy_from_slice(s.as_bytes());
    Ok(s.len())
}

fn utf16_len(kind: EncKind, p: &[u8]) -> Result<usize, Error> {
    if p.len() < 2 {
        return Err(enc_err("truncated UTF-16"));
    }
    let w = match kind {
        EncKind::Utf16Be => u16::from_be_bytes([p[0], p[1]]),
        _ => u16::from_le_bytes([p[0], p[1]]),
    };
    if (0xd800..=0xdbff).contains(&w) {
        if p.len() < 4 {
            return Err(enc_err("truncated surrogate pair"));
        }
        Ok(4)
    } else {
        Ok(2)
    }
}

fn utf16_to_code(b: &[u8], be: bool) -> Result<u32, Error> {
    let read = |i| {
        if be {
            u16::from_be_bytes([b[i], b[i + 1]])
        } else {
            u16::from_le_bytes([b[i], b[i + 1]])
        }
    };
    let w = read(0);
    if (0xd800..=0xdbff).contains(&w) && b.len() >= 4 {
        let w2 = read(2);
        if (0xdc00..=0xdfff).contains(&w2) {
            let hi = u32::from(w - 0xd800);
            let lo = u32::from(w2 - 0xdc00);
            return Ok(0x10000 + (hi << 10) + lo);
        }
    }
    Ok(u32::from(w))
}

fn code_to_utf16(code: u32, out: &mut [u8], be: bool) -> Result<usize, Error> {
    let put = |out: &mut [u8], w: u16| {
        let bytes = if be { w.to_be_bytes() } else { w.to_le_bytes() };
        out[0] = bytes[0];
        out[1] = bytes[1];
    };
    if code < 0x10000 {
        if out.len() < 2 {
            return Err(enc_err("short buffer"));
        }
        put(out, code as u16);
        Ok(2)
    } else {
        if out.len() < 4 {
            return Err(enc_err("short buffer"));
        }
        let v = code - 0x10000;
        put(out, (0xd800 + (v >> 10)) as u16);
        put(&mut out[2..], (0xdc00 + (v & 0x3ff)) as u16);
        Ok(4)
    }
}

fn sjis_len(p: &[u8]) -> usize {
    match p[0] {
        0x81..=0x9f | 0xe0..=0xfc if p.len() >= 2 => 2,
        _ => 1,
    }
}

fn big5_len(p: &[u8]) -> usize {
    match p[0] {
        0xa1..=0xfe if p.len() >= 2 && matches!(p[1], 0x40..=0x7e | 0xa1..=0xfe) => 2,
        _ => 1,
    }
}

fn gb18030_len(p: &[u8]) -> Result<usize, Error> {
    let b0 = p[0];
    if b0 < 0x80 {
        return Ok(1);
    }
    if !(0x81..=0xfe).contains(&b0) {
        return Err(enc_err("invalid GB18030"));
    }
    if p.len() < 2 {
        return Err(enc_err("truncated GB18030"));
    }
    let b1 = p[1];
    if (0x30..=0x39).contains(&b1) {
        if p.len() < 4 {
            return Err(enc_err("truncated GB18030"));
        }
        Ok(4)
    } else {
        Ok(2)
    }
}

fn euc_jp_len(p: &[u8]) -> usize {
    match p[0] {
        0x8e if p.len() >= 2 => 2,
        0x8f if p.len() >= 3 => 3,
        0xa1..=0xfe if p.len() >= 2 => 2,
        _ => 1,
    }
}

fn euc_kr_len(p: &[u8]) -> usize {
    match p[0] {
        0xa1..=0xfe if p.len() >= 2 => 2,
        _ => 1,
    }
}

fn pack_mbc(b: &[u8]) -> u32 {
    let mut v = 0u32;
    for &x in b {
        v = (v << 8) | u32::from(x);
    }
    v
}

fn unpack_mbc(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    let n = if code <= 0xff {
        1
    } else if code <= 0xffff {
        2
    } else if code <= 0xff_ffff {
        3
    } else {
        4
    };
    if out.len() < n {
        return Err(enc_err("short buffer"));
    }
    for i in 0..n {
        out[n - 1 - i] = (code >> (8 * i)) as u8;
    }
    Ok(n)
}

fn sjis_to_unicode(b: &[u8]) -> u32 {
    if b.len() == 1 {
        let x = b[0];
        if x < 0x80 {
            return u32::from(x);
        }
        if (0xa1..=0xdf).contains(&x) {
            return 0xff61 + u32::from(x - 0xa1);
        }
        return u32::from(x);
    }
    let mbc = pack_mbc(b);
    cjk::lookup_dec(cjk::SJIS_DEC, mbc).unwrap_or(mbc)
}

fn unicode_to_sjis(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    if code < 0x80 {
        return unpack_mbc(code, out);
    }
    if (0xff61..=0xff9f).contains(&code) {
        if out.is_empty() {
            return Err(enc_err("short buffer"));
        }
        out[0] = (0xa1 + (code - 0xff61)) as u8;
        return Ok(1);
    }
    if let Some(mbc) = cjk::lookup_enc(cjk::SJIS_ENC, code) {
        return unpack_mbc(mbc, out);
    }
    unpack_mbc(code, out)
}

fn eucjp_to_unicode(b: &[u8]) -> u32 {
    if b.len() == 1 {
        return u32::from(b[0]);
    }
    if b[0] == 0x8e && b.len() >= 2 && (0xa1..=0xdf).contains(&b[1]) {
        return 0xff61 + u32::from(b[1] - 0xa1);
    }
    let mbc = pack_mbc(b);
    cjk::lookup_dec(cjk::EUCJP_DEC, mbc).unwrap_or(mbc)
}

fn unicode_to_eucjp(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    if code < 0x80 {
        return unpack_mbc(code, out);
    }
    if (0xff61..=0xff9f).contains(&code) {
        if out.len() < 2 {
            return Err(enc_err("short buffer"));
        }
        out[0] = 0x8e;
        out[1] = (0xa1 + (code - 0xff61)) as u8;
        return Ok(2);
    }
    if let Some(mbc) = cjk::lookup_enc(cjk::EUCJP_ENC, code) {
        return unpack_mbc(mbc, out);
    }
    unpack_mbc(code, out)
}

fn big5_to_unicode(b: &[u8]) -> u32 {
    if b.len() == 1 {
        return u32::from(b[0]);
    }
    let mbc = pack_mbc(b);
    cjk::lookup_dec(cjk::BIG5_DEC, mbc).unwrap_or(mbc)
}

fn unicode_to_big5(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    if code < 0x80 {
        return unpack_mbc(code, out);
    }
    if let Some(mbc) = cjk::lookup_enc(cjk::BIG5_ENC, code) {
        return unpack_mbc(mbc, out);
    }
    unpack_mbc(code, out)
}

fn gb18030_to_unicode(b: &[u8]) -> Result<u32, Error> {
    if b.len() == 1 {
        return Ok(u32::from(b[0]));
    }
    if b.len() == 4 {
        let b0 = b[0];
        let b1 = b[1];
        let b2 = b[2];
        let b3 = b[3];
        if !(0x81..=0xfe).contains(&b0)
            || !(0x30..=0x39).contains(&b1)
            || !(0x81..=0xfe).contains(&b2)
            || !(0x30..=0x39).contains(&b3)
        {
            return Ok(pack_mbc(b));
        }
        let pointer = (((u32::from(b0) - 0x81) * 10 + (u32::from(b1) - 0x30)) * 126
            + (u32::from(b2) - 0x81))
            * 10
            + (u32::from(b3) - 0x30);
        return Ok(cjk::gb4_to_cp(pointer).unwrap_or(pack_mbc(b)));
    }
    let mbc = pack_mbc(b);
    Ok(cjk::lookup_dec(cjk::GB_DEC, mbc).unwrap_or(mbc))
}

fn unicode_to_gb18030(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    if code < 0x80 {
        return unpack_mbc(code, out);
    }
    if let Some(mbc) = cjk::lookup_enc(cjk::GB_ENC, code) {
        return unpack_mbc(mbc, out);
    }
    if let Some(pointer) = cjk::cp_to_gb4(code) {
        if out.len() < 4 {
            return Err(enc_err("short buffer"));
        }
        out[0] = ((pointer / (10 * 126 * 10)) + 0x81) as u8;
        let pointer = pointer % (10 * 126 * 10);
        out[1] = ((pointer / (10 * 126)) + 0x30) as u8;
        let pointer = pointer % (10 * 126);
        out[2] = ((pointer / 10) + 0x81) as u8;
        out[3] = ((pointer % 10) + 0x30) as u8;
        return Ok(4);
    }
    unpack_mbc(code, out)
}

fn unicode_to_euccn(code: u32, out: &mut [u8]) -> Result<usize, Error> {
    if code < 0x80 {
        return unpack_mbc(code, out);
    }
    if let Some(mbc) = cjk::lookup_enc(cjk::GB_ENC, code) {
        if mbc <= 0xffff {
            let lead = (mbc >> 8) as u8;
            let trail = mbc as u8;
            if (0xa1..=0xfe).contains(&lead) && (0xa1..=0xfe).contains(&trail) {
                return unpack_mbc(mbc, out);
            }
        }
    }
    unpack_mbc(code, out)
}

fn fold_ascii(code: u32) -> u32 {
    if (b'A' as u32..=b'Z' as u32).contains(&code) {
        code + 32
    } else {
        code
    }
}

fn fold_unicode(code: u32) -> u32 {
    match char::from_u32(code) {
        Some(c) => {
            let mut iter = c.to_lowercase();
            iter.next().map(|x| x as u32).unwrap_or(code)
        }
        None => code,
    }
}
