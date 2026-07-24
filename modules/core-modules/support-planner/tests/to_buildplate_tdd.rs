//! RED-phase TDD tests for packet 123 (`support-planner-to-buildplate-pruning`).
//!
//! The tests drive the public `run_support_geometry` entry point and observe
//! the planner's branch emission + typed diagnostics to verify the new
//! `to_buildplate` flag's two effects:
//!
//! 1. **Contact creation** — `to_buildplate = !point_in_any_expoly(collision_polys, x, y)`
//!    for each contact. The "outside the footprint" test uses
//!    `support_on_build_plate_only = true` as a binding constraint: a contact
//!    whose `to_buildplate` is `false` is rejected at creation, so a contact
//!    accepted under that config MUST have been classified `to_buildplate = true`.
//! 2. **Propagation pruning** — when a node's clamped target lies inside
//!    `collision_polys`, the existing code 1002 drop fires for ALL nodes.
//!
//! The `to_buildplate = true` prune is observable only through the
//! `support_on_build_plate_only` config (which gates contact creation).
//!
//! All tests are authored BEFORE the implementation; they are expected to
//! FAIL TO COMPILE or FAIL TO PASS. The compile failure (missing struct
//! field `to_buildplate` on `PlannedSupportNode`) is the canonical RED
//! state for AC-1.

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

use support_planner::SupportPlanner;

// ── AC-2: contact XY outside the per-layer footprint → to_buildplate=true ────

/// AC-2: A contact whose XY lies outside the object's per-layer footprint
/// (the `SupportGeometryView` outline at the contact's layer) is classified
/// `to_buildplate = true`. The acceptance gate: with
/// `support_on_build_plate_only = true`, only `to_buildplate = true`
/// contacts are admitted. We place the overhang contact at (2.25, 2.0) and
/// set a small footprint at the contact's layer (z=1.8 = layer 8) that
/// EXCLUDES the centroid. The contact must be admitted, and the plan
/// must emit ≥ 1 entry (the contact's origin tip on layer 8).
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
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

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

// ── AC-3: to_buildplate=true node whose clamped target is in collision_polys
//    is dropped during propagation ──────────────────────────────────────────

/// AC-3: A `to_buildplate = true` contact whose move target is clamped
/// into a region of `collision_polys` (the entire planner arena is covered
/// by collision_polys at the layer below the contact layer so the
/// only valid move is into the collision) is dropped at propagation time.
/// The existing code 1002 diagnostic is emitted. The test asserts: at
/// least one code-1002 diagnostic with "node-clamped-out" in the message
/// is emitted.
#[test]
fn unreachable_buildplate_node_pruned() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("tree_support_branch_diameter", ConfigValue::Float(5.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

    // A 2x2 grid of overhang triangles at z=1.8 → 4 contact centroids
    // forming a tight cluster. The MST has 3 edges; each node has at
    // least one neighbour, so the propagation's move path runs.
    let obj = multi_overhang_grid("ac3", 2, 2, 0.4);
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

    let diagnostics = output.diagnostics();
    let clamped: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == 1002 && d.message.contains("node-clamped-out"))
        .collect();
    assert!(
        !clamped.is_empty(),
        "AC-3: expected at least one code-1002 node-clamped-out diagnostic; \
         got {} diagnostics: {:?}",
        diagnostics.len(),
        diagnostics,
    );
}

// ── AC-4: support_on_build_plate_only=true rejects to_model contacts ─────────

/// AC-4: With `support_on_build_plate_only = true` and a contact whose XY
/// is INSIDE the object's per-layer footprint, the contact is rejected at
/// creation time. The footprint at the contact's layer (8) covers the
/// contact centroid (2.25, 2.0); the contact is therefore
/// `to_buildplate = false` and must NOT be admitted. The test asserts:
/// the emitted plan is empty (no entry carries the rejected contact's
/// geometry).
///
/// The footprint at all OTHER layers is small/absent so the propagation
/// would not be blocked by collision_polys (no false-positive drops).
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
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

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

    // At the contact's layer (8), a large footprint covers the contact
    // centroid (2.67, 1.33) ⇒ to_buildplate = false after the
    // implementation. Before the implementation, the contact is admitted
    // and the origin tip is emitted on layer 8 (entries non-empty). The
    // test asserts the plan IS empty (contact rejected at creation).
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
            object_id: "ac4".to_string(),
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
        entries.is_empty(),
        "AC-4: with support_on_build_plate_only=true and the contact inside \
         the [0,0]..[10,10] footprint at the contact's layer, the contact \
         must be rejected at creation. Expected empty plan, got {} entries. \
         diagnostics={:?}",
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
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

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

// ── AC-N2: to_buildplate=false node with collision still gets existing drop ─

/// AC-N2: A `to_buildplate = false` node whose clamped target lies inside
/// `collision_polys` is NOT pruned by the new packet's logic
/// (`to_buildplate = true`-only branch does not fire), but the EXISTING
/// drop at the propagation site (`point_in_any_expoly(collision_polys, ...)`)
/// still fires. Externally observable: at least one code-1002 diagnostic
/// is emitted. This test exercises a multi-node contact whose centroids
/// are inside the contact-layer footprint (`to_buildplate = false`),
/// and a big footprint at all lower layers so the propagation's clamped
/// target lands in collision.
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
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

    // A 2x2 grid of overhang triangles at z=1.8 → 4 contact centroids
    // forming a tight cluster. The MST has 3 edges; the propagation's
    // move path runs.
    let obj = multi_overhang_grid("ac-n2", 2, 2, 0.4);
    let lp = make_layer_plan(10, 0.0, 0.2);
    let rs = make_region_segmentation("ac-n2", 10);

    // At the contact's layer (8), a footprint covers the contact centroids
    // (in [0..0.8]×[0..0.8]) ⇒ to_buildplate = false. At all other layers,
    // a big footprint covers the whole arena so the propagation's clamped
    // move target is always inside collision_polys and the drop fires.
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
    for layer in 0..10 {
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

    let diagnostics = output.diagnostics();
    let clamped: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == 1002 && d.message.contains("node-clamped-out"))
        .collect();
    assert!(
        !clamped.is_empty(),
        "AC-N2: existing drop must still fire for to_buildplate=false nodes \
         when their clamped target lands in collision_polys. Expected ≥ 1 \
         code-1002 node-clamped-out diagnostic; got {} diagnostics: {:?}",
        diagnostics.len(),
        diagnostics,
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
