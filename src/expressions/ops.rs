//! Public ops: callable by a test, CLI, and example (API-first).

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

use super::error::Error;
use super::param::MatchParam;
use super::syntax::{Options, Syntax};
use super::Encoding;
use super::Regex;

/// Grep-like op: return byte ranges of matches of `pattern` in `hay`.
pub fn find_all(
    pattern: &[u8],
    hay: &[u8],
    encoding: Encoding,
    syntax: Syntax,
    options: Options,
) -> Result<Vec<Range<usize>>, Error> {
    let re = Regex::new(pattern, options, encoding, syntax)?;
    let hits = super::scan::scan(&re, hay, &MatchParam::default())?;
    Ok(hits.into_iter().map(|r| r.range()).collect())
}

/// UTF-8 convenience for [`find_all`].
pub fn find_all_str(pattern: &str, hay: &str) -> Result<Vec<Range<usize>>, Error> {
    find_all(
        pattern.as_bytes(),
        hay.as_bytes(),
        Encoding::UTF8,
        Syntax::ONIGURUMA,
        Options::NONE,
    )
}

/// True if `pattern` matches anywhere in `hay`.
pub fn is_match_str(pattern: &str, hay: &str) -> Result<bool, Error> {
    let re = Regex::new(pattern.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)?;
    re.is_match(hay.as_bytes())
}

/// Format matches for a CLI/example surface.
pub fn format_matches(hay: &str, ranges: &[Range<usize>]) -> String {
    let mut s = String::new();
    for r in ranges {
        let frag = hay.get(r.start..r.end).unwrap_or("");
        s.push_str(&alloc::format!("{}..{} {}\n", r.start, r.end, frag));
    }
    s
}
