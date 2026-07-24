// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: FillBase.cpp::chain_or_connect_infill
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

use slicer_core::polygon_ops::union_ex;
use slicer_ir::{
    ConfigValue, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillRegion, Point2, Point3WithWidth,
    Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::views::PerimeterRegionView;

use crate::connect::chain_or_connect_infill;
use crate::offset::{clip_to_offset_boundary, remove_short_polylines, ExPolygonWithOffset};

const WIDTH_EPSILON_MM: f32 = 0.000001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathBucket {
    Sparse,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegionConfig {
    line_width: u32,
    density: u32,
}

/// The host-partitioned fill polygons of one region, kept per role.
///
/// ADR-0025 §2 requires the linker to re-clip against "the partitioned fill
/// polygons" — plural, one per role. Substituting their union (what
/// `PerimeterRegionView::infill_areas` returns) is a containment hole: a sparse
/// path re-clipped against the union is free to run across the top-solid or
/// bridge polygon beside it. Canonical enforces the same separation three times
/// over in `libslic3r/Fill/` — `group_fills` buckets surfaces by
/// `SurfaceFillParams` keyed on `extrusion_role`, the mutual-clipping pass
/// `diff_ex(polys, all_polygons, ApplySafetyOffset::Yes)` subtracts every other
/// bucket, and `FillGyroid::_fill_surface_single` finishes with
/// `intersection_pl(polylines, expolygon)`.
#[derive(Debug, Clone, Default)]
struct RoleBoundaries {
    sparse: Vec<ExPolygon>,
    top: Vec<ExPolygon>,
    bottom: Vec<ExPolygon>,
    bridge: Vec<ExPolygon>,
    /// `infill_areas` — the union. Only used for views the host never
    /// partitioned, and for roles that have no dedicated partition.
    union: Vec<ExPolygon>,
}

impl RoleBoundaries {
    fn from_view(view: &PerimeterRegionView) -> Self {
        Self {
            sparse: view.sparse_infill_area().to_vec(),
            top: view.top_solid_fill().to_vec(),
            bottom: view.bottom_solid_fill().to_vec(),
            bridge: view.bridge_areas().to_vec(),
            union: view.infill_areas().to_vec(),
        }
    }

    fn is_partitioned(&self) -> bool {
        !(self.sparse.is_empty()
            && self.top.is_empty()
            && self.bottom.is_empty()
            && self.bridge.is_empty())
    }

    /// The boundary `role` must be confined to.
    ///
    /// `None` means no boundary is known — the caller passes those paths
    /// through untouched, which is what the linker has always done for a region
    /// whose boundary it cannot resolve. `Some(empty)` is a *different* answer:
    /// the host partitioned this region and gave `role` no area at all, so the
    /// role's paths have nowhere legal to go and clip away to nothing.
    fn for_role(&self, role: &ExtrusionRole) -> Option<Vec<ExPolygon>> {
        let partitioned = match role {
            ExtrusionRole::SparseInfill => Some(self.sparse.clone()),
            ExtrusionRole::TopSolidInfill => Some(self.top.clone()),
            ExtrusionRole::BottomSolidInfill => Some(self.bottom.clone()),
            ExtrusionRole::BridgeInfill => Some(self.bridge.clone()),
            // `solid_role` in rectilinear-infill / gyroid-infill relabels a
            // top or bottom shell at depth ≥ 1 as InternalSolidInfill, so its
            // legal area is the union of the two solid-shell polygons.
            ExtrusionRole::InternalSolidInfill => Some(union_ex(
                &self
                    .top
                    .iter()
                    .chain(self.bottom.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            )),
            // Roles the host does not partition (raft, custom, …) keep the
            // historical union boundary.
            _ => None,
        };
        match partitioned {
            Some(polygons) if self.is_partitioned() => Some(polygons),
            _ => (!self.union.is_empty()).then(|| self.union.clone()),
        }
    }
}

#[derive(Debug, Clone)]
struct RegionRecord {
    prior_index: usize,
    object_id: String,
    region_id: u64,
    tool_index: u32,
    wall_source_region_id: Option<u64>,
    boundaries: RoleBoundaries,
    config: RegionConfig,
    sparse_spacing_mm: f32,
    solid_spacing_mm: f32,
    sparse_paths: Vec<ExtrusionPath3D>,
    solid_paths: Vec<ExtrusionPath3D>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WallGroupKey {
    Shared(u64),
    Owned(usize),
}

#[derive(Debug, Clone, Copy)]
struct SourceSegment {
    owner: usize,
    start: Point2,
    end: Point2,
    length: f64,
}

#[derive(Debug, Clone, Copy)]
struct BoundarySegment {
    start: Point2,
    end: Point2,
}

#[derive(Debug, Clone, Default)]
struct BucketPaths {
    sparse: Vec<ExtrusionPath3D>,
    solid: Vec<ExtrusionPath3D>,
}

/// Runs non-ironing infill orchestration and emits paths into region buckets.
pub fn orchestrate_infill(
    prior_infill: &[InfillRegion],
    regions: &[PerimeterRegionView],
    infill_overlap: f32,
    default_line_width: f32,
    output: &mut InfillOutputBuilder,
) -> Result<(), String> {
    let mut records = Vec::with_capacity(prior_infill.len());
    let mut buckets = vec![BucketPaths::default(); prior_infill.len()];

    for (prior_index, region) in prior_infill.iter().enumerate() {
        let Some(view) = regions.iter().find(|view| {
            view.object_id() == &region.object_id && view.region_id() == &region.region_id
        }) else {
            buckets[prior_index].sparse = region.sparse_infill.clone();
            buckets[prior_index].solid = region.solid_infill.clone();
            continue;
        };

        let line_width = config_float(view.config(), "line_width")
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(default_line_width);
        let density = config_float(view.config(), "infill_density")
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(0.2);

        records.push(RegionRecord {
            prior_index,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            tool_index: view.tool_index(),
            wall_source_region_id: view.wall_source_region_id().copied(),
            boundaries: RoleBoundaries::from_view(view),
            config: RegionConfig {
                line_width: line_width.to_bits(),
                density: density.to_bits(),
            },
            sparse_spacing_mm: (line_width / density).max(f32::EPSILON),
            solid_spacing_mm: line_width.max(f32::EPSILON),
            sparse_paths: region.sparse_infill.clone(),
            solid_paths: region.solid_infill.clone(),
        });
    }

    process_bucket(&records, PathBucket::Sparse, infill_overlap, &mut buckets);
    process_bucket(&records, PathBucket::Solid, infill_overlap, &mut buckets);

    for (index, region) in prior_infill.iter().enumerate() {
        output.begin_region(&region.object_id, region.region_id);
        for path in buckets[index].sparse.drain(..) {
            output.push_sparse_path(path)?;
        }
        for path in buckets[index].solid.drain(..) {
            output.push_solid_path(path)?;
        }
    }

    Ok(())
}

fn process_bucket(
    records: &[RegionRecord],
    bucket: PathBucket,
    infill_overlap: f32,
    buckets: &mut [BucketPaths],
) {
    for role in roles_for_bucket(records, bucket) {
        process_bucket_role(records, bucket, &role, infill_overlap, buckets);
    }
}

fn process_bucket_role(
    records: &[RegionRecord],
    bucket: PathBucket,
    role: &ExtrusionRole,
    infill_overlap: f32,
    buckets: &mut [BucketPaths],
) {
    // Per-role, per-region: `role`'s own partitioned polygon, never the union
    // of all four (see `RoleBoundaries::for_role`).
    let boundaries = records
        .iter()
        .map(|record| record.boundaries.for_role(role))
        .collect::<Vec<_>>();

    let active = (0..records.len())
        .filter(|&index| {
            !selected_paths(&records[index], bucket, role).is_empty() && boundaries[index].is_some()
        })
        .collect::<Vec<_>>();

    for (index, record) in records.iter().enumerate() {
        let selected = selected_paths(record, bucket, role);
        if selected.is_empty() || boundaries[index].is_none() {
            append_paths(
                &mut buckets[record.prior_index],
                bucket,
                selected.into_iter().cloned().collect(),
            );
        }
    }

    let shared_sources = records
        .iter()
        .filter_map(|record| record.wall_source_region_id)
        .collect::<Vec<_>>();
    let groups = wall_groups(records, &active, &shared_sources);

    for group in groups {
        let same_config = group
            .windows(2)
            .all(|pair| records[pair[0]].config == records[pair[1]].config);
        let compatible = group.iter().enumerate().all(|(position, &left)| {
            group[position + 1..].iter().all(|&right| {
                compatible_regions(
                    &records[left],
                    &records[right],
                    bucket,
                    role,
                    wall_group_key(&records[left], &shared_sources)
                        == wall_group_key(&records[right], &shared_sources),
                )
            })
        });

        if group.len() > 1 && same_config && compatible {
            link_union_group(
                records,
                &boundaries,
                &group,
                bucket,
                role,
                infill_overlap,
                buckets,
            );
        } else {
            link_region_group(
                records,
                &boundaries,
                &group,
                bucket,
                role,
                infill_overlap,
                buckets,
            );
        }
    }
}

fn roles_for_bucket(records: &[RegionRecord], bucket: PathBucket) -> Vec<ExtrusionRole> {
    let mut roles = Vec::new();
    for record in records {
        for path in all_paths(record, bucket) {
            if !roles.contains(&path.role) {
                roles.push(path.role.clone());
            }
        }
    }
    roles
}

fn wall_groups(
    records: &[RegionRecord],
    active: &[usize],
    shared_sources: &[u64],
) -> Vec<Vec<usize>> {
    let mut groups: Vec<(WallGroupKey, Vec<usize>)> = Vec::new();
    for &index in active {
        let key = wall_group_key(&records[index], shared_sources);
        if let Some((_, members)) = groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            members.push(index);
        } else {
            groups.push((key, vec![index]));
        }
    }
    groups.into_iter().map(|(_, members)| members).collect()
}

fn wall_group_key(record: &RegionRecord, shared_sources: &[u64]) -> WallGroupKey {
    match record.wall_source_region_id {
        Some(source) => WallGroupKey::Shared(source),
        None if shared_sources.contains(&record.region_id) => {
            WallGroupKey::Shared(record.region_id)
        }
        None => WallGroupKey::Owned(record.prior_index),
    }
}

fn compatible_regions(
    first: &RegionRecord,
    second: &RegionRecord,
    bucket: PathBucket,
    role: &ExtrusionRole,
    same_group: bool,
) -> bool {
    first.object_id == second.object_id
        && first.tool_index == second.tool_index
        && same_group
        && selected_paths(first, bucket, role).iter().all(|left| {
            selected_paths(second, bucket, role)
                .iter()
                .all(|right| paths_compatible(left, right))
        })
}

fn paths_compatible(first: &ExtrusionPath3D, second: &ExtrusionPath3D) -> bool {
    first.role == second.role
        && first.speed_factor.to_bits() == second.speed_factor.to_bits()
        && endpoint_widths_compatible(first, second)
}

fn endpoint_widths_compatible(first: &ExtrusionPath3D, second: &ExtrusionPath3D) -> bool {
    let Some(first_start) = first.points.first() else {
        return false;
    };
    let Some(first_end) = first.points.last() else {
        return false;
    };
    let Some(second_start) = second.points.first() else {
        return false;
    };
    let Some(second_end) = second.points.last() else {
        return false;
    };
    (first_start.width - second_start.width).abs() <= WIDTH_EPSILON_MM
        && (first_end.width - second_end.width).abs() <= WIDTH_EPSILON_MM
}

/// The resolved boundary for one record, or the empty slice.
///
/// Callers only reach this for records that passed the `is_some()` filter in
/// `process_bucket_role`, so the fallback is unreachable in practice.
fn boundary_for(boundaries: &[Option<Vec<ExPolygon>>], index: usize) -> &[ExPolygon] {
    boundaries[index].as_deref().unwrap_or(&[])
}

fn link_union_group(
    records: &[RegionRecord],
    boundaries: &[Option<Vec<ExPolygon>>],
    group: &[usize],
    bucket: PathBucket,
    role: &ExtrusionRole,
    infill_overlap: f32,
    buckets: &mut [BucketPaths],
) {
    // Cross-region joining (ADR-0025 Amendment 2026-07-01) is an intentional
    // PnP improvement over canonical and is preserved: the union is taken
    // across sibling regions of the wall-sharing group, but only over *this
    // role's* polygon in each of them.
    let source = group
        .iter()
        .flat_map(|&index| boundary_for(boundaries, index).iter().cloned())
        .collect::<Vec<_>>();
    let boundary = union_ex(&source);
    let spacing = spacing(records, group[0], bucket);
    let tagged = group
        .iter()
        .flat_map(|&index| {
            selected_paths(&records[index], bucket, role)
                .into_iter()
                .cloned()
                .map(move |path| (index, path))
        })
        .collect::<Vec<_>>();
    let (linked, source_segments) = link_paths(tagged, &boundary, spacing, infill_overlap);
    for path in linked {
        let owner = majority_owner(&path, &source_segments, records, group);
        append_paths(&mut buckets[records[owner].prior_index], bucket, vec![path]);
    }
}

fn link_region_group(
    records: &[RegionRecord],
    boundaries: &[Option<Vec<ExPolygon>>],
    group: &[usize],
    bucket: PathBucket,
    role: &ExtrusionRole,
    infill_overlap: f32,
    buckets: &mut [BucketPaths],
) {
    for &index in group {
        let spacing_mm = spacing(records, index, bucket);
        let boundary = if group.len() == 1 {
            ExPolygonWithOffset::for_infill_overlap(
                boundary_for(boundaries, index),
                infill_overlap,
                spacing_mm,
            )
            .polygons_outer()
            .to_vec()
        } else {
            mixed_boundary(boundaries, group, index, infill_overlap, spacing_mm)
        };
        let tagged = selected_paths(&records[index], bucket, role)
            .into_iter()
            .cloned()
            .map(|path| (index, path))
            .collect::<Vec<_>>();
        let (linked, _) = link_paths_without_offset(tagged, &boundary, spacing_mm);
        append_paths(&mut buckets[records[index].prior_index], bucket, linked);
    }
}

fn mixed_boundary(
    boundaries: &[Option<Vec<ExPolygon>>],
    group: &[usize],
    index: usize,
    infill_overlap: f32,
    spacing_mm: f32,
) -> Vec<ExPolygon> {
    let offset = ExPolygonWithOffset::for_infill_overlap(
        boundary_for(boundaries, index),
        infill_overlap,
        spacing_mm,
    );
    let shared = shared_segments(boundary_for(boundaries, index), group, index, boundaries);
    if shared.is_empty() {
        return offset.polygons_outer().to_vec();
    }
    offset
        .polygons_outer()
        .iter()
        .map(|polygon| replace_shared_arcs(polygon, &shared))
        .collect()
}

fn shared_segments(
    source: &[ExPolygon],
    group: &[usize],
    source_index: usize,
    boundaries: &[Option<Vec<ExPolygon>>],
) -> Vec<BoundarySegment> {
    let peers = group
        .iter()
        .filter(|&&index| index != source_index)
        .flat_map(|&index| boundary_segments(boundary_for(boundaries, index)))
        .collect::<Vec<_>>();
    boundary_segments(source)
        .into_iter()
        .filter(|candidate| peers.iter().any(|peer| segments_share_arc(candidate, peer)))
        .collect()
}

fn replace_shared_arcs(polygon: &ExPolygon, shared: &[BoundarySegment]) -> ExPolygon {
    ExPolygon {
        contour: replace_ring(&polygon.contour, shared),
        holes: polygon
            .holes
            .iter()
            .map(|hole| replace_ring(hole, shared))
            .collect(),
    }
}

fn replace_ring(ring: &Polygon, shared: &[BoundarySegment]) -> Polygon {
    if ring.points.is_empty() {
        return ring.clone();
    }
    let mut points = Vec::new();
    for index in 0..ring.points.len() {
        let offset_start = ring.points[index];
        let offset_end = ring.points[(index + 1) % ring.points.len()];
        let edge = BoundarySegment {
            start: offset_start,
            end: offset_end,
        };
        let (start, end) = shared
            .iter()
            .find_map(|candidate| matching_shared_edge(&edge, candidate))
            .map_or((offset_start, offset_end), |candidate| {
                (candidate.start, candidate.end)
            });
        if points.last().copied() != Some(start) {
            points.push(start);
        }
        if points.last().copied() != Some(end) {
            points.push(end);
        }
    }
    Polygon { points }
}

fn matching_shared_edge(
    offset: &BoundarySegment,
    shared: &BoundarySegment,
) -> Option<BoundarySegment> {
    let offset_dx = offset.end.x - offset.start.x;
    let offset_dy = offset.end.y - offset.start.y;
    let shared_dx = shared.end.x - shared.start.x;
    let shared_dy = shared.end.y - shared.start.y;
    let offset_len = ((offset_dx as f64).hypot(offset_dy as f64)).max(1.0);
    let shared_len = ((shared_dx as f64).hypot(shared_dy as f64)).max(1.0);
    let parallel = ((offset_dx as f64 * shared_dy as f64) - (offset_dy as f64 * shared_dx as f64))
        .abs()
        <= offset_len * shared_len * 0.000001;
    if !parallel {
        return None;
    }
    let midpoint = Point2 {
        x: (offset.start.x + offset.end.x) / 2,
        y: (offset.start.y + offset.end.y) / 2,
    };
    let distance = point_segment_distance_squared(midpoint, shared);
    let tolerance = (shared_len * 0.25).max(1.0);
    if distance > tolerance * tolerance {
        return None;
    }
    let same_direction =
        (offset_dx as i128) * (shared_dx as i128) + (offset_dy as i128) * (shared_dy as i128) >= 0;
    Some(if same_direction {
        *shared
    } else {
        BoundarySegment {
            start: shared.end,
            end: shared.start,
        }
    })
}

fn boundary_segments(boundary: &[ExPolygon]) -> Vec<BoundarySegment> {
    boundary
        .iter()
        .flat_map(|expolygon| std::iter::once(&expolygon.contour).chain(expolygon.holes.iter()))
        .flat_map(|ring| {
            (0..ring.points.len()).map(move |index| BoundarySegment {
                start: ring.points[index],
                end: ring.points[(index + 1) % ring.points.len()],
            })
        })
        .filter(|segment| segment.start != segment.end)
        .collect()
}

fn segments_share_arc(first: &BoundarySegment, second: &BoundarySegment) -> bool {
    let first_dx = first.end.x - first.start.x;
    let first_dy = first.end.y - first.start.y;
    let second_dx = second.end.x - second.start.x;
    let second_dy = second.end.y - second.start.y;
    let first_len = ((first_dx as f64).hypot(first_dy as f64)).max(1.0);
    let second_len = ((second_dx as f64).hypot(second_dy as f64)).max(1.0);
    let cross = ((first_dx as f64 * second_dy as f64) - (first_dy as f64 * second_dx as f64)).abs();
    if cross > first_len * second_len * 0.000001 {
        return false;
    }
    point_segment_distance_squared(first.start, second) < 1.0
        || point_segment_distance_squared(first.end, second) < 1.0
        || point_segment_distance_squared(second.start, first) < 1.0
        || point_segment_distance_squared(second.end, first) < 1.0
}

fn point_segment_distance_squared(point: Point2, segment: &BoundarySegment) -> f64 {
    let dx = (segment.end.x - segment.start.x) as f64;
    let dy = (segment.end.y - segment.start.y) as f64;
    let length_squared = dx * dx + dy * dy;
    let parameter = if length_squared == 0.0 {
        0.0
    } else {
        (((point.x - segment.start.x) as f64 * dx + (point.y - segment.start.y) as f64 * dy)
            / length_squared)
            .clamp(0.0, 1.0)
    };
    let projected_x = segment.start.x as f64 + parameter * dx;
    let projected_y = segment.start.y as f64 + parameter * dy;
    (point.x as f64 - projected_x).powi(2) + (point.y as f64 - projected_y).powi(2)
}

fn link_paths(
    tagged: Vec<(usize, ExtrusionPath3D)>,
    boundary: &[ExPolygon],
    spacing_mm: f32,
    infill_overlap: f32,
) -> (Vec<ExtrusionPath3D>, Vec<SourceSegment>) {
    let offset = ExPolygonWithOffset::for_infill_overlap(boundary, infill_overlap, spacing_mm);
    link_paths_without_offset(tagged, offset.polygons_outer(), spacing_mm)
}

fn link_paths_without_offset(
    tagged: Vec<(usize, ExtrusionPath3D)>,
    boundary: &[ExPolygon],
    spacing_mm: f32,
) -> (Vec<ExtrusionPath3D>, Vec<SourceSegment>) {
    let mut clipped = Vec::new();
    let mut source_segments = Vec::new();
    for (owner, path) in tagged {
        let polyline = path
            .points
            .iter()
            .map(|point| Point2::from_mm(point.x, point.y))
            .collect::<Vec<_>>();
        let clipped_polylines = clip_to_offset_boundary(&[polyline], boundary);
        for points in remove_short_polylines(&clipped_polylines, spacing_mm) {
            source_segments.extend(points.windows(2).filter_map(|segment| {
                let length = ((segment[1].x - segment[0].x) as f64)
                    .hypot((segment[1].y - segment[0].y) as f64);
                (length > 0.0).then_some(SourceSegment {
                    owner,
                    start: segment[0],
                    end: segment[1],
                    length,
                })
            }));
            let mut clipped_path = path.clone();
            clipped_path.points = points
                .into_iter()
                .map(|point| point_with_metadata(&path, point))
                .collect();
            clipped.push(clipped_path);
        }
    }
    (
        chain_or_connect_infill(
            clipped,
            &crate::graph::BoundaryInfillGraph::new(boundary),
            spacing_mm,
        ),
        source_segments,
    )
}

fn majority_owner(
    path: &ExtrusionPath3D,
    source_segments: &[SourceSegment],
    records: &[RegionRecord],
    group: &[usize],
) -> usize {
    let mut lengths = group
        .iter()
        .map(|&index| (index, 0.0_f64))
        .collect::<Vec<_>>();
    for edge in path.points.windows(2) {
        let start = Point2::from_mm(edge[0].x, edge[0].y);
        let end = Point2::from_mm(edge[1].x, edge[1].y);
        let length = ((end.x - start.x) as f64).hypot((end.y - start.y) as f64);
        if length == 0.0 {
            continue;
        }
        if let Some(source) = source_segments.iter().find(|source| {
            (source.start == start && source.end == end)
                || (source.start == end && source.end == start)
        }) {
            if let Some((_, total)) = lengths.iter_mut().find(|(index, _)| *index == source.owner) {
                *total += source.length.min(length);
            }
        }
    }
    lengths.sort_by(|(left_index, left_length), (right_index, right_length)| {
        right_length
            .total_cmp(left_length)
            .then_with(|| {
                records[*left_index]
                    .region_id
                    .cmp(&records[*right_index].region_id)
            })
            .then_with(|| left_index.cmp(right_index))
    });
    lengths[0].0
}

fn point_with_metadata(path: &ExtrusionPath3D, point: Point2) -> Point3WithWidth {
    let (x, y) = point.to_mm();
    let source = path
        .points
        .iter()
        .min_by_key(|candidate| {
            let candidate_point = Point2::from_mm(candidate.x, candidate.y);
            let dx = candidate_point.x - point.x;
            let dy = candidate_point.y - point.y;
            dx.saturating_mul(dx) + dy.saturating_mul(dy)
        })
        .cloned()
        .unwrap_or_default();
    Point3WithWidth { x, y, ..source }
}

fn append_paths(bucket: &mut BucketPaths, kind: PathBucket, paths_to_append: Vec<ExtrusionPath3D>) {
    match kind {
        PathBucket::Sparse => bucket.sparse.extend(paths_to_append),
        PathBucket::Solid => bucket.solid.extend(paths_to_append),
    }
}

fn all_paths(record: &RegionRecord, bucket: PathBucket) -> &[ExtrusionPath3D] {
    match bucket {
        PathBucket::Sparse => &record.sparse_paths,
        PathBucket::Solid => &record.solid_paths,
    }
}

fn selected_paths<'a>(
    record: &'a RegionRecord,
    bucket: PathBucket,
    role: &ExtrusionRole,
) -> Vec<&'a ExtrusionPath3D> {
    all_paths(record, bucket)
        .iter()
        .filter(|path| &path.role == role)
        .collect()
}

fn spacing(records: &[RegionRecord], index: usize, bucket: PathBucket) -> f32 {
    match bucket {
        PathBucket::Sparse => records[index].sparse_spacing_mm,
        PathBucket::Solid => records[index].solid_spacing_mm,
    }
}

fn config_float(config: Option<&slicer_ir::ConfigView>, key: &str) -> Option<f32> {
    config.and_then(|config| match config.get(key) {
        Some(ConfigValue::Float(value)) => Some(*value as f32),
        Some(ConfigValue::Int(value)) => Some(*value as f32),
        _ => None,
    })
}
