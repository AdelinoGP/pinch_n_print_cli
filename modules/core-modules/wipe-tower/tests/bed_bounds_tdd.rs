//! TDD scaffold: wipe-tower placement validation against config-supplied bed polygon.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 2 scaffolding.
//! AC6 — tower vertices are inside the bed polygon.
//!
//! Bed-containment only. The object-footprint non-intersection half of AC6 is
//! deferred to a follow-up packet (DEV-054, closed) — the test name now
//! reflects this scope rather than over-claiming "outside_objects".

#![allow(missing_docs)]

use slicer_ir::{ConfigValue, ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::test_prelude::{
    print_entity, tool_change, ConfigViewBuilder, LayerCollectionFixtureBuilder,
};
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};
use wipe_tower::WipeTower;

/// Build a minimal ConfigView for the given key-value pairs.
fn config_from_pairs(pairs: &[(&str, ConfigValue)]) -> slicer_ir::ConfigView {
    let mut builder = ConfigViewBuilder::new();
    for (k, v) in pairs {
        match v {
            ConfigValue::Bool(b) => builder = builder.bool(*k, *b),
            ConfigValue::Float(f) => builder = builder.float(*k, *f),
            ConfigValue::Int(i) => builder = builder.int(*k, *i),
            ConfigValue::String(s) => builder = builder.string(*k, s.clone()),
            ConfigValue::List(l) => builder = builder.list(*k, l.clone()),
            ConfigValue::Percent(p) => builder = builder.float(*k, *p),
            ConfigValue::FloatOrPercent { value, .. } => builder = builder.float(*k, *value),
        }
    }
    builder.build()
}

/// Build a minimal single-layer IR with one ToolChange after entity 0.
fn layer_with_tool_change() -> slicer_ir::LayerCollectionIR {
    let entity = print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            Point3WithWidth {
                x: 5.0,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: 6.0,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                ..Default::default()
            },
        ],
        RegionKey {
            global_layer_index: 0,
            object_id: "cube".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        0,
    );
    LayerCollectionFixtureBuilder::new()
        .global_layer_index(0)
        .z(0.2)
        .add_entity(entity)
        .add_tool_change(tool_change(0, 0, 1))
        .build()
}

/// AC6 — tower geometry within config-supplied bed polygon (bed-containment half).
///
/// Setup: `enable_prime_tower=true`, `printable_area=[0,0, 250,0, 250,250, 0,250]`
/// (250×250 mm bed), `wipe_tower_x=10.0`, `wipe_tower_y=10.0`,
/// `prime_tower_width=60.0`. When the wipe-tower module emits purge paths for
/// the first layer, `run_finalization` must return `Ok`, at least one
/// `WipeTower` entity must be produced, and all path points of every
/// `WipeTower` entity must lie within `[0, 250] × [0, 250]`.
///
/// The "outside object footprint" half of AC6 is deferred to a follow-up
/// packet (DEV-054, closed) and intentionally NOT asserted here. The
/// test name reflects only the assertions that actually run.
#[test]
fn tower_geometry_within_config_bed_only() {
    let config = config_from_pairs(&[
        ("enable_prime_tower", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(10.0)),
        ("wipe_tower_y", ConfigValue::Float(10.0)),
        ("prime_tower_width", ConfigValue::Float(60.0)),
        ("prime_volume", ConfigValue::Float(70.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("retract_length", ConfigValue::Float(2.0)),
        (
            "printable_area",
            ConfigValue::List(vec![
                ConfigValue::Float(0.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
            ]),
        ),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");

    let ir_layer = layer_with_tool_change();
    let mut layers = vec![ir_layer.clone()];
    let sdk_layers: Vec<LayerCollectionView> = vec![LayerCollectionView::new(ir_layer)];
    let mut output = FinalizationOutputBuilder::new();

    let result = tower.run_finalization(&sdk_layers, &mut output, &config);
    assert!(
        result.is_ok(),
        "AC6 FAIL: run_finalization returned Err for a valid tower inside bed polygon: {:?}",
        result.unwrap_err()
    );

    // Apply the insertions to get the final layer state.
    output
        .apply_to(&mut layers)
        .expect("apply_to must succeed for valid insertions");

    // Collect all WipeTower entities from the modified layer.
    let wipe_entities: Vec<&slicer_ir::PrintEntity> = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| matches!(e.role, ExtrusionRole::WipeTower))
        .collect();

    assert!(
        !wipe_entities.is_empty(),
        "AC6 FAIL: no WipeTower entities in layer after run_finalization"
    );

    // All path points of WipeTower entities must lie within [0, 250] × [0, 250].
    for entity in &wipe_entities {
        for pt in &entity.path.points {
            assert!(
                pt.x >= 0.0 && pt.x <= 250.0,
                "AC6 FAIL: WipeTower point X={:.3} lies outside [0, 250] bed bounds",
                pt.x
            );
            assert!(
                pt.y >= 0.0 && pt.y <= 250.0,
                "AC6 FAIL: WipeTower point Y={:.3} lies outside [0, 250] bed bounds",
                pt.y
            );
        }
    }

    // Object-footprint non-intersection is deferred to a follow-up packet
    // (DEV-054, closed). When that lands, replace this comment with the
    // assertion and rename the test back to include "outside_objects".
}

/// Regression (ticket 100): an Orca 3MF plate serialises `printable_area` as
/// point strings (`["0x0", "50x0", "50x50", "0x50"]`), not interleaved
/// numbers. The old reader filtered every non-numeric entry out, so the
/// polygon silently collapsed to nothing and the module fell back to its
/// 250×250 mm default bed — a 60 mm tower at x=10 would then be *accepted* on
/// a 50 mm bed.
///
/// Setup: 50×50 mm bed in Orca point-string form, `wipe_tower_x=10.0`,
/// `prime_tower_width=60.0` — the tower spans x ∈ [10, 70], past the bed's
/// 50 mm edge. `run_finalization` must return `Err`; an `Ok` means the point
/// strings were dropped and the default bed was used instead.
#[test]
fn orca_point_string_bed_is_parsed_not_silently_defaulted() {
    let config = config_from_pairs(&[
        ("enable_prime_tower", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(10.0)),
        ("wipe_tower_y", ConfigValue::Float(10.0)),
        ("prime_tower_width", ConfigValue::Float(60.0)),
        ("prime_volume", ConfigValue::Float(45.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("retract_length", ConfigValue::Float(2.0)),
        (
            "printable_area",
            ConfigValue::List(vec![
                ConfigValue::String("0x0".into()),
                ConfigValue::String("50x0".into()),
                ConfigValue::String("50x50".into()),
                ConfigValue::String("0x50".into()),
            ]),
        ),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");

    let ir_layer = layer_with_tool_change();
    let sdk_layers: Vec<LayerCollectionView> = vec![LayerCollectionView::new(ir_layer)];
    let mut output = FinalizationOutputBuilder::new();

    assert!(
        tower
            .run_finalization(&sdk_layers, &mut output, &config)
            .is_err(),
        "a 60 mm tower at x=10 must be rejected by the 50 mm Orca point-string bed; \
         Ok means the point strings were dropped and the 250×250 default bed was used"
    );
}

/// The same Orca point-string bed, sized so the tower *does* fit, must be
/// accepted — the adapter widens what is understood, it does not reject.
#[test]
fn orca_point_string_bed_accepts_a_tower_that_fits() {
    let config = config_from_pairs(&[
        ("enable_prime_tower", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(10.0)),
        ("wipe_tower_y", ConfigValue::Float(10.0)),
        ("prime_tower_width", ConfigValue::Float(60.0)),
        ("prime_volume", ConfigValue::Float(45.0)),
        ("line_width", ConfigValue::Float(0.4)),
        ("retract_length", ConfigValue::Float(2.0)),
        (
            "printable_area",
            ConfigValue::List(vec![
                ConfigValue::String("0x0".into()),
                ConfigValue::String("250x0".into()),
                ConfigValue::String("250x210".into()),
                ConfigValue::String("0x210".into()),
            ]),
        ),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");

    let ir_layer = layer_with_tool_change();
    let sdk_layers: Vec<LayerCollectionView> = vec![LayerCollectionView::new(ir_layer)];
    let mut output = FinalizationOutputBuilder::new();

    assert!(
        tower
            .run_finalization(&sdk_layers, &mut output, &config)
            .is_ok(),
        "a 60 mm tower at x=10 fits the 250×210 Orca plate and must be accepted"
    );
}

// ── Ticket 129: the footprint is `width × depth`, never `width × width` ──────
//
// `run_finalization` used to validate the corners of a `tower_width` square.
// The box `generate_purge_paths` actually emits is `tower_width` wide and
// `purge_volume / (line_width · layer_height · tower_width)` DEEP — the same
// width/depth split canonical draws between `WipeTower::width` and
// `WipeTower::get_depth` (paired by `Print.cpp`'s `construct_mesh` call).
// The square model rejected towers that fit.

/// A layer with print geometry but no tool change: nothing for the tower to do.
fn layer_without_tool_change() -> slicer_ir::LayerCollectionIR {
    let entity = print_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            Point3WithWidth {
                x: 5.0,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: 6.0,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
        ],
        RegionKey {
            global_layer_index: 0,
            object_id: "cube".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        0,
    );
    LayerCollectionFixtureBuilder::new()
        .global_layer_index(0)
        .z(0.2)
        .add_entity(entity)
        .build()
}

/// The 220 x 200 mm bed of `resources/support_test_modifier_normal_in_tree_gui.3mf`,
/// in the Orca point-string spelling its project settings use.
fn bed_220x200() -> ConfigValue {
    ConfigValue::List(vec![
        ConfigValue::String("0x0".into()),
        ConfigValue::String("220x0".into()),
        ConfigValue::String("220x200".into()),
        ConfigValue::String("0x200".into()),
    ])
}

/// Ticket 129 regression, with the failing fixture's measured numbers.
///
/// `support_test_modifier_normal_in_tree_gui.3mf` places the tower at
/// `(3, 172.972)`, `prime_tower_width = 60`, on a 220 x 200 mm bed; the sidecar
/// `..._gui.config.json` sets `line_width = 0.42` and the layer height is 0.2.
/// The purge box is therefore `45 / (0.42 * 0.2 * 60) = 8.93` mm deep, reaching
/// y = 181.90 — comfortably on the bed. The old `width x width` square reached
/// y = 172.972 + 60 = 232.972 and failed the slice with
/// "wipe-tower corner (63.000, 232.972) lies outside bed polygon".
#[test]
fn shallow_tower_near_far_bed_edge_is_accepted() {
    let config = config_from_pairs(&[
        ("enable_prime_tower", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(3.0)),
        ("wipe_tower_y", ConfigValue::Float(172.972)),
        ("prime_tower_width", ConfigValue::Float(60.0)),
        ("prime_volume", ConfigValue::Float(45.0)),
        ("line_width", ConfigValue::Float(0.42)),
        ("retract_length", ConfigValue::Float(2.0)),
        ("printable_area", bed_220x200()),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");
    let sdk_layers: Vec<LayerCollectionView> =
        vec![LayerCollectionView::new(layer_with_tool_change())];
    let mut output = FinalizationOutputBuilder::new();

    let result = tower.run_finalization(&sdk_layers, &mut output, &config);
    assert!(
        result.is_ok(),
        "a 60 x 8.93 mm tower at (3, 172.972) fits the 220 x 200 mm bed and must be \
         accepted; Err means the footprint is still being modelled as a \
         tower_width square: {:?}",
        result.unwrap_err()
    );
}

/// The other side of the same check: a purge volume large enough that the REAL
/// depth overruns the far bed edge must still be rejected, naming the y it
/// reaches. Load-bearing — this fails if the depth term is dropped rather than
/// corrected.
///
/// Same bed and position, `prime_volume = 200` mm^3: depth =
/// `200 / (0.42 * 0.2 * 60) = 39.68` mm, so the far corners sit at
/// y = 212.65 > 200.
#[test]
fn deep_tower_overrunning_far_bed_edge_is_rejected() {
    let config = config_from_pairs(&[
        ("enable_prime_tower", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(3.0)),
        ("wipe_tower_y", ConfigValue::Float(172.972)),
        ("prime_tower_width", ConfigValue::Float(60.0)),
        ("prime_volume", ConfigValue::Float(200.0)),
        ("line_width", ConfigValue::Float(0.42)),
        ("retract_length", ConfigValue::Float(2.0)),
        ("printable_area", bed_220x200()),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");
    let sdk_layers: Vec<LayerCollectionView> =
        vec![LayerCollectionView::new(layer_with_tool_change())];
    let mut output = FinalizationOutputBuilder::new();

    let err = tower
        .run_finalization(&sdk_layers, &mut output, &config)
        .expect_err("a 39.68 mm deep tower at y = 172.972 overruns the 200 mm bed edge");
    assert!(err.fatal, "bed rejection must stay fatal: {}", err.message);
    assert!(
        err.message.contains("212.6"),
        "the error must name the y the real footprint reaches (212.65), not the \
         square's 232.972. Got: {}",
        err.message
    );
}

/// A single-filament print has no tool changes, so no purge path — and
/// therefore no tower footprint to place. It must not be rejected for the
/// placement of a tower that is never built, even with the nominal position
/// off the bed.
#[test]
fn single_filament_print_skips_bed_validation() {
    let config = config_from_pairs(&[
        ("enable_prime_tower", ConfigValue::Bool(true)),
        ("wipe_tower_x", ConfigValue::Float(300.0)),
        ("wipe_tower_y", ConfigValue::Float(300.0)),
        ("prime_tower_width", ConfigValue::Float(60.0)),
        ("prime_volume", ConfigValue::Float(45.0)),
        ("line_width", ConfigValue::Float(0.42)),
        ("retract_length", ConfigValue::Float(2.0)),
        ("printable_area", bed_220x200()),
    ]);

    let tower = WipeTower::from_config(&config).expect("from_config must succeed");
    let sdk_layers: Vec<LayerCollectionView> =
        vec![LayerCollectionView::new(layer_without_tool_change())];
    let mut output = FinalizationOutputBuilder::new();

    assert!(
        tower
            .run_finalization(&sdk_layers, &mut output, &config)
            .is_ok(),
        "no tool changes means no tower geometry; the slice must not fail on the \
         placement of a tower that is never emitted"
    );
}
