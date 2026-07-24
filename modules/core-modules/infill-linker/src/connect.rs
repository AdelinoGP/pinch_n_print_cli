// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: FillBase.cpp::connect_infill
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

use std::cmp::Ordering;

use crate::graph::{contour_connector, BoundaryInfillGraph, BoundaryRing};
use slicer_ir::{mm_to_units, ExtrusionPath3D, Point2, Point3WithWidth};

// OrcaSlicer `connect_infill`'s disabled candidate threshold is `1000. / density * spacing`; 1000 nm is 10 units here.
const LINK_THRESHOLD_SPACINGS: f64 = 10.0;
const ENDPOINT_WIDTH_EPSILON: f32 = 0.000001;

#[derive(Debug, Clone, Copy)]
struct BoundaryPosition {
    ring_index: usize,
    arc_position: f64,
}

#[derive(Debug, Clone, Copy)]
struct Endpoint {
    path_index: usize,
    at_start: bool,
    position: BoundaryPosition,
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    first: Endpoint,
    second: Endpoint,
    distance: f64,
}

/// Greedily joins compatible extrusion paths across short boundary walks.
pub fn connect_infill(
    paths: Vec<ExtrusionPath3D>,
    graph: &BoundaryInfillGraph,
    spacing_mm: f32,
) -> Vec<ExtrusionPath3D> {
    let mut active = paths
        .into_iter()
        .filter(|path| path.points.len() >= 2)
        .map(Some)
        .collect::<Vec<_>>();
    let threshold = if spacing_mm.is_finite() && spacing_mm > 0.0 {
        mm_to_units((LINK_THRESHOLD_SPACINGS * spacing_mm as f64) as f32) as f64
    } else {
        0.0
    };

    while let Some(candidate) = nearest_pair_candidate(&active, graph, threshold) {
        let first_index = candidate.first.path_index;
        let second_index = candidate.second.path_index;
        if first_index == second_index {
            break;
        }

        // Canonical `connect_infill` wires `prev_on_contour` / `next_on_contour`
        // within a single ring, and `take` / `take_limited` always receive one
        // ring's point array — there is no outer-contour-to-hole bridging
        // connector. Endpoints that do not share a ring are therefore left
        // unconnected rather than joined by a chord across the interior. The
        // candidate filter already enforces this; the guard makes the invariant
        // local to the splice that depends on it.
        let Some(ring) = graph
            .rings()
            .get(candidate.first.position.ring_index)
            .filter(|_| {
                candidate.first.position.ring_index == candidate.second.position.ring_index
            })
        else {
            break;
        };

        let (first, second) = if first_index < second_index {
            let (left, right) = active.split_at_mut(second_index);
            (left[first_index].take(), right[0].take())
        } else {
            let (left, right) = active.split_at_mut(first_index);
            (right[0].take(), left[second_index].take())
        };
        let (Some(mut first), Some(mut second)) = (first, second) else {
            break;
        };

        orient_for_join(&mut first, candidate.first.at_start, true);
        orient_for_join(&mut second, candidate.second.at_start, false);
        // After orientation the join runs from `first`'s last point to
        // `second`'s first point. Route it along the contour instead of
        // extruding a bare chord between them.
        if let (Some(start), Some(end)) =
            (first.points.last().copied(), second.points.first().copied())
        {
            first.points.extend(contour_connector(
                ring,
                candidate.first.position.arc_position,
                candidate.second.position.arc_position,
                &start,
                &end,
            ));
        }
        first.points.extend(second.points);
        active[first_index.min(second_index)] = Some(first);
    }

    active.into_iter().flatten().collect()
}

/// Orders and connects extrusion paths by their nearest boundary endpoints.
pub fn chain_or_connect_infill(
    paths: Vec<ExtrusionPath3D>,
    graph: &BoundaryInfillGraph,
    spacing_mm: f32,
) -> Vec<ExtrusionPath3D> {
    let mut paths = connect_infill(paths, graph, spacing_mm);
    if paths.len() < 2 {
        return paths;
    }

    let first_index = (0..paths.len())
        .min_by(|left, right| {
            endpoint_order_cmp(
                &path_start_order(&paths[*left]),
                &path_start_order(&paths[*right]),
            )
        })
        .expect("non-empty paths");
    paths.swap(0, first_index);
    if endpoint_order_cmp(&path_end_order(&paths[0]), &path_start_order(&paths[0]))
        == Ordering::Less
    {
        paths[0].points.reverse();
    }

    let mut ordered = vec![paths.remove(0)];
    while !paths.is_empty() {
        let current = ordered
            .last()
            .and_then(|path| path.points.last())
            .expect("ordered path has points");
        let (next_index, reverse) = (0..paths.len())
            .map(|index| {
                let start_distance = point_distance_squared(current, paths[index].points.first());
                let end_distance = point_distance_squared(current, paths[index].points.last());
                if end_distance < start_distance {
                    (index, true, end_distance)
                } else {
                    (index, false, start_distance)
                }
            })
            .min_by(|left, right| {
                left.2
                    .total_cmp(&right.2)
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|(index, reverse, _)| (index, reverse))
            .expect("remaining paths are non-empty");
        let mut next = paths.swap_remove(next_index);
        if reverse {
            next.points.reverse();
        }
        ordered.push(next);
    }
    ordered
}

fn path_start_order(path: &ExtrusionPath3D) -> (f32, f32, f32) {
    path.points
        .first()
        .map_or((f32::INFINITY, f32::INFINITY, f32::INFINITY), |point| {
            (point.x, point.y, point.z)
        })
}

fn path_end_order(path: &ExtrusionPath3D) -> (f32, f32, f32) {
    path.points
        .last()
        .map_or((f32::INFINITY, f32::INFINITY, f32::INFINITY), |point| {
            (point.x, point.y, point.z)
        })
}

fn endpoint_order_cmp(left: &(f32, f32, f32), right: &(f32, f32, f32)) -> Ordering {
    left.0
        .total_cmp(&right.0)
        .then_with(|| left.1.total_cmp(&right.1))
        .then_with(|| left.2.total_cmp(&right.2))
}

fn point_distance_squared(first: &Point3WithWidth, second: Option<&Point3WithWidth>) -> f64 {
    let Some(second) = second else {
        return f64::INFINITY;
    };
    let dx = f64::from(first.x) - f64::from(second.x);
    let dy = f64::from(first.y) - f64::from(second.y);
    let dz = f64::from(first.z) - f64::from(second.z);
    dx.mul_add(dx, dy.mul_add(dy, dz * dz))
}

fn nearest_pair_candidate(
    active: &[Option<ExtrusionPath3D>],
    graph: &BoundaryInfillGraph,
    threshold: f64,
) -> Option<Candidate> {
    let mut endpoints = active
        .iter()
        .enumerate()
        .flat_map(|(path_index, path)| {
            let path = path.as_ref()?;
            let first = boundary_position(graph, path.points.first()?)?;
            let last = boundary_position(graph, path.points.last()?)?;
            Some([
                Endpoint {
                    path_index,
                    at_start: true,
                    position: first,
                },
                Endpoint {
                    path_index,
                    at_start: false,
                    position: last,
                },
            ])
        })
        .flatten()
        .collect::<Vec<_>>();
    endpoints.sort_by(endpoint_order);

    let mut candidates = Vec::new();
    for endpoint in &endpoints {
        let Some(best) = endpoints
            .iter()
            .filter(|other| {
                other.path_index != endpoint.path_index
                    && compatible_paths(active, endpoint.path_index, other.path_index)
                    && other.position.ring_index == endpoint.position.ring_index
            })
            .filter_map(|other| {
                let (distance, _) = graph
                    .rings()
                    .get(endpoint.position.ring_index)?
                    .directed_distance(endpoint.position.arc_position, other.position.arc_position);
                (distance <= threshold).then_some((*other, distance))
            })
            .min_by(|(left, left_distance), (right, right_distance)| {
                left_distance
                    .total_cmp(right_distance)
                    .then_with(|| endpoint_order(left, right))
            })
        else {
            continue;
        };
        candidates.push(Candidate {
            first: *endpoint,
            second: best.0,
            distance: best.1,
        });
    }

    candidates.sort_by(|left, right| {
        endpoint_order(&left.first, &right.first)
            .then_with(|| left.distance.total_cmp(&right.distance))
            .then_with(|| endpoint_order(&left.second, &right.second))
    });
    candidates.into_iter().find(|candidate| {
        candidate.first.path_index != candidate.second.path_index
            && active[candidate.first.path_index].is_some()
            && active[candidate.second.path_index].is_some()
    })
}

fn compatible_paths(
    active: &[Option<ExtrusionPath3D>],
    first_index: usize,
    second_index: usize,
) -> bool {
    let (Some(first), Some(second)) = (&active[first_index], &active[second_index]) else {
        return false;
    };
    first.role == second.role
        && first.speed_factor.to_bits() == second.speed_factor.to_bits()
        && endpoint_widths_compatible(first, second)
}

fn endpoint_widths_compatible(first: &ExtrusionPath3D, second: &ExtrusionPath3D) -> bool {
    let (Some(first_start), Some(first_end), Some(second_start), Some(second_end)) = (
        first.points.first(),
        first.points.last(),
        second.points.first(),
        second.points.last(),
    ) else {
        return false;
    };
    (first_start.width - second_start.width).abs() <= ENDPOINT_WIDTH_EPSILON
        && (first_end.width - second_end.width).abs() <= ENDPOINT_WIDTH_EPSILON
}

fn endpoint_order(left: &Endpoint, right: &Endpoint) -> std::cmp::Ordering {
    left.position
        .arc_position
        .total_cmp(&right.position.arc_position)
        .then_with(|| left.path_index.cmp(&right.path_index))
        .then_with(|| left.at_start.cmp(&right.at_start))
}

fn boundary_position(
    graph: &BoundaryInfillGraph,
    point: &slicer_ir::Point3WithWidth,
) -> Option<BoundaryPosition> {
    let point = Point2::from_mm(point.x, point.y);
    graph
        .rings()
        .iter()
        .enumerate()
        .filter_map(|(ring_index, ring)| {
            project_on_ring(ring, point).map(|(distance_squared, local_arc)| {
                (
                    distance_squared,
                    BoundaryPosition {
                        ring_index,
                        arc_position: ring.pos_of_first_point + local_arc,
                    },
                )
            })
        })
        .min_by(
            |(left_distance, left_position), (right_distance, right_position)| {
                left_distance
                    .total_cmp(right_distance)
                    .then_with(|| left_position.ring_index.cmp(&right_position.ring_index))
                    .then_with(|| {
                        left_position
                            .arc_position
                            .total_cmp(&right_position.arc_position)
                    })
            },
        )
        .map(|(_, position)| position)
}

fn project_on_ring(ring: &BoundaryRing, point: Point2) -> Option<(f64, f64)> {
    let points = &ring.polygon.points;
    if points.is_empty() {
        return None;
    }

    let mut best = None;
    let mut segment_start = 0.0;
    for index in 0..points.len() {
        let start = points[index];
        let end = points[(index + 1) % points.len()];
        let dx = end.x as f64 - start.x as f64;
        let dy = end.y as f64 - start.y as f64;
        let segment_len = dx.hypot(dy);
        let parameter = if segment_len == 0.0 {
            0.0
        } else {
            let point_dx = point.x as f64 - start.x as f64;
            let point_dy = point.y as f64 - start.y as f64;
            ((point_dx * dx + point_dy * dy) / (segment_len * segment_len)).clamp(0.0, 1.0)
        };
        let projected_x = start.x as f64 + parameter * dx;
        let projected_y = start.y as f64 + parameter * dy;
        let distance_x = point.x as f64 - projected_x;
        let distance_y = point.y as f64 - projected_y;
        let distance_squared = distance_x * distance_x + distance_y * distance_y;
        let local_arc = segment_start + parameter * segment_len;
        let candidate = (distance_squared, local_arc);
        if best
            .as_ref()
            .is_none_or(|current: &(f64, f64)| candidate.0 < current.0)
        {
            best = Some(candidate);
        }
        segment_start += segment_len;
    }
    best
}

fn orient_for_join(path: &mut ExtrusionPath3D, at_start: bool, first: bool) {
    if (first && at_start) || (!first && !at_start) {
        path.points.reverse();
    }
}
