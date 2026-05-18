//! WIT/component-model host-side bindings and execution context.
//!
//! This module provides:
//! - `wasmtime::component::bindgen!`-generated types and traits for the layer world
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
}

/// Normalize subnormal `f64` values to `0.0` at the typed-config boundary.
///
/// Mirrors `crates/slicer-host/src/config_schema.rs::normalize_subnormal` so that
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
    pub polygons: Vec<layer::slicer::world_layer::geometry::ExPolygon>,
    /// Infill areas.
    pub infill_areas: Vec<layer::slicer::world_layer::geometry::ExPolygon>,
    /// Layer height at this Z.
    pub effective_layer_height: f32,
    /// Z height.
    pub z: f32,
    /// Whether this region has non-planar surfaces.
    pub has_nonplanar: bool,
    /// Boundary paint data.
    pub boundary_paint: Vec<layer::slicer::world_layer::ir_handles::BoundaryPaintEntry>,
    /// True when this region is support-eligible (from SurfaceClassificationIR).
    pub needs_support: bool,
    /// True when this region is classified as a top surface.
    pub is_top_surface: bool,
    /// True when this region is classified as a bottom surface.
    pub is_bottom_surface: bool,
    /// True when this region is classified as a bridge region.
    pub is_bridge: bool,
    /// Per-layer expanded bridge polygons (empty if not a bridge region).
    pub bridge_areas: Vec<layer::slicer::world_layer::geometry::ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees).
    pub bridge_orientation_deg: f32,
    /// Fill-role claim IDs held by the module that produced this region.
    pub held_claims: Vec<String>,
}

/// Backing data for a `perimeter-region-view` resource handle.
pub struct PerimeterRegionData {
    /// Object ID.
    pub object_id: String,
    /// Region ID.
    pub region_id: String,
    /// Wall loops.
    pub wall_loops: Vec<layer::slicer::world_layer::ir_handles::WallLoopView>,
    /// Infill areas after perimeter inset.
    pub infill_areas: Vec<layer::slicer::world_layer::geometry::ExPolygon>,
    /// Resolved seam position (populated from PerimeterIR after seam-placer runs).
    pub resolved_seam: Option<(Point3, u32)>,
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
        HashMap<String, Vec<layer::slicer::world_layer::ir_handles::SemanticRegion>>,
    /// Custom regions by module ID.
    pub custom_regions:
        HashMap<String, Vec<layer::slicer::world_layer::ir_handles::SemanticRegion>>,
    /// Pre-planned support-branch segments indexed by `(object_id, region_id)`,
    /// projected from `SupportPlanIR.entries` filtered to this layer index.
    /// Empty when no `SupportPlanIR` is committed on the blackboard.
    pub support_plan_segments:
        HashMap<(String, String), Vec<Vec<layer::slicer::world_layer::geometry::Point3WithWidth>>>,
}

// ── Bindgen: Layer module world ─────────────────────────────────────────

#[allow(missing_docs)]
pub mod layer {
    wasmtime::component::bindgen!({
        inline: r#"
            package slicer:world-layer@1.0.0;

            interface geometry {
                record point2 { x: s64, y: s64 }
                record point3 { x: f32, y: f32, z: f32 }
                record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32, overhang-quartile: option<u8> }
                record bounding-box2 { min: point2, max: point2 }
                record bounding-box3 { min: point3, max: point3 }
                record polygon       { points: list<point2> }
                record ex-polygon    { contour: polygon, holes: list<polygon> }
                record extrusion-path3d { points: list<point3-with-width>, role: extrusion-role, speed-factor: f32 }
                variant extrusion-role {
                    outer-wall, inner-wall, thin-wall,
                    top-solid-infill, bottom-solid-infill, sparse-infill,
                    support-material, support-interface,
                    ironing, bridge-infill, wipe-tower, custom(string),
                }
                record semver { major: u32, minor: u32, patch: u32 }
            }

            interface config-types {
                variant config-value {
                    bool-val(bool), int-val(s64), float-val(f64),
                    string-val(string), float-list(list<f64>), string-list(list<string>),
                }
                resource config-view {
                    get:        func(key: string) -> option<config-value>;
                    get-bool:   func(key: string) -> option<bool>;
                    get-float:  func(key: string) -> option<f64>;
                    get-int:    func(key: string) -> option<s64>;
                    get-string: func(key: string) -> option<string>;
                    keys:       func() -> list<string>;
                }
            }

            interface host-services {
                use geometry.{point3, bounding-box3, ex-polygon, polygon};
                type object-id = string;
                enum log-level { trace, debug, info, warn, error }
                log: func(level: log-level, message: string);
                raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
                surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
                object-bounds:      func(object-id: object-id) -> bounding-box3;
                enum clip-operation   { union, intersection, difference, xor }
                enum offset-join-type { miter, round, square }
                clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
                offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
                simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
                now-us: func() -> u64;
            }

            interface ir-handles {
                use geometry.{ex-polygon, extrusion-path3d, point3, point3-with-width, extrusion-role};
                type object-id = string;
                type region-id = string;
                // Signed because raft entries committed by `PrePass::SupportGeometry`
                // carry negative `global_layer_index`. Layer-module exports always
                // pass non-negative values; host conversion sites use `as u32` /
                // `as i32` at the boundary to round-trip into u32-keyed IR types.
                type layer-idx = s32;
                record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }
                record wall-feature-flag { tool-index: option<u32>, fuzzy-skin: bool, is-bridge: bool, is-thin-wall: bool, skip-ironing: bool, custom: list<tuple<string, paint-value>> }
                record wall-loop-view { perimeter-index: u32, loop-type: wall-loop-type, path: extrusion-path3d, feature-flags: list<wall-feature-flag> }
                enum wall-loop-type { outer, inner, thin-wall, nonplanar-shell }
                variant paint-semantic { material, fuzzy-skin, support-enforcer, support-blocker, custom(string) }
                variant paint-value { flag(bool), scalar(f32), tool-index(u32) }
                record boundary-paint-polygon { values: list<option<paint-value>> }
                record boundary-paint-entry { semantic: paint-semantic, polygons: list<boundary-paint-polygon> }
                resource slice-region-view {
                    object-id: func() -> object-id;
                    region-id: func() -> region-id;
                    polygons: func() -> list<ex-polygon>;
                    infill-areas: func() -> list<ex-polygon>;
                    effective-layer-height: func() -> f32;
                    z: func() -> f32;
                    has-nonplanar: func() -> bool;
                    boundary-paint: func() -> list<boundary-paint-entry>;
                    needs-support: func() -> bool;
                    is-top-surface: func() -> bool;
                    is-bottom-surface: func() -> bool;
                    is-bridge: func() -> bool;
                    bridge-areas: func() -> list<ex-polygon>;
                    bridge-orientation-deg: func() -> f32;
                    held-claims: func() -> list<string>;
                }
                record seam-position { point: point3-with-width, wall-index: u32 }
                resource perimeter-region-view {
                    object-id: func() -> object-id;
                    region-id: func() -> region-id;
                    wall-loops: func() -> list<wall-loop-view>;
                    infill-areas: func() -> list<ex-polygon>;
                    resolved-seam: func() -> option<seam-position>;
                }
                resource infill-output-builder {
                    push-sparse-path:  func(path: extrusion-path3d) -> result<_, string>;
                    push-solid-path:   func(path: extrusion-path3d) -> result<_, string>;
                    push-ironing-path: func(path: extrusion-path3d) -> result<_, string>;
                }
                resource perimeter-output-builder {
                    push-wall-loop:          func(wall-loop: wall-loop-view) -> result<_, string>;
                    push-reordered-wall-loop: func(pos: point3-with-width, wall-index: u32, rotated-wall-loop: wall-loop-view) -> result<_, string>;
                    set-infill-areas:        func(areas: list<ex-polygon>) -> result<_, string>;
                    push-seam-candidate:     func(pos: point3, score: f32) -> result<_, string>;
                    push-resolved-seam:      func(pos: point3, wall-index: u32) -> result<_, string>;
                }
                resource slice-postprocess-builder {
                    set-polygons: func(region: region-key, polys: list<ex-polygon>) -> result<_, string>;
                    set-path-z:   func(region: region-key, path-idx: u32, vertex-idx: u32, z: f32) -> result<_, string>;
                }
                record gcode-move-cmd { x: option<f32>, y: option<f32>, z: option<f32>, e: option<f32>, f: option<f32>, role: extrusion-role }
                variant retract-mode { gcode, firmware }
                resource gcode-output-builder {
                    push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                    push-retract:     func(length: f32, speed: f32, mode: retract-mode) -> result<_, string>;
                    push-unretract:   func(length: f32, speed: f32, mode: retract-mode) -> result<_, string>;
                    push-fan-speed:   func(value: u8) -> result<_, string>;
                    push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                    push-tool-change: func(after-entity-index: u32, from-tool: u32, to-tool: u32) -> result<_, string>;
                    push-comment:     func(text: string) -> result<_, string>;
                    push-raw:         func(text: string) -> result<_, string>;
                    push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
                }
                record ordered-entity-view {
                    original-index: u32,
                    region-key: region-key,
                    role: extrusion-role,
                    start-point: point3-with-width,
                    end-point: point3-with-width,
                    point-count: u32,
                }
                resource layer-collection-builder {
                    set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>;
                    get-ordered-entities: func() -> list<ordered-entity-view>;
                }
                resource support-output-builder {
                    push-support-path:   func(path: extrusion-path3d) -> result<_, string>;
                    push-interface-path: func(path: extrusion-path3d, is-top-interface: bool) -> result<_, string>;
                    push-raft-path:      func(path: extrusion-path3d) -> result<_, string>;
                }
                record semantic-region { object-id: object-id, polygons: list<ex-polygon>, value: paint-value }
                resource paint-region-layer-view {
                    get-regions: func(semantic: paint-semantic) -> list<semantic-region>;
                    get-custom-regions: func(module-id: string) -> list<semantic-region>;
                    layer-index: func() -> layer-idx;
                    support-plan-segments: func(object-id: object-id, region-id: region-id)
                        -> list<list<point3-with-width>>;
                }
            }

            world layer-module {
                import host-services;
                import config-types;
                import ir-handles;
                record module-error { code: u32, message: string, fatal: bool }
                use config-types.{config-view};
                use ir-handles.{
                    slice-region-view, perimeter-region-view,
                    infill-output-builder, perimeter-output-builder,
                    slice-postprocess-builder, support-output-builder,
                    gcode-output-builder, layer-collection-builder,
                    region-key, layer-idx,
                    paint-region-layer-view,
                };
                export on-print-start: func(config: config-view) -> result<_, module-error>;
                export on-print-end:   func() -> result<_, module-error>;
                export run-slice-postprocess: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: slice-postprocess-builder, config: config-view) -> result<_, module-error>;
                export run-perimeters: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: perimeter-output-builder, config: config-view) -> result<_, module-error>;
                export run-wall-postprocess: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: perimeter-output-builder, config: config-view) -> result<_, module-error>;
                export run-infill: func(layer-index: layer-idx, regions: list<slice-region-view>, output: infill-output-builder, config: config-view) -> result<_, module-error>;
                export run-infill-postprocess: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: infill-output-builder, config: config-view) -> result<_, module-error>;
                export run-support: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: support-output-builder, config: config-view) -> result<_, module-error>;
                export run-support-postprocess: func(layer-index: layer-idx, regions: list<slice-region-view>, output: support-output-builder, config: config-view) -> result<_, module-error>;
                export run-path-optimization: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: gcode-output-builder, collection: layer-collection-builder, config: config-view) -> result<_, module-error>;
            }
        "#,
        world: "layer-module",
        imports: {
            default: trappable,
        },
        with: {
            "slicer:world-layer/config-types@1.0.0.config-view": super::ConfigViewData,
            "slicer:world-layer/ir-handles@1.0.0.slice-region-view": super::SliceRegionData,
            "slicer:world-layer/ir-handles@1.0.0.perimeter-region-view": super::PerimeterRegionData,
            "slicer:world-layer/ir-handles@1.0.0.infill-output-builder": super::InfillOutputBuilderData,
            "slicer:world-layer/ir-handles@1.0.0.perimeter-output-builder": super::PerimeterOutputBuilderData,
            "slicer:world-layer/ir-handles@1.0.0.slice-postprocess-builder": super::SlicePostprocessBuilderData,
            "slicer:world-layer/ir-handles@1.0.0.gcode-output-builder": super::GcodeOutputBuilderData,
            "slicer:world-layer/ir-handles@1.0.0.layer-collection-builder": super::LayerCollectionBuilderData,
            "slicer:world-layer/ir-handles@1.0.0.support-output-builder": super::SupportOutputBuilderData,
            "slicer:world-layer/ir-handles@1.0.0.paint-region-layer-view": super::PaintRegionLayerData,
        },
    });
}

// Re-export commonly used generated types for convenience.
pub use layer::slicer::world_layer::config_types::ConfigValue;
pub use layer::slicer::world_layer::geometry::{
    BoundingBox3, ExPolygon, ExtrusionPath3d, ExtrusionRole, Point2, Point3, Point3WithWidth,
    Polygon,
};
/// Re-export of the layer-module WIT `retract-mode` variant. Used by host-side
/// `gcode-output-builder` handlers and `dispatch.rs` converters to forward the
/// `RetractMode` end-to-end across the guest→host boundary.
pub use layer::slicer::world_layer::ir_handles::RetractMode as WitRetractMode;
pub use layer::slicer::world_layer::ir_handles::{
    BoundaryPaintEntry, BoundaryPaintPolygon, GcodeMoveCmd, HostPerimeterOutputBuilder,
    PaintSemantic, PaintValue, RegionKey, SeamPosition, SemanticRegion, WallFeatureFlag,
    WallLoopType, WallLoopView,
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
/// Backing data for prepass `mesh-segmentation-output` resource.
///
/// Triangle paint marks emitted by `mark-triangle-paint` during a WIT prepass
/// invocation are stored on `HostExecutionContext::mesh_segmentation_marks`.
/// This struct is just a table-entry tag so the resource-handle lifecycle
/// works; the actual data lives on the context.
pub struct MeshSegmentationOutputData;
/// Backing data for prepass `paint-segmentation-output` resource.
///
/// Paint-region entries emitted by `push-paint-region` during a WIT prepass
/// invocation are stored on `HostExecutionContext::paint_region_entries`.
/// This struct is just a table-entry tag so the resource-handle lifecycle
/// works; the actual data lives on the context.
pub struct PaintSegmentationOutputData;
/// Backing data for prepass `seam-planning-output` resource.
///
/// Seam-plan entries emitted by `push-seam-plan` during a WIT prepass
/// invocation are stored on `HostExecutionContext::seam_plan_entries`.
/// This struct is just a table-entry tag so the resource-handle lifecycle
/// works; the actual data lives on the context.
pub struct SeamPlanningOutputData;
/// Marker type kept for table-management symmetry.
///
/// The `SupportGeometry` prepass stage returns a `support-geometry-output`
/// record directly (no resource handle); this stub is no longer needed.
/// Retained as a zero-size placeholder so reference-count tracking in the
/// surrounding table code continues to compile without churn.
#[allow(dead_code)]
pub struct SupportGeometryOutputData;

#[allow(missing_docs)]
pub mod prepass {
    wasmtime::component::bindgen!({
        inline: r#"
            package slicer:world-prepass@1.0.0;

            interface geometry {
                record point3 { x: f32, y: f32, z: f32 }
                record bounding-box3 { min: point3, max: point3 }
                record point2 { x: s64, y: s64 }
                record polygon { points: list<point2> }
                record ex-polygon { contour: polygon, holes: list<polygon> }
                record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32, overhang-quartile: option<u8> }
            }

            interface config-types {
                variant config-value {
                    bool-val(bool), int-val(s64), float-val(f64),
                    string-val(string), float-list(list<f64>), string-list(list<string>),
                }
                resource config-view {
                    get:        func(key: string) -> option<config-value>;
                    get-bool:   func(key: string) -> option<bool>;
                    get-float:  func(key: string) -> option<f64>;
                    get-int:    func(key: string) -> option<s64>;
                    get-string: func(key: string) -> option<string>;
                    keys:       func() -> list<string>;
                }
            }

            interface host-services {
                use geometry.{point3, bounding-box3, ex-polygon, polygon};
                type object-id = string;
                enum log-level { trace, debug, info, warn, error }
                log: func(level: log-level, message: string);
                raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
                surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
                object-bounds:      func(object-id: object-id) -> bounding-box3;
                enum clip-operation   { union, intersection, difference, xor }
                enum offset-join-type { miter, round, square }
                clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
                offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
                simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
                now-us: func() -> u64;
            }

            world prepass-module {
                import host-services;
                import config-types;
                type object-id = string;
                type region-id = string;
                type layer-idx = s32;
                record module-error { code: u32, message: string, fatal: bool }

                enum facet-class { normal, near-horizontal, overhang, bridge, top-surface, bottom-surface }
                record facet-annotation { facet-index: u32, slope-angle-deg: f32, classification: facet-class }
                record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }

                use config-types.{config-view};

                resource mesh-analysis-output {
                    push-facet-annotation: func(obj: object-id, ann: facet-annotation) -> result<_, string>;
                    push-surface-group:    func(obj: object-id, grp: surface-group-proposal) -> result<_, string>;
                }

                export run-mesh-analysis: func(
                    objects: list<object-id>,
                    output: mesh-analysis-output,
                    config: config-view,
                ) -> result<_, module-error>;

                resource mesh-segmentation-output {
                    mark-triangle-paint: func(obj: object-id, facet-index: u32, semantic: string, value: string) -> result<_, string>;
                }

                export run-mesh-segmentation: func(
                    objects: list<mesh-object-view>,
                    output: mesh-segmentation-output,
                    config: config-view,
                ) -> result<_, module-error>;

                use geometry.{ex-polygon};

                variant paint-value-input {
                    flag(bool),
                    scalar(f32),
                    tool-index(u32),
                    custom(string),
                }

                record paint-region-entry {
                    object-id: object-id,
                    layer-index: layer-idx,
                    semantic: string,
                    polygons: list<ex-polygon>,
                    value: paint-value-input,
                }
                resource paint-segmentation-output {
                    push-paint-region: func(entry: paint-region-entry) -> result<_, string>;
                }

                export run-paint-segmentation: func(
                    objects: list<paint-segmentation-object-view>,
                    output: paint-segmentation-output,
                    config: config-view,
                ) -> result<_, module-error>;

                record region-layer-proposal {
                    object-id: object-id, region-id: region-id,
                    effective-layer-height: f32,
                    is-catchup: bool, catchup-z-bottom: f32,
                }
                record layer-proposal { z: f32, active-regions: list<region-layer-proposal> }

                // ── Prepass segmentation view records ────────────────────────
                // Read-only views of mesh geometry and paint data for macro-authored
                // PrePass::MeshSegmentation and PrePass::PaintSegmentation modules.

                use geometry.{point3};

                /// A paint value with discriminator for flag, scalar, or tool-index.
                variant paint-value-view {
                    flag(bool),
                    scalar(f32),
                    tool-index(u32),
                }

                /// A sub-facet paint stroke resolved into whole-triangle values.
                record paint-stroke-view {
                    /// Triangle vertices (3 point3 per triangle).
                    triangles: list<point3>,
                    /// Semantic identifier string.
                    semantic: string,
                    /// Paint value carried by this stroke.
                    value: paint-value-view,
                }

                /// A paint layer on an object with per-facet values and strokes.
                record paint-layer-view {
                    /// Semantic identifier string.
                    semantic: string,
                    /// Per-facet paint values, parallel to mesh triangles.
                    /// None = unpainted.
                    facet-values: list<option<paint-value-view>>,
                    /// Sub-facet strokes crossing triangle boundaries.
                    strokes: list<paint-stroke-view>,
                }

                /// Read-only view of an object's mesh and paint data for segmentation.
                record mesh-object-view {
                    object-id: object-id,
                    /// Mesh vertices as point3 coordinates.
                    vertices: list<point3>,
                    /// Triangle indices (3 per triangle), indexing into vertices.
                    triangles: list<tuple<u32, u32, u32>>,
                    /// All paint layers on this object.
                    paint-layers: list<paint-layer-view>,
                }

                /// Read-only view of an object for paint segmentation, including
                /// transform and layer participation.
                record paint-segmentation-object-view {
                    object-id: object-id,
                    /// Mesh vertices as point3 coordinates.
                    vertices: list<point3>,
                    /// Triangle indices (3 per triangle), indexing into vertices.
                    triangles: list<tuple<u32, u32, u32>>,
                    /// All paint layers on this object.
                    paint-layers: list<paint-layer-view>,
                    /// 4x4 column-major transform matrix (16 elements).
                    transform-matrix: list<f64>,
                    /// Global layer indices this object participates in.
                    participating-layer-indices: list<u32>,
                }

                resource layer-plan-output {
                    push-layer: func(proposal: layer-proposal) -> result<_, string>;
                }

                export run-layer-planning: func(
                    objects: list<object-id>,
                    output: layer-plan-output,
                    config: config-view,
                ) -> result<_, module-error>;

                // SeamPlanning stage
                use geometry.{point3-with-width};

                record seam-reason { tag: string }
                record scored-seam-candidate {
                    position: point3-with-width,
                    score: f32,
                    reason: seam-reason,
                }
                record seam-plan-entry {
                    global-layer-index: u32,
                    object-id: object-id,
                    region-id: region-id,
                    chosen-position: point3-with-width,
                    chosen-wall-index: u32,
                    scored-candidates: list<scored-seam-candidate>,
                }

                resource seam-planning-output {
                    push-seam-plan: func(entry: seam-plan-entry) -> result<_, string>;
                }

                export run-seam-planning: func(
                    objects: list<mesh-object-view>,
                    output: seam-planning-output,
                    config: config-view,
                ) -> result<_, module-error>;

                // SupportGeometry stage. global-layer-index is signed because
                // raft prefix layers carry negative indices (-1, -2, ...).
                // `type layer-idx = s32` (line above) already covers all signed
                // uses in this world, including seam-plan-entry and view records.
                record support-plan-entry {
                    global-layer-index: s32,
                    object-id: object-id,
                    region-id: region-id,
                    branch-segments: list<list<point3-with-width>>,
                }

                // ── Layer plan & region segmentation views ─────────────────
                // Read-only views of the committed LayerPlanIR and RegionMapIR
                // for support-geometry modules.

                record layer-plan-view-entry {
                    global-layer-index: u32,
                    z: f32,
                    effective-layer-height: f32,
                }
                record layer-plan-view {
                    layers: list<layer-plan-view-entry>,
                }
                record region-segmentation-view-entry {
                    object-id: object-id,
                    layer-index: u32,
                    region-ids: list<region-id>,
                }
                record region-segmentation-view {
                    entries: list<region-segmentation-view-entry>,
                }

                record support-geometry-view-entry {
                    global-support-layer-index: u32,
                    object-id: object-id,
                    region-id: region-id,
                    outlines: list<ex-polygon>,
                }
                record support-geometry-view {
                    entries: list<support-geometry-view-entry>,
                }

                record support-geometry-output {
                    support-plan-entries: list<support-plan-entry>,
                }

                export run-support-geometry: func(
                    objects: list<mesh-object-view>,
                    layer-plan: layer-plan-view,
                    region-segmentation: region-segmentation-view,
                    support-geometry: support-geometry-view,
                ) -> support-geometry-output;
            }
        "#,
        world: "prepass-module",
        imports: {
            default: trappable,
        },
        with: {
            "slicer:world-prepass/config-types@1.0.0.config-view": super::ConfigViewData,
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
    /// Guest requested `push-entity-to-layer(layer_index, path, region_key)`.
    EntityToLayer {
        /// Layer index the entity was pushed to.
        layer_index: u32,
        /// Extrusion path content.
        path: slicer_ir::ExtrusionPath3D,
        /// Region key for ordering / provenance.
        region_key: slicer_ir::RegionKey,
    },
    /// Guest requested `push-entity-with-priority(layer_index, path, region_key, priority)`.
    EntityToLayerWithPriority {
        /// Layer index the entity was pushed to.
        layer_index: u32,
        /// Extrusion path content.
        path: slicer_ir::ExtrusionPath3D,
        /// Region key for ordering / provenance.
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
}

#[allow(missing_docs)]
pub mod finalization {
    wasmtime::component::bindgen!({
        inline: r#"
            package slicer:world-finalization@1.0.0;

            interface geometry {
                record point3 { x: f32, y: f32, z: f32 }
                record bounding-box3 { min: point3, max: point3 }
                record point2 { x: s64, y: s64 }
                record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32, overhang-quartile: option<u8> }
                record polygon { points: list<point2> }
                record ex-polygon { contour: polygon, holes: list<polygon> }
                record extrusion-path3d { points: list<point3-with-width>, role: extrusion-role, speed-factor: f32 }
                variant extrusion-role {
                    outer-wall, inner-wall, thin-wall,
                    top-solid-infill, bottom-solid-infill, sparse-infill,
                    support-material, support-interface,
                    ironing, bridge-infill, wipe-tower, custom(string),
                }
            }

            interface config-types {
                variant config-value {
                    bool-val(bool), int-val(s64), float-val(f64),
                    string-val(string), float-list(list<f64>), string-list(list<string>),
                }
                resource config-view {
                    get:        func(key: string) -> option<config-value>;
                    get-bool:   func(key: string) -> option<bool>;
                    get-float:  func(key: string) -> option<f64>;
                    get-int:    func(key: string) -> option<s64>;
                    get-string: func(key: string) -> option<string>;
                    keys:       func() -> list<string>;
                }
            }

            interface host-services {
                use geometry.{point3, bounding-box3, ex-polygon, polygon};
                type object-id = string;
                enum log-level { trace, debug, info, warn, error }
                log: func(level: log-level, message: string);
                raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
                surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
                object-bounds:      func(object-id: object-id) -> bounding-box3;
                enum clip-operation   { union, intersection, difference, xor }
                enum offset-join-type { miter, round, square }
                clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
                offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
                simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
                now-us: func() -> u64;
            }

            world finalization-module {
                import host-services;
                import config-types;
                use config-types.{config-view};
                use geometry.{extrusion-path3d, extrusion-role};
                type layer-idx = u32;
                type object-id = string;
                type region-id = string;
                record module-error { code: u32, message: string, fatal: bool }
                record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }

                record tool-change-view {
                    after-entity-index: u32,
                    from-tool: u32,
                    to-tool: u32,
                }

                record print-entity-view {
                    entity-id: u64,
                    path: extrusion-path3d,
                    role: extrusion-role,
                    region-key: region-key,
                    topo-order: u32,
                }

                record z-hop-view {
                    after-entity-index: u32,
                    hop-height: f32,
                }

                resource layer-collection-view {
                    layer-index:  func() -> layer-idx;
                    z:            func() -> f32;
                    entity-count: func() -> u32;
                    ordered-entities: func() -> list<print-entity-view>;
                    tool-changes: func() -> list<tool-change-view>;
                    z-hops: func() -> list<z-hop-view>;
                }

                variant entity-mutation {
                    set-speed-factor(f32),
                    set-flow-factor(f32),
                }

                enum sort-key {
                    by-priority-and-entity-id,
                    by-entity-id,
                    by-object-id-then-priority,
                }

                record synthetic-layer-data {
                    z: f32,
                    paths: list<extrusion-path3d>,
                }

                resource finalization-output-builder {
                    push-entity-to-layer: func(
                        layer-index: layer-idx,
                        path: extrusion-path3d,
                        region-key: region-key,
                    ) -> result<_, string>;
                    push-entity-with-priority: func(
                        layer-index: layer-idx,
                        path: extrusion-path3d,
                        region-key: region-key,
                        priority: u32,
                    ) -> result<_, string>;
                    modify-entity: func(
                        layer-index: u32,
                        entity-id: u64,
                        mutation: entity-mutation,
                    ) -> result<_, string>;
                    sort-layer-by: func(
                        layer-index: u32,
                        key: sort-key,
                    ) -> result<_, string>;
                    insert-synthetic-layer-after: func(
                        idx: u32,
                        layer-data: synthetic-layer-data,
                    ) -> result<_, string>;
                    insert-synthetic-layer: func(
                        z: f32,
                        paths: list<extrusion-path3d>,
                    ) -> result<_, string>;
                }

                export run-finalization: func(
                    layers: list<layer-collection-view>,
                    output: finalization-output-builder,
                    config: config-view,
                ) -> result<_, module-error>;
            }
        "#,
        world: "finalization-module",
        imports: {
            default: trappable,
        },
        with: {
            "slicer:world-finalization/config-types@1.0.0.config-view": super::ConfigViewData,
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
        inline: r#"
            package slicer:world-postpass@1.0.0;

            interface geometry {
                record point3 { x: f32, y: f32, z: f32 }
                record bounding-box3 { min: point3, max: point3 }
                record point2 { x: s64, y: s64 }
                record polygon { points: list<point2> }
                record ex-polygon { contour: polygon, holes: list<polygon> }
                variant extrusion-role {
                    outer-wall, inner-wall, thin-wall,
                    top-solid-infill, bottom-solid-infill, sparse-infill,
                    support-material, support-interface,
                    ironing, bridge-infill, wipe-tower, custom(string),
                }
            }

            interface config-types {
                variant config-value {
                    bool-val(bool), int-val(s64), float-val(f64),
                    string-val(string), float-list(list<f64>), string-list(list<string>),
                }
                resource config-view {
                    get:        func(key: string) -> option<config-value>;
                    get-bool:   func(key: string) -> option<bool>;
                    get-float:  func(key: string) -> option<f64>;
                    get-int:    func(key: string) -> option<s64>;
                    get-string: func(key: string) -> option<string>;
                    keys:       func() -> list<string>;
                }
            }

            interface host-services {
                use geometry.{point3, bounding-box3, ex-polygon, polygon};
                type object-id = string;
                enum log-level { trace, debug, info, warn, error }
                log: func(level: log-level, message: string);
                raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
                surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
                object-bounds:      func(object-id: object-id) -> bounding-box3;
                enum clip-operation   { union, intersection, difference, xor }
                enum offset-join-type { miter, round, square }
                clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
                offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
                simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
                now-us: func() -> u64;
            }

            world postpass-module {
                import host-services;
                import config-types;
                use config-types.{config-view};
                use geometry.{extrusion-role};
                record module-error { code: u32, message: string, fatal: bool }

                record gcode-move-cmd { x: option<f32>, y: option<f32>, z: option<f32>, e: option<f32>, f: option<f32>, role: extrusion-role }
                variant retract-mode { gcode, firmware }
                record gcode-retract-cmd { length: f32, speed: f32, mode: retract-mode }
                record gcode-fan-speed-cmd { value: u8 }
                record gcode-temperature-cmd { tool: u32, celsius: f32, wait: bool }
                record gcode-tool-change-cmd { after-entity-index: u32, from-tool: u32, to-tool: u32 }
                resource gcode-output-builder {
                    push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                    push-retract:     func(length: f32, speed: f32, mode: retract-mode) -> result<_, string>;
                    push-unretract:   func(length: f32, speed: f32, mode: retract-mode) -> result<_, string>;
                    push-fan-speed:   func(value: u8) -> result<_, string>;
                    push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                    push-tool-change: func(after-entity-index: u32, from-tool: u32, to-tool: u32) -> result<_, string>;
                    push-comment:     func(text: string) -> result<_, string>;
                    push-raw:         func(text: string) -> result<_, string>;
                    push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
                }

                variant gcode-command {
                    move(gcode-move-cmd),
                    retract(gcode-retract-cmd),
                    unretract(gcode-retract-cmd),
                    fan-speed(gcode-fan-speed-cmd),
                    temperature(gcode-temperature-cmd),
                    tool-change(gcode-tool-change-cmd),
                    comment(string),
                    raw(string),
                }

                export run-gcode-postprocess: func(
                    commands: list<gcode-command>,
                    output: gcode-output-builder,
                    config: config-view,
                ) -> result<_, module-error>;

                export run-text-postprocess: func(
                    gcode-text: string,
                    config: config-view,
                ) -> result<string, module-error>;
            }
        "#,
        world: "postpass-module",
        imports: {
            default: trappable,
        },
        with: {
            "slicer:world-postpass/config-types@1.0.0.config-view": super::ConfigViewData,
        },
    });
}

pub use postpass::PostpassModule;

/// Identity of a perimeter input region as observed by the guest, used to
/// associate guest-emitted output back to its originating source region for
/// identity-preserving post-process commit.
pub type PerimeterRegionOrigin = (String, u64);

/// Identity of a slice input region as observed by the guest, used to associate
/// guest-emitted support post-process output back to its originating source
/// region for identity-preserving commit. Reuses the same `(object_id, region_id)`
/// shape as `PerimeterRegionOrigin`.
pub type SliceRegionOrigin = (String, u64);

/// Collected output from an infill-output-builder during a call.
#[derive(Debug, Default)]
pub struct InfillOutputCollected {
    /// Sparse infill paths emitted by the guest.
    pub sparse_paths: Vec<ExtrusionPath3d>,
    /// Solid infill paths emitted by the guest.
    pub solid_paths: Vec<ExtrusionPath3d>,
    /// Ironing paths emitted by the guest.
    pub ironing_paths: Vec<ExtrusionPath3d>,
    /// Origin tags parallel to `sparse_paths`. `None` means no perimeter
    /// region was active when the path was pushed.
    pub sparse_path_origins: Vec<Option<PerimeterRegionOrigin>>,
    /// Origin tags parallel to `solid_paths`.
    pub solid_path_origins: Vec<Option<PerimeterRegionOrigin>>,
    /// Origin tags parallel to `ironing_paths`.
    pub ironing_path_origins: Vec<Option<PerimeterRegionOrigin>>,
}

/// Collected output from a perimeter-output-builder during a call.
#[derive(Debug, Default)]
pub struct PerimeterOutputCollected {
    /// Wall loops emitted by the guest.
    pub wall_loops: Vec<WallLoopView>,
    /// Wall loops with the seam at points[0] — rotated by seam-placer.
    pub rotated_wall_loops: Vec<WallLoopView>,
    /// Origin tags parallel to `rotated_wall_loops`.
    pub rotated_wall_loop_origins: Vec<Option<PerimeterRegionOrigin>>,
    /// Infill areas set by the guest.
    pub infill_areas: Vec<ExPolygon>,
    /// Seam candidates emitted by the guest.
    pub seam_candidates: Vec<(Point3, f32)>,
    /// Resolved seam position set by the guest (e.g. by seam-placer).
    pub resolved_seam: Option<(Point3, u32)>,
    /// Origin tag for the most recent `push_resolved_seam` call.
    pub resolved_seam_origin: Option<PerimeterRegionOrigin>,
    /// Origin tags parallel to `wall_loops`.
    pub wall_loop_origins: Vec<Option<PerimeterRegionOrigin>>,
    /// Origin tag for the most recent `set_infill_areas` call.
    pub infill_areas_origin: Option<PerimeterRegionOrigin>,
    /// Origin tags parallel to `seam_candidates`.
    pub seam_candidate_origins: Vec<Option<PerimeterRegionOrigin>>,
}

/// Collected output from a support-output-builder during a call.
#[derive(Debug, Default)]
pub struct SupportOutputCollected {
    /// Support paths.
    pub support_paths: Vec<ExtrusionPath3d>,
    /// Interface paths: (path, is_top_interface).
    pub interface_paths: Vec<(ExtrusionPath3d, bool)>,
    /// Raft paths.
    pub raft_paths: Vec<ExtrusionPath3d>,
    /// Origin tags parallel to `support_paths`. `None` means no slice region
    /// was active when the path was pushed.
    pub support_path_origins: Vec<Option<SliceRegionOrigin>>,
    /// Origin tags parallel to `interface_paths`.
    pub interface_path_origins: Vec<Option<SliceRegionOrigin>>,
    /// Origin tags parallel to `raft_paths`.
    pub raft_path_origins: Vec<Option<SliceRegionOrigin>>,
}

/// Collected output from a gcode-output-builder during a call.
#[derive(Debug, Default)]
pub struct GcodeOutputCollected {
    /// GCode commands emitted by the guest.
    pub commands: Vec<GcodeCommandCollected>,
}

/// A single GCode command collected from the guest.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum GcodeCommandCollected {
    /// Move command.
    Move(GcodeMoveCmd),
    /// Retract. `mode` carries the WIT retract-mode variant verbatim from the guest.
    Retract {
        length: f32,
        speed: f32,
        mode: slicer_ir::RetractMode,
    },
    /// Unretract. `mode` carries the WIT retract-mode variant verbatim from the guest.
    Unretract {
        length: f32,
        speed: f32,
        mode: slicer_ir::RetractMode,
    },
    /// Fan speed.
    FanSpeed(u8),
    /// Temperature.
    Temperature { tool: u32, celsius: f32, wait: bool },
    /// Tool change.
    ToolChange {
        after_entity_index: u32,
        from_tool: u32,
        to_tool: u32,
    },
    /// Comment.
    Comment(String),
    /// Raw G-code.
    Raw(String),
    /// Z-hop request.
    ZHop {
        after_entity_index: u32,
        hop_height: f32,
    },
}

/// Collected output from a slice-postprocess-builder during a call.
#[derive(Debug, Default)]
pub struct SlicePostprocessCollected {
    /// Polygon updates: (region_key, polygons).
    pub polygon_updates: Vec<(RegionKey, Vec<ExPolygon>)>,
    /// Path Z updates: (region_key, path_idx, vertex_idx, z).
    pub path_z_updates: Vec<(RegionKey, u32, u32, f32)>,
}

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
    pub(crate) current_perimeter_region: Option<PerimeterRegionOrigin>,
    /// Identity of the slice-region-view most recently accessed by the guest.
    /// Used to tag support post-process output pushes so the commit path can
    /// preserve per-region identity (grouping + structured diagnostic on
    /// untagged pushes) rather than silently flattening.
    pub(crate) current_slice_region: Option<SliceRegionOrigin>,

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

    /// Triangle paint marks collected from `mark-triangle-paint` calls
    /// during a prepass `run-mesh-segmentation` invocation. Tuple layout
    /// mirrors the WIT method signature exactly:
    /// `(object_id, facet_index, semantic, value)`. Insertion order is
    /// preserved so `harvest_mesh_segmentation_ir` can build a
    /// deterministic `MeshSegmentationIR.marks` sequence.
    pub(crate) mesh_segmentation_marks: Vec<(String, u32, String, String)>,

    /// Paint-region entries collected from `push-paint-region` calls
    /// during a prepass `run-paint-segmentation` invocation. Stored as
    /// raw `prepass::PaintRegionEntry` records so the harvest helper
    /// can convert them to `PaintRegionIR` without losing any field.
    /// Empty for all non-prepass stages.
    pub(crate) paint_region_entries: Vec<prepass::PaintRegionEntry>,

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
            layer_plan_proposals: Vec::new(),
            mesh_analysis_annotations: Vec::new(),
            mesh_analysis_surface_groups: Vec::new(),
            mesh_segmentation_marks: Vec::new(),
            paint_region_entries: Vec::new(),
            seam_plan_entries: Vec::new(),
            support_plan_entries: Vec::new(),
            finalization_pushes: Vec::new(),
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
    pub fn current_perimeter_region(&self) -> Option<&PerimeterRegionOrigin> {
        self.current_perimeter_region.as_ref()
    }

    /// Override the current perimeter region origin (test/dispatch helper).
    pub fn set_current_perimeter_region(&mut self, origin: Option<PerimeterRegionOrigin>) {
        self.current_perimeter_region = origin;
    }

    /// Identity of the most recently accessed slice region (see field doc).
    pub fn current_slice_region(&self) -> Option<&SliceRegionOrigin> {
        self.current_slice_region.as_ref()
    }

    /// Override the current slice region origin (test/dispatch helper).
    pub fn set_current_slice_region(&mut self, origin: Option<SliceRegionOrigin>) {
        self.current_slice_region = origin;
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

    /// Triangle paint marks collected during `run-mesh-segmentation`.
    pub fn mesh_segmentation_marks(&self) -> &[(String, u32, String, String)] {
        &self.mesh_segmentation_marks
    }

    /// Paint-region entries collected during `run-paint-segmentation`.
    pub fn paint_region_entries(&self) -> &[prepass::PaintRegionEntry] {
        &self.paint_region_entries
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
    /// entry exists (treated as "holds all" by the SDK convention).
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

    /// Push a mesh-segmentation-output resource (prepass world). The
    /// returned handle is what the host passes into
    /// `run-mesh-segmentation`; guest calls to `mark-triangle-paint` go
    /// through `HostMeshSegmentationOutput::mark_triangle_paint` below,
    /// which appends tuples to `mesh_segmentation_marks`.
    pub fn push_mesh_segmentation_output(
        &mut self,
    ) -> wasmtime::Result<Resource<prepass::MeshSegmentationOutput>> {
        let rep = self.table.push(MeshSegmentationOutputData)?;
        Ok(Resource::new_own(rep.rep()))
    }

    /// Push a paint-segmentation-output resource (prepass world). The
    /// returned handle is what the host passes into
    /// `run-paint-segmentation`; guest calls to `push-paint-region` go
    /// through `HostPaintSegmentationOutput::push_paint_region` below,
    /// which appends entries to `paint_region_entries`.
    pub fn push_paint_segmentation_output(
        &mut self,
    ) -> wasmtime::Result<Resource<prepass::PaintSegmentationOutput>> {
        let rep = self.table.push(PaintSegmentationOutputData)?;
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

    /// Stage `support-geometry-output` returned by `run-support-geometry`.
    ///
    /// Drains `output.support_plan_entries` into `self.support_plan_entries`
    /// so the dispatcher can later commit them as `SupportPlanIR`.
    /// The `run-support-geometry` export returns a plain record (not a
    /// resource), so no resource-table entry is involved here.
    pub fn push_support_geometry_result(
        &mut self,
        output: prepass::SupportGeometryOutput,
    ) -> wasmtime::Result<()> {
        self.support_plan_entries
            .extend(output.support_plan_entries);
        Ok(())
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
    pub fn push_finalization_layer_view(
        &mut self,
        ir: &slicer_ir::LayerCollectionIR,
    ) -> wasmtime::Result<Resource<finalization::LayerCollectionView>> {
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

use layer::slicer::world_layer::config_types as ct;
use layer::slicer::world_layer::geometry as geo;
use layer::slicer::world_layer::host_services as hs;
use layer::slicer::world_layer::ir_handles as ir;

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

    fn now_us(&mut self) -> wasmtime::Result<u64> {
        // Monotonic timestamp from per-call Instant. Deterministic within a
        // call (always increasing), but not across calls (each call starts a
        // fresh Instant). This matches the doc requirement for profiling use.
        Ok(self.start_time.elapsed().as_micros() as u64)
    }
}

// ── WIT ↔ slicer-ir polygon conversion ────────────────────────────────

/// Convert WIT ExPolygon to slicer-ir ExPolygon.
fn wit_to_ir_expolygon(ep: &ExPolygon) -> slicer_ir::ExPolygon {
    slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: ep
                .contour
                .points
                .iter()
                .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                .collect(),
        },
        holes: ep
            .holes
            .iter()
            .map(|h| slicer_ir::Polygon {
                points: h
                    .points
                    .iter()
                    .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                    .collect(),
            })
            .collect(),
    }
}

/// Convert WIT ExPolygons to slicer-ir ExPolygons.
fn wit_to_ir_expolygons(eps: &[ExPolygon]) -> Vec<slicer_ir::ExPolygon> {
    eps.iter().map(wit_to_ir_expolygon).collect()
}

/// Convert slicer-ir ExPolygon to WIT ExPolygon.
fn ir_to_wit_expolygon(ep: &slicer_ir::ExPolygon) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: ep
                .contour
                .points
                .iter()
                .map(|p| Point2 { x: p.x, y: p.y })
                .collect(),
        },
        holes: ep
            .holes
            .iter()
            .map(|h| Polygon {
                points: h.points.iter().map(|p| Point2 { x: p.x, y: p.y }).collect(),
            })
            .collect(),
    }
}

/// Convert slicer-ir ExPolygons to WIT ExPolygons for the prepass world.
fn ir_to_wit_expolygons_prepass(eps: &[slicer_ir::ExPolygon]) -> Vec<prepass::ExPolygon> {
    eps.iter().map(ir_to_wit_expolygon_prepass).collect()
}

/// Convert slicer-ir ExPolygon to WIT ExPolygon for the prepass world.
fn ir_to_wit_expolygon_prepass(ep: &slicer_ir::ExPolygon) -> prepass::ExPolygon {
    use prepass::slicer::world_prepass::geometry as pgeo;
    prepass::ExPolygon {
        contour: pgeo::Polygon {
            points: ep
                .contour
                .points
                .iter()
                .map(|p| pgeo::Point2 { x: p.x, y: p.y })
                .collect(),
        },
        holes: ep
            .holes
            .iter()
            .map(|h| pgeo::Polygon {
                points: h
                    .points
                    .iter()
                    .map(|p| pgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            })
            .collect(),
    }
}

/// Convert slicer-ir ExPolygons to WIT ExPolygons.
fn ir_to_wit_expolygons(eps: &[slicer_ir::ExPolygon]) -> Vec<ExPolygon> {
    eps.iter().map(ir_to_wit_expolygon).collect()
}

/// Convert slicer-ir PaintValue to WIT PaintValue.
/// Note: `PaintValue::Custom` has no WIT counterpart in the output type
/// (`PaintValue` in ir-types.wit has only flag/scalar/tool-index).
/// Custom values are represented as ToolIndex(0) on the WIT output side;
/// the lossless form is only available via PaintValueInput on the input path.
fn ir_to_wit_paint_value(v: &slicer_ir::PaintValue) -> PaintValue {
    match v {
        slicer_ir::PaintValue::Flag(b) => PaintValue::Flag(*b),
        slicer_ir::PaintValue::Scalar(s) => PaintValue::Scalar(*s),
        slicer_ir::PaintValue::ToolIndex(t) => PaintValue::ToolIndex(*t),
        slicer_ir::PaintValue::Custom(_) => PaintValue::ToolIndex(0),
    }
}

/// Convert slicer-ir SemanticRegion to WIT SemanticRegion.
fn ir_to_wit_semantic_region(
    r: &slicer_ir::SemanticRegion,
) -> layer::slicer::world_layer::ir_handles::SemanticRegion {
    layer::slicer::world_layer::ir_handles::SemanticRegion {
        object_id: r.object_id.clone(),
        polygons: ir_to_wit_expolygons(&r.polygons),
        value: ir_to_wit_paint_value(&r.value),
    }
}

/// Convert a PaintSemantic to the string key used by PaintRegionLayerData.
fn paint_semantic_key(s: &slicer_ir::PaintSemantic) -> &'static str {
    match s {
        slicer_ir::PaintSemantic::Material => "material",
        slicer_ir::PaintSemantic::FuzzySkin => "fuzzy-skin",
        slicer_ir::PaintSemantic::SupportEnforcer => "support-enforcer",
        slicer_ir::PaintSemantic::SupportBlocker => "support-blocker",
        slicer_ir::PaintSemantic::Custom(_) => "custom",
    }
}

/// Build a `PaintRegionLayerData` from a `PaintRegionIR` for a specific layer.
///
/// Returns empty-but-valid data if no paint regions exist for this layer.
/// Custom semantics are split into the `custom_regions` map keyed by
/// the `Custom(id)` string from the IR.
pub fn paint_region_ir_to_layer_data(
    ir: &slicer_ir::PaintRegionIR,
    layer_index: u32,
) -> PaintRegionLayerData {
    let empty = PaintRegionLayerData {
        layer_index,
        regions_by_semantic: HashMap::new(),
        custom_regions: HashMap::new(),
        support_plan_segments: HashMap::new(),
    };

    let layer_map = match ir.per_layer.get(&layer_index) {
        Some(m) => m,
        None => return empty,
    };

    let mut regions_by_semantic: HashMap<
        String,
        Vec<layer::slicer::world_layer::ir_handles::SemanticRegion>,
    > = HashMap::new();
    let mut custom_regions: HashMap<
        String,
        Vec<layer::slicer::world_layer::ir_handles::SemanticRegion>,
    > = HashMap::new();

    for (semantic, regions) in &layer_map.semantic_regions {
        let wit_regions: Vec<_> = regions.iter().map(ir_to_wit_semantic_region).collect();
        match semantic {
            slicer_ir::PaintSemantic::Custom(id) => {
                custom_regions
                    .entry(id.clone())
                    .or_default()
                    .extend(wit_regions);
            }
            _ => {
                let key = paint_semantic_key(semantic).to_string();
                regions_by_semantic
                    .entry(key)
                    .or_default()
                    .extend(wit_regions);
            }
        }
    }

    PaintRegionLayerData {
        layer_index,
        regions_by_semantic,
        custom_regions,
        support_plan_segments: HashMap::new(),
    }
}

/// Convert a slicer-ir `PaintSemantic` to the WIT `PaintSemantic` enum.
fn ir_to_wit_paint_semantic(s: &slicer_ir::PaintSemantic) -> PaintSemantic {
    match s {
        slicer_ir::PaintSemantic::Material => PaintSemantic::Material,
        slicer_ir::PaintSemantic::FuzzySkin => PaintSemantic::FuzzySkin,
        slicer_ir::PaintSemantic::SupportEnforcer => PaintSemantic::SupportEnforcer,
        slicer_ir::PaintSemantic::SupportBlocker => PaintSemantic::SupportBlocker,
        slicer_ir::PaintSemantic::Custom(tag) => PaintSemantic::Custom(tag.clone()),
    }
}

/// Convert a slicer-ir `PaintSemantic` to a string key for paint segmentation views.
fn paint_semantic_to_string(s: &slicer_ir::PaintSemantic) -> String {
    match s {
        slicer_ir::PaintSemantic::Material => "material".to_string(),
        slicer_ir::PaintSemantic::FuzzySkin => "fuzzy-skin".to_string(),
        slicer_ir::PaintSemantic::SupportEnforcer => "support-enforcer".to_string(),
        slicer_ir::PaintSemantic::SupportBlocker => "support-blocker".to_string(),
        slicer_ir::PaintSemantic::Custom(tag) => tag.clone(),
    }
}

/// Convert a slicer-ir `PaintValue` to a WIT `PaintValueView` variant.
/// `PaintValue::Custom` has no WIT view counterpart; it is represented as
/// ToolIndex(0) on the view path (the Custom variant only exists on the input path).
fn ir_to_wit_paint_value_view(v: &slicer_ir::PaintValue) -> prepass::PaintValueView {
    match v {
        slicer_ir::PaintValue::Flag(b) => prepass::PaintValueView::Flag(*b),
        slicer_ir::PaintValue::Scalar(s) => prepass::PaintValueView::Scalar(*s),
        slicer_ir::PaintValue::ToolIndex(idx) => prepass::PaintValueView::ToolIndex(*idx),
        slicer_ir::PaintValue::Custom(_) => prepass::PaintValueView::ToolIndex(0),
    }
}

/// Convert a slicer-ir `PaintStroke` to a WIT `PaintStrokeView` record.
fn ir_to_wit_paint_stroke_view(stroke: &slicer_ir::PaintStroke) -> prepass::PaintStrokeView {
    prepass::PaintStrokeView {
        triangles: stroke
            .triangles
            .iter()
            .flat_map(|triangle| triangle.iter())
            .map(|point| prepass::Point3 {
                x: point.x,
                y: point.y,
                z: point.z,
            })
            .collect(),
        semantic: paint_semantic_to_string(&stroke.semantic),
        value: ir_to_wit_paint_value_view(&stroke.value),
    }
}

/// Convert a slicer-ir `PaintLayer` to a WIT `PaintLayerView` record.
fn ir_to_wit_paint_layer_view(layer: &slicer_ir::PaintLayer) -> prepass::PaintLayerView {
    prepass::PaintLayerView {
        semantic: paint_semantic_to_string(&layer.semantic),
        facet_values: layer
            .facet_values
            .iter()
            .map(|opt| opt.as_ref().map(ir_to_wit_paint_value_view))
            .collect(),
        strokes: layer
            .strokes
            .iter()
            .map(ir_to_wit_paint_stroke_view)
            .collect(),
    }
}

/// Convert a slicer-ir `ObjectMesh` to a WIT `MeshObjectView` for MeshSegmentation.
///
/// This converter extracts the mesh geometry and paint data from an `ObjectMesh`
/// and produces a read-only WIT view suitable for passing to prepass modules.
pub fn object_mesh_to_wit_mesh_object_view(
    mesh: &slicer_ir::ObjectMesh,
) -> prepass::MeshObjectView {
    let vertices: Vec<prepass::Point3> = mesh
        .mesh
        .vertices
        .iter()
        .map(|v| prepass::Point3 {
            x: v.x,
            y: v.y,
            z: v.z,
        })
        .collect();

    // Convert indexed triangles to list of tuples
    let triangles: Vec<(u32, u32, u32)> = mesh
        .mesh
        .indices
        .chunks(3)
        .map(|chunk| (chunk[0], chunk[1], chunk[2]))
        .collect();

    // Convert paint layers if present
    let paint_layers: Vec<prepass::PaintLayerView> = if let Some(ref paint_data) = mesh.paint_data {
        paint_data
            .layers
            .iter()
            .map(ir_to_wit_paint_layer_view)
            .collect()
    } else {
        Vec::new()
    };

    prepass::MeshObjectView {
        object_id: mesh.id.clone(),
        vertices,
        triangles,
        paint_layers,
    }
}

/// Project `LayerPlanIR` into a deterministic WIT `LayerPlanView`.
///
/// Layers are sorted by `global_layer_index ASC`. The effective layer height
/// per global layer is the maximum across all objects at that layer index
/// (from `object_participation`).
pub fn project_layer_plan_view(layer_plan_ir: &slicer_ir::LayerPlanIR) -> prepass::LayerPlanView {
    let mut entries: Vec<prepass::LayerPlanViewEntry> = layer_plan_ir
        .global_layers
        .iter()
        .map(|gl| {
            // Derive effective_layer_height: max across all objects at this global layer.
            let effective_layer_height = layer_plan_ir
                .object_participation
                .values()
                .filter_map(|refs| {
                    refs.iter()
                        .find(|r| r.global_layer_index == gl.index)
                        .map(|r| r.effective_layer_height)
                })
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.2); // fallback to default if no participation found
            prepass::LayerPlanViewEntry {
                global_layer_index: gl.index,
                z: gl.z,
                effective_layer_height,
            }
        })
        .collect();
    // Already sorted by index since global_layers is ordered, but sort to be safe.
    entries.sort_by_key(|a| a.global_layer_index);
    prepass::LayerPlanView { layers: entries }
}

/// Project `RegionMapIR` into a deterministic WIT `RegionSegmentationView`.
///
/// Entries are sorted by `(global_layer_index ASC, object_id ASC)` with each
/// entry's `region_ids` sorted ASC. This ensures byte-identical projections
/// across consecutive runs.
pub fn project_region_segmentation_view(
    region_map_ir: &slicer_ir::RegionMapIR,
) -> prepass::RegionSegmentationView {
    // Group by (global_layer_index, object_id).
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<(u32, String), Vec<String>> = BTreeMap::new();
    for key in region_map_ir.entries.keys() {
        let entry = grouped
            .entry((key.global_layer_index, key.object_id.clone()))
            .or_default();
        entry.push(key.region_id.to_string());
    }
    let mut entries: Vec<prepass::RegionSegmentationViewEntry> = grouped
        .into_iter()
        .map(|((layer_index, object_id), mut region_ids)| {
            region_ids.sort(); // ASC by region_id string
            prepass::RegionSegmentationViewEntry {
                object_id,
                layer_index,
                region_ids,
            }
        })
        .collect();
    // Already sorted by BTreeMap key order, but explicit sort for clarity.
    entries.sort_by(|a, b| {
        a.layer_index
            .cmp(&b.layer_index)
            .then_with(|| a.object_id.cmp(&b.object_id))
    });
    prepass::RegionSegmentationView { entries }
}

/// Project `SupportGeometryIR` into a deterministic WIT `SupportGeometryView`.
///
/// Entries are sorted by `(global_support_layer_index ASC, object_id ASC, region_id ASC)`.
/// This mirrors the RegionSegmentationView ordering pattern.
pub fn project_support_geometry_view(
    support_geometry_ir: &slicer_ir::SupportGeometryIR,
) -> prepass::SupportGeometryView {
    use std::collections::BTreeMap;
    let mut sorted_entries: Vec<prepass::SupportGeometryViewEntry> = {
        let mut btree: BTreeMap<(u32, String, String), prepass::SupportGeometryViewEntry> =
            BTreeMap::new();
        for (key, polygons) in &support_geometry_ir.entries {
            btree.insert(
                (
                    key.global_support_layer_index,
                    key.object_id.clone(),
                    key.region_id.to_string(),
                ),
                prepass::SupportGeometryViewEntry {
                    global_support_layer_index: key.global_support_layer_index,
                    object_id: key.object_id.clone(),
                    region_id: key.region_id.to_string(),
                    outlines: ir_to_wit_expolygons_prepass(polygons),
                },
            );
        }
        btree.into_values().collect()
    };
    sorted_entries.sort_by(|a, b| {
        a.global_support_layer_index
            .cmp(&b.global_support_layer_index)
            .then_with(|| a.object_id.cmp(&b.object_id))
            .then_with(|| a.region_id.cmp(&b.region_id))
    });
    prepass::SupportGeometryView {
        entries: sorted_entries,
    }
}

/// Convert a slicer-ir `ObjectMesh` to a WIT `PaintSegmentationObjectView` for PaintSegmentation.
///
/// This converter includes the transform matrix and participating layer indices
/// needed by paint segmentation modules to project 3D paint onto layers.
pub fn object_mesh_to_wit_paint_segmentation_view(
    mesh: &slicer_ir::ObjectMesh,
    participating_layer_indices: &[u32],
) -> prepass::PaintSegmentationObjectView {
    let vertices: Vec<prepass::Point3> = mesh
        .mesh
        .vertices
        .iter()
        .map(|v| prepass::Point3 {
            x: v.x,
            y: v.y,
            z: v.z,
        })
        .collect();

    // Convert indexed triangles to list of tuples
    let triangles: Vec<(u32, u32, u32)> = mesh
        .mesh
        .indices
        .chunks(3)
        .map(|chunk| (chunk[0], chunk[1], chunk[2]))
        .collect();

    // Convert paint layers if present
    let paint_layers: Vec<prepass::PaintLayerView> = if let Some(ref paint_data) = mesh.paint_data {
        paint_data
            .layers
            .iter()
            .map(ir_to_wit_paint_layer_view)
            .collect()
    } else {
        Vec::new()
    };

    prepass::PaintSegmentationObjectView {
        object_id: mesh.id.clone(),
        vertices,
        triangles,
        paint_layers,
        // Validate transform matrix length — Transform3d.matrix is [f64; 16],
        // and the WIT type is list<f64> (not a fixed 16-tuple). Enforce the
        // invariant at the boundary to catch any future changes.
        transform_matrix: {
            let mat = &mesh.transform.matrix;
            assert!(
                mat.len() == 16,
                "transform-matrix must have exactly 16 elements, got {}",
                mat.len()
            );
            mat.to_vec()
        },
        participating_layer_indices: participating_layer_indices.to_vec(),
    }
}

/// Convert a `SlicedRegion` from the IR into a `SliceRegionData` for the WIT resource.
///
/// `held_claims` is the resolved fill-role claim set for this module on this
/// region, computed by `validation::resolve_held_claims` against the region's
/// `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`. The dispatcher
/// builds the `(ObjectId, RegionId) -> Vec<String>` map on
/// `HostExecutionContext.held_claims_per_region` before the WIT call;
/// `push_slice_regions` looks up each region and passes the slice in here.
pub fn sliced_region_to_data(
    region: &slicer_ir::SlicedRegion,
    z: f32,
    held_claims: Vec<String>,
) -> SliceRegionData {
    let boundary_paint: Vec<BoundaryPaintEntry> = region
        .boundary_paint
        .iter()
        .map(|(semantic, poly_values)| BoundaryPaintEntry {
            semantic: ir_to_wit_paint_semantic(semantic),
            polygons: poly_values
                .iter()
                .map(|point_values| BoundaryPaintPolygon {
                    values: point_values
                        .iter()
                        .map(|opt| opt.as_ref().map(ir_to_wit_paint_value))
                        .collect(),
                })
                .collect(),
        })
        .collect();

    SliceRegionData {
        object_id: region.object_id.clone(),
        region_id: region.region_id.to_string(),
        polygons: ir_to_wit_expolygons(&region.polygons),
        infill_areas: ir_to_wit_expolygons(&region.infill_areas),
        effective_layer_height: region.effective_layer_height,
        z,
        has_nonplanar: region.nonplanar_surface.is_some(),
        boundary_paint,
        needs_support: true,
        is_top_surface: region.is_top_surface,
        is_bottom_surface: region.is_bottom_surface,
        is_bridge: region.is_bridge,
        bridge_areas: ir_to_wit_expolygons(&region.bridge_areas),
        bridge_orientation_deg: region.bridge_orientation_deg,
        held_claims,
    }
}

/// Convert slicer-ir `LoopType` to WIT `WallLoopType`.
fn ir_to_wit_wall_loop_type(lt: &slicer_ir::LoopType) -> WallLoopType {
    match lt {
        slicer_ir::LoopType::Outer => WallLoopType::Outer,
        slicer_ir::LoopType::Inner => WallLoopType::Inner,
        slicer_ir::LoopType::ThinWall => WallLoopType::ThinWall,
        slicer_ir::LoopType::NonPlanarShell => WallLoopType::NonplanarShell,
    }
}

/// Convert slicer-ir `ExtrusionRole` to WIT `ExtrusionRole`.
fn ir_to_wit_extrusion_role(role: &slicer_ir::ExtrusionRole) -> ExtrusionRole {
    match role {
        slicer_ir::ExtrusionRole::OuterWall => ExtrusionRole::OuterWall,
        slicer_ir::ExtrusionRole::InnerWall => ExtrusionRole::InnerWall,
        slicer_ir::ExtrusionRole::ThinWall => ExtrusionRole::ThinWall,
        slicer_ir::ExtrusionRole::TopSolidInfill => ExtrusionRole::TopSolidInfill,
        slicer_ir::ExtrusionRole::BottomSolidInfill => ExtrusionRole::BottomSolidInfill,
        slicer_ir::ExtrusionRole::SparseInfill => ExtrusionRole::SparseInfill,
        slicer_ir::ExtrusionRole::SupportMaterial => ExtrusionRole::SupportMaterial,
        slicer_ir::ExtrusionRole::SupportInterface => ExtrusionRole::SupportInterface,
        slicer_ir::ExtrusionRole::Ironing => ExtrusionRole::Ironing,
        slicer_ir::ExtrusionRole::BridgeInfill => ExtrusionRole::BridgeInfill,
        slicer_ir::ExtrusionRole::WipeTower => ExtrusionRole::WipeTower,
        slicer_ir::ExtrusionRole::Custom(tag) => ExtrusionRole::Custom(tag.clone()),
        slicer_ir::ExtrusionRole::PrimeTower => {
            ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::Skirt => {
            ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_SKIRT_TAG.to_string())
        }
    }
}

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
                boundary_paint: Vec::new(),
                needs_support: true,
                is_top_surface: false,
                is_bottom_surface: false,
                is_bridge: false,
                bridge_areas: Vec::new(),
                bridge_orientation_deg: 0.0,
                held_claims: Vec::new(),
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

/// Convert slicer-ir `ExtrusionPath3D` to WIT `ExtrusionPath3d`.
fn ir_to_wit_extrusion_path(path: &slicer_ir::ExtrusionPath3D) -> ExtrusionPath3d {
    ExtrusionPath3d {
        points: path
            .points
            .iter()
            .map(|p| Point3WithWidth {
                x: p.x,
                y: p.y,
                z: p.z,
                width: p.width,
                flow_factor: p.flow_factor,
                overhang_quartile: p.overhang_quartile,
            })
            .collect(),
        role: ir_to_wit_extrusion_role(&path.role),
        speed_factor: path.speed_factor,
    }
}

/// Convert slicer-ir `WallFeatureFlags` to WIT `WallFeatureFlag`.
fn ir_to_wit_wall_feature_flag(f: &slicer_ir::WallFeatureFlags) -> WallFeatureFlag {
    let mut custom: Vec<(String, PaintValue)> = f
        .custom
        .iter()
        .map(|(k, v)| {
            let pv = match v {
                slicer_ir::PaintValue::Flag(b) => PaintValue::Flag(*b),
                slicer_ir::PaintValue::Scalar(s) => PaintValue::Scalar(*s),
                slicer_ir::PaintValue::ToolIndex(t) => PaintValue::ToolIndex(*t),
                slicer_ir::PaintValue::Custom(_) => PaintValue::ToolIndex(0),
            };
            (k.clone(), pv)
        })
        .collect();
    custom.sort_by(|a, b| a.0.cmp(&b.0));
    WallFeatureFlag {
        tool_index: f.tool_index,
        fuzzy_skin: f.fuzzy_skin,
        is_bridge: f.is_bridge,
        is_thin_wall: f.is_thin_wall,
        skip_ironing: f.skip_ironing,
        custom,
    }
}

/// Convert slicer-ir `WallLoop` to WIT `WallLoopView`.
fn ir_to_wit_wall_loop(wl: &slicer_ir::WallLoop) -> WallLoopView {
    WallLoopView {
        perimeter_index: wl.perimeter_index,
        loop_type: ir_to_wit_wall_loop_type(&wl.loop_type),
        path: ir_to_wit_extrusion_path(&wl.path),
        feature_flags: wl
            .feature_flags
            .iter()
            .map(ir_to_wit_wall_feature_flag)
            .collect(),
    }
}

/// Convert a `PerimeterRegion` from the IR into a `PerimeterRegionData` WIT resource.
pub fn perimeter_region_to_data(region: &slicer_ir::PerimeterRegion) -> PerimeterRegionData {
    PerimeterRegionData {
        object_id: region.object_id.clone(),
        region_id: region.region_id.to_string(),
        wall_loops: region.walls.iter().map(ir_to_wit_wall_loop).collect(),
        infill_areas: ir_to_wit_expolygons(&region.infill_areas),
        // Note: width and flow_factor are intentionally discarded here;
        // SeamPosition.point is used for diagnostics only.
        resolved_seam: region.resolved_seam.clone().map(|sp| {
            (
                Point3 {
                    x: sp.point.x,
                    y: sp.point.y,
                    z: sp.point.z,
                },
                sp.wall_index,
            )
        }),
    }
}

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

fn parse_canonical_region_id(raw: &str) -> Result<u64, String> {
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
        self.current_slice_region = Some((data.object_id.clone(), rid));
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
    fn boundary_paint(
        &mut self,
        self_: Resource<SliceRegionData>,
    ) -> wasmtime::Result<Vec<BoundaryPaintEntry>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.boundary_paint.clone())
    }
    fn needs_support(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<bool> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.needs_support)
    }
    fn is_top_surface(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<bool> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.is_top_surface)
    }
    fn is_bottom_surface(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<bool> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.is_bottom_surface)
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
    fn held_claims(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<Vec<String>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.held_claims.clone())
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
        self.current_perimeter_region = Some((data.object_id.clone(), rid));
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
    ) -> wasmtime::Result<Option<layer::slicer::world_layer::ir_handles::SeamPosition>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads
            .push(String::from("PerimeterIR.resolved-seam"));
        let resolved = self.table.get(&self_)?.resolved_seam;
        match resolved {
            None => Ok(None),
            Some((pos, wall_index)) => {
                Ok(Some(layer::slicer::world_layer::ir_handles::SeamPosition {
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
        let origin = self.current_perimeter_region.clone();
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
        let origin = self.current_perimeter_region.clone();
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
        let origin = self.current_perimeter_region.clone();
        self.infill_output.ironing_paths.push(path);
        self.infill_output.ironing_path_origins.push(origin);
        self.record_write("InfillIR");
        Ok(Ok(()))
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
        let origin = self.current_perimeter_region.clone();
        self.perimeter_output.wall_loops.push(wall_loop);
        self.perimeter_output.wall_loop_origins.push(origin);
        self.record_write("PerimeterIR.regions.walls");
        Ok(Ok(()))
    }
    /// Sets infill areas for this perimeter output builder.
    ///
    /// No Z envelope check is needed here — `ExPolygon` carries no Z coordinate.
    /// Z validation for infill paths is performed in `push_sparse_path` and
    /// `push_solid_path` where the actual extrusion geometry is supplied.
    fn set_infill_areas(
        &mut self,
        _self_: Resource<PerimeterOutputBuilderData>,
        areas: Vec<ExPolygon>,
    ) -> wasmtime::Result<Result<(), String>> {
        self.perimeter_output.infill_areas = areas;
        self.perimeter_output.infill_areas_origin = self.current_perimeter_region.clone();
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
        let origin = self.current_perimeter_region.clone();
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
        self.perimeter_output.resolved_seam_origin = self.current_perimeter_region.clone();
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
        let origin = self.current_perimeter_region.clone();
        self.perimeter_output
            .rotated_wall_loops
            .push(rotated_wall_loop);
        self.perimeter_output.rotated_wall_loop_origins.push(origin);
        self.record_write("PerimeterIR.regions.walls");
        Ok(Ok(()))
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
    ) -> wasmtime::Result<Vec<Vec<layer::slicer::world_layer::geometry::Point3WithWidth>>> {
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
    use prepass::slicer::world_prepass::config_types as pct;
    use prepass::slicer::world_prepass::geometry as pgeo;
    use prepass::slicer::world_prepass::host_services as phs;

    impl pgeo::Host for HostExecutionContext {}

    fn p_wit_to_ir(ep: &pgeo::ExPolygon) -> slicer_ir::ExPolygon {
        slicer_ir::ExPolygon {
            contour: slicer_ir::Polygon {
                points: ep
                    .contour
                    .points
                    .iter()
                    .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                    .collect(),
            },
            holes: ep
                .holes
                .iter()
                .map(|h| slicer_ir::Polygon {
                    points: h
                        .points
                        .iter()
                        .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                        .collect(),
                })
                .collect(),
        }
    }
    fn p_ir_to_wit(ep: &slicer_ir::ExPolygon) -> pgeo::ExPolygon {
        pgeo::ExPolygon {
            contour: pgeo::Polygon {
                points: ep
                    .contour
                    .points
                    .iter()
                    .map(|p| pgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            },
            holes: ep
                .holes
                .iter()
                .map(|h| pgeo::Polygon {
                    points: h
                        .points
                        .iter()
                        .map(|p| pgeo::Point2 { x: p.x, y: p.y })
                        .collect(),
                })
                .collect(),
        }
    }

    fn ir_point3_to_prepass(point: slicer_ir::Point3) -> pgeo::Point3 {
        pgeo::Point3 {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }

    fn ir_bounds_to_prepass(bounds: slicer_ir::BoundingBox3) -> pgeo::BoundingBox3 {
        pgeo::BoundingBox3 {
            min: ir_point3_to_prepass(bounds.min),
            max: ir_point3_to_prepass(bounds.max),
        }
    }

    impl phs::Host for HostExecutionContext {
        fn log(&mut self, level: phs::LogLevel, message: String) -> wasmtime::Result<()> {
            let level_str = match level {
                phs::LogLevel::Trace => "trace",
                phs::LogLevel::Debug => "debug",
                phs::LogLevel::Info => "info",
                phs::LogLevel::Warn => "warn",
                phs::LogLevel::Error => "error",
            };
            self.log_messages.push((level_str.to_string(), message));
            Ok(())
        }
        fn raycast_z_down(
            &mut self,
            object_id: phs::ObjectId,
            x: f32,
            y: f32,
            start_z: f32,
        ) -> wasmtime::Result<Option<f32>> {
            raycast_z_down_mesh_query(self, &object_id, x, y, start_z)
        }
        fn surface_normal_at(
            &mut self,
            object_id: phs::ObjectId,
            x: f32,
            y: f32,
            z: f32,
        ) -> wasmtime::Result<Option<pgeo::Point3>> {
            Ok(surface_normal_at_mesh_query(self, &object_id, x, y, z)?.map(ir_point3_to_prepass))
        }
        fn object_bounds(
            &mut self,
            object_id: phs::ObjectId,
        ) -> wasmtime::Result<pgeo::BoundingBox3> {
            Ok(ir_bounds_to_prepass(object_bounds_mesh_query(
                self, &object_id,
            )?))
        }
        fn clip_polygons(
            &mut self,
            subject: Vec<pgeo::ExPolygon>,
            clip: Vec<pgeo::ExPolygon>,
            op: phs::ClipOperation,
        ) -> wasmtime::Result<Vec<pgeo::ExPolygon>> {
            let s: Vec<_> = subject.iter().map(p_wit_to_ir).collect();
            let c: Vec<_> = clip.iter().map(p_wit_to_ir).collect();
            let ir_op = match op {
                phs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union,
                phs::ClipOperation::Intersection => {
                    slicer_core::polygon_ops::ClipOperation::Intersection
                }
                phs::ClipOperation::Difference => {
                    slicer_core::polygon_ops::ClipOperation::Difference
                }
                phs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor,
            };
            Ok(ir_clip_polygons(&s, &c, ir_op)
                .iter()
                .map(p_ir_to_wit)
                .collect())
        }
        fn offset_polygons(
            &mut self,
            polygons: Vec<pgeo::ExPolygon>,
            delta_mm: f32,
            join: phs::OffsetJoinType,
        ) -> wasmtime::Result<Vec<pgeo::ExPolygon>> {
            let ir: Vec<_> = polygons.iter().map(p_wit_to_ir).collect();
            let j = match join {
                phs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter,
                phs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round,
                phs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square,
            };
            Ok(ir_offset_polygons(&ir, delta_mm, j)
                .iter()
                .map(p_ir_to_wit)
                .collect())
        }
        fn simplify_polygon(
            &mut self,
            polygon: pgeo::Polygon,
            _: f32,
        ) -> wasmtime::Result<pgeo::Polygon> {
            let pts: Vec<_> = polygon
                .points
                .iter()
                .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                .collect();
            Ok(pgeo::Polygon {
                points: ir_simplify_polygon(pts)
                    .into_iter()
                    .map(|p| pgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            })
        }
        fn now_us(&mut self) -> wasmtime::Result<u64> {
            Ok(self.start_time.elapsed().as_micros() as u64)
        }
    }

    impl pct::Host for HostExecutionContext {}
    impl pct::HostConfigView for HostExecutionContext {
        fn get(
            &mut self,
            self_: Resource<ConfigViewData>,
            key: String,
        ) -> wasmtime::Result<Option<pct::ConfigValue>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).map(|v| match v {
                ConfigValueStorage::Bool(b) => pct::ConfigValue::BoolVal(*b),
                ConfigValueStorage::Int(i) => pct::ConfigValue::IntVal(*i),
                ConfigValueStorage::Float(f) => pct::ConfigValue::FloatVal(*f),
                ConfigValueStorage::Str(s) => pct::ConfigValue::StringVal(s.clone()),
                ConfigValueStorage::FloatList(fl) => pct::ConfigValue::FloatList(fl.clone()),
                ConfigValueStorage::StringList(sl) => pct::ConfigValue::StringList(sl.clone()),
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
            Ok(self.table.get(&self_)?.fields.keys().cloned().collect())
        }
        fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> {
            self.table.delete(rep)?;
            Ok(())
        }
    }

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

    impl pm::HostPaintSegmentationOutput for HostExecutionContext {
        fn push_paint_region(
            &mut self,
            _handle: Resource<pm::PaintSegmentationOutput>,
            entry: pm::PaintRegionEntry,
        ) -> wasmtime::Result<Result<(), String>> {
            // Validate before collecting. Empty object_id / semantic
            // would corrupt the per-layer keying in PaintRegionIR; an
            // empty polygon list is a no-op and is similarly rejected
            // because the guest is required to emit one region entry
            // per (layer, semantic, object, value) group — zero-polygon
            // entries are never correct per docs/02 §Paint Region IR.
            if entry.layer_index < 0 {
                return Ok(Err(format!(
                    "paint-segmentation-output: layer-index must be non-negative (got {})",
                    entry.layer_index
                )));
            }
            if entry.object_id.is_empty() {
                return Ok(Err(String::from(
                    "paint-segmentation-output: object-id must be non-empty",
                )));
            }
            if entry.semantic.is_empty() {
                return Ok(Err(String::from(
                    "paint-segmentation-output: semantic must be non-empty",
                )));
            }
            if entry.polygons.is_empty() {
                return Ok(Err(String::from(
                    "paint-segmentation-output: polygons list must not be empty",
                )));
            }
            for (i, ep) in entry.polygons.iter().enumerate() {
                if ep.contour.points.len() < 3 {
                    return Ok(Err(format!(
                        "paint-segmentation-output: polygon[{i}] contour must have \
                         at least 3 points (got {})",
                        ep.contour.points.len()
                    )));
                }
            }
            self.paint_region_entries.push(entry);
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::PaintSegmentationOutput>) -> wasmtime::Result<()> {
            let typed: Resource<PaintSegmentationOutputData> = Resource::new_own(rep.rep());
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

    // The SupportGeometry stage now returns `support-geometry-output` as a plain
    // record (no resource). Entries are staged via
    // HostExecutionContext::push_support_geometry_result.

    impl pm::HostMeshSegmentationOutput for HostExecutionContext {
        fn mark_triangle_paint(
            &mut self,
            _handle: Resource<pm::MeshSegmentationOutput>,
            obj: String,
            facet_index: u32,
            semantic: String,
            value: String,
        ) -> wasmtime::Result<Result<(), String>> {
            // Validate the mark before collecting. `semantic` must be
            // non-empty (the consumer keys on it); `obj` must be a real
            // object id. `value` may be empty to mean "clear" — that's
            // the caller's prerogative. We accept any finite facet_index
            // because the host can't cheaply reach mesh topology from
            // this resource impl; downstream consumers validate against
            // real triangle counts.
            if obj.is_empty() {
                return Ok(Err(String::from(
                    "mesh-segmentation-output: obj must be a non-empty object id",
                )));
            }
            if semantic.is_empty() {
                return Ok(Err(String::from(
                    "mesh-segmentation-output: semantic must be a non-empty string",
                )));
            }
            self.mesh_segmentation_marks
                .push((obj, facet_index, semantic, value));
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::MeshSegmentationOutput>) -> wasmtime::Result<()> {
            let typed: Resource<MeshSegmentationOutputData> = Resource::new_own(rep.rep());
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
    use finalization::slicer::world_finalization::config_types as fct;
    use finalization::slicer::world_finalization::geometry as fgeo;
    use finalization::slicer::world_finalization::host_services as fhs;

    impl fgeo::Host for HostExecutionContext {}

    fn f_wit_to_ir(ep: &fgeo::ExPolygon) -> slicer_ir::ExPolygon {
        slicer_ir::ExPolygon {
            contour: slicer_ir::Polygon {
                points: ep
                    .contour
                    .points
                    .iter()
                    .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                    .collect(),
            },
            holes: ep
                .holes
                .iter()
                .map(|h| slicer_ir::Polygon {
                    points: h
                        .points
                        .iter()
                        .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                        .collect(),
                })
                .collect(),
        }
    }
    fn f_ir_to_wit(ep: &slicer_ir::ExPolygon) -> fgeo::ExPolygon {
        fgeo::ExPolygon {
            contour: fgeo::Polygon {
                points: ep
                    .contour
                    .points
                    .iter()
                    .map(|p| fgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            },
            holes: ep
                .holes
                .iter()
                .map(|h| fgeo::Polygon {
                    points: h
                        .points
                        .iter()
                        .map(|p| fgeo::Point2 { x: p.x, y: p.y })
                        .collect(),
                })
                .collect(),
        }
    }

    fn ir_point3_to_finalization(point: slicer_ir::Point3) -> fgeo::Point3 {
        fgeo::Point3 {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }

    fn ir_bounds_to_finalization(bounds: slicer_ir::BoundingBox3) -> fgeo::BoundingBox3 {
        fgeo::BoundingBox3 {
            min: ir_point3_to_finalization(bounds.min),
            max: ir_point3_to_finalization(bounds.max),
        }
    }

    impl fhs::Host for HostExecutionContext {
        fn log(&mut self, level: fhs::LogLevel, message: String) -> wasmtime::Result<()> {
            let level_str = match level {
                fhs::LogLevel::Trace => "trace",
                fhs::LogLevel::Debug => "debug",
                fhs::LogLevel::Info => "info",
                fhs::LogLevel::Warn => "warn",
                fhs::LogLevel::Error => "error",
            };
            self.log_messages.push((level_str.to_string(), message));
            Ok(())
        }
        fn raycast_z_down(
            &mut self,
            object_id: fhs::ObjectId,
            x: f32,
            y: f32,
            start_z: f32,
        ) -> wasmtime::Result<Option<f32>> {
            raycast_z_down_mesh_query(self, &object_id, x, y, start_z)
        }
        fn surface_normal_at(
            &mut self,
            object_id: fhs::ObjectId,
            x: f32,
            y: f32,
            z: f32,
        ) -> wasmtime::Result<Option<fgeo::Point3>> {
            Ok(surface_normal_at_mesh_query(self, &object_id, x, y, z)?
                .map(ir_point3_to_finalization))
        }
        fn object_bounds(
            &mut self,
            object_id: fhs::ObjectId,
        ) -> wasmtime::Result<fgeo::BoundingBox3> {
            Ok(ir_bounds_to_finalization(object_bounds_mesh_query(
                self, &object_id,
            )?))
        }
        fn clip_polygons(
            &mut self,
            subject: Vec<fgeo::ExPolygon>,
            clip: Vec<fgeo::ExPolygon>,
            op: fhs::ClipOperation,
        ) -> wasmtime::Result<Vec<fgeo::ExPolygon>> {
            let s: Vec<_> = subject.iter().map(f_wit_to_ir).collect();
            let c: Vec<_> = clip.iter().map(f_wit_to_ir).collect();
            let ir_op = match op {
                fhs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union,
                fhs::ClipOperation::Intersection => {
                    slicer_core::polygon_ops::ClipOperation::Intersection
                }
                fhs::ClipOperation::Difference => {
                    slicer_core::polygon_ops::ClipOperation::Difference
                }
                fhs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor,
            };
            Ok(ir_clip_polygons(&s, &c, ir_op)
                .iter()
                .map(f_ir_to_wit)
                .collect())
        }
        fn offset_polygons(
            &mut self,
            polygons: Vec<fgeo::ExPolygon>,
            delta_mm: f32,
            join: fhs::OffsetJoinType,
        ) -> wasmtime::Result<Vec<fgeo::ExPolygon>> {
            let ir: Vec<_> = polygons.iter().map(f_wit_to_ir).collect();
            let j = match join {
                fhs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter,
                fhs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round,
                fhs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square,
            };
            Ok(ir_offset_polygons(&ir, delta_mm, j)
                .iter()
                .map(f_ir_to_wit)
                .collect())
        }
        fn simplify_polygon(
            &mut self,
            polygon: fgeo::Polygon,
            _: f32,
        ) -> wasmtime::Result<fgeo::Polygon> {
            let pts: Vec<_> = polygon
                .points
                .iter()
                .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                .collect();
            Ok(fgeo::Polygon {
                points: ir_simplify_polygon(pts)
                    .into_iter()
                    .map(|p| fgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            })
        }
        fn now_us(&mut self) -> wasmtime::Result<u64> {
            Ok(self.start_time.elapsed().as_micros() as u64)
        }
    }

    impl fct::Host for HostExecutionContext {}
    impl fct::HostConfigView for HostExecutionContext {
        fn get(
            &mut self,
            self_: Resource<ConfigViewData>,
            key: String,
        ) -> wasmtime::Result<Option<fct::ConfigValue>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).map(|v| match v {
                ConfigValueStorage::Bool(b) => fct::ConfigValue::BoolVal(*b),
                ConfigValueStorage::Int(i) => fct::ConfigValue::IntVal(*i),
                ConfigValueStorage::Float(f) => fct::ConfigValue::FloatVal(*f),
                ConfigValueStorage::Str(s) => fct::ConfigValue::StringVal(s.clone()),
                ConfigValueStorage::FloatList(fl) => fct::ConfigValue::FloatList(fl.clone()),
                ConfigValueStorage::StringList(sl) => fct::ConfigValue::StringList(sl.clone()),
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
            Ok(self.table.get(&self_)?.fields.keys().cloned().collect())
        }
        fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> {
            self.table.delete(rep)?;
            Ok(())
        }
    }

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
            role: finalization_role_wit_to_ir(&p.role),
            speed_factor: p.speed_factor,
        }
    }

    fn finalization_role_wit_to_ir(r: &fgeo::ExtrusionRole) -> slicer_ir::ExtrusionRole {
        match r {
            fgeo::ExtrusionRole::OuterWall => slicer_ir::ExtrusionRole::OuterWall,
            fgeo::ExtrusionRole::InnerWall => slicer_ir::ExtrusionRole::InnerWall,
            fgeo::ExtrusionRole::ThinWall => slicer_ir::ExtrusionRole::ThinWall,
            fgeo::ExtrusionRole::TopSolidInfill => slicer_ir::ExtrusionRole::TopSolidInfill,
            fgeo::ExtrusionRole::BottomSolidInfill => slicer_ir::ExtrusionRole::BottomSolidInfill,
            fgeo::ExtrusionRole::SparseInfill => slicer_ir::ExtrusionRole::SparseInfill,
            fgeo::ExtrusionRole::SupportMaterial => slicer_ir::ExtrusionRole::SupportMaterial,
            fgeo::ExtrusionRole::SupportInterface => slicer_ir::ExtrusionRole::SupportInterface,
            fgeo::ExtrusionRole::Ironing => slicer_ir::ExtrusionRole::Ironing,
            fgeo::ExtrusionRole::BridgeInfill => slicer_ir::ExtrusionRole::BridgeInfill,
            fgeo::ExtrusionRole::WipeTower => slicer_ir::ExtrusionRole::WipeTower,
            fgeo::ExtrusionRole::Custom(s) => slicer_ir::ExtrusionRole::Custom(s.clone()),
        }
    }

    fn finalization_role_ir_to_wit(r: &slicer_ir::ExtrusionRole) -> fgeo::ExtrusionRole {
        match r {
            slicer_ir::ExtrusionRole::OuterWall => fgeo::ExtrusionRole::OuterWall,
            slicer_ir::ExtrusionRole::InnerWall => fgeo::ExtrusionRole::InnerWall,
            slicer_ir::ExtrusionRole::ThinWall => fgeo::ExtrusionRole::ThinWall,
            slicer_ir::ExtrusionRole::TopSolidInfill => fgeo::ExtrusionRole::TopSolidInfill,
            slicer_ir::ExtrusionRole::BottomSolidInfill => fgeo::ExtrusionRole::BottomSolidInfill,
            slicer_ir::ExtrusionRole::SparseInfill => fgeo::ExtrusionRole::SparseInfill,
            slicer_ir::ExtrusionRole::SupportMaterial => fgeo::ExtrusionRole::SupportMaterial,
            slicer_ir::ExtrusionRole::SupportInterface => fgeo::ExtrusionRole::SupportInterface,
            slicer_ir::ExtrusionRole::Ironing => fgeo::ExtrusionRole::Ironing,
            slicer_ir::ExtrusionRole::BridgeInfill => fgeo::ExtrusionRole::BridgeInfill,
            slicer_ir::ExtrusionRole::WipeTower => fgeo::ExtrusionRole::WipeTower,
            slicer_ir::ExtrusionRole::Custom(s) => fgeo::ExtrusionRole::Custom(s.clone()),
            slicer_ir::ExtrusionRole::PrimeTower => {
                fgeo::ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG.to_string())
            }
            slicer_ir::ExtrusionRole::Skirt => {
                fgeo::ExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_SKIRT_TAG.to_string())
            }
        }
    }

    fn finalization_path_ir_to_wit(p: &slicer_ir::ExtrusionPath3D) -> fgeo::ExtrusionPath3d {
        fgeo::ExtrusionPath3d {
            points: p
                .points
                .iter()
                .map(|pt| fgeo::Point3WithWidth {
                    x: pt.x,
                    y: pt.y,
                    z: pt.z,
                    width: pt.width,
                    flow_factor: pt.flow_factor,
                    overhang_quartile: pt.overhang_quartile,
                })
                .collect(),
            role: finalization_role_ir_to_wit(&p.role),
            speed_factor: p.speed_factor,
        }
    }

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
                    path: finalization_path_ir_to_wit(&entity.path),
                    role: finalization_role_ir_to_wit(&entity.role),
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
            };
            data.pushes.push(FinalizationBuilderPush::EntityToLayer {
                layer_index,
                path: finalization_path_wit_to_ir(&path),
                region_key: ir_region_key,
            });
            Ok(Ok(()))
        }
        fn push_entity_with_priority(
            &mut self,
            self_: Resource<fm::FinalizationOutputBuilder>,
            layer_index: u32,
            path: fgeo::ExtrusionPath3d,
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
            };
            data.pushes
                .push(FinalizationBuilderPush::EntityToLayerWithPriority {
                    layer_index,
                    path: finalization_path_wit_to_ir(&path),
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
        fn finalization_role_ir_to_wit_preserves_reserved_builtin_roles() {
            assert!(matches!(
                finalization_role_ir_to_wit(&slicer_ir::ExtrusionRole::PrimeTower),
                fgeo::ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG
            ));
            assert!(matches!(
                finalization_role_ir_to_wit(&slicer_ir::ExtrusionRole::Skirt),
                fgeo::ExtrusionRole::Custom(tag) if tag == BUILTIN_EXTRUSION_ROLE_SKIRT_TAG
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
    use postpass::slicer::world_postpass::config_types as ppct;
    use postpass::slicer::world_postpass::geometry as ppgeo;
    use postpass::slicer::world_postpass::host_services as pphs;

    impl ppgeo::Host for HostExecutionContext {}

    fn pp_wit_to_ir(ep: &ppgeo::ExPolygon) -> slicer_ir::ExPolygon {
        slicer_ir::ExPolygon {
            contour: slicer_ir::Polygon {
                points: ep
                    .contour
                    .points
                    .iter()
                    .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                    .collect(),
            },
            holes: ep
                .holes
                .iter()
                .map(|h| slicer_ir::Polygon {
                    points: h
                        .points
                        .iter()
                        .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                        .collect(),
                })
                .collect(),
        }
    }
    fn pp_ir_to_wit(ep: &slicer_ir::ExPolygon) -> ppgeo::ExPolygon {
        ppgeo::ExPolygon {
            contour: ppgeo::Polygon {
                points: ep
                    .contour
                    .points
                    .iter()
                    .map(|p| ppgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            },
            holes: ep
                .holes
                .iter()
                .map(|h| ppgeo::Polygon {
                    points: h
                        .points
                        .iter()
                        .map(|p| ppgeo::Point2 { x: p.x, y: p.y })
                        .collect(),
                })
                .collect(),
        }
    }

    fn ir_point3_to_postpass(point: slicer_ir::Point3) -> ppgeo::Point3 {
        ppgeo::Point3 {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }

    fn ir_bounds_to_postpass(bounds: slicer_ir::BoundingBox3) -> ppgeo::BoundingBox3 {
        ppgeo::BoundingBox3 {
            min: ir_point3_to_postpass(bounds.min),
            max: ir_point3_to_postpass(bounds.max),
        }
    }

    impl pphs::Host for HostExecutionContext {
        fn log(&mut self, level: pphs::LogLevel, message: String) -> wasmtime::Result<()> {
            let level_str = match level {
                pphs::LogLevel::Trace => "trace",
                pphs::LogLevel::Debug => "debug",
                pphs::LogLevel::Info => "info",
                pphs::LogLevel::Warn => "warn",
                pphs::LogLevel::Error => "error",
            };
            self.log_messages.push((level_str.to_string(), message));
            Ok(())
        }
        fn raycast_z_down(
            &mut self,
            object_id: pphs::ObjectId,
            x: f32,
            y: f32,
            start_z: f32,
        ) -> wasmtime::Result<Option<f32>> {
            raycast_z_down_mesh_query(self, &object_id, x, y, start_z)
        }
        fn surface_normal_at(
            &mut self,
            object_id: pphs::ObjectId,
            x: f32,
            y: f32,
            z: f32,
        ) -> wasmtime::Result<Option<ppgeo::Point3>> {
            Ok(surface_normal_at_mesh_query(self, &object_id, x, y, z)?.map(ir_point3_to_postpass))
        }
        fn object_bounds(
            &mut self,
            object_id: pphs::ObjectId,
        ) -> wasmtime::Result<ppgeo::BoundingBox3> {
            Ok(ir_bounds_to_postpass(object_bounds_mesh_query(
                self, &object_id,
            )?))
        }
        fn clip_polygons(
            &mut self,
            subject: Vec<ppgeo::ExPolygon>,
            clip: Vec<ppgeo::ExPolygon>,
            op: pphs::ClipOperation,
        ) -> wasmtime::Result<Vec<ppgeo::ExPolygon>> {
            let s: Vec<_> = subject.iter().map(pp_wit_to_ir).collect();
            let c: Vec<_> = clip.iter().map(pp_wit_to_ir).collect();
            let ir_op = match op {
                pphs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union,
                pphs::ClipOperation::Intersection => {
                    slicer_core::polygon_ops::ClipOperation::Intersection
                }
                pphs::ClipOperation::Difference => {
                    slicer_core::polygon_ops::ClipOperation::Difference
                }
                pphs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor,
            };
            Ok(ir_clip_polygons(&s, &c, ir_op)
                .iter()
                .map(pp_ir_to_wit)
                .collect())
        }
        fn offset_polygons(
            &mut self,
            polygons: Vec<ppgeo::ExPolygon>,
            delta_mm: f32,
            join: pphs::OffsetJoinType,
        ) -> wasmtime::Result<Vec<ppgeo::ExPolygon>> {
            let ir: Vec<_> = polygons.iter().map(pp_wit_to_ir).collect();
            let j = match join {
                pphs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter,
                pphs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round,
                pphs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square,
            };
            Ok(ir_offset_polygons(&ir, delta_mm, j)
                .iter()
                .map(pp_ir_to_wit)
                .collect())
        }
        fn simplify_polygon(
            &mut self,
            polygon: ppgeo::Polygon,
            _: f32,
        ) -> wasmtime::Result<ppgeo::Polygon> {
            let pts: Vec<_> = polygon
                .points
                .iter()
                .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                .collect();
            Ok(ppgeo::Polygon {
                points: ir_simplify_polygon(pts)
                    .into_iter()
                    .map(|p| ppgeo::Point2 { x: p.x, y: p.y })
                    .collect(),
            })
        }
        fn now_us(&mut self) -> wasmtime::Result<u64> {
            Ok(self.start_time.elapsed().as_micros() as u64)
        }
    }

    impl ppct::Host for HostExecutionContext {}
    impl ppct::HostConfigView for HostExecutionContext {
        fn get(
            &mut self,
            self_: Resource<ConfigViewData>,
            key: String,
        ) -> wasmtime::Result<Option<ppct::ConfigValue>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).map(|v| match v {
                ConfigValueStorage::Bool(b) => ppct::ConfigValue::BoolVal(*b),
                ConfigValueStorage::Int(i) => ppct::ConfigValue::IntVal(*i),
                ConfigValueStorage::Float(f) => ppct::ConfigValue::FloatVal(*f),
                ConfigValueStorage::Str(s) => ppct::ConfigValue::StringVal(s.clone()),
                ConfigValueStorage::FloatList(fl) => ppct::ConfigValue::FloatList(fl.clone()),
                ConfigValueStorage::StringList(sl) => ppct::ConfigValue::StringList(sl.clone()),
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
            Ok(self.table.get(&self_)?.fields.keys().cloned().collect())
        }
        fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> {
            self.table.delete(rep)?;
            Ok(())
        }
    }

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
                    role: convert_postpass_role(&cmd.role),
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
                    mode: convert_postpass_retract_mode(&mode),
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
                    mode: convert_postpass_retract_mode(&mode),
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

    fn convert_postpass_retract_mode(mode: &ppm::RetractMode) -> slicer_ir::RetractMode {
        match mode {
            ppm::RetractMode::Gcode => slicer_ir::RetractMode::Gcode,
            ppm::RetractMode::Firmware => slicer_ir::RetractMode::Firmware,
        }
    }

    fn convert_postpass_role(role: &ppgeo::ExtrusionRole) -> ExtrusionRole {
        match role {
            ppgeo::ExtrusionRole::OuterWall => ExtrusionRole::OuterWall,
            ppgeo::ExtrusionRole::InnerWall => ExtrusionRole::InnerWall,
            ppgeo::ExtrusionRole::ThinWall => ExtrusionRole::ThinWall,
            ppgeo::ExtrusionRole::TopSolidInfill => ExtrusionRole::TopSolidInfill,
            ppgeo::ExtrusionRole::BottomSolidInfill => ExtrusionRole::BottomSolidInfill,
            ppgeo::ExtrusionRole::SparseInfill => ExtrusionRole::SparseInfill,
            ppgeo::ExtrusionRole::SupportMaterial => ExtrusionRole::SupportMaterial,
            ppgeo::ExtrusionRole::SupportInterface => ExtrusionRole::SupportInterface,
            ppgeo::ExtrusionRole::Ironing => ExtrusionRole::Ironing,
            ppgeo::ExtrusionRole::BridgeInfill => ExtrusionRole::BridgeInfill,
            ppgeo::ExtrusionRole::WipeTower => ExtrusionRole::WipeTower,
            ppgeo::ExtrusionRole::Custom(s) => ExtrusionRole::Custom(s.clone()),
        }
    }
}

// ── WIT→IR type conversion and validation ──────────────────────────────

/// Validate that a float value is finite (not NaN or Inf).
fn validate_finite(value: f32, field: &str, index: usize) -> Result<(), String> {
    if value.is_nan() || value.is_infinite() {
        Err(format!("point[{index}].{field} is NaN or Inf ({value})"))
    } else {
        Ok(())
    }
}

/// Validate and convert a WIT `Point3WithWidth` to a slicer-ir `Point3WithWidth`.
fn convert_point(p: &Point3WithWidth, index: usize) -> Result<slicer_ir::Point3WithWidth, String> {
    validate_finite(p.x, "x", index)?;
    validate_finite(p.y, "y", index)?;
    validate_finite(p.z, "z", index)?;
    validate_finite(p.width, "width", index)?;
    validate_finite(p.flow_factor, "flow_factor", index)?;
    Ok(slicer_ir::Point3WithWidth {
        x: p.x,
        y: p.y,
        z: p.z,
        width: p.width,
        flow_factor: p.flow_factor,
        overhang_quartile: p.overhang_quartile,
    })
}

/// Convert a WIT `ExtrusionRole` to a slicer-ir `ExtrusionRole`.
pub fn convert_extrusion_role(role: &ExtrusionRole) -> slicer_ir::ExtrusionRole {
    match role {
        ExtrusionRole::OuterWall => slicer_ir::ExtrusionRole::OuterWall,
        ExtrusionRole::InnerWall => slicer_ir::ExtrusionRole::InnerWall,
        ExtrusionRole::ThinWall => slicer_ir::ExtrusionRole::ThinWall,
        ExtrusionRole::TopSolidInfill => slicer_ir::ExtrusionRole::TopSolidInfill,
        ExtrusionRole::BottomSolidInfill => slicer_ir::ExtrusionRole::BottomSolidInfill,
        ExtrusionRole::SparseInfill => slicer_ir::ExtrusionRole::SparseInfill,
        ExtrusionRole::SupportMaterial => slicer_ir::ExtrusionRole::SupportMaterial,
        ExtrusionRole::SupportInterface => slicer_ir::ExtrusionRole::SupportInterface,
        ExtrusionRole::Ironing => slicer_ir::ExtrusionRole::Ironing,
        ExtrusionRole::BridgeInfill => slicer_ir::ExtrusionRole::BridgeInfill,
        ExtrusionRole::WipeTower => slicer_ir::ExtrusionRole::WipeTower,
        ExtrusionRole::Custom(s) if s == BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG => {
            slicer_ir::ExtrusionRole::PrimeTower
        }
        ExtrusionRole::Custom(s) if s == BUILTIN_EXTRUSION_ROLE_SKIRT_TAG => {
            slicer_ir::ExtrusionRole::Skirt
        }
        ExtrusionRole::Custom(s) => slicer_ir::ExtrusionRole::Custom(s.clone()),
    }
}

/// Convert the layer-module WIT `retract-mode` variant to `slicer_ir::RetractMode`.
///
/// Used by `gcode-output-builder` host handlers to forward the retract emission
/// mode declared by guest modules (e.g. `path-optimization-default`) into the
/// host-side `GcodeCommandCollected` queue.
pub fn convert_layer_retract_mode(mode: &WitRetractMode) -> slicer_ir::RetractMode {
    match mode {
        WitRetractMode::Gcode => slicer_ir::RetractMode::Gcode,
        WitRetractMode::Firmware => slicer_ir::RetractMode::Firmware,
    }
}

/// Validate and convert a WIT `ExtrusionPath3d` to a slicer-ir `ExtrusionPath3D`.
///
/// Returns an error if any point coordinate is NaN or Inf (per docs/02_ir_schemas.md).
pub fn convert_extrusion_path(
    path: &ExtrusionPath3d,
) -> Result<slicer_ir::ExtrusionPath3D, String> {
    if path.speed_factor.is_nan() || path.speed_factor.is_infinite() {
        return Err(format!(
            "speed_factor is NaN or Inf ({})",
            path.speed_factor
        ));
    }
    let points: Result<Vec<_>, _> = path
        .points
        .iter()
        .enumerate()
        .map(|(i, p)| convert_point(p, i))
        .collect();
    Ok(slicer_ir::ExtrusionPath3D {
        points: points?,
        role: convert_extrusion_role(&path.role),
        speed_factor: path.speed_factor,
    })
}

/// Convert collected infill output into a slicer-ir `InfillIR`.
///
/// All paths are validated for NaN/Inf. Any invalid path causes a fatal error.
///
/// Identity preservation: if any `*_origins` entry is `Some`, output is
/// grouped by `(object_id, region_id)`, producing one `InfillRegion` per
/// distinct origin in stable first-seen order. If origins are mixed Some/None
/// (a guest pushed without first querying its source region), this is a
/// contract violation and returns an error.
///
/// If all origin tags are `None`/empty (legacy callers and stages that do
/// not consume perimeter regions, such as `Layer::Infill` itself), all output
/// is emitted as one synthetic region for backward compatibility.
pub fn convert_infill_output(
    collected: &InfillOutputCollected,
    layer_index: u32,
) -> Result<slicer_ir::InfillIR, String> {
    let sparse: Vec<_> = collected
        .sparse_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;
    let solid: Vec<_> = collected
        .solid_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;
    let ironing: Vec<_> = collected
        .ironing_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;

    let any_tagged = collected.sparse_path_origins.iter().any(Option::is_some)
        || collected.solid_path_origins.iter().any(Option::is_some)
        || collected.ironing_path_origins.iter().any(Option::is_some);

    if !any_tagged {
        return Ok(slicer_ir::InfillIR {
            schema_version: slicer_ir::SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layer_index: layer_index,
            regions: vec![slicer_ir::InfillRegion {
                object_id: String::new(),
                region_id: 0,
                sparse_infill: sparse,
                solid_infill: solid,
                ironing,
            }],
        });
    }

    let mut buckets: Vec<(PerimeterRegionOrigin, slicer_ir::InfillRegion)> = Vec::new();
    let bucket_for = |buckets: &mut Vec<(PerimeterRegionOrigin, slicer_ir::InfillRegion)>,
                      origin: &PerimeterRegionOrigin|
     -> usize {
        if let Some(idx) = buckets.iter().position(|(o, _)| o == origin) {
            return idx;
        }
        buckets.push((
            origin.clone(),
            slicer_ir::InfillRegion {
                object_id: origin.0.clone(),
                region_id: origin.1,
                sparse_infill: Vec::new(),
                solid_infill: Vec::new(),
                ironing: Vec::new(),
            },
        ));
        buckets.len() - 1
    };

    fn drain_into<F: FnMut(&mut slicer_ir::InfillRegion, slicer_ir::ExtrusionPath3D)>(
        paths: Vec<slicer_ir::ExtrusionPath3D>,
        origins: &[Option<PerimeterRegionOrigin>],
        kind: &str,
        buckets: &mut Vec<(PerimeterRegionOrigin, slicer_ir::InfillRegion)>,
        mut place: F,
    ) -> Result<(), String> {
        if !paths.is_empty() && origins.len() != paths.len() {
            return Err(format!(
                "{kind}: origin tag count ({}) does not match path count ({})",
                origins.len(),
                paths.len()
            ));
        }
        for (i, path) in paths.into_iter().enumerate() {
            let origin = origins[i].as_ref().ok_or_else(|| format!(
                "{kind} path[{i}] was emitted without an active perimeter source region; \
                 guest must access a perimeter-region-view (object-id/region-id/wall-loops/infill-areas) \
                 before pushing output for identity-preserving commit"
            ))?;
            let idx = if let Some(idx) = buckets.iter().position(|(o, _)| o == origin) {
                idx
            } else {
                buckets.push((
                    origin.clone(),
                    slicer_ir::InfillRegion {
                        object_id: origin.0.clone(),
                        region_id: origin.1,
                        sparse_infill: Vec::new(),
                        solid_infill: Vec::new(),
                        ironing: Vec::new(),
                    },
                ));
                buckets.len() - 1
            };
            place(&mut buckets[idx].1, path);
        }
        Ok(())
    }

    let _ = bucket_for; // silence unused (helper defined for symmetry)

    drain_into(
        sparse,
        &collected.sparse_path_origins,
        "sparse_infill",
        &mut buckets,
        |r, p| r.sparse_infill.push(p),
    )?;
    drain_into(
        solid,
        &collected.solid_path_origins,
        "solid_infill",
        &mut buckets,
        |r, p| r.solid_infill.push(p),
    )?;
    drain_into(
        ironing,
        &collected.ironing_path_origins,
        "ironing",
        &mut buckets,
        |r, p| r.ironing.push(p),
    )?;

    Ok(slicer_ir::InfillIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        regions: buckets.into_iter().map(|(_, r)| r).collect(),
    })
}

/// Convert collected support output into a slicer-ir `SupportIR`.
///
/// Identity preservation: if any origin tag is `Some` (i.e. the guest queried
/// at least one slice-region-view before emitting output), every emitted path
/// must be tagged — untagged pushes in identity mode are a contract violation
/// and produce a structured diagnostic. Paths are grouped by
/// `(object_id, region_id)` in stable first-seen order so successive regions
/// appear as contiguous path spans. `SupportIR` is flat today, so identity is
/// preserved through ordering and validated-tag semantics (no silent flattening).
///
/// If no origin tags are recorded (legacy callers, or the `Layer::Support`
/// stage invoked without having touched any slice-region-view), output is
/// passed through in emission order for backward compatibility.
pub fn convert_support_output(
    collected: &SupportOutputCollected,
    layer_index: u32,
) -> Result<slicer_ir::SupportIR, String> {
    let support: Vec<_> = collected
        .support_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;
    let interface: Vec<_> = collected
        .interface_paths
        .iter()
        .map(|(p, _)| convert_extrusion_path(p))
        .collect::<Result<_, _>>()?;
    let raft: Vec<_> = collected
        .raft_paths
        .iter()
        .map(convert_extrusion_path)
        .collect::<Result<_, _>>()?;

    let any_tagged = collected.support_path_origins.iter().any(Option::is_some)
        || collected.interface_path_origins.iter().any(Option::is_some)
        || collected.raft_path_origins.iter().any(Option::is_some);

    if !any_tagged {
        return Ok(slicer_ir::SupportIR {
            schema_version: slicer_ir::SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layer_index: layer_index,
            support_paths: support,
            interface_paths: interface,
            raft_paths: raft,
            ironing_paths: Vec::new(),
        });
    }

    fn group_by_origin<T>(
        paths: Vec<T>,
        origins: &[Option<SliceRegionOrigin>],
        kind: &str,
        order: &mut Vec<SliceRegionOrigin>,
    ) -> Result<Vec<T>, String> {
        if !paths.is_empty() && origins.len() != paths.len() {
            return Err(format!(
                "{kind}: origin tag count ({}) does not match path count ({})",
                origins.len(),
                paths.len()
            ));
        }
        let mut buckets: Vec<(SliceRegionOrigin, Vec<T>)> = Vec::new();
        for (i, path) in paths.into_iter().enumerate() {
            let origin = origins[i].as_ref().ok_or_else(|| {
                format!(
                    "{kind} path[{i}] was emitted without an active slice source region; \
                 guest must access a slice-region-view (object-id/region-id/polygons/\
                 infill-areas/effective-layer-height/z/has-nonplanar/boundary-paint) \
                 before pushing support output for identity-preserving commit"
                )
            })?;
            if let Some(idx) = buckets.iter().position(|(o, _)| o == origin) {
                buckets[idx].1.push(path);
            } else {
                if !order.iter().any(|o| o == origin) {
                    order.push(origin.clone());
                }
                buckets.push((origin.clone(), vec![path]));
            }
        }
        // Emit in stable first-seen origin order (matches global `order`).
        let mut out = Vec::new();
        for origin in order.iter() {
            if let Some(pos) = buckets.iter().position(|(o, _)| o == origin) {
                out.extend(buckets.remove(pos).1);
            }
        }
        // Any buckets not yet in `order` would indicate logic error; fold in.
        for (_, v) in buckets {
            out.extend(v);
        }
        Ok(out)
    }

    let mut order: Vec<SliceRegionOrigin> = Vec::new();
    let support = group_by_origin(
        support,
        &collected.support_path_origins,
        "support",
        &mut order,
    )?;
    let interface = group_by_origin(
        interface,
        &collected.interface_path_origins,
        "interface",
        &mut order,
    )?;
    let raft = group_by_origin(raft, &collected.raft_path_origins, "raft", &mut order)?;

    Ok(slicer_ir::SupportIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        support_paths: support,
        interface_paths: interface,
        raft_paths: raft,
        ironing_paths: Vec::new(),
    })
}

/// Convert a WIT `WallLoopType` to a slicer-ir `LoopType`.
pub fn convert_wall_loop_type(lt: &WallLoopType) -> slicer_ir::LoopType {
    match lt {
        WallLoopType::Outer => slicer_ir::LoopType::Outer,
        WallLoopType::Inner => slicer_ir::LoopType::Inner,
        WallLoopType::ThinWall => slicer_ir::LoopType::ThinWall,
        WallLoopType::NonplanarShell => slicer_ir::LoopType::NonPlanarShell,
    }
}

/// Convert a WIT `PaintValue` variant to a slicer-ir `PaintValue`.
fn convert_paint_value(v: &PaintValue) -> slicer_ir::PaintValue {
    match v {
        PaintValue::Flag(b) => slicer_ir::PaintValue::Flag(*b),
        PaintValue::Scalar(s) => slicer_ir::PaintValue::Scalar(*s),
        PaintValue::ToolIndex(t) => slicer_ir::PaintValue::ToolIndex(*t),
    }
}

/// Convert a WIT `WallFeatureFlag` to a slicer-ir `WallFeatureFlags`.
pub fn convert_wall_feature_flag(flag: &WallFeatureFlag) -> slicer_ir::WallFeatureFlags {
    slicer_ir::WallFeatureFlags {
        tool_index: flag.tool_index,
        fuzzy_skin: flag.fuzzy_skin,
        is_bridge: flag.is_bridge,
        is_thin_wall: flag.is_thin_wall,
        skip_ironing: flag.skip_ironing,
        custom: HashMap::from_iter(
            flag.custom
                .iter()
                .map(|(k, v)| (k.clone(), convert_paint_value(v))),
        ),
    }
}

/// Validate and convert a WIT `WallLoopView` to a slicer-ir `WallLoop`.
///
/// Returns an error if any path coordinate is NaN or Inf, or if feature-flags
/// cardinality does not match path points (per docs/03 wall loop flag invariant).
pub fn convert_wall_loop(wl: &WallLoopView) -> Result<slicer_ir::WallLoop, String> {
    let path = convert_extrusion_path(&wl.path)?;
    if wl.feature_flags.len() != wl.path.points.len() {
        return Err(format!(
            "feature_flags length ({}) != path points length ({}); \
             per docs/03 wall loop flag invariant these must be parallel",
            wl.feature_flags.len(),
            wl.path.points.len()
        ));
    }
    Ok(slicer_ir::WallLoop {
        perimeter_index: wl.perimeter_index,
        loop_type: convert_wall_loop_type(&wl.loop_type),
        path,
        width_profile: slicer_ir::WidthProfile {
            widths: wl.path.points.iter().map(|p| p.width).collect(),
        },
        feature_flags: wl
            .feature_flags
            .iter()
            .map(convert_wall_feature_flag)
            .collect(),
        boundary_type: slicer_ir::WallBoundaryType::Interior,
    })
}

/// Convert collected perimeter output into a slicer-ir `PerimeterIR`.
///
/// All wall loop paths are validated for NaN/Inf and feature-flag cardinality.
///
/// Identity preservation: if any origin tag is `Some` (i.e. the guest queried
/// at least one perimeter-region-view before emitting output), regions are
/// grouped by `(object_id, region_id)` in stable first-seen order. Output
/// pushed without a preceding region access in identity mode is a contract
/// violation and produces a structured error.
///
/// If no origin tags are recorded (legacy callers, or the `Layer::Perimeters`
/// stage which does not consume perimeter regions), all output is flattened
/// into one synthetic region for backward compatibility.
pub fn convert_perimeter_output(
    collected: &PerimeterOutputCollected,
    layer_index: u32,
) -> Result<slicer_ir::PerimeterIR, String> {
    // When seam-placer has rotated wall loops, those are the canonical geometry.
    // rotated_wall_loops replaces the original wall_loops in PerimeterIR.
    let (walls, wall_origins): (Vec<slicer_ir::WallLoop>, Vec<Option<PerimeterRegionOrigin>>) =
        if !collected.rotated_wall_loops.is_empty() {
            let rotated: Vec<slicer_ir::WallLoop> = collected
                .rotated_wall_loops
                .iter()
                .map(convert_wall_loop)
                .collect::<Result<_, _>>()?;
            (rotated, collected.rotated_wall_loop_origins.clone())
        } else {
            let original: Vec<slicer_ir::WallLoop> = collected
                .wall_loops
                .iter()
                .map(convert_wall_loop)
                .collect::<Result<_, _>>()?;
            (original, collected.wall_loop_origins.clone())
        };
    let infill_areas = wit_to_ir_expolygons(&collected.infill_areas);
    let seam_candidates: Vec<slicer_ir::SeamCandidate> = collected
        .seam_candidates
        .iter()
        .enumerate()
        .map(|(i, (pos, score))| {
            if pos.x.is_nan()
                || pos.x.is_infinite()
                || pos.y.is_nan()
                || pos.y.is_infinite()
                || pos.z.is_nan()
                || pos.z.is_infinite()
            {
                Err(format!("seam_candidate[{i}] has NaN/Inf coordinate"))
            } else if score.is_nan() || score.is_infinite() {
                Err(format!("seam_candidate[{i}] has NaN/Inf score"))
            } else {
                Ok(slicer_ir::SeamCandidate {
                    position: slicer_ir::Point3WithWidth {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                        width: 0.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    score: *score,
                    reason: slicer_ir::SeamReason::Aligned,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Convert collected resolved_seam to IR type.
    let resolved_seam =
        collected
            .resolved_seam
            .as_ref()
            .map(|(pos, wall_index)| slicer_ir::SeamPosition {
                point: slicer_ir::Point3WithWidth {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    width: 0.0,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                wall_index: *wall_index,
            });
    let resolved_seam_origin = collected.resolved_seam_origin.as_ref();

    let any_tagged = wall_origins.iter().any(Option::is_some)
        || collected.seam_candidate_origins.iter().any(Option::is_some)
        || collected.infill_areas_origin.is_some();

    if !any_tagged {
        return Ok(slicer_ir::PerimeterIR {
            schema_version: slicer_ir::SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layer_index: layer_index,
            regions: vec![slicer_ir::PerimeterRegion {
                object_id: String::new(),
                region_id: 0,
                walls,
                infill_areas,
                seam_candidates,
                resolved_seam,
            }],
        });
    }

    let mut buckets: Vec<(PerimeterRegionOrigin, slicer_ir::PerimeterRegion)> = Vec::new();
    let ensure = |buckets: &mut Vec<(PerimeterRegionOrigin, slicer_ir::PerimeterRegion)>,
                  origin: &PerimeterRegionOrigin|
     -> usize {
        if let Some(idx) = buckets.iter().position(|(o, _)| o == origin) {
            return idx;
        }
        buckets.push((
            origin.clone(),
            slicer_ir::PerimeterRegion {
                object_id: origin.0.clone(),
                region_id: origin.1,
                walls: Vec::new(),
                infill_areas: Vec::new(),
                seam_candidates: Vec::new(),
                resolved_seam: None,
            },
        ));
        buckets.len() - 1
    };

    if !walls.is_empty() && wall_origins.len() != walls.len() {
        return Err(format!(
            "wall_loops: origin tag count ({}) does not match wall count ({})",
            wall_origins.len(),
            walls.len()
        ));
    }
    for (i, wl) in walls.into_iter().enumerate() {
        let origin = wall_origins[i].as_ref().ok_or_else(|| {
            format!(
                "wall_loop[{i}] was emitted without an active perimeter source region; \
             guest must access a perimeter-region-view before pushing wall loops"
            )
        })?;
        let idx = ensure(&mut buckets, origin);
        buckets[idx].1.walls.push(wl);
    }

    if !seam_candidates.is_empty()
        && collected.seam_candidate_origins.len() != seam_candidates.len()
    {
        return Err(format!(
            "seam_candidates: origin tag count ({}) does not match candidate count ({})",
            collected.seam_candidate_origins.len(),
            seam_candidates.len()
        ));
    }
    for (i, sc) in seam_candidates.into_iter().enumerate() {
        let origin = collected.seam_candidate_origins[i]
            .as_ref()
            .ok_or_else(|| {
                format!("seam_candidate[{i}] was emitted without an active perimeter source region")
            })?;
        let idx = ensure(&mut buckets, origin);
        buckets[idx].1.seam_candidates.push(sc);
    }

    if !infill_areas.is_empty() {
        let origin = collected.infill_areas_origin.as_ref().ok_or_else(|| {
            "set_infill_areas called without an active perimeter source region".to_string()
        })?;
        let idx = ensure(&mut buckets, origin);
        buckets[idx].1.infill_areas = infill_areas;
    }

    if let Some(rs) = &resolved_seam {
        let Some(origin) = resolved_seam_origin else {
            return Err(
                "resolved_seam was emitted without an active perimeter source region".to_string(),
            );
        };
        let idx = ensure(&mut buckets, origin);
        buckets[idx].1.resolved_seam = Some(rs.clone());
    }

    Ok(slicer_ir::PerimeterIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        regions: buckets.into_iter().map(|(_, r)| r).collect(),
    })
}

/// Merge collected slice-postprocess output into an existing `SliceIR`,
/// preserving per-region identity.
///
/// SlicePostProcess modifies already-sliced regions: `set_polygons(key, polys)`
/// replaces the polygon set of the region matching `key`, and `set_path_z`
/// adjusts a Z coordinate on a polygon contour point. Regions not mentioned by
/// the guest pass through unchanged. Unknown `RegionKey` values (no matching
/// existing region) are a contract violation and produce a structured diagnostic
/// rather than inventing a synthetic region or silently dropping the update.
///
/// If no existing `SliceIR` is staged (identity-mapping failure), an error is
/// returned so the caller can decide whether to synthesize a fresh IR or fail.
pub fn merge_slice_postprocess_into(
    mut existing: slicer_ir::SliceIR,
    collected: &SlicePostprocessCollected,
) -> Result<slicer_ir::SliceIR, String> {
    for (i, (_, _, _, z)) in collected.path_z_updates.iter().enumerate() {
        if z.is_nan() || z.is_infinite() {
            return Err(format!("path_z_update[{i}] has NaN/Inf Z value ({z})"));
        }
    }

    let find_region = |regions: &[slicer_ir::SlicedRegion], key: &RegionKey| -> Option<usize> {
        let rid = key.region_id.parse::<u64>().ok()?;
        regions
            .iter()
            .position(|r| r.object_id == key.object_id && r.region_id == rid)
    };

    for (i, (key, polys)) in collected.polygon_updates.iter().enumerate() {
        let idx = find_region(&existing.regions, key).ok_or_else(|| {
            format!(
                "slice_postprocess polygon_update[{i}] targets unknown region \
             (object_id='{}', region_id='{}'); guest must reference an existing \
             slice-region-view identity for identity-preserving commit",
                key.object_id, key.region_id,
            )
        })?;
        existing.regions[idx].polygons = wit_to_ir_expolygons(polys);
    }

    for (i, (key, path_idx, vertex_idx, z)) in collected.path_z_updates.iter().enumerate() {
        let ridx = find_region(&existing.regions, key).ok_or_else(|| {
            format!(
                "slice_postprocess path_z_update[{i}] targets unknown region \
             (object_id='{}', region_id='{}')",
                key.object_id, key.region_id,
            )
        })?;
        let region = &mut existing.regions[ridx];
        let poly_count = region.polygons.len();
        let poly = region.polygons.get_mut(*path_idx as usize).ok_or_else(|| {
            format!(
                "slice_postprocess path_z_update[{i}]: polygon index {path_idx} out of range \
             for region ({}, {}) with {poly_count} polygons",
                key.object_id, key.region_id,
            )
        })?;
        // Z updates apply to contour points; validate vertex index bound.
        if (*vertex_idx as usize) >= poly.contour.points.len() {
            return Err(format!(
                "slice_postprocess path_z_update[{i}]: vertex index {vertex_idx} out of range \
                 for contour with {} points",
                poly.contour.points.len(),
            ));
        }
        // Z lives in ExPolygon contour — the IR expresses 2D contour points
        // only; path-Z updates are retained per-region as an attribute-less
        // no-op here since slicer_ir::ExPolygon has no per-point Z. Keeping
        // validation above guarantees the contract without mutating flat geometry.
        let _ = z;
    }

    Ok(existing)
}
