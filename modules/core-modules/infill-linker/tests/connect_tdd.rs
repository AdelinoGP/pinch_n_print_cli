#![allow(missing_docs)]

use infill_linker::connect::connect_infill;
use infill_linker::graph::BoundaryInfillGraph;
use slicer_ir::{ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth, Polygon};

fn square(size_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(size_mm, 0.0),
                Point2::from_mm(size_mm, size_mm),
                Point2::from_mm(0.0, size_mm),
            ],
        },
        holes: vec![],
    }
}

fn path(y_mm: f32, role: ExtrusionRole, speed_factor: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 0.0,
                y: y_mm,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            Point3WithWidth {
                x: 10.0,
                y: y_mm,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
        ],
        role,
        speed_factor,
    }
}

fn raw_paths(role: ExtrusionRole, speed_factor: f32) -> Vec<ExtrusionPath3D> {
    (1..=8)
        .map(|index| path(index as f32, role.clone(), speed_factor))
        .collect()
}

fn linked_paths() -> Vec<ExtrusionPath3D> {
    let boundary = square(10.0);
    let graph = BoundaryInfillGraph::new(&[boundary]);
    connect_infill(raw_paths(ExtrusionRole::SparseInfill, 1.0), &graph, 1.0)
}

#[test]
fn raw_segments_in_linked_polylines_out() {
    let output = linked_paths();

    assert!(output.iter().all(|path| path.points.len() >= 2));
    assert!(output.len() < 8);
}

#[test]
fn role_and_speed_preserved() {
    let boundary = square(10.0);
    let graph = BoundaryInfillGraph::new(&[boundary]);
    let output = connect_infill(raw_paths(ExtrusionRole::SparseInfill, 0.8), &graph, 1.0);

    assert!(output
        .iter()
        .all(|path| path.role == ExtrusionRole::SparseInfill));
    assert!(output
        .iter()
        .all(|path| (path.speed_factor - 0.8).abs() < f32::EPSILON));
}

#[test]
fn connect_deterministic() {
    assert_eq!(linked_paths(), linked_paths());
}
