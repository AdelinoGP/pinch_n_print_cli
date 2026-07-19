// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Fill/FillRectilinear.cpp + src/libslic3r/Fill/FillBase.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

use slicer_ir::{ExPolygon, Point2};
use slicer_sdk::host::{offset_polygons, OffsetJoinType};

#[derive(Debug, Clone, PartialEq)]
pub struct ExPolygonWithOffset {
    pub polygons_outer: Vec<ExPolygon>,
    pub polygons_inner: Vec<ExPolygon>,
    pub polygons_ccw: Vec<bool>,
}

impl ExPolygonWithOffset {
    pub fn new(source: &[ExPolygon], aoffset1_mm: f32, aoffset2_mm: f32) -> Self {
        let polygons_outer = offset_polygons(source, aoffset1_mm, OffsetJoinType::Miter);
        let inner_delta_mm = -(aoffset1_mm - aoffset2_mm);
        let polygons_inner = if inner_delta_mm == 0.0 {
            polygons_outer.clone()
        } else {
            offset_polygons(&polygons_outer, inner_delta_mm, OffsetJoinType::Miter)
        };

        Self {
            polygons_ccw: polygons_outer.iter().map(is_ccw).collect(),
            polygons_outer,
            polygons_inner,
        }
    }

    pub fn for_infill_overlap(source: &[ExPolygon], infill_overlap: f32, spacing_mm: f32) -> Self {
        let aoffset = (infill_overlap - 0.5) * spacing_mm;
        Self::new(source, aoffset, aoffset)
    }

    pub fn polygons_outer(&self) -> &[ExPolygon] {
        &self.polygons_outer
    }

    pub fn polygons_inner(&self) -> &[ExPolygon] {
        &self.polygons_inner
    }
}

pub fn clip_to_offset_boundary(
    polylines: &[Vec<Point2>],
    boundary: &[ExPolygon],
) -> Vec<Vec<Point2>> {
    slicer_core::polygon_ops::clip_polylines(polylines, boundary)
}

pub fn remove_short_polylines(polylines: &[Vec<Point2>], spacing_mm: f32) -> Vec<Vec<Point2>> {
    let threshold_units = slicer_ir::mm_to_units(0.8 * spacing_mm) as f64;
    polylines
        .iter()
        .filter(|polyline| polyline_length_units(polyline) >= threshold_units)
        .cloned()
        .collect()
}

fn is_ccw(expolygon: &ExPolygon) -> bool {
    expolygon
        .contour
        .points
        .iter()
        .zip(expolygon.contour.points.iter().cycle().skip(1))
        .take(expolygon.contour.points.len())
        .map(|(a, b)| (a.x as i128) * (b.y as i128) - (b.x as i128) * (a.y as i128))
        .sum::<i128>()
        > 0
}

fn polyline_length_units(polyline: &[Point2]) -> f64 {
    polyline
        .windows(2)
        .map(|segment| {
            let dx = (segment[1].x - segment[0].x) as f64;
            let dy = (segment[1].y - segment[0].y) as f64;
            dx.hypot(dy)
        })
        .sum()
}
