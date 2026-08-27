//! rusty_expressions tests: unit + harvested Oniguruma vectors.

#![cfg(feature = "expressions")]

use std::fs;
use std::path::PathBuf;

use thoth::expressions::{
    find_all_str, is_match_str, Encoding, ErrorKind, MatchParam, Options, Regex, Syntax,
};

#[test]
fn compile_search_basic() {
    let re = Regex::new("ca+t", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    assert!(re.is_match(b"caaat").unwrap());
    let m = re.search(b"one cat two").unwrap().expect("match");
    assert_eq!(m.range(), 4..7);
}

#[test]
fn stress_plus_and_literal_spans() {
    let plus = Regex::new_str("a+", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let hay = "a".repeat(80) + "b";
    let m = plus.search(hay.as_bytes()).unwrap().unwrap();
    assert_eq!(m.range(), 0..80);
    let lit = Regex::new_str("needle", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let hay = format!("{}needle{}", "x".repeat(400), "y".repeat(400));
    let m = lit.search(hay.as_bytes()).unwrap().unwrap();
    assert_eq!(m.range(), 400..406);
}

#[test]
fn numbered_captures() {
    let re = Regex::new("(ca)+t", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    let m = re.search(b"cacat").unwrap().unwrap();
    assert_eq!(m.range(), 0..5);
    assert_eq!(m.get(1), Some(2..4));
}

#[test]
fn retry_limit_errors() {
    let re = Regex::new("(a+)+b", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    let mut p = MatchParam::default();
    p.retry_limit_in_match = 8;
    let r = re.search_param(b"aaaaaaaaac", &p);
    assert!(r.is_err(), "hostile match must hit retry limit, got {r:?}");
}

#[test]
fn ops_are_callable() {
    assert!(is_match_str("ca+t", "caaat").unwrap());
    let hits = find_all_str("ca+t", "one cat two caaat").unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0], 4..7);
}

#[test]
fn utf8_multibyte() {
    let re = Regex::new_str("a.b", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let hay = "a\u{00e9}b";
    let m = re.search(hay.as_bytes()).unwrap().unwrap();
    assert_eq!(m.range(), 0..hay.len());
}

#[test]
fn lookaround_and_named() {
    let re = Regex::new_str(r"(?<=foo)bar", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert!(re.is_match(b"foobar").unwrap());
    assert!(!re.is_match(b"bar").unwrap());
    let re = Regex::new_str(r"(?<n>ca+)t", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let m = re.search(b"caaat").unwrap().unwrap();
    assert_eq!(m.name("n"), Some(0..4));
}

#[test]
fn possessive_and_atomic() {
    let re = Regex::new_str(r"a*+a", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert!(!re.is_match(b"aaa").unwrap());
    let re = Regex::new_str(r"(?>a*)a", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert!(!re.is_match(b"aaa").unwrap());
}

#[test]
fn backref_and_call() {
    let re = Regex::new_str(r"(cat|dog)\1", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert!(re.is_match(b"catcat").unwrap());
    assert!(!re.is_match(b"catdog").unwrap());
}

#[test]
fn scan_and_regset() {
    let re = Regex::new_str(r"\w+", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let hits = thoth::expressions::scan(&re, b"a bb ccc", &MatchParam::default()).unwrap();
    assert_eq!(hits.len(), 3);
    let a = Regex::new_str("cat", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let b = Regex::new_str("dog", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let set = thoth::expressions::RegSet::new(vec![a, b]).unwrap();
    let (i, r) = set.search(b"a dog", &MatchParam::default()).unwrap().unwrap();
    assert_eq!(i, 1);
    assert_eq!(r.range(), 2..5);
}

#[test]
fn encodings_utf16_and_latin1() {
    let re = Regex::new(b"a.b", Options::NONE, Encoding::ASCII, Syntax::ONIGURUMA).unwrap();
    assert!(re.is_match(b"axb").unwrap());
    let mut buf = [0u8; 8];
    buf[0] = 0;
    buf[1] = b'a';
    buf[2] = 0;
    buf[3] = b'x';
    buf[4] = 0;
    buf[5] = b'b';
    let re = Regex::new(
        &[0, b'a', 0, b'.', 0, b'b'],
        Options::NONE,
        Encoding::UTF16_BE,
        Syntax::ONIGURUMA,
    )
    .unwrap();
    assert!(re.is_match(&buf[..6]).unwrap());
}

#[test]
fn syntax_python_named() {
    let re = Regex::new_str("(?P<n>foo)", Options::NONE, Syntax::python()).unwrap();
    let m = re.search(b"foo").unwrap().unwrap();
    assert_eq!(m.name("n"), Some(0..3));
}

#[test]
fn callout_skip_and_user_prop() {
    // (*SKIP) succeeds going forward; it only redirects where the SEARCH
    // resumes if the whole attempt at this start fails. libonig matches 0..2.
    let re = Regex::new_str(r"a(*SKIP)b", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re.search(b"ab").unwrap().unwrap().range(), 0..2);
    // On failure the retry happens AT the skip point, not past it.
    let re = Regex::new_str(r"[ab](*SKIP)c", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re.search(b"abc").unwrap().unwrap().range(), 1..3);
    // A sibling alternative at the same start is still tried.
    let re = Regex::new_str(r"(?:a(*SKIP)x|ab)", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re.search(b"ab").unwrap().unwrap().range(), 0..2);
    // (*FAIL) / (*MISMATCH) are built in and fail where they stand.
    let re = Regex::new_str(r"a(*FAIL)b", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert!(!re.is_match(b"ab").unwrap());
    let re = Regex::new_str(r"(?:a(*MISMATCH)|b)", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re.search(b"ab").unwrap().unwrap().range(), 1..2);
    let mut param = MatchParam::default();
    param.named_callout = Some(|ctx| {
        if ctx.name == "FAIL" {
            thoth::expressions::CalloutResult::Fail
        } else {
            thoth::expressions::CalloutResult::Success
        }
    });
    let re = Regex::new_str(r"a(*FAIL)b|ab", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let m = re.search_param(b"ab", &param).unwrap();
    assert_eq!(m.unwrap().range(), 0..2);
    let mut re = Regex::new_str(r"\p{Vowel}", Options::NONE, Syntax::ONIGURUMA).unwrap();
    re.define_user_property(thoth::expressions::UserProperty {
        name: String::from("Vowel"),
        ranges: vec![(b'a' as u32, b'a' as u32), (b'e' as u32, b'e' as u32)],
    });
    assert!(re.is_match(b"a").unwrap());
    assert!(!re.is_match(b"b").unwrap());
}

#[test]
fn expressions_demo_op() {
    let hay = "one cat two caaat";
    assert!(is_match_str("ca+t", hay).unwrap());
    let hits = find_all_str("ca+t", hay).unwrap();
    let formatted = thoth::expressions::format_matches(hay, &hits);
    assert!(formatted.contains("4..7 cat"));
    assert!(formatted.contains("12..17 caaat"));
}

#[test]
fn phase2_oniguruma_utf8() {
    let re = |p: &str| Regex::new_str(p, Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re("foo(?=bar)").search(b"foobar").unwrap().unwrap().range(), 0..3);
    assert!(re("foo(?=bar)").search(b"foo").unwrap().is_none());
    assert_eq!(re("foo(?!bar)").search(b"foox").unwrap().unwrap().range(), 0..3);
    assert_eq!(re("(?<=foo)bar").search(b"foobar").unwrap().unwrap().range(), 3..6);
    assert!(re("(?<!foo)bar").search(b"xbar").unwrap().is_some());
    assert!(!re("(?<!foo)bar").is_match(b"foobar").unwrap());
    assert!(!re("a*+a").is_match(b"aaa").unwrap());
    assert!(!re("(?>a*)a").is_match(b"aaa").unwrap());
    let m = re("(?<n>ca+)t").search(b"caaat").unwrap().unwrap();
    assert_eq!(m.name("n"), Some(0..4));
    assert!(re(r"(?<n>cat)\k<n>").is_match(b"catcat").unwrap());
    assert!(!re(r"(?<n>cat)\k<n>").is_match(b"catdog").unwrap());
    assert!(re(r"(a)\g<1>").is_match(b"aa").unwrap());
    assert_eq!(re("(?~abc)").search(b"xxabc").unwrap().unwrap().range(), 0..2);
    assert_eq!(re("(a)?(?(1)a|b)").search(b"aa").unwrap().unwrap().range(), 0..2);
    assert_eq!(re("(a)?(?(1)a|b)").search(b"b").unwrap().unwrap().range(), 0..1);
    assert!(re(r"\p{L}+").is_match(b"Ab").unwrap());
    assert!(re(r"\p{Lu}").is_match(b"A").unwrap());
    assert!(!re(r"\p{Lu}").is_match(b"a").unwrap());
    assert!(re(r"\p{Latin}+").is_match("cafe".as_bytes()).unwrap());
    assert!(re(r"[[:digit:]]+").is_match(b"42").unwrap());
    let re_i = Regex::new_str("foo", Options::IGNORECASE, Syntax::ONIGURUMA).unwrap();
    assert!(re_i.is_match(b"FOO").unwrap());
    let re_i = Regex::new_str("(?i)foo", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert!(re_i.is_match(b"FoO").unwrap());
    assert!(re(r"a\Kb").search(b"ab").unwrap().unwrap().range() == (1..2));
    assert!(re(r"\d+\D+\d+").is_match(b"12x34").unwrap());
    assert_eq!(re(r"\R").search(b"a\r\nb").unwrap().unwrap().range(), 1..3);
    assert!(re(r"\h+").is_match(b"aF").unwrap());
    assert!(!re(r"\h+").is_match(b"G").unwrap());
    assert!(re(r"\X").is_match("e\u{0301}".as_bytes()).unwrap());
    let re_l = Regex::new_str("a+|aa", Options::FIND_LONGEST, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re_l.search(b"aaa").unwrap().unwrap().range(), 0..3);
    let re_w = Regex::new_str("cat", Options::MATCH_WHOLE_STRING, Syntax::ONIGURUMA).unwrap();
    assert!(re_w.search(b"cat").unwrap().is_some());
    assert!(re_w.search(b"xcat").unwrap().is_none());
    assert!(re(r"a{2,4}").is_match(b"aaa").unwrap());
    assert_eq!(re("xa{3}?").search(b"x").unwrap().unwrap().range(), 0..1);
    assert!(re(r"\Gcat").find_at(b"xcat", 1).unwrap().is_some());
    assert!(re(r"\Gcat").search(b"xcat").unwrap().is_none());
    assert!(re(r"\O").is_match(b"\n").unwrap());
    assert!(!re(r"\N").is_match(b"\n").unwrap());
    let re_ne = Regex::new_str("a*", Options::FIND_NOT_EMPTY, Syntax::ONIGURUMA).unwrap();
    assert!(re_ne.search(b"bbb").unwrap().is_none());
    let re_dc = Regex::new_str("(a)", Options::DONT_CAPTURE_GROUP, Syntax::ONIGURUMA).unwrap();
    assert_eq!(re_dc.capture_count(), 1);
}

#[test]
fn phase3_dialects_encodings_scan_set() {
    let asis = Regex::new_str(r"\d", Options::NONE, Syntax::ASIS).unwrap();
    assert!(asis.is_match(br"\d").unwrap());
    assert!(!asis.is_match(b"4").unwrap());
    let py = Regex::new_str("(?P<n>foo)", Options::NONE, Syntax::python()).unwrap();
    assert_eq!(py.search(b"foo").unwrap().unwrap().name("n"), Some(0..3));
    let perl = Regex::new_str(r"(?i)FOO", Options::NONE, Syntax::perl()).unwrap();
    assert!(perl.is_match(b"foo").unwrap());
    let java = Regex::new_str(r"\u0041", Options::NONE, Syntax::java()).unwrap();
    assert!(java.is_match(b"A").unwrap());
    let posix = Regex::new_str("a+", Options::NONE, Syntax::posix_extended()).unwrap();
    assert!(posix.is_match(b"aaa").unwrap());
    let gnu = Regex::new_str(r"\<cat\>", Options::NONE, Syntax::gnu_regex()).unwrap();
    assert!(gnu.is_match(b"a cat b").unwrap());
    let emacs = Regex::new_str(r"foo\|bar", Options::NONE, Syntax::emacs()).unwrap();
    assert!(emacs.is_match(b"bar").unwrap());
    let grep = Regex::new_str(r"foo\|bar", Options::NONE, Syntax::grep()).unwrap();
    assert!(grep.is_match(b"bar").unwrap());
    let perl_ng = Regex::new_str("(?<n>foo)", Options::NONE, Syntax::perl_ng()).unwrap();
    assert_eq!(perl_ng.search(b"foo").unwrap().unwrap().name("n"), Some(0..3));
    let posix_b = Regex::new_str(r"a\{1,3\}", Options::NONE, Syntax::posix_basic()).unwrap();
    assert!(posix_b.is_match(b"aa").unwrap());
    let sql = Regex::new_str("a%c", Options::NONE, thoth::expressions::sql_syntax()).unwrap();
    assert!(sql.is_match(b"axxxc").unwrap());
    let iso = Regex::new(b"a.b", Options::NONE, Encoding::ISO_8859_1, Syntax::ONIGURUMA).unwrap();
    assert!(iso.is_match(&[b'a', 0xe9, b'b']).unwrap());
    let u32be = Regex::new(
        &[0, 0, 0, b'a', 0, 0, 0, b'.', 0, 0, 0, b'b'],
        Options::NONE,
        Encoding::UTF32_BE,
        Syntax::ONIGURUMA,
    )
    .unwrap();
    assert!(u32be
        .is_match(&[0, 0, 0, b'a', 0, 0, 0, b'x', 0, 0, 0, b'b'])
        .unwrap());
    let sjis = Regex::new(b"ab", Options::NONE, Encoding::SJIS, Syntax::ONIGURUMA).unwrap();
    assert!(sjis.is_match(b"ab").unwrap());
    assert_eq!(Encoding::EUC_JP.name(), "EUC-JP");
    assert_eq!(Encoding::GB18030.name(), "GB18030");
    assert_eq!(Encoding::BIG5.name(), "Big5");
    assert_eq!(Encoding::KOI8_R.name(), "KOI8-R");
    assert_eq!(Encoding::CP1251.name(), "CP1251");
    let mut syn = Syntax::ONIGURUMA;
    syn.set_meta_char(1, b'_' as u32);
    assert_eq!(syn.meta_anychar, b'_' as u32);
}

type Vector = (String, String, Option<(usize, usize)>, Vec<Option<(usize, usize)>>);

fn load_vectors(name: &str) -> Vec<Vector> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/oniguruma")
        .join(name);
    let raw = fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {name}"));
    parse_json_vectors(&raw)
}

fn load_phase0() -> Vec<Vector> {
    load_vectors("phase0.json")
}

fn parse_json_vectors(raw: &str) -> Vec<Vector> {
    let mut out = Vec::new();
    for obj in json_objects(raw) {
        let id_pat = field(&obj, "pattern");
        let hay = field(&obj, "hay");
        if id_pat.is_empty() {
            continue;
        }
        if obj.contains("\"mismatch\": true") {
            out.push((id_pat, hay, None, Vec::new()));
        } else {
            let start = num_field(&obj, "start").unwrap_or(0);
            let end = num_field(&obj, "end").unwrap_or(0);
            out.push((id_pat, hay, Some((start, end)), captures_field(&obj)));
        }
    }
    out
}

fn json_objects(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0;
        let mut in_str = false;
        let mut esc = false;
        while i < bytes.len() {
            let b = bytes[i];
            if in_str {
                if esc {
                    esc = false;
                } else if b == b'\\' {
                    esc = true;
                } else if b == b'"' {
                    in_str = false;
                }
            } else if b == b'"' {
                in_str = true;
            } else if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth -= 1;
                if depth == 0 {
                    out.push(raw[start..=i].to_string());
                    i += 1;
                    break;
                }
            }
            i += 1;
        }
    }
    out
}

fn field(obj: &str, key: &str) -> String {
    let needle = format!("\"{key}\": \"");
    if let Some(i) = obj.find(&needle) {
        let s = &obj[i + needle.len()..];
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(n) = chars.next() {
                    match n {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        // \uXXXX: without this an escaped codepoint silently
                        // decayed to the literal text "uXXXX".
                        'u' => {
                            let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                            match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                                Some(ch) => out.push(ch),
                                None => panic!("bad unicode escape in fixture: {hex:?}"),
                            }
                        }
                        other => out.push(other),
                    }
                }
            } else if c == '"' {
                break;
            } else {
                out.push(c);
            }
        }
        return out;
    }
    String::new()
}

/// Parse `"captures": [[a, b], null, ...]` into group 1.. ranges.
fn captures_field(obj: &str) -> Vec<Option<(usize, usize)>> {
    let needle = "\"captures\": [";
    let Some(i) = obj.find(needle) else {
        return Vec::new();
    };
    let rest = &obj[i + needle.len()..];
    let end = match rest.find(']') {
        Some(_) => {
            // find the matching close of the outer array
            let bytes = rest.as_bytes();
            let mut depth = 1i32;
            let mut k = 0usize;
            while k < bytes.len() {
                match bytes[k] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            k
        }
        None => return Vec::new(),
    };
    let body = &rest[..end];
    let mut out = Vec::new();
    let mut i = 0usize;
    let b = body.as_bytes();
    while i < b.len() {
        match b[i] {
            b'[' => {
                let close = match body[i..].find(']') {
                    Some(c) => i + c,
                    None => break,
                };
                let nums: Vec<usize> = body[i + 1..close]
                    .split(',')
                    .filter_map(|t| t.trim().parse::<usize>().ok())
                    .collect();
                out.push(if nums.len() == 2 {
                    Some((nums[0], nums[1]))
                } else {
                    None
                });
                i = close + 1;
            }
            b'n' => {
                out.push(None);
                i += 4;
            }
            _ => i += 1,
        }
    }
    out
}

fn num_field(obj: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\": ");
    let i = obj.find(&needle)?;
    let s = &obj[i + needle.len()..];
    s.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

#[test]
fn harvested_phase0_vectors() {
    let vecs = load_phase0();
    assert!(!vecs.is_empty(), "expected harvested vectors");
    for (pat, hay, expect, caps) in vecs {
        check_vector("phase0.json", &pat, &hay, expect, &caps);
    }
}

/// Compare one harvested vector, captures included.
///
/// Checking only `range()` here is what let a `\g<>` capture bug sit green in
/// this suite while the side-by-side bench reported it.
fn check_vector(
    name: &str,
    pat: &str,
    hay: &str,
    expect: Option<(usize, usize)>,
    caps: &[Option<(usize, usize)>],
) {
    let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)
        .unwrap_or_else(|e| panic!("{name} compile {pat:?}: {e}"));
    let got = re
        .search(hay.as_bytes())
        .unwrap_or_else(|e| panic!("{name} search {pat:?}: {e}"));
    match (expect, got) {
        (None, None) => {}
        (Some((s, e)), Some(r)) => {
            assert_eq!(r.range(), s..e, "{name} pattern {pat:?} hay {hay:?}");
            for (k, want) in caps.iter().enumerate() {
                let group = k + 1;
                let have = r.get(group).map(|g| (g.start, g.end));
                assert_eq!(
                    have, *want,
                    "{name} pattern {pat:?} hay {hay:?} group {group}"
                );
            }
        }
        (expect, got) => {
            panic!("{name} pattern {pat:?} hay {hay:?} expect {expect:?} got {got:?}")
        }
    }
}

fn assert_harvested(name: &str) {
    let vecs = load_vectors(name);
    assert!(!vecs.is_empty(), "expected harvested vectors in {name}");
    for (pat, hay, expect, caps) in vecs {
        check_vector(name, &pat, &hay, expect, &caps);
    }
}

#[test]
fn harvested_phase2_vectors() {
    assert_harvested("phase2.json");
}

#[test]
fn harvested_phase3_vectors() {
    assert_harvested("phase3.json");
}

#[test]
fn capture_history_tree() {
    let re = Regex::new_str("(?@(ab|c))+", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let m = re.search(b"abcab").unwrap().expect("match");
    let tree = m.history.as_ref().expect("capture history");
    assert_eq!(tree.group, 0);
    assert_eq!(tree.range, 0..5);
    let mut groups = Vec::new();
    m.traverse_history(|n, d| groups.push((n.group, n.range.clone(), d)));
    assert!(groups.iter().any(|(g, r, d)| *g == 1 && *r == (0..2) && *d == 1));
    assert!(groups.iter().any(|(g, _, d)| *g == 1 && *d >= 1));
    assert!(groups.iter().filter(|(g, _, _)| *g == 1).count() >= 2);
}

#[test]
fn left_recursive_g_is_compile_error() {
    let err = Regex::new_str(r"(?<a>\g<a>|x)", Options::NONE, Syntax::ONIGURUMA).unwrap_err();
    assert_eq!(err.kind, ErrorKind::NeverEndingRecursion);
    let err = Regex::new_str(r"(\g<1>)", Options::NONE, Syntax::ONIGURUMA).unwrap_err();
    assert_eq!(err.kind, ErrorKind::NeverEndingRecursion);
    assert!(Regex::new_str(r"(a)\g<1>", Options::NONE, Syntax::ONIGURUMA).is_ok());
    assert!(Regex::new_str(r"(?<a>x\g<a>)", Options::NONE, Syntax::ONIGURUMA).is_ok());
}

#[test]
fn unicode_16_unassigned_is_cn_not_assigned() {
    let assigned = Regex::new_str(r"^\p{Assigned}$", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let cn = Regex::new_str(r"^\p{Cn}$", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let lu = Regex::new_str(r"^\p{Lu}$", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let unassigned = "\u{0378}";
    assert!(!assigned.is_match(unassigned.as_bytes()).unwrap());
    assert!(cn.is_match(unassigned.as_bytes()).unwrap());
    assert!(assigned.is_match(b"A").unwrap());
    assert!(lu.is_match(b"A").unwrap());
    assert!(!lu.is_match(unassigned.as_bytes()).unwrap());
}

#[test]
fn text_segment_word_mode() {
    let grapheme = Regex::new_str(r"\X", Options::NONE, Syntax::ONIGURUMA).unwrap();
    assert_eq!(grapheme.search(b"hello").unwrap().unwrap().range(), 0..1);
    let word = Regex::new(
        r"\X",
        Options::TEXT_SEGMENT_WORD,
        Encoding::UTF8,
        Syntax::ONIGURUMA,
    )
    .unwrap();
    assert_eq!(word.search(b"hello world").unwrap().unwrap().range(), 0..5);
    let bound = Regex::new(
        r"\y",
        Options::TEXT_SEGMENT_WORD,
        Encoding::UTF8,
        Syntax::ONIGURUMA,
    )
    .unwrap();
    assert_eq!(bound.search(b"hello").unwrap().unwrap().range(), 0..0);
}

#[test]
fn count_callout_persists_on_match_param() {
    let re = Regex::new_str("a(*COUNT)", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let p = MatchParam::default();
    assert!(re.search_param(b"a", &p).unwrap().is_some());
    assert!(re.search_param(b"a", &p).unwrap().is_some());
    assert_eq!(p.count.get(), 2);
    assert!(re.search_param(b"b", &p).unwrap().is_none());
    assert_eq!(p.count.get(), 2);
}

#[test]
fn east_asian_unicode_round_trip() {
    let mut buf = [0u8; 8];
    let sjis_a = [0x82, 0xa0];
    assert_eq!(Encoding::SJIS.mbc_to_code(&sjis_a).unwrap(), 0x3042);
    let n = Encoding::SJIS.code_to_mbc(0x3042, &mut buf).unwrap();
    assert_eq!(&buf[..n], &sjis_a);
    let euc_a = [0xa4, 0xa2];
    assert_eq!(Encoding::EUC_JP.mbc_to_code(&euc_a).unwrap(), 0x3042);
    let n = Encoding::EUC_JP.code_to_mbc(0x3042, &mut buf).unwrap();
    assert_eq!(&buf[..n], &euc_a);
    let big5_zhong = [0xa4, 0xa4];
    assert_eq!(Encoding::BIG5.mbc_to_code(&big5_zhong).unwrap(), 0x4e2d);
    let n = Encoding::BIG5.code_to_mbc(0x4e2d, &mut buf).unwrap();
    assert_eq!(&buf[..n], &big5_zhong);
    let gb_zhong = [0xd6, 0xd0];
    assert_eq!(Encoding::GB18030.mbc_to_code(&gb_zhong).unwrap(), 0x4e2d);
    let n = Encoding::GB18030.code_to_mbc(0x4e2d, &mut buf).unwrap();
    assert_eq!(&buf[..n], &gb_zhong);
    let re = Regex::new(&sjis_a, Options::NONE, Encoding::SJIS, Syntax::ONIGURUMA).unwrap();
    assert!(re.is_match(&sjis_a).unwrap());
}

#[cfg(feature = "compat")]
#[test]
fn onig_c_abi_new_search() {
    use std::ptr;
    use thoth::expressions::compat::*;
    let pat = b"ca+t";
    let mut reg = ptr::null_mut();
    let rc = unsafe {
        onig_new(
            &mut reg,
            pat.as_ptr(),
            pat.as_ptr().add(pat.len()),
            0,
            ONIG_ENCODING_UTF8,
            ONIG_SYNTAX_ONIGURUMA,
            ptr::null_mut(),
        )
    };
    assert_eq!(rc, ONIG_NORMAL);
    let hay = b"one cat two";
    let region = unsafe { onig_region_new() };
    let pos = unsafe {
        onig_search(
            reg,
            hay.as_ptr(),
            hay.as_ptr().add(hay.len()),
            hay.as_ptr(),
            hay.as_ptr().add(hay.len()),
            region,
            0,
        )
    };
    assert_eq!(pos, 4);
    unsafe {
        assert!(!(*region).beg.is_null());
        assert_eq!(*(*region).beg, 4);
        assert_eq!(*(*region).end, 7);
        onig_region_free(region, 1);
        onig_free(reg);
    }
}

#[test]
fn deferred_gaps_are_named() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/oniguruma/deferred.txt");
    let raw = fs::read_to_string(&path).expect("deferred.txt");
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert!(
        lines.is_empty(),
        "named Oniguruma gaps must be closed; leftover {lines:?}"
    );
}
