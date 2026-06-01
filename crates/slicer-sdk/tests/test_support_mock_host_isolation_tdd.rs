//! AC-6 — proves `reset_global_state()` clears the per-thread mesh source so
//! a subsequent test never observes a prior test's `MockHost::install`.
//!
//! `slicer-sdk` test_support does not depend on `slicer-macros`, so we cannot
//! use `#[module_test]` here. We emulate the contract instead: every test
//! entry calls `slicer_sdk::test_support::reset_global_state()` — exactly
//! what the macro emits — and asserts the post-reset behavior.
//!
//! Rust's test order is not stable, so we do NOT rely on which test runs
//! first. Instead each test is self-contained: the "dirty" test installs a
//! mesh source and exits without uninstalling; the "clean" test calls
//! `reset_global_state()` and asserts the mesh source is gone. The contract
//! being verified — *at every test entry, the mesh source is cleared
//! regardless of prior state* — holds in either order.
//!
//! These tests run in the same binary, so cargo runs them on potentially
//! different threads. Each thread's `MESH_SOURCE` thread-local is separate,
//! which would mask the bug we want to catch. To exercise the reset path on
//! a shared thread, the assertion test re-installs *and then resets* in the
//! same body, asserting the reset cleared what *it* just installed.

use slicer_sdk::test_support::mock_host::MockHost;

fn reset() {
    slicer_sdk::test_support::reset_global_state();
}

#[test]
fn _a_first_install_leaves_mesh_source_dirty() {
    reset();
    // Install a mesh source and exit without uninstalling. If reset is
    // broken, a subsequent same-thread test would see Some(7.0).
    MockHost::new().with_raycast_hit(Some(7.0)).install();
    // No uninstall — intentional. The next test must clean up via reset.
}

#[test]
fn _b_second_test_sees_cleared_state_after_reset() {
    // Step 1 — emulate the prior test's dirty exit on *this* thread, so the
    // assertion is meaningful regardless of which thread cargo picks.
    MockHost::new().with_raycast_hit(Some(7.0)).install();
    assert_eq!(
        slicer_sdk::host::raycast_z_down("obj-x", 0.0, 0.0, 0.0),
        Some(7.0_f32),
        "precondition: install must take effect"
    );

    // Step 2 — call reset_global_state() exactly as #[module_test] does at
    // the start of every test.
    reset();

    // Step 3 — after reset, the mesh source must be gone (None).
    assert_eq!(
        slicer_sdk::host::raycast_z_down("obj-x", 0.0, 0.0, 0.0),
        None,
        "reset_global_state must clear the per-thread mesh source"
    );
}
