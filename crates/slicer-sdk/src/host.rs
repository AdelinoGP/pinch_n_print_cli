//! Host service wrappers.
//!
//! These wrappers define a stable API surface for module code.
//! Runtime host bindings are intentionally placeholder-only in this phase.

use std::time::{SystemTime, UNIX_EPOCH};

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

/// Logs a message with the requested level.
pub fn log(level: LogLevel, message: &str) {
    let _ = (level, message);
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

/// Casts a vertical ray downward against an object mesh.
#[must_use]
pub fn raycast_z_down(object_id: &str, x: f32, y: f32, start_z: f32) -> Option<f32> {
    let _ = (object_id, x, y, start_z);
    None
}

/// Queries the surface normal at a 3D point.
#[must_use]
pub fn surface_normal_at(object_id: &str, x: f32, y: f32, z: f32) -> Option<Point3> {
    let _ = (object_id, x, y, z);
    None
}

/// Returns axis-aligned object bounds in millimeters.
#[must_use]
pub fn object_bounds(object_id: &str) -> BoundingBox3 {
    let _ = object_id;
    BoundingBox3 {
        min: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    }
}

/// Applies host-side polygon clipping.
#[must_use]
pub fn clip_polygons(
    subject: &[ExPolygon],
    clip: &[ExPolygon],
    op: ClipOperation,
) -> Vec<ExPolygon> {
    let _ = (subject, clip, op);
    Vec::new()
}

/// Applies host-side polygon offsetting.
#[must_use]
pub fn offset_polygons(
    polygons: &[ExPolygon],
    delta_mm: f32,
    join: OffsetJoinType,
) -> Vec<ExPolygon> {
    let _ = (polygons, delta_mm, join);
    Vec::new()
}

/// Simplifies a polygon using host geometry utilities.
#[must_use]
pub fn simplify_polygon(polygon: &Polygon, tolerance_mm: f32) -> Polygon {
    let _ = tolerance_mm;
    polygon.clone()
}

/// Returns a monotonic-like microsecond timestamp for lightweight profiling.
#[must_use]
pub fn now_us() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let micros = duration.as_micros();
            u64::try_from(micros).unwrap_or(u64::MAX)
        }
        Err(_) => 0,
    }
}
