//! TDD test for arachne per-vertex parity packet 148, AC-6.
//!
//! `run_perimeters` must emit sharp-corner seam candidates for each region's
//! outer contour, exactly once per region, via
//! `slicer_core::perimeter_utils::generate_sharp_corner_seam_candidates`
//! (mirrors `classic-perimeters`' own seam-candidate emission, lib.rs
//! ~889-900). Each candidate's position must coincide with one of the input
//! polygon's corner points (a square has four 90-degree corners, all sharper
//! than the default 30-degree threshold).

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{units_to_mm, ConfigView};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("inner_wall_line_width", line_width_mm as f64)
        .float("outer_wall_line_width", line_width_mm as f64)
        .build()
}

/// 10mm square region (centered at origin, per `square_polygon`'s own
/// center-based convention).
fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

#[test]
fn seam_candidates_emitted_at_input_polygon_corners() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let candidates = output.seam_candidates();
    assert!(
        !candidates.is_empty(),
        "expected at least one seam candidate for a square's sharp corners"
    );

    // square_polygon(0.0, 0.0, 10.0) corners, in mm.
    let corners_mm: Vec<(f32, f32)> = square_polygon(0.0, 0.0, 10.0)
        .contour
        .points
        .iter()
        .map(|p| (units_to_mm(p.x), units_to_mm(p.y)))
        .collect();

    for (pos, _score) in candidates {
        let is_at_corner = corners_mm
            .iter()
            .any(|(cx, cy)| (pos.x - cx).abs() < 1e-3 && (pos.y - cy).abs() < 1e-3);
        assert!(
            is_at_corner,
            "seam candidate at ({}, {}) mm does not match any input polygon corner {:?}",
            pos.x, pos.y, corners_mm
        );
    }
}

/// Two disjoint 10mm square islands (same region, side by side with a gap)
/// must each contribute seam candidates. Mirrors classic-perimeters'
/// per-island loop (`for (poly_idx, poly) in outer_polys.iter().enumerate()`,
/// lib.rs ~888-902): `run_perimeters` must not stop at the region's first
/// polygon (`polygons[0]`) — every island's sharp corners are candidates.
#[test]
fn seam_candidates_emitted_for_every_island_in_a_multi_island_region() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();

    // Two 10mm squares, centers 30mm apart on X, leaving a 20mm gap between
    // their facing edges (5mm half-width each) — comfortably disjoint.
    let island_a = square_polygon(0.0, 0.0, 10.0);
    let island_b = square_polygon(30.0, 0.0, 10.0);

    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(island_a.clone())
        .add_polygon(island_b.clone())
        .build();

    let regions = vec![region];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let candidates = output.seam_candidates();
    assert!(
        !candidates.is_empty(),
        "expected seam candidates for a two-island region's sharp corners"
    );

    let corners_mm = |poly: &slicer_ir::ExPolygon| -> Vec<(f32, f32)> {
        poly.contour
            .points
            .iter()
            .map(|p| (units_to_mm(p.x), units_to_mm(p.y)))
            .collect()
    };
    let corners_a = corners_mm(&island_a);
    let corners_b = corners_mm(&island_b);

    let has_candidate_near = |corners: &[(f32, f32)]| {
        candidates.iter().any(|(pos, _score)| {
            corners
                .iter()
                .any(|(cx, cy)| (pos.x - cx).abs() < 1e-3 && (pos.y - cy).abs() < 1e-3)
        })
    };

    assert!(
        has_candidate_near(&corners_a),
        "expected at least one seam candidate matching island A's corners {:?}, got {:?}",
        corners_a,
        candidates
    );
    assert!(
        has_candidate_near(&corners_b),
        "expected at least one seam candidate matching island B's corners {:?}, got {:?}",
        corners_b,
        candidates
    );

    // Every candidate must coincide with a corner of one of the two islands
    // (no fabricated positions).
    for (pos, _score) in candidates {
        let is_at_corner = corners_a
            .iter()
            .chain(corners_b.iter())
            .any(|(cx, cy)| (pos.x - cx).abs() < 1e-3 && (pos.y - cy).abs() < 1e-3);
        assert!(
            is_at_corner,
            "seam candidate at ({}, {}) mm does not match any island corner (A={:?}, B={:?})",
            pos.x, pos.y, corners_a, corners_b
        );
    }
}
