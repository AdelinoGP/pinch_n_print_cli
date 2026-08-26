//! Tests for `to_buildplate` classification and branch pruning.
//!
//! **Rewritten by packet 224 step 5 (F-14).** These were authored for packet
//! 123, whose model was:
//!
//! 1. `to_buildplate = !point_in_any_expoly(collision, x, y)` computed ONCE at
//!    contact creation, then copied unchanged down every propagation step; and
//! 2. a contact whose `to_buildplate` was `false` rejected outright at creation
//!    under `support_on_build_plate_only`, with a code-1002 `node-clamped-out`
//!    diagnostic as the propagation-time drop.
//!
//! Neither is canonical. Canonical contact seeding and branch-A merges classify
//! against xy-distance-inflated `get_collision(0, layer)`, while `drop_nodes`
//! recomputes each move-pass descendant as
//! `!is_inside_ex(m_layer_outlines[obj_layer_nr_next], next_layer_vertex)` —
//! against the RAW outlines of the layer below, not an inflated collision
//! volume, and against the node's *moved* position. Pruning is then done by
//! `unsupported_branch_leaves`, which walks the whole column up from the
//! unfooted leaf and erases it from every layer. The code-1002 diagnostic
//! reported the fractional move cap plus post-hoc avoidance clamp that F-13
//! deletes, so it no longer exists.
//!
//! Each test below therefore observes the pruning directly — geometry present
//! or absent on the layers a column would occupy — rather than a diagnostic
//! about it. That is a strictly stronger observation: the old assertions could
//! pass while support was still planned through the model.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    LayerPlanView, LayerPlanViewEntry, MeshObjectView, RegionSegmentationView,
    RegionSegmentationViewEntry, SupportGeometryView, SupportGeometryViewEntry,
};
use slicer_sdk::traits::PrepassModule;

use tree_support_planner::{
    branch_a_to_buildplate, contact_seed_to_buildplate, move_pass_to_buildplate, SupportPlanner,
};

fn box_outline(min: f32, max: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min, min),
                Point2::from_mm(max, min),
                Point2::from_mm(max, max),
                Point2::from_mm(min, max),
            ],
        },
        holes: vec![],
    }
}

#[test]
fn contact_and_branch_a_reject_xy_inflated_collision_fringe() {
    let raw = vec![box_outline(0.0, 1.0)];
    let inflated_collision = vec![box_outline(-1.0, 2.0)];
    let fringe = (1.5, 0.5);

    assert!(move_pass_to_buildplate(&raw, fringe));
    assert!(!contact_seed_to_buildplate(&inflated_collision, fringe));
    assert!(!branch_a_to_buildplate(&inflated_collision, fringe));
}

#[test]
fn move_pass_f14_accepts_xy_inflated_collision_fringe_from_raw_outlines() {
    let raw = vec![box_outline(0.0, 1.0)];
    let inflated_collision = vec![box_outline(-1.0, 2.0)];
    let fringe = (1.5, 0.5);

    assert!(!contact_seed_to_buildplate(&inflated_collision, fringe));
    assert!(
        move_pass_to_buildplate(&raw, fringe),
        "LOCKED: move-pass recompute tests RAW outlines forever"
    );
}

// ── AC-2: contact XY outside the per-layer footprint → to_buildplate=true ────

/// AC-2: a column that never enters the object footprint on any layer reaches
/// the build plate and survives `support_on_build_plate_only = true`.
///
/// The footprint sits in the far corner `[-10,-10]..[-5,-5]` at layer 8 and
/// nowhere else, so every descendant's `to_buildplate` recompute
/// (`!is_inside_ex(outlines[layer below], moved position)`) returns true and
/// no leaf is ever filed as unsupported. The plan must be non-empty.
///
/// Under the packet-123 model this test proved the *contact-creation*
/// classification; under canonical there is no such classification to prove
/// (contacts use collision(0)), so what it now pins is the negative case of the
/// pruning pass: a genuinely plate-bound column is not pruned.
#[test]
fn contact_xy_outside_footprint_sets_to_buildplate_true() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_on_build_plate_only", ConfigValue::Bool(true)),
        ("tree_support_branch_diameter", ConfigValue::Float(5.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    // Single-triangle contact at the same (2.67, 1.33) centroid as the
    // working `lone_fresh_contact_emits_tip_on_origin_layer` lib test, to
    // isolate the failure to the SupportGeometryView shape.
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.8],
        [4.0, 0.0, 1.8],
        [4.0, 4.0, 1.8],
    ];
    let triangles = vec![[1, 3, 2]];
    let obj = MeshObjectView {
        object_id: "ac2".to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    };
    let lp = make_layer_plan(10, 0.0, 0.2);
    let rs = make_region_segmentation("ac2", 10);

    // Layer 8 is the contact's layer (z = 0.2 * 9 = 1.8). At layer 8 the
    // footprint is a SMALL box in the far corner that excludes the contact
    // centroid (2.67, 1.33). All other layers have no footprint. The
    // contact at (2.67, 1.33) is therefore OUTSIDE the footprint at layer 8
    // and is admitted under support_on_build_plate_only=true.
    let small_footprint = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, -10.0),
                Point2::from_mm(-5.0, -10.0),
                Point2::from_mm(-5.0, -5.0),
                Point2::from_mm(-10.0, -5.0),
            ],
        },
        holes: vec![],
    };
    let sg = SupportGeometryView {
        entries: vec![SupportGeometryViewEntry {
            global_support_layer_index: 8,
            object_id: "ac2".to_string(),
            region_id: "0".to_string(),
            outlines: vec![small_footprint.clone()],
        }],
    };

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    let entries = output.entries();
    assert!(
        !entries.is_empty(),
        "AC-2: contact at the plate centroid is outside the [-10,-10]..[-5,-5] \
         footprint at layer 8 and must be admitted under \
         support_on_build_plate_only=true. Empty plan means to_buildplate \
         was incorrectly false. entries={}, diagnostics={:?}",
        entries.len(),
        output.diagnostics(),
    );
}

// ── AC-3: a column that cannot reach the plate is pruned from EVERY layer ──

/// AC-3: under `support_on_build_plate_only = true`, a column whose
/// descendants land inside the object footprint is filed as an unsupported
/// branch leaf and erased from every layer it occupied — not merely stopped
/// where it stood.
///
/// **Rewired by packet 224 step 5 (F-14).** This asserted a code-1002
/// `node-clamped-out` diagnostic, which pinned the fractional move cap and
/// post-hoc avoidance clamp that F-13 deletes. The canonical observable is the
/// prune itself: `unsupported_branch_leaves` walks up the parent chain from
/// the unfooted leaf marking `is_processed`, and every marked node is erased
/// across all layers. Asserting the absence of geometry is strictly stronger
/// than asserting a warning about it — the old form could pass while the
/// column was still planned.
///
/// The fixture also had to change. The packet-123 rule computed
/// `to_buildplate` at contact creation from the CONTACT layer's footprint, so
/// putting a big box on every layer *except* the contact layer was enough.
/// Canonical classifies contacts with collision(0) and recomputes per descendant
/// from the layer BELOW, so the big box now has to cover the layers the column
/// descends through — which it already does here (every layer but 8).
#[test]
fn unreachable_buildplate_node_pruned() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_on_build_plate_only", ConfigValue::Bool(true)),
        ("tree_support_branch_diameter", ConfigValue::Float(5.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    // A 2x2 grid of overhang triangles at z=1.8 → 4 contact centroids
    // separated into distinct dedup buckets. The MST has 3 edges; each node has at
    // least one neighbour, so the propagation's move path runs.
    let obj = multi_overhang_grid("ac3", 2, 2, 4.0);
    let lp = make_layer_plan(10, 0.0, 0.2);
    let rs = make_region_segmentation("ac3", 10);

    // At layer 8 (the contact's layer), the footprint is a small box in
    // the far corner that EXCLUDES the contact centroids. The 4 contact
    // centroids sit on a 0.4×0.4 grid in [0..0.8]×[0..0.8] (anchor at
    // origin, see fixture); they are well outside [-10,-10]..[-5,-5] and
    // so to_buildplate = true at the contact's layer.
    //
    // At all OTHER layers (0..7 and 9), a big footprint covers the
    // entire planner arena so the propagation's clamped move target is
    // always inside collision_polys and the drop fires.
    let small_footprint = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, -10.0),
                Point2::from_mm(-5.0, -10.0),
                Point2::from_mm(-5.0, -5.0),
                Point2::from_mm(-10.0, -5.0),
            ],
        },
        holes: vec![],
    };
    let big_box = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, -10.0),
                Point2::from_mm(14.0, -10.0),
                Point2::from_mm(14.0, 14.0),
                Point2::from_mm(-10.0, 14.0),
            ],
        },
        holes: vec![],
    };
    let mut entries: Vec<SupportGeometryViewEntry> = Vec::new();
    for layer in 0..10 {
        let outline = if layer == 8 {
            small_footprint.clone()
        } else {
            big_box.clone()
        };
        entries.push(SupportGeometryViewEntry {
            global_support_layer_index: layer,
            object_id: "ac3".to_string(),
            region_id: "0".to_string(),
            outlines: vec![outline],
        });
    }
    let sg = SupportGeometryView { entries };

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    // The whole column is erased: no layer may carry planned tree geometry.
    let planned: Vec<i32> = output
        .entries()
        .iter()
        .filter(|entry| {
            entry.decline_reason.is_none()
                && entry.roles.iter().any(|role| !role.regions.is_empty())
        })
        .map(|entry| entry.global_layer_index)
        .collect();
    assert!(
        planned.is_empty(),
        "AC-3: a column that cannot reach the build plate must be pruned from \
         EVERY layer by `unsupported_branch_leaves`; geometry survives on \
         layers {planned:?}. diagnostics={:?}",
        output.diagnostics(),
    );
}

// ── AC-4: support_on_build_plate_only=true rejects to_model contacts ─────────

/// AC-4: with `support_on_build_plate_only = true`, a column that would rest
/// on the model instead of the plate produces no plan at all.
///
/// **Fixture corrected by packet 224 step 5 (F-14).** The assertion (empty
/// plan) is unchanged; the fixture is not. The packet-123 rule read the
/// footprint at the CONTACT layer and rejected the contact at creation, so a
/// covering box on layer 7 alone was a binding constraint. Canonical classifies
/// the contact against collision(0) and recomputes the flag per descendant
/// against raw outlines on the layer BELOW, so a box on one layer proves nothing: the column
/// simply steps past it. The box now covers layers 0..=7 — the whole descent
/// path — which is what "rests on the model rather than the plate" actually
/// means.
#[test]
fn buildplate_only_rejects_to_model_contacts() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_on_build_plate_only", ConfigValue::Bool(true)),
        ("tree_support_branch_diameter", ConfigValue::Float(5.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    // Single-triangle plate so the contact has only one centroid at
    // (2.67, 1.33) on layer 8. No MST edge means the origin tip is the
    // only candidate for emission on the contact's layer.
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.8],
        [4.0, 0.0, 1.8],
        [4.0, 4.0, 1.8],
    ];
    let triangles = vec![[1, 3, 2]];
    let obj = MeshObjectView {
        object_id: "ac4".to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    };
    let lp = make_layer_plan(10, 0.0, 0.2);
    let rs = make_region_segmentation("ac4", 10);

    // The covering box spans the contact centroid (2.67, 1.33) on every layer
    // the column would descend through (0..=7), so the very first descendant
    // recomputes `to_buildplate = false` and the leaf is filed as unsupported.
    let covering_box = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    };
    let sg = SupportGeometryView {
        entries: (0..=7)
            .map(|layer| SupportGeometryViewEntry {
                global_support_layer_index: layer,
                object_id: "ac4".to_string(),
                region_id: "0".to_string(),
                outlines: vec![covering_box.clone()],
            })
            .collect(),
    };

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    let entries = output.entries();
    assert!(
        entries.is_empty(),
        "AC-4: with support_on_build_plate_only=true and the [0,0]..[10,10] \
         footprint covering the column's whole descent path, every descendant \
         is to_model and the column must be pruned. Expected empty plan, got \
         {} entries. diagnostics={:?}",
        entries.len(),
        output.diagnostics(),
    );
}

// ── AC-N1: default config keeps contacts inside the footprint ───────────────

/// AC-N1: With the default config (`support_on_build_plate_only = false`)
/// AND a contact whose XY is inside the footprint at the contact's layer,
/// the contact IS added to `contacts_by_layer` (no rejection) and the
/// planner emits ≥ 1 entry from the contact chain. The contact's
/// `to_buildplate` is `false` but that is internal — the externally
/// observable behavior is that the plan is non-empty.
#[test]
fn default_config_does_not_reject_to_model_contacts() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        // No support_on_build_plate_only key — defaults to false.
        ("tree_support_branch_diameter", ConfigValue::Float(5.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    // Single-triangle plate: the contact at (2.67, 1.33) is the lone
    // contact at layer 8. The footprint covers that centroid ⇒
    // to_buildplate = false after the implementation. The default
    // config admits the contact; the origin tip is emitted on layer 8
    // even though the contact lies inside collision_polys.
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.8],
        [4.0, 0.0, 1.8],
        [4.0, 4.0, 1.8],
    ];
    let triangles = vec![[1, 3, 2]];
    let obj = MeshObjectView {
        object_id: "ac-n1".to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    };
    let lp = make_layer_plan(10, 0.0, 0.2);
    let rs = make_region_segmentation("ac-n1", 10);

    // Footprint at the contact's layer (8) covers the centroid (2.67, 1.33).
    // No footprint at other layers — the propagation is unblocked.
    let covering_box = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    };
    let sg = SupportGeometryView {
        entries: vec![SupportGeometryViewEntry {
            global_support_layer_index: 8,
            object_id: "ac-n1".to_string(),
            region_id: "0".to_string(),
            outlines: vec![covering_box.clone()],
        }],
    };

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    let entries = output.entries();
    assert!(
        !entries.is_empty(),
        "AC-N1: default config must admit a to_model contact (centroid inside \
         footprint at the contact's layer). Expected non-empty plan, got {} \
         entries. diagnostics={:?}",
        entries.len(),
        output.diagnostics(),
    );
}

// ── AC-N2: with buildplate-only OFF, a to-model column is NOT pruned ────────

/// AC-N2: with `support_on_build_plate_only = false`, a column whose
/// descendants land inside the model footprint is NOT filed as an unsupported
/// branch leaf. It terminates on the model — canonical clears `valid`, which
/// stops propagation but still draws the node on its own layer — and the
/// layers above it keep their geometry.
///
/// **Rewired by packet 224 step 5 (F-14).** This asserted a code-1002
/// `node-clamped-out` diagnostic, on the premise that the "existing drop"
/// fired for to-model nodes too. Canonical has no such drop: the
/// `unsupported_branch_leaves` escalation is explicitly gated on
/// `support_on_buildplate_only`, and the else-branch is a plain
/// `p_node->valid = false`. So the canonical observable is the opposite of a
/// diagnostic — it is that the column SURVIVES on the layers above the model,
/// which is the property a false-positive prune would break.
#[test]
fn to_model_node_with_collision_not_pruned_by_new_rule() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        // No support_on_build_plate_only — contact is admitted.
        ("tree_support_branch_diameter", ConfigValue::Float(5.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    // A 2x2 grid of overhang triangles at z=1.8 → 4 contact centroids
    // separated into distinct dedup buckets. The MST has 3 edges; the propagation's
    // move path runs.
    let obj = multi_overhang_grid("ac-n2", 2, 2, 4.0);
    let lp = make_layer_plan(10, 0.0, 0.2);
    let rs = make_region_segmentation("ac-n2", 10);

    // The footprint covers the whole arena on the LOWER layers (0..=4) only.
    // The column is created around layer 7 and descends freely through the
    // clear layers 7, 6, 5, then meets the model at layer 4 and terminates
    // there. Covering every layer (as this fixture did before packet 224
    // step 5) makes even the contact tip collide, so the plan is empty for a
    // reason that has nothing to do with pruning.
    let covering_box = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, -10.0),
                Point2::from_mm(14.0, -10.0),
                Point2::from_mm(14.0, 14.0),
                Point2::from_mm(-10.0, 14.0),
            ],
        },
        holes: vec![],
    };
    let mut entries: Vec<SupportGeometryViewEntry> = Vec::new();
    for layer in 0..=4 {
        entries.push(SupportGeometryViewEntry {
            global_support_layer_index: layer,
            object_id: "ac-n2".to_string(),
            region_id: "0".to_string(),
            outlines: vec![covering_box.clone()],
        });
    }
    let sg = SupportGeometryView { entries };

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    // The column must survive: `support_on_build_plate_only` is off, so a
    // to-model node is a legal footing, not an unsupported leaf.
    assert!(
        !output.entries().is_empty(),
        "AC-N2: with support_on_build_plate_only=false a to-model column must \
         NOT be pruned; got an empty plan. diagnostics={:?}",
        output.diagnostics(),
    );
    // And it must not descend through the model: the branch stops where it
    // meets it rather than continuing to the plate.
    let planned_layers: Vec<i32> = output
        .entries()
        .iter()
        .filter(|entry| {
            entry.decline_reason.is_none()
                && entry.roles.iter().any(|role| !role.regions.is_empty())
        })
        .map(|entry| entry.global_layer_index)
        .collect();
    assert!(
        planned_layers.iter().all(|layer| *layer >= 4),
        "AC-N2: a branch that meets the model at layer 4 must terminate there, \
         not continue to the plate; planned layers={planned_layers:?}",
    );
}

// ── Test fixtures ──────────────────────────────────────────────────────────

fn make_planner_config(entries: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    for (k, v) in entries {
        map.insert((*k).to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

fn make_layer_plan(n: u32, base_z: f32, layer_height: f32) -> LayerPlanView {
    LayerPlanView {
        layers: (0..n)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: base_z + (i as f32 + 1.0) * layer_height,
                effective_layer_height: layer_height,
            })
            .collect(),
    }
}

fn make_region_segmentation(object_id: &str, n: u32) -> RegionSegmentationView {
    RegionSegmentationView {
        entries: (0..n)
            .map(|i| RegionSegmentationViewEntry {
                object_id: object_id.to_string(),
                layer_index: i,
                region_ids: vec!["0".to_string()],
            })
            .collect(),
        region_support_configs: Vec::new(),
    }
}

/// Standard two-triangle downward-facing overhang at z=1.8, anchored at the
/// origin so the object bounds span z=0..2.0 across the 11-layer plan at
/// 0.2 mm. The plate's two-triangle centroid is at (2.25, 2.0) — outside
/// the standard [0,0]..[2,2] footprint but inside a bigger [0,0]..[14,14] box.
fn overhang_plate_at_origin() -> MeshObjectView {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.8],
        [4.0, 0.0, 1.8],
        [4.0, 4.0, 1.8],
        [0.0, 4.0, 1.8],
    ];
    let triangles = vec![[1, 3, 2], [1, 4, 3]];
    MeshObjectView {
        object_id: "plate".to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    }
}

/// Build a mesh of `cols × rows` downward-facing overhang triangles laid
/// out on a `tile × tile` mm grid, anchored at the origin so bmin[2] = 0.0
/// and the rel_z gate (`rel_z >= first_layer_height * 0.5`) passes for
/// centroid z=1.8. All centroids sit at z=1.8, so they all funnel into
/// layer 8 (`z = 0.2 * 9 = 1.8`). Each tile contributes two CW-from-above
/// triangles so the normal z-component is negative (matching the
/// `detect_overhang_facets` threshold).
///
/// Returns an `MeshObjectView` whose `vertices[1..]` start the overhang
/// grid at (0, 0, 1.8); the first vertex (`vertices[0]`) is the anchor at
/// (0, 0, 0) so the object bounds span the full z range.
fn multi_overhang_grid(object_id: &str, cols: usize, rows: usize, tile: f32) -> MeshObjectView {
    let mut vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]];
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(cols * rows * 2);
    let overhang_z = 1.8_f32;
    for j in 0..rows {
        for i in 0..cols {
            let base = vertices.len() as u32;
            let gx = i as f32 * tile;
            let gy = j as f32 * tile;
            vertices.push([gx, gy, overhang_z]);
            vertices.push([gx + tile, gy, overhang_z]);
            vertices.push([gx + tile, gy + tile, overhang_z]);
            vertices.push([gx, gy + tile, overhang_z]);
            // CW winding from above ⇒ normal z < 0.
            triangles.push([base, base + 2, base + 1]);
            triangles.push([base, base + 3, base + 2]);
        }
    }
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    }
}
