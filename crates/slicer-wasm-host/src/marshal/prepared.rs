//! Pure preparation of host-neutral per-region data.
//!
//! This lives beside the existing transport adapters because it uses the same
//! polygon kernel and SDK view semantics. `slicer-runtime` already depends on
//! this crate (ADR-0005), while the reverse dependency is forbidden. Step C
//! can make the adapters consume these records; this step intentionally leaves
//! their transport behavior unchanged.

use slicer_ir::{ExPolygon, PreparedRegionData, QuartileBand, SliceIR, SlicedRegion};

/// Prepare derived data for regions in the same order as `regions`.
#[must_use]
pub fn prepare_regions(
    regions: &[SlicedRegion],
    surface_classification: Option<&slicer_ir::SurfaceClassificationIR>,
    global_layer_index: u32,
) -> Vec<PreparedRegionData> {
    let result = regions
        .iter()
        .map(|region| {
            let view = slicer_sdk::views::SliceRegionView::from_ir(region, 0.0, Vec::new());
            let needs_support = view.derive_needs_support(surface_classification);
            let surface_group = region.nonplanar_surface.and_then(|surface_group_id| {
                surface_classification
                    .and_then(|classification| classification.per_object.get(&region.object_id))
                    .and_then(|object| {
                        object
                            .surface_groups
                            .iter()
                            .find(|group| group.id == surface_group_id)
                    })
                    .cloned()
            });

            let region_bbox = expolygons_bbox(&region.polygons);
            let overhang_quartile_polygons = surface_classification
                .and_then(|classification| {
                    classification
                        .overhang_quartile_polygons
                        .get(&region.object_id)
                        .and_then(|by_layer| by_layer.get(&global_layer_index))
                })
                .map(|bands| {
                    bands
                        .iter()
                        .filter_map(|band| {
                            let prefiltered: Vec<ExPolygon> = band
                                .polygons
                                .iter()
                                .filter(|polygon| {
                                    region_bbox.is_some_and(|bbox| bbox_overlaps(bbox, polygon))
                                })
                                .cloned()
                                .collect();
                            if prefiltered.is_empty() {
                                return None;
                            }
                            let polygons = slicer_core::polygon_ops::intersection_ex(
                                &prefiltered,
                                &region.polygons,
                            );
                            (!polygons.is_empty()).then_some(QuartileBand {
                                quartile: band.quartile,
                                polygons,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let overhang_areas = overhang_quartile_polygons
                .iter()
                .flat_map(|band| band.polygons.iter().cloned())
                .collect();
            let prev_layer_boundary = surface_classification
                .and_then(|classification| {
                    classification
                        .prev_layer_boundaries
                        .get(&region.object_id)
                        .and_then(|by_layer| by_layer.get(&global_layer_index))
                })
                .cloned()
                .unwrap_or_default();

            PreparedRegionData {
                needs_support,
                surface_group,
                overhang_quartile_polygons,
                overhang_areas,
                prev_layer_boundary,
            }
        })
        .collect();
    result
}

/// Prepare the ordinary regions stored directly on a slice.
#[must_use]
pub fn prepare_slice_regions(
    slice: &SliceIR,
    surface_classification: Option<&slicer_ir::SurfaceClassificationIR>,
) -> Vec<PreparedRegionData> {
    prepare_regions(
        &slice.regions,
        surface_classification,
        slice.global_layer_index,
    )
}

/// Reconstruct wall-owning regions and prepare their derived values.
#[must_use]
pub fn prepare_perimeter_source_regions(
    slice: &SliceIR,
    surface_classification: Option<&slicer_ir::SurfaceClassificationIR>,
) -> Vec<PreparedRegionData> {
    let regions = super::perimeter_source_regions(slice);
    prepare_regions(&regions, surface_classification, slice.global_layer_index)
}

type Bbox = (i64, i64, i64, i64);

fn expolygons_bbox(polygons: &[ExPolygon]) -> Option<Bbox> {
    let mut bounds: Option<Bbox> = None;
    for point in polygons.iter().flat_map(|polygon| {
        polygon
            .contour
            .points
            .iter()
            .chain(polygon.holes.iter().flat_map(|hole| hole.points.iter()))
    }) {
        bounds = Some(match bounds {
            None => (point.x, point.y, point.x, point.y),
            Some((min_x, min_y, max_x, max_y)) => (
                min_x.min(point.x),
                min_y.min(point.y),
                max_x.max(point.x),
                max_y.max(point.y),
            ),
        });
    }
    bounds
}

fn bbox_overlaps(region: Bbox, polygon: &ExPolygon) -> bool {
    let Some(candidate) = expolygons_bbox(std::slice::from_ref(polygon)) else {
        return false;
    };
    region.0 <= candidate.2
        && candidate.0 <= region.2
        && region.1 <= candidate.3
        && candidate.1 <= region.3
}
