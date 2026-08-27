//! Side-by-side correctness + search timing: rusty_expressions vs harvested
//! Oniguruma fixtures, and optionally vs C libonig (`--features oracle`).
//!
//! ```sh
//! cargo run --release --manifest-path tools/onig-bench/Cargo.toml
//! cargo run --release --manifest-path tools/onig-bench/Cargo.toml --features oracle
//! ```
//!
//! Not a workspace member. Do not add `onig` to thoth's Cargo.toml.

use std::env;
use std::hint::black_box;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};

#[derive(Clone, Debug)]
struct Vector {
    id: String,
    corpus: String,
    pattern: String,
    hay: String,
    expect: Option<(usize, usize)>,
    captures: Vec<Option<(usize, usize)>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Hit {
    range: (usize, usize),
    caps: Vec<Option<(usize, usize)>>,
}

struct Row {
    id: String,
    corpus: String,
    fixture: String,
    ours: String,
    onig: String,
    compile_ours_ns: u128,
    compile_onig_ns: u128,
    search_ours_ns: u128,
    search_onig_ns: u128,
    ok: bool,
    oracle_skew: bool,
    note: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut iters: u32 = 80;
    let mut warmup: u32 = 8;
    let mut fail_fast = false;
    let mut want_oracle = false;
    let mut unlimited = false;
    let mut count_mode = false;
    let mut corpora: Vec<String> = vec![
        "phase0.json".into(),
        "phase2.json".into(),
        "phase3.json".into(),
    ];
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--iters" => {
                i += 1;
                iters = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(iters);
            }
            "--warmup" => {
                i += 1;
                warmup = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(warmup);
            }
            "--fail-fast" => fail_fast = true,
            "--oracle" => want_oracle = true,
            "--unlimited" => unlimited = true,
            "--count" => count_mode = true,
            "--corpus" => {
                i += 1;
                if let Some(list) = args.get(i) {
                    corpora = list
                        .split(',')
                        .map(|s| {
                            if s.ends_with(".json") {
                                s.to_string()
                            } else {
                                format!("{s}.json")
                            }
                        })
                        .collect();
                }
            }
            "-h" | "--help" => {
                eprint_help();
                return;
            }
            other => {
                eprintln!("unknown arg {other}");
                eprint_help();
                process::exit(2);
            }
        }
        i += 1;
    }

    let oracle = cfg!(feature = "oracle");
    if want_oracle && !oracle {
        eprintln!("--oracle needs: cargo run --features oracle --manifest-path tools/onig-bench/Cargo.toml");
        process::exit(3);
    }
    let use_oracle = oracle;

    let root = find_repo_root();
    let data = root.join("tests/data/oniguruma");
    let mut vectors = Vec::new();
    for name in &corpora {
        let path = data.join(name);
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("read {}: {e}", path.display());
            process::exit(2);
        });
        let stem = name.trim_end_matches(".json");
        vectors.extend(parse_json_vectors(&raw, stem));
    }
    vectors.extend(stress_vectors());

    if count_mode {
        run_count_mode(&vectors, unlimited);
        return;
    }

    let param = if unlimited {
        MatchParam::unlimited()
    } else {
        MatchParam::default()
    };
    let mut rows = Vec::new();
    let mut diffs = 0u32;
    let mut compile_fail = 0u32;
    let mut oracle_skews = 0u32;

    for v in &vectors {
        match run_one(v, &param, use_oracle, warmup, iters) {
            Ok(row) => {
                if !row.ok {
                    diffs += 1;
                    if fail_fast {
                        print_row(&row, use_oracle);
                        eprintln!("fail-fast: {}", row.note);
                        process::exit(1);
                    }
                } else if row.oracle_skew {
                    oracle_skews += 1;
                }
                rows.push(row);
            }
            Err(e) => {
                compile_fail += 1;
                let row = Row {
                    id: v.id.clone(),
                    corpus: v.corpus.clone(),
                    fixture: fmt_expect(v.expect),
                    ours: format!("ERR {e}"),
                    onig: "-".into(),
                    compile_ours_ns: 0,
                    compile_onig_ns: 0,
                    search_ours_ns: 0,
                    search_onig_ns: 0,
                    ok: false,
                    oracle_skew: false,
                    note: e,
                };
                if fail_fast {
                    print_row(&row, use_oracle);
                    process::exit(1);
                }
                rows.push(row);
            }
        }
    }

    println!(
        "onig-bench  vectors={}  iters={iters}  warmup={warmup}  oracle={}  matchparam={}",
        rows.len(),
        if use_oracle { "libonig" } else { "fixture-only" },
        if unlimited { "unlimited" } else { "default" }
    );
    if use_oracle {
        println!(
            "{:<22} {:<10} {:<12} {:<12} {:<12} {:>10} {:>10} {:>8} {}",
            "id", "corpus", "fixture", "ours", "onig", "ours_ns", "onig_ns", "ratio", "status"
        );
    } else {
        println!(
            "{:<22} {:<10} {:<12} {:<12} {:>10} {:>12} {}",
            "id", "corpus", "fixture", "ours", "search_ns", "compile_ns", "status"
        );
    }
    for row in &rows {
        print_row(row, use_oracle);
    }

    let ok = rows.iter().filter(|r| r.ok).count();
    let timed: Vec<&Row> = rows
        .iter()
        .filter(|r| r.ok && r.search_ours_ns > 0)
        .collect();
    let ours_sum: u128 = timed.iter().map(|r| r.search_ours_ns).sum();
    let onig_sum: u128 = timed.iter().map(|r| r.search_onig_ns).sum();
    println!();
    println!(
        "summary  ok={ok}/{}  diffs={diffs}  oracle_skew={oracle_skews}  compile_fail={compile_fail}",
        rows.len()
    );
    if !timed.is_empty() {
        let ours_c: u128 = timed.iter().map(|r| r.compile_ours_ns).sum();
        println!(
            "search ns (sum of per-vector medians, {} ok rows): ours={ours_sum}",
            timed.len()
        );
        println!("compile ns (sum of one-shot compiles): ours={ours_c}");
        if use_oracle && onig_sum > 0 {
            let ratio = ours_sum as f64 / onig_sum as f64;
            let onig_c: u128 = timed.iter().map(|r| r.compile_onig_ns).sum();
            println!("search ns: onig={onig_sum}  ours/onig={ratio:.3}");
            println!("compile ns: onig={onig_c}");
        }
    }
    if diffs > 0 || compile_fail > 0 {
        process::exit(1);
    }
}

fn eprint_help() {
    eprintln!(
        "onig-bench [--iters N] [--warmup N] [--corpus phase0,phase2,phase3] [--fail-fast] [--oracle] [--unlimited] [--count]"
    );
}

fn run_count_mode(vectors: &[Vector], unlimited: bool) {
    let param = if unlimited {
        MatchParam::unlimited()
    } else {
        MatchParam::default()
    };
    println!(
        "onig-bench --count  feature={}  matchparam={}",
        if cfg!(feature = "count") {
            "expr-count"
        } else {
            "OFF (rebuild with --features count)"
        },
        if unlimited { "unlimited" } else { "default" }
    );
    println!(
        "{:<22} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}  hit",
        "id",
        "work",
        "engine",
        "vm",
        "consume",
        "mbc_len",
        "mbc_code",
        "bump",
        "next",
        "clone",
        "spos",
        "utf8",
        "bscan"
    );
    for v in vectors {
        if v.corpus != "stress" {
            continue;
        }
        let re = match Regex::new(
            v.pattern.as_bytes(),
            Options::NONE,
            Encoding::UTF8,
            Syntax::ONIGURUMA,
        ) {
            Ok(r) => r,
            Err(e) => {
                println!("{:<22} compile-fail {e}", v.id);
                continue;
            }
        };
        thoth::expressions::count::reset();
        let hit = re.search_param(v.hay.as_bytes(), &param);
        let st = thoth::expressions::count::snapshot();
        let ok = match &hit {
            Ok(Some(r)) => {
                let rng = r.range();
                v.expect == Some((rng.start, rng.end))
            }
            Ok(None) => v.expect.is_none(),
            Err(_) => false,
        };
        println!(
            "{:<22} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}  {}",
            v.id,
            st.work(),
            st.engine_new,
            st.vm_steps,
            st.consume_char,
            st.mbc_len,
            st.mbc_to_code,
            st.bump_retry,
            st.next_pos,
            st.cap_clone,
            st.search_pos,
            st.utf8_str,
            st.byte_scan,
            if ok { "ok" } else { "DIFF" }
        );
    }
}

fn find_repo_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cand = here.join("../..");
    if cand.join("tests/data/oniguruma").is_dir() {
        return cand.canonicalize().unwrap_or(cand);
    }
    PathBuf::from(".")
}

fn run_one(
    v: &Vector,
    param: &MatchParam,
    use_oracle: bool,
    warmup: u32,
    iters: u32,
) -> Result<Row, String> {
    let t0 = Instant::now();
    let re = Regex::new(
        v.pattern.as_bytes(),
        Options::NONE,
        Encoding::UTF8,
        Syntax::ONIGURUMA,
    )
    .map_err(|e| format!("ours compile: {e}"))?;
    let compile_ours_ns = t0.elapsed().as_nanos();

    let ours_hit = ours_search(&re, &v.hay, param)?;
    let fixture_ok = match_fixture(&ours_hit, v);
    let mut note = String::new();
    if !fixture_ok {
        note = format!(
            "ours {} != fixture {}",
            fmt_hit(&ours_hit),
            fmt_expect_full(v)
        );
    }

    let mut onig_label = String::from("-");
    let mut compile_onig_ns = 0u128;
    let mut search_onig_ns = 0u128;
    let mut oracle_skew = false;

    if use_oracle {
        match oracle_compile_search(v, warmup, iters) {
            Ok((hit, cns, sns)) => {
                compile_onig_ns = cns;
                search_onig_ns = sns;
                onig_label = fmt_hit(&hit);
                let onig_fixture_ok = match_fixture(&hit, v);
                let same = ours_hit.as_ref().map(|h| h.range) == hit.as_ref().map(|h| h.range);
                if fixture_ok && !onig_fixture_ok {
                    oracle_skew = true;
                    if !note.is_empty() {
                        note.push_str("; ");
                    }
                    note.push_str("libonig != harvested fixture");
                } else if !same && onig_fixture_ok {
                    if !note.is_empty() {
                        note.push_str("; ");
                    }
                    note.push_str("ours != libonig (libonig matches fixture)");
                }
            }
            Err(e) => {
                onig_label = format!("ERR {e}");
                oracle_skew = true;
                if !note.is_empty() {
                    note.push_str("; ");
                }
                note.push_str(&format!("onig {e}"));
            }
        }
    }

    let search_ours_ns = time_ours_search(&re, &v.hay, param, warmup, iters);

    Ok(Row {
        id: v.id.clone(),
        corpus: v.corpus.clone(),
        fixture: fmt_expect(v.expect),
        ours: fmt_hit(&ours_hit),
        onig: onig_label,
        compile_ours_ns,
        compile_onig_ns,
        search_ours_ns,
        search_onig_ns,
        ok: fixture_ok,
        oracle_skew,
        note,
    })
}

fn ours_search(re: &Regex, hay: &str, param: &MatchParam) -> Result<Option<Hit>, String> {
    let m = re
        .search_param(hay.as_bytes(), param)
        .map_err(|e| format!("ours search: {e}"))?;
    Ok(m.map(|r| {
        let range = r.range();
        let mut caps = Vec::new();
        for i in 1..r.captures.len() {
            caps.push(r.get(i).map(|x| (x.start, x.end)));
        }
        Hit {
            range: (range.start, range.end),
            caps,
        }
    }))
}

fn time_ours_search(re: &Regex, hay: &str, param: &MatchParam, warmup: u32, iters: u32) -> u128 {
    let hay_b = hay.as_bytes();
    for _ in 0..warmup {
        let _ = black_box(re.search_param(hay_b, param));
    }
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        let _ = black_box(re.search_param(hay_b, param));
        samples.push(t.elapsed());
    }
    median_ns(&samples)
}

fn match_fixture(got: &Option<Hit>, v: &Vector) -> bool {
    match (got, v.expect) {
        (None, None) => true,
        (Some(h), Some((s, e))) => {
            if h.range != (s, e) {
                return false;
            }
            for (i, exp) in v.captures.iter().enumerate() {
                let g = h.caps.get(i).copied().flatten();
                if g != *exp {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn fmt_expect(e: Option<(usize, usize)>) -> String {
    match e {
        None => "miss".into(),
        Some((s, e)) => format!("{s}..{e}"),
    }
}

fn fmt_expect_full(v: &Vector) -> String {
    let mut s = fmt_expect(v.expect);
    if !v.captures.is_empty() {
        s.push_str(" caps=");
        s.push_str(&fmt_caps(&v.captures));
    }
    s
}

fn fmt_caps(caps: &[Option<(usize, usize)>]) -> String {
    let parts: Vec<String> = caps
        .iter()
        .map(|c| match c {
            None => "-".into(),
            Some((a, b)) => format!("{a}..{b}"),
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn fmt_hit(h: &Option<Hit>) -> String {
    match h {
        None => "miss".into(),
        Some(h) => {
            if h.caps.is_empty() {
                format!("{}..{}", h.range.0, h.range.1)
            } else {
                format!(
                    "{}..{} caps={}",
                    h.range.0,
                    h.range.1,
                    fmt_caps(&h.caps)
                )
            }
        }
    }
}

fn print_row(row: &Row, oracle: bool) {
    let status = if !row.ok {
        "DIFF"
    } else if row.oracle_skew {
        "ORACLE"
    } else {
        "ok"
    };
    if oracle {
        let ratio = if row.search_onig_ns > 0 {
            format!("{:.2}", row.search_ours_ns as f64 / row.search_onig_ns as f64)
        } else {
            "-".into()
        };
        println!(
            "{:<22} {:<10} {:<12} {:<12} {:<12} {:>10} {:>10} {:>8} {} {}",
            trunc(&row.id, 22),
            trunc(&row.corpus, 10),
            row.fixture,
            row.ours,
            row.onig,
            row.search_ours_ns,
            row.search_onig_ns,
            ratio,
            status,
            row.note
        );
    } else {
        println!(
            "{:<22} {:<10} {:<12} {:<12} {:>10} {:>12} {} {}",
            trunc(&row.id, 22),
            trunc(&row.corpus, 10),
            row.fixture,
            row.ours,
            row.search_ours_ns,
            row.compile_ours_ns,
            status,
            row.note
        );
    }
}

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}...", &s[..n.saturating_sub(3)])
    }
}

fn median_ns(samples: &[Duration]) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut v: Vec<u128> = samples.iter().map(|d| d.as_nanos()).collect();
    v.sort_unstable();
    v[v.len() / 2]
}

fn stress_vectors() -> Vec<Vector> {
    let long_a = "a".repeat(80) + "b";
    vec![
        Vector {
            id: "stress-long-plus".into(),
            corpus: "stress".into(),
            pattern: "a+".into(),
            hay: long_a,
            expect: Some((0, 80)),
            captures: vec![],
        },
        Vector {
            id: "stress-literal-scan".into(),
            corpus: "stress".into(),
            pattern: "needle".into(),
            hay: format!("{}needle{}", "x".repeat(400), "y".repeat(400)),
            expect: Some((400, 406)),
            captures: vec![],
        },
    ]
}

#[cfg(feature = "oracle")]
fn oracle_compile_search(
    v: &Vector,
    warmup: u32,
    iters: u32,
) -> Result<(Option<Hit>, u128, u128), String> {
    let t0 = Instant::now();
    let re = onig::Regex::with_options(
        &v.pattern,
        onig::RegexOptions::REGEX_OPTION_NONE,
        onig::Syntax::oniguruma(),
    )
    .map_err(|e| format!("compile: {e}"))?;
    let compile_ns = t0.elapsed().as_nanos();
    let hit = oracle_hit(&re, &v.hay);
    for _ in 0..warmup {
        let _ = black_box(oracle_hit(&re, &v.hay));
    }
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        let _ = black_box(oracle_hit(&re, &v.hay));
        samples.push(t.elapsed());
    }
    Ok((hit, compile_ns, median_ns(&samples)))
}

#[cfg(feature = "oracle")]
fn oracle_hit(re: &onig::Regex, hay: &str) -> Option<Hit> {
    let caps = re.captures(hay)?;
    let (s, e) = caps.pos(0)?;
    let mut groups = Vec::new();
    for i in 1..caps.len() {
        groups.push(caps.pos(i));
    }
    Some(Hit {
        range: (s, e),
        caps: groups,
    })
}

#[cfg(not(feature = "oracle"))]
fn oracle_compile_search(
    _v: &Vector,
    _warmup: u32,
    _iters: u32,
) -> Result<(Option<Hit>, u128, u128), String> {
    Err("oracle feature off".into())
}

fn parse_json_vectors(raw: &str, corpus: &str) -> Vec<Vector> {
    let mut out = Vec::new();
    for obj in json_objects(raw) {
        let id = field(&obj, "id");
        let pattern = field(&obj, "pattern");
        let hay = field(&obj, "hay");
        if pattern.is_empty() {
            continue;
        }
        let expect = if obj.contains("\"mismatch\": true") {
            None
        } else {
            Some((
                num_field(&obj, "start").unwrap_or(0),
                num_field(&obj, "end").unwrap_or(0),
            ))
        };
        let captures = parse_captures(&obj);
        out.push(Vector {
            id: if id.is_empty() {
                pattern.clone()
            } else {
                id
            },
            corpus: corpus.to_string(),
            pattern,
            hay,
            expect,
            captures,
        });
    }
    out
}

fn parse_captures(obj: &str) -> Vec<Option<(usize, usize)>> {
    let Some(i) = obj.find("\"captures\":") else {
        return Vec::new();
    };
    let rest = obj[i + 12..].trim_start();
    if !rest.starts_with('[') {
        return Vec::new();
    }
    let mut out = Vec::new();
    let bytes = rest.as_bytes();
    let mut p = 1usize;
    while p < bytes.len() {
        while p < bytes.len()
            && ((bytes[p] as char).is_ascii_whitespace() || bytes[p] == b',')
        {
            p += 1;
        }
        if p >= bytes.len() {
            break;
        }
        if bytes[p] == b']' {
            break;
        }
        if rest[p..].starts_with("null") {
            out.push(None);
            p += 4;
            continue;
        }
        if bytes[p] != b'[' {
            break;
        }
        let inner_end = rest[p..].find(']').unwrap_or(0);
        let inner = &rest[p + 1..p + inner_end];
        let nums: Vec<usize> = inner
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if nums.len() >= 2 {
            out.push(Some((nums[0], nums[1])));
        }
        p += inner_end + 1;
    }
    out
}

fn json_objects(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0;
        let mut in_str = false;
        let mut esc = false;
        while i < bytes.len() {
            let b = bytes[i];
            if in_str {
                if esc {
                    esc = false;
                } else if b == b'\\' {
                    esc = true;
                } else if b == b'"' {
                    in_str = false;
                }
            } else if b == b'"' {
                in_str = true;
            } else if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth -= 1;
                if depth == 0 {
                    out.push(raw[start..=i].to_string());
                    i += 1;
                    break;
                }
            }
            i += 1;
        }
    }
    out
}

fn field(obj: &str, key: &str) -> String {
    let needle = format!("\"{key}\": \"");
    let Some(i) = obj.find(&needle) else {
        return String::new();
    };
    let s = &obj[i + needle.len()..];
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                // \uXXXX in a haystack must become the character.
                Some('u') => {
                    let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                    match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        Some(ch) => out.push(ch),
                        None => {
                            out.push('\\');
                            out.push('u');
                            out.push_str(&hex);
                        }
                    }
                }
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => break,
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

fn num_field(obj: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\": ");
    let i = obj.find(&needle)?;
    let s = &obj[i + needle.len()..];
    s.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}
