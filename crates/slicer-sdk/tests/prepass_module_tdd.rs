//! TDD tests for PrepassModule trait and WIT bindings.
//!
//! These tests verify the API defined in docs/05_module_sdk.md and docs/03_wit_and_manifest.md.
//! Tests lock down trait signatures, prepass types, and output builders.

use slicer_sdk::prelude::*;
use std::collections::HashMap;

// =============================================================================
// Test 1: PrepassModule trait exists with run_mesh_analysis and run_layer_planning
// =============================================================================

/// A test module that implements PrepassModule.
struct TestPrepassModule {
    initialized: bool,
}

impl PrepassModule for TestPrepassModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        // Verify ConfigView is accessible
        let _ = config.fields.len();
        Ok(Self { initialized: true })
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_01_prepass_module_trait_exists_with_lifecycle() {
    // Test that PrepassModule trait can be implemented with on_print_start/on_print_end
    let config = ConfigView {
        fields: HashMap::new(),
    };

    let module = TestPrepassModule::on_print_start(&config).expect("on_print_start should succeed");
    assert!(module.initialized, "module should be initialized");

    module.on_print_end().expect("on_print_end should succeed");
}

// =============================================================================
// Test 2: FacetClass enum has all 6 variants
// =============================================================================

#[test]
fn test_02_facet_class_enum_has_all_variants() {
    // Per docs/03_wit_and_manifest.md (world-prepass.wit):
    // enum facet-class { normal, near-horizontal, overhang, bridge, top-surface, bottom-surface }

    // Test that all variants exist and can be created
    let normal = FacetClass::Normal;
    let near_horizontal = FacetClass::NearHorizontal;
    let overhang = FacetClass::Overhang;
    let bridge = FacetClass::Bridge;
    let top_surface = FacetClass::TopSurface;
    let bottom_surface = FacetClass::BottomSurface;

    // Test equality
    assert_eq!(normal, FacetClass::Normal);
    assert_eq!(near_horizontal, FacetClass::NearHorizontal);
    assert_eq!(overhang, FacetClass::Overhang);
    assert_eq!(bridge, FacetClass::Bridge);
    assert_eq!(top_surface, FacetClass::TopSurface);
    assert_eq!(bottom_surface, FacetClass::BottomSurface);

    // Test inequality
    assert_ne!(normal, overhang);
}

// =============================================================================
// Test 3: FacetAnnotation has facet_index, slope_angle_deg, classification
// =============================================================================

#[test]
fn test_03_facet_annotation_has_required_fields() {
    // Per docs/03_wit_and_manifest.md (world-prepass.wit):
    // record facet-annotation { facet-index: u32, slope-angle-deg: f32, classification: facet-class }

    let annotation = FacetAnnotation {
        facet_index: 42,
        slope_angle_deg: 45.0,
        classification: FacetClass::Overhang,
    };

    assert_eq!(annotation.facet_index, 42);
    assert!((annotation.slope_angle_deg - 45.0).abs() < 1e-6);
    assert_eq!(annotation.classification, FacetClass::Overhang);
}

#[test]
fn test_03b_facet_annotation_constructor() {
    let annotation = FacetAnnotation::new(100, 30.0, FacetClass::Bridge);

    assert_eq!(annotation.facet_index, 100);
    assert!((annotation.slope_angle_deg - 30.0).abs() < 1e-6);
    assert_eq!(annotation.classification, FacetClass::Bridge);
}

// =============================================================================
// Test 4: SurfaceGroupProposal has facet_indices, z_min, z_max, shell_count
// =============================================================================

#[test]
fn test_04_surface_group_proposal_has_required_fields() {
    // Per docs/03_wit_and_manifest.md (world-prepass.wit):
    // record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }

    let group = SurfaceGroupProposal {
        facet_indices: vec![1, 2, 3, 4],
        z_min: 0.0,
        z_max: 10.5,
        shell_count: 2,
    };

    assert_eq!(group.facet_indices, vec![1, 2, 3, 4]);
    assert!((group.z_min - 0.0).abs() < 1e-6);
    assert!((group.z_max - 10.5).abs() < 1e-6);
    assert_eq!(group.shell_count, 2);
}

#[test]
fn test_04b_surface_group_proposal_constructor() {
    let group = SurfaceGroupProposal::new(vec![5, 6, 7], 1.0, 5.0, 3);

    assert_eq!(group.facet_indices, vec![5, 6, 7]);
    assert!((group.z_min - 1.0).abs() < 1e-6);
    assert!((group.z_max - 5.0).abs() < 1e-6);
    assert_eq!(group.shell_count, 3);
}

// =============================================================================
// Test 5: MeshAnalysisOutput::push_facet_annotation
// =============================================================================

#[test]
fn test_05_mesh_analysis_output_push_facet_annotation() {
    let mut output = MeshAnalysisOutput::new();
    let annotation = FacetAnnotation::new(0, 45.0, FacetClass::Normal);

    let result = output.push_facet_annotation("obj-1".to_string(), annotation);
    assert!(result.is_ok());
    assert_eq!(output.facet_annotations().len(), 1);
}

// =============================================================================
// Test 6: MeshAnalysisOutput::push_surface_group
// =============================================================================

#[test]
fn test_06_mesh_analysis_output_push_surface_group() {
    let mut output = MeshAnalysisOutput::new();
    let group = SurfaceGroupProposal::new(vec![0, 1, 2], 0.0, 5.0, 2);

    let result = output.push_surface_group("obj-1".to_string(), group);
    assert!(result.is_ok());
    assert_eq!(output.surface_groups().len(), 1);
}

// =============================================================================
// Test 7: RegionLayerProposal has all required fields
// =============================================================================

#[test]
fn test_07_region_layer_proposal_has_required_fields() {
    // Per docs/03_wit_and_manifest.md (world-prepass.wit):
    // record region-layer-proposal {
    //     object-id: object-id, region-id: region-id,
    //     effective-layer-height: f32,
    //     is-catchup: bool, catchup-z-bottom: f32,
    // }

    let proposal = RegionLayerProposal {
        object_id: "obj-1".to_string(),
        region_id: "region-42".to_string(),
        effective_layer_height: 0.2,
        is_catchup: false,
        catchup_z_bottom: 0.0,
    };

    assert_eq!(proposal.object_id, "obj-1");
    assert_eq!(proposal.region_id, "region-42");
    assert!((proposal.effective_layer_height - 0.2).abs() < 1e-6);
    assert!(!proposal.is_catchup);
    assert!((proposal.catchup_z_bottom - 0.0).abs() < 1e-6);
}

#[test]
fn test_07b_region_layer_proposal_catchup() {
    let proposal = RegionLayerProposal::new(
        "obj-2".to_string(),
        "region-99".to_string(),
        0.15,
        true,
        1.5,
    );

    assert_eq!(proposal.object_id, "obj-2");
    assert_eq!(proposal.region_id, "region-99");
    assert!((proposal.effective_layer_height - 0.15).abs() < 1e-6);
    assert!(proposal.is_catchup);
    assert!((proposal.catchup_z_bottom - 1.5).abs() < 1e-6);
}

// =============================================================================
// Test 8: LayerProposal has z and active_regions
// =============================================================================

#[test]
fn test_08_layer_proposal_has_required_fields() {
    // Per docs/03_wit_and_manifest.md (world-prepass.wit):
    // record layer-proposal { z: f32, active-regions: list<region-layer-proposal> }

    let region =
        RegionLayerProposal::new("obj-1".to_string(), "region-1".to_string(), 0.2, false, 0.0);

    let proposal = LayerProposal {
        z: 0.2,
        active_regions: vec![region],
    };

    assert!((proposal.z - 0.2).abs() < 1e-6);
    assert_eq!(proposal.active_regions.len(), 1);
}

#[test]
fn test_08b_layer_proposal_constructor() {
    let regions = vec![
        RegionLayerProposal::new("obj-1".to_string(), "r1".to_string(), 0.2, false, 0.0),
        RegionLayerProposal::new("obj-2".to_string(), "r2".to_string(), 0.2, false, 0.0),
    ];

    let proposal = LayerProposal::new(0.4, regions);

    assert!((proposal.z - 0.4).abs() < 1e-6);
    assert_eq!(proposal.active_regions.len(), 2);
}

// =============================================================================
// Test 9: LayerPlanOutput::push_layer
// =============================================================================

#[test]
fn test_09_layer_plan_output_push_layer() {
    let mut output = LayerPlanOutput::new();
    let proposal = LayerProposal::new(0.2, vec![]);

    let result = output.push_layer(proposal);
    assert!(result.is_ok());
    assert_eq!(output.layers().len(), 1);
}

// =============================================================================
// Test 10: run_mesh_analysis signature matches WIT
// =============================================================================

struct MeshAnalysisTestModule;

impl PrepassModule for MeshAnalysisTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_mesh_analysis(
        &self,
        objects: &[ObjectId],
        output: &mut MeshAnalysisOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // This tests that the signature compiles correctly
        let _ = objects.len();
        let _ = output;
        let _ = config.fields.len();
        Ok(())
    }
}

#[test]
fn test_10_run_mesh_analysis_signature_matches_wit() {
    let config = ConfigView {
        fields: HashMap::new(),
    };
    let module = MeshAnalysisTestModule::on_print_start(&config).unwrap();
    let objects: Vec<ObjectId> = vec!["obj-1".to_string()];
    let mut output = MeshAnalysisOutput::new();

    let result = module.run_mesh_analysis(&objects, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 11: run_layer_planning signature matches WIT
// =============================================================================

struct LayerPlanningTestModule;

impl PrepassModule for LayerPlanningTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_layer_planning(
        &self,
        objects: &[ObjectId],
        output: &mut LayerPlanOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // This tests that the signature compiles correctly
        let _ = objects.len();
        let _ = output;
        let _ = config.fields.len();
        Ok(())
    }
}

#[test]
fn test_11_run_layer_planning_signature_matches_wit() {
    let config = ConfigView {
        fields: HashMap::new(),
    };
    let module = LayerPlanningTestModule::on_print_start(&config).unwrap();
    let objects: Vec<ObjectId> = vec!["obj-1".to_string()];
    let mut output = LayerPlanOutput::new();

    let result = module.run_layer_planning(&objects, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 12: Default implementations exist for both run methods
// =============================================================================

struct MinimalPrepassModule;

impl PrepassModule for MinimalPrepassModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    // Both run_mesh_analysis and run_layer_planning have default implementations
}

#[test]
fn test_12_default_implementations_exist() {
    let config = ConfigView {
        fields: HashMap::new(),
    };
    let module = MinimalPrepassModule::on_print_start(&config).unwrap();
    let objects: Vec<ObjectId> = vec![];
    let mut mesh_output = MeshAnalysisOutput::new();
    let mut layer_output = LayerPlanOutput::new();

    // Both default implementations should succeed
    let mesh_result = module.run_mesh_analysis(&objects, &mut mesh_output, &config);
    assert!(mesh_result.is_ok());

    let layer_result = module.run_layer_planning(&objects, &mut layer_output, &config);
    assert!(layer_result.is_ok());
}

// =============================================================================
// Test 13: Trait can be implemented on a custom struct (compile test)
// =============================================================================

struct CustomPrepassModule {
    overhang_threshold_deg: f32,
}

impl PrepassModule for CustomPrepassModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let threshold = config
            .fields
            .get("overhang_threshold_deg")
            .and_then(|v| match v {
                ConfigValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(45.0);
        Ok(Self {
            overhang_threshold_deg: threshold,
        })
    }

    fn run_mesh_analysis(
        &self,
        _objects: &[ObjectId],
        _output: &mut MeshAnalysisOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if self.overhang_threshold_deg <= 0.0 || self.overhang_threshold_deg > 90.0 {
            return Err(ModuleError::fatal(
                1,
                "overhang_threshold_deg must be in (0, 90]",
            ));
        }
        Ok(())
    }
}

#[test]
fn test_13_custom_module_implementation() {
    let mut fields = HashMap::new();
    fields.insert(
        "overhang_threshold_deg".to_string(),
        ConfigValue::Float(60.0),
    );
    let config = ConfigView { fields };

    let module = CustomPrepassModule::on_print_start(&config).expect("should create module");
    assert!((module.overhang_threshold_deg - 60.0).abs() < 1e-6);
}

// =============================================================================
// Test 14: All types are accessible via slicer_sdk::prelude::*
// =============================================================================

#[test]
fn test_14_prelude_exports_all_prepass_types() {
    // Verify all prepass types are accessible via prelude
    fn _check_types() {
        // PrepassModule is a trait, so we check it via a function signature
        fn _takes_prepass_module<T: PrepassModule>(_: T) {}

        let _: FacetClass;
        let _: FacetAnnotation;
        let _: SurfaceGroupProposal;
        let _: RegionLayerProposal;
        let _: LayerProposal;
        let _: MeshAnalysisOutput;
        let _: LayerPlanOutput;
        let _: ObjectId;
    }
}

#[test]
fn test_14b_prelude_types_are_constructible() {
    // Verify types can be constructed via prelude imports
    let _class = FacetClass::Normal;
    let _annotation = FacetAnnotation::new(0, 0.0, FacetClass::Normal);
    let _group = SurfaceGroupProposal::new(vec![], 0.0, 0.0, 0);
    let _region = RegionLayerProposal::new("".to_string(), "".to_string(), 0.0, false, 0.0);
    let _layer = LayerProposal::new(0.0, vec![]);
    let _mesh_output = MeshAnalysisOutput::new();
    let _layer_output = LayerPlanOutput::new();
    let _object_id: ObjectId = "test".to_string();
}
