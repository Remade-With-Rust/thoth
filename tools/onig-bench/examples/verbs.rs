//! Derive (*SKIP)/(*FAIL)/(*MISMATCH) semantics from libonig.
use thoth::expressions::{Encoding, Options, Regex, Syntax};
fn main() {
    let cases: &[(&str, &str)] = &[
        ("a(*SKIP)b", "ab"), ("a(*SKIP)b", "ac"), ("a(*SKIP)b", "acab"),
        ("(a|b)(*SKIP)c", "abc"), ("(?:a(*SKIP)x|ab)", "ab"),
        ("(?:ax|a(*SKIP)b)", "ab"), ("x(*SKIP)y|xz", "xz"),
        ("a(*FAIL)b", "ab"), ("a(*FAIL)", "ab"), ("(?:a(*FAIL)|b)", "ab"),
        ("a(*MISMATCH)b", "ab"), ("(?:a(*MISMATCH)|b)", "ab"),
        ("a(*SKIP)", "aa"), ("[ab](*SKIP)c", "abc"), ("(*FAIL)", "a"),
        ("a*(*SKIP)b", "aaac aaab"),
    ];
    let mut bad = 0;
    for (pat, hay) in cases {
        let r = onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma());
        let out = match r {
            Ok(re) => match re.find(hay) { Some((s, e)) => format!("{s}..{e}"), None => "miss".into() },
            Err(e) => format!("compile-err {e}"),
        };
        let ours = match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
            Ok(re) => match re.search(hay.as_bytes()) {
                Ok(Some(m)) => { let r = m.range(); format!("{}..{}", r.start, r.end) }
                Ok(None) => "miss".into(),
                Err(e) => format!("Err {:?}", e.kind),
            },
            Err(e) => format!("compile-err {}", e),
        };
        let ok = ours == out;
        if !ok { bad += 1; }
        println!("{} {:<22} {:<12} ours={:<10} onig={}", if ok { "ok  " } else { "DIFF" }, pat, format!("{:?}", hay), ours, out);
    }
    println!("
{bad} differ");
    if bad > 0 { std::process::exit(1); }
}
