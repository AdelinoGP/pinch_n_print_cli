//! Shared witness codec for test-guest ↔ host-test signal passing.
//!
//! The guest→host test signal is smuggled through positional `Point3WithWidth`
//! fields because that is the only observable output type the WIT boundary
//! exposes.  This crate defines named structs for each layout so the field
//! meanings are declared once; both the producer (SDK guest) and the consumer
//! (host test) reference these definitions instead of maintaining parallel
//! comments.
//!
//! # Layouts defined
//!
//! * [`SdkInfillWitness`]       — `sdk-layer-infill-guest` infill run output
//! * [`SdkInfillWitnessPoint1`] — second point of the same path
//! * [`SdkFinalizationLayerWitness`]  — `sdk-finalization-guest` per-layer summary (point[0])
//! * [`SdkFinalizationLayerWitness1`] — second point carrying first-entity / first-z-hop data
//! * [`RawInfillWitness`]       — raw `layer-infill-guest` infill point[0] layout
//!   (consumer-side decode only; the raw guest's encode side stays as-is in source)
//!
//! # wasm32 safety
//! This crate depends only on `slicer-ir` (which itself uses std).  It
//! compiles for both native and `wasm32-unknown-unknown` targets because
//! `slicer-ir` is already used by all guest crates on that target.

use slicer_ir::Point3WithWidth;

// ── helpers ──────────────────────────────────────────────────────────────────

fn point(x: f32, y: f32, z: f32, width: f32, flow_factor: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width,
        flow_factor,
        overhang_quartile: None,
    }
}

// ── SdkInfillWitness ─────────────────────────────────────────────────────────

/// Decoded content of `point[0]` emitted by `sdk-layer-infill-guest::run_infill`.
///
/// Encoding:
/// ```text
/// point[0].x           = region_count
/// point[0].y           = total polygon count across all regions
/// point[0].z           = first region's z (0.0 if no regions)
/// point[0].width       = first region's effective_layer_height (0.0 if none)
/// point[0].flow_factor = first region's infill_areas().len() as f32 (0.0 if none)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SdkInfillWitness {
    pub region_count: f32,
    pub total_polys: f32,
    pub first_region_z: f32,
    pub first_region_layer_height: f32,
    pub first_region_infill_areas_len: f32,
}

/// Decoded content of `point[1]` emitted by `sdk-layer-infill-guest::run_infill`.
///
/// Encoding:
/// ```text
/// point[1].x           = layer_index as f32 (proves typed u32 arrives independently)
/// point[1].width       = 0.4 (padding sentinel)
/// point[1].flow_factor = 1.0 (padding sentinel)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SdkInfillWitnessPoint1 {
    pub layer_index: f32,
}

impl SdkInfillWitness {
    /// Encode into the two `Point3WithWidth` values that form `path.points`.
    pub fn encode(&self, layer_index: u32) -> Vec<Point3WithWidth> {
        vec![
            point(
                self.region_count,
                self.total_polys,
                self.first_region_z,
                self.first_region_layer_height,
                self.first_region_infill_areas_len,
            ),
            point(layer_index as f32, 0.0, 0.0, 0.4, 1.0),
        ]
    }

    /// Decode `point[0]` from a path's points slice.
    ///
    /// # Panics
    /// Panics if `points` has fewer than 1 element.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let p = &points[0];
        Self {
            region_count: p.x,
            total_polys: p.y,
            first_region_z: p.z,
            first_region_layer_height: p.width,
            first_region_infill_areas_len: p.flow_factor,
        }
    }
}

impl SdkInfillWitnessPoint1 {
    /// Decode `point[1]` from a path's points slice.
    ///
    /// # Panics
    /// Panics if `points` has fewer than 2 elements.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let p = &points[1];
        Self { layer_index: p.x }
    }
}

// ── SdkFinalizationLayerWitness ───────────────────────────────────────────────

/// Decoded content of `point[0]` for one layer, as emitted by
/// `sdk-finalization-guest::run_finalization` per layer it processes.
///
/// Encoding:
/// ```text
/// point[0].x           = layer.layer_index() as f32
/// point[0].y           = layer.z()
/// point[0].z           = layer.entity_count() as f32
/// point[0].width       = tool_changes.len() as f32
/// point[0].flow_factor = z_hops.len() as f32
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SdkFinalizationLayerWitness {
    pub layer_index: f32,
    pub layer_z: f32,
    pub entity_count: f32,
    pub tool_changes_len: f32,
    pub z_hops_len: f32,
}

/// Decoded content of `point[1]` for one layer emitted by
/// `sdk-finalization-guest::run_finalization`.
///
/// Encoding:
/// ```text
/// point[1].x           = first ordered entity's topo_order as f32 (-1.0 if empty)
/// point[1].y           = first entity's path.points.len() as f32  (-1.0 if empty)
/// point[1].z           = first entity's path.speed_factor          (-1.0 if empty)
/// point[1].width       = first z-hop's after_entity_index as f32   (-1.0 if none)
/// point[1].flow_factor = first z-hop's hop_height                   (-1.0 if none)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SdkFinalizationLayerWitness1 {
    pub first_entity_topo: f32,
    pub first_entity_point_count: f32,
    pub first_entity_speed_factor: f32,
    pub first_zhop_after_entity: f32,
    pub first_zhop_height: f32,
}

impl SdkFinalizationLayerWitness {
    /// Encode into the two `Point3WithWidth` values that form one layer's marker path points.
    pub fn encode(&self, p1: &SdkFinalizationLayerWitness1) -> Vec<Point3WithWidth> {
        vec![
            point(
                self.layer_index,
                self.layer_z,
                self.entity_count,
                self.tool_changes_len,
                self.z_hops_len,
            ),
            point(
                p1.first_entity_topo,
                p1.first_entity_point_count,
                p1.first_entity_speed_factor,
                p1.first_zhop_after_entity,
                p1.first_zhop_height,
            ),
        ]
    }

    /// Decode `point[0]` from a path's points slice.
    ///
    /// # Panics
    /// Panics if `points` is empty.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let p = &points[0];
        Self {
            layer_index: p.x,
            layer_z: p.y,
            entity_count: p.z,
            tool_changes_len: p.width,
            z_hops_len: p.flow_factor,
        }
    }
}

impl SdkFinalizationLayerWitness1 {
    /// Decode `point[1]` from a path's points slice.
    ///
    /// # Panics
    /// Panics if `points` has fewer than 2 elements.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let p = &points[1];
        Self {
            first_entity_topo: p.x,
            first_entity_point_count: p.y,
            first_entity_speed_factor: p.z,
            first_zhop_after_entity: p.width,
            first_zhop_height: p.flow_factor,
        }
    }
}

// ── RawInfillWitness ──────────────────────────────────────────────────────────

/// Decoded content of `point[0]` from the raw (non-SDK) `layer-infill-guest::run_infill`.
///
/// That guest's encoding (note: different field order from SDK layout):
/// ```text
/// point[0].z           = first region's z
/// point[0].width       = total polygon count across all regions
/// point[0].flow_factor = region count as f32
/// point[1].x           = infill_spacing * 10.0 (from config)
/// ```
///
/// This struct is consumer-side only; the raw guest encode is not modified
/// by this packet (it is not in the allowed-edit list).
#[derive(Debug, Clone, PartialEq)]
pub struct RawInfillWitness {
    pub first_region_z: f32,
    pub total_polys: f32,
    pub region_count: f32,
}

/// Decoded content of `point[1]` from the raw `layer-infill-guest`.
#[derive(Debug, Clone, PartialEq)]
pub struct RawInfillWitnessPoint1 {
    /// infill_spacing * 10.0
    pub spacing_x10: f32,
}

impl RawInfillWitness {
    /// Decode `point[0]` from a path's points slice.
    ///
    /// # Panics
    /// Panics if `points` is empty.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let p = &points[0];
        Self {
            first_region_z: p.z,
            total_polys: p.width,
            region_count: p.flow_factor,
        }
    }
}

impl RawInfillWitnessPoint1 {
    /// Decode `point[1]` from a path's points slice.
    ///
    /// # Panics
    /// Panics if `points` has fewer than 2 elements.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        Self {
            spacing_x10: points[1].x,
        }
    }
}

// ── RawSupportWitness ─────────────────────────────────────────────────────────

/// Decoded content of `point[0]` from the raw `layer-infill-guest::run_support`.
///
/// Encoding:
/// ```text
/// point[0].x           = enforcer region count as f32
/// point[0].y           = blocker region count as f32
/// point[0].z           = first slice region's z
/// point[0].flow_factor = paint layer index as f32
/// ```
///
/// Consumer-side only; the raw guest encode is not in the allowed-edit list.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSupportWitness {
    pub enforcer_count: f32,
    pub blocker_count: f32,
    pub first_region_z: f32,
    pub paint_layer_index: f32,
}

impl RawSupportWitness {
    /// Decode `point[0]` from a support path's points slice.
    ///
    /// # Panics
    /// Panics if `points` is empty.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let p = &points[0];
        Self {
            enforcer_count: p.x,
            blocker_count: p.y,
            first_region_z: p.z,
            paint_layer_index: p.flow_factor,
        }
    }
}

// ── SliceRegionFieldsWitness ─────────────────────────────────────────────────
//
// Covers every field on `slicer_sdk::views::SliceRegionView` that has both a
// host-side WIT accessor and an SDK setter. Used by the
// `adapt_slice_regions_completeness_tdd` integration test to detect
// "the macro-generated `__slicer_adapt_slice_regions` adapter forgot
// field X". Pre-fix the adapter dropped `sparse_infill_area`, `bridge_areas`,
// `bridge_orientation_deg`, and `held_claims`.
//
// Encoding lives in 4 `Point3WithWidth` values (20 float slots) because the
// 17 covered fields plus content-aware digests for the two `String`-valued
// fields (`object_id`, first `held_claims` entry) won't fit in two points.

/// Decoded content of the field-witness path emitted by
/// `sdk-layer-infill-guest::run_infill` when the config carries
/// `emit_field_witness = 1`.
///
/// Each float is a function of one `SliceRegionView` field. Vector lengths,
/// option discriminants (`Some(_)` → value, `None` → `-1`), and booleans
/// (`true` → `1.0`, `false` → `0.0`) are encoded directly. The two
/// `String`-typed fields (`object_id`, first entry of `held_claims`) are
/// reduced to `(len as f32, byte_sum as f32)` digests so the test can
/// assert both length and content survived without needing a string channel.
///
/// Encoding (four `Point3WithWidth` values laid out as path.points):
/// ```text
/// point[0].x           = object_id.len() as f32
/// point[0].y           = region_id as f32 (u64 → f32)
/// point[0].z           = polygons.len() as f32
/// point[0].width       = infill_areas.len() as f32
/// point[0].flow_factor = effective_layer_height
/// point[1].x           = z
/// point[1].y           = has_nonplanar as 0.0/1.0
/// point[1].z           = segment_annotations.len() as f32
/// point[1].width       = top_shell_index    (Some(n) → n, None → -1)
/// point[1].flow_factor = bottom_shell_index (Some(n) → n, None → -1)
/// point[2].x           = top_solid_fill.len() as f32
/// point[2].y           = bottom_solid_fill.len() as f32
/// point[2].z           = is_bridge as 0.0/1.0
/// point[2].width       = bridge_areas.len() as f32
/// point[2].flow_factor = bridge_orientation_deg
/// point[3].x           = sparse_infill_area.len() as f32
/// point[3].y           = held_claims.len() as f32
/// point[3].z           = first_held_claim_byte_sum (0 if empty)
/// point[3].width       = object_id_byte_sum
/// point[3].flow_factor = sentinel marker (= 1.0 — witnesses path was emitted)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SliceRegionFieldsWitness {
    pub object_id_len: f32,
    pub region_id: f32,
    pub polygons_len: f32,
    pub infill_areas_len: f32,
    pub effective_layer_height: f32,
    pub z: f32,
    pub has_nonplanar: f32,
    pub segment_annotations_len: f32,
    pub top_shell_index: f32,
    pub bottom_shell_index: f32,
    pub top_solid_fill_len: f32,
    pub bottom_solid_fill_len: f32,
    pub is_bridge: f32,
    pub bridge_areas_len: f32,
    pub bridge_orientation_deg: f32,
    pub sparse_infill_area_len: f32,
    pub held_claims_len: f32,
    pub first_held_claim_byte_sum: f32,
    pub object_id_byte_sum: f32,
    pub marker: f32,
}

impl SliceRegionFieldsWitness {
    /// Encode into the five `Point3WithWidth` values that form `path.points`.
    ///
    /// `point[0]` is reserved as an envelope-safe header: its `z` is the
    /// real layer z (so the host's `check_z_envelope` admits the path),
    /// `flow_factor` is the constant `MARKER` (so the decoder can assert it
    /// is the field-witness path and not the legacy `SdkInfillWitness` path),
    /// and the remaining three slots are padding. Witness payload starts at
    /// `point[1]`.
    pub fn encode(&self, layer_z: f32) -> Vec<Point3WithWidth> {
        vec![
            // Envelope-safe header: real z keeps the host's Z guard happy.
            point(0.0, 0.0, layer_z, 0.0, Self::MARKER),
            point(
                self.object_id_len,
                self.region_id,
                self.polygons_len,
                self.infill_areas_len,
                self.effective_layer_height,
            ),
            point(
                self.z,
                self.has_nonplanar,
                self.segment_annotations_len,
                self.top_shell_index,
                self.bottom_shell_index,
            ),
            point(
                self.top_solid_fill_len,
                self.bottom_solid_fill_len,
                self.is_bridge,
                self.bridge_areas_len,
                self.bridge_orientation_deg,
            ),
            point(
                self.sparse_infill_area_len,
                self.held_claims_len,
                self.first_held_claim_byte_sum,
                self.object_id_byte_sum,
                0.0,
            ),
        ]
    }

    /// Decode the witness from a path's points slice. Reads the header at
    /// `points[0]` for the marker and payload at `points[1..5]`.
    ///
    /// # Panics
    /// Panics if `points` has fewer than 5 elements.
    pub fn decode(points: &[Point3WithWidth]) -> Self {
        let header = &points[0];
        let p1 = &points[1];
        let p2 = &points[2];
        let p3 = &points[3];
        let p4 = &points[4];
        Self {
            object_id_len: p1.x,
            region_id: p1.y,
            polygons_len: p1.z,
            infill_areas_len: p1.width,
            effective_layer_height: p1.flow_factor,
            z: p2.x,
            has_nonplanar: p2.y,
            segment_annotations_len: p2.z,
            top_shell_index: p2.width,
            bottom_shell_index: p2.flow_factor,
            top_solid_fill_len: p3.x,
            bottom_solid_fill_len: p3.y,
            is_bridge: p3.z,
            bridge_areas_len: p3.width,
            bridge_orientation_deg: p3.flow_factor,
            sparse_infill_area_len: p4.x,
            held_claims_len: p4.y,
            first_held_claim_byte_sum: p4.z,
            object_id_byte_sum: p4.width,
            marker: header.flow_factor,
        }
    }

    /// Sentinel marker value that distinguishes the field-witness path from
    /// the default infill-content witness path. Set in `point[0].flow_factor`
    /// by the encoder and asserted by the decoder so the wrong path can't
    /// be silently decoded as the wrong layout.
    pub const MARKER: f32 = 7.0;
}
