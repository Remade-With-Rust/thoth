//! Match-heavy path: candidates are frequent, so the prefilter cannot skip and
//! the VM runs. Exercises consume_class / class_hit per character.
use std::hint::black_box;
use std::time::Instant;
use thoth::expressions::{count, Encoding, MatchParam, Options, Regex, Syntax};

fn med(mut v: Vec<u128>) -> u128 { v.sort(); v[v.len() / 2] }

fn main() {
    // Dense digits: every position is a candidate, matches are short.
    let hay = "12 34 56 78 90 ".repeat(2000); // 30 KB, ~40% digits
    let param = MatchParam::default();
    println!("{:<12} {:>12} {:>12} {:>9} {:>14}", "pattern", "ours_ns", "onig_ns", "ratio", "work");
    for pat in ["[0-9]+", r"\d+", "[0-9]", r"\p{Nd}+", "[0-9][0-9]"] {
        let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
        let ore = onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()).unwrap();
        // count one full find_all pass
        count::reset();
        let mut at = 0usize;
        let mut n = 0u32;
        while at <= hay.len() {
            match re.search_range_param(hay.as_bytes(), at, hay.len(), &param) {
                Ok(Some(m)) => { let r = m.range(); n += 1; at = if r.end > r.start { r.end } else { r.end + 1 }; }
                _ => break,
            }
        }
        let work = count::snapshot().work();
        let scan_ours = |_: ()| {
            let mut at = 0usize;
            while at <= hay.len() {
                match re.search_range_param(hay.as_bytes(), at, hay.len(), &param) {
                    Ok(Some(m)) => { let r = m.range(); at = if r.end > r.start { r.end } else { r.end + 1 }; }
                    _ => break,
                }
            }
        };
        let scan_onig = |_: ()| {
            let mut at = 0usize;
            while at <= hay.len() {
                let mut region = onig::Region::new();
                match ore.search_with_options(&hay, at, hay.len(), onig::SearchOptions::SEARCH_OPTION_NONE, Some(&mut region)) {
                    Some(_) => {
                        let (s, e) = region.pos(0).unwrap();
                        at = if e > s { e } else { e + 1 };
                    }
                    None => break,
                }
            }
        };
        for _ in 0..3 { black_box(scan_ours(())); black_box(scan_onig(())); }
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for _ in 0..9 {
            let t = Instant::now(); black_box(scan_ours(())); a.push(t.elapsed().as_nanos());
            let t = Instant::now(); black_box(scan_onig(())); b.push(t.elapsed().as_nanos());
            let t = Instant::now(); black_box(scan_onig(())); b.push(t.elapsed().as_nanos());
            let t = Instant::now(); black_box(scan_ours(())); a.push(t.elapsed().as_nanos());
        }
        let (av, bv) = (med(a) as f64, med(b) as f64);
        println!("{:<12} {:>12.0} {:>12.0} {:>9.2} {:>14}  ({n} matches)", pat, av, bv, av / bv, work);
    }
}
