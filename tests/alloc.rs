//! Allocator feature smoke tests.

#[test]
fn secure_off_by_default() {
    assert!(!thoth::secure_allocator_enabled());
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
