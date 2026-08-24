//! Geometric invariants for traditional support scan-line filling.

use slicer_ir::{
    ConfigView, ExPolygon, Point2, Polygon, SupportPlanEntry, SupportPlanIR, SupportPlanRole,
    SupportPlanRoleRegion,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;
use std::sync::Arc;
use traditional_support::TraditionalSupport;

fn config(angle: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_density", 20.0)
        .float("support_angle", angle)
        .float("support_speed", 50.0)
        .float("line_width", line_width)
        .build()
}

fn region(points: &[(f32, f32)]) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(0.3)
        .add_polygon(ExPolygon {
            contour: Polygon {
                points: points.iter().map(|&(x, y)| Point2::from_mm(x, y)).collect(),
            },
            holes: vec![],
        })
        .build()
}

/// Since packets 220–222 the module renders ONLY planned structural entries
/// (no legacy filler over raw slice regions), so every fixture must seed a
/// plan carrying the region under test as a `SupportBody` polygon.
fn paint_with_plan(points: &[(f32, f32)]) -> PaintRegionLayerView {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    let entry = SupportPlanEntry {
        global_layer_index: 0,
        object_id: "obj1".into(),
        region_id: 1,
        family_id: "traditional".into(),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![ExPolygon {
                contour: Polygon {
                    points: points.iter().map(|&(x, y)| Point2::from_mm(x, y)).collect(),
                },
                holes: vec![],
            }],
        }],
        demand_ids: vec!["fill-geometry-demand".into()],
        body_ids: vec!["fill-geometry-body".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["test".into()],
        decline_reason: None,
    };
    PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: vec![entry],
        ..Default::default()
    }))
}

fn run_support(
    points: &[(f32, f32)],
    angle: f64,
    line_width: f64,
) -> Vec<slicer_ir::ExtrusionPath3D> {
    let config = config(angle, line_width);
    let module = TraditionalSupport::from_config(&config).unwrap();
    let paint = paint_with_plan(points);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region(points)], &paint, &mut output, &config)
        .unwrap();
    output.support_paths().to_vec()
}

#[test]
fn scan_starts_at_rotated_bbox_min() {
    let paths = run_support(
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        0.0,
        0.4,
    );
    assert!(!paths.is_empty());
    assert!(paths[0].points[0].y.abs() < 0.001);
}

#[test]
fn crossing_vertex_contributes_one_intersection() {
    let paths = run_support(
        &[(0.0, -4.0), (5.0, 0.0), (10.0, 6.0), (0.0, 6.0)],
        0.0,
        0.4,
    );
    let crossing = paths
        .iter()
        .find(|path| path.points[0].y.abs() < 0.001)
        .expect("visited scan line at crossing vertex");
    assert!((crossing.points[1].x - crossing.points[0].x).abs() > 4.9);
}

#[test]
fn fill_phase_is_translation_invariant() {
    let base = run_support(
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        27.0,
        0.4,
    );
    let shifted = run_support(
        &[(3.0, 2.0), (13.0, 2.0), (13.0, 12.0), (3.0, 12.0)],
        27.0,
        0.4,
    );
    assert_eq!(base.len(), shifted.len());
    for (a, b) in base.iter().zip(shifted.iter()) {
        for (pa, pb) in a.points.iter().zip(b.points.iter()) {
            assert!((pb.x - pa.x - 3.0).abs() < 0.001);
            assert!((pb.y - pa.y - 2.0).abs() < 0.001);
        }
    }
}

#[test]
fn zero_length_span_is_dropped() {
    let paths = run_support(&[(5.0, 0.0), (0.0, 10.0), (10.0, 10.0)], 0.0, 0.4);
    assert!(paths.iter().all(|path| {
        (path.points[0].x - path.points[1].x).abs() + (path.points[0].y - path.points[1].y).abs()
            > 0.0
    }));
    assert!(paths[0].points[0].y > 0.0);
}

#[test]
fn non_positive_spacing_yields_no_paths() {
    assert!(run_support(
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        0.0,
        0.0
    )
    .is_empty());
}
