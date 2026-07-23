//! Lightning sparse-infill tree generator.
//!
//! Contract landed in packet 137; the per-layer 2-point integer-unit
//! `tree_edge_segments` storage and per-object/per-layer `LightningTreeEntry`
//! shape are stable. The actual generator algorithm (distance field +
//! `TreeNode` + cross-layer `Generator`) is ported in packets 138
//! (data structures) and 139 (wiring into this seam).

use std::sync::Arc;

use slicer_ir::{
    mm_to_units, ExPolygon, LightningTreeEntry, LightningTreeIR, Point2, ResolvedConfig, SliceIR,
};

use crate::algos::lightning::error::LightningTreeError;
use crate::algos::lightning::generator::Generator;
use crate::polygon_ops::clip_polylines;

/// Discrete support-distance field used by the Lightning tree generator.
pub mod distance_field;
pub mod error;
/// Cross-layer initial overhang generation for Lightning sparse infill.
pub mod generator;
/// Per-layer tree seeding and line conversion for Lightning sparse infill.
pub mod layer;
/// Parent/child graph primitive used by the Lightning tree generator.
pub mod tree_node;

pub use distance_field::DistanceField;
pub use generator::generate_initial_internal_overhangs;
pub use layer::Layer;

/// Generate the per-object, per-region, per-layer `LightningTreeIR` for one print.
///
/// The caller supplies the print-wide resolved config. Each `(object_id,
/// region_id)` present in `slice_ir` gets independent generators for its
/// `ExPolygon` islands, so regions on the same layer cannot share tree
/// segments. Per-region holder selection is still represented by the single
/// resolved-config input at this seam; the producer only calls this driver
/// after the prepass lightning predicate passes.
#[allow(clippy::missing_errors_doc)]
pub fn generate_lightning_trees(
    slice_ir: &[SliceIR],
    config: &ResolvedConfig,
) -> Result<Arc<LightningTreeIR>, LightningTreeError> {
    if config.sparse_fill_holder != "lightning-infill" {
        return Ok(Arc::new(LightningTreeIR::default()));
    }

    let layer_indices: Vec<u32> = slice_ir
        .iter()
        .map(|slice| slice.global_layer_index)
        .collect();
    let mut region_polygons: std::collections::BTreeMap<(String, u64), Vec<Vec<ExPolygon>>> =
        std::collections::BTreeMap::new();

    for slice in slice_ir {
        for region in &slice.regions {
            let polygons = region_polygons
                .entry((region.object_id.clone(), region.region_id))
                .or_insert_with(|| vec![Vec::new(); layer_indices.len()]);
            if let Some(layer_id) = layer_indices
                .iter()
                .position(|index| *index == slice.global_layer_index)
            {
                polygons[layer_id] = region.polygons.clone();
            }
        }
    }

    let mut entries_by_key: std::collections::BTreeMap<(String, i32, u64), Vec<[Point2; 2]>> =
        std::collections::BTreeMap::new();
    for ((object_id, region_id), polygons_by_layer) in region_polygons {
        let polygon_count = polygons_by_layer.iter().map(Vec::len).max().unwrap_or(0);
        for polygon_id in 0..polygon_count {
            let outlines = polygons_by_layer
                .iter()
                .map(|polygons| {
                    polygons
                        .get(polygon_id)
                        .map_or_else(Vec::new, |polygon| polygon.contour.points.clone())
                })
                .collect();
            let mut generator = Generator::new(
                outlines,
                config.infill_density as f64,
                mm_to_units(config.line_width),
                1,
                mm_to_units(config.layer_height as f32),
                config.infill_angle as f64,
                5.0,
                0.0,
            );
            generator.generate_trees(&|| {});
            for (layer_id, global_layer_index) in layer_indices.iter().enumerate() {
                let Some(polygon) = polygons_by_layer[layer_id].get(polygon_id) else {
                    continue;
                };
                let polylines: Vec<Vec<Point2>> = generator
                    .committed_segments_for_layer(layer_id)
                    .iter()
                    .map(|[start, end]| vec![*start, *end])
                    .collect();
                for polyline in clip_polylines(&polylines, std::slice::from_ref(polygon)) {
                    entries_by_key
                        .entry((object_id.clone(), *global_layer_index as i32, region_id))
                        .or_default()
                        .extend(polyline.windows(2).map(|pair| [pair[0], pair[1]]));
                }
            }
        }
    }

    let entries = entries_by_key
        .into_iter()
        .map(
            |((object_id, global_layer_index, region_id), tree_edge_segments)| LightningTreeEntry {
                object_id,
                global_layer_index,
                region_id,
                tree_edge_segments,
            },
        )
        .collect();

    Ok(Arc::new(LightningTreeIR {
        entries,
        ..LightningTreeIR::default()
    }))
}
