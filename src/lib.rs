#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! Shared Unicode glyphs, design tokens, and chrome a11y helpers.
//!
//! - [`symbols`] -- semantic glyph constants as ASCII `\u{...}` escapes
//! - [`tokens`] -- design-token names + neutral defaults (rusty_tokens)
//! - [`a11y`] -- ARIA HTML helpers (`a11y` feature; rusty_a11y)
//!
//! All crate source stays pure ASCII so Windows-1252 round-trips cannot
//! mojibake constants.
//!
//! # Modules
//!
//! | Module | Role |
//! |---|---|
//! | [`symbols::status`] | ok / fail / warn / timer / play |
//! | [`symbols::nav`] | arrows, hooks, collapse |
//! | [`symbols::structure`] | rules, tree lines |
//! | [`symbols::math`] | inequalities, times |
//! | [`symbols::list`] | bullets / separators |
//! | [`symbols::html`] | labelled spans (`html` feature; re-exports a11y) |
//! | [`tokens`] | color / space / type / radius tokens |
//! | [`tokens::css`] | `:root` sheet (`css` feature) |
//! | [`a11y`] | label / live / status (`a11y` feature) |
//!
//! Plans: symbols / [tokens](https://github.com/Remade-With-Rust/thoth/blob/main/docs/plans/tokens-crate.md) /
//! [a11y](https://github.com/Remade-With-Rust/thoth/blob/main/docs/plans/a11y-crate.md).

pub mod symbols;
pub mod tokens;

#[cfg(feature = "a11y")]
#[cfg_attr(docsrs, doc(cfg(feature = "a11y")))]
pub mod a11y;

pub use symbols::{list, math, nav, status, structure};

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub use symbols::html;
