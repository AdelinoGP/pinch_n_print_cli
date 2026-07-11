// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Flow.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! Flow math for perimeter wall emission (T-050, packet 105).
//!
//! Ported from OrcaSlicer's `Flow::new_from_width_height` (a minimal subset).
//! Pinch 'n Print units: 1 unit = 100 nm; all widths/heights in mm at this boundary.
//!
//! The OrcaSlicer formula for extrusion spacing from a given line width:
//!
//! ```text
//! spacing = width - layer_height * (1.0 - PI / 4.0)
//! ```
//!
//! (OrcaSlicer `libslic3r/Flow.cpp` — `Flow::new_from_width_height`.)
//! For `width >= nozzle_diameter`, the spacing is clamped to the width itself
//! (the bead sits on top of itself rather than overlapping). For the common
//! case `width == nozzle_diameter == 0.4 mm` and `layer_height == 0.2 mm`,
//! the formula yields:
//!
//! ```text
//! spacing = 0.4 - 0.2 * (1.0 - PI/4.0)
//!         = 0.4 - 0.2 * 0.2146
//!         = 0.4 - 0.0429
//!         = 0.3571 mm
//! ```
//!
//! Pinch 'n Print stores all width / height inputs in mm; the function returns mm.
//!
//! The distinct outer / inner widths (T-051) and the canonical
//! `ext_perimeter_spacing2 = (outer + inner) / 2` (T-052) and
//! `perimeter_spacing = inner` (T-052) arithmetic live in `perimeter_utils`.

/// Convert an extrusion line width (mm) to inter-wall spacing (mm) per OrcaSlicer's
/// `Flow::new_from_width_height` formula.
///
/// `spacing = width - layer_height * (1.0 - PI / 4.0)`, clamped at `>= 0`.
///
/// Edge cases:
/// - `width <= 0`, `layer_height <= 0`, `nozzle_diameter <= 0` → returns `0.0`.
/// - `width < layer_height` → returns `0.0` (the formula would yield a negative number).
/// - `width >= nozzle_diameter` → returns `width` (bead is wider than nozzle; spacing == width).
pub fn line_width_to_spacing(width: f32, layer_height: f32, nozzle_diameter: f32) -> f32 {
    if width <= 0.0 || layer_height <= 0.0 || nozzle_diameter <= 0.0 {
        return 0.0;
    }
    if width < layer_height {
        return 0.0;
    }
    let pi_minus_quarter = 1.0_f32 - core::f32::consts::PI / 4.0_f32;
    let spacing = width - layer_height * pi_minus_quarter;
    if spacing < 0.0 {
        0.0
    } else {
        spacing
    }
}

/// Inverse of `line_width_to_spacing` (approximate). Returns the line width that
/// produces the given spacing under the OrcaSlicer formula.
///
/// `width = spacing + layer_height * (1.0 - PI / 4.0)`, clamped to `>= spacing`.
/// Used by gap-fill consumers that need a width matching the resolved spacing.
pub fn flow_to_width(spacing: f32, layer_height: f32) -> f32 {
    if spacing <= 0.0 || layer_height <= 0.0 {
        return 0.0;
    }
    let pi_minus_quarter = 1.0_f32 - core::f32::consts::PI / 4.0_f32;
    (spacing + layer_height * pi_minus_quarter).max(spacing)
}

/// Flow-rate multiplier applied to bridging extrusions (packet 149 D4;
/// packet 150 step 5 closes D-104g's `thick_bridges` stub).
///
/// Mirrors OrcaSlicer's `LayerRegion.cpp` `bridging_flow`. The non-thick
/// branch applies `bridge_flow_ratio` directly (the real formula is
/// `base_flow.with_flow_ratio(bridge_flow_ratio)` — `bridge_flow_ratio` is a
/// ratio applied on top of the base flow, not a standalone constant; PnP's
/// per-vertex `flow_factor` model applies the ratio as-is).
///
/// The `thick_bridges == true` branch now computes OrcaSlicer's
/// round-cross-section flow factor: a bridge is extruded as a round thread
/// of diameter `dmr = nozzle_diameter * sqrt(bridge_flow_ratio)`
/// (`Flow::bridging_flow`, `Flow.hpp`/`Flow.cpp`), and the returned factor is
/// that thread's cross-section area relative to a flat bead of the given
/// `bead_width` × `layer_height`:
///
/// ```text
/// dmr = nozzle_diameter * sqrt(bridge_flow_ratio)
/// factor = (PI * dmr^2 / 4) / (bead_width * layer_height)
/// ```
///
/// `nozzle_diameter`, `bead_width`, and `layer_height` must all be in the
/// same unit (mm at all current call sites) — the result is a dimensionless
/// area ratio, so a unit mismatch between them silently produces the wrong
/// factor. Sanity case: nozzle=0.4mm, bridge_flow_ratio=1.0, bead_width=0.4mm,
/// layer_height=0.2mm → dmr=0.4mm → factor ≈ 1.5708.
///
/// Degenerate inputs (`bead_width <= 0`, `layer_height <= 0`, or
/// `nozzle_diameter <= 0`) fall back to the non-thick behavior
/// (`bridge_flow_ratio`) rather than dividing by zero / producing NaN.
pub fn bridging_flow(
    bridge_flow_ratio: f32,
    thick_bridges: bool,
    nozzle_diameter: f32,
    bead_width: f32,
    layer_height: f32,
) -> f32 {
    if thick_bridges {
        if bead_width <= 0.0 || layer_height <= 0.0 || nozzle_diameter <= 0.0 {
            return bridge_flow_ratio;
        }
        let dmr = nozzle_diameter * bridge_flow_ratio.sqrt();
        core::f32::consts::PI * dmr * dmr / (4.0 * bead_width * layer_height)
    } else {
        bridge_flow_ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_0p4mm_bead_0p2mm_layer() {
        // The canonical OrcaSlicer sanity case: width=0.4, layer_height=0.2.
        // Result should be ~0.357 mm (width - layer_height * (1 - pi/4)).
        let s = line_width_to_spacing(0.4, 0.2, 0.4);
        assert!((s - 0.3571).abs() < 0.001, "got {s}");
    }

    #[test]
    fn width_at_or_above_nozzle_produces_larger_spacing() {
        // The OrcaSlicer formula yields spacing close to width when width >= nozzle.
        let s = line_width_to_spacing(0.5, 0.2, 0.4);
        // spacing = 0.5 - 0.2 * (1 - pi/4) ≈ 0.4571
        assert!(s > 0.4 && s <= 0.5, "got {s}");
    }

    #[test]
    fn width_below_layer_returns_zero() {
        assert_eq!(line_width_to_spacing(0.1, 0.2, 0.4), 0.0);
    }

    #[test]
    fn zero_or_negative_inputs_return_zero() {
        assert_eq!(line_width_to_spacing(0.0, 0.2, 0.4), 0.0);
        assert_eq!(line_width_to_spacing(0.4, 0.0, 0.4), 0.0);
        assert_eq!(line_width_to_spacing(0.4, 0.2, 0.0), 0.0);
        assert_eq!(line_width_to_spacing(-1.0, 0.2, 0.4), 0.0);
    }

    #[test]
    fn roundtrip_spacing_to_width() {
        let w = 0.4;
        let lh = 0.2;
        let s = line_width_to_spacing(w, lh, 0.4);
        let w2 = flow_to_width(s, lh);
        // For the canonical case, round-trip should match within epsilon.
        assert!((w - w2).abs() < 0.001, "w={w} w2={w2}");
    }

    #[test]
    fn bridging_flow_non_thick_returns_ratio_unchanged() {
        // Non-thick branch is unchanged by the signature/formula update:
        // it must still return bridge_flow_ratio verbatim, independent of
        // nozzle_diameter/bead_width/layer_height.
        assert_eq!(bridging_flow(1.0, false, 0.4, 0.4, 0.2), 1.0);
        assert_eq!(bridging_flow(0.85, false, 0.4, 0.4, 0.2), 0.85);
        assert_eq!(bridging_flow(0.85, false, 0.0, 0.0, 0.0), 0.85);
    }

    #[test]
    #[allow(clippy::approx_constant)] // 1.5708 is the OrcaSlicer-derived expected value, not FRAC_PI_2 used intentionally.
    fn bridging_flow_thick_round_cross_section_factor() {
        // OrcaSlicer sanity case: nozzle=0.4, bridge_flow_ratio=1.0,
        // bead_width=0.4, layer_height=0.2 -> dmr=0.4 ->
        // factor = PI*0.16/(4*0.4*0.2) = PI*0.16/0.32 ~= 1.5708.
        let f = bridging_flow(1.0, true, 0.4, 0.4, 0.2);
        assert!((f - 1.5708).abs() < 0.01, "got {f}");
    }

    #[test]
    fn bridging_flow_thick_degenerate_inputs_fall_back_to_ratio() {
        // Zero/negative bead_width, layer_height, or nozzle_diameter would
        // divide by zero / produce NaN under the round-cross-section
        // formula; fall back to the non-thick ratio instead.
        assert_eq!(bridging_flow(0.9, true, 0.4, 0.0, 0.2), 0.9);
        assert_eq!(bridging_flow(0.9, true, 0.4, 0.4, 0.0), 0.9);
        assert_eq!(bridging_flow(0.9, true, 0.0, 0.4, 0.2), 0.9);
    }
}
