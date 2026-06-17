// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! Phase 5 — Width limiting and interlocking for paint-segmentation.
//!
//! Ports OrcaSlicer's `cut_segmented_layers` (MultiMaterialSegmentation.cpp:1294).
//!
//! # Coordinate system
//! 1 unit = 100 nm = 1e-4 mm. `UNITS_PER_MM = 10_000.0`.

use crate::polygon_ops::{difference_ex, offset, OffsetJoinType};
use slicer_ir::ExPolygon;
use std::collections::BTreeMap;

use super::compose_variants::ChainKey;
use super::PaintSegmentationError;

/// Width-limit and interlocking kernel for paint-segmentation Phase 5.
///
/// Ports OrcaSlicer's `cut_segmented_layers` from `MultiMaterialSegmentation.cpp:1294`.
///
/// The caller (`execute_paint_segmentation`) is expected to guard this call with
/// `!interlocking_beam` — this kernel does NOT handle the interlocking-beam path.
///
/// # Per-layer depth rule (parity with `MultiMaterialSegmentation.cpp:1294`)
///
/// For each layer at index `layer_idx`:
/// - If `interlocking_depth_units != 0` **and** `layer_idx % 2 == 0`:
///   use `interlocking_depth_units` as the erosion depth (standalone, NOT additive).
/// - Otherwise: use `region_width_units`.
///
/// # Sign convention
/// A negative `delta_mm` passed to `offset` erodes inward.
/// `depth_units` → `delta_mm = -(depth_units as f32) / 10_000.0`.
///
/// # Length mismatch
/// If `input_expolygons_per_layer.len() != variants_per_layer.len()`, the
/// shorter length is used; excess layers in the longer slice are left unmodified.
pub fn cut_segmented_layers(
    variants_per_layer: &mut [BTreeMap<ChainKey, Vec<ExPolygon>>],
    input_expolygons_per_layer: &[Vec<ExPolygon>],
    region_width_units: i64,
    interlocking_depth_units: i64,
) -> Result<(), PaintSegmentationError> {
    // Validate: negative config values are rejected (AC-N1)
    if region_width_units < 0 {
        return Err(PaintSegmentationError::InvalidPhase5Config {
            key: "mmu_segmented_region_max_width".to_string(),
            value: region_width_units,
        });
    }
    if interlocking_depth_units < 0 {
        return Err(PaintSegmentationError::InvalidPhase5Config {
            key: "mmu_segmented_region_interlocking_depth".to_string(),
            value: interlocking_depth_units,
        });
    }

    // Short-circuit: both zero means no width limiting to apply
    if region_width_units == 0 && interlocking_depth_units == 0 {
        return Ok(());
    }

    // Process layers up to the shorter length to handle mismatched inputs
    let n_layers = variants_per_layer
        .len()
        .min(input_expolygons_per_layer.len());

    for layer_idx in 0..n_layers {
        // Per-layer depth rule (MultiMaterialSegmentation.cpp:1294):
        // Even layers use interlocking_depth when nonzero; all others use region_width.
        // Note: interlocking_depth is standalone (not additive with region_width).
        let depth_units = if layer_idx % 2 == 0 && interlocking_depth_units != 0 {
            interlocking_depth_units
        } else {
            region_width_units
        };

        // Skip layer if no erosion applies at this depth
        if depth_units == 0 {
            continue;
        }

        let layer_input = &input_expolygons_per_layer[layer_idx];
        // Convert depth to mm and negate for inward erosion
        let delta_mm = -(depth_units as f32) / 10_000.0;
        let inner = offset(layer_input, delta_mm, OffsetJoinType::Miter, 0.01);

        let layer_variants = &mut variants_per_layer[layer_idx];
        for (chain, expolys) in layer_variants.iter_mut() {
            // Skip base/unpainted area (empty chain key) — base region is left unchanged
            if chain.is_empty() {
                continue;
            }
            // Clip variant to the border band by subtracting the inward-eroded interior.
            // D15: entries remain in the map even when polygons become empty.
            *expolys = difference_ex(expolys, &inner);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ExPolygon, PaintValue, Point2, Polygon};

    // ---------------------------------------------------------------------------
    // Construction helpers
    // ---------------------------------------------------------------------------

    fn make_expoly(pts: &[(i64, i64)]) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
            },
            holes: Vec::new(),
        }
    }

    /// Axis-aligned square from (x0, y0) to (x1, y1).
    fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        make_expoly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    /// Non-empty ChainKey with a single ToolIndex entry (painted, non-base).
    fn painted_chain(tool: u32) -> ChainKey {
        vec![("material".to_string(), PaintValue::ToolIndex(tool))]
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    /// 1. Width-only erosion (no interlocking): a variant covering the full input
    ///    square erodes to a border band; a variant fully inside the inner region
    ///    disappears.
    #[test]
    fn width_limit_only_no_interlocking_erodes_to_band() {
        // 10 mm × 10 mm input at origin (100_000 units per side).
        // Width = 2 mm = 20_000 units → inner = [20_000..80_000]²
        // chain(1): full 10 mm square → difference yields 2 mm border band (non-empty)
        // chain(2): 4 mm square at [30_000..70_000]² fully inside inner → empty
        let input_sq = square(0, 0, 100_000, 100_000);
        let mut layer_map = BTreeMap::new();
        layer_map.insert(painted_chain(1), vec![square(0, 0, 100_000, 100_000)]);
        layer_map.insert(
            painted_chain(2),
            vec![square(30_000, 30_000, 70_000, 70_000)],
        );

        let mut variants = vec![layer_map];
        let inputs = vec![vec![input_sq]];

        let res = cut_segmented_layers(&mut variants, &inputs, 20_000, 0);
        assert!(res.is_ok());

        // Border-band variant should survive erosion
        assert!(
            !variants[0][&painted_chain(1)].is_empty(),
            "rim variant should yield a non-empty border band after 2 mm erosion"
        );
        // Interior variant (fully inside eroded inner) should disappear
        assert!(
            variants[0][&painted_chain(2)].is_empty(),
            "center variant fully inside the inner region should become empty"
        );
    }

    /// 2. Interlocking alternation: even layers use `interlocking_depth` (D), odd
    ///    layers use `region_width` (W). A variant placed in the D-band-only zone
    ///    (outside D-inner but inside W-inner) survives on even layers and disappears
    ///    on odd layers when D ≠ W.
    #[test]
    fn interlocking_alternates_when_depth_nonzero() {
        // Input: 10 mm square [0..100_000]²
        // W = 1 mm = 10_000 → W-inner = [10_000..90_000]²
        // D = 4 mm = 40_000 → D-inner = [40_000..60_000]²
        // Variant: [20_000..30_000]² — outside D-inner but inside W-inner.
        //   Layer 0 (even, uses D=40_000): variant OUTSIDE D-inner → survives
        //   Layer 1 (odd,  uses W=10_000): variant INSIDE  W-inner → empty
        let input_sq = square(0, 0, 100_000, 100_000);
        let variant = square(20_000, 20_000, 30_000, 30_000);

        let mut layer0 = BTreeMap::new();
        layer0.insert(painted_chain(1), vec![variant.clone()]);
        let mut layer1 = BTreeMap::new();
        layer1.insert(painted_chain(1), vec![variant.clone()]);

        let mut variants = vec![layer0, layer1];
        let inputs = vec![vec![input_sq.clone()], vec![input_sq.clone()]];

        let res = cut_segmented_layers(&mut variants, &inputs, 10_000, 40_000);
        assert!(res.is_ok());

        assert!(
            !variants[0][&painted_chain(1)].is_empty(),
            "layer 0 (even, D=4 mm): variant at 2–3 mm is outside D-inner and should survive"
        );
        assert!(
            variants[1][&painted_chain(1)].is_empty(),
            "layer 1 (odd, W=1 mm): variant at 2–3 mm is inside W-inner and should disappear"
        );
    }

    /// 3. When `interlocking_depth_units == 0`, even layers fall back to
    ///    `region_width` (no alternation). All layers receive identical treatment.
    #[test]
    fn interlocking_depth_zero_degenerates_to_width_limit() {
        // Same geometry as test 2, but D=0 → even layers use W, not D.
        // W = 1 mm = 10_000 → W-inner = [10_000..90_000]²
        // Variant [20_000..30_000]² is inside W-inner → empty on BOTH layers.
        // With D≠0 (test 2) layer 0 would survive; here it must also be empty.
        let input_sq = square(0, 0, 100_000, 100_000);
        let variant = square(20_000, 20_000, 30_000, 30_000);

        let mut layer0 = BTreeMap::new();
        layer0.insert(painted_chain(1), vec![variant.clone()]);
        let mut layer1 = BTreeMap::new();
        layer1.insert(painted_chain(1), vec![variant.clone()]);

        let mut variants = vec![layer0, layer1];
        let inputs = vec![vec![input_sq.clone()], vec![input_sq.clone()]];

        let res = cut_segmented_layers(&mut variants, &inputs, 10_000, 0);
        assert!(res.is_ok());

        // D=0 → no alternation; layer 0 uses W just like layer 1
        assert!(
            variants[0][&painted_chain(1)].is_empty(),
            "layer 0 (even, D=0 → uses W=1 mm): variant inside W-inner should be empty"
        );
        assert!(
            variants[1][&painted_chain(1)].is_empty(),
            "layer 1 (odd, W=1 mm): variant inside W-inner should be empty"
        );
    }

    /// 4. Negative config values are rejected with `InvalidPhase5Config` (AC-N1).
    #[test]
    fn width_limit_negative_rejected() {
        let input_sq = square(0, 0, 100_000, 100_000);
        let variant = square(0, 0, 100_000, 100_000);
        let mut layer = BTreeMap::new();
        layer.insert(painted_chain(1), vec![variant]);
        let mut variants = vec![layer];
        let inputs = vec![vec![input_sq]];

        // Negative region_width → error naming the width key
        let err_w = cut_segmented_layers(&mut variants, &inputs, -1, 0);
        match err_w {
            Err(PaintSegmentationError::InvalidPhase5Config { key, value }) => {
                assert_eq!(key, "mmu_segmented_region_max_width");
                assert_eq!(value, -1);
            }
            other => panic!("expected InvalidPhase5Config for negative width, got {other:?}"),
        }

        // Negative interlocking_depth → error naming the depth key
        let err_d = cut_segmented_layers(&mut variants, &inputs, 0, -5);
        match err_d {
            Err(PaintSegmentationError::InvalidPhase5Config { key, value }) => {
                assert_eq!(key, "mmu_segmented_region_interlocking_depth");
                assert_eq!(value, -5);
            }
            other => panic!("expected InvalidPhase5Config for negative depth, got {other:?}"),
        }
    }

    /// 5. Width large enough to swallow the entire variant footprint produces an
    ///    empty polygon list, but the map entry persists (D15). (AC-N2)
    #[test]
    fn width_limit_oversize_yields_empty() {
        // Input: 10 mm square [0..100_000]²
        // Width = 3 mm = 30_000 → inner = [30_000..70_000]²
        // Variant: 1 mm × 1 mm square at [45_000..55_000]² — fully inside inner
        let input_sq = square(0, 0, 100_000, 100_000);
        let tiny_variant = square(45_000, 45_000, 55_000, 55_000);

        let mut layer = BTreeMap::new();
        layer.insert(painted_chain(1), vec![tiny_variant]);
        let mut variants = vec![layer];
        let inputs = vec![vec![input_sq]];

        let res = cut_segmented_layers(&mut variants, &inputs, 30_000, 0);
        assert!(res.is_ok());

        // D15: the map entry must still exist even when polygons are gone
        assert!(
            variants[0].contains_key(&painted_chain(1)),
            "map entry must persist even when variant is fully clipped (D15)"
        );
        // The polygon list must be empty (variant ⊂ inner)
        assert!(
            variants[0][&painted_chain(1)].is_empty(),
            "variant fully inside the inner region must yield empty polygon list (AC-N2)"
        );
    }

    /// 6. When both config keys are zero the kernel short-circuits, returns Ok,
    ///    and leaves the input maps completely unmodified.
    #[test]
    fn kernel_short_circuits_when_both_keys_zero() {
        let input_sq = square(0, 0, 100_000, 100_000);
        let variant = square(0, 0, 100_000, 100_000);

        let mut layer = BTreeMap::new();
        layer.insert(painted_chain(1), vec![variant]);
        let mut variants = vec![layer];
        let inputs = vec![vec![input_sq]];

        // Record polygon state before the call
        let pts_before: usize = variants[0][&painted_chain(1)]
            .iter()
            .map(|e| e.contour.points.len())
            .sum();
        let first_x_before = variants[0][&painted_chain(1)][0].contour.points[0].x;
        let first_y_before = variants[0][&painted_chain(1)][0].contour.points[0].y;

        let res = cut_segmented_layers(&mut variants, &inputs, 0, 0);
        assert!(res.is_ok(), "both-zero must return Ok");

        // Polygon state must be byte-identical after the (no-op) call
        let pts_after: usize = variants[0][&painted_chain(1)]
            .iter()
            .map(|e| e.contour.points.len())
            .sum();
        assert_eq!(
            pts_before, pts_after,
            "short-circuit must leave polygon point count unchanged"
        );
        assert_eq!(
            variants[0][&painted_chain(1)][0].contour.points[0].x,
            first_x_before,
            "short-circuit must leave first point x unchanged"
        );
        assert_eq!(
            variants[0][&painted_chain(1)][0].contour.points[0].y,
            first_y_before,
            "short-circuit must leave first point y unchanged"
        );
    }
}
