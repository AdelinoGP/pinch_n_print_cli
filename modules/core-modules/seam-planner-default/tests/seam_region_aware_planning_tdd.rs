//! Regression tests for seam planning from active SliceIR regions.

#![allow(missing_docs)]

use seam_planner_default::run_aligned_planning_entries;
use slicer_sdk::prelude::*;

fn polygon(x: f32, y: f32, size: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, y),
                Point2::from_mm(x + size, y),
                Point2::from_mm(x + size, y + size),
                Point2::from_mm(x, y + size),
            ],
        },
        holes: Vec::new(),
    }
}

fn notched_polygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(20.0, 0.0),
                Point2::from_mm(20.0, 8.0),
                Point2::from_mm(8.0, 8.0),
                Point2::from_mm(8.0, 20.0),
                Point2::from_mm(0.0, 20.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn region(
    layer: u32,
    z: f32,
    x: f32,
    variant_chain: Vec<(String, PaintValue)>,
) -> SeamPlanningRegionInput {
    // exhaustive: this helper pins every region-input field for variant-aware planning
    SeamPlanningRegionInput {
        global_layer_index: layer,
        object_id: "object".to_string(),
        region_id: "1".to_string(),
        variant_chain,
        z,
        height: 0.2,
        ex_polygons: vec![polygon(x, 0.0, 10.0)],
        segment_annotations: Vec::new(),
        scoring_width: 0.4,
    }
}

#[test]
fn multi_region_two_variants_emit_independent_plans() {
    let view = SeamPlanningView {
        regions: vec![
            region(0, 0.2, 0.0, Vec::new()),
            region(
                0,
                0.2,
                20.0,
                vec![("material".to_string(), PaintValue::ToolIndex(1))],
            ),
        ],
    };

    let entries = run_aligned_planning_entries(&view, false);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].variant_chain, Vec::new());
    assert_eq!(
        entries[1].variant_chain,
        vec![("material".to_string(), PaintValue::ToolIndex(1))]
    );
    assert_ne!(
        (entries[0].chosen_position.x, entries[0].chosen_position.y),
        (entries[1].chosen_position.x, entries[1].chosen_position.y)
    );
}

#[test]
fn inactive_region_emits_no_plan() {
    // This fixture tests the absent-record case: layer 1 has no region record
    // at all, rather than an empty polygon record.
    let view = SeamPlanningView {
        regions: vec![region(0, 0.2, 0.0, Vec::new())],
    };

    let entries = run_aligned_planning_entries(&view, false);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].global_layer_index, 0);
    assert!(entries.iter().all(|entry| entry.global_layer_index != 1));
}

#[test]
fn notched_polygon_candidates_lie_on_supplied_boundary() {
    let supplied = notched_polygon();
    let view = SeamPlanningView {
        regions: vec![SeamPlanningRegionInput {
            ex_polygons: vec![supplied.clone()],
            ..region(0, 0.2, 3.0, Vec::new())
        }],
    };

    let entries = run_aligned_planning_entries(&view, false);
    let boundary = supplied.contour.points;
    for candidate in &entries[0].scored_candidates {
        assert!(boundary.iter().any(|point| {
            (candidate.position.x - units_to_mm(point.x)).abs() < 1e-5
                && (candidate.position.y - units_to_mm(point.y)).abs() < 1e-5
        }));
    }
}

#[test]
fn chosen_position_uses_supplied_layer_z() {
    let view = SeamPlanningView {
        regions: vec![region(0, 7.25, 0.0, Vec::new())],
    };

    let entries = run_aligned_planning_entries(&view, false);

    assert_eq!(entries[0].chosen_position.z, 7.25);
}

/// Regression guard for packet 206's delivery gap.
///
/// `paint_annotation_type` / `candidate_paint_classification` shipped correct
/// but with **zero production callers** — the whole of `src/visibility.rs` was
/// unreachable from `run_seam_planning`, so seam paint could not move the seam
/// on the default `seam_mode = "aligned"` (where `seam-placer` consumes the
/// planner's `resolved_seam` rather than the perimeter-side candidates). The
/// exactness tests passed anyway because they `#[path]`-include the module
/// source. This test drives the production entry point instead.
///
/// Both arms are load-bearing: the unpainted arm pins the plain aligned
/// tie-break so the painted arm proves a *change*, not a coincidence.
#[test]
fn seam_paint_moves_planner_resolved_seam() {
    // `region(…, 0.0, …)` supplies polygon(0,0,10): vertices (0,0), (10,0),
    // (10,10), (0,10). Aligned mode is min-y then min-x.
    let unpainted = SeamPlanningView {
        regions: vec![region(0, 0.2, 0.0, Vec::new())],
    };
    let baseline = run_aligned_planning_entries(&unpainted, false);
    assert_eq!(
        (baseline[0].chosen_position.x, baseline[0].chosen_position.y),
        (0.0, 0.0),
        "unpainted aligned planning must pick the min-y/min-x vertex"
    );

    // Enforce vertex 2 — (10,10), the vertex the unpainted tie-break ranks
    // last — so only a live classifier can produce it.
    let painted = SeamPlanningView {
        regions: vec![SeamPlanningRegionInput {
            segment_annotations: vec![(
                PaintSemantic::Custom("seam_enforcer".to_string()),
                vec![vec![None, None, Some(PaintValue::Flag(true)), None]],
            )],
            ..region(0, 0.2, 0.0, Vec::new())
        }],
    };
    let entries = run_aligned_planning_entries(&painted, false);

    assert_eq!(
        (entries[0].chosen_position.x, entries[0].chosen_position.y),
        (10.0, 10.0),
        "seam_enforcer paint must move the planner's chosen seam to the \
         enforced vertex"
    );
    assert!(entries[0]
        .scored_candidates
        .iter()
        .any(|candidate| candidate.reason.tag == "enforced"));
}

/// `seam_blocker` must remove a vertex from planning entirely, not merely
/// deprioritise it.
#[test]
fn seam_blocker_paint_excludes_vertex_from_planner_candidates() {
    let view = SeamPlanningView {
        regions: vec![SeamPlanningRegionInput {
            // Block vertex 0 — (0,0) — which unpainted planning would choose.
            segment_annotations: vec![(
                PaintSemantic::Custom("seam_blocker".to_string()),
                vec![vec![Some(PaintValue::Flag(true)), None, None, None]],
            )],
            ..region(0, 0.2, 0.0, Vec::new())
        }],
    };

    let entries = run_aligned_planning_entries(&view, false);

    assert_eq!(entries[0].scored_candidates.len(), 3, "blocked vertex dropped");
    assert!(
        !entries[0]
            .scored_candidates
            .iter()
            .any(|candidate| candidate.position.x == 0.0 && candidate.position.y == 0.0),
        "blocked vertex must not appear among the reported candidates"
    );
    // Next-best under min-y-then-min-x is (10,0).
    assert_eq!(
        (entries[0].chosen_position.x, entries[0].chosen_position.y),
        (10.0, 0.0)
    );
}

/// A region whose every vertex is blocked yields no candidates at all.
/// `seam-placer` degrades gracefully on this (it emits the walls pristine with
/// no resolved seam), so the planner must not panic or invent a seam.
#[test]
fn fully_blocked_region_yields_no_planner_candidates() {
    let view = SeamPlanningView {
        regions: vec![SeamPlanningRegionInput {
            segment_annotations: vec![(
                PaintSemantic::Custom("seam_blocker".to_string()),
                vec![vec![
                    Some(PaintValue::Flag(true)),
                    Some(PaintValue::Flag(true)),
                    Some(PaintValue::Flag(true)),
                    Some(PaintValue::Flag(true)),
                ]],
            )],
            ..region(0, 0.2, 0.0, Vec::new())
        }],
    };

    let entries = run_aligned_planning_entries(&view, false);

    assert!(
        entries.is_empty() || entries[0].scored_candidates.is_empty(),
        "a fully-blocked region must produce no seam candidates"
    );
}

#[test]
fn nonuniform_layer_z_chosen_position_uses_supplied_z() {
    let view = SeamPlanningView {
        regions: vec![
            region(0, 0.2, 0.0, Vec::new()),
            region(1, 0.6, 0.0, Vec::new()),
        ],
    };

    let entries = run_aligned_planning_entries(&view, false);

    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.global_layer_index == 0)
            .expect("layer 0 plan")
            .chosen_position
            .z,
        0.2
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.global_layer_index == 1)
            .expect("layer 1 plan")
            .chosen_position
            .z,
        0.6
    );
}

#[test]
fn no_candidate_sourced_from_mesh_object_view() {
    let mesh = MeshObjectView {
        object_id: "object".to_string(),
        vertices: vec![
            [1000.0f32, 1000.0, 1000.0],
            [1001.0, 1000.0, 1000.0],
            [1000.0, 1001.0, 1000.0],
        ],
        triangles: vec![[0, 1, 2]],
        paint_layers: Vec::new(),
    };
    let view = SeamPlanningView {
        regions: vec![region(0, 0.2, 0.0, Vec::new())],
    };

    // `mesh` is deliberately a decoy and is not part of the production
    // region-aware planning input. Candidates must remain on the supplied
    // region boundary instead of using these distant mesh vertices.
    let entries = run_aligned_planning_entries(&view, false);

    assert!(entries
        .iter()
        .flat_map(|e| &e.scored_candidates)
        .all(|candidate| {
            let mesh_vertex = mesh.vertices[mesh.triangles[0][0] as usize];
            let dx = candidate.position.x - mesh_vertex[0];
            let dy = candidate.position.y - mesh_vertex[1];
            let dz = candidate.position.z - mesh_vertex[2];
            (dx * dx + dy * dy + dz * dz).sqrt() > 1.0
        }));
}
