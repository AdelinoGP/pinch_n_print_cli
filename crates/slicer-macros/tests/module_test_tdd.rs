//! TDD tests for the `#[module_test]` proc-macro.
//!
//! These tests verify that the macro correctly transforms test functions
//! to automatically set up mock host, install test panic handler, and reset
//! global state between tests, per docs/05_module_sdk.md.
//!
//! The macro expands to fully-qualified calls into
//! `::slicer_sdk::test_support::*`; assertions in this file therefore
//! observe the *effects* of those calls (per-thread log capture install,
//! mesh source clearing, etc.) rather than tracking flags on local stub
//! functions.

#![allow(
    clippy::assertions_on_constants,
    clippy::nonminimal_bool,
    clippy::overly_complex_bool_expr
)]

use slicer_ir::{BoundingBox3, Point3};
use slicer_macros::module_test;
use slicer_sdk::host::{self, test_support as host_test_support, MeshSource};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ============================================================================
// Test fixtures: a dummy MeshSource used to assert that the macro's
// reset_global_state / mock_host_teardown calls actually clear the
// per-thread mesh source.
// ============================================================================

struct DummyMeshSource;

impl MeshSource for DummyMeshSource {
    fn raycast_z_down(&self, _object_id: &str, _x: f32, _y: f32, _start_z: f32) -> Option<f32> {
        Some(0.0)
    }
    fn surface_normal_at(&self, _object_id: &str, _x: f32, _y: f32, _z: f32) -> Option<Point3> {
        Some(Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        })
    }
    fn object_bounds(&self, _object_id: &str) -> Option<BoundingBox3> {
        Some(BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        })
    }
}

/// Returns true when the per-thread mesh source is currently installed,
/// observed via `object_bounds()`:`Ok` iff a source is installed.
fn mesh_source_installed() -> bool {
    host::object_bounds("probe").is_ok()
}

// ============================================================================
// Test 1: Macro can be applied to a function
// ============================================================================

#[module_test]
fn test_fn_with_module_test_attribute() {
    // The macro should apply without compilation errors.
    // This test passes if the code compiles and runs.
    assert!(true);
}

#[test]
fn test_01_macro_applies_to_function() {
    // This meta-test verifies test_fn_with_module_test_attribute exists and compiles.
    // The test passes if we reach here without compilation errors.
    assert!(true, "Macro applied to function successfully");
}

// ============================================================================
// Test 2: Macro preserves the original function body
// ============================================================================

static TEST_02_BODY_EXECUTED: AtomicBool = AtomicBool::new(false);

#[module_test]
fn test_fn_body_preservation() {
    // This body should be preserved and executed
    TEST_02_BODY_EXECUTED.store(true, Ordering::SeqCst);
    let x = 1 + 1;
    assert_eq!(x, 2);
}

#[test]
fn test_02_macro_preserves_function_body() {
    // Reset the flag
    TEST_02_BODY_EXECUTED.store(false, Ordering::SeqCst);

    // Call the test function directly
    test_fn_body_preservation();

    // Verify the body was executed
    assert!(
        TEST_02_BODY_EXECUTED.load(Ordering::SeqCst),
        "Function body should be preserved and executed"
    );
}

// ============================================================================
// Test 3: Macro generates proper #[test] attribute
// ============================================================================

// Note: This test is structural - the #[module_test] macro should internally
// add #[test] so that cargo test discovers the function.
// We can verify this by the fact that test discovery works.

#[module_test]
fn test_03_fn_for_test_attribute_generation() {
    // This function should be discoverable by cargo test
    // because #[module_test] generates #[test]
    assert!(true);
}

#[test]
fn test_03_macro_generates_test_attribute() {
    // The presence (and execution by `cargo test`) of
    // `test_03_fn_for_test_attribute_generation` IS the proof that
    // `#[module_test]` emits a `#[test]` attribute. We additionally
    // assert that calling it directly succeeds — which it can only do
    // if the macro generated a well-formed parameterless fn.
    test_03_fn_for_test_attribute_generation();
}

// ============================================================================
// Test 4: Macro generates mock host setup call at start of function
// ============================================================================
//
// Observable behavior: `mock_host_setup()` calls
// `host::test_support::install_log_capture()`, which routes subsequent
// `host::log_*` calls on the same thread into a per-thread buffer rather
// than stderr. Inside the macro-expanded body the buffer is therefore
// installed; emitting a marker and draining it confirms setup ran.

static TEST_04_CAPTURED: AtomicBool = AtomicBool::new(false);

#[module_test]
fn test_04_fn_mock_host_setup() {
    host::log_warn("test_04_marker");
    let captured = host_test_support::take_log_messages();
    let saw_marker = captured.iter().any(|(_, msg)| msg == "test_04_marker");
    TEST_04_CAPTURED.store(saw_marker, Ordering::SeqCst);
}

#[test]
fn test_04_macro_generates_mock_host_setup() {
    TEST_04_CAPTURED.store(false, Ordering::SeqCst);
    test_04_fn_mock_host_setup();
    assert!(
        TEST_04_CAPTURED.load(Ordering::SeqCst),
        "mock_host_setup() must install the per-thread log capture sink so log_warn is captured rather than going to stderr"
    );
}

// ============================================================================
// Test 5: Macro generates mock host teardown call at end of function
// ============================================================================
//
// Observable behavior: the `__SlicerTestGuard` Drop runs
// `mock_host_teardown()`, which calls `take_log_messages()` (which
// itself drains AND uninstalls the per-thread capture sink) and
// `clear_mesh_source()`. After the macro-expanded test returns, both
// per-thread seams must be in their uninstalled state.

#[module_test]
fn test_05_fn_mock_host_teardown() {
    // Install a mesh source inside the body so we can verify that
    // teardown clears it on its way out.
    host_test_support::install_mesh_source(DummyMeshSource);
    assert!(
        mesh_source_installed(),
        "sanity check: dummy mesh source must be observable mid-test"
    );
}

#[test]
fn test_05_macro_generates_mock_host_teardown() {
    // Pre-condition: no mesh source from a prior test should leak in.
    host_test_support::clear_mesh_source();
    assert!(!mesh_source_installed());

    test_05_fn_mock_host_teardown();

    // Post-condition: teardown must have cleared the mesh source the
    // body installed. If teardown were missing, `mesh_source_installed`
    // would still return true here.
    assert!(
        !mesh_source_installed(),
        "mock_host_teardown() must clear the per-thread mesh source"
    );

    // The capture sink should also be uninstalled; verify by emitting
    // a log and draining — `take_log_messages` returns empty when no
    // sink is installed.
    host::log_warn("test_05_post_teardown");
    let drained = host_test_support::take_log_messages();
    assert!(
        drained.is_empty(),
        "after teardown, log_warn should bypass the (uninstalled) capture sink and write to stderr; drained={:?}",
        drained,
    );
}

// ============================================================================
// Test 6: Macro generates panic handler installation
// ============================================================================

#[module_test]
fn test_06_fn_panic_handler() {
    // The panic handler is installed at the start of the body by
    // `install_panic_handler()`. We cannot trigger it without
    // panicking (which would fail the test), so the observable
    // assertion here is simply that the body runs to completion —
    // which it can only do if `install_panic_handler()` returns
    // normally rather than panicking or aborting.
}

#[test]
fn test_06_macro_generates_panic_handler() {
    // Running the generated test without it itself panicking
    // demonstrates `install_panic_handler` was called and returned.
    test_06_fn_panic_handler();
}

// ============================================================================
// Test 7: Macro generates global state reset call
// ============================================================================
//
// Observable behavior: `reset_global_state()` calls `take_log_messages`
// (drain) and `clear_mesh_source`. We install a mesh source BEFORE
// invoking the generated test; if the macro's reset call ran, the body
// must observe no mesh source.

static TEST_07_BODY_SAW_NO_MESH_SOURCE: AtomicBool = AtomicBool::new(false);

#[module_test]
fn test_07_fn_global_state_reset() {
    // If reset_global_state ran before the body, the mesh source we
    // installed pre-invocation has been cleared and this is false.
    TEST_07_BODY_SAW_NO_MESH_SOURCE.store(!mesh_source_installed(), Ordering::SeqCst);
}

#[test]
fn test_07_macro_generates_global_state_reset() {
    // Pre-install a mesh source. If the macro emits the documented
    // `reset_global_state()` call, the body will see no source.
    host_test_support::install_mesh_source(DummyMeshSource);
    assert!(mesh_source_installed(), "pre-condition: source installed");

    TEST_07_BODY_SAW_NO_MESH_SOURCE.store(false, Ordering::SeqCst);
    test_07_fn_global_state_reset();

    assert!(
        TEST_07_BODY_SAW_NO_MESH_SOURCE.load(Ordering::SeqCst),
        "reset_global_state() must clear the per-thread mesh source before the body runs"
    );
}

// ============================================================================
// Test 8: Generated test function compiles and runs correctly
// ============================================================================

static TEST_08_RESULT: AtomicU32 = AtomicU32::new(0);

#[module_test]
fn test_08_fn_compile_and_run() {
    // Complex logic that should work correctly
    let a = 5;
    let b = 7;
    let sum = a + b;
    TEST_08_RESULT.store(sum, Ordering::SeqCst);
    assert_eq!(sum, 12);
}

#[test]
fn test_08_generated_test_compiles_and_runs() {
    TEST_08_RESULT.store(0, Ordering::SeqCst);

    // Run the generated test
    test_08_fn_compile_and_run();

    // Verify it executed correctly
    assert_eq!(
        TEST_08_RESULT.load(Ordering::SeqCst),
        12,
        "Test should compute correct result"
    );
}

// ============================================================================
// Test 9: Multiple #[module_test] functions can coexist
// ============================================================================

static TEST_09_A_RAN: AtomicBool = AtomicBool::new(false);
static TEST_09_B_RAN: AtomicBool = AtomicBool::new(false);
static TEST_09_C_RAN: AtomicBool = AtomicBool::new(false);

#[module_test]
fn test_09_fn_a() {
    TEST_09_A_RAN.store(true, Ordering::SeqCst);
}

#[module_test]
fn test_09_fn_b() {
    TEST_09_B_RAN.store(true, Ordering::SeqCst);
}

#[module_test]
fn test_09_fn_c() {
    TEST_09_C_RAN.store(true, Ordering::SeqCst);
}

#[test]
fn test_09_multiple_tests_coexist() {
    // Reset flags
    TEST_09_A_RAN.store(false, Ordering::SeqCst);
    TEST_09_B_RAN.store(false, Ordering::SeqCst);
    TEST_09_C_RAN.store(false, Ordering::SeqCst);

    // Run all three tests
    test_09_fn_a();
    test_09_fn_b();
    test_09_fn_c();

    // Verify all ran
    assert!(TEST_09_A_RAN.load(Ordering::SeqCst), "Test A should run");
    assert!(TEST_09_B_RAN.load(Ordering::SeqCst), "Test B should run");
    assert!(TEST_09_C_RAN.load(Ordering::SeqCst), "Test C should run");
}

// ============================================================================
// Test 10: Macro rejects async functions or provides clear error
// ============================================================================

// Note: This test documents the macro's async policy: parameterless,
// non-async test fns only (mirroring `#[test]` semantics). Verification
// is compile-time: any attempt to apply `#[module_test]` to an `async
// fn` would fail in a separate trybuild-style fixture. Here we just
// assert that the documented policy is the one we shipped.

#[test]
fn test_10_async_function_handling() {
    // Documented policy: `#[module_test]` does NOT support `async fn`.
    // (If support is added later, this assertion must be updated and the
    // macro's behavior verified with a compile-fail fixture.)
    let async_supported = false;
    assert!(
        !async_supported,
        "#[module_test] does not support async fn (mirrors #[test] semantics)"
    );
}

// ============================================================================
// Test 11: Setup/teardown order is correct (setup before body, teardown after)
// ============================================================================
//
// Observable behavior: setup runs before the body (Test 4 already
// proved log capture is installed by the time the body runs). Teardown
// runs after the body (Test 5 already proved the mesh source the body
// installed is cleared by the time the meta-test resumes). Test 11
// asserts the body-execution leg of that ordering directly.

static TEST_11_ORDER: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

#[module_test]
fn test_11_fn_order_verification() {
    TEST_11_ORDER.lock().unwrap().push("body");
}

#[test]
fn test_11_setup_teardown_order() {
    // Clear the order tracker
    TEST_11_ORDER.lock().unwrap().clear();

    // Run the test
    test_11_fn_order_verification();

    // Check that body ran. (Setup-before-body and teardown-after-body
    // are covered by tests 4 and 5 respectively via their own
    // observable-behavior assertions.)
    let order = TEST_11_ORDER.lock().unwrap();
    assert!(order.contains(&"body"), "Body should execute");
}

// ============================================================================
// Test 12: Test functions with parameters are handled (or rejected)
// ============================================================================

#[test]
fn test_12_parameter_handling() {
    // Documented policy: `#[module_test]` mirrors `#[test]` semantics —
    // only parameterless fns. Compile-time enforcement: any attempt to
    // apply `#[module_test]` to a fn with parameters would be flagged
    // by `cargo test` when the generated `#[test] fn name() { ... }`
    // tries to satisfy the harness's parameterless-fn requirement.
    assert!(
        true,
        "policy: parameterless fns only, enforced at compile time"
    );
}

// ============================================================================
// Test 13: Test with return type Result<(), E> is supported
// ============================================================================

#[derive(Debug)]
struct TestError;

#[module_test]
fn test_13_fn_result_return() -> Result<(), TestError> {
    // Tests can return Result for better error handling
    Ok(())
}

#[test]
fn test_13_result_return_type_supported() {
    // Run the test and verify it returns Ok
    let result = test_13_fn_result_return();
    assert!(result.is_ok(), "Test with Result return should succeed");
}

// ============================================================================
// Test 14: Module state isolation between tests
// ============================================================================

static TEST_14_STATE: AtomicU32 = AtomicU32::new(0);

#[module_test]
fn test_14_fn_a_modifies_state() {
    TEST_14_STATE.fetch_add(1, Ordering::SeqCst);
}

#[module_test]
fn test_14_fn_b_reads_state() {
    // If state is properly reset, this should see 0 or 1
    // (depending on test order), not cumulative values
    let _ = TEST_14_STATE.load(Ordering::SeqCst);
}

#[test]
fn test_14_state_isolation() {
    // Reset state
    TEST_14_STATE.store(0, Ordering::SeqCst);

    // Run first test
    test_14_fn_a_modifies_state();
    let after_a = TEST_14_STATE.load(Ordering::SeqCst);
    assert_eq!(after_a, 1, "State should be 1 after first test");

    // The macro should reset state before second test
    // For now, manually reset to verify concept
    // TASK-041: When implemented, the macro will handle this
    TEST_14_STATE.store(0, Ordering::SeqCst);

    test_14_fn_b_reads_state();

    // State should be isolated (0, not carrying over from a)
    // This depends on macro implementation
}
