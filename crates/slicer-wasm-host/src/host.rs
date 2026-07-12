//! WIT/component-model host-side bindings and execution context.
//!
//! This module provides:
//! - WIT-bindgen-generated types and traits for the layer world
//! - `HostExecutionContext` — per-call execution state carrying config, IR views,
//!   output collectors, and a `ResourceTable` for host resource handle management
//! - Trait implementations bridging the generated WIT interface to real host data

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use slicer_ir::MeshIR;
use wasmtime::component::{Resource, ResourceTable};

// ── Resource backing data structs ───────────────────────────────────────
// These are the actual data stored in the ResourceTable.
// The `bindgen!` `with` option maps WIT resource types to these structs.

/// Backing data for a `config-view` resource handle.
pub struct ConfigViewData {
    /// Config fields, pre-filtered to the module's declared reads.
    pub fields: HashMap<String, ConfigValueStorage>,
}

/// Reserved custom extrusion-role tag used to preserve PrimeTower through WIT boundaries.
pub const BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG: &str = "slicer.builtin/prime-tower@1";
/// Reserved custom extrusion-role tag used to preserve Skirt through WIT boundaries.
pub const BUILTIN_EXTRUSION_ROLE_SKIRT_TAG: &str = "slicer.builtin/skirt@1";
/// Reserved custom extrusion-role tag used to preserve Brim through WIT boundaries.
pub const BUILTIN_EXTRUSION_ROLE_BRIM_TAG: &str = "slicer.builtin/brim@1";
/// Reserved custom extrusion-role tag used to preserve InternalSolidInfill through
/// WIT boundaries (the WIT `extrusion-role` variant has no dedicated case).
pub const BUILTIN_EXTRUSION_ROLE_INTERNAL_SOLID_TAG: &str =
    "slicer.builtin/internal-solid-infill@1";

/// Storage for a single config value on the host side.
#[derive(Debug, Clone)]
pub enum ConfigValueStorage {
    /// Boolean value.
    Bool(bool),
    /// 64-bit integer value.
    Int(i64),
    /// 64-bit float value.
    Float(f64),
    /// String value.
    Str(String),
    /// List of floats.
    FloatList(Vec<f64>),
    /// List of strings.
    StringList(Vec<String>),
    /// Percent value (raw percent number; caller must resolve against a base
    /// via `ConfigView::get_abs_value`, mirroring `slicer_ir::ConfigValue::Percent`).
    Percent(f64),
    /// Float-or-percent value, mirroring `slicer_ir::ConfigValue::FloatOrPercent`.
    FloatOrPercent {
        /// The literal numeric value.
        value: f64,
        /// Whether `value` should be interpreted as a percent of some base.
        is_percent: bool,
    },
}

/// Normalize subnormal `f64` values to `0.0` at the typed-config boundary.
///
/// Mirrors `crates/slicer-runtime/src/config_schema.rs::normalize_subnormal` so that
/// modules calling `config-view.get-float` over the WIT boundary observe the same
/// numeric semantics as values coming from the schema parser. Documented in
/// docs/05_module_sdk.md and exercised by `config_schema_tdd::*subnormal*`.
#[inline]
pub fn normalize_subnormal_boundary(value: f64) -> f64 {
    if value.is_subnormal() {
        0.0
    } else {
        value
    }
}

/// Convert a slicer-ir `ConfigView` into a host-side `ConfigViewData`.
///
/// Maps each `slicer_ir::ConfigValue` variant to its `ConfigValueStorage`
/// counterpart. `ConfigValue::List` is converted to `FloatList` if all
/// elements are `Float`, `StringList` if all are `String`, and falls back
/// to `Str` with a debug representation otherwise.
pub fn config_view_to_data(ir: &slicer_ir::ConfigView) -> ConfigViewData {
    ConfigViewData {
        fields: ir
            .iter_entries()
            .map(|(k, v)| (k.to_string(), config_value_to_storage(v)))
            .collect(),
    }
}

/// Convert a single `slicer_ir::ConfigValue` to host-side `ConfigValueStorage`.
pub fn config_value_to_storage(v: &slicer_ir::ConfigValue) -> ConfigValueStorage {
    match v {
        slicer_ir::ConfigValue::Bool(b) => ConfigValueStorage::Bool(*b),
        slicer_ir::ConfigValue::Int(i) => ConfigValueStorage::Int(*i),
        slicer_ir::ConfigValue::Float(f) => ConfigValueStorage::Float(*f),
        slicer_ir::ConfigValue::String(s) => ConfigValueStorage::Str(s.clone()),
        // Percent-typed values round-trip through the dedicated `Percent` /
        // `FloatOrPercent` `ConfigValueStorage` variants (packet 150), which
        // map to the WIT `percent-val` / `float-or-percent-val` cases in
        // `HostConfigView::get` below. This preserves the percent/absolute
        // distinction end-to-end so a guest module's `get_abs_value` sees a
        // real percent instead of a downgraded string.
        slicer_ir::ConfigValue::Percent(p) => ConfigValueStorage::Percent(*p),
        slicer_ir::ConfigValue::FloatOrPercent { value, is_percent } => {
            ConfigValueStorage::FloatOrPercent {
                value: *value,
                is_percent: *is_percent,
            }
        }
        slicer_ir::ConfigValue::List(items) => {
            // Attempt homogeneous float or string list.
            let all_float = items
                .iter()
                .all(|i| matches!(i, slicer_ir::ConfigValue::Float(_)));
            if all_float {
                return ConfigValueStorage::FloatList(
                    items
                        .iter()
                        .filter_map(|i| match i {
                            slicer_ir::ConfigValue::Float(f) => Some(*f),
                            _ => None,
                        })
                        .collect(),
                );
            }
            let all_string = items
                .iter()
                .all(|i| matches!(i, slicer_ir::ConfigValue::String(_)));
            if all_string {
                return ConfigValueStorage::StringList(
                    items
                        .iter()
                        .filter_map(|i| match i {
                            slicer_ir::ConfigValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                );
            }
            // Fallback: debug representation.
            ConfigValueStorage::Str(format!("{items:?}"))
        }
    }
}

/// Backing data for a `slice-region-view` resource handle.
pub struct SliceRegionData {
    /// Object ID this region belongs to.
    pub object_id: String,
    /// Region ID.
    pub region_id: String,
    /// Slice polygons (bindgen ExPolygon type).
    pub polygons: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Infill areas.
    pub infill_areas: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Layer height at this Z.
    pub effective_layer_height: f32,
    /// Z height.
    pub z: f32,
    /// Whether this region has non-planar surfaces.
    pub has_nonplanar: bool,
    /// Boundary paint data.
    pub segment_annotations: Vec<layer::slicer::ir_handles::ir_handles::SegmentAnnotationsEntry>,
    /// Ordered (paint-semantic-name, value) pairs identifying this region's
    /// paint variant. Carries the painted FuzzySkin signal to the guest's
    /// `variant-chain()` accessor (D14: keeps FuzzySkin off segment_annotations).
    pub variant_chain: Vec<(String, layer::slicer::ir_handles::ir_handles::PaintValue)>,
    /// True when this region is support-eligible (from SurfaceClassificationIR).
    pub needs_support: bool,
    /// Minimum top-shell depth (0 = exposed) from PrePass::ShellClassification.
    pub top_shell_index: Option<u8>,
    /// Minimum bottom-shell depth (0 = exposed) from PrePass::ShellClassification.
    pub bottom_shell_index: Option<u8>,
    /// Polygon-precise top solid fill from shrinking-shadow projection.
    pub top_solid_fill: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Polygon-precise bottom solid fill from shrinking-shadow projection.
    pub bottom_solid_fill: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// True when this region is classified as a bridge region.
    pub is_bridge: bool,
    /// Per-layer expanded bridge polygons (empty if not a bridge region).
    pub bridge_areas: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees).
    pub bridge_orientation_deg: f32,
    /// Sparse-only infill polygon after host-side fill partition.
    /// Populated by `sync_perimeter_infill_areas_into_slice` at `Layer::Perimeters`
    /// commit; empty before that hook runs.
    pub sparse_infill_area: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Fill-role claim IDs held by the module that produced this region.
    pub held_claims: Vec<String>,
    /// Overhang area polygons. Populated from `SurfaceClassificationIR.overhang_quartile_polygons`
    /// at this region's global layer index, pre-filtered to overlap the region (packet 107).
    pub overhang_areas: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Quartile-banded overhang polygons for this region's layer, pre-filtered to
    /// overlap the region (packet 107). Mirrors `overhang_areas` but preserves the
    /// per-quartile grouping for callers that need severity-aware handling.
    pub overhang_quartile_polygons: Vec<layer::slicer::ir_handles::ir_handles::QuartileBand>,
    /// Surface group resolved from SurfaceClassificationIR. None when no group applies.
    pub surface_group: Option<layer::slicer::ir_handles::ir_handles::SurfaceGroup>,
}

/// Backing data for a `perimeter-region-view` resource handle.
pub struct PerimeterRegionData {
    /// Object ID.
    pub object_id: String,
    /// Region ID.
    pub region_id: String,
    /// Wall loops.
    pub wall_loops: Vec<layer::slicer::ir_handles::ir_handles::WallLoopView>,
    /// Infill areas after perimeter inset.
    pub infill_areas: Vec<layer::slicer::types::geometry::ExPolygon>,
    /// Resolved seam position (populated from PerimeterIR after seam-placer runs).
    pub resolved_seam: Option<(Point3, u32)>,
    /// Seam candidates pushed by the `Layer::Perimeters` guest for this region
    /// (populated from `PerimeterIR.regions[].seam_candidates`). Read by
    /// `Layer::PerimetersPostProcess` consumers via the `seam-candidates`
    /// WIT accessor.
    pub seam_candidates: Vec<(Point3, f32)>,
}

/// Backing data for an `infill-output-builder` resource handle.
/// Actual output state is collected on the `HostExecutionContext` directly.
pub struct InfillOutputBuilderData;

/// Backing data for a `perimeter-output-builder` resource handle.
pub struct PerimeterOutputBuilderData;

/// Backing data for a `slice-postprocess-builder` resource handle.
pub struct SlicePostprocessBuilderData;

/// Backing data for a `gcode-output-builder` resource handle.
pub struct GcodeOutputBuilderData;

/// Backing data for a `layer-collection-builder` resource handle.
///
/// Carries a per-call snapshot of the host-staged
/// `LayerCollectionIR.ordered_entities`, projected to
/// [`crate::dispatch::OrderedEntityView`] by
/// [`crate::dispatch::project_ordered_entities`] at
/// `push_layer_collection_builder` time. The snapshot is what
/// `HostLayerCollectionBuilder::get_ordered_entities` returns — repeated
/// reads from the same call hit this snapshot rather than the live arena.
///
/// The actual proposal (a permutation of `LayerCollectionIR.ordered_entities`
/// plus per-entity reversal flags) is stored on
/// `HostExecutionContext::layer_collection_proposal`.
pub struct LayerCollectionBuilderData {
    /// Snapshot of the host-staged `LayerCollectionIR.ordered_entities`
    /// captured at `push_layer_collection_builder` time.
    pub ordered_entities: Vec<crate::dispatch::OrderedEntityView>,
}

/// Global test-instrumentation counter incremented every time
/// `HostLayerCollectionBuilder::get_ordered_entities` is invoked. The
/// per-context `host_get_ordered_entities_call_count` field is reset by
/// `push_layer_collection_builder` so it cannot be observed from a host
/// integration test that runs `execute_per_layer` (the post-call context is
/// consumed internally). This static is the cross-call observation point used
/// by `macro_drain_invokes_host_get_ordered_entities_exactly_once`. Tests
/// MUST `.store(0, Ordering::SeqCst)` before exercising a layer call.
#[doc(hidden)]
pub static HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// Backing data for a `support-output-builder` resource handle.
pub struct SupportOutputBuilderData;

/// Backing data for a `paint-region-layer-view` resource handle.
pub struct PaintRegionLayerData {
    /// Layer index.
    pub layer_index: u32,
    /// Regions by semantic key string.
    pub regions_by_semantic:
        HashMap<String, Vec<layer::slicer::ir_handles::ir_handles::SemanticRegion>>,
    /// Custom regions by module ID.
    pub custom_regions: HashMap<String, Vec<layer::slicer::ir_handles::ir_handles::SemanticRegion>>,
    /// Pre-planned support-branch segments indexed by `(object_id, region_id)`,
    /// projected from `SupportPlanIR.entries` filtered to this layer index.
    /// Empty when no `SupportPlanIR` is committed on the blackboard.
    pub support_plan_segments:
        HashMap<(String, String), Vec<Vec<layer::slicer::types::geometry::Point3WithWidth>>>,
}

// ── Bindgen: Layer module world ─────────────────────────────────────────

#[allow(missing_docs)]
pub mod layer {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:world-layer/layer-module@1.0.0",
        imports: {
            default: trappable,
        },
        with: {
            "slicer:config/config-types.config-view": super::ConfigViewData,
            "slicer:ir-handles/ir-handles.slice-region-view": super::SliceRegionData,
            "slicer:ir-handles/ir-handles.perimeter-region-view": super::PerimeterRegionData,
            "slicer:ir-handles/ir-handles.infill-output-builder": super::InfillOutputBuilderData,
            "slicer:ir-handles/ir-handles.perimeter-output-builder": super::PerimeterOutputBuilderData,
            "slicer:ir-handles/ir-handles.slice-postprocess-builder": super::SlicePostprocessBuilderData,
            "slicer:ir-handles/ir-handles.gcode-output-builder": super::GcodeOutputBuilderData,
            "slicer:ir-handles/ir-handles.layer-collection-builder": super::LayerCollectionBuilderData,
            "slicer:ir-handles/ir-handles.support-output-builder": super::SupportOutputBuilderData,
            "slicer:ir-handles/ir-handles.paint-region-layer-view": super::PaintRegionLayerData,
        },
    });
}

// Re-export commonly used generated types for convenience.
pub use layer::slicer::config::config_types::ConfigValue;
pub use layer::slicer::config::config_types::FloatOrPercent;
/// Re-exports of the layer-module WIT `wall-boundary-type` variant and its
/// `material-boundary-segment` payload record, aliased to avoid colliding with
/// `slicer_ir::WallBoundaryType` / `slicer_ir::MaterialBoundarySegment`.
pub use layer::slicer::ir_handles::ir_handles::MaterialBoundarySegment as WitMaterialBoundarySegment;
/// Re-export of the layer-module WIT `retract-mode` variant. Used by host-side
/// `gcode-output-builder` handlers and `dispatch.rs` converters to forward the
/// `RetractMode` end-to-end across the guest→host boundary.
pub use layer::slicer::ir_handles::ir_handles::RetractMode as WitRetractMode;
pub use layer::slicer::ir_handles::ir_handles::WallBoundaryType as WitWallBoundaryType;
pub use layer::slicer::ir_handles::ir_handles::{
    GcodeMoveCmd, HostPerimeterOutputBuilder, PaintSemantic, PaintValue, RegionKey, SeamCandidate,
    SeamPosition, SegmentAnnotationsEntry, SegmentAnnotationsPolygon, SemanticRegion,
    WallFeatureFlag, WallLoopType, WallLoopView,
};
pub use layer::slicer::types::geometry::{
    BoundingBox3, ExPolygon, ExtrusionPath3d, ExtrusionRole, Point2, Point3, Point3WithWidth,
    Polygon,
};
pub use layer::LayerModule;
pub use layer::ModuleError;

// ── Bindgen: Prepass module world ─────────────────────────────────────

/// Backing data for prepass `mesh-analysis-output` resource.
pub struct MeshAnalysisOutputData;
/// Backing data for prepass `layer-plan-output` resource.
///
/// Proposals collected by `push_layer` calls during a WIT prepass invocation
/// are stored on `HostExecutionContext::layer_plan_proposals`.  This struct is
/// just a table entry so the resource-handle lifecycle works; the actual data
/// lives on the context.
pub struct LayerPlanOutputData;
/// Backing data for prepass `seam-planning-output` resource.
///
/// Seam-plan entries emitted by `push-seam-plan` during a WIT prepass
/// invocation are stored on `HostExecutionContext::seam_plan_entries`.
/// This struct is just a table-entry tag so the resource-handle lifecycle
/// works; the actual data lives on the context.
pub struct SeamPlanningOutputData;
/// Table-entry tag for the `support-geometry-output` builder resource.
///
/// The `SupportGeometry` prepass stage now passes a `support-geometry-output`
/// resource handle to the guest (mirroring `SeamPlanningOutput`); guest calls
/// to `push-support-plan-entry` append to `HostExecutionContext::support_plan_entries`.
pub struct SupportGeometryOutputData;

#[allow(missing_docs)]
pub mod prepass {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:world-prepass/prepass-module@1.0.0",
        imports: {
            default: trappable,
        },
        with: {
            // Reuse the layer world's generated geometry + config types so the
            // four worlds share one set of Rust types (packet 75, Phase 3 / ADR-0002).
            "slicer:types/geometry": super::layer::slicer::types::geometry,
            "slicer:config/config-types": super::layer::slicer::config::config_types,
            "slicer:common/host-services": super::layer::slicer::common::host_services,
            "slicer:common/module-errors": super::layer::slicer::common::module_errors,
            // `host-services#generate-arachne-walls` (packet 112, Step 9A) now
            // `use`s `extrusion-line` from `slicer:ir-handles/ir-handles`,
            // which transitively pulls that whole interface into every world
            // that imports `host-services`. Alias it to the layer world's
            // already-`impl`'d `ir_handles` module (the trivial empty
            // `impl ir::Host for HostExecutionContext {}` at
            // `crates/slicer-wasm-host/src/host.rs`) rather than requiring a
            // second, separate `Host` impl for this world's own bindgen copy.
            "slicer:ir-handles/ir-handles": super::layer::slicer::ir_handles::ir_handles,
        },
    });
}

pub use prepass::PrepassModule;

// ── Bindgen: Finalization module world ────────────────────────────────

/// Backing data for finalization `layer-collection-view` resource.
pub struct LayerCollectionViewData {
    /// Layer index.
    pub layer_index: u32,
    /// Z height.
    pub z: f32,
    /// Entity count.
    pub entity_count: u32,
    /// Tool changes observable by the guest through `tool-changes()`
    /// (docs/03 world-finalization.wit). Carried on the resource so
    /// the guest can consume real per-layer metadata rather than the
    /// previous empty-shell stub.
    pub tool_changes: Vec<(u32, u32, u32)>,
    /// Ordered print entities exposed by `ordered-entities()`.
    pub ordered_entities: Vec<slicer_ir::PrintEntity>,
    /// Z hops exposed by `z-hops()`.
    pub z_hops: Vec<slicer_ir::ZHop>,
}

impl LayerCollectionViewData {
    /// Build from a completed `LayerCollectionIR`. Tool-change triples
    /// are stored as `(after_entity_index, from_tool, to_tool)`.
    pub fn from_ir(ir: &slicer_ir::LayerCollectionIR) -> Self {
        Self {
            layer_index: ir.global_layer_index,
            z: ir.z,
            entity_count: ir.ordered_entities.len() as u32,
            tool_changes: ir
                .tool_changes
                .iter()
                .map(|t| (t.after_entity_index, t.from_tool, t.to_tool))
                .collect(),
            ordered_entities: ir.ordered_entities.clone(),
            z_hops: ir.z_hops.clone(),
        }
    }
}

/// Serializable entity mutation (mirrors WIT `entity-mutation` variant).
/// Used to carry guest-side mutation requests across the WIT boundary; the
/// host translates these into closures before forwarding to the SDK builder.
#[derive(Clone, Debug)]
pub enum WitEntityMutation {
    /// Set the `speed_factor` on the matched entity's path.
    SetSpeedFactor(f32),
    /// Set the per-point `flow_factor` on the matched entity's path.
    SetFlowFactor(f32),
}

/// Sort key selector (mirrors WIT `sort-key` enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitSortKey {
    /// Sort by (priority, entity_id) ascending — mirrors AC-3 expected ordering.
    ByPriorityAndEntityId,
    /// Sort by entity_id ascending.
    ByEntityId,
    /// Sort by (object_id, priority) — forward-looking for SequentialPrintOrder.
    ByObjectIdThenPriority,
}

/// Captured `finalization-output-builder` side effect emitted by a
/// guest during `run-finalization`. Stored by resource rep so the
/// post-call drain in `FinalizationStageRunner` can apply them.
#[derive(Clone, Debug)]
pub enum FinalizationBuilderPush {
    /// Guest requested `push-entity-to-layer(layer_index, path, tool_index, region_key)`.
    EntityToLayer {
        /// Layer index the entity was pushed to.
        layer_index: u32,
        /// Extrusion path content.
        path: slicer_ir::ExtrusionPath3D,
        /// Tool/extruder selector for the entity (explicit since the split).
        tool_index: u32,
        /// Region key for ordering / provenance (pure identity).
        region_key: slicer_ir::RegionKey,
    },
    /// Guest requested `push-entity-with-priority(layer_index, path, tool_index, region_key, priority)`.
    EntityToLayerWithPriority {
        /// Layer index the entity was pushed to.
        layer_index: u32,
        /// Extrusion path content.
        path: slicer_ir::ExtrusionPath3D,
        /// Tool/extruder selector for the entity (explicit since the split).
        tool_index: u32,
        /// Region key for ordering / provenance (pure identity).
        region_key: slicer_ir::RegionKey,
        /// Merge priority (lower = earlier in sorted output).
        priority: u32,
    },
    /// Guest requested `modify-entity(layer_index, entity_id, mutation)`.
    ModifyEntity {
        /// Layer index containing the entity to mutate.
        layer_index: u32,
        /// Stable entity identifier of the target entity.
        entity_id: u64,
        /// Serializable mutation to apply.
        mutation: WitEntityMutation,
    },
    /// Guest requested `sort-layer-by(layer_index, key)`.
    SortLayerBy {
        /// Layer index to sort.
        layer_index: u32,
        /// Key selector to use for the sort.
        key: WitSortKey,
    },
    /// Guest requested `insert-synthetic-layer-after(idx, layer_data)`.
    InsertSyntheticLayerAfter {
        /// Insert after this position in the outer `Vec<LayerCollectionIR>`.
        idx: u32,
        /// Z height of the new synthetic layer.
        z: f32,
        /// Extrusion paths to place into the new layer.
        paths: Vec<slicer_ir::ExtrusionPath3D>,
    },
    /// Guest requested `insert-synthetic-layer(z, paths)`.
    SyntheticLayer {
        /// Z height of the inserted synthetic layer.
        z: f32,
        /// Extrusion paths belonging to the synthetic layer.
        paths: Vec<slicer_ir::ExtrusionPath3D>,
    },
    /// Guest requested `insert-entity-at(layer_index, position, path, tool_index, region_key)`.
    InsertEntityAt {
        /// Layer index the entity is inserted into.
        layer_index: u32,
        /// Positional index within ordered_entities at which to insert.
        position: u32,
        /// Extrusion path content.
        path: slicer_ir::ExtrusionPath3D,
        /// Tool/extruder selector for the entity (explicit since the split).
        tool_index: u32,
        /// Region key for the new entity (pure identity).
        region_key: slicer_ir::RegionKey,
    },
    /// Guest requested `set-entity-order(layer_index, items)`.
    SetEntityOrder {
        /// Layer index whose entity order is being set.
        layer_index: u32,
        /// Permutation: items[new_position] = (original_position, reversed).
        items: Vec<(u32, bool)>,
    },
}

/// Backing data for finalization `finalization-output-builder` resource.
///
/// Captures every guest-side `push_entity_to_layer` /
/// `insert_synthetic_layer` call so the host can drain the recorded
/// effects after the typed `run-finalization` export returns (docs/03
/// world-finalization.wit). Order-preserving: entries are pushed in
/// the order the guest emitted them.
#[derive(Default)]
pub struct FinalizationOutputBuilderData {
    /// Captured push stream in guest-emission order.
    pub pushes: Vec<FinalizationBuilderPush>,
    /// Layer indices that have already been permuted via `set-entity-order`
    /// within this builder's lifetime. Used to enforce the packet-58 locked
    /// invariant "single permutation per layer per `run_finalization`
    /// invocation" (mirrors PathOptimization's contract on
    /// `layer-collection-builder`).
    pub permuted_layers: std::collections::HashSet<u32>,
}

#[allow(missing_docs)]
pub mod finalization {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:world-finalization/finalization-module@1.0.0",
        imports: {
            default: trappable,
        },
        with: {
            // Reuse the layer world's geometry + config types (packet 75, Phase 3 / ADR-0002).
            "slicer:types/geometry": super::layer::slicer::types::geometry,
            "slicer:config/config-types": super::layer::slicer::config::config_types,
            "slicer:common/host-services": super::layer::slicer::common::host_services,
            "slicer:common/module-errors": super::layer::slicer::common::module_errors,
            // See the identical note in the `prepass` bindgen! block above:
            // `host-services#generate-arachne-walls` (packet 112, Step 9A)
            // transitively pulls in `slicer:ir-handles/ir-handles`.
            "slicer:ir-handles/ir-handles": super::layer::slicer::ir_handles::ir_handles,
        },
    });
}

pub use finalization::FinalizationModule;

// ── Bindgen: Postpass module world ────────────────────────────────────

/// Backing data for postpass `gcode-output-builder` resource (shared with layer world).
pub struct PostpassGcodeOutputBuilderData;

#[allow(missing_docs)]
pub mod postpass {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:world-postpass/postpass-module@1.0.0",
        imports: {
            default: trappable,
        },
        with: {
            // Reuse the layer world's geometry + config types (packet 75, Phase 3 / ADR-0002).
            "slicer:types/geometry": super::layer::slicer::types::geometry,
            "slicer:config/config-types": super::layer::slicer::config::config_types,
            "slicer:common/host-services": super::layer::slicer::common::host_services,
            "slicer:common/module-errors": super::layer::slicer::common::module_errors,
            // See the identical note in the `prepass` bindgen! block above:
            // `host-services#generate-arachne-walls` (packet 112, Step 9A)
            // transitively pulls in `slicer:ir-handles/ir-handles`.
            "slicer:ir-handles/ir-handles": super::layer::slicer::ir_handles::ir_handles,
        },
    });
}

pub use postpass::PostpassModule;

pub use crate::marshal::accumulators::{
    GcodeCommandCollected, GcodeOutputCollected, InfillOutputCollected, PerimeterOutputCollected,
    SlicePostprocessCollected, SupportOutputCollected,
};
pub use crate::marshal::out::{
    collect_postpass_output, convert_infill_output, convert_perimeter_output,
    convert_support_output, merge_slice_postprocess_into,
};
pub use crate::marshal::OriginId;

// ── Per-call execution context ──────────────────────────────────────────

/// Tracks the guest's linear-memory growth across a single dispatch call.
///
/// Installed as the `Store`'s `ResourceLimiter` so wasmtime invokes
/// [`memory_growing`](wasmtime::ResourceLimiter::memory_growing) on every
/// `memory.grow` (and once at instantiation to size the initial memory).
/// Components without a linear memory never trigger the callback, so both
/// fields remain 0 — the report treats `(0, 0)` as "no sample".
/// The dispatcher reads `current_bytes` / `peak_bytes` after the typed call
/// returns and forwards them to the report's `on_module_end` hook.
#[derive(Debug, Default)]
pub struct MemTracker {
    /// Linear-memory size in bytes as of the most recent grow notification.
    pub current_bytes: u64,
    /// Highwater mark observed across this dispatch call (in bytes).
    pub peak_bytes: u64,
}

impl wasmtime::ResourceLimiter for MemTracker {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.current_bytes = desired as u64;
        if self.current_bytes > self.peak_bytes {
            self.peak_bytes = self.current_bytes;
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(true)
    }
}

/// Per-WASM-call execution context used as the `wasmtime::Store` data.
///
/// Created fresh for each module invocation. Carries:
/// - A `ResourceTable` for managing WIT resource handle lifetimes
/// - Mutable output collectors that accumulate guest-emitted data
/// - Diagnostic sinks (log messages)
///
/// After the call returns, the dispatcher extracts collected outputs from
/// this context and integrates them into the pipeline state.
pub struct HostExecutionContext {
    /// Resource handle table — manages lifetimes of host-provided resources.
    pub(crate) table: ResourceTable,
    /// Module identifier (from manifest).
    pub(crate) module_id: String,
    /// Monotonic clock start for profiling.
    start_time: Instant,
    /// Log messages emitted by the guest via host-services.log.
    pub(crate) log_messages: Vec<(String, String)>,

    // ── Output collectors ───────────────────────────────────────────
    /// Infill output collected during a call.
    pub(crate) infill_output: InfillOutputCollected,
    /// Perimeter output collected during a call.
    pub(crate) perimeter_output: PerimeterOutputCollected,
    /// Support output collected during a call.
    pub(crate) support_output: SupportOutputCollected,
    /// GCode output collected during a call.
    pub(crate) gcode_output: GcodeOutputCollected,
    /// Slice postprocess output collected during a call.
    pub(crate) slice_postprocess_output: SlicePostprocessCollected,
    /// Identity of the perimeter-region-view most recently accessed by the
    /// guest. Used to tag pushed post-process output so the commit path can
    /// preserve per-region identity instead of flattening into one synthetic
    /// region. Reset to `None` between calls (HostExecutionContext is per-call).
    pub(crate) current_perimeter_region: Option<OriginId>,
    /// Identity of the slice-region-view most recently accessed by the guest.
    /// Used to tag support post-process output pushes so the commit path can
    /// preserve per-region identity (grouping + structured diagnostic on
    /// untagged pushes) rather than silently flattening.
    pub(crate) current_slice_region: Option<OriginId>,
    /// Highest-precedence explicit perimeter origin, set by the WIT
    /// `perimeter-output-builder.set-current-origin` method. When set,
    /// `effective_perimeter_origin` returns this before falling back to
    /// the `touch_*`-driven `current_perimeter_region` /
    /// `current_slice_region` chain. The fallback chain STAYS as
    /// defence-in-depth — guests that do not call `set-current-origin`
    /// (e.g. legacy guests, or stages where the explicit origin is not
    /// yet wired) continue to resolve origin via the LIFO `touch_*` path.
    pub(crate) explicit_perimeter_origin: Option<OriginId>,

    /// Layer proposals collected from `push_layer` calls during a prepass
    /// `run-layer-planning` invocation.  Empty for all non-prepass stages.
    /// Drained by the prepass dispatch path after the WIT call returns.
    pub(crate) layer_plan_proposals: Vec<prepass::LayerProposal>,

    /// Per-object facet annotations collected from `push-facet-annotation`
    /// calls during a prepass `run-mesh-analysis` invocation. Tuple is
    /// `(object_id, FacetAnnotation)`. Insertion order is preserved so
    /// a downstream harvest can build deterministic output. Empty for
    /// all non-MeshAnalysis stages and when the guest declines to emit
    /// annotations (e.g. the current production path where
    /// `SurfaceClassificationIR` is still produced by the host built-in;
    /// see `mesh_analysis::execute_mesh_analysis`).
    pub(crate) mesh_analysis_annotations: Vec<(String, prepass::FacetAnnotation)>,

    /// Per-object surface groups collected from `push-surface-group`
    /// calls during a prepass `run-mesh-analysis` invocation. Tuple is
    /// `(object_id, SurfaceGroupProposal)`; insertion order preserved.
    /// Empty for all non-MeshAnalysis stages.
    pub(crate) mesh_analysis_surface_groups: Vec<(String, prepass::SurfaceGroupProposal)>,

    /// Seam-plan entries collected during a prepass `run-seam-planning`
    /// invocation. Stored as raw `prepass::SeamPlanEntry` records so the
    /// harvest helper can convert them to `SeamPlanIR` without losing any field.
    /// Empty for all non-prepass stages.
    pub(crate) seam_plan_entries: Vec<prepass::SeamPlanEntry>,

    /// Support-plan entries collected during a prepass
    /// `run-support-geometry` invocation. Stored as raw
    /// `prepass::SupportPlanEntry` records so the harvest helper can convert
    /// them to `SupportPlanIR` without losing any field. Empty for all
    /// non-prepass stages.
    pub(crate) support_plan_entries: Vec<prepass::SupportPlanEntry>,

    /// Finalization builder pushes collected during a finalization
    /// `run-finalization` invocation. The host-side
    /// `HostFinalizationOutputBuilder::drop` moves the resource's
    /// captured `pushes` here just before the resource is released,
    /// so `FinalizationStageRunner` can drain them even after the
    /// guest has dropped the builder handle (docs/03
    /// world-finalization.wit §finalization-output-builder).
    pub(crate) finalization_pushes: Vec<FinalizationBuilderPush>,

    /// Snapshot of all `LayerCollectionIR` layers passed into the finalization
    /// stage for this invocation. Populated by `push_finalization_layer_view`
    /// so that `get_ordered_entities` can serve synchronous read-backs against
    /// the pre-apply layer state without needing to search the resource table.
    /// Empty for all non-finalization stages.
    pub(crate) finalization_layer_snapshot: Vec<slicer_ir::LayerCollectionIR>,

    /// Layer-collection ordering proposal captured during a
    /// `Layer::PathOptimization` call via
    /// `layer-collection-builder.set-entity-order`. `None` means the
    /// guest emitted no proposal and the host fallback ordering applies.
    /// Reset to `None` by `push_layer_collection_builder` so a single
    /// `HostExecutionContext` reused across two layer calls cannot leak
    /// a proposal between them.
    pub(crate) layer_collection_proposal: Option<Vec<(u32, bool)>>,

    /// Counter incremented at the top of `HostLayerCollectionBuilder::get_ordered_entities`,
    /// reset to `0` by `push_layer_collection_builder`. Exists to pin the macro-call-once
    /// contract — the macro drain MUST call `wit_resource.get_ordered_entities()` exactly
    /// once per `run-path-optimization` invocation; subsequent calls in the trait method
    /// hit the SDK-local cache instead of round-tripping to the WIT host.
    pub(crate) host_get_ordered_entities_call_count: u32,

    /// Runtime IR read paths accessed by the guest via WIT view methods
    /// during this call. Populated by instrumenting each view method to
    /// record the exact IR path (e.g. `SliceIR.regions.polygons`) when
    /// called. Extracted by the dispatcher and returned as part of
    /// `ModuleAccessAudit.runtime_reads`.
    pub(crate) runtime_reads: Vec<String>,

    /// Runtime IR write paths accessed by the guest via WIT builder methods
    /// during this call. Populated by instrumenting each builder method to
    /// record the exact IR path (e.g. `PerimeterIR.regions.walls`) when
    /// called. Extracted by the dispatcher and returned as part of
    /// `ModuleAccessAudit.runtime_writes`.
    pub(crate) runtime_writes: Vec<String>,

    // ── Z envelope fields ─────────────────────────────────────────────
    /// Layer Z floor (lower bound of the Z envelope).
    layer_z: f32,
    /// Effective layer height (envelope height).
    effective_layer_height: f32,
    /// Bottom Z of catch-up layer, or `None` if not a catch-up layer.
    catchup_z_bottom: Option<f32>,
    /// Host-owned mesh IR used by mesh-query host services.
    pub(crate) mesh_ir: Option<Arc<MeshIR>>,

    /// Fill-role claim IDs held by `module_id` per `(object_id, region_id)`,
    /// resolved by `validation::resolve_held_claims` against the per-region
    /// `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder` keys before
    /// the WIT call. Looked up by `push_slice_regions` to populate
    /// `SliceRegionData.held_claims`. Empty for non-`Layer::Infill` calls
    /// (the WIT accessor returns the empty list, which the SDK convention
    /// treats as "holds all" — packet 36 / 12-rev1 behavior).
    pub(crate) held_claims_per_region: std::collections::HashMap<(String, String), Vec<String>>,

    /// Linear-memory tracker, installed as the store's `ResourceLimiter`
    /// to sample guest memory growth for the slicer report.
    pub(crate) mem_tracker: MemTracker,
}

/// Consuming builder for [`HostExecutionContext`].
///
/// Per spec §6.4 — required positional args are `module_id`, `layer_z`,
/// `effective_layer_height`; the two optional Z-envelope/mesh slots
/// (`catchup_z_bottom`, `mesh_ir`) are settable via fluent setters. All
/// per-call output accumulators default to `Default::default()` (i.e.
/// empty) and are mutated through accessors on the built context.
#[must_use = "HostExecutionContextBuilder yields a HostExecutionContext via .build()"]
pub struct HostExecutionContextBuilder {
    module_id: String,
    layer_z: f32,
    effective_layer_height: f32,
    catchup_z_bottom: Option<f32>,
    mesh_ir: Option<Arc<MeshIR>>,
}

impl HostExecutionContextBuilder {
    /// Start a builder with the three required identity fields.
    ///
    /// `layer_z` is the layer floor (lower Z bound). `effective_layer_height`
    /// is the envelope height. The catch-up bottom and mesh IR slots default
    /// to `None`; set them via [`Self::catchup_z_bottom`] / [`Self::mesh_ir`]
    /// before calling [`Self::build`] when needed.
    pub fn new(module_id: impl Into<String>, layer_z: f32, effective_layer_height: f32) -> Self {
        Self {
            module_id: module_id.into(),
            layer_z,
            effective_layer_height,
            catchup_z_bottom: None,
            mesh_ir: None,
        }
    }

    /// Set the catch-up layer's bottom Z. `Some` marks this call as a
    /// catch-up layer (the envelope floor becomes `catchup_z_bottom`
    /// instead of `layer_z`).
    pub fn catchup_z_bottom(mut self, v: Option<f32>) -> Self {
        self.catchup_z_bottom = v;
        self
    }

    /// Set the host-owned mesh IR for mesh-query host services.
    pub fn mesh_ir(mut self, v: Option<Arc<MeshIR>>) -> Self {
        self.mesh_ir = v;
        self
    }

    /// Finalize the builder into a fresh `HostExecutionContext`.
    pub fn build(self) -> HostExecutionContext {
        HostExecutionContext {
            table: ResourceTable::new(),
            module_id: self.module_id,
            start_time: Instant::now(),
            log_messages: Vec::new(),
            infill_output: InfillOutputCollected::default(),
            perimeter_output: PerimeterOutputCollected::default(),
            support_output: SupportOutputCollected::default(),
            gcode_output: GcodeOutputCollected::default(),
            slice_postprocess_output: SlicePostprocessCollected::default(),
            current_perimeter_region: None,
            current_slice_region: None,
            explicit_perimeter_origin: None,
            layer_plan_proposals: Vec::new(),
            mesh_analysis_annotations: Vec::new(),
            mesh_analysis_surface_groups: Vec::new(),
            seam_plan_entries: Vec::new(),
            support_plan_entries: Vec::new(),
            finalization_pushes: Vec::new(),
            finalization_layer_snapshot: Vec::new(),
            layer_collection_proposal: None,
            host_get_ordered_entities_call_count: 0,
            runtime_reads: Vec::new(),
            runtime_writes: Vec::new(),
            layer_z: self.layer_z,
            effective_layer_height: self.effective_layer_height,
            catchup_z_bottom: self.catchup_z_bottom,
            mesh_ir: self.mesh_ir,
            held_claims_per_region: std::collections::HashMap::new(),
            mem_tracker: MemTracker::default(),
        }
    }
}

impl HostExecutionContext {
    /// Module identifier (from manifest).
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Log messages emitted by the guest via host-services.log.
    pub fn log_messages(&self) -> &[(String, String)] {
        &self.log_messages
    }

    /// Mutable handle to the log-messages collector.
    pub fn log_messages_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.log_messages
    }

    /// Per-call infill output collector.
    pub fn infill_output(&self) -> &InfillOutputCollected {
        &self.infill_output
    }

    /// Mutable handle to the per-call infill output collector.
    pub fn infill_output_mut(&mut self) -> &mut InfillOutputCollected {
        &mut self.infill_output
    }

    /// Per-call perimeter output collector.
    pub fn perimeter_output(&self) -> &PerimeterOutputCollected {
        &self.perimeter_output
    }

    /// Mutable handle to the per-call perimeter output collector.
    pub fn perimeter_output_mut(&mut self) -> &mut PerimeterOutputCollected {
        &mut self.perimeter_output
    }

    /// Per-call support output collector.
    pub fn support_output(&self) -> &SupportOutputCollected {
        &self.support_output
    }

    /// Mutable handle to the per-call support output collector.
    pub fn support_output_mut(&mut self) -> &mut SupportOutputCollected {
        &mut self.support_output
    }

    /// Per-call GCode output collector.
    pub fn gcode_output(&self) -> &GcodeOutputCollected {
        &self.gcode_output
    }

    /// Mutable handle to the per-call GCode output collector.
    pub fn gcode_output_mut(&mut self) -> &mut GcodeOutputCollected {
        &mut self.gcode_output
    }

    /// Per-call slice-postprocess output collector.
    pub fn slice_postprocess_output(&self) -> &SlicePostprocessCollected {
        &self.slice_postprocess_output
    }

    /// Mutable handle to the per-call slice-postprocess output collector.
    pub fn slice_postprocess_output_mut(&mut self) -> &mut SlicePostprocessCollected {
        &mut self.slice_postprocess_output
    }

    /// Identity of the most recently accessed perimeter region (see field doc).
    pub fn current_perimeter_region(&self) -> Option<&OriginId> {
        self.current_perimeter_region.as_ref()
    }

    /// Override the current perimeter region origin (test/dispatch helper).
    pub fn set_current_perimeter_region(&mut self, origin: Option<OriginId>) {
        self.current_perimeter_region = origin;
    }

    /// Identity of the most recently accessed slice region (see field doc).
    pub fn current_slice_region(&self) -> Option<&OriginId> {
        self.current_slice_region.as_ref()
    }

    /// Override the current slice region origin (test/dispatch helper).
    pub fn set_current_slice_region(&mut self, origin: Option<OriginId>) {
        self.current_slice_region = origin;
    }

    /// Effective perimeter-output origin, with slice-region fallback for
    /// `Layer::Perimeters` guests.
    ///
    /// `PerimeterRegionView` does not exist at `Layer::Perimeters` stage —
    /// guests like `arachne-perimeters` and `classic-perimeters` consume
    /// `SliceRegionView` and write a fresh `PerimeterIR`. With no perimeter
    /// touch site, `current_perimeter_region` is `None`, and origin-tagged
    /// pushes (`push_wall_loop`, `set_infill_areas`, etc.) used to fall
    /// through to the "untagged" path in `convert_perimeter_output`, which
    /// emitted a single `PerimeterRegion` with `object_id = ""`. The host
    /// region-partition then missed the slice-side `(object_id, region_id)`
    /// HashMap lookup and skipped every region, leaving `sparse_infill_area`
    /// empty (the cube "no sparse infill" symptom).
    ///
    /// At `Layer::PerimetersPostProcess` (e.g. seam-placer) the guest reads
    /// `PerimeterRegionView`, `current_perimeter_region` is set, and the
    /// fallback is a no-op.
    pub(crate) fn effective_perimeter_origin(&self) -> Option<OriginId> {
        self.explicit_perimeter_origin
            .clone()
            .or_else(|| self.current_perimeter_region.clone())
            .or_else(|| self.current_slice_region.clone())
    }

    /// Layer proposals collected during a prepass `run-layer-planning` call.
    pub fn layer_plan_proposals(&self) -> &[prepass::LayerProposal] {
        &self.layer_plan_proposals
    }

    /// Mutable access to the layer proposals collector.
    pub fn layer_plan_proposals_mut(&mut self) -> &mut Vec<prepass::LayerProposal> {
        &mut self.layer_plan_proposals
    }

    /// Per-object facet annotations collected during `run-mesh-analysis`.
    pub fn mesh_analysis_annotations(&self) -> &[(String, prepass::FacetAnnotation)] {
        &self.mesh_analysis_annotations
    }

    /// Per-object surface groups collected during `run-mesh-analysis`.
    pub fn mesh_analysis_surface_groups(&self) -> &[(String, prepass::SurfaceGroupProposal)] {
        &self.mesh_analysis_surface_groups
    }

    /// Seam-plan entries collected during `run-seam-planning`.
    pub fn seam_plan_entries(&self) -> &[prepass::SeamPlanEntry] {
        &self.seam_plan_entries
    }

    /// Support-plan entries collected during `run-support-geometry`.
    pub fn support_plan_entries(&self) -> &[prepass::SupportPlanEntry] {
        &self.support_plan_entries
    }

    /// Finalization builder pushes captured during `run-finalization`.
    pub fn finalization_pushes(&self) -> &[FinalizationBuilderPush] {
        &self.finalization_pushes
    }

    /// Layer-collection ordering proposal from `set-entity-order`, if any.
    pub fn layer_collection_proposal(&self) -> Option<&Vec<(u32, bool)>> {
        self.layer_collection_proposal.as_ref()
    }

    /// Runtime IR read paths recorded by view-method instrumentation.
    pub fn runtime_reads(&self) -> &[String] {
        &self.runtime_reads
    }

    /// Runtime IR write paths recorded by builder-method instrumentation.
    pub fn runtime_writes(&self) -> &[String] {
        &self.runtime_writes
    }

    /// Host-owned mesh IR used by mesh-query host services, if any.
    pub fn mesh_ir(&self) -> Option<&Arc<MeshIR>> {
        self.mesh_ir.as_ref()
    }

    /// Resource-limiter handle used by the wasmtime store.
    pub fn mem_tracker_mut(&mut self) -> &mut MemTracker {
        &mut self.mem_tracker
    }

    /// Read-only snapshot of the linear-memory tracker for reports.
    pub fn mem_tracker(&self) -> &MemTracker {
        &self.mem_tracker
    }

    /// Replace the per-region held-claim map. Called by the dispatcher after
    /// resolving each region's `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`
    /// against the active module's manifest claims.
    pub fn set_held_claims_per_region(
        &mut self,
        map: std::collections::HashMap<(String, String), Vec<String>>,
    ) {
        self.held_claims_per_region = map;
    }

    /// Look up the held-claim set for a specific region. Returns `&[]` when no
    /// entry exists (the SDK convention is that empty `held_claims` means the
    /// module holds no fill claims for this region, so emission is suppressed;
    /// see `slicer-sdk/src/views.rs`).
    pub fn held_claims_for(&self, object_id: &str, region_id: &str) -> &[String] {
        self.held_claims_per_region
            .get(&(object_id.to_string(), region_id.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Returns the Z envelope floor for this layer.
    ///
    /// For catch-up layers the floor is `catchup_z_bottom`; otherwise it is `layer_z`.
    fn z_envelope_floor(&self) -> f32 {
        self.catchup_z_bottom.unwrap_or(self.layer_z)
    }

    /// Returns the Z envelope ceiling for this layer.
    fn z_envelope_ceiling(&self) -> f32 {
        self.z_envelope_floor() + self.effective_layer_height
    }

    /// Validates that `z` is within the Z envelope `[floor, ceiling]` (inclusive).
    ///
    /// Returns `Err` with a descriptive message on violation.
    fn check_z_envelope(&self, z: f32) -> Result<(), String> {
        let floor = self.z_envelope_floor();
        let ceiling = self.z_envelope_ceiling();
        if z < floor {
            Err(format!(
                "Z_ENVELOPE_VIOLATION: Z {} below layer.z floor {}",
                z, floor
            ))
        } else if z > ceiling {
            Err(format!(
                "Z_ENVELOPE_VIOLATION: Z {} above layer.z ceiling {}",
                z, ceiling
            ))
        } else {
            Ok(())
        }
    }

    /// Record a runtime IR write path accessed by the guest.
    pub fn record_write(&mut self, path: &'static str) {
        self.runtime_writes.push(String::from(path));
    }

    /// Push a config-view resource and return its handle.
    pub fn push_config_view(
        &mut self,
        data: ConfigViewData,
    ) -> wasmtime::Result<Resource<ConfigViewData>> {
        Ok(self.table.push(data)?)
    }

    /// Push a slice-region-view resource and return its handle.
    pub fn push_slice_region(
        &mut self,
        data: SliceRegionData,
    ) -> wasmtime::Result<Resource<SliceRegionData>> {
        Ok(self.table.push(data)?)
    }

    /// Push a perimeter-region-view resource and return its handle.
    pub fn push_perimeter_region(
        &mut self,
        data: PerimeterRegionData,
    ) -> wasmtime::Result<Resource<PerimeterRegionData>> {
        Ok(self.table.push(data)?)
    }

    /// Push an infill-output-builder resource and return its handle.
    pub fn push_infill_output_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<InfillOutputBuilderData>> {
        Ok(self.table.push(InfillOutputBuilderData)?)
    }

    /// Push a perimeter-output-builder resource and return its handle.
    pub fn push_perimeter_output_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<PerimeterOutputBuilderData>> {
        Ok(self.table.push(PerimeterOutputBuilderData)?)
    }

    /// Push a support-output-builder resource and return its handle.
    pub fn push_support_output_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<SupportOutputBuilderData>> {
        Ok(self.table.push(SupportOutputBuilderData)?)
    }

    /// Push a gcode-output-builder resource and return its handle.
    pub fn push_gcode_output_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<GcodeOutputBuilderData>> {
        Ok(self.table.push(GcodeOutputBuilderData)?)
    }

    /// Push a layer-collection-builder resource and return its handle.
    /// Resets `layer_collection_proposal` to `None` so a context reused
    /// across two `Layer::PathOptimization` calls cannot leak a stale
    /// proposal from the previous call. Resets the
    /// `host_get_ordered_entities_call_count` counter to `0` so each new
    /// `Layer::PathOptimization` invocation starts a fresh count for the
    /// macro-call-once contract test.
    ///
    /// The `ordered_entities` snapshot is captured by
    /// [`crate::dispatch::project_ordered_entities`] at dispatch time and
    /// stashed on the resource so the host-side
    /// `HostLayerCollectionBuilder::get_ordered_entities` impl can serve
    /// reads from it without re-touching the live arena.
    pub fn push_layer_collection_builder(
        &mut self,
        ordered_entities: Vec<crate::dispatch::OrderedEntityView>,
    ) -> wasmtime::Result<Resource<LayerCollectionBuilderData>> {
        self.layer_collection_proposal = None;
        self.host_get_ordered_entities_call_count = 0;
        Ok(self
            .table
            .push(LayerCollectionBuilderData { ordered_entities })?)
    }

    /// Test-only accessor for the
    /// `host_get_ordered_entities_call_count` counter. Used by the
    /// host-side macro-call-once test to verify that the SDK macro
    /// drain hits the WIT host exactly once per
    /// `Layer::PathOptimization` invocation regardless of how many
    /// times the module's trait method is called.
    #[doc(hidden)]
    pub fn host_get_ordered_entities_call_count(&self) -> u32 {
        self.host_get_ordered_entities_call_count
    }

    /// Push a slice-postprocess-builder resource and return its handle.
    pub fn push_slice_postprocess_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<SlicePostprocessBuilderData>> {
        Ok(self.table.push(SlicePostprocessBuilderData)?)
    }

    /// Push a paint-region-layer-view resource and return its handle.
    pub fn push_paint_region_layer_view(
        &mut self,
        data: PaintRegionLayerData,
    ) -> wasmtime::Result<Resource<PaintRegionLayerData>> {
        Ok(self.table.push(data)?)
    }

    // ── Prepass world resource pushers ──────────────────────────────

    /// Push a mesh-analysis-output resource (prepass world).
    pub fn push_mesh_analysis_output(
        &mut self,
    ) -> wasmtime::Result<Resource<prepass::MeshAnalysisOutput>> {
        let rep = self.table.push(MeshAnalysisOutputData)?;
        Ok(Resource::new_own(rep.rep()))
    }

    /// Push a layer-plan-output resource (prepass world).
    pub fn push_layer_plan_output(
        &mut self,
    ) -> wasmtime::Result<Resource<prepass::LayerPlanOutput>> {
        let rep = self.table.push(LayerPlanOutputData)?;
        Ok(Resource::new_own(rep.rep()))
    }

    /// Push a seam-planning-output resource (prepass world). The
    /// returned handle is what the host passes into
    /// `run-seam-planning`; guest calls to `push-seam-plan` go
    /// through `HostSeamPlanningOutput::push_seam_plan` below,
    /// which appends entries to `seam_plan_entries`.
    pub fn push_seam_planning_output(
        &mut self,
    ) -> wasmtime::Result<Resource<prepass::SeamPlanningOutput>> {
        let rep = self.table.push(SeamPlanningOutputData)?;
        Ok(Resource::new_own(rep.rep()))
    }

    /// Push a `support-geometry-output` builder resource.
    ///
    /// The returned handle is what the host passes into
    /// `run-support-geometry`; guest calls to `push-support-plan-entry` go
    /// through `HostSupportGeometryOutput::push_support_plan_entry` below,
    /// which appends entries to `support_plan_entries`.
    pub fn push_support_geometry_output(
        &mut self,
    ) -> wasmtime::Result<Resource<prepass::SupportGeometryOutput>> {
        let rep = self.table.push(SupportGeometryOutputData)?;
        Ok(Resource::new_own(rep.rep()))
    }

    // ── Finalization world resource pushers ─────────────────────────

    /// Push a finalization-output-builder resource (finalization world).
    pub fn push_finalization_output_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<finalization::FinalizationOutputBuilder>> {
        let rep = self.table.push(FinalizationOutputBuilderData::default())?;
        Ok(Resource::new_own(rep.rep()))
    }

    /// Push one `LayerCollectionView` resource built from a completed
    /// `LayerCollectionIR`. Returns the typed wit-bindgen handle so it
    /// can be forwarded into `call_run_finalization` as part of the
    /// `list<layer-collection-view>` parameter.
    ///
    /// Also captures the layer into `finalization_layer_snapshot` so that
    /// `get_ordered_entities` can serve synchronous read-backs against the
    /// pre-apply layer state during the finalization call.
    pub fn push_finalization_layer_view(
        &mut self,
        ir: &slicer_ir::LayerCollectionIR,
    ) -> wasmtime::Result<Resource<finalization::LayerCollectionView>> {
        self.finalization_layer_snapshot.push(ir.clone());
        let rep = self.table.push(LayerCollectionViewData::from_ir(ir))?;
        Ok(Resource::new_own(rep.rep()))
    }

    /// Drain captured pushes collected by the finalization output
    /// builder. Reads from `finalization_pushes` (populated by the
    /// builder's `drop` handler) rather than from the builder's
    /// resource-table entry, which wit-bindgen has already reclaimed
    /// by the time this function is called (guest owns the resource
    /// handle; dropping it moves captured data onto the context).
    pub fn drain_finalization_output_builder(&mut self) -> Vec<FinalizationBuilderPush> {
        std::mem::take(&mut self.finalization_pushes)
    }

    // ── Postpass world resource pushers ─────────────────────────────

    /// Push a gcode-output-builder resource (postpass world).
    pub fn push_postpass_gcode_output_builder(
        &mut self,
    ) -> wasmtime::Result<Resource<postpass::GcodeOutputBuilder>> {
        let rep = self.table.push(PostpassGcodeOutputBuilderData)?;
        Ok(Resource::new_own(rep.rep()))
    }
}

const MESH_QUERY_EPSILON: f32 = 1.0e-4;

fn object_not_found_error(service: &str, object_id: &str) -> wasmtime::Error {
    wasmtime::Error::msg(format!(
        "OBJECT_NOT_FOUND: host-service {service} could not find object '{object_id}'"
    ))
}

fn lookup_object_mesh<'a>(
    ctx: &'a HostExecutionContext,
    service: &str,
    object_id: &str,
) -> wasmtime::Result<Option<&'a slicer_ir::ObjectMesh>> {
    let Some(mesh_ir) = ctx.mesh_ir.as_ref() else {
        return Ok(None);
    };

    mesh_ir
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .map(Some)
        .ok_or_else(|| object_not_found_error(service, object_id))
}

fn transform_mesh_point(
    transform: &slicer_ir::Transform3d,
    point: &slicer_ir::Point3,
) -> slicer_ir::Point3 {
    let matrix = &transform.matrix;
    if matrix.iter().all(|value| *value == 0.0) {
        return *point;
    }

    let x = f64::from(point.x);
    let y = f64::from(point.y);
    let z = f64::from(point.z);
    let transformed_x = matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12];
    let transformed_y = matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13];
    let transformed_z = matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14];
    let transformed_w = matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15];

    if transformed_w != 0.0 && transformed_w != 1.0 {
        return slicer_ir::Point3 {
            x: (transformed_x / transformed_w) as f32,
            y: (transformed_y / transformed_w) as f32,
            z: (transformed_z / transformed_w) as f32,
        };
    }

    slicer_ir::Point3 {
        x: transformed_x as f32,
        y: transformed_y as f32,
        z: transformed_z as f32,
    }
}

fn point3_to_array(point: slicer_ir::Point3) -> [f32; 3] {
    [point.x, point.y, point.z]
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn triangle_vertices(
    object: &slicer_ir::ObjectMesh,
    triangle: &[u32],
) -> Option<[slicer_ir::Point3; 3]> {
    if triangle.len() != 3 {
        return None;
    }

    let a = object.mesh.vertices.get(triangle[0] as usize)?;
    let b = object.mesh.vertices.get(triangle[1] as usize)?;
    let c = object.mesh.vertices.get(triangle[2] as usize)?;
    Some([
        transform_mesh_point(&object.transform, a),
        transform_mesh_point(&object.transform, b),
        transform_mesh_point(&object.transform, c),
    ])
}

fn raycast_vertical_triangle(
    triangle: [slicer_ir::Point3; 3],
    x: f32,
    y: f32,
    start_z: f32,
) -> Option<f32> {
    let origin = [x, y, start_z];
    let direction = [0.0, 0.0, -1.0];
    let a = point3_to_array(triangle[0]);
    let b = point3_to_array(triangle[1]);
    let c = point3_to_array(triangle[2]);
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let pvec = cross(direction, edge2);
    let det = dot(edge1, pvec);
    if det.abs() < MESH_QUERY_EPSILON {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = sub(origin, a);
    let u = dot(tvec, pvec) * inv_det;
    if !(-MESH_QUERY_EPSILON..=1.0 + MESH_QUERY_EPSILON).contains(&u) {
        return None;
    }

    let qvec = cross(tvec, edge1);
    let v = dot(direction, qvec) * inv_det;
    if v < -MESH_QUERY_EPSILON || u + v > 1.0 + MESH_QUERY_EPSILON {
        return None;
    }

    let distance = dot(edge2, qvec) * inv_det;
    if distance < -MESH_QUERY_EPSILON {
        return None;
    }

    Some(start_z - distance.max(0.0))
}

fn triangle_unit_normal(triangle: [slicer_ir::Point3; 3]) -> Option<[f32; 3]> {
    let a = point3_to_array(triangle[0]);
    let b = point3_to_array(triangle[1]);
    let c = point3_to_array(triangle[2]);
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let normal = cross(edge1, edge2);
    let magnitude = dot(normal, normal).sqrt();
    if magnitude <= MESH_QUERY_EPSILON {
        return None;
    }

    Some([
        normal[0] / magnitude,
        normal[1] / magnitude,
        normal[2] / magnitude,
    ])
}

fn point_on_triangle(point: slicer_ir::Point3, triangle: [slicer_ir::Point3; 3]) -> bool {
    let a = point3_to_array(triangle[0]);
    let b = point3_to_array(triangle[1]);
    let c = point3_to_array(triangle[2]);
    let p = point3_to_array(point);
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let normal = cross(edge1, edge2);
    let normal_length = dot(normal, normal).sqrt();
    if normal_length <= MESH_QUERY_EPSILON {
        return false;
    }

    let ap = sub(p, a);
    let plane_distance = dot(normal, ap).abs() / normal_length;
    if plane_distance > MESH_QUERY_EPSILON {
        return false;
    }

    let d00 = dot(edge1, edge1);
    let d01 = dot(edge1, edge2);
    let d11 = dot(edge2, edge2);
    let d20 = dot(ap, edge1);
    let d21 = dot(ap, edge2);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() <= MESH_QUERY_EPSILON {
        return false;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    u >= -MESH_QUERY_EPSILON
        && v >= -MESH_QUERY_EPSILON
        && w >= -MESH_QUERY_EPSILON
        && u <= 1.0 + MESH_QUERY_EPSILON
        && v <= 1.0 + MESH_QUERY_EPSILON
        && w <= 1.0 + MESH_QUERY_EPSILON
}

fn raycast_z_down_mesh_query(
    ctx: &mut HostExecutionContext,
    object_id: &str,
    x: f32,
    y: f32,
    start_z: f32,
) -> wasmtime::Result<Option<f32>> {
    ctx.runtime_reads.push(String::from("MeshIR"));
    let Some(object) = lookup_object_mesh(ctx, "raycast-z-down", object_id)? else {
        return Ok(None);
    };

    let mut best_hit = None;
    for triangle in object.mesh.indices.chunks_exact(3) {
        let Some(vertices) = triangle_vertices(object, triangle) else {
            continue;
        };
        let Some(hit_z) = raycast_vertical_triangle(vertices, x, y, start_z) else {
            continue;
        };
        if hit_z > start_z + MESH_QUERY_EPSILON {
            continue;
        }
        if best_hit.is_none_or(|current| hit_z > current) {
            best_hit = Some(hit_z);
        }
    }

    Ok(best_hit)
}

fn surface_normal_at_mesh_query(
    ctx: &mut HostExecutionContext,
    object_id: &str,
    x: f32,
    y: f32,
    z: f32,
) -> wasmtime::Result<Option<slicer_ir::Point3>> {
    ctx.runtime_reads.push(String::from("MeshIR"));
    let Some(object) = lookup_object_mesh(ctx, "surface-normal-at", object_id)? else {
        return Ok(None);
    };
    let query_point = slicer_ir::Point3 { x, y, z };

    for triangle in object.mesh.indices.chunks_exact(3) {
        let Some(vertices) = triangle_vertices(object, triangle) else {
            continue;
        };
        if !point_on_triangle(query_point, vertices) {
            continue;
        }
        let Some(normal) = triangle_unit_normal(vertices) else {
            continue;
        };
        return Ok(Some(slicer_ir::Point3 {
            x: normal[0],
            y: normal[1],
            z: normal[2],
        }));
    }

    Ok(None)
}

fn object_bounds_mesh_query(
    ctx: &mut HostExecutionContext,
    object_id: &str,
) -> wasmtime::Result<slicer_ir::BoundingBox3> {
    ctx.runtime_reads.push(String::from("MeshIR"));
    // Missing mesh data and missing object both return OBJECT_NOT_FOUND.
    let mesh_ir = ctx
        .mesh_ir
        .as_ref()
        .ok_or_else(|| object_not_found_error("object-bounds", object_id))?;
    let object = mesh_ir
        .objects
        .iter()
        .find(|candidate| candidate.id == object_id)
        .ok_or_else(|| object_not_found_error("object-bounds", object_id))?;

    let mut vertices = object
        .mesh
        .vertices
        .iter()
        .map(|vertex| transform_mesh_point(&object.transform, vertex));
    let Some(first_vertex) = vertices.next() else {
        return Err(wasmtime::Error::msg(format!(
            "host-service object-bounds could not compute bounds for empty mesh '{object_id}'"
        )));
    };

    let mut min = first_vertex;
    let mut max = first_vertex;
    for vertex in vertices {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
    }

    Ok(slicer_ir::BoundingBox3 { min, max })
}

// ── Host trait implementations ──────────────────────────────────────────

use layer::slicer::common::host_services as hs;
use layer::slicer::config::config_types as ct;
use layer::slicer::ir_handles::ir_handles as ir;
use layer::slicer::types::geometry as geo;

// `module-errors` only contains a record (no functions/resources),
// so the generated Host trait is empty and requires a trivial impl.
// Now sourced from canonical slicer:common/module-errors package.
impl layer::slicer::common::module_errors::Host for HostExecutionContext {}

impl geo::Host for HostExecutionContext {}

fn ir_point3_to_layer(point: slicer_ir::Point3) -> Point3 {
    Point3 {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

fn ir_bounds_to_layer(bounds: slicer_ir::BoundingBox3) -> BoundingBox3 {
    BoundingBox3 {
        min: ir_point3_to_layer(bounds.min),
        max: ir_point3_to_layer(bounds.max),
    }
}

impl hs::Host for HostExecutionContext {
    fn log(&mut self, level: hs::LogLevel, message: String) -> wasmtime::Result<()> {
        let level_str = match level {
            hs::LogLevel::Trace => "trace",
            hs::LogLevel::Debug => "debug",
            hs::LogLevel::Info => "info",
            hs::LogLevel::Warn => "warn",
            hs::LogLevel::Error => "error",
        };
        self.log_messages.push((level_str.to_string(), message));
        Ok(())
    }

    fn raycast_z_down(
        &mut self,
        object_id: hs::ObjectId,
        x: f32,
        y: f32,
        start_z: f32,
    ) -> wasmtime::Result<Option<f32>> {
        raycast_z_down_mesh_query(self, &object_id, x, y, start_z)
    }

    fn surface_normal_at(
        &mut self,
        object_id: hs::ObjectId,
        x: f32,
        y: f32,
        z: f32,
    ) -> wasmtime::Result<Option<Point3>> {
        Ok(surface_normal_at_mesh_query(self, &object_id, x, y, z)?.map(ir_point3_to_layer))
    }

    fn object_bounds(&mut self, object_id: hs::ObjectId) -> wasmtime::Result<BoundingBox3> {
        Ok(ir_bounds_to_layer(object_bounds_mesh_query(
            self, &object_id,
        )?))
    }

    fn clip_polygons(
        &mut self,
        subject: Vec<ExPolygon>,
        clip: Vec<ExPolygon>,
        op: hs::ClipOperation,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        let ir_subject = wit_to_ir_expolygons(&subject);
        let ir_clip = wit_to_ir_expolygons(&clip);
        let ir_op = match op {
            hs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union,
            hs::ClipOperation::Intersection => {
                slicer_core::polygon_ops::ClipOperation::Intersection
            }
            hs::ClipOperation::Difference => slicer_core::polygon_ops::ClipOperation::Difference,
            hs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor,
        };
        let result = slicer_core::polygon_ops::clip_polygons(&ir_subject, &ir_clip, ir_op);
        Ok(ir_to_wit_expolygons(&result))
    }

    fn offset_polygons(
        &mut self,
        polygons: Vec<ExPolygon>,
        delta_mm: f32,
        join: hs::OffsetJoinType,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        let ir_polys = wit_to_ir_expolygons(&polygons);
        let ir_join = match join {
            hs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter,
            hs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round,
            hs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square,
        };
        let result = slicer_core::polygon_ops::offset(&ir_polys, delta_mm, ir_join, 0.0);
        Ok(ir_to_wit_expolygons(&result))
    }

    fn simplify_polygon(
        &mut self,
        polygon: Polygon,
        _tolerance_mm: f32,
    ) -> wasmtime::Result<Polygon> {
        // Collinearity-based simplification: remove points that are collinear
        // with their neighbors. The tolerance_mm parameter is reserved for
        // future Douglas-Peucker support; current impl uses exact collinearity.
        let mut points = polygon.points;
        if points.len() < 3 {
            return Ok(Polygon { points });
        }
        let mut changed = true;
        while changed {
            changed = false;
            let n = points.len();
            if n < 3 {
                break;
            }
            let mut keep = vec![true; n];
            for i in 0..n {
                let a = &points[i];
                let b = &points[(i + 1) % n];
                let c = &points[(i + 2) % n];
                let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
                if cross == 0 {
                    keep[(i + 1) % n] = false;
                    changed = true;
                }
            }
            points = points
                .into_iter()
                .enumerate()
                .filter(|(i, _)| keep[*i])
                .map(|(_, p)| p)
                .collect();
        }
        Ok(Polygon { points })
    }

    fn medial_axis(
        &mut self,
        input: ExPolygon,
        min_width: f32,
        max_width: f32,
    ) -> wasmtime::Result<Result<Vec<geo::ThickPolyline>, String>> {
        let ir_input = crate::marshal::leaf::wit_to_ir_expolygon(&input);
        match slicer_core::medial_axis::medial_axis(&ir_input, min_width, max_width) {
            Ok(polylines) => {
                let wit_polylines: Vec<geo::ThickPolyline> = polylines
                    .into_iter()
                    .map(|tp| geo::ThickPolyline {
                        points: tp
                            .points
                            .into_iter()
                            .map(|p| geo::Point2WithWidth {
                                x: p.x,
                                y: p.y,
                                width: p.width,
                            })
                            .collect(),
                    })
                    .collect();
                Ok(Ok(wit_polylines))
            }
            Err(e) => Ok(Err(format!("{:?}", e))),
        }
    }

    fn generate_arachne_walls(
        &mut self,
        polygons: Vec<ExPolygon>,
        params: hs::ArachneParams,
    ) -> wasmtime::Result<Result<(Vec<ir::ExtrusionLine>, Vec<ir::ExtrusionLine>), String>> {
        let ir_polygons = wit_to_ir_expolygons(&polygons);
        let core_params = slicer_core::arachne::pipeline::ArachneParams {
            optimal_width: params.optimal_width as f64,
            preferred_bead_width_outer: params.preferred_bead_width_outer as f64,
            max_bead_count: params.max_bead_count,
            distribution_count: params.distribution_count,
            transition_filter_dist: params.transition_filter_dist as f64,
            min_central_distance: params.min_central_distance as f64,
            visvalingam_area_threshold: params.visvalingam_area_threshold as f64,
            min_length_factor: params.min_length_factor as f64,
            min_width: params.min_width as f64,
            print_thin_walls: params.print_thin_walls,
            min_feature_size: params.min_feature_size as f64,
            min_bead_width: params.min_bead_width as f64,
            wall_transition_length: params.wall_transition_length as f64,
            wall_transition_angle: params.wall_transition_angle as f64,
            initial_layer_min_bead_width: params.initial_layer_min_bead_width as f64,
            outer_wall_offset: params.outer_wall_offset as f64,
            is_initial_layer: params.is_initial_layer,
            smallest_line_segment_squared: params.smallest_line_segment_squared as f64,
            allowed_error_distance_squared: params.allowed_error_distance_squared as f64,
            maximum_extrusion_area_deviation: params.maximum_extrusion_area_deviation as f64,
        };

        let to_wit_lines = |lines: Vec<slicer_ir::ExtrusionLine>| -> Vec<ir::ExtrusionLine> {
            lines
                .into_iter()
                .map(|line| ir::ExtrusionLine {
                    junctions: line
                        .junctions
                        .into_iter()
                        .map(|j| ir::ExtrusionJunction {
                            p: Point3WithWidth {
                                x: j.p.x,
                                y: j.p.y,
                                z: j.p.z,
                                width: j.p.width,
                                flow_factor: j.p.flow_factor,
                                overhang_quartile: j.p.overhang_quartile,
                            },
                            perimeter_index: j.perimeter_index,
                        })
                        .collect(),
                    inset_idx: line.inset_idx,
                    is_odd: line.is_odd,
                    is_closed: line.is_closed,
                })
                .collect()
        };

        match slicer_core::arachne::pipeline::run_arachne_pipeline(
            &ir_polygons,
            &core_params,
            core_params.is_initial_layer,
        ) {
            Ok((toolpaths, inner_contour)) => {
                Ok(Ok((to_wit_lines(toolpaths), to_wit_lines(inner_contour))))
            }
            Err(e) => Ok(Err(format!("{:?}", e))),
        }
    }

    fn now_us(&mut self) -> wasmtime::Result<u64> {
        // Monotonic timestamp from per-call Instant. Deterministic within a
        // call (always increasing), but not across calls (each call starts a
        // fresh Instant). This matches the doc requirement for profiling use.
        Ok(self.start_time.elapsed().as_micros() as u64)
    }
}

// ── WIT ↔ slicer-ir polygon conversion — moved to marshal/leaf.rs ─────
//
// Re-exported here so callers within this file and inner `mod` blocks that
// do `use super::*` continue to resolve them without a path change.
pub(crate) use crate::marshal::leaf::{
    ir_to_wit_expolygons, ir_to_wit_extrusion_path, ir_to_wit_extrusion_role,
    ir_to_wit_paint_layer_view, ir_to_wit_paint_semantic, ir_to_wit_paint_value,
    ir_to_wit_wall_loop, wit_to_ir_expolygons,
};
// Public re-exports to maintain the `host::X` path used by dispatch.rs and
// external callers.
pub use crate::marshal::leaf::{
    convert_extrusion_path, convert_extrusion_role, convert_layer_retract_mode,
    convert_wall_feature_flag, convert_wall_loop, convert_wall_loop_type,
};

/// Build an empty `PaintRegionLayerData` — paint annotations now live in
/// SliceIR segment_annotations (AC-16, packet 95 step 12/13).
/// This function is retained for call-site compatibility; Phase B removes it.
pub fn paint_region_ir_to_layer_data(_ir: &(), layer_index: u32) -> PaintRegionLayerData {
    PaintRegionLayerData {
        layer_index,
        regions_by_semantic: HashMap::new(),
        custom_regions: HashMap::new(),
        support_plan_segments: HashMap::new(),
    }
}

// ir_to_wit_paint_semantic, paint_semantic_to_string, ir_to_wit_paint_value_view,
// ir_to_wit_paint_stroke_view, ir_to_wit_paint_layer_view moved to marshal/leaf.rs (packet 113).
// Re-exported above via pub(crate) use crate::marshal::leaf::*.

// object_mesh_to_wit_mesh_object_view, project_layer_plan_view,
// project_region_segmentation_view, project_support_geometry_view moved to
// marshal/in_.rs (packet 113, Step 7 / ADR-0021).
// Re-exported here so callers that import via `host::` continue to resolve.
pub use crate::marshal::in_::{
    object_mesh_to_wit_mesh_object_view, project_layer_plan_view, project_region_segmentation_view,
    project_support_geometry_view,
};

// sliced_region_to_data moved to marshal/in_.rs (packet 113, Step 7 / ADR-0021).
pub use crate::marshal::in_::sliced_region_to_data;

// ir_to_wit_wall_loop_type, ir_to_wit_extrusion_role moved to marshal/leaf.rs (packet 113).
// Re-exported above via pub(crate) use crate::marshal::leaf::*.

#[cfg(test)]
mod layer_role_tests {
    use super::*;

    #[test]
    fn ir_to_wit_extrusion_role_preserves_reserved_builtin_roles() {
        assert!(matches!(
            ir_to_wit_extrusion_role(&slicer_ir::ExtrusionRole::PrimeTower),
            ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG
        ));
        assert!(matches!(
            ir_to_wit_extrusion_role(&slicer_ir::ExtrusionRole::Skirt),
            ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_SKIRT_TAG
        ));
        assert!(matches!(
            ir_to_wit_extrusion_role(&slicer_ir::ExtrusionRole::Brim),
            ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_BRIM_TAG
        ));
    }
}

#[cfg(test)]
mod region_origin_tests {
    use super::*;

    #[test]
    fn touch_slice_region_rejects_noncanonical_region_id_strings() {
        let mut ctx =
            HostExecutionContextBuilder::new("com.test.slice-origin".to_string(), 0.0, 0.2).build();
        let handle = ctx
            .push_slice_region(SliceRegionData {
                object_id: "obj-1".to_string(),
                region_id: "01".to_string(),
                polygons: Vec::new(),
                infill_areas: Vec::new(),
                effective_layer_height: 0.2,
                z: 0.2,
                has_nonplanar: false,
                segment_annotations: Vec::new(),
                variant_chain: Vec::new(),
                needs_support: true,
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: Vec::new(),
                bridge_orientation_deg: 0.0,
                sparse_infill_area: Vec::new(),
                held_claims: Vec::new(),
                overhang_areas: Vec::new(),
                overhang_quartile_polygons: Vec::new(),
                surface_group: None,
            })
            .expect("push slice region");

        let err = ctx
            .touch_slice_region(&handle)
            .expect_err("non-canonical region-id must be rejected");
        let message = err.to_string();

        assert!(
            message.contains("region-id") && message.contains("01"),
            "diagnostic must explain the rejected non-canonical region-id: {message}"
        );
    }

    #[test]
    fn touch_perimeter_region_rejects_noncanonical_region_id_strings() {
        let mut ctx =
            HostExecutionContextBuilder::new("com.test.perimeter-origin".to_string(), 0.0, 0.2)
                .build();
        let handle = ctx
            .push_perimeter_region(PerimeterRegionData {
                object_id: "obj-1".to_string(),
                region_id: "01".to_string(),
                wall_loops: Vec::new(),
                infill_areas: Vec::new(),
                resolved_seam: None,
                seam_candidates: Vec::new(),
            })
            .expect("push perimeter region");

        let err = ctx
            .touch_perimeter_region(&handle)
            .expect_err("non-canonical region-id must be rejected");
        let message = err.to_string();

        assert!(
            message.contains("region-id") && message.contains("01"),
            "diagnostic must explain the rejected non-canonical region-id: {message}"
        );
    }
}

// ir_to_wit_extrusion_path, ir_to_wit_wall_feature_flag, ir_to_wit_wall_loop moved to
// marshal/leaf.rs (packet 113). Re-exported above via pub(crate) use crate::marshal::leaf::*.

// perimeter_region_to_data moved to marshal/in_.rs (packet 113, Step 7 / ADR-0021).
pub use crate::marshal::in_::perimeter_region_to_data;

// ── Shared IR-level geometry helpers for all worlds ────────────────────

/// Clip polygons at the slicer-ir level using Clipper2.
pub fn ir_clip_polygons(
    subject: &[slicer_ir::ExPolygon],
    clip: &[slicer_ir::ExPolygon],
    op: slicer_core::polygon_ops::ClipOperation,
) -> Vec<slicer_ir::ExPolygon> {
    slicer_core::polygon_ops::clip_polygons(subject, clip, op)
}

/// Offset polygons at the slicer-ir level using Clipper2.
pub fn ir_offset_polygons(
    polys: &[slicer_ir::ExPolygon],
    delta_mm: f32,
    join: slicer_core::polygon_ops::OffsetJoinType,
) -> Vec<slicer_ir::ExPolygon> {
    slicer_core::polygon_ops::offset(polys, delta_mm, join, 0.0)
}

/// Simplify a polygon by removing collinear points.
pub fn ir_simplify_polygon(points: Vec<slicer_ir::Point2>) -> Vec<slicer_ir::Point2> {
    let mut pts = points;
    if pts.len() < 3 {
        return pts;
    }
    let mut changed = true;
    while changed {
        changed = false;
        let n = pts.len();
        if n < 3 {
            break;
        }
        let mut keep = vec![true; n];
        for i in 0..n {
            let a = &pts[i];
            let b = &pts[(i + 1) % n];
            let c = &pts[(i + 2) % n];
            let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if cross == 0 {
                keep[(i + 1) % n] = false;
                changed = true;
            }
        }
        pts = pts
            .into_iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, p)| p)
            .collect();
    }
    pts
}

/// Parse a guest-supplied region-id string into its canonical `u64`.
///
/// The canonical host form is a decimal `u64` with no leading zeros; any other
/// spelling is rejected as a fatal contract error. This is the single owner of
/// the rule — `dispatch.rs`'s harvest cores call it rather than re-implementing
/// it (packet 75, Phase 2 / TASK-217). Kept host-side (not in `slicer-ir`) so a
/// host-only boundary validator does not force a guest rebuild.
pub(crate) fn parse_canonical_region_id(raw: &str) -> Result<u64, String> {
    let parsed = raw.parse::<u64>().map_err(|_| {
        format!("expected canonical decimal u64 string with no leading zeros, got '{raw}'")
    })?;

    if parsed.to_string() != raw {
        return Err(format!(
            "expected canonical decimal u64 string with no leading zeros, got '{raw}'"
        ));
    }

    Ok(parsed)
}

impl ct::HostConfigView for HostExecutionContext {
    fn get(
        &mut self,
        self_: Resource<ConfigViewData>,
        key: String,
    ) -> wasmtime::Result<Option<ConfigValue>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).map(|v| match v {
            ConfigValueStorage::Bool(b) => ConfigValue::BoolVal(*b),
            ConfigValueStorage::Int(i) => ConfigValue::IntVal(*i),
            ConfigValueStorage::Float(f) => ConfigValue::FloatVal(*f),
            ConfigValueStorage::Str(s) => ConfigValue::StringVal(s.clone()),
            ConfigValueStorage::FloatList(fl) => ConfigValue::FloatList(fl.clone()),
            ConfigValueStorage::StringList(sl) => ConfigValue::StringList(sl.clone()),
            ConfigValueStorage::Percent(p) => ConfigValue::PercentVal(*p),
            ConfigValueStorage::FloatOrPercent { value, is_percent } => {
                ConfigValue::FloatOrPercentVal(FloatOrPercent {
                    value: *value,
                    is_percent: *is_percent,
                })
            }
        }))
    }

    fn get_bool(
        &mut self,
        self_: Resource<ConfigViewData>,
        key: String,
    ) -> wasmtime::Result<Option<bool>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Bool(b) => Some(*b),
            _ => None,
        }))
    }

    fn get_float(
        &mut self,
        self_: Resource<ConfigViewData>,
        key: String,
    ) -> wasmtime::Result<Option<f64>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Float(f) => Some(normalize_subnormal_boundary(*f)),
            // Mirrors `slicer_ir::ConfigView::get_float`: a literal
            // `FloatOrPercent` (is_percent == false) yields its value, but a
            // bare `Percent` or a percent-flagged `FloatOrPercent` must NOT
            // leak out as a plain float — callers need `get_abs_value`.
            ConfigValueStorage::FloatOrPercent {
                value,
                is_percent: false,
            } => Some(normalize_subnormal_boundary(*value)),
            _ => None,
        }))
    }

    fn get_int(
        &mut self,
        self_: Resource<ConfigViewData>,
        key: String,
    ) -> wasmtime::Result<Option<i64>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Int(i) => Some(*i),
            _ => None,
        }))
    }

    fn get_string(
        &mut self,
        self_: Resource<ConfigViewData>,
        key: String,
    ) -> wasmtime::Result<Option<String>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Str(s) => Some(s.clone()),
            _ => None,
        }))
    }

    fn keys(&mut self, self_: Resource<ConfigViewData>) -> wasmtime::Result<Vec<String>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.keys().cloned().collect())
    }

    fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ct::Host for HostExecutionContext {}

impl HostExecutionContext {
    /// Record the region identity backed by the given slice-region-view resource
    /// as the currently-active slice origin. Subsequent pushes to the support
    /// output builder are tagged with this identity for identity-preserving commit.
    fn touch_slice_region(&mut self, self_: &Resource<SliceRegionData>) -> wasmtime::Result<()> {
        let data = self.table.get(self_)?;
        let rid = parse_canonical_region_id(&data.region_id).map_err(|reason| {
            wasmtime::Error::msg(format!(
                "slice-region-view '{}'/'{}' has invalid region-id: {reason}",
                data.object_id, data.region_id
            ))
        })?;
        self.current_slice_region = Some(OriginId {
            object_id: data.object_id.clone(),
            region_id: rid,
        });
        Ok(())
    }
}

impl ir::HostSliceRegionView for HostExecutionContext {
    fn object_id(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<String> {
        self.touch_slice_region(&self_)?;
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.object_id.clone())
    }
    fn region_id(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<String> {
        self.touch_slice_region(&self_)?;
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.region_id.clone())
    }
    fn polygons(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<Vec<ExPolygon>> {
        self.touch_slice_region(&self_)?;
        self.runtime_reads
            .push(String::from("SliceIR.regions.polygons"));
        Ok(self.table.get(&self_)?.polygons.clone())
    }
    fn infill_areas(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        self.touch_slice_region(&self_)?;
        self.runtime_reads
            .push(String::from("SliceIR.regions.infill-areas"));
        Ok(self.table.get(&self_)?.infill_areas.clone())
    }
    fn effective_layer_height(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<f32> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.effective_layer_height)
    }
    fn z(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<f32> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.z)
    }
    fn has_nonplanar(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<bool> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.has_nonplanar)
    }
    fn segment_annotations(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<SegmentAnnotationsEntry>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.segment_annotations.clone())
    }
    fn variant_chain(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<(String, layer::slicer::ir_handles::ir_handles::PaintValue)>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.variant_chain.clone())
    }
    fn needs_support(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<bool> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.needs_support)
    }
    fn top_shell_index(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Option<u8>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.top_shell_index)
    }
    fn bottom_shell_index(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Option<u8>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.bottom_shell_index)
    }
    fn top_solid_fill(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<layer::slicer::types::geometry::ExPolygon>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.top_solid_fill.clone())
    }
    fn bottom_solid_fill(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<layer::slicer::types::geometry::ExPolygon>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.bottom_solid_fill.clone())
    }
    fn is_bridge(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<bool> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.is_bridge)
    }
    fn bridge_areas(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        self.runtime_reads
            .push(String::from("SliceIR.regions.bridge-areas"));
        Ok(self.table.get(&self_)?.bridge_areas.clone())
    }
    fn bridge_orientation_deg(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<f32> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.bridge_orientation_deg)
    }
    fn sparse_infill_area(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        self.runtime_reads
            .push(String::from("SliceIR.regions.sparse-infill-area"));
        Ok(self.table.get(&self_)?.sparse_infill_area.clone())
    }
    fn held_claims(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<Vec<String>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.held_claims.clone())
    }
    fn overhang_areas(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        self.runtime_reads
            .push(String::from("SliceIR.regions.overhang-areas"));
        Ok(self.table.get(&self_)?.overhang_areas.clone())
    }
    fn overhang_quartile_polygons(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<layer::slicer::ir_handles::ir_handles::QuartileBand>> {
        self.runtime_reads.push(String::from(
            "SurfaceClassificationIR.overhang-quartile-polygons",
        ));
        Ok(self.table.get(&self_)?.overhang_quartile_polygons.clone())
    }
    fn surface_group(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Option<layer::slicer::ir_handles::ir_handles::SurfaceGroup>> {
        self.runtime_reads
            .push(String::from("SurfaceClassificationIR.surface-group"));
        Ok(self.table.get(&self_)?.surface_group.clone())
    }
    fn drop(&mut self, rep: Resource<SliceRegionData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl HostExecutionContext {
    /// Record the region identity backed by the given perimeter-region-view
    /// resource as the currently-active one. Subsequent pushes to perimeter
    /// or infill output builders are tagged with this identity so the commit
    /// path can preserve per-region identity.
    fn touch_perimeter_region(
        &mut self,
        self_: &Resource<PerimeterRegionData>,
    ) -> wasmtime::Result<()> {
        let data = self.table.get(self_)?;
        let rid = parse_canonical_region_id(&data.region_id).map_err(|reason| {
            wasmtime::Error::msg(format!(
                "perimeter-region-view '{}'/'{}' has invalid region-id: {reason}",
                data.object_id, data.region_id
            ))
        })?;
        self.current_perimeter_region = Some(OriginId {
            object_id: data.object_id.clone(),
            region_id: rid,
        });
        Ok(())
    }
}

impl ir::HostPerimeterRegionView for HostExecutionContext {
    fn object_id(&mut self, self_: Resource<PerimeterRegionData>) -> wasmtime::Result<String> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads.push(String::from("PerimeterIR"));
        Ok(self.table.get(&self_)?.object_id.clone())
    }
    fn region_id(&mut self, self_: Resource<PerimeterRegionData>) -> wasmtime::Result<String> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads.push(String::from("PerimeterIR"));
        Ok(self.table.get(&self_)?.region_id.clone())
    }
    fn wall_loops(
        &mut self,
        self_: Resource<PerimeterRegionData>,
    ) -> wasmtime::Result<Vec<WallLoopView>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads
            .push(String::from("PerimeterIR.wall-loops"));
        Ok(self.table.get(&self_)?.wall_loops.clone())
    }
    fn infill_areas(
        &mut self,
        self_: Resource<PerimeterRegionData>,
    ) -> wasmtime::Result<Vec<ExPolygon>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads
            .push(String::from("PerimeterIR.infill-areas"));
        Ok(self.table.get(&self_)?.infill_areas.clone())
    }
    fn resolved_seam(
        &mut self,
        self_: Resource<PerimeterRegionData>,
    ) -> wasmtime::Result<Option<layer::slicer::ir_handles::ir_handles::SeamPosition>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads
            .push(String::from("PerimeterIR.resolved-seam"));
        let resolved = self.table.get(&self_)?.resolved_seam;
        match resolved {
            None => Ok(None),
            Some((pos, wall_index)) => {
                Ok(Some(layer::slicer::ir_handles::ir_handles::SeamPosition {
                    point: Point3WithWidth {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                        width: 0.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    wall_index,
                }))
            }
        }
    }
    fn seam_candidates(
        &mut self,
        self_: Resource<PerimeterRegionData>,
    ) -> wasmtime::Result<Vec<layer::slicer::ir_handles::ir_handles::SeamCandidate>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads
            .push(String::from("PerimeterIR.seam-candidates"));
        Ok(self
            .table
            .get(&self_)?
            .seam_candidates
            .iter()
            .map(
                |(pos, score)| layer::slicer::ir_handles::ir_handles::SeamCandidate {
                    position: Point3 {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    },
                    score: *score,
                },
            )
            .collect())
    }
    fn drop(&mut self, rep: Resource<PerimeterRegionData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostInfillOutputBuilder for HostExecutionContext {
    fn push_sparse_path(
        &mut self,
        _self_: Resource<InfillOutputBuilderData>,
        path: ExtrusionPath3d,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.effective_perimeter_origin();
        self.infill_output.sparse_paths.push(path);
        self.infill_output.sparse_path_origins.push(origin);
        self.record_write("InfillIR");
        Ok(Ok(()))
    }
    fn push_solid_path(
        &mut self,
        _self_: Resource<InfillOutputBuilderData>,
        path: ExtrusionPath3d,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.effective_perimeter_origin();
        self.infill_output.solid_paths.push(path);
        self.infill_output.solid_path_origins.push(origin);
        self.record_write("InfillIR");
        Ok(Ok(()))
    }
    fn push_ironing_path(
        &mut self,
        _self_: Resource<InfillOutputBuilderData>,
        path: ExtrusionPath3d,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.effective_perimeter_origin();
        self.infill_output.ironing_paths.push(path);
        self.infill_output.ironing_path_origins.push(origin);
        self.record_write("InfillIR");
        Ok(Ok(()))
    }
    fn set_current_origin(
        &mut self,
        _self_: Resource<InfillOutputBuilderData>,
        object_id: String,
        region_id: String,
    ) -> wasmtime::Result<Result<(), String>> {
        match region_id.parse::<u64>() {
            Ok(parsed) => {
                self.explicit_perimeter_origin = Some(OriginId {
                    object_id,
                    region_id: parsed,
                });
                Ok(Ok(()))
            }
            Err(_) => Ok(Err(format!("invalid region-id: {region_id}"))),
        }
    }
    fn drop(&mut self, rep: Resource<InfillOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostPerimeterOutputBuilder for HostExecutionContext {
    fn push_wall_loop(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        wall_loop: WallLoopView,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = wall_loop.path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.effective_perimeter_origin();
        self.perimeter_output.wall_loops.push(wall_loop);
        self.perimeter_output.wall_loop_origins.push(origin);
        self.record_write("PerimeterIR.regions.walls");
        Ok(Ok(()))
    }
    /// Set infill areas for this perimeter output builder, accumulating
    /// per-origin entries (one `set_infill_areas` call → one entry in
    /// `infill_areas` paired with one entry in `infill_areas_origins`).
    ///
    /// Pre-fix this method REPLACED a single `Vec<ExPolygon>`, which meant
    /// every perimeters guest that called `set_infill_areas` more than once
    /// per dispatch (the painted-slice / multi-region case) silently lost
    /// every region except the LAST in dispatch order — producing the
    /// "missing infill across internal painted regions" symptom on
    /// `resources/cube_4color.3mf`.
    ///
    /// No Z envelope check is needed here — `ExPolygon` carries no Z coordinate.
    /// Z validation for infill paths is performed in `push_sparse_path` and
    /// `push_solid_path` where the actual extrusion geometry is supplied.
    fn set_infill_areas(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        areas: Vec<ExPolygon>,
    ) -> wasmtime::Result<Result<(), String>> {
        let origin = self.effective_perimeter_origin();
        self.perimeter_output.infill_areas.push(areas);
        self.perimeter_output.infill_areas_origins.push(origin);
        Ok(Ok(()))
    }
    fn push_seam_candidate(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        pos: Point3,
        score: f32,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Err(e) = self.check_z_envelope(pos.z) {
            return Ok(Err(e));
        }
        let origin = self.effective_perimeter_origin();
        self.perimeter_output.seam_candidates.push((pos, score));
        self.perimeter_output.seam_candidate_origins.push(origin);
        Ok(Ok(()))
    }
    fn push_resolved_seam(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        pos: Point3,
        wall_index: u32,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Err(e) = self.check_z_envelope(pos.z) {
            return Ok(Err(e));
        }
        self.perimeter_output.resolved_seam = Some((pos, wall_index));
        self.perimeter_output.resolved_seam_origin = self.effective_perimeter_origin();
        self.record_write("PerimeterIR.resolved-seam");
        Ok(Ok(()))
    }
    fn push_reordered_wall_loop(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        pos: Point3WithWidth,
        _wall_index: u32,
        rotated_wall_loop: WallLoopView,
    ) -> wasmtime::Result<Result<(), String>> {
        // Z envelope check: pos.z must be within [layer_z, layer_z + effective_layer_height]
        if let Err(e) = self.check_z_envelope(pos.z) {
            return Ok(Err(e));
        }
        // Cardinality invariant: feature_flags.len() == rotated_wall_loop.path.points.len()
        if rotated_wall_loop.feature_flags.len() != rotated_wall_loop.path.points.len() {
            return Ok(Err(format!(
                "CARDINALITY_MISMATCH: feature_flags.len() {} != path.points.len() {}",
                rotated_wall_loop.feature_flags.len(),
                rotated_wall_loop.path.points.len()
            )));
        }
        let origin = self.effective_perimeter_origin();
        self.perimeter_output
            .rotated_wall_loops
            .push(rotated_wall_loop);
        self.perimeter_output.rotated_wall_loop_origins.push(origin);
        self.record_write("PerimeterIR.regions.walls");
        Ok(Ok(()))
    }
    fn set_current_origin(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        object_id: String,
        region_id: String,
    ) -> wasmtime::Result<Result<(), String>> {
        match region_id.parse::<u64>() {
            Ok(parsed) => {
                self.explicit_perimeter_origin = Some(OriginId {
                    object_id,
                    region_id: parsed,
                });
                Ok(Ok(()))
            }
            Err(_) => Ok(Err(format!("invalid region-id: {region_id}"))),
        }
    }
    fn drop(&mut self, rep: Resource<PerimeterOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostSlicePostprocessBuilder for HostExecutionContext {
    fn set_polygons(
        &mut self,
        _self_: Resource<SlicePostprocessBuilderData>,
        region: RegionKey,
        polys: Vec<ExPolygon>,
    ) -> wasmtime::Result<Result<(), String>> {
        self.slice_postprocess_output
            .polygon_updates
            .push((region, polys));
        Ok(Ok(()))
    }
    fn set_path_z(
        &mut self,
        _self_: Resource<SlicePostprocessBuilderData>,
        region: RegionKey,
        path_idx: u32,
        vertex_idx: u32,
        z: f32,
    ) -> wasmtime::Result<Result<(), String>> {
        self.slice_postprocess_output
            .path_z_updates
            .push((region, path_idx, vertex_idx, z));
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<SlicePostprocessBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostGcodeOutputBuilder for HostExecutionContext {
    fn push_move(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        cmd: GcodeMoveCmd,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::Move(cmd));
        Ok(Ok(()))
    }
    fn push_retract(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        length: f32,
        speed: f32,
        mode: WitRetractMode,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::Retract {
                length,
                speed,
                mode: convert_layer_retract_mode(&mode),
            });
        Ok(Ok(()))
    }
    fn push_unretract(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        length: f32,
        speed: f32,
        mode: WitRetractMode,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::Unretract {
                length,
                speed,
                mode: convert_layer_retract_mode(&mode),
            });
        Ok(Ok(()))
    }
    fn push_fan_speed(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        value: u8,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::FanSpeed(value));
        Ok(Ok(()))
    }
    fn push_temperature(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        tool: u32,
        celsius: f32,
        wait: bool,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::Temperature {
                tool,
                celsius,
                wait,
            });
        Ok(Ok(()))
    }
    fn push_tool_change(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        after_entity_index: u32,
        from_tool: u32,
        to_tool: u32,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::ToolChange {
                after_entity_index,
                from_tool,
                to_tool,
            });
        Ok(Ok(()))
    }
    fn push_comment(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        text: String,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::Comment(text));
        Ok(Ok(()))
    }
    fn push_raw(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        text: String,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::Raw(text));
        Ok(Ok(()))
    }
    fn push_z_hop(
        &mut self,
        _self_: Resource<GcodeOutputBuilderData>,
        after_entity_index: u32,
        hop_height: f32,
    ) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output
            .commands
            .push(GcodeCommandCollected::ZHop {
                after_entity_index,
                hop_height,
            });
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<GcodeOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostLayerCollectionBuilder for HostExecutionContext {
    fn set_entity_order(
        &mut self,
        _self_: Resource<LayerCollectionBuilderData>,
        items: Vec<(u32, bool)>,
    ) -> wasmtime::Result<Result<(), String>> {
        if self.layer_collection_proposal.is_some() {
            return Ok(Err(
                "set-entity-order called twice within one run-path-optimization".into(),
            ));
        }
        self.layer_collection_proposal = Some(items);
        Ok(Ok(()))
    }
    fn get_ordered_entities(
        &mut self,
        self_: Resource<LayerCollectionBuilderData>,
    ) -> wasmtime::Result<Vec<ir::OrderedEntityView>> {
        self.host_get_ordered_entities_call_count =
            self.host_get_ordered_entities_call_count.saturating_add(1);
        HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let data = self.table.get(&self_)?;
        let views: Vec<ir::OrderedEntityView> = data
            .ordered_entities
            .iter()
            .map(|v| ir::OrderedEntityView {
                original_index: v.original_index,
                tool_index: v.tool_index,
                region_key: ir::RegionKey {
                    layer_index: v.region_key.global_layer_index as i32,
                    object_id: v.region_key.object_id.clone(),
                    region_id: v.region_key.region_id.to_string(),
                },
                role: ir_to_wit_extrusion_role(&v.role),
                start_point: Point3WithWidth {
                    x: v.start_point.x,
                    y: v.start_point.y,
                    z: v.start_point.z,
                    width: v.start_point.width,
                    flow_factor: v.start_point.flow_factor,
                    overhang_quartile: v.start_point.overhang_quartile,
                },
                end_point: Point3WithWidth {
                    x: v.end_point.x,
                    y: v.end_point.y,
                    z: v.end_point.z,
                    width: v.end_point.width,
                    flow_factor: v.end_point.flow_factor,
                    overhang_quartile: v.end_point.overhang_quartile,
                },
                point_count: v.point_count,
            })
            .collect();
        self.runtime_reads
            .push(String::from("LayerCollectionIR.ordered_entities"));
        Ok(views)
    }
    fn drop(&mut self, rep: Resource<LayerCollectionBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostSupportOutputBuilder for HostExecutionContext {
    fn push_support_path(
        &mut self,
        _self_: Resource<SupportOutputBuilderData>,
        path: ExtrusionPath3d,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_slice_region.clone();
        self.support_output.support_paths.push(path);
        self.support_output.support_path_origins.push(origin);
        self.record_write("SupportIR");
        Ok(Ok(()))
    }
    fn push_interface_path(
        &mut self,
        _self_: Resource<SupportOutputBuilderData>,
        path: ExtrusionPath3d,
        is_top_interface: bool,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_slice_region.clone();
        self.support_output
            .interface_paths
            .push((path, is_top_interface));
        self.support_output.interface_path_origins.push(origin);
        self.record_write("SupportIR");
        Ok(Ok(()))
    }
    fn push_raft_path(
        &mut self,
        _self_: Resource<SupportOutputBuilderData>,
        path: ExtrusionPath3d,
    ) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_slice_region.clone();
        self.support_output.raft_paths.push(path);
        self.support_output.raft_path_origins.push(origin);
        self.record_write("SupportIR");
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<SupportOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostPaintRegionLayerView for HostExecutionContext {
    fn get_regions(
        &mut self,
        self_: Resource<PaintRegionLayerData>,
        semantic: PaintSemantic,
    ) -> wasmtime::Result<Vec<SemanticRegion>> {
        self.runtime_reads.push(String::from("PaintRegionIR"));
        let data = self.table.get(&self_)?;
        let key = match semantic {
            PaintSemantic::Material => "material",
            PaintSemantic::FuzzySkin => "fuzzy-skin",
            PaintSemantic::SupportEnforcer => "support-enforcer",
            PaintSemantic::SupportBlocker => "support-blocker",
            PaintSemantic::Custom(ref s) => {
                // Leak the string so the &str is valid for the HashMap lookup.
                // The HashMap lookup is immediate; no lingering reference afterward.
                Box::leak(s.clone().into_boxed_str())
            }
        };
        Ok(data
            .regions_by_semantic
            .get(key)
            .cloned()
            .unwrap_or_default())
    }
    fn get_custom_regions(
        &mut self,
        self_: Resource<PaintRegionLayerData>,
        module_id: String,
    ) -> wasmtime::Result<Vec<SemanticRegion>> {
        self.runtime_reads.push(String::from("PaintRegionIR"));
        Ok(self
            .table
            .get(&self_)?
            .custom_regions
            .get(&module_id)
            .cloned()
            .unwrap_or_default())
    }
    fn layer_index(&mut self, self_: Resource<PaintRegionLayerData>) -> wasmtime::Result<i32> {
        self.runtime_reads.push(String::from("PaintRegionIR"));
        Ok(self.table.get(&self_)?.layer_index as i32)
    }
    fn support_plan_segments(
        &mut self,
        self_: Resource<PaintRegionLayerData>,
        object_id: String,
        region_id: String,
    ) -> wasmtime::Result<Vec<Vec<layer::slicer::types::geometry::Point3WithWidth>>> {
        self.runtime_reads.push(String::from("SupportPlanIR"));
        let data = self.table.get(&self_)?;
        Ok(data
            .support_plan_segments
            .get(&(object_id, region_id))
            .cloned()
            .unwrap_or_default())
    }
    fn drop(&mut self, rep: Resource<PaintRegionLayerData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::Host for HostExecutionContext {}

// The non-layer world host-services impls are generated inline in each
// world's mod block, calling through to the shared conversion+Clipper2
// infrastructure defined below the layer world's impls. The macro approach
// was abandoned due to Rust's qualified-path limitations in macro expansion.

// ── Prepass world host trait impls ─────────────────────────────────────

mod prepass_impls {
    use super::*;

    // Geometry, host-services, and module-errors types are all remapped onto
    // the layer world's types via `with:` in the bindgen! block (packet 114).
    // IR↔WIT converters (`wit_to_ir_expolygon`, `ir_to_wit_expolygon`,
    // `ir_point3_to_layer`, `ir_bounds_to_layer`) are reused via `use super::*`.

    // `pct` config-types `Host`/`HostConfigView` impls are the layer
    // world's (Phase 3 remap); reused via `use super::*`, not regenerated.

    // Prepass world resources
    use super::prepass as pm;

    impl pm::HostMeshAnalysisOutput for HostExecutionContext {
        fn push_facet_annotation(
            &mut self,
            _handle: Resource<pm::MeshAnalysisOutput>,
            object_id: String,
            annotation: pm::FacetAnnotation,
        ) -> wasmtime::Result<Result<(), String>> {
            if object_id.is_empty() {
                return Ok(Err(String::from(
                    "mesh-analysis-output: object-id must be non-empty",
                )));
            }
            if !annotation.slope_angle_deg.is_finite() {
                return Ok(Err(format!(
                    "mesh-analysis-output: object '{}' facet {} has non-finite slope_angle_deg={}",
                    object_id, annotation.facet_index, annotation.slope_angle_deg
                )));
            }
            self.mesh_analysis_annotations.push((object_id, annotation));
            Ok(Ok(()))
        }
        fn push_surface_group(
            &mut self,
            _handle: Resource<pm::MeshAnalysisOutput>,
            object_id: String,
            group: pm::SurfaceGroupProposal,
        ) -> wasmtime::Result<Result<(), String>> {
            if object_id.is_empty() {
                return Ok(Err(String::from(
                    "mesh-analysis-output: object-id must be non-empty",
                )));
            }
            if !group.z_min.is_finite() || !group.z_max.is_finite() {
                return Ok(Err(format!(
                    "mesh-analysis-output: object '{}' surface group has non-finite z bounds (z_min={}, z_max={})",
                    object_id, group.z_min, group.z_max
                )));
            }
            if group.z_max < group.z_min {
                return Ok(Err(format!(
                    "mesh-analysis-output: object '{}' surface group has z_max={} < z_min={}",
                    object_id, group.z_max, group.z_min
                )));
            }
            self.mesh_analysis_surface_groups.push((object_id, group));
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::MeshAnalysisOutput>) -> wasmtime::Result<()> {
            let typed: Resource<MeshAnalysisOutputData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

    impl pm::HostLayerPlanOutput for HostExecutionContext {
        fn push_layer(
            &mut self,
            _handle: Resource<pm::LayerPlanOutput>,
            proposal: pm::LayerProposal,
        ) -> wasmtime::Result<Result<(), String>> {
            // Validate the proposal before collecting it.
            if !proposal.z.is_finite() || proposal.z < 0.0 {
                return Ok(Err(format!(
                    "layer-plan-output: invalid z={} (must be finite and non-negative)",
                    proposal.z
                )));
            }
            for r in &proposal.active_regions {
                if !r.effective_layer_height.is_finite() || r.effective_layer_height <= 0.0 {
                    return Ok(Err(format!(
                        "layer-plan-output: region '{}'/'{}'  has invalid effective_layer_height={} \
                         (must be finite and positive)",
                        r.object_id, r.region_id, r.effective_layer_height
                    )));
                }
            }
            self.layer_plan_proposals.push(proposal);
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::LayerPlanOutput>) -> wasmtime::Result<()> {
            let typed: Resource<LayerPlanOutputData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

    impl pm::HostSeamPlanningOutput for HostExecutionContext {
        fn push_seam_plan(
            &mut self,
            _handle: Resource<pm::SeamPlanningOutput>,
            entry: pm::SeamPlanEntry,
        ) -> wasmtime::Result<Result<(), String>> {
            // Validate before collecting. Empty object-id would corrupt
            // the RegionKey construction in the harvest helper.
            if entry.object_id.is_empty() {
                return Ok(Err(String::from(
                    "seam-planning-output: object-id must be non-empty",
                )));
            }
            if entry.region_id.is_empty() {
                return Ok(Err(String::from(
                    "seam-planning-output: region-id must be non-empty",
                )));
            }
            if !entry.chosen_position.x.is_finite()
                || !entry.chosen_position.y.is_finite()
                || !entry.chosen_position.z.is_finite()
            {
                return Ok(Err(String::from(
                    "seam-planning-output: chosen_position must have finite coordinates",
                )));
            }
            self.seam_plan_entries.push(entry);
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::SeamPlanningOutput>) -> wasmtime::Result<()> {
            let typed: Resource<SeamPlanningOutputData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

    impl pm::HostSupportGeometryOutput for HostExecutionContext {
        fn push_support_plan_entry(
            &mut self,
            _handle: Resource<pm::SupportGeometryOutput>,
            entry: pm::SupportPlanEntry,
        ) -> wasmtime::Result<Result<(), String>> {
            // Validate before collecting: an empty object-id/region-id would
            // corrupt the RegionKey construction in harvest_support_plan_ir.
            // Matches the sibling seam-planning-output / mesh-analysis-output
            // validators (the packet's "mirror seam-planning" instruction).
            if entry.object_id.is_empty() {
                return Ok(Err(String::from(
                    "support-geometry-output: object-id must be non-empty",
                )));
            }
            if entry.region_id.is_empty() {
                return Ok(Err(String::from(
                    "support-geometry-output: region-id must be non-empty",
                )));
            }
            self.support_plan_entries.push(entry);
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::SupportGeometryOutput>) -> wasmtime::Result<()> {
            let typed: Resource<SupportGeometryOutputData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

    impl pm::PrepassModuleImports for HostExecutionContext {}
}

// ── Finalization world host trait impls ────────────────────────────────

mod finalization_impls {
    use super::finalization as fm;
    use super::*;
    use finalization::slicer::types::geometry as fgeo;

    // `fgeo` now aliases the layer world's geometry module (Phase 3 remap); its
    // `Host` impl and IR↔WIT geometry converters are the layer world's — reused
    // here via `use super::*` instead of regenerated copies.
    // `host-services` and `module-errors` are also remapped onto the layer
    // world's types via `with:` in the bindgen! block (packet 114).

    // `fct` config-types `Host`/`HostConfigView` impls are the layer
    // world's (Phase 3 remap); reused via `use super::*`, not regenerated.

    /// Convert a wit-bindgen finalization-world `ExtrusionPath3d` record
    /// into the documented `slicer_ir::ExtrusionPath3D`.
    fn finalization_path_wit_to_ir(p: &fgeo::ExtrusionPath3d) -> slicer_ir::ExtrusionPath3D {
        slicer_ir::ExtrusionPath3D {
            points: p
                .points
                .iter()
                .map(|pt| slicer_ir::Point3WithWidth {
                    x: pt.x,
                    y: pt.y,
                    z: pt.z,
                    width: pt.width,
                    flow_factor: pt.flow_factor,
                    overhang_quartile: pt.overhang_quartile,
                })
                .collect(),
            role: crate::marshal::leaf::convert_extrusion_role(&p.role),
            speed_factor: p.speed_factor,
        }
    }

    // finalization_role_wit_to_ir removed (packet 115); call site now uses convert_extrusion_role.

    impl fm::HostLayerCollectionView for HostExecutionContext {
        fn layer_index(
            &mut self,
            self_: Resource<fm::LayerCollectionView>,
        ) -> wasmtime::Result<u32> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data.layer_index)
        }
        fn z(&mut self, self_: Resource<fm::LayerCollectionView>) -> wasmtime::Result<f32> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data.z)
        }
        fn entity_count(
            &mut self,
            self_: Resource<fm::LayerCollectionView>,
        ) -> wasmtime::Result<u32> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data.entity_count)
        }
        fn tool_changes(
            &mut self,
            self_: Resource<fm::LayerCollectionView>,
        ) -> wasmtime::Result<Vec<fm::ToolChangeView>> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data
                .tool_changes
                .iter()
                .map(
                    |(after_entity_index, from_tool, to_tool)| fm::ToolChangeView {
                        after_entity_index: *after_entity_index,
                        from_tool: *from_tool,
                        to_tool: *to_tool,
                    },
                )
                .collect())
        }
        fn ordered_entities(
            &mut self,
            self_: Resource<fm::LayerCollectionView>,
        ) -> wasmtime::Result<Vec<fm::PrintEntityView>> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data
                .ordered_entities
                .iter()
                .map(|entity| fm::PrintEntityView {
                    entity_id: entity.entity_id,
                    path: ir_to_wit_extrusion_path(&entity.path),
                    role: ir_to_wit_extrusion_role(&entity.role),
                    tool_index: entity.tool_index,
                    region_key: fm::RegionKey {
                        layer_index: entity.region_key.global_layer_index,
                        object_id: entity.region_key.object_id.clone(),
                        region_id: entity.region_key.region_id.to_string(),
                    },
                    topo_order: entity.topo_order,
                })
                .collect())
        }
        fn z_hops(
            &mut self,
            self_: Resource<fm::LayerCollectionView>,
        ) -> wasmtime::Result<Vec<fm::ZHopView>> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data
                .z_hops
                .iter()
                .map(|z_hop| fm::ZHopView {
                    after_entity_index: z_hop.after_entity_index,
                    hop_height: z_hop.hop_height,
                })
                .collect())
        }
        fn drop(&mut self, rep: Resource<fm::LayerCollectionView>) -> wasmtime::Result<()> {
            let typed: Resource<LayerCollectionViewData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

    impl fm::HostFinalizationOutputBuilder for HostExecutionContext {
        fn push_entity_to_layer(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            path: fgeo::ExtrusionPath3d,
            tool_index: u32,
            region_key: fm::RegionKey,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            let region_id = match super::parse_canonical_region_id(&region_key.region_id) {
                Ok(region_id) => region_id,
                Err(reason) => {
                    return Ok(Err(format!(
                        "finalization-output-builder: region '{}'/'{}' has invalid region-id: {reason}",
                        region_key.object_id, region_key.region_id
                    )));
                }
            };
            let ir_region_key = slicer_ir::RegionKey {
                global_layer_index: region_key.layer_index,
                object_id: region_key.object_id,
                region_id,
                variant_chain: Vec::new(),
            };
            data.pushes.push(FinalizationBuilderPush::EntityToLayer {
                layer_index,
                path: finalization_path_wit_to_ir(&path),
                tool_index,
                region_key: ir_region_key,
            });
            Ok(Ok(()))
        }
        fn push_entity_with_priority(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            path: fgeo::ExtrusionPath3d,
            tool_index: u32,
            region_key: fm::RegionKey,
            priority: u32,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            let region_id = match super::parse_canonical_region_id(&region_key.region_id) {
                Ok(region_id) => region_id,
                Err(reason) => {
                    return Ok(Err(format!(
                        "finalization-output-builder: region '{}'/'{}' has invalid region-id: {reason}",
                        region_key.object_id, region_key.region_id
                    )));
                }
            };
            let ir_region_key = slicer_ir::RegionKey {
                global_layer_index: region_key.layer_index,
                object_id: region_key.object_id,
                region_id,
                variant_chain: Vec::new(),
            };
            data.pushes
                .push(FinalizationBuilderPush::EntityToLayerWithPriority {
                    layer_index,
                    path: finalization_path_wit_to_ir(&path),
                    tool_index,
                    region_key: ir_region_key,
                    priority,
                });
            Ok(Ok(()))
        }
        fn modify_entity(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            entity_id: u64,
            mutation: fm::EntityMutation,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            let wit_mutation = match mutation {
                fm::EntityMutation::SetSpeedFactor(v) => WitEntityMutation::SetSpeedFactor(v),
                fm::EntityMutation::SetFlowFactor(v) => WitEntityMutation::SetFlowFactor(v),
            };
            data.pushes.push(FinalizationBuilderPush::ModifyEntity {
                layer_index,
                entity_id,
                mutation: wit_mutation,
            });
            Ok(Ok(()))
        }
        fn sort_layer_by(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            key: fm::SortKey,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            let wit_key = match key {
                fm::SortKey::ByPriorityAndEntityId => WitSortKey::ByPriorityAndEntityId,
                fm::SortKey::ByEntityId => WitSortKey::ByEntityId,
                fm::SortKey::ByObjectIdThenPriority => WitSortKey::ByObjectIdThenPriority,
            };
            data.pushes.push(FinalizationBuilderPush::SortLayerBy {
                layer_index,
                key: wit_key,
            });
            Ok(Ok(()))
        }
        fn insert_synthetic_layer_after(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            idx: u32,
            layer_data: fm::SyntheticLayerData,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            data.pushes
                .push(FinalizationBuilderPush::InsertSyntheticLayerAfter {
                    idx,
                    z: layer_data.z,
                    paths: layer_data
                        .paths
                        .iter()
                        .map(finalization_path_wit_to_ir)
                        .collect(),
                });
            Ok(Ok(()))
        }
        fn insert_synthetic_layer(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            z: f32,
            paths: Vec<fgeo::ExtrusionPath3d>,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            data.pushes.push(FinalizationBuilderPush::SyntheticLayer {
                z,
                paths: paths.iter().map(finalization_path_wit_to_ir).collect(),
            });
            Ok(Ok(()))
        }
        fn insert_entity_at(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            position: u32,
            path: fgeo::ExtrusionPath3d,
            tool_index: u32,
            region_key: fm::RegionKey,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            let region_id = match super::parse_canonical_region_id(&region_key.region_id) {
                Ok(region_id) => region_id,
                Err(reason) => {
                    return Ok(Err(format!(
                        "finalization-output-builder: region '{}'/'{}' has invalid region-id: {reason}",
                        region_key.object_id, region_key.region_id
                    )));
                }
            };
            let ir_region_key = slicer_ir::RegionKey {
                global_layer_index: region_key.layer_index,
                object_id: region_key.object_id,
                region_id,
                variant_chain: Vec::new(),
            };
            data.pushes.push(FinalizationBuilderPush::InsertEntityAt {
                layer_index,
                position,
                path: finalization_path_wit_to_ir(&path),
                tool_index,
                region_key: ir_region_key,
            });
            Ok(Ok(()))
        }

        fn set_entity_order(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            items: Vec<(u32, bool)>,
        ) -> wasmtime::Result<Result<(), String>> {
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get_mut(&typed)?;
            if !data.permuted_layers.insert(layer_index) {
                return Ok(Err(format!(
                    "set-entity-order called twice for layer {layer_index} within one run-finalization"
                )));
            }
            data.pushes
                .push(FinalizationBuilderPush::SetEntityOrder { layer_index, items });
            Ok(Ok(()))
        }

        fn get_ordered_entities(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
        ) -> wasmtime::Result<Vec<fm::PrintEntityView>> {
            // Returns the staged state of `layer_index`'s ordered_entities,
            // simulating every recorded push / insert / permutation against the
            // pre-apply snapshot captured by `push_finalization_output_builder`.
            // Packet-58 locked invariant: this read-back reflects both
            // pre-existing entities and in-flight builder actions. Priority
            // sorting is intentionally NOT simulated (that is `apply_to`'s job);
            // pushes are appended in record order.
            self.runtime_reads.push(String::from("LayerCollectionIR"));

            let mut staged: Vec<slicer_ir::PrintEntity> = self
                .finalization_layer_snapshot
                .iter()
                .find(|l| l.global_layer_index == layer_index)
                .map(|l| l.ordered_entities.clone())
                .unwrap_or_default();
            let mut next_id: u64 = staged.iter().map(|e| e.entity_id).max().unwrap_or(0) + 1;

            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;

            for op in &data.pushes {
                match op {
                    FinalizationBuilderPush::EntityToLayer {
                        layer_index: li,
                        path,
                        tool_index,
                        region_key,
                    }
                    | FinalizationBuilderPush::EntityToLayerWithPriority {
                        layer_index: li,
                        path,
                        tool_index,
                        region_key,
                        ..
                    } if *li == layer_index => {
                        staged.push(slicer_ir::PrintEntity {
                            entity_id: next_id,
                            path: path.clone(),
                            role: path.role.clone(),
                            tool_index: *tool_index,
                            region_key: region_key.clone(),
                            topo_order: 0,
                        });
                        next_id += 1;
                    }
                    FinalizationBuilderPush::InsertEntityAt {
                        layer_index: li,
                        position,
                        path,
                        tool_index,
                        region_key,
                    } if *li == layer_index => {
                        let pos = (*position as usize).min(staged.len());
                        staged.insert(
                            pos,
                            slicer_ir::PrintEntity {
                                entity_id: next_id,
                                path: path.clone(),
                                role: path.role.clone(),
                                tool_index: *tool_index,
                                region_key: region_key.clone(),
                                topo_order: 0,
                            },
                        );
                        next_id += 1;
                    }
                    FinalizationBuilderPush::SetEntityOrder {
                        layer_index: li,
                        items,
                    } if *li == layer_index => {
                        if items.len() != staged.len() {
                            continue;
                        }
                        let mut seen = vec![false; staged.len()];
                        let mut valid = true;
                        for &(idx, _) in items {
                            let i = idx as usize;
                            if i >= staged.len() || seen[i] {
                                valid = false;
                                break;
                            }
                            seen[i] = true;
                        }
                        if !valid {
                            continue;
                        }
                        let original = staged.clone();
                        staged = items
                            .iter()
                            .map(|&(old_idx, _)| original[old_idx as usize].clone())
                            .collect();
                    }
                    _ => {}
                }
            }

            let result: Vec<fm::PrintEntityView> = staged
                .iter()
                .map(|e| {
                    let path_wit = ir_to_wit_extrusion_path(&e.path);
                    let role_wit = ir_to_wit_extrusion_role(&e.role);
                    fm::PrintEntityView {
                        entity_id: e.entity_id,
                        path: path_wit,
                        role: role_wit,
                        tool_index: e.tool_index,
                        region_key: fm::RegionKey {
                            layer_index: e.region_key.global_layer_index,
                            object_id: e.region_key.object_id.clone(),
                            region_id: e.region_key.region_id.to_string(),
                        },
                        topo_order: e.topo_order,
                    }
                })
                .collect();
            Ok(result)
        }

        fn drop(&mut self, rep: Resource<fm::FinalizationOutputBuilder>) -> wasmtime::Result<()> {
            // Move captured pushes onto the HostExecutionContext before
            // the resource's storage is reclaimed, so the dispatch path
            // can drain them even after the guest drops its handle.
            let typed: Resource<FinalizationOutputBuilderData> = Resource::new_own(rep.rep());
            let mut data = self.table.delete(typed)?;
            self.finalization_pushes.append(&mut data.pushes);
            Ok(())
        }
    }

    impl fm::FinalizationModuleImports for HostExecutionContext {}

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ir_to_wit_extrusion_role_preserves_reserved_builtin_roles() {
            assert!(matches!(
                ir_to_wit_extrusion_role(&slicer_ir::ExtrusionRole::PrimeTower),
                ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG
            ));
            assert!(matches!(
                ir_to_wit_extrusion_role(&slicer_ir::ExtrusionRole::Skirt),
                ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_SKIRT_TAG
            ));
            assert!(matches!(
                ir_to_wit_extrusion_role(&slicer_ir::ExtrusionRole::Brim),
                ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_BRIM_TAG
            ));
        }

        #[test]
        fn finalization_output_builder_rejects_noncanonical_region_id_strings() {
            let mut ctx =
                HostExecutionContextBuilder::new("com.test.finalization".to_string(), 0.0, 0.2)
                    .build();
            let handle = ctx
                .push_finalization_output_builder()
                .expect("push finalization output builder");

            let result =
                <HostExecutionContext as fm::HostFinalizationOutputBuilder>::push_entity_to_layer(
                    &mut ctx,
                    handle,
                    0,
                    fgeo::ExtrusionPath3d {
                        points: Vec::new(),
                        role: fgeo::ExtrusionRole::OuterWall,
                        speed_factor: 1.0,
                    },
                    0,
                    fm::RegionKey {
                        layer_index: 0,
                        object_id: "obj-1".to_string(),
                        region_id: "01".to_string(),
                    },
                )
                .expect("host call must succeed");

            let message = result.expect_err("non-canonical region-id must be rejected");
            assert!(
                message.contains("region-id") && message.contains("01"),
                "diagnostic must explain the rejected non-canonical region-id: {message}"
            );
        }
    }
}

// ── Postpass world host trait impls ───────────────────────────────────

mod postpass_impls {
    use super::postpass as ppm;
    use super::*;

    // Geometry, host-services, and module-errors types are all remapped onto
    // the layer world's types via `with:` in the bindgen! block (packet 114).
    // IR↔WIT converters are reused via `use super::*`.

    // `ppct` config-types `Host`/`HostConfigView` impls are the layer
    // world's (Phase 3 remap); reused via `use super::*`, not regenerated.

    impl ppm::HostGcodeOutputBuilder for HostExecutionContext {
        fn push_move(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            cmd: ppm::GcodeMoveCmd,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::Move(GcodeMoveCmd {
                    x: cmd.x,
                    y: cmd.y,
                    z: cmd.z,
                    e: cmd.e,
                    f: cmd.f,
                    role: cmd.role,
                }));
            Ok(Ok(()))
        }
        fn push_retract(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            length: f32,
            speed: f32,
            mode: ppm::RetractMode,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::Retract {
                    length,
                    speed,
                    mode: crate::marshal::leaf::convert_postpass_retract_mode(&mode),
                });
            Ok(Ok(()))
        }
        fn push_unretract(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            length: f32,
            speed: f32,
            mode: ppm::RetractMode,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::Unretract {
                    length,
                    speed,
                    mode: crate::marshal::leaf::convert_postpass_retract_mode(&mode),
                });
            Ok(Ok(()))
        }
        fn push_fan_speed(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            value: u8,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::FanSpeed(value));
            Ok(Ok(()))
        }
        fn push_temperature(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            tool: u32,
            celsius: f32,
            wait: bool,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::Temperature {
                    tool,
                    celsius,
                    wait,
                });
            Ok(Ok(()))
        }
        fn push_tool_change(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            after_entity_index: u32,
            from_tool: u32,
            to_tool: u32,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::ToolChange {
                    after_entity_index,
                    from_tool,
                    to_tool,
                });
            Ok(Ok(()))
        }
        fn push_comment(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            text: String,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::Comment(text));
            Ok(Ok(()))
        }
        fn push_raw(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            text: String,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::Raw(text));
            Ok(Ok(()))
        }
        fn push_z_hop(
            &mut self,
            _: Resource<ppm::GcodeOutputBuilder>,
            after_entity_index: u32,
            hop_height: f32,
        ) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output
                .commands
                .push(GcodeCommandCollected::ZHop {
                    after_entity_index,
                    hop_height,
                });
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<ppm::GcodeOutputBuilder>) -> wasmtime::Result<()> {
            let typed: Resource<PostpassGcodeOutputBuilderData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

    impl ppm::PostpassModuleImports for HostExecutionContext {}

    // convert_postpass_retract_mode moved to marshal/leaf.rs (packet 113, AC-1b).
    // convert_postpass_role removed (packet 115); call site now uses convert_extrusion_role.
}

// ── WIT→IR type conversion and validation — moved to marshal/leaf.rs ──
//
// validate_finite, convert_point, convert_extrusion_role, convert_layer_retract_mode,
// convert_extrusion_path, convert_wall_loop_type, convert_paint_value,
// convert_wall_feature_flag, convert_wall_loop (packet 113, ADR-0021).
// Re-exported above via pub / pub(crate) use crate::marshal::leaf::*.

// convert_infill_output moved to crate::marshal::out (packet 113, ADR-0021).
// Re-exported above via `pub use crate::marshal::out::convert_infill_output`.

// convert_support_output moved to crate::marshal::out (packet 113, ADR-0021).
// Re-exported above via `pub use crate::marshal::out::convert_support_output`.

// convert_perimeter_output moved to crate::marshal::out (packet 113, ADR-0021).
// Re-exported above via `pub use crate::marshal::out::convert_perimeter_output`.

// merge_slice_postprocess_into moved to crate::marshal::out (packet 113, ADR-0021).
// Re-exported above via `pub use crate::marshal::out::merge_slice_postprocess_into`.

#[cfg(test)]
mod tests {
    use super::*;

    /// Slice-region fallback: with only `current_slice_region` set (the
    /// `Layer::Perimeters` shape), the helper must surface that origin so
    /// origin-tagged pushes reach `convert_perimeter_output` with a non-empty
    /// `object_id`.
    #[test]
    fn effective_perimeter_origin_falls_back_to_slice_when_only_slice_set() {
        let mut ctx =
            HostExecutionContextBuilder::new("com.test.effective-perimeter-origin", 0.0, 0.2)
                .build();
        ctx.set_current_slice_region(Some(OriginId {
            object_id: "uuid".to_string(),
            region_id: 7,
        }));

        assert_eq!(
            ctx.effective_perimeter_origin(),
            Some(OriginId {
                object_id: "uuid".to_string(),
                region_id: 7
            })
        );
    }

    /// At `Layer::PerimetersPostProcess` both origins are set; the perimeter
    /// origin is canonical and must win over the slice-region fallback.
    #[test]
    fn effective_perimeter_origin_prefers_perimeter_when_both_set() {
        let mut ctx =
            HostExecutionContextBuilder::new("com.test.effective-perimeter-origin", 0.0, 0.2)
                .build();
        ctx.set_current_slice_region(Some(OriginId {
            object_id: "slice-uuid".to_string(),
            region_id: 1,
        }));
        ctx.set_current_perimeter_region(Some(OriginId {
            object_id: "perimeter-uuid".to_string(),
            region_id: 2,
        }));

        assert_eq!(
            ctx.effective_perimeter_origin(),
            Some(OriginId {
                object_id: "perimeter-uuid".to_string(),
                region_id: 2
            })
        );
    }

    /// Outside any touch site neither origin is set; the helper must report
    /// `None` so the untagged path in `convert_perimeter_output` stays reachable.
    #[test]
    fn effective_perimeter_origin_is_none_when_neither_set() {
        let ctx = HostExecutionContextBuilder::new("com.test.effective-perimeter-origin", 0.0, 0.2)
            .build();

        assert_eq!(ctx.effective_perimeter_origin(), None);
    }

    /// Regression lock (packet 150): `ConfigValue::Percent` must survive the
    /// host→guest config-delivery boundary as a genuine percent, not be
    /// silently downgraded to a `Str("25%")` / `StringVal("25%")`. That
    /// downgrade defeated module-side `get_abs_value`, which needs to
    /// distinguish "this is a percent of some base" from "this is a plain
    /// string". Locks both hops: `slicer_ir::ConfigValue` →
    /// `ConfigValueStorage` (`config_value_to_storage`), and
    /// `ConfigValueStorage` → the WIT `config-value` the guest actually reads
    /// (`HostConfigView::get`, what `config-view.get` dispatches to).
    #[test]
    fn percent_config_value_round_trips_through_storage_and_wit_get() {
        use ct::HostConfigView as _;

        // Hop 1: slicer_ir::ConfigValue -> ConfigValueStorage.
        let storage = config_value_to_storage(&slicer_ir::ConfigValue::Percent(25.0));
        match storage {
            ConfigValueStorage::Percent(p) => {
                assert!((p - 25.0).abs() < f64::EPSILON, "expected 25.0, got {p}");
            }
            other => panic!(
                "expected ConfigValueStorage::Percent(25.0), got {other:?} \
                 (percent downgraded to Str? regression reintroduced)"
            ),
        }

        let fop_storage = config_value_to_storage(&slicer_ir::ConfigValue::FloatOrPercent {
            value: 42.0,
            is_percent: true,
        });
        match fop_storage {
            ConfigValueStorage::FloatOrPercent { value, is_percent } => {
                assert!(
                    (value - 42.0).abs() < f64::EPSILON,
                    "expected 42.0, got {value}"
                );
                assert!(is_percent, "expected is_percent: true");
            }
            other => panic!(
                "expected ConfigValueStorage::FloatOrPercent{{ value: 42.0, is_percent: true }}, \
                 got {other:?} (percent downgraded to Str? regression reintroduced)"
            ),
        }

        // Hop 2: ConfigValueStorage -> WIT config-value, via the exact getter
        // the guest's `config-view.get` dispatches to.
        let mut ctx =
            HostExecutionContextBuilder::new("com.test.percent-roundtrip", 0.0, 0.2).build();
        let mut fields = HashMap::new();
        fields.insert(
            "infill_density".to_string(),
            ConfigValueStorage::Percent(25.0),
        );
        let resource = ctx
            .table
            .push(ConfigViewData { fields })
            .expect("push config-view resource");

        let value = ctx
            .get(resource, "infill_density".to_string())
            .expect("get call succeeds")
            .expect("value present for known key");
        match value {
            ConfigValue::PercentVal(p) => {
                assert!((p - 25.0).abs() < f64::EPSILON, "expected 25.0, got {p}");
            }
            ConfigValue::StringVal(s) => panic!(
                "regression reintroduced: percent downgraded to StringVal({s:?}) \
                 instead of PercentVal(25.0)"
            ),
            _ => panic!("expected ConfigValue::PercentVal(25.0)"),
        }
    }
}
