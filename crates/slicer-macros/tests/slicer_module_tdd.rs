//! TDD tests for the `#[slicer_module]` proc-macro.
//!
//! These tests verify that the macro correctly transforms `impl LayerModule for T` blocks
//! into WIT export bindings per docs/05_module_sdk.md.
//!
//! All tests must compile and run.

#![allow(missing_docs, clippy::new_without_default)]

use slicer_macros::slicer_module;

// ============================================================================
// Test support types (minimal mock types for testing macro expansion)
// ============================================================================

/// Mock ConfigView for testing (mirrors the SDK ConfigView concept)
pub struct ConfigView {
    values: std::collections::HashMap<String, ConfigValue>,
}

impl ConfigView {
    pub fn new() -> Self {
        Self {
            values: std::collections::HashMap::new(),
        }
    }

    pub fn with_float(mut self, key: &str, value: f64) -> Self {
        self.values
            .insert(key.to_string(), ConfigValue::Float(value));
        self
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.values.get(key) {
            Some(ConfigValue::Float(v)) => Some(*v),
            _ => None,
        }
    }
}

impl Default for ConfigView {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// Mock ModuleError for testing
#[derive(Debug, Clone)]
pub struct ModuleError {
    pub code: u32,
    pub message: String,
    pub fatal: bool,
}

impl ModuleError {
    pub fn fatal(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fatal: true,
        }
    }

    pub fn non_fatal(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fatal: false,
        }
    }
}

/// Mock SliceRegionView for testing
pub struct SliceRegionView {
    pub object_id: String,
    pub region_id: String,
    pub z: f32,
}

/// Mock PerimeterRegionView for testing
pub struct PerimeterRegionView;

/// Mock InfillOutputBuilder for testing
pub struct InfillOutputBuilder {
    paths: Vec<MockPath>,
}

impl InfillOutputBuilder {
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    pub fn push_sparse_path(&mut self, path: MockPath) -> Result<(), String> {
        self.paths.push(path);
        Ok(())
    }

    pub fn paths(&self) -> &[MockPath] {
        &self.paths
    }
}

impl Default for InfillOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock PerimeterOutputBuilder for testing
pub struct PerimeterOutputBuilder {
    loops: Vec<MockWallLoop>,
}

impl PerimeterOutputBuilder {
    pub fn new() -> Self {
        Self { loops: Vec::new() }
    }

    pub fn push_wall_loop(&mut self, loop_: MockWallLoop) -> Result<(), String> {
        self.loops.push(loop_);
        Ok(())
    }
}

impl Default for PerimeterOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock SupportOutputBuilder for testing
pub struct SupportOutputBuilder;

impl SupportOutputBuilder {
    pub fn new() -> Self {
        Self
    }
}

/// Mock GcodeOutputBuilder for testing
pub struct GcodeOutputBuilder;

impl GcodeOutputBuilder {
    pub fn new() -> Self {
        Self
    }
}

/// Mock LayerCollectionBuilder for testing
pub struct LayerCollectionBuilder;

impl LayerCollectionBuilder {
    pub fn new() -> Self {
        Self
    }
}

/// Mock PaintRegionLayerView for testing
pub struct PaintRegionLayerView;

pub struct MockPath {
    pub z: f32,
}

pub struct MockWallLoop {
    pub perimeter_index: u32,
}

// ============================================================================
// The LayerModule trait that the macro transforms
// ============================================================================

/// The LayerModule trait that modules implement and the macro transforms.
/// This mirrors the SDK's LayerModule trait with all stage method defaults.
pub trait LayerModule: Sized {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>;

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_perimeters(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_wall_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_infill_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_slice_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_support(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_support_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

// ============================================================================
// Test 1: Macro can be applied to impl LayerModule for T block
// ============================================================================

pub struct TestModule1;

#[slicer_module]
impl LayerModule for TestModule1 {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(TestModule1)
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_01_macro_applies_to_layer_module_impl() {
    let config = ConfigView::new();
    let result = TestModule1::on_print_start(&config);
    assert!(result.is_ok());
}

// ============================================================================
// Test 2: Macro preserves the original impl block methods
// ============================================================================

pub struct TestModule2 {
    density: f64,
}

#[slicer_module]
impl LayerModule for TestModule2 {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = config.get_float("density").unwrap_or(0.15);
        Ok(TestModule2 { density })
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

impl TestModule2 {
    pub fn get_density(&self) -> f64 {
        self.density
    }
}

#[test]
fn test_02_macro_preserves_impl_methods() {
    let config = ConfigView::new().with_float("density", 0.25);
    let module = TestModule2::on_print_start(&config).unwrap();
    assert!((module.get_density() - 0.25).abs() < 0.001);
}

// ============================================================================
// Test 3: Macro generates module entry point function
// ============================================================================

pub struct TestModule3;

#[slicer_module]
impl LayerModule for TestModule3 {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(TestModule3)
    }
}

#[test]
fn test_03_macro_generates_entry_point() {
    let entry_exists = TestModule3::__slicer_module_marker();
    assert!(entry_exists, "Module entry point marker should exist");
}

// ============================================================================
// Test 4: Macro detects run_infill as a stage method in the impl block
// ============================================================================

pub struct InfillModule;

#[slicer_module]
impl LayerModule for InfillModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(InfillModule)
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        output.push_sparse_path(MockPath { z: 1.0 }).unwrap();
        Ok(())
    }
}

#[test]
fn test_04_detects_infill_stage_in_impl_block() {
    assert!(InfillModule::__slicer_has_stage_function());
    assert_eq!(InfillModule::__slicer_stage_name(), "Layer::Infill");
}

// ============================================================================
// Test 5: Macro detects run_perimeters as a stage method
// ============================================================================

pub struct PerimeterModule;

#[slicer_module]
impl LayerModule for PerimeterModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(PerimeterModule)
    }

    fn run_perimeters(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_05_detects_perimeters_stage() {
    assert!(PerimeterModule::__slicer_has_stage_function());
    assert_eq!(PerimeterModule::__slicer_stage_name(), "Layer::Perimeters");
}

// ============================================================================
// Test 6: No stage method → __slicer_has_stage_function returns false
// ============================================================================

pub struct LifecycleOnlyModule;

#[slicer_module]
impl LayerModule for LifecycleOnlyModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(LifecycleOnlyModule)
    }
}

#[test]
fn test_06_no_stage_method_detected_when_absent() {
    assert!(!LifecycleOnlyModule::__slicer_has_stage_function());
    assert_eq!(LifecycleOnlyModule::__slicer_stage_name(), "");
}

// ============================================================================
// Test 7: Generated code compiles successfully for valid impl
// ============================================================================

pub struct ValidModule {
    value: i32,
}

#[slicer_module]
impl LayerModule for ValidModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(ValidModule { value: 42 })
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_07_valid_impl_compiles_and_runs() {
    let config = ConfigView::new();
    let module = ValidModule::on_print_start(&config).unwrap();
    assert_eq!(module.value, 42);
    assert!(module.on_print_end().is_ok());
}

// ============================================================================
// Test 8: Generated code preserves method signatures
// ============================================================================

pub struct SignatureModule;

#[slicer_module]
impl LayerModule for SignatureModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let _ = config.get_float("test");
        Ok(SignatureModule)
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        let _ref = self;
        Ok(())
    }
}

#[test]
fn test_08_method_signatures_preserved() {
    let config = ConfigView::new().with_float("test", 1.0);
    let module = SignatureModule::on_print_start(&config).unwrap();

    let start_result: Result<SignatureModule, ModuleError> =
        SignatureModule::on_print_start(&config);
    let end_result: Result<(), ModuleError> = module.on_print_end();

    assert!(start_result.is_ok());
    assert!(end_result.is_ok());
}

// ============================================================================
// Test 9: Generated code handles Result return types correctly
// ============================================================================

#[derive(Debug)]
pub struct ErrorHandlingModule;

#[slicer_module]
impl LayerModule for ErrorHandlingModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        if config.get_float("fail").is_some() {
            return Err(ModuleError::fatal(1, "Configured to fail"));
        }
        Ok(ErrorHandlingModule)
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_09_result_types_handled_correctly() {
    let config_ok = ConfigView::new();
    let result_ok = ErrorHandlingModule::on_print_start(&config_ok);
    assert!(result_ok.is_ok());

    let config_fail = ConfigView::new().with_float("fail", 1.0);
    let result_fail = ErrorHandlingModule::on_print_start(&config_fail);
    assert!(result_fail.is_err());

    let err = result_fail.unwrap_err();
    assert!(err.fatal);
    assert_eq!(err.code, 1);
}

// ============================================================================
// Test 10: __slicer_type_name returns the struct name
// ============================================================================

pub struct NamedModule;

#[slicer_module]
impl LayerModule for NamedModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(NamedModule)
    }
}

#[test]
fn test_10_type_name_accessible() {
    assert_eq!(NamedModule::__slicer_type_name(), "NamedModule");
}

// ============================================================================
// Test 11: WIT export compatibility marker
// ============================================================================

pub struct WitExportModule;

#[slicer_module]
impl LayerModule for WitExportModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(WitExportModule)
    }
}

#[test]
fn test_11_generates_wit_export_layer() {
    let wit_compatible = WitExportModule::__slicer_wit_compatible();
    assert!(wit_compatible, "Module should be WIT export compatible");
}

// ============================================================================
// Test 12: Macro detects run_wall_postprocess stage
// ============================================================================

pub struct PerimetersPostProcessModule;

#[slicer_module]
impl LayerModule for PerimetersPostProcessModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(PerimetersPostProcessModule)
    }

    fn run_wall_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_12_detects_wall_postprocess_stage() {
    assert!(PerimetersPostProcessModule::__slicer_has_stage_function());
    assert_eq!(
        PerimetersPostProcessModule::__slicer_stage_name(),
        "Layer::PerimetersPostProcess"
    );
}

// ============================================================================
// Test 13: Macro detects run_support stage
// ============================================================================

pub struct SupportModule;

#[slicer_module]
impl LayerModule for SupportModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(SupportModule)
    }

    fn run_support(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_13_detects_support_stage() {
    assert!(SupportModule::__slicer_has_stage_function());
    assert_eq!(SupportModule::__slicer_stage_name(), "Layer::Support");
}

// ============================================================================
// Test 14: Module state is properly initialized
// ============================================================================

pub struct StatefulModule {
    counter: std::cell::Cell<u32>,
}

#[slicer_module]
impl LayerModule for StatefulModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(StatefulModule {
            counter: std::cell::Cell::new(0),
        })
    }
}

impl StatefulModule {
    pub fn increment(&self) {
        self.counter.set(self.counter.get() + 1);
    }

    pub fn count(&self) -> u32 {
        self.counter.get()
    }
}

#[test]
fn test_14_module_state_initialized() {
    let config = ConfigView::new();
    let module = StatefulModule::on_print_start(&config).unwrap();

    assert_eq!(module.count(), 0);
    module.increment();
    assert_eq!(module.count(), 1);
}

// ============================================================================
// Test 15: Macro detects run_support_postprocess stage
// ============================================================================

pub struct SupportPostprocessModule;

#[slicer_module]
impl LayerModule for SupportPostprocessModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(SupportPostprocessModule)
    }

    fn run_support_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_15_detects_support_postprocess_stage() {
    assert!(SupportPostprocessModule::__slicer_has_stage_function());
    assert_eq!(
        SupportPostprocessModule::__slicer_stage_name(),
        "Layer::SupportPostProcess"
    );
}

// ============================================================================
// Test 16: Macro detects run_path_optimization stage
// ============================================================================

pub struct PathOptModule;

#[slicer_module]
impl LayerModule for PathOptModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(PathOptModule)
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_16_detects_path_optimization_stage() {
    assert!(PathOptModule::__slicer_has_stage_function());
    assert_eq!(
        PathOptModule::__slicer_stage_name(),
        "Layer::PathOptimization"
    );
}

// ============================================================================
// Test 17: Macro detects run_slice_postprocess stage
// ============================================================================

pub struct SlicePostprocessModule;

#[slicer_module]
impl LayerModule for SlicePostprocessModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(SlicePostprocessModule)
    }

    fn run_slice_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_17_detects_slice_postprocess_stage() {
    assert!(SlicePostprocessModule::__slicer_has_stage_function());
    assert_eq!(
        SlicePostprocessModule::__slicer_stage_name(),
        "Layer::SlicePostProcess"
    );
}

// ============================================================================
// Test 18: Macro detects run_infill_postprocess stage
// ============================================================================

pub struct InfillPostprocessModule;

#[slicer_module]
impl LayerModule for InfillPostprocessModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(InfillPostprocessModule)
    }

    fn run_infill_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_18_detects_infill_postprocess_stage() {
    assert!(InfillPostprocessModule::__slicer_has_stage_function());
    assert_eq!(
        InfillPostprocessModule::__slicer_stage_name(),
        "Layer::InfillPostProcess"
    );
}

// ============================================================================
// Test 19: Stage method callable through the trait impl
// ============================================================================

#[test]
fn test_19_stage_method_callable() {
    let config = ConfigView::new();
    let module = InfillModule::on_print_start(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let mut output = InfillOutputBuilder::new();

    let result = module.run_infill(0, &regions, &mut output, &config);
    assert!(result.is_ok());
    assert_eq!(output.paths().len(), 1);
}

// ============================================================================
// Test 20: All stage names in the documented matrix are recognized
// ============================================================================

#[test]
fn test_20_all_documented_stages_covered_by_macro() {
    // Verify the stage detection covers the full documented stage matrix.
    // This is a compile-time coverage test — each module struct below proves
    // the macro recognizes its stage method.
    //
    // Layer world stages tested above: Infill (test 4), Perimeters (test 5),
    // PerimetersPostProcess (test 12), SlicePostProcess (test 17),
    // InfillPostProcess (test 18), Support (test 13),
    // SupportPostProcess (test 15), PathOptimization (test 16).
    //
    // The remaining stages (PrePass::*, PostPass::*) use the same macro
    // but different traits. Stage method name detection is shared.
    // Verified by stage_name assertions in each test above.
    assert_eq!(InfillModule::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(PerimeterModule::__slicer_stage_name(), "Layer::Perimeters");
    assert_eq!(
        PerimetersPostProcessModule::__slicer_stage_name(),
        "Layer::PerimetersPostProcess"
    );
    assert_eq!(
        SlicePostprocessModule::__slicer_stage_name(),
        "Layer::SlicePostProcess"
    );
    assert_eq!(
        InfillPostprocessModule::__slicer_stage_name(),
        "Layer::InfillPostProcess"
    );
    assert_eq!(SupportModule::__slicer_stage_name(), "Layer::Support");
    assert_eq!(
        SupportPostprocessModule::__slicer_stage_name(),
        "Layer::SupportPostProcess"
    );
    assert_eq!(
        PathOptModule::__slicer_stage_name(),
        "Layer::PathOptimization"
    );
}
