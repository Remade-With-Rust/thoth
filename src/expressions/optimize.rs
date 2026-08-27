//! Compile-time search optimizations: facts about a program that let a search
//! skip start positions it can prove cannot match.
//!
//! The first-byte [`Lead`](super::opcode::Lead) filter answers "can a match
//! begin with this byte". This module answers a stronger question: "is there a
//! byte sequence that *every* match must contain, and how far from the start".
//! A pattern like `[\w.]+@[\w.]+\.\w+` must contain `@`; in prose that is rare,
//! so finding it first and working backwards skips almost every start position.

extern crate alloc;

use alloc::vec::Vec;

use super::ast::CharClass;
use super::opcode::{Inst, Program};
use super::syntax::Options;

/// A byte sequence every match must contain, with its distance from the match
/// start, and optionally the class run that must immediately precede it.
#[derive(Clone, Debug)]
pub struct ReqLit {
    /// The required bytes. Never empty.
    pub bytes: Vec<u8>,
    /// Smallest possible distance from match start to `bytes`.
    pub min_dist: u32,
    /// Largest possible distance, or `None` when unbounded.
    pub max_dist: Option<u32>,
    /// pc of a `Class` that forms an unbounded run immediately before the
    /// literal, when the literal's first byte is *not* in that class.
    ///
    /// That combination is what makes an unbounded distance usable: the match
    /// must begin at or after the start of the class run ending at the
    /// literal, because a start any earlier would have to match the non-class
    /// byte that breaks the run.
    pub run_class: Option<u16>,
}

/// Width contributed by one instruction span, in bytes.
#[derive(Clone, Copy)]
struct Dist {
    min: u32,
    max: Option<u32>,
}

impl Dist {
    fn zero() -> Self {
        Self {
            min: 0,
            max: Some(0),
        }
    }
    fn add(self, min: u32, max: Option<u32>) -> Self {
        Self {
            min: self.min.saturating_add(min),
            max: match (self.max, max) {
                (Some(a), Some(b)) => Some(a.saturating_add(b)),
                _ => None,
            },
        }
    }
}

/// Is `b` a possible first byte of this class, over ASCII only?
///
/// Conservative: `true` whenever the class could reach non-ASCII, so a
/// multi-byte member can never be mistaken for a run-breaking byte.
fn class_admits_byte(prog: &Program, class_pc: usize, b: u8) -> bool {
    match prog.class_plans.get(class_pc).copied().flatten() {
        Some(plan) => {
            if b >= 0x80 {
                return true;
            }
            plan.hit(u32::from(b))
        }
        None => true,
    }
}

fn class_of(prog: &Program, pc: usize) -> Option<&CharClass> {
    match prog.insts.get(pc) {
        Some(Inst::Class { class }) => Some(class),
        _ => None,
    }
}

/// Find a byte sequence that every match must contain.
///
/// Walks only the program's unconditional spine: it stops at any `Split`, and
/// steps over repetitions rather than into them, so anything it returns is on
/// every path by construction.
pub fn required_literal(prog: &Program, ascii_ok: bool, options: Options) -> Option<ReqLit> {
    if !ascii_ok {
        return None;
    }
    let mut cur = options;
    let mut opt_stack: Vec<Options> = Vec::new();
    let mut best: Option<ReqLit> = None;
    let mut dist = Dist::zero();
    let mut pc = 0usize;
    let mut fuel = 4096u32;
    // pc of an unbounded simple-class repeat seen immediately before the
    // current position, if any.
    let mut pending_run: Option<u16> = None;

    loop {
        if fuel == 0 {
            break;
        }
        fuel -= 1;
        let inst = match prog.insts.get(pc) {
            Some(i) => i,
            None => break,
        };
        match inst {
            Inst::Nop
            | Inst::Save(_)
            | Inst::Keep
            | Inst::Assert(_)
            | Inst::Callout { .. } => pc += 1,
            // A literal is only a byte filter while matching is case
            // sensitive, so track the options in force.
            Inst::PushOptions(set, clear) => {
                opt_stack.push(cur);
                cur = cur.union(*set).difference(*clear);
                pc += 1;
            }
            Inst::PopOptions => {
                if let Some(o) = opt_stack.pop() {
                    cur = o;
                }
                pc += 1;
            }
            Inst::Jump(j) => {
                let j = *j as usize;
                if j <= pc {
                    break;
                }
                pc = j;
            }
            // An atomic group is on every path, and its body ends with a
            // jump back to the spine, so simply walk into it.
            Inst::Atomic { body, .. } => pc = *body as usize,
            Inst::Look {
                body,
                after,
                behind,
                negative,
            } => {
                // A positive look-ahead asserts its body at exactly this
                // position, so its literal is required too -- and being
                // zero-width, it leaves any preceding class run intact.
                if !*behind && !*negative && !cur.contains(Options::IGNORECASE) {
                    match prog.insts.get(*body as usize) {
                        Some(Inst::Char(c)) if *c <= 0x7F => {
                            consider(&mut best, &[*c as u8], dist, pending_run, prog);
                        }
                        Some(Inst::Literal(v)) if v.iter().all(|c| *c <= 0x7F) => {
                            let bytes: Vec<u8> = v.iter().map(|c| *c as u8).collect();
                            consider(&mut best, &bytes, dist, pending_run, prog);
                        }
                        _ => {}
                    }
                }
                pc = *after as usize;
            }
            Inst::Char(c) => {
                if *c > 0x7F {
                    break;
                }
                let bytes = alloc::vec![*c as u8];
                if !cur.contains(Options::IGNORECASE) {
                    consider(&mut best, &bytes, dist, pending_run, prog);
                }
                dist = dist.add(1, Some(1));
                pending_run = None;
                pc += 1;
            }
            Inst::Literal(v) => {
                if v.iter().any(|c| *c > 0x7F) {
                    break;
                }
                let bytes: Vec<u8> = v.iter().map(|c| *c as u8).collect();
                let n = bytes.len() as u32;
                if !cur.contains(Options::IGNORECASE) {
                    consider(&mut best, &bytes, dist, pending_run, prog);
                }
                dist = dist.add(n, Some(n));
                pending_run = None;
                pc += 1;
            }
            Inst::Class { .. } | Inst::Any { .. } | Inst::SuperAny | Inst::TextSegment => {
                dist = dist.add(1, None);
                pending_run = None;
                pc += 1;
            }
            Inst::GeneralNewline => {
                dist = dist.add(1, None);
                pending_run = None;
                pc += 1;
            }
            Inst::Repeat {
                body,
                after,
                min,
                max,
                ..
            } => {
                let (body, after) = (*body as usize, *after as usize);
                let (rmin, rmax) = (*min, *max);
                // Only a single-instruction body has a width we can model.
                let unit = match prog.insts.get(body) {
                    Some(Inst::Char(_)) | Some(Inst::Class { .. }) => Some(1u32),
                    Some(Inst::Literal(v)) => Some(v.len() as u32),
                    _ => None,
                };
                match unit {
                    Some(w) => {
                        dist = dist.add(
                            rmin.saturating_mul(w),
                            rmax.map(|m| m.saturating_mul(w)),
                        );
                    }
                    None => {
                        dist = dist.add(rmin, None);
                    }
                }
                // Remember an unbounded class run so the literal after it can
                // use the backward-extend trick.
                pending_run = if rmax.is_none() && rmin >= 1 && matches!(prog.insts.get(body), Some(Inst::Class { .. })) {
                    Some(body as u16)
                } else {
                    None
                };
                pc = after;
            }
            // Anything else could match arbitrary text or diverge.
            _ => break,
        }
    }
    best
}

fn consider(
    best: &mut Option<ReqLit>,
    bytes: &[u8],
    dist: Dist,
    pending_run: Option<u16>,
    prog: &Program,
) {
    if bytes.is_empty() {
        return;
    }
    // A run-anchored literal is only usable when its first byte breaks the run.
    let run_class = pending_run.filter(|pc| !class_admits_byte(prog, *pc as usize, bytes[0]));
    // Unbounded distance is only usable with the backward-extend trick.
    if dist.max.is_none() && run_class.is_none() {
        return;
    }
    let cand = ReqLit {
        bytes: bytes.to_vec(),
        min_dist: dist.min,
        max_dist: dist.max,
        run_class,
    };
    // Prefer a longer literal (rarer), then one anchored to a run, then one
    // at a bounded distance.
    let better = match best {
        None => true,
        Some(b) => {
            (cand.bytes.len(), cand.run_class.is_some(), cand.max_dist.is_some())
                > (b.bytes.len(), b.run_class.is_some(), b.max_dist.is_some())
        }
    };
    if better {
        *best = Some(cand);
    }
}

/// Offset of `needle` in `hay`, or `None`.
///
/// Two-phase: scan for the first byte, then compare. That keeps the inner loop
/// a plain byte-equality search, which LLVM turns into a vector scan.
#[inline]
pub fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    let first = needle[0];
    if needle.len() == 1 {
        return hay.iter().position(|&b| b == first);
    }
    let last = hay.len() - needle.len();
    let mut i = 0usize;
    while i <= last {
        match hay[i..=last].iter().position(|&b| b == first) {
            Some(off) => {
                let p = i + off;
                if &hay[p..p + needle.len()] == needle {
                    return Some(p);
                }
                i = p + 1;
            }
            None => return None,
        }
    }
    None
}

/// Start of the maximal run of `class` ending at `at`, never before `floor`.
pub fn class_run_start(
    prog: &Program,
    class_pc: usize,
    hay: &[u8],
    floor: usize,
    at: usize,
) -> usize {
    if class_of(prog, class_pc).is_none() {
        return floor;
    }
    let mut s = at;
    while s > floor {
        let b = hay[s - 1];
        if !class_admits_byte(prog, class_pc, b) {
            break;
        }
        s -= 1;
    }
    s
}
