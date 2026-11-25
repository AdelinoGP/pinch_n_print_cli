// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PrintObject.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Deterministic geometry for internal bridges over sparse infill.

use crate::polygon_ops::{
    closing_ex, difference, intersection, offset, opening, union, OffsetJoinType,
};
use slicer_ir::{ExPolygon, Point2, Polygon};
use std::f32::consts::PI;

const SAMPLE_STEP: f64 = 20_000.0;

const SCALED_EPSILON_MM: f64 = 0.0001;
const UNITS_PER_MM: f64 = 10_000.0;

/// Canonical `target_flow_height_factor` fallback captured from PrintObject.cpp.
pub const BRIDGE_FLOW_HEIGHT_FACTOR: f32 = 1.0;

/// Geometry committed for one layer of bridge-depth harvesting.
#[derive(Clone, Debug, Default)]
pub struct BridgeDepthLayer {
    /// Layer print height in millimetres.
    pub print_z: f32,
    /// Lower-layer areas resolved as sparse infill.
    pub sparse_infill: Vec<ExPolygon>,
    /// Lower-layer areas that must mask sparse infill.
    pub not_sparse_infill: Vec<ExPolygon>,
}

/// Candidate geometry committed for one layer of bridge clustering.
#[derive(Clone, Debug, Default)]
pub struct BridgeCandidateLayer {
    /// Layer print height in millimetres.
    pub print_z: f32,
    /// Qualified bridge polygons authored on this layer.
    pub new_polys: Vec<ExPolygon>,
}

/// First index of the depth window [`gather_areas_w_depth`] can reach from
/// `layer_index`, given the layers' ascending `print_z` values.
///
/// The gather walks downward from `layer_index - 1` and stops at the first
/// layer below `bottom_z` that is not the immediately preceding layer, so it
/// consumes at most a one-flow-height suffix regardless of how tall the stack
/// is. Callers materialise `BridgeDepthLayer`s by cloning whole polygon sets,
/// which is why the window rule is exposed here rather than left implicit: it
/// lets a caller build `layers` for `start..=layer_index` only, instead of the
/// whole object below the layer. `gather_areas_w_depth(&layers[start..], i -
/// start, ..)` yields the same polygons as passing the full prefix.
pub fn depth_window_start(
    print_zs: &[f32],
    layer_index: usize,
    target_flow_height: f32,
    target_flow_height_factor: f32,
) -> usize {
    let Some(&print_z) = print_zs.get(layer_index) else {
        return layer_index;
    };
    let bottom_z =
        print_z - target_flow_height * target_flow_height_factor - SCALED_EPSILON_MM as f32;
    for i in (0..layer_index).rev() {
        if print_zs[i] < bottom_z && i < layer_index - 1 {
            return i + 1;
        }
    }
    0
}

/// First index whose layer still passes
/// [`remove_filled_polygons_on_lower_layers`]'s `print_z >= bottom_z` filter.
///
/// Same purpose as [`depth_window_start`]: the filter keeps only a
/// one-flow-height suffix, so a caller need not clone every lower layer's
/// candidate polygons to hand them over. `print_zs` must be ascending.
pub fn filled_window_start(
    print_zs: &[f32],
    current_print_z: f32,
    target_flow_height: f32,
) -> usize {
    let bottom_z = current_print_z - target_flow_height - SCALED_EPSILON_MM as f32;
    print_zs.partition_point(|&z| z < bottom_z)
}

/// Gather sparse lower-layer area within the canonical depth window.
pub fn gather_areas_w_depth(
    layers: &[BridgeDepthLayer],
    layer_index: usize,
    target_flow_height: f32,
    target_flow_height_factor: f32,
) -> Vec<ExPolygon> {
    let Some(layer) = layers.get(layer_index) else {
        return Vec::new();
    };
    let bottom_z =
        layer.print_z - target_flow_height * target_flow_height_factor - SCALED_EPSILON_MM as f32;
    let mut sparse = Vec::new();
    let mut solid = Vec::new();
    for i in (0..layer_index).rev() {
        let lower = &layers[i];
        if lower.print_z < bottom_z && i < layer_index - 1 {
            break;
        }
        sparse.extend(lower.sparse_infill.iter().cloned());
        solid.extend(lower.not_sparse_infill.iter().cloned());
    }
    if sparse.is_empty() {
        return Vec::new();
    }
    let sparse = closing_ex(
        &union(&sparse, &[]),
        SCALED_EPSILON_MM,
        OffsetJoinType::Miter,
    );
    let solid = closing_ex(
        &union(&solid, &[]),
        SCALED_EPSILON_MM,
        OffsetJoinType::Miter,
    );
    difference(&sparse, &solid)
}

/// Group consecutive candidate layers when both their Z gap and area overlap
/// satisfy the canonical bridge-thread conditions.
pub fn cluster_candidate_layers(
    layers: &[BridgeCandidateLayer],
    bridging_flow_height: f32,
    target_flow_height_factor: f32,
) -> Vec<Vec<usize>> {
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for (index, candidate) in layers.iter().enumerate() {
        let joins = clusters.last().and_then(|cluster| {
            let previous = cluster.last().copied()?;
            let gap = candidate.print_z - layers[previous].print_z;
            (gap < bridging_flow_height * target_flow_height_factor - SCALED_EPSILON_MM as f32
                && !intersection(&candidate.new_polys, &layers[previous].new_polys).is_empty())
            .then_some(())
        });
        if joins.is_some() {
            clusters.last_mut().expect("cluster exists").push(index);
        } else {
            clusters.push(vec![index]);
        }
    }
    clusters
}

/// Remove candidate polygons already filled on lower layers within the depth.
pub fn remove_filled_polygons_on_lower_layers(
    deep_infill_area: &[ExPolygon],
    lower_candidates: &[BridgeCandidateLayer],
    current_print_z: f32,
    target_flow_height: f32,
) -> Vec<ExPolygon> {
    let bottom_z = current_print_z - target_flow_height - SCALED_EPSILON_MM as f32;
    let filled: Vec<ExPolygon> = lower_candidates
        .iter()
        .filter(|candidate| candidate.print_z >= bottom_z)
        .flat_map(|candidate| candidate.new_polys.iter().cloned())
        .collect();
    if filled.is_empty() {
        deep_infill_area.to_vec()
    } else {
        difference(deep_infill_area, &union(&filled, &[]))
    }
}

/// Compute the unsupported portion left after accounting for lower-layer fills
/// and solids. Distances follow the workspace's 100 nm coordinate units.
pub fn unsupported_span_areas(
    lower_fills: &[ExPolygon],
    lower_solids: &[ExPolygon],
    spacing_mm: f32,
    expansion_multiplier: f64,
) -> Vec<ExPolygon> {
    if lower_fills.is_empty() || spacing_mm <= 0.0 {
        return Vec::new();
    }
    let spacing = spacing_mm as f64;
    // Canonical: initially consider the whole layer unsupported — the carrier
    // is the lower fills themselves (closing(SCALED_EPSILON)), NOT the bbox
    // complement of the fills. RC-A fix.
    let mut unsupported = closing_ex(lower_fills, SCALED_EPSILON_MM, OffsetJoinType::Miter);
    unsupported = offset(
        &unsupported,
        -(expansion_multiplier * spacing) as f32,
        OffsetJoinType::Miter,
        0.0,
    );
    if lower_solids.is_empty() || unsupported.is_empty() {
        return unsupported;
    }
    let solids_shrunk = offset(lower_solids, -spacing as f32, OffsetJoinType::Miter, 0.0);
    let solids_grown = offset(
        &solids_shrunk,
        ((1.0 + expansion_multiplier) * spacing) as f32,
        OffsetJoinType::Miter,
        0.0,
    );
    difference(&unsupported, &solids_grown)
}

/// Axis-aligned bounding box in workspace units: `(min_x, min_y, max_x, max_y)`.
pub type UnitBox = (i64, i64, i64, i64);

fn ring_box(points: &[Point2]) -> Option<UnitBox> {
    let first = points.first()?;
    Some(points.iter().fold(
        (first.x, first.y, first.x, first.y),
        |(min_x, min_y, max_x, max_y), p| {
            (
                min_x.min(p.x),
                min_y.min(p.y),
                max_x.max(p.x),
                max_y.max(p.y),
            )
        },
    ))
}

/// Union bounding box of every ring of `polygons`, or `None` when they carry
/// no points.
pub fn polygons_box(polygons: &[ExPolygon]) -> Option<UnitBox> {
    polygons
        .iter()
        .flat_map(|polygon| std::iter::once(&polygon.contour).chain(polygon.holes.iter()))
        .filter_map(|ring| ring_box(&ring.points))
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
}

/// `bbox` grown by `margin_mm` on every side, rounded outward to whole units.
pub fn grow_box(bbox: UnitBox, margin_mm: f64) -> UnitBox {
    let margin = (margin_mm * UNITS_PER_MM).ceil() as i64;
    (
        bbox.0 - margin,
        bbox.1 - margin,
        bbox.2 + margin,
        bbox.3 + margin,
    )
}

fn boxes_overlap(a: UnitBox, b: UnitBox) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

/// Drop every polygon, and every hole, whose bounding box misses `window`.
///
/// Nothing is clipped: each kept ring is passed through unchanged, so no new
/// vertex is created. A ring whose box misses the window can neither enclose
/// nor cross a point of the window, so removing it changes the winding number
/// — and therefore every boolean or offset result — only outside the window
/// grown by the operation's own reach. Returns the input borrowed when nothing
/// is dropped.
pub fn polygons_touching_box(
    polygons: &[ExPolygon],
    window: UnitBox,
) -> std::borrow::Cow<'_, [ExPolygon]> {
    let touches = |points: &[Point2]| ring_box(points).is_some_and(|b| boxes_overlap(b, window));
    let unchanged = polygons.iter().all(|polygon| {
        touches(&polygon.contour.points) && polygon.holes.iter().all(|hole| touches(&hole.points))
    });
    if unchanged {
        return std::borrow::Cow::Borrowed(polygons);
    }
    std::borrow::Cow::Owned(
        polygons
            .iter()
            .filter(|polygon| touches(&polygon.contour.points))
            .map(|polygon| ExPolygon {
                contour: polygon.contour.clone(),
                holes: polygon
                    .holes
                    .iter()
                    .filter(|hole| touches(&hole.points))
                    .cloned()
                    .collect(),
            })
            .collect(),
    )
}

/// Miter limit [`offset`] applies (Clipper2's default); a miter-joined offset
/// by `d` moves no boundary point farther than this times `|d|`.
const OFFSET_MITER_REACH: f64 = 2.0;

/// [`unsupported_span_areas`] restricted to what `surfaces` can observe.
///
/// [`qualify_internal_bridge_surface`] reads the unsupported set only through
/// `intersection(surface, unsupported)`, so only its geometry over the
/// surfaces matters. Each step of `unsupported_span_areas` is local: the fill
/// erosion moves boundaries at most `2 × m × s` (miter reach of
/// `offset(-m s)`), and the solid shrink-grow at most `2 s + 2 (1 + m) s`;
/// the `closing_ex` epsilon passes are negligible against the one-spacing
/// safety pad. Lower-layer polygons and holes wholly outside the surfaces'
/// bounding box grown by that reach are dropped with
/// [`polygons_touching_box`] before the call, which leaves the result over the
/// surfaces vertex-for-vertex unchanged.
pub fn unsupported_span_areas_near(
    lower_fills: &[ExPolygon],
    lower_solids: &[ExPolygon],
    spacing_mm: f32,
    expansion_multiplier: f64,
    surfaces: &[ExPolygon],
) -> Vec<ExPolygon> {
    let Some(bbox) = polygons_box(surfaces) else {
        return unsupported_span_areas(lower_fills, lower_solids, spacing_mm, expansion_multiplier);
    };
    let spacing = f64::from(spacing_mm);
    let fill_reach = OFFSET_MITER_REACH * expansion_multiplier * spacing;
    let solid_reach =
        OFFSET_MITER_REACH * spacing + OFFSET_MITER_REACH * (1.0 + expansion_multiplier) * spacing;
    let margin_mm = fill_reach.max(solid_reach) + spacing + 8.0 * SCALED_EPSILON_MM;
    let window = grow_box(bbox, margin_mm);
    let fills = polygons_touching_box(lower_fills, window);
    let solids = polygons_touching_box(lower_solids, window);
    unsupported_span_areas(&fills, &solids, spacing_mm, expansion_multiplier)
}

/// Qualify the genuinely unsupported part of one internal bridge surface.
pub fn qualify_internal_bridge_surface(
    surface: &ExPolygon,
    unsupported: &[ExPolygon],
    spacing_mm: f32,
    nofilter: bool,
) -> Option<Vec<ExPolygon>> {
    if spacing_mm <= 0.0 {
        return None;
    }
    let unsupported_surface = intersection(std::slice::from_ref(surface), unsupported);
    if unsupported_surface.is_empty() {
        return None;
    }
    let spacing_units = spacing_mm as f64 * UNITS_PER_MM;
    let unsupported_area = expolygons_area(&unsupported_surface);
    let surface_area = expolygon_area(surface);
    let partially_supported = unsupported_area < surface_area - 1.0;
    if !nofilter && partially_supported && unsupported_area <= 9.0 * spacing_units * spacing_units {
        return None;
    }

    let mut worth = intersection(
        std::slice::from_ref(surface),
        &offset(
            &unsupported_surface,
            4.0 * spacing_mm,
            OffsetJoinType::Miter,
            0.0,
        ),
    );
    let expanded_worth = offset(&worth, spacing_mm, OffsetJoinType::Miter, 0.0);
    for polygon in difference(std::slice::from_ref(surface), &expanded_worth) {
        let area = expolygon_area(&polygon);
        if area > spacing_units * spacing_units && area < spacing_units * 120_000.0 {
            worth.push(polygon);
        }
    }
    let closed = closing_ex(&worth, SCALED_EPSILON_MM, OffsetJoinType::Miter);
    Some(intersection(&closed, std::slice::from_ref(surface)))
}

fn expolygons_area(polygons: &[ExPolygon]) -> f64 {
    polygons.iter().map(expolygon_area).sum()
}

fn expolygon_area(polygon: &ExPolygon) -> f64 {
    fn ring_area(ring: &Polygon) -> f64 {
        ring.points
            .iter()
            .zip(ring.points.iter().cycle().skip(1))
            .take(ring.points.len())
            .map(|(a, b)| a.x as f64 * b.y as f64 - b.x as f64 * a.y as f64)
            .sum::<f64>()
            .abs()
            / 2.0
    }
    ring_area(&polygon.contour) - polygon.holes.iter().map(ring_area).sum::<f64>()
}

// -----------------------------------------------------------------------------
// Portions of this helper are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PrintObject.cpp
//
// This is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
/// Apply the canonical expansion-zone treatment to one bridge candidate.
///
/// `deep_infill_area` is the sparse-infill footprint at this construction site;
/// `provenance` is the qualified internal-bridge set that produced the candidate.
pub fn expand_candidate_area(
    candidate: &[ExPolygon],
    expansion_source: &[ExPolygon],
    deep_infill_area: &[ExPolygon],
    provenance: &[ExPolygon],
    spacing_mm: f32,
) -> Vec<ExPolygon> {
    if candidate.is_empty() || spacing_mm <= 0.0 {
        return Vec::new();
    }
    let limiting_area = union(candidate, expansion_source);
    let grown = offset(&limiting_area, spacing_mm, OffsetJoinType::Miter, 0.0);
    let clipped = intersection(&grown, deep_infill_area);
    let qualified: Vec<ExPolygon> = clipped
        .into_iter()
        .filter(|area| !intersection(std::slice::from_ref(area), provenance).is_empty())
        .collect();
    if qualified.is_empty() {
        return Vec::new();
    }
    let opened = opening(
        &qualified,
        f64::from(spacing_mm) * 0.75,
        OffsetJoinType::Miter,
    );
    closing_ex(&opened, f64::from(spacing_mm), OffsetJoinType::Miter)
}

/// Select a bridge direction from arc-length-weighted nearest anchor samples.
///
/// Same result as the original brute-force scan, bit for bit, without
/// its two quadratic scans: the nearest anchor segment of each sample comes
/// from a bounding-box hierarchy ([`AnchorSegmentIndex`]) that evaluates the
/// identical distance expression and keeps the identical first-segment tie
/// rule, and the angular vote only visits the samples that can fall inside a
/// candidate's window (see [`vote_bridging_angle`]).
pub fn determine_bridging_angle(
    anchors: &[Vec<Point2>],
    area_edges: &[Vec<Point2>],
    override_deg: f32,
) -> f32 {
    if override_deg > 0.0 {
        return override_deg;
    }
    let index = AnchorSegmentIndex::new(anchors);
    let mut samples = Vec::new();
    if index.is_empty() {
        return 0.001;
    }
    let mut stack = Vec::with_capacity(64);
    for edge in area_edges {
        for pair in edge.windows(2) {
            let dx = (pair[1].x - pair[0].x) as f64;
            let dy = (pair[1].y - pair[0].y) as f64;
            let length = dx.hypot(dy);
            if length == 0.0 {
                continue;
            }
            let count = (length / SAMPLE_STEP).ceil().max(1.0) as usize;
            for n in 0..count {
                let t = ((n as f64 + 0.5) / count as f64).min(1.0);
                let p = (pair[0].x as f64 + dx * t, pair[0].y as f64 + dy * t);
                if let Some(angle) = index.nearest_angle(p, &mut stack) {
                    samples.push(angle);
                }
            }
        }
    }
    if samples.is_empty() {
        return 0.001;
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    vote_bridging_angle(&samples)
}

/// Angular vote of the original `O(S²)` bridging-angle scan over ascending
/// `samples`, visiting only the samples that can pass its window test.
///
/// The reference scores every candidate against every sample. A sample passes
/// when its direction is within `window` of the candidate modulo `π`, which
/// (both angles lying in `[0, π]`) means its value lies in one of the three
/// intervals `candidate + kπ ± window`, `k ∈ {-1, 0, 1}`. Those intervals are
/// disjoint (`2 × window < π`) and ascend with `k`, so their index ranges in
/// the sorted samples are disjoint and ascending too. Each range is widened by
/// `RANGE_SLACK`, far beyond the rounding of the reference's `rem_euclid`
/// arithmetic, and then every sample in it is put through the reference's own
/// test. The passing set, the order it is summed in (ascending index) and
/// therefore `score` and `sum` are exactly the reference's. The ranges' total
/// size bounds the score, so a candidate that cannot beat `best_score` is
/// skipped without changing the result (the reference only replaces on a
/// strictly greater score).
fn vote_bridging_angle(samples: &[f64]) -> f32 {
    const RANGE_SLACK: f64 = 1e-9;
    let pi = PI as f64;
    let window = 18.0_f64.to_radians();
    let mut best_score = 0usize;
    let mut best_angle = 0.0;
    for &candidate in samples {
        let ranges = [-1.0_f64, 0.0, 1.0].map(|k| {
            let low = candidate + k * pi - window - RANGE_SLACK;
            let high = candidate + k * pi + window + RANGE_SLACK;
            let start = samples.partition_point(|&value| value < low);
            let end = samples.partition_point(|&value| value <= high);
            start..end.max(start)
        });
        let bound: usize = ranges.iter().map(|range| range.len()).sum();
        if bound <= best_score {
            continue;
        }
        let mut score = 0usize;
        let mut sum = 0.0;
        for range in ranges {
            for &direction in &samples[range] {
                let mut delta = (direction - candidate).rem_euclid(pi);
                if delta > pi / 2.0 {
                    delta -= pi;
                }
                if delta.abs() <= window {
                    score += 1;
                    sum += candidate + delta;
                }
            }
        }
        if score > best_score {
            best_score = score;
            best_angle = (sum / score as f64).rem_euclid(pi);
        }
    }
    (best_angle.to_degrees() as f32)
        .rem_euclid(180.0)
        .max(0.001)
}

/// One non-degenerate anchor segment, in the reference scan's order.
struct AnchorSegment {
    ax: f64,
    ay: f64,
    dx: f64,
    dy: f64,
    len2: f64,
    angle: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

struct AnchorNode {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    /// Leaf: `start..end` into `order`. Inner: child node indices.
    start: usize,
    end: usize,
    leaf: bool,
}

/// Bounding-box hierarchy over anchor segments answering
/// nearest-anchor queries exactly, matching the original linear scan.
///
/// Segment `i` is the `i`-th non-degenerate segment of the reference scan, and
/// its distance is evaluated with the reference's expression, so the computed
/// `distance2` values are identical. The reference keeps the first segment of
/// minimal `distance2`; the query keeps the smallest index among equal
/// distances, which is the same segment. A node is pruned only when its box
/// lower bound exceeds the best distance by a relative and absolute margin
/// that dominates the rounding of either computation, so no segment that
/// could win or tie is ever skipped.
struct AnchorSegmentIndex {
    segments: Vec<AnchorSegment>,
    order: Vec<usize>,
    nodes: Vec<AnchorNode>,
}

impl AnchorSegmentIndex {
    const LEAF_SIZE: usize = 8;

    fn new(anchors: &[Vec<Point2>]) -> Self {
        let mut segments = Vec::new();
        for line in anchors {
            for pair in line.windows(2) {
                let ax = pair[0].x as f64;
                let ay = pair[0].y as f64;
                let dx = (pair[1].x - pair[0].x) as f64;
                let dy = (pair[1].y - pair[0].y) as f64;
                let len2 = dx * dx + dy * dy;
                if len2 == 0.0 {
                    continue;
                }
                let bx = pair[1].x as f64;
                let by = pair[1].y as f64;
                segments.push(AnchorSegment {
                    ax,
                    ay,
                    dx,
                    dy,
                    len2,
                    angle: (dy.atan2(dx) + PI as f64 / 2.0).rem_euclid(PI as f64),
                    min_x: ax.min(bx),
                    min_y: ay.min(by),
                    max_x: ax.max(bx),
                    max_y: ay.max(by),
                });
            }
        }
        let mut order: Vec<usize> = (0..segments.len()).collect();
        let mut nodes = Vec::with_capacity(segments.len() / 4 + 1);
        if !segments.is_empty() {
            Self::build(&segments, &mut order, &mut nodes, 0, segments.len());
        }
        Self {
            segments,
            order,
            nodes,
        }
    }

    fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    fn build(
        segments: &[AnchorSegment],
        order: &mut [usize],
        nodes: &mut Vec<AnchorNode>,
        start: usize,
        end: usize,
    ) -> usize {
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &index in &order[start..end] {
            let segment = &segments[index];
            min_x = min_x.min(segment.min_x);
            min_y = min_y.min(segment.min_y);
            max_x = max_x.max(segment.max_x);
            max_y = max_y.max(segment.max_y);
        }
        let me = nodes.len();
        nodes.push(AnchorNode {
            min_x,
            min_y,
            max_x,
            max_y,
            start,
            end,
            leaf: end - start <= Self::LEAF_SIZE,
        });
        if end - start <= Self::LEAF_SIZE {
            return me;
        }
        let split_x = (max_x - min_x) >= (max_y - min_y);
        let mid = start + (end - start) / 2;
        let centre = |index: usize| {
            let segment = &segments[index];
            if split_x {
                segment.min_x + segment.max_x
            } else {
                segment.min_y + segment.max_y
            }
        };
        order[start..end].select_nth_unstable_by(mid - start, |&a, &b| {
            centre(a).total_cmp(&centre(b)).then(a.cmp(&b))
        });
        let left = Self::build(segments, order, nodes, start, mid);
        let right = Self::build(segments, order, nodes, mid, end);
        nodes[me].start = left;
        nodes[me].end = right;
        me
    }

    fn box_distance2(node: &AnchorNode, p: (f64, f64)) -> f64 {
        let dx = if p.0 < node.min_x {
            node.min_x - p.0
        } else if p.0 > node.max_x {
            p.0 - node.max_x
        } else {
            0.0
        };
        let dy = if p.1 < node.min_y {
            node.min_y - p.1
        } else if p.1 > node.max_y {
            p.1 - node.max_y
        } else {
            0.0
        };
        dx * dx + dy * dy
    }

    /// `true` when nothing in a box at lower bound `bound` can match `best`.
    fn prunable(bound: f64, best: f64) -> bool {
        bound > best * (1.0 + 1e-9) + 1.0
    }

    fn nearest_angle(&self, p: (f64, f64), stack: &mut Vec<usize>) -> Option<f64> {
        if self.segments.is_empty() {
            return None;
        }
        let mut best_d2 = f64::INFINITY;
        let mut best_index = usize::MAX;
        stack.clear();
        stack.push(0);
        while let Some(node_index) = stack.pop() {
            let node = &self.nodes[node_index];
            if Self::prunable(Self::box_distance2(node, p), best_d2) {
                continue;
            }
            if node.leaf {
                for &index in &self.order[node.start..node.end] {
                    let s = &self.segments[index];
                    // The reference's exact expression.
                    let t = (((p.0 - s.ax) * s.dx + (p.1 - s.ay) * s.dy) / s.len2).clamp(0.0, 1.0);
                    let distance2 =
                        (p.0 - s.ax - t * s.dx).powi(2) + (p.1 - s.ay - t * s.dy).powi(2);
                    if distance2 < best_d2 || (distance2 == best_d2 && index < best_index) {
                        best_d2 = distance2;
                        best_index = index;
                    }
                }
            } else {
                let (left, right) = (node.start, node.end);
                let left_bound = Self::box_distance2(&self.nodes[left], p);
                let right_bound = Self::box_distance2(&self.nodes[right], p);
                // Visit the nearer child first (pushed last).
                if left_bound <= right_bound {
                    stack.push(right);
                    stack.push(left);
                } else {
                    stack.push(left);
                    stack.push(right);
                }
            }
        }
        // `best_d2` stays infinite only if every distance was NaN, which the
        // reference would also never select.
        (best_index != usize::MAX).then(|| self.segments[best_index].angle)
    }
}

/// Presort internal-bridge candidates the way canonical
/// `PrintObject::bridge_over_infill` does before choosing directions: by
/// bounding-box minimum x, then minimum y; with more than two candidates the
/// tail is then stable-sorted by squared distance from the first candidate's
/// bounding-box maximum to each candidate's bounding-box minimum. Neighbouring
/// candidates therefore meet in a stable order, which is what lets the
/// collision rule in [`internal_bridge_angles`] hand them one shared angle.
pub fn presort_bridge_candidates(candidates: &mut [ExPolygon]) {
    fn extents(polygon: &ExPolygon) -> (i64, i64, i64, i64) {
        polygon.contour.points.iter().fold(
            (i64::MAX, i64::MAX, i64::MIN, i64::MIN),
            |(min_x, min_y, max_x, max_y), p| {
                (
                    min_x.min(p.x),
                    min_y.min(p.y),
                    max_x.max(p.x),
                    max_y.max(p.y),
                )
            },
        )
    }
    candidates.sort_by(|left, right| {
        let (a, b) = (extents(left), extents(right));
        a.0.cmp(&b.0).then(a.1.cmp(&b.1))
    });
    if candidates.len() > 2 {
        let (_, _, origin_x, origin_y) = extents(&candidates[0]);
        let distance2 = |polygon: &ExPolygon| {
            let (min_x, min_y, _, _) = extents(polygon);
            let dx = (origin_x - min_x) as f64;
            let dy = (origin_y - min_y) as f64;
            dx * dx + dy * dy
        };
        candidates[1..].sort_by(|left, right| distance2(left).total_cmp(&distance2(right)));
    }
}

/// Closed boundary polylines used as bridge anchors when no real lower-layer
/// sparse lines are available: the outlines of `total_fill_area` grown by
/// `1.3 × spacing` and of `limiting_area` grown by `0.3 × spacing`.
///
/// Canonical `PrintObject::bridge_over_infill` builds exactly these
/// `boundary_plines` and hands them to `determine_bridging_angle` whenever the
/// sparse-line anchor set is empty. Every ring is closed (first point
/// repeated), matching canonical `to_polylines(Polygons)`.
pub fn boundary_anchor_polylines(
    total_fill_area: &[ExPolygon],
    limiting_area: &[ExPolygon],
    spacing_mm: f32,
) -> Vec<Vec<Point2>> {
    let mut rings = Vec::new();
    for (area, factor) in [(total_fill_area, 1.3_f32), (limiting_area, 0.3_f32)] {
        for polygon in offset(area, factor * spacing_mm, OffsetJoinType::Miter, 0.0) {
            for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
                if ring.points.len() < 2 {
                    continue;
                }
                let mut points = ring.points.clone();
                points.push(ring.points[0]);
                rings.push(points);
            }
        }
    }
    rings
}

/// Layer-level inputs for [`internal_bridge_angles`].
#[derive(Clone, Copy, Debug)]
pub struct InternalBridgeAngleInputs<'a> {
    /// Deep sparse infill below, already grown by the clip margin
    /// (`deep_infill_area` after canonical's `expand(.., 1.5 × spacing)`).
    pub deep_infill_clip_area: &'a [ExPolygon],
    /// `deep_infill_area` shrunk by `4.5 × spacing`; parts of a candidate's
    /// bridge area that do not reach it are discarded.
    pub internal_unsupported_area: &'a [ExPolygon],
    /// Current-layer sparse ∪ internal-solid fill (canonical
    /// `stInternal ∪ stInternalSolid`), before closing and clipping.
    pub expansion_area: &'a [ExPolygon],
    /// Current-layer fill area (canonical `fill_expolygons`).
    pub total_fill_area: &'a [ExPolygon],
    /// Bridging-flow spacing in millimetres.
    pub spacing_mm: f32,
    /// Positive values replace the automatic angle (canonical absolute
    /// `internal_bridge_angle`).
    pub override_deg: f32,
}

/// Choose one bridge line direction (degrees, `[0, 180)`) per internal-bridge
/// candidate, index-aligned with `candidates`, which callers presort with
/// [`presort_bridge_candidates`].
///
/// Port of the per-candidate direction step in canonical
/// `PrintObject::bridge_over_infill`: `area_to_be_bridge = expand(candidate,
/// spacing) ∩ deep_infill_area`, keeping only the parts that reach
/// `internal_unsupported_area`; `limiting_area = area_to_be_bridge ∪
/// expansion_area`; the angle comes from `determine_bridging_angle` over the
/// boundary anchors ([`boundary_anchor_polylines`]); a positive override
/// replaces it; and a candidate whose area-class region grown by
/// `3 × spacing` meets an earlier candidate's region reuses that candidate's
/// angle. Each chosen candidate's area is removed from the expansion area seen
/// by later ones, as canonical removes each bridged area from
/// `expansion_area`.
///
/// # Collision operands
///
/// Canonical grows the **constructed** `bridging_area`
/// (`construct_anchored_polygon(area_to_be_bridge, …)`, after the
/// `expand(.., spacing)` clip) and intersects each earlier candidate's stored
/// `new_polys`. This crate does not construct the anchor-grown area (the
/// qualified polygon is the final geometry; canonical growth beyond the
/// candidate is DEV-150), so both operands here use `area_to_be_bridge`
/// itself. That `area` is exactly the object canonical constructs from, and
/// using it on both sides preserves canonical's detection radius — two
/// candidates whose qualified polygons are within roughly `5 × spacing`
/// share one direction. Using the unexpanded raw candidate on either side
/// (the previous behaviour) shrinks that radius to `3 × spacing`.
///
/// Canonical anchors on the real lower-layer sparse infill lines and falls back
/// to the boundary polylines only when there are none. The sparse lines are
/// generated by a fill module after this host pass runs, so this port always
/// takes the boundary fallback (recorded in `docs/DEVIATION_LOG.md`).
pub fn internal_bridge_angles(
    candidates: &[ExPolygon],
    inputs: &InternalBridgeAngleInputs<'_>,
) -> Vec<f32> {
    let spacing = inputs.spacing_mm;
    let total_fill_area = closing_ex(
        inputs.total_fill_area,
        SCALED_EPSILON_MM,
        OffsetJoinType::Miter,
    );
    let mut expansion_area = intersection(
        &closing_ex(
            inputs.expansion_area,
            SCALED_EPSILON_MM,
            OffsetJoinType::Miter,
        ),
        inputs.deep_infill_clip_area,
    );
    let mut angles: Vec<f32> = Vec::with_capacity(candidates.len());
    let mut bridge_areas: Vec<Vec<ExPolygon>> = Vec::with_capacity(candidates.len());
    // `boundary_anchor_polylines(&total_fill_area, &limiting_area, ..)` emits
    // the `total_fill_area` rings first and the `limiting_area` rings second.
    // Only the second half depends on the candidate, so the first half is
    // computed once here and each candidate's anchor set is that prefix
    // followed by its own rings — the identical sequence.
    let total_fill_rings = boundary_anchor_polylines(&total_fill_area, &[], spacing);
    for (index, candidate) in candidates.iter().enumerate() {
        let candidate = std::slice::from_ref(candidate);
        let mut area: Vec<ExPolygon> = intersection(
            &offset(candidate, spacing, OffsetJoinType::Miter, 0.0),
            inputs.deep_infill_clip_area,
        )
        .into_iter()
        .filter(|part| {
            !intersection(std::slice::from_ref(part), inputs.internal_unsupported_area).is_empty()
        })
        .collect();
        if area.is_empty() {
            // The caller only passes candidates that reach the unsupported
            // area, so this is a degenerate clip; sample the candidate itself.
            area = candidate.to_vec();
        }
        let limiting_area = union(&area, &expansion_area);
        let mut anchors = total_fill_rings.clone();
        anchors.extend(boundary_anchor_polylines(&[], &limiting_area, spacing));
        let edges: Vec<Vec<Point2>> = area
            .iter()
            .flat_map(|polygon| std::iter::once(&polygon.contour).chain(polygon.holes.iter()))
            .map(|ring| ring.points.clone())
            .collect();
        let mut angle = determine_bridging_angle(&anchors, &edges, inputs.override_deg);
        // Canonical collision check (`expand(bridging_area, 3 × spacing)` vs
        // each earlier `new_polys`): both operands are the area-class bridging
        // region, not the raw candidate. See the note above. The first
        // candidate has no earlier one to meet, so its grown area is unread.
        if index > 0 {
            let grown = offset(&area, 3.0 * spacing, OffsetJoinType::Miter, 0.0);
            if let Some(earlier) = bridge_areas[..index]
                .iter()
                .position(|earlier| !intersection(earlier, &grown).is_empty())
            {
                angle = angles[earlier];
            }
        }
        angles.push(angle);
        // The last candidate's updated expansion area and stored area are
        // never read, so the final `difference` is skipped.
        if index + 1 < candidates.len() {
            expansion_area = difference(&expansion_area, &area);
            bridge_areas.push(area);
        }
    }
    angles
}

/// Construct scan strips whose endpoints are supported by anchor geometry.
pub fn construct_anchored_polygon(
    anchors: &[Vec<Point2>],
    voids: &[ExPolygon],
    angle_deg: f32,
    spacing_mm: f32,
    thread_width_mm: f32,
) -> (Vec<ExPolygon>, Vec<Vec<Point2>>) {
    if voids.is_empty() || spacing_mm <= 0.0 {
        return (Vec::new(), Vec::new());
    }
    let theta = (-angle_deg as f64 + 90.0_f64).to_radians();
    let (sin, cos) = theta.sin_cos();
    let spacing = (spacing_mm as f64 * 10_000.0).round();
    // Sparse infill can leave a small gap before the void wall; the canonical
    // construction extends a section across that gap to the flanking anchor.
    let tolerance = (thread_width_mm as f64 * 10_000.0 * 5.0).max(1.0);
    let mut result = Vec::new();
    let mut lines = Vec::new();
    for void in voids {
        let rotated: Vec<(f64, f64)> = void
            .contour
            .points
            .iter()
            .map(|p| rotate(p, sin, cos))
            .collect();
        if rotated.is_empty() {
            continue;
        }
        let min_x = rotated.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = rotated
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = rotated.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_y = rotated
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let rotated_anchors: Vec<Vec<(f64, f64)>> = anchors
            .iter()
            .map(|a| a.iter().map(|p| rotate(p, sin, cos)).collect())
            .collect();
        let mut x = min_x + spacing / 2.0;
        while x < max_x {
            let mut sections = vertical_sections(&rotated, x);
            let supported = !anchors.is_empty();
            for section in &mut sections {
                let mut low = section.0;
                let mut high = section.1;
                let mut low_anchor = None;
                let mut high_anchor = None;
                for line in &rotated_anchors {
                    for pair in line.windows(2) {
                        if let Some(y) = crossing_y(pair, x) {
                            if (y - low).abs() <= tolerance {
                                low_anchor = Some(y);
                            }
                            if (y - high).abs() <= tolerance {
                                high_anchor = Some(y);
                            }
                        }
                    }
                }
                if let Some(y) = low_anchor {
                    low = y;
                }
                if let Some(y) = high_anchor {
                    high = y;
                }
                *section = (low.max(min_y), high.min(max_y));
            }
            if supported {
                sections.retain(|(lo, hi)| {
                    (hi - lo) >= 1.0
                        && has_anchor(&rotated_anchors, x, *lo, tolerance)
                        && has_anchor(&rotated_anchors, x, *hi, tolerance)
                });
            }
            for (lo, hi) in sections {
                if hi <= lo {
                    continue;
                }
                let a = unrotate(x, lo, sin, cos);
                let b = unrotate(x, hi, sin, cos);
                lines.push(vec![a, b]);
                result.extend(intersection(
                    &[strip(a, b, (tolerance / 2.0).round() as i64)],
                    std::slice::from_ref(void),
                ));
            }
            x += spacing;
        }
    }
    (result, lines)
}

fn rotate(p: &Point2, sin: f64, cos: f64) -> (f64, f64) {
    (
        p.x as f64 * cos + p.y as f64 * sin,
        -p.x as f64 * sin + p.y as f64 * cos,
    )
}
fn unrotate(x: f64, y: f64, sin: f64, cos: f64) -> Point2 {
    Point2 {
        x: (x * cos - y * sin).round() as i64,
        y: (x * sin + y * cos).round() as i64,
    }
}
fn crossing_y(pair: &[(f64, f64)], x: f64) -> Option<f64> {
    let (a, b) = (pair[0], pair[1]);
    if (a.0 <= x && x < b.0) || (b.0 <= x && x < a.0) {
        Some(a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0))
    } else {
        None
    }
}
fn vertical_sections(poly: &[(f64, f64)], x: f64) -> Vec<(f64, f64)> {
    let mut ys = Vec::new();
    for pair in poly
        .windows(2)
        .chain(std::iter::once(&[poly[poly.len() - 1], poly[0]][..]))
    {
        if let Some(y) = crossing_y(pair, x) {
            ys.push(y);
        }
    }
    ys.sort_by(|a, b| a.total_cmp(b));
    ys.as_chunks::<2>().0.iter().map(|v| (v[0], v[1])).collect()
}
fn has_anchor(anchors: &[Vec<(f64, f64)>], x: f64, y: f64, tolerance: f64) -> bool {
    anchors.iter().flat_map(|a| a.windows(2)).any(|p| {
        let (a, b) = (p[0], p[1]);
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let t =
            (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy).max(1e-12)).clamp(0.0, 1.0);
        (x - a.0 - t * dx).hypot(y - a.1 - t * dy) <= tolerance
    })
}
fn strip(a: Point2, b: Point2, half_width: i64) -> ExPolygon {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    let len = dx.hypot(dy).max(1.0);
    let nx = (-dy / len * half_width as f64).round() as i64;
    let ny = (dx / len * half_width as f64).round() as i64;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: a.x + nx,
                    y: a.y + ny,
                },
                Point2 {
                    x: b.x + nx,
                    y: b.y + ny,
                },
                Point2 {
                    x: b.x - nx,
                    y: b.y - ny,
                },
                Point2 {
                    x: a.x - nx,
                    y: a.y - ny,
                },
            ],
        },
        holes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(min: f32, max: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(min, 0.0),
                    Point2::from_mm(max, 0.0),
                    Point2::from_mm(max, 5.0),
                    Point2::from_mm(min, 5.0),
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn gather_depth_subtracts_solids_and_stops_at_cutoff() {
        let sparse = square(0.0, 5.0);
        let layers = vec![
            BridgeDepthLayer {
                print_z: 0.0,
                sparse_infill: vec![sparse.clone()],
                ..Default::default()
            },
            BridgeDepthLayer {
                print_z: 0.2,
                sparse_infill: vec![sparse.clone()],
                not_sparse_infill: vec![square(0.0, 2.0)],
            },
            BridgeDepthLayer {
                print_z: 0.4,
                ..Default::default()
            },
        ];
        let result = gather_areas_w_depth(&layers, 2, 0.2, 1.0);
        assert!(result
            .iter()
            .flat_map(|p| p.contour.points.iter())
            .all(|p| p.x >= Point2::from_mm(2.0, 0.0).x));
    }

    #[test]
    fn clustering_requires_z_gap_and_intersection() {
        let layers = vec![
            BridgeCandidateLayer {
                print_z: 0.0,
                new_polys: vec![square(0.0, 2.0)],
            },
            BridgeCandidateLayer {
                print_z: 0.1,
                new_polys: vec![square(1.0, 3.0)],
            },
            BridgeCandidateLayer {
                print_z: 0.3,
                new_polys: vec![square(1.0, 3.0)],
            },
        ];
        assert_eq!(
            cluster_candidate_layers(&layers, 0.2, 1.0),
            vec![vec![0, 1], vec![2]]
        );
    }

    #[test]
    fn filled_lower_candidates_are_removed() {
        let result = remove_filled_polygons_on_lower_layers(
            &[square(0.0, 5.0)],
            &[BridgeCandidateLayer {
                print_z: 0.2,
                new_polys: vec![square(0.0, 2.0)],
            }],
            0.4,
            0.2,
        );
        assert!(result
            .iter()
            .flat_map(|p| p.contour.points.iter())
            .all(|p| p.x >= Point2::from_mm(2.0, 0.0).x));
    }
}
