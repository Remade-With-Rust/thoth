//! Semantically grouped glyph constants.
//!
//! Grouping is by *role*, not appearance: `status::OK` and `list::BULLET` may
//! share a visual form today without coupling their futures.

pub mod list;
pub mod math;
pub mod nav;
pub mod status;
pub mod structure;

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
pub mod html;

/// Variation selector-15: force text presentation (not emoji colour).
pub const VS15: &str = "\u{FE0E}";

/// Variation selector-16: force emoji presentation.
pub const VS16: &str = "\u{FE0F}";
