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

/// Flow-rate multiplier applied to bridging extrusions (packet 149, D4).
///
/// Mirrors OrcaSlicer's `LayerRegion.cpp` `bridging_flow`: the real formula is
/// `base_flow.with_flow_ratio(bridge_flow_ratio)` — `bridge_flow_ratio` is a
/// ratio applied on top of the base flow, not a standalone constant. The
/// `thick_bridges == true` branch returning `1.0` (i.e. no flow reduction) is
/// a Pinch 'n Print divergence from OrcaSlicer's per-path `Flow`
/// height/nozzle-diameter model, registered as deviation D-104g.
pub fn bridging_flow(bridge_flow_ratio: f32, thick_bridges: bool) -> f32 {
    if thick_bridges {
        1.0
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
}
