use thoth::expressions::{Encoding, Options, Regex, Syntax};
fn main() {
    let cases: &[(&str, &str)] = &[
        (r"(?m)^$", "\n"), (r"(?m)^$", "a\n\nb"), (r"(?m)^$", "ab"),
        (r"(?m)^$", ""), (r"(?m)^$", "a\n"), (r"(?m)^$", "\n\n"),
        (r"(?m)$", "a\nb"), (r"(?m)^", "a\nb"), (r"$", "a\n"),
    ];
    let mut bad = 0;
    for (pat, hay) in cases {
        let ours = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)
            .and_then(|re| re.search(hay.as_bytes()))
            .map(|o| o.map(|m| (m.range().start, m.range().end)));
        let a = match &ours { Ok(Some((s,e))) => format!("{s}..{e}"), Ok(None) => "miss".into(), Err(e) => format!("Err {:?}", e.kind) };
        let ore = onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma());
        let b = match ore.ok().and_then(|r| { let mut rg = onig::Region::new(); r.search_with_options(hay, 0, hay.len(), onig::SearchOptions::SEARCH_OPTION_NONE, Some(&mut rg)).and_then(|_| rg.pos(0)) }) {
            Some((s,e)) => format!("{s}..{e}"), None => "miss".into() };
        let ok = a == b; if !ok { bad += 1; }
        println!("{} {:<10} {:<10} ours={:<8} onig={}", if ok {"ok  "} else {"DIFF"}, pat, format!("{:?}", hay), a, b);
    }
    println!("\n{bad} differ");
}
