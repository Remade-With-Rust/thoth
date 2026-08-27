//! Probes for things the existing gates structurally cannot see.
//!
//! 1. Deep *pattern* nesting. The repetition abort was fixed by moving the
//!    trail to the heap, but `run_stop` still recurses for Split, Look, Atomic
//!    and repeat bodies -- so nesting depth is still native stack depth.
//! 2. Capture-history entries are pushed on a closing Save and never popped on
//!    backtrack, so a failed alternative may leave phantom history.
//! 3. Callout hooks change control flow and were never fuzzed.
//! 4. Compile-time analysis walks (lead_walk, width_range, required_literal)
//!    recurse over the program too.
use thoth::expressions::{
    CalloutResult, Encoding, MatchParam, Options, Regex, Syntax,
};

fn try_compile(pat: &str) -> Option<Regex> {
    Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).ok()
}

fn probe(label: &str, pat: &str, hay: &str) {
    match try_compile(pat) {
        None => println!("  {label:<28} compile-err (graceful)"),
        Some(re) => match re.search_param(hay.as_bytes(), &MatchParam::default()) {
            Ok(Some(m)) => println!("  {label:<28} match {:?}", m.range()),
            Ok(None) => println!("  {label:<28} miss"),
            Err(e) => println!("  {label:<28} Err {:?} (graceful)", e.kind),
        },
    }
}

fn main() {
    println!("--- 1. deep pattern nesting (native recursion in run_stop) ---");
    for n in [10usize, 50, 200, 1000, 5000] {
        let pat = format!("{}a{}", "(".repeat(n), ")".repeat(n));
        probe(&format!("nested groups x{n}"), &pat, "a");
    }
    for n in [10usize, 50, 200, 1000, 5000] {
        let pat = format!("{}a{}", "(?:".repeat(n), ")".repeat(n));
        probe(&format!("nested non-capt x{n}"), &pat, "a");
    }
    for n in [10usize, 100, 1000, 5000] {
        let pat = (0..n).map(|_| "x").collect::<Vec<_>>().join("|");
        probe(&format!("alternation x{n}"), &pat, "x");
    }
    for n in [5usize, 20, 100, 500] {
        let pat = format!("{}a*{}", "(".repeat(n), ")*".repeat(n));
        probe(&format!("nested star x{n}"), &pat, "aaa");
    }
    for n in [10usize, 100, 1000] {
        let pat = format!("{}a{}", "(?=".repeat(n), ")".repeat(n));
        probe(&format!("nested lookahead x{n}"), &pat, "a");
    }
    for n in [10usize, 100, 1000] {
        let pat = format!("{}a{}", "(?>".repeat(n), ")".repeat(n));
        probe(&format!("nested atomic x{n}"), &pat, "a");
    }
    for n in [10usize, 100, 1000] {
        let pat = format!("[{}]", "a-z".repeat(n));
        probe(&format!("wide class x{n}"), &pat, "m");
    }
    for n in [10usize, 100, 1000] {
        let pat = format!("{}", "a{1,2}".repeat(n));
        probe(&format!("chained bounded x{n}"), &pat, &"a".repeat(n));
    }

    println!("\n--- 2. capture history across backtracking ---");
    // The alternation's first branch matches and records history, then the
    // overall match fails and backtracks. Does the discarded branch linger?
    for (pat, hay) in [
        (r"(?@(ab|a))+c", "abac"),
        (r"(?:(?@a)x|(?@a)b)", "ab"),
        (r"(?@(a))+(?@(b))+", "aabb"),
        (r"(?@a)*b", "aaab"),
    ] {
        let re = match try_compile(pat) {
            Some(r) => r,
            None => {
                println!("  {pat:<24} compile-err");
                continue;
            }
        };
        match re.search(hay.as_bytes()) {
            Ok(Some(m)) => {
                let mut nodes = Vec::new();
                m.traverse_history(|n, d| nodes.push((n.group, n.range.clone(), d)));
                let whole = m.range();
                let outside: Vec<_> = nodes
                    .iter()
                    .filter(|(_, r, _)| r.start < whole.start || r.end > whole.end)
                    .collect();
                println!(
                    "  {pat:<24} hay={hay:?} match={whole:?} history={} nodes{}",
                    nodes.len(),
                    if outside.is_empty() {
                        String::new()
                    } else {
                        format!("  <-- {} OUTSIDE the match: {outside:?}", outside.len())
                    }
                );
            }
            other => println!("  {pat:<24} hay={hay:?} -> {other:?}"),
        }
    }

    println!("\n--- 3. callout hooks driving control flow ---");
    fn always_fail(_c: &thoth::expressions::CalloutCtx<'_>) -> CalloutResult {
        CalloutResult::Fail
    }
    fn always_skip(_c: &thoth::expressions::CalloutCtx<'_>) -> CalloutResult {
        CalloutResult::Skip
    }
    fn always_ok(_c: &thoth::expressions::CalloutCtx<'_>) -> CalloutResult {
        CalloutResult::Success
    }
    for (name, hook) in [
        ("Fail", always_fail as fn(&thoth::expressions::CalloutCtx<'_>) -> CalloutResult),
        ("Skip", always_skip),
        ("Success", always_ok),
    ] {
        for (pat, hay) in [
            (r"a(?{x})b", "ab"),
            (r"(?:a(?{x})x|ab)", "ab"),
            (r"a(?{x})*b", "ab"),
            (r"\w+(?{x})=", "ab=c"),
        ] {
            let re = match try_compile(pat) {
                Some(r) => r,
                None => continue,
            };
            let mut p = MatchParam::default();
            p.progress_callout = Some(hook);
            p.retraction_callout = Some(hook);
            p.named_callout = Some(hook);
            let r = match re.search_param(hay.as_bytes(), &p) {
                Ok(Some(m)) => format!("{:?}", m.range()),
                Ok(None) => "miss".into(),
                Err(e) => format!("Err {:?}", e.kind),
            };
            println!("  hook={name:<8} {pat:<20} hay={hay:?} -> {r}");
        }
    }

    println!("\n--- 4. huge bounded repeats ---");
    for pat in ["a{1000000}", "a{0,1000000}", "(a){100000}", "a{2,}{2,}"] {
        probe(pat, pat, &"a".repeat(64));
    }

    println!("\ndone (reaching this line means nothing aborted the process)");
}
