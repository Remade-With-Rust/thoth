//! Design tokens -- semantic names + neutral default values (rusty_tokens).
//!
//! Token *names* are the CSS custom-property contract. Default *values* are a
//! small ASCII starter theme; apps override via CSS. Optional feature `css`
//! emits a `:root` stylesheet for WebView / Dioxus injection.

pub mod color;
pub mod radius;
pub mod space;
pub mod type_scale;

#[cfg(feature = "css")]
#[cfg_attr(docsrs, doc(cfg(feature = "css")))]
pub mod css;
