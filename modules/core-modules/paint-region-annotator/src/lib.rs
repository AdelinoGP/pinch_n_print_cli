//! Paint region annotator module for ModularSlicer.
//!
//! Implements `LayerModule::run_slice_postprocess` to write contour-parallel
//! `boundary_paint` annotations from `PaintRegionIR` onto `SlicedRegion`
//! contour points. For each region and each semantic with paint data on the
//! current layer, queries point-in-polygon containment for every contour point
//! and writes the result into the `SlicePostprocessBuilder`.
//!
//! Per OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp
//! (paint region annotation concepts) and
//! OrcaSlicerDocumented/src/libslic3r/ExPolygon.cpp lines 182-205
//! (contour-plus-hole containment).

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_core::paint_region::{point_in_paint_region, BoundaryInclusion, PaintRegionQueryError};
use slicer_ir::{ConfigView, PaintSemantic, PaintValue, RegionKey};
use slicer_sdk::builders::SlicePostprocessBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Paint region annotator module.
///
/// Annotates `SlicedRegion` contour points with `boundary_paint` values
/// by querying paint region containment for each point on each semantic.
/// Runs last in the `Layer::SlicePostProcess` stage.
pub struct PaintRegionAnnotator;

impl LayerModule for PaintRegionAnnotator {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(PaintRegionAnnotator)
    }

    fn run_slice_postprocess(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut SlicePostprocessBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let semantics = paint.semantics_on_layer();

        // If no paint data on this layer, skip all regions
        if semantics.is_empty() {
            return Ok(());
        }

        let paint_regions = paint.paint_regions();

        for region in regions {
            let mut boundary_paint: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
                HashMap::new();

            for semantic in &semantics {
                let mut per_polygon = Vec::with_capacity(region.polygons().len());

                for polygon in region.polygons() {
                    let mut per_point = Vec::with_capacity(polygon.contour.points.len());

                    for &point in &polygon.contour.points {
                        match point_in_paint_region(
                            paint_regions,
                            layer_index,
                            semantic,
                            point,
                            BoundaryInclusion::Include,
                        ) {
                            Ok(value) => per_point.push(value),
                            Err(PaintRegionQueryError::DeterministicConflict) => {
                                return Err(ModuleError::fatal(
                                    503,
                                    format!(
                                        "deterministic conflict for semantic {:?} at point ({}, {}) on layer {}",
                                        semantic, point.x, point.y, layer_index
                                    ),
                                ));
                            }
                        }
                    }

                    per_polygon.push(per_point);
                }

                boundary_paint.insert(semantic.clone(), per_polygon);
            }

            let key = RegionKey {
                global_layer_index: layer_index,
                object_id: region.object_id().clone(),
                region_id: *region.region_id(),
            };

            let _ = output.set_boundary_paint(key, boundary_paint);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_creates_successfully() {
        let config = ConfigView {
            fields: HashMap::new(),
        };
        let annotator = PaintRegionAnnotator::on_print_start(&config);
        assert!(annotator.is_ok());
    }
}
