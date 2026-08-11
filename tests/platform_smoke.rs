//! Build-time checks that tokens + a11y APIs are available on this target.
//! Used by CI / local validation across host + wasm32.

#![cfg(all(feature = "a11y", feature = "css"))]

use thoth::a11y::{label, live, status};
use thoth::symbols::status as glyph;
use thoth::tokens::{color, css};

#[test]
fn tokens_and_a11y_smoke() {
    let sheet = css::root_sheet();
    assert!(sheet.contains(":root"));
    assert!(sheet.contains(color::ACCENT_VALUE));

    let labelled = label::img(glyph::OK, "verified");
    assert!(labelled.contains("aria-label=\"verified\""));

    assert!(live::polite("Syncing").contains("aria-live=\"polite\""));
    assert_eq!(status::Kind::Ready.as_str(), "Ready");
    assert!(status::announce(status::Kind::Offline).contains("Offline"));
}
