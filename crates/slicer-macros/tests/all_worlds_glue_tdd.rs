//! TASK-109 closure: `#[slicer_module]` must emit real typed
//! `wit_bindgen`-backed export glue for every supported world — not a
//! placeholder `-> i32 { 0 }` shim. Source-level guards sit alongside
//! the end-to-end round-trip witnesses in
//! `crates/slicer-runtime/tests/macro_all_worlds_roundtrip_tdd.rs`.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

fn macro_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    fs::read_to_string(path).expect("read slicer-macros src/lib.rs")
}

#[test]
fn macro_has_world_dispatch_with_all_four_world_kinds() {
    let src = macro_src();
    assert!(
        src.contains("enum WorldGlueKind"),
        "world dispatch type is present"
    );
    for kind in ["Postpass", "Finalization", "Prepass", "Layer"] {
        assert!(
            src.contains(&format!("WorldGlueKind::{kind}")),
            "WorldGlueKind::{kind} variant must be routed by resolve_world_glue"
        );
    }
}

#[test]
fn macro_emits_world_builder_for_each_supported_world() {
    let src = macro_src();
    for builder in [
        "build_postpass_world_glue",
        "build_finalization_world_glue",
        "build_prepass_world_glue",
        "build_layer_world_glue",
    ] {
        assert!(
            src.contains(builder),
            "missing per-world glue builder: {builder}"
        );
    }
}

#[test]
fn macro_emits_wit_bindgen_generate_for_all_world_names() {
    let src = macro_src();
    // Every world's glue builder must include a wit_bindgen::generate!
    // invocation targeting the correct world string.
    assert!(src.contains("::wit_bindgen::generate!"));
    for world in [
        "postpass-module",
        "finalization-module",
        "prepass-module",
        "layer-module",
    ] {
        assert!(
            src.contains(&format!("\"{world}\"")),
            "macro must target world {world}"
        );
    }
}

#[test]
fn macro_emits_export_registration_for_every_world_component() {
    let src = macro_src();
    for component in [
        "__SlicerPostpassComponent",
        "__SlicerFinalizationComponent",
        "__SlicerPrepassComponent",
        "__SlicerLayerComponent",
    ] {
        assert!(
            src.contains(&format!("export!({component})")),
            "macro must register {component} via export!"
        );
    }
}

#[test]
fn macro_routes_supported_stages_into_trait_methods() {
    let src = macro_src();
    for path in [
        "::slicer_sdk::traits::PostpassModule",
        "::slicer_sdk::traits::FinalizationModule",
        "::slicer_sdk::traits::PrepassModule",
        "::slicer_sdk::traits::LayerModule",
    ] {
        assert!(
            src.contains(path),
            "macro must route through the {path} trait"
        );
    }
}

#[test]
fn macro_no_longer_emits_placeholder_shim_for_supported_worlds() {
    let src = macro_src();
    // The placeholder-skip predicate is `real_glue_world.is_some()`;
    // worlds without real glue are the only ones that still emit shims.
    assert!(
        src.contains("real_glue_world.is_some()"),
        "macro must gate the placeholder shim path behind the real-glue detector"
    );
    assert!(
        src.contains("let skip_lifecycle_shims = real_glue_world.is_some();"),
        "macro must skip lifecycle shims wherever real glue is emitted"
    );
}

#[test]
fn macro_layer_world_covers_all_eight_stage_exports_plus_lifecycle() {
    let src = macro_src();
    for export_arm in [
        "fn on_print_start",
        "fn on_print_end",
        "fn run_slice_postprocess",
        "fn run_perimeters",
        "fn run_wall_postprocess",
        "fn run_infill",
        "fn run_infill_postprocess",
        "fn run_support",
        "fn run_support_postprocess",
        "fn run_path_optimization",
    ] {
        assert!(
            src.contains(export_arm),
            "macro Guest impl for layer-module must implement {export_arm}"
        );
    }
}

#[test]
fn macro_prepass_covers_mesh_analysis_and_layer_planning() {
    let src = macro_src();
    assert!(src.contains("fn run_mesh_analysis"));
    assert!(src.contains("fn run_layer_planning"));
}

#[test]
fn macro_groups_flat_paint_stroke_vertices_into_triangle_triplets() {
    let src = macro_src();
    assert!(
        src.contains("chunks_exact(3)"),
        "paint stroke bridge must regroup the flat WIT point stream into triangle triplets"
    );
    assert!(
        !src.contains(".map(|point| [[point.x, point.y, point.z]; 3])"),
        "paint stroke bridge must not duplicate each point into a degenerate triangle"
    );
}

#[test]
fn macro_finalization_covers_run_finalization() {
    let src = macro_src();
    assert!(src.contains("fn run_finalization"));
}

#[test]
fn macro_config_adapter_is_shared_across_worlds() {
    let src = macro_src();
    assert!(
        src.contains("fn emit_world_preamble"),
        "macro must share the wit_bindgen preamble + config adapter emission across worlds"
    );
    assert!(
        src.contains("fn __slicer_adapt_config"),
        "macro preamble must emit the shared ConfigView adapter"
    );
}
