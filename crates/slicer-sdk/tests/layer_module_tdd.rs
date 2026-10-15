//! TDD tests for LayerModule trait and WIT bindings.
//!
//! These tests verify the API defined in docs/05_module_sdk.md and docs/03_wit_and_manifest.md.
//! Tests lock down trait signatures, ModuleError type, view types, and output builders.

use slicer_ir::{ObjectSurfaceData, OverhangRegion, SurfaceClassificationIR};
use slicer_schema::preparation::PreparationTransportError;
use slicer_sdk::prelude::*;
use slicer_sdk::preparation::{
    with_prepared_plan, LayerPreparation, PreparationError, PreparedPieceCallbacks,
    PreparedPlanCallbacks, PreparedPlanOutput, PreparedPlanView, PrintPreparationInput,
};
use slicer_sdk::preparation::{with_prepared_plan_view, PreparedPieceRead, PreparedPlanRead};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::test_support::fixtures::wall_loop_base;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

struct TransportPiece(&'static [u8]);

impl PreparedPieceRead for TransportPiece {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationTransportError> {
        let start = usize::try_from(offset).map_err(|_| PreparationTransportError::RangeError)?;
        if start > self.0.len() {
            return Err(PreparationTransportError::RangeError);
        }
        let take = max_bytes.min((self.0.len() - start) as u64) as usize;
        Ok(self.0[start..start + take].to_vec())
    }
}

struct TransportPlan(&'static [u8]);

struct RecordingTransportPiece {
    reads: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl PreparedPieceRead for RecordingTransportPiece {
    fn len(&self) -> u64 {
        4
    }

    fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationTransportError> {
        self.reads.lock().unwrap().push((offset, max_bytes));
        TransportPiece(&[3, 5, 11, 17]).read(offset, max_bytes)
    }
}

struct RecordingTransportPlan {
    reads: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl PreparedPlanRead for RecordingTransportPlan {
    fn open(
        &self,
        name: &str,
    ) -> Result<Option<Arc<dyn PreparedPieceRead + Send + Sync>>, PreparationTransportError> {
        Ok((name == "fixture").then(|| {
            Arc::new(RecordingTransportPiece {
                reads: Arc::clone(&self.reads),
            }) as Arc<dyn PreparedPieceRead + Send + Sync>
        }))
    }
}

#[test]
fn preparation_transport_host_preserves_original_read_maximum() {
    let reads = Arc::new(Mutex::new(Vec::new()));
    let plan = PreparedPlanView::from_host_view(Arc::new(RecordingTransportPlan {
        reads: Arc::clone(&reads),
    }));
    let piece = plan.require("fixture").unwrap();
    assert_eq!(piece.read(0, u64::MAX).unwrap(), [3, 5, 11, 17]);
    assert_eq!(piece.read(1, u64::MAX).unwrap(), [5, 11, 17]);
    assert_eq!(*reads.lock().unwrap(), [(0, u64::MAX), (1, u64::MAX)]);
}

#[test]
fn preparation_transport_callbacks_preserve_original_read_maximum() {
    let reads = Arc::new(Mutex::new(Vec::new()));
    with_prepared_plan_view(
        Arc::new(RecordingTransportPlan {
            reads: Arc::clone(&reads),
        }),
        || {
            let plan = PreparedPlanView::current().unwrap();
            let piece = plan.require("fixture").unwrap();
            assert_eq!(piece.read(0, u64::MAX).unwrap(), [3, 5, 11, 17]);
            assert_eq!(piece.read(1, u64::MAX).unwrap(), [5, 11, 17]);
        },
    );
    assert_eq!(*reads.lock().unwrap(), [(0, u64::MAX), (1, u64::MAX)]);
}

impl PreparedPlanRead for TransportPlan {
    fn open(
        &self,
        name: &str,
    ) -> Result<Option<Arc<dyn PreparedPieceRead + Send + Sync>>, PreparationTransportError> {
        match name {
            "fixture" => Ok(Some(Arc::new(TransportPiece(self.0)))),
            "denied" => Err(PreparationTransportError::AccessDenied),
            _ => Ok(None),
        }
    }
}

#[test]
fn preparation_transport_control_character_names_are_exact_nonempty_utf8() {
    let written = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&written);
    let mut output = PreparedPlanOutput::from_callback(move |name, bytes| {
        recorded
            .borrow_mut()
            .push((name.to_owned(), bytes.to_vec()));
        Ok(())
    });
    assert_eq!(output.put("\u{0}", &[1]), Ok(()));
    assert_eq!(output.put("a\tb\n", &[2]), Ok(()));
    assert_eq!(output.put("", &[3]), Err(PreparationError::InvalidName));
    assert_eq!(
        *written.borrow(),
        [
            ("\u{0}".to_owned(), vec![1]),
            ("a\tb\n".to_owned(), vec![2])
        ]
    );

    let view = PreparedPlanView::from_host_view(Arc::new(TransportPlan(b"abc")));
    assert!(view.open("\u{0}").expect("valid absent NUL name").is_none());
    assert!(view
        .open("a\tb\n")
        .expect("valid absent control name")
        .is_none());
    assert_eq!(view.open("").err(), Some(PreparationError::InvalidName));
}

#[test]
fn preparation_transport_require_missing_is_fatal_57401() {
    let view = PreparedPlanView::from_host_view(Arc::new(TransportPlan(b"abc")));
    let error = view.require("absent").err().expect("missing must fail");
    assert_eq!(error.code, 57401);
    assert!(error.fatal);
    assert_eq!(error.message, "required preparation piece missing: absent");
    assert_eq!(view.require("fixture").unwrap().len().unwrap(), 3);
}

#[test]
fn preparation_transport_require_does_not_decode() {
    let view = PreparedPlanView::from_host_view(Arc::new(TransportPlan(b"\xff\x00{broken")));
    let piece = view
        .require("fixture")
        .expect("opaque bytes must not be decoded");
    assert_eq!(piece.read(0, u64::MAX).unwrap(), b"\xff\x00{broken");
}

#[test]
fn preparation_transport_host_reader_writer_round_trip() {
    let reads = Arc::new(Mutex::new(Vec::new()));
    let recorded_reads = Arc::clone(&reads);
    let input = PrintPreparationInput::from_host_reader(Arc::new(move |offset, max_bytes| {
        recorded_reads.lock().unwrap().push((offset, max_bytes));
        match (offset, max_bytes) {
            (2, 3) => Ok(b"xyz".to_vec()),
            (5, 0) => Ok(Vec::new()),
            _ => Err(PreparationTransportError::RangeError),
        }
    }));
    let writes = Arc::new(Mutex::new(Vec::new()));
    let recorded_writes = Arc::clone(&writes);
    let mut output = PreparedPlanOutput::from_host_writer(Arc::new(move |name, bytes| {
        recorded_writes
            .lock()
            .unwrap()
            .push((name.to_owned(), bytes.to_vec()));
        Ok(())
    }));
    let bytes = input.read_fixture(2, 3).unwrap();
    assert_eq!(bytes, b"xyz");
    output.put("payload", &bytes).unwrap();
    let empty = input.read_fixture(5, 0).unwrap();
    assert_eq!(empty, Vec::<u8>::new());
    output.put("empty", &empty).unwrap();
    assert_eq!(output.complete(Ok(())), Ok(()));
    assert_eq!(*reads.lock().unwrap(), [(2, 3), (5, 0)]);
    assert_eq!(
        *writes.lock().unwrap(),
        [
            ("payload".to_owned(), b"xyz".to_vec()),
            ("empty".to_owned(), Vec::new()),
        ]
    );
}

#[test]
fn preparation_transport_scoped_binding_restores_on_unwind() {
    let mut escaped = None;
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"previous")),
        || {
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_prepared_plan_view(Arc::new(TransportPlan(b"outer")), || {
                    with_prepared_plan_view(Arc::new(TransportPlan(b"inner")), || {
                        assert_eq!(
                            PreparedPlanView::current()
                                .unwrap()
                                .require("fixture")
                                .unwrap()
                                .read(0, u64::MAX)
                                .unwrap(),
                            b"inner"
                        );
                    });
                    let outer = PreparedPlanView::current()
                        .unwrap()
                        .require("fixture")
                        .unwrap();
                    assert_eq!(outer.read(0, u64::MAX).unwrap(), b"outer");
                    escaped = Some(outer);
                    panic!("transport unwind sentinel");
                });
            }));
            assert_eq!(
                panic.unwrap_err().downcast_ref::<&str>(),
                Some(&"transport unwind sentinel")
            );
            assert_eq!(
                PreparedPlanView::current()
                    .unwrap()
                    .require("fixture")
                    .unwrap()
                    .read(0, u64::MAX)
                    .unwrap(),
                b"previous"
            );
            assert_eq!(
                escaped.as_ref().unwrap().read(0, 1),
                Err(PreparationError::NotReady)
            );
        },
    );
    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );
}

#[test]
fn preparation_transport_error_variant_survives_facades() {
    let input = PrintPreparationInput::from_host_reader(Arc::new(|_, _| {
        Err(PreparationTransportError::RangeError)
    }));
    let result = input
        .read_fixture(0, 1)
        .map_err(PreparationTransportError::from);
    assert_eq!(result, Err(PreparationTransportError::RangeError));
    let view = PreparedPlanView::from_host_view(Arc::new(TransportPlan(b"")));
    assert_eq!(
        view.open("denied")
            .err()
            .map(PreparationTransportError::from),
        Some(PreparationTransportError::AccessDenied)
    );
}

#[test]
fn preparation_pilot_callback_binding_signature_and_bytes_unchanged() {
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"pilot")),
        || {
            assert_eq!(
                PreparedPlanView::current()
                    .unwrap()
                    .open("fixture")
                    .unwrap()
                    .unwrap()
                    .read(0, u64::MAX)
                    .unwrap(),
                b"pilot"
            );
        },
    );
    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );
}

// =============================================================================
// Test 1: LayerModule trait exists with from_config
// =============================================================================

/// A test module that implements LayerModule.
struct TestModule {
    initialized: bool,
}

impl LayerModule for TestModule {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        // Verify ConfigView is accessible
        let _ = config.len();
        Ok(Self { initialized: true })
    }
}

#[test]
fn test_01_layer_module_trait_exists_with_from_config() {
    // Test that LayerModule trait can be implemented with from_config
    let config = ConfigView::from_map(HashMap::new());

    let module = TestModule::from_config(&config).expect("from_config should succeed");
    assert!(module.initialized, "module should be initialized");
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
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
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
    let module = InfillTestModule::from_config(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let mut output = InfillOutputBuilder::new();

    let result = module.run_infill(
        0,
        &regions,
        &PaintRegionLayerView::new(0),
        &mut output,
        &config,
    );
    assert!(result.is_ok());
}

// =============================================================================
// Test 5: run_perimeters signature matches WIT
// =============================================================================

struct PerimeterTestModule;

impl LayerModule for PerimeterTestModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
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
    let module = PerimeterTestModule::from_config(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let result = module.run_perimeters(0, &regions, &paint, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 6: run_wall_postprocess signature matches WIT
// =============================================================================

struct PerimetersPostProcessTestModule;

impl LayerModule for PerimetersPostProcessTestModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
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
    let module = PerimetersPostProcessTestModule::from_config(&config).unwrap();
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
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill_postprocess(
        &self,
        layer_index: u32,
        regions: &[PerimeterRegionView],
        prior_infill: &[slicer_ir::InfillRegion],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let _ = layer_index;
        let _ = regions.len();
        let _ = prior_infill.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_09_run_infill_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = InfillPostprocessTestModule::from_config(&config).unwrap();
    let regions: Vec<PerimeterRegionView> = vec![];
    let mut output = InfillOutputBuilder::new();

    let result = module.run_infill_postprocess(0, &regions, &[], &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 8: run_slice_postprocess signature matches WIT
// =============================================================================

struct SlicePostprocessTestModule;

impl LayerModule for SlicePostprocessTestModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
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
    let module = SlicePostprocessTestModule::from_config(&config).unwrap();
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
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut SupportOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
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
    let module = SupportTestModule::from_config(&config).unwrap();
    let regions: Vec<SliceRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    let result = module.run_support(
        0,
        &regions,
        &paint,
        &mut output,
        &mut slicer_sdk::LayerCollectionBuilder::new(),
        &config,
    );
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

fn support_test_square(x: f32, y: f32, size: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, y),
                Point2::from_mm(x + size, y),
                Point2::from_mm(x + size, y + size),
                Point2::from_mm(x, y + size),
            ],
        },
        holes: vec![],
    }
}

fn support_test_classification(footprint: ExPolygon) -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        per_object: HashMap::from([(
            "obj-1".to_string(),
            ObjectSurfaceData {
                overhang_regions: vec![OverhangRegion {
                    xy_footprint: vec![footprint],
                    ..Default::default()
                }],
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

#[test]
fn slice_region_view_derive_needs_support_disjoint_footprint_is_false() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_polygons(vec![support_test_square(0.0, 0.0, 1.0)]);
    let classification = support_test_classification(support_test_square(2.0, 0.0, 1.0));
    assert!(!view.derive_needs_support(Some(&classification)));
}

#[test]
fn slice_region_view_derive_needs_support_overlapping_bboxes_but_disjoint_polygons_is_false() {
    // The two triangles' bounding boxes overlap in both axes, but their
    // interiors do not. A bbox-only replacement for the overhang footprint
    // would incorrectly mark this region support-eligible.
    let triangle = |points: [(f32, f32); 3]| ExPolygon {
        contour: Polygon {
            points: points
                .into_iter()
                .map(|(x, y)| Point2::from_mm(x, y))
                .collect(),
        },
        holes: vec![],
    };
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_polygons(vec![triangle([(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)])]);
    let classification =
        support_test_classification(triangle([(3.0, 3.0), (7.0, 3.0), (3.0, 7.0)]));
    assert!(!view.derive_needs_support(Some(&classification)));
}

#[test]
fn slice_region_view_derive_needs_support_overlapping_footprint_is_true() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_polygons(vec![support_test_square(0.0, 0.0, 1.0)]);
    let classification = support_test_classification(support_test_square(0.5, 0.0, 1.0));
    assert!(view.derive_needs_support(Some(&classification)));
}

#[test]
fn slice_region_view_derive_needs_support_empty_overhang_regions_is_true() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_polygons(vec![support_test_square(0.0, 0.0, 1.0)]);
    let classification = SurfaceClassificationIR {
        per_object: HashMap::from([("obj-1".to_string(), ObjectSurfaceData::default())]),
        ..Default::default()
    };
    assert!(view.derive_needs_support(Some(&classification)));
}

#[test]
fn slice_region_view_derive_needs_support_empty_footprints_are_true() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_polygons(vec![support_test_square(0.0, 0.0, 1.0)]);
    let classification = SurfaceClassificationIR {
        per_object: HashMap::from([(
            "obj-1".to_string(),
            ObjectSurfaceData {
                overhang_regions: vec![OverhangRegion::default()],
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    assert!(view.derive_needs_support(Some(&classification)));
}

#[test]
fn slice_region_view_derive_needs_support_missing_classification_is_true() {
    let mut view = SliceRegionView::default();
    view.set_object_id("obj-1".to_string());
    view.set_polygons(vec![support_test_square(0.0, 0.0, 1.0)]);
    assert!(view.derive_needs_support(None));
    assert!(view.derive_needs_support(Some(&SurfaceClassificationIR::default())));
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
        path: ExtrusionPath3D {
            points: vec![],
            ..extrusion_path3d_base(ExtrusionRole::OuterWall)
        },
        width_profile: slicer_ir::WidthProfile { widths: vec![] },
        ..wall_loop_base(
            slicer_ir::LoopType::Outer,
            slicer_ir::WallBoundaryType::ExteriorSurface,
        )
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
        ..extrusion_path3d_base(ExtrusionRole::SparseInfill)
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
        ..extrusion_path3d_base(ExtrusionRole::TopSolidInfill)
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
        ..extrusion_path3d_base(ExtrusionRole::Ironing)
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
        path: ExtrusionPath3D {
            points: vec![],
            ..extrusion_path3d_base(ExtrusionRole::OuterWall)
        },
        width_profile: slicer_ir::WidthProfile { widths: vec![] },
        ..wall_loop_base(
            slicer_ir::LoopType::Outer,
            slicer_ir::WallBoundaryType::ExteriorSurface,
        )
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
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
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
        _paint: &PaintRegionLayerView,
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

    let module = CustomModule::from_config(&config).expect("should create module");
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
        let _: fn(&ConfigView) -> Result<TestModule, ModuleError> = TestModule::from_config;
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
// Test 17: run_support_postprocess signature matches WIT
// =============================================================================

struct SupportPostprocessTestModule;

impl LayerModule for SupportPostprocessTestModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
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
    let module = SupportPostprocessTestModule::from_config(&config).unwrap();
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
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
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
    let module = PathOptimizationTestModule::from_config(&config).unwrap();
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

struct MinimalModule;

impl LayerModule for MinimalModule {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

#[test]
fn test_35_layer_module_defaults_do_not_panic() {
    let config = ConfigView::from_map(HashMap::new());
    let module = MinimalModule::from_config(&config).unwrap();
    let slice_regions: Vec<SliceRegionView> = vec![];
    let perim_regions: Vec<PerimeterRegionView> = vec![];
    let paint = PaintRegionLayerView::new(0);

    // All default methods should return Ok(()) without panicking
    assert!(module
        .run_infill(
            0,
            &slice_regions,
            &paint,
            &mut InfillOutputBuilder::new(),
            &config
        )
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
        .run_infill_postprocess(
            0,
            &perim_regions,
            &[],
            &mut InfillOutputBuilder::new(),
            &config
        )
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
            &mut slicer_sdk::LayerCollectionBuilder::new(),
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
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

impl LayerModule for ModuleB {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

#[test]
fn test_32_multiple_implementations_coexist() {
    let config = ConfigView::from_map(HashMap::new());

    let _a = ModuleA::from_config(&config).unwrap();
    let _b = ModuleB::from_config(&config).unwrap();
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
        // exhaustive: cache round-trip pins every view field
        OrderedEntityView {
            original_index: 0,
            tool_index: 1,
            order_lock: None,
            region_key: RegionKey {
                global_layer_index: 5,
                object_id: "obj-A".to_string(),
                region_id: 1,
                variant_chain: Vec::new(),
            },
            role: ExtrusionRole::OuterWall,
            start_point: Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            end_point: Point3WithWidth {
                x: 1.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            point_count: 4,
        },
        // exhaustive: cache round-trip pins every view field
        OrderedEntityView {
            original_index: 1,
            tool_index: 1,
            order_lock: None,
            region_key: RegionKey {
                global_layer_index: 5,
                object_id: "obj-A".to_string(),
                region_id: 1,
                variant_chain: Vec::new(),
            },
            role: ExtrusionRole::SparseInfill,
            start_point: Point3WithWidth {
                x: 1.0,
                y: 1.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            end_point: Point3WithWidth {
                x: 2.0,
                y: 1.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            point_count: 6,
        },
        // exhaustive: cache round-trip pins every view field
        OrderedEntityView {
            original_index: 2,
            tool_index: 2,
            order_lock: None,
            region_key: RegionKey {
                global_layer_index: 5,
                object_id: "obj-B".to_string(),
                region_id: 2,
                variant_chain: Vec::new(),
            },
            role: ExtrusionRole::TopSolidInfill,
            start_point: Point3WithWidth {
                x: 2.0,
                y: 2.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            end_point: Point3WithWidth {
                x: 3.0,
                y: 2.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
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

// =============================================================================
// Preparation capability: serialized resources are owned by one scoped call.
// The target has no compile-fail harness; LayerPreparation's rustdoc checks
// that omitting prepare_print is rejected with E0046 without new dependencies.
// =============================================================================

fn preparation_fixture_plan(bytes: &'static [u8]) -> PreparedPlanCallbacks {
    PreparedPlanCallbacks::new(move |name| {
        if name != "fixture" {
            return Ok(None);
        }
        Ok(Some(PreparedPieceCallbacks::new(
            move || bytes.len() as u64,
            move |offset, max_bytes| {
                if offset > bytes.len() as u64 {
                    return Err(PreparationError::RangeError);
                }
                let remaining = bytes.len() as u64 - offset;
                let take = max_bytes.min(remaining);
                let take =
                    usize::try_from(take).map_err(|_| PreparationError::ArithmeticOverflow)?;
                let offset = usize::try_from(offset).map_err(|_| PreparationError::RangeError)?;
                Ok(bytes[offset..offset + take].to_vec())
            },
        )))
    })
}

#[test]
fn preparation_scoped_serialized_reads_remove_binding_after_return() {
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"serialized")),
        || {
            let plan = PreparedPlanView::current().expect("current scoped plan");
            assert!(plan.open("absent").unwrap().is_none());
            let piece = plan.open("fixture").unwrap().expect("serialized piece");
            assert_eq!(piece.len(), Ok(10));
            assert_eq!(piece.is_empty(), Ok(false));
            assert_eq!(piece.read(2, 3).unwrap(), b"ria");
            assert_eq!(piece.read(8, 8).unwrap(), b"ed");
            assert_eq!(piece.read(10, 1).unwrap(), b"");
        },
    );
    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );
}

#[test]
fn preparation_missing_current_plan_is_an_error() {
    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );
}

#[test]
fn preparation_unwind_restores_previous_binding_and_invalidates_inner_resources() {
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"outer")),
        || {
            let outer = PreparedPlanView::current().unwrap();
            let outer_piece = outer.open("fixture").unwrap().unwrap();
            let mut escaped = None;
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_prepared_plan(
                    || Ok(preparation_fixture_plan(b"inner")),
                    || {
                        assert_eq!(outer_piece.read(0, 1), Err(PreparationError::AccessDenied));
                        let inner = PreparedPlanView::current().unwrap();
                        let piece = inner.open("fixture").unwrap().unwrap();
                        assert_eq!(piece.read(0, 5).unwrap(), b"inner");
                        escaped = Some(piece);
                        panic!("preparation scope unwind probe");
                    },
                );
            }));
            assert!(panic.is_err());
            assert_eq!(escaped.unwrap().read(0, 1), Err(PreparationError::NotReady));
            assert_eq!(outer_piece.read(0, 5).unwrap(), b"outer");
            assert_eq!(
                PreparedPlanView::current()
                    .unwrap()
                    .open("fixture")
                    .unwrap()
                    .unwrap()
                    .read(0, 5)
                    .unwrap(),
                b"outer"
            );
        },
    );
    assert_eq!(
        PreparedPlanView::current().err(),
        Some(PreparationError::NotReady)
    );
}

struct PreparationResourceDrop(Rc<Cell<usize>>);

impl Drop for PreparationResourceDrop {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn preparation_retained_wrappers_neither_retain_resources_nor_read_another_scope() {
    let dropped = Rc::new(Cell::new(0));
    let observed = Rc::clone(&dropped);
    let (plan, piece) = with_prepared_plan(
        move || {
            let view_resource = PreparationResourceDrop(Rc::clone(&observed));
            Ok(PreparedPlanCallbacks::new(move |_| {
                let piece_resource = PreparationResourceDrop(Rc::clone(&view_resource.0));
                Ok(Some(PreparedPieceCallbacks::new(
                    || 1,
                    move |_, max_bytes| {
                        let _resource = &piece_resource;
                        Ok(b"x"[..max_bytes as usize].to_vec())
                    },
                )))
            }))
        },
        || {
            let plan = PreparedPlanView::current().unwrap();
            let piece = plan.open("fixture").unwrap().unwrap();
            assert_eq!(piece.read(0, 1).unwrap(), b"x");
            (plan, piece)
        },
    );
    assert_eq!(dropped.get(), 2, "scope must drop both imported resources");
    assert_eq!(plan.open("fixture").err(), Some(PreparationError::NotReady));
    assert_eq!(piece.len(), Err(PreparationError::NotReady));
    assert_eq!(piece.read(0, 1), Err(PreparationError::NotReady));
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"new")),
        || {
            assert_eq!(plan.open("fixture").err(), Some(PreparationError::NotReady));
            assert_eq!(piece.read(0, 1), Err(PreparationError::NotReady));
        },
    );
}

#[test]
fn preparation_current_is_fresh_and_does_not_copy_the_plan_eagerly() {
    let acquisitions = Rc::new(Cell::new(0));
    let reads = Rc::new(Cell::new(0));
    let current_calls = Rc::clone(&acquisitions);
    let read_calls = Rc::clone(&reads);
    with_prepared_plan(
        move || {
            current_calls.set(current_calls.get() + 1);
            let read_calls = Rc::clone(&read_calls);
            Ok(PreparedPlanCallbacks::new(move |_| {
                let read_calls = Rc::clone(&read_calls);
                Ok(Some(PreparedPieceCallbacks::new(
                    || 1,
                    move |_, max_bytes| {
                        read_calls.set(read_calls.get() + 1);
                        Ok(b"x"[..max_bytes as usize].to_vec())
                    },
                )))
            }))
        },
        || {
            assert_eq!(acquisitions.get(), 0, "scope entry must be lazy");
            let first = PreparedPlanView::current().unwrap();
            let _second = PreparedPlanView::current().unwrap();
            assert_eq!(acquisitions.get(), 2, "each facade must invoke current");
            let piece = first.open("fixture").unwrap().unwrap();
            assert_eq!(piece.len(), Ok(1));
            assert_eq!(reads.get(), 0, "acquisition and open must not copy bytes");
            assert_eq!(piece.read(0, 1).unwrap(), b"x");
            assert_eq!(reads.get(), 1);
        },
    );
}

#[test]
fn preparation_binding_does_not_leak_to_another_thread() {
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"owner")),
        || {
            assert_eq!(
                std::thread::spawn(|| PreparedPlanView::current().err())
                    .join()
                    .unwrap(),
                Some(PreparationError::NotReady)
            );
            let piece = PreparedPlanView::current()
                .unwrap()
                .open("fixture")
                .unwrap()
                .unwrap();
            assert_eq!(piece.read(0, 5).unwrap(), b"owner");
        },
    );
}

#[test]
fn preparation_bounded_reads_reject_invalid_ranges_and_oversized_host_results() {
    with_prepared_plan(
        || Ok(preparation_fixture_plan(b"abc")),
        || {
            let plan = PreparedPlanView::current().unwrap();
            assert_eq!(plan.open("").err(), Some(PreparationError::InvalidName));
            let piece = plan.open("fixture").unwrap().unwrap();
            assert_eq!(piece.read(4, 0), Err(PreparationError::RangeError));
            // EOF bounds are checked first; then clip by remaining length,
            // never by adding the requested maximum to the offset.
            assert_eq!(piece.read(1, u64::MAX), Ok(b"bc".to_vec()));
            assert_eq!(piece.read(u64::MAX, 1), Err(PreparationError::RangeError));
        },
    );
    with_prepared_plan(
        || {
            Ok(PreparedPlanCallbacks::new(|_| {
                Ok(Some(PreparedPieceCallbacks::new(
                    || 3,
                    |_, _| Ok(b"abc".to_vec()),
                )))
            }))
        },
        || {
            let piece = PreparedPlanView::current()
                .unwrap()
                .open("fixture")
                .unwrap()
                .unwrap();
            assert_eq!(piece.read(0, 1), Err(PreparationError::RangeError));
        },
    );
    let input = PrintPreparationInput::from_callback(|_, _| Ok(b"abc".to_vec()));
    assert_eq!(input.read_fixture(0, 1), Err(PreparationError::RangeError));
    // The earlier callback deliberately violates the returned-count bound.
    // This bounded transport checks EOF first and clips by remaining length.
    let input = PrintPreparationInput::from_callback(|offset, max_bytes| {
        let bytes = b"abc";
        let len = u64::try_from(bytes.len()).unwrap();
        if offset > len {
            return Err(PreparationError::RangeError);
        }
        let remaining = len - offset;
        let take = usize::try_from(max_bytes.min(remaining)).unwrap();
        let start = usize::try_from(offset).unwrap();
        Ok(bytes[start..start + take].to_vec())
    });
    assert_eq!(input.read_fixture(1, u64::MAX), Ok(b"bc".to_vec()));
    assert_eq!(
        input.read_fixture(u64::MAX, 1),
        Err(PreparationError::RangeError)
    );
}

#[test]
fn preparation_output_is_unique_complete_and_poisoned_after_any_error() {
    let written = Rc::new(RefCell::new(Vec::new()));
    let staged = Rc::clone(&written);
    let mut output = PreparedPlanOutput::from_callback(move |name, bytes| {
        staged.borrow_mut().push((name.to_owned(), bytes.to_vec()));
        Ok(())
    });
    output.put("piece", b"complete").unwrap();
    assert_eq!(
        output.put("piece", b"replacement"),
        Err(PreparationError::DuplicateName)
    );
    assert_eq!(
        output.put("another", b"later"),
        Err(PreparationError::PoisonedOutput)
    );
    assert_eq!(
        *written.borrow(),
        [("piece".to_owned(), b"complete".to_vec())]
    );

    let mut invalid = PreparedPlanOutput::from_callback(|_, _| panic!("invalid name reached host"));
    assert_eq!(
        invalid.put("", b"bytes"),
        Err(PreparationError::InvalidName)
    );
    assert_eq!(
        invalid.put("valid", b"bytes"),
        Err(PreparationError::PoisonedOutput)
    );
    let mut failed = PreparedPlanOutput::from_callback(|_, _| Err(PreparationError::AccessDenied));
    assert_eq!(
        failed.put("piece", b"bytes"),
        Err(PreparationError::AccessDenied)
    );
    assert_eq!(
        failed.put("next", b"bytes"),
        Err(PreparationError::PoisonedOutput)
    );
}

struct PreparationTestModule;

#[test]
fn preparation_poison_preserves_returned_non_fatal_error() {
    let mut output = PreparedPlanOutput::from_callback(|_, _| Err(PreparationError::AccessDenied));
    assert_eq!(
        output.put("piece", b"bytes"),
        Err(PreparationError::AccessDenied)
    );
    let error = output
        .complete(Err(ModuleError::non_fatal(
            731,
            "distinct mixed preparation failure",
        )))
        .unwrap_err();
    assert_eq!(error.code, 731);
    assert!(!error.fatal);
    assert_eq!(error.message, "distinct mixed preparation failure");
    let poisoned_success = output.complete(Ok(())).unwrap_err();
    assert_eq!(
        poisoned_success.code,
        slicer_sdk::preparation::POISONED_PREPARATION_CODE
    );
    assert!(poisoned_success.fatal);
}

#[test]
fn preparation_panic_reporting_invokes_previous_hook() {
    const CHILD: &str = "PNP_PREPARATION_HOOK_CONTROL";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "preparation_panic_reporting_invokes_previous_hook",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "hook control failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    } else {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let invoked = Arc::new(AtomicBool::new(false));
        let sentinel = invoked.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |_| sentinel.store(true, Ordering::SeqCst)));
        let _input = PrintPreparationInput::from_callback(|_, _| Ok(Vec::new()));
        let panic = std::panic::catch_unwind(|| panic!("previous preparation hook sentinel"));
        std::panic::set_hook(previous);
        assert!(panic.is_err());
        assert!(
            invoked.load(Ordering::SeqCst),
            "previous hook was not chained"
        );
    }
}

impl LayerPreparation for PreparationTestModule {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        output.put("fixture", &input.read_fixture(0, 3)?)?;
        Ok(())
    }
}

#[test]
fn preparation_errors_map_to_existing_module_error() {
    let input = PrintPreparationInput::from_callback(|_, _| Err(PreparationError::NotReady));
    let mut output =
        PreparedPlanOutput::from_callback(|_, _| panic!("failed input reached output"));
    let error = PreparationTestModule::prepare_print(&input, &mut output).unwrap_err();
    assert!(error.fatal);
    assert_eq!(error.message, "layer preparation: not-ready");
}

#[test]
fn native_prepared_sidecar_calls_preparation_without_changing_ordinary_entry() {
    use slicer_sdk::native::{
        NativePostpassInput, NativePostpassRequest, NativePostpassResponse,
        NativePreparedStageEntry, NativeStageEntry,
    };

    fn ordinary(request: &NativePostpassRequest) -> Result<NativePostpassResponse, ModuleError> {
        match &request.input {
            NativePostpassInput::Text(text) => Ok(NativePostpassResponse::Text(format!(
                "{}:{text}",
                request.stage_export
            ))),
            NativePostpassInput::Gcode(_) => Err(ModuleError::fatal(1, "expected text")),
        }
    }

    let stage = NativeStageEntry::Postpass(ordinary);
    let paired = NativePreparedStageEntry {
        stage,
        prepare: PreparationTestModule::prepare_print,
    };
    let input = PrintPreparationInput::from_callback(|offset, max_bytes| {
        assert_eq!((offset, max_bytes), (0, 3));
        Ok(b"abc".to_vec())
    });
    let pieces = Rc::new(RefCell::new(Vec::new()));
    let staged = Rc::clone(&pieces);
    let mut output = PreparedPlanOutput::from_callback(move |name, bytes| {
        staged.borrow_mut().push((name.to_owned(), bytes.to_vec()));
        Ok(())
    });
    (paired.prepare)(&input, &mut output).unwrap();
    assert_eq!(*pieces.borrow(), [("fixture".to_owned(), b"abc".to_vec())]);
    let (NativeStageEntry::Postpass(original), NativeStageEntry::Postpass(paired_stage)) =
        (stage, paired.stage)
    else {
        panic!("sidecar changed the ordinary stage family");
    };
    assert!(std::ptr::fn_addr_eq(original, paired_stage));
    let request = NativePostpassRequest {
        input: NativePostpassInput::Text("still usable".to_owned()),
        config: ConfigView::from_map(HashMap::new()),
        stage_export: "postpass",
    };
    for call in [original, paired_stage] {
        let NativePostpassResponse::Text(text) = call(&request).unwrap() else {
            panic!("ordinary entry changed its response family");
        };
        assert_eq!(text, "postpass:still usable");
    }
}
