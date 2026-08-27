//! Callout seam: function pointers so compiled [`Regex`](super::Regex) stays `Send + Sync`.

extern crate alloc;

use alloc::string::String;

/// Result of a callout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalloutResult {
    /// Continue matching (Oniguruma `ONIG_CALLOUT_SUCCESS` / 0).
    Success,
    /// Fail this alternative.
    Fail,
    /// `(*SKIP)`: continue matching, but if this whole attempt fails, resume
    /// the search at this position rather than at the next one.
    Skip,
}

/// Direction a contents-callout fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalloutDir {
    Progress,
    Retraction,
    Both,
}

/// Context passed to a callout.
#[derive(Clone, Debug)]
pub struct CalloutCtx<'a> {
    pub name: &'a str,
    pub args: &'a str,
    pub tag: Option<&'a str>,
    pub body: &'a str,
    pub haystack: &'a [u8],
    pub current: usize,
    pub dir: CalloutDir,
}

/// `fn` pointer: `Send + Sync`, no capture. Closures belong in a match-time wrapper.
pub type CalloutFn = fn(&CalloutCtx<'_>) -> CalloutResult;

/// Built-in `(*COUNT)` state: increment a counter the caller owns via args parse.
/// The engine treats `(*COUNT)` as Success and records nothing unless a named
/// callout hook is installed. `(*SKIP)` is implemented in exec.
pub fn builtin_skip(_ctx: &CalloutCtx<'_>) -> CalloutResult {
    CalloutResult::Skip
}

/// Format a contents-callout body for debugging (no C callback).
pub fn describe(ctx: &CalloutCtx<'_>) -> String {
    alloc::format!(
        "callout name={} args={} pos={}",
        ctx.name,
        ctx.args,
        ctx.current
    )
}
