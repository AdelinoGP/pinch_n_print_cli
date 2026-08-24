#![allow(missing_docs)]

use std::sync::Arc;

use sdk_layer_infill_guest::SdkLayerInfillModule;
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LayerStageCommit, Point2,
    Point3WithWidth, Polygon, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::native::NativePrepassResponse;
use slicer_sdk::native::{NativeLayerRequest, NativeLayerResponse, NativeStageEntry};
use slicer_sdk::prepass_builders::{
    LayerPlanOutput, MeshAnalysisOutput, SeamPlanningOutput, SupportGeometryOutput,
};
use slicer_sdk::prepass_types::{
    FacetAnnotation, FacetClass, LayerProposal, RegionLayerProposal, ScoredSeamCandidate,
    SeamPlanEntry, SeamReason, SupportPlanEntry, SurfaceGroupProposal,
};
use slicer_wasm_host::marshal::native::commit_native_prepass_response;

use crate::common::wasm_cache;
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

fn non_empty_slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 5,
        z: 1.0,
        regions: vec![SlicedRegion {
            object_id: "parity-object".to_string(),
            region_id: 0,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 10_000, y: 0 },
                        Point2 {
                            x: 10_000,
                            y: 10_000,
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            ..Default::default()
        }],
    }
}

fn support_slice() -> SliceIR {
    let mut slice = non_empty_slice();
    slice.regions = vec![
        SlicedRegion {
            object_id: "support-object-a".to_string(),
            region_id: 7,
            ..slice.regions[0].clone()
        },
        SlicedRegion {
            object_id: "support-object-b".to_string(),
            region_id: 11,
            ..slice.regions[0].clone()
        },
    ];
    slice
}

fn support_path(x: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x,
                y: 0.0,
                z: 1.0,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: x + 1.0,
                y: 0.0,
                z: 1.0,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
        ],
        ..extrusion_path3d_base(ExtrusionRole::SupportMaterial)
    }
}

fn native_support_entry(_: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
    let mut support = SupportOutputBuilder::new();
    support.begin_region("support-object-a", 7);
    support.push_support_path(support_path(0.0))?;
    support.begin_region("support-object-b", 11);
    support.push_interface_path(support_path(10.0), true)?;
    support.push_raft_path(support_path(20.0))?;
    // exhaustive: test-only native layer response; every stage slot named explicitly by this parity fixture
    Ok(NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: Some(support),
        slice_postprocess: None,
        path_optimization: None,
    })
}

fn module_id() -> slicer_ir::ModuleId {
    "com.test.sdk-layer-infill-parity".to_string()
}

fn wasm_live<'a>(
    module: &'a slicer_runtime::CompiledModule,
) -> (CompiledModuleLive<'a>, Arc<slicer_runtime::WasmComponent>) {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::Infill",
        String::new(),
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    let component = wasm_cache::compiled_guest("sdk-layer-infill-guest");
    let live = CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(Arc::clone(&component)),
        module.claims(),
        Arc::clone(module.config_view()),
    );
    (live, component)
}

fn commit_shape(commit: &LayerStageCommit) -> Vec<(usize, Vec<(usize, slicer_ir::ExtrusionRole)>)> {
    let LayerStageCommit::Infill(infill) = commit else {
        panic!("expected infill commit")
    };
    infill
        .regions
        .iter()
        .map(|region| {
            (
                region.sparse_infill.len(),
                region
                    .sparse_infill
                    .iter()
                    .map(|path| (path.points.len(), path.role.clone()))
                    .collect(),
            )
        })
        .collect()
}

fn run_pair() -> (LayerStageCommit, LayerStageCommit) {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let wasm_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let native_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let (wasm_live, _component) = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SdkLayerInfillModule::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_slice(non_empty_slice())
        .expect("set wasm slice");
    native_arena
        .set_slice(non_empty_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".to_string();
    let mut wasm_input = crate::common::layer_input(&bb, &wasm_arena);
    let mut native_input = crate::common::layer_input(&bb, &native_arena);
    wasm_input.paint_regions = Some(());
    native_input.paint_regions = Some(());
    let wasm = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &wasm_live, wasm_input)
        .expect("wasm dispatch")
        .expect("wasm commit");
    let native =
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &native_live, native_input)
            .expect("native dispatch")
            .expect("native commit");
    (wasm, native)
}

fn run_native_only() -> LayerStageCommit {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let native_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SdkLayerInfillModule::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut arena = LayerArena::new();
    arena
        .set_slice(non_empty_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".to_string();
    let mut input = crate::common::layer_input(&bb, &arena);
    input.paint_regions = Some(());
    LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &native_live, input)
        .expect("native dispatch")
        .expect("native commit")
}

#[test]
fn native_dispatch_matches_wasm_structurally() {
    let (wasm, native) = run_pair();
    assert_eq!(commit_shape(&wasm), commit_shape(&native));
}

#[test]
fn native_dispatch_without_component() {
    assert!(matches!(run_native_only(), LayerStageCommit::Infill(_)));
}

#[test]
fn native_support_dispatch_preserves_per_region_origins() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = CompiledModuleBuilder::new("com.test.native-support-origins".to_string())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let live = CompiledModuleLive::new(
        module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        module.claims(),
        Arc::clone(module.config_view()),
    )
    .with_native_entry(NativeStageEntry::Layer(native_support_entry));
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut arena = LayerArena::new();
    arena.set_slice(support_slice()).expect("set support slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let input = crate::common::layer_input(&blackboard, &arena);
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &live,
        input,
    )
    .expect("native support dispatch")
    .expect("native support commit");

    let LayerStageCommit::Support(support) = commit else {
        panic!("expected support commit");
    };
    assert_eq!(support.entries.len(), 3);
    assert_eq!(support.entries[0].object_id, "support-object-a");
    assert_eq!(support.entries[0].region_id, 7);
    assert_eq!(support.entries[0].paths.len(), 1);
    assert_eq!(support.entries[1].object_id, "support-object-b");
    assert_eq!(support.entries[1].region_id, 11);
    assert_eq!(support.entries[1].paths.len(), 1);
    assert_eq!(support.entries[2].object_id, "support-object-b");
    assert_eq!(support.entries[2].region_id, 11);
    assert_eq!(support.entries[2].paths.len(), 1);
}

#[test]
fn native_prepass_commit_preserves_seam_candidate_reason() {
    let mut output = SeamPlanningOutput::new();
    output
        .push_seam_plan(SeamPlanEntry {
            global_layer_index: 3,
            object_id: "object".into(),
            region_id: "4".into(),
            chosen_position: Point3WithWidth::default(),
            scored_candidates: vec![ScoredSeamCandidate {
                reason: SeamReason {
                    tag: "sharp".into(),
                },
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    // exhaustive: test-only native prepass response; every stage slot named explicitly by this parity fixture
    let response = NativePrepassResponse {
        mesh_analysis: None,
        layer_plan: None,
        paint_segmentation: None,
        seam_planning: Some(output),
        support_geometry: None,
    };
    let slicer_core::PrepassStageOutput::SeamPlan(plan) =
        commit_native_prepass_response(&response, "PrePass::SeamPlanning").unwrap()
    else {
        panic!("expected seam plan");
    };
    assert_eq!(
        plan.entries[0].scored_candidates[0].reason,
        slicer_ir::SeamReason::Sharp
    );
}

#[test]
fn native_prepass_commit_rejects_invalid_region_id() {
    let mut output = SeamPlanningOutput::new();
    output
        .push_seam_plan(SeamPlanEntry {
            object_id: "object".into(),
            region_id: "not-a-number".into(),
            ..Default::default()
        })
        .unwrap();
    // exhaustive: test-only native prepass response; every stage slot named explicitly by this parity fixture
    let response = NativePrepassResponse {
        mesh_analysis: None,
        layer_plan: None,
        paint_segmentation: None,
        seam_planning: Some(output),
        support_geometry: None,
    };
    let error = commit_native_prepass_response(&response, "PrePass::SeamPlanning").unwrap_err();
    assert!(error.contains("invalid seam region id"));
}

fn empty_native_prepass_response() -> NativePrepassResponse {
    // exhaustive: empty native prepass response fixture; every stage slot explicitly None
    NativePrepassResponse {
        mesh_analysis: None,
        layer_plan: None,
        paint_segmentation: None,
        seam_planning: None,
        support_geometry: None,
    }
}

#[test]
fn native_prepass_commit_rejects_absent_layer_plan_output() {
    let error =
        commit_native_prepass_response(&empty_native_prepass_response(), "PrePass::LayerPlanning")
            .unwrap_err();
    assert!(error.contains("LayerPlanning"));
    assert!(error.contains("layer-plan output"));
}

#[test]
fn native_prepass_commit_rejects_absent_seam_plan_output() {
    let error =
        commit_native_prepass_response(&empty_native_prepass_response(), "PrePass::SeamPlanning")
            .unwrap_err();
    assert!(error.contains("SeamPlanning"));
    assert!(error.contains("seam-planning output"));
}

#[test]
fn native_prepass_commit_rejects_absent_support_plan_output() {
    let error = commit_native_prepass_response(
        &empty_native_prepass_response(),
        "PrePass::SupportGeometry",
    )
    .unwrap_err();
    assert!(error.contains("SupportGeometry"));
    assert!(error.contains("support-geometry output"));
}

#[test]
fn native_prepass_commit_rejects_absent_mesh_analysis_output() {
    let error =
        commit_native_prepass_response(&empty_native_prepass_response(), "PrePass::MeshAnalysis")
            .unwrap_err();
    assert!(error.contains("MeshAnalysis"));
    assert!(error.contains("mesh-analysis output"));
}

#[test]
fn native_paint_segmentation_commit_mirrors_wasm_leg() {
    let response = empty_native_prepass_response();
    let output = commit_native_prepass_response(&response, "PrePass::PaintSegmentation")
        .expect("paint segmentation is intentionally outputless");
    assert!(matches!(output, slicer_core::PrepassStageOutput::None));
}

#[test]
fn native_prepass_commit_preserves_layer_support_and_mesh_outputs() {
    let mut layers = LayerPlanOutput::new();
    layers
        .push_layer(LayerProposal {
            z: 0.2,
            active_regions: vec![RegionLayerProposal {
                object_id: "object".into(),
                region_id: "2".into(),
                effective_layer_height: 0.2,
                ..Default::default()
            }],
        })
        .unwrap();
    let mut mesh = MeshAnalysisOutput::new();
    mesh.push_facet_annotation(
        "object".into(),
        FacetAnnotation {
            facet_index: 7,
            classification: FacetClass::Overhang,
            ..Default::default()
        },
    )
    .unwrap();
    mesh.push_surface_group(
        "object".into(),
        SurfaceGroupProposal {
            facet_indices: vec![7, 8],
            z_min: 0.1,
            z_max: 0.3,
            shell_count: 2,
        },
    )
    .unwrap();
    let mut support = SupportGeometryOutput::new();
    support
        // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
        .push_support_plan_entry(SupportPlanEntry {
            global_layer_index: 0,
            object_id: "object".into(),
            region_id: "2".into(),
            family_id: "tree".into(),
            demand_ids: vec!["demand".into()],
            body_ids: vec!["body".into()],
            anchor_layer_index: 0,
            anchor_z: 200,
            roles: vec![],
            skeleton: Some(slicer_ir::SupportPlanSkeleton { points: vec![] }),
            capabilities: vec![],
            provenance: vec![],
            decline_reason: None,
        })
        .unwrap();
    // exhaustive: test-only native prepass response; every stage slot named explicitly by this parity fixture
    let response = NativePrepassResponse {
        mesh_analysis: Some(mesh),
        layer_plan: Some(layers),
        paint_segmentation: None,
        seam_planning: None,
        support_geometry: Some(support),
    };
    let slicer_core::PrepassStageOutput::LayerPlan(plan) =
        commit_native_prepass_response(&response, "PrePass::LayerPlanning").unwrap()
    else {
        panic!("expected layer plan");
    };
    assert_eq!(plan.global_layers.len(), 1);
    let participation = &plan.object_participation["object"];
    assert_eq!(participation.len(), 1);
    assert_eq!(participation[0].local_layer_index, 0);
    assert_eq!(participation[0].global_layer_index, 0);
    assert_eq!(participation[0].effective_layer_height, 0.2);
    let slicer_core::PrepassStageOutput::MeshAnalysisAuxiliary(analysis) =
        commit_native_prepass_response(&response, "PrePass::MeshAnalysis").unwrap()
    else {
        panic!("expected mesh analysis");
    };
    assert_eq!(analysis.facet_annotations.len(), 1);
    let (annotation_object, annotation) = &analysis.facet_annotations[0];
    assert_eq!(annotation_object, "object");
    assert_eq!(annotation.facet_index, 7);
    assert_eq!(annotation.slope_angle_deg, 0.0);
    assert_eq!(
        annotation.classification,
        slicer_core::FacetClassRecord::Overhang
    );
    let (group_object, group) = &analysis.surface_groups[0];
    assert_eq!(group_object, "object");
    assert_eq!(group.facet_indices, vec![7, 8]);
    assert_eq!(group.z_min, 0.1);
    assert_eq!(group.z_max, 0.3);
    assert_eq!(group.shell_count, 2);
    let slicer_core::PrepassStageOutput::SupportPlan(support) =
        commit_native_prepass_response(&response, "PrePass::SupportGeometry").unwrap()
    else {
        panic!("expected support plan");
    };
    assert_eq!(support.entries.len(), 1);
    let entry = &support.entries[0];
    assert_eq!(entry.global_layer_index, 0);
    assert_eq!(entry.object_id, "object");
    assert_eq!(entry.region_id, 2);
    assert_eq!(entry.roles.len(), 0);
    assert!(entry.skeleton.is_some());
}
