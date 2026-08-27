use thoth::expressions::{Encoding, Options, Regex, Syntax};
fn main() {
    let cases: &[(&str, &str)] = &[
        (r"[\x{00e9}]", "\u{e9}"), (r"[\x{00e9}]", "e"), (r"[\x{00e9}]", "9"),
        (r"[\x41]", "A"), (r"[\x41-\x43]", "B"), (r"[\x{80}-\x{ff}]", "\u{e9}"),
        (r"[\x{4e2d}]", "\u{4e2d}"), (r"[\p{Lu}]", "A"), (r"[\p{Lu}]", "a"),
        (r"[\p{Greek}]", "\u{394}"), (r"[\P{L}]", "1"), (r"[a\p{Nd}]", "5"),
        (r"[\t\n]", "\t"), (r"[\r]", "\r"), (r"[\e]", "\u{1b}"), (r"[\a]", "\u{7}"),
        (r"[\cA]", "\u{1}"), (r"[\o{101}]", "A"), (r"[\u0041]", "A"),
        (r"[^\x{00e9}]", "e"), (r"[^\x{00e9}]", "\u{e9}"),
    ];
    let mut bad = 0;
    for (pat, hay) in cases {
        let ours = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)
            .and_then(|re| re.search(hay.as_bytes()))
            .map(|o| o.map(|m| m.range()));
        let ore = onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma());
        let theirs = ore.ok().and_then(|r| r.find(hay));
        let a = match &ours { Ok(Some(r)) => format!("{}..{}", r.start, r.end), Ok(None) => "miss".into(), Err(e) => format!("Err {:?}", e.kind) };
        let b = match theirs { Some((s, e)) => format!("{s}..{e}"), None => "miss".into() };
        let ok = a == b;
        if !ok { bad += 1; }
        println!("{} {:<16} {:<8} ours={:<8} onig={}", if ok { "ok  " } else { "DIFF" }, pat, format!("{:?}", hay), a, b);
    }
    println!("\n{bad} differ");
    if bad > 0 { std::process::exit(1); }
}
