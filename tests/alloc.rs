//! Allocator feature smoke tests.

/// `secure` is opt-in, so with it off the flag must stay off.
///
/// Gated: without the `cfg`, this assertion also ran under
/// `cargo test --features secure`, where it asserts the opposite of what the
/// build asked for and always failed.
#[cfg(not(feature = "secure"))]
#[test]
fn secure_off_by_default() {
    assert!(!thoth::secure_allocator_enabled());
}

/// The mirror of the above: when `secure` IS opted into, the flag must report
/// it. The feature previously had no positive test at all.
#[cfg(feature = "secure")]
#[test]
fn secure_on_when_feature_on() {
    assert!(thoth::secure_allocator_enabled());
}

#[cfg(feature = "rusty-alloc")]
#[test]
fn rusty_alloc_enabled_when_feature_on() {
    assert!(thoth::rusty_alloc_enabled());
}

#[cfg(not(feature = "rusty-alloc"))]
#[test]
fn rusty_alloc_disabled_when_opted_out() {
    assert!(!thoth::rusty_alloc_enabled());
}
