#!/usr/bin/env python3
"""Generate ASCII Rust WHATWG CJK encode/decode tables."""
import os
import sys
import urllib.request

INDEXES = {
    "jis0208": "https://encoding.spec.whatwg.org/index-jis0208.txt",
    "jis0212": "https://encoding.spec.whatwg.org/index-jis0212.txt",
    "big5": "https://encoding.spec.whatwg.org/index-big5.txt",
    "gb18030": "https://encoding.spec.whatwg.org/index-gb18030.txt",
    "gb18030-ranges": "https://encoding.spec.whatwg.org/index-gb18030-ranges.txt",
}


def fetch(cache_dir, key, url):
    path = os.path.join(cache_dir, key + ".txt")
    if not os.path.exists(path):
        print("fetching", url)
        urllib.request.urlretrieve(url, path)
    return path


def parse_index(path):
    out = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) < 2:
                continue
            try:
                ptr = int(parts[0].strip())
                cp = int(parts[1].strip(), 16)
            except ValueError:
                continue
            out[ptr] = cp
    return out


def parse_ranges(path):
    rows = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) < 2:
                continue
            try:
                ptr = int(parts[0].strip())
                cp = int(parts[1].strip(), 16)
            except ValueError:
                continue
            rows.append((ptr, cp))
    rows.sort()
    return rows


def sjis_pairs(jis):
    pairs = []
    for lead in range(0x81, 0xFD):
        if not (0x81 <= lead <= 0x9F or 0xE0 <= lead <= 0xFC):
            continue
        lead_offset = 0x81 if lead < 0xA0 else 0xC1
        for trail in range(0x40, 0xFD):
            if trail == 0x7F:
                continue
            offset = 0x40 if trail < 0x7F else 0x41
            pointer = (lead - lead_offset) * 188 + (trail - offset)
            cp = jis.get(pointer)
            if cp is not None:
                pairs.append(((lead << 8) | trail, cp))
    return pairs


def eucjp_pairs(jis0208, jis0212):
    pairs = []
    for lead in range(0xA1, 0xFF):
        for trail in range(0xA1, 0xFF):
            pointer = (lead - 0xA1) * 94 + (trail - 0xA1)
            cp = jis0208.get(pointer)
            if cp is not None:
                pairs.append(((lead << 8) | trail, cp))
    for b1 in range(0xA1, 0xFF):
        for b2 in range(0xA1, 0xFF):
            pointer = (b1 - 0xA1) * 94 + (b2 - 0xA1)
            cp = jis0212.get(pointer)
            if cp is not None:
                mbc = (0x8F << 16) | (b1 << 8) | b2
                pairs.append((mbc, cp))
    return pairs


def big5_pairs(idx):
    pairs = []
    for lead in range(0x81, 0xFF):
        for trail in list(range(0x40, 0x7F)) + list(range(0xA1, 0xFF)):
            offset = 0x40 if trail < 0x7F else 0x62
            pointer = (lead - 0x81) * 157 + (trail - offset)
            cp = idx.get(pointer)
            if cp is not None:
                pairs.append(((lead << 8) | trail, cp))
    return pairs


def gb_pairs(idx):
    pairs = []
    for lead in range(0x81, 0xFF):
        for trail in list(range(0x40, 0x7F)) + list(range(0x80, 0xFF)):
            offset = 0x40 if trail < 0x7F else 0x41
            pointer = (lead - 0x81) * 190 + (trail - offset)
            cp = idx.get(pointer)
            if cp is not None:
                pairs.append(((lead << 8) | trail, cp))
    return pairs


def pairs_for_bin(pairs, key_first):
    if key_first:
        return sorted(set(pairs), key=lambda x: x[0])
    seen = {}
    for mbc, cp in sorted(pairs, key=lambda x: x[0]):
        seen.setdefault(cp, mbc)
    return sorted((cp, mbc) for cp, mbc in seen.items())


def write_bin(path, items):
    with open(path, "wb") as f:
        for a, b in items:
            f.write(a.to_bytes(4, "little"))
            f.write(b.to_bytes(4, "little"))


def main():
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    cache = os.path.join(root, "tools", "whatwg")
    os.makedirs(cache, exist_ok=True)
    expr = os.path.join(root, "src", "expressions")
    dst = os.path.join(expr, "encoding_cjk.rs")
    paths = {k: fetch(cache, k, url) for k, url in INDEXES.items()}
    jis0208 = parse_index(paths["jis0208"])
    jis0212 = parse_index(paths["jis0212"])
    big5 = parse_index(paths["big5"])
    gb = parse_index(paths["gb18030"])
    gb4 = parse_ranges(paths["gb18030-ranges"])

    sjis = sjis_pairs(jis0208)
    euc = eucjp_pairs(jis0208, jis0212)
    b5 = big5_pairs(big5)
    g2 = gb_pairs(gb)

    tables = [
        ("sjis_dec", sjis, True),
        ("sjis_enc", sjis, False),
        ("eucjp_dec", euc, True),
        ("eucjp_enc", euc, False),
        ("big5_dec", b5, True),
        ("big5_enc", b5, False),
        ("gb_dec", g2, True),
        ("gb_enc", g2, False),
    ]
    for name, pairs, dec in tables:
        items = pairs_for_bin(pairs, dec)
        path = os.path.join(expr, "cjk_" + name + ".bin")
        write_bin(path, items)
        print(name, len(items), "->", path)

    gb4_lines = ["#[rustfmt::skip]", "pub static GB4_RANGES: &[(u32, u32)] = &["]
    row = []
    for ptr, cp in gb4:
        row.append(f"(0x{ptr:x}, 0x{cp:04x})")
        if len(row) == 6:
            gb4_lines.append("    " + ", ".join(row) + ",")
            row = []
    if row:
        gb4_lines.append("    " + ", ".join(row) + ",")
    gb4_lines.append("];")

    rs = '''//! WHATWG CJK maps (Shift_JIS, EUC-JP, Big5, GB18030).
//! Generated by tools/gen_cjk_maps.py. Pair tables are packed LE u32 files.

pub static SJIS_DEC: &[u8] = include_bytes!("cjk_sjis_dec.bin");
pub static SJIS_ENC: &[u8] = include_bytes!("cjk_sjis_enc.bin");
pub static EUCJP_DEC: &[u8] = include_bytes!("cjk_eucjp_dec.bin");
pub static EUCJP_ENC: &[u8] = include_bytes!("cjk_eucjp_enc.bin");
pub static BIG5_DEC: &[u8] = include_bytes!("cjk_big5_dec.bin");
pub static BIG5_ENC: &[u8] = include_bytes!("cjk_big5_enc.bin");
pub static GB_DEC: &[u8] = include_bytes!("cjk_gb_dec.bin");
pub static GB_ENC: &[u8] = include_bytes!("cjk_gb_enc.bin");

pub fn lookup_dec(tab: &[u8], mbc: u32) -> Option<u32> {
    lookup_packed(tab, mbc)
}

pub fn lookup_enc(tab: &[u8], cp: u32) -> Option<u32> {
    lookup_packed(tab, cp)
}

fn lookup_packed(tab: &[u8], key: u32) -> Option<u32> {
    let n = tab.len() / 8;
    let mut lo = 0usize;
    let mut hi = n;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let off = mid * 8;
        let k = u32::from_le_bytes([tab[off], tab[off + 1], tab[off + 2], tab[off + 3]]);
        if k < key {
            lo = mid + 1;
        } else if k > key {
            hi = mid;
        } else {
            return Some(u32::from_le_bytes([
                tab[off + 4],
                tab[off + 5],
                tab[off + 6],
                tab[off + 7],
            ]));
        }
    }
    None
}

/// WHATWG gb18030 4-byte pointer -> code point.
pub fn gb4_to_cp(pointer: u32) -> Option<u32> {
    let t = GB4_RANGES;
    let mut lo = 0usize;
    let mut hi = t.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if t[mid].0 <= pointer {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return None;
    }
    let (off, cp) = t[lo - 1];
    Some(cp + (pointer - off))
}

/// WHATWG gb18030 code point -> 4-byte pointer (when not in the 2-byte map).
pub fn cp_to_gb4(cp: u32) -> Option<u32> {
    let t = GB4_RANGES;
    let mut lo = 0usize;
    let mut hi = t.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if t[mid].1 <= cp {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return None;
    }
    let (off, base) = t[lo - 1];
    Some(off + (cp - base))
}

'''
    with open(dst, "w", encoding="ascii", newline="\n") as f:
        f.write(rs)
        f.write("\n".join(gb4_lines))
        f.write("\n")
    print("wrote", dst)


if __name__ == "__main__":
    main()
