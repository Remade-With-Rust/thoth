//! The greedy-repeat abort: was a process kill at ~2 KB. Now must either match
//! or return a graceful Err, at sizes libonig handles.
use std::io::Write;
use std::time::Instant;
use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};

fn main() {
    let pats = ["(?:ab)+", "[a-z]+?x", "[a-z]+x", r"(\w)+", r"\w+", "(a|b)+"];
    let sizes = [1_000usize, 10_000, 100_000, 1_000_000];
    println!("{:<14} {:>10} {:>16} {:>16}", "pattern", "bytes", "ours", "onig");
    for pat in pats {
        for n in sizes {
            let hay = "ab".repeat(n / 2);
            let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
            let t = Instant::now();
            let ours = match re.search_param(hay.as_bytes(), &MatchParam::default()) {
                Ok(Some(m)) => format!("{:?}", m.range()),
                Ok(None) => "miss".into(),
                Err(e) => format!("Err {:?}", e.kind),
            };
            let ore = onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()).unwrap();
            let theirs = match ore.find(&hay) { Some((s, e)) => format!("{s}..{e}"), None => "miss".into() };
            let flag = if ours == theirs || ours.starts_with("Err") { "" } else { "  <-- DIFFERS" };
            println!("{:<14} {:>10} {:>16} {:>16} {:>9.1}ms{}", pat, hay.len(), ours, theirs, t.elapsed().as_secs_f64()*1000.0, flag);
            std::io::stdout().flush().unwrap();
        }
    }
}
