//! Accessible HTML helpers for Dioxus / WebView UIs.
//!
//! Enabled with the `html` feature (which enables `a11y`). Prefer
//! [`crate::a11y`] in new code. This module remains for v0.1 compatibility.

pub use crate::a11y::label::img as labelled;
