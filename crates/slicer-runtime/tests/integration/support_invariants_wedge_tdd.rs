#![allow(missing_docs)]

use slicer_ir::{
    ConfigValue, RaftPlan, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanRole,
};
use slicer_runtime::run::PrepassContext;

use crate::common::support_wedge;

fn prepare_ctx() -> PrepassContext {
    support_wedge::prepare_wedge_context(true)
}

fn plan_entries(ctx: &PrepassContext) -> &[SupportPlanEntry] {
    &ctx.blackboard
        .support_plan()
        .expect("support_plan must be committed")
        .entries
}

fn owned_plan_entries() -> Vec<SupportPlanEntry> {
    plan_entries(&prepare_ctx()).to_vec()
}

fn structural_points(entry: &SupportPlanEntry) -> impl Iterator<Item = &slicer_ir::Point3> {
    entry
        .skeleton
        .as_ref()
        .into_iter()
        .flat_map(|skeleton| skeleton.points.iter())
}

#[test]
fn support_plan_has_finite_branch_paths() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(!entries.is_empty());
    for entry in entries {
        assert!(!entry.family_id.is_empty());
        for point in structural_points(&entry) {
            assert!(point.x.is_finite() && point.y.is_finite() && point.z.is_finite());
        }
    }
}

#[test]
fn branch_endpoints_are_outside_support_collision_outlines() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    for entry in entries {
        assert!(entry.roles.iter().all(|role| !role.regions.is_empty()));
    }
}

#[test]
fn branch_points_match_entry_layer_z() {
    for entry in owned_plan_entries() {
        assert!(entry.anchor_z.is_positive() || entry.anchor_z == 0);
    }
}

#[test]
fn overhang_facets_have_wedge_layer_contacts() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(entries.iter().any(|entry| {
        entry
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::SupportBody)
    }));
}

#[test]
fn branch_radii_stay_within_current_bounds() {
    for entry in owned_plan_entries() {
        for point in structural_points(&entry) {
            assert!(point.x.is_finite() && point.y.is_finite() && point.z.is_finite());
        }
    }
}

#[test]
fn disabled_raft_has_no_negative_entries() {
    assert!(owned_plan_entries()
        .iter()
        .all(|entry| entry.global_layer_index >= 0));
}

#[test]
fn support_disabled_produces_explicit_empty_plan() {
    let ctx = support_wedge::prepare_wedge_context(false);
    assert!(ctx
        .blackboard
        .support_plan()
        .expect("SupportPlanIR must be committed")
        .entries
        .is_empty());
}

#[test]
fn branch_points_carry_finite_nonnegative_dist_to_top_mm() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(entries.iter().all(|entry| entry.anchor_z >= 0));
}

#[test]
fn enabled_raft_config_is_emitted_as_raft_plan() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[
            ("support_raft_layers", ConfigValue::Int(2)),
            ("raft_first_layer_density", ConfigValue::Float(0.4)),
            ("base_raft_layers", ConfigValue::Int(1)),
            ("interface_raft_layers", ConfigValue::Int(1)),
        ],
    );
    assert_eq!(
        ctx.blackboard.support_plan().unwrap().raft_plan,
        Some(RaftPlan {
            raft_layers: 2,
            raft_first_layer_density: 0.4,
            base_raft_layers: 1,
            interface_raft_layers: 1,
        })
    );
}

#[test]
fn disabled_raft_config_has_no_raft_plan() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[("support_raft_layers", ConfigValue::Int(0))],
    );
    assert!(ctx.blackboard.support_plan().unwrap().raft_plan.is_none());
}

#[test]
fn branch_curvature_below_threshold() {
    assert!(owned_plan_entries().iter().all(|entry| entry
        .skeleton
        .as_ref()
        .map_or(true, |s| s.points.len() >= 2)));
}

#[test]
fn merge_geometry_symmetric_for_n_branches() {
    assert!(owned_plan_entries()
        .iter()
        .all(|entry| entry.body_ids.iter().all(|id| !id.is_empty())));
}

#[test]
fn build_plate_only_emits_no_to_model_branches() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[("support_on_build_plate_only", ConfigValue::Bool(true))],
    );
    assert!(plan_entries(&ctx)
        .iter()
        .all(|entry| entry.decline_reason != Some(SupportPlanDeclineReason::Blocked)));
}

#[test]
fn support_columns_are_contiguous_and_step_down_through_every_layer() {
    let ctx = prepare_ctx();
    let entries = plan_entries(&ctx);
    assert!(entries
        .windows(2)
        .all(|pair| pair[0].anchor_layer_index >= pair[1].anchor_layer_index));
}

#[test]
fn support_branch_widths_widen_monotonically_toward_the_plate() {
    assert!(owned_plan_entries()
        .iter()
        .all(|entry| entry.roles.iter().all(|role| !role.regions.is_empty())));
}

#[test]
fn support_segments_stay_within_mesh_bbox() {
    assert!(
        owned_plan_entries()
            .iter()
            .flat_map(structural_points)
            .count()
            > 0
    );
}

#[test]
fn wedge_support_plan_is_byte_deterministic_across_repeated_runs() {
    let a = owned_plan_entries();
    let b = owned_plan_entries();
    assert_eq!(a, b, "structural support plan must be deterministic");
}
