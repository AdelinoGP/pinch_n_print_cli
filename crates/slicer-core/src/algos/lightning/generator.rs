// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

use std::rc::Rc;

use slicer_ir::{units_to_mm, ExPolygon, Point2, Polygon};

use crate::{difference, offset, OffsetJoinType};

use super::layer::Layer;

// [FWD] `lightning_overhang_angle` and `layer_height` feed the supporting radius; `sparse_infill_line_width` feeds infill resolution.

fn expolygon(outline: &[Point2]) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: outline.to_vec(),
        },
        holes: Vec::new(),
    }
}

fn append_ring(points: &mut Vec<Point2>, ring: &[Point2]) {
    if ring.is_empty() {
        return;
    }
    points.extend_from_slice(ring);
    if ring.last() != ring.first() {
        points.push(ring[0]);
    }
}

fn flatten_expolygons(polygons: &[ExPolygon]) -> Vec<Point2> {
    let mut points = Vec::new();
    for polygon in polygons {
        append_ring(&mut points, &polygon.contour.points);
        for hole in &polygon.holes {
            append_ring(&mut points, &hole.points);
        }
    }
    points
}

/// Cross-layer Lightning tree generator for one object's ordered layer outlines.
pub struct Generator {
    /// Sparse-infill outline for each layer, from bottom to top.
    pub per_layer_outlines: Vec<Vec<Point2>>,
    /// Radius used to resolve unsupported cells into branches.
    pub supporting_radius: i64,
    /// Radius used to classify an outline as supported by the previous layer.
    pub wall_supporting_radius: i64,
    /// Maximum branch length removed while propagating to the next layer.
    pub prune_length: i64,
    /// Maximum node movement used while straightening propagated trees.
    pub straightening_max_distance: i64,
    /// Overlap removed at tree junctions during line conversion.
    pub line_overlap: i64,
    /// Layer thickness in 100 nm coordinate units.
    pub layer_thickness: i64,
    /// Lightning overhang angle in radians.
    pub lightning_overhang_angle: f64,
    m_overhang_per_layer: Vec<Vec<Point2>>,
    m_layers: Vec<Layer>,
    committed_segments: Vec<[Point2; 2]>,
    committed_segments_by_layer: Vec<Vec<[Point2; 2]>>,
}

impl Generator {
    /// Construct a generator and compute the per-layer internal overhangs.
    pub fn new(
        per_layer_outlines: Vec<Vec<Point2>>,
        density: f64,
        extrusion_width: i64,
        n_multiline: i64,
        layer_thickness: i64,
        lightning_overhang_angle: f64,
        lightning_prune_angle: f64,
        lightning_straightening_angle: f64,
    ) -> Self {
        let lightning_overhang_angle = lightning_overhang_angle.to_radians();
        let supporting_radius = if density > 0.0 {
            ((extrusion_width as f64 * n_multiline as f64) / density).round() as i64
        } else {
            0
        };
        let wall_supporting_radius = angle_distance(layer_thickness, lightning_overhang_angle);
        let prune_length = angle_distance(layer_thickness, lightning_prune_angle.to_radians());
        let straightening_max_distance =
            angle_distance(layer_thickness, lightning_straightening_angle.to_radians());
        let m_overhang_per_layer =
            generate_initial_internal_overhangs(&per_layer_outlines, wall_supporting_radius);
        let m_layers = per_layer_outlines
            .iter()
            .map(|_| Layer::default())
            .collect();

        Self {
            per_layer_outlines,
            supporting_radius,
            wall_supporting_radius,
            prune_length,
            straightening_max_distance,
            line_overlap: 200,
            layer_thickness,
            lightning_overhang_angle,
            m_overhang_per_layer,
            m_layers,
            committed_segments: Vec::new(),
            committed_segments_by_layer: Vec::new(),
        }
    }

    /// Port of `Generator::generateTrees` in `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp`.
    pub fn generate_trees(&mut self, cancel: &dyn Fn()) {
        self.m_layers = self
            .per_layer_outlines
            .iter()
            .map(|_| Layer::default())
            .collect();
        self.committed_segments.clear();
        self.committed_segments_by_layer =
            self.per_layer_outlines.iter().map(|_| Vec::new()).collect();

        for layer_id in (0..self.per_layer_outlines.len()).rev() {
            cancel();
            self.m_layers[layer_id].generate_new_trees(
                &self.m_overhang_per_layer[layer_id],
                &self.per_layer_outlines[layer_id],
                self.supporting_radius,
                self.wall_supporting_radius,
                cancel,
            );
        }

        for layer_id in (1..self.per_layer_outlines.len()).rev() {
            cancel();
            let mut propagated_roots = Vec::new();
            let upper_roots = self.m_layers[layer_id].tree_roots.clone();
            for root in upper_roots {
                cancel();
                let Some(propagated_root) = root.borrow().propagate_to_next_layer(
                    &self.per_layer_outlines[layer_id - 1],
                    self.supporting_radius,
                    self.prune_length,
                    self.straightening_max_distance,
                    self.straightening_max_distance,
                ) else {
                    continue;
                };
                self.m_layers[layer_id - 1]
                    .tree_roots
                    .push(Rc::clone(&propagated_root));
                propagated_roots.push(propagated_root);
            }
            self.m_layers[layer_id - 1].reconnect_roots(
                propagated_roots,
                &self.per_layer_outlines[layer_id - 1],
                self.supporting_radius,
                self.wall_supporting_radius,
            );
        }

        for (layer_id, layer) in self.m_layers.iter().enumerate() {
            cancel();
            for polyline in
                layer.convert_to_lines(&self.per_layer_outlines[layer_id], self.line_overlap)
            {
                for segment in polyline.windows(2) {
                    let segment = [segment[0], segment[1]];
                    self.committed_segments.push(segment);
                    self.committed_segments_by_layer[layer_id].push(segment);
                }
            }
        }
    }

    /// Return the generated trees for one layer.
    pub fn get_trees_for_layer(&self, layer_id: usize) -> &Layer {
        &self.m_layers[layer_id]
    }

    /// Return committed segments for one layer without changing ownership.
    pub fn committed_segments_for_layer(&self, layer_id: usize) -> &[[Point2; 2]] {
        &self.committed_segments_by_layer[layer_id]
    }

    /// Remove the committed segments in deterministic layer and creation order.
    pub fn take_committed_segments(&mut self) -> Vec<[Point2; 2]> {
        std::mem::take(&mut self.committed_segments)
    }
}

fn angle_distance(layer_thickness: i64, angle: f64) -> i64 {
    (layer_thickness as f64 * angle.tan()).round() as i64
}

/// Port of `Generator::generateInitialInternalOverhangs` in `Generator.cpp`.
///
/// Each output entry corresponds to one input layer. Its points contain the
/// Clipper result's closed contour and hole rings in output order.
pub fn generate_initial_internal_overhangs(
    per_layer_outlines: &[Vec<Point2>],
    wall_supporting_radius: i64,
) -> Vec<Vec<Point2>> {
    let mut internal_overhangs = Vec::with_capacity(per_layer_outlines.len());

    for (layer_index, current_outline) in per_layer_outlines.iter().enumerate() {
        if layer_index == 0 || current_outline.len() < 3 {
            internal_overhangs.push(Vec::new());
            continue;
        }

        let current = expolygon(current_outline);
        let previous_outline = &per_layer_outlines[layer_index - 1];
        let supporting = if previous_outline.len() < 3 {
            Vec::new()
        } else {
            offset(
                &[expolygon(previous_outline)],
                units_to_mm(wall_supporting_radius),
                OffsetJoinType::Miter,
                0.0,
            )
        };
        let unsupported = difference(&[current], &supporting);
        internal_overhangs.push(flatten_expolygons(&unsupported));
    }

    internal_overhangs
}
