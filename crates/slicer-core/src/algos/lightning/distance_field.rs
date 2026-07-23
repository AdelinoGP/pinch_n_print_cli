// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.{hpp,cpp}
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

use slicer_ir::{slice_ir::BoundingBox2, Point2};

// Orca ref: DistanceField::DistanceField and radius_per_cell_size (DistanceField.cpp).
const RADIUS_PER_CELL_SIZE: u32 = 6;

struct UnsupportedCell {
    loc: Point2,
    dist_to_boundary: i64,
}

/// Discrete field of overhang locations that still need support.
pub struct DistanceField {
    cell_size: i64,
    supporting_radius: i64,
    supporting_radius2: i128,
    unsupported_points: Vec<UnsupportedCell>,
    unsupported_points_erased: Vec<bool>,
    unsupported_points_bbox: BoundingBox2,
}

impl DistanceField {
    /// Construct a field using the supplied supporting radius and polygons.
    pub fn new(
        supporting_radius: i64,
        outline: &[Point2],
        outline_bbox: BoundingBox2,
        overhang: &[Point2],
    ) -> Self {
        let cell_size = supporting_radius / i64::from(RADIUS_PER_CELL_SIZE);
        let mut unsupported_points = Vec::new();

        if cell_size > 0 && outline.len() >= 3 && overhang.len() >= 3 {
            if let Some((overhang_min, overhang_max)) = polygon_bounds(overhang) {
                let half_cell_size = cell_size / 2;
                let mut y = overhang_min.y.checked_add(half_cell_size);
                while let Some(sample_y) = y.filter(|sample_y| *sample_y < overhang_max.y) {
                    let mut x = overhang_min.x.checked_add(half_cell_size);
                    while let Some(sample_x) = x.filter(|sample_x| *sample_x < overhang_max.x) {
                        let sample = Point2 {
                            x: sample_x,
                            y: sample_y,
                        };
                        if outline_bbox.contains_point(sample) && point_in_polygon(sample, overhang)
                        {
                            unsupported_points.push(UnsupportedCell {
                                loc: sample,
                                dist_to_boundary: distance_to_polygon(sample, outline),
                            });
                        }
                        x = sample_x.checked_add(cell_size);
                    }
                    y = sample_y.checked_add(cell_size);
                }
            }
        }

        unsupported_points.sort_by(|first, second| {
            let distance_delta =
                (i128::from(second.dist_to_boundary) - i128::from(first.dist_to_boundary)).abs();
            if distance_delta > i128::from(supporting_radius) {
                first.dist_to_boundary.cmp(&second.dist_to_boundary)
            } else {
                point_hash(first.loc)
                    .wrapping_rem(191)
                    .cmp(&point_hash(second.loc).wrapping_rem(191))
            }
        });

        let unsupported_points_erased = vec![false; unsupported_points.len()];
        Self {
            cell_size,
            supporting_radius,
            supporting_radius2: i128::from(supporting_radius).pow(2),
            unsupported_points,
            unsupported_points_erased,
            unsupported_points_bbox: outline_bbox,
        }
    }

    /// Return the first unsupported location in deterministic insertion order.
    pub fn try_get_next_point(&self) -> Option<Point2> {
        self.unsupported_points
            .iter()
            .zip(self.unsupported_points_erased.iter())
            .find_map(|(cell, erased)| (!erased).then_some(cell.loc))
    }

    /// Return the number of cells that have not been erased.
    pub fn unsupported_count(&self) -> usize {
        self.unsupported_points_erased
            .iter()
            .filter(|erased| !**erased)
            .count()
    }

    /// Mark cells supported by a newly added branch.
    pub fn update(&mut self, to_node: Point2, added_leaf: Point2) {
        if self.cell_size <= 0 || self.supporting_radius <= 0 || self.unsupported_points.is_empty()
        {
            return;
        }

        let grid = self.branch_search_bbox(to_node, added_leaf);
        let segment_x = added_leaf.x as f64 - to_node.x as f64;
        let segment_y = added_leaf.y as f64 - to_node.y as f64;
        let segment_length2 = segment_x * segment_x + segment_y * segment_y;
        let radius = self.supporting_radius as f64;
        for (point, erased) in self
            .unsupported_points
            .iter()
            .zip(self.unsupported_points_erased.iter_mut())
        {
            if *erased || !grid.contains_point(point.loc) {
                continue;
            }

            let in_supporting_circle =
                squared_distance(point.loc, added_leaf) <= self.supporting_radius2;
            let in_supporting_rectangle = if segment_length2 > 0.0 {
                let point_x = point.loc.x as f64 - to_node.x as f64;
                let point_y = point.loc.y as f64 - to_node.y as f64;
                let projection = point_x * segment_x + point_y * segment_y;
                if (0.0..=segment_length2).contains(&projection) {
                    let cross = point_x * segment_y - point_y * segment_x;
                    cross * cross <= radius * radius * segment_length2
                } else {
                    false
                }
            } else {
                false
            };

            if in_supporting_circle || in_supporting_rectangle {
                *erased = true;
            }
        }
    }

    fn branch_search_bbox(&self, to_node: Point2, added_leaf: Point2) -> BoundingBox2 {
        let radius = self.supporting_radius;
        let mut min = Point2 {
            x: added_leaf.x.saturating_sub(radius),
            y: added_leaf.y.saturating_sub(radius),
        };
        let mut max = Point2 {
            x: added_leaf.x.saturating_add(radius),
            y: added_leaf.y.saturating_add(radius),
        };

        let dx = added_leaf.x as f64 - to_node.x as f64;
        let dy = added_leaf.y as f64 - to_node.y as f64;
        let length = dx.hypot(dy);
        if length > 0.0 {
            let extent = Point2 {
                x: (-dy * radius as f64 / length).trunc() as i64,
                y: (dx * radius as f64 / length).trunc() as i64,
            };
            for point in [
                Point2 {
                    x: to_node.x.saturating_sub(extent.x),
                    y: to_node.y.saturating_sub(extent.y),
                },
                Point2 {
                    x: to_node.x.saturating_add(extent.x),
                    y: to_node.y.saturating_add(extent.y),
                },
                Point2 {
                    x: added_leaf.x.saturating_sub(extent.x),
                    y: added_leaf.y.saturating_sub(extent.y),
                },
                Point2 {
                    x: added_leaf.x.saturating_add(extent.x),
                    y: added_leaf.y.saturating_add(extent.y),
                },
            ] {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
            }
        }

        BoundingBox2 {
            min: Point2 {
                x: min.x.max(self.unsupported_points_bbox.min.x),
                y: min.y.max(self.unsupported_points_bbox.min.y),
            },
            max: Point2 {
                x: max.x.min(self.unsupported_points_bbox.max.x),
                y: max.y.min(self.unsupported_points_bbox.max.y),
            },
        }
    }
}

fn polygon_bounds(polygon: &[Point2]) -> Option<(Point2, Point2)> {
    let first = *polygon.first()?;
    let mut min = first;
    let mut max = first;
    for point in polygon.iter().skip(1) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    Some((min, max))
}

fn distance_to_polygon(point: Point2, polygon: &[Point2]) -> i64 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(start, end)| distance_to_segment(point, *start, *end))
        .fold(f64::INFINITY, f64::min) as i64
}

fn distance_to_segment(point: Point2, start: Point2, end: Point2) -> f64 {
    let start_x = start.x as f64;
    let start_y = start.y as f64;
    let segment_x = end.x as f64 - start.x as f64;
    let segment_y = end.y as f64 - start.y as f64;
    let segment_length2 = segment_x * segment_x + segment_y * segment_y;
    let projection = if segment_length2 == 0.0 {
        0.0
    } else {
        (((point.x as f64 - start_x) * segment_x + (point.y as f64 - start_y) * segment_y)
            / segment_length2)
            .clamp(0.0, 1.0)
    };
    let nearest_x = start_x + projection * segment_x;
    let nearest_y = start_y + projection * segment_y;
    (point.x as f64 - nearest_x).hypot(point.y as f64 - nearest_y)
}

fn point_in_polygon(point: Point2, polygon: &[Point2]) -> bool {
    let mut inside = false;
    for (start, end) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if point_on_segment(point, *start, *end) {
            return true;
        }

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

fn point_on_segment(point: Point2, start: Point2, end: Point2) -> bool {
    let cross = (i128::from(point.x) - i128::from(start.x))
        * (i128::from(end.y) - i128::from(start.y))
        - (i128::from(point.y) - i128::from(start.y)) * (i128::from(end.x) - i128::from(start.x));
    if cross != 0 {
        return false;
    }

    point.x >= start.x.min(end.x)
        && point.x <= start.x.max(end.x)
        && point.y >= start.y.min(end.y)
        && point.y <= start.y.max(end.y)
}

fn squared_distance(first: Point2, second: Point2) -> i128 {
    let dx = i128::from(first.x) - i128::from(second.x);
    let dy = i128::from(first.y) - i128::from(second.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn point_hash(point: Point2) -> u64 {
    (point.x as u64)
        .wrapping_mul(31)
        .wrapping_add(point.y as u64)
}
