//! AC-4: gap-fill emission contract (T-063/T-064/T-065, packet 105).
//!
//! Gap-fill is collected as an OrcaSlicer-parity port (diagnose 2026-06-24):
//! gaps are gathered INCREMENTALLY between consecutive perimeter insets and at
//! the final innermost-wall→infill transition. The final-transition gap is
//! `diff(offset(innermost, -0.5d), offset(infill_area, +0.5d))` where
//! `infill_area = offset(innermost, -inner_wall_line_width)`. This is ~empty for
//! WIDE regions (the infill fills the center, so the two offsets meet) and equals
//! the whole leftover core for THIN features where no infill line fits — exactly
//! the discriminator OrcaSlicer uses. It does NOT ring the outer region boundary,
//! so per-color MMU bisector edges produce no phantom slivers.
//!
//! Positive fixture: a 1.8 mm × 8 mm thin rectangle. With `wall_count = 2`,
//! `outer_wall_line_width = inner_wall_line_width = 0.4 mm` and no overlap
//! keys configured (code fallback `infill_wall_overlap = 0.0`):
//!
//! - Wall insets use Flow spacing per packet 185's D-105 closure:
//!   `spacing = line_width_to_spacing(0.4, 0.2) ≈ 0.3571 mm`. Outer wall at
//!   `0.5 × spacing`, inner wall at `0.5 × (ext_spacing + perimeter_spacing)`
//!   = spacing, leaving a core of `1.8 − 0.3571 − 2 × 0.3571 ≈ 0.7286 mm`.
//! - The infill inset is `spacing − infill_wall_overlap = 0.3571 − 0 = 0.3571`
//!   mm per side, so the infill region needs `2 × 0.3571 ≈ 0.7143` mm of
//!   core and the 0.7286 mm core fits: `offset(core, −0.3571)` still yields a
//!   hairline (~0.014 mm) strip. This hairline is CANONICAL (spacing-derived
//!   inset, not raw width) and its centroid lies inside the arm footprint.
//!
//! - The infill-transition gap is `diff(offset(core, −0.5×0.4), offset(infill, +0.5×0.4))`
//!   ≈ a 0.3 mm × 6.4 mm strip whose medial-axis spine (~6 mm) passes the
//!   0.5 mm length filter and is emitted as GapFill.
//!
//! The `no_gaps_case` test uses a clean 10 mm square: the infill fills the center,
//! the infill-transition gap is empty, and zero GapFill loops are emitted.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{mm_to_units, ExPolygon, ExtrusionRole, LoopType, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a 1.8 mm × 8 mm thin rectangle centered at the origin.
///
/// With `wall_count = 2` and `outer/inner_wall_line_width = 0.4 mm`:
/// after two wall insets (Flow-spacing based per packet 185's D-105 closure;
/// outer at ~0.179, inner at ~0.357, total ~0.536 mm per side) the core is
/// ≈0.73 mm × 6.93 mm. The infill inset is `spacing − overlap` ≈ 0.357 mm
/// per side, which needs 0.714 mm of core; the residual hairline infill
/// strip is ~0.014 mm wide — canonical (spacing-derived) and NOT an empty
/// infill region. The infill-transition gap collection yields a
/// ~0.3 mm × 6.4 mm strip (`offset(core, -0.2)` diffed against the grown
/// infill) that becomes the gap polygon. A WIDE region instead keeps a
/// non-empty infill area and produces no gap (see `no_gaps_case`), which is
/// the OrcaSlicer-parity discriminator.
fn make_thin_arm_region(z: f32) -> SliceRegionView {
    // CCW winding: BL → BR → TR → TL
    let poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-0.9, -4.0),
                Point2::from_mm(0.9, -4.0),
                Point2::from_mm(0.9, 4.0),
                Point2::from_mm(-0.9, 4.0),
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

/// AC-4: a 1.8 mm × 8 mm thin arm must produce ≥1 GapFill loop after
/// two wall insets leave a residual arm as a gap.
///
/// Config: `inner_wall_line_width = 0.4 mm`, `wall_count = 2`,
/// `gap_infill_speed = 30.0 mm/s`, `filter_out_gap_fill = 0.5 mm` (AC-4 value).
/// The medial-axis width floor is computed internally as
/// `inner_wall_line_width * 0.25 ≈ 0.1 mm`; the ~0.3 mm gap width passes.
/// The ~6.5 mm spine length exceeds the 0.5 mm length filter.
///
/// Assertions:
/// - At least one WallLoop with `loop_type == GapFill`.
/// - Every GapFill loop has `path.role == ExtrusionRole::GapFill`.
/// - GapFill widths vary along the path (medial-axis output, not constant).
/// - Every GapFill polyline's TOTAL length is ≥ 0.5 mm (AC-4 contract; this
///   mirrors the production filter, which sums segment lengths — it is NOT a
///   per-segment guarantee).
/// - `infill_areas` contains no polygon with a NON-HAIRLINE width whose
///   centroid lies inside the arm bounding box: since packet 185's D-105
///   closure the infill inset is Flow spacing minus overlap (0.3571 mm per
///   side at these values), which leaves a canonical hairline residual strip
///   (~0.014 mm) whose centroid sits inside the arm. A wide (>0.1 mm)
///   residual would still indicate the gap was NOT consumed by gap-fill.
#[test]
fn gap_fill_emitted_for_narrow_gap() {
    let inner_w = 0.4_f32;
    // Assertion threshold for TOTAL polyline length (AC-4 contract: 0.5 mm).
    let filter_mm = 0.5_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", inner_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .float("gap_infill_speed", 30.0)
        .float("filter_out_gap_fill", 0.5_f64)
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();

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
        "Expected ≥1 WallLoop with LoopType::GapFill for 1.9 mm × 8 mm arm fixture, got walls: {:?}",
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

        // The AC-4 contract threshold applies to the TOTAL polyline length, not to
        // individual segments. The production filter in `ClassicPerimeters`
        // (`modules/core-modules/classic-perimeters/src/lib.rs`) sums the segment
        // lengths of the medial axis and drops the polyline only when that TOTAL is
        // below `filter_out_gap_fill`. A per-segment assertion was over-strict:
        // canonical guarantees nothing about individual medial-axis segment lengths,
        // and a legitimate long spine routinely contains sub-0.5 mm segments.
        let pts = &gl.path.points;
        let total_len: f32 = pts
            .windows(2)
            .map(|w| {
                let dx = w[1].x - w[0].x;
                let dy = w[1].y - w[0].y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum();
        assert!(
            total_len >= filter_mm - 1e-4,
            "GapFill polyline total length {:.4} mm is below the {:.4} mm contract threshold",
            total_len,
            filter_mm
        );
    }

    // The gap must be consumed by gap-fill, not left as residual infill area.
    // For the 1.8 mm × 8 mm arm fixture: the ~0.73 mm core is below the full
    // 0.7143 mm spacing-derived infill inset, so the infill region collapses
    // to a canonical HAIRLINE residual (see the module docs) whose centroid
    // legitimately sits inside the arm footprint. We verify no *wide*
    // (> 0.1 mm, i.e. beyond the hairline) residual infill polygon centroid
    // falls inside the arm footprint.
    let arm_x_min = mm_to_units(-0.8);
    let arm_x_max = mm_to_units(0.8);
    let arm_y_min = mm_to_units(-4.1);
    let arm_y_max = mm_to_units(4.1);
    // Hairline residual width bound (mm): anything wider than this inside the
    // arm means a real infill region survived the gap-fill consumption.
    // Width proxy: the residual strip's minimum bounding-box extent, the same
    // measure the perimeter modules use for width classification.
    let hairline_max_width_mm = 0.1_f64;

    for call_areas in output.infill_areas() {
        for area in call_areas {
            if area.contour.points.is_empty() {
                continue;
            }
            let cx: i64 = area.contour.points.iter().map(|p| p.x).sum::<i64>()
                / area.contour.points.len() as i64;
            let cy: i64 = area.contour.points.iter().map(|p| p.y).sum::<i64>()
                / area.contour.points.len() as i64;
            let inside_arm =
                cx >= arm_x_min && cx <= arm_x_max && cy >= arm_y_min && cy <= arm_y_max;
            if !inside_arm {
                continue;
            }
            let mut min_x = i64::MAX;
            let mut max_x = i64::MIN;
            let mut min_y = i64::MAX;
            let mut max_y = i64::MIN;
            for p in &area.contour.points {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
            let width_mm = ((max_x - min_x) as f64).min((max_y - min_y) as f64) / 10_000.0;
            assert!(
                width_mm <= hairline_max_width_mm,
                "infill_area centroid ({}, {}) lies inside the arm region with width \
                 {width_mm:.4} mm > hairline bound {hairline_max_width_mm} mm — a real \
                 infill region survived; gap was not consumed",
                cx,
                cy
            );
        }
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

    let module = ClassicPerimeters::from_config(&config).unwrap();
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
