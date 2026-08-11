//! Token + a11y feature tests (values, CSS sheet, ARIA helpers).

use thoth::tokens::{color, radius, space, type_scale};

#[test]
fn color_names_are_css_vars() {
    assert!(color::FG.starts_with("--thoth-"));
    assert!(color::FG_VALUE.starts_with('#'));
    assert_eq!(color::FG_VALUE.len(), 7);
}

#[test]
fn space_and_radius_defaults() {
    assert_eq!(space::MD_VALUE, "1rem");
    assert_eq!(radius::MD_VALUE, "0.5rem");
    assert_eq!(type_scale::SIZE_BODY_VALUE, "1rem");
}

#[cfg(feature = "css")]
#[test]
fn root_sheet_contains_core_tokens() {
    let sheet = thoth::tokens::css::root_sheet();
    assert!(sheet.starts_with(":root {"));
    assert!(sheet.contains(color::FG));
    assert!(sheet.contains(color::FG_VALUE));
    assert!(sheet.contains(space::MD));
    assert!(sheet.contains(radius::LG_VALUE));
    assert!(sheet.contains(type_scale::SIZE_DISPLAY_VALUE));
}

#[cfg(feature = "a11y")]
mod a11y_tests {
    use thoth::a11y::{label, live, status};
    use thoth::symbols::status as glyph;

    #[test]
    fn label_img_sets_role_and_aria() {
        let s = label::img(glyph::OK, "verified");
        assert!(s.contains("role=\"img\""));
        assert!(s.contains("aria-label=\"verified\""));
        assert!(s.contains(glyph::OK));
    }

    #[test]
    fn label_escapes_quotes() {
        let s = label::img(glyph::OK, "say \"hi\"");
        assert!(s.contains("aria-label=\"say &quot;hi&quot;\""));
    }

    #[test]
    fn live_polite_and_assertive() {
        let p = live::polite("Syncing");
        assert!(p.contains("aria-live=\"polite\""));
        assert!(p.contains("Syncing"));
        let a = live::assertive("Error");
        assert!(a.contains("aria-live=\"assertive\""));
    }

    #[test]
    fn status_announce() {
        let s = status::announce(status::Kind::Saved);
        assert!(s.contains("Saved"));
        assert!(s.contains("aria-live=\"polite\""));
        assert_eq!(status::Kind::Offline.as_str(), "Offline");
    }

    #[cfg(feature = "html")]
    #[test]
    fn html_labelled_reexports_img() {
        let a = label::img(glyph::OK, "verified");
        let b = thoth::symbols::html::labelled(glyph::OK, "verified");
        assert_eq!(a, b);
    }
}
