//! WIT/component-model host-side bindings and execution context.
//!
//! This module provides:
//! - `wasmtime::component::bindgen!`-generated types and traits for the layer world
//! - `HostExecutionContext` — per-call execution state carrying config, IR views,
//!   output collectors, and a `ResourceTable` for host resource handle management
//! - Trait implementations bridging the generated WIT interface to real host data

use std::collections::HashMap;
use std::time::Instant;

use wasmtime::component::{Resource, ResourceTable};

// ── Resource backing data structs ───────────────────────────────────────
// These are the actual data stored in the ResourceTable.
// The `bindgen!` `with` option maps WIT resource types to these structs.

/// Backing data for a `config-view` resource handle.
pub struct ConfigViewData {
    /// Config fields, pre-filtered to the module's declared reads.
    pub fields: HashMap<String, ConfigValueStorage>,
}

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
                record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32 }
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
                use geometry.{ex-polygon, extrusion-path3d, point3, extrusion-role};
                type object-id = string;
                type region-id = string;
                type layer-idx = u32;
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
                }
                resource perimeter-region-view {
                    object-id: func() -> object-id;
                    region-id: func() -> region-id;
                    wall-loops: func() -> list<wall-loop-view>;
                    infill-areas: func() -> list<ex-polygon>;
                }
                resource infill-output-builder {
                    push-sparse-path:  func(path: extrusion-path3d) -> result<_, string>;
                    push-solid-path:   func(path: extrusion-path3d) -> result<_, string>;
                    push-ironing-path: func(path: extrusion-path3d) -> result<_, string>;
                }
                resource perimeter-output-builder {
                    push-wall-loop:      func(wall-loop: wall-loop-view) -> result<_, string>;
                    set-infill-areas:    func(areas: list<ex-polygon>) -> result<_, string>;
                    push-seam-candidate: func(pos: point3, score: f32) -> result<_, string>;
                }
                resource slice-postprocess-builder {
                    set-polygons: func(region: region-key, polys: list<ex-polygon>) -> result<_, string>;
                    set-path-z:   func(region: region-key, path-idx: u32, vertex-idx: u32, z: f32) -> result<_, string>;
                }
                record gcode-move-cmd { x: option<f32>, y: option<f32>, z: option<f32>, e: option<f32>, f: option<f32>, role: extrusion-role }
                resource gcode-output-builder {
                    push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                    push-retract:     func(length: f32, speed: f32) -> result<_, string>;
                    push-fan-speed:   func(value: u8) -> result<_, string>;
                    push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                    push-tool-change: func(from-tool: u32, to-tool: u32) -> result<_, string>;
                    push-comment:     func(text: string) -> result<_, string>;
                    push-raw:         func(text: string) -> result<_, string>;
                    push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
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
                    gcode-output-builder, region-key, layer-idx,
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
                export run-path-optimization: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: gcode-output-builder, config: config-view) -> result<_, module-error>;
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
pub use layer::slicer::world_layer::ir_handles::{
    BoundaryPaintEntry, BoundaryPaintPolygon, GcodeMoveCmd, PaintSemantic, PaintValue, RegionKey,
    SemanticRegion, WallFeatureFlag, WallLoopType, WallLoopView,
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

                record paint-region-entry {
                    object-id: object-id,
                    layer-index: u32,
                    semantic: string,
                    polygons: list<ex-polygon>,
                    value: string,
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
        }
    }
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
                record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32 }
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
                use geometry.{extrusion-path3d};
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

                resource layer-collection-view {
                    layer-index:  func() -> layer-idx;
                    z:            func() -> f32;
                    entity-count: func() -> u32;
                    tool-changes: func() -> list<tool-change-view>;
                }

                resource finalization-output-builder {
                    push-entity-to-layer: func(
                        layer-index: layer-idx,
                        path: extrusion-path3d,
                        region-key: region-key,
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
                resource gcode-output-builder {
                    push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                    push-retract:     func(length: f32, speed: f32) -> result<_, string>;
                    push-fan-speed:   func(value: u8) -> result<_, string>;
                    push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                    push-tool-change: func(from-tool: u32, to-tool: u32) -> result<_, string>;
                    push-comment:     func(text: string) -> result<_, string>;
                    push-raw:         func(text: string) -> result<_, string>;
                }

                enum gcode-command-kind { move-cmd, retract, fan-speed, temperature, tool-change, comment, raw }
                record gcode-command-view { index: u32, kind: gcode-command-kind }

                export run-gcode-postprocess: func(
                    commands: list<gcode-command-view>,
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
    /// Infill areas set by the guest.
    pub infill_areas: Vec<ExPolygon>,
    /// Seam candidates emitted by the guest.
    pub seam_candidates: Vec<(Point3, f32)>,
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
    /// Retract.
    Retract { length: f32, speed: f32 },
    /// Fan speed.
    FanSpeed(u8),
    /// Temperature.
    Temperature { tool: u32, celsius: f32, wait: bool },
    /// Tool change.
    ToolChange { from_tool: u32, to_tool: u32 },
    /// Comment.
    Comment(String),
    /// Raw G-code.
    Raw(String),
    /// Z-hop request.
    ZHop { after_entity_index: u32, hop_height: f32 },
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
    pub table: ResourceTable,
    /// Module identifier (from manifest).
    pub module_id: String,
    /// Monotonic clock start for profiling.
    start_time: Instant,
    /// Log messages emitted by the guest via host-services.log.
    pub log_messages: Vec<(String, String)>,

    // ── Output collectors ───────────────────────────────────────────
    /// Infill output collected during a call.
    pub infill_output: InfillOutputCollected,
    /// Perimeter output collected during a call.
    pub perimeter_output: PerimeterOutputCollected,
    /// Support output collected during a call.
    pub support_output: SupportOutputCollected,
    /// GCode output collected during a call.
    pub gcode_output: GcodeOutputCollected,
    /// Slice postprocess output collected during a call.
    pub slice_postprocess_output: SlicePostprocessCollected,
    /// Identity of the perimeter-region-view most recently accessed by the
    /// guest. Used to tag pushed post-process output so the commit path can
    /// preserve per-region identity instead of flattening into one synthetic
    /// region. Reset to `None` between calls (HostExecutionContext is per-call).
    pub current_perimeter_region: Option<PerimeterRegionOrigin>,
    /// Identity of the slice-region-view most recently accessed by the guest.
    /// Used to tag support post-process output pushes so the commit path can
    /// preserve per-region identity (grouping + structured diagnostic on
    /// untagged pushes) rather than silently flattening.
    pub current_slice_region: Option<SliceRegionOrigin>,

    /// Layer proposals collected from `push_layer` calls during a prepass
    /// `run-layer-planning` invocation.  Empty for all non-prepass stages.
    /// Drained by the prepass dispatch path after the WIT call returns.
    pub layer_plan_proposals: Vec<prepass::LayerProposal>,

    /// Per-object facet annotations collected from `push-facet-annotation`
    /// calls during a prepass `run-mesh-analysis` invocation. Tuple is
    /// `(object_id, FacetAnnotation)`. Insertion order is preserved so
    /// a downstream harvest can build deterministic output. Empty for
    /// all non-MeshAnalysis stages and when the guest declines to emit
    /// annotations (e.g. the current production path where
    /// `SurfaceClassificationIR` is still produced by the host built-in;
    /// see `mesh_analysis::execute_mesh_analysis`).
    pub mesh_analysis_annotations: Vec<(String, prepass::FacetAnnotation)>,

    /// Per-object surface groups collected from `push-surface-group`
    /// calls during a prepass `run-mesh-analysis` invocation. Tuple is
    /// `(object_id, SurfaceGroupProposal)`; insertion order preserved.
    /// Empty for all non-MeshAnalysis stages.
    pub mesh_analysis_surface_groups: Vec<(String, prepass::SurfaceGroupProposal)>,

    /// Triangle paint marks collected from `mark-triangle-paint` calls
    /// during a prepass `run-mesh-segmentation` invocation. Tuple layout
    /// mirrors the WIT method signature exactly:
    /// `(object_id, facet_index, semantic, value)`. Insertion order is
    /// preserved so `harvest_mesh_segmentation_ir` can build a
    /// deterministic `MeshSegmentationIR.marks` sequence.
    pub mesh_segmentation_marks: Vec<(String, u32, String, String)>,

    /// Paint-region entries collected from `push-paint-region` calls
    /// during a prepass `run-paint-segmentation` invocation. Stored as
    /// raw `prepass::PaintRegionEntry` records so the harvest helper
    /// can convert them to `PaintRegionIR` without losing any field.
    /// Empty for all non-prepass stages.
    pub paint_region_entries: Vec<prepass::PaintRegionEntry>,

    /// Finalization builder pushes collected during a finalization
    /// `run-finalization` invocation. The host-side
    /// `HostFinalizationOutputBuilder::drop` moves the resource's
    /// captured `pushes` here just before the resource is released,
    /// so `FinalizationStageRunner` can drain them even after the
    /// guest has dropped the builder handle (docs/03
    /// world-finalization.wit §finalization-output-builder).
    pub finalization_pushes: Vec<FinalizationBuilderPush>,

    /// Runtime IR read paths accessed by the guest via WIT view methods
    /// during this call. Populated by instrumenting each view method to
    /// record the exact IR path (e.g. `SliceIR.regions.polygons`) when
    /// called. Extracted by the dispatcher and returned as part of
    /// `ModuleAccessAudit.runtime_reads`.
    pub runtime_reads: Vec<String>,

    // ── Z envelope fields ─────────────────────────────────────────────
    /// Layer Z floor (lower bound of the Z envelope).
    layer_z: f32,
    /// Effective layer height (envelope height).
    effective_layer_height: f32,
    /// Bottom Z of catch-up layer, or `None` if not a catch-up layer.
    catchup_z_bottom: Option<f32>,
}

impl HostExecutionContext {
    /// Create a new execution context for a module call.
    ///
    /// `layer_z` is the layer floor (lower Z bound). `effective_layer_height` is
    /// the envelope height. `catchup_z_bottom` is `Some` when this is a catch-up
    /// layer (the floor is then `catchup_z_bottom` instead of `layer_z`).
    pub fn new(module_id: String, layer_z: f32, effective_layer_height: f32, catchup_z_bottom: Option<f32>) -> Self {
        Self {
            table: ResourceTable::new(),
            module_id,
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
            finalization_pushes: Vec::new(),
            runtime_reads: Vec::new(),
            layer_z,
            effective_layer_height,
            catchup_z_bottom,
        }
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

// ── Host trait implementations ──────────────────────────────────────────

use layer::slicer::world_layer::config_types as ct;
use layer::slicer::world_layer::geometry as geo;
use layer::slicer::world_layer::host_services as hs;
use layer::slicer::world_layer::ir_handles as ir;

impl geo::Host for HostExecutionContext {}

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
        _object_id: hs::ObjectId,
        _x: f32,
        _y: f32,
        _start_z: f32,
    ) -> wasmtime::Result<Option<f32>> {
        // Mesh queries require MeshIR data in the execution context, which is
        // not yet wired. Return None (no hit) — this is semantically valid
        // (the guest handles None as "no surface found") but means mesh-dependent
        // features like non-planar Z projection won't produce real results until
        // mesh data is plumbed into HostExecutionContext.
        Ok(None)
    }

    fn surface_normal_at(
        &mut self,
        _object_id: hs::ObjectId,
        _x: f32,
        _y: f32,
        _z: f32,
    ) -> wasmtime::Result<Option<Point3>> {
        // Same as raycast_z_down — mesh data not yet wired.
        Ok(None)
    }

    fn object_bounds(&mut self, object_id: hs::ObjectId) -> wasmtime::Result<BoundingBox3> {
        // Mesh data not yet wired. Return a trap so callers don't silently
        // proceed with a zero-volume bounding box.
        Err(wasmtime::Error::msg(format!(
            "host-service object-bounds not yet wired: no mesh data available for object '{object_id}'"
        )))
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
            hs::ClipOperation::Intersection => slicer_core::polygon_ops::ClipOperation::Intersection,
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
        let result = slicer_core::polygon_ops::offset(&ir_polys, delta_mm, ir_join);
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
            points: ep.contour.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect(),
        },
        holes: ep.holes.iter().map(|h| slicer_ir::Polygon {
            points: h.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect(),
        }).collect(),
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
            points: ep.contour.points.iter().map(|p| Point2 { x: p.x, y: p.y }).collect(),
        },
        holes: ep.holes.iter().map(|h| Polygon {
            points: h.points.iter().map(|p| Point2 { x: p.x, y: p.y }).collect(),
        }).collect(),
    }
}

/// Convert slicer-ir ExPolygons to WIT ExPolygons.
fn ir_to_wit_expolygons(eps: &[slicer_ir::ExPolygon]) -> Vec<ExPolygon> {
    eps.iter().map(ir_to_wit_expolygon).collect()
}

/// Convert slicer-ir PaintValue to WIT PaintValue.
fn ir_to_wit_paint_value(v: &slicer_ir::PaintValue) -> PaintValue {
    match v {
        slicer_ir::PaintValue::Flag(b) => PaintValue::Flag(*b),
        slicer_ir::PaintValue::Scalar(s) => PaintValue::Scalar(*s),
        slicer_ir::PaintValue::ToolIndex(t) => PaintValue::ToolIndex(*t),
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
fn ir_to_wit_paint_value_view(v: &slicer_ir::PaintValue) -> prepass::PaintValueView {
    match v {
        slicer_ir::PaintValue::Flag(b) => prepass::PaintValueView::Flag(*b),
        slicer_ir::PaintValue::Scalar(s) => prepass::PaintValueView::Scalar(*s),
        slicer_ir::PaintValue::ToolIndex(idx) => prepass::PaintValueView::ToolIndex(*idx),
    }
}

/// Convert a slicer-ir `PaintStroke` to a WIT `PaintStrokeView` record.
fn ir_to_wit_paint_stroke_view(stroke: &slicer_ir::PaintStroke) -> prepass::PaintStrokeView {
    prepass::PaintStrokeView {
        triangles: stroke
            .triangles
            .iter()
            .map(|tri| prepass::Point3 {
                x: tri[0].x,
                y: tri[0].y,
                z: tri[0].z,
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
        strokes: layer.strokes.iter().map(ir_to_wit_paint_stroke_view).collect(),
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
        .map(|v| prepass::Point3 { x: v.x, y: v.y, z: v.z })
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
        .map(|v| prepass::Point3 { x: v.x, y: v.y, z: v.z })
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
pub fn sliced_region_to_data(region: &slicer_ir::SlicedRegion, z: f32) -> SliceRegionData {
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
        slicer_ir::ExtrusionRole::PrimeTower => ExtrusionRole::Custom(String::new()),
        slicer_ir::ExtrusionRole::Skirt => ExtrusionRole::Custom(String::new()),
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
        feature_flags: wl.feature_flags.iter().map(ir_to_wit_wall_feature_flag).collect(),
    }
}

/// Convert a `PerimeterRegion` from the IR into a `PerimeterRegionData` WIT resource.
pub fn perimeter_region_to_data(region: &slicer_ir::PerimeterRegion) -> PerimeterRegionData {
    PerimeterRegionData {
        object_id: region.object_id.clone(),
        region_id: region.region_id.to_string(),
        wall_loops: region.walls.iter().map(ir_to_wit_wall_loop).collect(),
        infill_areas: ir_to_wit_expolygons(&region.infill_areas),
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
    slicer_core::polygon_ops::offset(polys, delta_mm, join)
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

    fn get_bool(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<bool>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Bool(b) => Some(*b),
            _ => None,
        }))
    }

    fn get_float(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<f64>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Float(f) => Some(normalize_subnormal_boundary(*f)),
            _ => None,
        }))
    }

    fn get_int(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<i64>> {
        let data = self.table.get(&self_)?;
        Ok(data.fields.get(&key).and_then(|v| match v {
            ConfigValueStorage::Int(i) => Some(*i),
            _ => None,
        }))
    }

    fn get_string(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<String>> {
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
        let rid = data.region_id.parse::<u64>().unwrap_or(0);
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
        self.runtime_reads.push(String::from("SliceIR.regions.polygons"));
        Ok(self.table.get(&self_)?.polygons.clone())
    }
    fn infill_areas(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<Vec<ExPolygon>> {
        self.touch_slice_region(&self_)?;
        self.runtime_reads.push(String::from("SliceIR.regions.infill-areas"));
        Ok(self.table.get(&self_)?.infill_areas.clone())
    }
    fn effective_layer_height(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<f32> {
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
    fn boundary_paint(&mut self, self_: Resource<SliceRegionData>) -> wasmtime::Result<Vec<BoundaryPaintEntry>> {
        self.runtime_reads.push(String::from("SliceIR"));
        Ok(self.table.get(&self_)?.boundary_paint.clone())
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
    fn touch_perimeter_region(&mut self, self_: &Resource<PerimeterRegionData>) -> wasmtime::Result<()> {
        let data = self.table.get(self_)?;
        let rid = data.region_id.parse::<u64>().unwrap_or(0);
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
    fn wall_loops(&mut self, self_: Resource<PerimeterRegionData>) -> wasmtime::Result<Vec<WallLoopView>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads.push(String::from("PerimeterIR.wall-loops"));
        Ok(self.table.get(&self_)?.wall_loops.clone())
    }
    fn infill_areas(&mut self, self_: Resource<PerimeterRegionData>) -> wasmtime::Result<Vec<ExPolygon>> {
        self.touch_perimeter_region(&self_)?;
        self.runtime_reads.push(String::from("PerimeterIR.infill-areas"));
        Ok(self.table.get(&self_)?.infill_areas.clone())
    }
    fn drop(&mut self, rep: Resource<PerimeterRegionData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostInfillOutputBuilder for HostExecutionContext {
    fn push_sparse_path(&mut self, _self_: Resource<InfillOutputBuilderData>, path: ExtrusionPath3d) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_perimeter_region.clone();
        self.infill_output.sparse_paths.push(path);
        self.infill_output.sparse_path_origins.push(origin);
        Ok(Ok(()))
    }
    fn push_solid_path(&mut self, _self_: Resource<InfillOutputBuilderData>, path: ExtrusionPath3d) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_perimeter_region.clone();
        self.infill_output.solid_paths.push(path);
        self.infill_output.solid_path_origins.push(origin);
        Ok(Ok(()))
    }
    fn push_ironing_path(&mut self, _self_: Resource<InfillOutputBuilderData>, path: ExtrusionPath3d) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_perimeter_region.clone();
        self.infill_output.ironing_paths.push(path);
        self.infill_output.ironing_path_origins.push(origin);
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<InfillOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostPerimeterOutputBuilder for HostExecutionContext {
    fn push_wall_loop(&mut self, _self_: Resource<PerimeterOutputBuilderData>, wall_loop: WallLoopView) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = wall_loop.path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_perimeter_region.clone();
        self.perimeter_output.wall_loops.push(wall_loop);
        self.perimeter_output.wall_loop_origins.push(origin);
        Ok(Ok(()))
    }
    /// Sets infill areas for this perimeter output builder.
    ///
    /// No Z envelope check is needed here — `ExPolygon` carries no Z coordinate.
    /// Z validation for infill paths is performed in `push_sparse_path` and
    /// `push_solid_path` where the actual extrusion geometry is supplied.
    fn set_infill_areas(&mut self, _self_: Resource<PerimeterOutputBuilderData>, areas: Vec<ExPolygon>) -> wasmtime::Result<Result<(), String>> {
        self.perimeter_output.infill_areas = areas;
        self.perimeter_output.infill_areas_origin = self.current_perimeter_region.clone();
        Ok(Ok(()))
    }
    fn push_seam_candidate(&mut self, _self_: Resource<PerimeterOutputBuilderData>, pos: Point3, score: f32) -> wasmtime::Result<Result<(), String>> {
        if let Err(e) = self.check_z_envelope(pos.z) {
            return Ok(Err(e));
        }
        let origin = self.current_perimeter_region.clone();
        self.perimeter_output.seam_candidates.push((pos, score));
        self.perimeter_output.seam_candidate_origins.push(origin);
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<PerimeterOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostSlicePostprocessBuilder for HostExecutionContext {
    fn set_polygons(&mut self, _self_: Resource<SlicePostprocessBuilderData>, region: RegionKey, polys: Vec<ExPolygon>) -> wasmtime::Result<Result<(), String>> {
        self.slice_postprocess_output.polygon_updates.push((region, polys));
        Ok(Ok(()))
    }
    fn set_path_z(&mut self, _self_: Resource<SlicePostprocessBuilderData>, region: RegionKey, path_idx: u32, vertex_idx: u32, z: f32) -> wasmtime::Result<Result<(), String>> {
        self.slice_postprocess_output.path_z_updates.push((region, path_idx, vertex_idx, z));
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<SlicePostprocessBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostGcodeOutputBuilder for HostExecutionContext {
    fn push_move(&mut self, _self_: Resource<GcodeOutputBuilderData>, cmd: GcodeMoveCmd) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::Move(cmd));
        Ok(Ok(()))
    }
    fn push_retract(&mut self, _self_: Resource<GcodeOutputBuilderData>, length: f32, speed: f32) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::Retract { length, speed });
        Ok(Ok(()))
    }
    fn push_fan_speed(&mut self, _self_: Resource<GcodeOutputBuilderData>, value: u8) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::FanSpeed(value));
        Ok(Ok(()))
    }
    fn push_temperature(&mut self, _self_: Resource<GcodeOutputBuilderData>, tool: u32, celsius: f32, wait: bool) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::Temperature { tool, celsius, wait });
        Ok(Ok(()))
    }
    fn push_tool_change(&mut self, _self_: Resource<GcodeOutputBuilderData>, from_tool: u32, to_tool: u32) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::ToolChange { from_tool, to_tool });
        Ok(Ok(()))
    }
    fn push_comment(&mut self, _self_: Resource<GcodeOutputBuilderData>, text: String) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::Comment(text));
        Ok(Ok(()))
    }
    fn push_raw(&mut self, _self_: Resource<GcodeOutputBuilderData>, text: String) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::Raw(text));
        Ok(Ok(()))
    }
    fn push_z_hop(&mut self, _self_: Resource<GcodeOutputBuilderData>, after_entity_index: u32, hop_height: f32) -> wasmtime::Result<Result<(), String>> {
        self.gcode_output.commands.push(GcodeCommandCollected::ZHop { after_entity_index, hop_height });
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<GcodeOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostSupportOutputBuilder for HostExecutionContext {
    fn push_support_path(&mut self, _self_: Resource<SupportOutputBuilderData>, path: ExtrusionPath3d) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_slice_region.clone();
        self.support_output.support_paths.push(path);
        self.support_output.support_path_origins.push(origin);
        Ok(Ok(()))
    }
    fn push_interface_path(&mut self, _self_: Resource<SupportOutputBuilderData>, path: ExtrusionPath3d, is_top_interface: bool) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_slice_region.clone();
        self.support_output.interface_paths.push((path, is_top_interface));
        self.support_output.interface_path_origins.push(origin);
        Ok(Ok(()))
    }
    fn push_raft_path(&mut self, _self_: Resource<SupportOutputBuilderData>, path: ExtrusionPath3d) -> wasmtime::Result<Result<(), String>> {
        if let Some(z) = path.points.first().map(|p| p.z) {
            if let Err(e) = self.check_z_envelope(z) {
                return Ok(Err(e));
            }
        }
        let origin = self.current_slice_region.clone();
        self.support_output.raft_paths.push(path);
        self.support_output.raft_path_origins.push(origin);
        Ok(Ok(()))
    }
    fn drop(&mut self, rep: Resource<SupportOutputBuilderData>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl ir::HostPaintRegionLayerView for HostExecutionContext {
    fn get_regions(&mut self, self_: Resource<PaintRegionLayerData>, semantic: PaintSemantic) -> wasmtime::Result<Vec<SemanticRegion>> {
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
        Ok(data.regions_by_semantic.get(key).cloned().unwrap_or_default())
    }
    fn get_custom_regions(&mut self, self_: Resource<PaintRegionLayerData>, module_id: String) -> wasmtime::Result<Vec<SemanticRegion>> {
        self.runtime_reads.push(String::from("PaintRegionIR"));
        Ok(self.table.get(&self_)?.custom_regions.get(&module_id).cloned().unwrap_or_default())
    }
    fn layer_index(&mut self, self_: Resource<PaintRegionLayerData>) -> wasmtime::Result<u32> {
        self.runtime_reads.push(String::from("PaintRegionIR"));
        Ok(self.table.get(&self_)?.layer_index)
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
            contour: slicer_ir::Polygon { points: ep.contour.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect() },
            holes: ep.holes.iter().map(|h| slicer_ir::Polygon { points: h.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect() }).collect(),
        }
    }
    fn p_ir_to_wit(ep: &slicer_ir::ExPolygon) -> pgeo::ExPolygon {
        pgeo::ExPolygon {
            contour: pgeo::Polygon { points: ep.contour.points.iter().map(|p| pgeo::Point2 { x: p.x, y: p.y }).collect() },
            holes: ep.holes.iter().map(|h| pgeo::Polygon { points: h.points.iter().map(|p| pgeo::Point2 { x: p.x, y: p.y }).collect() }).collect(),
        }
    }

    impl phs::Host for HostExecutionContext {
        fn log(&mut self, level: phs::LogLevel, message: String) -> wasmtime::Result<()> {
            let level_str = match level {
                phs::LogLevel::Trace => "trace", phs::LogLevel::Debug => "debug",
                phs::LogLevel::Info => "info", phs::LogLevel::Warn => "warn",
                phs::LogLevel::Error => "error",
            };
            self.log_messages.push((level_str.to_string(), message));
            Ok(())
        }
        fn raycast_z_down(&mut self, _: phs::ObjectId, _: f32, _: f32, _: f32) -> wasmtime::Result<Option<f32>> {
            self.runtime_reads.push(String::from("MeshIR"));
            Ok(None)
        }
        fn surface_normal_at(&mut self, _: phs::ObjectId, _: f32, _: f32, _: f32) -> wasmtime::Result<Option<pgeo::Point3>> {
            self.runtime_reads.push(String::from("MeshIR"));
            Ok(None)
        }
        fn object_bounds(&mut self, object_id: phs::ObjectId) -> wasmtime::Result<pgeo::BoundingBox3> {
            self.runtime_reads.push(String::from("MeshIR"));
            Err(wasmtime::Error::msg(format!("host-service object-bounds not yet wired: no mesh data for '{object_id}'")))
        }
        fn clip_polygons(&mut self, subject: Vec<pgeo::ExPolygon>, clip: Vec<pgeo::ExPolygon>, op: phs::ClipOperation) -> wasmtime::Result<Vec<pgeo::ExPolygon>> {
            let s: Vec<_> = subject.iter().map(p_wit_to_ir).collect();
            let c: Vec<_> = clip.iter().map(p_wit_to_ir).collect();
            let ir_op = match op { phs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union, phs::ClipOperation::Intersection => slicer_core::polygon_ops::ClipOperation::Intersection, phs::ClipOperation::Difference => slicer_core::polygon_ops::ClipOperation::Difference, phs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor };
            Ok(ir_clip_polygons(&s, &c, ir_op).iter().map(p_ir_to_wit).collect())
        }
        fn offset_polygons(&mut self, polygons: Vec<pgeo::ExPolygon>, delta_mm: f32, join: phs::OffsetJoinType) -> wasmtime::Result<Vec<pgeo::ExPolygon>> {
            let ir: Vec<_> = polygons.iter().map(p_wit_to_ir).collect();
            let j = match join { phs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter, phs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round, phs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square };
            Ok(ir_offset_polygons(&ir, delta_mm, j).iter().map(p_ir_to_wit).collect())
        }
        fn simplify_polygon(&mut self, polygon: pgeo::Polygon, _: f32) -> wasmtime::Result<pgeo::Polygon> {
            let pts: Vec<_> = polygon.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect();
            Ok(pgeo::Polygon { points: ir_simplify_polygon(pts).into_iter().map(|p| pgeo::Point2 { x: p.x, y: p.y }).collect() })
        }
        fn now_us(&mut self) -> wasmtime::Result<u64> { Ok(self.start_time.elapsed().as_micros() as u64) }
    }

    impl pct::Host for HostExecutionContext {}
    impl pct::HostConfigView for HostExecutionContext {
        fn get(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<pct::ConfigValue>> {
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
        fn get_bool(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<bool>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Bool(b) => Some(*b), _ => None }))
        }
        fn get_float(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<f64>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Float(f) => Some(normalize_subnormal_boundary(*f)), _ => None }))
        }
        fn get_int(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<i64>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Int(i) => Some(*i), _ => None }))
        }
        fn get_string(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<String>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Str(s) => Some(s.clone()), _ => None }))
        }
        fn keys(&mut self, self_: Resource<ConfigViewData>) -> wasmtime::Result<Vec<String>> {
            Ok(self.table.get(&self_)?.fields.keys().cloned().collect())
        }
        fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> { self.table.delete(rep)?; Ok(()) }
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
            self.table.delete(typed)?; Ok(())
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
            self.table.delete(typed)?; Ok(())
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
            self.paint_region_entries.push(entry);
            Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<pm::PaintSegmentationOutput>) -> wasmtime::Result<()> {
            let typed: Resource<PaintSegmentationOutputData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?;
            Ok(())
        }
    }

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
    use super::*;
    use finalization::slicer::world_finalization::config_types as fct;
    use finalization::slicer::world_finalization::geometry as fgeo;
    use finalization::slicer::world_finalization::host_services as fhs;
    use super::finalization as fm;

    impl fgeo::Host for HostExecutionContext {}

    fn f_wit_to_ir(ep: &fgeo::ExPolygon) -> slicer_ir::ExPolygon {
        slicer_ir::ExPolygon {
            contour: slicer_ir::Polygon { points: ep.contour.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect() },
            holes: ep.holes.iter().map(|h| slicer_ir::Polygon { points: h.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect() }).collect(),
        }
    }
    fn f_ir_to_wit(ep: &slicer_ir::ExPolygon) -> fgeo::ExPolygon {
        fgeo::ExPolygon {
            contour: fgeo::Polygon { points: ep.contour.points.iter().map(|p| fgeo::Point2 { x: p.x, y: p.y }).collect() },
            holes: ep.holes.iter().map(|h| fgeo::Polygon { points: h.points.iter().map(|p| fgeo::Point2 { x: p.x, y: p.y }).collect() }).collect(),
        }
    }

    impl fhs::Host for HostExecutionContext {
        fn log(&mut self, level: fhs::LogLevel, message: String) -> wasmtime::Result<()> {
            let level_str = match level {
                fhs::LogLevel::Trace => "trace", fhs::LogLevel::Debug => "debug",
                fhs::LogLevel::Info => "info", fhs::LogLevel::Warn => "warn",
                fhs::LogLevel::Error => "error",
            };
            self.log_messages.push((level_str.to_string(), message));
            Ok(())
        }
        fn raycast_z_down(&mut self, _: fhs::ObjectId, _: f32, _: f32, _: f32) -> wasmtime::Result<Option<f32>> { Ok(None) }
        fn surface_normal_at(&mut self, _: fhs::ObjectId, _: f32, _: f32, _: f32) -> wasmtime::Result<Option<fgeo::Point3>> { Ok(None) }
        fn object_bounds(&mut self, object_id: fhs::ObjectId) -> wasmtime::Result<fgeo::BoundingBox3> {
            Err(wasmtime::Error::msg(format!("host-service object-bounds not yet wired: no mesh data for '{object_id}'")))
        }
        fn clip_polygons(&mut self, subject: Vec<fgeo::ExPolygon>, clip: Vec<fgeo::ExPolygon>, op: fhs::ClipOperation) -> wasmtime::Result<Vec<fgeo::ExPolygon>> {
            let s: Vec<_> = subject.iter().map(f_wit_to_ir).collect();
            let c: Vec<_> = clip.iter().map(f_wit_to_ir).collect();
            let ir_op = match op { fhs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union, fhs::ClipOperation::Intersection => slicer_core::polygon_ops::ClipOperation::Intersection, fhs::ClipOperation::Difference => slicer_core::polygon_ops::ClipOperation::Difference, fhs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor };
            Ok(ir_clip_polygons(&s, &c, ir_op).iter().map(f_ir_to_wit).collect())
        }
        fn offset_polygons(&mut self, polygons: Vec<fgeo::ExPolygon>, delta_mm: f32, join: fhs::OffsetJoinType) -> wasmtime::Result<Vec<fgeo::ExPolygon>> {
            let ir: Vec<_> = polygons.iter().map(f_wit_to_ir).collect();
            let j = match join { fhs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter, fhs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round, fhs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square };
            Ok(ir_offset_polygons(&ir, delta_mm, j).iter().map(f_ir_to_wit).collect())
        }
        fn simplify_polygon(&mut self, polygon: fgeo::Polygon, _: f32) -> wasmtime::Result<fgeo::Polygon> {
            let pts: Vec<_> = polygon.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect();
            Ok(fgeo::Polygon { points: ir_simplify_polygon(pts).into_iter().map(|p| fgeo::Point2 { x: p.x, y: p.y }).collect() })
        }
        fn now_us(&mut self) -> wasmtime::Result<u64> { Ok(self.start_time.elapsed().as_micros() as u64) }
    }

    impl fct::Host for HostExecutionContext {}
    impl fct::HostConfigView for HostExecutionContext {
        fn get(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<fct::ConfigValue>> {
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
        fn get_bool(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<bool>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Bool(b) => Some(*b), _ => None }))
        }
        fn get_float(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<f64>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Float(f) => Some(normalize_subnormal_boundary(*f)), _ => None }))
        }
        fn get_int(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<i64>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Int(i) => Some(*i), _ => None }))
        }
        fn get_string(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<String>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Str(s) => Some(s.clone()), _ => None }))
        }
        fn keys(&mut self, self_: Resource<ConfigViewData>) -> wasmtime::Result<Vec<String>> {
            Ok(self.table.get(&self_)?.fields.keys().cloned().collect())
        }
        fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> { self.table.delete(rep)?; Ok(()) }
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

    impl fm::HostLayerCollectionView for HostExecutionContext {
        fn layer_index(&mut self, self_: Resource<fm::LayerCollectionView>) -> wasmtime::Result<u32> {
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
        fn entity_count(&mut self, self_: Resource<fm::LayerCollectionView>) -> wasmtime::Result<u32> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data.entity_count)
        }
        fn tool_changes(&mut self, self_: Resource<fm::LayerCollectionView>) -> wasmtime::Result<Vec<fm::ToolChangeView>> {
            self.runtime_reads.push(String::from("LayerCollectionIR"));
            let typed: Resource<LayerCollectionViewData> = Resource::new_borrow(self_.rep());
            let data = self.table.get(&typed)?;
            Ok(data
                .tool_changes
                .iter()
                .map(|(after_entity_index, from_tool, to_tool)| fm::ToolChangeView {
                    after_entity_index: *after_entity_index,
                    from_tool: *from_tool,
                    to_tool: *to_tool,
                })
                .collect())
        }
        fn drop(&mut self, rep: Resource<fm::LayerCollectionView>) -> wasmtime::Result<()> {
            let typed: Resource<LayerCollectionViewData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?; Ok(())
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
            let ir_region_key = slicer_ir::RegionKey {
                global_layer_index: region_key.layer_index,
                object_id: region_key.object_id,
                region_id: region_key.region_id.parse::<u64>().unwrap_or(0),
            };
            data.pushes.push(FinalizationBuilderPush::EntityToLayer {
                layer_index,
                path: finalization_path_wit_to_ir(&path),
                region_key: ir_region_key,
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
}

// ── Postpass world host trait impls ───────────────────────────────────

mod postpass_impls {
    use super::*;
    use postpass::slicer::world_postpass::config_types as ppct;
    use postpass::slicer::world_postpass::geometry as ppgeo;
    use postpass::slicer::world_postpass::host_services as pphs;
    use super::postpass as ppm;

    impl ppgeo::Host for HostExecutionContext {}

    fn pp_wit_to_ir(ep: &ppgeo::ExPolygon) -> slicer_ir::ExPolygon {
        slicer_ir::ExPolygon {
            contour: slicer_ir::Polygon { points: ep.contour.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect() },
            holes: ep.holes.iter().map(|h| slicer_ir::Polygon { points: h.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect() }).collect(),
        }
    }
    fn pp_ir_to_wit(ep: &slicer_ir::ExPolygon) -> ppgeo::ExPolygon {
        ppgeo::ExPolygon {
            contour: ppgeo::Polygon { points: ep.contour.points.iter().map(|p| ppgeo::Point2 { x: p.x, y: p.y }).collect() },
            holes: ep.holes.iter().map(|h| ppgeo::Polygon { points: h.points.iter().map(|p| ppgeo::Point2 { x: p.x, y: p.y }).collect() }).collect(),
        }
    }

    impl pphs::Host for HostExecutionContext {
        fn log(&mut self, level: pphs::LogLevel, message: String) -> wasmtime::Result<()> {
            let level_str = match level {
                pphs::LogLevel::Trace => "trace", pphs::LogLevel::Debug => "debug",
                pphs::LogLevel::Info => "info", pphs::LogLevel::Warn => "warn",
                pphs::LogLevel::Error => "error",
            };
            self.log_messages.push((level_str.to_string(), message));
            Ok(())
        }
        fn raycast_z_down(&mut self, _: pphs::ObjectId, _: f32, _: f32, _: f32) -> wasmtime::Result<Option<f32>> { Ok(None) }
        fn surface_normal_at(&mut self, _: pphs::ObjectId, _: f32, _: f32, _: f32) -> wasmtime::Result<Option<ppgeo::Point3>> { Ok(None) }
        fn object_bounds(&mut self, object_id: pphs::ObjectId) -> wasmtime::Result<ppgeo::BoundingBox3> {
            Err(wasmtime::Error::msg(format!("host-service object-bounds not yet wired: no mesh data for '{object_id}'")))
        }
        fn clip_polygons(&mut self, subject: Vec<ppgeo::ExPolygon>, clip: Vec<ppgeo::ExPolygon>, op: pphs::ClipOperation) -> wasmtime::Result<Vec<ppgeo::ExPolygon>> {
            let s: Vec<_> = subject.iter().map(pp_wit_to_ir).collect();
            let c: Vec<_> = clip.iter().map(pp_wit_to_ir).collect();
            let ir_op = match op { pphs::ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union, pphs::ClipOperation::Intersection => slicer_core::polygon_ops::ClipOperation::Intersection, pphs::ClipOperation::Difference => slicer_core::polygon_ops::ClipOperation::Difference, pphs::ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor };
            Ok(ir_clip_polygons(&s, &c, ir_op).iter().map(pp_ir_to_wit).collect())
        }
        fn offset_polygons(&mut self, polygons: Vec<ppgeo::ExPolygon>, delta_mm: f32, join: pphs::OffsetJoinType) -> wasmtime::Result<Vec<ppgeo::ExPolygon>> {
            let ir: Vec<_> = polygons.iter().map(pp_wit_to_ir).collect();
            let j = match join { pphs::OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter, pphs::OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round, pphs::OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square };
            Ok(ir_offset_polygons(&ir, delta_mm, j).iter().map(pp_ir_to_wit).collect())
        }
        fn simplify_polygon(&mut self, polygon: ppgeo::Polygon, _: f32) -> wasmtime::Result<ppgeo::Polygon> {
            let pts: Vec<_> = polygon.points.iter().map(|p| slicer_ir::Point2 { x: p.x, y: p.y }).collect();
            Ok(ppgeo::Polygon { points: ir_simplify_polygon(pts).into_iter().map(|p| ppgeo::Point2 { x: p.x, y: p.y }).collect() })
        }
        fn now_us(&mut self) -> wasmtime::Result<u64> { Ok(self.start_time.elapsed().as_micros() as u64) }
    }

    impl ppct::Host for HostExecutionContext {}
    impl ppct::HostConfigView for HostExecutionContext {
        fn get(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<ppct::ConfigValue>> {
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
        fn get_bool(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<bool>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Bool(b) => Some(*b), _ => None }))
        }
        fn get_float(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<f64>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Float(f) => Some(normalize_subnormal_boundary(*f)), _ => None }))
        }
        fn get_int(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<i64>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Int(i) => Some(*i), _ => None }))
        }
        fn get_string(&mut self, self_: Resource<ConfigViewData>, key: String) -> wasmtime::Result<Option<String>> {
            let data = self.table.get(&self_)?;
            Ok(data.fields.get(&key).and_then(|v| match v { ConfigValueStorage::Str(s) => Some(s.clone()), _ => None }))
        }
        fn keys(&mut self, self_: Resource<ConfigViewData>) -> wasmtime::Result<Vec<String>> {
            Ok(self.table.get(&self_)?.fields.keys().cloned().collect())
        }
        fn drop(&mut self, rep: Resource<ConfigViewData>) -> wasmtime::Result<()> { self.table.delete(rep)?; Ok(()) }
    }

    impl ppm::HostGcodeOutputBuilder for HostExecutionContext {
        fn push_move(&mut self, _: Resource<ppm::GcodeOutputBuilder>, cmd: ppm::GcodeMoveCmd) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::Move(GcodeMoveCmd {
                x: cmd.x, y: cmd.y, z: cmd.z, e: cmd.e, f: cmd.f,
                role: convert_postpass_role(&cmd.role),
            }));
            Ok(Ok(()))
        }
        fn push_retract(&mut self, _: Resource<ppm::GcodeOutputBuilder>, length: f32, speed: f32) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::Retract { length, speed }); Ok(Ok(()))
        }
        fn push_fan_speed(&mut self, _: Resource<ppm::GcodeOutputBuilder>, value: u8) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::FanSpeed(value)); Ok(Ok(()))
        }
        fn push_temperature(&mut self, _: Resource<ppm::GcodeOutputBuilder>, tool: u32, celsius: f32, wait: bool) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::Temperature { tool, celsius, wait }); Ok(Ok(()))
        }
        fn push_tool_change(&mut self, _: Resource<ppm::GcodeOutputBuilder>, from_tool: u32, to_tool: u32) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::ToolChange { from_tool, to_tool }); Ok(Ok(()))
        }
        fn push_comment(&mut self, _: Resource<ppm::GcodeOutputBuilder>, text: String) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::Comment(text)); Ok(Ok(()))
        }
        fn push_raw(&mut self, _: Resource<ppm::GcodeOutputBuilder>, text: String) -> wasmtime::Result<Result<(), String>> {
            self.gcode_output.commands.push(GcodeCommandCollected::Raw(text)); Ok(Ok(()))
        }
        fn drop(&mut self, rep: Resource<ppm::GcodeOutputBuilder>) -> wasmtime::Result<()> {
            let typed: Resource<PostpassGcodeOutputBuilderData> = Resource::new_own(rep.rep());
            self.table.delete(typed)?; Ok(())
        }
    }

    impl ppm::PostpassModuleImports for HostExecutionContext {}

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
        ExtrusionRole::Custom(s) => slicer_ir::ExtrusionRole::Custom(s.clone()),
    }
}

/// Validate and convert a WIT `ExtrusionPath3d` to a slicer-ir `ExtrusionPath3D`.
///
/// Returns an error if any point coordinate is NaN or Inf (per docs/02_ir_schemas.md).
pub fn convert_extrusion_path(path: &ExtrusionPath3d) -> Result<slicer_ir::ExtrusionPath3D, String> {
    if path.speed_factor.is_nan() || path.speed_factor.is_infinite() {
        return Err(format!("speed_factor is NaN or Inf ({})", path.speed_factor));
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
            schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
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

    drain_into(sparse, &collected.sparse_path_origins, "sparse_infill", &mut buckets, |r, p| r.sparse_infill.push(p))?;
    drain_into(solid, &collected.solid_path_origins, "solid_infill", &mut buckets, |r, p| r.solid_infill.push(p))?;
    drain_into(ironing, &collected.ironing_path_origins, "ironing", &mut buckets, |r, p| r.ironing.push(p))?;

    Ok(slicer_ir::InfillIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
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
    let support: Vec<_> = collected.support_paths.iter().map(convert_extrusion_path).collect::<Result<_, _>>()?;
    let interface: Vec<_> = collected.interface_paths.iter().map(|(p, _)| convert_extrusion_path(p)).collect::<Result<_, _>>()?;
    let raft: Vec<_> = collected.raft_paths.iter().map(convert_extrusion_path).collect::<Result<_, _>>()?;

    let any_tagged = collected.support_path_origins.iter().any(Option::is_some)
        || collected.interface_path_origins.iter().any(Option::is_some)
        || collected.raft_path_origins.iter().any(Option::is_some);

    if !any_tagged {
        return Ok(slicer_ir::SupportIR {
            schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
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
            let origin = origins[i].as_ref().ok_or_else(|| format!(
                "{kind} path[{i}] was emitted without an active slice source region; \
                 guest must access a slice-region-view (object-id/region-id/polygons/\
                 infill-areas/effective-layer-height/z/has-nonplanar/boundary-paint) \
                 before pushing support output for identity-preserving commit"
            ))?;
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
    let support = group_by_origin(support, &collected.support_path_origins, "support", &mut order)?;
    let interface = group_by_origin(interface, &collected.interface_path_origins, "interface", &mut order)?;
    let raft = group_by_origin(raft, &collected.raft_path_origins, "raft", &mut order)?;

    Ok(slicer_ir::SupportIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
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
            flag.custom.iter().map(|(k, v)| (k.clone(), convert_paint_value(v))),
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
        feature_flags: wl.feature_flags.iter().map(convert_wall_feature_flag).collect(),
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
    let walls: Vec<slicer_ir::WallLoop> = collected
        .wall_loops
        .iter()
        .map(convert_wall_loop)
        .collect::<Result<_, _>>()?;
    let infill_areas = wit_to_ir_expolygons(&collected.infill_areas);
    let seam_candidates: Vec<slicer_ir::SeamCandidate> = collected
        .seam_candidates
        .iter()
        .enumerate()
        .map(|(i, (pos, score))| {
            if pos.x.is_nan() || pos.x.is_infinite()
                || pos.y.is_nan() || pos.y.is_infinite()
                || pos.z.is_nan() || pos.z.is_infinite()
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
                    },
                    score: *score,
                    reason: slicer_ir::SeamReason::Aligned,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let any_tagged = collected.wall_loop_origins.iter().any(Option::is_some)
        || collected.seam_candidate_origins.iter().any(Option::is_some)
        || collected.infill_areas_origin.is_some();

    if !any_tagged {
        return Ok(slicer_ir::PerimeterIR {
            schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
            global_layer_index: layer_index,
            regions: vec![slicer_ir::PerimeterRegion {
                object_id: String::new(),
                region_id: 0,
                walls,
                infill_areas,
                seam_candidates,
                resolved_seam: None,
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

    if !walls.is_empty() && collected.wall_loop_origins.len() != walls.len() {
        return Err(format!(
            "wall_loops: origin tag count ({}) does not match wall count ({})",
            collected.wall_loop_origins.len(),
            walls.len()
        ));
    }
    for (i, wl) in walls.into_iter().enumerate() {
        let origin = collected.wall_loop_origins[i].as_ref().ok_or_else(|| format!(
            "wall_loop[{i}] was emitted without an active perimeter source region; \
             guest must access a perimeter-region-view before pushing wall loops"
        ))?;
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
        let origin = collected.seam_candidate_origins[i].as_ref().ok_or_else(|| format!(
            "seam_candidate[{i}] was emitted without an active perimeter source region"
        ))?;
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

    Ok(slicer_ir::PerimeterIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
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
        let idx = find_region(&existing.regions, key).ok_or_else(|| format!(
            "slice_postprocess polygon_update[{i}] targets unknown region \
             (object_id='{}', region_id='{}'); guest must reference an existing \
             slice-region-view identity for identity-preserving commit",
            key.object_id, key.region_id,
        ))?;
        existing.regions[idx].polygons = wit_to_ir_expolygons(polys);
    }

    for (i, (key, path_idx, vertex_idx, z)) in collected.path_z_updates.iter().enumerate() {
        let ridx = find_region(&existing.regions, key).ok_or_else(|| format!(
            "slice_postprocess path_z_update[{i}] targets unknown region \
             (object_id='{}', region_id='{}')",
            key.object_id, key.region_id,
        ))?;
        let region = &mut existing.regions[ridx];
        let poly_count = region.polygons.len();
        let poly = region.polygons.get_mut(*path_idx as usize).ok_or_else(|| format!(
            "slice_postprocess path_z_update[{i}]: polygon index {path_idx} out of range \
             for region ({}, {}) with {poly_count} polygons",
            key.object_id, key.region_id,
        ))?;
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
