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
//! (OrcaSlicer `libslic3r/Flow.cpp` — `Flow::rounded_rectangle_extrusion_spacing`.)
//! The formula is unconditional: there is no nozzle-diameter clamp and no
//! width-versus-layer-height guard. For the common case
//! `width == nozzle_diameter == 0.4 mm` and `layer_height == 0.2 mm`,
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

/// Error returned when the flow formula produces a non-positive spacing —
/// the Rust analog of canonical `FlowErrorNegativeSpacing` (`Flow.hpp`, a
/// `Slic3r::InvalidArgument`): canonical throws and the slice aborts with a
/// config diagnosis, so callers here must treat this as slice-fatal (D-162).
#[derive(Debug, Clone, PartialEq)]
pub struct NegativeSpacingError {
    /// The line width (mm) that was too small.
    pub width_mm: f32,
    /// The layer height (mm) it was paired with.
    pub layer_height_mm: f32,
    /// The non-positive result: `width − layer_height·(1 − π/4)`.
    pub spacing_mm: f32,
}

impl std::fmt::Display for NegativeSpacingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let threshold = self.layer_height_mm * (1.0 - core::f32::consts::PI / 4.0);
        write!(
            f,
            "line width {:.2}mm is too small for layer height {:.2}mm: extrusion spacing \
             would be {:.2}mm (width must exceed layer_height*(1 - pi/4) = {:.2}mm). \
             Increase the wall line width or reduce layer height.",
            self.width_mm, self.layer_height_mm, self.spacing_mm, threshold
        )
    }
}

impl std::error::Error for NegativeSpacingError {}

/// Convert an extrusion line width (mm) to inter-wall spacing (mm) per OrcaSlicer's
/// `Flow::rounded_rectangle_extrusion_spacing`:
///
/// `spacing = width - layer_height * (1.0 - PI / 4.0)`
///
/// Errors **iff** the result is non-positive — the single canonical rejection
/// rule (canonical throws `FlowErrorNegativeSpacing` there; Rust has no
/// exceptions, so `Result` is the throw analog). There is no other guard: a
/// non-positive `width` or `layer_height` yields a non-positive spacing and is
/// covered by the same rule (canonical relies on upstream config validation,
/// as does PnP — manifest `[min, max]` ranges enforced at config resolution).
///
/// Canonical's formula does not reference the nozzle; a former vestigial
/// `nozzle_diameter` parameter was removed with D-162 so the signature cannot
/// re-grow a nozzle clamp unnoticed.
pub fn line_width_to_spacing(
    width: f32,
    layer_height: f32,
) -> Result<f32, NegativeSpacingError> {
    let pi_minus_quarter = 1.0_f32 - core::f32::consts::PI / 4.0_f32;
    let spacing = width - layer_height * pi_minus_quarter;
    if spacing <= 0.0 {
        Err(NegativeSpacingError {
            width_mm: width,
            layer_height_mm: layer_height,
            spacing_mm: spacing,
        })
    } else {
        Ok(spacing)
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
        let s = line_width_to_spacing(0.4, 0.2).unwrap();
        assert!((s - 0.3571).abs() < 0.001, "got {s}");
    }

    #[test]
    fn width_at_or_above_nozzle_produces_larger_spacing() {
        // The OrcaSlicer formula yields spacing close to width when width >= nozzle.
        let s = line_width_to_spacing(0.5, 0.2).unwrap();
        // spacing = 0.5 - 0.2 * (1 - pi/4) ≈ 0.4571
        assert!(s > 0.4 && s <= 0.5, "got {s}");
    }

    #[test]
    fn width_below_layer_height_still_has_positive_spacing() {
        // Regression: a fabricated `width < layer_height -> 0.0` guard used to
        // fire here, documented as "the formula would yield a negative number".
        // It does not. Canonical `rounded_rectangle_extrusion_spacing` rejects
        // only when `width - height * (1 - pi/4) <= 0`, which for height=0.2 is
        // width <= 0.0429 — not width < 0.2. The old test asserted the guard.
        //
        // 0.1 - 0.2 * (1 - pi/4) = 0.1 - 0.0429 = 0.0571, comfortably positive.
        let s = line_width_to_spacing(0.1, 0.2).unwrap();
        assert!(
            (s - 0.0571).abs() < 1e-3,
            "width 0.1 < layer_height 0.2 must still yield the formula's 0.0571, got {s}"
        );

        // The case the guard actually broke in production: narrow_strip_widening
        // runs layer_height 1.0mm, so every 0.4mm wall tripped it and the
        // beading strategy was handed a raw WIDTH where it expects a SPACING.
        let s = line_width_to_spacing(0.4, 1.0).unwrap();
        assert!(
            (s - 0.1854).abs() < 1e-3,
            "0.4mm width at 1.0mm layer height must yield spacing 0.1854, got {s}"
        );
    }

    #[test]
    fn spacing_errors_at_canonicals_actual_threshold() {
        // Canonical throws iff `width - height * (1 - pi/4) <= 0`. For
        // height = 0.2 that boundary is width = 0.0429. PnP now errors there
        // (D-162), mirroring the throw — no 0.0 sentinel survives.
        let boundary = 0.2 * (1.0 - core::f32::consts::PI / 4.0);
        let err = line_width_to_spacing(boundary, 0.2).unwrap_err();
        assert_eq!(err.width_mm, boundary);
        assert_eq!(err.layer_height_mm, 0.2);
        assert!(err.spacing_mm <= 0.0);
        assert!(line_width_to_spacing(boundary * 0.5, 0.2).is_err());
        assert!(line_width_to_spacing(boundary * 1.5, 0.2).unwrap() > 0.0);
    }

    #[test]
    fn zero_or_negative_inputs_error() {
        // No separate defensive guard (D-162): non-positive width/height
        // yields a non-positive spacing, so the single canonical rule rejects
        // them. Config validation upstream (manifest [min,max]) is the real
        // gate, as in canonical.
        assert!(line_width_to_spacing(0.0, 0.2).is_err());
        assert!(line_width_to_spacing(-1.0, 0.2).is_err());
        // Zero layer height with a positive width is NOT an error under the
        // canonical rule: spacing = width, positive.
        assert_eq!(line_width_to_spacing(0.4, 0.0).unwrap(), 0.4);
    }

    #[test]
    fn error_message_names_inputs_and_fix() {
        let err = line_width_to_spacing(0.4, 2.0).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("0.40mm"), "msg: {msg}");
        assert!(msg.contains("2.00mm"), "msg: {msg}");
        assert!(
            msg.contains("Increase the wall line width or reduce layer height"),
            "msg: {msg}"
        );
    }

    #[test]
    fn roundtrip_spacing_to_width() {
        let w = 0.4;
        let lh = 0.2;
        let s = line_width_to_spacing(w, lh).unwrap();
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
