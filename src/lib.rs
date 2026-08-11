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
//! # Memory allocation (`rusty_alloc`) -- on by default
//!
//! With the default `rusty-alloc` feature, thoth installs
//! [`rusty_alloc`](https://github.com/Remade-With-Rust/rusty_alloc) as the
//! process-wide allocator: pure Rust, mimalloc-class layout, double-free
//! aborts instead of heap corruption, and `wasm32-unknown-unknown` without a
//! C toolchain. Opt out when the app already owns the allocator:
//!
//! ```toml
//! thoth = { version = "0.3", default-features = false }
//! thoth = { version = "0.3", features = ["secure"] }  # + guard pages
//! ```
//!
//! ## If you are writing a LIBRARY that depends on thoth
//!
//! Set `default-features = false`. A program may contain exactly **one**
//! `#[global_allocator]`, and Cargo features are additive across the whole
//! graph -- a library that pulled thoth with defaults on would impose this
//! allocator on every downstream application.
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

/// The pure-Rust global allocator, installed process-wide.
///
/// Present with the **default** `rusty-alloc` feature. See the crate-level
/// docs for opt-out and library-consumer guidance.
#[cfg(feature = "rusty-alloc")]
#[global_allocator]
static GLOBAL: rusty_alloc_api::RustyAlloc = rusty_alloc_api::RustyAlloc;

/// Whether this build installed `rusty_alloc` as the global allocator.
pub const fn rusty_alloc_enabled() -> bool {
    cfg!(feature = "rusty-alloc")
}

/// Whether the hardened `secure` profile is compiled in.
pub const fn secure_allocator_enabled() -> bool {
    cfg!(feature = "secure")
}

pub mod symbols;
pub mod tokens;

#[cfg(feature = "a11y")]
#[cfg_attr(docsrs, doc(cfg(feature = "a11y")))]
pub mod a11y;

pub use symbols::{list, math, nav, status, structure};

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub use symbols::html;
