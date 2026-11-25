//! Exact, non-perturbing per-layer spatial tile index for phase-1 collision
//! carving (`build_roles_batched` phase 1, wayfinder ticket 175).
//!
//! # Why non-perturbing homotopic windowing instead of rectangular clipping
//!
//! Subtracting a layer's full `collision_polys` (~6,241 vertices over ~4.34
//! rings on `base.stl`) from each of ~68,381 small (~98.7-vertex) branch
//! regions pushes ~426.9 M clip points across the WIT boundary even though
//! only ~0.067 % lie inside each subject's bounding box (ticket 173).
//!
//! Cutting `collision_polys` into rectangular tiles via `Intersection` bends
//! any sloped edge crossing a tile boundary by up to 0.5 slicer units (50 nm)
//! when the boundary intersection rounds to integer coordinates, which can
//! shift carved output vertices by 1 unit (100 nm) and break byte-identity.
//!
//! Instead, this index bins the original directed edges of every ring into a
//! uniform tile grid of size `tile_units` (stored in flat CSR buffers) and
//! records the sparse list of edges crossing each tile row's reference line
//! `y = origin_y + ty * tile_units + 0.5`. For any query bounding box `B`, let
//! `U` be the union of tiles covering `B` padded by 1 unit:
//!
//! 1. Every edge touching `U` is kept **verbatim** with its exact original
//!    integer endpoints.
//! 2. Every maximal chain of unkept (strictly exterior) edges between two kept
//!    runs is replaced by a short axis-aligned detour along an exterior frame
//!    `F` strictly enclosing both `collision_polys` and `U`, matching the
//!    chain's exact signed crossing count of the horizontal ray from an
//!    interior reference point `p_c = (x_ref + 0.5, y_ref + 0.5) ∈ int(U)`.
//! 3. Non-touching rings contribute only their constant signed winding number
//!    `w_r = W(ring, p_c)`, summed into one integer `w_const`.
//!
//! By winding-number invariance under homotopy in `R² \ U`, the contracted
//! clip set has the **exact same integer winding number** at every point of
//! `U` (and every touching edge bit-for-bit identical) as the full
//! `collision_polys`, even for overlapping `ExPolygon`s, nested holes, or
//! non-canonical winding. In addition:
//!
//! - If no edges touch `U` and `w_const == 0`, the subject is provably
//!   disjoint from `collision_polys` (`WindowedCollision::Empty`).
//! - If no edges touch `U` and `w_const != 0`, every point of the subject lies
//!   strictly inside `collision_polys` (`WindowedCollision::Covered`), so
//!   `Difference(subject, collision)` is provably empty without calling
//!   Clipper.

use std::collections::HashMap;

use slicer_ir::{ExPolygon, Point2, Polygon};

/// Maximum grid cells per layer index before `tile_units` is doubled to bound
/// memory on pathological coordinate spans (256 × 256 = 65,536 cells = 256 KiB
/// for `cell_offsets`). Normal 150 mm beds at 1 mm tiles use ~22,500 cells.
const MAX_GRID_CELLS: usize = 65_536;

/// Exterior chains at or below this edge count are kept verbatim because a
/// frame detour can add up to 6 vertices.
const MIN_CONTRACTED_CHAIN_EDGES: usize = 7;

/// Minimum total collision vertices before building a per-layer tile index.
/// Tiny collision sets (e.g. unit-test squares) cost less to clip directly.
pub(crate) const MIN_INDEXED_COLLISION_VERTICES: usize = 64;

/// Default tile size in canonical slicer units (`1 unit = 100 nm`, so
/// `10_000 = 1 mm`). Chosen by the ticket-175 tile-size sweep on
/// `base classic-on`.
pub(crate) const DEFAULT_COLLISION_TILE_UNITS: i64 = 20_000;

/// Exact collision operand for a subject region's tile-union window.
#[derive(Debug, Clone)]
pub(crate) enum WindowedCollision {
    /// No collision edges touch the window and net winding inside the window
    /// is zero: `subject` is provably disjoint from `collision_polys`.
    Empty,
    /// No collision edges touch the window and net winding inside the window
    /// is non-zero: every point of `subject` is strictly inside
    /// `collision_polys`, so `Difference(subject, collision)` is empty.
    Covered,
    /// Exact, non-perturbing contracted clip set for `Difference`.
    Polys(Vec<ExPolygon>),
}

#[derive(Debug, Clone, Copy)]
struct RowCrossing {
    ring_id: u32,
    edge_idx: u32,
    /// `+1` for upward crossing (`p0.y <= y_ref < p1.y`), `-1` for downward.
    dir: i8,
}

/// Optional counters for attribution probes; zero-cost when unused.
#[derive(Debug, Default, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct TileQueryStats {
    pub(crate) queries: u64,
    pub(crate) cache_hits: u64,
    pub(crate) empty_hits: u64,
    pub(crate) covered_hits: u64,
    pub(crate) poly_hits: u64,
    pub(crate) original_clip_points: u64,
    pub(crate) windowed_clip_points: u64,
}

/// Per-layer exact spatial tile index over `collision_polys`.
pub(crate) struct CollisionTileIndex<'a> {
    rings: Vec<&'a [Point2]>,
    total_points: usize,
    grid_min_x: i64,
    grid_max_x: i64,
    grid_min_y: i64,
    grid_max_y: i64,
    tile_units: i64,
    cols: usize,
    rows: usize,
    cell_offsets: Vec<u32>,
    cell_entries: Vec<(u32, u32)>,
    row_offsets: Vec<u32>,
    row_entries: Vec<RowCrossing>,
    /// Reusable per-query scratch to avoid per-region allocations.
    scratch_candidates: Vec<(u32, u32)>,
    scratch_ring_stamp: Vec<u32>,
    scratch_stamp: u32,
    /// Memo keyed by `(tx0_raw, tx1_raw, ty0_raw, ty1_raw)` so multiple branch
    /// regions sharing the same tile union reuse the extracted clip set.
    window_cache: HashMap<(i64, i64, i64, i64), WindowedCollision>,
    stats: TileQueryStats,
}

impl<'a> CollisionTileIndex<'a> {
    /// Build a per-layer tile index for `collision_polys` using `tile_units`
    /// (`> 0`). Returns `None` when `collision_polys` has no valid rings.
    pub(crate) fn build(
        collision_polys: &'a [ExPolygon],
        requested_tile_units: i64,
    ) -> Option<Self> {
        let mut rings: Vec<&'a [Point2]> = Vec::new();
        let mut total_points: usize = 0;
        let mut min_x = i64::MAX;
        let mut max_x = i64::MIN;
        let mut min_y = i64::MAX;
        let mut max_y = i64::MIN;

        let mut push_ring = |pts: &'a [Point2]| {
            if pts.len() < 3 {
                return;
            }
            for p in pts {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
            total_points += pts.len();
            rings.push(pts);
        };

        for poly in collision_polys {
            push_ring(&poly.contour.points);
            for hole in &poly.holes {
                push_ring(&hole.points);
            }
        }
        if rings.is_empty() {
            return None;
        }

        let mut tile_units = requested_tile_units.max(2);
        let span_x = (max_x as i128 - min_x as i128).max(0);
        let span_y = (max_y as i128 - min_y as i128).max(0);
        let (cols, rows) = loop {
            let c = (span_x / tile_units as i128) + 1;
            let r = (span_y / tile_units as i128) + 1;
            if c.saturating_mul(r) <= MAX_GRID_CELLS as i128 || tile_units >= (i64::MAX / 4) {
                break (c as usize, r as usize);
            }
            tile_units = tile_units.saturating_mul(2);
        };

        let cell_count = cols * rows;
        let mut cell_counts: Vec<u32> = vec![0; cell_count];
        let mut row_counts: Vec<u32> = vec![0; rows];

        // Pass 1: count CSR entries per cell and per horizontal row line.
        for pts in &rings {
            let m = pts.len();
            for k in 0..m {
                let p0 = pts[k];
                let p1 = pts[(k + 1) % m];
                for_each_edge_cell(p0, p1, min_x, min_y, tile_units, cols, rows, |cell_idx| {
                    cell_counts[cell_idx] += 1;
                });
                if let Some((ty_start, ty_end, _dir)) =
                    edge_row_crossing_range(p0, p1, min_y, tile_units, rows)
                {
                    for ty in ty_start..=ty_end {
                        row_counts[ty] += 1;
                    }
                }
            }
        }

        // Prefix sums for CSR layouts.
        let mut cell_offsets: Vec<u32> = Vec::with_capacity(cell_count + 1);
        let mut acc: u32 = 0;
        for &c in &cell_counts {
            cell_offsets.push(acc);
            acc += c;
        }
        cell_offsets.push(acc);
        let mut cell_entries: Vec<(u32, u32)> = vec![(0, 0); acc as usize];
        let mut cell_write = cell_offsets[..cell_count].to_vec();

        let mut row_offsets: Vec<u32> = Vec::with_capacity(rows + 1);
        let mut racc: u32 = 0;
        for &c in &row_counts {
            row_offsets.push(racc);
            racc += c;
        }
        row_offsets.push(racc);
        let mut row_entries: Vec<RowCrossing> = vec![
            RowCrossing {
                ring_id: 0,
                edge_idx: 0,
                dir: 0,
            };
            racc as usize
        ];
        let mut row_write = row_offsets[..rows].to_vec();

        // Pass 2: fill CSR arrays.
        for (ring_idx, pts) in rings.iter().enumerate() {
            let ring_id = ring_idx as u32;
            let m = pts.len();
            for k in 0..m {
                let edge_idx = k as u32;
                let p0 = pts[k];
                let p1 = pts[(k + 1) % m];
                for_each_edge_cell(p0, p1, min_x, min_y, tile_units, cols, rows, |cell_idx| {
                    let dst = cell_write[cell_idx] as usize;
                    cell_entries[dst] = (ring_id, edge_idx);
                    cell_write[cell_idx] = (dst + 1) as u32;
                });
                if let Some((ty_start, ty_end, dir)) =
                    edge_row_crossing_range(p0, p1, min_y, tile_units, rows)
                {
                    let crossing = RowCrossing {
                        ring_id,
                        edge_idx,
                        dir,
                    };
                    for ty in ty_start..=ty_end {
                        let dst = row_write[ty] as usize;
                        row_entries[dst] = crossing;
                        row_write[ty] = (dst + 1) as u32;
                    }
                }
            }
        }

        let ring_count = rings.len();
        Some(Self {
            rings,
            total_points,
            grid_min_x: min_x,
            grid_max_x: max_x,
            grid_min_y: min_y,
            grid_max_y: max_y,
            tile_units,
            cols,
            rows,
            cell_offsets,
            cell_entries,
            row_offsets,
            row_entries,
            scratch_candidates: Vec::new(),
            scratch_ring_stamp: vec![0; ring_count],
            scratch_stamp: 0,
            window_cache: HashMap::new(),
            stats: TileQueryStats::default(),
        })
    }

    /// Return the exact, non-perturbing collision operand covering the subject
    /// bounding box `(rx0, rx1, ry0, ry1)`.
    pub(crate) fn window_for_bbox(&mut self, bbox: (i64, i64, i64, i64)) -> WindowedCollision {
        let (rx0, rx1, ry0, ry1) = bbox;
        self.stats.queries += 1;
        self.stats.original_clip_points += self.total_points as u64;

        if rx1 < self.grid_min_x
            || rx0 > self.grid_max_x
            || ry1 < self.grid_min_y
            || ry0 > self.grid_max_y
        {
            self.stats.empty_hits += 1;
            return WindowedCollision::Empty;
        }

        let s = self.tile_units;
        // Pad by 1 unit so the subject lies strictly inside the tile union U.
        let tx0_raw = (rx0.saturating_sub(1) - self.grid_min_x).div_euclid(s);
        let tx1_raw = (rx1.saturating_add(1) - self.grid_min_x).div_euclid(s);
        let ty0_raw = (ry0.saturating_sub(1) - self.grid_min_y).div_euclid(s);
        let ty1_raw = (ry1.saturating_add(1) - self.grid_min_y).div_euclid(s);

        let key = (tx0_raw, tx1_raw, ty0_raw, ty1_raw);
        if let Some(cached) = self.window_cache.get(&key) {
            self.stats.cache_hits += 1;
            match cached {
                WindowedCollision::Empty => self.stats.empty_hits += 1,
                WindowedCollision::Covered => self.stats.covered_hits += 1,
                WindowedCollision::Polys(polys) => {
                    self.stats.poly_hits += 1;
                    self.stats.windowed_clip_points += count_expolygon_points(polys) as u64;
                }
            }
            return cached.clone();
        }

        let result = self.compute_window(tx0_raw, tx1_raw, ty0_raw, ty1_raw);
        match &result {
            WindowedCollision::Empty => self.stats.empty_hits += 1,
            WindowedCollision::Covered => self.stats.covered_hits += 1,
            WindowedCollision::Polys(polys) => {
                self.stats.poly_hits += 1;
                self.stats.windowed_clip_points += count_expolygon_points(polys) as u64;
            }
        }
        self.window_cache.insert(key, result.clone());
        result
    }

    #[allow(dead_code)]
    pub(crate) fn stats(&self) -> TileQueryStats {
        self.stats
    }

    fn compute_window(
        &mut self,
        tx0_raw: i64,
        tx1_raw: i64,
        ty0_raw: i64,
        ty1_raw: i64,
    ) -> WindowedCollision {
        let s = self.tile_units;
        let gx0 = tx0_raw.max(0) as usize;
        let gx1 = tx1_raw.min(self.cols as i64 - 1) as usize;
        let gy0 = ty0_raw.max(0) as usize;
        let gy1 = ty1_raw.min(self.rows as i64 - 1) as usize;

        // Exact tile-union rectangle U covering the padded subject bbox.
        let ux0 = self.grid_min_x + tx0_raw * s;
        let ux1 = self.grid_min_x + (tx1_raw + 1) * s;
        let uy0 = self.grid_min_y + ty0_raw * s;
        let uy1 = self.grid_min_y + (ty1_raw + 1) * s;

        // Interior reference point p_c = (x_ref + 0.5, y_ref + 0.5) ∈ int(U).
        let x_ref = self.grid_min_x + (gx0 as i64) * s;
        let y_ref = self.grid_min_y + (gy0 as i64) * s;

        // Gather all candidate edges from tiles overlapped by U.
        self.scratch_candidates.clear();
        for gy in gy0..=gy1 {
            let row_base = gy * self.cols;
            for gx in gx0..=gx1 {
                let idx = row_base + gx;
                let start = self.cell_offsets[idx] as usize;
                let end = self.cell_offsets[idx + 1] as usize;
                self.scratch_candidates
                    .extend_from_slice(&self.cell_entries[start..end]);
            }
        }

        // Advance generation stamp for touched rings.
        self.scratch_stamp = self.scratch_stamp.wrapping_add(1);
        if self.scratch_stamp == 0 {
            self.scratch_ring_stamp.fill(0);
            self.scratch_stamp = 1;
        }
        let stamp = self.scratch_stamp;

        if !self.scratch_candidates.is_empty() {
            self.scratch_candidates.sort_unstable();
            self.scratch_candidates.dedup();
            for &(ring_id, _) in &self.scratch_candidates {
                self.scratch_ring_stamp[ring_id as usize] = stamp;
            }
        }

        // Row crossings on y = y_ref + 0.5 give O(crossings) exact winding for
        // all non-touching rings AND all exterior chains of touching rings.
        let rstart = self.row_offsets[gy0] as usize;
        let rend = self.row_offsets[gy0 + 1] as usize;
        let row_slice = &self.row_entries[rstart..rend];

        let mut w_const: i32 = 0;
        for rc in row_slice {
            if self.scratch_ring_stamp[rc.ring_id as usize] == stamp {
                continue;
            }
            let pts = self.rings[rc.ring_id as usize];
            let k = rc.edge_idx as usize;
            let p0 = pts[k];
            let p1 = pts[(k + 1) % pts.len()];
            if crosses_right_ray(p0, p1, x_ref, y_ref) {
                w_const += rc.dir as i32;
            }
        }

        // Fast paths when NO collision edge touches U at all!
        if self.scratch_candidates.is_empty() {
            return if w_const == 0 {
                WindowedCollision::Empty
            } else {
                WindowedCollision::Covered
            };
        }

        // Exterior frame F strictly encloses both collision_polys and U so
        // every ring vertex and every point of U sits strictly inside F.
        let fx0 = self.grid_min_x.min(ux0).saturating_sub(2);
        let fx1 = self.grid_max_x.max(ux1).saturating_add(2);
        let fy0 = self.grid_min_y.min(uy0).saturating_sub(2);
        let fy1 = self.grid_max_y.max(uy1).saturating_add(2);
        let frame = Frame {
            ux0,
            ux1,
            uy0,
            uy1,
            fx0,
            fx1,
            fy0,
            fy1,
            y_ref,
        };

        let mut out_polys: Vec<ExPolygon> = Vec::new();

        // Represent constant winding from all non-touching enclosing rings as
        // |w_const| 4-vertex rectangles strictly enclosing F (so they never
        // overlap F's detour segments or U).
        if w_const != 0 {
            let bx0 = fx0.saturating_sub(1);
            let bx1 = fx1.saturating_add(1);
            let by0 = fy0.saturating_sub(1);
            let by1 = fy1.saturating_add(1);
            let rect_pts = if w_const > 0 {
                vec![
                    Point2 { x: bx0, y: by0 },
                    Point2 { x: bx1, y: by0 },
                    Point2 { x: bx1, y: by1 },
                    Point2 { x: bx0, y: by1 },
                ]
            } else {
                vec![
                    Point2 { x: bx0, y: by0 },
                    Point2 { x: bx0, y: by1 },
                    Point2 { x: bx1, y: by1 },
                    Point2 { x: bx1, y: by0 },
                ]
            };
            for _ in 0..w_const.unsigned_abs() {
                out_polys.push(ExPolygon {
                    contour: Polygon {
                        points: rect_pts.clone(),
                    },
                    holes: Vec::new(),
                });
            }
        }

        // Group sorted unique candidate edges by ring_id and contract each
        // touching ring's long exterior chains homotopically on ∂F.
        let mut cursor = 0;
        let cand_len = self.scratch_candidates.len();
        while cursor < cand_len {
            let ring_id = self.scratch_candidates[cursor].0;
            let group_start = cursor;
            while cursor < cand_len && self.scratch_candidates[cursor].0 == ring_id {
                cursor += 1;
            }
            let pts = self.rings[ring_id as usize];
            let m = pts.len();
            let kept_count = cursor - group_start;

            let ring_points = if kept_count == m {
                pts.to_vec()
            } else {
                contract_ring(
                    ring_id,
                    pts,
                    &self.scratch_candidates[group_start..cursor],
                    row_slice,
                    x_ref,
                    y_ref,
                    &frame,
                )
            };
            if ring_points.len() >= 3 {
                out_polys.push(ExPolygon {
                    contour: Polygon {
                        points: ring_points,
                    },
                    holes: Vec::new(),
                });
            }
        }

        WindowedCollision::Polys(out_polys)
    }
}

struct Frame {
    ux0: i64,
    ux1: i64,
    uy0: i64,
    uy1: i64,
    fx0: i64,
    fx1: i64,
    fy0: i64,
    fy1: i64,
    y_ref: i64,
}

impl Frame {
    /// Project a point `q` strictly outside `U` onto `∂F` along an axis-aligned
    /// segment strictly outside `U` with zero crossings of the reference ray
    /// `(x > x_ref + 0.5, y = y_ref + 0.5)`. Returns `(projected_point, seg)`
    /// where `seg ∈ 0..=4` orders `∂F \ {P_*}` counter-clockwise starting from
    /// `P_* = (fx1, y_ref + 0.5)`.
    #[inline]
    fn project(&self, q: Point2) -> (Point2, i64) {
        if q.x > self.ux1 {
            let p = Point2 {
                x: self.fx1,
                y: q.y,
            };
            let seg = if q.y > self.y_ref { 0 } else { 4 };
            (p, seg)
        } else if q.x < self.ux0 {
            (
                Point2 {
                    x: self.fx0,
                    y: q.y,
                },
                2,
            )
        } else if q.y > self.uy1 {
            (
                Point2 {
                    x: q.x,
                    y: self.fy1,
                },
                1,
            )
        } else {
            debug_assert!(
                q.y < self.uy0,
                "exterior chain endpoint must lie strictly outside U"
            );
            (
                Point2 {
                    x: q.x,
                    y: self.fy0,
                },
                3,
            )
        }
    }

    #[inline]
    fn corner_for_lift(&self, s: i64) -> Option<Point2> {
        match s.rem_euclid(5) {
            1 => Some(Point2 {
                x: self.fx1,
                y: self.fy1,
            }),
            2 => Some(Point2 {
                x: self.fx0,
                y: self.fy1,
            }),
            3 => Some(Point2 {
                x: self.fx0,
                y: self.fy0,
            }),
            4 => Some(Point2 {
                x: self.fx1,
                y: self.fy0,
            }),
            _ => None, // `0` is P_* on the right edge between C3 and C0 (collinear).
        }
    }
}

#[inline]
fn push_dedup(out: &mut Vec<Point2>, p: Point2) {
    if out.last() != Some(&p) {
        out.push(p);
    }
}

/// Rebuild a touching ring keeping every edge in `kept` verbatim and replacing
/// every long exterior chain with an exact homotopic detour on `∂F`.
fn contract_ring(
    ring_id: u32,
    pts: &[Point2],
    kept: &[(u32, u32)],
    row_crossings: &[RowCrossing],
    x_ref: i64,
    y_ref: i64,
    frame: &Frame,
) -> Vec<Point2> {
    let m = pts.len();
    let s = kept.len();
    let mut out: Vec<Point2> = Vec::with_capacity(s + 16);

    for j in 0..s {
        let k = kept[j].1 as usize;
        let next_k = kept[(j + 1) % s].1 as usize;
        let after_k = (k + 1) % m;

        push_dedup(&mut out, pts[k]);

        if after_k == next_k {
            continue;
        }

        // Exterior chain covers cyclic edge indices [after_k .. next_k).
        let a = pts[after_k];
        push_dedup(&mut out, a);

        let chain_len = if after_k < next_k {
            next_k - after_k
        } else {
            m - after_k + next_k
        };

        if chain_len < MIN_CONTRACTED_CHAIN_EDGES {
            let mut idx = (after_k + 1) % m;
            while idx != next_k {
                push_dedup(&mut out, pts[idx]);
                idx = (idx + 1) % m;
            }
            continue;
        }

        // Exact signed crossing count of the reference ray by exterior edges
        // strictly inside the cyclic range [after_k .. next_k).
        let mut w_chain: i32 = 0;
        for rc in row_crossings {
            if rc.ring_id != ring_id {
                continue;
            }
            let e = rc.edge_idx as usize;
            let in_chain = if after_k < next_k {
                e >= after_k && e < next_k
            } else {
                e >= after_k || e < next_k
            };
            if !in_chain {
                continue;
            }
            let p0 = pts[e];
            let p1 = pts[(e + 1) % m];
            if crosses_right_ray(p0, p1, x_ref, y_ref) {
                w_chain += rc.dir as i32;
            }
        }

        let b = pts[next_k];
        let (a_proj, seg_a) = frame.project(a);
        let (b_proj, seg_b) = frame.project(b);
        let end_seg = seg_b + 5 * (w_chain as i64);

        push_dedup(&mut out, a_proj);
        if seg_a < end_seg {
            for lift in (seg_a + 1)..=end_seg {
                if let Some(corner) = frame.corner_for_lift(lift) {
                    push_dedup(&mut out, corner);
                }
            }
        } else if seg_a > end_seg {
            for lift in ((end_seg + 1)..=seg_a).rev() {
                if let Some(corner) = frame.corner_for_lift(lift) {
                    push_dedup(&mut out, corner);
                }
            }
        }
        push_dedup(&mut out, b_proj);
    }

    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Exact `i128` test: does the segment `p0 -> p1` cross the open horizontal
/// ray `{(x, y_ref + 0.5) : x > x_ref + 0.5}`? Assumes `p0` and `p1` straddle
/// `y_ref + 0.5` (`p0.y <= y_ref < p1.y` or `p1.y <= y_ref < p0.y`).
#[inline]
fn crosses_right_ray(p0: Point2, p1: Point2, x_ref: i64, y_ref: i64) -> bool {
    let (lo, hi) = if p0.y < p1.y { (p0, p1) } else { (p1, p0) };
    let dy = (hi.y - lo.y) as i128;
    debug_assert!(dy > 0);
    // Quick x-bound reject before 128-bit multiply.
    if lo.x <= x_ref && hi.x <= x_ref {
        return false;
    }
    if lo.x > x_ref && hi.x > x_ref {
        return true;
    }
    let dx = (hi.x - lo.x) as i128;
    let lhs = 2 * dy * ((lo.x - x_ref) as i128) + dx * (2 * ((y_ref - lo.y) as i128) + 1);
    lhs > dy
}

/// Horizontal tile-row lines `y = origin_y + ty * tile_units + 0.5` crossed by
/// directed edge `p0 -> p1`. Returns `(ty_start, ty_end, dir)`.
#[inline]
fn edge_row_crossing_range(
    p0: Point2,
    p1: Point2,
    origin_y: i64,
    tile_units: i64,
    rows: usize,
) -> Option<(usize, usize, i8)> {
    if p0.y == p1.y {
        return None;
    }
    let (lo_y, hi_y, dir) = if p0.y < p1.y {
        (p0.y, p1.y, 1i8)
    } else {
        (p1.y, p0.y, -1i8)
    };
    let s = tile_units;
    let ty_start = ((lo_y - origin_y + s - 1).div_euclid(s)).max(0);
    let ty_end = ((hi_y - 1 - origin_y).div_euclid(s)).min(rows as i64 - 1);
    if ty_start <= ty_end {
        Some((ty_start as usize, ty_end as usize, dir))
    } else {
        None
    }
}

/// Visit every grid cell `(gx, gy)` whose closed tile rectangle
/// `[origin_x + gx*S, origin_x + (gx+1)*S] × [origin_y + gy*S, origin_y + (gy+1)*S]`
/// touches the segment `p0 -> p1` (with conservative ±1 rounding on row slabs
/// so no touching edge is ever missed, while keeping diagonal edges O(dx + dy)).
#[inline]
#[allow(clippy::too_many_arguments)]
fn for_each_edge_cell<F: FnMut(usize)>(
    p0: Point2,
    p1: Point2,
    origin_x: i64,
    origin_y: i64,
    tile_units: i64,
    cols: usize,
    rows: usize,
    mut visit: F,
) {
    let min_x = p0.x.min(p1.x);
    let max_x = p0.x.max(p1.x);
    let min_y = p0.y.min(p1.y);
    let max_y = p0.y.max(p1.y);

    let s = tile_units;
    let gx_start = ((min_x - origin_x - 1).div_euclid(s)).max(0) as usize;
    let gx_end = ((max_x - origin_x).div_euclid(s)).min(cols as i64 - 1) as usize;
    let gy_start = ((min_y - origin_y - 1).div_euclid(s)).max(0) as usize;
    let gy_end = ((max_y - origin_y).div_euclid(s)).min(rows as i64 - 1) as usize;

    // Fast path for edges spanning <= 2 cells in X or Y (the 99% case).
    if gx_end - gx_start <= 1 || gy_end - gy_start <= 1 {
        for gy in gy_start..=gy_end {
            let row_base = gy * cols;
            for gx in gx_start..=gx_end {
                visit(row_base + gx);
            }
        }
        return;
    }

    // Row-slab supercover for long diagonal edges: O(dx + dy) instead of O(dx * dy).
    let (lo, hi) = if p0.y < p1.y { (p0, p1) } else { (p1, p0) };
    let dx = (hi.x - lo.x) as i128;
    let dy = (hi.y - lo.y) as i128;
    for gy in gy_start..=gy_end {
        let by0 = origin_y + (gy as i64) * s;
        let by1 = by0 + s;
        let ya = lo.y.max(by0);
        let yb = hi.y.min(by1);
        let xa = lo.x + (dx * ((ya - lo.y) as i128)).div_euclid(dy) as i64;
        let xb = lo.x + (dx * ((yb - lo.y) as i128)).div_euclid(dy) as i64;
        let rmin_x = xa.min(xb).max(min_x);
        let rmax_x = (xa.max(xb).saturating_add(1)).min(max_x);
        let rgx0 = ((rmin_x - origin_x - 1).div_euclid(s)).max(0) as usize;
        let rgx1 = ((rmax_x - origin_x).div_euclid(s)).min(cols as i64 - 1) as usize;
        let row_base = gy * cols;
        for gx in rgx0..=rgx1 {
            visit(row_base + gx);
        }
    }
}

pub(crate) fn count_expolygon_points(polys: &[ExPolygon]) -> usize {
    polys
        .iter()
        .map(|p| p.contour.points.len() + p.holes.iter().map(|h| h.points.len()).sum::<usize>())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_sdk::host::{self, ClipOperation};
    use slicer_sdk::ir::mm_to_units;

    fn rect_ccw(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
        Polygon {
            points: vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x1, y: y1 },
                Point2 { x: x0, y: y1 },
            ],
        }
    }

    fn rect_cw(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
        Polygon {
            points: vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x0, y: y1 },
                Point2 { x: x1, y: y1 },
                Point2 { x: x1, y: y0 },
            ],
        }
    }

    fn dense_ngon(cx: f64, cy: f64, r: f64, n: usize, ccw: bool, wobble: f64) -> Polygon {
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let idx = if ccw { i } else { (n - i) % n };
            let angle = (idx as f64 / n as f64) * std::f64::consts::TAU;
            let rad = r * (1.0 + wobble * (angle * 7.0).sin());
            pts.push(Point2 {
                x: (cx + rad * angle.cos()).round() as i64,
                y: (cy + rad * angle.sin()).round() as i64,
            });
        }
        Polygon { points: pts }
    }

    fn subject_bbox(subject: &ExPolygon) -> (i64, i64, i64, i64) {
        let mut min_x = i64::MAX;
        let mut max_x = i64::MIN;
        let mut min_y = i64::MAX;
        let mut max_y = i64::MIN;
        for p in subject
            .contour
            .points
            .iter()
            .chain(subject.holes.iter().flat_map(|h| h.points.iter()))
        {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        (min_x, max_x, min_y, max_y)
    }

    fn assert_window_matches_full(
        collision: &[ExPolygon],
        subject: &ExPolygon,
        tile_units: i64,
        label: &str,
    ) {
        let expected = host::clip_polygons(
            std::slice::from_ref(subject),
            collision,
            ClipOperation::Difference,
        );
        let mut index = CollisionTileIndex::build(collision, tile_units)
            .expect("non-empty collision must build an index");
        let got = match index.window_for_bbox(subject_bbox(subject)) {
            WindowedCollision::Empty => host::clip_polygons(
                std::slice::from_ref(subject),
                &[],
                ClipOperation::Difference,
            ),
            WindowedCollision::Covered => Vec::new(),
            WindowedCollision::Polys(clip) => host::clip_polygons(
                std::slice::from_ref(subject),
                &clip,
                ClipOperation::Difference,
            ),
        };
        assert_eq!(
            got, expected,
            "{label} (tile_units={tile_units}): windowed difference diverged from full difference"
        );
    }

    #[test]
    fn slanted_edge_across_tile_boundary_has_zero_rounding_perturbation() {
        // Nearly horizontal long edge from (0, 0) to (100_000, 1): rectangular
        // clipping at x = 50_000 rounds (50_000, 0.5) to an integer and shifts
        // the edge by up to 0.5 units inside the tile. Our non-perturbing
        // window keeps the exact original segment verbatim.
        let collision = vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: 100_000, y: 1 },
                    Point2 {
                        x: 100_000,
                        y: 20_000,
                    },
                    Point2 { x: 0, y: 20_000 },
                ],
            },
            holes: Vec::new(),
        }];
        let subject = ExPolygon {
            contour: rect_ccw(45_000, -10_000, 55_000, 10_000),
            holes: Vec::new(),
        };
        for tile_units in [10_000, 20_000, 50_000, 100_000] {
            assert_window_matches_full(&collision, &subject, tile_units, "slanted_boundary");
        }
    }

    #[test]
    fn nested_hole_island_and_overlapping_planes_match_full_across_tile_sizes() {
        let outer1 = dense_ngon(0.0, 0.0, 400_000.0, 256, true, 0.12);
        let hole1 = dense_ngon(20_000.0, -10_000.0, 180_000.0, 192, false, 0.15);
        let island1 = dense_ngon(20_000.0, -10_000.0, 60_000.0, 128, true, 0.10);

        // Second overlapping plane (like midpoint + print-plane occupancy).
        let outer2 = dense_ngon(5_000.0, 3_000.0, 405_000.0, 256, true, 0.12);
        let hole2 = dense_ngon(25_000.0, -8_000.0, 175_000.0, 192, false, 0.15);

        let collision = vec![
            ExPolygon {
                contour: outer1,
                holes: vec![hole1],
            },
            ExPolygon {
                contour: island1,
                holes: Vec::new(),
            },
            ExPolygon {
                contour: outer2,
                holes: vec![hole2],
            },
        ];

        // Probe subjects across every topological region:
        // outside, straddling outer edge, deep inside solid (Covered),
        // straddling hole edge, deep inside hole (Empty), straddling island,
        // deep inside island (Covered), and extending beyond grid_bbox.
        let centers: [(f64, f64); 16] = [
            (-480_000.0, 0.0),
            (-400_000.0, 0.0),
            (400_000.0, 15_000.0),
            (0.0, 400_000.0),
            (0.0, -400_000.0),
            (-260_000.0, 100_000.0),
            (260_000.0, -120_000.0),
            (200_000.0, -10_000.0),
            (-150_000.0, -10_000.0),
            (115_000.0, -10_000.0),
            (20_000.0, 110_000.0),
            (20_000.0, -10_000.0),
            (80_000.0, -10_000.0),
            (-40_000.0, -10_000.0),
            (410_000.0, 410_000.0),
            (-410_000.0, -410_000.0),
        ];

        for tile_units in [10_000, 20_000, 50_000, 100_000] {
            let mut idx = CollisionTileIndex::build(&collision, tile_units).unwrap();
            let mut saw_empty = false;
            let mut saw_covered = false;
            let mut saw_polys = false;
            for (ci, &(cx, cy)) in centers.iter().enumerate() {
                let subj = ExPolygon {
                    contour: dense_ngon(cx, cy, 12_000.0, 32, true, 0.0),
                    holes: Vec::new(),
                };
                let win = idx.window_for_bbox(subject_bbox(&subj));
                let got = match win {
                    WindowedCollision::Empty => {
                        saw_empty = true;
                        host::clip_polygons(&[subj.clone()], &[], ClipOperation::Difference)
                    }
                    WindowedCollision::Covered => {
                        saw_covered = true;
                        Vec::new()
                    }
                    WindowedCollision::Polys(ref clip) => {
                        saw_polys = true;
                        assert!(
                            count_expolygon_points(clip) < count_expolygon_points(&collision),
                            "contracted clip set must shrink dense rings (tile={tile_units}, c={ci})"
                        );
                        host::clip_polygons(&[subj.clone()], clip, ClipOperation::Difference)
                    }
                };
                let expected = host::clip_polygons(&[subj], &collision, ClipOperation::Difference);
                assert_eq!(
                    got, expected,
                    "center #{ci} ({cx}, {cy}) tile_units={tile_units} diverged"
                );
            }
            assert!(
                saw_empty,
                "test must cover Empty fast path (tile={tile_units})"
            );
            assert!(
                saw_covered,
                "test must cover Covered fast path (tile={tile_units})"
            );
            assert!(
                saw_polys,
                "test must cover Polys contraction path (tile={tile_units})"
            );
        }
    }

    #[test]
    fn edges_on_exact_tile_grid_lines_are_never_missed() {
        let s = mm_to_units(2.0);
        // Collision edges sit directly on x = 0, x = s, x = 2*s, y = 0, y = s, y = 2*s.
        let collision = vec![ExPolygon {
            contour: rect_ccw(0, 0, 2 * s, 2 * s),
            holes: vec![rect_cw(s / 2, s / 2, s, s)],
        }];
        for x in [0, s / 2, s, 3 * s / 2, 2 * s] {
            for y in [0, s / 2, s, 3 * s / 2, 2 * s] {
                let subj = ExPolygon {
                    contour: rect_ccw(x - 1_000, y - 1_000, x + 1_000, y + 1_000),
                    holes: Vec::new(),
                };
                assert_window_matches_full(&collision, &subj, s, "grid_line_boundary");
            }
        }
    }

    #[test]
    fn long_diagonal_edge_row_slab_supercover_matches_full() {
        let collision = vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 {
                        x: 500_000,
                        y: 400_000,
                    },
                    Point2 {
                        x: 450_000,
                        y: 500_000,
                    },
                    Point2 {
                        x: -50_000,
                        y: 100_000,
                    },
                ],
            },
            holes: Vec::new(),
        }];
        for step in 0..=10 {
            let cx = step * 50_000;
            let cy = step * 40_000;
            let subj = ExPolygon {
                contour: rect_ccw(cx - 8_000, cy - 8_000, cx + 8_000, cy + 8_000),
                holes: Vec::new(),
            };
            assert_window_matches_full(&collision, &subj, 10_000, "long_diagonal");
        }
    }

    #[test]
    fn double_winding_enclosing_rings_and_non_canonical_subject_match_full() {
        // Two overlapping CCW squares enclosing a hole: winding +2 outside
        // hole, +1 inside hole (still non-zero -> Covered inside hole too!).
        let collision = vec![
            ExPolygon {
                contour: rect_ccw(-200_000, -200_000, 200_000, 200_000),
                holes: vec![rect_cw(-80_000, -80_000, 80_000, 80_000)],
            },
            ExPolygon {
                contour: rect_ccw(-210_000, -210_000, 210_000, 210_000),
                holes: Vec::new(),
            },
        ];
        // Inside the hole of polygon 0, winding is +1 - 1 + 1 = +1 (Covered!).
        let inside_hole = ExPolygon {
            contour: rect_ccw(-20_000, -20_000, 20_000, 20_000),
            holes: Vec::new(),
        };
        assert_window_matches_full(&collision, &inside_hole, 20_000, "double_winding_hole");

        // Straddling the hole boundary: w_const = +2, hole ring touches U.
        let straddle = ExPolygon {
            contour: rect_ccw(60_000, -20_000, 100_000, 20_000),
            holes: Vec::new(),
        };
        assert_window_matches_full(&collision, &straddle, 20_000, "double_winding_straddle");
    }
}
