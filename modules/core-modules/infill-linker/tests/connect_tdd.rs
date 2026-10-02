#![allow(missing_docs)]

use infill_linker::connect::{connect_infill, AnchorParams};
use infill_linker::graph::BoundaryInfillGraph;
use slicer_ir::{
    point_in_polygon_winding, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth,
    Polygon,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

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
                ..Default::default()
            },
            Point3WithWidth {
                x: 10.0,
                y: y_mm,
                z: 0.2,
                width: 0.4,
                ..Default::default()
            },
        ],
        speed_factor,
        ..extrusion_path3d_base(role)
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
    connect_infill(
        raw_paths(ExtrusionRole::SparseInfill, 1.0),
        &graph,
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 10.0,
        },
    )
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
    let output = connect_infill(
        raw_paths(ExtrusionRole::SparseInfill, 0.8),
        &graph,
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 10.0,
        },
    );

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

/// ADR-0058: authored per-path `tool_index` is a chaining-compatibility axis.
/// Two adjacent infill lines that the linker *does* chain when their tools match
/// must stay separate when the tools differ.
#[test]
fn cross_tool_paths_not_chained() {
    fn link_with_tools(first_tool: Option<u32>, second_tool: Option<u32>) -> Vec<ExtrusionPath3D> {
        let boundary = square(10.0);
        let graph = BoundaryInfillGraph::new(&[boundary]);
        let mut first = path(1.0, ExtrusionRole::SparseInfill, 1.0);
        let mut second = path(2.0, ExtrusionRole::SparseInfill, 1.0);
        first.tool_index = first_tool;
        second.tool_index = second_tool;
        connect_infill(
            vec![first, second],
            &graph,
            AnchorParams {
                anchor_length_mm: 0.0,
                anchor_length_max_mm: 10.0,
            },
        )
    }

    // Control: identical geometry with a shared tool DOES chain into one path.
    let same_tool = link_with_tools(Some(0), Some(0));
    assert_eq!(
        same_tool.len(),
        1,
        "control: same-tool adjacent paths must chain into a single path"
    );
    assert_eq!(
        same_tool[0].tool_index,
        Some(0),
        "chained path must carry the shared authored tool, not None"
    );

    // Same geometry, differing authored tools: must NOT chain.
    let cross_tool = link_with_tools(Some(0), Some(1));
    assert_eq!(
        cross_tool.len(),
        2,
        "differing tool_index must prevent chaining"
    );
    let mut tools = cross_tool
        .iter()
        .map(|path| path.tool_index)
        .collect::<Vec<_>>();
    tools.sort();
    assert_eq!(tools, vec![Some(0), Some(1)]);
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
        ..Default::default()
    }
}

fn segment(start: (f32, f32), end: (f32, f32)) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![point(start.0, start.1), point(end.0, end.1)],
        ..extrusion_path3d_base(ExtrusionRole::SparseInfill)
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
    // Endpoints sit ON the two contour edges meeting at the reflex corner,
    // the way clipped scan lines do: (7,4) on the y=4 edge, (4,7) on the x=4
    // edge. The far ends anchor the paths in the L's two arms.
    let output = connect_infill(
        vec![
            segment((7.0, 1.0), (7.0, 4.0)),
            segment((4.0, 7.0), (1.0, 7.0)),
        ],
        &graph,
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 10.0,
        },
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
fn outer_ring_connector_never_routes_through_a_hole_ring() {
    // L0 benchy bottom-fill regression: a hole ring and the outer contour are
    // separate boundary loops. When one endpoint projects to the outer contour
    // and the other to a hole, no shared ring exists, so the endpoints must
    // stay unjoined — routing the walk along the hole ring would drag
    // extrusion across the void the hole reserves (and a forced outer-ring
    // walk would swing the long way round the part instead).
    //
    // Canonical Fill::connect_infill (FillBase.cpp::create_boundary_infill_graph
    // + connect_infill) keys every T-joint to one contour_idx and only ever
    // takes contour runs between joints on the SAME contour; cross-contour
    // pairs fall through to take_limited stubs or stay separate, never a
    // full-contour take across the gap.
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
    let graph = BoundaryInfillGraph::new(&[frame]);
    // (0,5)->(3.9,5) ends just outside the hole's left edge; (4.1,5)->(5,5)
    // starts just inside the hole span. The outer-projected endpoint must not
    // be spliced to the hole-projected one via either ring.
    let output = connect_infill(
        vec![
            segment((0.0, 5.0), (3.9, 5.0)),
            segment((4.1, 5.0), (5.0, 5.0)),
        ],
        &graph,
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 50.0,
        },
    );

    assert_eq!(
        output.len(),
        2,
        "outer-anchored and hole-anchored endpoints share no ring and must stay separate; got {:?}",
        output
            .iter()
            .map(|p| p.points.iter().map(|q| (q.x, q.y)).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
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
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 50.0,
        },
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

fn point_segment_distance_mm(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

fn ring_edges(boundary: &ExPolygon) -> Vec<((f32, f32), (f32, f32))> {
    std::iter::once(&boundary.contour)
        .chain(boundary.holes.iter())
        .flat_map(|ring| {
            (0..ring.points.len()).map(move |index| {
                let a = ring.points[index].to_mm();
                let b = ring.points[(index + 1) % ring.points.len()].to_mm();
                (a, b)
            })
        })
        .collect()
}

#[test]
fn merged_path_is_extended_from_the_end_that_owns_the_joined_endpoint() {
    // Benchy L33 sparse-infill regression (31.5 mm chord across the hull
    // interior). Three scan lines; the middle one (index 1) is first merged
    // with the short line of higher index 2 at the west wall (nearest pair,
    // 1 mm), so the merged path lives in the lower slot and starts with line 2's
    // hole-side end (6,4). The next join (east wall, 2 mm) pairs line 0's east
    // end with line 1's east end (20,3) — now the merged path's LAST point.
    // Extending the merged path from its other end instead splices a bare
    // chord (20,1)->(6,4) across the interior.
    //
    // Canonical `FillBase.cpp::connect_infill` resolves the owning polyline
    // with `get_and_update_merged_with` and orients it by comparing the
    // T-joint's contour point with `polyline.points.front()` / `.back()`, so
    // the connector is always the contour run between the two T-joints.
    let boundary = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(20.0, 0.0),
                Point2::from_mm(20.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(6.0, 3.5),
                Point2::from_mm(6.0, 8.0),
                Point2::from_mm(10.0, 8.0),
                Point2::from_mm(10.0, 3.5),
            ],
        }],
    };
    let scans = vec![
        segment((0.0, 1.0), (20.0, 1.0)),
        segment((0.0, 3.0), (20.0, 3.0)),
        segment((0.0, 4.0), (6.0, 4.0)),
    ];
    let graph = BoundaryInfillGraph::new(std::slice::from_ref(&boundary));
    let output = connect_infill(
        scans.clone(),
        &graph,
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 20.0,
        },
    );

    let shape = output
        .iter()
        .map(|p| p.points.iter().map(|q| (q.x, q.y)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let edges = ring_edges(&boundary);
    let scan_edges = scans
        .iter()
        .map(|s| {
            (
                (s.points[0].x, s.points[0].y),
                (s.points[1].x, s.points[1].y),
            )
        })
        .collect::<Vec<_>>();
    for path in &output {
        for pair in path.points.windows(2) {
            let (a, b) = ((pair[0].x, pair[0].y), (pair[1].x, pair[1].y));
            let on_scan = scan_edges.iter().any(|&(s, e)| {
                point_segment_distance_mm(a, s, e) < 1e-3
                    && point_segment_distance_mm(b, s, e) < 1e-3
            });
            let on_ring = (0..=10).all(|step| {
                let t = step as f32 / 10.0;
                let p = (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1));
                edges
                    .iter()
                    .any(|&(s, e)| point_segment_distance_mm(p, s, e) < 1e-3)
            });
            assert!(
                on_scan || on_ring,
                "connector {a:?}->{b:?} is neither a scan line nor a contour run; got {shape:?}"
            );
        }
    }
    assert_eq!(
        output.len(),
        1,
        "all three lines share the outer ring and link into one polyline; got {shape:?}"
    );
}
