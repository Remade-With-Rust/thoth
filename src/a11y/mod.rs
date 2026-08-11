//! Accessibility helpers for UI chrome (rusty_a11y).
//!
//! Enabled with the `a11y` feature. Requires `alloc`. Emits small HTML
//! snippets with `role` / `aria-*` only -- no DOM crate, no JS.
//!
//! Prefer this module over `symbols::html` in new code. The `html` feature
//! re-exports [`label::img`] as `symbols::html::labelled` for v0.1 compat.

pub mod label;
pub mod live;
pub mod status;
