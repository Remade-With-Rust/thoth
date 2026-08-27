//! Packed bytecode (Oniguruma-shaped opcodes, Rust enum).

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::ast::{AbsentKind, Anchor, Backref, CallTarget, CharClass, Cond};
use super::callout::CalloutDir;
use super::syntax::Options;

#[derive(Clone, Debug)]
pub enum Inst {
    Nop,
    Char(u32),
    Literal(Vec<u32>),
    Any { newline: bool },
    SuperAny,
    Class { class: CharClass },
    Split(u16, u16),
    Jump(u16),
    Save(u16),
    Match,
    Fail,
    Assert(Anchor),
    Repeat {
        body: u16,
        after: u16,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        possessive: bool,
    },
    Look {
        body: u16,
        after: u16,
        behind: bool,
        negative: bool,
    },
    Atomic {
        body: u16,
        after: u16,
    },
    Backref(Backref),
    Call(CallTarget),
    Keep,
    Absent {
        stopper: u16,
        expr: Option<u16>,
        after: u16,
        kind: AbsentKind,
    },
    Cond {
        cond: Cond,
        then_pc: u16,
        else_pc: Option<u16>,
        after: u16,
    },
    GeneralNewline,
    TextSegment,
    Callout {
        named: bool,
        name: String,
        args: String,
        tag: Option<String>,
        body: String,
        dir: CalloutDir,
    },
    PushOptions(Options, Options),
    PopOptions,
}

/// Possible first bytes of any match, as a 256-bit set.
///
/// A search may skip any haystack byte whose bit is clear. The set must never
/// under-approximate: a false negative would drop a real match, a false
/// positive only costs a wasted match attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lead {
    /// Bit `b` set means a match may begin with byte `b`.
    pub set: [u64; 4],
    /// `Some(b)` when the set holds exactly one byte (enables the memchr path).
    pub single: Option<u8>,
}

impl Lead {
    #[inline(always)]
    pub fn contains(&self, b: u8) -> bool {
        self.set[(b >> 6) as usize] & (1u64 << (b & 63)) != 0
    }

    #[inline]
    pub fn insert(&mut self, b: u8) {
        self.set[(b >> 6) as usize] |= 1u64 << (b & 63);
    }

    pub fn empty() -> Self {
        Self {
            set: [0; 4],
            single: None,
        }
    }

    /// Mark every non-ASCII byte possible (conservative for Unicode classes).
    pub fn insert_non_ascii(&mut self) {
        // Bytes 0x80..=0xFF live in words 2 and 3 (b >> 6).
        self.set[2] = 0xFFFF_FFFF_FFFF_FFFF;
        self.set[3] = 0xFFFF_FFFF_FFFF_FFFF;
    }

    pub fn count(&self) -> u32 {
        self.set.iter().map(|w| w.count_ones()).sum()
    }

    /// Fill `single` and reject sets so dense they cannot filter anything.
    pub fn finish(mut self) -> Option<Self> {
        let n = self.count();
        if n == 0 || n >= 256 {
            return None;
        }
        if n == 1 {
            for b in 0..=255u8 {
                if self.contains(b) {
                    self.single = Some(b);
                    break;
                }
            }
        }
        Some(self)
    }
}

/// What a repetition body looks like, decided once at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimpleBody {
    /// Not a single-instruction body.
    None,
    /// Exactly one `Char`.
    Char(u32),
    /// Exactly one `Class`, at this pc.
    Class(u16),
}

/// A character class lowered to a 128-bit ASCII membership bitmap.
///
/// Walking `Vec<ClassItem>` and dispatching into `unicode::is_word` and
/// friends ran 1_219_897 times across the benchmark workload -- once per
/// character tested. Below U+0080 that collapses to a shift and an AND.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassPlan {
    /// Bit `cp` set means codepoint `cp` (< 128) is a member.
    pub ascii: [u64; 2],
    /// The options this bitmap was built under. The VM only trusts the plan
    /// when the options in force match, so an inline `(?i)` cannot make it lie.
    pub options: Options,
}

impl ClassPlan {
    #[inline(always)]
    pub fn hit(&self, cp: u32) -> bool {
        self.ascii[(cp >> 6) as usize] & (1u64 << (cp & 63)) != 0
    }
}

/// Facts about one repetition that never change at match time.
#[derive(Clone, Copy, Debug)]
pub struct RepeatShape {
    pub writes_caps: bool,
    pub simple: SimpleBody,
    /// The next consuming instruction after this repetition is a character the
    /// body's class cannot match.
    ///
    /// Then a greedy run has exactly one viable length -- the maximal one --
    /// because every shorter length leaves a class member where the required
    /// character must go. `\w+=` stops backtracking altogether.
    pub follow_disjoint: bool,
    /// First byte of the character required right after the repetition.
    ///
    /// Only run lengths that leave this byte in place can succeed, so a greedy
    /// run can jump straight between its occurrences instead of trying every
    /// length. `[a-z]+ing` over a word tests the two or three positions where
    /// an `i` actually sits, not every position in the run.
    pub follow_byte: Option<u8>,
    /// Compare `follow_byte` case-insensitively.
    pub follow_icase: bool,
}

impl Default for RepeatShape {
    fn default() -> Self {
        Self {
            writes_caps: true,
            simple: SimpleBody::None,
            follow_disjoint: false,
            follow_byte: None,
            follow_icase: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub insts: Vec<Inst>,
    pub capture_count: usize,
    pub names: Vec<Option<String>>,
    pub has_named: bool,
    pub history_groups: Vec<bool>,
    /// First-byte prefilter, or `None` when no useful set exists.
    /// Filled by [`super::exec::compute_lead`] once the encoding is known.
    pub lead: Option<Lead>,
    /// Per-repetition shape, indexed by the repeat's *body* pc. Recomputing
    /// this per repeat entry cost 341_832 body scans across the benchmark.
    pub repeat_shapes: Vec<RepeatShape>,
    /// Pre-rendered ASCII literal when the whole program is one, so the
    /// literal fast path costs no allocation per search.
    pub ascii_literal: Option<Vec<u8>>,
    /// Group number -> (open Save pc, close Save pc), for `\g<>` calls.
    pub group_spans: Vec<Option<(u16, u16)>>,
    /// ASCII membership bitmap per `Class` instruction, indexed by pc.
    pub class_plans: Vec<Option<ClassPlan>>,
    /// Pre-rendered bytes per all-ASCII `Literal` instruction, indexed by pc,
    /// so a literal is one slice compare instead of a per-character decode.
    pub literal_bytes: Vec<Option<Vec<u8>>>,
    /// A byte sequence every match must contain, used to skip start positions.
    pub req_lit: Option<super::optimize::ReqLit>,
    /// Every match must begin at a line start (`^` in multi-line mode).
    ///
    /// Then the candidate set is exactly the positions after a newline, which
    /// one scan enumerates -- far rarer than the first byte of the literal
    /// that follows.
    pub anchored_bol: bool,
}
