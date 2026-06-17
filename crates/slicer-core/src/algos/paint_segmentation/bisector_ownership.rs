// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! External-contour tagging for paint-segmented regions (AC-22b).
//!
//! # Background
//!
//! When paint segmentation decomposes an object's cross-section into cells (one
//! per paint colour, plus an unpainted base region for the bulk), adjacent cells
//! share interface boundaries. A per-region perimeter generator naively emits an
//! outer wall along *every* edge of each cell polygon, including shared interface
//! edges — tracing each interface twice (once per adjacent cell) and inflating the
//! per-layer outer-wall count far above the unpainted baseline.
//!
//! # Mechanism
//!
//! The fix gives every region of a painted cell group the group's **clean external
//! contour** — the gap-free outer boundary of the whole object cross-section. The
//! perimeter generator keeps an outer-wall edge only when it lies on that contour
//! (a real model-perimeter edge) and skips edges interior to it (paint-cell
//! interfaces). Each cell owns only its slice of the perimeter, so each interface
//! is emitted by no cell and the model perimeter is traced exactly once.
//!
//! # Why the boundary is computed host-side
//!
//! The boundary is derived from the **pre-segmentation** slice (`original`), whose
//! per-object regions are the un-partitioned, gap-free cross-section. Unioning an
//! object's original polygons with [`union_ex`] yields the exact model perimeter.
//! This MUST run on the host: boolean polygon ops (`union_ex`) are reliable here
//! but are effectively no-ops inside the WASM perimeter guest, which is why the
//! boundary is shipped to the guest as `SlicedRegion::external_contour` rather than
//! recomputed there.
//!
//! # Invariants
//!
//! - Fully-unpainted layer (no region has a non-empty `variant_chain`) → every
//!   region's `external_contour = None`; the guest traces each polygon in full
//!   (byte-identical output on unpainted slices, AC-8a).
//! - Painted layer → every region whose `object_id` has at least one painted
//!   region on that layer receives `Some(boundary)`; regions of fully-unpainted
//!   objects in the same layer keep `None`.

use std::collections::{BTreeMap, BTreeSet};

use slicer_ir::{ExPolygon, ObjectId, SliceIR};

use crate::polygon_ops::union_ex;

/// Populate `external_contour` on every region of every painted layer.
///
/// `original[i]` is the pre-segmentation slice for layer `i` (the un-painted,
/// gap-free cross-section). Per object, the union of its original polygons is the
/// clean model perimeter assigned to every cell of that object on that layer.
///
/// Must be called **after** variant-composition writes `working[i].regions` and
/// **before** Phase 5 width-limiting, so the contour reflects pre-erosion geometry.
pub fn populate_external_contours(working: &mut [SliceIR], original: &[SliceIR]) {
    for (layer_idx, layer) in working.iter_mut().enumerate() {
        // Fully-unpainted layer: every region traces its own polygon.
        if !layer.regions.iter().any(|r| !r.variant_chain.is_empty()) {
            for region in layer.regions.iter_mut() {
                region.external_contour = None;
            }
            continue;
        }

        // Objects that carry at least one painted cell on this layer.
        let painted_objects: BTreeSet<ObjectId> = layer
            .regions
            .iter()
            .filter(|r| !r.variant_chain.is_empty())
            .map(|r| r.object_id.clone())
            .collect();

        // Clean per-object boundary from the pre-segmentation slice: union of each
        // object's original (un-partitioned) polygons.
        let mut object_boundary: BTreeMap<ObjectId, Vec<ExPolygon>> = BTreeMap::new();
        if let Some(orig_layer) = original.get(layer_idx) {
            let mut polys_by_object: BTreeMap<ObjectId, Vec<ExPolygon>> = BTreeMap::new();
            for r in &orig_layer.regions {
                if painted_objects.contains(&r.object_id) {
                    polys_by_object
                        .entry(r.object_id.clone())
                        .or_default()
                        .extend(r.polygons.iter().cloned());
                }
            }
            for (object_id, polys) in polys_by_object {
                object_boundary.insert(object_id, union_ex(&polys));
            }
        }

        for region in layer.regions.iter_mut() {
            region.external_contour = if painted_objects.contains(&region.object_id) {
                object_boundary.get(&region.object_id).cloned()
            } else {
                None
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{PaintValue, Point2, Polygon, SliceIR, SlicedRegion};

    /// Rectangular `ExPolygon` with integer (100 nm unit) coordinates, CCW.
    fn sq(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn painted_region(object_id: &str, polygons: Vec<ExPolygon>, tool: u32) -> SlicedRegion {
        SlicedRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            polygons,
            variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(tool))],
            ..Default::default()
        }
    }

    fn unpainted_region(object_id: &str, polygons: Vec<ExPolygon>) -> SlicedRegion {
        SlicedRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            polygons,
            ..Default::default()
        }
    }

    fn layer(regions: Vec<SlicedRegion>) -> SliceIR {
        SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions,
        }
    }

    /// Bounding box of the union of every contour point across a boundary.
    fn bbox(boundary: &[ExPolygon]) -> (i64, i64, i64, i64) {
        let mut xmin = i64::MAX;
        let mut ymin = i64::MAX;
        let mut xmax = i64::MIN;
        let mut ymax = i64::MIN;
        for ep in boundary {
            for p in &ep.contour.points {
                xmin = xmin.min(p.x);
                ymin = ymin.min(p.y);
                xmax = xmax.max(p.x);
                ymax = ymax.max(p.y);
            }
        }
        (xmin, ymin, xmax, ymax)
    }

    /// Two abutting painted cells of one object get the union (full rectangle)
    /// as their shared external contour.
    #[test]
    fn abutting_cells_share_object_external_contour() {
        let left = painted_region("cube", vec![sq(0, 0, 10_000, 10_000)], 0);
        let right = painted_region("cube", vec![sq(10_000, 0, 20_000, 10_000)], 1);
        // Pre-segmentation slice: one gap-free 2×1 mm rectangle for "cube".
        let original = vec![layer(vec![unpainted_region(
            "cube",
            vec![sq(0, 0, 20_000, 10_000)],
        )])];
        let mut working = vec![layer(vec![left, right])];

        populate_external_contours(&mut working, &original);

        for region in &working[0].regions {
            let b = region
                .external_contour
                .as_ref()
                .expect("painted cell must carry its object's external contour");
            assert_eq!(
                bbox(b),
                (0, 0, 20_000, 10_000),
                "boundary is the merged rectangle"
            );
        }
    }

    /// Fully-unpainted layer: every region's external contour stays `None`.
    #[test]
    fn fully_unpainted_layer_all_none() {
        let original = vec![layer(vec![unpainted_region(
            "cube",
            vec![sq(0, 0, 20_000, 10_000)],
        )])];
        let mut working = vec![layer(vec![
            unpainted_region("cube", vec![sq(0, 0, 10_000, 10_000)]),
            unpainted_region("cube", vec![sq(10_000, 0, 20_000, 10_000)]),
        ])];

        populate_external_contours(&mut working, &original);

        for region in &working[0].regions {
            assert!(region.external_contour.is_none(), "unpainted layer → None");
        }
    }

    /// A fully-unpainted object sharing a painted layer keeps `None`; the painted
    /// object gets its boundary.
    #[test]
    fn unpainted_object_in_painted_layer_keeps_none() {
        let original = vec![layer(vec![
            unpainted_region("painted", vec![sq(0, 0, 20_000, 10_000)]),
            unpainted_region("plain", vec![sq(50_000, 0, 60_000, 10_000)]),
        ])];
        let mut working = vec![layer(vec![
            painted_region("painted", vec![sq(0, 0, 10_000, 10_000)], 1),
            painted_region("painted", vec![sq(10_000, 0, 20_000, 10_000)], 2),
            unpainted_region("plain", vec![sq(50_000, 0, 60_000, 10_000)]),
        ])];

        populate_external_contours(&mut working, &original);

        assert!(
            working[0].regions[0].external_contour.is_some(),
            "painted object cell"
        );
        assert!(
            working[0].regions[1].external_contour.is_some(),
            "painted object cell"
        );
        assert!(
            working[0].regions[2].external_contour.is_none(),
            "fully-unpainted object stays None even in a painted layer"
        );
    }
}
