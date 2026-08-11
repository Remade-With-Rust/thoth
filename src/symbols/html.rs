//! Accessible HTML helpers for Dioxus / WebView UIs.
//!
//! Enabled with the `html` feature. Requires the `alloc` crate (provided by
//! `std` in normal application builds).

extern crate alloc;

use alloc::string::String;

/// Wrap a glyph in a labelled image role for screen readers.
///
/// ```
/// # #[cfg(feature = "html")]
/// # {
/// use thoth::symbols::{html, status};
/// let s = html::labelled(status::OK, "verified");
/// assert!(s.contains("aria-label=\"verified\""));
/// assert!(s.contains("role=\"img\""));
/// # }
/// ```
pub fn labelled(glyph: &str, aria_label: &str) -> String {
    let safe = aria_label.replace('"', "&quot;");
    alloc::format!(r#"<span role="img" aria-label="{safe}">{glyph}</span>"#)
}
