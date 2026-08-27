//! Deterministic work counts over the suite's workload.
//!
//! cargo run --release --features count --manifest-path tools/onig-bench/Cargo.toml --example work
//!
//! Counts, not durations: no pinning, no noise floor, and a drop here is a
//! real drop in work rather than a lucky sample.
use thoth::expressions::{count, Encoding, MatchParam, Options, Regex, Syntax};

fn prose(target: usize) -> String {
    let unit = concat!(
        "The quick brown fox jumps over the lazy dog. ",
        "Contact ops@example.com or visit https://example.com/docs?id=4821 for details. ",
        "Server 10.0.14.7 responded in 128 ms with status 200 on 2026-08-27. ",
        "Le renard brun rapide saute par-dessus le chien paresseux. ",
        "\u{4e2d}\u{6587}\u{6d4b}\u{8bd5} \u{03b1}\u{03b2}\u{03b3}. "
    );
    let mut s = String::with_capacity(target + unit.len());
    while s.len() < target {
        s.push_str(unit);
    }
    s
}

fn logs(target: usize) -> String {
    let mut s = String::with_capacity(target + 200);
    let mut i = 0u32;
    while s.len() < target {
        s.push_str(&format!(
            "2026-08-27T06:{:02}:{:02}Z INFO  [worker-{}] req id=8a3f2b{:04x} path=/api/v1/users/{} status={} dur={}ms\n",
            i % 60, (i * 7) % 60, i % 8, i, i % 997,
            if i % 13 == 0 { 500 } else { 200 }, i % 250
        ));
        i += 1;
    }
    s
}

const CASES: &[(&str, &str, u8, bool)] = &[
    ("literal", "fox", 0, true),
    ("digits", "[0-9]+", 0, true),
    ("word run", r"\w+", 0, true),
    ("alt 3-way", "fox|dog|cat", 0, true),
    ("keyword set", "INFO|WARN|ERROR|DEBUG", 1, true),
    ("line start", r"(?m)^2026", 1, true),
    ("date parts", r"(\d{4})-(\d{2})-(\d{2})", 1, true),
    ("kv pairs", r"(\w+)=(\w+)", 1, true),
    ("named groups", r"(?<k>\w+)=(?<v>\w+)", 1, true),
    ("email", r"[\w.]+@[\w.]+\.\w+", 0, true),
    ("url", r"https?://[\w./?=&-]+", 0, true),
    ("ipv4", r"\d+\.\d+\.\d+\.\d+", 0, true),
    ("backref", r"(\w+) \1", 0, false),
    ("lookahead", r"\d+(?= ms)", 0, true),
    ("lookbehind", r"(?<=status=)\d+", 1, true),
    ("atomic group", r"(?>\w+)=", 1, true),
    ("possessive", r"\d++ms", 1, true),
    ("icase literal", "(?i)THE QUICK", 0, true),
    ("icase class", "(?i)[a-z]+ing", 0, true),
];

fn main() {
    let kb: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);
    let corpora = [prose(kb * 1024), logs(kb * 1024)];
    let param = MatchParam::default();
    println!("work counts, {} KB corpus, expr-count={}\n", kb, cfg!(feature = "count"));
    println!(
        "{:<15} {:>9} {:>10} {:>10} {:>9} {:>10} {:>9} {:>9} {:>9} {:>12}",
        "case", "srch_pos", "engine_new", "cap_clone", "region", "vm_steps", "class_test", "req_scan", "req_skip", "work"
    );
    let mut tot_engine = 0u64;
    let mut tot_clone = 0u64;
    let mut tot_region = 0u64;
    let mut tot_bwc = 0u64;
    let mut tot_work = 0u64;
    for (name, pat, corp, all) in CASES {
        let hay = &corpora[*corp as usize];
        let re = match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
            Ok(r) => r,
            Err(e) => {
                println!("{:<15} compile-err {}", name, e);
                continue;
            }
        };
        count::reset();
        let mut at = 0usize;
        loop {
            match re.search_range_param(hay.as_bytes(), at, hay.len(), &param) {
                Ok(Some(m)) => {
                    let r = m.range();
                    if !*all {
                        break;
                    }
                    at = if r.end > r.start { r.end } else { r.end + 1 };
                    if at > hay.len() {
                        break;
                    }
                }
                _ => break,
            }
        }
        let s = count::snapshot();
        tot_engine += s.engine_new;
        tot_clone += s.cap_clone;
        tot_region += s.lit_clone;
        tot_bwc += s.utf8_str;
        tot_work += s.work();
        println!(
            "{:<15} {:>9} {:>10} {:>10} {:>9} {:>10} {:>9} {:>9} {:>9} {:>12}",
            name,
            s.search_pos,
            s.engine_new,
            s.cap_clone,
            s.lit_clone,
            s.vm_steps,
            s.class_test,
            s.req_scan,
            s.req_skip,
            s.work()
        );
    }
    println!(
        "\ntotals  engine_new={} cap_clone={} region_build={} body_writes_scan={} work={}",
        tot_engine, tot_clone, tot_region, tot_bwc, tot_work
    );
    println!(
        "\nheap allocations implied: {} (engine capture vec) + {} (capture clones) + {} (region 2x vec) = {}",
        tot_engine,
        tot_clone,
        tot_region * 2,
        tot_engine + tot_clone + tot_region * 2
    );
}
