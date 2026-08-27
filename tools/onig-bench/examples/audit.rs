//! Quality audit: does the prefiltered search agree with an unfiltered scan,
//! in the contexts the differential fuzz never exercises?
//!
//! cargo run --release --manifest-path tools/onig-bench/Cargo.toml --example audit
//!
//! The oracle here is our own engine with every prefilter bypassed: `find_at`
//! goes straight to a match attempt at one position, so scanning it across the
//! haystack gives the answer `search` must produce. That needs no libonig, so
//! it covers encodings, syntaxes, options and user properties that the
//! UTF-8-only differential gate cannot reach.
use thoth::expressions::{
    Encoding, MatchParam, Options, Regex, Syntax, UserProperty,
};

/// Leftmost match by brute force, with the prefilters bypassed.
fn reference(re: &Regex, hay: &[u8], opts: Options) -> Option<(usize, usize)> {
    let whole = opts.contains(Options::MATCH_WHOLE_STRING);
    let longest = opts.contains(Options::FIND_LONGEST);
    let not_empty = opts.contains(Options::FIND_NOT_EMPTY);
    let mut best: Option<(usize, usize)> = None;
    for at in 0..=hay.len() {
        if whole && at != 0 {
            break;
        }
        let m = match re.find_at(hay, at) {
            Ok(Some(m)) => m.range(),
            _ => continue,
        };
        let cand = (m.start, m.end);
        if whole && cand.1 != hay.len() {
            continue;
        }
        if not_empty && cand.0 == cand.1 {
            continue;
        }
        if longest {
            let better = best
                .map(|b| (cand.1 - cand.0) > (b.1 - b.0))
                .unwrap_or(true);
            if better {
                best = Some(cand);
            }
        } else {
            return Some(cand);
        }
    }
    best
}

struct Case {
    label: &'static str,
    pat: &'static str,
    hay: Vec<u8>,
    enc: Encoding,
    syn: Syntax,
    opts: Options,
    props: Vec<UserProperty>,
}

fn c(label: &'static str, pat: &'static str, hay: &str) -> Case {
    Case {
        label,
        pat,
        hay: hay.as_bytes().to_vec(),
        enc: Encoding::UTF8,
        syn: Syntax::ONIGURUMA,
        opts: Options::NONE,
        props: Vec::new(),
    }
}

fn main() {
    let mut cases: Vec<Case> = Vec::new();

    // --- 1. user-defined properties: analyze() ran before these existed ---
    for (pat, hay) in [
        (r"\p{Vowel}+=", "xaeiou=y"),
        (r"\p{Vowel}+z", "aeiz"),
        (r"a\p{Vowel}+b", "aeb"),
        (r"\p{Vowel}+", "bcaeiou"),
    ] {
        cases.push(Case {
            label: "user-prop",
            pat,
            hay: hay.as_bytes().to_vec(),
            enc: Encoding::UTF8,
            syn: Syntax::ONIGURUMA,
            opts: Options::NONE,
            props: alloc_props(),
        });
    }

    // --- 2. capture spill past the inline buffer (>7 groups) ---
    cases.push(c(
        "caps-spill",
        r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)=",
        "zabcdefghij=q",
    ));
    cases.push(c(
        "caps-spill",
        r"(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)x",
        "abcdefghijklx",
    ));

    // --- 3. option combinations against the new prefilters ---
    for (label, pat, hay, opts) in [
        ("opt-longest", "a+|aa", "aaa", Options::FIND_LONGEST),
        ("opt-longest", r"\w+=|\w+", "ab=c", Options::FIND_LONGEST),
        ("opt-notempty", "a*", "bbba", Options::FIND_NOT_EMPTY),
        ("opt-notempty", r"\w*=", "==", Options::FIND_NOT_EMPTY),
        ("opt-whole", "cat", "cat", Options::MATCH_WHOLE_STRING),
        ("opt-whole", "cat", "xcat", Options::MATCH_WHOLE_STRING),
        ("opt-whole", r"\w+=", "ab=", Options::MATCH_WHOLE_STRING),
        ("opt-notbol", "^a", "a\na", Options::NOTBOL),
        ("opt-notbol", "(?m)^a", "a\na", Options::NOTBOL),
        ("opt-noteol", "a$", "a\na", Options::NOTEOL),
        ("opt-icase", r"[a-z]+ING", "testing", Options::IGNORECASE),
        ("opt-icase", r"\w+=", "AB=", Options::IGNORECASE),
        ("opt-single", "^a", "b\na", Options::SINGLELINE),
        ("opt-multi", ".+", "a\nb", Options::MULTILINE),
    ] {
        cases.push(Case {
            label,
            pat,
            hay: hay.as_bytes().to_vec(),
            enc: Encoding::UTF8,
            syn: Syntax::ONIGURUMA,
            opts,
            props: Vec::new(),
        });
    }

    // --- 4. encodings other than UTF-8 ---
    let latin1: Vec<u8> = vec![b'x', 0xe9, b'=', b'a', 0xe9, b'b'];
    for (label, pat, hay, enc) in [
        ("enc-ascii", r"\w+=", b"ab=c".to_vec(), Encoding::ASCII),
        ("enc-ascii", "^a", b"b\na".to_vec(), Encoding::ASCII),
        ("enc-latin1", r"\w+=", latin1.clone(), Encoding::ISO_8859_1),
        ("enc-latin1", "a.b", latin1.clone(), Encoding::ISO_8859_1),
        ("enc-koi8", r"\w+=", b"ab=c".to_vec(), Encoding::KOI8_R),
        ("enc-cp1251", r"\w+=", b"ab=c".to_vec(), Encoding::CP1251),
        ("enc-sjis", r"\w+=", b"ab=c".to_vec(), Encoding::SJIS),
        ("enc-sjis", "^a", b"b\na".to_vec(), Encoding::SJIS),
        ("enc-big5", r"\w+=", b"ab=c".to_vec(), Encoding::BIG5),
        ("enc-eucjp", r"\w+=", b"ab=c".to_vec(), Encoding::EUC_JP),
        ("enc-gb18030", r"\w+=", b"ab=c".to_vec(), Encoding::GB18030),
    ] {
        cases.push(Case {
            label,
            pat,
            hay,
            enc,
            syn: Syntax::ONIGURUMA,
            opts: Options::NONE,
            props: Vec::new(),
        });
    }

    // --- 5. non-default syntaxes ---
    for (label, pat, hay, syn) in [
        ("syn-perl", r"\w+=", "ab=c", Syntax::perl()),
        ("syn-python", r"(?P<n>\w+)=", "ab=c", Syntax::python()),
        ("syn-java", r"\w+=", "ab=c", Syntax::java()),
        ("syn-posix-ext", "a+=", "aa=", Syntax::posix_extended()),
        ("syn-grep", r"foo\|bar", "bar", Syntax::grep()),
        ("syn-emacs", r"foo\|bar", "bar", Syntax::emacs()),
        ("syn-asis", r"\d", r"\d", Syntax::ASIS),
    ] {
        cases.push(Case {
            label,
            pat,
            hay: hay.as_bytes().to_vec(),
            enc: Encoding::UTF8,
            syn,
            opts: Options::NONE,
            props: Vec::new(),
        });
    }

    // --- 6. anchors and boundaries against the line-start filter ---
    for (pat, hay) in [
        ("(?m)^a", "\na"),
        ("(?m)^a", "a"),
        ("(?m)^a", "b\n"),
        ("(?m)^$", "a\n"),
        ("(?m)^$", "\n"),
        (r"^\w+=", "ab=\ncd="),
        (r"(?m)^\w+=", "x\nab="),
        (r"\Aab", "ab"),
        (r"(?m)^.", "\n\na"),
        (r"^", ""),
        (r"(?m)^", "\n"),
    ] {
        cases.push(c("anchor", pat, hay));
    }

    let mut checked = 0u32;
    let mut bad = 0u32;
    for case in &cases {
        let mut re = match Regex::new(case.pat.as_bytes(), case.opts, case.enc, case.syn) {
            Ok(r) => r,
            Err(e) => {
                println!("SKIP {:<12} {:<28} compile: {e}", case.label, case.pat);
                continue;
            }
        };
        for p in &case.props {
            re.define_user_property(p.clone());
        }
        let got = match re.search_param(&case.hay, &MatchParam::default()) {
            Ok(Some(m)) => Some((m.range().start, m.range().end)),
            Ok(None) => None,
            Err(e) => {
                println!("SKIP {:<12} {:<28} search: {e}", case.label, case.pat);
                continue;
            }
        };
        let want = reference(&re, &case.hay, re.options());
        checked += 1;
        if got != want {
            bad += 1;
            println!(
                "MISMATCH {:<12} {:<28} hay={:?}\n         filtered={got:?} unfiltered={want:?}",
                case.label, case.pat, case.hay
            );
        }
    }

    // --- 7. search_range_param with a non-zero start ---
    let mut range_bad = 0u32;
    for (pat, hay) in [
        (r"\w+=", "ab=cd=ef"),
        ("(?m)^a", "a\na\na"),
        (r"[\w.]+@\w+", "a@b c@d"),
        ("cat", "cat cat cat"),
        (r"\d+\.\d+", "1.2 3.4"),
    ] {
        let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)
            .unwrap();
        for start in 0..=hay.len() {
            let got = re
                .search_range_param(hay.as_bytes(), start, hay.len(), &MatchParam::default())
                .ok()
                .flatten()
                .map(|m| (m.range().start, m.range().end));
            // Brute force from `start`.
            let mut want = None;
            for at in start..=hay.len() {
                if let Ok(Some(m)) = re.find_at(hay.as_bytes(), at) {
                    want = Some((m.range().start, m.range().end));
                    break;
                }
            }
            checked += 1;
            if got != want {
                range_bad += 1;
                if range_bad <= 6 {
                    println!(
                        "MISMATCH range-start {pat:?} hay={hay:?} start={start} filtered={got:?} unfiltered={want:?}"
                    );
                }
            }
        }
    }
    bad += range_bad;

    // --- 8. randomized sweep over the same contexts ---
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }
    let pats = [
        r"\w+=", r"[\w.]+@\w+", r"\d+\.\d+", r"(\w+)=(\w+)", r"(?m)^\w+",
        r"^\d+", r"(?>\w+)=", r"\d++ms", r"[a-z]+ing", r"a.*b", r"\bcat\b",
        r"(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)(\w)=", r"\p{Vowel}+=", r"(?m)^$",
        r"\w+", r"[0-9]+", r"(a|b)+c", r"x(?=\d)", r"(?<=a)b", r"(?~ab)",
    ];
    let encs = [
        ("utf8", Encoding::UTF8),
        ("ascii", Encoding::ASCII),
        ("latin1", Encoding::ISO_8859_1),
        ("sjis", Encoding::SJIS),
        ("eucjp", Encoding::EUC_JP),
        ("cp1251", Encoding::CP1251),
    ];
    let optsets = [
        Options::NONE,
        Options::FIND_LONGEST,
        Options::FIND_NOT_EMPTY,
        Options::IGNORECASE,
        Options::NOTBOL,
        Options::MATCH_WHOLE_STRING,
    ];
    let alphabets = ["ab=c", "a\nb=", "aeiou=z", "0.9 x", "A@b.c", "ing t", "\nab\n"];
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    let mut sweep_bad = 0u32;
    for _ in 0..30000 {
        let pat = pats[rng.below(pats.len())];
        let (ename, enc) = encs[rng.below(encs.len())];
        let opts = optsets[rng.below(optsets.len())];
        let with_props = rng.below(4) == 0;
        let alpha: Vec<u8> = alphabets[rng.below(alphabets.len())].bytes().collect();
        let len = rng.below(18);
        let hay: Vec<u8> = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
        let mut re = match Regex::new(pat.as_bytes(), opts, enc, Syntax::ONIGURUMA) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if with_props {
            for pr in alloc_props() {
                re.define_user_property(pr);
            }
        }
        let got = match re.search_param(&hay, &MatchParam::default()) {
            Ok(v) => v.map(|m| (m.range().start, m.range().end)),
            Err(_) => continue,
        };
        let want = reference(&re, &hay, re.options());
        checked += 1;
        if got != want {
            sweep_bad += 1;
            if sweep_bad <= 8 {
                println!(
                    "MISMATCH sweep enc={ename} props={with_props} {pat:?} hay={hay:?} filtered={got:?} unfiltered={want:?}"
                );
            }
        }
    }
    bad += sweep_bad;

    println!("\naudit: {checked} checks, {bad} mismatches");
    if bad > 0 {
        std::process::exit(1);
    }
}

fn alloc_props() -> Vec<UserProperty> {
    vec![UserProperty {
        name: String::from("Vowel"),
        ranges: vec![
            (b'a' as u32, b'a' as u32),
            (b'e' as u32, b'e' as u32),
            (b'i' as u32, b'i' as u32),
            (b'o' as u32, b'o' as u32),
            (b'u' as u32, b'u' as u32),
        ],
    }]
}
