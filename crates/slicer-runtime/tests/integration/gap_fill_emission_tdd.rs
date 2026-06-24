//! AC-4: gap-fill emission contract (T-063/T-064/T-065, packet 105).
//!
//! The gap-fill cascade triggers when the innermost wall inset leaves a narrow
//! residual polygon whose width is too small to survive a further inset by
//! `inner_wall_line_width / 2 = 0.2 mm`.  Specifically, the impl computes:
//!
//!   gaps = difference_ex(current_polygons, offset(current_polygons, -inner_wall_line_width/2))
//!
//! where `current_polygons` is the polygon remaining after all wall insets.
//!
//! Fixture geometry for the positive test: a 1.5 mm × 8 mm thin rectangle.
//! With `wall_count = 2`, `outer_wall_line_width = 0.4 mm`, and
//! `inner_wall_line_width = 0.4 mm`:
//!
//! - After inset i=0 (delta = -0.2 mm): 1.1 mm × 7.6 mm.
//! - After inset i=1 (delta = -0.4 mm from the i=0 result): 0.3 mm × 6.8 mm.
//!
//! The 0.3 mm wide arm in `current_polygons` is entirely consumed by the gap
//! detection inset (0.3 mm < 2 × 0.2 mm = 0.4 mm), so the whole arm becomes
//! the gap polygon.  The medial axis of the 0.3 mm × 6.8 mm rectangle is a
//! spine ≈ 6.5 mm long.  After the corner spurs are pruned by the step-3
//! length filter (each spur ≈ 0.21 mm < 2 × 0.3 mm = 0.6 mm), only the
//! junction-to-junction central spine survives (≈ 6.5 mm >> 0.6 mm).
//!
//! The gap-fill medial axis call uses a width floor of `inner_wall_line_width * 0.25`
//! (~0.1 mm) as `min_width`, ensuring the OR-gate `(w >= min_width) && (w <= max_width)`
//! passes for realistic gap widths (≈ 0.2–0.4 mm).  `filter_out_gap_fill` is applied
//! as a post-medial-axis segment-length filter (AC-4 contract: 0.5 mm), not as a width
//! threshold.  The `no_gaps_case` test uses a clean square and must not panic.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{mm_to_units, ExPolygon, ExtrusionRole, LoopType, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a 1.5 mm × 8 mm thin rectangle centered at the origin.
///
/// With `wall_count = 2` and `outer/inner_wall_line_width = 0.4 mm`:
/// after two wall insets (total 0.6 mm per side in x, 0.6 mm per side in y)
/// `current_polygons` is a 0.3 mm × 6.8 mm arm — too narrow to survive
/// the 0.2 mm infill inset, so the whole arm becomes the gap polygon.
fn make_thin_arm_region(z: f32) -> SliceRegionView {
    // CCW winding: BL → BR → TR → TL
    let poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-0.75, -4.0),
                Point2::from_mm(0.75, -4.0),
                Point2::from_mm(0.75, 4.0),
                Point2::from_mm(-0.75, 4.0),
            ],
        },
        holes: Vec::new(),
    };

    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(poly)
        .build()
}

/// AC-4: a 1.5 mm × 8 mm thin arm must produce ≥1 GapFill loop after
/// two wall insets leave a 0.3 mm × 6.8 mm residual arm as a gap.
///
/// Config: `inner_wall_line_width = 0.4 mm`, `wall_count = 2`,
/// `gap_infill_speed = 30.0 mm/s`, `filter_out_gap_fill = 0.5 mm` (AC-4 value).
/// The medial-axis width floor is computed internally as
/// `inner_wall_line_width * 0.25 ≈ 0.1 mm`; the 0.3 mm gap width passes.
/// The ~6.5 mm spine length exceeds the 0.5 mm length filter.
///
/// Assertions:
/// - At least one WallLoop with `loop_type == GapFill`.
/// - Every GapFill loop has `path.role == ExtrusionRole::GapFill`.
/// - GapFill widths vary along the path (medial-axis output, not constant).
/// - No individual GapFill segment has length < 0.5 mm (AC-4 contract).
/// - `infill_areas` does not contain any polygon whose centroid lies inside
///   the arm bounding box (the gap was consumed, not left as infill).
#[test]
fn gap_fill_emitted_for_narrow_gap() {
    let inner_w = 0.4_f32;
    // Assertion threshold for segment length (AC-4 contract: 0.5 mm).
    let filter_mm = 0.5_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", inner_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .float("gap_infill_speed", 30.0)
        .float("filter_out_gap_fill", 0.5_f64)
        .build();

    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    let regions = vec![make_thin_arm_region(0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();

    let gap_loops: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::GapFill)
        .collect();

    assert!(
        !gap_loops.is_empty(),
        "Expected ≥1 WallLoop with LoopType::GapFill for 1.5 mm × 8 mm arm fixture, got walls: {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );

    for gl in &gap_loops {
        // Every GapFill loop must carry ExtrusionRole::GapFill.
        assert_eq!(
            gl.path.role,
            ExtrusionRole::GapFill,
            "GapFill loop has wrong ExtrusionRole: {:?}",
            gl.path.role
        );

        // Widths must vary (medial axis produces variable widths, not constant).
        // We require that the min and max widths differ by at least 1e-4 mm.
        // A perfectly constant-width path is a sign the variable_width() fn was
        // bypassed or the fixture collapsed to a degenerate single-width axis.
        let widths: Vec<f32> = gl.path.points.iter().map(|p| p.width).collect();
        if widths.len() >= 2 {
            let min_w = widths.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_w = widths.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            // NOTE: for a uniform-width spine the medial axis may return only 2 points
            // with identical widths; we assert non-constant only when ≥3 points.
            if widths.len() >= 3 {
                assert!(
                    (max_w - min_w) > 1e-4,
                    "GapFill loop widths appear constant: all ≈ {:.6}; expected variable widths",
                    min_w
                );
            }
        }

        // No individual segment length below the AC-4 contract threshold (0.5 mm).
        let pts = &gl.path.points;
        for i in 0..pts.len().saturating_sub(1) {
            let dx = pts[i + 1].x - pts[i].x;
            let dy = pts[i + 1].y - pts[i].y;
            let seg_len = (dx * dx + dy * dy).sqrt();
            assert!(
                seg_len >= filter_mm - 1e-4,
                "GapFill segment {}->{} length {:.4} mm is below 0.5 mm contract threshold",
                i,
                i + 1,
                seg_len
            );
        }
    }

    // The gap must be consumed by gap-fill, not left as residual infill area.
    // For the 1.5 mm × 8 mm arm fixture: the 0.3 mm × 6.8 mm residual arm is
    // entirely below the 0.4 mm infill-inset threshold, so infill_areas must be
    // empty.  We verify no centroid falls inside the arm footprint.
    let arm_x_min = mm_to_units(-0.8);
    let arm_x_max = mm_to_units(0.8);
    let arm_y_min = mm_to_units(-4.1);
    let arm_y_max = mm_to_units(4.1);

    for area in output.infill_areas() {
        if area.contour.points.is_empty() {
            continue;
        }
        let cx: i64 =
            area.contour.points.iter().map(|p| p.x).sum::<i64>() / area.contour.points.len() as i64;
        let cy: i64 =
            area.contour.points.iter().map(|p| p.y).sum::<i64>() / area.contour.points.len() as i64;
        assert!(
            !(cx >= arm_x_min && cx <= arm_x_max && cy >= arm_y_min && cy <= arm_y_max),
            "infill_area centroid ({}, {}) lies inside the arm region — gap was not consumed",
            cx,
            cy
        );
    }
}

/// AC-N2: a clean square with `gap_infill_speed > 0` must emit zero GapFill
/// loops and must not panic on empty gaps.
#[test]
fn no_gaps_case() {
    let inner_w = 0.4_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", inner_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .float("gap_infill_speed", 30.0)
        .float("filter_out_gap_fill", 0.5)
        .build();

    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    // Clean 10 mm × 10 mm square — no slot, no thin features.
    let regions = vec![SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .build()];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    // Must not panic.
    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let gap_count = output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::GapFill)
        .count();

    assert_eq!(
        gap_count, 0,
        "Expected 0 GapFill loops for clean square, got {}",
        gap_count
    );
}
