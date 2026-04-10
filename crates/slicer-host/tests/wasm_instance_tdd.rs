//! TDD red tests for TASK-100: WasmInstance wrapper over wasmtime.

use slicer_host::wasm_instance::{HostState, WasmEngine, WasmLoadError};

/// WasmEngine::new() succeeds and produces a usable engine with component model enabled.
#[test]
fn engine_creates_successfully() {
    let engine = WasmEngine::new();
    // If component model is not enabled, wasmtime would panic on component ops.
    // Just verify construction succeeds.
    assert!(!format!("{:?}", engine).is_empty());
}

/// Compiling garbage bytes returns a structured WasmLoadError.
#[test]
fn compile_invalid_bytes_returns_error() {
    let engine = WasmEngine::new();
    let result = engine.compile_component(b"not valid wasm at all");
    assert!(result.is_err());
    let err = result.unwrap_err();
    match &err {
        WasmLoadError::CompilationFailed { reason, .. } => {
            assert!(!reason.is_empty(), "error reason must not be empty");
        }
        other => panic!("expected CompilationFailed, got: {other}"),
    }
}

/// Compiling a minimal valid WASM component succeeds.
#[test]
fn compile_minimal_component_succeeds() {
    let engine = WasmEngine::new();
    // Minimal valid component WAT (empty component).
    let wat = r#"(component)"#;
    let wasm_bytes = wat::parse_str(wat).expect("WAT parse failed");
    let component = engine.compile_component(&wasm_bytes);
    assert!(component.is_ok(), "valid component should compile: {:?}", component.err());
}

/// HostState preserves module_id through construction.
#[test]
fn host_state_preserves_module_id() {
    let state = HostState::new("my-test-module".to_string());
    assert_eq!(state.module_id(), "my-test-module");
}

/// Instantiation of a minimal component preserves module_id accessible via WasmInstance.
#[test]
fn instantiate_preserves_module_id() {
    let engine = WasmEngine::new();
    let wat = r#"(component)"#;
    let wasm_bytes = wat::parse_str(wat).expect("WAT parse failed");
    let component = engine.compile_component(&wasm_bytes).expect("compile failed");
    let state = HostState::new("instance-mod".to_string());
    let instance = component.instantiate(&engine, state).expect("instantiate failed");
    assert_eq!(instance.module_id(), "instance-mod");
}

/// WasmLoadError variants have meaningful Display output.
#[test]
fn error_display_is_meaningful() {
    let err = WasmLoadError::CompilationFailed {
        reason: "bad magic number".to_string(),
    };
    let display = format!("{err}");
    assert!(
        display.contains("bad magic number"),
        "Display should include reason: {display}"
    );

    let err2 = WasmLoadError::InstantiationFailed {
        module_id: "test-mod".to_string(),
        reason: "missing import".to_string(),
    };
    let display2 = format!("{err2}");
    assert!(
        display2.contains("test-mod"),
        "Display should include module_id: {display2}"
    );
    assert!(
        display2.contains("missing import"),
        "Display should include reason: {display2}"
    );
}

/// Compiling empty bytes returns CompilationFailed (not a panic).
#[test]
fn compile_empty_bytes_returns_error() {
    let engine = WasmEngine::new();
    let result = engine.compile_component(b"");
    assert!(result.is_err());
    match result.unwrap_err() {
        WasmLoadError::CompilationFailed { .. } => {}
        other => panic!("expected CompilationFailed, got: {other}"),
    }
}
