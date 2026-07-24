#![allow(missing_docs)]

use infill_linker::connect::connect_infill;
use infill_linker::graph::BoundaryInfillGraph;
use slicer_ir::{
    point_in_polygon_winding, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth,
    Polygon,
};

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

// ── contour-routed connectors (ADR-0025 §2 containment) ─────────────────────
//
// Canonical `Fill::connect_infill` (`src/libslic3r/Fill/FillBase.cpp`) never
// emits a bare chord: `take_ccw_full` / `take_cw_full` copy the run of contour
// vertices between two T-joints verbatim, so the connector IS boundary geometry
// and containment is structural. These guard that property in-tree.

/// L-shape: the left column + bottom row of a 10×10 area. The 6×6 top-right
/// "notch" is outside the polygon but inside its bounding box, so a straight
/// chord between two boundary points either side of the reflex corner at (4,4)
/// escapes the polygon.
fn l_shape() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 4.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(4.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    }
}

fn point(x_mm: f32, y_mm: f32) -> Point3WithWidth {
    Point3WithWidth {
        x: x_mm,
        y: y_mm,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn segment(start: (f32, f32), end: (f32, f32)) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![point(start.0, start.1), point(end.0, end.1)],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn has_vertex(path: &ExtrusionPath3D, x_mm: f32, y_mm: f32) -> bool {
    path.points
        .iter()
        .any(|p| (p.x - x_mm).abs() < 1e-3 && (p.y - y_mm).abs() < 1e-3)
}

#[test]
fn connector_routes_through_the_reflex_corner_instead_of_chording_the_notch() {
    // Endpoints (7,4) and (4,7) sit on the two contour edges that meet at the
    // reflex corner. A bare chord between them passes through (5.5,5.5), which
    // is outside the L; the contour-routed connector must materialise (4,4).
    let graph = BoundaryInfillGraph::new(&[l_shape()]);
    let output = connect_infill(
        vec![
            segment((7.0, 1.0), (7.0, 4.0)),
            segment((4.0, 7.0), (1.0, 7.0)),
        ],
        &graph,
        1.0,
    );

    assert_eq!(output.len(), 1, "the two lines share a ring and must link");
    let linked = &output[0];
    assert!(
        has_vertex(linked, 4.0, 4.0),
        "connector must materialise the reflex corner (4,4); got {:?}",
        linked.points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()
    );

    let container = l_shape();
    for p in &linked.points {
        assert!(
            point_in_polygon_winding(&container, p.x as f64, p.y as f64, 0.01),
            "linked vertex ({}, {}) escaped the L-shape — connector chorded the notch",
            p.x,
            p.y
        );
    }
}

#[test]
fn connector_walks_a_hole_ring_rather_than_cutting_across_it() {
    // Both joined endpoints project onto the hole ring; the walk between them
    // passes the hole corner (4,6), which must appear as a real vertex.
    let frame = ExPolygon {
        contour: square(10.0).contour,
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(6.0, 4.0),
                Point2::from_mm(6.0, 6.0),
                Point2::from_mm(4.0, 6.0),
            ],
        }],
    };
    let graph = BoundaryInfillGraph::new(&[frame.clone()]);
    // spacing 0.4 mm → a 4 mm walk budget, which admits only the 2 mm hole-ring
    // pair and not the 10 mm outer-ring pair.
    let output = connect_infill(
        vec![
            segment((0.0, 5.0), (4.0, 5.0)),
            segment((5.0, 10.0), (5.0, 6.0)),
        ],
        &graph,
        0.4,
    );

    assert_eq!(output.len(), 1, "the two lines share the hole ring");
    let linked = &output[0];
    assert!(
        has_vertex(linked, 4.0, 6.0),
        "connector must materialise the hole corner (4,6); got {:?}",
        linked.points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()
    );
    // Structural: no vertex may land strictly inside the hole.
    for p in &linked.points {
        assert!(
            !(p.x > 4.0 + 1e-3 && p.x < 6.0 - 1e-3 && p.y > 4.0 + 1e-3 && p.y < 6.0 - 1e-3),
            "linked vertex ({}, {}) is inside the hole — connector cut across it",
            p.x,
            p.y
        );
    }
}

#[test]
fn endpoints_on_different_rings_are_never_joined() {
    // Two disjoint islands 0.5 mm apart. (10,5) and (10.5,5) are by far the
    // nearest compatible endpoint pair, and the walk budget here (10 × 5 mm) is
    // large enough that distance is not what rejects them — they resolve to
    // different rings, and canonical never bridges rings.
    let left = square(10.0);
    let right = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(10.5, 0.0),
                Point2::from_mm(20.5, 0.0),
                Point2::from_mm(20.5, 10.0),
                Point2::from_mm(10.5, 10.0),
            ],
        },
        holes: vec![],
    };
    let graph = BoundaryInfillGraph::new(&[left, right]);
    let output = connect_infill(
        vec![
            segment((5.0, 5.0), (10.0, 5.0)),
            segment((10.5, 5.0), (15.0, 5.0)),
        ],
        &graph,
        5.0,
    );

    assert_eq!(
        output.len(),
        2,
        "cross-ring endpoints must stay separate, not be chorded across the gap"
    );
    for path in &output {
        let min_x = path
            .points
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = path
            .points
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            !(min_x <= 10.0 && max_x >= 10.5),
            "a path spans the gap between the two islands"
        );
    }
}
