//! TDD tests for LayerModule trait and WIT bindings.
//!
//! These tests verify the API defined in docs/05_module_sdk.md and docs/03_wit_and_manifest.md.
//! Tests lock down trait signatures, ModuleError type, view types, and output builders.

use slicer_sdk::prelude::*;
use std::collections::HashMap;

// =============================================================================
// Test 1: LayerModule trait exists with on_print_start and on_print_end
// =============================================================================

/// A test module that implements LayerModule.
struct TestModule {
    initialized: bool,
}

impl LayerModule for TestModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        // Verify ConfigView is accessible
        let _ = config.len();
        Ok(Self { initialized: true })
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_01_layer_module_trait_exists_with_lifecycle() {
    // Test that LayerModule trait can be implemented with on_print_start/on_print_end
    let config = ConfigView::from_map(HashMap::new());

    let module = TestModule::on_print_start(&config).expect("on_print_start should succeed");
    assert!(module.initialized, "module should be initialized");

    module.on_print_end().expect("on_print_end should succeed");
}

// =============================================================================
// Test 2: ModuleError has fatal() and non_fatal() constructors
// =============================================================================

#[test]
fn test_02_module_error_fatal_constructor() {
    let err = ModuleError::fatal(1, "fatal error message");
    assert_eq!(err.code, 1);
    assert!(err.fatal, "fatal() should set fatal=true");
    assert_eq!(err.message, "fatal error message");
}

#[test]
fn test_03_module_error_non_fatal_constructor() {
    let err = ModuleError::non_fatal(2, "non-fatal error message");
    assert_eq!(err.code, 2);
    assert!(!err.fatal, "non_fatal() should set fatal=false");
    assert_eq!(err.message, "non-fatal error message");
}

#[test]
fn test_04_module_error_from_str_constructor() {
    let err = ModuleError::from_str("string error");
    assert!(!err.fatal, "from_str() should be non-fatal");
    assert_eq!(err.message, "string error");
}

// =============================================================================
// Test 3: ModuleError has code, message, and fatal fields
// =============================================================================

#[test]
fn test_05_module_error_fields() {
    let err = ModuleError {
        code: 42,
        message: "test message".to_string(),
        fatal: true,
    };

    assert_eq!(err.code, 42);
    assert_eq!(err.message, "test message");
    assert!(err.fatal);
}

// =============================================================================
// Test 4: run_infill signature matches WIT
// =============================================================================

/// Per docs/03_wit_and_manifest.md:
/// ```wit
/// export run-infill: func(
///     layer-index: layer-idx,
///     regions: list<slice-region-view>,
///     output: infill-output-builder,
///     config: config-view,
/// ) -> result<_, module-error>;
/// ```
struct InfillTestModule;

impl LayerModule for InfillTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // This tests that the signature compiles correctly
        let _ = layer_index;
        let _ = regions.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_06_run_infill_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = InfillTestModule::on_print_start(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let mut output = InfillOutputBuilder::new();

    let result = module.run_infill(0, &regions, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 5: run_perimeters signature matches WIT
// =============================================================================

struct PerimeterTestModule;

impl LayerModule for PerimeterTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_perimeters(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut PerimeterOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = paint.layer_index();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_07_run_perimeters_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = PerimeterTestModule::on_print_start(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let result = module.run_perimeters(0, &regions, &paint, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 6: run_wall_postprocess signature matches WIT
// =============================================================================

struct WallPostprocessTestModule;

impl LayerModule for WallPostprocessTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_wall_postprocess(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut PerimeterOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_08_run_wall_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = WallPostprocessTestModule::on_print_start(&config).unwrap();
    let regions: Vec<PerimeterRegionView> = vec![];
    let mut output = PerimeterOutputBuilder::new();

    let result = module.run_wall_postprocess(0, &regions, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 7: run_infill_postprocess signature matches WIT
// =============================================================================

struct InfillPostprocessTestModule;

impl LayerModule for InfillPostprocessTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill_postprocess(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_09_run_infill_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = InfillPostprocessTestModule::on_print_start(&config).unwrap();
    let regions: Vec<PerimeterRegionView> = vec![];
    let mut output = InfillOutputBuilder::new();

    let result = module.run_infill_postprocess(0, &regions, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 8: run_slice_postprocess signature matches WIT
// =============================================================================

struct SlicePostprocessTestModule;

impl LayerModule for SlicePostprocessTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_slice_postprocess(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut SlicePostprocessBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = paint.layer_index();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_10_run_slice_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SlicePostprocessTestModule::on_print_start(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);
    let mut output = SlicePostprocessBuilder::new();

    let result = module.run_slice_postprocess(0, &regions, &paint, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 9: run_support signature matches WIT
// =============================================================================

struct SupportTestModule;

impl LayerModule for SupportTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut SupportOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = paint.layer_index();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_11_run_support_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SupportTestModule::on_print_start(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    let result = module.run_support(0, &regions, &paint, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 10: SliceRegionView methods per docs/03_wit_and_manifest.md
// =============================================================================

#[test]
fn test_12_slice_region_view_object_id() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    assert_eq!(view.object_id(), "obj-1");
}

#[test]
fn test_13_slice_region_view_region_id() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(42);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    assert_eq!(*view.region_id(), 42);
}

#[test]
fn test_14_slice_region_view_polygons() {
    let poly = ExPolygon {
        contour: Polygon {
            points: vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
        },
        holes: vec![],
    };
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![poly.clone()]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    assert_eq!(view.polygons().len(), 1);
}

#[test]
fn test_15_slice_region_view_infill_areas() {
    let poly = ExPolygon {
        contour: Polygon {
            points: vec![Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
        },
        holes: vec![],
    };
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![poly.clone()]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    assert_eq!(view.infill_areas().len(), 1);
}

#[test]
fn test_16_slice_region_view_z() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.5);
    view.set_has_nonplanar(false);
    assert!((view.z() - 1.5).abs() < 1e-6);
}

#[test]
fn test_17_slice_region_view_effective_layer_height() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.25);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    assert!((view.effective_layer_height() - 0.25).abs() < 1e-6);
}

#[test]
fn test_18_slice_region_view_has_nonplanar() {
    let mut view_planar = SliceRegionView::default();
    view_planar.set_object_id("obj-1".to_string());
    view_planar.set_region_id(0);
    view_planar.set_polygons(vec![]);
    view_planar.set_infill_areas(vec![]);
    view_planar.set_effective_layer_height(0.2);
    view_planar.set_z(1.0);
    view_planar.set_has_nonplanar(false);
    assert!(!view_planar.has_nonplanar());

    let view_nonplanar = {
        let mut tmp = SliceRegionView::default();
        tmp.set_object_id("obj-1".to_string());
        tmp.set_region_id(0);
        tmp.set_polygons(vec![]);
        tmp.set_infill_areas(vec![]);
        tmp.set_effective_layer_height(0.2);
        tmp.set_z(1.0);
        tmp.set_has_nonplanar(true);
        tmp
    };
    assert!(view_nonplanar.has_nonplanar());
}

// =============================================================================
// SurfaceClassificationIR-derived needs_support flag
// (docs/02_ir_schemas.md and docs/01_system_architecture.md §"Layer::Support")
// =============================================================================

#[test]
fn slice_region_view_needs_support_defaults_true() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    assert!(
        view.needs_support(),
        "default constructor must keep needs_support=true to preserve pre-classification eligibility"
    );
}

#[test]
fn slice_region_view_set_needs_support_overrides() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_region_id(0);
    view.set_polygons(vec![]);
    view.set_infill_areas(vec![]);
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    view.set_needs_support(false);
    assert!(!view.needs_support());
    view.set_needs_support(true);
    assert!(view.needs_support());
}

// =============================================================================
// Test 11: PerimeterRegionView methods per docs/03_wit_and_manifest.md
// =============================================================================

#[test]
fn test_19_perimeter_region_view_object_id() {
    let mut view = PerimeterRegionView::default();
    view.set_object_id("obj-2".to_string());
    view.set_region_id(0);
    view.set_wall_loops(vec![]);
    view.set_infill_areas(vec![]);
    view.set_seam_candidates(vec![]);
    view.set_resolved_seam(None);
    assert_eq!(view.object_id(), "obj-2");
}

#[test]
fn test_20_perimeter_region_view_region_id() {
    let mut view = PerimeterRegionView::default();
    view.set_object_id("obj-2".to_string());
    view.set_region_id(99);
    view.set_wall_loops(vec![]);
    view.set_infill_areas(vec![]);
    view.set_seam_candidates(vec![]);
    view.set_resolved_seam(None);
    assert_eq!(*view.region_id(), 99);
}

#[test]
fn test_21_perimeter_region_view_wall_loops() {
    let wall = WallLoop {
        perimeter_index: 0,
        loop_type: slicer_ir::LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: slicer_ir::WidthProfile { widths: vec![] },
        feature_flags: vec![],
        boundary_type: slicer_ir::WallBoundaryType::ExteriorSurface,
    };
    let mut view = PerimeterRegionView::default();
    view.set_object_id("obj-2".to_string());
    view.set_region_id(0);
    view.set_wall_loops(vec![wall]);
    view.set_infill_areas(vec![]);
    view.set_seam_candidates(vec![]);
    view.set_resolved_seam(None);
    assert_eq!(view.wall_loops().len(), 1);
}

#[test]
fn test_22_perimeter_region_view_infill_areas() {
    let poly = ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    };
    let mut view = PerimeterRegionView::default();
    view.set_object_id("obj-2".to_string());
    view.set_region_id(0);
    view.set_wall_loops(vec![]);
    view.set_infill_areas(vec![poly]);
    view.set_seam_candidates(vec![]);
    view.set_resolved_seam(None);
    assert_eq!(view.infill_areas().len(), 1);
}

// =============================================================================
// Test 12: InfillOutputBuilder methods per docs/03_wit_and_manifest.md
// =============================================================================

#[test]
fn test_23_infill_output_builder_push_sparse_path() {
    let mut builder = InfillOutputBuilder::new();
    let path = ExtrusionPath3D {
        points: vec![],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    };
    let result = builder.push_sparse_path(path);
    assert!(result.is_ok());
    assert_eq!(builder.sparse_paths().len(), 1);
}

#[test]
fn test_24_infill_output_builder_push_solid_path() {
    let mut builder = InfillOutputBuilder::new();
    let path = ExtrusionPath3D {
        points: vec![],
        role: ExtrusionRole::TopSolidInfill,
        speed_factor: 1.0,
    };
    let result = builder.push_solid_path(path);
    assert!(result.is_ok());
    assert_eq!(builder.solid_paths().len(), 1);
}

#[test]
fn test_25_infill_output_builder_push_ironing_path() {
    let mut builder = InfillOutputBuilder::new();
    let path = ExtrusionPath3D {
        points: vec![],
        role: ExtrusionRole::Ironing,
        speed_factor: 1.0,
    };
    let result = builder.push_ironing_path(path);
    assert!(result.is_ok());
    assert_eq!(builder.ironing_paths().len(), 1);
}

// =============================================================================
// Test 13: PerimeterOutputBuilder methods per docs/03_wit_and_manifest.md
// =============================================================================

#[test]
fn test_26_perimeter_output_builder_push_wall_loop() {
    let mut builder = PerimeterOutputBuilder::new();
    let wall = WallLoop {
        perimeter_index: 0,
        loop_type: slicer_ir::LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: slicer_ir::WidthProfile { widths: vec![] },
        feature_flags: vec![],
        boundary_type: slicer_ir::WallBoundaryType::ExteriorSurface,
    };
    let result = builder.push_wall_loop(wall);
    assert!(result.is_ok());
    assert_eq!(builder.wall_loops().len(), 1);
}

#[test]
fn test_27_perimeter_output_builder_set_infill_areas() {
    let mut builder = PerimeterOutputBuilder::new();
    let areas = vec![ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    }];
    let result = builder.set_infill_areas(areas);
    assert!(result.is_ok());
    assert_eq!(builder.infill_areas().len(), 1);
}

#[test]
fn test_28_perimeter_output_builder_push_seam_candidate() {
    let mut builder = PerimeterOutputBuilder::new();
    let pos = Point3 {
        x: 1.0,
        y: 2.0,
        z: 0.5,
    };
    let result = builder.push_seam_candidate(pos, 0.8);
    assert!(result.is_ok());
    assert_eq!(builder.seam_candidates().len(), 1);
}

// =============================================================================
// Test 14: Trait can be implemented on a custom struct (compile test)
// =============================================================================

struct CustomModule {
    density: f32,
}

impl LayerModule for CustomModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = config
            .get("density")
            .and_then(|v| match v {
                ConfigValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(0.15);
        Ok(Self { density })
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if self.density <= 0.0 || self.density >= 1.0 {
            return Err(ModuleError::fatal(1, "density must be in (0, 1)"));
        }
        Ok(())
    }
}

#[test]
fn test_29_custom_module_implementation() {
    let mut fields = HashMap::new();
    fields.insert("density".to_string(), ConfigValue::Float(0.2));
    let config = ConfigView::from_map(fields);

    let module = CustomModule::on_print_start(&config).expect("should create module");
    assert!((module.density - 0.2).abs() < 1e-6);
}

// =============================================================================
// Test 15: All types are accessible via slicer_sdk::prelude::*
// =============================================================================

#[test]
fn test_30_prelude_exports_all_types() {
    // Verify all types are accessible via prelude
    fn _check_types() {
        let _: ModuleError;
        let _: fn(&ConfigView) -> Result<TestModule, ModuleError> = TestModule::on_print_start;
        let _: SliceRegionView;
        let _: PerimeterRegionView;
        let _: InfillOutputBuilder;
        let _: PerimeterOutputBuilder;
        let _: SlicePostprocessBuilder;
        let _: SupportOutputBuilder;
        let _: PaintRegionLayerView;
    }
}

// =============================================================================
// Test 16: Default implementations exist for optional methods
// =============================================================================

struct MinimalModule;

impl LayerModule for MinimalModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    // on_print_end has default implementation
}

#[test]
fn test_31_default_on_print_end_implementation() {
    let config = ConfigView::from_map(HashMap::new());
    let module = MinimalModule::on_print_start(&config).unwrap();

    // Should use default implementation and succeed
    let result = module.on_print_end();
    assert!(result.is_ok());
}

// =============================================================================
// Test 17: run_support_postprocess signature matches WIT
// =============================================================================

struct SupportPostprocessTestModule;

impl LayerModule for SupportPostprocessTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support_postprocess(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut SupportOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_33_run_support_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SupportPostprocessTestModule::on_print_start(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let mut output = SupportOutputBuilder::new();

    let result = module.run_support_postprocess(0, &regions, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 18: run_path_optimization signature matches WIT
// =============================================================================

struct PathOptimizationTestModule;

impl LayerModule for PathOptimizationTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_path_optimization(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        collection: &mut LayerCollectionBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = output;
        let _ = collection;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_34_run_path_optimization_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = PathOptimizationTestModule::on_print_start(&config).unwrap();
    let regions: Vec<PerimeterRegionView> = vec![];
    let mut output = GcodeOutputBuilder::new();
    let mut collection = LayerCollectionBuilder::new();

    let result = module.run_path_optimization(0, &regions, &mut output, &mut collection, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Packet 32 — TASK-152g: SDK builder rejects a second set_entity_order
// =============================================================================

#[test]
fn set_entity_order_second_call_returns_err() {
    let mut collection = LayerCollectionBuilder::new();
    let first = vec![(0u32, false), (1, false), (2, false)];
    let second = vec![(2u32, false), (1, false), (0, false)];

    collection
        .set_entity_order(first.clone())
        .expect("first set_entity_order must succeed");

    let err = collection
        .set_entity_order(second)
        .expect_err("second set_entity_order must return Err");
    assert!(
        err.contains("set-entity-order called twice"),
        "diagnostic must mention the double-call rule; got: {err}"
    );

    assert_eq!(
        collection.proposal(),
        Some(first.as_slice()),
        "first proposal must be preserved after rejected second call"
    );
}

// =============================================================================
// Test 19: LayerModule defaults are non-panicking Ok(())
// =============================================================================

#[test]
fn test_35_layer_module_defaults_do_not_panic() {
    let config = ConfigView::from_map(HashMap::new());
    let module = MinimalModule::on_print_start(&config).unwrap();
    let slice_regions: Vec<SliceRegionView> = vec![];
    let perim_regions: Vec<PerimeterRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);

    // All default methods should return Ok(()) without panicking
    assert!(module
        .run_infill(0, &slice_regions, &mut InfillOutputBuilder::new(), &config)
        .is_ok());
    assert!(module
        .run_perimeters(
            0,
            &slice_regions,
            &paint,
            &mut PerimeterOutputBuilder::new(),
            &config
        )
        .is_ok());
    assert!(module
        .run_wall_postprocess(
            0,
            &perim_regions,
            &mut PerimeterOutputBuilder::new(),
            &config
        )
        .is_ok());
    assert!(module
        .run_infill_postprocess(0, &perim_regions, &mut InfillOutputBuilder::new(), &config)
        .is_ok());
    assert!(module
        .run_slice_postprocess(
            0,
            &slice_regions,
            &paint,
            &mut SlicePostprocessBuilder::new(),
            &config
        )
        .is_ok());
    assert!(module
        .run_support(
            0,
            &slice_regions,
            &paint,
            &mut SupportOutputBuilder::new(),
            &config
        )
        .is_ok());
    assert!(module
        .run_support_postprocess(0, &slice_regions, &mut SupportOutputBuilder::new(), &config)
        .is_ok());
    assert!(module
        .run_path_optimization(
            0,
            &perim_regions,
            &mut GcodeOutputBuilder::new(),
            &mut LayerCollectionBuilder::new(),
            &config,
        )
        .is_ok());
}

// =============================================================================
// Test 20: Multiple LayerModule implementations can coexist
// =============================================================================

struct ModuleA;
struct ModuleB;

impl LayerModule for ModuleA {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

impl LayerModule for ModuleB {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

#[test]
fn test_32_multiple_implementations_coexist() {
    let config = ConfigView::from_map(HashMap::new());

    let _a = ModuleA::on_print_start(&config).unwrap();
    let _b = ModuleB::on_print_start(&config).unwrap();
}

// =============================================================================
// SDK cache contract: LayerCollectionBuilder::get_ordered_entities returns
// the snapshot stashed by set_ordered_entities and serves repeated reads
// from the local cache without round-tripping anywhere.
// =============================================================================

#[test]
fn layer_collection_builder_get_ordered_entities_reads_local_cache() {
    let mut builder = slicer_sdk::LayerCollectionBuilder::new();
    let snapshot = vec![
        OrderedEntityView {
            original_index: 0,
            region_key: RegionKey {
                global_layer_index: 5,
                object_id: "obj-A".to_string(),
                region_id: 1,
            },
            role: ExtrusionRole::OuterWall,
            start_point: Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            end_point: Point3WithWidth {
                x: 1.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            point_count: 4,
        },
        OrderedEntityView {
            original_index: 1,
            region_key: RegionKey {
                global_layer_index: 5,
                object_id: "obj-A".to_string(),
                region_id: 1,
            },
            role: ExtrusionRole::SparseInfill,
            start_point: Point3WithWidth {
                x: 1.0,
                y: 1.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            end_point: Point3WithWidth {
                x: 2.0,
                y: 1.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            point_count: 6,
        },
        OrderedEntityView {
            original_index: 2,
            region_key: RegionKey {
                global_layer_index: 5,
                object_id: "obj-B".to_string(),
                region_id: 2,
            },
            role: ExtrusionRole::TopSolidInfill,
            start_point: Point3WithWidth {
                x: 2.0,
                y: 2.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            end_point: Point3WithWidth {
                x: 3.0,
                y: 2.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            point_count: 8,
        },
    ];
    builder.set_ordered_entities(snapshot.clone());
    let first = builder.get_ordered_entities().to_vec();
    let second = builder.get_ordered_entities().to_vec();
    assert_eq!(first.len(), 3);
    assert_eq!(first, second);
    assert_eq!(first, snapshot);
}
