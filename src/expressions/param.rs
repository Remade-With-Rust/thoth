//! Match-time limits and callout hooks (Oniguruma MatchParam).

use core::cell::Cell;

use super::callout::CalloutFn;

/// Oniguruma default: 10_000_000 retries in match. 0 = unlimited.
pub const DEFAULT_RETRY_LIMIT_IN_MATCH: u64 = 10_000_000;
/// Finite search-retry default (Oniguruma ships 0/unlimited; we bound wasm/mesh).
pub const DEFAULT_RETRY_LIMIT_IN_SEARCH: u64 = 10_000_000;
/// Finite match-stack default (Oniguruma ships 0/unlimited).
pub const DEFAULT_MATCH_STACK_LIMIT: u32 = 10_000_000;
/// Oniguruma default subexp-call cap in search.
pub const DEFAULT_SUBEXP_CALL_LIMIT: u32 = 10_000;

/// Limits applied during match/search. 0 on a counter means unlimited.
#[derive(Clone, Debug)]
pub struct MatchParam {
    /// Maximum backtrack-stack depth. 0 = unlimited.
    pub stack_limit: u32,
    /// Retry count inside one match attempt. 0 = unlimited.
    pub retry_limit_in_match: u64,
    /// Retry count across a search (all start positions). 0 = unlimited.
    pub retry_limit_in_search: u64,
    /// Maximum `\g` subexp-calls during search. 0 = unlimited.
    pub subexp_call_limit: u32,
    /// Progress (forward) contents-callout.
    pub progress_callout: Option<CalloutFn>,
    /// Retraction (backtrack) contents-callout.
    pub retraction_callout: Option<CalloutFn>,
    /// Named callout (`(*name)`).
    pub named_callout: Option<CalloutFn>,
    /// Persistent `(*COUNT)` slot (`onig_get_callout_data_by_callout_args`).
    pub count: Cell<u32>,
}

impl Default for MatchParam {
    fn default() -> Self {
        Self {
            stack_limit: DEFAULT_MATCH_STACK_LIMIT,
            retry_limit_in_match: DEFAULT_RETRY_LIMIT_IN_MATCH,
            retry_limit_in_search: DEFAULT_RETRY_LIMIT_IN_SEARCH,
            subexp_call_limit: DEFAULT_SUBEXP_CALL_LIMIT,
            progress_callout: None,
            retraction_callout: None,
            named_callout: None,
            count: Cell::new(0),
        }
    }
}

impl MatchParam {
    /// Unlimited stack and retries (Oniguruma's 0-means-unlimited profile).
    pub fn unlimited() -> Self {
        Self {
            stack_limit: 0,
            retry_limit_in_match: 0,
            retry_limit_in_search: 0,
            subexp_call_limit: 0,
            progress_callout: None,
            retraction_callout: None,
            named_callout: None,
            count: Cell::new(0),
        }
    }
}
