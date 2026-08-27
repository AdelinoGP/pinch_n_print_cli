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

struct DeliveredWorld {
    stage_id: &'static str,
    glue_kind: &'static str,
    builder: &'static str,
    package: &'static str,
    interface: &'static str,
    world: &'static str,
    component: &'static str,
}

const DELIVERED_WORLDS: &[DeliveredWorld] = &[
    DeliveredWorld {
        stage_id: "PostPass::GCodePostProcess",
        glue_kind: "Postpass",
        builder: "build_postpass_gcode_glue",
        package: "postpass-gcode-postprocess",
        interface: "gcode-postprocess",
        world: "gcode-postprocess-module",
        component: "__SlicerPostpassGcodeComponent",
    },
    DeliveredWorld {
        stage_id: "PostPass::TextPostProcess",
        glue_kind: "Postpass",
        builder: "build_postpass_text_glue",
        package: "postpass-text-postprocess",
        interface: "text-postprocess",
        world: "text-postprocess-module",
        component: "__SlicerPostpassTextComponent",
    },
    DeliveredWorld {
        stage_id: "PostPass::LayerFinalization",
        glue_kind: "Finalization",
        builder: "build_finalization_world_glue",
        package: "finalization-layer-finalization",
        interface: "layer-finalization",
        world: "layer-finalization-module",
        component: "__SlicerFinalizationComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::SlicePostProcess",
        glue_kind: "LayerSlicePostprocess",
        builder: "build_layer_slice_postprocess_glue",
        package: "layer-slice-postprocess",
        interface: "slice-postprocess",
        world: "slice-postprocess-module",
        component: "__SlicerLayerSlicePostprocessComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::Perimeters",
        glue_kind: "LayerPerimeters",
        builder: "build_layer_perimeters_glue",
        package: "layer-perimeters",
        interface: "perimeters",
        world: "perimeters-module",
        component: "__SlicerLayerPerimetersComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::PerimetersPostProcess",
        glue_kind: "LayerPerimetersPostprocess",
        builder: "build_layer_perimeters_postprocess_glue",
        package: "layer-perimeters-postprocess",
        interface: "perimeters-postprocess",
        world: "perimeters-postprocess-module",
        component: "__SlicerLayerPerimetersPostprocessComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::Infill",
        glue_kind: "LayerInfill",
        builder: "build_layer_infill_glue",
        package: "layer-infill",
        interface: "infill",
        world: "infill-module",
        component: "__SlicerLayerInfillComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::InfillPostProcess",
        glue_kind: "LayerInfillPostprocess",
        builder: "build_layer_infill_postprocess_glue",
        package: "layer-infill-postprocess",
        interface: "infill-postprocess",
        world: "infill-postprocess-module",
        component: "__SlicerLayerInfillPostprocessComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::Support",
        glue_kind: "LayerSupport",
        builder: "build_layer_support_glue",
        package: "layer-support",
        interface: "support",
        world: "support-module",
        component: "__SlicerLayerSupportComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::SupportPostProcess",
        glue_kind: "LayerSupportPostprocess",
        builder: "build_layer_support_postprocess_glue",
        package: "layer-support-postprocess",
        interface: "support-postprocess",
        world: "support-postprocess-module",
        component: "__SlicerLayerSupportPostprocessComponent",
    },
    DeliveredWorld {
        stage_id: "Layer::PathOptimization",
        glue_kind: "LayerPathOptimization",
        builder: "build_layer_path_optimization_glue",
        package: "layer-path-optimization",
        interface: "path-optimization",
        world: "path-optimization-module",
        component: "__SlicerLayerPathOptimizationComponent",
    },
    DeliveredWorld {
        stage_id: "PrePass::MeshAnalysis",
        glue_kind: "PrepassMeshAnalysis",
        builder: "build_prepass_mesh_analysis_glue",
        package: "prepass-mesh-analysis",
        interface: "mesh-analysis",
        world: "mesh-analysis-module",
        component: "__SlicerPrepassMeshAnalysisComponent",
    },
    DeliveredWorld {
        stage_id: "PrePass::LayerPlanning",
        glue_kind: "PrepassLayerPlanning",
        builder: "build_prepass_layer_planning_glue",
        package: "prepass-layer-planning",
        interface: "layer-planning",
        world: "layer-planning-module",
        component: "__SlicerPrepassLayerPlanningComponent",
    },
    DeliveredWorld {
        stage_id: "PrePass::SeamPlanning",
        glue_kind: "PrepassSeamPlanning",
        builder: "build_prepass_seam_planning_glue",
        package: "prepass-seam-planning",
        interface: "seam-planning",
        world: "seam-planning-module",
        component: "__SlicerPrepassSeamPlanningComponent",
    },
    DeliveredWorld {
        stage_id: "PrePass::SupportGeometry",
        glue_kind: "PrepassSupportGeometry",
        builder: "build_prepass_support_geometry_glue",
        package: "prepass-support-geometry",
        interface: "support-geometry",
        world: "support-geometry-module",
        component: "__SlicerPrepassSupportGeometryComponent",
    },
];

fn snake_case(value: &str) -> String {
    value.replace('-', "_")
}

fn guest_impl<'a>(src: &'a str, world: &DeliveredWorld) -> &'a str {
    let marker = format!(
        "impl exports::slicer::{}::{}::Guest for {}",
        snake_case(world.package),
        snake_case(world.interface),
        world.component
    );
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("missing qualified Guest impl: {marker}"));
    let rest = &src[start..];
    let end_marker = format!("export!({})", world.component);
    let end = rest
        .find(&end_marker)
        .unwrap_or_else(|| panic!("missing export registration: {end_marker}"));
    &rest[..end + end_marker.len()]
}

#[test]
fn macro_has_stage_dispatch_for_all_delivered_worlds() {
    let src = macro_src();
    assert!(
        src.contains("enum StageGlueKind"),
        "stage dispatch type is present"
    );
    let resolver = src
        .split("fn resolve_stage_glue")
        .nth(1)
        .and_then(|tail| tail.split("/// The statement every macro-generated").next())
        .expect("resolve_stage_glue body is present");

    for world in DELIVERED_WORLDS {
        let matching_line = resolver
            .lines()
            .find(|line| line.contains(world.stage_id))
            .unwrap_or_else(|| panic!("missing stage dispatch for {}", world.stage_id));
        assert!(
            matching_line.contains(&format!("StageGlueKind::{}", world.glue_kind)),
            "{} must dispatch to StageGlueKind::{}",
            world.stage_id,
            world.glue_kind
        );
    }
    assert!(!resolver.contains("Some(\"LayerModule\")"));
    assert!(!resolver.contains("Some(\"PrepassModule\")"));
}

#[test]
fn macro_emits_world_builder_for_each_supported_world() {
    let src = macro_src();
    for world in DELIVERED_WORLDS {
        assert!(
            src.contains(&format!("fn {}(", world.builder)),
            "missing per-stage glue builder: {}",
            world.builder
        );
    }
    assert!(!src.contains("fn build_layer_world_glue("));
    assert!(!src.contains("fn build_prepass_world_glue("));
    assert!(!src.contains("StageGlueKind::Layer)"));
    assert!(!src.contains("StageGlueKind::Prepass)"));
    let active_glue = src
        .split("let world_glue: TokenStream2 = match real_glue_world")
        .nth(1)
        .and_then(|tail| tail.split("let wasm_export_shims").next())
        .expect("active stage glue selection is present");
    // `retired_layer_glue` / `retired_prepass_glue` were 1,847 lines of
    // `#[cfg(any())]` code that never compiled and could not be re-enabled
    // (one `include_str!`d a deleted .wit, the other named an undefined
    // const). Deleted in packet 164; git holds the history.
    for retired in [
        "build_layer_world_glue",
        "build_prepass_world_glue",
        "world_layer",
        "world_prepass",
    ] {
        assert!(
            !active_glue.contains(retired),
            "active glue selection must not generate retired tier-world glue: {retired}"
        );
    }
}

#[test]
fn macro_emits_wit_bindgen_generate_for_all_world_names() {
    let src = macro_src();
    assert!(src.contains("::wit_bindgen::generate!"));
    for world in DELIVERED_WORLDS {
        let package_path = format!("../../slicer-schema/wit/deps/{0}/{0}.wit", world.package);
        let qualified_guest = format!(
            "exports::slicer::{}::{}::Guest",
            snake_case(world.package),
            snake_case(world.interface)
        );
        assert!(
            src.contains(&format!("\"{}\"", world.world)),
            "macro must target world {}",
            world.world
        );
        assert!(
            src.contains(&package_path),
            "macro must load package {}",
            package_path
        );
        assert!(
            src.contains(&qualified_guest),
            "macro must use qualified package/interface {}",
            qualified_guest
        );
    }
}

#[test]
fn macro_emits_export_registration_for_every_world_component() {
    let src = macro_src();
    for world in DELIVERED_WORLDS {
        let body = guest_impl(&src, world);
        assert!(
            src.contains(&format!("export!({})", world.component)),
            "macro must register {} via export!",
            world.component
        );
        assert_eq!(
            body.matches("fn run(").count(),
            1,
            "{} must expose exactly one run export",
            world.component
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
}

/// Each of the eight layer stages must delegate to its own `LayerModule`
/// trait method from its own per-stage glue builder.
///
/// This previously asserted `fn run_slice_postprocess` etc. appeared in the
/// macro source — the signatures of the single tier-world `Guest` impl. Since
/// packet 164 each stage has its own `Guest` impl whose method is `fn run`, so
/// those strings survived only inside 1,847 lines of `#[cfg(any())]` dead code
/// and the assertions passed for the wrong reason. Pin the delegation instead,
/// which is what the glue actually has to get right.
#[test]
fn macro_layer_stages_each_delegate_to_their_sdk_trait_method() {
    let src = macro_src();
    for method in [
        "run_slice_postprocess",
        "run_perimeters",
        "run_wall_postprocess",
        "run_infill",
        "run_infill_postprocess",
        "run_support",
        "run_support_postprocess",
        "run_path_optimization",
    ] {
        let delegation = format!("<#self_ty as ::slicer_sdk::traits::LayerModule>::{method}(");
        assert!(
            src.contains(&delegation),
            "per-stage layer glue must delegate to LayerModule::{method}"
        );
    }
}

/// Same correction as the layer case above, for the four prepass stages.
#[test]
fn macro_prepass_stages_each_delegate_to_their_sdk_trait_method() {
    let src = macro_src();
    // Each stage lists every trait method whose presence satisfies delegation.
    //
    // `PrePass::SupportGeometry` accepts two spellings. The glue calls
    // `PrepassModule::run_support_geometry_with_analysis`, which is a *trait*
    // method on `PrepassModule` whose SDK default impl (see
    // `run_support_geometry_with_analysis` in `crates/slicer-sdk/src/traits.rs`)
    // forwards to `run_support_geometry`. Delegation still lands on the trait —
    // one hop later — so accepting either spelling is not a weakened assertion;
    // requiring the bare name would demand the glue skip the analysis argument.
    // The other three stages have no such default-delegation chain and must
    // match their method directly.
    for (stage_method, accepted) in [
        ("run_mesh_analysis", &["run_mesh_analysis"][..]),
        ("run_layer_planning", &["run_layer_planning"][..]),
        ("run_seam_planning", &["run_seam_planning"][..]),
        (
            "run_support_geometry",
            &["run_support_geometry", "run_support_geometry_with_analysis"][..],
        ),
    ] {
        assert!(
            accepted.iter().any(|method| src.contains(&format!(
                "<#self_ty as ::slicer_sdk::traits::PrepassModule>::{method}("
            ))),
            "per-stage prepass glue must delegate to PrepassModule::{stage_method}              (accepted spellings: {accepted:?})"
        );
    }
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
    let finalization = DELIVERED_WORLDS
        .iter()
        .find(|world| world.stage_id == "PostPass::LayerFinalization")
        .expect("finalization world is in the delivered-world table");
    let body = guest_impl(&src, finalization);
    assert!(body.contains(">::run_finalization("));
    assert_eq!(body.matches("fn run(").count(), 1);
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
