#![allow(missing_docs)]

//! Standalone TDD tests for `slicer_core::flow` (Step 4b, packet 105).
//!
//! Tests the public `line_width_to_spacing` and `flow_to_width` functions
//! against the OrcaSlicer formula documented in the module. Updated for
//! D-162: `line_width_to_spacing` now returns `Result`, erroring exactly
//! where canonical `Flow::rounded_rectangle_extrusion_spacing` throws
//! `FlowErrorNegativeSpacing` (iff the result is non-positive), and the
//! vestigial `nozzle_diameter` parameter is gone.

use slicer_core::flow::{
    flow_to_width, line_width_to_spacing, resolve_role_width, ExtrusionRole, RoleWidthContext,
};

/// Canonical OrcaSlicer case: width=0.4 mm, layer_height=0.2 mm.
/// Expected: 0.4 - 0.2 * (1 - PI/4) ≈ 0.3571 mm.
#[test]
fn canonical_0p4mm_bead_0p2mm_layer_spacing() {
    let s = line_width_to_spacing(0.4, 0.2).unwrap();
    // From the doc comment: ≈ 0.3571 mm
    assert!((s - 0.3571_f32).abs() < 1e-3, "expected ~0.3571, got {s}");
}

/// Wider bead (0.5 mm) with same layer height.
/// spacing = 0.5 - 0.2 * (1 - PI/4) ≈ 0.4571 mm.
#[test]
fn wider_bead_spacing_is_larger_than_canonical() {
    let s = line_width_to_spacing(0.5, 0.2).unwrap();
    let expected = 0.5 - 0.2 * (1.0_f32 - std::f32::consts::PI / 4.0);
    assert!((s - expected).abs() < 1e-4, "expected {expected}, got {s}");
    // Sanity: wider bead → wider spacing
    let s_canonical = line_width_to_spacing(0.4, 0.2).unwrap();
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
    let s = line_width_to_spacing(0.1, 0.2).unwrap();
    assert!((s - 0.0571).abs() < 1e-3, "expected 0.0571, got {s}");
}

/// The error boundary is exactly canonical's throw condition,
/// `width <= layer_height * (1 - PI/4)` — D-162 replaces the former 0.0
/// sentinel with `Err(NegativeSpacingError)`.
#[test]
fn spacing_errors_at_the_canonical_threshold() {
    let boundary = 0.2 * (1.0 - std::f32::consts::PI / 4.0);
    let err = line_width_to_spacing(boundary, 0.2).unwrap_err();
    assert_eq!(err.width_mm, boundary);
    assert_eq!(err.layer_height_mm, 0.2);
    assert!(err.spacing_mm <= 0.0);
    assert!(line_width_to_spacing(boundary * 1.5, 0.2).unwrap() > 0.0);
}

/// Non-positive width errors under the single canonical rule — no separate
/// defensive guard (D-162). Zero layer height with a positive width is NOT
/// an error: spacing = width, positive, matching canonical (which relies on
/// upstream config validation to reject a zero height).
#[test]
fn zero_or_negative_width_errors() {
    assert!(line_width_to_spacing(0.0, 0.2).is_err());
    assert!(line_width_to_spacing(-0.4, 0.2).is_err());
    assert_eq!(line_width_to_spacing(0.4, 0.0).unwrap(), 0.4);
}

/// The production-reachable case (classic schema allows layer_height up to
/// 2.0mm): a 0.4mm width at 2.0mm layer height is below the 0.429mm
/// threshold and must error with an actionable message.
#[test]
fn production_reachable_negative_spacing_errors_with_actionable_message() {
    let err = line_width_to_spacing(0.4, 2.0).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("0.40mm"), "msg must name the width: {msg}");
    assert!(
        msg.contains("2.00mm"),
        "msg must name the layer height: {msg}"
    );
    assert!(
        msg.contains("Increase the wall line width or reduce layer height"),
        "msg must state the fix: {msg}"
    );
}

/// Round-trip: line_width_to_spacing then flow_to_width should recover the
/// original width within floating-point epsilon.
#[test]
fn roundtrip_spacing_to_width_recovers_original() {
    let original_width = 0.4_f32;
    let layer_height = 0.2_f32;
    let spacing = line_width_to_spacing(original_width, layer_height).unwrap();
    let recovered = flow_to_width(spacing, layer_height);
    assert!(
        (original_width - recovered).abs() < 1e-4,
        "round-trip failed: original={original_width}, recovered={recovered}"
    );
}

/// Monotonicity: increasing line width → increasing spacing (for fixed
/// layer_height, in the normal operating range).
#[test]
fn spacing_is_monotone_in_line_width() {
    let layer_height = 0.2_f32;
    let widths = [0.3_f32, 0.4, 0.5, 0.6];
    let spacings: Vec<f32> = widths
        .iter()
        .map(|&w| line_width_to_spacing(w, layer_height).unwrap())
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

fn role_width_context() -> RoleWidthContext {
    RoleWidthContext {
        line_width: 0.4,
        nozzle_diameter: 0.4,
        bridge_line_width: 0.91,
        initial_layer_line_width: 0.81,
        outer_wall_line_width: 0.71,
        inner_wall_line_width: 0.61,
        top_surface_line_width: 0.51,
        internal_solid_infill_line_width: 0.49,
        sparse_infill_line_width: 0.39,
    }
}

fn roles() -> [ExtrusionRole; 9] {
    [
        ExtrusionRole::OuterWall,
        ExtrusionRole::InnerWall,
        ExtrusionRole::ThinWall,
        ExtrusionRole::TopSolidInfill,
        ExtrusionRole::BottomSolidInfill,
        ExtrusionRole::InternalSolidInfill,
        ExtrusionRole::SparseInfill,
        ExtrusionRole::BridgeInfill,
        ExtrusionRole::GapFill,
    ]
}

fn role_width(role: ExtrusionRole, context: &RoleWidthContext) -> f32 {
    match role {
        ExtrusionRole::OuterWall => context.outer_wall_line_width,
        ExtrusionRole::InnerWall | ExtrusionRole::ThinWall | ExtrusionRole::GapFill => {
            context.inner_wall_line_width
        }
        ExtrusionRole::TopSolidInfill => context.top_surface_line_width,
        ExtrusionRole::BottomSolidInfill | ExtrusionRole::InternalSolidInfill => {
            context.internal_solid_infill_line_width
        }
        ExtrusionRole::SparseInfill => context.sparse_infill_line_width,
        ExtrusionRole::BridgeInfill => context.bridge_line_width,
        _ => 0.0,
    }
}

fn set_role_width(context: &mut RoleWidthContext, role: ExtrusionRole, width: f32) {
    match role {
        ExtrusionRole::OuterWall => context.outer_wall_line_width = width,
        ExtrusionRole::InnerWall | ExtrusionRole::ThinWall | ExtrusionRole::GapFill => {
            context.inner_wall_line_width = width
        }
        ExtrusionRole::TopSolidInfill => context.top_surface_line_width = width,
        ExtrusionRole::BottomSolidInfill | ExtrusionRole::InternalSolidInfill => {
            context.internal_solid_infill_line_width = width
        }
        ExtrusionRole::SparseInfill => context.sparse_infill_line_width = width,
        ExtrusionRole::BridgeInfill => context.bridge_line_width = width,
        _ => {}
    }
}

#[test]
fn resolve_role_width_bridge_and_first_layer_precedence_matrix() {
    for role in roles() {
        let context = role_width_context();
        for first_layer in [false, true] {
            for bridge in [false, true] {
                let expected = if bridge {
                    context.bridge_line_width
                } else if first_layer {
                    context.initial_layer_line_width
                } else {
                    role_width(role.clone(), &context)
                };
                let actual = resolve_role_width(role.clone(), first_layer, bridge, &context);
                assert_eq!(
                    actual, expected,
                    "role={role:?}, first_layer={first_layer}, bridge={bridge}"
                );
            }
        }
    }
}

#[test]
fn resolve_role_width_bridge_fallback_covers_zero_and_absent_widths() {
    for role in roles() {
        let mut context = role_width_context();
        context.bridge_line_width = 0.0;
        context.initial_layer_line_width = 0.0;
        let expected_role_width = role_width(role.clone(), &context);
        let expected = if expected_role_width > 0.0 {
            expected_role_width
        } else {
            context.line_width
        };
        assert_eq!(
            resolve_role_width(role.clone(), false, true, &context),
            expected,
            "bridge zero must fall back to role width for {role:?}"
        );

        context.initial_layer_line_width = 0.81;
        assert_eq!(
            resolve_role_width(role.clone(), true, true, &context),
            context.initial_layer_line_width,
            "bridge fallback must allow the first-layer override for {role:?}"
        );

        context.initial_layer_line_width = 0.0;
        context.line_width = 0.0;
        let expected = if expected_role_width > 0.0 {
            expected_role_width
        } else {
            1.125 * context.nozzle_diameter
        };
        assert_eq!(
            resolve_role_width(role.clone(), true, true, &context),
            expected,
            "zero bridge and first-layer widths must fall through for {role:?}"
        );
    }
}

#[test]
fn resolve_role_width_role_zero_and_positive_matrix() {
    for role in roles() {
        let mut context = role_width_context();
        context.initial_layer_line_width = 0.0;
        let expected = role_width(role.clone(), &context);
        assert_eq!(
            resolve_role_width(role.clone(), false, false, &context),
            expected,
            "configured role width must stay unchanged for {role:?}"
        );

        set_role_width(&mut context, role.clone(), 0.0);
        assert_eq!(
            resolve_role_width(role.clone(), false, false, &context),
            context.line_width,
            "zero or absent role width must fall back to line width for {role:?}"
        );
    }
}

#[test]
fn resolve_role_width_auto_sentinel_uses_line_width_then_nozzle_width() {
    for role in roles() {
        let mut context = RoleWidthContext {
            nozzle_diameter: 0.4,
            ..RoleWidthContext::default()
        };
        set_role_width(&mut context, role.clone(), 0.0);
        assert_eq!(
            resolve_role_width(role, false, false, &context),
            1.125 * context.nozzle_diameter
        );
    }
}
