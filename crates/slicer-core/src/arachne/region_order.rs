// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, which is licensed under
// the GNU Affero General Public License, version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp,
// WallToolPaths::getRegionOrder.
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Spatially-derived emission constraints for Arachne extrusion lines.

use slicer_ir::ExtrusionLine;
use std::collections::BTreeSet;

use super::sparse_point_grid::{Point2, SparsePointGrid};

/// Returns `(before, after)` line-index constraints inferred from nearby
/// junctions, matching OrcaSlicer's `WallToolPaths::getRegionOrder`.
pub fn get_region_order(input: &[ExtrusionLine], outer_to_inner: bool) -> Vec<(usize, usize)> {
    let max_line_w = input
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|junction| junction.p.width)
        .fold(0.0_f32, f32::max);
    if max_line_w == 0.0 {
        return Vec::new();
    }

    let searching_radius = max_line_w * 1.9;
    let mut grid: SparsePointGrid<(usize, usize), _> = SparsePointGrid::new(
        searching_radius,
        |&(line_index, junction_index): &(usize, usize)| {
            let junction = &input[line_index].junctions[junction_index];
            Point2 {
                x: junction.p.x,
                y: junction.p.y,
            }
        },
    );
    let mut order_requirements = BTreeSet::new();

    for (line_index, line) in input.iter().enumerate() {
        for (junction_index, junction) in line.junctions.iter().enumerate() {
            grid.insert((line_index, junction_index));
            let nearby = grid.get_nearby(
                Point2 {
                    x: junction.p.x,
                    y: junction.p.y,
                },
                searching_radius,
            );

            for (nearby_line_index, nearby_junction_index) in nearby {
                if nearby_line_index == line_index && nearby_junction_index == junction_index {
                    continue;
                }

                let nearby_junction = &input[nearby_line_index].junctions[nearby_junction_index];
                let here = &input[line_index];
                let nearby_line = &input[nearby_line_index];
                if line_index == nearby_line_index
                    || here.inset_idx == nearby_line.inset_idx
                    || here.inset_idx.abs_diff(nearby_line.inset_idx) > 1
                {
                    continue;
                }

                let dx = junction.p.x - nearby_junction.p.x;
                let dy = junction.p.y - nearby_junction.p.y;
                let distance_squared = dx * dx + dy * dy;
                let width_limit = (junction.p.width + nearby_junction.p.width) / 2.0 * 1.9;
                if distance_squared > width_limit * width_limit {
                    continue;
                }

                if here.is_odd || nearby_line.is_odd {
                    if here.is_odd && !nearby_line.is_odd && nearby_line.inset_idx < here.inset_idx
                    {
                        order_requirements.insert((nearby_line_index, line_index));
                    }
                    if nearby_line.is_odd && !here.is_odd && here.inset_idx < nearby_line.inset_idx
                    {
                        order_requirements.insert((line_index, nearby_line_index));
                    }
                } else if (nearby_line.inset_idx < here.inset_idx) == outer_to_inner {
                    order_requirements.insert((nearby_line_index, line_index));
                } else {
                    order_requirements.insert((line_index, nearby_line_index));
                }
            }
        }
    }

    order_requirements.into_iter().collect()
}

/// Emits line indices in a nearest-first topological order.
pub fn topological_walk(lines: &[ExtrusionLine], constraints: &[(usize, usize)]) -> Vec<usize> {
    let mut blocked: Vec<usize> = vec![0; lines.len()];
    let mut blocking: Vec<Vec<usize>> = vec![Vec::new(); lines.len()];
    for &(before, after) in constraints {
        blocked[after] += 1;
        blocking[before].push(after);
    }

    let mut processed = vec![false; lines.len()];
    let mut result = Vec::with_capacity(lines.len());
    let seed_line =
        (constraints.is_empty() && lines.first().is_some_and(|line| !line.is_closed)).then_some(0);
    let mut current_position = if lines.is_empty() {
        Point2 { x: 0.0, y: 0.0 }
    } else if !lines[0].junctions.is_empty() {
        let junction = &lines[0].junctions[0];
        Point2 {
            x: junction.p.x,
            y: junction.p.y,
        }
    } else {
        Point2 { x: 0.0, y: 0.0 }
    };

    while result.len() < lines.len() {
        let mut available_candidates = (0..lines.len())
            .filter(|&index| {
                !processed[index]
                    && blocked[index] == 0
                    && !(result.is_empty() && seed_line == Some(index))
            })
            .collect::<Vec<_>>();
        available_candidates
            .sort_by_key(|&index| (lines[index].is_closed, lines[index].inset_idx, index));
        let best_candidate = if result.is_empty() && seed_line.is_some() {
            available_candidates.first().copied()
        } else {
            available_candidates.into_iter().min_by(|&a, &b| {
                let distance_a = squared_distance(&lines[a], current_position);
                let distance_b = squared_distance(&lines[b], current_position);
                distance_a
                    .partial_cmp(&distance_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.cmp(&b))
            })
        };
        let Some(best_candidate) = best_candidate else {
            break;
        };

        result.push(best_candidate);
        processed[best_candidate] = true;
        for &unlocked in &blocking[best_candidate] {
            blocked[unlocked] -= 1;
        }

        let best_path = &lines[best_candidate];
        if !best_path.junctions.is_empty() {
            let junction = if best_path.is_closed {
                &best_path.junctions[0]
            } else {
                best_path.junctions.last().unwrap()
            };
            current_position = Point2 {
                x: junction.p.x,
                y: junction.p.y,
            };
        }
    }

    result
}

/// Reorders lines in place according to their inferred region constraints.
pub fn reorder_by_region_order(lines: &mut Vec<ExtrusionLine>, outer_to_inner: bool) {
    let constraints = get_region_order(lines, outer_to_inner);
    let permutation = topological_walk(lines, &constraints);
    let reordered = permutation
        .iter()
        .map(|&index| lines[index].clone())
        .collect();
    *lines = reordered;
}

fn squared_distance(line: &ExtrusionLine, position: Point2) -> f64 {
    let Some(junction) = line.junctions.first() else {
        return f64::INFINITY;
    };
    let dx = f64::from(junction.p.x) - f64::from(position.x);
    let dy = f64::from(junction.p.y) - f64::from(position.y);
    dx * dx + dy * dy
}
