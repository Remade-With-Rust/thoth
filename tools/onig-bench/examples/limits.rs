use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};
fn main() {
    let hay = "a".repeat(50_000);
    let re = Regex::new(br"\w+", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    for lim in [0u32, 100, 1_000, 49_999, 50_000, 10_000_000] {
        let mut p = MatchParam::default();
        p.stack_limit = lim;
        let r = match re.search_param(hay.as_bytes(), &p) {
            Ok(Some(m)) => format!("match {:?}", m.range()),
            Ok(None) => "miss".into(),
            Err(e) => format!("Err {:?}", e.kind),
        };
        println!("  stack_limit={lim:<10} (0=unlimited) -> {r}");
    }
}
