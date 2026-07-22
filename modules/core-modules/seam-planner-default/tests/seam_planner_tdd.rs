//! Behavioral tests for the seam-planner-default `PrepassModule`.
//!
//! Lifted from the former inline `#[cfg(test)]` module in `src/lib.rs` (which
//! constructed the planner via its private `mode` field) and expanded. External
//! tests build the planner through the public `on_print_start` constructor.

#![allow(missing_docs)]

use seam_planner_default::SeamPlannerDefault;
use slicer_sdk::prelude::*;

fn planner() -> SeamPlannerDefault {
    SeamPlannerDefault::on_print_start(&ConfigView::default()).expect("on_print_start must succeed")
}

/// Unit cube: 8 vertices, 12 triangles.
fn unit_cube() -> MeshObjectView {
    MeshObjectView {
        object_id: "cube".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        triangles: vec![
            [0, 1, 2],
            [0, 2, 3],
            [4, 6, 5],
            [4, 7, 6],
            [0, 4, 5],
            [0, 5, 1],
            [2, 6, 7],
            [2, 7, 3],
            [0, 3, 7],
            [0, 7, 4],
            [1, 5, 6],
            [1, 6, 2],
        ],
        paint_layers: vec![],
    }
}

fn cube_region_input() -> SeamPlanningView {
    SeamPlanningView {
        regions: vec![SeamPlanningRegionInput {
            global_layer_index: 0,
            object_id: "cube".to_string(),
            region_id: "0".to_string(),
            variant_chain: Vec::new(),
            z: 0.2,
            height: 0.2,
            ex_polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(1.0, 0.0),
                        Point2::from_mm(1.0, 1.0),
                        Point2::from_mm(0.0, 1.0),
                    ],
                },
                holes: Vec::new(),
            }],
            segment_annotations: Vec::new(),
            scoring_width: 0.4,
        }],
    }
}

#[test]
fn no_objects_emits_nothing() {
    let mut output = SeamPlanningOutput::new();
    let result = planner().run_seam_planning(
        &[],
        &LayerPlanView { layers: vec![] },
        &mut output,
        &ConfigView::default(),
        &SeamPlanningView::default(),
    );
    assert!(result.is_ok());
    assert!(output.entries().is_empty());
}

#[test]
fn object_with_no_triangles_is_skipped() {
    let degenerate = MeshObjectView {
        object_id: "empty".to_string(),
        vertices: vec![[0.0, 0.0, 0.0]],
        triangles: vec![],
        paint_layers: vec![],
    };
    let mut output = SeamPlanningOutput::new();
    planner()
        .run_seam_planning(
            &[degenerate],
            &LayerPlanView { layers: vec![] },
            &mut output,
            &ConfigView::default(),
            &SeamPlanningView::default(),
        )
        .expect("run_seam_planning must succeed");
    assert!(
        output.entries().is_empty(),
        "an object with zero triangles must produce no seam entries"
    );
}

#[test]
fn cube_generates_corner_candidates() {
    let mut output = SeamPlanningOutput::new();
    let result = planner().run_seam_planning(
        &[unit_cube()],
        &LayerPlanView { layers: vec![] },
        &mut output,
        &ConfigView::default(),
        &cube_region_input(),
    );
    assert!(result.is_ok(), "seam planning should succeed");
    let entries = output.entries();
    assert!(
        !entries.is_empty(),
        "cube should generate seam plan entries"
    );

    for entry in entries {
        assert!(!entry.object_id.is_empty());
        assert!(!entry.region_id.is_empty());
        assert!(entry.scored_candidates.len() <= 10);
        assert!(entry.chosen_position.x.is_finite());
        assert!(entry.chosen_position.y.is_finite());
        assert!(entry.chosen_position.z.is_finite());
    }
}

#[test]
fn seam_planning_is_deterministic_across_runs() {
    let run_once = || {
        let mut output = SeamPlanningOutput::new();
        planner()
            .run_seam_planning(
                &[unit_cube()],
                &LayerPlanView { layers: vec![] },
                &mut output,
                &ConfigView::default(),
                &cube_region_input(),
            )
            .unwrap();
        output
            .entries()
            .iter()
            .map(|e| {
                (
                    e.object_id.clone(),
                    e.region_id.clone(),
                    e.chosen_position.x.to_bits(),
                    e.chosen_position.y.to_bits(),
                    e.chosen_position.z.to_bits(),
                )
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(
        run_once(),
        run_once(),
        "seam planning must be deterministic across repeated runs"
    );
}
