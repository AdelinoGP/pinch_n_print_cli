//! TASK-109: `#[slicer_module]` must emit real typed export glue for
//! `PostPass::TextPostProcess`, not the placeholder `#[export_name] ->
//! i32 { 0 }` shim (the `slicer:postpass-text-postprocess` package;
//! docs/05 §Module Entry Point).
//!
//! Source-level witness: the macro's own `src/lib.rs` must contain the
//! wit_bindgen::generate! invocation for the text-postprocess-module world and
//! must gate the placeholder stage shim out when the detected stage is
//! `PostPass::TextPostProcess`. If either regresses, this test fails
//! in CI — protecting the macro-level contract without requiring a
//! wasm32 build on every test run.
//!
//! End-to-end proof (a macro-authored guest round-tripping through
//! `WasmRuntimeDispatcher`) lives in
//! `crates/slicer-runtime/tests/macro_postpass_text_roundtrip_tdd.rs`.

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
        "macro must emit `wit_bindgen::generate!` for the typed postpass-text export"
    );
    assert!(
        src.contains(
            r#"../../slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit"#
        ),
        "macro must load the canonical postpass-text WIT package"
    );
    assert!(
        src.contains(
            r#"emit_world_preamble("text-postprocess-module", "text_postprocess", wit_inline)"#
        ),
        "macro must feed the text-postprocess-module world into the shared bindgen preamble"
    );
    let postpass_wit =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit",
        ))
        .expect("read canonical postpass-text-postprocess.wit");
    assert!(
        postpass_wit.contains("package slicer:postpass-text-postprocess@1.0.0;")
            && postpass_wit.contains("interface text-postprocess")
            && postpass_wit.contains("world text-postprocess-module")
            && postpass_wit.contains("export text-postprocess"),
        "canonical WIT must declare the qualified \
         `slicer:postpass-text-postprocess/text-postprocess@1.0.0` contract"
    );
}

#[test]
fn macro_wires_user_trait_into_run_text_postprocess_export() {
    // The emitted Guest impl must route into the user's `PostpassModule`
    // trait — anything else would be a marker-only export.
    let src = macro_src();
    assert!(
        src.contains(
            "impl exports::slicer::postpass_text_postprocess::text_postprocess::Guest for \
             __SlicerPostpassTextComponent"
        ),
        "macro must implement the qualified postpass-text Guest interface"
    );
    assert!(
        src.contains(
            "let out = <#self_ty as ::slicer_sdk::traits::PostpassModule>::run_text_postprocess("
        ) && src.contains("&module, &gcode_text, &ir_config"),
        "macro's Guest::run must route text and config through PostpassModule::run_text_postprocess"
    );
    assert!(
        src.contains("export!(__SlicerPostpassTextComponent)"),
        "macro must register the postpass-text component with `export!`"
    );
}

#[test]
fn macro_skips_placeholder_shim_for_postpass_text_stage() {
    // Postpass stages use the real wit_bindgen export glue. The generic
    // stage-shim path remains available for unsupported worlds, but must be
    // suppressed when a real per-stage world is selected.
    let src = macro_src();
    assert!(
        src.contains("PostPass::TextPostProcess"),
        "macro source must reference the PostPass::TextPostProcess stage gate"
    );
    assert!(
        src.contains("if stage_export_name_literal.is_empty() || real_glue_world.is_some()"),
        "macro must suppress the placeholder shim whenever real world glue is selected"
    );
    assert!(
        src.contains(
            r#"if stage_id_literal == "PostPass::TextPostProcess" {
                build_postpass_text_glue(self_ty)"#
        ),
        "macro must route the text stage through its per-stage builder"
    );
    assert!(
        src.contains("#[export_name = #stage_export_name_literal]")
            && src.contains(r#"pub extern "C" fn #shim_name() -> i32 { 0 }"#),
        "macro must retain the placeholder shim only as the unsupported-world fallback"
    );
    let text_glue = src
        .split("fn build_postpass_text_glue")
        .nth(1)
        .and_then(|tail| tail.split("fn build_finalization_world_glue").next())
        .expect("postpass-text builder body is present");
    assert!(
        !text_glue.contains("#[export_name")
            && !text_glue.contains("extern \"C\"")
            && !text_glue.contains("-> i32 { 0 }"),
        "postpass-text builder must not contain an obsolete placeholder export shim"
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
