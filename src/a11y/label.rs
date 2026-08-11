//! Labelled graphics and control naming helpers.

extern crate alloc;

use alloc::string::String;

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

/// Wrap a glyph in a labelled image role for screen readers.
///
/// Preferred path for new code. Also re-exported as
/// `thoth::symbols::html::labelled` when feature `html` is enabled.
///
/// ```
/// # #[cfg(feature = "a11y")]
/// # {
/// use thoth::a11y::label;
/// use thoth::symbols::status;
/// let s = label::img(status::OK, "verified");
/// assert!(s.contains("aria-label=\"verified\""));
/// assert!(s.contains("role=\"img\""));
/// # }
/// ```
pub fn img(glyph: &str, aria_label: &str) -> String {
    let safe = escape_attr(aria_label);
    alloc::format!(r#"<span role="img" aria-label="{safe}">{glyph}</span>"#)
}

/// Associate visible text with an explicit accessible name via `aria-label`.
///
/// Use when the visible string is insufficient (icon-only buttons, abbreviated
/// labels). Escapes quotes in the label.
pub fn named(visible: &str, aria_label: &str) -> String {
    let safe = escape_attr(aria_label);
    let vis = escape_attr(visible);
    alloc::format!(r#"<span aria-label="{safe}">{vis}</span>"#)
}
