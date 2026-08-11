//! ARIA live-region HTML snippets for status updates.

extern crate alloc;

use alloc::string::String;

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Polite live region -- announces when the user is idle.
///
/// ```
/// # #[cfg(feature = "a11y")]
/// # {
/// use thoth::a11y::live;
/// let s = live::polite("Syncing");
/// assert!(s.contains("aria-live=\"polite\""));
/// # }
/// ```
pub fn polite(message: &str) -> String {
    region("polite", message)
}

/// Assertive live region -- interrupts for urgent errors.
pub fn assertive(message: &str) -> String {
    region("assertive", message)
}

fn region(politely: &str, message: &str) -> String {
    let msg = escape_text(message);
    alloc::format!(r#"<div role="status" aria-live="{politely}">{msg}</div>"#)
}
