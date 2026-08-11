//! Common chrome status announcements (English, v0.2 -- i18n is a non-goal).

extern crate alloc;

use alloc::string::String;

use super::live;

/// Well-known chrome status kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Document or form saved.
    Saved,
    /// Background sync in progress.
    Syncing,
    /// No network / offline mode.
    Offline,
    /// Generic error surface.
    Error,
    /// Ready / idle.
    Ready,
}

impl Kind {
    /// Short English phrase for this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Saved => "Saved",
            Kind::Syncing => "Syncing",
            Kind::Offline => "Offline",
            Kind::Error => "Error",
            Kind::Ready => "Ready",
        }
    }
}

/// Announce a status kind in a polite live region.
///
/// ```
/// # #[cfg(feature = "a11y")]
/// # {
/// use thoth::a11y::status::{announce, Kind};
/// let s = announce(Kind::Saved);
/// assert!(s.contains("Saved"));
/// assert!(s.contains("aria-live=\"polite\""));
/// # }
/// ```
pub fn announce(kind: Kind) -> String {
    live::polite(kind.as_str())
}

/// Announce an error assertively.
pub fn announce_error(detail: &str) -> String {
    live::assertive(detail)
}
