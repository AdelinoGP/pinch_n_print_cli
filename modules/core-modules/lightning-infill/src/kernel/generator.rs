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

use slicer_ir::{units_to_mm, ExPolygon, Point2, Polygon};

use slicer_core::{difference, offset, OffsetJoinType};

use super::layer::Layer;
use super::Polygons;

/// Cell size of the outline locator used while propagating trees downward.
///
/// Canonical `Generator::generateTrees` builds its `EdgeGrid::Grid` with
/// `_locator_cell_size = scaled<coord_t>(4.)`, i.e. 4 mm. Pinch 'n Print units
/// are 100 nm, so 4 mm is 40 000 units. Canonical derives two further lengths
/// from it: the boundary-crossing tolerance in `Node::realign` is
/// `resolution() * 2` (8 mm), and `propagateToNextLayer` receives
/// `_locator_cell_size / 2` (2 mm) as `max_remove_colinear_dist`.
const OUTLINE_LOCATOR_CELL_SIZE: i64 = 40_000;

// [FWD] `lightning_overhang_angle` and `layer_height` feed the supporting radius; `sparse_infill_line_width` feeds infill resolution.

fn append_ring(rings: &mut Polygons, ring: &[Point2]) {
    if ring.len() < 3 {
        return;
    }
    let mut closed = ring.to_vec();
    if closed.last() != closed.first() {
        closed.push(closed[0]);
    }
    rings.push(closed);
}

pub(crate) fn flatten_expolygons(polygons: &[ExPolygon]) -> Polygons {
    let mut rings = Vec::new();
    for polygon in polygons {
        append_ring(&mut rings, &polygon.contour.points);
        for hole in &polygon.holes {
            append_ring(&mut rings, &hole.points);
        }
    }
    rings
}

/// Group flat rings back into expolygons for a Clipper boolean operation.
///
/// Canonical passes flat `Polygons` to Clipper with its non-zero fill rule:
/// outer rings are CCW and holes are CW, and an island nested inside a hole is
/// CCW again. `slicer_core`'s boolean entry points take `ExPolygon`s, so this
/// reconstructs the same filled region from ring orientation and containment.
/// Rings are closed first so a caller's open ring is not treated as missing its
/// final edge.
fn expolygons_from_rings(rings: &Polygons) -> Vec<ExPolygon> {
    let mut closed: Vec<(usize, i128)> = rings
        .iter()
        .filter(|ring| ring.len() >= 3)
        .enumerate()
        .map(|(index, ring)| {
            let area2: i128 = ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
                .map(|(a, b)| i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y))
                .sum();
            (index, area2)
        })
        .collect();
    // Canonical's flat `Polygons` order is contour-then-hole per expolygon. Sort
    // outers before holes so the smallest-containing-outer search sees them.
    closed.sort_by_key(|(_, area2)| -*area2);

    let mut out: Vec<ExPolygon> = Vec::new();
    let mut outer_index_for: Vec<Option<usize>> = vec![None; rings.len()];
    for &(index, area2) in &closed {
        if area2 > 0 {
            outer_index_for[index] = Some(out.len());
            out.push(ExPolygon {
                contour: Polygon {
                    points: rings[index].clone(),
                },
                holes: Vec::new(),
            });
        } else if area2 < 0 {
            let hole = &rings[index];
            let probe = hole[0];
            let mut best: Option<(i128, usize)> = None;
            for &(outer_index, outer_area2) in &closed {
                if outer_area2 <= 0 || outer_index == index {
                    continue;
                }
                if point_in_ring(probe, &rings[outer_index]) {
                    let area = outer_area2.abs();
                    if best.is_none_or(|(best_area, _)| area < best_area) {
                        best = Some((area, outer_index_for[outer_index].unwrap()));
                    }
                }
            }
            if let Some((_, outer_index)) = best {
                out[outer_index].holes.push(Polygon {
                    points: hole.clone(),
                });
            } else {
                // A non-zero fill rule fills a lone clockwise ring too. Reversing
                // it to the outer orientation preserves that filled area instead
                // of silently dropping the ring.
                let mut contour = hole.clone();
                contour.reverse();
                out.push(ExPolygon {
                    contour: Polygon { points: contour },
                    holes: Vec::new(),
                });
            }
        }
    }
    out
}

fn point_in_ring(point: Point2, ring: &[Point2]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    for (start, end) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if (start.y > point.y) != (end.y > point.y) {
            let intersection_x = start.x as f64
                + (point.y - start.y) as f64 * (end.x - start.x) as f64 / (end.y - start.y) as f64;
            if (point.x as f64) < intersection_x {
                inside = !inside;
            }
        }
    }
    inside
}

/// Cross-layer Lightning tree generator for one object's ordered layer outlines.
pub struct Generator {
    /// Sparse-infill outline for each layer, from bottom to top. Each layer is a
    /// flat ring set: contour rings and reversed hole rings are siblings, exactly
    /// as canonical `to_polygons(surface.expolygon)` produces them.
    pub per_layer_outlines: Vec<Polygons>,
    /// Radius used to resolve unsupported cells into branches.
    pub supporting_radius: i64,
    /// Inward wall-support erosion applied before subtracting the layer above.
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
    m_overhang_per_layer: Vec<Polygons>,
    m_layers: Vec<Layer>,
    committed_segments: Vec<[Point2; 2]>,
    committed_segments_by_layer: Vec<Vec<[Point2; 2]>>,
}

impl Generator {
    /// Construct a generator and compute the per-layer internal overhangs.
    ///
    /// `line_overlap` is canonical's `0.5 * spacing - overlap`, which the fill
    /// driver passes to `Layer::convertToLines`; it is not derived from the
    /// supporting radius.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        per_layer_outlines: Vec<Polygons>,
        density: f64,
        extrusion_width: i64,
        n_multiline: i64,
        layer_thickness: i64,
        lightning_overhang_angle: f64,
        lightning_prune_angle: f64,
        lightning_straightening_angle: f64,
        line_overlap: i64,
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
            line_overlap,
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

        // Canonical `Generator::generateTrees` interleaves seeding, reconnection
        // and downward propagation inside ONE top-down loop, so the trees that
        // layer N pushes down are already present in layer N-1 when layer N-1
        // seeds its own trees and can therefore be grounded onto.
        for layer_id in (0..self.per_layer_outlines.len()).rev() {
            cancel();
            // Snapshot BEFORE `generate_new_trees`, exactly as canonical does:
            // the roots to reconnect are the ones propagated down from above,
            // never the ones this layer is about to seed.
            let to_be_reconnected = self.m_layers[layer_id].tree_roots.clone();
            self.m_layers[layer_id].generate_new_trees(
                &self.m_overhang_per_layer[layer_id],
                &self.per_layer_outlines[layer_id],
                self.supporting_radius,
                self.wall_supporting_radius,
                cancel,
            );
            self.m_layers[layer_id].reconnect_roots(
                to_be_reconnected,
                &self.per_layer_outlines[layer_id],
                self.supporting_radius,
                self.wall_supporting_radius,
            );

            if layer_id == 0 {
                continue;
            }

            let mut lower_trees = Vec::new();
            for root in self.m_layers[layer_id].tree_roots.clone() {
                cancel();
                lower_trees.extend(root.borrow().propagate_to_next_layer(
                    &self.per_layer_outlines[layer_id - 1],
                    OUTLINE_LOCATOR_CELL_SIZE,
                    self.prune_length,
                    self.straightening_max_distance,
                    OUTLINE_LOCATOR_CELL_SIZE / 2,
                ));
            }
            self.m_layers[layer_id - 1].tree_roots.extend(lower_trees);
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
/// Canonical walks top-down. For each layer it erodes that layer's raw fill
/// area by the wall-supporting radius and subtracts the **un-eroded** area of
/// the layer above, then retains that raw area as the next `above`. The result
/// is a flat ring set: the Clipper result's contours and reversed hole rings,
/// in output order.
pub fn generate_initial_internal_overhangs(
    per_layer_outlines: &[Polygons],
    wall_supporting_radius: i64,
) -> Vec<Polygons> {
    let mut internal_overhangs = vec![Vec::new(); per_layer_outlines.len()];
    let mut above = Vec::new();
    for layer_index in (0..per_layer_outlines.len()).rev() {
        let current = expolygons_from_rings(&per_layer_outlines[layer_index]);
        let eroded = offset(
            &current,
            -units_to_mm(wall_supporting_radius),
            OffsetJoinType::Miter,
            0.0,
        );
        let unsupported = difference(&eroded, &above);
        internal_overhangs[layer_index] = flatten_expolygons(&unsupported);
        above = current;
    }

    internal_overhangs
}
