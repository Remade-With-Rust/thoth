//! Stable A/B for class-led scans. ABBA-interleaved vs libonig when available.
use std::hint::black_box;
use std::time::Instant;
use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};

fn med(mut v: Vec<u128>) -> u128 { v.sort(); v[v.len() / 2] }

const CASES: &[(&str, &str)] = &[
    ("qqq", "literal (control)"),
    ("q[0-9]", "literal-led class (control)"),
    ("[0-9]+", "class-led"),
    (r"\d", "class-led"),
    (r"\p{Lu}", "class-led"),
    ("[a-y]+", "class-led (no match)"),
    ("[aeiou]", "class-led"),
];

fn main() {
    let hay = "z".repeat(100_000);
    let param = MatchParam::default();
    println!("{:<10} {:<26} {:>12} {:>12} {:>9}", "pattern", "kind", "ours_ns", "onig_ns", "ratio");
    for (pat, kind) in CASES {
        let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
        #[cfg(feature = "oracle")]
        let ore = onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()).unwrap();
        for _ in 0..10 {
            black_box(re.search_param(hay.as_bytes(), &param).ok());
            #[cfg(feature = "oracle")]
            black_box(ore.find(&hay));
        }
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for _ in 0..40 {
            let t = Instant::now();
            black_box(re.search_param(hay.as_bytes(), &param).ok());
            a.push(t.elapsed().as_nanos());
            #[cfg(feature = "oracle")]
            {
                let t = Instant::now();
                black_box(ore.find(&hay));
                b.push(t.elapsed().as_nanos());
                let t = Instant::now();
                black_box(ore.find(&hay));
                b.push(t.elapsed().as_nanos());
            }
            let t = Instant::now();
            black_box(re.search_param(hay.as_bytes(), &param).ok());
            a.push(t.elapsed().as_nanos());
        }
        let av = med(a) as f64;
        let bv = if b.is_empty() { f64::NAN } else { med(b) as f64 };
        println!("{:<10} {:<26} {:>12.0} {:>12.0} {:>9.2}", pat, kind, av, bv, av / bv);
    }
}
