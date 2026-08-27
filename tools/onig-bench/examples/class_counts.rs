//! Deterministic work counts for class-led scans over a 100 KB no-match haystack.
//! cargo run --release --features count --example class_counts
use thoth::expressions::{count, Encoding, MatchParam, Options, Regex, Syntax};

fn main() {
    let hay = "z".repeat(100_000);
    println!("feature expr-count = {}", cfg!(feature = "count"));
    println!(
        "{:<10} {:>9} {:>9} {:>9} {:>9} {:>10} {:>10} {:>9} {:>10} {:>12}",
        "pattern", "srch_pos", "eng_new", "vm_steps", "consume", "mbc_len", "mbc_code", "next_pos", "byte_scan", "work"
    );
    for pat in ["qqq", "[0-9]+", r"\d", r"\p{Lu}", "[0-9]", "q[0-9]"] {
        let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
        count::reset();
        let _ = re.search_param(hay.as_bytes(), &MatchParam::default());
        let s = count::snapshot();
        println!(
            "{:<10} {:>9} {:>9} {:>9} {:>9} {:>10} {:>10} {:>9} {:>10} {:>12}",
            pat, s.search_pos, s.engine_new, s.vm_steps, s.consume_char,
            s.mbc_len, s.mbc_to_code, s.next_pos, s.byte_scan, s.work()
        );
    }
}
