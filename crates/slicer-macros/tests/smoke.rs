//! Smoke tests for slicer-macros crate.
//!
//! These tests verify basic macro compilation and usage.

use slicer_macros::{module_test, slicer_module};

// ============================================================================
// Mock host setup/teardown functions for #[module_test] macro
// These are stubs that the macro-generated code calls
// ============================================================================

/// Mock host setup function (stub for smoke tests)
#[doc(hidden)]
pub fn __slicer_test_mock_host_setup() {}

/// Mock host teardown function (stub for smoke tests)
#[doc(hidden)]
pub fn __slicer_test_mock_host_teardown() {}

/// Panic handler installation function (stub for smoke tests)
#[doc(hidden)]
pub fn __slicer_test_install_panic_handler() {}

/// Global state reset function (stub for smoke tests)
#[doc(hidden)]
pub fn __slicer_test_reset_global_state() {}

/// Mock LayerModule trait for smoke tests.
pub trait LayerModule: Sized {
    fn on_print_start() -> Result<Self, ()>;
}

/// Simple module struct for smoke testing.
pub struct SmokeModule {
    value: i32,
}

#[slicer_module]
impl LayerModule for SmokeModule {
    fn on_print_start() -> Result<Self, ()> {
        Ok(SmokeModule { value: 7 })
    }
}

#[module_test]
fn smoke_uses_placeholder_macros() {
    let module = SmokeModule::on_print_start().unwrap();
    assert_eq!(module.value, 7);
}
