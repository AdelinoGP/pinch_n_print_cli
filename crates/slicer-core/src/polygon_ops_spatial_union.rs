//! Host-side spatial hierarchical polygon union with exact convex-containment pruning.
//!
//! Designed for large overlapping polygon union batches (notably tree-support
//! planner Phase-3 `ROLE_BODY` unions where hundreds of overlapping 100-gon
//! branch discs collapse ~11× in vertex count). Wired strictly into batched
//! `ClipOperation::Union` dispatch (`slicer-wasm-host` and `slicer-sdk` native
//! batch fallback), leaving serial `clip_polygons` callers and all non-union
//! boolean operations untouched.
//!
//! # Algorithm
//!
//! 1. **Canonical winding guard:** checks that every input contour is CCW
//!    (`double_area > 0`) and every input hole is CW (`double_area < 0`). If any
//!    polygon violates canonical winding (e.g. opposite-wound rings relying on
//!    `FillRule::NonZero` hole punching), delegates immediately to monolithic
//!    [`crate::polygon_ops::clip_polygons`].
//! 2. **Exact convex-containment pruning:** removes any canonical polygon whose
//!    vertices all lie inside a single larger hole-free simple convex CCW
//!    sibling (`A ⊆ B => A ∪ B = B`), using an $O(1)$ conservative inscribed-disk
//!    fast-accept backed by exact `i128` $O(\log V)$ binary-search
//!    point-in-convex-polygon checks.
//! 3. **Deterministic X-sweep + Union-Find bbox-connected components:** isolates
//!    disjoint clusters; canonical singletons bypass Clipper2.
//! 4. **2D Morton Z-order spatial hierarchical reduction:** sorts each
//!    multi-polygon component by bbox-center Morton code and reduces in chunks
//!    of size `k`, with optional nested Rayon parallelism on large rounds and a
//!    strict termination guard (forcing a single final union when a round fails
//!    to shrink a component or at `MAX_HIER_DEPTH`).

use std::cmp::Reverse;
use std::collections::BTreeMap;

#[cfg(feature = "host-algos")]
use rayon::prelude::*;
#[cfg(test)]
use slicer_ir::Polygon;
use slicer_ir::{ExPolygon, Point2};

use crate::polygon_ops::{clip_polygons, ClipOperation};

/// Default spatial chunk fan-in `k` for [`union_spatial_tree`].
pub const DEFAULT_SPATIAL_UNION_CHUNK_SIZE: usize = 8;

/// Default nested Rayon parallelism flag for [`union_spatial_tree`].
///
/// Selected `false` (`worker-k8`) by the `t182` `base-classic-on` sweep
/// (`24.262 s` wall / `132.39 s` CPU vs `24.411 s` / `132.81 s` for `nested-k8`,
/// byte-identical output).
pub const DEFAULT_SPATIAL_UNION_NESTED_PAR: bool = false;

/// Maximum hierarchical reduction rounds before forcing a single final union.
const MAX_HIER_DEPTH: usize = 5;

/// Minimum chunk tasks required to engage nested Rayon fan-out.
#[cfg(feature = "host-algos")]
const NESTED_PAR_MIN_TASKS: usize = 4;

/// Minimum total input vertices in a reduction round to engage nested Rayon fan-out.
#[cfg(feature = "host-algos")]
const NESTED_PAR_MIN_VERTS: usize = 4_096;

/// Integer axis-aligned bounding box in workspace units (1 unit = 100 nm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BBox {
    min_x: i64,
    min_y: i64,
    max_x: i64,
    max_y: i64,
}

/// Computes the union of `subject` and `clip` via exact convex-containment
/// pruning, bbox-connected component decomposition, and Morton Z-order spatial
/// hierarchical reduction using the default chunk size
/// ([`DEFAULT_SPATIAL_UNION_CHUNK_SIZE`]) and nested-parallelism setting
/// ([`DEFAULT_SPATIAL_UNION_NESTED_PAR`]).
///
/// Falls back directly to [`clip_polygons`] for trivial (`<= 1`) inputs or when
/// any input polygon fails canonical Slic3r winding (CCW contour, CW holes).
#[must_use]
pub fn union_spatial_tree(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    union_spatial_tree_with_options(
        subject,
        clip,
        DEFAULT_SPATIAL_UNION_CHUNK_SIZE,
        DEFAULT_SPATIAL_UNION_NESTED_PAR,
    )
}

/// Parameterized entry point for [`union_spatial_tree`], allowing callers and
/// tests to specify the chunk fan-in `chunk_size` (clamped to `>= 2`) and
/// whether large reduction rounds may fan out via nested Rayon tasks
/// (`nested_par`, active only when compiled with `feature = "host-algos"`).
#[must_use]
pub fn union_spatial_tree_with_options(
    subject: &[ExPolygon],
    clip: &[ExPolygon],
    chunk_size: usize,
    nested_par: bool,
) -> Vec<ExPolygon> {
    #[cfg(not(feature = "host-algos"))]
    let _ = nested_par;

    let total_len = subject.len() + clip.len();
    if total_len == 0 {
        return Vec::new();
    }
    if total_len <= 1 {
        return clip_polygons(subject, clip, ClipOperation::Union);
    }

    // Verify canonical Slic3r winding (CCW contour, CW holes) across all inputs.
    // Under FillRule::NonZero, opposite-wound rings can punch holes in larger
    // rings, which is neither prune-safe nor associative under chunked union.
    if !subject.iter().chain(clip.iter()).all(has_canonical_winding) {
        return clip_polygons(subject, clip, ClipOperation::Union);
    }

    let k = chunk_size.max(2);
    let mut combined: Vec<ExPolygon> = Vec::with_capacity(total_len);
    combined.extend_from_slice(subject);
    combined.extend_from_slice(clip);

    let base_in = prune_convex_contained(&combined);
    if base_in.len() <= 1 {
        return base_in;
    }

    let comps = bbox_connected_components(&base_in);
    let mut final_out = Vec::new();
    let mut active: Vec<(Vec<ExPolygon>, bool)> = Vec::new();
    for mut comp in comps {
        if comp.len() == 1 {
            // Canonical winding was already verified for every input polygon.
            final_out.push(base_in[comp[0]].clone());
        } else {
            if comp.len() > k {
                morton_sort_indices(&base_in, &mut comp);
            }
            active.push((
                comp.into_iter().map(|i| base_in[i].clone()).collect(),
                false,
            ));
        }
    }

    let mut depth = 0usize;
    while !active.is_empty() {
        depth += 1;
        let mut tasks: Vec<(usize, Vec<ExPolygon>)> = Vec::new();
        let mut comp_final = vec![false; active.len()];
        let mut comp_prev = vec![0usize; active.len()];
        let mut next_accum: Vec<Vec<ExPolygon>> = vec![Vec::new(); active.len()];

        for (cid, (polys, force)) in active.iter().enumerate() {
            comp_prev[cid] = polys.len();
            if *force || polys.len() <= k || depth >= MAX_HIER_DEPTH {
                comp_final[cid] = true;
                if polys.len() == 1 {
                    next_accum[cid].push(polys[0].clone());
                } else {
                    tasks.push((cid, polys.clone()));
                }
            } else {
                for chunk in polys.chunks(k) {
                    if chunk.len() == 1 {
                        next_accum[cid].push(chunk[0].clone());
                    } else {
                        tasks.push((cid, chunk.to_vec()));
                    }
                }
            }
        }

        if !tasks.is_empty() {
            let run_one = |(cid, polys): (usize, Vec<ExPolygon>)| -> (usize, Vec<ExPolygon>) {
                let out = clip_union_head_tail(&polys);
                (cid, out)
            };

            #[cfg(feature = "host-algos")]
            let res: Vec<(usize, Vec<ExPolygon>)> = {
                let total_v: usize = tasks.iter().map(|t| slice_verts(&t.1)).sum();
                if nested_par
                    && tasks.len() >= NESTED_PAR_MIN_TASKS
                    && total_v >= NESTED_PAR_MIN_VERTS
                {
                    // Indexed `into_par_iter().collect()` preserves exact task order.
                    tasks.into_par_iter().map(run_one).collect()
                } else {
                    tasks.into_iter().map(run_one).collect()
                }
            };

            #[cfg(not(feature = "host-algos"))]
            let res: Vec<(usize, Vec<ExPolygon>)> = tasks.into_iter().map(run_one).collect();

            for (cid, out) in res {
                next_accum[cid].extend(out);
            }
        }

        let mut next_active = Vec::new();
        for cid in 0..active.len() {
            let merged = std::mem::take(&mut next_accum[cid]);
            if comp_final[cid] || merged.len() <= 1 {
                final_out.extend(merged);
            } else {
                let shrank = merged.len() < comp_prev[cid];
                let sub_comps = bbox_connected_components(&merged);
                for mut sc in sub_comps {
                    if sc.len() == 1 {
                        final_out.push(merged[sc[0]].clone());
                    } else {
                        let force = !shrank && sc.len() >= comp_prev[cid];
                        if !force && sc.len() > k {
                            morton_sort_indices(&merged, &mut sc);
                        }
                        next_active
                            .push((sc.into_iter().map(|i| merged[i].clone()).collect(), force));
                    }
                }
            }
        }
        active = next_active;
    }

    final_out
}

/// Executes a head-tail `ClipOperation::Union` (`subject = [first]`, `clip = rest`),
/// matching the tree-support planner's native convention.
#[inline]
fn clip_union_head_tail(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    if polys.len() <= 1 {
        return polys.to_vec();
    }
    let (head, tail) = polys.split_at(1);
    clip_polygons(head, tail, ClipOperation::Union)
}

#[cfg(feature = "host-algos")]
#[inline]
fn poly_verts(ep: &ExPolygon) -> usize {
    ep.contour.points.len() + ep.holes.iter().map(|h| h.points.len()).sum::<usize>()
}

#[cfg(feature = "host-algos")]
#[inline]
fn slice_verts(ps: &[ExPolygon]) -> usize {
    ps.iter().map(poly_verts).sum()
}

/// Exact `i128` doubled signed area of a closed ring (positive = CCW, negative = CW).
#[inline]
fn ring_double_area(pts: &[Point2]) -> i128 {
    let n = pts.len();
    if n < 3 {
        return 0;
    }
    let mut sum: i128 = 0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        sum += i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y);
    }
    sum
}

/// Exact `i128` net doubled area of a canonical `ExPolygon` (contour minus holes).
#[inline]
fn expolygon_net_double_area(ep: &ExPolygon) -> i128 {
    let mut a = ring_double_area(&ep.contour.points).abs();
    for hole in &ep.holes {
        a -= ring_double_area(&hole.points).abs();
    }
    a.max(0)
}

/// True when `ep` has Slic3r canonical winding: outer contour CCW (`area > 0`)
/// and every hole CW (`area < 0`), each with at least 3 vertices.
#[inline]
fn has_canonical_winding(ep: &ExPolygon) -> bool {
    ring_double_area(&ep.contour.points) > 0
        && ep.holes.iter().all(|h| ring_double_area(&h.points) < 0)
}

#[inline]
fn poly_bbox(ep: &ExPolygon) -> Option<BBox> {
    let mut it = ep
        .contour
        .points
        .iter()
        .chain(ep.holes.iter().flat_map(|h| h.points.iter()));
    let first = *it.next()?;
    let mut b = BBox {
        min_x: first.x,
        min_y: first.y,
        max_x: first.x,
        max_y: first.y,
    };
    for p in it {
        if p.x < b.min_x {
            b.min_x = p.x;
        }
        if p.y < b.min_y {
            b.min_y = p.y;
        }
        if p.x > b.max_x {
            b.max_x = p.x;
        }
        if p.y > b.max_y {
            b.max_y = p.y;
        }
    }
    Some(b)
}

#[inline]
fn bbox_contains(outer: BBox, inner: BBox) -> bool {
    outer.min_x <= inner.min_x
        && inner.max_x <= outer.max_x
        && outer.min_y <= inner.min_y
        && inner.max_y <= outer.max_y
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            if ra < rb {
                self.parent[rb] = ra;
            } else {
                self.parent[ra] = rb;
            }
        }
    }
}

/// Partition polygon indices into bbox-overlapping connected components via
/// X-axis sweep line + union-find, ordered deterministically by smallest member
/// index.
fn bbox_connected_components(polys: &[ExPolygon]) -> Vec<Vec<usize>> {
    let n = polys.len();
    if n == 0 {
        return Vec::new();
    }
    let bboxes: Vec<Option<BBox>> = polys.iter().map(poly_bbox).collect();
    let mut order: Vec<usize> = (0..n).filter(|&i| bboxes[i].is_some()).collect();
    order.sort_by_key(|&i| (bboxes[i].unwrap().min_x, i));

    let mut uf = UnionFind::new(n);
    for pos in 0..order.len() {
        let i = order[pos];
        let bi = bboxes[i].unwrap();
        for &j in &order[pos + 1..] {
            let bj = bboxes[j].unwrap();
            if bj.min_x > bi.max_x {
                break;
            }
            if bi.min_y <= bj.max_y && bj.min_y <= bi.max_y {
                uf.union(i, j);
            }
        }
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let root = uf.find(i);
        groups.entry(root).or_default().push(i);
    }
    groups.into_values().collect()
}

#[inline]
fn cross_i128(a: Point2, b: Point2, c: Point2) -> i128 {
    (i128::from(b.x) - i128::from(a.x)) * (i128::from(c.y) - i128::from(b.y))
        - (i128::from(b.y) - i128::from(a.y)) * (i128::from(c.x) - i128::from(b.x))
}

#[inline]
fn is_lower_half_plane(dx: i128, dy: i128) -> bool {
    dy < 0 || (dy == 0 && dx < 0)
}

/// Exact $O(V)$ certificate that `ep` is a hole-free, simple, convex CCW
/// polygon with winding number 1 (no duplicate vertices, no 180° reversals, no
/// double-wound star polygons).
fn is_convex_ccw(ep: &ExPolygon) -> bool {
    if !ep.holes.is_empty() || ep.contour.points.len() < 3 {
        return false;
    }
    let pts = &ep.contour.points;
    let n = pts.len();
    if ring_double_area(pts) <= 0 {
        return false;
    }

    let mut wraps: usize = 0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let c = pts[(i + 2) % n];
        let e1x = i128::from(b.x) - i128::from(a.x);
        let e1y = i128::from(b.y) - i128::from(a.y);
        let e2x = i128::from(c.x) - i128::from(b.x);
        let e2y = i128::from(c.y) - i128::from(b.y);
        if (e1x == 0 && e1y == 0) || (e2x == 0 && e2y == 0) {
            return false;
        }
        let cross = e1x * e2y - e1y * e2x;
        if cross < 0 {
            return false;
        }
        if cross == 0 {
            let dot = e1x * e2x + e1y * e2y;
            if dot <= 0 {
                // 180° foldback / spike.
                return false;
            }
        }
        if is_lower_half_plane(e1x, e1y) && !is_lower_half_plane(e2x, e2y) {
            wraps += 1;
            if wraps > 1 {
                return false;
            }
        }
    }
    wraps == 1
}

/// True when `p` lies on the closed segment `a -> b` (assumed collinear).
#[inline]
fn point_on_collinear_segment(p: Point2, a: Point2, b: Point2) -> bool {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y
}

/// Exact `i128` $O(\log V)$ point-in-convex-CCW-polygon test (inclusive of
/// boundary), exact even when edges contain collinear intermediate vertices.
///
/// Precondition: `convex` satisfies [`is_convex_ccw`].
fn point_in_convex_ccw(p: Point2, convex: &[Point2]) -> bool {
    let n = convex.len();
    if n < 3 {
        return false;
    }
    let o = convex[0];
    let c1 = cross_i128(o, convex[1], p);
    if c1 < 0 {
        return false;
    }
    if c1 == 0 {
        // Walk along any vertices collinear with `o -> convex[1]`.
        let mut far = 1usize;
        while far + 1 < n && cross_i128(o, convex[1], convex[far + 1]) == 0 {
            far += 1;
        }
        return point_on_collinear_segment(p, o, convex[far]);
    }
    let cn = cross_i128(o, convex[n - 1], p);
    if cn > 0 {
        return false;
    }
    if cn == 0 {
        // Walk back along any vertices collinear with `o -> convex[n - 1]`.
        let mut far = n - 1;
        while far > 1 && cross_i128(o, convex[n - 1], convex[far - 1]) == 0 {
            far -= 1;
        }
        return point_on_collinear_segment(p, o, convex[far]);
    }
    // Now `p` lies strictly inside the angular wedge between `o -> convex[1]`
    // and `o -> convex[n - 1]`. Binary search maintains:
    //   cross(o, convex[lo], p) >= 0  and  cross(o, convex[hi], p) < 0.
    let mut lo = 1usize;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if cross_i128(o, convex[mid], p) >= 0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    cross_i128(convex[lo], convex[lo + 1], p) >= 0
}

/// Computes bbox center `(cx, cy)`, conservative circumscribed radius `r_out`
/// (strict upper bound on vertex distance from `(cx, cy)`), and conservative
/// inscribed radius `r_in` (strict lower bound on distance from `(cx, cy)` to
/// any edge of a convex CCW polygon; `0.0` if center is not strictly inside).
fn convex_disk_bounds(pts: &[Point2], bb: BBox) -> (f64, f64, f64, f64) {
    let cx = 0.5 * (bb.min_x as f64 + bb.max_x as f64);
    let cy = 0.5 * (bb.min_y as f64 + bb.max_y as f64);
    let mut max_d2 = 0.0f64;
    for p in pts {
        let dx = p.x as f64 - cx;
        let dy = p.y as f64 - cy;
        let d2 = dx * dx + dy * dy;
        if d2 > max_d2 {
            max_d2 = d2;
        }
    }
    let r_out = max_d2.sqrt() * (1.0 + 1e-12) + 2.0;

    let mut min_edge_d = f64::INFINITY;
    let n = pts.len();
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let ex = (b.x - a.x) as f64;
        let ey = (b.y - a.y) as f64;
        let len = (ex * ex + ey * ey).sqrt();
        if len <= 0.0 {
            continue;
        }
        let sd = (ex * (cy - a.y as f64) - ey * (cx - a.x as f64)) / len;
        if sd <= 0.0 {
            return (cx, cy, r_out, 0.0);
        }
        if sd < min_edge_d {
            min_edge_d = sd;
        }
    }
    let r_in = if min_edge_d.is_finite() {
        (min_edge_d * (1.0 - 1e-12) - 2.0).max(0.0)
    } else {
        0.0
    };
    (cx, cy, r_out, r_in)
}

/// Fast exact pruning of canonical polygons strictly contained inside a single
/// larger hole-free simple convex CCW sibling (`A ⊆ B => A ∪ B = B`).
fn prune_convex_contained(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    let n = polys.len();
    if n <= 1 {
        return polys.to_vec();
    }
    let bboxes: Vec<Option<BBox>> = polys.iter().map(poly_bbox).collect();
    let areas: Vec<i128> = polys.iter().map(expolygon_net_double_area).collect();
    let convex: Vec<bool> = polys.iter().map(is_convex_ccw).collect();
    let disks: Vec<(f64, f64, f64, f64)> = (0..n)
        .map(|i| match bboxes[i] {
            Some(bb) if convex[i] => convex_disk_bounds(&polys[i].contour.points, bb),
            Some(bb) => {
                let cx = 0.5 * (bb.min_x as f64 + bb.max_x as f64);
                let cy = 0.5 * (bb.min_y as f64 + bb.max_y as f64);
                let hx = 0.5 * (bb.max_x as f64 - bb.min_x as f64);
                let hy = 0.5 * (bb.max_y as f64 - bb.min_y as f64);
                (
                    cx,
                    cy,
                    (hx * hx + hy * hy).sqrt() * (1.0 + 1e-12) + 2.0,
                    0.0,
                )
            }
            None => (0.0, 0.0, 0.0, 0.0),
        })
        .collect();

    // Sort convex hole-free containers by descending exact doubled area (tie-break by index).
    let mut convex_containers: Vec<usize> = (0..n)
        .filter(|&i| convex[i] && bboxes[i].is_some())
        .collect();
    convex_containers.sort_by_key(|&i| (Reverse(areas[i]), i));

    if convex_containers.is_empty() {
        return polys.to_vec();
    }

    // Visit smallest candidates first so identical convex siblings never drop each other.
    let mut by_area: Vec<usize> = (0..n).collect();
    by_area.sort_by_key(|&i| (areas[i], i));

    let mut dropped = vec![false; n];
    for &i in &by_area {
        let Some(bi) = bboxes[i] else {
            dropped[i] = true;
            continue;
        };
        let (cxi, cyi, rout_i, _) = disks[i];
        for &j in &convex_containers {
            if j == i || dropped[j] {
                continue;
            }
            if areas[j] < areas[i] {
                break;
            }
            let bj = bboxes[j].unwrap();
            if !bbox_contains(bj, bi) {
                continue;
            }
            let (cxj, cyj, _, rin_j) = disks[j];
            if rin_j > rout_i {
                let dx = cxi - cxj;
                let dy = cyi - cyj;
                let margin = rin_j - rout_i;
                if dx * dx + dy * dy <= margin * margin {
                    dropped[i] = true;
                    break;
                }
            }
            let container_pts = &polys[j].contour.points;
            let contour_inside = polys[i]
                .contour
                .points
                .iter()
                .all(|&p| point_in_convex_ccw(p, container_pts));
            if contour_inside
                && polys[i].holes.iter().all(|h| {
                    h.points
                        .iter()
                        .all(|&p| point_in_convex_ccw(p, container_pts))
                })
            {
                dropped[i] = true;
                break;
            }
        }
    }

    polys
        .iter()
        .enumerate()
        .filter_map(|(i, p)| if dropped[i] { None } else { Some(p.clone()) })
        .collect()
}

#[inline]
fn part1by1(mut x: u32) -> u64 {
    x &= 0xFFFF;
    let mut v = u64::from(x);
    v = (v | (v << 16)) & 0x0000_FFFF_0000_FFFF;
    v = (v | (v << 8)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333_3333_3333;
    v = (v | (v << 1)) & 0x5555_5555_5555_5555;
    v
}

/// Deterministic 2D Morton Z-order sort of `indices` by polygon bbox center.
fn morton_sort_indices(polys: &[ExPolygon], indices: &mut [usize]) {
    if indices.len() <= 1 {
        return;
    }
    let mut min_x = i128::MAX;
    let mut max_x = i128::MIN;
    let mut min_y = i128::MAX;
    let mut max_y = i128::MIN;
    let centers: Vec<(i128, i128)> = indices
        .iter()
        .map(|&idx| {
            let b = poly_bbox(&polys[idx]).unwrap_or(BBox {
                min_x: 0,
                min_y: 0,
                max_x: 0,
                max_y: 0,
            });
            let cx = (i128::from(b.min_x) + i128::from(b.max_x)) / 2;
            let cy = (i128::from(b.min_y) + i128::from(b.max_y)) / 2;
            min_x = min_x.min(cx);
            max_x = max_x.max(cx);
            min_y = min_y.min(cy);
            max_y = max_y.max(cy);
            (cx, cy)
        })
        .collect();
    let span_x = (max_x - min_x).max(1) as f64;
    let span_y = (max_y - min_y).max(1) as f64;
    let mut keyed: Vec<(u64, usize)> = indices
        .iter()
        .zip(centers.iter())
        .map(|(&idx, &(cx, cy))| {
            let nx = (((cx - min_x) as f64 / span_x) * 65535.0).clamp(0.0, 65535.0) as u32;
            let ny = (((cy - min_y) as f64 / span_y) * 65535.0).clamp(0.0, 65535.0) as u32;
            let code = part1by1(nx) | (part1by1(ny) << 1);
            (code, idx)
        })
        .collect();
    keyed.sort_unstable_by_key(|&(code, idx)| (code, idx));
    for (slot, (_, idx)) in indices.iter_mut().zip(keyed) {
        *slot = idx;
    }
}

/// Ensures `Polygon` helper usage stays clean when referenced only in tests.
#[cfg(test)]
fn _polygon_ring(pts: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_ops::xor;

    fn make_expoly(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
        ExPolygon {
            contour: _polygon_ring(contour),
            holes: holes.iter().map(|h| _polygon_ring(h)).collect(),
        }
    }

    fn regular_ngon(cx: i64, cy: i64, r: f64, sides: usize) -> ExPolygon {
        let mut pts = Vec::with_capacity(sides);
        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            pts.push(Point2 {
                x: cx + (r * theta.cos()).round() as i64,
                y: cy + (r * theta.sin()).round() as i64,
            });
        }
        ExPolygon {
            contour: Polygon { points: pts },
            holes: Vec::new(),
        }
    }

    fn total_area_units2(polys: &[ExPolygon]) -> f64 {
        polys
            .iter()
            .map(|ep| expolygon_net_double_area(ep) as f64 * 0.5)
            .sum()
    }

    #[test]
    fn convex_containment_prunes_strictly_inside_and_keeps_protruding() {
        let big = regular_ngon(0, 0, 10_000.0, 64);
        let inside_a = regular_ngon(1_000, -1_000, 3_000.0, 32);
        let inside_b = regular_ngon(-2_000, 2_000, 2_000.0, 16);
        let protruding = regular_ngon(9_500, 0, 3_000.0, 32);

        let pruned = prune_convex_contained(&[
            inside_a.clone(),
            big.clone(),
            protruding.clone(),
            inside_b.clone(),
        ]);
        assert_eq!(pruned.len(), 2, "only big and protruding should survive");
        assert_eq!(pruned[0], big);
        assert_eq!(pruned[1], protruding);
    }

    #[test]
    fn identical_convex_siblings_keep_exactly_one_copy() {
        let a = make_expoly(&[(0, 0), (10_000, 0), (10_000, 10_000), (0, 10_000)], &[]);
        let pruned = prune_convex_contained(&[a.clone(), a.clone(), a.clone()]);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0], a);
    }

    #[test]
    fn non_convex_c_shape_never_prunes_polygon_in_its_bay() {
        // C-shaped concave container whose bounding box contains `bay_square`,
        // even though `bay_square` is completely outside the C-shape's solid area.
        let c_shape = make_expoly(
            &[
                (0, 0),
                (10_000, 0),
                (10_000, 2_000),
                (2_000, 2_000),
                (2_000, 8_000),
                (10_000, 8_000),
                (10_000, 10_000),
                (0, 10_000),
            ],
            &[],
        );
        let bay_square = make_expoly(
            &[
                (4_000, 4_000),
                (8_000, 4_000),
                (8_000, 6_000),
                (4_000, 6_000),
            ],
            &[],
        );
        assert!(
            !is_convex_ccw(&c_shape),
            "C-shape must not pass convexity check"
        );
        let pruned = prune_convex_contained(&[c_shape.clone(), bay_square.clone()]);
        assert_eq!(
            pruned.len(),
            2,
            "polygon inside concave bay must never be pruned"
        );

        let got =
            union_spatial_tree_with_options(&[c_shape.clone()], &[bay_square.clone()], 2, false);
        let want = clip_polygons(&[c_shape], &[bay_square], ClipOperation::Union);
        let xor_area = total_area_units2(&xor(&got, &want));
        assert_eq!(
            xor_area, 0.0,
            "bay polygon must remain disjoint solid in union"
        );
    }

    #[test]
    fn star_pentagram_and_foldback_rejected_as_convex_containers() {
        // Double-wound CCW star pentagram: every local vertex turn is strictly left,
        // signed area > 0, yet total turning angle is 4*pi (not simple convex).
        let star = make_expoly(
            &[
                (0, 10_000),
                (-5_878, -8_090),
                (9_511, 3_090),
                (-9_511, 3_090),
                (5_878, -8_090),
            ],
            &[],
        );
        assert!(
            !is_convex_ccw(&star),
            "double-wound star pentagram must be rejected by half-plane wrap check"
        );

        // 180-degree spike/foldback on bottom edge.
        let spike = make_expoly(
            &[
                (0, 0),
                (10_000, 0),
                (5_000, 0),
                (10_000, 10_000),
                (0, 10_000),
            ],
            &[],
        );
        assert!(
            !is_convex_ccw(&spike),
            "180-degree reversal must be rejected by collinear dot check"
        );
    }

    #[test]
    fn point_in_convex_ccw_rejects_point_on_collinear_closing_ray_extension() {
        // Convex polygon with collinear vertex `(0, 10)` on the closing edge `(0, 20) -> (0, 0)`.
        // Point `(0, 25)` lies on the ray `o -> convex[n - 1]` strictly OUTSIDE the polygon.
        let poly = make_expoly(&[(0, 0), (10, 0), (10, 10), (0, 20), (0, 10)], &[]);
        assert!(is_convex_ccw(&poly));
        assert!(point_in_convex_ccw(
            Point2 { x: 0, y: 15 },
            &poly.contour.points
        ));
        assert!(point_in_convex_ccw(
            Point2 { x: 0, y: 20 },
            &poly.contour.points
        ));
        assert!(
            !point_in_convex_ccw(Point2 { x: 0, y: 25 }, &poly.contour.points),
            "point beyond closing collinear edge must not test inside"
        );
        assert!(
            !point_in_convex_ccw(Point2 { x: 0, y: -5 }, &poly.contour.points),
            "point behind origin on closing ray must not test inside"
        );
    }

    #[test]
    fn hole_bearing_container_never_prunes_island_inside_its_hole() {
        // Outer square with large hole + small island sitting inside the hole.
        // The hole-bearing polygon must NOT be treated as a convex container.
        let frame = make_expoly(
            &[(0, 0), (20_000, 0), (20_000, 20_000), (0, 20_000)],
            &[&[
                (4_000, 4_000),
                (4_000, 16_000),
                (16_000, 16_000),
                (16_000, 4_000),
            ]],
        );
        let island = make_expoly(
            &[
                (8_000, 8_000),
                (12_000, 8_000),
                (12_000, 12_000),
                (8_000, 12_000),
            ],
            &[],
        );
        assert!(!is_convex_ccw(&frame));
        let got = union_spatial_tree_with_options(&[frame.clone()], &[island.clone()], 2, false);
        let want = clip_polygons(&[frame], &[island], ClipOperation::Union);

        // Both must preserve the frame with 1 hole AND the separate nested island.
        assert_eq!(
            got.len(),
            2,
            "frame + island inside its hole must both survive"
        );
        assert_eq!(total_area_units2(&xor(&got, &want)), 0.0);
    }

    #[test]
    fn opposite_wound_non_canonical_ring_falls_back_to_nonzero_hole_punching() {
        // Regression guard: an inner CW ring passed as a flat ExPolygon punches a
        // hole in a larger CCW square under FillRule::NonZero. `union_spatial_tree`
        // must detect non-canonical winding and preserve the hole verbatim.
        let outer = make_expoly(&[(0, 0), (10_000, 0), (10_000, 10_000), (0, 10_000)], &[]);
        let inner_cw = make_expoly(
            &[
                (3_000, 3_000),
                (3_000, 7_000),
                (7_000, 7_000),
                (7_000, 3_000),
            ],
            &[],
        );
        let extra = make_expoly(
            &[(20_000, 0), (30_000, 0), (30_000, 10_000), (20_000, 10_000)],
            &[],
        );

        let got = union_spatial_tree_with_options(
            &[outer.clone(), inner_cw.clone(), extra.clone()],
            &[],
            2,
            false,
        );
        let want = clip_polygons(&[outer, inner_cw, extra], &[], ClipOperation::Union);
        assert_eq!(got, want);
        assert_eq!(
            got.iter().map(|e| e.holes.len()).sum::<usize>(),
            1,
            "CW inner ring must punch a hole, never be pruned"
        );
    }

    #[test]
    fn non_shrinking_disjoint_bbox_chain_terminates_cleanly() {
        // 25 thin diagonal diamonds whose axis-aligned bounding boxes all overlap
        // into a single connected component, yet the polygons themselves are
        // strictly disjoint so chunked unions never reduce the polygon count.
        // Verifies the strict termination guard (`!shrank` -> `force = true`).
        let mut polys = Vec::new();
        for i in 0..25i64 {
            let d = i * 400;
            // Thin parallelogram hugging the diagonal x - y = d, spanning 0..20_000.
            let p = make_expoly(
                &[
                    (d, 0),
                    (d + 100, 0),
                    (d + 20_100, 20_000),
                    (d + 20_000, 20_000),
                ],
                &[],
            );
            polys.push(p);
        }
        let got = union_spatial_tree_with_options(&polys[..1], &polys[1..], 4, false);
        let want = clip_polygons(&polys[..1], &polys[1..], ClipOperation::Union);
        assert_eq!(got.len(), 25, "all 25 disjoint strips must survive");
        assert_eq!(total_area_units2(&xor(&got, &want)), 0.0);
    }

    #[test]
    fn dense_overlapping_branch_cluster_matches_monolithic_within_submicron_xor() {
        // Mimics tree-support Phase-3 ROLE_BODY: 96 overlapping 64-gon discs across
        // 3 clusters plus 16 strictly-nested smaller discs and 4 isolated singletons.
        let mut polys = Vec::new();
        for c in 0..3i64 {
            let bx = c * 150_000;
            for j in 0..32i64 {
                let angle = (j as f64) * 0.45;
                let dist = (j as f64) * 1_200.0;
                let x = bx + (dist * angle.cos()).round() as i64;
                let y = (dist * angle.sin()).round() as i64;
                let r = 12_000.0 + ((j % 5) as f64) * 2_500.0;
                polys.push(regular_ngon(x, y, r, 64));
                if j % 2 == 0 {
                    // Strictly nested disc inside the one just pushed.
                    polys.push(regular_ngon(x + 1_000, y - 1_000, 5_000.0, 32));
                }
            }
        }
        for s in 0..4i64 {
            polys.push(regular_ngon(600_000 + s * 50_000, 0, 8_000.0, 48));
        }

        let want = clip_polygons(&polys[..1], &polys[1..], ClipOperation::Union);
        let want_area = total_area_units2(&want);
        assert!(want_area > 0.0);

        for &k in &[4usize, 8, 16] {
            let got = union_spatial_tree_with_options(&polys[..1], &polys[1..], k, false);
            assert!(!got.is_empty());
            let xor_area_units2 = total_area_units2(&xor(&got, &want));
            let rel_xor = xor_area_units2 / want_area;
            assert!(
                rel_xor < 1e-6,
                "k={k}: relative XOR area {rel_xor:.3e} ({xor_area_units2} units^2) exceeded 1e-6"
            );
        }
    }

    #[cfg(feature = "host-algos")]
    #[test]
    fn nested_parallel_is_bit_identical_to_serial() {
        // test-quality: determinism pin — proves nested Rayon fan-out preserves exact
        // order and outputs bit-identical polygons to worker-local serial execution.
        let mut polys = Vec::new();
        for j in 0..64i64 {
            let x = (j % 8) * 9_000;
            let y = (j / 8) * 9_000;
            polys.push(regular_ngon(x, y, 7_500.0, 80));
        }
        let serial = union_spatial_tree_with_options(&polys[..1], &polys[1..], 8, false);
        let parallel = union_spatial_tree_with_options(&polys[..1], &polys[1..], 8, true);
        assert!(!serial.is_empty());
        assert_eq!(serial, parallel);
    }
}
