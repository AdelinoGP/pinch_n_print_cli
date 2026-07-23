//! Behavioral tests for the `Aligned` / `AlignedBack` seam planning modes
//! (packet 168, step 6): `run_seam_planning` must drive the contour ->
//! candidate -> alignment pipeline over the REAL layer z's from the
//! `LayerPlanView`, emitting exactly one `SeamPlanEntry` per
//! `(global_layer_index, object_id, region_id)` with full scored candidates.

#![allow(missing_docs)]

use seam_planner_default::SeamPlannerDefault;
use slicer_sdk::prelude::*;
use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
use std::collections::BTreeMap;

/// Axis-aligned square prism `[0,10] x [0,10] x [0,4]` mm with outward-facing
/// triangle winding: 8 vertices, 12 triangles.
fn square_prism() -> MeshObjectView {
    let (sx, sy, sz) = (10.0f32, 10.0f32, 4.0f32);
    MeshObjectView {
        object_id: "prism".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [sx, 0.0, 0.0],
            [sx, sy, 0.0],
            [0.0, sy, 0.0],
            [0.0, 0.0, sz],
            [sx, 0.0, sz],
            [sx, sy, sz],
            [0.0, sy, sz],
        ],
        triangles: vec![
            // bottom (-z)
            [0, 2, 1],
            [0, 3, 2],
            // top (+z)
            [4, 5, 6],
            [4, 6, 7],
            // front (-y)
            [0, 1, 5],
            [0, 5, 4],
            // right (+x)
            [1, 2, 6],
            [1, 6, 5],
            // back (+y)
            [2, 3, 7],
            [2, 7, 6],
            // left (-x)
            [3, 0, 4],
            [3, 4, 7],
        ],
        paint_layers: vec![],
    }
}

/// 20-entry layer plan: z = (i + 1) * 0.2 mm, global indices exactly 0..=19.
fn layer_plan() -> LayerPlanView {
    LayerPlanView {
        layers: (0..20)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: (i + 1) as f32 * 0.2,
                effective_layer_height: 0.2,
            })
            .collect(),
    }
}

fn region_input() -> SeamPlanningView {
    SeamPlanningView {
        regions: (0..20)
            .map(|i| SeamPlanningRegionInput {
                global_layer_index: i,
                object_id: "prism".to_string(),
                region_id: "0".to_string(),
                variant_chain: Vec::new(),
                z: (i + 1) as f32 * 0.2,
                height: 0.2,
                ex_polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2::from_mm(0.0, 0.0),
                            Point2::from_mm(10.0, 0.0),
                            Point2::from_mm(10.0, 10.0),
                            Point2::from_mm(0.0, 10.0),
                        ],
                    },
                    holes: Vec::new(),
                }],
                segment_annotations: Vec::new(),
                scoring_width: 0.4,
            })
            .collect(),
    }
}

fn run(seam_mode: &str) -> Vec<SeamPlanEntry> {
    let config = ConfigViewBuilder::new()
        .string("seam_mode", seam_mode)
        .build();
    let planner = SeamPlannerDefault::on_print_start(&config).expect("on_print_start must succeed");
    let mut output = SeamPlanningOutput::new();
    planner
        .run_seam_planning(
            &[square_prism()],
            &layer_plan(),
            &mut output,
            &config,
            &region_input(),
        )
        .expect("run_seam_planning must succeed");
    output.entries().to_vec()
}

/// AC-4: aligned mode emits exactly one entry per
/// `(global_layer_index, object_id, region_id)` for all 20 layers, every
/// entry has non-empty scored candidates, entries carry the REAL plan z's,
/// and all chosen positions lock onto one single prism corner (max pairwise
/// XY spread <= 0.5 mm).
#[test]
fn aligned_chain_locks_single_corner() {
    let entries = run("aligned");

    // Exactly one entry per (layer, object, region), covering all 20 layers.
    let mut per_key: BTreeMap<(u32, String, String), usize> = BTreeMap::new();
    for e in &entries {
        *per_key
            .entry((
                e.global_layer_index,
                e.object_id.clone(),
                e.region_id.clone(),
            ))
            .or_insert(0) += 1;
    }
    for (key, count) in &per_key {
        assert_eq!(*count, 1, "duplicate entry for {key:?}");
    }
    let layers_seen: Vec<u32> = per_key.keys().map(|k| k.0).collect();
    assert_eq!(
        layers_seen,
        (0..20).collect::<Vec<u32>>(),
        "must cover exactly global layer indices 0..=19"
    );

    for e in &entries {
        assert!(
            !e.scored_candidates.is_empty(),
            "layer {}: scored_candidates must be non-empty",
            e.global_layer_index
        );
        // The pipeline must run over the REAL plan z's: z = (i + 1) * 0.2.
        let expected_z = (e.global_layer_index + 1) as f32 * 0.2;
        assert!(
            (e.chosen_position.z - expected_z).abs() < 1e-4,
            "layer {}: z {} != plan z {}",
            e.global_layer_index,
            e.chosen_position.z,
            expected_z
        );
    }

    // All chosen positions within 0.5 mm XY of one single corner.
    let corners = [(0.0f32, 0.0f32), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    let near_one_corner = corners.iter().any(|&(cx, cy)| {
        entries.iter().all(|e| {
            (e.chosen_position.x - cx).abs() <= 0.5 && (e.chosen_position.y - cy).abs() <= 0.5
        })
    });
    // Max pairwise XY spread <= 0.5 mm.
    let mut max_spread = 0.0f32;
    for a in &entries {
        for b in &entries {
            let dx = a.chosen_position.x - b.chosen_position.x;
            let dy = a.chosen_position.y - b.chosen_position.y;
            max_spread = max_spread.max((dx * dx + dy * dy).sqrt());
        }
    }
    assert!(
        near_one_corner && max_spread <= 0.5,
        "seams must lock one corner: near_one_corner={near_one_corner}, max_spread={max_spread}"
    );
}

/// AC-5: aligned_back mode places every layer's seam within 0.5 mm of the
/// prism's max Y (rear).
#[test]
fn aligned_back_prefers_rear_corner() {
    let entries = run("aligned_back");
    assert_eq!(entries.len(), 20, "one entry per layer expected");
    for e in &entries {
        assert!(
            (e.chosen_position.y - 10.0).abs() <= 0.5,
            "layer {}: y = {} not within 0.5 mm of max Y (10.0)",
            e.global_layer_index,
            e.chosen_position.y
        );
    }
}
