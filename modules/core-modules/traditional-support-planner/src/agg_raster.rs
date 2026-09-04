// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Support/SupportMaterial.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! AGG-equivalent support grid construction and rasterization.
//!
//! Port of canonical `SupportGridPattern`'s `smsGrid`/AGG branch: the
//! constructor arithmetic that derives the sampling grid from the support
//! polygons, the polygon rasterizer, and the (misleadingly named) canonical
//! `dilate_trimming_region`, which is in fact a 3x3 erosion.
//!
//! # Scale translation
//!
//! OrcaSlicer's scaled unit is 1 nm; a Pinch 'n Print unit is 100 nm, so every
//! canonical scaled constant is divided by 100 at this boundary. The three
//! sub-unit constants in the canonical constructor are translated as follows,
//! and each translation is repeated at its use site:
//!
//! * canonical `bbox.offset(20)` -> 20/100 = 0.2 PnP units, which truncates to
//!   0 on `i64`. Rounded **up** to `1` so the epsilon stays meaningful.
//! * canonical `+100` in the oversampling denominator -> exactly `1` PnP unit.
//! * canonical `+21` in the `pixel_size` max-form -> 0.21 PnP units. Rounded
//!   **up** to `1` (whole-unit `i64` arithmetic; see `PIXEL_SIZE_EPSILON`).
//!
//! Rounding up rather than down is the conservative direction for all three:
//! each term only ever enlarges a bound or a pixel, so rounding up can never
//! under-cover the support region.

use slicer_sdk::prelude::*;

/// Canonical `bbox.offset(20)` (orca nm) = 0.2 PnP units; rounded up to 1 so
/// the epsilon survives `i64` truncation.
const BBOX_EPSILON: i64 = 1;
/// Canonical `+100` (orca nm) in the oversampling denominator = 1 PnP unit.
const OVERSAMPLING_EPSILON: i64 = 1;
/// Canonical `+21` (orca nm) in the `pixel_size` max-form = 0.21 PnP units;
/// rounded up to 1 for whole-unit `i64` arithmetic.
const PIXEL_SIZE_EPSILON: i64 = 1;
/// Canonical lower clamp bound on the oversampling factor.
const OVERSAMPLING_MIN: i64 = 1;
/// Canonical upper clamp bound on the oversampling factor.
const OVERSAMPLING_MAX: i64 = 8;

/// Sampling-grid geometry derived from the support polygons.
///
/// Mirrors the state canonical `SupportGridPattern` computes in its constructor
/// before rasterizing. Canonical rotates both polygon sets by `-support_angle`
/// ahead of this arithmetic; Pinch 'n Print has no support-angle config key yet,
/// so this constructor is only ever exercised at angle 0 and takes already
/// oriented polygons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridParams {
    /// Side length of one grid cell, in PnP units (1 unit = 100 nm).
    pub pixel_size: i64,
    /// Lower-left corner of the grid, i.e. canonical `bbox.min`, in PnP units.
    pub origin: Point2,
    /// Final grid dimensions in cells, `(columns, rows)`.
    pub grid_size: (usize, usize),
    /// Unrounded cell count needed to cover the bounding box, `(columns, rows)`.
    /// Retained because canonical asserts `grid_size >= grid_size_raw`.
    pub grid_size_raw: (usize, usize),
    /// Block counts used by the step-4 seed fill, `(columns, rows)`.
    pub blocks: (usize, usize),
    /// Cells per block along each axis; canonical clamps this into `1..=8`.
    pub oversampling: usize,
}

impl GridParams {
    /// Builds grid parameters directly, for tests and for callers that already
    /// know the geometry. Production code uses [`GridParams::from_polygons`].
    pub fn for_test(
        pixel_size: i64,
        origin: Point2,
        grid_size: (usize, usize),
        blocks: (usize, usize),
        oversampling: usize,
    ) -> Self {
        Self {
            pixel_size,
            origin,
            grid_size,
            grid_size_raw: grid_size,
            blocks,
            oversampling,
        }
    }

    /// Number of cells in the grid.
    pub fn cell_count(&self) -> usize {
        self.grid_size.0 * self.grid_size.1
    }

    /// Row stride of a rasterized buffer, in cells.
    pub fn stride(&self) -> usize {
        self.grid_size.0
    }

    /// Ports the canonical `SupportGridPattern` constructor arithmetic.
    ///
    /// `support_spacing_mm` is the support base pattern spacing and
    /// `extrusion_width_mm` the support extrusion width, both in mm.
    pub fn from_polygons(
        support: &[ExPolygon],
        support_spacing_mm: f32,
        extrusion_width_mm: f32,
    ) -> GridParams {
        // Canonical `grid_resolution` is the UNOVERSAMPLED spacing.
        let grid_resolution = mm_to_units(support_spacing_mm).max(1);
        let width_units = mm_to_units(extrusion_width_mm).max(0);

        // Canonical `+100` orca nm == 1 PnP unit (see `OVERSAMPLING_EPSILON`).
        let oversampling = (grid_resolution / (width_units + OVERSAMPLING_EPSILON))
            .clamp(OVERSAMPLING_MIN, OVERSAMPLING_MAX) as usize;

        // Canonical `+21` orca nm == 0.21 PnP units, rounded up to 1.
        let pixel_size = std::cmp::max(
            width_units + PIXEL_SIZE_EPSILON,
            mm_to_units(support_spacing_mm / oversampling as f32),
        )
        .max(1);

        let (mut min, mut max) = extents(support);
        // Canonical `bbox.offset(20)` orca nm == 0.2 PnP units, rounded up to 1.
        min.x -= BBOX_EPSILON;
        min.y -= BBOX_EPSILON;
        max.x += BBOX_EPSILON;
        max.y += BBOX_EPSILON;
        // Canonical `bbox.align_to_grid(grid_resolution)` pulls the minimum
        // corner down onto a multiple of the unoversampled spacing.
        min.x = align_to_grid(min.x, grid_resolution);
        min.y = align_to_grid(min.y, grid_resolution);
        // Canonical `bbox.offset(pixel_size)`: a FULL-PIXEL margin on all four
        // sides. This is what guarantees the empty boundary ring the seed fill
        // relies on -- it is not a one-cell ring by accident.
        min.x -= pixel_size;
        min.y -= pixel_size;
        max.x += pixel_size;
        max.y += pixel_size;

        let grid_size_raw = (
            ceil_div(max.x - min.x, pixel_size) as usize,
            ceil_div(max.y - min.y, pixel_size) as usize,
        );
        // Canonical `(raw + oversampling - 1 - 2) / oversampling`. Saturating
        // subtraction guards degenerate grids smaller than 3 cells, which
        // canonical cannot reach because of the full-pixel margin above.
        let blocks = (
            (grid_size_raw.0 + oversampling).saturating_sub(3) / oversampling,
            (grid_size_raw.1 + oversampling).saturating_sub(3) / oversampling,
        );
        let grid_size = (blocks.0 * oversampling + 2, blocks.1 * oversampling + 2);
        debug_assert!(
            grid_size.0 >= grid_size_raw.0 && grid_size.1 >= grid_size_raw.1,
            "canonical asserts grid_size >= grid_size_raw"
        );

        GridParams {
            pixel_size,
            origin: min,
            grid_size,
            grid_size_raw,
            blocks,
            oversampling,
        }
    }
}

/// Canonical Slic3r `align_to_grid`: the largest multiple of `spacing` that is
/// less than or equal to `v`.
fn align_to_grid(v: i64, spacing: i64) -> i64 {
    v - v.rem_euclid(spacing)
}

/// Ceiling division for non-negative spans.
fn ceil_div(span: i64, divisor: i64) -> i64 {
    if span <= 0 {
        0
    } else {
        (span + divisor - 1) / divisor
    }
}

/// Axis-aligned extents of a polygon set, in PnP units.
fn extents(polys: &[ExPolygon]) -> (Point2, Point2) {
    let mut min = Point2 {
        x: i64::MAX,
        y: i64::MAX,
    };
    let mut max = Point2 {
        x: i64::MIN,
        y: i64::MIN,
    };
    let mut any = false;
    for ex in polys {
        for pt in &ex.contour.points {
            any = true;
            min.x = min.x.min(pt.x);
            min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x);
            max.y = max.y.max(pt.y);
        }
    }
    if !any {
        return (Point2 { x: 0, y: 0 }, Point2 { x: 0, y: 0 });
    }
    (min, max)
}

/// Rasterizes `polys` into the grid described by `params`, returning a
/// row-major `0/1` buffer of `params.cell_count()` cells with row stride
/// `params.stride()`.
///
/// Two canonical properties are preserved exactly:
///
/// * **Nonzero winding.** Canonical never sets AGG's `filling_rule`, so the
///   `fill_non_zero` default applies. Every contour and every hole of every
///   polygon is added as a closed path in a SINGLE rasterization pass, so
///   winding numbers combine and holes come out unset.
/// * **No coverage threshold.** Canonical rasterizes to antialiased gray8 and
///   every consumer tests `!= 0`, so ANY partial coverage counts as set. A cell
///   is therefore marked when the polygon set covers any part of it: cells whose
///   centre lies inside the nonzero-winding region, plus every cell any edge
///   passes through.
pub fn rasterize_polygons(polys: &[ExPolygon], params: &GridParams) -> Vec<u8> {
    let (nx, ny) = params.grid_size;
    let mut grid = vec![0u8; nx * ny];
    if nx == 0 || ny == 0 {
        return grid;
    }
    let px = params.pixel_size as f64;
    let ox = params.origin.x as f64;
    let oy = params.origin.y as f64;
    let to_grid = |p: &Point2| ((p.x as f64 - ox) / px, (p.y as f64 - oy) / px);

    // Every ring of every polygon, in grid space, as one combined path set.
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for ex in polys {
        for ring in std::iter::once(&ex.contour).chain(ex.holes.iter()) {
            if ring.points.len() < 3 {
                continue;
            }
            rings.push(ring.points.iter().map(to_grid).collect());
        }
    }

    // Pass 1: nonzero-winding scanline through cell centres.
    for r in 0..ny {
        let yc = r as f64 + 0.5;
        // (x crossing, winding delta)
        let mut xs: Vec<(f64, i32)> = Vec::new();
        for ring in &rings {
            for i in 0..ring.len() {
                let (x0, y0) = ring[i];
                let (x1, y1) = ring[(i + 1) % ring.len()];
                let dir = if y0 <= yc && y1 > yc {
                    1
                } else if y1 <= yc && y0 > yc {
                    -1
                } else {
                    continue;
                };
                let t = (yc - y0) / (y1 - y0);
                xs.push((x0 + t * (x1 - x0), dir));
            }
        }
        if xs.is_empty() {
            continue;
        }
        xs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut winding = 0;
        for w in 0..xs.len().saturating_sub(1) {
            winding += xs[w].1;
            if winding == 0 {
                continue;
            }
            // Cell centres in [xs[w].0, xs[w + 1].0) are inside.
            let lo = (xs[w].0 - 0.5).ceil().max(0.0);
            let hi = (xs[w + 1].0 - 0.5).ceil().min(nx as f64);
            let mut c = lo as usize;
            while (c as f64) < hi {
                grid[r * nx + c] = 1;
                c += 1;
            }
        }
    }

    // Pass 2: every cell any edge passes through, so partial coverage counts.
    for ring in &rings {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            mark_segment_cells(a, b, nx, ny, &mut grid);
        }
    }
    grid
}

/// Marks every grid cell the segment `a -> b` (grid-space coordinates) passes
/// through, by splitting the segment at each integer grid line and testing the
/// midpoint of every resulting sub-segment.
fn mark_segment_cells(a: (f64, f64), b: (f64, f64), nx: usize, ny: usize, grid: &mut [u8]) {
    let (x0, y0) = a;
    let (x1, y1) = b;
    let mut ts: Vec<f64> = vec![0.0, 1.0];
    if x1 != x0 {
        let (lo, hi) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let mut g = lo.ceil();
        while g <= hi {
            ts.push((g - x0) / (x1 - x0));
            g += 1.0;
        }
    }
    if y1 != y0 {
        let (lo, hi) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        let mut g = lo.ceil();
        while g <= hi {
            ts.push((g - y0) / (y1 - y0));
            g += 1.0;
        }
    }
    ts.retain(|t| (0.0..=1.0).contains(t));
    ts.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
    for w in 0..ts.len().saturating_sub(1) {
        if ts[w + 1] - ts[w] <= 0.0 {
            continue;
        }
        let tm = 0.5 * (ts[w] + ts[w + 1]);
        let mx = x0 + tm * (x1 - x0);
        let my = y0 + tm * (y1 - y0);
        if mx < 0.0 || my < 0.0 {
            continue;
        }
        let (c, r) = (mx.floor() as usize, my.floor() as usize);
        if c < nx && r < ny {
            grid[r * nx + c] = 1;
        }
    }
}

/// Ports canonical `dilate_trimming_region`, which despite its name is an
/// **erosion**: output cell `(c, r)` is set iff ALL NINE cells of the 3x3
/// neighbourhood centred on it are set in `trimming`.
///
/// The outermost row and column are never written and stay unset, matching
/// canonical's `1..size - 1` iteration bounds. Output is stored as `0/1`.
pub fn dilate_trimming_region(trimming: &[u8], params: &GridParams) -> Vec<u8> {
    let (nx, ny) = params.grid_size;
    let mut out = vec![0u8; nx * ny];
    if nx < 3 || ny < 3 || trimming.len() < nx * ny {
        return out;
    }
    for r in 1..ny - 1 {
        for c in 1..nx - 1 {
            let all_set =
                (r - 1..=r + 1).all(|rr| (c - 1..=c + 1).all(|cc| trimming[rr * nx + cc] != 0));
            out[r * nx + c] = u8::from(all_set);
        }
    }
    out
}

// -----------------------------------------------------------------------------
// Step 4: seed fill, contour extraction, island filter.
// -----------------------------------------------------------------------------

/// Canonical `contours_simplified` opens with the in-cell bound
/// `assert(abs(2 * offset_in_grid) < pixel_size - 10)`. Ten orca nm is 0.1 PnP
/// units, which truncates to 0 on `i64`; it is rounded **up** to `1` because
/// the constant is subtracted from the bound, so rounding up makes the bound
/// STRICTER. That is the conservative direction here: a slightly smaller legal
/// offset can never let an expansion leak out of its macro cell.
const IN_CELL_BOUND_EPSILON: i64 = 1;

/// True when `offset_in_grid` satisfies canonical's in-cell bound for a grid of
/// the given `pixel_size`, i.e. the expansion applied at a contour corner can
/// never reach beyond the macro cell that corner came from.
pub fn in_cell_offset_is_valid(offset_in_grid: i64, pixel_size: i64) -> bool {
    2 * offset_in_grid.abs() < pixel_size - IN_CELL_BOUND_EPSILON
}

/// Largest non-negative offset that satisfies [`in_cell_offset_is_valid`].
pub fn max_in_cell_offset(pixel_size: i64) -> i64 {
    let bound = pixel_size - IN_CELL_BOUND_EPSILON;
    if bound <= 1 {
        0
    } else {
        (bound - 1) / 2
    }
}

/// One canonical propagation step: copy "set" from `addr2` into `addr` when the
/// dilated trimming mask is clear at **both** endpoints.
#[inline]
fn seed_step(grid: &mut [u8], mask: &[u8], addr: usize, offset: isize) {
    let addr2 = (addr as isize + offset) as usize;
    if grid[addr2] != 0 && mask[addr] == 0 && mask[addr2] == 0 {
        grid[addr] = 1;
    }
}

/// Ports canonical `seed_fill_block`: exactly TWO block-local passes that close
/// gaps inside each macro block without ever crossing a block boundary.
///
/// `mask` is the DILATED (in fact eroded) trimming region from
/// [`dilate_trimming_region`]; a step is refused when the mask is set at the
/// source **or** the destination. There is no iterate-to-fixpoint: canonical
/// runs a top-to-bottom pass and a bottom-to-top pass, each interleaving a
/// left-to-right and a right-to-left sweep, and stops.
///
/// The horizontal sweeps deliberately stop one cell short of each block edge
/// (`1..size` and `(0..size - 1).rev()`), which is what keeps propagation
/// inside the block; the one-pixel boundary ring the grid was built with is
/// never written.
pub fn seed_fill_block(
    grid: &mut [u8],
    grid_size: (usize, usize),
    mask: &[u8],
    blocks: (usize, usize),
    oversampling: usize,
) {
    let stride = grid_size.0;
    let size = oversampling;
    if size == 0 || stride == 0 {
        return;
    }
    assert_eq!(
        mask.len(),
        grid.len(),
        "seed_fill_block: mask and grid must have the same extent"
    );
    let up = -(stride as isize);
    let down = stride as isize;

    for block_r in 0..blocks.1 {
        for block_c in 0..blocks.0 {
            // Canonical block origin; the `+ 1` terms skip the boundary ring.
            let block_offset = block_c * size + 1 + (block_r * size + 1) * stride;

            // Pass 1: top to bottom.
            for r in 0..size {
                if r > 0 {
                    for c in 0..size {
                        seed_step(grid, mask, block_offset + r * stride + c, up);
                    }
                }
                for c in 1..size {
                    seed_step(grid, mask, block_offset + r * stride + c, -1);
                }
                for c in (0..size - 1).rev() {
                    seed_step(grid, mask, block_offset + r * stride + c, 1);
                }
            }

            // Pass 2: bottom to top.
            for r in (0..size - 1).rev() {
                for c in 0..size {
                    seed_step(grid, mask, block_offset + r * stride + c, down);
                }
                for c in 1..size {
                    seed_step(grid, mask, block_offset + r * stride + c, -1);
                }
                for c in (0..size - 1).rev() {
                    seed_step(grid, mask, block_offset + r * stride + c, 1);
                }
            }
        }
    }
}

/// Ports the `fill_holes` branch of canonical `contours_simplified`: a SINGLE
/// non-iterative pass that reads the unmodified `grid` and writes into a copy.
///
/// A cell is filled when it is horizontally bracketed (both left and right
/// neighbours set) or vertically bracketed (both vertical neighbours set) in
/// the ORIGINAL grid. Reading the original rather than the partially written
/// copy is load-bearing: it is what keeps a two-cell gap unbridged.
///
/// The boundary ring is never written (`1..h - 1`, `1..w - 1`).
pub fn fill_grid_holes(grid: &[u8], grid_size: (usize, usize)) -> Vec<u8> {
    let (w, h) = grid_size;
    let mut out = grid.to_vec();
    if w < 3 || h < 3 || grid.len() < w * h {
        return out;
    }
    for r in 1..h - 1 {
        for c in 1..w - 1 {
            let addr = r * w + c;
            if (grid[addr - 1] != 0 && grid[addr + 1] != 0)
                || (grid[addr - w] != 0 && grid[addr + w] != 0)
            {
                out[addr] = 1;
            }
        }
    }
    out
}

/// One cell-boundary edge in grid coordinates.
#[derive(Clone, Copy, PartialEq, Eq)]
struct GridLine {
    a: (i64, i64),
    b: (i64, i64),
}

/// Ports canonical `contours_simplified`.
///
/// Walks the cell-boundary edges of the set region, chains them into closed
/// loops, rescales them into PnP units, and emits only the CORNERS of each
/// loop, displacing every corner by `offset_in_grid` according to the local
/// turn direction. Collinear points are dropped outright — that is the
/// "simplified" in the canonical name.
///
/// `offset_in_grid` must satisfy [`in_cell_offset_is_valid`]; canonical asserts
/// the same bound at the top of the function. Restricting the expansion to
/// within one macro cell is what stops support from leaking through a thin wall
/// (upstream `fb7b995050`) — an unbounded global polygon offset does not have
/// that property.
///
/// The grid's boundary ring must be unset, which the full-pixel margin in
/// [`GridParams::from_polygons`] guarantees.
///
/// Canonical returns a flat `Polygons` list that its caller feeds to
/// `diff_ex`; this port assembles the loops into `ExPolygon`s directly, because
/// this crate's clipping entry point is polygon-with-holes shaped. Outer loops
/// come out clockwise from the edge walk and are reversed to the CCW contour
/// convention; hole loops come out counter-clockwise and are reversed to CW.
pub fn contours_simplified(
    grid_size: (usize, usize),
    pixel_size: i64,
    left_bottom: Point2,
    grid: &[u8],
    offset_in_grid: i64,
    fill_holes: bool,
) -> Vec<ExPolygon> {
    assert!(
        in_cell_offset_is_valid(offset_in_grid, pixel_size),
        "contours_simplified: offset_in_grid {offset_in_grid} escapes the macro cell \
         (canonical asserts abs(2 * offset) < pixel_size - epsilon, pixel_size {pixel_size})"
    );
    let (w, h) = grid_size;
    if w < 2 || h < 2 || grid.len() < w * h {
        return Vec::new();
    }
    debug_assert!(
        (0..w).all(|c| grid[c] == 0 && grid[(h - 1) * w + c] == 0)
            && (0..h).all(|r| grid[r * w] == 0 && grid[r * w + w - 1] == 0),
        "contours_simplified requires an unset boundary ring"
    );

    let filled;
    let grid: &[u8] = if fill_holes {
        filled = fill_grid_holes(grid, grid_size);
        &filled
    } else {
        grid
    };

    // Cell-boundary edges. Cell (c, r) owns the corners (c, r)..(c + 1, r + 1).
    let mut lines: Vec<GridLine> = Vec::new();
    for r in 1..h {
        for c in 1..w {
            let addr = r * w + c;
            let current = grid[addr] != 0;
            let left = grid[addr - 1] != 0;
            let top = grid[addr - w] != 0;
            let (c, r) = (c as i64, r as i64);
            if left != current {
                lines.push(if left {
                    GridLine { a: (c, r + 1), b: (c, r) }
                } else {
                    GridLine { a: (c, r), b: (c, r + 1) }
                });
            }
            if top != current {
                lines.push(if top {
                    GridLine { a: (c, r), b: (c + 1, r) }
                } else {
                    GridLine { a: (c + 1, r), b: (c, r) }
                });
            }
        }
    }
    if lines.is_empty() {
        return Vec::new();
    }

    // Chain the edges into closed loops via a sorted index of start points.
    let mut starts: Vec<((i64, i64), usize)> =
        lines.iter().enumerate().map(|(i, l)| (l.a, i)).collect();
    starts.sort_unstable();
    let mut processed = vec![false; lines.len()];
    let mut loops: Vec<Vec<(i64, i64)>> = Vec::new();

    for i in 0..lines.len() {
        if processed[i] {
            continue;
        }
        let mut pts: Vec<(i64, i64)> = Vec::new();
        let mut cur = i;
        loop {
            processed[cur] = true;
            pts.push(lines[cur].a);
            let pt = lines[cur].b;
            let v1 = (
                lines[cur].b.0 - lines[cur].a.0,
                lines[cur].b.1 - lines[cur].a.1,
            );
            let lo = starts.partition_point(|e| e.0 < pt);
            let mut next: Option<usize> = None;
            let mut closed = false;
            let mut k = lo;
            while k < starts.len() && starts[k].0 == pt {
                let j = starts[k].1;
                if j == i {
                    closed = true;
                    break;
                }
                if !processed[j] {
                    if next.is_none() {
                        next = Some(j);
                    }
                    // Exact corner touch: two unprocessed candidates start at
                    // this point. Canonical takes the convex right angle, the
                    // one whose turn has a positive cross product.
                    let v2 = (lines[j].b.0 - lines[j].a.0, lines[j].b.1 - lines[j].a.1);
                    if v1.0 * v2.1 - v2.0 * v1.1 > 0 {
                        next = Some(j);
                        break;
                    }
                }
                k += 1;
            }
            if closed {
                break;
            }
            match next {
                Some(j) => cur = j,
                None => break,
            }
        }
        if pts.len() >= 4 {
            loops.push(pts);
        }
    }

    // Rescale, drop collinear points, offset each surviving corner.
    let mut rings: Vec<Vec<Point2>> = Vec::new();
    for lp in &loops {
        let ring = emit_corners(lp, pixel_size, left_bottom, offset_in_grid);
        if ring.len() >= 3 {
            rings.push(ring);
        }
    }
    assemble_expolygons(rings)
}

/// Rescales a grid-space loop into PnP units and emits only its corners,
/// displacing each corner by `offset` per axis according to the turn direction.
///
/// `v = points[j2] - points[j0]` spans the previous and next loop points. Every
/// loop edge is a unit grid segment, so `v` has a zero component exactly when
/// `points[j]` is collinear — those points are dropped.
fn emit_corners(
    pts_grid: &[(i64, i64)],
    pixel_size: i64,
    left_bottom: Point2,
    offset: i64,
) -> Vec<Point2> {
    let n = pts_grid.len();
    let scaled: Vec<Point2> = pts_grid
        .iter()
        .map(|&(x, y)| Point2 {
            x: x * pixel_size + left_bottom.x,
            y: y * pixel_size + left_bottom.y,
        })
        .collect();
    let mut out = Vec::new();
    for j in 0..n {
        let j0 = if j == 0 { n - 1 } else { j - 1 };
        let j2 = (j + 1) % n;
        let vx = scaled[j2].x - scaled[j0].x;
        let vy = scaled[j2].y - scaled[j0].y;
        if vx != 0 && vy != 0 {
            let mut p = scaled[j];
            p.y += if vx < 0 { -offset } else { offset };
            p.x += if vy > 0 { -offset } else { offset };
            out.push(p);
        }
    }
    out
}

/// Twice the signed area of a ring; positive is counter-clockwise.
fn signed_area2(ring: &[Point2]) -> i128 {
    let n = ring.len();
    let mut acc: i128 = 0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        acc += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    acc
}

/// Groups extracted rings into `ExPolygon`s.
///
/// The edge walk emits the boundary of a SET region clockwise and the boundary
/// of an unset region enclosed by set cells counter-clockwise, so the area sign
/// classifies the two. Each hole is attached to the smallest-area contour that
/// contains it.
fn assemble_expolygons(rings: Vec<Point2Ring>) -> Vec<ExPolygon> {
    let mut contours: Vec<(i128, Vec<Point2>)> = Vec::new();
    let mut holes: Vec<Vec<Point2>> = Vec::new();
    for mut ring in rings {
        let area = signed_area2(&ring);
        if area < 0 {
            ring.reverse(); // clockwise outer loop -> CCW contour
            contours.push((-area, ring));
        } else if area > 0 {
            ring.reverse(); // counter-clockwise hole loop -> CW hole
            holes.push(ring);
        }
    }
    let mut out: Vec<ExPolygon> = contours
        .iter()
        .map(|(_, ring)| ExPolygon {
            contour: Polygon { points: ring.clone() },
            holes: Vec::new(),
        })
        .collect();
    for hole in holes {
        let probe = hole[0];
        let mut best: Option<usize> = None;
        for (idx, (area, ring)) in contours.iter().enumerate() {
            if !ring_contains(ring, probe) {
                continue;
            }
            match best {
                Some(b) if contours[b].0 <= *area => {}
                _ => best = Some(idx),
            }
        }
        if let Some(idx) = best {
            out[idx].holes.push(Polygon { points: hole });
        }
    }
    out
}

/// A ring of points in PnP units.
type Point2Ring = Vec<Point2>;

/// Odd-crossing ray test of a single ring.
fn ring_contains(ring: &[Point2], p: Point2) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (ring[j], ring[i]);
        if (a.y > p.y) != (b.y > p.y) {
            let cross = (b.x - a.x) as i128 * (p.y - a.y) as i128
                - (b.y - a.y) as i128 * (p.x - a.x) as i128;
            if (cross > 0) == (b.y > a.y) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Crossing-parity containment against an island's contour AND all its holes,
/// with the parity accumulated across contours exactly as canonical does.
fn island_contains(island: &ExPolygon, p: Point2) -> bool {
    let mut inside = false;
    for ring in std::iter::once(&island.contour).chain(island.holes.iter()) {
        inside ^= ring_contains(&ring.points, p);
    }
    inside
}

/// Axis-aligned extent of an island, contour only.
fn island_bbox(island: &ExPolygon) -> Option<(Point2, Point2)> {
    let pts = &island.contour.points;
    let first = *pts.first()?;
    let mut min = first;
    let mut max = first;
    for p in pts {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    Some((min, max))
}

/// Canonical `SupportGridPattern` in its `smsGrid` / AGG form: the rasterized
/// support region plus the trimming polygons the extraction differences away.
///
/// Built once per layer by the caller, then driven with
/// [`SupportGrid::extract_support`].
pub struct SupportGrid {
    /// Grid geometry the rasters were built with.
    params: GridParams,
    /// Seed-filled support raster, row-major `0/1`, `params.cell_count()` long.
    support: Vec<u8>,
    /// Trimming polygons, retained for the canonical difference in
    /// `extract_support`. Canonical keeps these as `m_trimming_polygons`.
    trimming_polys: Vec<ExPolygon>,
}

impl SupportGrid {
    /// Builds the grid from support and trimming polygons.
    ///
    /// Mirrors the canonical constructor's `smsGrid` branch: derive the grid,
    /// rasterize both polygon sets, erode the trimming raster
    /// ([`dilate_trimming_region`]) into the seed-fill mask, then run
    /// [`seed_fill_block`] over the support raster.
    ///
    /// Canonical rotates both polygon sets by `-support_angle` first and rotates
    /// the extracted islands back by `+support_angle`. Pinch 'n Print has no
    /// support-angle config key, so this port is angle-0 only and performs no
    /// rotation; the polygons are consumed as given.
    pub fn new(
        support: &[ExPolygon],
        trimming: &[ExPolygon],
        support_spacing_mm: f32,
        extrusion_width_mm: f32,
    ) -> Self {
        let params = GridParams::from_polygons(support, support_spacing_mm, extrusion_width_mm);
        let mut support_grid = rasterize_polygons(support, &params);
        let trimming_grid = rasterize_polygons(trimming, &params);
        let mask = dilate_trimming_region(&trimming_grid, &params);
        seed_fill_block(
            &mut support_grid,
            params.grid_size,
            &mask,
            params.blocks,
            params.oversampling,
        );
        Self {
            params,
            support: support_grid,
            trimming_polys: trimming.to_vec(),
        }
    }

    /// Builds a grid from an already-rasterized support buffer, for tests.
    pub fn for_test(params: GridParams, support: Vec<u8>, trimming_polys: Vec<ExPolygon>) -> Self {
        Self {
            params,
            support,
            trimming_polys,
        }
    }

    /// Grid geometry this instance was built with.
    pub fn params(&self) -> &GridParams {
        &self.params
    }

    /// The seed-filled support raster.
    pub fn grid(&self) -> &[u8] {
        &self.support
    }

    /// Ports canonical `SupportGridPattern::extract_support`.
    ///
    /// Extracts the simplified contours at `offset_in_grid`, differences the
    /// trimming polygons away, and keeps only the islands that contain at least
    /// one of `samples`.
    ///
    /// **Sample choice belongs to the caller.** Canonical picks an EXPANDING
    /// sample set (the union of the support polygons) when `offset_in_grid > 0`
    /// and a SHRINKING one (the support polygons intersected with the islands)
    /// otherwise; that decision needs a polygon offset, which is deliberately
    /// not performed here.
    ///
    /// Canonical's `difference_ex` is not available in this crate, so the
    /// difference goes through the host clipper directly.
    pub fn extract_support(
        &self,
        offset_in_grid: i64,
        fill_holes: bool,
        samples: &[Point2],
    ) -> Vec<ExPolygon> {
        let islands = self.extract_islands(offset_in_grid, fill_holes);
        self.filter_islands_by_samples(islands, samples)
    }

    /// Extract the raw islands (contours simplified, then the trimming polygons
    /// differenced away) BEFORE the sample-containment filter.
    ///
    /// Canonical `extract_support` computes the islands first and only then
    /// chooses its sample set, because the shrinking branch samples
    /// `intersection(support_polygons, islands)` — it needs the islands in hand.
    /// The caller generates the samples from these and calls
    /// [`SupportGrid::filter_islands_by_samples`].
    pub fn extract_islands(&self, offset_in_grid: i64, fill_holes: bool) -> Vec<ExPolygon> {
        let simplified = contours_simplified(
            self.params.grid_size,
            self.params.pixel_size,
            self.params.origin,
            &self.support,
            offset_in_grid,
            fill_holes,
        );
        if self.trimming_polys.is_empty() {
            // Difference against nothing is the identity; short-circuited so an
            // empty clip set can never be mistaken for an empty subject.
            simplified
        } else {
            host::clip_polygons(
                &simplified,
                &self.trimming_polys,
                ClipOperation::Difference,
            )
        }
    }

    /// Keep only the islands containing at least one sample point — canonical
    /// `extract_support`'s containment filter, split out so the caller can pick
    /// the sample set from the islands themselves.
    pub fn filter_islands_by_samples(
        &self,
        islands: Vec<ExPolygon>,
        samples: &[Point2],
    ) -> Vec<ExPolygon> {
        // Canonical's lexicographic binary-search prefilter over the sorted
        // samples, bounded by the island bbox expanded by one unit. The bbox is
        // a superset of the island, so the prefilter cannot change the result.
        let mut sorted: Vec<Point2> = samples.to_vec();
        sorted.sort_unstable_by_key(|p| (p.x, p.y));

        islands
            .into_iter()
            .filter(|island| {
                let Some((min, max)) = island_bbox(island) else {
                    return false;
                };
                let (lo_x, hi_x) = (min.x - 1, max.x + 1);
                let (lo_y, hi_y) = (min.y - 1, max.y + 1);
                let start = sorted.partition_point(|p| p.x < lo_x);
                sorted[start..]
                    .iter()
                    .take_while(|p| p.x <= hi_x)
                    .any(|p| p.y >= lo_y && p.y <= hi_y && island_contains(island, *p))
            })
            .collect()
    }
}
