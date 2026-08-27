//! Deterministic work counters for the expressions VM.
//!
//! Enabled with feature `expr-count`. Ticks compile out otherwise. Weights are
//! fixed for the stress-harness campaign so a drop in `work` is a drop in
//! modeled x86-64 instructions (call/alloc/decode), not wall time.

use core::sync::atomic::{AtomicU64, Ordering::Relaxed};

static ENGINE_NEW: AtomicU64 = AtomicU64::new(0);
static VM_STEPS: AtomicU64 = AtomicU64::new(0);
static CONSUME_CHAR: AtomicU64 = AtomicU64::new(0);
static MBC_LEN: AtomicU64 = AtomicU64::new(0);
static MBC_TO_CODE: AtomicU64 = AtomicU64::new(0);
static BUMP_RETRY: AtomicU64 = AtomicU64::new(0);
static NEXT_POS: AtomicU64 = AtomicU64::new(0);
static CAP_CLONE: AtomicU64 = AtomicU64::new(0);
static SEARCH_POS: AtomicU64 = AtomicU64::new(0);
static LIT_CLONE: AtomicU64 = AtomicU64::new(0);
static UTF8_STR: AtomicU64 = AtomicU64::new(0);
static BYTE_SCAN: AtomicU64 = AtomicU64::new(0);
// Diagnostic only: weight 0, so `work()` stays comparable across the campaign.
static CLASS_TEST: AtomicU64 = AtomicU64::new(0);
static SPLIT_STEP: AtomicU64 = AtomicU64::new(0);
static SCRATCH_ALLOC: AtomicU64 = AtomicU64::new(0);
static PLAN_HIT: AtomicU64 = AtomicU64::new(0);
static REQ_SCAN: AtomicU64 = AtomicU64::new(0);
static REQ_SKIP: AtomicU64 = AtomicU64::new(0);
static ONLY_MAX: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub engine_new: u64,
    pub vm_steps: u64,
    pub consume_char: u64,
    pub mbc_len: u64,
    pub mbc_to_code: u64,
    pub bump_retry: u64,
    pub next_pos: u64,
    pub cap_clone: u64,
    pub search_pos: u64,
    pub lit_clone: u64,
    pub utf8_str: u64,
    pub byte_scan: u64,
    pub class_test: u64,
    pub split_step: u64,
    pub scratch_alloc: u64,
    pub plan_hit: u64,
    pub req_scan: u64,
    pub req_skip: u64,
    pub only_max: u64,
}

impl Stats {
    /// Fixed instruction model. Do not retune mid-campaign.
    pub fn work(&self) -> u64 {
        self.engine_new.saturating_mul(250)
            + self.vm_steps.saturating_mul(18)
            + self.consume_char.saturating_mul(12)
            + self.mbc_len.saturating_mul(20)
            + self.mbc_to_code.saturating_mul(55)
            + self.bump_retry.saturating_mul(14)
            + self.next_pos.saturating_mul(10)
            + self.cap_clone.saturating_mul(45)
            + self.search_pos.saturating_mul(15)
            + self.lit_clone.saturating_mul(35)
            + self.utf8_str.saturating_mul(40)
            + self.byte_scan.saturating_mul(1)
            // class_test and split_step are diagnostic: weight 0.
    }
}

#[inline(always)]
pub fn tick_engine_new() {
    #[cfg(feature = "expr-count")]
    ENGINE_NEW.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_vm() {
    #[cfg(feature = "expr-count")]
    VM_STEPS.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_consume() {
    #[cfg(feature = "expr-count")]
    CONSUME_CHAR.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_mbc_len() {
    #[cfg(feature = "expr-count")]
    MBC_LEN.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_mbc_to_code() {
    #[cfg(feature = "expr-count")]
    MBC_TO_CODE.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_bump() {
    #[cfg(feature = "expr-count")]
    BUMP_RETRY.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_next_pos() {
    #[cfg(feature = "expr-count")]
    NEXT_POS.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_cap_clone() {
    #[cfg(feature = "expr-count")]
    CAP_CLONE.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_search_pos() {
    #[cfg(feature = "expr-count")]
    SEARCH_POS.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_lit_clone() {
    #[cfg(feature = "expr-count")]
    LIT_CLONE.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_utf8_str() {
    #[cfg(feature = "expr-count")]
    UTF8_STR.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_class_test() {
    #[cfg(feature = "expr-count")]
    CLASS_TEST.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_only_max() {
    #[cfg(feature = "expr-count")]
    ONLY_MAX.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_req_scan() {
    #[cfg(feature = "expr-count")]
    REQ_SCAN.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_req_skip(n: u64) {
    #[cfg(feature = "expr-count")]
    REQ_SKIP.fetch_add(n, Relaxed);
    let _ = n;
}

#[inline(always)]
pub fn tick_plan_hit() {
    #[cfg(feature = "expr-count")]
    PLAN_HIT.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_scratch_alloc() {
    #[cfg(feature = "expr-count")]
    SCRATCH_ALLOC.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_split_step() {
    #[cfg(feature = "expr-count")]
    SPLIT_STEP.fetch_add(1, Relaxed);
}

#[inline(always)]
pub fn tick_byte_scan(n: u64) {
    #[cfg(feature = "expr-count")]
    BYTE_SCAN.fetch_add(n, Relaxed);
    let _ = n;
}

pub fn reset() {
    ENGINE_NEW.store(0, Relaxed);
    VM_STEPS.store(0, Relaxed);
    CONSUME_CHAR.store(0, Relaxed);
    MBC_LEN.store(0, Relaxed);
    MBC_TO_CODE.store(0, Relaxed);
    BUMP_RETRY.store(0, Relaxed);
    NEXT_POS.store(0, Relaxed);
    CAP_CLONE.store(0, Relaxed);
    SEARCH_POS.store(0, Relaxed);
    LIT_CLONE.store(0, Relaxed);
    UTF8_STR.store(0, Relaxed);
    BYTE_SCAN.store(0, Relaxed);
    CLASS_TEST.store(0, Relaxed);
    SPLIT_STEP.store(0, Relaxed);
    SCRATCH_ALLOC.store(0, Relaxed);
    PLAN_HIT.store(0, Relaxed);
    REQ_SCAN.store(0, Relaxed);
    REQ_SKIP.store(0, Relaxed);
    ONLY_MAX.store(0, Relaxed);
}

pub fn snapshot() -> Stats {
    Stats {
        engine_new: ENGINE_NEW.load(Relaxed),
        vm_steps: VM_STEPS.load(Relaxed),
        consume_char: CONSUME_CHAR.load(Relaxed),
        mbc_len: MBC_LEN.load(Relaxed),
        mbc_to_code: MBC_TO_CODE.load(Relaxed),
        bump_retry: BUMP_RETRY.load(Relaxed),
        next_pos: NEXT_POS.load(Relaxed),
        cap_clone: CAP_CLONE.load(Relaxed),
        search_pos: SEARCH_POS.load(Relaxed),
        lit_clone: LIT_CLONE.load(Relaxed),
        utf8_str: UTF8_STR.load(Relaxed),
        byte_scan: BYTE_SCAN.load(Relaxed),
        class_test: CLASS_TEST.load(Relaxed),
        split_step: SPLIT_STEP.load(Relaxed),
        scratch_alloc: SCRATCH_ALLOC.load(Relaxed),
        plan_hit: PLAN_HIT.load(Relaxed),
        req_scan: REQ_SCAN.load(Relaxed),
        req_skip: REQ_SKIP.load(Relaxed),
        only_max: ONLY_MAX.load(Relaxed),
    }
}
