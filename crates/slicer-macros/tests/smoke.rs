use slicer_macros::{module_test, slicer_module};

#[slicer_module]
fn placeholder_module_value() -> i32 {
    7
}

#[module_test]
#[test]
fn smoke_uses_placeholder_macros() {
    assert_eq!(placeholder_module_value(), 7);
}
