//! Host service wrappers.
//!
//! These wrappers are the SDK-side entry points that mirror the WIT
//! `host-services` interface (see `wit/host-api.wit` and
//! `crates/slicer-runtime/src/wit_host.rs`). They are no longer silent
//! placeholders — each wrapper either performs the same work the host
//! does on the WASM boundary, or returns an explicit, observable signal
//! when the underlying service is unavailable.
//!
//! Specifically:
//! - Logging is routed through an installable thread-local sink that
//!   defaults to writing to `stderr`. Tests install a capture sink to
//!   assert log behavior.
//! - Geometry helpers (`clip_polygons`, `offset_polygons`,
//!   `simplify_polygon`) delegate to `slicer_core::polygon_ops`, the same
//!   crate the host uses, ensuring SDK and host produce identical results.
//! - Mesh queries (`raycast_z_down`, `surface_normal_at`, `object_bounds`)
//!   route through an optional thread-local [`MeshSource`]. When no source
//!   is installed, ray/normal queries return `None` (the documented
//!   "no surface found" signal) and `object_bounds` returns
//!   `Err(HostUnavailable)` instead of a meaningless zero box.
//! - Timestamps come from a monotonic process-start `Instant`, matching
//!   the host's per-call deterministic profiling baseline.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::Instant;

use slicer_ir::{BoundingBox3, ExPolygon, Point3, Polygon};

/// Wall emission sequence, mirroring
/// `slicer_core::perimeter_utils::WallSequence`. Re-exported here so
/// module authors can refer to the SDK's host-side `ArachneParams.wall_sequence`
/// field without taking a direct dependency on `slicer-core` (which is
/// `host-algos`-gated and unavailable on `wasm32` guest builds — the
/// `WallSequence` type itself is plain data, so the re-export is safe on
/// both targets).
pub use slicer_core::perimeter_utils::WallSequence;

/// Log levels accepted by [`log`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Verbose trace diagnostics.
    Trace,
    /// Debug diagnostics.
    Debug,
    /// Informational diagnostics.
    Info,
    /// Warning diagnostics.
    Warn,
    /// Error diagnostics.
    Error,
}

impl LogLevel {
    /// Returns the canonical lower-case label used by the host
    /// (matches `wit_host::hs::LogLevel` mapping).
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// Polygon clipping operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipOperation {
    /// Merge all input areas.
    Union,
    /// Keep only overlap between subject and clip sets.
    Intersection,
    /// Subtract clip set from subject set.
    Difference,
    /// Keep non-overlapping portions.
    Xor,
}

/// Join type for polygon offset operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetJoinType {
    /// Miter joins.
    Miter,
    /// Round joins.
    Round,
    /// Square joins.
    Square,
}

/// Returned by host services that require backing data the SDK was not
/// given (typically mesh data for `object_bounds`). Callers MUST treat
/// this as a contract failure rather than silently substituting a zero
/// or default value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostUnavailable {
    /// Name of the missing service or backing data source.
    pub service: &'static str,
    /// Identifier the caller asked about (e.g. an object id).
    pub subject: String,
}

impl std::fmt::Display for HostUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "host service '{}' is unavailable for '{}': install a backing source via slicer_sdk::host::test_support",
            self.service, self.subject
        )
    }
}

impl std::error::Error for HostUnavailable {}

// ── Logging sink ────────────────────────────────────────────────────────

thread_local! {
    static LOG_CAPTURE: RefCell<Option<Vec<(LogLevel, String)>>> = const { RefCell::new(None) };
}

/// Logs a message with the requested level.
///
/// If a capture sink is installed via [`test_support::install_log_capture`],
/// the message is appended there; otherwise the message is written to
/// `stderr`. Both modes are observable, replacing the prior silent no-op.
pub fn log(level: LogLevel, message: &str) {
    LOG_CAPTURE.with(|cell| {
        if let Some(buf) = cell.borrow_mut().as_mut() {
            buf.push((level, message.to_string()));
            return;
        }
        eprintln!("[{}] {}", level.as_str(), message);
    });
}

/// Logs a trace message.
pub fn log_trace(message: &str) {
    log(LogLevel::Trace, message);
}

/// Logs a debug message.
pub fn log_debug(message: &str) {
    log(LogLevel::Debug, message);
}

/// Logs an informational message.
pub fn log_info(message: &str) {
    log(LogLevel::Info, message);
}

/// Logs a warning message.
pub fn log_warn(message: &str) {
    log(LogLevel::Warn, message);
}

/// Logs an error message.
pub fn log_error(message: &str) {
    log(LogLevel::Error, message);
}

// ── Mesh source injection ───────────────────────────────────────────────

/// Backing source for mesh queries (`raycast_z_down`, `surface_normal_at`,
/// `object_bounds`). Installed via [`test_support::install_mesh_source`]
/// for tests; production embedders install a real implementation that
/// reads from the host's MeshIR.
pub trait MeshSource: Send + Sync + 'static {
    /// Cast a ray from `(x, y, start_z)` straight down `-Z`. Returns the
    /// hit Z if any, `None` if no surface is hit.
    fn raycast_z_down(&self, object_id: &str, x: f32, y: f32, start_z: f32) -> Option<f32>;
    /// Returns the surface normal at the given world-space point, if known.
    fn surface_normal_at(&self, object_id: &str, x: f32, y: f32, z: f32) -> Option<Point3>;
    /// Returns axis-aligned object bounds (mm) or `None` if the object is
    /// unknown to the source.
    fn object_bounds(&self, object_id: &str) -> Option<BoundingBox3>;
}

thread_local! {
    static MESH_SOURCE: RefCell<Option<Box<dyn MeshSource>>> = const { RefCell::new(None) };
}

fn with_mesh_source<R>(f: impl FnOnce(Option<&dyn MeshSource>) -> R) -> R {
    MESH_SOURCE.with(|cell| {
        let borrow = cell.borrow();
        f(borrow.as_deref())
    })
}

/// Casts a vertical ray downward against an object mesh.
///
/// Returns `Some(z)` for a hit, `None` for "no surface found" — the
/// documented signal callers must already handle. When no [`MeshSource`]
/// is installed, returns `None`.
#[must_use]
pub fn raycast_z_down(object_id: &str, x: f32, y: f32, start_z: f32) -> Option<f32> {
    with_mesh_source(|src| src.and_then(|s| s.raycast_z_down(object_id, x, y, start_z)))
}

/// Queries the surface normal at a 3D point. `None` when unknown or when
/// no [`MeshSource`] is installed.
#[must_use]
pub fn surface_normal_at(object_id: &str, x: f32, y: f32, z: f32) -> Option<Point3> {
    with_mesh_source(|src| src.and_then(|s| s.surface_normal_at(object_id, x, y, z)))
}

/// Returns axis-aligned object bounds in millimeters, or
/// `Err(HostUnavailable)` when no [`MeshSource`] is installed (or the
/// source does not know the object). This replaces the previous silent
/// zero-box no-op so callers cannot accidentally proceed with bogus
/// bounds.
pub fn object_bounds(object_id: &str) -> Result<BoundingBox3, HostUnavailable> {
    with_mesh_source(|src| {
        src.and_then(|s| s.object_bounds(object_id))
            .ok_or_else(|| HostUnavailable {
                service: "object_bounds",
                subject: object_id.to_string(),
            })
    })
}

// ── Geometry (delegates to slicer-core, the same backing the host uses) ─

fn to_core_clip_op(op: ClipOperation) -> slicer_core::polygon_ops::ClipOperation {
    match op {
        ClipOperation::Union => slicer_core::polygon_ops::ClipOperation::Union,
        ClipOperation::Intersection => slicer_core::polygon_ops::ClipOperation::Intersection,
        ClipOperation::Difference => slicer_core::polygon_ops::ClipOperation::Difference,
        ClipOperation::Xor => slicer_core::polygon_ops::ClipOperation::Xor,
    }
}

fn to_core_join(join: OffsetJoinType) -> slicer_core::polygon_ops::OffsetJoinType {
    match join {
        OffsetJoinType::Miter => slicer_core::polygon_ops::OffsetJoinType::Miter,
        OffsetJoinType::Round => slicer_core::polygon_ops::OffsetJoinType::Round,
        OffsetJoinType::Square => slicer_core::polygon_ops::OffsetJoinType::Square,
    }
}

/// Applies host-side polygon clipping. Backed by the same Clipper2
/// implementation the host uses on the WASM boundary
/// (`slicer_core::polygon_ops::clip_polygons`).
#[must_use]
pub fn clip_polygons(
    subject: &[ExPolygon],
    clip: &[ExPolygon],
    op: ClipOperation,
) -> Vec<ExPolygon> {
    slicer_core::polygon_ops::clip_polygons(subject, clip, to_core_clip_op(op))
}

/// Applies host-side polygon offsetting. Backed by
/// `slicer_core::polygon_ops::offset`.
#[must_use]
pub fn offset_polygons(
    polygons: &[ExPolygon],
    delta_mm: f32,
    join: OffsetJoinType,
) -> Vec<ExPolygon> {
    slicer_core::polygon_ops::offset(polygons, delta_mm, to_core_join(join), 0.0)
}

/// Simplifies a polygon by removing collinear vertices, mirroring the
/// host's `simplify_polygon` impl in `wit_host.rs`. The `tolerance_mm`
/// parameter is reserved for a future Douglas-Peucker variant; today it
/// is ignored — the call is no longer a no-op clone, it actually drops
/// collinear vertices.
#[must_use]
pub fn simplify_polygon(polygon: &Polygon, tolerance_mm: f32) -> Polygon {
    let _ = tolerance_mm;
    let mut points = polygon.points.clone();
    if points.len() < 3 {
        return Polygon { points };
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
            let prev = points[(i + n - 1) % n];
            let curr = points[i];
            let next = points[(i + 1) % n];
            // Cross product of (curr - prev) × (next - curr) on i64 lattice.
            let cross = (curr.x - prev.x) as i128 * (next.y - curr.y) as i128
                - (curr.y - prev.y) as i128 * (next.x - curr.x) as i128;
            if cross == 0 {
                keep[i] = false;
                changed = true;
            }
        }
        if changed {
            points = points
                .into_iter()
                .enumerate()
                .filter(|(i, _)| keep[*i])
                .map(|(_, p)| p)
                .collect();
        }
    }
    Polygon { points }
}

// ── Medial axis (host-only Voronoi; cfg-split native vs wasm32) ─────────

/// Compute the medial axis (centerline) of an [`ExPolygon`] returning
/// variable-width [`ThickPolyline`] chains.
///
/// On native targets this delegates directly to
/// `slicer_core::medial_axis::medial_axis` (host-algos feature, available
/// via the target-specific dep in Cargo.toml).  On wasm32 targets the
/// boostvoronoi implementation is unavailable so the call is forwarded to
/// the host through the `slicer:common/host-services#medial-axis` WIT import.
pub fn medial_axis(
    input: &ExPolygon,
    min_width: f32,
    max_width: f32,
) -> Result<Vec<slicer_ir::ThickPolyline>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        slicer_core::medial_axis::medial_axis(input, min_width, max_width)
            .map_err(|e| format!("{:?}", e))
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On wasm32, invoke the host import generated by wit-bindgen.
        // We generate a minimal import-only binding here so we can call
        // host-services::medial-axis without depending on the full world
        // generated by the #[slicer_module] macro (which lives in a private
        // inner module and is not accessible from this crate).
        #[allow(dead_code)]
        mod __sdk_host_medial_axis_import {
            ::wit_bindgen::generate!({
                inline: r#"
package slicer:sdk-medial-axis-helper;

package slicer:types {
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
            ironing, bridge-infill, wipe-tower, gap-fill, custom(string),
        }
        record semver { major: u32, minor: u32, patch: u32 }
        record point2-with-width { x: f32, y: f32, width: f32 }
        record thick-polyline { points: list<point2-with-width> }
    }
}

package slicer:common {
    interface host-services {
        use slicer:types/geometry.{ex-polygon, thick-polyline};
        medial-axis: func(input: ex-polygon, min-width: f32, max-width: f32) -> result<list<thick-polyline>, string>;
    }
}

world sdk-medial-axis {
    import slicer:common/host-services;
}
"#,
                world: "sdk-medial-axis",
                generate_all,
            });
        }

        // Build WIT ex-polygon from slicer_ir::ExPolygon
        let wit_input = __sdk_host_medial_axis_import::slicer::types::geometry::ExPolygon {
            contour: __sdk_host_medial_axis_import::slicer::types::geometry::Polygon {
                points: input
                    .contour
                    .points
                    .iter()
                    .map(
                        |p| __sdk_host_medial_axis_import::slicer::types::geometry::Point2 {
                            x: p.x,
                            y: p.y,
                        },
                    )
                    .collect(),
            },
            holes: input
                .holes
                .iter()
                .map(
                    |h| __sdk_host_medial_axis_import::slicer::types::geometry::Polygon {
                        points: h
                            .points
                            .iter()
                            .map(|p| {
                                __sdk_host_medial_axis_import::slicer::types::geometry::Point2 {
                                    x: p.x,
                                    y: p.y,
                                }
                            })
                            .collect(),
                    },
                )
                .collect(),
        };

        let result = __sdk_host_medial_axis_import::slicer::common::host_services::medial_axis(
            &wit_input, min_width, max_width,
        );

        result.map(|polylines| {
            polylines
                .into_iter()
                .map(|tp| slicer_ir::ThickPolyline {
                    points: tp
                        .points
                        .into_iter()
                        .map(|p| slicer_ir::Point2WithWidth {
                            x: p.x,
                            y: p.y,
                            width: p.width,
                        })
                        .collect(),
                })
                .collect()
        })
    }
}

// ── Arachne wall generation (host-only Voronoi/SKT; cfg-split native vs wasm32) ─

/// Parameters for [`generate_arachne_walls`], mirroring
/// `slicer_core::arachne::pipeline::ArachneParams` field-for-field.
///
/// Defined locally rather than re-exported from `slicer_core`:
/// `slicer_core::arachne::pipeline` (and the `skeletal_trapezoidation`/
/// `beading` stack it orchestrates) is gated behind the `host-algos`
/// feature, which `crates/slicer-sdk/Cargo.toml` enables only for this
/// crate's native target (`cfg(not(target_arch = "wasm32"))`) — the wasm32
/// guest build has no such type to alias. Matches [`medial_axis`]'s own
/// native-vs-wasm32 type-boundary handling (that function's inputs/outputs
/// happen to be plain `slicer_ir` types available on both targets, so it
/// needed no equivalent mirror struct; `ArachneParams` does, since its
/// canonical home is host-algos-gated).
///
/// Every distance/width field is in millimeters, matching
/// `slicer_core::arachne::pipeline::ArachneParams`'s own convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArachneParams {
    /// Nominal wall width (mm).
    pub optimal_width: f64,
    /// Target width for the outermost/innermost bead (mm).
    pub preferred_bead_width_outer: f64,
    /// Maximum bead count the composed beading strategy will ever request.
    pub max_bead_count: u32,
    /// Gaussian decay radius (bead-count units) for the base distribution
    /// strategy.
    pub distribution_count: u32,
    /// Whisker-dissolve length budget (mm) for the centrality filter.
    pub transition_filter_dist: f64,
    /// Depth floor (mm) for the centrality filter.
    pub min_central_distance: f64,
    /// Visvalingam-Whyatt width-weighted area threshold (mm²) for toolpath
    /// simplification.
    pub visvalingam_area_threshold: f64,
    /// Length-factor multiplier for the small-line removal threshold.
    pub min_length_factor: f64,
    /// Nominal width (mm) used by the small-line removal threshold.
    pub min_width: f64,
    /// Gates whether the composed beading-strategy stack's thin-wall
    /// decorator (`WideningBeadingStrategy`) is wrapped in at all. Maps to
    /// the `detect_thin_wall` config key (packet 112, Step 9C).
    pub print_thin_walls: bool,
    /// Threshold (mm) below which the thin-wall decorator reports no beads
    /// at all. Maps to the `min_feature_size` config key.
    pub min_feature_size: f64,
    /// Minimum bead width (mm) the thin-wall decorator clamps its emitted
    /// bead up to. Maps to the `min_bead_width` config key.
    pub min_bead_width: f64,
    /// Transition-ramp length (mm) for the base distribution strategy. Maps
    /// to the `wall_transition_length` config key.
    pub wall_transition_length: f64,
    /// Transition angle (radians) used by beading strategies that reject a
    /// transition when the turn exceeds this angle. Converted from the
    /// `wall_transition_angle` config key (degrees) by
    /// `arachne_params_from_config`.
    pub wall_transition_angle: f64,
    /// Minimum bead width (mm) for the initial layer, overriding the general
    /// thin-wall clamp where the strategy supports layer-specific output.
    /// Maps to the `initial_layer_min_bead_width` config key.
    pub initial_layer_min_bead_width: f64,
    /// Inward offset (mm) applied to the outer wall's toolpath location.
    /// Maps to the `outer_wall_offset` config key.
    pub outer_wall_offset: f64,
    /// Whether this run corresponds to the initial layer, which lets layer-
    /// aware beading strategies override `min_output_width` with
    /// `initial_layer_min_bead_width`.
    pub is_initial_layer: bool,
    /// Whether this run corresponds to the bottom layer of the object (layer
    /// index 0 in object coordinates). Mirrors `is_initial_layer` but kept
    /// distinct (per packet 152): the classic "first/last layer" small-line
    /// threshold keys on this alone. Both fire on layer 0.
    pub is_bottom_layer: bool,
    /// Whether this run corresponds to the topmost shell of the object
    /// (`top_shell_index == Some(0)`, Orca's `upper_slices == nullptr`).
    /// Region metadata, not a layer-wide property — only meaningful in the
    /// per-region module path. Combined with `is_bottom_layer` it derives
    /// `is_top_or_bottom_layer` for the G10 lenient small-line threshold.
    pub is_topmost_layer: bool,
    /// Squared distance gate (mm²) from meshfix_maximum_resolution.
    pub smallest_line_segment_squared: f64,
    /// Squared error distance gate (mm²) from meshfix_maximum_deviation.
    pub allowed_error_distance_squared: f64,
    /// Area deviation threshold (mm²) for near-colinear fast-path guard.
    pub maximum_extrusion_area_deviation: f64,
    /// Wall emission sequence, matching the core Arachne pipeline.
    pub wall_sequence: WallSequence,
}

impl Default for ArachneParams {
    /// Mirrors `slicer_core::arachne::pipeline::ArachneParams::default()`
    /// exactly (see that type's own doc comment for the rationale behind
    /// each default value).
    fn default() -> Self {
        Self {
            optimal_width: 0.4,
            preferred_bead_width_outer: 0.4,
            max_bead_count: 9,
            distribution_count: 1,
            transition_filter_dist: 0.1,
            min_central_distance: 0.0,
            visvalingam_area_threshold: 0.01,
            min_length_factor: 0.5,
            min_width: 0.4,
            print_thin_walls: false,
            min_feature_size: 0.1,
            min_bead_width: 0.4,
            wall_transition_length: 0.4,
            wall_transition_angle: 10.0_f64.to_radians(),
            initial_layer_min_bead_width: 0.34,
            outer_wall_offset: 0.0,
            is_initial_layer: false,
            is_bottom_layer: false,
            is_topmost_layer: false,
            smallest_line_segment_squared: 0.0025,
            allowed_error_distance_squared: 0.000025,
            maximum_extrusion_area_deviation: 0.005,
            wall_sequence: WallSequence::InnerOuter,
        }
    }
}

/// Runs the full Arachne beading-strategy pipeline over `polygons`, producing
/// variable-width wall [`slicer_ir::ExtrusionLine`]s.
///
/// On native targets this delegates directly to
/// `slicer_core::arachne::pipeline::run_arachne_pipeline` (host-algos
/// feature). On wasm32 targets the voronoi/SKT stack is unavailable so the
/// call is forwarded to the host through the
/// `slicer:common/host-services#generate-arachne-walls` WIT import —
/// mirroring [`medial_axis`]'s own native-vs-wasm32 split.
pub fn generate_arachne_walls(
    polygons: &[ExPolygon],
    params: &ArachneParams,
) -> Result<(Vec<slicer_ir::ExtrusionLine>, Vec<slicer_ir::ExtrusionLine>), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let core_params = slicer_core::arachne::pipeline::ArachneParams {
            optimal_width: params.optimal_width,
            preferred_bead_width_outer: params.preferred_bead_width_outer,
            max_bead_count: params.max_bead_count,
            distribution_count: params.distribution_count,
            transition_filter_dist: params.transition_filter_dist,
            min_central_distance: params.min_central_distance,
            visvalingam_area_threshold: params.visvalingam_area_threshold,
            min_length_factor: params.min_length_factor,
            min_width: params.min_width,
            print_thin_walls: params.print_thin_walls,
            min_feature_size: params.min_feature_size,
            min_bead_width: params.min_bead_width,
            wall_transition_length: params.wall_transition_length,
            wall_transition_angle: params.wall_transition_angle,
            initial_layer_min_bead_width: params.initial_layer_min_bead_width,
            outer_wall_offset: params.outer_wall_offset,
            is_initial_layer: params.is_initial_layer,
            is_bottom_layer: params.is_bottom_layer,
            is_topmost_layer: params.is_topmost_layer,
            smallest_line_segment_squared: params.smallest_line_segment_squared,
            allowed_error_distance_squared: params.allowed_error_distance_squared,
            maximum_extrusion_area_deviation: params.maximum_extrusion_area_deviation,
            wall_sequence: params.wall_sequence,
        };
        slicer_core::arachne::pipeline::run_arachne_pipeline(
            polygons,
            &core_params,
            core_params.is_initial_layer,
        )
        .map_err(|e| format!("{:?}", e))
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On wasm32, invoke the host import generated by wit-bindgen. As with
        // `medial_axis`'s equivalent inline binding, this is a self-contained
        // mini WIT world scoped to just this one function — it does not
        // depend on the full world generated by the `#[slicer_module]` macro
        // (private inner module, not accessible from this crate). Component
        // imports resolve structurally against the real host runtime's
        // `slicer:common/host-services` interface, so this mini world's
        // internal organization (e.g. placing `extrusion-line` under
        // `slicer:types/geometry` here vs. `slicer:ir-handles/ir-handles` in
        // the canonical schema) does not need to match — only the wire shape
        // does.
        #[allow(dead_code)]
        mod __sdk_host_arachne_import {
            ::wit_bindgen::generate!({
                inline: r#"
package slicer:sdk-arachne-helper;

package slicer:types {
    interface geometry {
        record point2 { x: s64, y: s64 }
        record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32, overhang-quartile: option<u8> }
        record polygon       { points: list<point2> }
        record ex-polygon    { contour: polygon, holes: list<polygon> }
        record extrusion-junction { p: point3-with-width, perimeter-index: u32 }
        record extrusion-line { junctions: list<extrusion-junction>, inset-idx: u32, is-odd: bool, is-closed: bool }
    }
}

package slicer:common {
    interface host-services {
        use slicer:types/geometry.{ex-polygon, extrusion-line};
        enum wall-sequence { inner-outer, outer-inner, inner-outer-inner }
        record arachne-params {
            optimal-width: f32,
            preferred-bead-width-outer: f32,
            max-bead-count: u32,
            distribution-count: u32,
            transition-filter-dist: f32,
            min-central-distance: f32,
            visvalingam-area-threshold: f32,
            min-length-factor: f32,
            min-width: f32,
            print-thin-walls: bool,
            min-feature-size: f32,
            min-bead-width: f32,
            wall-transition-length: f32,
            wall-transition-angle: f32,
            initial-layer-min-bead-width: f32,
            outer-wall-offset: f32,
            is-initial-layer: bool,
            is-bottom-layer: bool,
            is-topmost-layer: bool,
            smallest-line-segment-squared: f32,
            allowed-error-distance-squared: f32,
            maximum-extrusion-area-deviation: f32,
            wall-sequence: wall-sequence,
        }
        generate-arachne-walls: func(polygons: list<ex-polygon>, params: arachne-params) -> result<tuple<list<extrusion-line>, list<extrusion-line>>, string>;
    }
}

world sdk-arachne {
    import slicer:common/host-services;
}
"#,
                world: "sdk-arachne",
                generate_all,
            });
        }

        let wit_polygons: Vec<__sdk_host_arachne_import::slicer::types::geometry::ExPolygon> =
            polygons
                .iter()
                .map(
                    |input| __sdk_host_arachne_import::slicer::types::geometry::ExPolygon {
                        contour: __sdk_host_arachne_import::slicer::types::geometry::Polygon {
                            points: input
                                .contour
                                .points
                                .iter()
                                .map(|p| {
                                    __sdk_host_arachne_import::slicer::types::geometry::Point2 {
                                        x: p.x,
                                        y: p.y,
                                    }
                                })
                                .collect(),
                        },
                        holes: input
                            .holes
                            .iter()
                            .map(
                                |h| {
                                    __sdk_host_arachne_import::slicer::types::geometry::Polygon {
                                points: h
                                    .points
                                    .iter()
                                    .map(|p| {
                                        __sdk_host_arachne_import::slicer::types::geometry::Point2 {
                                            x: p.x,
                                            y: p.y,
                                        }
                                    })
                                    .collect(),
                            }
                                },
                            )
                            .collect(),
                    },
                )
                .collect();

        let wit_params = __sdk_host_arachne_import::slicer::common::host_services::ArachneParams {
            optimal_width: params.optimal_width as f32,
            preferred_bead_width_outer: params.preferred_bead_width_outer as f32,
            max_bead_count: params.max_bead_count,
            distribution_count: params.distribution_count,
            transition_filter_dist: params.transition_filter_dist as f32,
            min_central_distance: params.min_central_distance as f32,
            visvalingam_area_threshold: params.visvalingam_area_threshold as f32,
            min_length_factor: params.min_length_factor as f32,
            min_width: params.min_width as f32,
            print_thin_walls: params.print_thin_walls,
            min_feature_size: params.min_feature_size as f32,
            min_bead_width: params.min_bead_width as f32,
            wall_transition_length: params.wall_transition_length as f32,
            wall_transition_angle: params.wall_transition_angle as f32,
            initial_layer_min_bead_width: params.initial_layer_min_bead_width as f32,
            outer_wall_offset: params.outer_wall_offset as f32,
            is_initial_layer: params.is_initial_layer,
            is_bottom_layer: params.is_bottom_layer,
            is_topmost_layer: params.is_topmost_layer,
            smallest_line_segment_squared: params.smallest_line_segment_squared as f32,
            allowed_error_distance_squared: params.allowed_error_distance_squared as f32,
            maximum_extrusion_area_deviation: params.maximum_extrusion_area_deviation as f32,
            wall_sequence: match params.wall_sequence {
                WallSequence::InnerOuter => {
                    __sdk_host_arachne_import::slicer::common::host_services::WallSequence::InnerOuter
                }
                WallSequence::OuterInner => {
                    __sdk_host_arachne_import::slicer::common::host_services::WallSequence::OuterInner
                }
                WallSequence::InnerOuterInner => {
                    __sdk_host_arachne_import::slicer::common::host_services::WallSequence::InnerOuterInner
                }
            },
        };

        let result =
            __sdk_host_arachne_import::slicer::common::host_services::generate_arachne_walls(
                &wit_polygons,
                wit_params,
            );

        result.map(|(toolpaths, inner_contour)| {
            let to_ir_lines =
                |lines: Vec<__sdk_host_arachne_import::slicer::types::geometry::ExtrusionLine>| {
                    lines
                        .into_iter()
                        .map(|line| slicer_ir::ExtrusionLine {
                            junctions: line
                                .junctions
                                .into_iter()
                                .map(|j| slicer_ir::ExtrusionJunction {
                                    p: slicer_ir::Point3WithWidth {
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
            (to_ir_lines(toolpaths), to_ir_lines(inner_contour))
        })
    }
}

// ── Time ────────────────────────────────────────────────────────────────

fn process_start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Returns a monotonic microsecond timestamp measured from the SDK's
/// process-start `Instant`. Matches the host's per-call deterministic
/// profiling baseline (`HostExecutionContext::start_time.elapsed()`).
#[must_use]
pub fn now_us() -> u64 {
    process_start().elapsed().as_micros() as u64
}

// ── Test support ────────────────────────────────────────────────────────

/// Test hooks for installing log capture and mesh sources. Production
/// embedders install a real `MeshSource`; tests use these helpers to
/// observe wrapper behavior without crossing the WASM boundary.
pub mod test_support {
    use super::*;

    /// Install a per-thread log capture sink. Subsequent `log*` calls on
    /// this thread append to the sink instead of writing to stderr.
    pub fn install_log_capture() {
        LOG_CAPTURE.with(|cell| *cell.borrow_mut() = Some(Vec::new()));
    }

    /// Drain captured log messages and uninstall the sink.
    pub fn take_log_messages() -> Vec<(LogLevel, String)> {
        LOG_CAPTURE.with(|cell| cell.borrow_mut().take().unwrap_or_default())
    }

    /// Install a per-thread [`MeshSource`].
    pub fn install_mesh_source<S: MeshSource>(source: S) {
        MESH_SOURCE.with(|cell| *cell.borrow_mut() = Some(Box::new(source)));
    }

    /// Uninstall the per-thread mesh source.
    pub fn clear_mesh_source() {
        MESH_SOURCE.with(|cell| *cell.borrow_mut() = None);
    }
}
