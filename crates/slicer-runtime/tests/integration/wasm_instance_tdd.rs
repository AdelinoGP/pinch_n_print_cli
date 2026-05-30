//! TDD red tests for TASK-100: WasmInstance wrapper over wasmtime.

use slicer_runtime::wasm_instance::{HostState, WasmEngine, WasmLoadError};

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
    assert!(
        component.is_ok(),
        "valid component should compile: {:?}",
        component.err()
    );
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
    let component = engine
        .compile_component(&wasm_bytes)
        .expect("compile failed");
    let state = HostState::new("instance-mod".to_string());
    let instance = component
        .instantiate(&engine, state)
        .expect("instantiate failed");
    assert_eq!(instance.module_id(), "instance-mod");
}

/// Instantiation can use an explicit linker so future host imports are not
/// hard-coded behind the default path.
#[test]
fn instantiate_with_explicit_linker_preserves_module_id() {
    let engine = WasmEngine::new();
    let linker = engine.new_linker();
    let wat = r#"(component)"#;
    let wasm_bytes = wat::parse_str(wat).expect("WAT parse failed");
    let component = engine
        .compile_component(&wasm_bytes)
        .expect("compile failed");
    let state = HostState::new("linked-mod".to_string());
    let instance = component
        .instantiate_with_linker(&engine, state, &linker)
        .expect("instantiate failed");
    assert_eq!(instance.module_id(), "linked-mod");
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

/// call_void_export invokes a real WASM function and returns Ok.
#[test]
fn call_void_export_invokes_real_function() {
    let engine = WasmEngine::new();
    let wat = r#"
        (component
            (core module $m
                (func $f (export "run-infill"))
            )
            (core instance $i (instantiate $m))
            (func (export "run-infill") (canon lift (core func $i "run-infill")))
        )
    "#;
    let bytes = wat::parse_str(wat).expect("WAT parse failed");
    let component = engine.compile_component(&bytes).expect("compile failed");
    let state = HostState::new("call-test".to_string());
    let mut instance = component
        .instantiate(&engine, state)
        .expect("instantiate failed");

    let result = instance.call_void_export("run-infill");
    assert!(
        result.is_ok(),
        "call_void_export should succeed: {:?}",
        result.err()
    );
}

/// call_void_export on a missing export returns ExportNotFound.
#[test]
fn call_void_export_missing_export_returns_error() {
    let engine = WasmEngine::new();
    let wat = r#"(component)"#;
    let bytes = wat::parse_str(wat).expect("WAT parse failed");
    let component = engine.compile_component(&bytes).expect("compile failed");
    let state = HostState::new("missing-export".to_string());
    let mut instance = component
        .instantiate(&engine, state)
        .expect("instantiate failed");

    let result = instance.call_void_export("run-nonexistent");
    assert!(result.is_err());
    match result.unwrap_err() {
        slicer_runtime::WasmCallError::ExportNotFound {
            module_id,
            export_name,
            ..
        } => {
            assert_eq!(module_id, "missing-export");
            assert_eq!(export_name, "run-nonexistent");
        }
        other => panic!("expected ExportNotFound, got: {other}"),
    }
}

/// call_text_transform invokes a stringâ†’string WASM function.
#[test]
fn call_text_transform_invokes_real_function() {
    let engine = WasmEngine::new();
    let wat = r#"
        (component
            (core module $m
                (memory (export "memory") 1)
                (func $realloc (param i32 i32 i32 i32) (result i32) i32.const 16)
                (export "cabi_realloc" (func $realloc))
                (func $transform (param i32 i32) (result i32)
                    i32.const 0 i32.const 16 i32.store
                    i32.const 4 i32.const 0 i32.store
                    i32.const 0
                )
                (export "run-text-postprocess" (func $transform))
            )
            (core instance $i (instantiate $m))
            (alias core export $i "memory" (core memory $mem))
            (alias core export $i "cabi_realloc" (core func $realloc))
            (func (export "run-text-postprocess") (param "text" string) (result string)
                (canon lift (core func $i "run-text-postprocess") (memory $mem) (realloc (func $realloc)))
            )
        )
    "#;
    let bytes = wat::parse_str(wat).expect("WAT parse failed");
    let component = engine.compile_component(&bytes).expect("compile failed");
    let state = HostState::new("text-test".to_string());
    let mut instance = component
        .instantiate(&engine, state)
        .expect("instantiate failed");

    let result = instance.call_text_transform("run-text-postprocess", "; some gcode\n");
    assert!(
        result.is_ok(),
        "call_text_transform should succeed: {:?}",
        result.err()
    );
}
