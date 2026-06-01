//! AC-5 + AC-N2 — `MockHost::install` routes the live SDK host wrappers
//! (`slicer_sdk::host::raycast_z_down`) through the installed adapter, and a
//! second `install` call replaces the first without an explicit uninstall.
//!
//! `slicer-test` does not depend on `slicer-macros`, so `#[module_test]` is
//! not in scope here. To preserve the same test-isolation contract the macro
//! provides, every test entry calls
//! `slicer_sdk::test_support::reset_global_state()` first — exactly as the
//! macro emits at the start of every wrapped test body.

use slicer_test::MockHost;

/// Reset per-thread SDK state (mesh source + log capture) at test entry.
/// Mirrors the prelude `#[module_test]` emits.
fn reset() {
    slicer_sdk::test_support::reset_global_state();
}

#[test]
fn mock_host_install_routes_raycast_through_host_wrapper() {
    reset();

    MockHost::new().with_raycast_hit(Some(4.8)).install();

    let got = slicer_sdk::host::raycast_z_down("obj-x", 1.0, 2.0, 5.0);

    // Exact f32 equality — the configured constant must round-trip through
    // the SDK host wrapper unchanged.
    assert_eq!(got, Some(4.8_f32));

    // Clean up so this thread does not leak state into a subsequent test.
    MockHost::uninstall();
}

#[test]
fn mock_host_second_install_replaces_first() {
    reset();

    // First install: raycast returns Some(1.0).
    MockHost::new().with_raycast_hit(Some(1.0)).install();

    // Second install without explicit uninstall: must replace, not stack.
    MockHost::new().with_raycast_hit(Some(2.0)).install();

    let got = slicer_sdk::host::raycast_z_down("obj-x", 0.0, 0.0, 0.0);
    assert_eq!(got, Some(2.0_f32));

    MockHost::uninstall();
}
