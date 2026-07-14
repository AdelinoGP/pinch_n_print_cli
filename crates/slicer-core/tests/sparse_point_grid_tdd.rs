#![allow(missing_docs)]

use slicer_core::arachne::sparse_point_grid::{Point2, SparsePointGrid};

#[test]
fn sparse_point_grid_returns_touched_cell_candidates() {
    let mut state = 0x5eed_5eed_5eed_5eed_u64;
    let mut points = Vec::with_capacity(100);
    for _ in 0..100 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let x = ((state >> 32) as u32) as f32 / u32::MAX as f32 * 10.0;
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let y = ((state >> 32) as u32) as f32 / u32::MAX as f32 * 10.0;
        points.push(Point2 { x, y });
    }

    let mut grid = SparsePointGrid::new(1.5, |point: &Point2| *point);
    for point in points.iter().copied() {
        grid.insert(point);
    }

    for query in points.iter().copied() {
        let nearby = grid.get_nearby(query, 1.5);
        // Candidate lookup must include every point in a touched cell. The
        // region-order predicate, not the grid, owns exact distance filtering.
        assert!(nearby.contains(&query));
    }

    let mut boundary_grid = SparsePointGrid::new(1.5, |point: &Point2| *point);
    let query = Point2 { x: 0.0, y: 0.0 };
    let candidate = Point2 { x: 2.0, y: 0.0 };
    boundary_grid.insert(query);
    boundary_grid.insert(candidate);
    assert!(
        boundary_grid.get_nearby(query, 1.5).contains(&candidate),
        "candidate in a touched cell must not be exact-distance filtered by the grid"
    );
}

#[test]
fn sparse_point_grid_single_insert_get_nearby_self() {
    let point = Point2 { x: 1.0, y: 2.0 };
    let mut grid = SparsePointGrid::new(1.0, |point: &Point2| *point);
    grid.insert(point);

    assert_eq!(grid.get_nearby(point, 0.5), vec![point]);
    assert_eq!(grid.get_nearby(point, 5.0), vec![point]);
}
