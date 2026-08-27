//! rusty_expressions vs libonig: the standing performance suite.
//!
//! cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example suite
//!
//! Discipline: ABBA interleaving so drift hits both arms equally, medians not
//! means, a null arm to establish the noise floor, and identical work in both
//! arms (same haystack, same start, same find-all stepping rule). Every case
//! asserts both engines found the same number of matches before it is timed.
use std::hint::black_box;
use std::time::Instant;
use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};

fn med(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn spread(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = v[v.len() / 10];
    let hi = v[v.len() * 9 / 10];
    if lo > 0.0 {
        (hi - lo) / lo * 100.0
    } else {
        0.0
    }
}

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
            i % 60,
            (i * 7) % 60,
            i % 8,
            i,
            i % 997,
            if i % 13 == 0 { 500 } else { 200 },
            i % 250
        ));
        i += 1;
    }
    s
}

struct Case {
    cat: &'static str,
    name: &'static str,
    pat: &'static str,
    corpus: u8,
    all: bool,
}

// corpus: 0 = prose, 1 = logs
const CASES: &[Case] = &[
    Case { cat: "literal",   name: "rare literal",   pat: "zzzqqq",                   corpus: 0, all: false },
    Case { cat: "literal",   name: "common word",    pat: "fox",                      corpus: 0, all: true  },
    Case { cat: "class",     name: "digits",         pat: "[0-9]+",                   corpus: 0, all: true  },
    Case { cat: "class",     name: "word run",       pat: r"\w+",                     corpus: 0, all: true  },
    Case { cat: "class",     name: "no match",       pat: "[#@%^&]+",                 corpus: 1, all: false },
    Case { cat: "unicode",   name: "uppercase prop", pat: r"\p{Lu}+",                 corpus: 0, all: true  },
    Case { cat: "unicode",   name: "greek prop",     pat: r"\p{Greek}+",              corpus: 0, all: true  },
    Case { cat: "alt",       name: "3-way literal",  pat: "fox|dog|cat",              corpus: 0, all: true  },
    Case { cat: "alt",       name: "keyword set",    pat: "INFO|WARN|ERROR|DEBUG",    corpus: 1, all: true  },
    Case { cat: "anchor",    name: "line start",     pat: r"(?m)^2026",               corpus: 1, all: true  },
    Case { cat: "capture",   name: "date parts",     pat: r"(\d{4})-(\d{2})-(\d{2})", corpus: 1, all: true  },
    Case { cat: "capture",   name: "kv pairs",       pat: r"(\w+)=(\w+)",             corpus: 1, all: true  },
    Case { cat: "capture",   name: "named groups",   pat: r"(?<k>\w+)=(?<v>\w+)",     corpus: 1, all: true  },
    Case { cat: "structure", name: "email",          pat: r"[\w.]+@[\w.]+\.\w+",      corpus: 0, all: true  },
    Case { cat: "structure", name: "url",            pat: r"https?://[\w./?=&-]+",    corpus: 0, all: true  },
    Case { cat: "structure", name: "ipv4",           pat: r"\d+\.\d+\.\d+\.\d+",      corpus: 0, all: true  },
    Case { cat: "onig-only", name: "backref",        pat: r"(\w+) \1",                corpus: 0, all: false },
    Case { cat: "onig-only", name: "lookahead",      pat: r"\d+(?= ms)",              corpus: 0, all: true  },
    Case { cat: "onig-only", name: "lookbehind",     pat: r"(?<=status=)\d+",         corpus: 1, all: true  },
    Case { cat: "onig-only", name: "atomic group",   pat: r"(?>\w+)=",                corpus: 1, all: true  },
    Case { cat: "onig-only", name: "possessive",     pat: r"\d++ms",                  corpus: 1, all: true  },
    Case { cat: "icase",     name: "literal icase",  pat: "(?i)THE QUICK",            corpus: 0, all: true  },
    Case { cat: "icase",     name: "class icase",    pat: "(?i)[a-z]+ing",            corpus: 0, all: true  },
];

fn ours_all(re: &Regex, hay: &str, param: &MatchParam, all: bool) -> usize {
    let mut at = 0usize;
    let mut n = 0usize;
    loop {
        match re.search_range_param(hay.as_bytes(), at, hay.len(), param) {
            Ok(Some(m)) => {
                let r = m.range();
                n += 1;
                if !all {
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
    n
}

fn onig_all(re: &onig::Regex, hay: &str, all: bool) -> usize {
    let mut at = 0usize;
    let mut n = 0usize;
    let mut region = onig::Region::new();
    loop {
        region.clear();
        let hit = re.search_with_options(
            hay,
            at,
            hay.len(),
            onig::SearchOptions::SEARCH_OPTION_NONE,
            Some(&mut region),
        );
        match hit {
            Some(_) => {
                let (s, e) = region.pos(0).unwrap();
                n += 1;
                if !all {
                    break;
                }
                at = if e > s { e } else { e + 1 };
                if at > hay.len() {
                    break;
                }
            }
            None => break,
        }
    }
    n
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let getn = |flag: &str, dflt: usize| -> usize {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(dflt)
    };
    let kb = getn("--kb", 256);
    let reps = getn("--reps", 15);
    let bytes = kb * 1024;
    let corpora = [prose(bytes), logs(bytes)];
    let param = MatchParam::default();

    println!(
        "rusty_expressions vs libonig   corpus={} KB   reps={} (ABBA interleaved, medians)\n",
        kb, reps
    );

    // Null arm: identical code in both slots, to expose the noise floor.
    {
        let hay = &corpora[0];
        let re = Regex::new(b"zzzqqq", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
        let mut a = Vec::new();
        let mut b = Vec::new();
        for _ in 0..reps * 2 {
            let t = Instant::now();
            black_box(ours_all(&re, hay, &param, false));
            a.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(ours_all(&re, hay, &param, false));
            b.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let ma = med(&mut a);
        let mb = med(&mut b);
        println!(
            "noise floor (identical code in both arms): {:.1} vs {:.1} us -> arm skew {:+.1}%, p10-p90 spread {:.0}%",
            ma,
            mb,
            (ma / mb - 1.0) * 100.0,
            spread(&mut a)
        );
        println!("   Any ratio inside that skew is a tie, not a result.\n");
    }

    println!(
        "{:<10} {:<16} {:>9} {:>11} {:>11} {:>8} {:>10}  {}",
        "category", "case", "matches", "ours_us", "onig_us", "ratio", "ours", "verdict"
    );
    println!("{}", "-".repeat(94));

    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut ties = 0u32;
    let mut sum_ours = 0.0f64;
    let mut sum_onig = 0.0f64;

    for c in CASES {
        let hay = &corpora[c.corpus as usize];
        let re = match Regex::new(c.pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
            Ok(r) => r,
            Err(e) => {
                println!("{:<10} {:<16} ours compile-err {}", c.cat, c.name, e);
                continue;
            }
        };
        let ore = match onig::Regex::with_options(
            c.pat,
            onig::RegexOptions::REGEX_OPTION_NONE,
            onig::Syntax::oniguruma(),
        ) {
            Ok(r) => r,
            Err(e) => {
                println!("{:<10} {:<16} onig compile-err {}", c.cat, c.name, e);
                continue;
            }
        };
        // Work-count parity: refuse to time arms that are not doing the same job.
        let n_ours = ours_all(&re, hay, &param, c.all);
        let n_onig = onig_all(&ore, hay, c.all);
        if n_ours != n_onig {
            println!(
                "{:<10} {:<16} {:>9}  WORK MISMATCH ours={} onig={} (not timed)",
                c.cat, c.name, "!", n_ours, n_onig
            );
            continue;
        }
        for _ in 0..3 {
            black_box(ours_all(&re, hay, &param, c.all));
            black_box(onig_all(&ore, hay, c.all));
        }
        let mut a = Vec::new();
        let mut b = Vec::new();
        for _ in 0..reps {
            let t = Instant::now();
            black_box(ours_all(&re, hay, &param, c.all));
            a.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(onig_all(&ore, hay, c.all));
            b.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(onig_all(&ore, hay, c.all));
            b.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(ours_all(&re, hay, &param, c.all));
            a.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let ma = med(&mut a);
        let mb = med(&mut b);
        let ratio = ma / mb;
        sum_ours += ma;
        sum_onig += mb;
        let mbps = (hay.len() as f64 / 1_048_576.0) / (ma / 1e6);
        let verdict = if ratio < 0.90 {
            wins += 1;
            "ours faster"
        } else if ratio > 1.11 {
            losses += 1;
            "onig faster"
        } else {
            ties += 1;
            "tie"
        };
        println!(
            "{:<10} {:<16} {:>9} {:>11.1} {:>11.1} {:>8.2} {:>7.0}MB/s  {}",
            c.cat, c.name, n_ours, ma, mb, ratio, mbps, verdict
        );
    }

    println!("{}", "-".repeat(94));
    println!(
        "\nsearch totals: ours {:.0} us vs onig {:.0} us -> ours/onig = {:.2}",
        sum_ours,
        sum_onig,
        sum_ours / sum_onig
    );
    println!(
        "cases: {} ours-faster, {} tie, {} onig-faster",
        wins, ties, losses
    );

    // Compile cost, measured separately: a fresh compile every iteration.
    println!("\ncompile (fresh compile per iteration, median of {}):", reps * 4);
    println!(
        "{:<10} {:<16} {:>11} {:>11} {:>8}",
        "category", "case", "ours_us", "onig_us", "ratio"
    );
    let mut cs_ours = 0.0f64;
    let mut cs_onig = 0.0f64;
    for c in CASES {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for _ in 0..reps * 2 {
            let t = Instant::now();
            black_box(Regex::new(c.pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).ok());
            a.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(onig::Regex::with_options(c.pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()).ok());
            b.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(onig::Regex::with_options(c.pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()).ok());
            b.push(t.elapsed().as_secs_f64() * 1e6);
            let t = Instant::now();
            black_box(Regex::new(c.pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).ok());
            a.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let ma = med(&mut a);
        let mb = med(&mut b);
        cs_ours += ma;
        cs_onig += mb;
        println!("{:<10} {:<16} {:>11.2} {:>11.2} {:>8.2}", c.cat, c.name, ma, mb, ma / mb);
    }
    println!(
        "\ncompile totals: ours {:.0} us vs onig {:.0} us -> ours/onig = {:.2}",
        cs_ours,
        cs_onig,
        cs_ours / cs_onig
    );
}
