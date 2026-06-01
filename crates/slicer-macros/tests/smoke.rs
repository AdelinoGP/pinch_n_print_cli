//! Smoke tests for slicer-macros crate.
//!
//! These tests verify basic macro compilation and usage.

#![allow(missing_docs, clippy::result_unit_err)]

use slicer_macros::{module_test, slicer_module};

// The `#[module_test]` macro now expands to fully-qualified calls into
// `::slicer_sdk::test_support::*`, so smoke tests pull in `slicer-sdk`
// with the `test` feature via dev-dependencies — no local stubs needed.

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
