//! TDD tests for the top-surface-ironing module (rev1, PostPass::LayerFinalization).
//!
//! These tests exercise the rewritten `TopSurfaceIroning` `FinalizationModule`
//! across object-scope `Vec<LayerCollectionIR>` fixtures. They mirror the
//! object-scope test style used by `skirt-brim/tests/finalization_live_tdd.rs`.
//!
//! Coordinate system reminder: 1 unit = 100 nm (NOT 1 nm). Geometry inputs
//! here are constructed in mm via `Point3WithWidth { x: f32, y: f32, ... }`
//! since `ExtrusionPath3D::points` carry mm values directly.

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer,
};
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};
use top_surface_ironing::TopSurfaceIroning;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn semver() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn config_with(pairs: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

/// Build a `PrintEntity` with the given role over a 10mm × 10mm closed
/// rectangle at (0, 0)-(10, 10) at the supplied z. The path has 5 points
/// (closing the loop) so unions / scan-line fills produce realistic output.
fn rect_entity(role: ExtrusionRole, z: f32, region_id: u64) -> PrintEntity {
    let mk = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    };
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![
                mk(0.0, 0.0),
                mk(10.0, 0.0),
                mk(10.0, 10.0),
                mk(0.0, 10.0),
                mk(0.0, 0.0),
            ],
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0, // overwritten by make_layer
            object_id: "obj-0".to_string(),
            region_id,
        },
        topo_order: 0,
    }
}

fn make_layer(index: u32, z: f32, mut entities: Vec<PrintEntity>) -> LayerCollectionIR {
    // Stamp the global_layer_index into each entity's region_key for realism.
    for e in entities.iter_mut() {
        e.region_key.global_layer_index = index;
    }
    LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: index,
        z,
        ordered_entities: entities,
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

/// Default ironing-enabled config matching Orca defaults.
fn enabled_config() -> ConfigView {
    config_with(&[
        ("ironing", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow", ConfigValue::Float(0.10)),
        ("ironing_spacing", ConfigValue::Float(0.1)),
        (
            "ironing_pattern",
            ConfigValue::String("rectilinear".to_string()),
        ),
    ])
}

/// Convert an `IR` Vec into the `View` slice expected by `run_finalization`.
fn views_from(layers: Vec<LayerCollectionIR>) -> Vec<LayerCollectionView> {
    layers.into_iter().map(LayerCollectionView::new).collect()
}

// ---------------------------------------------------------------------------
// AC-1: topmost layer emits Ironing path with reduced flow
// ---------------------------------------------------------------------------

#[test]
fn topmost_layer_emits_ironing_with_reduced_flow() {
    let config = enabled_config();
    let module = TopSurfaceIroning::on_print_start(&config).expect("config valid");

    // 5 layers (z = 0.0 / 0.2 / 0.4 / 0.6 / 0.8); only layer 4 carries
    // TopSolidInfill paths over a 10mm × 10mm square.
    let layers = vec![
        make_layer(0, 0.0, vec![]),
        make_layer(1, 0.2, vec![]),
        make_layer(2, 0.4, vec![]),
        make_layer(3, 0.6, vec![]),
        make_layer(
            4,
            0.8,
            vec![rect_entity(ExtrusionRole::TopSolidInfill, 0.8, 1)],
        ),
    ];
    let views = views_from(layers);
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    let on_top: Vec<_> = pushes
        .iter()
        .filter(|(li, p, _)| *li == 4 && p.role == ExtrusionRole::Ironing)
        .collect();
    assert!(
        !on_top.is_empty(),
        "expected at least one Ironing push on layer 4, got {} pushes total",
        pushes.len()
    );
    for (_, path, _) in &on_top {
        assert!(
            path.points.len() >= 4,
            "ironing path must have >= 4 points, got {}",
            path.points.len()
        );
        for pt in &path.points {
            assert!(
                pt.flow_factor < 0.5,
                "flow_factor must be < 0.5 for ironing, got {}",
                pt.flow_factor
            );
        }
    }

    // Zero pushes for layers 0..=3.
    for (li, p, _) in pushes.iter() {
        if p.role == ExtrusionRole::Ironing {
            assert_ne!(*li, 0, "no ironing should target layer 0");
            assert_ne!(*li, 1, "no ironing should target layer 1");
            assert_ne!(*li, 2, "no ironing should target layer 2");
            assert_ne!(*li, 3, "no ironing should target layer 3");
        }
    }
}

// ---------------------------------------------------------------------------
// AC-2: non-top-solid layer emits no ironing
// ---------------------------------------------------------------------------

#[test]
fn non_top_solid_layer_emits_no_ironing() {
    let config = enabled_config();
    let module = TopSurfaceIroning::on_print_start(&config).expect("config valid");

    let layers = vec![make_layer(
        0,
        0.2,
        vec![rect_entity(ExtrusionRole::BottomSolidInfill, 0.2, 1)],
    )];
    let views = views_from(layers);
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let ironing_count = output
        .entity_pushes()
        .iter()
        .filter(|(_, p, _)| p.role == ExtrusionRole::Ironing)
        .count();
    assert_eq!(
        ironing_count, 0,
        "expected zero Ironing pushes for bottom-only layer, got {ironing_count}"
    );
}

// ---------------------------------------------------------------------------
// AC-3: interior top-solid layer emits no ironing (real geometry)
// ---------------------------------------------------------------------------

#[test]
fn interior_top_solid_layer_emits_no_ironing() {
    let config = enabled_config();
    let module = TopSurfaceIroning::on_print_start(&config).expect("config valid");

    // 6 layers; layers 3, 4, 5 ALL carry real TopSolidInfill paths over the
    // same XY region. Only layer 5 is the topmost (no further layer above).
    let layers = vec![
        make_layer(0, 0.0, vec![]),
        make_layer(1, 0.2, vec![]),
        make_layer(2, 0.4, vec![]),
        make_layer(
            3,
            0.6,
            vec![rect_entity(ExtrusionRole::TopSolidInfill, 0.6, 1)],
        ),
        make_layer(
            4,
            0.8,
            vec![rect_entity(ExtrusionRole::TopSolidInfill, 0.8, 1)],
        ),
        make_layer(
            5,
            1.0,
            vec![rect_entity(ExtrusionRole::TopSolidInfill, 1.0, 1)],
        ),
    ];
    let views = views_from(layers);
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let pushes = output.entity_pushes();
    let on_top_5: usize = pushes
        .iter()
        .filter(|(li, p, _)| *li == 5 && p.role == ExtrusionRole::Ironing)
        .count();
    assert!(
        on_top_5 >= 1,
        "expected at least one Ironing push on layer 5 (the topmost), got {on_top_5}"
    );
    for (li, p, _) in pushes {
        if p.role == ExtrusionRole::Ironing {
            assert_ne!(*li, 3, "interior layer 3 must not receive Ironing pushes");
            assert_ne!(*li, 4, "interior layer 4 must not receive Ironing pushes");
        }
    }
}

// ---------------------------------------------------------------------------
// AC-4: disabled config emits no ironing AND preserves input
// ---------------------------------------------------------------------------

#[test]
fn disabled_config_emits_no_ironing_preserves_input() {
    let config = config_with(&[
        ("ironing", ConfigValue::Bool(false)),
        ("ironing_flow", ConfigValue::Float(0.10)),
    ]);
    let module = TopSurfaceIroning::on_print_start(&config).expect("config valid");

    let layers_orig = vec![make_layer(
        0,
        0.2,
        vec![rect_entity(ExtrusionRole::TopSolidInfill, 0.2, 1)],
    )];
    let layers_clone = layers_orig.clone();
    let views = views_from(layers_orig);
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    assert!(
        output.entity_pushes().is_empty(),
        "disabled ironing must emit zero entity pushes, got {}",
        output.entity_pushes().len()
    );

    // Reconstruct the underlying IR from the views (via clone of original)
    // and confirm bytewise equality with the pre-call clone.
    // Since `LayerCollectionView` consumed the original IR by move, we
    // compare `layers_clone` against itself reconstituted from views.
    let layers_after: Vec<LayerCollectionIR> = views
        .into_iter()
        .map(|v| {
            // Reconstruct an IR snapshot by accessor reads — these accessors
            // are read-only, so values must match the originals.
            LayerCollectionIR {
                schema_version: semver(),
                global_layer_index: v.layer_index(),
                z: v.z(),
                ordered_entities: v.ordered_entities().to_vec(),
                tool_changes: v.tool_changes().to_vec(),
                z_hops: v.z_hops().to_vec(),
                annotations: vec![],
                retracts: vec![],
                travel_moves: vec![],
            }
        })
        .collect();
    assert_eq!(
        layers_after, layers_clone,
        "input LayerCollectionIR must be bytewise unchanged when ironing is disabled"
    );
}

// ---------------------------------------------------------------------------
// AC-5: ironing_spacing controls stroke count
// ---------------------------------------------------------------------------

#[test]
fn ironing_spacing_controls_stroke_count() {
    let config = config_with(&[
        ("ironing", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow", ConfigValue::Float(0.10)),
        ("ironing_spacing", ConfigValue::Float(0.1)),
        (
            "ironing_pattern",
            ConfigValue::String("rectilinear".to_string()),
        ),
    ]);
    let module = TopSurfaceIroning::on_print_start(&config).expect("config valid");

    // Single topmost layer with a 10mm × 10mm TopSolidInfill region.
    let layers = vec![make_layer(
        0,
        0.2,
        vec![rect_entity(ExtrusionRole::TopSolidInfill, 0.2, 1)],
    )];
    let views = views_from(layers);
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let total_points: usize = output
        .entity_pushes()
        .iter()
        .filter(|(_, p, _)| p.role == ExtrusionRole::Ironing)
        .map(|(_, p, _)| p.points.len())
        .sum();
    assert!(
        total_points >= 100,
        "ironing_spacing=0.1mm over 10mm × 10mm should produce >= 100 stroke points, got {total_points}"
    );
}

// ---------------------------------------------------------------------------
// AC-6 (negative): bottom-only layer emits no ironing
// ---------------------------------------------------------------------------

#[test]
fn bottom_only_layer_emits_no_ironing() {
    let config = enabled_config();
    let module = TopSurfaceIroning::on_print_start(&config).expect("config valid");

    let layers = vec![make_layer(
        0,
        0.2,
        vec![rect_entity(ExtrusionRole::BottomSolidInfill, 0.2, 1)],
    )];
    let views = views_from(layers);
    let mut output = FinalizationOutputBuilder::new();
    module
        .run_finalization(&views, &mut output, &config)
        .expect("run_finalization must succeed");

    let ironing_count = output
        .entity_pushes()
        .iter()
        .filter(|(_, p, _)| p.role == ExtrusionRole::Ironing)
        .count();
    assert_eq!(
        ironing_count, 0,
        "bottom-only layer must emit zero Ironing pushes, got {ironing_count}"
    );
}

// ---------------------------------------------------------------------------
// AC-7 (negative): zero ironing flow is config error naming the key
// ---------------------------------------------------------------------------

#[test]
fn zero_ironing_flow_is_config_error() {
    let config = config_with(&[
        ("ironing", ConfigValue::Bool(true)),
        ("ironing_flow", ConfigValue::Float(0.0)),
    ]);

    let result = TopSurfaceIroning::on_print_start(&config);
    assert!(
        result.is_err(),
        "ironing_flow=0.0 must be rejected by on_print_start"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("ironing_flow"),
        "error diagnostic must contain 'ironing_flow'; got: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// AC-8 (negative): unsupported ironing_pattern is config error naming the key
// ---------------------------------------------------------------------------

#[test]
fn unsupported_ironing_pattern_is_config_error() {
    let config = config_with(&[
        ("ironing", ConfigValue::Bool(true)),
        ("ironing_flow", ConfigValue::Float(0.10)),
        (
            "ironing_pattern",
            ConfigValue::String("concentric".to_string()),
        ),
    ]);

    let result = TopSurfaceIroning::on_print_start(&config);
    assert!(
        result.is_err(),
        "ironing_pattern=concentric must be rejected by on_print_start"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("ironing_pattern"),
        "error diagnostic must contain 'ironing_pattern'; got: {err_msg}"
    );
}
