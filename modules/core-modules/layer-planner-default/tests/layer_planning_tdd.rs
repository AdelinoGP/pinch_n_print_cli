//! TDD tests for DefaultLayerPlanner (TASK-070).
//!
//! Tests verify uniform layer planning, multi-object LCM sync, catch-up layers,
//! and error cases per docs/02_ir_schemas.md and docs/05_module_sdk.md.

use layer_planner_default::DefaultLayerPlanner;
use slicer_sdk::prelude::*;
use slicer_sdk::test_prelude::*;

/// Helper: build a ConfigView with the given parameters.
fn make_config(layer_height: f64, first_layer_height: f64, objects: &[(&str, f64)]) -> ConfigView {
    objects
        .iter()
        .fold(
            ConfigViewBuilder::new()
                .float("layer_height", layer_height)
                .float("first_layer_height", first_layer_height),
            |b, (id, h)| b.float(format!("object_height:{}", id), *h),
        )
        .build()
}

/// Helper: build config with per-object layer height overrides.
fn make_lh_config(
    default_layer_height: f64,
    first_layer_height: f64,
    objects: &[(&str, f64, f64)],
) -> ConfigView {
    objects
        .iter()
        .fold(
            ConfigViewBuilder::new()
                .float("layer_height", default_layer_height)
                .float("first_layer_height", first_layer_height),
            |b, (id, h, lh)| {
                b.float(format!("object_height:{}", id), *h)
                    .float(format!("layer_height:{}", id), *lh)
            },
        )
        .build()
}

// =============================================================================
// Test 1: Single object, uniform layers
// =============================================================================

#[test]
fn test_single_object_uniform_layers() {
    // 1 object, 2mm tall, layer_height=0.2 → 10 layers, ascending Z
    let config = make_config(0.2, 0.2, &[("obj-1", 2.0)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-1".to_string()];
    let mut output = LayerPlanOutput::new();

    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("should succeed");

    let layers = output.layers();
    assert_eq!(layers.len(), 10, "2mm / 0.2mm = 10 layers");

    // Ascending Z
    for i in 1..layers.len() {
        assert!(
            layers[i].z > layers[i - 1].z,
            "Z must be strictly ascending: layer {} z={} vs layer {} z={}",
            i - 1,
            layers[i - 1].z,
            i,
            layers[i].z,
        );
    }

    // First layer at first_layer_height
    assert!(
        (layers[0].z - 0.2).abs() < 1e-4,
        "first layer z={}, expected 0.2",
        layers[0].z,
    );

    // Each layer has obj-1 participating
    for layer in layers {
        assert_eq!(layer.active_regions.len(), 1);
        assert_eq!(layer.active_regions[0].object_id, "obj-1");
    }
}

// =============================================================================
// Test 2: First layer height respected
// =============================================================================

#[test]
fn test_first_layer_height_respected() {
    // first_layer=0.3, rest=0.2 → layer 0 z=0.3, layer 1 z=0.5, ...
    let config = make_config(0.2, 0.3, &[("obj-1", 2.0)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-1".to_string()];
    let mut output = LayerPlanOutput::new();

    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("should succeed");

    let layers = output.layers();

    // First layer Z = first_layer_height = 0.3
    assert!(
        (layers[0].z - 0.3).abs() < 1e-4,
        "first layer z={}, expected 0.3",
        layers[0].z,
    );

    // First layer effective_layer_height = 0.3
    assert!(
        (layers[0].active_regions[0].effective_layer_height - 0.3).abs() < 1e-4,
        "first layer effective_layer_height={}, expected 0.3",
        layers[0].active_regions[0].effective_layer_height,
    );

    // Second layer Z = 0.3 + 0.2 = 0.5
    assert!(
        (layers[1].z - 0.5).abs() < 1e-4,
        "second layer z={}, expected 0.5",
        layers[1].z,
    );

    // Second layer effective_layer_height = 0.2
    assert!(
        (layers[1].active_regions[0].effective_layer_height - 0.2).abs() < 1e-4,
        "second layer effective_layer_height={}, expected 0.2",
        layers[1].active_regions[0].effective_layer_height,
    );

    // Total layers: first at 0.3, then (2.0 - 0.3) / 0.2 = 8.5 → 9 more = 10 total
    // Actually: 0.3, 0.5, 0.7, 0.9, 1.1, 1.3, 1.5, 1.7, 1.9 = 9 layers
    assert_eq!(
        layers.len(),
        9,
        "0.3 + 8*0.2 = 1.9 ≤ 2.0, 0.3 + 9*0.2 = 2.1 > 2.0"
    );
}

// =============================================================================
// Test 3: Multi-object, same layer height
// =============================================================================

#[test]
fn test_multi_object_same_height() {
    // 2 objects at same 0.2mm layer height, different object heights (1.0 and 2.0)
    let config = make_config(0.2, 0.2, &[("obj-A", 1.0), ("obj-B", 2.0)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-A".to_string(), "obj-B".to_string()];
    let mut output = LayerPlanOutput::new();

    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("should succeed");

    let layers = output.layers();

    // Total layers = max(1.0, 2.0) / 0.2 = 10
    assert_eq!(layers.len(), 10);

    // First 5 layers (z=0.2..1.0) both objects participate
    for layer in &layers[..5] {
        assert_eq!(
            layer.active_regions.len(),
            2,
            "both objects should participate at z={}",
            layer.z
        );
    }

    // Last 5 layers (z=1.2..2.0) only obj-B participates
    for layer in &layers[5..] {
        assert_eq!(
            layer.active_regions.len(),
            1,
            "only obj-B should participate at z={}",
            layer.z
        );
        assert_eq!(layer.active_regions[0].object_id, "obj-B");
    }

    // No catch-up layers when same layer height
    for layer in layers {
        for region in &layer.active_regions {
            assert!(
                !region.is_catchup,
                "no catch-up needed when layer heights match"
            );
        }
    }
}

// =============================================================================
// Test 4: Multi-object LCM sync
// =============================================================================

#[test]
fn test_multi_object_lcm_sync() {
    // Object A at 0.2mm, Object B at 0.3mm → LCM sync at 0.6mm multiples
    // Both objects 1.2mm tall to get clean layer counts
    let config = make_lh_config(0.2, 0.2, &[("obj-A", 1.2, 0.2), ("obj-B", 1.2, 0.3)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-A".to_string(), "obj-B".to_string()];
    let mut output = LayerPlanOutput::new();

    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("should succeed");

    let layers = output.layers();

    // Both objects should have layers
    assert!(!layers.is_empty());

    // There should be sync points at 0.6mm multiples where both participate
    let sync_layers: Vec<_> = layers
        .iter()
        .filter(|l| {
            let z_after_first = l.z - 0.2;
            z_after_first >= -1e-4 && (z_after_first % 0.6).abs() < 0.05
        })
        .collect();

    assert!(
        !sync_layers.is_empty(),
        "should have sync layers at LCM=0.6mm intervals"
    );

    // There should be at least one catch-up layer somewhere
    let has_catchup = layers
        .iter()
        .any(|l| l.active_regions.iter().any(|r| r.is_catchup));
    assert!(has_catchup, "LCM sync should produce catch-up layers");

    for layer in layers {
        for region in &layer.active_regions {
            assert_eq!(
                region.region_id, "0",
                "layer planner must emit canonical decimal region ids on mixed-height paths"
            );
        }
    }
}

// =============================================================================
// Test 5: Catch-up layer fields
// =============================================================================

#[test]
fn test_catch_up_layer_fields() {
    // Object A at 0.2mm, Object B at 0.3mm — catch-up layers need correct fields
    let config = make_lh_config(0.2, 0.2, &[("obj-A", 1.2, 0.2), ("obj-B", 1.2, 0.3)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-A".to_string(), "obj-B".to_string()];
    let mut output = LayerPlanOutput::new();

    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("should succeed");

    let layers = output.layers();

    // Find a catch-up layer
    let catchup_region = layers
        .iter()
        .flat_map(|l| {
            l.active_regions
                .iter()
                .filter(|r| r.is_catchup)
                .map(move |r| (l.z, r))
        })
        .next();

    assert!(
        catchup_region.is_some(),
        "should have at least one catch-up region"
    );

    let (layer_z, region) = catchup_region.unwrap();

    // is_catchup must be true
    assert!(region.is_catchup);

    // catchup_z_bottom must be less than current layer Z
    assert!(
        region.catchup_z_bottom < layer_z,
        "catchup_z_bottom ({}) must be < layer z ({})",
        region.catchup_z_bottom,
        layer_z,
    );

    // effective_layer_height should equal z - catchup_z_bottom
    let expected_height = layer_z - region.catchup_z_bottom;
    assert!(
        (region.effective_layer_height - expected_height).abs() < 1e-4,
        "effective_layer_height ({}) should equal z - catchup_z_bottom ({})",
        region.effective_layer_height,
        expected_height,
    );
}

// =============================================================================
// Test 6: Empty objects returns error
// =============================================================================

#[test]
fn test_empty_objects_error() {
    let config = make_config(0.2, 0.2, &[]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec![];
    let mut output = LayerPlanOutput::new();

    let result = module.run_layer_planning(&objects, &mut output, &config);
    assert!(result.is_err(), "empty objects should return error");

    let err = result.unwrap_err();
    assert!(err.fatal, "empty objects error should be fatal");
}

// =============================================================================
// Test 7: Zero layer height returns error
// =============================================================================

#[test]
fn test_zero_layer_height_error() {
    let config = make_config(0.0, 0.2, &[("obj-1", 2.0)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-1".to_string()];
    let mut output = LayerPlanOutput::new();

    let result = module.run_layer_planning(&objects, &mut output, &config);
    assert!(result.is_err(), "zero layer height should return error");

    let err = result.unwrap_err();
    assert!(err.fatal, "zero layer height error should be fatal");
}

// =============================================================================
// Test 8: Object participation map (correct local/global index mapping)
// =============================================================================

#[test]
fn test_object_participation_map() {
    // 1 object, 2mm tall, 0.2mm layers → 10 layers
    // Each layer should have obj-1 with correct effective_layer_height
    let config = make_config(0.2, 0.2, &[("obj-1", 2.0)]);
    let module = DefaultLayerPlanner::on_print_start(&config).unwrap();

    let objects: Vec<ObjectId> = vec!["obj-1".to_string()];
    let mut output = LayerPlanOutput::new();

    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("should succeed");

    let layers = output.layers();
    assert_eq!(layers.len(), 10);

    // Verify Z values match expected sequence
    for (i, layer) in layers.iter().enumerate() {
        let expected_z = 0.2 * (i as f32 + 1.0);
        assert!(
            (layer.z - expected_z).abs() < 1e-4,
            "layer {} z={}, expected {}",
            i,
            layer.z,
            expected_z,
        );
    }

    // Verify all regions have object_id = "obj-1" and consistent effective_layer_height
    for layer in layers {
        assert_eq!(layer.active_regions.len(), 1);
        let r = &layer.active_regions[0];
        assert_eq!(r.object_id, "obj-1");
        assert!((r.effective_layer_height - 0.2).abs() < 1e-4);
        assert!(!r.is_catchup);
        assert!((r.catchup_z_bottom - 0.0).abs() < 1e-6);
    }
}
