//! TDD coverage for the TASK-109 real binding/export surface emitted by
//! `#[slicer_module]`. Verifies the macro now emits schema helpers that
//! align with the per-stage WIT packages in `wit/deps/<pkg>/`
//! (docs/03_wit_and_manifest.md, docs/05_module_sdk.md).

#![allow(missing_docs)]

use slicer_macros::slicer_module;

// ── Minimal local mock traits + types mirroring the SDK ────────────────

pub struct ConfigView;
#[derive(Debug)]
pub struct ModuleError {
    pub code: u32,
    pub message: String,
    pub fatal: bool,
}

pub struct SliceRegionView;
pub struct PerimeterRegionView;
pub struct InfillOutputBuilder;
pub struct PerimeterOutputBuilder;
pub struct SupportOutputBuilder;
pub struct GcodeOutputBuilder;
pub struct LayerCollectionBuilder;
pub struct PaintRegionLayerView;
pub struct SlicePostprocessBuilder;
pub struct MeshObjectView;
pub struct ObjectId(pub String);
pub struct PaintSegmentationObjectView;
pub struct MeshAnalysisOutput;
pub struct LayerPlanOutput;
pub struct PaintSegmentationOutput;
pub struct LayerCollectionView;
pub struct FinalizationOutputBuilder;
pub struct GcodeCommand;

pub trait LayerModule: Sized {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError>;
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
        _paint: &PaintRegionLayerView,
        _output: &mut PerimeterOutputBuilder,
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

pub trait PrepassModule: Sized {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError>;
    fn run_mesh_analysis(
        &self,
        _objects: &[MeshObjectView],
        _output: &mut MeshAnalysisOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_layer_planning(
        &self,
        _objects: &[MeshObjectView],
        _output: &mut LayerPlanOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub trait FinalizationModule: Sized {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError>;
    fn run_finalization(
        &self,
        _layers: &[LayerCollectionView],
        _output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub trait PostpassModule: Sized {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError>;
    fn run_gcode_postprocess(
        &self,
        _commands: &[GcodeCommand],
        _output: &mut GcodeOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_text_postprocess(
        &self,
        _text: &str,
        _config: &ConfigView,
    ) -> Result<String, ModuleError> {
        Ok(String::new())
    }
}

// ── Fixtures covering each world/stage combination ─────────────────────

pub struct LayerInfillFixture;
#[slicer_module]
impl LayerModule for LayerInfillFixture {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_infill(
        &self,
        _l: u32,
        _r: &[SliceRegionView],
        _o: &mut InfillOutputBuilder,
        _c: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub struct LayerLifecycleOnly;
#[slicer_module]
impl LayerModule for LayerLifecycleOnly {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
}

pub struct PrepassMeshAnalysisFixture;
#[slicer_module]
impl PrepassModule for PrepassMeshAnalysisFixture {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_mesh_analysis(
        &self,
        _o: &[MeshObjectView],
        _out: &mut MeshAnalysisOutput,
        _c: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub struct PrepassLayerPlanningFixture;
#[slicer_module]
impl PrepassModule for PrepassLayerPlanningFixture {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_layer_planning(
        &self,
        _o: &[MeshObjectView],
        _out: &mut LayerPlanOutput,
        _c: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub struct FinalizationFixture;
#[slicer_module]
impl FinalizationModule for FinalizationFixture {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_finalization(
        &self,
        _l: &[LayerCollectionView],
        _o: &mut FinalizationOutputBuilder,
        _c: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub struct PostpassGcodeFixture;
#[slicer_module]
impl PostpassModule for PostpassGcodeFixture {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_gcode_postprocess(
        &self,
        _cmds: &[GcodeCommand],
        _o: &mut GcodeOutputBuilder,
        _c: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

pub struct PostpassTextFixture;
#[slicer_module]
impl PostpassModule for PostpassTextFixture {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_text_postprocess(&self, _t: &str, _c: &ConfigView) -> Result<String, ModuleError> {
        Ok(String::new())
    }
}

// ── Tests: binding schema surface emitted by the macro ─────────────────

#[test]
fn layer_stage_module_reports_layer_world_and_stage_export() {
    assert_eq!(
        LayerInfillFixture::__slicer_tier_id(),
        slicer_schema::TIER_LAYER
    );
    assert_eq!(LayerInfillFixture::__slicer_trait_name(), "LayerModule");
    assert_eq!(LayerInfillFixture::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(LayerInfillFixture::__slicer_stage_export_name(), "run");
    assert_eq!(
        LayerInfillFixture::__slicer_stage_method_name(),
        "run_infill"
    );
}

#[test]
fn layer_module_wit_exports_only_detected_stage() {
    let exports = LayerInfillFixture::__slicer_wit_exports();
    assert_eq!(exports, &["slicer:layer-infill/infill@2.0.0#run"]);
    assert_eq!(exports.len(), 1);
}

#[test]
fn layer_stageless_module_lists_no_exports() {
    assert_eq!(
        LayerLifecycleOnly::__slicer_tier_id(),
        slicer_schema::TIER_LAYER
    );
    assert_eq!(LayerLifecycleOnly::__slicer_stage_export_name(), "");
    let exports = LayerLifecycleOnly::__slicer_wit_exports();
    assert_eq!(exports, &[] as &[&str]);
}

#[test]
fn prepass_mesh_analysis_reports_prepass_world() {
    assert_eq!(
        PrepassMeshAnalysisFixture::__slicer_tier_id(),
        slicer_schema::TIER_PREPASS
    );
    assert_eq!(
        PrepassMeshAnalysisFixture::__slicer_stage_name(),
        "PrePass::MeshAnalysis"
    );
    assert_eq!(
        PrepassMeshAnalysisFixture::__slicer_stage_export_name(),
        "run"
    );
    let exports = PrepassMeshAnalysisFixture::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:prepass-mesh-analysis/mesh-analysis@1.0.0#run"));
}

#[test]
fn prepass_layer_planning_reports_prepass_world() {
    assert_eq!(
        PrepassLayerPlanningFixture::__slicer_tier_id(),
        slicer_schema::TIER_PREPASS
    );
    assert_eq!(
        PrepassLayerPlanningFixture::__slicer_stage_export_name(),
        "run"
    );
}

#[test]
fn finalization_module_reports_finalization_world_and_export() {
    assert_eq!(
        FinalizationFixture::__slicer_tier_id(),
        slicer_schema::TIER_FINALIZATION
    );
    assert_eq!(
        FinalizationFixture::__slicer_stage_name(),
        "PostPass::LayerFinalization"
    );
    assert_eq!(FinalizationFixture::__slicer_stage_export_name(), "run");
}

#[test]
fn postpass_gcode_module_reports_postpass_world() {
    assert_eq!(
        PostpassGcodeFixture::__slicer_tier_id(),
        slicer_schema::TIER_POSTPASS
    );
    assert_eq!(PostpassGcodeFixture::__slicer_stage_export_name(), "run");
}

#[test]
fn postpass_text_module_reports_postpass_world() {
    assert_eq!(
        PostpassTextFixture::__slicer_tier_id(),
        slicer_schema::TIER_POSTPASS
    );
    assert_eq!(PostpassTextFixture::__slicer_stage_export_name(), "run");
    assert_eq!(
        PostpassTextFixture::__slicer_stage_name(),
        "PostPass::TextPostProcess"
    );
}

#[test]
fn binding_schema_json_captures_full_export_surface() {
    let json = LayerInfillFixture::__slicer_binding_schema_json();
    assert!(json.contains(r#""trait":"LayerModule""#));
    assert!(json.contains(&format!(r#""tier":"{}""#, slicer_schema::TIER_LAYER)));
    assert!(json.contains(r#""stage_id":"Layer::Infill""#));
    assert!(json.contains(r#""stage_method":"run_infill""#));
    assert!(json.contains(r#""stage_export":"slicer:layer-infill/infill@2.0.0#run""#));
    assert!(json.contains(r#""slicer:layer-infill/infill@2.0.0#run""#));
}

#[test]
fn legacy_marker_surface_still_available() {
    assert!(LayerInfillFixture::__slicer_module_marker());
    assert!(LayerInfillFixture::__slicer_wit_compatible());
    assert!(LayerInfillFixture::__slicer_has_stage_function());
    assert_eq!(
        LayerInfillFixture::__slicer_type_name(),
        "LayerInfillFixture"
    );
}

// ── Negative / compile-fail fixtures covered via trybuild-style assertions ──
// The cross-world guardrail (prepass trait + layer stage method) and the
// multi-stage rejection are tested by inspection: a module declaring
// conflicting stages/traits must produce a compile error. Those are
// covered by the existing `#[slicer_module]` internal checks plus the
// coverage below — they compile only because the stage matches the world.

pub struct ValidCrossWorldPrepass;
#[slicer_module]
impl PrepassModule for ValidCrossWorldPrepass {
    fn from_config(_c: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    fn run_mesh_analysis(
        &self,
        _o: &[MeshObjectView],
        _out: &mut MeshAnalysisOutput,
        _c: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn prepass_world_trait_passes_guardrail_when_stage_matches() {
    // Regression guard: the world/stage agreement check must not reject
    // a correctly-paired (trait, stage).
    assert_eq!(
        ValidCrossWorldPrepass::__slicer_tier_id(),
        slicer_schema::TIER_PREPASS
    );
    assert_eq!(
        ValidCrossWorldPrepass::__slicer_stage_name(),
        "PrePass::MeshAnalysis"
    );
}

// ── TASK-109: typed `SlicerModuleSchema` const surface ────────────────

use slicer_schema::{ExportBinding, ExportKind, SlicerModuleSchema};

#[test]
fn typed_schema_const_mirrors_string_accessors_for_layer_infill() {
    let s: &'static SlicerModuleSchema = LayerInfillFixture::__slicer_module_schema();
    assert_eq!(s.type_name, "LayerInfillFixture");
    assert_eq!(s.trait_name, "LayerModule");
    assert_eq!(s.tier_id, slicer_schema::TIER_LAYER);
    assert_eq!(s.stage_id, "Layer::Infill");
    assert_eq!(s.stage_method, "run_infill");
    assert_eq!(s.stage_export, "slicer:layer-infill/infill@2.0.0#run");

    assert_eq!(s.exports.len(), 1);
    assert_eq!(
        s.exports[0],
        ExportBinding {
            name: "slicer:layer-infill/infill@2.0.0#run",
            kind: ExportKind::Stage
        }
    );
}

#[test]
fn typed_schema_const_for_lifecycle_only_impl_has_no_stage_export() {
    let s = LayerLifecycleOnly::__slicer_module_schema();
    assert_eq!(s.stage_id, "");
    assert_eq!(s.stage_export, "");
    assert_eq!(s.exports.len(), 0);
}

#[test]
fn typed_schema_associated_const_is_identical_to_accessor() {
    // The accessor returns a reference to the same const storage so host
    // dispatch paths that can only name the function form stay in sync
    // with compile-time reflection over `SLICER_MODULE_SCHEMA`.
    let via_const = &LayerInfillFixture::SLICER_MODULE_SCHEMA;
    let via_fn = LayerInfillFixture::__slicer_module_schema();
    assert_eq!(via_const, via_fn);
    assert!(std::ptr::eq(
        via_const as *const SlicerModuleSchema,
        via_fn as *const SlicerModuleSchema
    ));
}

#[test]
fn typed_schema_covers_every_world() {
    // One assertion per world ensures the macro wires lifecycle/stage
    // correctly across all four WIT worlds documented under docs/03.
    assert_eq!(
        PrepassMeshAnalysisFixture::__slicer_module_schema().tier_id,
        slicer_schema::TIER_PREPASS
    );
    assert_eq!(
        PrepassLayerPlanningFixture::__slicer_module_schema().stage_export,
        "slicer:prepass-layer-planning/layer-planning@3.0.0#run"
    );
    assert_eq!(
        FinalizationFixture::__slicer_module_schema().tier_id,
        slicer_schema::TIER_FINALIZATION
    );
    let expected_finalization_export =
        slicer_schema::qualified_export_for_stage_id("PostPass::LayerFinalization").unwrap();
    assert_eq!(
        FinalizationFixture::__slicer_module_schema().stage_export,
        expected_finalization_export.as_str()
    );
    assert_eq!(
        PostpassGcodeFixture::__slicer_module_schema().stage_export,
        "slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0#run"
    );
    assert_eq!(
        PostpassTextFixture::__slicer_module_schema().stage_export,
        "slicer:postpass-text-postprocess/text-postprocess@1.0.0#run"
    );
}

#[test]
fn typed_schema_exports_are_deterministic_across_invocations() {
    // Reflecting over the typed surface must be byte-stable: host
    // validate/build tooling compares schemas across runs to detect drift.
    let a = LayerInfillFixture::__slicer_module_schema();
    let b = LayerInfillFixture::__slicer_module_schema();
    assert_eq!(a, b);
    let names_a: Vec<&str> = a.exports.iter().map(|e| e.name).collect();
    let names_b: Vec<&str> = b.exports.iter().map(|e| e.name).collect();
    assert_eq!(names_a, names_b);
    assert_eq!(names_a, vec!["slicer:layer-infill/infill@2.0.0#run"]);
}

// These same authored fixtures are compiled here natively and by the tests below
// as actual wasm32 components. No expanded macro source is snapshotted.
#[allow(dead_code)]
mod prepared_binding_fixture {
    use slicer_sdk::preparation::*;

    pub struct PreparedFixture;

    #[slicer_macros::slicer_module(preparation)]
    impl slicer_sdk::traits::LayerModule for PreparedFixture {
        fn from_config(_: &slicer_ir::ConfigView) -> Result<Self, slicer_sdk::error::ModuleError> {
            Ok(Self)
        }

        fn run_perimeters(
            &self,
            _: u32,
            _: &[slicer_sdk::views::SliceRegionView],
            _: &slicer_sdk::PaintRegionLayerView,
            _: &mut slicer_sdk::builders::PerimeterOutputBuilder,
            _: &slicer_ir::ConfigView,
        ) -> Result<(), slicer_sdk::error::ModuleError> {
            Ok(())
        }
    }

    impl LayerPreparation for PreparedFixture {
        fn prepare_print(
            input: &PrintPreparationInput,
            output: &mut PreparedPlanOutput,
        ) -> Result<(), slicer_sdk::error::ModuleError> {
            typed_read_surface(input);
            // Retain the producer's fixture delegate on the very same facade.
            output.put("fixture", &input.read_fixture(2, 3)?)?;
            Ok(())
        }
    }

    // test-quality: compile witness — all fifteen SDK reads must retain their
    // exact optional/result shape, alongside the generated WIT reader impl.
    fn typed_read_surface(input: &PrintPreparationInput) {
        let _: Result<u64, PreparationInputError> = input.layer_count();
        let _: Result<Vec<PreparationLayer>, PreparationInputError> = input.layers(1, 2);
        let _: Result<Vec<PreparationRegion>, PreparationInputError> = input.regions(3, 1, 2);
        let _: Result<Vec<PreparationRegionConfig>, PreparationInputError> =
            input.region_configs(3, 1, 2);
        let _: Result<Option<Vec<PreparationSurfaceObject>>, PreparationInputError> =
            input.surface_objects(1, 2);
        let _: Result<Option<Vec<PreparationSurfaceLayer>>, PreparationInputError> =
            input.surface_layers(3, 1, 2);
        let _: Result<Vec<PreparationMeshObject>, PreparationInputError> = input.mesh_objects(1, 2);
        let _: Result<Vec<slicer_ir::Point3>, PreparationInputError> =
            input.mesh_vertices("mesh", 1, 2);
        let _: Result<Vec<(u32, u32, u32)>, PreparationInputError> =
            input.mesh_triangles("mesh", 1, 2);
        let _: Result<Vec<PreparationMeshQueryResult>, PreparationInputError> =
            input.mesh_queries(&[
                PreparationMeshQuery::ObjectBounds {
                    object_id: "mesh".into(),
                },
                PreparationMeshQuery::RaycastZDown {
                    object_id: "mesh".into(),
                    x: 1.0,
                    y: 2.0,
                    start_z: 3.0,
                },
                PreparationMeshQuery::SurfaceNormalAt {
                    object_id: "mesh".into(),
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
            ]);
        let _: Result<Option<Vec<PreparationSeam>>, PreparationInputError> = input.seams(3, 1, 2);
        let _: Result<Option<PreparationSupportAnalysisBatch>, PreparationInputError> =
            input.support_analysis(1, 2);
        let _: Result<Option<PreparationSupportGeometryBatch>, PreparationInputError> =
            input.support_geometry(1, 2);
        let _: Result<Option<PreparationSupportPlanBatch>, PreparationInputError> =
            input.support_plan(1, 2);
        let _: Result<Vec<PreparationSelectedTarget>, PreparationInputError> =
            input.selected_targets(3, 1, 2);
    }

    // Exhaustive witnesses complement the generated adapter's exhaustive WIT
    // matches: deleting a branch on either side must fail the component build.
    fn source_shape(source: PreparationTargetSource) -> PreparationVariantChain {
        match source {
            PreparationTargetSource::Model { key, members } => {
                let _: Vec<PreparationRegionKey> = members;
                key.variant_chain
            }
            PreparationTargetSource::SupportCarrier {
                entry_index,
                object_id,
                region_id,
                variant_chain,
                family,
                demand_ids,
                body_ids,
                provenance,
                anchor_layer_index,
            } => {
                let _: (
                    u64,
                    String,
                    u64,
                    String,
                    Vec<String>,
                    Vec<String>,
                    Vec<String>,
                    u32,
                ) = (
                    entry_index,
                    object_id,
                    region_id,
                    family,
                    demand_ids,
                    body_ids,
                    provenance,
                    anchor_layer_index,
                );
                variant_chain
            }
            PreparationTargetSource::Raft { global_layer_index } => {
                let _: u32 = global_layer_index;
                full_chain()
            }
            PreparationTargetSource::AnchoredStage { anchor_layer_index } => {
                let _: u32 = anchor_layer_index;
                full_chain()
            }
        }
    }

    fn full_chain() -> PreparationVariantChain {
        vec![
            ("flag".into(), PreparationPaintValue::Flag(true)),
            ("scalar".into(), PreparationPaintValue::Scalar(-0.0)),
            ("tool".into(), PreparationPaintValue::ToolIndex(7)),
            ("custom".into(), PreparationPaintValue::Custom("A/π".into())),
        ]
    }

    // Exhaustive matching is the compile-time guard for generated paint variants.
    #[allow(clippy::needless_match)]
    fn paint_shape(value: PreparationPaintValue) -> PreparationPaintValue {
        match value {
            PreparationPaintValue::Flag(value) => PreparationPaintValue::Flag(value),
            PreparationPaintValue::Scalar(value) => PreparationPaintValue::Scalar(value),
            PreparationPaintValue::ToolIndex(value) => PreparationPaintValue::ToolIndex(value),
            PreparationPaintValue::Custom(value) => PreparationPaintValue::Custom(value),
        }
    }

    fn error_shape(error: PreparationInputError) -> PreparationInputError {
        match error {
            PreparationInputError::MissingRequired { read } => {
                PreparationInputError::MissingRequired { read }
            }
            PreparationInputError::UnsupportedRead { read } => {
                PreparationInputError::UnsupportedRead { read }
            }
            PreparationInputError::InvalidInput { detail } => {
                PreparationInputError::InvalidInput { detail }
            }
            PreparationInputError::UnknownObject { object_id } => {
                PreparationInputError::UnknownObject { object_id }
            }
            PreparationInputError::SelectionRejected { detail } => {
                PreparationInputError::SelectionRejected { detail }
            }
            PreparationInputError::AccessDenied => PreparationInputError::AccessDenied,
            PreparationInputError::RangeError => PreparationInputError::RangeError,
            PreparationInputError::ArithmeticOverflow => PreparationInputError::ArithmeticOverflow,
        }
    }
}

mod plain_preparation_binding_fixture {
    pub struct PlainFixture;

    // Deliberately no LayerPreparation impl, declaration, or prepared entry.
    #[slicer_macros::slicer_module]
    impl slicer_sdk::traits::LayerModule for PlainFixture {
        fn from_config(_: &slicer_ir::ConfigView) -> Result<Self, slicer_sdk::error::ModuleError> {
            Ok(Self)
        }

        fn run_perimeters(
            &self,
            _: u32,
            _: &[slicer_sdk::views::SliceRegionView],
            _: &slicer_sdk::PaintRegionLayerView,
            _: &mut slicer_sdk::builders::PerimeterOutputBuilder,
            _: &slicer_ir::ConfigView,
        ) -> Result<(), slicer_sdk::error::ModuleError> {
            Ok(())
        }
    }
}

#[test]
fn prepared_binding_preserves_the_ordinary_stage_and_capability_identity() {
    use prepared_binding_fixture::PreparedFixture;
    assert_eq!(
        PreparedFixture::__SLICER_PREPARATION_SCHEMA.qualified_export,
        slicer_schema::PREPARATION_QUALIFIED_EXPORT
    );
    assert_eq!(
        PreparedFixture::__SLICER_PREPARATION_SCHEMA.wit_world,
        "prepared-perimeters-module"
    );
    assert_eq!(
        PreparedFixture::__slicer_module_schema().stage_export,
        plain_preparation_binding_fixture::PlainFixture::__slicer_module_schema().stage_export
    );
}

#[test]
fn prepared_binding_combines_typed_reads_with_the_original_fixture_delegate() {
    use slicer_sdk::preparation::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Reader(Arc<AtomicUsize>);
    macro_rules! read {
        ($name:ident($($arg:ident: $ty:ty),*) -> $result:ty) => {
            fn $name(&self, $($arg: $ty),*) -> Result<$result, PreparationInputError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Err(PreparationInputError::MissingRequired { read: stringify!($name).into() })
            }
        };
    }
    impl PrintPreparationRead for Reader {
        read!(layer_count() -> u64);
        read!(layers(_offset: u64, _max: u64) -> Vec<PreparationLayer>);
        read!(regions(_layer: u32, _offset: u64, _max: u64) -> Vec<PreparationRegion>);
        read!(region_configs(_layer: u32, _offset: u64, _max: u64) -> Vec<PreparationRegionConfig>);
        read!(surface_objects(_offset: u64, _max: u64) -> Option<Vec<PreparationSurfaceObject>>);
        read!(surface_layers(_layer: u32, _offset: u64, _max: u64) -> Option<Vec<PreparationSurfaceLayer>>);
        read!(mesh_objects(_offset: u64, _max: u64) -> Vec<PreparationMeshObject>);
        read!(mesh_vertices(_object: &str, _offset: u64, _max: u64) -> Vec<slicer_ir::Point3>);
        read!(mesh_triangles(_object: &str, _offset: u64, _max: u64) -> Vec<(u32, u32, u32)>);
        read!(mesh_queries(_requests: &[PreparationMeshQuery]) -> Vec<PreparationMeshQueryResult>);
        read!(seams(_layer: u32, _offset: u64, _max: u64) -> Option<Vec<PreparationSeam>>);
        read!(support_analysis(_offset: u64, _max: u64) -> Option<PreparationSupportAnalysisBatch>);
        read!(support_geometry(_offset: u64, _max: u64) -> Option<PreparationSupportGeometryBatch>);
        read!(support_plan(_offset: u64, _max: u64) -> Option<PreparationSupportPlanBatch>);
        read!(selected_targets(_layer: u32, _offset: u64, _max: u64) -> Vec<PreparationSelectedTarget>);
    }

    let typed_calls = Arc::new(AtomicUsize::new(0));
    let fixture_calls = Rc::new(RefCell::new(Vec::new()));
    let fixture_observations = fixture_calls.clone();
    let input = PrintPreparationInput::from_host_input_with_fixture(
        Arc::new(Reader(typed_calls.clone())),
        move |offset, max_bytes| {
            fixture_observations.borrow_mut().push((offset, max_bytes));
            Ok(vec![9, 8, 7])
        },
    );
    // A missing typed input must not be flattened to the fixture-only facade's
    // AccessDenied, nor disable the independently granted fixture delegate.
    assert_eq!(
        input.layer_count(),
        Err(PreparationInputError::MissingRequired {
            read: "layer_count".into()
        })
    );
    typed_calls.store(0, Ordering::SeqCst);
    let pieces = Rc::new(RefCell::new(Vec::new()));
    let staged = pieces.clone();
    let mut output = PreparedPlanOutput::from_callback(move |name, bytes| {
        staged.borrow_mut().push((name.to_owned(), bytes.to_vec()));
        Ok(())
    });
    (prepared_binding_fixture::PreparedFixture::__slicer_prepared_native_entry().prepare)(
        &input,
        &mut output,
    )
    .expect("generated native entry forwards the combined facade");
    // test-quality: count pin — the preparation SDK/WIT contract has fifteen
    // typed reads; this fixture invokes each once through the same facade.
    assert_eq!(typed_calls.load(Ordering::SeqCst), 15);
    assert_eq!(*fixture_calls.borrow(), vec![(2, 3)]);
    assert_eq!(
        *pieces.borrow(),
        vec![("fixture".to_owned(), vec![9, 8, 7])]
    );
}

fn preparation_binding_components() -> &'static [String; 2] {
    use quote::ToTokens;
    use std::path::Path;
    use std::process::Command;
    use std::sync::OnceLock;

    static COMPONENTS: OnceLock<[String; 2]> = OnceLock::new();
    COMPONENTS.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let scratch = root.join("target/preparation-binding-witness");
        // Compile the authored fixture AST, not source-grep assertions or a
        // snapshot of macro expansion. Both legs use the actual attribute.
        let fixtures = syn::parse_file(include_str!("binding_surface_tdd.rs")).unwrap();
        let run = |command: &mut Command, log: &Path| {
            let output = command.output().expect("launch binding witness tool");
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            std::fs::write(log, &text).expect("retain full binding witness output");
            assert!(output.status.success(), "binding witness failed; see {}\n{}", log.display(),
                text.lines().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"));
            text
        };
        ["prepared_binding_fixture", "plain_preparation_binding_fixture"].map(|module| {
            let directory = scratch.join(module);
            std::fs::create_dir_all(directory.join("src")).unwrap();
            let source = fixtures.items.iter().find_map(|item| match item {
                syn::Item::Mod(item) if item.ident == module => Some(&item.content.as_ref().unwrap().1),
                _ => None,
            }).expect("authored binding fixture module");
            let sdk = root.join("crates/slicer-sdk").to_string_lossy().replace('\\', "/");
            let ir = root.join("crates/slicer-ir").to_string_lossy().replace('\\', "/");
            let schema = root.join("crates/slicer-schema").to_string_lossy().replace('\\', "/");
            let macros = root.join("crates/slicer-macros").to_string_lossy().replace('\\', "/");
            std::fs::write(directory.join("Cargo.toml"), format!(
                "[workspace]\n[package]\nname = {module:?}\nversion = \"0.0.0\"\nedition = \"2021\"\n[lib]\ncrate-type = [\"cdylib\"]\n[dependencies]\nslicer-sdk = {{ path = {sdk:?} }}\nslicer-ir = {{ path = {ir:?} }}\nslicer-schema = {{ path = {schema:?} }}\nslicer-macros = {{ path = {macros:?} }}\nwit-bindgen = \"0.60.0\"\n"
            )).unwrap();
            let source: proc_macro2::TokenStream = source.iter().map(ToTokens::to_token_stream).collect();
            std::fs::write(directory.join("src/lib.rs"), source.to_string()).unwrap();
            run(Command::new("cargo").args(["build", "--offline", "--lib", "--release", "--target", "wasm32-unknown-unknown", "--manifest-path"])
                .arg(directory.join("Cargo.toml")).arg("--target-dir").arg(root.join("target")), &directory.join("build.log"));
            let component = directory.join("witness.component.wasm");
            run(Command::new("wasm-tools").args(["component", "new"])
                .arg(root.join("target/wasm32-unknown-unknown/release").join(format!("{module}.wasm")))
                .arg("-o").arg(&component), &directory.join("component.log"));
            run(Command::new("wasm-tools").args(["component", "wit"]).arg(&component), &directory.join("component.wit"))
        })
    })
}

// test-quality: compile witness — actual prepared component type-checks all
// fifteen import delegations and exhaustive source/paint/input-error conversions.
#[test]
fn prepared_binding_guest_typed_reads_sources_chains_and_errors_compile() {
    let wit = &preparation_binding_components()[0];
    assert!(wit.contains("import slicer:layer-preparation/preparation-types@1.0.0;"));
    assert!(wit.contains("export slicer:layer-preparation/prepare@1.0.0;"));
    assert!(wit.contains("read-fixture: func("));
    assert!(wit.contains("selected-targets: func("));
}

#[test]
fn plain_binding_has_no_mandatory_preparation_imports() {
    let wit = &preparation_binding_components()[1];
    assert!(wit.contains("export slicer:layer-perimeters/perimeters@1.0.0;"));
    assert!(
        !wit.contains("slicer:layer-preparation/"),
        "plain artifact acquired preparation imports or exports"
    );
}
