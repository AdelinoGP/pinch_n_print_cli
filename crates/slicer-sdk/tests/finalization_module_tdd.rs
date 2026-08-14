//! TDD tests for FinalizationModule trait and WIT bindings.
//!
//! These tests verify the API defined in docs/03_wit_and_manifest.md (world-finalization.wit).
//! Tests lock down trait signatures, view types, and output builders.

use slicer_sdk::prelude::*;
use std::collections::HashMap;
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

// =============================================================================
// Test 1: FinalizationModule trait exists with lifecycle methods
// =============================================================================

struct TestFinalizationModule {
    initialized: bool,
}

impl FinalizationModule for TestFinalizationModule {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let _ = config.len();
        Ok(Self { initialized: true })
    }
}

#[test]
fn test_01_finalization_module_trait_exists_with_from_config() {
    let config = ConfigView::from_map(HashMap::new());

    let module = TestFinalizationModule::from_config(&config).expect("from_config should succeed");
    assert!(module.initialized);
}

// =============================================================================
// Test 2: run_finalization signature matches WIT
// =============================================================================

struct FinalizationTestModule;

impl FinalizationModule for FinalizationTestModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layers.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_02_run_finalization_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = FinalizationTestModule::from_config(&config).unwrap();
    let layers: Vec<LayerCollectionView> = vec![];
    let mut output = FinalizationOutputBuilder::new();

    let result = module.run_finalization(&layers, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 3: LayerCollectionView accessors match WIT resource
// =============================================================================

#[test]
fn test_03_layer_collection_view_accessors() {
    use slicer_ir::LayerCollectionIR;

    let layer_ir = LayerCollectionIR {
        global_layer_index: 42,
        z: 8.4,
        ..Default::default()
    };

    let view = LayerCollectionView::new(layer_ir);
    assert_eq!(view.layer_index(), 42);
    assert!((view.z() - 8.4).abs() < 1e-6);
    assert_eq!(view.entity_count(), 0);
    assert!(view.tool_changes().is_empty());
}

// =============================================================================
// Test 4: FinalizationOutputBuilder push_entity_to_layer
// =============================================================================

#[test]
fn test_04_finalization_output_builder_push_entity() {
    let mut builder = FinalizationOutputBuilder::new();
    let path = ExtrusionPath3D {
        points: vec![],
        ..extrusion_path3d_base(ExtrusionRole::WipeTower)
    };
    let region_key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-1".to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    };

    let result = builder.push_entity_to_layer(0, path, 0, region_key);
    assert!(result.is_ok());
    assert_eq!(builder.entity_pushes().len(), 1);
}

// =============================================================================
// Test 5: FinalizationOutputBuilder insert_synthetic_layer
// =============================================================================

#[test]
fn test_05_finalization_output_builder_insert_synthetic() {
    let mut builder = FinalizationOutputBuilder::new();
    let path = ExtrusionPath3D {
        points: vec![],
        ..extrusion_path3d_base(ExtrusionRole::WipeTower)
    };

    let result = builder.insert_synthetic_layer(5.0, vec![path]);
    assert!(result.is_ok());
    assert_eq!(builder.synthetic_layers().len(), 1);
    assert!((builder.synthetic_layers()[0].0 - 5.0).abs() < 1e-6);
}

// =============================================================================
// Test 6: Default run_finalization is non-panicking Ok(())
// =============================================================================

struct MinimalFinalizationModule;

impl FinalizationModule for MinimalFinalizationModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

#[test]
fn test_06_default_run_finalization_does_not_panic() {
    let config = ConfigView::from_map(HashMap::new());
    let module = MinimalFinalizationModule::from_config(&config).unwrap();
    let layers: Vec<LayerCollectionView> = vec![];
    let mut output = FinalizationOutputBuilder::new();

    let result = module.run_finalization(&layers, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 7: All finalization types accessible via prelude
// =============================================================================

#[test]
fn test_07_prelude_exports_all_finalization_types() {
    fn _check_types() {
        fn _takes_finalization_module<T: FinalizationModule>(_: T) {}

        let _: LayerCollectionView;
        let _: FinalizationOutputBuilder;
    }
}
