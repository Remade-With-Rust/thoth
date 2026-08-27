//! `thoth::expressions` is a re-export of the standalone `rusty_expressions`
//! crate. The engine's own suite -- harvested Oniguruma vectors, the
//! differential gates against live libonig, the property fuzz -- lives in that
//! crate's repository and runs there.
//!
//! What belongs here is the seam: that every path consumers used before the
//! split still resolves, and that the re-export behaves.

#![cfg(feature = "expressions")]

use thoth::expressions::{
    find_all_str, is_match_str, CalloutResult, Encoding, ErrorKind, MatchParam, Options, RegSet,
    Regex, Region, Syntax, UserProperty,
};

#[test]
fn re_export_paths_still_resolve() {
    // The exact import shape consumers wrote before the split.
    let re = Regex::new("ca+t", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    let m: Region = re.search(b"one cat two").unwrap().expect("match");
    assert_eq!(m.range(), 4..7);

    assert!(is_match_str("ca+t", "caaat").unwrap());
    assert_eq!(find_all_str("ca+t", "one cat two caaat").unwrap().len(), 2);

    let _ = MatchParam::default();
    let _ = CalloutResult::Success;
    let _ = ErrorKind::Mismatch;
    let _ = UserProperty {
        name: String::from("Vowel"),
        ranges: vec![(b'a' as u32, b'a' as u32)],
    };
}

#[test]
fn oniguruma_class_features_reachable_through_the_re_export() {
    let re = |p: &str| Regex::new_str(p, Options::NONE, Syntax::ONIGURUMA).unwrap();
    // The features `regex` refuses -- the reason this engine exists.
    assert!(re(r"(?<n>cat)\k<n>").is_match(b"catcat").unwrap());
    assert!(re(r"(?<=foo)bar").is_match(b"foobar").unwrap());
    assert!(!re(r"(?>a*)a").is_match(b"aaa").unwrap());
    assert_eq!(
        re(r"(a)\g<1>").search(b"aa").unwrap().unwrap().get(1),
        Some(1..2)
    );
    let iso = Regex::new(b"a.b", Options::NONE, Encoding::ISO_8859_1, Syntax::ONIGURUMA).unwrap();
    assert!(iso.is_match(&[b'a', 0xe9, b'b']).unwrap());
    let py = Regex::new_str("(?P<n>foo)", Options::NONE, Syntax::python()).unwrap();
    assert_eq!(py.search(b"foo").unwrap().unwrap().name("n"), Some(0..3));
}

#[test]
fn scan_and_regset_reachable() {
    let re = Regex::new_str(r"\w+", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let hits = thoth::expressions::scan(&re, b"a bb ccc", &MatchParam::default()).unwrap();
    assert_eq!(hits.len(), 3);

    let a = Regex::new_str("cat", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let b = Regex::new_str("dog", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let set = RegSet::new(vec![a, b]).unwrap();
    let (i, r) = set.search(b"a dog", &MatchParam::default()).unwrap().unwrap();
    assert_eq!((i, r.range()), (1, 2..5));
}

#[test]
fn limits_are_errors_not_aborts() {
    let re = Regex::new_str("(a+)+b", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let mut p = MatchParam::default();
    p.retry_limit_in_match = 8;
    assert!(re.search_param(b"aaaaaaaaac", &p).is_err());

    let deep = format!("{}a{}", "(".repeat(5000), ")".repeat(5000));
    let err = Regex::new_str(&deep, Options::NONE, Syntax::ONIGURUMA).unwrap_err();
    assert_eq!(err.kind, ErrorKind::ParseDepthLimit);
}

/// thoth installs `rusty_alloc` as the global allocator under its default
/// feature, and so does `rusty_expressions` under its own. A program may
/// define exactly one, so the dependency must be taken with
/// `default-features = false`. If that regresses this crate stops linking, so
/// the failure is loud rather than subtle -- this test just pins the intent.
#[test]
fn exactly_one_global_allocator() {
    assert_eq!(thoth::rusty_alloc_enabled(), cfg!(feature = "rusty-alloc"));
}
