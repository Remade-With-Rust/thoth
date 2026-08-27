//! Differential gate for constructs that run a BOUNDED sub-program: atomic
//! groups, look-around, absent expressions, conditionals, subexp calls.
//!
//! These share a failure mode: a quantifier inside the sub-program used to run
//! the continuation past the sub-program's stop boundary, so the caller then
//! re-ran the tail from the wrong position. `(?>\w+)=` matched nothing.
use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};

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
    // atomic
    r"(?>\w+)=", r"(?>a+)b", r"(?>[a-z]+)x", r"(?>a*)a", r"(?>\d+)\w",
    r"(?>ab|a)b", r"x(?>y+)z", r"(?>\w+)(?>=)", r"(?>(a)+)b", r"(?>a{2,4})b",
    // possessive
    r"\w++=", r"a++b", r"[a-z]++x", r"\d++\w", r"a?+b", r"a*+b",
    // look-ahead / behind with quantifiers inside
    r"\w+(?=\d+)", r"(?<=\d+)[a-z]", r"a(?!\d+)", r"(?<!a+)b",
    r"(?=\w+=)\w+", r"\d+(?= )", r"(?<=[a-z]+)\d",
    // conditionals and calls
    r"(a)?(?(1)b|c)", r"(\w)\g<1>", r"(?<r>a\g<r>?b)", r"(a)(?(1)\1)",
    // absent
    r"(?~ab)", r"a(?~b)c", r"(?~\d+)",
    // nesting of the above
    r"(?>\w+(?=\d))x", r"(?=(?>a+))ab", r"((?>a+)b)+", r"(?>a+)+b",
    r"(?:(?>\w+)|x)=", r"(?>\w+)?=", r"(?>a+)(?>b+)c",
    // plain quantifier shapes as controls
    r"\w+=", r"a+b", r"(a+)+b", r"a{2,4}b", r"\w+?=", r"a??b",
];

const ALPHABETS: &[&str] = &[
    "ab", "abc=", "a1b2=", "abcdef", "0123", "az09=", "ab= ", "aabb==",
    "xyz", "a=b=c=", "\u{e9}a1=", "AB12ab",
];

fn ours(re: &Regex, hay: &str) -> Option<(usize, usize)> {
    match re.search_param(hay.as_bytes(), &MatchParam::default()) {
        Ok(Some(m)) => {
            let r = m.range();
            Some((r.start, r.end))
        }
        _ => None,
    }
}

fn theirs(re: &onig::Regex, hay: &str) -> Option<(usize, usize)> {
    let mut region = onig::Region::new();
    re.search_with_options(
        hay,
        0,
        hay.len(),
        onig::SearchOptions::SEARCH_OPTION_NONE,
        Some(&mut region),
    )
    .and_then(|_| region.pos(0))
}

fn main() {
    let mut rng = Rng(0xDEAD_BEEF_1234_5678);
    let mut cases = 0u32;
    let mut diffs = 0u32;
    let mut shown = 0u32;
    let mut per_pat: Vec<(String, u32)> = Vec::new();

    for pat in PATS {
        let re = match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
            Ok(r) => r,
            Err(e) => {
                println!("SKIP ours-compile {pat:?}: {e}");
                continue;
            }
        };
        let ore = match onig::Regex::with_options(
            pat,
            onig::RegexOptions::REGEX_OPTION_NONE,
            onig::Syntax::oniguruma(),
        ) {
            Ok(r) => r,
            Err(e) => {
                println!("SKIP onig-compile {pat:?}: {e}");
                continue;
            }
        };
        let mut d = 0u32;
        for _ in 0..500 {
            let alpha: Vec<char> = ALPHABETS[rng.below(ALPHABETS.len())].chars().collect();
            let len = rng.below(24);
            let hay: String = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
            let a = ours(&re, &hay);
            let b = theirs(&ore, &hay);
            cases += 1;
            if a != b {
                d += 1;
                if shown < 12 {
                    shown += 1;
                    println!("DIFF {pat:<22} hay={hay:?}  ours={a:?} onig={b:?}");
                }
            }
        }
        if d > 0 {
            per_pat.push((pat.to_string(), d));
        }
        diffs += d;
    }

    if !per_pat.is_empty() {
        println!("\n{:<26} {:>8}", "pattern", "diffs");
        for (p, d) in &per_pat {
            println!("{:<26} {:>8}", p, d);
        }
    }
    println!("\ncases={cases}  diffs={diffs}");
    if diffs > 0 {
        std::process::exit(1);
    }
}
