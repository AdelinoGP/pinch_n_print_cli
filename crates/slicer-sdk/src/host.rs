//! Host service wrappers.
//!
//! These wrappers are the SDK-side entry points that mirror the WIT
//! `host-services` interface (see `wit/host-api.wit` and
//! `crates/slicer-host/src/wit_host.rs`). They are no longer silent
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
    slicer_core::polygon_ops::offset(polygons, delta_mm, to_core_join(join))
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
