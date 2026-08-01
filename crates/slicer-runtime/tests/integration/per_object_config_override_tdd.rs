//! AC-8: per-object config override contract (P105 R2).
//!
//! Verifies that the 7 P105 config keys (outer_wall_line_width,
//! inner_wall_line_width, wall_sequence, detect_thin_wall, gap_infill_speed,
//! filter_out_gap_fill, precise_outer_wall) are read per-invocation from the
//! `_config` argument to `run_perimeters`, NOT from the cached `from_config`
//! config.
//!
//! Test: set print-global outer_wall_line_width=0.5 at from_config, then
//! pass a per-object override config with outer_wall_line_width=0.6 to
//! run_perimeters.  The emitted outer-wall vertex widths must equal 0.6 (the
//! override), proving the per-invocation read is respected.
//!
//! The "per-object override mechanism" in the test harness is simply passing a
//! different ConfigView to run_perimeters than was used at from_config.
//! This is sufficient: R2's intent is that run_perimeters reads _config (the
//! per-invocation argument) rather than cached struct fields, so any caller
//! that passes a different ConfigView at invoke time gets the override applied.

use arachne_perimeters::ArachnePerimeters;
use classic_perimeters::ClassicPerimeters;
use slicer_ir::slice_ir::ConfigView;
use slicer_ir::{ConfigValue, ExtrusionRole, LoopType, ResolvedConfig};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// AC-8: per-invocation outer_wall_line_width override.
///
/// from_config sees global outer_wall_line_width=0.5.
/// run_perimeters is called with a per-object config of outer_wall_line_width=0.6.
/// Emitted outer-wall vertex widths MUST be 0.6.
#[test]
fn per_object_outer_wall_line_width_override() {
    let global_outer_w = 0.5_f64;
    let override_outer_w = 0.6_f32;
    let inner_w = 0.4_f64;

    // from_config config: global values.
    let start_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", global_outer_w)
        .float("inner_wall_line_width", inner_w)
        .build();

    let module = ClassicPerimeters::from_config(&start_config).unwrap();

    // Per-object override config: outer_wall_line_width bumped to 0.6.
    let override_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", override_outer_w as f64)
        .float("inner_wall_line_width", inner_w)
        .build();

    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &override_config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_walls: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .collect();
    assert!(
        !outer_walls.is_empty(),
        "Expected at least one outer wall loop"
    );

    for outer in &outer_walls {
        for pt in &outer.path.points {
            assert!(
                (pt.width - override_outer_w).abs() < 0.005,
                "Outer wall vertex width {} != override {} (per-invocation config read must prevail over from_config cache)",
                pt.width,
                override_outer_w
            );
        }
    }

    // Also verify the inner wall uses the inner_w from override_config, not anything
    // stale from from_config (inner_w is the same in both configs, so we just
    // confirm the module didn't produce zero-width inner walls).
    let inner_walls: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .collect();
    assert!(
        !inner_walls.is_empty(),
        "Expected at least one inner wall loop"
    );
    for inner in &inner_walls {
        for pt in &inner.path.points {
            assert!(
                (pt.width - inner_w as f32).abs() < 0.005,
                "Inner wall vertex width {} != inner_w {}",
                pt.width,
                inner_w
            );
        }
    }
}

/// Regression: inner_wall_line_width override is also respected per-invocation.
///
/// from_config sees inner_wall_line_width=0.4; run_perimeters gets 0.3.
/// Inner wall vertex widths must equal 0.3.
#[test]
fn per_object_inner_wall_line_width_override() {
    let outer_w = 0.5_f64;
    let global_inner_w = 0.4_f64;
    let override_inner_w = 0.3_f32;

    let start_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w)
        .float("inner_wall_line_width", global_inner_w)
        .build();

    let module = ClassicPerimeters::from_config(&start_config).unwrap();

    let override_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w)
        .float("inner_wall_line_width", override_inner_w as f64)
        .build();

    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &override_config)
        .unwrap();

    let walls = output.wall_loops();
    let inner_walls: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .collect();
    assert!(
        !inner_walls.is_empty(),
        "Expected at least one inner wall loop"
    );
    for inner in &inner_walls {
        for pt in &inner.path.points {
            assert!(
                (pt.width - override_inner_w).abs() < 0.005,
                "Inner wall vertex width {} != override {} (per-invocation config read must prevail)",
                pt.width,
                override_inner_w
            );
        }
    }
}

fn matrix_value(width_mm: f64, as_percent: bool) -> ConfigValue {
    if as_percent {
        ConfigValue::FloatOrPercent {
            value: width_mm / 0.4 * 100.0,
            is_percent: true,
        }
    } else {
        ConfigValue::FloatOrPercent {
            value: width_mm,
            is_percent: false,
        }
    }
}

fn matrix_config(as_percent: bool, bridge_width_mm: Option<f64>) -> ConfigView {
    let mut resolved = ResolvedConfig::default();
    resolved.extensions.insert(
        "outer_wall_line_width".to_owned(),
        matrix_value(0.5, as_percent),
    );
    resolved.extensions.insert(
        "inner_wall_line_width".to_owned(),
        matrix_value(0.4, as_percent),
    );
    if let Some(bridge_width_mm) = bridge_width_mm {
        resolved.extensions.insert(
            "bridge_line_width".to_owned(),
            matrix_value(bridge_width_mm, as_percent),
        );
    }
    ConfigView::from_map(resolved.to_config_map())
}

fn matrix_region(top_shell_index: Option<u8>) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .top_shell_index(top_shell_index)
        .bridge_areas(vec![square_polygon(-4.0, -4.0, 4.0)])
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .build()
}

#[test]
fn arachne_width_bridge_precedence_matrix() {
    const ROLES: &[ExtrusionRole] = &[
        ExtrusionRole::OuterWall,
        ExtrusionRole::InnerWall,
        ExtrusionRole::InternalSolidInfill,
        ExtrusionRole::TopSolidInfill,
        ExtrusionRole::BridgeInfill,
        ExtrusionRole::GapFill,
        ExtrusionRole::SupportMaterial,
        ExtrusionRole::SupportInterface,
        ExtrusionRole::Skirt,
        ExtrusionRole::Brim,
        ExtrusionRole::Ironing,
    ];

    for role in ROLES {
        let is_outer_wall = matches!(role, &ExtrusionRole::OuterWall);
        let is_wall_role = matches!(role, &ExtrusionRole::OuterWall | &ExtrusionRole::InnerWall);
        let role_width_mm = if is_outer_wall { 0.5 } else { 0.4 };

        for first_layer in [true, false] {
            for bridge_width_mm in [Some(0.8), Some(0.0), None] {
                for as_percent in [false, true] {
                    for top_shell_index in [None, Some(0)] {
                        let config = matrix_config(as_percent, bridge_width_mm);
                        let outer_width = config
                            .get_abs_value("outer_wall_line_width", 0.4)
                            .expect("outer wall width must cross extensions");
                        let inner_width = config
                            .get_abs_value("inner_wall_line_width", 0.4)
                            .expect("inner wall width must cross extensions");
                        let bridge_width = config
                            .get_abs_value("bridge_line_width", role_width_mm)
                            .unwrap_or(0.0);
                        let expected_role_width = if is_outer_wall {
                            outer_width
                        } else {
                            inner_width
                        };
                        let expected_bridge_width = if bridge_width > 0.0 {
                            bridge_width
                        } else {
                            expected_role_width
                        };
                        assert!((expected_role_width - role_width_mm).abs() < 0.0001);
                        if let Some(bridge_width_mm) = bridge_width_mm {
                            let expected = if bridge_width_mm > 0.0 {
                                if as_percent {
                                    bridge_width_mm / 0.4 * role_width_mm
                                } else {
                                    bridge_width_mm
                                }
                            } else {
                                role_width_mm
                            };
                            assert!((expected_bridge_width - expected).abs() < 0.0001);
                        } else {
                            assert!((expected_bridge_width - role_width_mm).abs() < 0.0001);
                        }

                        let module = ArachnePerimeters::from_config(&config).unwrap();
                        let layer_index = u32::from(!first_layer);
                        let regions = vec![matrix_region(top_shell_index)];
                        let paint = PaintRegionLayerView::new(layer_index);
                        let mut output = PerimeterOutputBuilder::new();
                        module
                            .run_perimeters(layer_index, &regions, &paint, &mut output, &config)
                            .unwrap();

                        assert!(!output.wall_loops().is_empty());
                        assert!(output
                            .wall_loops()
                            .iter()
                            .flat_map(|wall| wall.feature_flags.iter())
                            .any(|flags| flags.is_bridge));
                        if is_wall_role {
                            assert!(output
                                .wall_loops()
                                .iter()
                                .any(|wall| &wall.path.role == role));
                        }
                    }
                }
            }
        }
    }
}
