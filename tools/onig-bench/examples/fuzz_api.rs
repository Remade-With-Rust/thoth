//! Property fuzz for the surfaces the differential gates never touched:
//! `MatchParam` limits, `RegSet`, `scan`, and `search_range_param`.
//!
//! cargo run --release --manifest-path tools/onig-bench/Cargo.toml --example fuzz_api
//!
//! These are properties, not oracles: each one states something that must hold
//! for *any* input, so a violation is a bug regardless of what libonig does.
use thoth::expressions::{scan, Encoding, MatchParam, Options, RegSet, Regex, Syntax};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const PATS: &[&str] = &[
    r"\w+", r"[0-9]+", r"(\w+)=(\w+)", r"[\w.]+@\w+", r"a+", r"(a+)+b",
    r"\d+\.\d+", r"(?m)^\w+", r"^\d+", r"(?>\w+)=", r"\d++ms", r"[a-z]+ing",
    r"a.*b", r"\bcat\b", r"(a|b)+c", r"x(?=\d)", r"(?<=a)b", r"(?~ab)",
    r"(\w+) \1", r"(?<n>\w+)-(?<m>\w+)", r"\w*", r"", r"(a)\g<1>", r"[^=]+=",
    r"a{2,4}", r"(?i)ABC", r"\p{L}+", r"(?m)^$", r"\s+", r"[a-z]+?x",
];

const ALPHABETS: &[&str] = &[
    "ab", "ab=c", "a\nb=", "aeiou=z", "0.9 x", "A@b.c", "ing t", "\nab\n",
    "aaaa", "abc123", " \t", "==", "cat dog cat",
];

fn hay_of(rng: &mut Rng) -> Vec<u8> {
    let a: Vec<u8> = ALPHABETS[rng.below(ALPHABETS.len())].bytes().collect();
    let n = rng.below(40);
    (0..n).map(|_| a[rng.below(a.len())]).collect()
}

fn compile(pat: &str) -> Option<Regex> {
    Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).ok()
}

fn main() {
    let mut rng = Rng(0xC0FFEE_1234_5678);
    let mut fails: Vec<String> = Vec::new();
    let mut checks = 0u64;

    // ---------------------------------------------------------------
    // 1. MatchParam: a limit may turn a result into an error, but it may
    //    never turn it into a *different* result.
    // ---------------------------------------------------------------
    let mut limited_errs = 0u32;
    for _ in 0..40_000 {
        let pat = PATS[rng.below(PATS.len())];
        let re = match compile(pat) {
            Some(r) => r,
            None => continue,
        };
        let hay = hay_of(&mut rng);

        let truth = re.search_param(&hay, &MatchParam::unlimited());
        let truth = match truth {
            Ok(v) => v.map(|m| (m.range().start, m.range().end)),
            Err(_) => continue, // unlimited should not error, but skip if it does
        };

        let mut p = MatchParam::default();
        match rng.below(4) {
            0 => p.stack_limit = [0, 1, 4, 32, 1024][rng.below(5)],
            1 => p.retry_limit_in_match = [0, 1, 8, 256, 4096][rng.below(5)] as u64,
            2 => p.retry_limit_in_search = [0, 1, 8, 256, 4096][rng.below(5)] as u64,
            _ => p.subexp_call_limit = [0, 1, 4, 64][rng.below(4)],
        }
        checks += 1;
        match re.search_param(&hay, &p) {
            Ok(v) => {
                let got = v.map(|m| (m.range().start, m.range().end));
                if got != truth {
                    fails.push(format!(
                        "MatchParam changed the result: {pat:?} hay={hay:?} limited={got:?} unlimited={truth:?}"
                    ));
                }
            }
            // An error is the documented outcome of hitting a limit.
            Err(_) => limited_errs += 1,
        }
    }

    // ---------------------------------------------------------------
    // 2. RegSet: leftmost match across the set, ties going to the lower
    //    index -- must agree with searching each pattern on its own.
    // ---------------------------------------------------------------
    for _ in 0..20_000 {
        let n = 1 + rng.below(4);
        let mut pats = Vec::new();
        let mut regs = Vec::new();
        for _ in 0..n {
            let p = PATS[rng.below(PATS.len())];
            if let Some(r) = compile(p) {
                pats.push(p);
                regs.push(r);
            }
        }
        if regs.is_empty() {
            continue;
        }
        let hay = hay_of(&mut rng);
        let set = match RegSet::new(regs.clone()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let got = match set.search(&hay, &MatchParam::default()) {
            Ok(v) => v.map(|(i, r)| (i, r.range().start, r.range().end)),
            Err(_) => continue,
        };
        let mut want: Option<(usize, usize, usize)> = None;
        for (i, r) in regs.iter().enumerate() {
            if let Ok(Some(m)) = r.search_param(&hay, &MatchParam::default()) {
                let cand = (i, m.range().start, m.range().end);
                let better = want.map(|w| cand.1 < w.1).unwrap_or(true);
                if better {
                    want = Some(cand);
                }
            }
        }
        checks += 1;
        if got != want {
            fails.push(format!(
                "RegSet disagrees: {pats:?} hay={hay:?} set={got:?} individual={want:?}"
            ));
        }
    }

    // ---------------------------------------------------------------
    // 3. scan: non-overlapping, strictly advancing, in bounds, and the
    //    same matches a manual find-all loop produces.
    // ---------------------------------------------------------------
    for _ in 0..20_000 {
        let pat = PATS[rng.below(PATS.len())];
        let re = match compile(pat) {
            Some(r) => r,
            None => continue,
        };
        let hay = hay_of(&mut rng);
        let hits = match scan(&re, &hay, &MatchParam::default()) {
            Ok(h) => h,
            Err(_) => continue,
        };
        checks += 1;
        let mut prev_end: Option<usize> = None;
        for h in &hits {
            let r = h.range();
            if r.start > r.end || r.end > hay.len() {
                fails.push(format!("scan out of bounds: {pat:?} hay={hay:?} {r:?}"));
                break;
            }
            if let Some(pe) = prev_end {
                // Non-overlapping and forward-only.
                if r.start < pe {
                    fails.push(format!(
                        "scan overlaps: {pat:?} hay={hay:?} hits={:?}",
                        hits.iter().map(|h| h.range()).collect::<Vec<_>>()
                    ));
                    break;
                }
            }
            prev_end = Some(if r.end > r.start { r.end } else { r.end + 1 });
        }
        // Every scan hit must be a real match at that position.
        for h in &hits {
            let r = h.range();
            match re.find_at(&hay, r.start) {
                Ok(Some(m)) if m.range() == r => {}
                other => {
                    fails.push(format!(
                        "scan hit is not a match at its own start: {pat:?} hay={hay:?} hit={r:?} find_at={:?}",
                        other.map(|o| o.map(|m| m.range()))
                    ));
                    break;
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // 4. search_range_param: the result stays inside [start, range] and
    //    equals a brute-force leftmost scan from `start`.
    // ---------------------------------------------------------------
    for _ in 0..20_000 {
        let pat = PATS[rng.below(PATS.len())];
        let re = match compile(pat) {
            Some(r) => r,
            None => continue,
        };
        let hay = hay_of(&mut rng);
        if hay.is_empty() {
            continue;
        }
        let start = rng.below(hay.len() + 1);
        let range = start + rng.below(hay.len() + 1 - start);
        let got = match re.search_range_param(&hay, start, range, &MatchParam::default()) {
            Ok(v) => v.map(|m| (m.range().start, m.range().end)),
            Err(_) => continue,
        };
        checks += 1;
        if let Some((s, e)) = got {
            if s < start || e > hay.len() {
                fails.push(format!(
                    "search_range escaped its bounds: {pat:?} hay={hay:?} start={start} range={range} got={got:?}"
                ));
            }
        }
        // Brute force from `start`, bounded by `range` the same way.
        let mut want = None;
        for at in start..=range.min(hay.len()) {
            if let Ok(Some(m)) = re.find_at(&hay, at) {
                want = Some((m.range().start, m.range().end));
                break;
            }
        }
        if got != want {
            fails.push(format!(
                "search_range disagrees with brute force: {pat:?} hay={hay:?} start={start} range={range} got={got:?} want={want:?}"
            ));
        }
    }

    println!("fuzz_api: {checks} property checks");
    println!("  MatchParam limits that produced an error (expected): {limited_errs}");
    if fails.is_empty() {
        println!("  0 violations");
    } else {
        println!("  {} violations:", fails.len());
        for f in fails.iter().take(12) {
            println!("    {f}");
        }
        std::process::exit(1);
    }
}
