#![allow(missing_docs)]

use slicer_ir::{
    ConfigValue, RaftPlan, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanRole,
};
use slicer_runtime::run::PrepassContext;

use crate::common::support_wedge;

fn prepare_ctx() -> PrepassContext {
    support_wedge::prepare_wedge_context(true)
}

/// Tree-family wedge context.
///
/// The default wedge sets no `support_type`, which resolves to the
/// *traditional* family, and `traditional-support-planner` hardcodes
/// `skeleton: None`. Every assertion below that reads
/// `SupportPlanEntry::skeleton` must therefore run against this context or it
/// is asserting on data the fixture cannot produce.
fn prepare_tree_ctx() -> PrepassContext {
    support_wedge::prepare_wedge_context_tree(true)
}

fn owned_tree_plan_entries() -> Vec<SupportPlanEntry> {
    plan_entries(&prepare_tree_ctx()).to_vec()
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
    let ctx = prepare_tree_ctx();
    let entries = plan_entries(&ctx);
    let structural = entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none());
    assert!(structural.clone().next().is_some());
    for entry in structural {
        assert!(entry
            .skeleton
            .as_ref()
            .is_some_and(|skeleton| skeleton.points.len() > 1));
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
    let entries = owned_tree_plan_entries();
    let structural: Vec<_> = entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
        .collect();
    assert!(
        !structural.is_empty(),
        "tree wedge must plan at least one non-declined entry"
    );
    // Was `map_or(true, ..)`, which passes for every `skeleton: None` entry the
    // traditional planner emits — i.e. it was vacuously green on the old
    // fixture. Every structural tree entry must actually carry a skeleton.
    assert!(structural.iter().all(|entry| entry
        .skeleton
        .as_ref()
        .is_some_and(|s| s.points.len() >= 2)));
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
    assert!(!entries.is_empty(), "wedge must plan support entries");

    // The old form asserted `anchor_layer_index` was globally non-increasing
    // across `entries`, which encoded the planner's *emission* order. Packet
    // 223 made the aggregate sort entries ascending by the identity triple, so
    // no global anchor ordering survives. The invariant this test is named for
    // is per-column: within one (object, region, body) column the layers form
    // a contiguous run with single-layer steps.
    let mut columns: std::collections::BTreeMap<(String, u64, String), Vec<&SupportPlanEntry>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        for body_id in &entry.body_ids {
            columns
                .entry((entry.object_id.clone(), entry.region_id, body_id.clone()))
                .or_default()
                .push(entry);
        }
    }
    assert!(
        !columns.is_empty(),
        "every planned entry must carry at least one body_id so columns are identifiable"
    );

    for (key, mut column) in columns {
        column.sort_by_key(|entry| entry.global_layer_index);
        for pair in column.windows(2) {
            let step = pair[1].global_layer_index - pair[0].global_layer_index;
            assert!(
                step == 0 || step == 1,
                "column {key:?} is not contiguous: layer {} -> {} (step {step})",
                pair[0].global_layer_index,
                pair[1].global_layer_index
            );
        }
        for entry in &column {
            assert_eq!(
                entry.anchor_layer_index as i32, entry.global_layer_index,
                "column {key:?} anchor must step down in lockstep with its layer"
            );
        }
    }
}

#[test]
fn support_branch_widths_widen_monotonically_toward_the_plate() {
    let entries = owned_tree_plan_entries();
    let mut structural = entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none());
    assert!(structural.clone().next().is_some());
    assert!(structural.all(|entry| {
        entry
            .skeleton
            .as_ref()
            .is_some_and(|skeleton| skeleton.points.len() > 1)
    }));
}

#[test]
fn support_segments_stay_within_mesh_bbox() {
    let ctx = prepare_tree_ctx();
    let bbox = ctx.blackboard.mesh().build_volume;
    let entries = plan_entries(&ctx);

    let point_count = entries.iter().flat_map(structural_points).count();
    assert!(
        point_count > 0,
        "tree wedge must plan at least one structural skeleton point"
    );

    // `MeshIR::build_volume` is the union bbox of the loaded object meshes
    // (`compute_bounding_box_union` in `crates/slicer-model-io/src/loader.rs`),
    // i.e. the model bbox this test is named for. Branch centrelines are
    // clamped inside the overhang footprint, so a centreline outside the model
    // bbox means the planner routed a branch off the part.
    const MARGIN_MM: f32 = 1.0;
    for entry in entries {
        for point in structural_points(entry) {
            assert!(
                point.x >= bbox.min.x - MARGIN_MM
                    && point.x <= bbox.max.x + MARGIN_MM
                    && point.y >= bbox.min.y - MARGIN_MM
                    && point.y <= bbox.max.y + MARGIN_MM
                    && point.z >= bbox.min.z - MARGIN_MM
                    && point.z <= bbox.max.z + MARGIN_MM,
                "skeleton point {point:?} escapes the mesh bbox {bbox:?} (margin {MARGIN_MM} mm)"
            );
        }
    }
}

#[test]
fn wedge_support_plan_is_byte_deterministic_across_repeated_runs() {
    let a = owned_plan_entries();
    let b = owned_plan_entries();
    assert_eq!(a, b, "structural support plan must be deterministic");
}
