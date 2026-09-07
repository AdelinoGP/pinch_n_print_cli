//! Host-prepared, per-region data derived from immutable layer inputs.

use serde::{Deserialize, Serialize};

use crate::{ExPolygon, QuartileBand, SurfaceGroup};

/// Derived values associated with one sliced region in source-region order.
///
/// This is an IR-typed transport record rather than a WIT record. The runtime
/// owns instances in its per-layer arena, and the wasm host may project them
/// onto either transport without introducing a dependency back to the runtime.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PreparedRegionData {
    /// Whether the region overlaps an eligible overhang footprint.
    pub needs_support: bool,
    /// Resolved non-planar surface group, when the region names one.
    pub surface_group: Option<SurfaceGroup>,
    /// Overhang quartile polygons clipped exactly to the region footprint.
    pub overhang_quartile_polygons: Vec<QuartileBand>,
    /// Region-clipped overhang polygons flattened across quartiles.
    pub overhang_areas: Vec<ExPolygon>,
    /// Boundary polygons for this object's previous layer.
    pub prev_layer_boundary: Vec<ExPolygon>,
}
