//! Differential gate for the first-byte prefilter: ours vs libonig over
//! generated pattern x haystack pairs. A prefilter bug shows up as ours=miss
//! where onig matched, so that direction is fatal.
use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

const PATS: &[&str] = &[
    "[0-9]", "[0-9]+", "[a-y]+", "[aeiou]", "[^0-9]", "[^a-z]+", r"\d", r"\d+",
    r"\p{Lu}", r"\p{L}", r"\p{Nd}+", r"\w", r"\s+", "[[:alpha:]]", "[[:punct:]]+",
    "[0-9a-fA-F]+", r"[\x{00e9}\x{4e2d}]", r"\p{Greek}", "[a-z0-9_]+", r"[^\x{00e9}]",
    "q[0-9]", "qqq", "[0-9]*x", "a|[0-9]", "(?:[0-9]|x)y", "[0-9]{2,3}",
    r"\D", r"\S", r"[\d\s]", r"[^\W]", r"\p{Han}", r"[\x{80}-\x{ff}]",
    // required-literal territory: run-anchored and fixed-distance literals
    r"(\w+)=(\w+)", r"[\w.]+@[\w.]+\.\w+", r"\d+\.\d+", r"(\w+) \1",
    r"\w+=", r"[a-z]+X", r"x[0-9]+y", r"a.*b", r"\bcat\b", r"(?:ab)+cd",
    r"https?://\w+", r"(\d{2})-(\d{2})", r"[a-z]+ing", r"(?i)[a-z]+ing",
    r"(?i)abc", r"a\w*z", r"\w+\.\w+", r"(?>\w+)=", r"\d++ms", r"[^=]+=",
    // line-anchored: exercises the newline-scan candidate set
    r"^a", r"^\d+", r"(?m)^a", r"(?m)^ab", r"(?m)^\w+", r"(?m)^", r"(?m)^$",
    r"(?m)^[0-9]", r"\Aab", r"(?m)^a$", r"^", r"(?m)^.",
];

const ALPHABETS: &[&str] = &[
    "z", "abcdefghij", "0123456789", "az09 ", "\u{e9}\u{4e2d}a1",
    "ABCXYZ", "\u{394}\u{3b1}b2", " \t\n", "!?.,;", "\u{1F600}a0",
    "ab=cd", "a.b0", "x@y.z", "ing ", "abcX", "ms12", "http://a",
    "a
b", "ab
12
", "

a", "x
y
z", "
", "a

b1",
];

fn ours(re: &Regex, hay: &str) -> Option<(usize, usize)> {
    match re.search_param(hay.as_bytes(), &MatchParam::default()) {
        Ok(Some(m)) => { let r = m.range(); Some((r.start, r.end)) }
        _ => None,
    }
}

fn theirs(re: &onig::Regex, hay: &str) -> Option<(usize, usize)> {
    let mut region = onig::Region::new();
    re.search_with_options(hay, 0, hay.len(), onig::SearchOptions::SEARCH_OPTION_NONE, Some(&mut region))
        .and_then(|_| region.pos(0))
}

/// Scan every start position with find_at, which never consults the prefilter.
/// If this finds a match the prefiltered search missed, the prefilter is wrong.
fn ours_unfiltered(re: &Regex, hay: &str) -> Option<(usize, usize)> {
    for at in 0..=hay.len() {
        if !hay.is_char_boundary(at) { continue; }
        if let Ok(Some(m)) = re.find_at(hay.as_bytes(), at) {
            let r = m.range();
            return Some((r.start, r.end));
        }
    }
    None
}

fn main() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut cases = 0u32;
    let mut prefilter_bugs = 0u32;
    let mut semantic = 0u32;
    let mut per_pat: Vec<(String, u32, u32)> = Vec::new();
    for pat in PATS {
        let re = match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
            Ok(r) => r,
            Err(e) => { println!("SKIP ours-compile {pat:?}: {e}"); continue; }
        };
        let ore = match onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()) {
            Ok(r) => r,
            Err(e) => { println!("SKIP onig-compile {pat:?}: {e}"); continue; }
        };
        let (mut pf, mut sem) = (0u32, 0u32);
        for _ in 0..400 {
            let alpha: Vec<char> = ALPHABETS[rng.below(ALPHABETS.len())].chars().collect();
            let len = rng.below(60);
            let hay: String = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
            let a = ours(&re, &hay);
            let b = theirs(&ore, &hay);
            cases += 1;
            if a == b { continue; }
            // Does our own engine, with the prefilter bypassed, agree with the
            // prefiltered search? If not, the prefilter dropped a match.
            let unf = ours_unfiltered(&re, &hay);
            if unf != a {
                pf += 1;
                if prefilter_bugs + pf <= 6 {
                    println!("PREFILTER-BUG {pat:?} hay={hay:?} filtered={a:?} unfiltered={unf:?} onig={b:?}");
                }
            } else {
                sem += 1;
            }
        }
        prefilter_bugs += pf;
        semantic += sem;
        if pf > 0 || sem > 0 {
            per_pat.push((pat.to_string(), pf, sem));
        }
    }
    println!("\n{:<26} {:>14} {:>14}", "pattern", "prefilter-bug", "semantic-diff");
    for (p, pf, sem) in &per_pat {
        println!("{:<26} {:>14} {:>14}", p, pf, sem);
    }
    println!("\ncases={cases}  PREFILTER BUGS={prefilter_bugs}  pre-existing semantic diffs={semantic}");
    if prefilter_bugs > 0 { std::process::exit(1); }
}
