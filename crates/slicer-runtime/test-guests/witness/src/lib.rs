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
