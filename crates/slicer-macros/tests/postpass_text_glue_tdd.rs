//! TASK-109: `#[slicer_module]` must emit real typed export glue for
//! `PostPass::TextPostProcess`, not the placeholder `#[export_name] ->
//! i32 { 0 }` shim (docs/03 wit/world-postpass.wit; docs/05 §Module
//! Entry Point).
//!
//! Source-level witness: the macro's own `src/lib.rs` must contain the
//! wit_bindgen::generate! invocation for the postpass-module world and
//! must gate the placeholder stage shim out when the detected stage is
//! `PostPass::TextPostProcess`. If either regresses, this test fails
//! in CI — protecting the macro-level contract without requiring a
//! wasm32 build on every test run.
//!
//! End-to-end proof (a macro-authored guest round-tripping through
//! `WasmRuntimeDispatcher`) lives in
//! `crates/slicer-host/tests/macro_postpass_text_roundtrip_tdd.rs`.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

fn macro_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    fs::read_to_string(path).expect("read slicer-macros src/lib.rs")
}

#[test]
fn macro_emits_wit_bindgen_generate_for_postpass_text_world() {
    let src = macro_src();
    assert!(
        src.contains("::wit_bindgen::generate!"),
        "macro must emit `wit_bindgen::generate!` so PostPass::TextPostProcess \
         modules get a real component-model export, not a placeholder i32 shim"
    );
    assert!(
        src.contains(r#""postpass-module""#),
        "macro's wit_bindgen invocation must target the `postpass-module` world"
    );
    assert!(
        src.contains("run-text-postprocess"),
        "macro inline WIT must declare the documented `run-text-postprocess` export"
    );
}

#[test]
fn macro_wires_user_trait_into_run_text_postprocess_export() {
    // The emitted Guest impl must route into the user's `PostpassModule`
    // trait — anything else would be a marker-only export.
    let src = macro_src();
    assert!(
        src.contains("impl Guest for __SlicerPostpassComponent"),
        "macro must emit `impl Guest for ...` to wire the component export"
    );
    assert!(
        src.contains("::slicer_sdk::traits::PostpassModule"),
        "macro must route through the `PostpassModule` trait for typed dispatch"
    );
    assert!(
        src.contains("export!(__SlicerPostpassComponent)"),
        "macro must register the component with `export!` so the .wasm artifact \
         actually exposes the postpass-module world"
    );
}

#[test]
fn macro_skips_placeholder_shim_for_postpass_text_stage() {
    // The stage-export shim (`extern "C" fn ... -> i32 { 0 }`) must NOT
    // be emitted when the detected stage is PostPass::TextPostProcess —
    // it would collide at link time with the real wit_bindgen export
    // and leak a non-component symbol into the .wasm.
    let src = macro_src();
    assert!(
        src.contains("PostPass::TextPostProcess"),
        "macro source must reference the PostPass::TextPostProcess stage gate"
    );
    assert!(
        src.contains("real_glue_world"),
        "macro must carry the world-dispatch gate that routes \
         supported worlds through the real glue"
    );
    assert!(
        src.contains("skip_lifecycle_shims"),
        "macro must also skip the fake lifecycle shims for the world that emits \
         real glue — the postpass-module WIT world does not declare \
         `on-print-start` / `on-print-end` as exports"
    );
}

#[test]
fn macro_inline_wit_configures_typed_config_view_resource() {
    // The glue relies on the wit-bindgen-generated ConfigView resource
    // carrying typed accessors (`get`, `keys`). The macro includes
    // config.wit (which defines `resource config-view`) via the WIT
    // `include` directive.
    let src = macro_src();
    assert!(
        src.contains("resource config-view")
            || src.contains("include") && src.contains("config.wit"),
        "macro inline WIT must declare the `config-view` resource (directly or via include) \
         so typed config reads are available inside the guest's run_text_postprocess body"
    );
    assert!(
        src.contains("get-string: func(key: string) -> option<string>")
            || src.contains("config.wit"),
        "macro inline WIT must expose the typed `get-string` accessor (directly or via config.wit include)"
    );
}

#[test]
fn macro_imports_config_value_from_config_types_interface() {
    // The variant lives in the `config-types` interface namespace (the
    // world only re-imports `config-view`). The adapter needs an
    // explicit `use` statement inside the generated module or else the
    // match arms fail to resolve `ConfigValue::*`.
    let src = macro_src();
    // After the refactor, the preamble uses a typed alias path built
    // per world, so assert the postpass world's namespace is referenced.
    assert!(
        src.contains("postpass_world::config_types::ConfigValue")
            || src.contains("__SlicerWitConfigValue"),
        "macro's emitted module must bring the wit-bindgen ConfigValue variant \
         from the postpass_world::config_types namespace into scope"
    );
}
