#![allow(missing_docs)]

//! Standalone TDD tests for `slicer_core::flow` (Step 4b, packet 105).
//!
//! Tests the public `line_width_to_spacing` and `flow_to_width` functions
//! against the OrcaSlicer formula documented in the module.

use slicer_core::flow::{flow_to_width, line_width_to_spacing};

/// Canonical OrcaSlicer case: width=0.4 mm, layer_height=0.2 mm, nozzle=0.4 mm.
/// Expected: 0.4 - 0.2 * (1 - PI/4) ≈ 0.3571 mm.
#[test]
fn canonical_0p4mm_bead_0p2mm_layer_spacing() {
    let s = line_width_to_spacing(0.4, 0.2, 0.4);
    // From the doc comment: ≈ 0.3571 mm
    assert!((s - 0.3571_f32).abs() < 1e-3, "expected ~0.3571, got {s}");
}

/// Wider bead (0.5 mm) with same layer height / nozzle.
/// spacing = 0.5 - 0.2 * (1 - PI/4) ≈ 0.4571 mm.
#[test]
fn wider_bead_spacing_is_larger_than_canonical() {
    let s = line_width_to_spacing(0.5, 0.2, 0.4);
    let expected = 0.5 - 0.2 * (1.0_f32 - std::f32::consts::PI / 4.0);
    assert!((s - expected).abs() < 1e-4, "expected {expected}, got {s}");
    // Sanity: wider bead → wider spacing
    let s_canonical = line_width_to_spacing(0.4, 0.2, 0.4);
    assert!(s > s_canonical, "wider bead must produce wider spacing");
}

/// `width < layer_height` is NOT degenerate — the formula stays positive well
/// below it. Canonical `Flow::rounded_rectangle_extrusion_spacing` rejects only
/// `width - height * (1 - PI/4) <= 0`; for height 0.2 that is width <= 0.0429,
/// not width < 0.2.
///
/// This test previously asserted `line_width_to_spacing(0.1, 0.2, 0.4) == 0.0`,
/// pinning a fabricated guard whose doc claimed "the formula would go negative".
/// It does not: 0.1 - 0.0429 = 0.0571.
#[test]
fn width_below_layer_height_still_has_positive_spacing() {
    let s = line_width_to_spacing(0.1, 0.2, 0.4);
    assert!((s - 0.0571).abs() < 1e-3, "expected 0.0571, got {s}");
}

/// Spacing collapses to 0.0 only at canonical's real threshold,
/// `width <= layer_height * (1 - PI/4)` (where canonical throws instead).
#[test]
fn spacing_is_zero_only_at_the_canonical_threshold() {
    let boundary = 0.2 * (1.0 - std::f32::consts::PI / 4.0);
    assert_eq!(line_width_to_spacing(boundary, 0.2, 0.4), 0.0);
    assert!(line_width_to_spacing(boundary * 1.5, 0.2, 0.4) > 0.0);
}

/// Zero or negative inputs → returns 0.0.
#[test]
fn zero_or_negative_inputs_return_zero() {
    assert_eq!(line_width_to_spacing(0.0, 0.2, 0.4), 0.0);
    assert_eq!(line_width_to_spacing(0.4, 0.0, 0.4), 0.0);
    assert_eq!(line_width_to_spacing(0.4, 0.2, 0.0), 0.0);
    assert_eq!(line_width_to_spacing(-0.4, 0.2, 0.4), 0.0);
}

/// Round-trip: line_width_to_spacing then flow_to_width should recover the
/// original width within floating-point epsilon.
#[test]
fn roundtrip_spacing_to_width_recovers_original() {
    let original_width = 0.4_f32;
    let layer_height = 0.2_f32;
    let spacing = line_width_to_spacing(original_width, layer_height, 0.4);
    let recovered = flow_to_width(spacing, layer_height);
    assert!(
        (original_width - recovered).abs() < 1e-4,
        "round-trip failed: original={original_width}, recovered={recovered}"
    );
}

/// Monotonicity: increasing line width → increasing spacing (for fixed
/// layer_height and nozzle diameter, in the normal operating range).
#[test]
fn spacing_is_monotone_in_line_width() {
    let layer_height = 0.2_f32;
    let nozzle = 0.4_f32;
    let widths = [0.3_f32, 0.4, 0.5, 0.6];
    let spacings: Vec<f32> = widths
        .iter()
        .map(|&w| line_width_to_spacing(w, layer_height, nozzle))
        .collect();
    for pair in spacings.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "spacing not monotone: {} < {} (widths {:?})",
            pair[1],
            pair[0],
            widths
        );
    }
}

/// `flow_to_width` with zero or negative inputs → returns 0.0.
#[test]
fn flow_to_width_zero_inputs_return_zero() {
    assert_eq!(flow_to_width(0.0, 0.2), 0.0);
    assert_eq!(flow_to_width(0.4, 0.0), 0.0);
    assert_eq!(flow_to_width(-0.1, 0.2), 0.0);
}

/// `flow_to_width` returns at least `spacing` (clamped to >= spacing).
#[test]
fn flow_to_width_result_is_at_least_spacing() {
    let spacing = 0.357_f32;
    let layer_height = 0.2_f32;
    let w = flow_to_width(spacing, layer_height);
    assert!(
        w >= spacing,
        "flow_to_width({spacing}, {layer_height}) = {w} < spacing"
    );
}
