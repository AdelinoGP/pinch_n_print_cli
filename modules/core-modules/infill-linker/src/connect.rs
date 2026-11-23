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

pub use crate::graph::contour_stub;
use crate::graph::{contour_connector, BoundaryInfillGraph, BoundaryRing, RingDirection};
use slicer_ir::{mm_to_units, ExtrusionPath3D, Point2, Point3WithWidth};
use slicer_sdk::error::ModuleError;

const ENDPOINT_WIDTH_EPSILON: f32 = 0.000001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchorParams {
    pub anchor_length_mm: f32,
    pub anchor_length_max_mm: f32,
}

impl AnchorParams {
    pub const UNLIMITED_MM: f32 = 1000.0;
    pub const DONT_CONNECT_MAX_MM: f32 = 0.05;

    #[must_use]
    pub fn solid() -> Self {
        Self {
            anchor_length_mm: Self::UNLIMITED_MM,
            anchor_length_max_mm: Self::UNLIMITED_MM,
        }
    }

    #[must_use]
    pub fn dont_connect(&self) -> bool {
        self.anchor_length_max_mm < Self::DONT_CONNECT_MAX_MM
    }

    /// Resolve anchor lengths from the region config.
    ///
    /// Packet 06 (AC-3): `infill_anchor_max` is declared
    /// `float_or_percent` with a registry default, so a bound view always
    /// holds it; a missing or mistyped value is a fatal config defect, not
    /// a fallback case. `infill_anchor` keeps its excluded
    /// `get_abs_value(..)` + `4.0 * base_spacing` fallback chain, and a
    /// `None` region config keeps the old default behavior.
    pub fn from_config(
        config: Option<&slicer_ir::ConfigView>,
        base_spacing_mm: f32,
    ) -> Result<Self, ModuleError> {
        let base_spacing = base_spacing_mm as f64;
        let anchor_length = config
            .and_then(|config| config.get_abs_value("infill_anchor", base_spacing))
            .unwrap_or(4.0 * base_spacing);
        let anchor_length_max = config
            .map(|config| config.require_abs_value("infill_anchor_max", base_spacing))
            .transpose()
            .map_err(ModuleError::from)?
            .unwrap_or(20.0);
        let anchor_length = nonnegative_finite_mm(anchor_length);
        let anchor_length_max = nonnegative_finite_mm(anchor_length_max);

        Ok(Self {
            anchor_length_mm: anchor_length.min(anchor_length_max),
            anchor_length_max_mm: anchor_length_max,
        })
    }
}

fn nonnegative_finite_mm(value: f64) -> f32 {
    if value.is_finite() && value >= 0.0 {
        let value = value as f32;
        if value.is_finite() {
            return value;
        }
    }
    0.0
}

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
    /// The endpoint's own coordinates. A join finds which end of a (possibly
    /// already merged) path carries this endpoint by comparing against it.
    point: (f32, f32),
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
    anchor: AnchorParams,
) -> Vec<ExtrusionPath3D> {
    if anchor.dont_connect() {
        return paths;
    }

    let mut active = paths
        .into_iter()
        .filter(|path| path.points.len() >= 2)
        .map(Some)
        .collect::<Vec<_>>();
    let anchor_length_max_units =
        if anchor.anchor_length_max_mm.is_finite() && anchor.anchor_length_max_mm > 0.0 {
            mm_to_units(anchor.anchor_length_max_mm) as f64
        } else {
            0.0
        };
    let anchor_length_units =
        if anchor.anchor_length_mm.is_finite() && anchor.anchor_length_mm > 0.0 {
            mm_to_units(anchor.anchor_length_mm) as f64
        } else {
            0.0
        };

    let mut candidates = nearest_pair_candidates(&active, graph);
    candidates.sort_by(|left, right| {
        left.distance
            .total_cmp(&right.distance)
            .then_with(|| endpoint_order(&left.first, &right.first))
            .then_with(|| endpoint_order(&left.second, &right.second))
    });
    let mut consumed = vec![[false; 2]; active.len()];
    // Canonical `connect_infill`'s `merged_with` table: a merged polyline lives
    // in the lower of its two slots, and an endpoint of a path that has been
    // merged away resolves to that slot through `merged_representative`.
    let mut merged_with = (0..active.len()).collect::<Vec<_>>();
    let mut boundary_positions = vec![Vec::new(); graph.rings().len()];
    for candidate in &candidates {
        for endpoint in [candidate.first, candidate.second] {
            if let Some(positions) = boundary_positions.get_mut(endpoint.position.ring_index) {
                positions.push(endpoint.position.arc_position);
            }
        }
    }
    for positions in &mut boundary_positions {
        positions.sort_by(f64::total_cmp);
    }

    for candidate in candidates {
        if endpoint_consumed(&consumed, candidate.first)
            || endpoint_consumed(&consumed, candidate.second)
        {
            continue;
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
            continue;
        };

        let first_index = merged_representative(&mut merged_with, candidate.first.path_index);
        let second_index = merged_representative(&mut merged_with, candidate.second.path_index);
        // Both endpoints already belong to one merged polyline: canonical only
        // connects when `polyline_idx1 != polyline_idx2` (never closes a loop).
        if first_index == second_index
            || active[first_index].is_none()
            || active[second_index].is_none()
        {
            continue;
        }

        let (first, second) = if first_index < second_index {
            let (left, right) = active.split_at_mut(second_index);
            (left[first_index].take(), right[0].take())
        } else {
            let (left, right) = active.split_at_mut(first_index);
            (right[0].take(), left[second_index].take())
        };
        let (Some(mut first), Some(mut second)) = (first, second) else {
            continue;
        };

        // Orient by geometry, not by bookkeeping: the joined endpoint must be
        // `first`'s last point and `second`'s first point. Canonical compares
        // the T-joint's contour point with `polyline.points.front()` /
        // `.back()`. A merged polyline no longer starts with the path whose
        // endpoint a later candidate names, so a per-slot orientation flag goes
        // stale after the first merge and splices the wrong end — a bare chord
        // across the interior (benchy L33 sparse infill).
        let (Some(first_at_start), Some(second_at_start)) = (
            endpoint_side(&first, candidate.first.point),
            endpoint_side(&second, candidate.second.point),
        ) else {
            active[first_index] = Some(first);
            active[second_index] = Some(second);
            continue;
        };
        mark_endpoint_consumed(&mut consumed, candidate.first);
        mark_endpoint_consumed(&mut consumed, candidate.second);
        let joinable = candidate.distance < anchor_length_max_units;
        if joinable || anchor_length_units > 0.0 {
            if first_at_start {
                first.points.reverse();
            }
            if !second_at_start {
                second.points.reverse();
            }
        }

        if joinable {
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
            let (lower, upper) = (first_index.min(second_index), first_index.max(second_index));
            active[lower] = Some(first);
            merged_with[upper] = lower;
        } else if anchor_length_units > 0.0 {
            let direction = ring
                .directed_distance(
                    candidate.first.position.arc_position,
                    candidate.second.position.arc_position,
                )
                .1;
            if let (Some(first_anchor), Some(second_anchor)) =
                (first.points.last().copied(), second.points.first().copied())
            {
                let first_budget = stub_budget(
                    ring,
                    candidate.first.position.arc_position,
                    direction,
                    anchor_length_units,
                    &boundary_positions[candidate.first.position.ring_index],
                );
                let second_budget = stub_budget(
                    ring,
                    candidate.second.position.arc_position,
                    opposite_direction(direction),
                    anchor_length_units,
                    &boundary_positions[candidate.second.position.ring_index],
                );
                let first_stub = contour_stub(
                    ring,
                    candidate.first.position.arc_position,
                    direction,
                    first_budget,
                    &first_anchor,
                );
                let second_stub = contour_stub(
                    ring,
                    candidate.second.position.arc_position,
                    opposite_direction(direction),
                    second_budget,
                    &second_anchor,
                );
                first.points.extend(first_stub);
                if !second_stub.is_empty() {
                    let mut second_stub = second_stub;
                    second_stub.reverse();
                    second_stub.extend(second.points);
                    second.points = second_stub;
                }
            }
            active[first_index] = Some(first);
            active[second_index] = Some(second);
        } else {
            active[first_index] = Some(first);
            active[second_index] = Some(second);
        }
    }

    active.into_iter().flatten().collect()
}

/// Orders and connects extrusion paths by their nearest boundary endpoints.
pub fn chain_or_connect_infill(
    paths: Vec<ExtrusionPath3D>,
    graph: &BoundaryInfillGraph,
    anchor: AnchorParams,
) -> Vec<ExtrusionPath3D> {
    let mut paths = if anchor.dont_connect() {
        paths
    } else {
        connect_infill(paths, graph, anchor)
    };
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

fn nearest_pair_candidates(
    active: &[Option<ExtrusionPath3D>],
    graph: &BoundaryInfillGraph,
) -> Vec<Candidate> {
    let mut endpoints = active
        .iter()
        .enumerate()
        .flat_map(|(path_index, path)| {
            let path = path.as_ref()?;
            // Canonical admits only on-contour endpoints as T-joints
            // (boundary_idx_unconnected otherwise); an interior end anchors no
            // take. A path with one anchored end still contributes that end —
            // filtering the whole path on either end would also drop every
            // legitimately clipped path whose far end stops mid-span.
            let (head, tail) = (path.points.first()?, path.points.last()?);
            let first = boundary_position(graph, head).map(|position| Endpoint {
                path_index,
                at_start: true,
                position,
                point: (head.x, head.y),
            });
            let last = boundary_position(graph, tail).map(|position| Endpoint {
                path_index,
                at_start: false,
                position,
                point: (tail.x, tail.y),
            });
            Some([first, last].into_iter().flatten())
        })
        .flatten()
        .collect::<Vec<_>>();
    endpoints.sort_by(endpoint_order);

    let rings = graph.rings();
    let mut by_ring: Vec<Vec<(f64, usize)>> = vec![Vec::new(); rings.len()];
    for (ep_idx, endpoint) in endpoints.iter().enumerate() {
        if let Some(ring) = rings.get(endpoint.position.ring_index) {
            let local = ring.local_position(endpoint.position.arc_position);
            by_ring[endpoint.position.ring_index].push((local, ep_idx));
        }
    }
    for ring_eps in &mut by_ring {
        ring_eps.sort_by(|left, right| left.0.total_cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    }

    let mut best_for: Vec<Option<(usize, f64)>> = vec![None; endpoints.len()];
    for (ring_index, ring_eps) in by_ring.iter().enumerate() {
        let m = ring_eps.len();
        if m < 2 {
            continue;
        }
        let ring = &rings[ring_index];
        let ring_len = ring.length;

        for k in 0..m {
            let (my_local, my_idx) = ring_eps[k];
            let endpoint = endpoints[my_idx];
            let mut best: Option<(usize, f64)> = None;

            if !ring_len.is_finite() || ring_len <= 0.0 {
                for &(_, other_idx) in ring_eps {
                    if other_idx == my_idx {
                        continue;
                    }
                    let other = endpoints[other_idx];
                    if other.path_index == endpoint.path_index
                        || !compatible_paths(active, endpoint.path_index, other.path_index)
                    {
                        continue;
                    }
                    let (distance, _) = ring.directed_distance(
                        endpoint.position.arc_position,
                        other.position.arc_position,
                    );
                    let better = best.is_none_or(|(best_idx, best_dist)| {
                        distance
                            .total_cmp(&best_dist)
                            .then_with(|| other_idx.cmp(&best_idx))
                            == Ordering::Less
                    });
                    if better {
                        best = Some((other_idx, distance));
                    }
                }
                best_for[my_idx] = best;
                continue;
            }

            // Forward circular scan from k+1..: stops as soon as forward arc
            // distance exceeds the current best distance.
            for step in 1..m {
                let j = (k + step) % m;
                let (other_local, other_idx) = ring_eps[j];
                let fwd = (other_local - my_local).rem_euclid(ring_len);
                if let Some((_, best_dist)) = best {
                    if fwd > best_dist {
                        break;
                    }
                }
                let other = endpoints[other_idx];
                if other.path_index == endpoint.path_index
                    || !compatible_paths(active, endpoint.path_index, other.path_index)
                {
                    continue;
                }
                let bwd = (my_local - other_local).rem_euclid(ring_len);
                let distance = if fwd <= bwd { fwd } else { bwd };
                let better = best.is_none_or(|(best_idx, best_dist)| {
                    distance
                        .total_cmp(&best_dist)
                        .then_with(|| other_idx.cmp(&best_idx))
                        == Ordering::Less
                });
                if better {
                    best = Some((other_idx, distance));
                }
            }

            if best.is_none() {
                continue;
            }

            // Backward circular scan from k-1..: stops as soon as backward arc
            // distance exceeds the current best distance.
            for step in 1..m {
                let j = (k + m - step) % m;
                let (other_local, other_idx) = ring_eps[j];
                let bwd = (my_local - other_local).rem_euclid(ring_len);
                if let Some((_, best_dist)) = best {
                    if bwd > best_dist {
                        break;
                    }
                }
                let other = endpoints[other_idx];
                if other.path_index == endpoint.path_index
                    || !compatible_paths(active, endpoint.path_index, other.path_index)
                {
                    continue;
                }
                let fwd = (other_local - my_local).rem_euclid(ring_len);
                let distance = if fwd <= bwd { fwd } else { bwd };
                let better = best.is_none_or(|(best_idx, best_dist)| {
                    distance
                        .total_cmp(&best_dist)
                        .then_with(|| other_idx.cmp(&best_idx))
                        == Ordering::Less
                });
                if better {
                    best = Some((other_idx, distance));
                }
            }

            best_for[my_idx] = best;
        }
    }

    let mut candidates = Vec::with_capacity(endpoints.len());
    for (ep_idx, endpoint) in endpoints.iter().enumerate() {
        if let Some((second_idx, distance)) = best_for[ep_idx] {
            candidates.push(Candidate {
                first: *endpoint,
                second: endpoints[second_idx],
                distance,
            });
        }
    }
    candidates
}

fn endpoint_consumed(consumed: &[[bool; 2]], endpoint: Endpoint) -> bool {
    consumed[endpoint.path_index][endpoint_slot(endpoint)]
}

fn mark_endpoint_consumed(consumed: &mut [[bool; 2]], endpoint: Endpoint) {
    consumed[endpoint.path_index][endpoint_slot(endpoint)] = true;
}

fn endpoint_slot(endpoint: Endpoint) -> usize {
    if endpoint.at_start {
        0
    } else {
        1
    }
}

fn opposite_direction(direction: RingDirection) -> RingDirection {
    match direction {
        RingDirection::Forward => RingDirection::Backward,
        RingDirection::Backward => RingDirection::Forward,
    }
}

fn stub_budget(
    ring: &BoundaryRing,
    from: f64,
    direction: RingDirection,
    requested: f64,
    boundary_positions: &[f64],
) -> f64 {
    if !ring.length.is_finite() || ring.length <= 0.0 {
        return requested;
    }
    let from = ring.local_position(from);
    let next = boundary_positions
        .iter()
        .map(|position| ring.local_position(*position))
        .filter_map(|position| {
            let distance = match direction {
                RingDirection::Forward => (position - from).rem_euclid(ring.length),
                RingDirection::Backward => (from - position).rem_euclid(ring.length),
            };
            (distance > 0.0).then_some(distance)
        })
        .min_by(|left, right| left.total_cmp(right));
    requested.min(next.unwrap_or(f64::INFINITY))
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
        // ADR-0058: per-path authored tool is a chaining-compatibility axis.
        // Every merge in `connect_infill` originates from a candidate produced
        // by `nearest_pair_candidates`, whose filter consults this predicate, so
        // differing-tool paths are never spliced. The merge itself reuses the
        // `first` path struct (`first.points.extend(second.points)`), so the
        // surviving path keeps its `tool_index`.
        && first.tool_index == second.tool_index
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
    // Canonical FillBase.cpp::create_boundary_infill_graph projects every
    // infill endpoint onto the boundary graph (`grid.closest_point_signed_
    // distance` per end) and keys the T-joint to the nearest contour — there
    // is no on-contour admission gate at all. A clipped scan line ends
    // ~overlap inside the fill boundary by construction (inset
    // `(0.5-overlap)*spacing`, e.g. 0.1 mm at defaults), so a tight
    // epsilon-gate would reject every legitimate endpoint and union linking
    // across wall-sharing regions would silently stop. The nearest
    // qualifying ring still wins; what the gate must reject is only the
    // genuinely ambiguous case — an endpoint strictly inside a HOLE, which
    // canonical leaves unconnected (`contour_idx ==
    // boundary_idx_unconnected`) because a connector routed along the hole
    // ring would drag extrusion across the void the hole reserves. So: admit
    // the nearest ring unless the point is strictly inside a hole polygon,
    // in which case admit only that hole's ring when the point sits on it
    // (within the clip stage's +-2-unit boundary tolerance, plus scan-emit
    // rounding) and reject otherwise.
    const ON_RING_TOLERANCE_UNITS_SQUARED: f64 = 16.0;
    // Strictly-inside-a-hole test up front: hole rings below are found by
    // `hole_index`, and the point-in-ring test runs on the hole polygon
    // itself (units space, exact integer arithmetic where it matters).
    let inside_hole: Option<usize> =
        graph
            .rings()
            .iter()
            .enumerate()
            .find_map(|(ring_index, ring)| {
                (ring.hole_index.is_some()
                    && ring.contains_bbox(point)
                    && point_strictly_in_ring(point, ring))
                .then_some(ring_index)
            });
    if let Some(hole_ring) = inside_hole {
        // Inside a void: only that hole's own ring can anchor, and only when
        // the point sits on it (a scan line clipped exactly at the hole
        // edge). Anything deeper has no T-joint anywhere.
        let ring = &graph.rings()[hole_ring];
        let (distance_squared, local_arc) = ring.project_bounded(point, f64::INFINITY)?;
        if distance_squared > ON_RING_TOLERANCE_UNITS_SQUARED {
            return None;
        }
        return Some(BoundaryPosition {
            ring_index: hole_ring,
            arc_position: ring.pos_of_first_point + local_arc,
        });
    }
    let mut best_distance = f64::INFINITY;
    let mut best: Option<BoundaryPosition> = None;
    for (ring_index, ring) in graph.rings().iter().enumerate() {
        let Some((distance_squared, local_arc)) = ring.project_bounded(point, best_distance) else {
            continue;
        };
        best_distance = distance_squared;
        best = Some(BoundaryPosition {
            ring_index,
            arc_position: ring.pos_of_first_point + local_arc,
        });
        if best_distance == 0.0 {
            break;
        }
    }
    best
}

/// Strict point-in-polygon over a single closed ring (units space).
/// Boundary-exact points count as NOT strictly inside — the hole-edge scan-line
/// case must still reach the tolerance check above rather than being treated as void.
fn point_strictly_in_ring(point: Point2, ring: &BoundaryRing) -> bool {
    let pts = &ring.polygon.points;
    let n = pts.len();
    if n < 3 || !ring.contains_bbox(point) {
        return false;
    }
    let x = point.x as f64;
    let y = point.y as f64;
    // On-edge counts as NOT strictly inside.
    for edge in &ring.edges {
        if point.x < edge.min_x.saturating_sub(3)
            || point.x > edge.max_x.saturating_add(3)
            || point.y < edge.min_y.saturating_sub(3)
            || point.y > edge.max_y.saturating_add(3)
        {
            continue;
        }
        let len2 = edge.dx * edge.dx + edge.dy * edge.dy;
        if len2 == 0.0 {
            continue;
        }
        let t = ((x - edge.start.x as f64) * edge.dx + (y - edge.start.y as f64) * edge.dy) / len2;
        if (0.0..=1.0).contains(&t) {
            let px = edge.start.x as f64 + t * edge.dx;
            let py = edge.start.y as f64 + t * edge.dy;
            if (x - px).hypot(y - py) <= 2.0 {
                return false;
            }
        }
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = pts[i].x as f64;
        let yi = pts[i].y as f64;
        let xj = pts[j].x as f64;
        let yj = pts[j].y as f64;
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Canonical `get_and_update_merged_with`: the slot that now holds the
/// polyline `path_index` was merged into, with path compression.
fn merged_representative(merged_with: &mut [usize], path_index: usize) -> usize {
    let mut last = path_index;
    loop {
        let lower = merged_with[last];
        if lower == last {
            merged_with[path_index] = last;
            return last;
        }
        last = lower;
    }
}

/// Which end of `path` carries `point`: `Some(true)` for the first point,
/// `Some(false)` for the last, `None` when neither does.
///
/// A path whose ends coincide reports its last point, which needs no
/// reversal on either side of a join.
fn endpoint_side(path: &ExtrusionPath3D, point: (f32, f32)) -> Option<bool> {
    let matches = |candidate: &Point3WithWidth| {
        candidate.x.to_bits() == point.0.to_bits() && candidate.y.to_bits() == point.1.to_bits()
    };
    if path.points.last().is_some_and(matches) {
        Some(false)
    } else if path.points.first().is_some_and(matches) {
        Some(true)
    } else {
        None
    }
}

#[cfg(test)]
mod il3_equivalence_tests {
    use super::*;
    use slicer_ir::{ExPolygon, ExtrusionRole, Polygon};

    fn brute_force_project_on_ring(ring: &BoundaryRing, point: Point2) -> Option<(f64, f64)> {
        let points = &ring.polygon.points;
        if points.is_empty() {
            return None;
        }
        let mut best: Option<(f64, f64)> = None;
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
            if best.is_none_or(|current| distance_squared < current.0) {
                best = Some((distance_squared, local_arc));
            }
            segment_start += segment_len;
        }
        best
    }

    fn brute_force_nearest_pair_candidates(
        active: &[Option<ExtrusionPath3D>],
        graph: &BoundaryInfillGraph,
    ) -> Vec<Candidate> {
        let mut endpoints = active
            .iter()
            .enumerate()
            .flat_map(|(path_index, path)| {
                let path = path.as_ref()?;
                let (head, tail) = (path.points.first()?, path.points.last()?);
                let first = boundary_position(graph, head).map(|position| Endpoint {
                    path_index,
                    at_start: true,
                    position,
                    point: (head.x, head.y),
                });
                let last = boundary_position(graph, tail).map(|position| Endpoint {
                    path_index,
                    at_start: false,
                    position,
                    point: (tail.x, tail.y),
                });
                Some([first, last].into_iter().flatten())
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
                        .directed_distance(
                            endpoint.position.arc_position,
                            other.position.arc_position,
                        );
                    Some((*other, distance))
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
        candidates
    }

    fn path_2pt(
        a: (f32, f32),
        b: (f32, f32),
        width: f32,
        tool_index: Option<u32>,
    ) -> ExtrusionPath3D {
        ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: a.0,
                    y: a.1,
                    z: 0.2,
                    width,
                    flow_factor: 1.0,
                    ..Default::default()
                },
                Point3WithWidth {
                    x: b.0,
                    y: b.1,
                    z: 0.2,
                    width,
                    flow_factor: 1.0,
                    ..Default::default()
                },
            ],
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
            tool_index,
            order_lock: None, // exhaustive: ExtrusionPath3D has no Default impl
        }
    }

    fn multi_hole_graph() -> BoundaryInfillGraph {
        let ep1 = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(40.0, 0.0),
                    Point2::from_mm(40.0, 30.0),
                    Point2::from_mm(20.0, 35.0),
                    Point2::from_mm(0.0, 30.0),
                ],
            },
            holes: vec![
                Polygon {
                    points: vec![
                        Point2::from_mm(5.0, 5.0),
                        Point2::from_mm(15.0, 5.0),
                        Point2::from_mm(15.0, 15.0),
                        Point2::from_mm(5.0, 15.0),
                    ],
                },
                Polygon {
                    points: vec![
                        Point2::from_mm(24.0, 8.0),
                        Point2::from_mm(34.0, 8.0),
                        Point2::from_mm(34.0, 22.0),
                        Point2::from_mm(24.0, 22.0),
                    ],
                },
            ],
        };
        let ep2 = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(50.0, 0.0),
                    Point2::from_mm(70.0, 0.0),
                    Point2::from_mm(70.0, 20.0),
                    Point2::from_mm(50.0, 20.0),
                ],
            },
            holes: Vec::new(),
        };
        BoundaryInfillGraph::new(&[ep1, ep2])
    }

    #[test]
    fn bounded_ring_projection_is_bit_identical_to_bruteforce_across_multi_hole_graph() {
        let graph = multi_hole_graph();
        let mut state: u64 = 0x517CC1B727220A95;
        let mut next_mm = |lo: f32, hi: f32| -> f32 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let frac = ((state >> 40) as f32) / ((1u32 << 24) as f32);
            lo + (hi - lo) * frac
        };

        // Include exact vertices, wrap-around origin (0,0), hole interiors, hole edges, and random points.
        let mut probes = vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(40.0, 0.0),
            Point2::from_mm(5.0, 5.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(15.0, 10.0),
            Point2::from_mm(29.0, 15.0),
            Point2::from_mm(45.0, 10.0),
            Point2::from_mm(50.0, 0.0),
        ];
        for _ in 0..250 {
            probes.push(Point2::from_mm(next_mm(-5.0, 75.0), next_mm(-5.0, 40.0)));
        }

        for point in probes {
            // Check per-ring projection bit identity.
            for ring in graph.rings() {
                let fast = ring.project_bounded(point, f64::INFINITY);
                let slow = brute_force_project_on_ring(ring, point);
                assert_eq!(
                    fast.map(|(d, a)| (d.to_bits(), a.to_bits())),
                    slow.map(|(d, a)| (d.to_bits(), a.to_bits())),
                    "ring projection mismatch for point {point:?}"
                );
            }
            // Check cross-ring graph.project bit identity.
            let mut slow_best: Option<(f64, f64)> = None;
            for ring in graph.rings() {
                if let Some((d2, local_arc)) = brute_force_project_on_ring(ring, point) {
                    let pos = ring.pos_of_first_point + local_arc;
                    if slow_best.is_none_or(|(bd, _)| d2 < bd) {
                        slow_best = Some((d2, pos));
                    }
                }
            }
            assert_eq!(
                graph.project(point).map(f64::to_bits),
                slow_best.map(|(_, pos)| pos.to_bits()),
                "graph.project mismatch for point {point:?}"
            );
        }
    }

    #[test]
    fn sorted_ring_nearest_pair_candidates_matches_bruteforce_across_seams_ties_and_holes() {
        let graph = multi_hole_graph();
        // Craft endpoints deliberately straddling ring wrap-around seam (0,0),
        // coincident endpoints at identical arc positions, hole-edge endpoints,
        // incompatible tool/width endpoints interleaved with compatible ones,
        // and multi-island rings.
        let active: Vec<Option<ExtrusionPath3D>> = vec![
            Some(path_2pt((0.0, 0.0), (0.0, 29.5), 0.4, None)), // seam at 0 and near L
            Some(path_2pt((0.0, 0.0), (1.0, 0.0), 0.4, None)),  // coincident at seam 0
            Some(path_2pt((0.0, 29.8), (39.5, 0.0), 0.4, None)), // just before wrap L
            Some(path_2pt((0.2, 0.0), (20.0, 0.0), 0.4, Some(1))), // incompatible tool at seam!
            Some(path_2pt((0.3, 0.0), (21.0, 0.0), 0.6, None)), // incompatible width at seam!
            Some(path_2pt((5.0, 7.0), (5.0, 12.0), 0.4, None)), // hole 0 edges
            Some(path_2pt((5.0, 7.0), (15.0, 12.0), 0.4, None)), // coincident on hole 0
            Some(path_2pt((15.0, 8.0), (24.0, 10.0), 0.4, None)), // hole 0 -> hole 1
            Some(path_2pt((24.0, 11.0), (34.0, 15.0), 0.4, None)), // hole 1 edges
            Some(path_2pt((50.0, 0.0), (50.0, 19.5), 0.4, None)), // island 2 seam
            Some(path_2pt((50.5, 0.0), (70.0, 10.0), 0.4, None)), // island 2
            Some(path_2pt((50.0, 19.9), (60.0, 20.0), 0.4, None)), // island 2 wrap
            Some(path_2pt((10.0, 10.0), (10.0, 11.0), 0.4, None)), // strictly inside hole -> rejected
        ];

        let fast = nearest_pair_candidates(&active, &graph);
        let slow = brute_force_nearest_pair_candidates(&active, &graph);
        assert_eq!(fast.len(), slow.len(), "candidate count mismatch");
        for (idx, (f, s)) in fast.iter().zip(slow.iter()).enumerate() {
            assert_eq!(
                (
                    f.first.path_index,
                    f.first.at_start,
                    f.second.path_index,
                    f.second.at_start,
                    f.distance.to_bits()
                ),
                (
                    s.first.path_index,
                    s.first.at_start,
                    s.second.path_index,
                    s.second.at_start,
                    s.distance.to_bits()
                ),
                "candidate[{idx}] diverged from brute-force reference"
            );
        }
    }
}
