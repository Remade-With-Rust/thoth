#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! Shared Unicode glyph constants for House Rust apps.
//!
//! All glyph bytes are expressed as `\u{...}` escapes so this crate's source stays
//! pure ASCII and cannot mojibake under Windows-1252 round-trips. Prefer these
//! named constants over literal glyphs in application code.
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
//! | [`symbols::html`] | labelled HTML spans (`html` feature) |
//!
//! Plan: [`docs/plans/symbols-crate.md`](https://github.com/Remade-With-Rust/thoth/blob/main/docs/plans/symbols-crate.md).

pub mod symbols;

pub use symbols::{list, math, nav, status, structure};

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub use symbols::html;
