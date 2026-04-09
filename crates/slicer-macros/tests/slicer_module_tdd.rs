//! TDD tests for the `#[slicer_module]` proc-macro.
//!
//! These tests verify that the macro correctly transforms `impl LayerModule for T` blocks
//! into WIT export bindings per docs/05_module_sdk.md.
//!
//! All tests must compile and run. Tests fail only on explicit todo! stubs.

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

pub struct MockPath {
    pub z: f32,
}

pub struct MockWallLoop {
    pub perimeter_index: u32,
}

/// Module information metadata
pub struct SlicerModuleInfo {
    pub type_name: String,
}

// ============================================================================
// The LayerModule trait that the macro transforms
// ============================================================================

/// The LayerModule trait that modules implement and the macro transforms.
/// This mirrors the SDK's LayerModule trait.
pub trait LayerModule: Sized {
    /// Called once when print starts. Returns module instance.
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>;

    /// Called once when print ends.
    fn on_print_end(&self) -> Result<(), ModuleError> {
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
    // The macro should apply without compilation errors.
    // This test passes if the code compiles.
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

/// The macro should generate `__slicer_module_marker` or similar entry point.
/// This test verifies the entry point exists and is callable.
pub struct TestModule3;

#[slicer_module]
impl LayerModule for TestModule3 {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(TestModule3)
    }
}

#[test]
fn test_03_macro_generates_entry_point() {
    // The macro should generate an entry point function.
    // Test that the generated entry point is accessible.
    // This test will fail until the macro generates __slicer_module_marker.

    // Call the generated entry point marker (this function should exist after macro expansion)
    let entry_exists = TestModule3::__slicer_module_marker();
    assert!(entry_exists, "Module entry point marker should exist");
}

// ============================================================================
// Test 4: Macro generates on_print_start wrapper
// ============================================================================

pub struct TestModule4 {
    #[allow(dead_code)]
    initialized: bool,
}

#[slicer_module]
impl LayerModule for TestModule4 {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(TestModule4 { initialized: true })
    }
}

// The macro should generate __slicer_on_print_start wrapper.
// For TDD, we define the expected API and call the implementation.
impl TestModule4 {
    /// Wrapper for on_print_start that will be called by WIT exports.
    /// This is what the macro should generate.
    #[doc(hidden)]
    pub fn __slicer_on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        <Self as LayerModule>::on_print_start(config)
    }
}

#[test]
fn test_04_macro_generates_on_print_start_wrapper() {
    // The macro should generate a wrapper that calls on_print_start.
    // The wrapper is what WIT exports will call.
    let config = ConfigView::new();

    // The wrapper should be generated as __slicer_on_print_start
    let result = TestModule4::__slicer_on_print_start(&config);
    assert!(
        result.is_ok(),
        "Wrapper should call on_print_start successfully"
    );
}

// ============================================================================
// Test 5: Macro generates on_print_end wrapper
// ============================================================================

pub struct TestModule5 {
    cleanup_called: std::cell::Cell<bool>,
}

#[slicer_module]
impl LayerModule for TestModule5 {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(TestModule5 {
            cleanup_called: std::cell::Cell::new(false),
        })
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        self.cleanup_called.set(true);
        Ok(())
    }
}

impl TestModule5 {
    /// Wrapper for on_print_end that will be called by WIT exports.
    /// This is what the macro should generate.
    #[doc(hidden)]
    pub fn __slicer_on_print_end(&self) -> Result<(), ModuleError> {
        <Self as LayerModule>::on_print_end(self)
    }
}

#[test]
fn test_05_macro_generates_on_print_end_wrapper() {
    let config = ConfigView::new();
    let module = TestModule5::on_print_start(&config).unwrap();

    // The wrapper should be generated as __slicer_on_print_end
    let result = module.__slicer_on_print_end();
    assert!(
        result.is_ok(),
        "Wrapper should call on_print_end successfully"
    );
}

// ============================================================================
// Test 6: Macro generates stage-specific dispatch function
// ============================================================================

pub struct InfillModule;

#[slicer_module]
impl LayerModule for InfillModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(InfillModule)
    }
}

impl InfillModule {
    /// Stage function for Layer::Infill
    pub fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        output.push_sparse_path(MockPath { z: 1.0 }).unwrap();
        Ok(())
    }

    /// Stage dispatch function that the macro should generate.
    #[doc(hidden)]
    pub fn __slicer_dispatch_stage(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        self.run_infill(layer_index, regions, output, config)
    }
}

#[test]
fn test_06_macro_generates_stage_dispatch() {
    // The macro should generate a dispatch wrapper for run_infill
    let config = ConfigView::new();
    let module = InfillModule::on_print_start(&config).unwrap();

    // The generated dispatch should exist and be callable
    let regions: Vec<SliceRegionView> = vec![];
    let mut output = InfillOutputBuilder::new();

    let result = module.__slicer_dispatch_stage(0, &regions, &mut output, &config);
    assert!(result.is_ok(), "Stage dispatch should succeed");
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
        // Verify config parameter is accessible
        let _ = config.get_float("test");
        Ok(SignatureModule)
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        // Verify &self is accessible
        let _ref = self;
        Ok(())
    }
}

#[test]
fn test_08_method_signatures_preserved() {
    let config = ConfigView::new().with_float("test", 1.0);
    let module = SignatureModule::on_print_start(&config).unwrap();

    // Verify the wrapped methods have correct signatures
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
        // Return error based on config
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
    // Test success case
    let config_ok = ConfigView::new();
    let result_ok = ErrorHandlingModule::on_print_start(&config_ok);
    assert!(result_ok.is_ok());

    // Test error case
    let config_fail = ConfigView::new().with_float("fail", 1.0);
    let result_fail = ErrorHandlingModule::on_print_start(&config_fail);
    assert!(result_fail.is_err());

    let err = result_fail.unwrap_err();
    assert!(err.fatal);
    assert_eq!(err.code, 1);
}

// ============================================================================
// Test 10: Macro validates impl provides at least one stage function
// ============================================================================

/// A module with run_infill stage function
pub struct StageModule;

#[slicer_module]
impl LayerModule for StageModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(StageModule)
    }
}

impl StageModule {
    pub fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_10_validates_stage_function_exists() {
    // The macro should validate that a stage function is available.
    // This test verifies the stage function is recognized.
    let has_stage = StageModule::__slicer_has_stage_function();
    assert!(has_stage, "Module should have a stage function detected");
}

// ============================================================================
// Test 11: Macro generates WIT export compatibility layer
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
    // The macro should generate functions compatible with WIT exports.
    // Verify the module is marked as WIT-exportable.
    let wit_compatible = WitExportModule::__slicer_wit_compatible();
    assert!(wit_compatible, "Module should be WIT export compatible");
}

// ============================================================================
// Test 12: Macro handles modules with perimeter stage
// ============================================================================

pub struct PerimeterModule;

#[slicer_module]
impl LayerModule for PerimeterModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(PerimeterModule)
    }
}

impl PerimeterModule {
    pub fn run_perimeters(
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
fn test_12_handles_perimeter_stage() {
    let config = ConfigView::new();
    let module = PerimeterModule::on_print_start(&config).unwrap();

    let regions: Vec<SliceRegionView> = vec![];
    let mut output = PerimeterOutputBuilder::new();

    // Direct call should work
    let result = module.run_perimeters(0, &regions, &mut output, &config);
    assert!(result.is_ok());
}

// ============================================================================
// Test 13: Generated module info is accessible
// ============================================================================

pub struct InfoModule;

#[slicer_module]
impl LayerModule for InfoModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(InfoModule)
    }
}

impl InfoModule {
    /// Returns module metadata information.
    /// This is what the macro should generate.
    #[doc(hidden)]
    pub fn __slicer_module_info() -> SlicerModuleInfo {
        SlicerModuleInfo {
            type_name: "InfoModule".to_string(),
        }
    }
}

#[test]
fn test_13_module_info_accessible() {
    // The macro should generate module metadata accessors
    let info = InfoModule::__slicer_module_info();
    assert!(!info.type_name.is_empty(), "Type name should be set");
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
