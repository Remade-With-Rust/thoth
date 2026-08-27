//! Find-all scan (onig_scan).

extern crate alloc;

use alloc::vec::Vec;
use core::ops::Range;

use super::error::Error;
use super::param::MatchParam;
use super::region::Region;
use super::Regex;

/// Scan `hay` for non-overlapping matches. Empty matches advance one character.
pub fn scan(re: &Regex, hay: &[u8], param: &MatchParam) -> Result<Vec<Region>, Error> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos <= hay.len() {
        match re.search_range_param(hay, pos, hay.len(), param)? {
            Some(r) => {
                let Range { start, end } = r.range();
                out.push(r);
                if end == start {
                    // Advance past the MATCH, not past the cursor. The search
                    // may have skipped ahead to find this empty match, and
                    // stepping from `pos` would leave the cursor behind it --
                    // so the next round found the very same match again.
                    if start >= hay.len() {
                        break;
                    }
                    let step = match re.encoding().mbc_len(hay.get(start..).unwrap_or(&[])) {
                        Ok(n) if n > 0 => n,
                        _ => 1,
                    };
                    pos = start + step;
                } else {
                    pos = end;
                }
            }
            None => break,
        }
        if pos == 0 && !out.is_empty() {
            break;
        }
    }
    Ok(out)
}
