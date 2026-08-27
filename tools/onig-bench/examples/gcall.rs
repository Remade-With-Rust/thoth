use thoth::expressions::{Encoding, Options, Regex, Syntax};
fn caps_ours(pat: &str, hay: &str) -> String {
    match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
        Ok(re) => match re.search(hay.as_bytes()) {
            Ok(Some(m)) => {
                let r = m.range();
                let mut s = format!("{}..{}", r.start, r.end);
                for i in 1..m.captures.len() {
                    match m.get(i) { Some(g) => s.push_str(&format!(" g{i}={}..{}", g.start, g.end)), None => s.push_str(&format!(" g{i}=-")) }
                }
                s
            }
            Ok(None) => "miss".into(),
            Err(e) => format!("Err {:?}", e.kind),
        },
        Err(e) => format!("compile-err {e}"),
    }
}
fn caps_onig(pat: &str, hay: &str) -> String {
    let re = match onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()) { Ok(r) => r, Err(e) => return format!("compile-err {e}") };
    let mut region = onig::Region::new();
    match re.search_with_options(hay, 0, hay.len(), onig::SearchOptions::SEARCH_OPTION_NONE, Some(&mut region)) {
        Some(_) => {
            let (s0, e0) = region.pos(0).unwrap();
            let mut s = format!("{s0}..{e0}");
            for i in 1..region.len() {
                match region.pos(i) { Some((a, b)) => s.push_str(&format!(" g{i}={a}..{b}")), None => s.push_str(&format!(" g{i}=-")) }
            }
            s
        }
        None => "miss".into(),
    }
}
fn main() {
    let cases: &[(&str, &str)] = &[
        (r"(?<a>x\g<a>?y)", "xxyy"),
        (r"(?<a>x\g<a>?y)", "xy"),
        (r"(?<a>x\g<a>?y)", "xxxyyy"),
        (r"(?<a>a\g<a>?)", "aaa"),
        (r"(?<a>(?<b>b)\g<a>?)", "bb"),
        (r"(x)(?<a>y\g<a>?z)", "xyyzz"),
        (r"(?<a>x\g<a>y|q)", "xqy"),
        (r"(a)\g<1>", "aa"),
        (r"((a)\g<2>)", "aa"),
    ];
    let mut bad = 0;
    for (pat, hay) in cases {
        let (a, b) = (caps_ours(pat, hay), caps_onig(pat, hay));
        let ok = a == b;
        if !ok { bad += 1; }
        println!("{} {:<20} {:<8} ours={:<26} onig={}", if ok { "ok  " } else { "DIFF" }, pat, format!("{:?}", hay), a, b);
    }
    println!("\n{bad} differ");
    if bad > 0 { std::process::exit(1); }
}
