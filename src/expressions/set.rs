//! Multi-pattern RegSet (same encoding).

extern crate alloc;

use alloc::vec::Vec;

use super::error::{Error, ErrorKind};
use super::param::MatchParam;
use super::region::Region;
use super::Regex;

/// Set of compiled regexes. All must share an encoding. FIND_LONGEST is refused.
pub struct RegSet {
    regs: Vec<Regex>,
}

impl RegSet {
    pub fn new(regs: Vec<Regex>) -> Result<Self, Error> {
        if let Some(first) = regs.first() {
            let enc = first.encoding();
            for r in &regs {
                if r.encoding() != enc {
                    return Err(Error::kind_msg(
                        ErrorKind::InvalidArgument,
                        "regset encodings differ",
                    ));
                }
                if r.options().contains(super::syntax::Options::FIND_LONGEST) {
                    return Err(Error::kind_msg(
                        ErrorKind::InvalidArgument,
                        "FIND_LONGEST not allowed in regset",
                    ));
                }
            }
        }
        Ok(Self { regs })
    }

    pub fn add(&mut self, re: Regex) -> Result<(), Error> {
        if let Some(first) = self.regs.first() {
            if first.encoding() != re.encoding() {
                return Err(Error::kind_msg(
                    ErrorKind::InvalidArgument,
                    "regset encodings differ",
                ));
            }
        }
        self.regs.push(re);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.regs.len()
    }

    pub fn get(&self, i: usize) -> Option<&Regex> {
        self.regs.get(i)
    }

    /// Search all patterns; return (index, region) of the leftmost match
    /// (Oniguruma lead: position then index).
    pub fn search(
        &self,
        hay: &[u8],
        param: &MatchParam,
    ) -> Result<Option<(usize, Region)>, Error> {
        let mut best: Option<(usize, Region)> = None;
        for (i, re) in self.regs.iter().enumerate() {
            if let Some(r) = re.search_param(hay, param)? {
                let better = match &best {
                    None => true,
                    Some((_, b)) => r.range().start < b.range().start,
                };
                if better {
                    best = Some((i, r));
                }
            }
        }
        Ok(best)
    }
}
