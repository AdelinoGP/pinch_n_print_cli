//! TDD coverage for prepass output-builder rejection branches.
//!
//! Tests call push/mark methods DIRECTLY on `HostExecutionContext` (no WASM
//! needed). Each method returns `wasmtime::Result<Result<(), String>>`; the
//! outer `Ok` is the wasmtime result and the inner `Err` is the host-level
//! rejection we are exercising here.
//!
//! Modelled after `z_envelope_contract_tdd.rs`.

#![allow(missing_docs)]

use slicer_wasm_host::host::prepass_layer_planning::LayerProposal;
use slicer_wasm_host::host::prepass_layer_planning::slicer::prepass_layer_planning::layer_planning_types::RegionLayerProposal;
use slicer_wasm_host::host::prepass_mesh_analysis::{
    FacetAnnotation, FacetClass, SurfaceGroupProposal,
};
use slicer_wasm_host::host::prepass_seam_planning::SeamPlanEntry;
use slicer_wasm_host::host::prepass_seam_planning::slicer::prepass_seam_planning::seam_planning_types::{ScoredSeamCandidate, SeamReason};
use slicer_wasm_host::host::prepass_support_geometry::SupportPlanEntry;
use slicer_wasm_host::host::{
    prepass_layer_planning, prepass_mesh_analysis, prepass_seam_planning, prepass_support_geometry,
    HostExecutionContextBuilder,
};

// SeamPlanEntry positions use the seam-specific public WIT record rather than
// the support-facing Point3WithWidth record.
use slicer_wasm_host::host::layer_perimeters::slicer::types::geometry::SeamPoint3WithWidth;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_point3_with_width(x: f32, y: f32, z: f32) -> SeamPoint3WithWidth {
    SeamPoint3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn valid_seam_entry() -> SeamPlanEntry {
    // exhaustive: SeamPlanEntry has no Default; valid seam fixture supplies the complete entry
    SeamPlanEntry {
        variant_chain: Vec::new(),
        global_layer_index: 0,
        object_id: "obj-a".to_string(),
        region_id: "region-1".to_string(),
        chosen_position: make_point3_with_width(1.0, 2.0, 0.2),
        chosen_wall_index: 0,
        scored_candidates: vec![ScoredSeamCandidate {
            position: make_point3_with_width(1.0, 2.0, 0.2),
            score: 1.0,
            reason: SeamReason {
                tag: "aligned".to_string(),
            },
        }],
    }
}

fn valid_support_entry() -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: 0,
        object_id: "obj-a".to_string(),
        region_id: "region-1".to_string(),
        family_id: "tree".to_string(),
        demand_ids: vec!["demand-1".to_string()],
        body_ids: vec!["body-1".to_string()],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: vec![],
        skeleton: None,
        capabilities: vec![],
        provenance: vec![],
        decline_reason: None,
    }
}

fn valid_facet_annotation() -> FacetAnnotation {
    FacetAnnotation {
        facet_index: 0,
        slope_angle_deg: 45.0,
        classification: FacetClass::Normal,
    }
}

fn valid_surface_group() -> SurfaceGroupProposal {
    SurfaceGroupProposal {
        facet_indices: vec![0],
        z_min: 0.0,
        z_max: 1.0,
        shell_count: 1,
    }
}

fn valid_layer_proposal() -> LayerProposal {
    LayerProposal {
        z: 0.2,
        active_regions: vec![RegionLayerProposal {
            effective_layer_height: 0.2,
            ..region_layer_proposal_base()
        }],
    }
}

fn region_layer_proposal_base() -> RegionLayerProposal {
    // exhaustive: RegionLayerProposal has no Default; shared base for repeated validation fixtures
    RegionLayerProposal {
        object_id: "obj-a".to_string(),
        region_id: "region-1".to_string(),
        effective_layer_height: 0.2,
        is_catchup: false,
        catchup_z_bottom: 0.0,
    }
}

// ─── HostMeshAnalysisOutput ────────────────────────────────────────────────

#[test]
fn mesh_analysis_push_facet_annotation_positive_control() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_facet_annotation(
        &mut ctx,
        handle,
        "obj-a".to_string(),
        valid_facet_annotation(),
    );
    let inner = result.unwrap();
    assert_eq!(inner, Ok(()), "valid facet annotation should be accepted");
    assert_eq!(
        ctx.mesh_analysis_annotations().len(),
        1,
        "annotation should be stored"
    );
}

#[test]
fn mesh_analysis_push_facet_annotation_empty_object_id() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_facet_annotation(
        &mut ctx,
        handle,
        "".to_string(),
        valid_facet_annotation(),
    );
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("mesh-analysis-output: object-id must be non-empty".to_string()),
        "empty object-id should be rejected"
    );
}

#[test]
fn mesh_analysis_push_facet_annotation_non_finite_slope() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let bad = FacetAnnotation {
        facet_index: 7,
        slope_angle_deg: f32::NAN,
        classification: FacetClass::Overhang,
    };
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_facet_annotation(
        &mut ctx,
        handle,
        "obj-a".to_string(),
        bad,
    );
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.starts_with(
            "mesh-analysis-output: object 'obj-a' facet 7 has non-finite slope_angle_deg="
        ),
        "error should start with stable prefix, got: {err}"
    );
}

#[test]
fn mesh_analysis_push_surface_group_positive_control() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_surface_group(
        &mut ctx,
        handle,
        "obj-a".to_string(),
        valid_surface_group(),
    );
    let inner = result.unwrap();
    assert_eq!(inner, Ok(()), "valid surface group should be accepted");
    assert_eq!(
        ctx.mesh_analysis_surface_groups().len(),
        1,
        "group should be stored"
    );
}

#[test]
fn mesh_analysis_push_surface_group_empty_object_id() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_surface_group(
        &mut ctx,
        handle,
        "".to_string(),
        valid_surface_group(),
    );
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("mesh-analysis-output: object-id must be non-empty".to_string()),
        "empty object-id should be rejected"
    );
}

#[test]
fn mesh_analysis_push_surface_group_non_finite_z_bounds() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let bad = SurfaceGroupProposal {
        facet_indices: vec![],
        z_min: f32::NAN,
        z_max: 1.0,
        shell_count: 1,
    };
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_surface_group(
        &mut ctx,
        handle,
        "obj-a".to_string(),
        bad,
    );
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.contains("non-finite z bounds"),
        "error should mention non-finite z bounds, got: {err}"
    );
    assert!(
        err.starts_with(
            "mesh-analysis-output: object 'obj-a' surface group has non-finite z bounds"
        ),
        "error should start with stable prefix, got: {err}"
    );
}

#[test]
fn mesh_analysis_push_surface_group_z_max_less_than_z_min() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_mesh_analysis_output().unwrap();
    let bad = SurfaceGroupProposal {
        facet_indices: vec![],
        z_min: 2.0,
        z_max: 1.0,
        shell_count: 1,
    };
    let result = prepass_mesh_analysis::HostMeshAnalysisOutput::push_surface_group(
        &mut ctx,
        handle,
        "obj-b".to_string(),
        bad,
    );
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.starts_with("mesh-analysis-output: object 'obj-b' surface group has z_max="),
        "error should start with stable prefix, got: {err}"
    );
    assert!(
        err.contains("< z_min="),
        "error should contain '< z_min=', got: {err}"
    );
}

// ─── HostLayerPlanOutput ───────────────────────────────────────────────────

#[test]
fn layer_plan_push_layer_positive_control() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_layer_plan_output().unwrap();
    let result = prepass_layer_planning::HostLayerPlanOutput::push_layer(
        &mut ctx,
        handle,
        valid_layer_proposal(),
    );
    let inner = result.unwrap();
    assert_eq!(inner, Ok(()), "valid layer proposal should be accepted");
    assert_eq!(
        ctx.layer_plan_proposals().len(),
        1,
        "proposal should be stored"
    );
}

#[test]
fn layer_plan_push_layer_z_is_nan() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_layer_plan_output().unwrap();
    let bad = LayerProposal {
        z: f32::NAN,
        active_regions: vec![],
    };
    let result = prepass_layer_planning::HostLayerPlanOutput::push_layer(&mut ctx, handle, bad);
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.starts_with("layer-plan-output: invalid z="),
        "error should start with stable prefix, got: {err}"
    );
    assert!(
        err.contains("(must be finite and non-negative)"),
        "error should mention constraint, got: {err}"
    );
}

#[test]
fn layer_plan_push_layer_z_is_negative() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_layer_plan_output().unwrap();
    let bad = LayerProposal {
        z: -0.1,
        active_regions: vec![],
    };
    let result = prepass_layer_planning::HostLayerPlanOutput::push_layer(&mut ctx, handle, bad);
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.starts_with("layer-plan-output: invalid z="),
        "error should start with stable prefix, got: {err}"
    );
    assert!(
        err.contains("(must be finite and non-negative)"),
        "error should mention constraint, got: {err}"
    );
}

#[test]
fn layer_plan_push_layer_region_effective_layer_height_nan() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_layer_plan_output().unwrap();
    let bad = LayerProposal {
        z: 0.2,
        active_regions: vec![RegionLayerProposal {
            effective_layer_height: f32::NAN,
            ..region_layer_proposal_base()
        }],
    };
    let result = prepass_layer_planning::HostLayerPlanOutput::push_layer(&mut ctx, handle, bad);
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.starts_with("layer-plan-output: region 'obj-a'/'region-1'"),
        "error should name the region, got: {err}"
    );
    assert!(
        err.contains("(must be finite and positive)"),
        "error should mention constraint, got: {err}"
    );
}

#[test]
fn layer_plan_push_layer_region_effective_layer_height_zero() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_layer_plan_output().unwrap();
    let bad = LayerProposal {
        z: 0.2,
        active_regions: vec![RegionLayerProposal {
            effective_layer_height: 0.0,
            ..region_layer_proposal_base()
        }],
    };
    let result = prepass_layer_planning::HostLayerPlanOutput::push_layer(&mut ctx, handle, bad);
    let inner = result.unwrap();
    let err = inner.unwrap_err();
    assert!(
        err.contains("has invalid effective_layer_height="),
        "error should mention field name, got: {err}"
    );
    assert!(
        err.contains("(must be finite and positive)"),
        "error should mention constraint, got: {err}"
    );
}

// ─── HostSeamPlanningOutput ────────────────────────────────────────────────

#[test]
fn seam_planning_push_seam_plan_positive_control() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_seam_planning_output().unwrap();
    let result = prepass_seam_planning::HostSeamPlanningOutput::push_seam_plan(
        &mut ctx,
        handle,
        valid_seam_entry(),
    );
    let inner = result.unwrap();
    assert_eq!(inner, Ok(()), "valid seam entry should be accepted");
    assert_eq!(ctx.seam_plan_entries().len(), 1, "entry should be stored");
}

#[test]
fn seam_planning_push_seam_plan_empty_object_id() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_seam_planning_output().unwrap();
    let mut bad = valid_seam_entry();
    bad.object_id = "".to_string();
    let result =
        prepass_seam_planning::HostSeamPlanningOutput::push_seam_plan(&mut ctx, handle, bad);
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("seam-planning-output: object-id must be non-empty".to_string()),
    );
}

#[test]
fn seam_planning_push_seam_plan_empty_region_id() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_seam_planning_output().unwrap();
    let mut bad = valid_seam_entry();
    bad.region_id = "".to_string();
    let result =
        prepass_seam_planning::HostSeamPlanningOutput::push_seam_plan(&mut ctx, handle, bad);
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("seam-planning-output: region-id must be non-empty".to_string()),
    );
}

#[test]
fn seam_planning_push_seam_plan_non_finite_chosen_position_x() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_seam_planning_output().unwrap();
    let mut bad = valid_seam_entry();
    bad.chosen_position = make_point3_with_width(f32::NAN, 2.0, 0.2);
    let result =
        prepass_seam_planning::HostSeamPlanningOutput::push_seam_plan(&mut ctx, handle, bad);
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("seam-planning-output: chosen_position must have finite coordinates".to_string()),
    );
}

#[test]
fn seam_planning_push_seam_plan_non_finite_chosen_position_y() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_seam_planning_output().unwrap();
    let mut bad = valid_seam_entry();
    bad.chosen_position = make_point3_with_width(1.0, f32::NAN, 0.2);
    let result =
        prepass_seam_planning::HostSeamPlanningOutput::push_seam_plan(&mut ctx, handle, bad);
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("seam-planning-output: chosen_position must have finite coordinates".to_string()),
    );
}

#[test]
fn seam_planning_push_seam_plan_non_finite_chosen_position_z() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_seam_planning_output().unwrap();
    let mut bad = valid_seam_entry();
    bad.chosen_position = make_point3_with_width(1.0, 2.0, f32::NAN);
    let result =
        prepass_seam_planning::HostSeamPlanningOutput::push_seam_plan(&mut ctx, handle, bad);
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("seam-planning-output: chosen_position must have finite coordinates".to_string()),
    );
}

// ─── HostSupportGeometryOutput ─────────────────────────────────────────────

#[test]
fn support_geometry_push_support_plan_entry_positive_control() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_support_geometry_output().unwrap();
    let result = prepass_support_geometry::HostSupportGeometryOutput::push_support_plan_entry(
        &mut ctx,
        handle,
        valid_support_entry(),
    );
    let inner = result.unwrap();
    assert_eq!(inner, Ok(()), "valid support entry should be accepted");
    assert_eq!(
        ctx.support_plan_entries().len(),
        1,
        "entry should be stored"
    );
}

#[test]
fn support_geometry_push_support_plan_entry_empty_object_id() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_support_geometry_output().unwrap();
    let mut bad = valid_support_entry();
    bad.object_id = "".to_string();
    let result = prepass_support_geometry::HostSupportGeometryOutput::push_support_plan_entry(
        &mut ctx, handle, bad,
    );
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("support-geometry-output: object-id must be non-empty".to_string()),
    );
}

#[test]
fn support_geometry_push_support_plan_entry_empty_region_id() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx.push_support_geometry_output().unwrap();
    let mut bad = valid_support_entry();
    bad.region_id = "".to_string();
    let result = prepass_support_geometry::HostSupportGeometryOutput::push_support_plan_entry(
        &mut ctx, handle, bad,
    );
    let inner = result.unwrap();
    assert_eq!(
        inner,
        Err("support-geometry-output: region-id must be non-empty".to_string()),
    );
}
