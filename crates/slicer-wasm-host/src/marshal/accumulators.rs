//! Accumulator structs for collected guest output.
//!
//! These structs hold the data emitted by guest WASM modules during a single
//! dispatch call, before it is committed back into the IR. Builder methods
//! (the `impl HostXxx for HostExecutionContext` blocks) live in `host.rs`;
//! only the data containers live here.
//!
//! No wasm-runtime dependency is permitted in this module (AC-2): marshalling
//! is pure IR<->WIT data translation, free of the embedder runtime types.

use super::origin::OriginId;

use crate::host::{ExPolygon, ExtrusionPath3d, GcodeMoveCmd, Point3, RegionKey, WallLoopView};

// ---------------------------------------------------------------------------
// InfillOutputCollected
// ---------------------------------------------------------------------------

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
    pub sparse_path_origins: Vec<Option<OriginId>>,
    /// Origin tags parallel to `solid_paths`.
    pub solid_path_origins: Vec<Option<OriginId>>,
    /// Origin tags parallel to `ironing_paths`.
    pub ironing_path_origins: Vec<Option<OriginId>>,
}

// ---------------------------------------------------------------------------
// PerimeterOutputCollected
// ---------------------------------------------------------------------------

/// Collected output from a perimeter-output-builder during a call.
#[derive(Debug, Default)]
pub struct PerimeterOutputCollected {
    /// Wall loops emitted by the guest.
    pub wall_loops: Vec<WallLoopView>,
    /// Wall loops with the seam at points[0] — rotated by seam-placer.
    pub rotated_wall_loops: Vec<WallLoopView>,
    /// Origin tags parallel to `rotated_wall_loops`.
    pub rotated_wall_loop_origins: Vec<Option<OriginId>>,
    /// Infill areas set by the guest, one entry per `set_infill_areas` call.
    ///
    /// Each call corresponds to one `(object_id, region_id)` origin — see
    /// [`PerimeterOutputCollected::infill_areas_origins`]. Parallel to
    /// `wall_loops` / `wall_loop_origins`.
    ///
    /// Was previously a single `Vec<ExPolygon>` + `Option<OriginId>` that got
    /// REPLACED on every `set_infill_areas` call. That single-shot shape
    /// caused `convert_perimeter_output` to emit infill for exactly one
    /// `(object_id, region_id)` bucket — the LAST region's origin in
    /// dispatch order — and silently dropped infill for every other region.
    /// Downstream `sync_perimeter_infill_areas_into_slice` then computed
    /// `sparse_infill_area = []` for those (N-1) regions, producing the
    /// visible "missing infill across internal painted regions" bug in
    /// multi-color prints (e.g. `resources/cube_4color.3mf`).
    pub infill_areas: Vec<Vec<ExPolygon>>,
    /// Seam candidates emitted by the guest.
    pub seam_candidates: Vec<(Point3, f32)>,
    /// Resolved seam position set by the guest (e.g. by seam-placer).
    pub resolved_seam: Option<(Point3, u32)>,
    /// Origin tag for the most recent `push_resolved_seam` call.
    pub resolved_seam_origin: Option<OriginId>,
    /// Origin tags parallel to `wall_loops`.
    pub wall_loop_origins: Vec<Option<OriginId>>,
    /// Origin tags parallel to `infill_areas` (one per `set_infill_areas` call).
    pub infill_areas_origins: Vec<Option<OriginId>>,
    /// Origin tags parallel to `seam_candidates`.
    pub seam_candidate_origins: Vec<Option<OriginId>>,
}

// ---------------------------------------------------------------------------
// SupportOutputCollected
// ---------------------------------------------------------------------------

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
    pub support_path_origins: Vec<Option<OriginId>>,
    /// Origin tags parallel to `interface_paths`.
    pub interface_path_origins: Vec<Option<OriginId>>,
    /// Origin tags parallel to `raft_paths`.
    pub raft_path_origins: Vec<Option<OriginId>>,
}

// ---------------------------------------------------------------------------
// GcodeOutputCollected + GcodeCommandCollected
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// SlicePostprocessCollected
// ---------------------------------------------------------------------------

/// Collected output from a slice-postprocess-builder during a call.
#[derive(Debug, Default)]
pub struct SlicePostprocessCollected {
    /// Polygon updates: (region_key, polygons).
    pub polygon_updates: Vec<(RegionKey, Vec<ExPolygon>)>,
    /// Path Z updates: (region_key, path_idx, vertex_idx, z).
    pub path_z_updates: Vec<(RegionKey, u32, u32, f32)>,
}
