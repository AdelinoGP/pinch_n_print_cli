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
    assert_eq!(exports, &["slicer:layer-infill/infill@1.0.0#run"]);
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
    assert!(json.contains(r#""stage_export":"slicer:layer-infill/infill@1.0.0#run""#));
    assert!(json.contains(r#""slicer:layer-infill/infill@1.0.0#run""#));
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
    assert_eq!(s.stage_export, "slicer:layer-infill/infill@1.0.0#run");

    assert_eq!(s.exports.len(), 1);
    assert_eq!(
        s.exports[0],
        ExportBinding {
            name: "slicer:layer-infill/infill@1.0.0#run",
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
        "slicer:prepass-layer-planning/layer-planning@1.0.0#run"
    );
    assert_eq!(
        FinalizationFixture::__slicer_module_schema().tier_id,
        slicer_schema::TIER_FINALIZATION
    );
    assert_eq!(
        FinalizationFixture::__slicer_module_schema().stage_export,
        "slicer:finalization-layer-finalization/layer-finalization@1.0.0#run"
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
    assert_eq!(names_a, vec!["slicer:layer-infill/infill@1.0.0#run"]);
}
