#![allow(missing_docs)]

//! TDD coverage for canonical support-flow density derivation.

use slicer_core::support_regularize::{
    body_density, bottom_interface_density, interface_density, resolved_interface_flow_ratio,
};

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn canonical_density_derivations_match_formulas() {
    let width = 0.4_f32;
    let layer_height = 0.2_f32;
    let flow_spacing = slicer_core::flow::line_width_to_spacing(width, layer_height).unwrap();

    close(
        body_density(width, layer_height, 0.8).unwrap(),
        flow_spacing / (0.8 + flow_spacing),
    );
    close(
        interface_density(width, layer_height, 0.6).unwrap(),
        flow_spacing / (0.6 + flow_spacing),
    );
    close(
        bottom_interface_density(width, layer_height, 0.5).unwrap(),
        flow_spacing / (0.5 + flow_spacing),
    );

    let other_flow_spacing = slicer_core::flow::line_width_to_spacing(width, 0.3).unwrap();
    assert_ne!(flow_spacing, other_flow_spacing);
}

#[test]
fn densities_clamp_to_one_solid_pitch() {
    close(body_density(0.4, 0.2, 0.0).unwrap(), 1.0);
    close(interface_density(0.4, 0.2, 0.0).unwrap(), 1.0);
    close(bottom_interface_density(0.4, 0.2, 0.0).unwrap(), 1.0);
}

#[test]
fn nonpositive_interface_flow_falls_back_to_default() {
    close(resolved_interface_flow_ratio(0.0), 100.0);
    close(resolved_interface_flow_ratio(-25.0), 100.0);
    close(resolved_interface_flow_ratio(125.0), 125.0);
}
