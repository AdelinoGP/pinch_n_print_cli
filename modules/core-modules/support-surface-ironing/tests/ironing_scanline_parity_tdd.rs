//! Parity tests for the axis-aligned ironing scan-line contract.

use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;
use support_surface_ironing::SupportSurfaceIroning;

fn config_with(entries: Vec<(&str, ConfigValue)>) -> ConfigView {
    let mut fields = HashMap::new();
    for (key, value) in entries {
        fields.insert(key.to_string(), value);
    }
    ConfigView::from_map(fields)
}

fn region_with_polygon(polygon: ExPolygon, z: f32) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![polygon]);
    region.set_z(z);
    region
}

fn run(polygon: ExPolygon, spacing: f32) -> Vec<slicer_ir::ExtrusionPath3D> {
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_spacing", ConfigValue::Float(spacing as f64)),
    ]);
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_polygon(polygon, 1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    output.support_paths().to_vec()
}

#[test]
fn scan_starts_at_bbox_min() {
    let polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: 50_000,
                    y: 50_000,
                },
                Point2 {
                    x: 150_000,
                    y: 50_000,
                },
                Point2 {
                    x: 150_000,
                    y: 150_000,
                },
                Point2 {
                    x: 50_000,
                    y: 150_000,
                },
            ],
        },
        holes: Vec::new(),
    };
    let paths = run(polygon, 1.0);
    assert!(!paths.is_empty());
    assert!(
        (paths[0].points[0].y - 5.0).abs() < 0.001,
        "first scan y was {}",
        paths[0].points[0].y
    );
}

#[test]
fn crossing_vertex_contributes_one_intersection() {
    let polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: -40_000 },
                Point2 { x: 50_000, y: 0 },
                Point2 {
                    x: 100_000,
                    y: 60_000,
                },
                Point2 { x: 0, y: 60_000 },
            ],
        },
        holes: Vec::new(),
    };
    let paths = run(polygon, 0.5);
    let crossing = paths
        .iter()
        .find(|path| path.points[0].y.abs() < 0.001)
        .expect("visited scan line at crossing vertex");
    assert!(crossing.points[0].x < crossing.points[1].x);
    assert!((crossing.points[1].x - crossing.points[0].x) > 4.9);
}

#[test]
fn zero_length_span_is_dropped() {
    let polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: 10_000,
                    y: 5_000,
                },
                Point2 {
                    x: 5_000,
                    y: 10_000,
                },
                Point2 {
                    x: 15_000,
                    y: 10_000,
                },
            ],
        },
        holes: Vec::new(),
    };
    let paths = run(polygon, 5.0);
    assert!(paths.is_empty());
}
