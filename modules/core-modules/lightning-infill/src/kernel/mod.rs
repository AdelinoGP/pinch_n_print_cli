//! Module-private lightning planning kernel.
//!
//! The planning algorithm and its two plan types are relocated here from the
//! host `crates/slicer-core/src/algos/lightning/` directory so the consuming
//! artifact owns the plan. Planning numeric logic is frozen: the permitted
//! deltas are file location, the plan-type import, `crate::` helper paths
//! becoming `slicer_core::`, the error type's name/path, the additive
//! `planning_input` adapter, and doc comments that named the old public types.

use std::sync::Arc;

use slicer_core::polygon_ops::clip_polylines;
use slicer_ir::{mm_to_units, ExPolygon, PaintValue, Point2, ResolvedConfig, SliceIR};

use self::generator::Generator;

mod distance_field;
mod error;
mod generator;
mod layer;
mod plan;
mod tree_node;

#[cfg(test)]
mod tests;

pub(crate) use error::KernelError;
pub(crate) use plan::{KernelPlan, PlanEntry};

const DEFAULT_NOZZLE_DIAMETER_MM: f32 = 0.4;

/// OrcaSlicer's lightning defaults, recorded identically by the canonical capture.
/// PnP exposes no lightning-specific config keys yet, so these are fixed rather than per-profile.
const ORCA_LIGHTNING_OVERHANG_ANGLE_DEG: f64 = 45.0;
const ORCA_LIGHTNING_PRUNE_ANGLE_DEG: f64 = 5.0;
const ORCA_LIGHTNING_STRAIGHTENING_ANGLE_DEG: f64 = 5.0;

/// Generate the per-object, per-region, per-layer plan for one print.
///
/// The caller supplies the print-wide resolved config. Each `(object_id,
/// region_id)` present in `slice_ir` gets independent generators for its
/// `ExPolygon` islands, so regions on the same layer cannot share tree
/// segments. Per-region holder selection is represented by the single
/// resolved-config input at this seam; preparation only reaches this driver
/// after the shared selection predicate passes.
#[allow(clippy::missing_errors_doc)]
pub(crate) fn generate_lightning_trees(
    slice_ir: &[SliceIR],
    config: &ResolvedConfig,
) -> Result<Arc<KernelPlan>, KernelError> {
    if config.sparse_fill_holder != "lightning-infill" {
        return Ok(Arc::new(KernelPlan::default()));
    }

    let layer_indices: Vec<u32> = slice_ir
        .iter()
        .map(|slice| slice.global_layer_index)
        .collect();
    // Group source regions by their exact ordered identity: two regions that
    // share `(object_id, region_id)` but differ in ordered `variant_chain` are
    // distinct sources and must never share a generator or an output entry.
    let mut region_polygons: std::collections::BTreeMap<
        (String, u64, Vec<(String, PaintValue)>),
        Vec<Vec<ExPolygon>>,
    > = std::collections::BTreeMap::new();

    for slice in slice_ir {
        for region in &slice.regions {
            let polygons = region_polygons
                .entry((
                    region.object_id.clone(),
                    region.region_id,
                    region.variant_chain.clone(),
                ))
                .or_insert_with(|| vec![Vec::new(); layer_indices.len()]);
            if let Some(layer_id) = layer_indices
                .iter()
                .position(|index| *index == slice.global_layer_index)
            {
                polygons[layer_id] = region.polygons.clone();
            }
        }
    }

    let mut entries_by_key: std::collections::BTreeMap<
        (String, i32, u64, Vec<(String, PaintValue)>),
        Vec<[Point2; 2]>,
    > = std::collections::BTreeMap::new();
    for ((object_id, region_id, variant_chain), polygons_by_layer) in region_polygons {
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
            let line_width_mm = slicer_core::flow::resolve_role_width(
                slicer_ir::ExtrusionRole::SparseInfill,
                false,
                false,
                &slicer_core::flow::RoleWidthContext {
                    line_width: config.line_width as f32,
                    nozzle_diameter: DEFAULT_NOZZLE_DIAMETER_MM,
                    ..Default::default()
                },
            );
            let mut generator = Generator::new(
                outlines,
                config.infill_density as f64,
                mm_to_units(line_width_mm),
                1,
                mm_to_units(config.layer_height as f32),
                ORCA_LIGHTNING_OVERHANG_ANGLE_DEG,
                ORCA_LIGHTNING_PRUNE_ANGLE_DEG,
                ORCA_LIGHTNING_STRAIGHTENING_ANGLE_DEG,
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
                        .entry((
                            object_id.clone(),
                            *global_layer_index as i32,
                            region_id,
                            variant_chain.clone(),
                        ))
                        .or_default()
                        .extend(polyline.windows(2).map(|pair| [pair[0], pair[1]]));
                }
            }
        }
    }

    let entries = entries_by_key
        .into_iter()
        .map(
            |((object_id, global_layer_index, region_id, variant_chain), tree_edge_segments)| {
                PlanEntry {
                    object_id,
                    global_layer_index,
                    region_id,
                    tree_edge_segments,
                    variant_chain,
                }
            },
        )
        .collect();

    Ok(Arc::new(KernelPlan { entries }))
}
