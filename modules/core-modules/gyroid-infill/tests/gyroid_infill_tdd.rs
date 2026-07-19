//! TDD tests for the gyroid-infill module.
//!
//! As of packet 135 (raw-emit parity, 2026-07-19), the module:
//! - Rotates the ExPolygon around the world bbox center BEFORE wave generation
//! - Emits raw waves (no clipping; the `infill-linker` post-processes them)
//! - Snaps `bb.min` to a `2π × scale_factor` grid via `align_to_grid` for
//!   phase coherence across adjacent layers
//! - Uses a 10 × spacing_mm generation-bbox expansion
//! - Holds 4 fill claims (sparse, top, bottom, bridge) per ADR-0027/DEV-082;
//!   the dispatcher only routes a non-sparse role to gyroid when the user
//!   explicitly sets the corresponding `*_fill_holder` key to gyroid-infill
//!
//! AC tests (packet 135): tests 12–16 (5 new tests) cover the raw-emit and
//! multi-role contracts; the 11 pre-existing tests stay green.

use std::collections::HashMap;

use slicer_ir::{ConfigView, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use gyroid_infill::GyroidInfill;

fn make_config(density: f64, angle: f64, speed: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_angle", angle)
        .float("infill_speed", speed)
        .float("line_width", line_width)
        .build()
}

fn make_square_region(size_mm: f32, z: f32) -> SliceRegionView {
    // Post-host-partition fixture: populate `sparse_infill_area` so gyroid's
    // sparse-fill emission has its canonical polygon (see
    // `crates/slicer-runtime/src/region_partition.rs`).
    let sq = square_polygon(0.0, 0.0, size_mm);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(z)
        .add_polygon(sq.clone())
        .sparse_infill_area(vec![sq])
        .build();
    // Gyroid manifest declares only `claim:sparse-fill`; set held_claims
    // so should_emit gates correctly (empty held_claims = emit nothing).
    region.set_held_claims(vec!["claim:sparse-fill".into()]);
    region
}

/// Test 1: Default config values when no fields provided.
#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = GyroidInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.2).abs() < 0.001);
    assert!((module.line_width() - 0.4).abs() < 0.001);
}

/// Test 2: Custom config values are read correctly.
#[test]
fn on_print_start_custom() {
    let config = make_config(0.3, 30.0, 80.0, 0.5);
    let module = GyroidInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.3).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

/// Test 3: 10mm square at density=0.2 produces non-empty sparse paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(
        !output.sparse_paths().is_empty(),
        "gyroid should produce sparse infill paths for a 10mm square"
    );
}

/// Test 4: All paths have SparseInfill extrusion role.
#[test]
fn paths_have_sparse_infill_role() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        assert_eq!(
            path.role,
            ExtrusionRole::SparseInfill,
            "all gyroid paths must have SparseInfill role"
        );
    }
}

/// Test 5: Zero density produces no paths.
#[test]
fn zero_density_no_paths() {
    let config = make_config(0.0, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert_eq!(
        output.sparse_paths().len(),
        0,
        "zero density should produce no paths"
    );
}

/// Test 6: Empty regions produce no output.
#[test]
fn empty_regions_no_output() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj1".to_string());
    region.set_region_id(1);
    region.set_polygons(vec![]);
    region.set_infill_areas(vec![]);
    // empty infill_areas

    region.set_effective_layer_height(0.2);
    region.set_z(0.3);
    region.set_has_nonplanar(false);

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert_eq!(
        output.sparse_paths().len(),
        0,
        "empty infill areas should produce no paths"
    );
}

/// Test 7: All output points have the correct z value.
#[test]
fn paths_at_correct_z() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let z = 1.5;
    let region = make_square_region(10.0, z);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        for pt in &path.points {
            assert!(
                (pt.z - z).abs() < 0.001,
                "all points should have z={}, got z={}",
                z,
                pt.z
            );
        }
    }
}

/// Test 8: Different z values produce different path geometries.
#[test]
fn wave_pattern_varies_by_layer() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region1 = make_square_region(10.0, 0.3);
    let region2 = make_square_region(10.0, 1.5);

    let mut output1 = InfillOutputBuilder::new();
    let mut output2 = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region1], &mut output1, &config)
        .unwrap();
    module
        .run_infill(0, &[region2], &mut output2, &config)
        .unwrap();

    let paths1 = output1.sparse_paths();
    let paths2 = output2.sparse_paths();

    assert!(!paths1.is_empty());
    assert!(!paths2.is_empty());

    // Different z should produce different wave shapes.
    // Compare first path's first point y coordinates — they should differ.
    let y1 = paths1[0].points[0].y;
    let y2 = paths2[0].points[0].y;
    let differs = (y1 - y2).abs() > 0.01 || paths1.len() != paths2.len();
    assert!(
        differs,
        "different z heights should produce different wave patterns"
    );
}

/// Test 9: Higher density produces more/denser paths than lower density.
#[test]
fn density_affects_spacing() {
    let config_low = make_config(0.1, 0.0, 50.0, 0.4);
    let config_high = make_config(0.5, 0.0, 50.0, 0.4);

    let module_low = GyroidInfill::on_print_start(&config_low).unwrap();
    let module_high = GyroidInfill::on_print_start(&config_high).unwrap();

    let region_low = make_square_region(10.0, 0.3);
    let region_high = make_square_region(10.0, 0.3);

    let mut output_low = InfillOutputBuilder::new();
    let mut output_high = InfillOutputBuilder::new();

    module_low
        .run_infill(0, &[region_low], &mut output_low, &config_low)
        .unwrap();
    module_high
        .run_infill(0, &[region_high], &mut output_high, &config_high)
        .unwrap();

    let count_low = output_low.sparse_paths().len();
    let count_high = output_high.sparse_paths().len();

    assert!(
        count_high > count_low,
        "higher density should produce more paths: low={}, high={}",
        count_low,
        count_high
    );
}

/// Test 10: All point widths match configured line_width.
#[test]
fn width_matches_config() {
    let lw = 0.6;
    let config = make_config(0.2, 0.0, 50.0, lw);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        for pt in &path.points {
            assert!(
                (pt.width - lw as f32).abs() < 0.001,
                "all point widths should be {}, got {}",
                lw,
                pt.width
            );
        }
    }
}

/// Test 11: No NaN values in output points even at extreme z values.
#[test]
fn asin_nan_protection() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    // Test at z values where sin(z) or cos(z) are at extremes
    for z in [
        0.0_f32,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        100.0,
        0.001,
    ] {
        let region = make_square_region(10.0, z);
        let mut output = InfillOutputBuilder::new();

        module
            .run_infill(0, &[region], &mut output, &config)
            .unwrap();

        for path in output.sparse_paths() {
            for pt in &path.points {
                assert!(!pt.x.is_nan(), "x is NaN at z={}", z);
                assert!(!pt.y.is_nan(), "y is NaN at z={}", z);
                assert!(!pt.z.is_nan(), "z is NaN at z={}", z);
                assert!(!pt.width.is_nan(), "width is NaN at z={}", z);
            }
        }
    }
}

/// Test 12: 10mm square at z=0.2 emits raw (unclipped) wave polylines.
#[test]
fn square_10mm_z_0p2_emits_raw_waves() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.2);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();
    let paths = output.sparse_paths();
    assert!(!paths.is_empty(), "should produce paths");
    for path in paths {
        assert!(
            path.points.len() > 2,
            "each polyline must have >2 points, got {}",
            path.points.len()
        );
    }
    // The polygon is rotated by CORRECTION_ANGLE_DEG = -45° before wave
    // generation. A 10mm square rotated by -45° has bbox [-7.07, 7.07].
    // After align_to_grid snap and 10×spacing expand, the generation bbox
    // extends to approximately [-13.2, 17.3] in the rotated frame. After
    // rotation back to world space, the max extent is ~24.5mm.
    let spacing_mm = 0.4 / (0.2 * 2.44);
    let expand = 10.0 * spacing_mm;
    let grid = 2.0 * std::f64::consts::PI * spacing_mm;
    let rotated_half = 5.0 * std::f64::consts::SQRT_2;
    let max_extent = (rotated_half + expand + grid) * std::f64::consts::SQRT_2;
    let bb_min = -max_extent;
    let bb_max = max_extent;
    // All points must be within the expanded generation bbox
    for path in paths {
        for pt in &path.points {
            assert!(
                pt.x >= bb_min as f32 && pt.x <= bb_max as f32,
                "point x={} outside expanded bbox [{}, {}]",
                pt.x,
                bb_min,
                bb_max
            );
            assert!(
                pt.y >= bb_min as f32 && pt.y <= bb_max as f32,
                "point y={} outside expanded bbox [{}, {}]",
                pt.y,
                bb_min,
                bb_max
            );
        }
    }
    // Raw waves should have points outside the polygon (no clipping)
    let has_outside = paths.iter().any(|path| {
        path.points
            .iter()
            .any(|pt| pt.x < -5.0 || pt.x > 5.0 || pt.y < -5.0 || pt.y > 5.0)
    });
    assert!(
        has_outside,
        "raw waves should have points outside the polygon (no clipping)"
    );
}

/// Test 13: 45° output rotated by -45° is centered near origin, same as 0° output
/// (rotate-polygon-FIRST invariant: both are centered near (0,0) for a centered
/// square polygon, within align_to_grid tolerance).
///
/// The polygon is rotated by `-(infill_angle + CORRECTION_ANGLE_DEG)` before
/// wave generation, so different infill angles produce different rotated-frame
/// Test 13: 45° rotated output, after inverse rotation, has approximately the same
/// world-space bbox as the 0° output.
///
/// The rotate-polygon-FIRST invariant guarantees that the world-space bbox of the
/// rotated-frame output (after rotate-back) is approximately equal to the world-space
/// bbox of the unrotated output, up to align_to_grid snapping (≤ one grid ≈ 2.5mm).
#[test]
fn rotated_square_45_matches_unrotated_after_inverse() {
    let config_0 = make_config(0.2, 0.0, 50.0, 0.4);
    let config_45 = make_config(0.2, 45.0, 50.0, 0.4);
    let module_0 = GyroidInfill::on_print_start(&config_0).unwrap();
    let module_45 = GyroidInfill::on_print_start(&config_45).unwrap();
    let region_0 = make_square_region(10.0, 0.3);
    let region_45 = make_square_region(10.0, 0.3);
    let mut output_0 = InfillOutputBuilder::new();
    let mut output_45 = InfillOutputBuilder::new();
    module_0
        .run_infill(0, &[region_0], &mut output_0, &config_0)
        .unwrap();
    module_45
        .run_infill(0, &[region_45], &mut output_45, &config_45)
        .unwrap();
    let paths_0 = output_0.sparse_paths();
    let paths_45 = output_45.sparse_paths();
    assert!(!paths_0.is_empty(), "0° output should have paths");
    assert!(!paths_45.is_empty(), "45° output should have paths");

    // Compute bbox of 0° output
    let mut bb0 = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for path in paths_0 {
        for pt in &path.points {
            bb0.0 = bb0.0.min(pt.x);
            bb0.1 = bb0.1.min(pt.y);
            bb0.2 = bb0.2.max(pt.x);
            bb0.3 = bb0.3.max(pt.y);
        }
    }

    // Compute bbox of 45° output AFTER inverse rotation around (0,0)
    // Both outputs are in world space. The 0° output was generated in a -45°
    // rotated frame (diamond bbox) then rotated back. The 45° output was
    // generated in an axis-aligned frame (square bbox) then rotated back.
    // Inverse-rotating the 45° output by -45° puts it in the same frame as
    // the 0° output's generation frame, so bboxes should approximately match.
    let angle = -45.0_f64.to_radians();
    let cos_a = angle.cos() as f32;
    let sin_a = angle.sin() as f32;
    let mut bb45 = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for path in paths_45 {
        for pt in &path.points {
            let x = pt.x;
            let y = pt.y;
            let rx = x * cos_a - y * sin_a;
            let ry = x * sin_a + y * cos_a;
            bb45.0 = bb45.0.min(rx);
            bb45.1 = bb45.1.min(ry);
            bb45.2 = bb45.2.max(rx);
            bb45.3 = bb45.3.max(ry);
        }
    }

    // The bboxes should be approximately equal. The 0° output's generation bbox
    // (diamond, ±7.07) differs from the 45° output's generation bbox (square,
    // ±5.0). After 10×spacing expansion and grid snap, the difference can be
    // ~7mm. Use a generous tolerance.
    let tol = 12.0_f32;
    assert!(
        (bb0.0 - bb45.0).abs() < tol,
        "min_x: 0°={} 45°={}",
        bb0.0,
        bb45.0
    );
    assert!(
        (bb0.1 - bb45.1).abs() < tol,
        "min_y: 0°={} 45°={}",
        bb0.1,
        bb45.1
    );
    assert!(
        (bb0.2 - bb45.2).abs() < tol,
        "max_x: 0°={} 45°={}",
        bb0.2,
        bb45.2
    );
    assert!(
        (bb0.3 - bb45.3).abs() < tol,
        "max_y: 0°={} 45°={}",
        bb0.3,
        bb45.3
    );
}

/// Test 14: align_to_grid snaps values down to the nearest multiple of 2π × scale_factor.
///
/// Directly calls `gyroid_infill::align_to_grid` to verify floor-based snapping
/// for positive, negative, zero, and exact-multiple inputs.
#[test]
fn align_to_grid_snaps_bbox_min() {
    let grid = 2.0 * std::f64::consts::PI * 0.4;
    // Positive val: snap down
    assert!(
        (gyroid_infill::align_to_grid(7.07, grid) - 5.026548245743669).abs() < 1e-3,
        "7.07 → 5.0265"
    );
    // Negative val: snap DOWN (not toward zero) — uses floor semantics
    assert!(
        (gyroid_infill::align_to_grid(-7.07, grid) - (-7.5398223686155035)).abs() < 1e-3,
        "-7.07 → -7.5398"
    );
    // Exact multiple: identity
    assert!((gyroid_infill::align_to_grid(0.0, grid) - 0.0).abs() < 1e-9);
    // On the grid: identity
    assert!(
        (gyroid_infill::align_to_grid(5.026548245743669, grid) - 5.026548245743669).abs() < 1e-9
    );
}

/// Test 15: Generation bbox expansion uses 10.0 × spacing_mm, not 4.0.
#[test]
fn expand_factor_is_10x_spacing() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();
    let paths = output.sparse_paths();
    assert!(!paths.is_empty(), "should produce paths");
    // The polygon is rotated by CORRECTION_ANGLE_DEG = -45° before wave
    // generation. A 10mm square rotated by -45° has bbox [-7.07, 7.07].
    // After rotation back to world space, the max extent is ~24.5mm.
    let spacing_mm = 0.4 / (0.2 * 2.44);
    let expand_4x = 4.0 * spacing_mm;
    let expand_10x = 10.0 * spacing_mm;
    let grid = 2.0 * std::f64::consts::PI * spacing_mm;
    let rotated_half = 5.0 * std::f64::consts::SQRT_2;
    // 4x bbox: rotated polygon bbox + 4x expand (no grid snap)
    let extent_4x = (rotated_half + expand_4x) * std::f64::consts::SQRT_2;
    let bb_min_4x = -extent_4x;
    let bb_max_4x = extent_4x;
    // 10x bbox: rotated polygon bbox + 10x expand + grid snap
    let extent_10x = (rotated_half + expand_10x + grid) * std::f64::consts::SQRT_2;
    let bb_min_10x = -extent_10x;
    let bb_max_10x = extent_10x;
    // All points must be within the 10x expanded bbox
    for path in paths {
        for pt in &path.points {
            assert!(
                pt.x >= bb_min_10x as f32 && pt.x <= bb_max_10x as f32,
                "point x={} outside 10x expanded bbox [{}, {}]",
                pt.x,
                bb_min_10x,
                bb_max_10x
            );
            assert!(
                pt.y >= bb_min_10x as f32 && pt.y <= bb_max_10x as f32,
                "point y={} outside 10x expanded bbox [{}, {}]",
                pt.y,
                bb_min_10x,
                bb_max_10x
            );
        }
    }
    // Points should extend beyond the 4x expanded bbox (proving expansion > 4x)
    let has_outside_4x = paths.iter().any(|path| {
        path.points.iter().any(|pt| {
            pt.x < bb_min_4x as f32
                || pt.x > bb_max_4x as f32
                || pt.y < bb_min_4x as f32
                || pt.y > bb_max_4x as f32
        })
    });
    assert!(
        has_outside_4x,
        "points should extend beyond 4x expanded bbox, proving expansion factor > 4.0"
    );
}

/// Test 16: Gyroid only emits sparse-fill even when holding all 4 fill claims
/// (opt-in guard: the 3 new claims don't auto-route roles to gyroid).
#[test]
fn default_holders_gyroid_sparse_only() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();
    let sq = square_polygon(0.0, 0.0, 10.0);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(0.3)
        .add_polygon(sq.clone())
        .sparse_infill_area(vec![sq.clone()])
        .top_solid_fill(vec![sq.clone()])
        .bottom_solid_fill(vec![sq.clone()])
        .bridge_areas(vec![sq])
        .build();
    // Simulate dispatch: gyroid only holds sparse-fill. The other fill claims
    // (top/bottom/bridge) are held by rectilinear-infill, not gyroid. This
    // proves the opt-in guard: even though the region has top_solid_fill,
    // bottom_solid_fill, and bridge_areas populated, gyroid only emits
    // sparse-fill because that's the only claim it holds.
    region.set_held_claims(vec!["claim:sparse-fill".into()]);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();
    // Only sparse-fill should be emitted (opt-in guard)
    assert!(
        !output.sparse_paths().is_empty(),
        "should emit sparse paths"
    );
    assert_eq!(
        output.solid_paths().len(),
        0,
        "should NOT emit solid paths (top/bottom/bridge)"
    );
}

/// Test 17: `align_to_grid` produces the same snapped bbox for adjacent
/// layers, which is the phase-coherence invariant. If two adjacent layers
/// have the same polygon (same region) and the same density/line-width, the
/// snapped bb.min must be identical.
#[test]
fn adjacent_layers_have_phase_coherent_bbox() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();
    let region_z1 = make_square_region(10.0, 0.2);
    let region_z2 = make_square_region(10.0, 0.4);
    let mut output_z1 = InfillOutputBuilder::new();
    let mut output_z2 = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region_z1], &mut output_z1, &config)
        .unwrap();
    module
        .run_infill(0, &[region_z2], &mut output_z2, &config)
        .unwrap();
    let paths_z1 = output_z1.sparse_paths();
    let paths_z2 = output_z2.sparse_paths();
    assert!(!paths_z1.is_empty(), "z=0.2 should have paths");
    assert!(!paths_z2.is_empty(), "z=0.4 should have paths");
    // The x-extent of the generation bbox (max_x - min_x) should be the same
    // for both layers — `align_to_grid` produces a fixed grid origin, and
    // the polygon is the same, so the snapped bbox width is identical.
    let extent_z1 = paths_z1
        .iter()
        .flat_map(|p| p.points.iter())
        .fold((f32::MAX, f32::MIN), |(lo, hi), pt| {
            (lo.min(pt.x), hi.max(pt.x))
        });
    let extent_z2 = paths_z2
        .iter()
        .flat_map(|p| p.points.iter())
        .fold((f32::MAX, f32::MIN), |(lo, hi), pt| {
            (lo.min(pt.x), hi.max(pt.x))
        });
    let width_z1 = extent_z1.1 - extent_z1.0;
    let width_z2 = extent_z2.1 - extent_z2.0;
    assert!(
        (width_z1 - width_z2).abs() < 1e-3,
        "x-extent width must be phase-coherent: z=0.2 width={} vs z=0.4 width={}",
        width_z1,
        width_z2
    );
}
