//! `CompiledModuleLive<'a>` — wasm-host-side view of a CompiledModule for the duration of a dispatch call.
//!
//! Carries the 5 fields the wasm dispatcher reads from `CompiledModule` (per packet 83
//! Step 4-pre survey of dispatch.rs trait impl bodies). Three are borrows (`&'a ModuleId`,
//! `&'a [String]`, `&'a` slices from the static-side module); two are `Arc` handles
//! (`Arc<WasmInstancePool>`, `Arc<WasmComponent>` wrapped in `Option` because some
//! placeholder/test modules don't have a real wasm component).
//!
//! Lifetime `'a` ties to the runtime-side `CompiledModuleStatic` (P85 territory) that
//! owns the underlying ModuleId / claims. Constructed at the dispatch call site in
//! `slicer-runtime/src/layer_executor.rs` (and the other executor files).

use std::sync::Arc;

use slicer_ir::{
    ConfigView, LayerCollectionIR, LayerPlanIR, MeshIR, ModuleId, PerimeterIR, RegionMapIR,
    SeamPlanIR, SliceIR, SupportGeometryIR, SupportPlanIR, SurfaceClassificationIR,
};

use crate::instance::WasmComponent;
use crate::pool::WasmInstancePool;

/// Wasm-host-side view of a `CompiledModule` valid for the duration of one dispatch call.
pub struct CompiledModuleLive<'a> {
    /// Borrowed module identifier from the owning `CompiledModuleStatic`.
    pub module_id: &'a ModuleId,
    /// Arc-cloned instance pool handle for leasing WASM instances during dispatch.
    pub instance_pool: Arc<WasmInstancePool>,
    /// Optional WASM component handle; `None` for placeholder or test modules.
    pub wasm_component: Option<Arc<WasmComponent>>,
    /// Borrowed claim list from the owning `CompiledModuleStatic`.
    pub claims: &'a [String],
    /// Arc-cloned config view projected for this module's declared reads.
    pub config_view: Arc<ConfigView>,
}

impl<'a> CompiledModuleLive<'a> {
    /// Construct a new `CompiledModuleLive` from its component fields.
    pub fn new(
        module_id: &'a ModuleId,
        instance_pool: Arc<WasmInstancePool>,
        wasm_component: Option<Arc<WasmComponent>>,
        claims: &'a [String],
        config_view: Arc<ConfigView>,
    ) -> Self {
        Self {
            module_id,
            instance_pool,
            wasm_component,
            claims,
            config_view,
        }
    }
}

// ---------------------------------------------------------------------------
// *StageInput<'a> borrow structs
// ---------------------------------------------------------------------------
//
// Borrow-struct inputs for runner trait methods. Each carries only the field-level
// borrows the dispatcher reads (per packet 83 Step 4-pre Blackboard/LayerArena
// access survey, 18/18 Category-B). Blackboard and LayerArena stay in slicer-runtime;
// the executor side projects their relevant fields into these structs before
// invoking the runner trait method (P83 design.md "Symmetric IR-typed trait boundary").

/// Input borrow struct for `LayerStageRunner::run_stage`. Carries IR-typed borrows
/// projected from the runtime-side Blackboard + LayerArena before the dispatch call.
pub struct LayerStageInput<'a> {
    /// Arc-cloned from `blackboard.mesh()` at the call site.
    pub mesh: Arc<MeshIR>,
    /// Reserved: paint annotations are now in SliceIR segment_annotations (AC-16).
    pub paint_regions: Option<()>,
    /// From `blackboard.seam_plan()`.
    pub seam_plan: Option<Arc<SeamPlanIR>>,
    /// From `blackboard.support_plan()`.
    pub support_plan: Option<Arc<SupportPlanIR>>,
    /// From `blackboard.region_map()`.
    pub region_map: Option<Arc<RegionMapIR>>,
    /// Pre-call read from `arena.slice()`.
    pub slice: Option<&'a SliceIR>,
    /// Pre-call read from `arena.perimeter()`.
    pub perimeter: Option<&'a PerimeterIR>,
    /// Pre-call read from `arena.layer_collection()`.
    pub layer_collection: Option<&'a LayerCollectionIR>,
    /// Committed-once global IR from `blackboard.surface_classification()`.
    /// Threaded into `push_slice_regions` so the WIT `surface-group` accessor resolves.
    pub surface_classification: Option<&'a SurfaceClassificationIR>,
}

/// Input borrow struct for `PrepassStageRunner::run_stage`. Mesh + optional IR slots
/// the SupportGeometry stage reads from the blackboard.
pub struct PrepassStageInput<'a> {
    /// Arc-cloned from `blackboard.mesh()` at the call site.
    pub mesh: Arc<MeshIR>,
    /// Arc-cloned from `blackboard.layer_plan()` at the call site.
    pub layer_plan: Option<Arc<LayerPlanIR>>,
    /// Arc-cloned from `blackboard.region_map()` at the call site.
    pub region_map: Option<Arc<RegionMapIR>>,
    /// Arc-cloned from `blackboard.support_geometry()` at the call site.
    /// Carries the coarse support-geometry IR (not the support plan) — used by
    /// `PrePass::SupportGeometry` as an input to subsequent prepass stages.
    pub support_geometry: Option<Arc<SupportGeometryIR>>,
    /// Unused lifetime anchor; kept for signature symmetry and future field-level borrows.
    pub _phantom: std::marker::PhantomData<&'a ()>,
}

/// Input borrow struct for `FinalizationStageRunner::run_stage`. Only `mesh` is read
/// from the blackboard inside the dispatcher; `layers: &mut Vec<LayerCollectionIR>`
/// stays a separate parameter on the trait method (it is the OUTPUT buffer, not an input).
pub struct FinalizationStageInput<'a> {
    /// Arc-cloned from `blackboard.mesh()` at the call site.
    pub mesh: Arc<MeshIR>,
    /// Unused lifetime anchor; kept for signature symmetry and future field-level borrows.
    pub _phantom: std::marker::PhantomData<&'a ()>,
}

/// Input borrow struct for `PostpassStageRunner::run_gcode_postprocess` and
/// `run_text_postprocess`. Mesh is the only blackboard field read in either variant;
/// the input/output payload (`commands` Vec or `text` String) is a separate parameter
/// on the trait method.
pub struct PostpassStageInput<'a> {
    /// Arc-cloned from `blackboard.mesh()` at the call site.
    pub mesh: Arc<MeshIR>,
    /// Unused lifetime anchor; kept for signature symmetry and future field-level borrows.
    pub _phantom: std::marker::PhantomData<&'a ()>,
}
