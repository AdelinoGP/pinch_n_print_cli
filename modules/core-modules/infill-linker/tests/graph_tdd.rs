#![allow(missing_docs)]

use infill_linker::graph::BoundaryInfillGraph;
use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon};

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

fn square_with_hole() -> ExPolygon {
    ExPolygon {
        contour: square(10.0).contour,
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(3.0, 3.0),
                Point2::from_mm(7.0, 3.0),
                Point2::from_mm(7.0, 7.0),
                Point2::from_mm(3.0, 7.0),
            ],
        }],
    }
}

#[test]
fn projection_on_square() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);

    assert_eq!(graph.total_len(), mm_to_units(40.0) as f64);
    assert_eq!(
        graph.project(Point2::from_mm(5.0, 5.0)),
        Some(mm_to_units(5.0) as f64)
    );
    assert_eq!(
        graph.project(Point2::from_mm(5.0, -2.0)),
        Some(mm_to_units(5.0) as f64)
    );
}

#[test]
fn arc_distance_on_square_with_hole() {
    let graph = BoundaryInfillGraph::new(&[square_with_hole()]);
    let from = graph
        .project(Point2::from_mm(0.0, 0.0))
        .expect("outer start");
    let to = graph
        .project(Point2::from_mm(10.0, 0.0))
        .expect("outer edge end");

    assert_eq!(graph.total_len(), mm_to_units(56.0) as f64);
    assert_eq!(
        graph.distance_along_boundary(from, to),
        mm_to_units(10.0) as f64
    );
    assert_eq!(graph.distance_along_boundary(graph.total_len(), 0.0), 0.0);
}

#[test]
fn walk_query_within_threshold() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let from = graph.project(Point2::from_mm(1.0, 0.0)).expect("from");
    let to = graph.project(Point2::from_mm(2.0, 0.0)).expect("to");
    let expected = mm_to_units(1.0) as f64;

    assert_eq!(
        graph.walk_distance(from, to, mm_to_units(2.0) as f64),
        Some(expected)
    );
}
