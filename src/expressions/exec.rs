//! Backtracking VM: bytecode -> match/search.
//!
//! Oniguruma is a backtracking NFA and so is this. The mitigations for that
//! are the limits on [`MatchParam`], not a Thompson rewrite.

extern crate alloc;

use alloc::vec::Vec;
use core::ops::Range;

use super::ast::{AbsentKind, Anchor, Backref, CallTarget, CharClass, ClassItem, Cond};
use super::callout::{CalloutCtx, CalloutDir, CalloutResult};
use super::encoding::Encoding;
use super::error::{Error, ErrorKind};
use super::opcode::{ClassPlan, Inst, Lead, Program, RepeatShape, SimpleBody};
use super::param::MatchParam;
use super::region::{CaptureTree, Region};
use super::syntax::Options;
use super::unicode::{self, grapheme_break};

/// Capture slots, kept inline for the patterns that fit.
///
/// Almost every pattern has a handful of groups, and the whole vector used to
/// be a heap allocation per search -- 26_306 of them across the benchmark
/// workload. Sixteen slots (seven groups plus the whole match) live in the
/// engine itself; anything larger spills.
const INLINE_CAPS: usize = 16;

struct Caps {
    inline: [Option<usize>; INLINE_CAPS],
    heap: Vec<Option<usize>>,
    len: usize,
    spilled: bool,
}

impl Caps {
    fn new(n: usize) -> Self {
        if n <= INLINE_CAPS {
            Self {
                inline: [None; INLINE_CAPS],
                heap: Vec::new(),
                len: n,
                spilled: false,
            }
        } else {
            Self {
                inline: [None; INLINE_CAPS],
                heap: alloc::vec![None; n],
                len: n,
                spilled: true,
            }
        }
    }
}

impl Clone for Caps {
    fn clone(&self) -> Self {
        if self.spilled {
            Self {
                inline: [None; INLINE_CAPS],
                heap: self.heap.clone(),
                len: self.len,
                spilled: true,
            }
        } else {
            // Copy only the live slots, not the whole array.
            let mut c = Self {
                inline: [None; INLINE_CAPS],
                heap: Vec::new(),
                len: self.len,
                spilled: false,
            };
            c.inline[..self.len].copy_from_slice(&self.inline[..self.len]);
            c
        }
    }
}

impl core::ops::Deref for Caps {
    type Target = [Option<usize>];
    #[inline(always)]
    fn deref(&self) -> &[Option<usize>] {
        if self.spilled {
            &self.heap
        } else {
            &self.inline[..self.len]
        }
    }
}

impl core::ops::DerefMut for Caps {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [Option<usize>] {
        if self.spilled {
            &mut self.heap
        } else {
            &mut self.inline[..self.len]
        }
    }
}

pub struct Engine<'a> {
    hay: &'a [u8],
    enc: Encoding,
    prog: &'a Program,
    options: Options,
    /// Options as compiled, so an attempt can reset after inline `(?i)`.
    base_options: Options,
    param: &'a MatchParam,
    user_props: &'a [unicode::UserProperty],
    /// Flat capture slots: `2*g` is group `g`'s start, `2*g + 1` its end.
    captures: Caps,
    option_stack: Vec<Options>,
    keep: usize,
    search_origin: usize,
    hay_start: usize,
    hay_end: usize,
    retry_match: u64,
    retry_search: u64,
    calls: u32,
    skip_to: Option<usize>,
    hist: Vec<(usize, usize, usize)>,
    /// Can anything in this program write a capture slot?
    ///
    /// Group numbering starts at 1, so `capture_count == 1` means the compiler
    /// emitted no `Save` at all and every snapshot/restore of `captures` is
    /// dead work. Computed once per attempt rather than per branch.
    writes_caps: bool,
    /// Reused buffer for repetition end positions: one allocation per search
    /// instead of one per repeat entry.
    scratch: Vec<usize>,
    /// Class bitmaps are usable. They are rebuilt by `analyze` whenever the
    /// user-property set changes, so they cannot fall out of step.
    plans_ok: bool,
    /// Encoding facts, read once instead of re-derived per character.
    ascii_fast: bool,
    enc_min_len: usize,
    /// The search only offers line starts, so a `^` at the attempt's own start
    /// is already known to hold and need not be re-tested.
    bol_guaranteed: bool,
    /// Offset this attempt began at.
    attempt_start: usize,
    /// Current `run_stop` recursion depth.
    ///
    /// The VM recurses for alternation, look-around, atomic groups, subexp
    /// calls and a repeat's continuation -- so a flat chain like
    /// `a{1,2}a{1,2}...` is call depth too, not just nesting. The retry
    /// counters cannot catch that: they are counts, and the native stack runs
    /// out thousands of retries before a count limit set for pathological
    /// backtracking would fire.
    rdepth: u32,
}

// ---------------------------------------------------------------------------
// Search driver
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn search(
    prog: &Program,
    hay: &[u8],
    enc: Encoding,
    options: Options,
    start: usize,
    range: usize,
    param: &MatchParam,
    user_props: &[unicode::UserProperty],
) -> Result<Option<Region>, Error> {
    let end = range.min(hay.len());
    let mut pos = start.min(hay.len());

    let plain = !options.contains(Options::FIND_LONGEST)
        && !options.contains(Options::FIND_NOT_EMPTY)
        && !options.contains(Options::MATCH_WHOLE_STRING)
        && !options.contains(Options::IGNORECASE)
        && enc.min_len() == 1;
    if plain {
        if let Some(lit) = prog.ascii_literal.as_deref() {
            return search_ascii_literal(hay, pos, end, lit, prog, param);
        }
    }

    let mut best: Option<Region> = None;
    let whole = options.contains(Options::MATCH_WHOLE_STRING);
    let lead = prog.lead;
    let mut eng = Engine::new(prog, hay, enc, options, param, user_props, start);
    eng.bol_guaranteed = prog.anchored_bol
        && !options.contains(Options::NOTBOL)
        && !whole
        && enc.min_len() == 1;
    // `pos == end` is a legal start position, so a scan looking for candidate
    // STARTS must be able to reach it -- bounding at `end` made every filter
    // stop one position early. A match may also run past `end`, so anything
    // looking for bytes the match will CONSUME searches the whole haystack.
    let scan_end = end.saturating_add(1).min(hay.len());
    let lit_end = hay.len();
    let single_byte = enc.max_len() == 1;
    let find_longest = options.contains(Options::FIND_LONGEST);
    let find_not_empty = options.contains(Options::FIND_NOT_EMPTY);
    // Required-literal filter: a byte sequence every match must contain.
    // `req_q` caches the last occurrence found so the scan stays linear across
    // the whole search rather than restarting at every start position.
    let req = if whole || enc.min_len() != 1 {
        None
    } else {
        prog.req_lit.as_ref()
    };
    let mut req_q: Option<usize> = None;
    let bol_only = prog.anchored_bol && !whole && enc.min_len() == 1;
    // A `^`-anchored pattern whose required literal sits at the match start:
    // the candidate set is "line start followed by that literal", which one
    // fused loop settles without also running the generic literal scan.
    let bol_lit: Option<&[u8]> = if bol_only {
        req.filter(|r| r.min_dist == 0 && r.max_dist == Some(0))
            .map(|r| r.bytes.as_slice())
    } else {
        None
    };

    'outer: while pos <= end {
        super::count::tick_search_pos();
        if whole && pos != start {
            break;
        }
        // `^`-anchored: only positions after a newline can start a match.
        if bol_only {
            loop {
                if pos == 0 || hay.get(pos - 1) == Some(&b'\n') {
                    // On a line start. When the literal must sit here too,
                    // check it now instead of scanning for it separately.
                    match bol_lit {
                        Some(lit) => {
                            let stop_at = pos + lit.len();
                            if stop_at <= lit_end && &hay[pos..stop_at] == lit {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                match hay[pos..scan_end].iter().position(|&b| b == b'\n') {
                    Some(off) => {
                        super::count::tick_byte_scan(off as u64 + 1);
                        let n = pos + off + 1;
                        if n <= pos {
                            break 'outer;
                        }
                        pos = n;
                        if pos > end {
                            break 'outer;
                        }
                    }
                    None => {
                        super::count::tick_byte_scan((scan_end - pos) as u64);
                        break 'outer;
                    }
                }
            }
        }
        let bol_pos = pos;
        if bol_lit.is_none() {
        if let Some(rl) = req {
            let need_from = pos.saturating_add(rl.min_dist as usize);
            if need_from > lit_end {
                break;
            }
            if req_q.map(|q| q < need_from).unwrap_or(true) {
                super::count::tick_req_scan();
                req_q = super::optimize::find_bytes(&hay[need_from..lit_end], &rl.bytes)
                    .map(|off| need_from + off);
                if req_q.is_none() {
                    // The literal does not occur again, so nothing can match.
                    break;
                }
            }
            let q = match req_q {
                Some(q) => q,
                None => break,
            };
            // Earliest start that could still reach this occurrence.
            let floor = match rl.run_class {
                Some(cpc) => super::optimize::class_run_start(prog, cpc as usize, hay, pos, q),
                None => match rl.max_dist {
                    Some(d) => q.saturating_sub(d as usize),
                    None => pos,
                },
            };
            if floor > pos {
                super::count::tick_req_skip((floor - pos) as u64);
                pos = floor;
                // The literal may sit past the last legal start position.
                if pos > end {
                    break;
                }
            }
        }
        }
        // Skip start positions no match can begin at. When the fused
        // line-start path already matched the required literal here, the first
        // byte is known good and this test is dead work.
        if let (None, Some(ref lead)) = (bol_lit, lead) {
            if !whole && enc.min_len() == 1 && pos < hay.len() && !lead.contains(hay[pos]) {
                if pos >= scan_end {
                    break;
                }
                match scan_lead(&hay[pos..scan_end], lead) {
                    Some(off) => {
                        super::count::tick_byte_scan(off as u64);
                        pos += off;
                    }
                    None => {
                        super::count::tick_byte_scan((scan_end - pos) as u64);
                        break;
                    }
                }
            }
        }
        // The later filters may have moved `pos` off the line start the Bol
        // scan established. A non-line-start cannot match an anchored pattern,
        // so go back and rescan rather than attempting here -- and only then
        // is `bol_guaranteed` sound.
        if bol_only && pos != bol_pos && pos != 0 && hay[pos - 1] != b'\n' {
            continue 'outer;
        }
        let hit = eng.attempt(pos)?;
        let skip_to = eng.skip_to;
        match hit {
            Some(r) => {
                if whole && r.range().end != hay.len() {
                    let n = if single_byte { pos + 1 } else { next_pos(enc, hay, pos) };
                    if n <= pos {
                        break;
                    }
                    pos = n;
                    continue;
                }
                if find_not_empty && r.is_empty_match() {
                    let n = if single_byte { pos + 1 } else { next_pos(enc, hay, pos) };
                    if n <= pos {
                        break;
                    }
                    pos = n;
                    continue;
                }
                if find_longest {
                    let better = best
                        .as_ref()
                        .map(|b| {
                            let br = b.range();
                            let rr = r.range();
                            (rr.end - rr.start) > (br.end - br.start)
                        })
                        .unwrap_or(true);
                    if better {
                        best = Some(r);
                    }
                    let n = if single_byte { pos + 1 } else { next_pos(enc, hay, pos) };
                    if n <= pos {
                        break;
                    }
                    pos = n;
                    continue;
                }
                return Ok(Some(r));
            }
            None => {
                if let Some(s) = skip_to {
                    // Resume AT the position (*SKIP) reached, not one past it,
                    // but always make at least one character of progress.
                    let n = s.max(next_pos(enc, hay, pos));
                    if n <= pos {
                        break;
                    }
                    pos = n;
                    continue;
                }
                if pos == end {
                    break;
                }
                let n = if single_byte { pos + 1 } else { next_pos(enc, hay, pos) };
                if n <= pos {
                    break;
                }
                pos = n;
            }
        }
    }
    Ok(best)
}

#[allow(clippy::too_many_arguments)]
pub fn match_at(
    prog: &Program,
    hay: &[u8],
    enc: Encoding,
    options: Options,
    at: usize,
    search_origin: usize,
    param: &MatchParam,
    user_props: &[unicode::UserProperty],
) -> Result<Option<Region>, Error> {
    let mut eng = Engine::new(prog, hay, enc, options, param, user_props, search_origin);
    eng.attempt(at)
}

impl<'a> Engine<'a> {
    /// Build one engine for a whole search.
    ///
    /// Constructing this per start position -- with its capture vector -- was
    /// 342_852 allocations across the benchmark workload, 92% of them for
    /// attempts that then failed.
    #[allow(clippy::too_many_arguments)]
    fn new(
        prog: &'a Program,
        hay: &'a [u8],
        enc: Encoding,
        options: Options,
        param: &'a MatchParam,
        user_props: &'a [unicode::UserProperty],
        search_origin: usize,
    ) -> Self {
        super::count::tick_engine_new();
        Engine {
            hay,
            enc,
            prog,
            options,
            base_options: options,
            param,
            user_props,
            captures: Caps::new(prog.capture_count * 2),
            option_stack: Vec::new(),
            keep: 0,
            search_origin,
            hay_start: 0,
            hay_end: hay.len(),
            retry_match: 0,
            retry_search: 0,
            calls: 0,
            skip_to: None,
            hist: Vec::new(),
            writes_caps: prog.capture_count > 1,
            scratch: Vec::new(),
            plans_ok: true,
            ascii_fast: enc.ascii_transparent(),
            enc_min_len: enc.min_len(),
            bol_guaranteed: false,
            attempt_start: 0,
            rdepth: 0,
        }
    }

    /// Reset the per-attempt state and match at `at`.
    ///
    /// `retry_search` deliberately survives: it is a search-wide budget.
    fn reset_at(&mut self, at: usize) {
        for slot in self.captures.iter_mut() {
            *slot = None;
        }
        if !self.captures.is_empty() {
            self.captures[0] = Some(at);
        }
        self.option_stack.clear();
        self.hist.clear();
        self.options = self.base_options;
        self.hay_start = 0;
        self.hay_end = self.hay.len();
        self.keep = at;
        self.attempt_start = at;
        self.rdepth = 0;
        self.retry_match = 0;
        self.calls = 0;
        self.skip_to = None;
    }

    fn attempt(&mut self, at: usize) -> Result<Option<Region>, Error> {
        self.reset_at(at);
        let ok = self.run(0, at)?;
        match ok {
            Some(end) => {
                if self.captures.len() > 1 {
                    self.captures[1] = Some(end);
                    self.captures[0] = Some(self.keep);
                }
                Ok(Some(self.to_region()))
            }
            None => Ok(None),
        }
    }
}

fn next_pos(enc: Encoding, hay: &[u8], pos: usize) -> usize {
    super::count::tick_next_pos();
    if pos >= hay.len() {
        return hay.len();
    }
    if enc.max_len() == 1 {
        return pos + 1;
    }
    if hay[pos] < 0x80 && enc.min_len() == 1 {
        return pos + 1;
    }
    match enc.mbc_len(&hay[pos..]) {
        Ok(n) if n > 0 => pos + n,
        _ => pos + 1,
    }
}

// ---------------------------------------------------------------------------
// Literal fast path
// ---------------------------------------------------------------------------

fn ascii_literal_program(prog: &Program) -> Option<Vec<u8>> {
    match prog.insts.as_slice() {
        [Inst::Literal(v), Inst::Match] => {
            if v.iter().all(|c| *c <= 0x7F) {
                Some(v.iter().map(|c| *c as u8).collect())
            } else {
                None
            }
        }
        [Inst::Char(c), Inst::Match] if *c <= 0x7F => Some(alloc::vec![*c as u8]),
        _ => None,
    }
}

fn search_ascii_literal(
    hay: &[u8],
    start: usize,
    end: usize,
    lit: &[u8],
    prog: &Program,
    param: &MatchParam,
) -> Result<Option<Region>, Error> {
    super::count::tick_search_pos();
    let _ = param;
    if lit.is_empty() || start > end {
        return Ok(None);
    }
    let first = lit[0];
    let mut p = start;
    let limit = end.min(hay.len());
    while p + lit.len() <= limit {
        match hay[p..limit].iter().position(|&b| b == first) {
            Some(off) => {
                super::count::tick_byte_scan(off as u64 + 1);
                p += off;
                if p + lit.len() > limit {
                    break;
                }
                if &hay[p..p + lit.len()] == lit {
                    let mut r = Region::with_names(prog.capture_count, prog.has_named);
                    if !r.captures.is_empty() {
                        r.captures[0] = Some(p..p + lit.len());
                    }
                    if prog.has_named {
                        for (i, n) in prog.names.iter().enumerate() {
                            if let Some(slot) = r.names.get_mut(i) {
                                *slot = n.clone();
                            }
                        }
                    }
                    return Ok(Some(r));
                }
                p += 1;
            }
            None => {
                super::count::tick_byte_scan((limit - p) as u64);
                break;
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// First-byte prefilter
// ---------------------------------------------------------------------------

/// Offset of the first byte in `hay` that `lead` admits.
///
/// Split so the single-byte case keeps the equality-compare loop LLVM already
/// turns into a vector scan; the general case tests the 256-bit set.
#[inline]
fn scan_lead(hay: &[u8], lead: &Lead) -> Option<usize> {
    match lead.single {
        Some(fb) => hay.iter().position(|&b| b == fb),
        None => hay.iter().position(|&b| lead.contains(b)),
    }
}

/// Can this class match any codepoint at or above U+0080?
///
/// Conservative on purpose: `true` when we cannot cheaply prove otherwise, so
/// the prefilter over-approximates rather than dropping a match.
fn class_may_be_non_ascii(cc: &CharClass) -> bool {
    if cc.negate {
        return true;
    }
    cc.items.iter().any(|item| match item {
        ClassItem::Char(c) => *c >= 0x80,
        ClassItem::Range(_, b) => *b >= 0x80,
        ClassItem::Nested(inner) => class_may_be_non_ascii(inner),
        // Posix/Prop/Word/Digit/Space/Xdigit are Unicode-aware under a
        // multi-byte encoding; Intersect we do not model. Assume non-ASCII.
        _ => true,
    })
}

/// Add the byte(s) a single codepoint could start with.
fn insert_char_lead(lead: &mut Lead, c: u32, enc: Encoding, options: Options) {
    let icase = options.contains(Options::IGNORECASE);
    if c > 0x7F || enc.min_len() != 1 {
        lead.insert_non_ascii();
        if icase || enc.min_len() != 1 {
            for b in 0..=0x7Fu8 {
                lead.insert(b);
            }
        }
        return;
    }
    let b = c as u8;
    lead.insert(b);
    if icase {
        lead.insert(b.to_ascii_lowercase());
        lead.insert(b.to_ascii_uppercase());
        // A non-ASCII character can case-fold onto an ASCII one.
        if !options.contains(Options::IGNORECASE_IS_ASCII) {
            lead.insert_non_ascii();
        }
    }
}

/// Walk the program from `pc`, unioning every byte a match could start with.
///
/// Returns `false` the moment it cannot bound the set, and the caller then
/// ships no prefilter at all. It must never under-approximate: a missing byte
/// silently drops a match, while a spare byte only costs a wasted attempt.
///
/// This walks *through* instructions that consume nothing -- `Save`, anchors,
/// option pushes, look-around -- and unions across `Split`. Looking only at
/// the single instruction at `pc` leaves every capturing, alternating,
/// anchored or case-insensitive pattern with no prefilter at all.
#[allow(clippy::too_many_arguments)]
fn lead_walk(
    prog: &Program,
    pc: usize,
    enc: Encoding,
    options: Options,
    user_props: &[unicode::UserProperty],
    lead: &mut Lead,
    fuel: &mut u32,
    depth: u32,
) -> bool {
    if depth > 24 {
        return false;
    }
    let mut pc = pc;
    let mut options = options;
    loop {
        if *fuel == 0 {
            return false;
        }
        *fuel -= 1;
        let inst = match prog.insts.get(pc) {
            Some(i) => i,
            None => return false,
        };
        match inst {
            // Zero-width: the first byte comes from further on.
            Inst::Nop
            | Inst::Save(_)
            | Inst::Keep
            | Inst::Assert(_)
            | Inst::Callout { .. }
            | Inst::PopOptions => pc += 1,
            Inst::PushOptions(set, clear) => {
                options = options.union(*set).difference(*clear);
                pc += 1;
            }
            // A path that cannot match contributes no bytes.
            Inst::Fail => return true,
            // The empty string matches here, so any byte can start a match.
            Inst::Match => return false,
            Inst::Char(c) => {
                insert_char_lead(lead, *c, enc, options);
                return true;
            }
            Inst::Literal(v) => match v.first() {
                Some(c) => {
                    insert_char_lead(lead, *c, enc, options);
                    return true;
                }
                None => pc += 1,
            },
            Inst::Class { class } => {
                // Exact over ASCII: probe the same matcher the VM will run,
                // under the options in force here, so IGNORECASE is handled
                // by the matcher rather than guessed at.
                for cp in 0u32..0x80 {
                    if class_hit_in(class, cp, enc, options, user_props) {
                        lead.insert(cp as u8);
                    }
                }
                if class_may_be_non_ascii(class) || enc.min_len() != 1 {
                    lead.insert_non_ascii();
                }
                return true;
            }
            Inst::Any { .. } | Inst::SuperAny | Inst::GeneralNewline | Inst::TextSegment => {
                return false
            }
            Inst::Jump(j) => {
                let j = *j as usize;
                if j <= pc {
                    return false;
                }
                pc = j;
            }
            Inst::Split(a, b) => {
                let (a, b) = (*a as usize, *b as usize);
                return lead_walk(prog, a, enc, options, user_props, lead, fuel, depth + 1)
                    && lead_walk(prog, b, enc, options, user_props, lead, fuel, depth + 1);
            }
            Inst::Repeat {
                body, after, min, ..
            } => {
                let (body, after, min) = (*body as usize, *after as usize, *min);
                if !lead_walk(prog, body, enc, options, user_props, lead, fuel, depth + 1) {
                    return false;
                }
                if min >= 1 {
                    return true;
                }
                // It may match zero times, so what follows can start too.
                pc = after;
            }
            // Look-around consumes nothing.
            Inst::Look { after, .. } => pc = *after as usize,
            // An atomic body may match empty, so union it with what follows.
            Inst::Atomic { body, after } => {
                let (body, after) = (*body as usize, *after as usize);
                if !lead_walk(prog, body, enc, options, user_props, lead, fuel, depth + 1) {
                    return false;
                }
                pc = after;
            }
            // Backrefs, calls, conditionals and absent spans could start with
            // almost anything.
            _ => return false,
        }
    }
}

/// Precompute everything about the program that never changes at match time.
///
/// Each of these was previously re-derived on the hot path: the repeat shape
/// on every repeat entry (341_832 body scans across the benchmark workload),
/// the literal fast path on every `search()` call (with an allocation), and a
/// group's Save-pair span on every subexp call (an O(program) scan each time).
pub(crate) fn analyze(
    prog: &mut Program,
    enc: Encoding,
    options: Options,
    user_props: &[unicode::UserProperty],
) {
    let n = prog.insts.len();
    let mut shapes = alloc::vec![RepeatShape::default(); n];
    for pc in 0..n {
        if let Inst::Repeat { body, after, .. } = prog.insts[pc] {
            let (body, after) = (body as usize, after as usize);
            if body < n {
                shapes[body] = RepeatShape {
                    writes_caps: prog_body_writes_captures(prog, body, after),
                    simple: simple_body(prog, body, after),
                    follow_disjoint: false,
                    follow_byte: None,
                    follow_icase: false,
                };
            }
        }
    }
    prog.repeat_shapes = shapes;
    prog.ascii_literal = ascii_literal_program(prog);
    let groups = prog.capture_count;
    let mut spans = alloc::vec![None; groups];
    for (idx, slot) in spans.iter_mut().enumerate().skip(1) {
        *slot = group_pc_span_of(prog, idx).map(|(a, b)| (a as u16, b as u16));
    }
    prog.group_spans = spans;

    // Lower every class to an ASCII bitmap under the pattern's base options.
    // The VM only uses a plan when no inline option change is active and no
    // user property is registered, so this can never disagree with the
    // matcher it replaces.
    // Track the options in force at each pc so a class under an inline
    // `(?i)` still gets a bitmap -- built under the options that will actually
    // be live when the VM reaches it.
    let mut plans = alloc::vec![None; n];
    let mut cur = options;
    let mut stack: Vec<Options> = Vec::new();
    for pc in 0..n {
        match &prog.insts[pc] {
            Inst::PushOptions(set, clear) => {
                stack.push(cur);
                cur = cur.union(*set).difference(*clear);
            }
            Inst::PopOptions => {
                if let Some(o) = stack.pop() {
                    cur = o;
                }
            }
            Inst::Class { class } => {
                let mut ascii = [0u64; 2];
                for cp in 0u32..0x80 {
                    if class_hit_in(class, cp, enc, cur, user_props) {
                        ascii[(cp >> 6) as usize] |= 1u64 << (cp & 63);
                    }
                }
                plans[pc] = Some(ClassPlan {
                    ascii,
                    options: cur,
                });
            }
            _ => {}
        }
    }
    prog.class_plans = plans;

    let mut lits: Vec<Option<Vec<u8>>> = alloc::vec![None; n];
    if enc.min_len() == 1 {
        for pc in 0..n {
            if let Inst::Literal(v) = &prog.insts[pc] {
                if v.iter().all(|c| *c <= 0x7F) {
                    lits[pc] = Some(v.iter().map(|c| *c as u8).collect());
                }
            }
        }
    }
    prog.literal_bytes = lits;

    // Needs the class plans, so it runs after them.
    let mut shapes = core::mem::take(&mut prog.repeat_shapes);
    for pc in 0..n {
        if let Inst::Repeat { body, after, .. } = prog.insts[pc] {
            let (body, after) = (body as usize, after as usize);
            if let Some(sh) = shapes.get_mut(body) {
                if let SimpleBody::Class(cpc) = sh.simple {
                    sh.follow_disjoint = follow_is_disjoint(prog, cpc as usize, after);
                    if let Some((b, icase)) = follow_byte_of(prog, cpc as usize, after) {
                        sh.follow_byte = Some(b);
                        sh.follow_icase = icase;
                    }
                }
            }
        }
    }
    prog.repeat_shapes = shapes;

    prog.req_lit = super::optimize::required_literal(prog, enc.min_len() == 1, options);
    prog.anchored_bol = starts_with_bol(prog);
}

/// Does every match have to start at a line beginning?
///
/// True when a `Bol` assert precedes any consuming instruction on the spine.
fn starts_with_bol(prog: &Program) -> bool {
    let mut pc = 0usize;
    let mut fuel = 64;
    while fuel > 0 {
        fuel -= 1;
        match prog.insts.get(pc) {
            Some(Inst::Assert(Anchor::Bol)) => return true,
            Some(Inst::Nop)
            | Some(Inst::Save(_))
            | Some(Inst::Keep)
            | Some(Inst::PushOptions(_, _))
            | Some(Inst::PopOptions) => pc += 1,
            _ => return false,
        }
    }
    false
}

/// First byte of the character required immediately after the repetition.
fn follow_byte_of(prog: &Program, class_pc: usize, after: usize) -> Option<(u8, bool)> {
    let icase = prog
        .class_plans
        .get(class_pc)
        .copied()
        .flatten()
        .map(|p| p.options.contains(Options::IGNORECASE))
        .unwrap_or(false);
    let mut pc = after;
    let mut fuel = 32;
    while fuel > 0 {
        fuel -= 1;
        match prog.insts.get(pc) {
            Some(Inst::Nop) | Some(Inst::Save(_)) | Some(Inst::Keep) => pc += 1,
            Some(Inst::Jump(j)) => {
                let j = *j as usize;
                if j <= pc {
                    return None;
                }
                pc = j;
            }
            Some(Inst::Char(c)) if *c < 0x80 => return Some((*c as u8, icase)),
            Some(Inst::Literal(v)) => {
                return match v.first() {
                    Some(c) if *c < 0x80 => Some((*c as u8, icase)),
                    _ => None,
                }
            }
            _ => return None,
        }
    }
    None
}

/// Is the next consuming instruction after `after` a character the class at
/// `class_pc` provably cannot match?
fn follow_is_disjoint(prog: &Program, class_pc: usize, after: usize) -> bool {
    let plan = match prog.class_plans.get(class_pc).copied().flatten() {
        Some(p) => p,
        None => return false,
    };
    // Case folding would widen what the follow character can be.
    if plan.options.contains(Options::IGNORECASE) {
        return false;
    }
    let mut pc = after;
    let mut fuel = 32;
    while fuel > 0 {
        fuel -= 1;
        match prog.insts.get(pc) {
            // Zero-width: keep looking.
            Some(Inst::Nop) | Some(Inst::Save(_)) | Some(Inst::Keep) => pc += 1,
            Some(Inst::Jump(j)) => {
                let j = *j as usize;
                if j <= pc {
                    return false;
                }
                pc = j;
            }
            Some(Inst::Char(c)) => return *c < 0x80 && !plan.hit(*c),
            Some(Inst::Literal(v)) => {
                return match v.first() {
                    Some(c) => *c < 0x80 && !plan.hit(*c),
                    None => false,
                }
            }
            _ => return false,
        }
    }
    false
}

fn prog_body_writes_captures(prog: &Program, body: usize, after: usize) -> bool {
    let mut pc = body;
    while pc < after && pc < prog.insts.len() {
        match &prog.insts[pc] {
            Inst::Save(_)
            | Inst::Call(_)
            | Inst::Backref(_)
            | Inst::Cond { .. }
            | Inst::Absent { .. } => return true,
            _ => {}
        }
        pc += 1;
    }
    false
}

fn simple_body(prog: &Program, body: usize, after: usize) -> SimpleBody {
    let tail_ok = match prog.insts.get(body + 1) {
        Some(Inst::Jump(j)) => *j as usize == after,
        _ => body + 1 == after,
    };
    if !tail_ok {
        return SimpleBody::None;
    }
    match prog.insts.get(body) {
        Some(Inst::Char(c)) => SimpleBody::Char(*c),
        Some(Inst::Class { .. }) => SimpleBody::Class(body as u16),
        _ => SimpleBody::None,
    }
}

/// `[open Save, close Save]` instruction span of group `idx`.
fn group_pc_span_of(prog: &Program, idx: usize) -> Option<(usize, usize)> {
    let mut start_pc = None;
    let mut depth: i32 = 0;
    for (i, inst) in prog.insts.iter().enumerate() {
        if let Inst::Save(s) = inst {
            if *s as usize == idx * 2 {
                if depth == 0 {
                    start_pc = Some(i);
                }
                depth += 1;
            }
            if *s as usize == idx * 2 + 1 {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(st) = start_pc {
                        return Some((st, i));
                    }
                }
            }
        }
    }
    None
}

/// Build the first-byte prefilter for `prog`, or `None` when none is useful.
///
/// Recomputed when a user property is registered, since that can change what
/// an ASCII codepoint matches.
pub(crate) fn compute_lead(
    prog: &Program,
    enc: Encoding,
    options: Options,
    user_props: &[unicode::UserProperty],
) -> Option<Lead> {
    // A sub-byte-aligned encoding cannot be byte-scanned at all.
    if enc.min_len() != 1 {
        return None;
    }
    let mut lead = Lead::empty();
    let mut fuel = 4096u32;
    if !lead_walk(prog, 0, enc, options, user_props, &mut lead, &mut fuel, 0) {
        return None;
    }
    lead.finish()
}

// ---------------------------------------------------------------------------
// Class membership, shared by the VM and the prefilter
// ---------------------------------------------------------------------------

/// Class membership, independent of any live [`Engine`].
///
/// The VM and the first-byte prefilter share this one definition so a
/// prefilter can never disagree with the matcher it is filtering for.
pub(crate) fn class_hit_in(
    cc: &CharClass,
    cp: u32,
    enc: Encoding,
    opt: Options,
    user_props: &[unicode::UserProperty],
) -> bool {
    let mut ok = false;
    for item in &cc.items {
        match item {
            ClassItem::Intersect(rest) => {
                let left = ok;
                let right_cc = CharClass {
                    negate: false,
                    items: rest.clone(),
                };
                let right = class_hit_in(&right_cc, cp, enc, opt, user_props);
                ok = left && right;
            }
            other => {
                if item_hit_in(other, cp, enc, opt, user_props) {
                    ok = true;
                }
            }
        }
    }
    if cc.negate {
        !ok
    } else {
        ok
    }
}

fn item_hit_in(
    item: &ClassItem,
    cp: u32,
    enc: Encoding,
    opt: Options,
    user_props: &[unicode::UserProperty],
) -> bool {
    match item {
        ClassItem::Char(c) => {
            if *c == cp {
                return true;
            }
            if opt.contains(Options::IGNORECASE) {
                fold_eq(*c, cp, opt.contains(Options::IGNORECASE_IS_ASCII))
            } else {
                false
            }
        }
        ClassItem::Range(a, b) => {
            if (*a..=*b).contains(&cp) {
                return true;
            }
            if opt.contains(Options::IGNORECASE) {
                let f = fold_cp(cp, opt.contains(Options::IGNORECASE_IS_ASCII));
                if (*a..=*b).contains(&f) {
                    return true;
                }
                let u = unfold_up(cp);
                (*a..=*b).contains(&u)
            } else {
                false
            }
        }
        ClassItem::Posix { name, neg } => unicode::posix(name, enc, opt, cp) != *neg,
        ClassItem::Prop { name, neg } => {
            let mut hit = unicode::property(name, enc, opt, cp);
            for u in user_props {
                if u.name == *name {
                    hit = u.contains(cp);
                }
            }
            hit != *neg
        }
        ClassItem::Word { neg } => unicode::is_word(enc, opt, cp) != *neg,
        ClassItem::Digit { neg } => unicode::is_digit(enc, opt, cp) != *neg,
        ClassItem::Space { neg } => unicode::is_space(enc, opt, cp) != *neg,
        ClassItem::Xdigit { neg } => unicode::is_xdigit(cp) != *neg,
        ClassItem::Nested(cc) => class_hit_in(cc, cp, enc, opt, user_props),
        ClassItem::Intersect(_) => false,
    }
}

fn fold_cp(cp: u32, ascii_only: bool) -> u32 {
    if cp < 0x80 {
        return u32::from((cp as u8).to_ascii_lowercase());
    }
    if ascii_only {
        return cp;
    }
    match char::from_u32(cp) {
        Some(c) => {
            let mut it = c.to_lowercase();
            match (it.next(), it.next()) {
                (Some(one), None) => one as u32,
                _ => cp,
            }
        }
        None => cp,
    }
}

/// Upper-case counterpart, for range membership under IGNORECASE.
fn unfold_up(cp: u32) -> u32 {
    if cp < 0x80 {
        return u32::from((cp as u8).to_ascii_uppercase());
    }
    match char::from_u32(cp) {
        Some(c) => {
            let mut it = c.to_uppercase();
            match (it.next(), it.next()) {
                (Some(one), None) => one as u32,
                _ => cp,
            }
        }
        None => cp,
    }
}

fn fold_eq(a: u32, b: u32, ascii_only: bool) -> bool {
    a == b || fold_cp(a, ascii_only) == fold_cp(b, ascii_only)
}

// ---------------------------------------------------------------------------
// The VM
// ---------------------------------------------------------------------------

impl<'a> Engine<'a> {
    fn opt(&self) -> Options {
        self.options
    }

    fn bump_retry(&mut self) -> Result<(), Error> {
        self.bump_retry_n(1)
    }

    fn bump_retry_n(&mut self, n: u64) -> Result<(), Error> {
        if n == 0 {
            return Ok(());
        }
        super::count::tick_bump();
        self.retry_match = self.retry_match.saturating_add(n);
        self.retry_search = self.retry_search.saturating_add(n);
        if self.param.retry_limit_in_match != 0 && self.retry_match > self.param.retry_limit_in_match
        {
            return Err(Error::kind_msg(
                ErrorKind::RetryLimitMatch,
                "retry limit in match",
            ));
        }
        if self.param.retry_limit_in_search != 0
            && self.retry_search > self.param.retry_limit_in_search
        {
            return Err(Error::kind_msg(
                ErrorKind::RetryLimitSearch,
                "retry limit in search",
            ));
        }
        Ok(())
    }

    /// Bound an outstanding repetition trail with the documented match-stack
    /// limit. A push/pop counter around one body iteration cannot do this job:
    /// it never reflects how many iterations are still outstanding.
    fn check_repeat_depth(&self, depth: usize) -> Result<(), Error> {
        if self.param.stack_limit != 0 && depth as u64 > u64::from(self.param.stack_limit) {
            return Err(Error::kind_msg(
                ErrorKind::MatchStackLimit,
                "match stack limit",
            ));
        }
        Ok(())
    }

    /// Record where `(*SKIP)` was reached; the furthest one wins.
    fn note_skip(&mut self, pos: usize) {
        self.skip_to = Some(match self.skip_to {
            Some(s) => s.max(pos),
            None => pos,
        });
    }

    fn run(&mut self, pc: usize, pos: usize) -> Result<Option<usize>, Error> {
        self.run_stop(pc, pos, None)
    }

    fn run_until(&mut self, start: usize, end: usize, pos: usize) -> Result<Option<usize>, Error> {
        self.run_stop(start, pos, Some(end))
    }

    /// Maximum VM recursion depth.
    ///
    /// A native-stack bound, deliberately well under the measured ceiling
    /// (~1000 frames on a 1 MB stack) so callers on smaller stacks are safe
    /// too. Exceeding it is a `MatchStackLimit` error, never an abort.
    const MAX_RUN_DEPTH: u32 = 300;

    fn run_stop(
        &mut self,
        pc: usize,
        pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        self.rdepth += 1;
        if self.rdepth > Self::MAX_RUN_DEPTH {
            self.rdepth -= 1;
            return Err(Error::kind_msg(
                ErrorKind::MatchStackLimit,
                "match recursion depth",
            ));
        }
        let r = self.run_stop_inner(pc, pos, stop);
        self.rdepth -= 1;
        r
    }

    fn run_stop_inner(
        &mut self,
        mut pc: usize,
        mut pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        // Reborrow the program at 'a so instruction operands (notably a
        // CharClass) do not borrow `self`; otherwise every dispatch needing
        // `&mut self` would have to clone its operand first.
        let prog: &'a Program = self.prog;
        loop {
            super::count::tick_vm();
            if stop == Some(pc) {
                return Ok(Some(pos));
            }
            let inst = match prog.insts.get(pc) {
                Some(i) => i,
                None => return Ok(None),
            };
            match inst {
                Inst::Nop => pc += 1,
                Inst::Match => return Ok(Some(pos)),
                Inst::Fail => return Ok(None),
                Inst::Char(c) => match self.consume_char(pos, *c)? {
                    Some(n) => {
                        pos = n;
                        pc += 1;
                    }
                    None => return Ok(None),
                },
                Inst::Literal(v) => {
                    let fast = if self.options.contains(Options::IGNORECASE) {
                        None
                    } else {
                        prog.literal_bytes.get(pc).and_then(|o| o.as_deref())
                    };
                    if let Some(lit) = fast {
                        // memcmp, rather than decode-and-compare per character.
                        let end = pos + lit.len();
                        if end > self.hay_end || &self.hay[pos..end] != lit {
                            return Ok(None);
                        }
                        pos = end;
                    } else {
                        let mut p = pos;
                        for c in v.iter() {
                            match self.consume_char(p, *c)? {
                                Some(n) => p = n,
                                None => return Ok(None),
                            }
                        }
                        pos = p;
                    }
                    pc += 1;
                }
                Inst::Any { newline } => match self.consume_any(pos, *newline, false)? {
                    Some(n) => {
                        pos = n;
                        pc += 1;
                    }
                    None => return Ok(None),
                },
                Inst::SuperAny => match self.consume_any(pos, true, true)? {
                    Some(n) => {
                        pos = n;
                        pc += 1;
                    }
                    None => return Ok(None),
                },
                Inst::Class { class } => {
                    let plan = prog.class_plans.get(pc).copied().flatten();
                    match self.consume_class(pos, class, plan)? {
                        Some(n) => {
                            pos = n;
                            pc += 1;
                        }
                        None => return Ok(None),
                    }
                }
                Inst::Split(a, b) => {
                    super::count::tick_split_step();
                    self.bump_retry()?;
                    let (a, b) = (*a as usize, *b as usize);
                    let saved = if self.writes_caps {
                        super::count::tick_cap_clone();
                        Some((self.captures.clone(), self.hist.len()))
                    } else {
                        None
                    };
                    if let Some(r) = self.run_stop(a, pos, stop)? {
                        return Ok(Some(r));
                    }
                    if let Some((saved, hlen)) = saved {
                        self.captures = saved;
                        // Drop capture-history recorded by the abandoned
                        // branch; it describes captures the match never made.
                        self.hist.truncate(hlen);
                    }
                    pc = b;
                }
                Inst::Jump(j) => pc = *j as usize,
                Inst::Save(s) => {
                    let s = *s as usize;
                    if s < self.captures.len() {
                        self.captures[s] = Some(pos);
                        if s % 2 == 1 {
                            let g = s / 2;
                            if prog.history_groups.get(g).copied().unwrap_or(false) {
                                if let Some(st) = self.captures.get(s - 1).copied().flatten() {
                                    self.hist.push((g, st, pos));
                                }
                            }
                        }
                    }
                    pc += 1;
                }
                Inst::Assert(a) => {
                    let a = *a;
                    if self.anchor(a, pos)? {
                        pc += 1;
                    } else {
                        return Ok(None);
                    }
                }
                Inst::Repeat {
                    body,
                    after,
                    min,
                    max,
                    greedy,
                    possessive,
                } => {
                    let (body, after) = (*body as usize, *after as usize);
                    let (min, max) = (*min, *max);
                    let (greedy, possessive) = (*greedy, *possessive);
                    return self.repeat(body, after, min, max, greedy, possessive, pos, stop);
                }
                Inst::Look {
                    body,
                    after,
                    behind,
                    negative,
                } => {
                    let (body, after) = (*body as usize, *after as usize);
                    let (behind, negative) = (*behind, *negative);
                    let ok = self.look(body, after, behind, pos)?;
                    if ok == negative {
                        return Ok(None);
                    }
                    pc = after;
                }
                Inst::Atomic { body, after } => {
                    let (body, after) = (*body as usize, *after as usize);
                    match self.run_until(body, after, pos)? {
                        Some(n) => {
                            pos = n;
                            pc = after;
                        }
                        None => return Ok(None),
                    }
                }
                Inst::Backref(b) => match self.backref(b, pos)? {
                    Some(n) => {
                        pos = n;
                        pc += 1;
                    }
                    None => return Ok(None),
                },
                Inst::Call(t) => {
                    self.calls += 1;
                    if self.param.subexp_call_limit != 0
                        && self.calls > self.param.subexp_call_limit
                    {
                        return Err(Error::kind_msg(
                            ErrorKind::SubexpCallLimit,
                            "subexp-call limit",
                        ));
                    }
                    match self.subexp_call(t, pos, pc)? {
                        Some(n) => {
                            pos = n;
                            pc += 1;
                        }
                        None => return Ok(None),
                    }
                }
                Inst::Keep => {
                    self.keep = pos;
                    pc += 1;
                }
                Inst::Absent {
                    stopper,
                    expr,
                    after,
                    kind,
                } => {
                    let stopper = *stopper as usize;
                    let expr = expr.map(|e| e as usize);
                    let after = *after as usize;
                    let kind = *kind;
                    // `absent` runs the continuation itself -- the repeater has
                    // to be able to give bytes back so the rest of the pattern
                    // can match -- so its result is final.
                    return self.absent(stopper, expr, after, kind, pos, stop);
                }
                Inst::Cond {
                    cond,
                    then_pc,
                    else_pc,
                    after,
                } => {
                    let then_pc = *then_pc as usize;
                    let else_pc = else_pc.map(|e| e as usize);
                    let after = *after as usize;
                    if self.cond_true(cond, pos)? {
                        pc = then_pc;
                    } else {
                        pc = else_pc.unwrap_or(after);
                    }
                }
                Inst::GeneralNewline => match self.consume_r(pos)? {
                    Some(n) => {
                        pos = n;
                        pc += 1;
                    }
                    None => return Ok(None),
                },
                Inst::TextSegment => match self.consume_x(pos)? {
                    Some(n) => {
                        pos = n;
                        pc += 1;
                    }
                    None => return Ok(None),
                },
                Inst::Callout {
                    named,
                    name,
                    args,
                    tag,
                    body,
                    dir,
                } => {
                    let named = *named;
                    let dir = *dir;
                    if named {
                        match name.as_str() {
                            // (*FAIL) / (*MISMATCH) fail where they stand.
                            "FAIL" | "MISMATCH" => return Ok(None),
                            // (*SKIP) SUCCEEDS going forward. It only records
                            // where it was reached; if the whole attempt at
                            // this start then fails, the search resumes there
                            // instead of at start + 1. Failing here would
                            // break `(?:a(*SKIP)x|ab)`, which matches 0..2.
                            "SKIP" => {
                                self.note_skip(pos);
                                pc += 1;
                                continue;
                            }
                            "COUNT" | "TOTAL_COUNT" => {
                                self.param
                                    .count
                                    .set(self.param.count.get().saturating_add(1));
                                pc += 1;
                                continue;
                            }
                            "ERROR" => {
                                return Err(Error::kind_msg(
                                    ErrorKind::InvalidArgument,
                                    "(*ERROR) callout",
                                ))
                            }
                            _ => {}
                        }
                    }
                    let ctx = CalloutCtx {
                        name,
                        args,
                        tag: tag.as_deref(),
                        body,
                        haystack: self.hay,
                        current: pos,
                        dir,
                    };
                    let hook = if named {
                        self.param.named_callout
                    } else if dir == CalloutDir::Retraction {
                        self.param.retraction_callout
                    } else {
                        self.param.progress_callout
                    };
                    match hook.map(|f| f(&ctx)).unwrap_or(CalloutResult::Success) {
                        CalloutResult::Success => pc += 1,
                        CalloutResult::Fail => return Ok(None),
                        CalloutResult::Skip => {
                            self.note_skip(pos);
                            pc += 1;
                        }
                    }
                }
                Inst::PushOptions(set, clear) => {
                    self.option_stack.push(self.options);
                    self.options = self.options.union(*set).difference(*clear);
                    pc += 1;
                }
                Inst::PopOptions => {
                    if let Some(o) = self.option_stack.pop() {
                        self.options = o;
                    }
                    pc += 1;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Repetition
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &mut self,
        body: usize,
        after: usize,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        possessive: bool,
        pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        let shape = self
            .prog
            .repeat_shapes
            .get(body)
            .copied()
            .unwrap_or_default();
        let writes = shape.writes_caps;
        if !writes && !possessive {
            match shape.simple {
                SimpleBody::Char(want) => {
                    return self.repeat_simple_char(want, after, min, max, greedy, pos, stop)
                }
                SimpleBody::Class(cpc) => {
                    return self.repeat_simple_class(
                        cpc as usize,
                        after,
                        min,
                        max,
                        greedy,
                        pos,
                        stop,
                    )
                }
                SimpleBody::None => {}
            }
        }
        self.repeat_rec(body, after, min, max, greedy, possessive, pos, 0, writes, stop)
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat_simple_char(
        &mut self,
        want: u32,
        after: usize,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        if want <= 0x7F && !self.opt().contains(Options::IGNORECASE) && self.enc_min_len == 1 {
            return self.repeat_ascii_run(want as u8, after, min, max, greedy, pos, stop);
        }
        let cap = max.unwrap_or(u32::MAX);
        let mut ends = core::mem::take(&mut self.scratch);
        if ends.capacity() == 0 {
            super::count::tick_scratch_alloc();
        }
        ends.clear();
        ends.push(pos);
        let mut p = pos;
        let mut c = 0u32;
        while c < cap {
            match self.consume_char(p, want)? {
                Some(n) if n != p => {
                    p = n;
                    c += 1;
                    ends.push(p);
                    // `c` is the repetition count; `ends` also holds the
                    // zero-repetition entry, so counting it would fire the
                    // limit one repetition early.
                    self.check_repeat_depth(c as usize)?;
                }
                _ => break,
            }
        }
        if c < min {
            self.scratch = ends;
            return Ok(None);
        }
        self.bump_retry_n(u64::from(c))?;
        let r = self.try_lengths(&ends, after, min, c, greedy, stop);
        self.scratch = ends;
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat_ascii_run(
        &mut self,
        want: u8,
        after: usize,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        let cap = max.unwrap_or(u32::MAX);
        let mut end = pos;
        let mut c = 0u32;
        while end < self.hay_end && c < cap && self.hay[end] == want {
            end += 1;
            c += 1;
        }
        if c < min {
            return Ok(None);
        }
        self.bump_retry_n(u64::from(c))?;
        self.check_repeat_depth(c as usize)?;
        // Positions are pos + k: no trail needed.
        if greedy {
            let mut k = c;
            loop {
                if let Some(r) = self.run_stop(after, pos + k as usize, stop)? {
                    return Ok(Some(r));
                }
                if k == min {
                    return Ok(None);
                }
                k -= 1;
            }
        } else {
            for k in min..=c {
                if let Some(r) = self.run_stop(after, pos + k as usize, stop)? {
                    return Ok(Some(r));
                }
            }
            Ok(None)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat_simple_class(
        &mut self,
        class_pc: usize,
        after: usize,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        let prog: &'a Program = self.prog;
        let class = match prog.insts.get(class_pc) {
            Some(Inst::Class { class }) => class,
            _ => return Ok(None),
        };
        let plan = prog.class_plans.get(class_pc).copied().flatten();
        let cap = max.unwrap_or(u32::MAX);
        let plan_ok = self.plans_ok
            && self.ascii_fast
            && plan.map(|pl| pl.options == self.options).unwrap_or(false);
        let shape = prog.repeat_shapes.get(class_pc).copied().unwrap_or_default();
        let only_max = greedy && shape.follow_disjoint;
        let follow = if greedy && !shape.follow_disjoint {
            shape.follow_byte
        } else {
            None
        };

        // Fast path: while the run is ASCII, membership is one bitmap test and
        // the k-th end position is simply `pos + k`, so no position vector is
        // needed at all. This is the whole of `\w+` over ASCII text.
        if plan_ok {
            let pl = match plan {
                Some(pl) => pl,
                None => return Ok(None),
            };
            let hay = self.hay;
            let hay_end = self.hay_end;
            let mut p = pos;
            let mut c = 0u32;
            while c < cap && p < hay_end {
                let b = hay[p];
                if b >= 0x80 {
                    break;
                }
                super::count::tick_class_test();
                super::count::tick_plan_hit();
                if !pl.hit(u32::from(b)) {
                    break;
                }
                p += 1;
                c += 1;
            }
            // The run stayed ASCII to the end of what the class admits.
            if p >= hay_end || hay[p] < 0x80 {
                if c < min {
                    return Ok(None);
                }
                self.bump_retry_n(u64::from(c))?;
                self.check_repeat_depth(c as usize)?;
                if only_max {
                    // Every shorter length would put a class member where the
                    // following character must go.
                    super::count::tick_only_max();
                    return self.run_stop(after, pos + c as usize, stop);
                }
                if let Some(fb) = follow {
                    // Only lengths that leave the required byte in place can
                    // succeed; jump between its occurrences in the run.
                    super::count::tick_only_max();
                    if let Some(r) = self.run_stop(after, pos + c as usize, stop)? {
                        return Ok(Some(r));
                    }
                    let mut k = c;
                    while k > min {
                        k -= 1;
                        let b = hay[pos + k as usize];
                        let hit = if shape.follow_icase {
                            b.eq_ignore_ascii_case(&fb)
                        } else {
                            b == fb
                        };
                        if hit {
                            if let Some(r) = self.run_stop(after, pos + k as usize, stop)? {
                                return Ok(Some(r));
                            }
                        }
                    }
                    return Ok(None);
                }
                if greedy {
                    let mut k = c;
                    loop {
                        if let Some(r) = self.run_stop(after, pos + k as usize, stop)? {
                            return Ok(Some(r));
                        }
                        if k == min {
                            return Ok(None);
                        }
                        k -= 1;
                    }
                } else {
                    for k in min..=c {
                        if let Some(r) = self.run_stop(after, pos + k as usize, stop)? {
                            return Ok(Some(r));
                        }
                    }
                    return Ok(None);
                }
            }
            // A multi-byte character is in reach; fall through to the general
            // path, which re-walks from `pos` and handles variable widths.
        }

        let mut ends = core::mem::take(&mut self.scratch);
        if ends.capacity() == 0 {
            super::count::tick_scratch_alloc();
        }
        ends.clear();
        ends.push(pos);
        let mut p = pos;
        let mut c = 0u32;
        while c < cap {
            match self.consume_class(p, class, plan)? {
                Some(n) if n != p => {
                    p = n;
                    c += 1;
                    ends.push(p);
                    self.check_repeat_depth(c as usize)?;
                }
                _ => break,
            }
        }
        if c < min {
            self.scratch = ends;
            return Ok(None);
        }
        self.bump_retry_n(u64::from(c))?;
        let r = self.try_lengths(&ends, after, min, c, greedy, stop);
        self.scratch = ends;
        r
    }

    /// Try the continuation at each admissible repetition count, longest first
    /// when greedy and shortest first when lazy.
    fn try_lengths(
        &mut self,
        ends: &[usize],
        after: usize,
        min: u32,
        count: u32,
        greedy: bool,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        if greedy {
            let mut k = count;
            loop {
                if let Some(&q) = ends.get(k as usize) {
                    if let Some(r) = self.run_stop(after, q, stop)? {
                        return Ok(Some(r));
                    }
                }
                if k == min {
                    return Ok(None);
                }
                k -= 1;
            }
        } else {
            for k in min..=count {
                if let Some(&q) = ends.get(k as usize) {
                    if let Some(r) = self.run_stop(after, q, stop)? {
                        return Ok(Some(r));
                    }
                }
            }
            Ok(None)
        }
    }

    /// General repetition.
    ///
    /// Iterative on purpose. Recursing once per repetition makes backtrack
    /// depth equal native stack depth: `\w+` over ~2 KB of word characters
    /// aborted the process, and `stack_limit` could not catch it. The trail
    /// below is the same search in the same order, on the heap.
    #[allow(clippy::too_many_arguments)]
    fn repeat_rec(
        &mut self,
        body: usize,
        after: usize,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        possessive: bool,
        pos: usize,
        count: u32,
        writes_caps: bool,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        self.bump_retry()?;
        if let Some(m) = max {
            if count > m {
                return Ok(None);
            }
        }

        if possessive {
            let mut p = pos;
            let mut c = count;
            while c < min {
                match self.run_until(body, after, p)? {
                    Some(n) => {
                        p = n;
                        c += 1;
                    }
                    None => return Ok(None),
                }
            }
            let cap = max.unwrap_or(u32::MAX);
            while c < cap {
                match self.run_until(body, after, p)? {
                    Some(n) if n != p => {
                        p = n;
                        c += 1;
                        self.check_repeat_depth(c as usize)?;
                    }
                    _ => break,
                }
            }
            return self.run_stop(after, p, stop);
        }

        // One entry per successful body iteration: the position it started
        // from, plus (only when the body writes captures) the capture state to
        // restore if the whole repetition fails from here.
        let mut trail: Vec<usize> = Vec::new();
        let mut caps_trail: Vec<(Caps, usize)> = Vec::new();
        let mut p = pos;
        let mut c = count;
        // `count > max` at a level yields nothing at all, not even the
        // "stop repeating and run the rest" arm.
        let mut level_over_max = false;

        if greedy {
            loop {
                if let Some(m) = max {
                    if c > m {
                        level_over_max = true;
                        break;
                    }
                }
                if !max.map(|m| c < m).unwrap_or(true) {
                    break;
                }
                let cap = if writes_caps {
                    super::count::tick_cap_clone();
                    Some((self.captures.clone(), self.hist.len()))
                } else {
                    None
                };
                let inner = self.run_until(body, after, p)?;
                match inner {
                    // An empty iteration once `min` is satisfied stops the
                    // repetition, without restoring captures.
                    Some(n) if n == p && c >= min => break,
                    Some(n) => {
                        trail.push(p);
                        if let Some(cc) = cap {
                            caps_trail.push(cc);
                        }
                        p = n;
                        c += 1;
                        self.bump_retry()?;
                        self.check_repeat_depth(trail.len())?;
                    }
                    None => {
                        if let Some((cc, hlen)) = cap {
                            self.captures = cc;
                            self.hist.truncate(hlen);
                        }
                        break;
                    }
                }
            }
            // Unwind: longest first, trying the rest at each length down to min.
            loop {
                if !level_over_max && c >= min {
                    if let Some(r) = self.run_stop(after, p, stop)? {
                        return Ok(Some(r));
                    }
                }
                level_over_max = false;
                match trail.pop() {
                    None => return Ok(None),
                    Some(prev) => {
                        if writes_caps {
                            if let Some((cc, hlen)) = caps_trail.pop() {
                                self.captures = cc;
                                self.hist.truncate(hlen);
                            }
                        }
                        p = prev;
                        c -= 1;
                    }
                }
            }
        }

        // Lazy: try the rest first at each length, shortest first.
        loop {
            if let Some(m) = max {
                if c > m {
                    break;
                }
            }
            if c >= min {
                if let Some(r) = self.run_stop(after, p, stop)? {
                    return Ok(Some(r));
                }
            }
            if !max.map(|m| c < m).unwrap_or(true) {
                break;
            }
            let cap = if writes_caps {
                super::count::tick_cap_clone();
                Some((self.captures.clone(), self.hist.len()))
            } else {
                None
            };
            let inner = self.run_until(body, after, p)?;
            match inner {
                Some(n) if n == p && c >= min => break,
                Some(n) => {
                    if let Some(cc) = cap {
                        caps_trail.push(cc);
                    }
                    trail.push(p);
                    p = n;
                    c += 1;
                    self.bump_retry()?;
                    self.check_repeat_depth(trail.len())?;
                }
                None => {
                    if let Some((cc, hlen)) = cap {
                        self.captures = cc;
                        self.hist.truncate(hlen);
                    }
                    break;
                }
            }
        }
        // Every level that failed restores the captures it snapshotted, from
        // the deepest outwards, so the shallowest snapshot lands last.
        while let Some((cc, hlen)) = caps_trail.pop() {
            self.captures = cc;
            self.hist.truncate(hlen);
        }
        Ok(None)
    }

    // -----------------------------------------------------------------------
    // Look-around
    // -----------------------------------------------------------------------

    /// Character-width range of the sub-program in `[start, end)`.
    ///
    /// `None` means "cannot tell", and the caller must then scan. `Some((min,
    /// None))` means unbounded above.
    fn width_range(&self, start: usize, end: usize, fuel: &mut u32) -> Option<(u32, Option<u32>)> {
        let mut lo = 0u32;
        let mut hi = Some(0u32);
        let mut pc = start;
        while pc < end {
            if *fuel == 0 {
                return None;
            }
            *fuel -= 1;
            match self.prog.insts.get(pc)? {
                Inst::Nop
                | Inst::Save(_)
                | Inst::Keep
                | Inst::Assert(_)
                | Inst::PushOptions(_, _)
                | Inst::PopOptions
                | Inst::Callout { .. } => pc += 1,
                Inst::Char(_)
                | Inst::Class { .. }
                | Inst::Any { .. }
                | Inst::SuperAny
                | Inst::GeneralNewline
                | Inst::TextSegment => {
                    lo += 1;
                    hi = hi.map(|h| h + 1);
                    pc += 1;
                }
                Inst::Literal(v) => {
                    let n = v.len() as u32;
                    lo += n;
                    hi = hi.map(|h| h + n);
                    pc += 1;
                }
                Inst::Look { after, .. } => pc = *after as usize,
                Inst::Jump(j) => {
                    let j = *j as usize;
                    // Only follow a forward jump; a backward one is a loop we
                    // are not going to reason about.
                    if j <= pc {
                        return None;
                    }
                    pc = j;
                }
                Inst::Split(a, b) => {
                    let (a, b) = (*a as usize, *b as usize);
                    let wa = self.width_range(a, end, fuel)?;
                    let wb = self.width_range(b, end, fuel)?;
                    let alt_hi = match (wa.1, wb.1) {
                        (Some(x), Some(y)) => Some(x.max(y)),
                        _ => None,
                    };
                    lo += wa.0.min(wb.0);
                    hi = match (hi, alt_hi) {
                        (Some(h), Some(x)) => Some(h + x),
                        _ => None,
                    };
                    return Some((lo, hi));
                }
                Inst::Repeat {
                    body,
                    after,
                    min,
                    max,
                    ..
                } => {
                    let (body, after) = (*body as usize, *after as usize);
                    let (min, max) = (*min, *max);
                    let w = self.width_range(body, after, fuel)?;
                    lo += w.0.saturating_mul(min);
                    hi = match (hi, w.1, max) {
                        (Some(h), Some(wh), Some(m)) => Some(h + wh.saturating_mul(m)),
                        _ => None,
                    };
                    pc = after;
                }
                // Backrefs, calls, conditionals, absent and atomic spans are
                // not worth modelling here.
                _ => return None,
            }
        }
        Some((lo, hi))
    }

    fn look(
        &mut self,
        body: usize,
        after: usize,
        behind: bool,
        pos: usize,
    ) -> Result<bool, Error> {
        let cap = if self.writes_caps {
            super::count::tick_cap_clone();
            Some((self.captures.clone(), self.hist.len()))
        } else {
            None
        };
        let r = if behind {
            // Only start positions whose distance back from `pos` lies in the
            // body's width range can end at `pos`. Without this the scan walks
            // back to offset 0 at every position, which is O(n^2) across a
            // search: `(?<=status=)\d+` over 32 KB took ~11 s.
            let mut fuel = 512u32;
            let (min_w, max_w) = match self.width_range(body, after, &mut fuel) {
                Some((lo, hi)) => (lo, hi),
                None => (0, None),
            };
            let mut p = pos;
            let mut back = 0u32;
            let mut found = false;
            loop {
                if back >= min_w {
                    if let Some(n) = self.run_until(body, after, p)? {
                        if n == pos {
                            found = true;
                            break;
                        }
                    }
                }
                if p == self.hay_start || max_w.map(|m| back >= m).unwrap_or(false) {
                    break;
                }
                match self.enc.prev_char_start(self.hay, self.hay_start, p) {
                    Some(q) => {
                        p = q;
                        back += 1;
                    }
                    None => break,
                }
            }
            found
        } else {
            self.run_until(body, after, pos)?.is_some()
        };
        if let Some((cap, hlen)) = cap {
            self.captures = cap;
            self.hist.truncate(hlen);
        }
        Ok(r)
    }

    // -----------------------------------------------------------------------
    // Absent expressions
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn absent(
        &mut self,
        stopper: usize,
        expr: Option<usize>,
        after: usize,
        kind: AbsentKind,
        pos: usize,
        stop: Option<usize>,
    ) -> Result<Option<usize>, Error> {
        let limit = self.find_absent_limit(stopper, after, pos)?;
        match kind {
            AbsentKind::Clear => self.run_stop(after, pos, stop),
            AbsentKind::Stopper => {
                let saved = self.hay_end;
                self.hay_end = limit.unwrap_or(self.hay_end);
                let r = self.run_stop(after, pos, stop);
                self.hay_end = saved;
                r
            }
            // `(?~subexp)` is a repeater, not a fixed bite: it may match any
            // span up to the stopper, so it has to be tried longest-first with
            // the continuation at each length. Consuming maximally and handing
            // back one position made `a(?~b)c` never match.
            AbsentKind::Repeater => {
                let end = limit.unwrap_or(self.hay_end);
                let mut stops = alloc::vec![pos];
                let mut p = pos;
                while p < end {
                    match self.enc.mbc_len(&self.hay[p..end]) {
                        Ok(n) if n > 0 && p + n <= end => {
                            p += n;
                            stops.push(p);
                        }
                        _ => break,
                    }
                }
                while let Some(q) = stops.pop() {
                    self.bump_retry()?;
                    if let Some(r) = self.run_stop(after, q, stop)? {
                        return Ok(Some(r));
                    }
                }
                Ok(None)
            }
            AbsentKind::Expression => {
                let saved = self.hay_end;
                self.hay_end = limit.unwrap_or(self.hay_end);
                let start = expr.unwrap_or(after);
                let inner = self.run_until(start, after, pos);
                self.hay_end = saved;
                match inner? {
                    Some(n) => self.run_stop(after, n, stop),
                    None => Ok(None),
                }
            }
        }
    }

    fn find_absent_limit(
        &mut self,
        stopper: usize,
        after: usize,
        pos: usize,
    ) -> Result<Option<usize>, Error> {
        let mut p = pos;
        while p <= self.hay_end {
            let cap = if self.writes_caps {
                super::count::tick_cap_clone();
                Some((self.captures.clone(), self.hist.len()))
            } else {
                None
            };
            let hit = self.run_until(stopper, after, p)?.is_some();
            if let Some((cap, hlen)) = cap {
                self.captures = cap;
                self.hist.truncate(hlen);
            }
            if hit {
                return Ok(Some(p));
            }
            if p == self.hay_end {
                break;
            }
            let n = next_pos(self.enc, self.hay, p);
            if n <= p {
                break;
            }
            p = n;
        }
        Ok(None)
    }

    // -----------------------------------------------------------------------
    // Backrefs, subexp calls, conditionals
    // -----------------------------------------------------------------------

    /// Haystack span of a captured group.
    fn group_span(&self, n: usize) -> Option<(usize, usize)> {
        let i = n * 2;
        match (
            self.captures.get(i).copied().flatten(),
            self.captures.get(i + 1).copied().flatten(),
        ) {
            (Some(a), Some(b)) if a <= b && b <= self.hay.len() => Some((a, b)),
            _ => None,
        }
    }

    fn group_index(&self, b: &Backref) -> Option<usize> {
        match b {
            Backref::Number(n) => {
                if *n < 0 {
                    let i = self.prog.capture_count as i32 + n;
                    if i > 0 {
                        Some(i as usize)
                    } else {
                        None
                    }
                } else {
                    Some(*n as usize)
                }
            }
            Backref::Name(name) => self
                .prog
                .names
                .iter()
                .position(|n| n.as_deref() == Some(name.as_str())),
            Backref::Rel { back, n } => {
                let base = self.prog.capture_count as i32;
                let i = if *back { base - *n } else { *n };
                if i > 0 {
                    Some(i as usize)
                } else {
                    None
                }
            }
        }
    }

    fn backref(&mut self, b: &Backref, pos: usize) -> Result<Option<usize>, Error> {
        let idx = match self.group_index(b) {
            Some(i) => i,
            None => return Ok(None),
        };
        let (s, e) = match self.group_span(idx) {
            Some(v) => v,
            None => return Ok(None),
        };
        let len = e - s;
        if pos + len > self.hay_end {
            return Ok(None);
        }
        let opt = self.opt();
        if opt.contains(Options::IGNORECASE) {
            // Compare character by character under folding.
            let mut a = s;
            let mut b2 = pos;
            while a < e {
                let (ca, na) = match self.char_at_in(a, e)? {
                    Some(v) => v,
                    None => return Ok(None),
                };
                let (cb, nb) = match self.char_at(b2)? {
                    Some(v) => v,
                    None => return Ok(None),
                };
                if !fold_eq(ca, cb, opt.contains(Options::IGNORECASE_IS_ASCII)) {
                    return Ok(None);
                }
                a += na;
                b2 += nb;
            }
            Ok(Some(b2))
        } else if self.hay[pos..pos + len] == self.hay[s..e] {
            Ok(Some(pos + len))
        } else {
            Ok(None)
        }
    }

    fn subexp_call(
        &mut self,
        t: &CallTarget,
        pos: usize,
        call_pc: usize,
    ) -> Result<Option<usize>, Error> {
        let idx = match t {
            CallTarget::Whole => 0,
            CallTarget::Number(n) => {
                if *n < 0 {
                    let i = self.prog.capture_count as i32 + n;
                    if i < 0 {
                        return Ok(None);
                    }
                    i as usize
                } else {
                    *n as usize
                }
            }
            CallTarget::Name(n) => self
                .prog
                .names
                .iter()
                .position(|nm| nm.as_deref() == Some(n.as_str()))
                .unwrap_or(0),
        };
        if idx == 0 {
            return self.run(0, pos);
        }
        let (open, close) = match self.prog.group_spans.get(idx).copied().flatten() {
            Some((a, b)) => (a as usize, b as usize),
            None => return Ok(None),
        };
        // Run the group *including* its Save pair so the call writes the
        // capture: Oniguruma leaves group 1 at 1..2 for `(a)\g<1>` on "aa".
        //
        // Unless the call sits inside the very group it calls -- true
        // recursion. There the outermost invocation's span is the one
        // Oniguruma reports, so put this group's slots back on the way out
        // and let the enclosing Save pair have the last word.
        let recursive = call_pc > open && call_pc < close;
        let saved = if recursive {
            Some((
                self.captures.get(idx * 2).copied().flatten(),
                self.captures.get(idx * 2 + 1).copied().flatten(),
            ))
        } else {
            None
        };
        let r = self.run_until(open, close + 1, pos)?;
        if r.is_some() {
            if let Some((a, b)) = saved {
                if let Some(slot) = self.captures.get_mut(idx * 2) {
                    *slot = a;
                }
                if let Some(slot) = self.captures.get_mut(idx * 2 + 1) {
                    *slot = b;
                }
            }
        }
        Ok(r)
    }

    fn cond_true(&mut self, cond: &Cond, pos: usize) -> Result<bool, Error> {
        match cond {
            Cond::Group(n) => Ok(self.group_span(*n).is_some()),
            Cond::Name(name) => {
                let idx = self
                    .prog
                    .names
                    .iter()
                    .position(|nm| nm.as_deref() == Some(name.as_str()));
                Ok(idx.map(|i| self.group_span(i).is_some()).unwrap_or(false))
            }
            Cond::ValidRef(b) => {
                let idx = self.group_index(b);
                Ok(idx.map(|i| self.group_span(i).is_some()).unwrap_or(false))
            }
            Cond::Look { body, after } => {
                let (body, after) = (*body as usize, *after as usize);
                self.look(body, after, false, pos)
            }
            // Replaced by Cond::Look at compile time.
            Cond::Expr(_) => Ok(false),
        }
    }

    // -----------------------------------------------------------------------
    // Consumption
    // -----------------------------------------------------------------------

    /// Decode the character at `pos`, bounded by `end`.
    fn char_at_in(&self, pos: usize, end: usize) -> Result<Option<(u32, usize)>, Error> {
        if pos >= end {
            return Ok(None);
        }
        let rest = &self.hay[pos..end];
        if self.ascii_fast && rest[0] < 0x80 {
            return Ok(Some((u32::from(rest[0]), 1)));
        }
        let n = match self.enc.mbc_len(rest) {
            Ok(n) if n > 0 && n <= rest.len() => n,
            _ => return Ok(None),
        };
        match self.enc.decode_len(rest, n) {
            Ok(c) => Ok(Some((c, n))),
            Err(_) => Ok(None),
        }
    }

    fn char_at(&self, pos: usize) -> Result<Option<(u32, usize)>, Error> {
        self.char_at_in(pos, self.hay_end)
    }

    fn char_before(&self, pos: usize) -> Option<u32> {
        if pos <= self.hay_start {
            return None;
        }
        // One byte back is the whole character in an ASCII-transparent
        // encoding; no need to find the boundary and decode.
        if self.ascii_fast {
            let b = self.hay[pos - 1];
            if b < 0x80 {
                return Some(u32::from(b));
            }
        }
        let q = self.enc.prev_char_start(self.hay, self.hay_start, pos)?;
        let rest = &self.hay[q..pos];
        if self.enc.ascii_transparent() && !rest.is_empty() && rest[0] < 0x80 {
            return Some(u32::from(rest[0]));
        }
        self.enc.decode_len(rest, rest.len()).ok()
    }

    fn consume_char(&mut self, pos: usize, want: u32) -> Result<Option<usize>, Error> {
        super::count::tick_consume();
        let (cp, n) = match self.char_at(pos)? {
            Some(v) => v,
            None => return Ok(None),
        };
        if cp == want {
            return Ok(Some(pos + n));
        }
        let opt = self.opt();
        if opt.contains(Options::IGNORECASE)
            && fold_eq(want, cp, opt.contains(Options::IGNORECASE_IS_ASCII))
        {
            return Ok(Some(pos + n));
        }
        Ok(None)
    }

    fn consume_class(
        &mut self,
        pos: usize,
        cc: &CharClass,
        plan: Option<ClassPlan>,
    ) -> Result<Option<usize>, Error> {
        super::count::tick_class_test();
        if pos >= self.hay_end {
            return Ok(None);
        }
        let rest = &self.hay[pos..self.hay_end];
        // Single-byte ASCII needs no decode at all, and is the common case.
        let (n, cp) = if self.ascii_fast && rest[0] < 0x80 {
            (1usize, u32::from(rest[0]))
        } else {
            let n = match self.enc.mbc_len(rest) {
                Ok(n) if n > 0 && n <= rest.len() => n,
                _ => return Ok(None),
            };
            // decode_len reuses the length just computed; mbc_to_code would
            // redo mbc_len, and on an unbounded slice at that.
            match self.enc.decode_len(rest, n) {
                Ok(c) => (n, c),
                Err(_) => return Ok(None),
            }
        };
        // Below U+0080 the precompiled bitmap answers in two instructions.
        if cp < 0x80 && self.plans_ok {
            if let Some(plan) = plan {
                if plan.options == self.options {
                    super::count::tick_plan_hit();
                    return Ok(if plan.hit(cp) { Some(pos + n) } else { None });
                }
            }
        }
        if class_hit_in(cc, cp, self.enc, self.opt(), self.user_props) {
            Ok(Some(pos + n))
        } else {
            Ok(None)
        }
    }

    fn consume_any(
        &mut self,
        pos: usize,
        newline_ok: bool,
        super_any: bool,
    ) -> Result<Option<usize>, Error> {
        super::count::tick_consume();
        let (cp, n) = match self.char_at(pos)? {
            Some(v) => v,
            None => return Ok(None),
        };
        if !super_any && !newline_ok && is_newline_cp(cp) {
            return Ok(None);
        }
        Ok(Some(pos + n))
    }

    /// `\R`: CRLF as one unit, or any single general newline.
    fn consume_r(&mut self, pos: usize) -> Result<Option<usize>, Error> {
        let (cp, n) = match self.char_at(pos)? {
            Some(v) => v,
            None => return Ok(None),
        };
        if cp == 0x0d {
            if let Some((c2, n2)) = self.char_at(pos + n)? {
                if c2 == 0x0a {
                    return Ok(Some(pos + n + n2));
                }
            }
            return Ok(Some(pos + n));
        }
        if is_general_newline_cp(cp) {
            return Ok(Some(pos + n));
        }
        Ok(None)
    }

    /// `\X`: one extended grapheme cluster, or one word segment under
    /// `TEXT_SEGMENT_WORD`.
    fn consume_x(&mut self, pos: usize) -> Result<Option<usize>, Error> {
        let (first, n) = match self.char_at(pos)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let mut p = pos + n;
        let opt = self.opt();
        if opt.contains(Options::TEXT_SEGMENT_WORD) {
            if unicode::is_word(self.enc, opt, first) {
                while let Some((c2, n2)) = self.char_at(p)? {
                    if !unicode::is_word(self.enc, opt, c2) {
                        break;
                    }
                    p += n2;
                }
            }
            return Ok(Some(p));
        }
        let mut prev = first;
        while let Some((c2, n2)) = self.char_at(p)? {
            if grapheme_break(Some(prev), c2) {
                break;
            }
            prev = c2;
            p += n2;
        }
        Ok(Some(p))
    }

    // -----------------------------------------------------------------------
    // Anchors
    // -----------------------------------------------------------------------

    fn is_word_at(&self, pos: usize) -> Result<bool, Error> {
        Ok(match self.char_at(pos)? {
            Some((c, _)) => unicode::is_word(self.enc, self.opt(), c),
            None => false,
        })
    }

    fn is_word_before(&self, pos: usize) -> bool {
        match self.char_before(pos) {
            Some(c) => unicode::is_word(self.enc, self.opt(), c),
            None => false,
        }
    }

    fn anchor(&mut self, a: Anchor, pos: usize) -> Result<bool, Error> {
        let opt = self.opt();
        Ok(match a {
            Anchor::Bol => {
                // The search only offered line starts for this attempt --
                // except at end-of-string, where a trailing newline still
                // begins no line, so that rule below must be kept.
                if self.bol_guaranteed
                    && pos == self.attempt_start
                    && !(pos >= self.hay_end && pos > self.hay_start)
                {
                    return Ok(true);
                }
                if pos == self.hay_start {
                    !opt.contains(Options::NOTBOL)
                } else if pos >= self.hay_end {
                    // A trailing newline does not begin another line:
                    // libonig matches nothing for `(?m)^$` against a
                    // string ending in a newline.
                    false
                } else {
                    matches!(self.char_before(pos), Some(c) if is_newline_cp(c))
                }
            }
            Anchor::Eol => {
                if pos >= self.hay_end {
                    !opt.contains(Options::NOTEOL)
                } else {
                    matches!(self.char_at(pos)?, Some((c, _)) if is_newline_cp(c))
                }
            }
            Anchor::Bos => pos == self.hay_start && !opt.contains(Options::NOT_BEGIN_STRING),
            Anchor::Eos => pos >= self.hay_end && !opt.contains(Options::NOT_END_STRING),
            Anchor::EosNl => {
                if pos >= self.hay_end {
                    !opt.contains(Options::NOT_END_STRING)
                } else {
                    match self.char_at(pos)? {
                        Some((c, n)) if is_newline_cp(c) && pos + n >= self.hay_end => true,
                        _ => false,
                    }
                }
            }
            Anchor::WordBound => self.is_word_before(pos) != self.is_word_at(pos)?,
            Anchor::NotWordBound => self.is_word_before(pos) == self.is_word_at(pos)?,
            Anchor::WordBegin => !self.is_word_before(pos) && self.is_word_at(pos)?,
            Anchor::WordEnd => self.is_word_before(pos) && !self.is_word_at(pos)?,
            Anchor::G => pos == self.search_origin && !opt.contains(Options::NOT_BEGIN_POSITION),
            Anchor::TextSegBound => self.is_text_seg_bound(pos)?,
            Anchor::NotTextSegBound => !self.is_text_seg_bound(pos)?,
        })
    }

    fn is_text_seg_bound(&self, pos: usize) -> Result<bool, Error> {
        if pos == self.hay_start || pos >= self.hay_end {
            return Ok(true);
        }
        let prev = self.char_before(pos);
        match self.char_at(pos)? {
            Some((cur, _)) => Ok(grapheme_break(prev, cur)),
            None => Ok(true),
        }
    }

    // -----------------------------------------------------------------------
    // Result
    // -----------------------------------------------------------------------

    fn to_region(&self) -> Region {
        super::count::tick_lit_clone();
        let n = self.prog.capture_count;
        let mut r = Region::with_names(n, self.prog.has_named);
        if n == 1 {
            // Whole match only: no per-group loop, no name lookup.
            let a = self.captures.first().copied().flatten();
            let b = self.captures.get(1).copied().flatten();
            r.captures[0] = match (a, b) {
                (Some(a), Some(b)) if a <= b => Some(a..b),
                _ => None,
            };
            r.history = self.build_history();
            return r;
        }
        for i in 0..n {
            let a = self.captures.get(i * 2).copied().flatten();
            let b = self.captures.get(i * 2 + 1).copied().flatten();
            r.captures[i] = match (a, b) {
                (Some(a), Some(b)) if a <= b => Some(a..b),
                _ => None,
            };
            if self.prog.has_named {
                r.names[i] = self.prog.names.get(i).cloned().flatten();
            }
        }
        r.history = self.build_history();
        r
    }

    fn build_history(&self) -> Option<CaptureTree> {
        if self.hist.is_empty() {
            return None;
        }
        let a = self.captures.first().copied().flatten()?;
        let b = self.captures.get(1).copied().flatten()?;
        let mut root = CaptureTree {
            group: 0,
            range: a..b,
            children: Vec::new(),
        };
        for (g, s, e) in self.hist.iter().copied() {
            if s < a || e > b {
                continue;
            }
            insert_hist(&mut root, g, s..e);
        }
        Some(root)
    }
}

fn insert_hist(node: &mut CaptureTree, g: usize, range: Range<usize>) {
    for child in node.children.iter_mut() {
        if child.range.start <= range.start && range.end <= child.range.end {
            insert_hist(child, g, range);
            return;
        }
    }
    node.children.push(CaptureTree {
        group: g,
        range,
        children: Vec::new(),
    });
}

fn is_newline_cp(cp: u32) -> bool {
    cp == 0x0a
}

fn is_general_newline_cp(cp: u32) -> bool {
    matches!(cp, 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029)
}
