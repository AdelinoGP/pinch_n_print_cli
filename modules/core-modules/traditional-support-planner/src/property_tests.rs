//! Opt-in property tests for this planner's AGG raster kernels and its private
//! territory clip.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_14_pbt-support-modules`):
//!
//! * `M-SUPPORT-RASTER` — real `rasterize_polygons` on well-wound orthogonal
//!   rectangles, U notches, rectangles with a rectangular hole and overlapping
//!   multi-shape sets whose every vertex lies strictly off the pixel-grid
//!   lattice. The expected buffer is enumerated cell by cell with exact
//!   integer interval arithmetic: a cell is set iff its open box meets the
//!   open material interior with positive area. This restricted open-interior
//!   model says nothing about coincident-boundary raster conventions.
//! * `M-SUPPORT-HOLES` — `fill_grid_holes` against the simultaneous relation
//!   on the ORIGINAL grid (set, or both horizontal, or both vertical
//!   neighbours set; outer ring unchanged; tiny/short buffers unchanged). No
//!   general enclosed-hole filling or idempotence is claimed.
//! * `M-SUPPORT-GRID` — `SupportGrid::new` + `grid` on bounded multi-block
//!   scenes, direct `seed_fill_block` on constructed per-block rectangle
//!   subdomains, and `SupportGrid::for_test` + `extract_islands` +
//!   `filter_islands_by_samples` on analytic rectangular cell islands. The
//!   erosion mask is modelled independently; the queue-based flood model is
//!   asserted as an EQUALITY only in blocks whose unmasked cells form one
//!   axis-aligned rectangle (see `check_seed_fill`), and only as an upper
//!   bound elsewhere, because `seed_fill_block` deliberately runs two sweeps.
//! * `M-SUPPORT-TERRITORY` — the private `SupportPlanner::clip_to_territory`
//!   on fresh test-local `SupportAnalysisView`s with exact full-source chains.
//!   Expected cells come from rectangle arithmetic: own territory intersects,
//!   a base region removes the line-width-expanded foreign rectangles. This is
//!   the planner-local application of the PnP (DEV-165) rule, not #11's host
//!   territory proof or canonical routing.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! # Seed-fill subdomain proof
//!
//! Inside one block `seed_fill_block` (pass 1, top to bottom) copies each row
//! from the row above, then sweeps left-to-right and right-to-left, so once a
//! row of an unmasked rectangle `R` holds a set cell the whole row of `R` is
//! set and every later row inherits it; pass 2 (bottom to top) then copies the
//! full rows upward. Hence, when the block's unmasked cells are exactly `R`
//! and `R` holds a seed, both passes together set all of `R` — the same set a
//! four-neighbour flood from the seeds reaches, because `R` is connected.
//! Masked cells are never written and never act as a source.
//!
//! # Literal review
//!
//! `cargo xtask check-literals` and `cargo xtask check-test-quality` are
//! **not** evidence for this file: neither collects `modules/core-modules/*/src`
//! and `LiteralVisitor::visit_item_mod` does not follow an out-of-line `mod`.
//! Watched-type sites here (a watched type is a `pub` struct with at least
//! five named fields under `crates/*/src`):
//!
//! * `SupportAnalysisView` (`crates/slicer-sdk/src/prepass_types.rs`) — the one
//!   literal in `TerritoryCase::view` uses `..SupportAnalysisView::default()`.
//! * `SupportAnalysisGeometryEntry` (`crates/slicer-sdk/src/prepass_types.rs`)
//!   — the one literal in `TerritoryCase::view` carries an `// exhaustive:`
//!   note: every field is the exact territory source identity under test.
//! * `SupportFamilyAssignment` has four fields and is not watched; `GridParams`
//!   and `SupportPlanner` live under `modules/` and are not watched.
//!
//! # Test-quality review (manual)
//!
//! Every control is a mutation over the SAME checker the property uses and
//! asserts the specific `CheckFailure` class. Each property builds its
//! deterministic positive witnesses inside its callback and requires its own
//! per-subcase counters positive after `PropertyRun::check`.

#![allow(missing_docs)]
// The pinned toolchain has `is_multiple_of` only for unsigned integers and no
// `div_ceil` for `i64` (E0658 `int_roundings`), so the remainder forms stay and
// the suggested rewrites are suppressed rather than silently disabled.
#![allow(clippy::manual_is_multiple_of, clippy::manual_div_ceil)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_sdk::prelude::{
    host, ClipOperation, ExPolygon, PaintValue, Point2, Polygon, SupportAnalysisGeometryEntry,
    SupportAnalysisView, SupportFamilyAssignment,
};
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, signed_area2, validate_ring, PointLocation,
};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

use super::agg_raster::{
    dilate_trimming_region, fill_grid_holes, rasterize_polygons, seed_fill_block, GridParams,
    SupportGrid,
};
use super::{RasterizerMode, SupportPlanner};

/// Step ceiling for the shrink probe's transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;
/// Coordinate ceiling the SDK geometry oracles accept.
const ABS_CEILING: i64 = 1_000_000;
/// Runtime ceiling on any configured grid dimension.
const DIM_CEILING: usize = 4_096;
/// Runtime ceiling on any configured cell count.
const CELLS_CEILING: usize = 1 << 20;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared plumbing
// ─────────────────────────────────────────────────────────────────────────────

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn bound_i64(bounds: &BTreeMap<String, u64>, key: &str) -> Result<i64, DomainError> {
    i64::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
}

fn bound_usize(bounds: &BTreeMap<String, u64>, key: &str) -> Result<usize, DomainError> {
    let value = required(bounds, key)?;
    // Keep every bound inside u32 so all later products are checked i64/usize.
    u32::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)?;
    usize::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

fn invalid(message: impl Into<String>) -> DomainError {
    DomainError::InvalidBounds(message.into())
}

fn live_bounds(run: &PropertyRun, keys: &[&str]) -> BTreeMap<String, u64> {
    let mut bounds = BTreeMap::new();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    bounds
}

/// Inclusive `lo..=hi` pick; an empty range is a generator bug and panics (a
/// failure, never a rejection).
fn pick(raw: u32, lo: i64, hi: i64) -> i64 {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    lo + i64::from(raw) % (hi - lo + 1)
}

fn pick_usize(raw: u32, lo: usize, hi: usize) -> usize {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    lo + raw as usize % (hi - lo + 1)
}

fn as_i64(v: usize) -> i64 {
    i64::try_from(v).expect("bounded index fits i64")
}

/// `raw.len()` strictly increasing local coordinates in `[lo, hi]`, none a
/// multiple of `pixel` (i.e. none on a pixel-grid line). For `pixel >= 2` one
/// of any two consecutive integers is off the lattice, so reserving two units
/// per remaining value keeps every later range nonempty.
fn off_grid_increasing(raw: &[u32], lo: i64, hi: i64, pixel: i64) -> Vec<i64> {
    let k = as_i64(raw.len());
    assert!(
        pixel >= 2 && hi - lo >= 2 * k - 1,
        "off-grid range {lo}..={hi} cannot hold {k} values at pixel {pixel}"
    );
    let mut out = Vec::with_capacity(raw.len());
    let mut next = lo;
    for (i, &r) in raw.iter().enumerate() {
        let remaining = k - 1 - as_i64(i);
        let mut v = pick(r, next, hi - 2 * remaining - 1);
        if v.rem_euclid(pixel) == 0 {
            v += 1;
        }
        out.push(v);
        next = v + 1;
    }
    out
}

/// `raw.len()` strictly increasing EVEN values `2 * v`, `v` in `[lo_h, hi_h]`.
fn even_increasing(raw: &[u32], lo_h: i64, hi_h: i64) -> Vec<i64> {
    let k = as_i64(raw.len());
    assert!(hi_h - lo_h >= k - 1, "even range too small");
    let mut out = Vec::with_capacity(raw.len());
    let mut next = lo_h;
    for (i, &r) in raw.iter().enumerate() {
        let v = pick(r, next, hi_h - (k - 1 - as_i64(i)));
        out.push(2 * v);
        next = v + 1;
    }
    out
}

fn pt(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn ccw(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
    Polygon {
        points: vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)],
    }
}

fn cw(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
    Polygon {
        points: vec![pt(x0, y0), pt(x0, y1), pt(x1, y1), pt(x1, y0)],
    }
}

/// Axis-aligned rectangle `(x0, y0, x1, y1)` in canonical units.
type Rect = (i64, i64, i64, i64);

fn rect_poly(r: Rect) -> ExPolygon {
    ExPolygon {
        contour: ccw(r.0, r.1, r.2, r.3),
        holes: Vec::new(),
    }
}

fn ring_bbox(poly: &Polygon) -> Option<Rect> {
    let first = *poly.points.first()?;
    let mut b = (first.x, first.y, first.x, first.y);
    for p in &poly.points {
        b.0 = b.0.min(p.x);
        b.1 = b.1.min(p.y);
        b.2 = b.2.max(p.x);
        b.3 = b.3.max(p.y);
    }
    Some(b)
}

fn locate(p: Point2, region: &[ExPolygon]) -> Result<PointLocation, CheckFailure> {
    point_in_region(p, region).map_err(|e| CheckFailure::Fixture(format!("point oracle: {e}")))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    /// The fixture itself is outside the declared domain (generator bug).
    Fixture(String),
    /// Derived grid parameters contradict the hand-derived frame.
    Params(String),
    /// A buffer has the wrong length.
    Size(String),
    /// A cell differs from the independent model.
    Cell(String),
    /// A cell was set in a block that has no unmasked seed.
    SeedLeak(String),
    /// A gained cell is outside the fixpoint reach of an unproven block.
    Bound(String),
    /// A masked cell changed.
    Masked(String),
    /// A cell outside every block (the outer ring) changed.
    Ring(String),
    /// Extracted island geometry differs from the analytic islands.
    Island(String),
    /// The sample-selected island set is wrong.
    Selection(String),
    /// A territory cell or area differs from the rectangle arithmetic.
    Territory(String),
    /// Output enters the line-width-expanded foreign territory.
    Clearance(String),
    /// A fixture with expected material produced none.
    FalseEmpty(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(m) => write!(f, "fixture outside the domain: {m}"),
            Self::Params(m) => write!(f, "grid params: {m}"),
            Self::Size(m) => write!(f, "size: {m}"),
            Self::Cell(m) => write!(f, "cell: {m}"),
            Self::SeedLeak(m) => write!(f, "seed leak: {m}"),
            Self::Bound(m) => write!(f, "fixpoint bound: {m}"),
            Self::Masked(m) => write!(f, "masked cell: {m}"),
            Self::Ring(m) => write!(f, "outer ring: {m}"),
            Self::Island(m) => write!(f, "island: {m}"),
            Self::Selection(m) => write!(f, "selection: {m}"),
            Self::Territory(m) => write!(f, "territory: {m}"),
            Self::Clearance(m) => write!(f, "clearance: {m}"),
            Self::FalseEmpty(m) => write!(f, "false empty: {m}"),
        }
    }
}

fn fixture<T>(message: impl Into<String>) -> Result<T, CheckFailure> {
    Err(CheckFailure::Fixture(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Axis-aligned shapes and the exact open-interior raster model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CutKind {
    /// A rectangular hole strictly inside the outer rectangle.
    Hole,
    /// An open-to-top notch: the cut reaches the outer top edge.
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cut {
    kind: CutKind,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

/// Material = outer rectangle minus the CLOSED cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shape {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    cut: Option<Cut>,
}

impl Shape {
    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Self {
        Self {
            x0,
            y0,
            x1,
            y1,
            cut: None,
        }
    }

    fn with_cut(x0: i64, y0: i64, x1: i64, y1: i64, cut: Cut) -> Self {
        Self {
            x0,
            y0,
            x1,
            y1,
            cut: Some(cut),
        }
    }

    fn kind(&self) -> Option<CutKind> {
        self.cut.map(|c| c.kind)
    }

    fn expoly(&self) -> ExPolygon {
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1, self.y1);
        match self.cut {
            None => rect_poly((x0, y0, x1, y1)),
            Some(c) if c.kind == CutKind::Hole => ExPolygon {
                contour: ccw(x0, y0, x1, y1),
                holes: vec![cw(c.x0, c.y0, c.x1, c.y1)],
            },
            Some(c) => ExPolygon {
                contour: Polygon {
                    points: vec![
                        pt(x0, y0),
                        pt(x1, y0),
                        pt(x1, y1),
                        pt(c.x1, y1),
                        pt(c.x1, c.y0),
                        pt(c.x0, c.y0),
                        pt(c.x0, y1),
                        pt(x0, y1),
                    ],
                },
                holes: Vec::new(),
            },
        }
    }

    fn well_formed(&self) -> bool {
        if !(self.x0 < self.x1 && self.y0 < self.y1) {
            return false;
        }
        match self.cut {
            None => true,
            Some(c) => {
                self.x0 < c.x0
                    && c.x0 < c.x1
                    && c.x1 < self.x1
                    && self.y0 < c.y0
                    && c.y0 < c.y1
                    && match c.kind {
                        CutKind::Hole => c.y1 < self.y1,
                        CutKind::Notch => c.y1 == self.y1,
                    }
            }
        }
    }

    fn xs(&self) -> Vec<i64> {
        let mut v = vec![self.x0, self.x1];
        if let Some(c) = self.cut {
            v.extend([c.x0, c.x1]);
        }
        v
    }

    fn ys(&self) -> Vec<i64> {
        let mut v = vec![self.y0, self.y1];
        if let Some(c) = self.cut {
            v.extend([c.y0, c.y1]);
        }
        v
    }

    /// The open box `(bx0,bx1) × (by0,by1)` meets the open material interior
    /// with positive area. The intersection `I` of the box and the open outer
    /// rectangle is an open rectangle; `I` minus the closed cut has positive
    /// area iff `I` is not contained in the cut's closure.
    fn meets_open_box(&self, bx0: i64, by0: i64, bx1: i64, by1: i64) -> bool {
        let (ix0, iy0) = (bx0.max(self.x0), by0.max(self.y0));
        let (ix1, iy1) = (bx1.min(self.x1), by1.min(self.y1));
        if ix0 >= ix1 || iy0 >= iy1 {
            return false;
        }
        match self.cut {
            None => true,
            Some(c) => !(ix0 >= c.x0 && ix1 <= c.x1 && iy0 >= c.y0 && iy1 <= c.y1),
        }
    }

    /// The box lies inside the outer rectangle AND inside the closed cut of
    /// the given kind (a hole-/notch-interior cell).
    fn box_in_cut(&self, kind: CutKind, bx0: i64, by0: i64, bx1: i64, by1: i64) -> bool {
        self.cut.is_some_and(|c| {
            c.kind == kind
                && bx0 >= c.x0
                && bx1 <= c.x1
                && by0 >= c.y0
                && by1 <= c.y1
                && bx0 >= self.x0
                && bx1 <= self.x1
                && by0 >= self.y0
                && by1 <= self.y1
        })
    }

    /// A DOUBLED point `(X, Y) = (2x, 2y)` lies strictly inside the material.
    fn contains_doubled(&self, x2: i64, y2: i64) -> bool {
        let inside_outer =
            2 * self.x0 < x2 && x2 < 2 * self.x1 && 2 * self.y0 < y2 && y2 < 2 * self.y1;
        let in_cut = self
            .cut
            .is_some_and(|c| 2 * c.x0 <= x2 && x2 <= 2 * c.x1 && 2 * c.y0 <= y2 && y2 <= 2 * c.y1);
        inside_outer && !in_cut
    }
}

fn cell_edge(origin: i64, k: usize, pixel: i64) -> Result<i64, CheckFailure> {
    i64::try_from(k)
        .ok()
        .and_then(|k| k.checked_mul(pixel))
        .and_then(|v| v.checked_add(origin))
        .ok_or_else(|| CheckFailure::Fixture("cell-edge arithmetic overflow".to_owned()))
}

/// Exact expected raster: cell `(c, r)` is set iff its open box meets the open
/// interior of any shape with positive area (the union of shapes).
fn raster_model(
    shapes: &[Shape],
    pixel: i64,
    origin: Point2,
    size: (usize, usize),
) -> Result<Vec<u8>, CheckFailure> {
    let (w, h) = size;
    let cells = w
        .checked_mul(h)
        .ok_or_else(|| CheckFailure::Fixture("cell count overflow".to_owned()))?;
    let mut out = vec![0_u8; cells];
    for r in 0..h {
        let (by0, by1) = (
            cell_edge(origin.y, r, pixel)?,
            cell_edge(origin.y, r + 1, pixel)?,
        );
        for c in 0..w {
            let (bx0, bx1) = (
                cell_edge(origin.x, c, pixel)?,
                cell_edge(origin.x, c + 1, pixel)?,
            );
            out[r * w + c] = u8::from(shapes.iter().any(|s| s.meets_open_box(bx0, by0, bx1, by1)));
        }
    }
    Ok(out)
}

/// Every vertex coordinate is strictly off the lattice `origin + k * pixel`.
fn off_lattice(shapes: &[Shape], origin: Point2, pixel: i64) -> Result<(), CheckFailure> {
    for s in shapes {
        for x in s.xs() {
            if (x - origin.x).rem_euclid(pixel) == 0 {
                return fixture(format!("x = {x} lies on a pixel-grid line"));
            }
        }
        for y in s.ys() {
            if (y - origin.y).rem_euclid(pixel) == 0 {
                return fixture(format!("y = {y} lies on a pixel-grid line"));
            }
        }
    }
    Ok(())
}

fn validate_shapes(shapes: &[Shape], max_abs: i64) -> Result<(), CheckFailure> {
    for s in shapes {
        if !s.well_formed() {
            return fixture(format!("malformed shape {s:?}"));
        }
        for v in s.xs().into_iter().chain(s.ys()) {
            if v.abs() > max_abs {
                return fixture(format!("coordinate {v} exceeds max_abs_units {max_abs}"));
            }
        }
        let e = s.expoly();
        for ring in std::iter::once(&e.contour).chain(e.holes.iter()) {
            validate_ring(ring).map_err(|err| CheckFailure::Fixture(format!("ring: {err}")))?;
        }
        let area = signed_area2(&e.contour)
            .map_err(|err| CheckFailure::Fixture(format!("area: {err}")))?;
        if area <= 0 {
            return fixture("contour must be counter-clockwise");
        }
        for hole in &e.holes {
            let a = signed_area2(hole).map_err(|err| CheckFailure::Fixture(format!("{err}")))?;
            if a >= 0 {
                return fixture("hole must be clockwise");
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SUPPORT-RASTER
// ─────────────────────────────────────────────────────────────────────────────

const RASTER_ID: &str = "M-SUPPORT-RASTER";
const RASTER_TEST: &str = "property_tests::proptest_support_raster_cells";
const RASTER_KEYS: [&str; 5] = [
    "max_abs_units",
    "max_pixel_units",
    "max_grid_width",
    "max_grid_height",
    "max_cells",
];
/// Smallest generated pixel: a 3-cell span then holds the four strictly
/// increasing off-grid values a hole or notch needs per axis.
const RASTER_MIN_PIXEL: i64 = 4;
const RASTER_MIN_DIM: usize = 3;
const RASTER_WITNESS_PIXEL: i64 = 10;
const RASTER_WITNESS_DIM: usize = 8;
/// The witnesses span `[-40, 80]` units.
const RASTER_WITNESS_ABS: i64 = 80;
const RASTER_RAW: usize = 32;

#[derive(Debug, Clone, Copy)]
struct RasterLimits {
    max_abs: i64,
    max_pixel: i64,
    max_w: usize,
    max_h: usize,
    max_cells: usize,
}

fn raster_limits(bounds: &BTreeMap<String, u64>) -> Result<RasterLimits, DomainError> {
    let l = RasterLimits {
        max_abs: bound_i64(bounds, "max_abs_units")?,
        max_pixel: bound_i64(bounds, "max_pixel_units")?,
        max_w: bound_usize(bounds, "max_grid_width")?,
        max_h: bound_usize(bounds, "max_grid_height")?,
        max_cells: bound_usize(bounds, "max_cells")?,
    };
    if !(RASTER_WITNESS_ABS..=ABS_CEILING).contains(&l.max_abs) {
        return Err(invalid(format!(
            "max_abs_units must be in {RASTER_WITNESS_ABS}..={ABS_CEILING}"
        )));
    }
    if !(RASTER_WITNESS_PIXEL..=l.max_abs).contains(&l.max_pixel) {
        return Err(invalid(
            "max_pixel_units must admit the 10-unit witness and not exceed max_abs_units",
        ));
    }
    for v in [l.max_w, l.max_h] {
        if !(RASTER_WITNESS_DIM..=DIM_CEILING).contains(&v) {
            return Err(invalid(format!(
                "grid dimensions must be in {RASTER_WITNESS_DIM}..={DIM_CEILING}"
            )));
        }
    }
    if !(RASTER_WITNESS_DIM * RASTER_WITNESS_DIM..=CELLS_CEILING).contains(&l.max_cells) {
        return Err(invalid("max_cells must admit the 8x8 witnesses"));
    }
    Ok(l)
}

fn raster_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 80),
        ("max_pixel_units".to_owned(), 10),
        ("max_grid_width".to_owned(), 8),
        ("max_grid_height".to_owned(), 8),
        ("max_cells".to_owned(), 64),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RasterMode {
    Rect,
    Notch,
    Hole,
    Multi,
    NoWork,
}

const RASTER_MODES: [RasterMode; 5] = [
    RasterMode::Rect,
    RasterMode::Notch,
    RasterMode::Hole,
    RasterMode::Multi,
    RasterMode::NoWork,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoWork {
    EmptyRing,
    ShortRing,
    ZeroWidth,
    ZeroHeight,
}

const NO_WORKS: [NoWork; 4] = [
    NoWork::EmptyRing,
    NoWork::ShortRing,
    NoWork::ZeroWidth,
    NoWork::ZeroHeight,
];

#[derive(Debug, Clone, PartialEq)]
struct RasterCase {
    mode: RasterMode,
    no_work: Option<NoWork>,
    pixel: i64,
    origin: Point2,
    size: (usize, usize),
    /// Analytic material; empty for the empty/short-ring modes.
    shapes: Vec<Shape>,
}

impl RasterCase {
    fn params(&self) -> GridParams {
        GridParams::for_test(self.pixel, self.origin, self.size, (0, 0), 1)
    }

    fn polygons(&self) -> Vec<ExPolygon> {
        let o = self.origin;
        let p = self.pixel;
        match self.no_work {
            Some(NoWork::EmptyRing) => vec![ExPolygon {
                contour: Polygon { points: Vec::new() },
                holes: Vec::new(),
            }],
            Some(NoWork::ShortRing) => vec![ExPolygon {
                // Two-point contour and one-point hole: both rings are skipped.
                contour: Polygon {
                    points: vec![pt(o.x + 1, o.y + 1), pt(o.x + 2 * p + 1, o.y + 2 * p + 1)],
                },
                holes: vec![Polygon {
                    points: vec![pt(o.x + p + 1, o.y + 1)],
                }],
            }],
            _ => self.shapes.iter().map(Shape::expoly).collect(),
        }
    }
}

fn validate_raster(case: &RasterCase, l: &RasterLimits) -> Result<(), CheckFailure> {
    if (case.mode == RasterMode::NoWork) != case.no_work.is_some() {
        return fixture("no-work tag disagrees with the mode");
    }
    if !(RASTER_MIN_PIXEL..=l.max_pixel).contains(&case.pixel) {
        return fixture(format!("pixel {} outside the domain", case.pixel));
    }
    let (w, h) = case.size;
    if w > l.max_w || h > l.max_h {
        return fixture(format!("grid {w}x{h} exceeds the grid bounds"));
    }
    let cells = w
        .checked_mul(h)
        .ok_or_else(|| CheckFailure::Fixture("cell overflow".to_owned()))?;
    if cells > l.max_cells {
        return fixture(format!("{cells} cells exceed max_cells"));
    }
    let x_end = cell_edge(case.origin.x, w, case.pixel)?;
    let y_end = cell_edge(case.origin.y, h, case.pixel)?;
    for v in [case.origin.x, case.origin.y, x_end, y_end] {
        if v.abs() > l.max_abs {
            return fixture(format!("grid extent {v} exceeds max_abs_units"));
        }
    }
    validate_shapes(&case.shapes, l.max_abs)?;
    off_lattice(&case.shapes, case.origin, case.pixel)?;
    let kinds: Vec<Option<CutKind>> = case.shapes.iter().map(Shape::kind).collect();
    let ok = match (case.mode, case.no_work) {
        (RasterMode::Rect, _) => kinds == [None] && w > 0 && h > 0,
        (RasterMode::Notch, _) => kinds == [Some(CutKind::Notch)] && w > 0 && h > 0,
        (RasterMode::Hole, _) => kinds == [Some(CutKind::Hole)] && w > 0 && h > 0,
        (RasterMode::Multi, _) => {
            (2..=3).contains(&kinds.len())
                && kinds.iter().all(|k| *k != Some(CutKind::Notch))
                && w > 0
                && h > 0
        }
        (RasterMode::NoWork, Some(NoWork::EmptyRing | NoWork::ShortRing)) => {
            kinds.is_empty() && w > 0 && h > 0
        }
        (RasterMode::NoWork, Some(NoWork::ZeroWidth)) => kinds == [None] && w == 0,
        (RasterMode::NoWork, Some(NoWork::ZeroHeight)) => kinds == [None] && h == 0,
        (RasterMode::NoWork, None) => false,
    };
    if !ok {
        return fixture(format!("{:?} case has the wrong structure", case.mode));
    }
    Ok(())
}

#[derive(Debug, Default)]
struct RasterStats {
    cells: usize,
    set: usize,
    /// Set cells whose centre is NOT strictly inside the material.
    centre_outside: usize,
    /// Unset cells lying wholly inside a hole.
    hole_interior: usize,
    /// Unset cells lying wholly inside a notch.
    notch_interior: usize,
}

/// The one raster checker: whole-buffer comparison against the exact model.
fn check_raster(
    case: &RasterCase,
    l: &RasterLimits,
    observed: &[u8],
) -> Result<RasterStats, CheckFailure> {
    validate_raster(case, l)?;
    let expected = raster_model(&case.shapes, case.pixel, case.origin, case.size)?;
    if observed.len() != expected.len() {
        return Err(CheckFailure::Size(format!(
            "buffer has {} cells, the {}x{} grid has {}",
            observed.len(),
            case.size.0,
            case.size.1,
            expected.len()
        )));
    }
    let (w, h) = case.size;
    let p = case.pixel;
    let mut stats = RasterStats::default();
    for r in 0..h {
        let (by0, by1) = (
            cell_edge(case.origin.y, r, p)?,
            cell_edge(case.origin.y, r + 1, p)?,
        );
        for c in 0..w {
            let (bx0, bx1) = (
                cell_edge(case.origin.x, c, p)?,
                cell_edge(case.origin.x, c + 1, p)?,
            );
            let i = r * w + c;
            if observed[i] > 1 || observed[i] != expected[i] {
                return Err(CheckFailure::Cell(format!(
                    "cell ({c},{r}) box [{bx0},{bx1}]x[{by0},{by1}]: expected {}, observed {}",
                    expected[i], observed[i]
                )));
            }
            stats.cells += 1;
            if expected[i] == 1 {
                stats.set += 1;
                let centre_inside = case
                    .shapes
                    .iter()
                    .any(|s| s.contains_doubled(bx0 + bx1, by0 + by1));
                if !centre_inside {
                    stats.centre_outside += 1;
                }
            } else {
                if case
                    .shapes
                    .iter()
                    .any(|s| s.box_in_cut(CutKind::Hole, bx0, by0, bx1, by1))
                {
                    stats.hole_interior += 1;
                }
                if case
                    .shapes
                    .iter()
                    .any(|s| s.box_in_cut(CutKind::Notch, bx0, by0, bx1, by1))
                {
                    stats.notch_interior += 1;
                }
            }
        }
    }
    Ok(stats)
}

fn run_raster(case: &RasterCase) -> Vec<u8> {
    rasterize_polygons(&case.polygons(), &case.params())
}

fn build_raster_case(
    mode: RasterMode,
    no_work: NoWork,
    raw: [u32; RASTER_RAW],
    l: &RasterLimits,
) -> RasterCase {
    let two_m = 2 * l.max_abs;
    let p = pick(
        raw[0],
        RASTER_MIN_PIXEL,
        l.max_pixel.min(two_m / as_i64(RASTER_MIN_DIM)),
    );
    let fit = usize::try_from(two_m / p).expect("fit");
    let w = pick_usize(
        raw[1],
        RASTER_MIN_DIM,
        l.max_w.min(fit).min(l.max_cells / RASTER_MIN_DIM),
    );
    let h = pick_usize(
        raw[2],
        RASTER_MIN_DIM,
        l.max_h.min(fit).min(l.max_cells / w),
    );
    let (sx, sy) = (as_i64(w) * p, as_i64(h) * p);
    let ox = -l.max_abs + pick(raw[3], 0, two_m - sx);
    let oy = -l.max_abs + pick(raw[4], 0, two_m - sy);
    let shape_at = |kind: Option<CutKind>, r: &[u32]| -> Shape {
        match kind {
            None => {
                let xs = off_grid_increasing(&r[0..2], 1, sx - 1, p);
                let ys = off_grid_increasing(&r[2..4], 1, sy - 1, p);
                Shape::rect(ox + xs[0], oy + ys[0], ox + xs[1], oy + ys[1])
            }
            Some(CutKind::Hole) => {
                let xs = off_grid_increasing(&r[0..4], 1, sx - 1, p);
                let ys = off_grid_increasing(&r[4..8], 1, sy - 1, p);
                Shape::with_cut(
                    ox + xs[0],
                    oy + ys[0],
                    ox + xs[3],
                    oy + ys[3],
                    Cut {
                        kind: CutKind::Hole,
                        x0: ox + xs[1],
                        y0: oy + ys[1],
                        x1: ox + xs[2],
                        y1: oy + ys[2],
                    },
                )
            }
            Some(CutKind::Notch) => {
                let xs = off_grid_increasing(&r[0..4], 1, sx - 1, p);
                let ys = off_grid_increasing(&r[4..7], 1, sy - 1, p);
                Shape::with_cut(
                    ox + xs[0],
                    oy + ys[0],
                    ox + xs[3],
                    oy + ys[2],
                    Cut {
                        kind: CutKind::Notch,
                        x0: ox + xs[1],
                        y0: oy + ys[1],
                        x1: ox + xs[2],
                        y1: oy + ys[2],
                    },
                )
            }
        }
    };
    let no_work = (mode == RasterMode::NoWork).then_some(no_work);
    let shapes = match mode {
        RasterMode::Rect => vec![shape_at(None, &raw[5..13])],
        RasterMode::Notch => vec![shape_at(Some(CutKind::Notch), &raw[5..13])],
        RasterMode::Hole => vec![shape_at(Some(CutKind::Hole), &raw[5..13])],
        RasterMode::Multi => {
            let n = 2 + (raw[29] % 2) as usize;
            (0..n)
                .map(|i| {
                    let kind = ((raw[30] >> i) & 1 == 1).then_some(CutKind::Hole);
                    shape_at(kind, &raw[5 + 8 * i..13 + 8 * i])
                })
                .collect()
        }
        RasterMode::NoWork => match no_work {
            Some(NoWork::ZeroWidth | NoWork::ZeroHeight) => vec![shape_at(None, &raw[5..13])],
            _ => Vec::new(),
        },
    };
    let size = match no_work {
        Some(NoWork::ZeroWidth) => (0, h),
        Some(NoWork::ZeroHeight) => (w, 0),
        _ => (w, h),
    };
    RasterCase {
        mode,
        no_work,
        pixel: p,
        origin: pt(ox, oy),
        size,
        shapes,
    }
}

fn raster_strategy(
    l: &RasterLimits,
    modes: Vec<RasterMode>,
    no_works: Vec<NoWork>,
) -> BoxedStrategy<RasterCase> {
    let l = *l;
    (
        prop::sample::select(modes).no_shrink(),
        prop::sample::select(no_works).no_shrink(),
        any::<[u32; RASTER_RAW]>(),
    )
        .prop_map(move |(mode, kind, raw)| build_raster_case(mode, kind, raw, &l))
        .boxed()
}

/// Pixel 10, origin (0,0), rectangle `[11,12] x [11,19]` units: cell (1,1) is
/// covered but its centre (15,15) is outside.
fn raster_centre_witness() -> RasterCase {
    RasterCase {
        mode: RasterMode::Rect,
        no_work: None,
        pixel: 10,
        origin: pt(0, 0),
        size: (4, 4),
        shapes: vec![Shape::rect(11, 11, 12, 19)],
    }
}

/// Outer `[5,75]²`, hole `[25,55]²` units: cell (3,3) = `[30,40]²` is hole
/// interior; cell (2,2) = `[20,30]²` is partially covered.
fn raster_hole_witness() -> RasterCase {
    RasterCase {
        mode: RasterMode::Hole,
        no_work: None,
        pixel: 10,
        origin: pt(0, 0),
        size: (8, 8),
        shapes: vec![Shape::with_cut(
            5,
            5,
            75,
            75,
            Cut {
                kind: CutKind::Hole,
                x0: 25,
                y0: 25,
                x1: 55,
                y1: 55,
            },
        )],
    }
}

/// Negative origin (-40,-40); outer `[-35,35]²` with the open-to-top notch
/// `[-15,15] x [5,35]`: cell (4,6) = `[0,10] x [20,30]` is notch interior.
fn raster_notch_witness() -> RasterCase {
    RasterCase {
        mode: RasterMode::Notch,
        no_work: None,
        pixel: 10,
        origin: pt(-40, -40),
        size: (8, 8),
        shapes: vec![Shape::with_cut(
            -35,
            -35,
            35,
            35,
            Cut {
                kind: CutKind::Notch,
                x0: -15,
                y0: 5,
                x1: 15,
                y1: 35,
            },
        )],
    }
}

fn raster_no_work_witness(kind: NoWork) -> RasterCase {
    let (size, shapes) = match kind {
        NoWork::EmptyRing | NoWork::ShortRing => ((4, 4), Vec::new()),
        NoWork::ZeroWidth => ((0, 4), vec![Shape::rect(11, 11, 29, 29)]),
        NoWork::ZeroHeight => ((4, 0), vec![Shape::rect(11, 11, 29, 29)]),
    };
    RasterCase {
        mode: RasterMode::NoWork,
        no_work: Some(kind),
        pixel: 10,
        origin: pt(0, 0),
        size,
        shapes,
    }
}

#[derive(Debug, Default)]
struct RasterWitnessStats {
    centre_outside: u64,
    hole_unset: u64,
    negative_notch: u64,
    no_work: u64,
}

fn raster_witness_observations(l: &RasterLimits) -> Result<RasterWitnessStats, String> {
    let mut out = RasterWitnessStats::default();
    let checked = |case: &RasterCase| -> Result<(Vec<u8>, RasterStats), String> {
        let observed = run_raster(case);
        let stats = check_raster(case, l, &observed)
            .map_err(|e| format!("{:?} raster witness: {e}", case.mode))?;
        Ok((observed, stats))
    };

    let (obs, stats) = checked(&raster_centre_witness())?;
    if obs[4 + 1] != 1 || stats.set != 1 || stats.centre_outside != 1 {
        return Err("centre-outside cell (1,1) was not the single set cell".to_owned());
    }
    out.centre_outside += 1;

    let (obs, stats) = checked(&raster_hole_witness())?;
    if obs[3 * 8 + 3] != 0 || obs[4 * 8 + 4] != 0 || obs[2 * 8 + 2] != 1 || stats.hole_interior == 0
    {
        return Err("hole witness lost its unset interior or partial cell".to_owned());
    }
    out.hole_unset += stats.hole_interior as u64;

    let (obs, stats) = checked(&raster_notch_witness())?;
    if obs[6 * 8 + 4] != 0 || obs[0] != 1 || stats.notch_interior == 0 {
        return Err("negative-origin notch witness lost its notch cell".to_owned());
    }
    out.negative_notch += stats.notch_interior as u64;

    for kind in NO_WORKS {
        let case = raster_no_work_witness(kind);
        let (obs, _) = checked(&case)?;
        if obs.len() != case.size.0 * case.size.1 || obs.iter().any(|&v| v != 0) {
            return Err(format!("{kind:?} produced work"));
        }
        out.no_work += 1;
    }
    Ok(out)
}

#[test]
fn proptest_support_raster_cells() {
    let run = PropertyRun::new(RASTER_ID, RASTER_TEST).expect("M-SUPPORT-RASTER context");
    let limits = raster_limits(&live_bounds(&run, &RASTER_KEYS)).expect("raster bounds");
    let strategy = raster_strategy(&limits, RASTER_MODES.to_vec(), NO_WORKS.to_vec());

    let calls = Cell::new(0_u64);
    let w_centre = Cell::new(0_u64);
    let w_hole = Cell::new(0_u64);
    let w_notch = Cell::new(0_u64);
    let w_no_work = Cell::new(0_u64);
    let cells = Cell::new(0_u64);
    let set = Cell::new(0_u64);
    let centre_outside = Cell::new(0_u64);
    let hole_interior = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 5] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let w = match raster_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_centre.set(w_centre.get() + w.centre_outside);
        w_hole.set(w_hole.get() + w.hole_unset);
        w_notch.set(w_notch.get() + w.negative_notch);
        w_no_work.set(w_no_work.get() + w.no_work);

        let observed = run_raster(&case);
        let stats = match check_raster(&case, &limits, &observed) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{:?} raster case: {e}", case.mode)),
        };
        match case.mode {
            RasterMode::NoWork => {
                if observed.iter().any(|&v| v != 0) {
                    return fail("no-work case produced set cells");
                }
            }
            _ => {
                if stats.set == 0 {
                    return fail(format!("{:?} case set no cells", case.mode));
                }
            }
        }
        let slot = RASTER_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        cells.set(cells.get() + stats.cells as u64);
        set.set(set.get() + stats.set as u64);
        centre_outside.set(centre_outside.get() + stats.centre_outside as u64);
        hole_interior.set(hole_interior.get() + stats.hole_interior as u64);
        Ok(())
    })
    .expect("M-SUPPORT-RASTER whole-buffer equality with the exact coverage model");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_centre.get() > 0, "centre-outside witness unobserved");
    assert!(w_hole.get() > 0, "hole-interior witness unobserved");
    assert!(
        w_notch.get() > 0,
        "negative-origin notch witness unobserved"
    );
    assert!(w_no_work.get() > 0, "no-work witnesses unobserved");
    assert!(set.get() > 0, "no generated set cell was compared");
    println!(
        "M-SUPPORT-RASTER accepted_calls={} rect={} notch={} hole={} multi={} no_work={} \
         cells={} set={} centre_outside={} hole_interior={} witness_centre={} \
         witness_hole={} witness_notch={} witness_no_work={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        per_mode[4].get(),
        cells.get(),
        set.get(),
        centre_outside.get(),
        hole_interior.get(),
        w_centre.get(),
        w_hole.get(),
        w_notch.get(),
        w_no_work.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SUPPORT-HOLES
// ─────────────────────────────────────────────────────────────────────────────

const HOLES_ID: &str = "M-SUPPORT-HOLES";
const HOLES_TEST: &str = "property_tests::proptest_support_hole_fill";
const HOLES_KEYS: [&str; 3] = ["max_grid_width", "max_grid_height", "max_cells"];

#[derive(Debug, Clone, Copy)]
struct HoleLimits {
    max_w: usize,
    max_h: usize,
    max_cells: usize,
}

fn hole_limits(bounds: &BTreeMap<String, u64>) -> Result<HoleLimits, DomainError> {
    let l = HoleLimits {
        max_w: bound_usize(bounds, "max_grid_width")?,
        max_h: bound_usize(bounds, "max_grid_height")?,
        max_cells: bound_usize(bounds, "max_cells")?,
    };
    // The 6x3 two-gap witness and the 5x5 cascade witness.
    if !(6..=DIM_CEILING).contains(&l.max_w) || !(5..=DIM_CEILING).contains(&l.max_h) {
        return Err(invalid(
            "max_grid_width >= 6 and max_grid_height >= 5 admit the witnesses",
        ));
    }
    if !(25..=CELLS_CEILING).contains(&l.max_cells) {
        return Err(invalid("max_cells must admit the 5x5 witness"));
    }
    Ok(l)
}

fn hole_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_grid_width".to_owned(), 6),
        ("max_grid_height".to_owned(), 5),
        ("max_cells".to_owned(), 25),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoleMode {
    Grid,
    Tiny,
    Short,
}

const HOLE_MODES: [HoleMode; 3] = [HoleMode::Grid, HoleMode::Tiny, HoleMode::Short];

#[derive(Debug, Clone, PartialEq)]
struct HoleCase {
    mode: HoleMode,
    size: (usize, usize),
    grid: Vec<u8>,
}

/// The simultaneous relation, written per cell on the ORIGINAL grid.
fn hole_model(grid: &[u8], (w, h): (usize, usize)) -> Vec<u8> {
    let Some(cells) = w.checked_mul(h) else {
        return grid.to_vec();
    };
    if w < 3 || h < 3 || grid.len() < cells {
        return grid.to_vec();
    }
    let orig = |c: usize, r: usize| grid[r * w + c] != 0;
    let mut out = grid.to_vec();
    for r in 0..h {
        for c in 0..w {
            let ring = c == 0 || r == 0 || c == w - 1 || r == h - 1;
            if ring || orig(c, r) {
                continue;
            }
            let horizontal = orig(c - 1, r) && orig(c + 1, r);
            let vertical = orig(c, r - 1) && orig(c, r + 1);
            if horizontal || vertical {
                out[r * w + c] = 1;
            }
        }
    }
    out
}

#[derive(Debug, Default)]
struct HoleStats {
    cells: usize,
    filled: usize,
    horizontal: usize,
    vertical: usize,
}

fn check_holes(
    case: &HoleCase,
    l: &HoleLimits,
    observed: &[u8],
) -> Result<HoleStats, CheckFailure> {
    let (w, h) = case.size;
    let cells = w
        .checked_mul(h)
        .ok_or_else(|| CheckFailure::Fixture("cell overflow".to_owned()))?;
    if w > l.max_w || h > l.max_h || cells > l.max_cells {
        return fixture(format!("{w}x{h} exceeds the hole bounds"));
    }
    if case.grid.iter().any(|&v| v > 1) {
        return fixture("input is not a 0/1 grid");
    }
    let ok = match case.mode {
        HoleMode::Grid => w >= 3 && h >= 3 && case.grid.len() == cells,
        HoleMode::Tiny => (w < 3 || h < 3) && case.grid.len() == cells,
        HoleMode::Short => w >= 3 && h >= 3 && case.grid.len() < cells,
    };
    if !ok {
        return fixture(format!("{:?} case has the wrong shape", case.mode));
    }
    let expected = hole_model(&case.grid, case.size);
    if observed.len() != expected.len() {
        return Err(CheckFailure::Size(format!(
            "output has {} cells, input {}",
            observed.len(),
            expected.len()
        )));
    }
    let mut stats = HoleStats::default();
    for (i, (&e, &o)) in expected.iter().zip(observed).enumerate() {
        if e != o {
            let (c, r) = if w == 0 { (i, 0) } else { (i % w, i / w) };
            return Err(CheckFailure::Cell(format!(
                "cell ({c},{r}): expected {e}, observed {o} (input {})",
                case.grid[i]
            )));
        }
        stats.cells += 1;
        if e == 1 && case.grid[i] == 0 {
            stats.filled += 1;
            let (c, r) = (i % w, i / w);
            if case.grid[i - 1] != 0 && case.grid[i + 1] != 0 {
                stats.horizontal += 1;
            }
            if case.grid[(r - 1) * w + c] != 0 && case.grid[(r + 1) * w + c] != 0 {
                stats.vertical += 1;
            }
        }
    }
    Ok(stats)
}

fn build_hole_case(mode: HoleMode, raw: [u32; 4], bits: &[bool], l: &HoleLimits) -> HoleCase {
    let to_u8 = |n: usize| -> Vec<u8> { bits[..n].iter().map(|&b| u8::from(b)).collect() };
    let grid_dims = || {
        let w = pick_usize(raw[0], 3, l.max_w.min(l.max_cells / 3));
        let h = pick_usize(raw[1], 3, l.max_h.min(l.max_cells / w));
        (w, h)
    };
    match mode {
        HoleMode::Grid => {
            let (w, h) = grid_dims();
            HoleCase {
                mode,
                size: (w, h),
                grid: to_u8(w * h),
            }
        }
        HoleMode::Tiny => {
            let (w, h) = if raw[2] % 2 == 0 {
                let w = pick_usize(raw[0], 0, 2);
                (
                    w,
                    pick_usize(raw[1], 0, l.max_h.min(l.max_cells / w.max(1))),
                )
            } else {
                let h = pick_usize(raw[1], 0, 2);
                (
                    pick_usize(raw[0], 0, l.max_w.min(l.max_cells / h.max(1))),
                    h,
                )
            };
            HoleCase {
                mode,
                size: (w, h),
                grid: to_u8(w * h),
            }
        }
        HoleMode::Short => {
            let (w, h) = grid_dims();
            let len = pick_usize(raw[3], 0, w * h - 1);
            HoleCase {
                mode,
                size: (w, h),
                grid: to_u8(len),
            }
        }
    }
}

fn hole_strategy(l: &HoleLimits, modes: Vec<HoleMode>) -> BoxedStrategy<HoleCase> {
    let l = *l;
    (
        prop::sample::select(modes).no_shrink(),
        any::<[u32; 4]>(),
        prop::collection::vec(any::<bool>(), l.max_cells),
    )
        .prop_map(move |(mode, raw, bits)| build_hole_case(mode, raw, &bits, &l))
        .boxed()
}

fn hole_grid(w: usize, h: usize, set: &[(usize, usize)]) -> HoleCase {
    let mut grid = vec![0_u8; w * h];
    for &(c, r) in set {
        grid[r * w + c] = 1;
    }
    HoleCase {
        mode: HoleMode::Grid,
        size: (w, h),
        grid,
    }
}

/// Charter cascade witness: 5x5, set `(1,1), (3,2), (1,3)`.
fn hole_cascade_witness() -> HoleCase {
    hole_grid(5, 5, &[(1, 1), (3, 2), (1, 3)])
}

/// Two-cell horizontal gap `(2,1), (3,1)` between `(1,1)` and `(4,1)`.
fn hole_two_gap_witness() -> HoleCase {
    hole_grid(6, 3, &[(1, 1), (4, 1)])
}

/// One-cell gap `(2,1)` between `(1,1)` and `(3,1)`: filled.
fn hole_single_gap_witness() -> HoleCase {
    hole_grid(5, 3, &[(1, 1), (3, 1)])
}

fn hole_tiny_witness() -> HoleCase {
    HoleCase {
        mode: HoleMode::Tiny,
        size: (2, 4),
        grid: vec![1; 8],
    }
}

fn hole_short_witness() -> HoleCase {
    HoleCase {
        mode: HoleMode::Short,
        size: (5, 5),
        grid: vec![1, 1, 1, 1, 1, 1, 0, 1, 0, 1],
    }
}

#[derive(Debug, Default)]
struct HoleWitnessStats {
    cascade: u64,
    two_gap: u64,
    single_gap: u64,
    unchanged: u64,
}

fn hole_witness_observations(l: &HoleLimits) -> Result<HoleWitnessStats, String> {
    let mut out = HoleWitnessStats::default();
    let checked = |case: &HoleCase| -> Result<(Vec<u8>, HoleStats), String> {
        let observed = fill_grid_holes(&case.grid, case.size);
        let stats = check_holes(case, l, &observed).map_err(|e| format!("hole witness: {e}"))?;
        Ok((observed, stats))
    };
    let (obs, stats) = checked(&hole_cascade_witness())?;
    if obs[2 * 5 + 1] != 1 || obs[2 * 5 + 2] != 0 || stats.filled != 1 {
        return Err("cascade witness: (1,2) must fill and (2,2) must stay unset".to_owned());
    }
    out.cascade += 1;
    let case = hole_two_gap_witness();
    let (obs, stats) = checked(&case)?;
    if obs != case.grid || stats.filled != 0 {
        return Err("two-cell gap was bridged".to_owned());
    }
    out.two_gap += 1;
    let (obs, stats) = checked(&hole_single_gap_witness())?;
    if obs[5 + 2] != 1 || stats.horizontal != 1 {
        return Err("single-cell gap was not filled".to_owned());
    }
    out.single_gap += 1;
    for case in [hole_tiny_witness(), hole_short_witness()] {
        let (obs, _) = checked(&case)?;
        if obs != case.grid {
            return Err(format!("{:?} input changed", case.mode));
        }
        out.unchanged += 1;
    }
    Ok(out)
}

#[test]
fn proptest_support_hole_fill() {
    let run = PropertyRun::new(HOLES_ID, HOLES_TEST).expect("M-SUPPORT-HOLES context");
    let limits = hole_limits(&live_bounds(&run, &HOLES_KEYS)).expect("hole bounds");
    let strategy = hole_strategy(&limits, HOLE_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let w_cascade = Cell::new(0_u64);
    let w_two_gap = Cell::new(0_u64);
    let w_single = Cell::new(0_u64);
    let w_unchanged = Cell::new(0_u64);
    let cells = Cell::new(0_u64);
    let filled = Cell::new(0_u64);
    let horizontal = Cell::new(0_u64);
    let vertical = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 3] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let w = match hole_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_cascade.set(w_cascade.get() + w.cascade);
        w_two_gap.set(w_two_gap.get() + w.two_gap);
        w_single.set(w_single.get() + w.single_gap);
        w_unchanged.set(w_unchanged.get() + w.unchanged);

        let observed = fill_grid_holes(&case.grid, case.size);
        let stats = match check_holes(&case, &limits, &observed) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{:?} hole case: {e}", case.mode)),
        };
        if case.mode != HoleMode::Grid && observed != case.grid {
            return fail("tiny/short input changed");
        }
        let slot = HOLE_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        cells.set(cells.get() + stats.cells as u64);
        filled.set(filled.get() + stats.filled as u64);
        horizontal.set(horizontal.get() + stats.horizontal as u64);
        vertical.set(vertical.get() + stats.vertical as u64);
        Ok(())
    })
    .expect("M-SUPPORT-HOLES equality with the simultaneous original-grid relation");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_cascade.get() > 0, "cascade witness unobserved");
    assert!(w_two_gap.get() > 0, "two-gap witness unobserved");
    assert!(w_single.get() > 0, "single-gap witness unobserved");
    assert!(w_unchanged.get() > 0, "tiny/short witnesses unobserved");
    println!(
        "M-SUPPORT-HOLES accepted_calls={} grid={} tiny={} short={} cells={} filled={} \
         horizontal={} vertical={} witness_cascade={} witness_two_gap={} \
         witness_single_gap={} witness_unchanged={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        cells.get(),
        filled.get(),
        horizontal.get(),
        vertical.get(),
        w_cascade.get(),
        w_two_gap.get(),
        w_single.get(),
        w_unchanged.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SUPPORT-GRID
// ─────────────────────────────────────────────────────────────────────────────

const GRID_ID: &str = "M-SUPPORT-GRID";
const GRID_TEST: &str = "property_tests::proptest_support_grid_extraction";
const GRID_KEYS: [&str; 6] = [
    "max_abs_units",
    "max_pixel_units",
    "max_grid_width",
    "max_grid_height",
    "max_blocks",
    "max_cells",
];
const GRID_RAW: usize = 32;
/// The halo witness reaches 40000 units.
const GRID_MIN_ABS: i64 = 40_000;
/// The island generator needs three 4-wide islands with 2-cell gaps.
const GRID_MIN_W: usize = 18;
/// The island generator needs a 4-tall holed island plus the ring.
const GRID_MIN_H: usize = 6;

/// A `SupportGrid::new` configuration with its hand-derived frame. With
/// `res = spacing` in units, canonical `oversampling = res / (width + 1)`
/// (clamped to 1..=8) and `pixel = max(width + 1, spacing / oversampling)`.
/// Every combo has `res = oversampling * pixel`, so the grid origin (aligned
/// to `res` minus one pixel) is a multiple of `pixel`, and every macro block
/// spans exactly one `res`-aligned square.
#[derive(Debug, Clone, Copy)]
struct Combo {
    spacing_mm: f32,
    width_mm: f32,
    pixel: i64,
    os: usize,
    res: i64,
}

const COMBOS: [Combo; 3] = [
    // 20000 / 4001 = 4; max(4001, 5000) = 5000.
    Combo {
        spacing_mm: 2.0,
        width_mm: 0.4,
        pixel: 5_000,
        os: 4,
        res: 20_000,
    },
    // 10000 / 2001 = 4; max(2001, 2500) = 2500.
    Combo {
        spacing_mm: 1.0,
        width_mm: 0.2,
        pixel: 2_500,
        os: 4,
        res: 10_000,
    },
    // 10000 / 4001 = 2; max(4001, 5000) = 5000.
    Combo {
        spacing_mm: 1.0,
        width_mm: 0.4,
        pixel: 5_000,
        os: 2,
        res: 10_000,
    },
];

#[derive(Debug, Clone, Copy)]
struct GridLimits {
    max_abs: i64,
    max_pixel: i64,
    max_w: usize,
    max_h: usize,
    max_blocks: usize,
    max_cells: usize,
}

/// Block-index range `k` with `k*res - margin >= -max_abs` and
/// `(k+1)*res + margin <= max_abs`, returned as `(k_lo, k_hi_plus_one)`.
fn block_index_range(max_abs: i64, res: i64, margin: i64) -> (i64, i64) {
    let k_lo = -((max_abs - margin).div_euclid(res));
    let k_end = (max_abs - margin).div_euclid(res);
    (k_lo, k_end)
}

fn grid_limits(bounds: &BTreeMap<String, u64>) -> Result<GridLimits, DomainError> {
    let l = GridLimits {
        max_abs: bound_i64(bounds, "max_abs_units")?,
        max_pixel: bound_i64(bounds, "max_pixel_units")?,
        max_w: bound_usize(bounds, "max_grid_width")?,
        max_h: bound_usize(bounds, "max_grid_height")?,
        max_blocks: bound_usize(bounds, "max_blocks")?,
        max_cells: bound_usize(bounds, "max_cells")?,
    };
    if !(GRID_MIN_ABS..=ABS_CEILING).contains(&l.max_abs) {
        return Err(invalid(format!(
            "max_abs_units must be in {GRID_MIN_ABS}..={ABS_CEILING}"
        )));
    }
    let combo_pixel = COMBOS.iter().map(|c| c.pixel).max().unwrap_or(0);
    if !(combo_pixel..=l.max_abs).contains(&l.max_pixel) {
        return Err(invalid(format!(
            "max_pixel_units must admit the {combo_pixel}-unit scene pixel"
        )));
    }
    if !(GRID_MIN_W..=DIM_CEILING).contains(&l.max_w)
        || !(GRID_MIN_H..=DIM_CEILING).contains(&l.max_h)
    {
        return Err(invalid(format!(
            "max_grid_width >= {GRID_MIN_W} and max_grid_height >= {GRID_MIN_H}"
        )));
    }
    if !(2..=DIM_CEILING).contains(&l.max_blocks) {
        return Err(invalid("max_blocks >= 2 admits the two-block control"));
    }
    if l.max_cells > CELLS_CEILING || l.max_cells < l.max_w * GRID_MIN_H {
        return Err(invalid(
            "max_cells must admit a max-width island grid of six rows",
        ));
    }
    for c in &COMBOS {
        let margin = 2 * c.pixel + 1;
        let (k_lo, k_end) = block_index_range(l.max_abs, c.res, margin);
        if k_end - k_lo < 1 || (c.os + 2) * (c.os + 2) > l.max_cells {
            return Err(invalid(format!(
                "max_abs_units/max_cells cannot hold one {}-unit block",
                c.res
            )));
        }
    }
    Ok(l)
}

fn grid_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 40_000),
        ("max_pixel_units".to_owned(), 5_000),
        ("max_grid_width".to_owned(), 18),
        ("max_grid_height".to_owned(), 8),
        ("max_blocks".to_owned(), 2),
        ("max_cells".to_owned(), 144),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridMode {
    SceneClear,
    SceneTrim,
    SeedRect,
    Islands,
}

const GRID_MODES: [GridMode; 4] = [
    GridMode::SceneClear,
    GridMode::SceneTrim,
    GridMode::SeedRect,
    GridMode::Islands,
];

#[derive(Debug, Clone, PartialEq)]
struct SceneCase {
    combo: usize,
    support: Vec<Shape>,
    trim: Vec<Shape>,
}

impl SceneCase {
    fn combo(&self) -> Combo {
        COMBOS[self.combo]
    }

    fn build(&self) -> SupportGrid {
        let c = self.combo();
        let support: Vec<ExPolygon> = self.support.iter().map(Shape::expoly).collect();
        let trim: Vec<ExPolygon> = self.trim.iter().map(Shape::expoly).collect();
        SupportGrid::new(&support, &trim, c.spacing_mm, c.width_mm)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SeedCase {
    size: (usize, usize),
    blocks: (usize, usize),
    os: usize,
    input: Vec<u8>,
    mask: Vec<u8>,
}

impl SeedCase {
    fn run(&self) -> Vec<u8> {
        let mut grid = self.input.clone();
        seed_fill_block(&mut grid, self.size, &self.mask, self.blocks, self.os);
        grid
    }
}

#[derive(Debug, Clone, PartialEq)]
struct IslandDef {
    c0: usize,
    r0: usize,
    c1: usize,
    r1: usize,
    /// The full interior `[c0+1, c1-1] x [r0+1, r1-1]` is a hole.
    hole: bool,
    /// Trimming polygon `[trim_x, x1e] x [y0e, y1e]` cuts the right part.
    trim_x: Option<i64>,
    selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct IslandCase {
    pixel: i64,
    origin: Point2,
    size: (usize, usize),
    offset: i64,
    fill_holes: bool,
    islands: Vec<IslandDef>,
    decoys: bool,
    no_samples: bool,
}

#[derive(Debug, Clone)]
struct IslandSpec {
    outer: Rect,
    hole: Option<Rect>,
    inside: Vec<Point2>,
    outside: Vec<Point2>,
    sample: Point2,
}

/// Largest `o >= 0` with `2 * o < pixel - 1` (the source's strict in-cell
/// bound), derived here, not read from production.
fn legal_offset_max(pixel: i64) -> i64 {
    if pixel < 2 {
        -1
    } else {
        (pixel - 2) / 2
    }
}

impl IslandCase {
    fn params(&self) -> GridParams {
        GridParams::for_test(self.pixel, self.origin, self.size, (0, 0), 1)
    }

    fn buffer(&self) -> Vec<u8> {
        let (w, h) = self.size;
        let mut g = vec![0_u8; w * h];
        for d in &self.islands {
            for r in d.r0..=d.r1 {
                for c in d.c0..=d.c1 {
                    let interior = c > d.c0 && c < d.c1 && r > d.r0 && r < d.r1;
                    if !(d.hole && interior) {
                        g[r * w + c] = 1;
                    }
                }
            }
        }
        g
    }

    fn x(&self, c: usize) -> i64 {
        self.origin.x + as_i64(c) * self.pixel
    }

    fn y(&self, r: usize) -> i64 {
        self.origin.y + as_i64(r) * self.pixel
    }

    /// Untrimmed expanded outer rectangle: grid corners moved outward by the
    /// offset (inward for a negative offset).
    fn outer_untrimmed(&self, d: &IslandDef) -> Rect {
        let o = self.offset;
        (
            self.x(d.c0) - o,
            self.y(d.r0) - o,
            self.x(d.c1 + 1) + o,
            self.y(d.r1 + 1) + o,
        )
    }

    fn trimming(&self) -> Vec<ExPolygon> {
        self.islands
            .iter()
            .filter_map(|d| {
                let tx = d.trim_x?;
                let (_, y0, x1, y1) = self.outer_untrimmed(d);
                Some(rect_poly((tx, y0, x1, y1)))
            })
            .collect()
    }

    fn spec(&self, d: &IslandDef) -> IslandSpec {
        let (p, o) = (self.pixel, self.offset);
        let (x0e, y0e, x1e, y1e) = self.outer_untrimmed(d);
        let x_right = d.trim_x.unwrap_or(x1e);
        // The hole shrinks by the offset as the material grows.
        let hole = d.hole.then(|| {
            (
                self.x(d.c0 + 1) + o,
                self.y(d.r0 + 1) + o,
                self.x(d.c1) - o,
                self.y(d.r1) - o,
            )
        });
        let solid_right = hole.map_or(x_right, |h| h.0);
        let solid_top = hole.map_or(y1e, |h| h.1);
        let xs = (x0e + solid_right).div_euclid(2);
        let ys = (y0e + solid_top).div_euclid(2);
        let mut inside = vec![
            pt(xs, ys),
            pt(x0e + 1, ys),
            pt(x_right - 1, ys),
            pt(xs, y0e + 1),
            pt(xs, y1e - 1),
        ];
        let mut outside = vec![
            pt(x0e - 1, ys),
            pt(x_right + 1, ys),
            pt(xs, y0e - 1),
            pt(xs, y1e + 1),
        ];
        for r in d.r0..=d.r1 {
            for c in d.c0..=d.c1 {
                let centre = pt(self.x(c) + p / 2, self.y(r) + p / 2);
                let interior = c > d.c0 && c < d.c1 && r > d.r0 && r < d.r1;
                if d.hole && interior {
                    outside.push(centre);
                } else if centre.x < x_right {
                    inside.push(centre);
                }
            }
        }
        if d.trim_x.is_some() {
            outside.push(pt((x_right + x1e).div_euclid(2), ys));
        }
        IslandSpec {
            outer: (x0e, y0e, x_right, y1e),
            hole,
            inside,
            outside,
            sample: pt(xs, ys),
        }
    }

    fn specs(&self) -> Vec<IslandSpec> {
        self.islands.iter().map(|d| self.spec(d)).collect()
    }

    fn selected_specs(&self) -> Vec<IslandSpec> {
        if self.no_samples {
            return Vec::new();
        }
        self.islands
            .iter()
            .filter(|d| d.selected)
            .map(|d| self.spec(d))
            .collect()
    }

    /// Points outside every island: ring centre, gaps, hole and trim interiors.
    fn global_outside(&self) -> Vec<Point2> {
        let p = self.pixel;
        let mut out = vec![pt(self.origin.x + p / 2, self.origin.y + p / 2)];
        for pair in self.islands.windows(2) {
            let (_, _, a_x1, _) = self.outer_untrimmed(&pair[0]);
            let (b_x0, _, _, _) = self.outer_untrimmed(&pair[1]);
            out.push(pt((a_x1 + b_x0).div_euclid(2), self.y(pair[0].r0) + p / 2));
        }
        for d in &self.islands {
            let spec = self.spec(d);
            if let Some(h) = spec.hole {
                out.push(pt((h.0 + h.2).div_euclid(2), (h.1 + h.3).div_euclid(2)));
            }
            if d.trim_x.is_some() {
                out.push(*spec.outside.last().expect("trim point"));
            }
        }
        out
    }

    fn samples(&self) -> Vec<Point2> {
        if self.no_samples {
            return Vec::new();
        }
        let mut s: Vec<Point2> = self.selected_specs().iter().map(|x| x.sample).collect();
        if self.decoys {
            s.extend(self.global_outside());
        }
        s
    }
}

fn validate_islands(case: &IslandCase, l: &GridLimits) -> Result<(), CheckFailure> {
    let (w, h) = case.size;
    if w > l.max_w || h > l.max_h || w.saturating_mul(h) > l.max_cells {
        return fixture(format!("island grid {w}x{h} exceeds the bounds"));
    }
    if !(2..=l.max_pixel).contains(&case.pixel) {
        return fixture(format!("pixel {} outside 2..=max_pixel", case.pixel));
    }
    if 2 * case.offset.abs() >= case.pixel - 1 {
        return fixture(format!(
            "offset {} violates the strict in-cell bound for pixel {}",
            case.offset, case.pixel
        ));
    }
    let x_end = cell_edge(case.origin.x, w, case.pixel)?;
    let y_end = cell_edge(case.origin.y, h, case.pixel)?;
    for v in [case.origin.x, case.origin.y, x_end, y_end] {
        if v.abs() > l.max_abs {
            return fixture("island grid extent exceeds max_abs_units");
        }
    }
    let sep = if case.fill_holes { 2 } else { 1 };
    let hole_min = if case.fill_holes { 4 } else { 3 };
    let mut prev_c1: Option<usize> = None;
    for d in &case.islands {
        if d.c0 < 1 || d.r0 < 1 || d.c1 + 2 > w || d.r1 + 2 > h || d.c0 > d.c1 || d.r0 > d.r1 {
            return fixture(format!("island {d:?} touches the outer ring"));
        }
        if let Some(pc1) = prev_c1 {
            if d.c0 < pc1 + 1 + sep {
                return fixture("islands are not separated");
            }
        }
        prev_c1 = Some(d.c1);
        if d.hole && (d.c1 + 1 - d.c0 < hole_min || d.r1 + 1 - d.r0 < hole_min) {
            return fixture("holed island too small for its hole to survive");
        }
        if let Some(tx) = d.trim_x {
            let (x0e, _, x1e, _) = case.outer_untrimmed(d);
            if d.hole || tx < x0e + 2 || tx > x1e - 2 {
                return fixture("trim must leave strict material on both sides");
            }
        }
    }
    if case.buffer().len() != w * h {
        return fixture("island buffer size");
    }
    Ok(())
}

/// Compare observed islands with analytic specs as geometric sets: matched by
/// exact derived rectangle limits, then classified at test-owned points.
fn check_islands(
    expected: &[IslandSpec],
    global_outside: &[Point2],
    observed: &[ExPolygon],
) -> Result<usize, CheckFailure> {
    let observed_limits: Vec<Option<Rect>> =
        observed.iter().map(|o| ring_bbox(&o.contour)).collect();
    if observed.len() != expected.len() {
        return Err(CheckFailure::Island(format!(
            "{} islands observed {:?}, {} expected {:?}",
            observed.len(),
            observed_limits,
            expected.len(),
            expected.iter().map(|s| s.outer).collect::<Vec<_>>()
        )));
    }
    let mut used = vec![false; observed.len()];
    for spec in expected {
        let found =
            (0..observed.len()).find(|&i| !used[i] && observed_limits[i] == Some(spec.outer));
        let Some(i) = found else {
            return Err(CheckFailure::Island(format!(
                "no observed island has the derived limits {:?}; observed {:?}",
                spec.outer, observed_limits
            )));
        };
        used[i] = true;
        let island = &observed[i];
        let holes: Vec<Option<Rect>> = island.holes.iter().map(ring_bbox).collect();
        let want: Vec<Option<Rect>> = spec.hole.into_iter().map(Some).collect();
        if holes != want {
            return Err(CheckFailure::Island(format!(
                "island {:?} holes {holes:?}, expected {want:?}",
                spec.outer
            )));
        }
        let region = std::slice::from_ref(island);
        for &p in &spec.inside {
            if locate(p, region)? != PointLocation::Inside {
                return Err(CheckFailure::Island(format!(
                    "interior point {p:?} is not inside island {:?}",
                    spec.outer
                )));
            }
        }
        for &p in &spec.outside {
            if locate(p, region)? != PointLocation::Outside {
                return Err(CheckFailure::Island(format!(
                    "excluded point {p:?} is retained by island {:?}",
                    spec.outer
                )));
            }
        }
    }
    for &p in global_outside {
        for island in observed {
            if locate(p, std::slice::from_ref(island))? != PointLocation::Outside {
                return Err(CheckFailure::Island(format!(
                    "point {p:?} outside every analytic island is retained"
                )));
            }
        }
    }
    Ok(expected.len())
}

fn as_selection(e: CheckFailure) -> CheckFailure {
    match e {
        CheckFailure::Island(m) => CheckFailure::Selection(m),
        other => other,
    }
}

fn erosion_model(t: &[u8], (w, h): (usize, usize)) -> Vec<u8> {
    let mut out = vec![0_u8; w * h];
    if w < 3 || h < 3 || t.len() < w * h {
        return out;
    }
    for r in 1..h - 1 {
        for c in 1..w - 1 {
            let all = (r - 1..=r + 1).all(|rr| (c - 1..=c + 1).all(|cc| t[rr * w + cc] != 0));
            out[r * w + c] = u8::from(all);
        }
    }
    out
}

#[derive(Debug, Default, Clone, Copy)]
struct SeedStats {
    blocks: usize,
    proven: usize,
    proven_seeded: usize,
    proven_masked: usize,
    unproven: usize,
    filled: usize,
    masked_seeds: usize,
}

/// The one seed-fill checker. Outside every block nothing changes; masked
/// cells never change. In a block whose unmasked cells form one axis-aligned
/// rectangle the result is EXACTLY "input, plus the whole rectangle when it
/// holds a seed" (see the module doc proof). Elsewhere only the sound bounds
/// `input ⊆ output ⊆ input ∪ flood_fixpoint(unmasked seeds)` are asserted.
fn check_seed_fill(
    input: &[u8],
    mask: &[u8],
    size: (usize, usize),
    blocks: (usize, usize),
    os: usize,
    observed: &[u8],
) -> Result<SeedStats, CheckFailure> {
    let (w, h) = size;
    let cells = w
        .checked_mul(h)
        .ok_or_else(|| CheckFailure::Fixture("cell overflow".to_owned()))?;
    let span = |b: usize| b.checked_mul(os).and_then(|v| v.checked_add(2));
    if os == 0 || span(blocks.0) != Some(w) || span(blocks.1) != Some(h) {
        return fixture(format!("{w}x{h} is not blocks {blocks:?} * {os} + 2"));
    }
    if input.len() != cells || mask.len() != cells {
        return fixture("input/mask length");
    }
    if observed.len() != cells {
        return Err(CheckFailure::Size(format!(
            "output {} cells, grid {cells}",
            observed.len()
        )));
    }
    let at = |c: usize, r: usize| r * w + c;
    let (bw, bh) = (blocks.0 * os, blocks.1 * os);
    for r in 0..h {
        for c in 0..w {
            let in_blocks = (1..=bw).contains(&c) && (1..=bh).contains(&r);
            if !in_blocks && observed[at(c, r)] != input[at(c, r)] {
                return Err(CheckFailure::Ring(format!(
                    "cell ({c},{r}) outside every block changed"
                )));
            }
        }
    }
    let mut stats = SeedStats::default();
    for br in 0..blocks.1 {
        for bc in 0..blocks.0 {
            stats.blocks += 1;
            let cols = bc * os + 1..=bc * os + os;
            let rows = br * os + 1..=br * os + os;
            let mut unmasked: BTreeSet<(usize, usize)> = BTreeSet::new();
            let mut masked_here = 0_usize;
            for r in rows.clone() {
                for c in cols.clone() {
                    let i = at(c, r);
                    if mask[i] != 0 {
                        masked_here += 1;
                        if observed[i] != input[i] {
                            return Err(CheckFailure::Masked(format!(
                                "masked cell ({c},{r}) changed {} -> {}",
                                input[i], observed[i]
                            )));
                        }
                        if input[i] != 0 {
                            stats.masked_seeds += 1;
                        }
                    } else {
                        unmasked.insert((c, r));
                    }
                }
            }
            let seeds: Vec<(usize, usize)> = unmasked
                .iter()
                .copied()
                .filter(|&(c, r)| input[at(c, r)] != 0)
                .collect();
            // Queue-based four-neighbour flood through the unmasked cells.
            let mut reach: BTreeSet<(usize, usize)> = seeds.iter().copied().collect();
            let mut queue: VecDeque<(usize, usize)> = seeds.iter().copied().collect();
            while let Some((c, r)) = queue.pop_front() {
                let neighbours = [
                    (c.wrapping_sub(1), r),
                    (c + 1, r),
                    (c, r.wrapping_sub(1)),
                    (c, r + 1),
                ];
                for n in neighbours {
                    if unmasked.contains(&n) && reach.insert(n) {
                        queue.push_back(n);
                    }
                }
            }
            let proven = match (unmasked.first(), unmasked.last()) {
                (Some(_), Some(_)) => {
                    let min_c = unmasked.iter().map(|p| p.0).min().unwrap_or(0);
                    let max_c = unmasked.iter().map(|p| p.0).max().unwrap_or(0);
                    let min_r = unmasked.iter().map(|p| p.1).min().unwrap_or(0);
                    let max_r = unmasked.iter().map(|p| p.1).max().unwrap_or(0);
                    (max_c - min_c + 1) * (max_r - min_r + 1) == unmasked.len()
                }
                _ => true,
            };
            if proven && !seeds.is_empty() && reach.len() != unmasked.len() {
                return fixture("a seeded rectangle is not flood-connected (model bug)");
            }
            for &(c, r) in &unmasked {
                let i = at(c, r);
                if observed[i] > 1 {
                    return Err(CheckFailure::Cell(format!("cell ({c},{r}) is not 0/1")));
                }
                let (inp, obs) = (input[i] != 0, observed[i] != 0);
                if proven {
                    let want = inp || !seeds.is_empty();
                    if obs != want {
                        return Err(if obs && seeds.is_empty() {
                            CheckFailure::SeedLeak(format!(
                                "cell ({c},{r}) set in block ({bc},{br}) with no unmasked seed"
                            ))
                        } else {
                            CheckFailure::Cell(format!(
                                "cell ({c},{r}) in rectangular block ({bc},{br}): expected {}, \
                                 observed {}",
                                u8::from(want),
                                u8::from(obs)
                            ))
                        });
                    }
                } else {
                    if inp && !obs {
                        return Err(CheckFailure::Cell(format!("seed ({c},{r}) was cleared")));
                    }
                    if obs && !inp && !reach.contains(&(c, r)) {
                        return Err(if seeds.is_empty() {
                            CheckFailure::SeedLeak(format!(
                                "cell ({c},{r}) set in block ({bc},{br}) with no unmasked seed"
                            ))
                        } else {
                            CheckFailure::Bound(format!(
                                "cell ({c},{r}) is unreachable from block ({bc},{br})'s seeds"
                            ))
                        });
                    }
                }
                if obs && !inp {
                    stats.filled += 1;
                }
            }
            if proven {
                stats.proven += 1;
                if !seeds.is_empty() && !unmasked.is_empty() {
                    stats.proven_seeded += 1;
                    if masked_here > 0 {
                        stats.proven_masked += 1;
                    }
                }
            } else {
                stats.unproven += 1;
            }
        }
    }
    Ok(stats)
}

#[derive(Debug, Default, Clone, Copy)]
struct SceneStats {
    seed: SeedStats,
    blocks: usize,
    mask_cells: usize,
}

fn validate_scene(case: &SceneCase, l: &GridLimits) -> Result<Combo, CheckFailure> {
    let Some(c) = COMBOS.get(case.combo).copied() else {
        return fixture("combo index");
    };
    if case.support.is_empty() || case.support.iter().any(|s| s.cut.is_some()) {
        return fixture("scene support must be plain rectangles");
    }
    validate_shapes(&case.support, l.max_abs)?;
    validate_shapes(&case.trim, l.max_abs)?;
    // Off the pixel lattice (multiples of the combo pixel).
    off_lattice(&case.support, pt(0, 0), c.pixel)?;
    off_lattice(&case.trim, pt(0, 0), c.pixel)?;
    Ok(c)
}

/// The one scene checker for `SupportGrid::new` + `grid`.
fn check_scene(
    case: &SceneCase,
    l: &GridLimits,
    grid: &SupportGrid,
) -> Result<SceneStats, CheckFailure> {
    let c = validate_scene(case, l)?;
    let gp = grid.params();
    if gp.pixel_size != c.pixel || gp.oversampling != c.os {
        return Err(CheckFailure::Params(format!(
            "pixel {} / oversampling {}, hand-derived {} / {}",
            gp.pixel_size, gp.oversampling, c.pixel, c.os
        )));
    }
    let (gw, gh) = gp.grid_size;
    let span = |b: usize| b.checked_mul(c.os).and_then(|v| v.checked_add(2));
    if span(gp.blocks.0) != Some(gw) || span(gp.blocks.1) != Some(gh) {
        return Err(CheckFailure::Params(format!(
            "grid {gw}x{gh} is not blocks {:?} * {} + 2",
            gp.blocks, c.os
        )));
    }
    let lattice = |v: i64| v.rem_euclid(c.pixel) == 0 && (v + c.pixel).rem_euclid(c.res) == 0;
    if !lattice(gp.origin.x) || !lattice(gp.origin.y) {
        return Err(CheckFailure::Params(format!(
            "origin {:?} is not one pixel below a {}-unit block line",
            gp.origin, c.res
        )));
    }
    let blocks = gp.blocks.0.saturating_mul(gp.blocks.1);
    let cells = gw.saturating_mul(gh);
    if blocks > l.max_blocks || gw > l.max_w || gh > l.max_h || cells > l.max_cells {
        return fixture(format!(
            "scene derived {gw}x{gh} cells / {blocks} blocks beyond the bounds"
        ));
    }
    let inner_x = (
        gp.origin.x + c.pixel,
        gp.origin.x + as_i64(gw - 1) * c.pixel,
    );
    let inner_y = (
        gp.origin.y + c.pixel,
        gp.origin.y + as_i64(gh - 1) * c.pixel,
    );
    for s in &case.support {
        if !(inner_x.0 < s.x0 && s.x1 < inner_x.1 && inner_y.0 < s.y0 && s.y1 < inner_y.1) {
            return Err(CheckFailure::Params(format!(
                "support {s:?} not inside the block interior {inner_x:?} x {inner_y:?}"
            )));
        }
    }
    let support_model = raster_model(&case.support, c.pixel, gp.origin, gp.grid_size)?;
    for r in 0..gh {
        for col in 0..gw {
            let ring = col == 0 || r == 0 || col == gw - 1 || r == gh - 1;
            if ring && support_model[r * gw + col] != 0 {
                return Err(CheckFailure::Params(
                    "support reaches the outer ring".to_owned(),
                ));
            }
        }
    }
    let trim_model = raster_model(&case.trim, c.pixel, gp.origin, gp.grid_size)?;
    let mask = erosion_model(&trim_model, gp.grid_size);
    if dilate_trimming_region(&trim_model, gp) != mask {
        return Err(CheckFailure::Cell(
            "dilate_trimming_region differs from the 3x3 all-set erosion model".to_owned(),
        ));
    }
    let seed = check_seed_fill(
        &support_model,
        &mask,
        gp.grid_size,
        gp.blocks,
        gp.oversampling,
        grid.grid(),
    )?;
    Ok(SceneStats {
        seed,
        blocks,
        mask_cells: mask.iter().filter(|&&v| v != 0).count(),
    })
}

fn validate_seed(case: &SeedCase, l: &GridLimits) -> Result<(), CheckFailure> {
    let (w, h) = case.size;
    if w > l.max_w
        || h > l.max_h
        || w.saturating_mul(h) > l.max_cells
        || case.blocks.0.saturating_mul(case.blocks.1) > l.max_blocks
        || !(1..=8).contains(&case.os)
    {
        return fixture("seed grid beyond the bounds");
    }
    for r in 0..h {
        for c in 0..w {
            let ring = c == 0 || r == 0 || c == w - 1 || r == h - 1;
            if ring
                && (case.input.get(r * w + c) != Some(&0) || case.mask.get(r * w + c) != Some(&0))
            {
                return fixture("the outer ring must be prevalidated empty");
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum GridBody {
    Scene(SceneCase),
    Seed(SeedCase),
    Islands(IslandCase),
}

#[derive(Debug, Clone, PartialEq)]
struct GridCase {
    mode: GridMode,
    body: GridBody,
}

#[derive(Debug, Default, Clone, Copy)]
struct GridStats {
    seed: SeedStats,
    multi_block: bool,
    mask_cells: usize,
    islands: usize,
    selected: usize,
    holes: usize,
    trimmed: usize,
}

/// Run the SUT for one generated grid case and check it with the shared
/// checkers.
fn check_grid_case(case: &GridCase, l: &GridLimits) -> Result<GridStats, CheckFailure> {
    let mut stats = GridStats::default();
    match (&case.mode, &case.body) {
        (GridMode::SceneClear | GridMode::SceneTrim, GridBody::Scene(scene)) => {
            if (case.mode == GridMode::SceneTrim) == scene.trim.is_empty() {
                return fixture("scene trim presence disagrees with the mode");
            }
            let grid = scene.build();
            let s = check_scene(scene, l, &grid)?;
            stats.seed = s.seed;
            stats.multi_block = s.blocks >= 2;
            stats.mask_cells = s.mask_cells;
        }
        (GridMode::SeedRect, GridBody::Seed(seed)) => {
            validate_seed(seed, l)?;
            let s = check_seed_fill(
                &seed.input,
                &seed.mask,
                seed.size,
                seed.blocks,
                seed.os,
                &seed.run(),
            )?;
            if s.unproven != 0 {
                return fixture("SeedRect must construct only rectangular subdomains");
            }
            stats.seed = s;
            stats.multi_block = s.blocks >= 2;
        }
        (GridMode::Islands, GridBody::Islands(ic)) => {
            validate_islands(ic, l)?;
            let sg = SupportGrid::for_test(ic.params(), ic.buffer(), ic.trimming());
            let all = sg.extract_islands(ic.offset, ic.fill_holes);
            let outside = ic.global_outside();
            stats.islands = check_islands(&ic.specs(), &outside, &all)?;
            let selected = sg.filter_islands_by_samples(all, &ic.samples());
            stats.selected =
                check_islands(&ic.selected_specs(), &outside, &selected).map_err(as_selection)?;
            stats.holes = ic.islands.iter().filter(|d| d.hole).count();
            stats.trimmed = ic.islands.iter().filter(|d| d.trim_x.is_some()).count();
        }
        _ => return fixture("grid mode and body disagree"),
    }
    Ok(stats)
}

fn build_scene(trim: bool, raw: &[u32; GRID_RAW], l: &GridLimits) -> SceneCase {
    let combo = raw[0] as usize % COMBOS.len();
    let c = COMBOS[combo];
    let (p, os, res) = (c.pixel, c.os, c.res);
    let margin = 2 * p + 1;
    let (k_lo, k_end) = block_index_range(l.max_abs, res, margin);
    let fit = usize::try_from(k_end - k_lo).expect("fit");
    let bx_cap = ((l.max_w - 2) / os)
        .min(l.max_blocks)
        .min(fit)
        .min((l.max_cells / (os + 2)).saturating_sub(2) / os);
    let bx = pick_usize(raw[1], 1, bx_cap);
    let by_cap = ((l.max_h - 2) / os)
        .min(l.max_blocks / bx)
        .min(fit)
        .min((l.max_cells / (bx * os + 2)).saturating_sub(2) / os);
    let by = pick_usize(raw[2], 1, by_cap);
    let kx = pick(raw[3], k_lo, k_end - as_i64(bx));
    let ky = pick(raw[4], k_lo, k_end - as_i64(by));
    let (rx, ry) = (kx * res, ky * res);
    let (sx, sy) = (as_i64(bx) * res, as_i64(by) * res);
    let n = 1 + (raw[5] % 3) as usize;
    let support = (0..n)
        .map(|i| {
            let xs = off_grid_increasing(&raw[6 + 4 * i..8 + 4 * i], 1, sx - 1, p);
            let ys = off_grid_increasing(&raw[8 + 4 * i..10 + 4 * i], 1, sy - 1, p);
            Shape::rect(rx + xs[0], ry + ys[0], rx + xs[1], ry + ys[1])
        })
        .collect();
    let trim = if trim {
        let vertical = raw[18] % 2 == 0;
        let along = if vertical { sx } else { sy };
        let band = off_grid_increasing(&raw[19..21], 1, along - 1, p);
        vec![if vertical {
            Shape::rect(rx + band[0], ry - margin, rx + band[1], ry + sy + margin)
        } else {
            Shape::rect(rx - margin, ry + band[0], rx + sx + margin, ry + band[1])
        }]
    } else {
        Vec::new()
    };
    SceneCase {
        combo,
        support,
        trim,
    }
}

fn isqrt(n: usize) -> usize {
    let mut r = 0;
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r
}

fn build_seed(raw: &[u32; GRID_RAW], braw: &[u32], bits: &[bool], l: &GridLimits) -> SeedCase {
    let os_cap = 8
        .min(l.max_w - 2)
        .min(l.max_h - 2)
        .min(isqrt(l.max_cells) - 2);
    let os = pick_usize(raw[0], 1, os_cap);
    let bx_cap = ((l.max_w - 2) / os)
        .min(l.max_blocks)
        .min((l.max_cells / (os + 2)).saturating_sub(2) / os);
    let bx = pick_usize(raw[1], 1, bx_cap);
    let by_cap = ((l.max_h - 2) / os)
        .min(l.max_blocks / bx)
        .min((l.max_cells / (bx * os + 2)).saturating_sub(2) / os);
    let by = pick_usize(raw[2], 1, by_cap);
    let size = (bx * os + 2, by * os + 2);
    let w = size.0;
    let mut input = vec![0_u8; size.0 * size.1];
    let mut mask = vec![0_u8; size.0 * size.1];
    for k in 0..bx * by {
        let (bc, br) = (k % bx, k / bx);
        let r4 = &braw[4 * k..4 * k + 4];
        // Style 0: clear; 1: fully masked; otherwise one unmasked sub-rectangle.
        let (a0, a1, b0, b1, full_mask) = match r4[0] % 6 {
            0 => (0, os - 1, 0, os - 1, false),
            1 => (0, 0, 0, 0, true),
            _ => {
                let a0 = r4[1] as usize % os;
                let a1 = a0 + (r4[1] >> 8) as usize % (os - a0);
                let b0 = r4[2] as usize % os;
                let b1 = b0 + (r4[2] >> 8) as usize % (os - b0);
                (a0, a1, b0, b1, false)
            }
        };
        for lr in 0..os {
            for lc in 0..os {
                let (c, r) = (bc * os + 1 + lc, br * os + 1 + lr);
                let i = r * w + c;
                let inside = !full_mask && (a0..=a1).contains(&lc) && (b0..=b1).contains(&lr);
                mask[i] = u8::from(!inside);
                input[i] = u8::from(bits[i] && bits[(i * 7 + 3) % bits.len()]);
            }
        }
        if !full_mask && r4[3] % 3 != 0 {
            let lc = a0 + r4[3] as usize % (a1 - a0 + 1);
            let lr = b0 + (r4[3] >> 4) as usize % (b1 - b0 + 1);
            input[(br * os + 1 + lr) * w + bc * os + 1 + lc] = 1;
        }
    }
    SeedCase {
        size,
        blocks: (bx, by),
        os,
        input,
        mask,
    }
}

fn build_islands(raw: &[u32; GRID_RAW], l: &GridLimits) -> IslandCase {
    let fill_holes = raw[0] % 2 == 1;
    let sep = if fill_holes { 2 } else { 1 };
    let hole_min = if fill_holes { 4 } else { 3 };
    let n = pick_usize(raw[1], 1, 3);
    let hole_at = raw[2] as usize % (n + 1);
    let wcap = (l.max_w - 2 - 2 * sep) / 3;
    let h_lim = l.max_h.min(l.max_cells / l.max_w);
    let hcap = h_lim - 2;
    let mut islands = Vec::with_capacity(n);
    let mut cursor = 1;
    for i in 0..n {
        let hole = i == hole_at;
        let lo = if hole { hole_min } else { 1 };
        let w = pick_usize(raw[3 + i], lo, wcap);
        let h = pick_usize(raw[6 + i], lo, hcap);
        let r0 = 1 + pick_usize(raw[9 + i], 0, hcap - h);
        islands.push(IslandDef {
            c0: cursor,
            r0,
            c1: cursor + w - 1,
            r1: r0 + h - 1,
            hole,
            trim_x: None,
            selected: false,
        });
        cursor += w + sep;
    }
    let min_w = cursor - sep + 1;
    let gw = min_w + pick_usize(raw[12], 0, l.max_w - min_w);
    let min_h = islands.iter().map(|d| d.r1).max().unwrap_or(1) + 2;
    let gh = pick_usize(raw[13], min_h, h_lim);
    let pixel = pick(raw[14], 2, l.max_pixel.min(2 * l.max_abs / as_i64(gw)));
    let ox = -l.max_abs + pick(raw[15], 0, 2 * l.max_abs - as_i64(gw) * pixel);
    let oy = -l.max_abs + pick(raw[16], 0, 2 * l.max_abs - as_i64(gh) * pixel);
    let omax = legal_offset_max(pixel);
    let offset = pick(raw[17], -omax, omax);
    let mut case = IslandCase {
        pixel,
        origin: pt(ox, oy),
        size: (gw, gh),
        offset,
        fill_holes,
        islands,
        decoys: raw[28] % 2 == 1,
        no_samples: raw[27] % 4 == 0,
    };
    for i in 0..n {
        let d = case.islands[i].clone();
        if !d.hole && raw[18 + i] % 2 == 1 {
            let (x0e, _, x1e, _) = case.outer_untrimmed(&d);
            if x1e - x0e >= 4 {
                case.islands[i].trim_x = Some(pick(raw[21 + i], x0e + 2, x1e - 2));
            }
        }
        case.islands[i].selected = raw[24 + i] % 2 == 1;
    }
    case
}

fn build_grid_case(
    mode: GridMode,
    raw: &[u32; GRID_RAW],
    braw: &[u32],
    bits: &[bool],
    l: &GridLimits,
) -> GridCase {
    let body = match mode {
        GridMode::SceneClear => GridBody::Scene(build_scene(false, raw, l)),
        GridMode::SceneTrim => GridBody::Scene(build_scene(true, raw, l)),
        GridMode::SeedRect => GridBody::Seed(build_seed(raw, braw, bits, l)),
        GridMode::Islands => GridBody::Islands(build_islands(raw, l)),
    };
    GridCase { mode, body }
}

fn grid_strategy(l: &GridLimits, modes: Vec<GridMode>) -> BoxedStrategy<GridCase> {
    let l = *l;
    (
        prop::sample::select(modes).no_shrink(),
        any::<[u32; GRID_RAW]>(),
        prop::collection::vec(any::<u32>(), 4 * l.max_blocks),
        prop::collection::vec(any::<bool>(), l.max_cells),
    )
        .prop_map(move |(mode, raw, braw, bits)| build_grid_case(mode, &raw, &braw, &bits, &l))
        .boxed()
}

/// `grid_size = (6,4)`, `blocks = (2,1)`, `oversampling = 2`: block 0 owns
/// columns 1..=2, block 1 columns 3..=4, rows 1..=2.
fn seed_control(seed: (usize, usize), masked: &[(usize, usize)]) -> SeedCase {
    let (w, h) = (6, 4);
    let mut input = vec![0_u8; w * h];
    let mut mask = vec![0_u8; w * h];
    input[seed.1 * w + seed.0] = 1;
    for &(c, r) in masked {
        mask[r * w + c] = 1;
    }
    SeedCase {
        size: (w, h),
        blocks: (2, 1),
        os: 2,
        input,
        mask,
    }
}

/// Combo 0 (2.0 mm / 0.4 mm): the support rectangle sits inside the single
/// block `[20000, 40000]²`; derived origin `align(21233, 20000) - 5000 =
/// 15000`.
fn halo_scene() -> SceneCase {
    SceneCase {
        combo: 0,
        support: vec![Shape::rect(21_234, 23_456, 27_777, 31_111)],
        trim: Vec::new(),
    }
}

fn halo_spec() -> IslandSpec {
    IslandSpec {
        outer: (20_000, 20_000, 40_000, 40_000),
        hole: None,
        // Inside the block but outside the original polygon.
        inside: vec![pt(35_000, 35_000), pt(20_001, 39_999), pt(39_999, 20_001)],
        outside: vec![pt(19_999, 30_000), pt(40_001, 30_000), pt(30_000, 40_001)],
        sample: pt(35_000, 35_000),
    }
}

/// Pixel 10, origin 0: islands at cells (1,1) and (4,1), only the first
/// sampled (its sample is (15,15)).
fn two_island_case() -> IslandCase {
    let one = |c: usize, selected: bool| IslandDef {
        c0: c,
        r0: 1,
        c1: c,
        r1: 1,
        hole: false,
        trim_x: None,
        selected,
    };
    IslandCase {
        pixel: 10,
        origin: pt(0, 0),
        size: (6, 3),
        offset: 0,
        fill_holes: false,
        islands: vec![one(1, true), one(4, false)],
        decoys: false,
        no_samples: false,
    }
}

/// Cell (1,1) = `[10,20]²` trimmed by `[15,20] x [10,20]`: sample (12,15),
/// excluded point (17,15).
fn trim_island_case() -> IslandCase {
    IslandCase {
        pixel: 10,
        origin: pt(0, 0),
        size: (4, 3),
        offset: 0,
        fill_holes: false,
        islands: vec![IslandDef {
            c0: 1,
            r0: 1,
            c1: 1,
            r1: 1,
            hole: false,
            trim_x: Some(15),
            selected: true,
        }],
        decoys: true,
        no_samples: false,
    }
}

/// A 4x4-cell ring island with a 2x2 hole, fill_holes on, positive offset.
fn holed_island_case() -> IslandCase {
    IslandCase {
        pixel: 10,
        origin: pt(-30, -30),
        size: (7, 7),
        offset: 3,
        fill_holes: true,
        islands: vec![IslandDef {
            c0: 1,
            r0: 1,
            c1: 4,
            r1: 4,
            hole: true,
            trim_x: None,
            selected: true,
        }],
        decoys: true,
        no_samples: false,
    }
}

#[derive(Debug, Default)]
struct GridWitnessStats {
    block_isolation: u64,
    masked_source: u64,
    masked_destination: u64,
    halo: u64,
    selection: u64,
    trim: u64,
    hole_exclusion: u64,
    no_samples: u64,
    empty_extraction: u64,
    legal_boundary: u64,
}

fn cell6(g: &[u8], c: usize, r: usize) -> u8 {
    g[r * 6 + c]
}

fn grid_witness_observations(l: &GridLimits) -> Result<GridWitnessStats, String> {
    let mut out = GridWitnessStats::default();
    let seed_checked = |case: &SeedCase| -> Result<Vec<u8>, String> {
        let observed = case.run();
        check_seed_fill(
            &case.input,
            &case.mask,
            case.size,
            case.blocks,
            case.os,
            &observed,
        )
        .map_err(|e| format!("seed witness: {e}"))?;
        Ok(observed)
    };

    // Clear mask, seed (2,1): block 0 fills, block 1 stays empty.
    let g = seed_checked(&seed_control((2, 1), &[]))?;
    let block0 = [(1, 1), (2, 1), (1, 2), (2, 2)]
        .iter()
        .all(|&(c, r)| cell6(&g, c, r) == 1);
    let block1 = [(3, 1), (4, 1), (3, 2), (4, 2)]
        .iter()
        .all(|&(c, r)| cell6(&g, c, r) == 0);
    if !(block0 && block1) {
        return Err(format!("block isolation witness: {g:?}"));
    }
    out.block_isolation += 1;

    // Masked source (1,1): nothing propagates; the masked seed stays.
    let case = seed_control((1, 1), &[(1, 1)]);
    let g = seed_checked(&case)?;
    if g != case.input || cell6(&g, 1, 1) != 1 || cell6(&g, 2, 1) != 0 {
        return Err("masked-source witness propagated".to_owned());
    }
    out.masked_source += 1;

    // Masked destination (2,1): never written; the rest is reached around it.
    let g = seed_checked(&seed_control((1, 1), &[(2, 1)]))?;
    if cell6(&g, 2, 1) != 0 || cell6(&g, 1, 2) != 1 || cell6(&g, 2, 2) != 1 {
        return Err("masked-destination witness".to_owned());
    }
    out.masked_destination += 1;

    // Halo: SupportGrid::new fills the whole block, and extraction returns the
    // block-local extent, not the original polygon.
    let scene = halo_scene();
    let grid = scene.build();
    let s = check_scene(&scene, l, &grid).map_err(|e| format!("halo scene: {e}"))?;
    if s.seed.proven_seeded != 1 || s.seed.filled == 0 {
        return Err("halo scene did not flood its block".to_owned());
    }
    let islands = grid.extract_islands(0, true);
    check_islands(&[halo_spec()], &[], &islands).map_err(|e| format!("halo extraction: {e}"))?;
    let kept = grid.filter_islands_by_samples(islands, &[halo_spec().sample]);
    check_islands(&[halo_spec()], &[], &kept).map_err(|e| format!("halo selection: {e}"))?;
    out.halo += 1;

    for (label, case) in [
        ("selection", two_island_case()),
        ("trim", trim_island_case()),
        ("hole", holed_island_case()),
    ] {
        let gc = GridCase {
            mode: GridMode::Islands,
            body: GridBody::Islands(case),
        };
        let st = check_grid_case(&gc, l).map_err(|e| format!("{label} witness: {e}"))?;
        match label {
            "selection" if st.islands == 2 && st.selected == 1 => out.selection += 1,
            "trim" if st.trimmed == 1 && st.selected == 1 => out.trim += 1,
            "hole" if st.holes == 1 && st.selected == 1 => out.hole_exclusion += 1,
            _ => return Err(format!("{label} witness population {st:?}")),
        }
    }

    let mut none = two_island_case();
    none.no_samples = true;
    let sg = SupportGrid::for_test(none.params(), none.buffer(), Vec::new());
    let all = sg.extract_islands(0, false);
    if all.len() != 2 || !sg.filter_islands_by_samples(all, &[]).is_empty() {
        return Err("no samples must select no island".to_owned());
    }
    out.no_samples += 1;

    // Tiny and short extraction buffers at a valid offset: actual empty result.
    let tiny = SupportGrid::for_test(
        GridParams::for_test(10, pt(0, 0), (1, 4), (0, 0), 1),
        vec![1; 4],
        Vec::new(),
    );
    let short = SupportGrid::for_test(
        GridParams::for_test(10, pt(0, 0), (5, 5), (0, 0), 1),
        vec![0, 0, 0, 0, 0, 0, 1, 1, 0, 0],
        Vec::new(),
    );
    if !tiny.extract_islands(0, false).is_empty() || !short.extract_islands(0, true).is_empty() {
        return Err("tiny/short extraction buffers produced islands".to_owned());
    }
    out.empty_extraction += 1;

    // The largest legal offset (4 for a 10-unit pixel) is accepted.
    let mut boundary = two_island_case();
    boundary.offset = legal_offset_max(10);
    let gc = GridCase {
        mode: GridMode::Islands,
        body: GridBody::Islands(boundary),
    };
    check_grid_case(&gc, l).map_err(|e| format!("legal boundary offset: {e}"))?;
    out.legal_boundary += 1;
    Ok(out)
}

#[test]
fn proptest_support_grid_extraction() {
    let run = PropertyRun::new(GRID_ID, GRID_TEST).expect("M-SUPPORT-GRID context");
    let limits = grid_limits(&live_bounds(&run, &GRID_KEYS)).expect("grid bounds");
    let strategy = grid_strategy(&limits, GRID_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let witness = std::cell::RefCell::new(GridWitnessStats::default());
    let proven = Cell::new(0_u64);
    let proven_seeded = Cell::new(0_u64);
    let proven_masked = Cell::new(0_u64);
    let unproven = Cell::new(0_u64);
    let masked_seeds = Cell::new(0_u64);
    let filled = Cell::new(0_u64);
    let multi_block_scenes = Cell::new(0_u64);
    let trim_mask_scenes = Cell::new(0_u64);
    let islands = Cell::new(0_u64);
    let selected = Cell::new(0_u64);
    let holed = Cell::new(0_u64);
    let trimmed = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 4] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let w = match grid_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        {
            let mut acc = witness.borrow_mut();
            acc.block_isolation += w.block_isolation;
            acc.masked_source += w.masked_source;
            acc.masked_destination += w.masked_destination;
            acc.halo += w.halo;
            acc.selection += w.selection;
            acc.trim += w.trim;
            acc.hole_exclusion += w.hole_exclusion;
            acc.no_samples += w.no_samples;
            acc.empty_extraction += w.empty_extraction;
            acc.legal_boundary += w.legal_boundary;
        }
        let stats = match check_grid_case(&case, &limits) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{:?} grid case: {e}", case.mode)),
        };
        if matches!(case.mode, GridMode::SceneClear | GridMode::SceneTrim)
            && stats.seed.proven_seeded + stats.seed.unproven == 0
            && stats.seed.masked_seeds == 0
        {
            return fail("a scene produced no seeded block");
        }
        let slot = GRID_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        proven.set(proven.get() + stats.seed.proven as u64);
        proven_seeded.set(proven_seeded.get() + stats.seed.proven_seeded as u64);
        proven_masked.set(proven_masked.get() + stats.seed.proven_masked as u64);
        unproven.set(unproven.get() + stats.seed.unproven as u64);
        masked_seeds.set(masked_seeds.get() + stats.seed.masked_seeds as u64);
        filled.set(filled.get() + stats.seed.filled as u64);
        if stats.multi_block && case.mode != GridMode::Islands {
            multi_block_scenes.set(multi_block_scenes.get() + 1);
        }
        if case.mode == GridMode::SceneTrim && stats.mask_cells > 0 {
            trim_mask_scenes.set(trim_mask_scenes.get() + 1);
        }
        islands.set(islands.get() + stats.islands as u64);
        selected.set(selected.get() + stats.selected as u64);
        holed.set(holed.get() + stats.holes as u64);
        trimmed.set(trimmed.get() + stats.trimmed as u64);
        Ok(())
    })
    .expect("M-SUPPORT-GRID block-local seed fill, erosion and island extraction");

    let w = witness.into_inner();
    assert!(calls.get() > 0, "the property callback never ran");
    for (name, v) in [
        ("block isolation", w.block_isolation),
        ("masked source", w.masked_source),
        ("masked destination", w.masked_destination),
        ("halo", w.halo),
        ("selection", w.selection),
        ("trim", w.trim),
        ("hole exclusion", w.hole_exclusion),
        ("no samples", w.no_samples),
        ("empty extraction", w.empty_extraction),
        ("legal boundary", w.legal_boundary),
    ] {
        assert!(v > 0, "{name} witness unobserved");
    }
    assert!(
        proven_seeded.get() > 0,
        "no seeded rectangular block was compared"
    );
    println!(
        "M-SUPPORT-GRID accepted_calls={} scene_clear={} scene_trim={} seed_rect={} islands_mode={} \
         proven_blocks={} proven_seeded={} proven_masked={} unproven_blocks={} masked_seeds={} \
         filled={} multi_block={} trim_masked_scenes={} islands={} selected={} holed={} \
         trimmed={} witnesses={w:?}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        proven.get(),
        proven_seeded.get(),
        proven_masked.get(),
        unproven.get(),
        masked_seeds.get(),
        filled.get(),
        multi_block_scenes.get(),
        trim_mask_scenes.get(),
        islands.get(),
        selected.get(),
        holed.get(),
        trimmed.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SUPPORT-TERRITORY
// ─────────────────────────────────────────────────────────────────────────────

const TERRITORY_ID: &str = "M-SUPPORT-TERRITORY";
const TERRITORY_TEST: &str = "property_tests::proptest_support_planner_territory";
const TERRITORY_KEYS: [&str; 3] = ["max_abs_units", "max_clearance_units", "max_cells"];
const TRADITIONAL: &str = "traditional";
const FOREIGN_FAMILY: &str = "tree";
/// The witnesses use `[0, 10]` mm = `[0, 100000]` units.
const TERRITORY_WITNESS_ABS: i64 = 100_000;
/// The witnesses use a 1 mm line width.
const TERRITORY_WITNESS_CLEARANCE: i64 = 10_000;
/// Two subject rectangles plus two own/foreign rectangles and their expanded
/// bars give at most 12 distinct edges per axis.
const TERRITORY_MAX_GENERATED_CELLS: usize = 121;
const TERRITORY_RAW: usize = 32;

#[derive(Debug, Clone, Copy)]
struct TerritoryLimits {
    max_abs: i64,
    max_clearance: i64,
    max_cells: usize,
}

fn territory_limits(bounds: &BTreeMap<String, u64>) -> Result<TerritoryLimits, DomainError> {
    let l = TerritoryLimits {
        max_abs: bound_i64(bounds, "max_abs_units")?,
        max_clearance: bound_i64(bounds, "max_clearance_units")?,
        max_cells: bound_usize(bounds, "max_cells")?,
    };
    if !(TERRITORY_WITNESS_ABS..=ABS_CEILING).contains(&l.max_abs) {
        return Err(invalid(format!(
            "max_abs_units must be in {TERRITORY_WITNESS_ABS}..={ABS_CEILING}"
        )));
    }
    if !(TERRITORY_WITNESS_CLEARANCE..=l.max_abs).contains(&l.max_clearance)
        || l.max_clearance % 2 != 0
    {
        return Err(invalid(
            "max_clearance_units must be even, admit the 1 mm witness and not exceed max_abs",
        ));
    }
    if !(TERRITORY_MAX_GENERATED_CELLS..=CELLS_CEILING).contains(&l.max_cells) {
        return Err(invalid(format!(
            "max_cells must admit {TERRITORY_MAX_GENERATED_CELLS} classification cells"
        )));
    }
    Ok(l)
}

fn territory_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 100_000),
        ("max_clearance_units".to_owned(), 10_000),
        ("max_cells".to_owned(), 121),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerritoryMode {
    Own,
    Base,
    NoPartition,
    OwnFamilyOnly,
    EmptySubject,
    FullyRemoved,
}

const TERRITORY_MODES: [TerritoryMode; 6] = [
    TerritoryMode::Own,
    TerritoryMode::Base,
    TerritoryMode::NoPartition,
    TerritoryMode::OwnFamilyOnly,
    TerritoryMode::EmptySubject,
    TerritoryMode::FullyRemoved,
];

#[derive(Debug, Clone, PartialEq)]
struct Source {
    object: String,
    region: String,
    chain: Vec<(String, PaintValue)>,
}

#[derive(Debug, Clone, PartialEq)]
struct TerritoryEntry {
    layer: u32,
    source: Source,
    rects: Vec<Rect>,
}

#[derive(Debug, Clone, PartialEq)]
struct TerritoryCase {
    mode: TerritoryMode,
    layer: u32,
    width_units: i64,
    subject: Vec<Rect>,
    query: Source,
    entries: Vec<TerritoryEntry>,
    families: Vec<(Source, String)>,
}

enum Expectation {
    Own(Vec<Rect>),
    Base { foreign: Vec<Rect>, bar: Vec<Rect> },
    Unchanged,
}

fn src(object: &str, region: &str, chain: Vec<(String, PaintValue)>) -> Source {
    Source {
        object: object.to_owned(),
        region: region.to_owned(),
        chain,
    }
}

fn paint_a() -> (String, PaintValue) {
    (
        "support_family_paint".to_owned(),
        PaintValue::Custom("tree".to_owned()),
    )
}

fn paint_b() -> (String, PaintValue) {
    ("fuzzy_skin_paint".to_owned(), PaintValue::Flag(true))
}

fn chain_of(k: u32) -> Vec<(String, PaintValue)> {
    match k % 4 {
        0 => Vec::new(),
        1 => vec![paint_a()],
        2 => vec![paint_a(), paint_b()],
        _ => vec![paint_b(), paint_a()],
    }
}

/// A chain that differs from `c` as an ordered vector.
fn other_chain(c: &[(String, PaintValue)]) -> Vec<(String, PaintValue)> {
    match c.len() {
        0 => vec![paint_a()],
        1 => Vec::new(),
        _ => c.iter().rev().cloned().collect(),
    }
}

fn contains_strict(r: &Rect, x: i64, y: i64) -> bool {
    r.0 < x && x < r.2 && r.1 < y && y < r.3
}

impl TerritoryCase {
    fn width_mm(&self) -> f32 {
        self.width_units as f32 / 10_000.0
    }

    fn view(&self) -> SupportAnalysisView {
        SupportAnalysisView {
            support_territory: self
                .entries
                .iter()
                // exhaustive: every field is the exact territory source identity under test.
                .map(|e| SupportAnalysisGeometryEntry {
                    global_support_layer_index: e.layer,
                    object_id: e.source.object.clone(),
                    region_id: e.source.region.clone(),
                    variant_chain: e.source.chain.clone(),
                    polygons: e.rects.iter().copied().map(rect_poly).collect(),
                })
                .collect(),
            family_assignments: self
                .families
                .iter()
                .map(|(s, family)| SupportFamilyAssignment {
                    object_id: s.object.clone(),
                    region_id: s.region.clone(),
                    variant_chain: s.chain.clone(),
                    family_id: family.clone(),
                })
                .collect(),
            ..SupportAnalysisView::default()
        }
    }

    /// Test-local exact family lookup over the fixture's own list.
    fn family(&self, s: &Source) -> Option<&str> {
        self.families
            .iter()
            .find(|(f, _)| f == s)
            .map(|(_, family)| family.as_str())
    }

    fn expectation(&self) -> Expectation {
        if let Some(e) = self
            .entries
            .iter()
            .find(|e| e.layer == self.layer && e.source == self.query)
        {
            return Expectation::Own(e.rects.clone());
        }
        let partition: Vec<&TerritoryEntry> = self
            .entries
            .iter()
            .filter(|e| e.layer == self.layer && e.source.object == self.query.object)
            .collect();
        if partition.is_empty() {
            return Expectation::Unchanged;
        }
        let foreign: Vec<Rect> = partition
            .iter()
            .filter(|e| self.family(&e.source) != Some(TRADITIONAL))
            .flat_map(|e| e.rects.iter().copied())
            .collect();
        if foreign.is_empty() {
            return Expectation::Unchanged;
        }
        let w = self.width_units;
        let bar = foreign
            .iter()
            .map(|r| (r.0 - w, r.1 - w, r.2 + w, r.3 + w))
            .collect();
        Expectation::Base { foreign, bar }
    }

    fn run(&self) -> Vec<ExPolygon> {
        let subject: Vec<ExPolygon> = self.subject.iter().copied().map(rect_poly).collect();
        planner(self.width_mm()).clip_to_territory(
            subject,
            &self.view(),
            &self.query.object,
            &self.query.region,
            &self.query.chain,
            self.layer,
        )
    }
}

fn planner(line_width_mm: f32) -> SupportPlanner {
    SupportPlanner {
        enabled: true,
        support_family: TRADITIONAL.to_owned(),
        support_interface_top_layers: 0,
        support_interface_bottom_layers: 0,
        support_base_pattern: "rectilinear".to_owned(),
        support_top_z_distance: 0.2,
        support_layer_height_mm: 0.0,
        independent_support_layer_height: false,
        support_object_xy_distance: 0.0,
        line_width_mm,
        support_base_pattern_spacing_mm: 2.0,
        support_area_rasterizer: RasterizerMode::LegacySemantic,
    }
}

fn validate_territory(case: &TerritoryCase, l: &TerritoryLimits) -> Result<(), CheckFailure> {
    let w = case.width_units;
    if !(2..=l.max_clearance).contains(&w) || w % 2 != 0 {
        return fixture(format!("width {w} units outside the even positive domain"));
    }
    if ((case.width_mm() as f64) * 10_000.0).round() as i64 != w {
        return fixture("line width does not round-trip through mm");
    }
    let rects = case
        .subject
        .iter()
        .chain(case.entries.iter().flat_map(|e| e.rects.iter()));
    for r in rects {
        if !(r.0 < r.2 && r.1 < r.3) {
            return fixture(format!("degenerate rectangle {r:?}"));
        }
        for v in [r.0, r.1, r.2, r.3] {
            if v.abs() > l.max_abs || v % 2 != 0 {
                return fixture(format!("coordinate {v} not even within max_abs_units"));
            }
        }
    }
    for (i, a) in case.subject.iter().enumerate() {
        for b in &case.subject[i + 1..] {
            if a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3 {
                return fixture("subject rectangles overlap");
            }
        }
    }
    let expectation = case.expectation();
    let ok = match case.mode {
        TerritoryMode::Own => {
            matches!(expectation, Expectation::Own(_)) && !case.subject.is_empty()
        }
        TerritoryMode::Base | TerritoryMode::FullyRemoved => {
            matches!(expectation, Expectation::Base { .. }) && !case.subject.is_empty()
        }
        TerritoryMode::NoPartition => {
            !case
                .entries
                .iter()
                .any(|e| e.layer == case.layer && e.source.object == case.query.object)
                && !case.subject.is_empty()
        }
        TerritoryMode::OwnFamilyOnly => {
            matches!(expectation, Expectation::Unchanged)
                && case
                    .entries
                    .iter()
                    .any(|e| e.layer == case.layer && e.source.object == case.query.object)
                && !case.subject.is_empty()
        }
        TerritoryMode::EmptySubject => case.subject.is_empty(),
    };
    if !ok {
        return fixture(format!("{:?} case has the wrong structure", case.mode));
    }
    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
struct TerritoryStats {
    cells: usize,
    kept: usize,
    removed: usize,
    /// Subject cells inside the expanded bar but outside the raw foreign rect.
    clearance: usize,
}

fn area2(polys: &[ExPolygon]) -> Result<i128, CheckFailure> {
    let mut total = 0_i128;
    for e in polys {
        let a = signed_area2(&e.contour).map_err(|err| CheckFailure::Fixture(format!("{err}")))?;
        total += a.abs();
        for h in &e.holes {
            let ha = signed_area2(h).map_err(|err| CheckFailure::Fixture(format!("{err}")))?;
            total -= ha.abs();
        }
    }
    Ok(total)
}

/// The one territory checker: every classification cell of the subject,
/// clearance of every output vertex, and exact total area.
fn check_territory(
    case: &TerritoryCase,
    l: &TerritoryLimits,
    observed: &[ExPolygon],
) -> Result<TerritoryStats, CheckFailure> {
    validate_territory(case, l)?;
    let expectation = case.expectation();
    let (kind_rects, foreign, bar): (Vec<Rect>, Vec<Rect>, Vec<Rect>) = match &expectation {
        Expectation::Own(own) => (own.clone(), Vec::new(), Vec::new()),
        Expectation::Base { foreign, bar } => {
            let mut v = foreign.clone();
            v.extend(bar.iter().copied());
            (v, foreign.clone(), bar.clone())
        }
        Expectation::Unchanged => (Vec::new(), Vec::new(), Vec::new()),
    };
    let keep = |x: i64, y: i64| -> bool {
        if !case.subject.iter().any(|r| contains_strict(r, x, y)) {
            return false;
        }
        match &expectation {
            Expectation::Own(own) => own.iter().any(|r| contains_strict(r, x, y)),
            Expectation::Base { bar, .. } => !bar.iter().any(|r| contains_strict(r, x, y)),
            Expectation::Unchanged => true,
        }
    };
    let mut xs: Vec<i64> = Vec::new();
    let mut ys: Vec<i64> = Vec::new();
    for r in case.subject.iter().chain(kind_rects.iter()) {
        xs.extend([r.0, r.2]);
        ys.extend([r.1, r.3]);
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let cells = xs.len().saturating_sub(1) * ys.len().saturating_sub(1);
    if cells > l.max_cells {
        return fixture(format!("{cells} classification cells exceed max_cells"));
    }
    let mut expected_area2 = 0_i128;
    let mut stats = TerritoryStats::default();
    let mut cell_list = Vec::with_capacity(cells);
    for wy in ys.windows(2) {
        for wx in xs.windows(2) {
            let (mx, my) = ((wx[0] + wx[1]) / 2, (wy[0] + wy[1]) / 2);
            let want = keep(mx, my);
            if want {
                expected_area2 += 2 * i128::from(wx[1] - wx[0]) * i128::from(wy[1] - wy[0]);
            }
            cell_list.push((mx, my, want));
        }
    }
    if expected_area2 > 0 && observed.is_empty() {
        return Err(CheckFailure::FalseEmpty(format!(
            "{:?}: expected area2 {expected_area2}, observed nothing",
            case.mode
        )));
    }
    for (mx, my, want) in cell_list {
        let got = locate(pt(mx, my), observed)? == PointLocation::Inside;
        if want != got {
            let in_bar = bar.iter().any(|r| contains_strict(r, mx, my));
            return Err(if got && in_bar {
                CheckFailure::Clearance(format!(
                    "cell centre ({mx},{my}) survives inside the expanded foreign territory"
                ))
            } else {
                CheckFailure::Territory(format!(
                    "{:?}: cell centre ({mx},{my}) expected {want}, observed {got}",
                    case.mode
                ))
            });
        }
        stats.cells += 1;
        let in_subject = case.subject.iter().any(|r| contains_strict(r, mx, my));
        if in_subject {
            if want {
                stats.kept += 1;
            } else {
                stats.removed += 1;
                let in_bar = bar.iter().any(|r| contains_strict(r, mx, my));
                let in_foreign = foreign.iter().any(|r| contains_strict(r, mx, my));
                if in_bar && !in_foreign {
                    stats.clearance += 1;
                }
            }
        }
    }
    for e in observed {
        for p in e
            .contour
            .points
            .iter()
            .chain(e.holes.iter().flat_map(|h| h.points.iter()))
        {
            if !case
                .subject
                .iter()
                .any(|r| r.0 <= p.x && p.x <= r.2 && r.1 <= p.y && p.y <= r.3)
            {
                return Err(CheckFailure::Territory(format!(
                    "output vertex {p:?} lies outside the subject"
                )));
            }
            if bar.iter().any(|r| contains_strict(r, p.x, p.y)) {
                return Err(CheckFailure::Clearance(format!(
                    "output vertex {p:?} lies inside the expanded foreign territory"
                )));
            }
        }
    }
    let observed_area2 = area2(observed)?;
    if observed_area2 != expected_area2 {
        return Err(CheckFailure::Territory(format!(
            "{:?}: observed area2 {observed_area2}, expected {expected_area2}",
            case.mode
        )));
    }
    Ok(stats)
}

fn build_territory(
    mode: TerritoryMode,
    raw: [u32; TERRITORY_RAW],
    l: &TerritoryLimits,
) -> TerritoryCase {
    let half = l.max_abs / 2;
    let rect_in = |r: &[u32], lo_h: i64, hi_h: i64| -> Rect {
        let xs = even_increasing(&r[0..2], lo_h, hi_h);
        let ys = even_increasing(&r[2..4], -half, half);
        (xs[0], ys[0], xs[1], ys[1])
    };
    let subject: Vec<Rect> = match (mode, raw[0] % 2) {
        (TerritoryMode::EmptySubject, _) => Vec::new(),
        (_, 0) => vec![rect_in(&raw[1..5], -half, half)],
        _ => vec![rect_in(&raw[1..5], -half, -1), rect_in(&raw[5..9], 1, half)],
    };
    let width_units = 2 * pick(raw[9], 1, l.max_clearance / 2);
    let layer = u32::try_from(pick(raw[10], 0, 7)).expect("layer");
    let q_chain = chain_of(raw[11]);
    let f_chain = chain_of(raw[12]);
    let n_terr = 1 + (raw[13] % 2) as usize;
    let terr: Vec<Rect> = vec![
        rect_in(&raw[14..18], -half, half),
        rect_in(&raw[18..22], -half, half),
    ][..n_terr]
        .to_vec();
    let decoy = rect_in(&raw[22..26], -half, half);
    let entry = |layer: u32, source: &Source, rects: Vec<Rect>| TerritoryEntry {
        layer,
        source: source.clone(),
        rects,
    };
    let fam = |s: &Source, f: &str| (s.clone(), f.to_owned());

    if mode == TerritoryMode::Own {
        let query = src("object-a", "modifier-1", q_chain.clone());
        let chain_decoy = src("object-a", "modifier-1", other_chain(&q_chain));
        let foreign = src("object-a", "modifier-2", f_chain);
        return TerritoryCase {
            mode,
            layer,
            width_units,
            subject,
            entries: vec![
                // First, so a chain-blind lookup would find the wrong entry.
                entry(layer, &chain_decoy, vec![decoy]),
                entry(layer + 1, &query, vec![decoy]),
                entry(layer, &query, terr),
                entry(layer, &foreign, vec![decoy]),
            ],
            families: vec![
                fam(&query, TRADITIONAL),
                fam(&chain_decoy, FOREIGN_FAMILY),
                fam(&foreign, FOREIGN_FAMILY),
            ],
            query,
        };
    }

    let query = src("object-a", "base", q_chain.clone());
    let foreign = src("object-a", "modifier-2", f_chain.clone());
    let sibling = src("object-a", "modifier-2", other_chain(&f_chain));
    let same_region = src("object-a", "base", other_chain(&q_chain));
    let other_object = src("object-b", "modifier-2", f_chain);
    let foreign_rects = if mode == TerritoryMode::FullyRemoved {
        let b = subject.iter().fold(subject[0], |acc, r| {
            (
                acc.0.min(r.0),
                acc.1.min(r.1),
                acc.2.max(r.2),
                acc.3.max(r.3),
            )
        });
        vec![b]
    } else {
        terr
    };
    let entries = match mode {
        TerritoryMode::NoPartition => vec![
            entry(layer, &other_object, vec![decoy]),
            entry(layer + 1, &foreign, vec![decoy]),
        ],
        TerritoryMode::OwnFamilyOnly => vec![
            entry(layer, &sibling, vec![decoy]),
            entry(layer, &same_region, vec![decoy]),
            entry(layer + 1, &foreign, vec![decoy]),
        ],
        _ => vec![
            entry(layer, &same_region, vec![decoy]),
            entry(layer, &sibling, vec![decoy]),
            entry(layer, &foreign, foreign_rects),
            entry(layer, &other_object, vec![decoy]),
            entry(layer + 1, &foreign, vec![decoy]),
        ],
    };
    TerritoryCase {
        mode,
        layer,
        width_units,
        subject,
        entries,
        families: vec![
            fam(&query, TRADITIONAL),
            fam(&foreign, FOREIGN_FAMILY),
            fam(&sibling, TRADITIONAL),
            fam(&same_region, TRADITIONAL),
            fam(&other_object, FOREIGN_FAMILY),
        ],
        query,
    }
}

fn territory_strategy(
    l: &TerritoryLimits,
    modes: Vec<TerritoryMode>,
) -> BoxedStrategy<TerritoryCase> {
    let l = *l;
    (
        prop::sample::select(modes).no_shrink(),
        any::<[u32; TERRITORY_RAW]>(),
    )
        .prop_map(move |(mode, raw)| build_territory(mode, raw, &l))
        .boxed()
}

const MM: i64 = 10_000;

/// Subject `[0,10]²` mm, own territory `[0,4] x [0,10]` mm under chain
/// `[a, b]`; the order-distinct chain `[b, a]` owns `[6,10] x [0,10]` mm.
fn territory_own_witness() -> TerritoryCase {
    let query = src("object-a", "modifier-1", vec![paint_a(), paint_b()]);
    let swapped = src("object-a", "modifier-1", vec![paint_b(), paint_a()]);
    TerritoryCase {
        mode: TerritoryMode::Own,
        layer: 3,
        width_units: MM,
        subject: vec![(0, 0, 10 * MM, 10 * MM)],
        entries: vec![
            TerritoryEntry {
                layer: 3,
                source: swapped.clone(),
                rects: vec![(6 * MM, 0, 10 * MM, 10 * MM)],
            },
            TerritoryEntry {
                layer: 3,
                source: query.clone(),
                rects: vec![(0, 0, 4 * MM, 10 * MM)],
            },
        ],
        families: vec![
            (query.clone(), TRADITIONAL.to_owned()),
            (swapped, FOREIGN_FAMILY.to_owned()),
        ],
        query,
    }
}

/// Subject `[0,10]²` mm, foreign `[5,7] x [0,10]` mm, line width 1 mm.
fn territory_base_witness() -> TerritoryCase {
    let query = src("object-a", "base", Vec::new());
    let foreign = src("object-a", "modifier-2", vec![paint_a(), paint_b()]);
    TerritoryCase {
        mode: TerritoryMode::Base,
        layer: 3,
        width_units: MM,
        subject: vec![(0, 0, 10 * MM, 10 * MM)],
        entries: vec![TerritoryEntry {
            layer: 3,
            source: foreign.clone(),
            rects: vec![(5 * MM, 0, 7 * MM, 10 * MM)],
        }],
        families: vec![
            (query.clone(), TRADITIONAL.to_owned()),
            (foreign, FOREIGN_FAMILY.to_owned()),
        ],
        query,
    }
}

/// The base witness with its only territory moved to another object and
/// another layer.
fn territory_no_partition_witness() -> TerritoryCase {
    let mut case = territory_base_witness();
    case.mode = TerritoryMode::NoPartition;
    case.entries[0].source.object = "object-b".to_owned();
    let mut other_layer = case.entries[0].clone();
    other_layer.source.object = "object-a".to_owned();
    other_layer.layer = 4;
    case.entries.push(other_layer);
    case.families
        .push((case.entries[0].source.clone(), FOREIGN_FAMILY.to_owned()));
    case
}

fn territory_removed_witness() -> TerritoryCase {
    let mut case = territory_base_witness();
    case.mode = TerritoryMode::FullyRemoved;
    // The subject must sit inside the foreign territory grown by the line
    // width (foreign [5,7] x [0,10] grown by 1 mm is [4,8] x [-1,11]), so the
    // whole subject is removed rather than just its middle band.
    case.subject = vec![(5 * MM, MM, 7 * MM, 9 * MM)];
    case
}

fn territory_empty_witness() -> TerritoryCase {
    let mut case = territory_base_witness();
    case.mode = TerritoryMode::EmptySubject;
    case.subject.clear();
    case
}

#[derive(Debug, Default)]
struct TerritoryWitnessStats {
    own_kept: u64,
    own_removed: u64,
    base_kept: u64,
    base_clearance: u64,
    no_partition: u64,
    removed: u64,
    empty: u64,
}

fn territory_witness_observations(l: &TerritoryLimits) -> Result<TerritoryWitnessStats, String> {
    let mut out = TerritoryWitnessStats::default();
    let checked = |case: &TerritoryCase| -> Result<(Vec<ExPolygon>, TerritoryStats), String> {
        let observed = case.run();
        let stats = check_territory(case, l, &observed)
            .map_err(|e| format!("{:?} territory witness: {e}", case.mode))?;
        Ok((observed, stats))
    };
    let at = |polys: &[ExPolygon], x: i64, y: i64| -> Result<bool, String> {
        Ok(locate(pt(x, y), polys).map_err(|e| e.to_string())? == PointLocation::Inside)
    };

    let (obs, stats) = checked(&territory_own_witness())?;
    if !at(&obs, 2 * MM, 5 * MM)? || at(&obs, 6 * MM, 5 * MM)? || stats.kept == 0 {
        return Err("own witness must keep (2,5) and drop (6,5) mm".to_owned());
    }
    out.own_kept += stats.kept as u64;
    out.own_removed += stats.removed as u64;

    let (obs, stats) = checked(&territory_base_witness())?;
    if !at(&obs, 2 * MM, 5 * MM)?
        || !at(&obs, 9 * MM, 5 * MM)?
        || at(&obs, 45 * MM / 10, 5 * MM)?
        || at(&obs, 6 * MM, 5 * MM)?
        || stats.clearance == 0
    {
        return Err("base witness clearance classification".to_owned());
    }
    out.base_kept += stats.kept as u64;
    out.base_clearance += stats.clearance as u64;

    let (obs, stats) = checked(&territory_no_partition_witness())?;
    if !at(&obs, 6 * MM, 5 * MM)? || stats.removed != 0 {
        return Err("other object/layer territory clipped the subject".to_owned());
    }
    out.no_partition += 1;

    let (obs, _) = checked(&territory_removed_witness())?;
    if !obs.is_empty() {
        return Err("fully covered subject survived".to_owned());
    }
    out.removed += 1;

    let (obs, _) = checked(&territory_empty_witness())?;
    if !obs.is_empty() {
        return Err("empty subject produced output".to_owned());
    }
    out.empty += 1;
    Ok(out)
}

#[test]
fn proptest_support_planner_territory() {
    let run = PropertyRun::new(TERRITORY_ID, TERRITORY_TEST).expect("M-SUPPORT-TERRITORY context");
    let limits = territory_limits(&live_bounds(&run, &TERRITORY_KEYS)).expect("territory bounds");
    let strategy = territory_strategy(&limits, TERRITORY_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let w_own_kept = Cell::new(0_u64);
    let w_own_removed = Cell::new(0_u64);
    let w_base_kept = Cell::new(0_u64);
    let w_base_clearance = Cell::new(0_u64);
    let w_no_partition = Cell::new(0_u64);
    let w_removed = Cell::new(0_u64);
    let w_empty = Cell::new(0_u64);
    let cells = Cell::new(0_u64);
    let kept = Cell::new(0_u64);
    let removed = Cell::new(0_u64);
    let clearance = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 6] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let w = match territory_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_own_kept.set(w_own_kept.get() + w.own_kept);
        w_own_removed.set(w_own_removed.get() + w.own_removed);
        w_base_kept.set(w_base_kept.get() + w.base_kept);
        w_base_clearance.set(w_base_clearance.get() + w.base_clearance);
        w_no_partition.set(w_no_partition.get() + w.no_partition);
        w_removed.set(w_removed.get() + w.removed);
        w_empty.set(w_empty.get() + w.empty);

        let observed = case.run();
        let stats = match check_territory(&case, &limits, &observed) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{:?} territory case: {e}", case.mode)),
        };
        if matches!(
            case.mode,
            TerritoryMode::EmptySubject | TerritoryMode::FullyRemoved
        ) && !observed.is_empty()
        {
            return fail("an empty/fully removed case produced output");
        }
        let slot = TERRITORY_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        cells.set(cells.get() + stats.cells as u64);
        kept.set(kept.get() + stats.kept as u64);
        removed.set(removed.get() + stats.removed as u64);
        clearance.set(clearance.get() + stats.clearance as u64);
        Ok(())
    })
    .expect("M-SUPPORT-TERRITORY own intersection and base-minus-expanded-foreign cells");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_own_kept.get() > 0, "own kept witness unobserved");
    assert!(w_own_removed.get() > 0, "own removed witness unobserved");
    assert!(w_base_kept.get() > 0, "base kept witness unobserved");
    assert!(
        w_base_clearance.get() > 0,
        "base clearance witness unobserved"
    );
    assert!(w_no_partition.get() > 0, "no-partition witness unobserved");
    assert!(w_removed.get() > 0, "fully removed witness unobserved");
    assert!(w_empty.get() > 0, "empty-subject witness unobserved");
    println!(
        "M-SUPPORT-TERRITORY accepted_calls={} own={} base={} no_partition={} \
         own_family_only={} empty_subject={} fully_removed={} cells={} kept={} removed={} \
         clearance={} witness_own_kept={} witness_own_removed={} witness_base_kept={} \
         witness_base_clearance={} witness_no_partition={} witness_removed={} witness_empty={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        per_mode[4].get(),
        per_mode[5].get(),
        cells.get(),
        kept.get(),
        removed.get(),
        clearance.get(),
        w_own_kept.get(),
        w_own_removed.get(),
        w_base_kept.get(),
        w_base_clearance.get(),
        w_no_partition.get(),
        w_removed.get(),
        w_empty.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic mutation controls
// ─────────────────────────────────────────────────────────────────────────────

fn expect_failure<T: fmt::Debug>(
    result: Result<T, CheckFailure>,
    what: &str,
    class: impl Fn(&CheckFailure) -> bool,
) {
    match result {
        Ok(v) => panic!("{what}: the mutation was accepted ({v:?})"),
        Err(e) => assert!(class(&e), "{what}: rejected for the wrong reason: {e}"),
    }
}

fn is_cell(e: &CheckFailure) -> bool {
    matches!(e, CheckFailure::Cell(_))
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default()
}

fn expect_invalid_bounds<T: fmt::Debug>(
    label: &str,
    probe: BTreeMap<String, u64>,
    keys: &[&str],
    overrides: &[(&str, u64)],
    limits: impl Fn(&BTreeMap<String, u64>) -> Result<T, DomainError>,
) {
    limits(&probe).unwrap_or_else(|e| panic!("{label} probe bounds must be valid: {e}"));
    for key in keys {
        let mut b = probe.clone();
        b.remove(*key);
        assert!(
            matches!(limits(&b), Err(DomainError::InvalidBounds(_))),
            "{label}: missing {key} accepted"
        );
        let mut b = probe.clone();
        b.insert((*key).to_owned(), u64::MAX);
        assert!(
            matches!(limits(&b), Err(DomainError::ArithmeticOverflow)),
            "{label}: overflowing {key} accepted"
        );
    }
    for &(key, value) in overrides {
        let mut b = probe.clone();
        b.insert(key.to_owned(), value);
        assert!(
            matches!(limits(&b), Err(DomainError::InvalidBounds(_))),
            "{label}: {key}={value} accepted"
        );
    }
}

#[test]
fn traditional_planner_controls() {
    let _guard = CaseGuard::new();

    // ── Raster ────────────────────────────────────────────────────────────
    let rl = raster_limits(&raster_probe_bounds()).expect("raster probe bounds");
    let centre = raster_centre_witness();
    let obs = run_raster(&centre);
    let stats = check_raster(&centre, &rl, &obs).expect("unmodified centre witness");
    assert_eq!(stats.centre_outside, 1);
    assert_eq!(obs[4 + 1], 1, "cell (1,1) is covered");
    assert!(
        !centre.shapes[0].contains_doubled(30, 30),
        "its centre (15,15) is outside"
    );
    let mut centre_only = obs.clone();
    centre_only[4 + 1] = 0;
    expect_failure(
        check_raster(&centre, &rl, &centre_only),
        "centre-only rasterization",
        is_cell,
    );
    let hole = raster_hole_witness();
    let obs = run_raster(&hole);
    check_raster(&hole, &rl, &obs).expect("unmodified hole witness");
    assert_eq!(obs[3 * 8 + 3], 0, "hole-interior cell (3,3) is unset");
    let mut filled_hole = obs.clone();
    filled_hole[3 * 8 + 3] = 1;
    expect_failure(
        check_raster(&hole, &rl, &filled_hole),
        "filled hole cell",
        is_cell,
    );
    expect_failure(
        check_raster(&hole, &rl, &obs[..obs.len() - 1]),
        "short raster buffer",
        |e| matches!(e, CheckFailure::Size(_)),
    );
    let notch = raster_notch_witness();
    check_raster(&notch, &rl, &run_raster(&notch)).expect("unmodified notch witness");
    for kind in NO_WORKS {
        let case = raster_no_work_witness(kind);
        let obs = run_raster(&case);
        check_raster(&case, &rl, &obs).expect("no-work witness");
        if !obs.is_empty() {
            let mut work = obs.clone();
            work[0] = 1;
            expect_failure(
                check_raster(&case, &rl, &work),
                "no-work with output",
                is_cell,
            );
        }
    }
    // An on-lattice vertex is outside this restricted model's domain.
    let mut on_line = raster_centre_witness();
    on_line.shapes[0].x0 = 10;
    expect_failure(
        check_raster(&on_line, &rl, &run_raster(&on_line)),
        "vertex on a grid line",
        |e| matches!(e, CheckFailure::Fixture(_)),
    );

    // ── Hole fill ─────────────────────────────────────────────────────────
    let hl = hole_limits(&hole_probe_bounds()).expect("hole probe bounds");
    let cascade = hole_cascade_witness();
    let out = fill_grid_holes(&cascade.grid, cascade.size);
    check_holes(&cascade, &hl, &out).expect("unmodified cascade");
    assert_eq!(
        (out[2 * 5 + 1], out[2 * 5 + 2]),
        (1, 0),
        "(1,2)=1 and (2,2)=0"
    );
    let mut in_place = out.clone();
    in_place[2 * 5 + 2] = 1;
    expect_failure(
        check_holes(&cascade, &hl, &in_place),
        "in-place cascading fill",
        is_cell,
    );
    let gap = hole_two_gap_witness();
    let out = fill_grid_holes(&gap.grid, gap.size);
    check_holes(&gap, &hl, &out).expect("unmodified two-cell gap");
    let mut bridged = out.clone();
    bridged[6 + 2] = 1;
    expect_failure(
        check_holes(&gap, &hl, &bridged),
        "bridged two-cell gap",
        is_cell,
    );
    let tiny = hole_tiny_witness();
    let mut tiny_out = fill_grid_holes(&tiny.grid, tiny.size);
    check_holes(&tiny, &hl, &tiny_out).expect("tiny unchanged");
    tiny_out[0] = 0;
    expect_failure(
        check_holes(&tiny, &hl, &tiny_out),
        "changed tiny input",
        is_cell,
    );

    // ── Grid, seed fill and samples ───────────────────────────────────────
    let gl = grid_limits(&grid_probe_bounds()).expect("grid probe bounds");
    let check_seed = |case: &SeedCase, observed: &[u8]| {
        check_seed_fill(
            &case.input,
            &case.mask,
            case.size,
            case.blocks,
            case.os,
            observed,
        )
    };
    let clear = seed_control((2, 1), &[]);
    let out = clear.run();
    check_seed(&clear, &out).expect("unmodified clear-mask block fill");
    assert_eq!(cell6(&out, 3, 1), 0, "(3,1) in block 1 stays 0");
    let mut crossing = out.clone();
    crossing[6 + 3] = 1;
    expect_failure(check_seed(&clear, &crossing), "block-crossing fill", |e| {
        matches!(e, CheckFailure::SeedLeak(_))
    });
    let src_masked = seed_control((1, 1), &[(1, 1)]);
    let out = src_masked.run();
    check_seed(&src_masked, &out).expect("unmodified masked source");
    let mut leaked = out.clone();
    leaked[6 + 2] = 1;
    expect_failure(
        check_seed(&src_masked, &leaked),
        "propagation from a masked seed",
        |e| matches!(e, CheckFailure::SeedLeak(_)),
    );
    let mut cleared = out.clone();
    cleared[6 + 1] = 0;
    expect_failure(
        check_seed(&src_masked, &cleared),
        "masked seed removed",
        |e| matches!(e, CheckFailure::Masked(_)),
    );
    let dst_masked = seed_control((1, 1), &[(2, 1)]);
    let out = dst_masked.run();
    check_seed(&dst_masked, &out).expect("unmodified masked destination");
    let mut written = out.clone();
    written[6 + 2] = 1;
    expect_failure(
        check_seed(&dst_masked, &written),
        "masked destination written",
        |e| matches!(e, CheckFailure::Masked(_)),
    );
    let mut ring = clear.run();
    ring[0] = 1;
    expect_failure(check_seed(&clear, &ring), "outer ring written", |e| {
        matches!(e, CheckFailure::Ring(_))
    });

    // seed_fill_block: early return on zero size; equal-length assertion.
    let mut g = vec![1_u8; 24];
    seed_fill_block(&mut g, (6, 4), &[0; 3], (2, 1), 0);
    assert_eq!(
        g,
        vec![1; 24],
        "oversampling 0 returns before any check or write"
    );
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let mut g = vec![0_u8; 24];
        seed_fill_block(&mut g, (6, 4), &[0; 23], (2, 1), 2);
    }));
    let message = panic_message(
        panicked
            .as_ref()
            .expect_err("length mismatch must panic")
            .as_ref(),
    );
    assert!(
        message.contains("mask and grid must have the same extent"),
        "unexpected mismatch panic: {message}"
    );

    // Sample selection: adding the unsampled island is a failure.
    let two = two_island_case();
    let sg = SupportGrid::for_test(two.params(), two.buffer(), two.trimming());
    let all = sg.extract_islands(0, false);
    let outside = two.global_outside();
    check_islands(&two.specs(), &outside, &all).expect("both islands extracted");
    assert_eq!(two.samples(), vec![pt(15, 15)], "the charter sample");
    let kept = sg.filter_islands_by_samples(all.clone(), &two.samples());
    check_islands(&two.selected_specs(), &outside, &kept)
        .map_err(as_selection)
        .expect("only the sampled island");
    let mut with_second = kept.clone();
    let second = all
        .iter()
        .find(|e| ring_bbox(&e.contour) == Some((40, 10, 50, 20)))
        .expect("second island")
        .clone();
    with_second.push(second);
    expect_failure(
        check_islands(&two.selected_specs(), &outside, &with_second).map_err(as_selection),
        "unsampled island selected",
        |e| matches!(e, CheckFailure::Selection(_)),
    );
    expect_failure(
        check_islands(&[], &outside, &kept).map_err(as_selection),
        "no samples but an island selected",
        |e| matches!(e, CheckFailure::Selection(_)),
    );

    // Trim: the retained (17,15) material of the untrimmed island fails.
    let trim = trim_island_case();
    let sg = SupportGrid::for_test(trim.params(), trim.buffer(), trim.trimming());
    let trimmed = sg.extract_islands(0, false);
    let spec = trim.specs();
    assert_eq!(spec[0].sample, pt(12, 15));
    assert!(spec[0].outside.contains(&pt(17, 15)));
    assert_eq!(trim.trimming(), vec![rect_poly((15, 10, 20, 20))]);
    check_islands(&spec, &trim.global_outside(), &trimmed).expect("trimmed island");
    let untrimmed =
        SupportGrid::for_test(trim.params(), trim.buffer(), Vec::new()).extract_islands(0, false);
    expect_failure(
        check_islands(&spec, &trim.global_outside(), &untrimmed),
        "missing trim",
        |e| matches!(e, CheckFailure::Island(_)),
    );

    // Hole exclusion: dropping the extracted hole fails.
    let holed = holed_island_case();
    let sg = SupportGrid::for_test(holed.params(), holed.buffer(), Vec::new());
    let islands = sg.extract_islands(holed.offset, holed.fill_holes);
    check_islands(&holed.specs(), &holed.global_outside(), &islands).expect("holed island");
    let mut no_hole = islands.clone();
    no_hole[0].holes.clear();
    expect_failure(
        check_islands(&holed.specs(), &holed.global_outside(), &no_hole),
        "hole filled in extraction",
        |e| matches!(e, CheckFailure::Island(_)),
    );

    // Halo: the original polygon instead of the block-local extent fails.
    let scene = halo_scene();
    let grid = scene.build();
    check_scene(&scene, &gl, &grid).expect("halo scene");
    let islands = grid.extract_islands(0, true);
    check_islands(&[halo_spec()], &[], &islands).expect("block-local halo");
    let polygon_only = vec![scene.support[0].expoly()];
    expect_failure(
        check_islands(&[halo_spec()], &[], &polygon_only),
        "polygon subset instead of the block halo",
        |e| matches!(e, CheckFailure::Island(_)),
    );

    // The legal boundary offset is accepted; one more unit hits the guard.
    let legal = legal_offset_max(10);
    assert_eq!(legal, 4, "largest o with 2o < 10 - 1");
    let sg = SupportGrid::for_test(two.params(), two.buffer(), Vec::new());
    assert_eq!(sg.extract_islands(legal, false).len(), 2);
    assert_eq!(sg.extract_islands(-legal, false).len(), 2);
    for bad in [legal + 1, -(legal + 1)] {
        let panicked = catch_unwind(AssertUnwindSafe(|| sg.extract_islands(bad, false)));
        let message = panic_message(
            panicked
                .as_ref()
                .expect_err("invalid offset must panic")
                .as_ref(),
        );
        assert!(
            message.contains("escapes the macro cell"),
            "offset {bad}: unexpected panic {message}"
        );
    }

    // ── Traditional territory ─────────────────────────────────────────────
    let tl = territory_limits(&territory_probe_bounds()).expect("territory probe bounds");
    let own = territory_own_witness();
    let obs = own.run();
    check_territory(&own, &tl, &obs).expect("unmodified own witness");
    let subject: Vec<ExPolygon> = own.subject.iter().copied().map(rect_poly).collect();
    let other_chain_result = host::clip_polygons(
        &subject,
        &[rect_poly((6 * MM, 0, 10 * MM, 10 * MM))],
        ClipOperation::Intersection,
    );
    expect_failure(
        check_territory(&own, &tl, &other_chain_result),
        "the other ordered-chain territory",
        |e| matches!(e, CheckFailure::Territory(_)),
    );
    expect_failure(
        check_territory(&own, &tl, &[]),
        "false all-empty own result",
        |e| matches!(e, CheckFailure::FalseEmpty(_)),
    );
    let base = territory_base_witness();
    let obs = base.run();
    check_territory(&base, &tl, &obs).expect("unmodified base witness");
    let no_clearance = host::clip_polygons(
        &subject,
        &[rect_poly((5 * MM, 0, 7 * MM, 10 * MM))],
        ClipOperation::Difference,
    );
    assert_eq!(
        locate(pt(45 * MM / 10, 5 * MM), &no_clearance).expect("point"),
        PointLocation::Inside,
        "the mutation really inserts (4.5,5) mm"
    );
    expect_failure(
        check_territory(&base, &tl, &no_clearance),
        "omitted clearance",
        |e| matches!(e, CheckFailure::Clearance(_)),
    );
    expect_failure(
        check_territory(&base, &tl, &subject),
        "foreign bar not removed",
        |e| matches!(e, CheckFailure::Clearance(_)),
    );
    let np = territory_no_partition_witness();
    expect_failure(
        check_territory(&np, &tl, &[]),
        "no-partition subject dropped",
        |e| matches!(e, CheckFailure::FalseEmpty(_)),
    );

    // ── Bound-construction rejection matrices ────────────────────────────
    expect_invalid_bounds(
        "raster",
        raster_probe_bounds(),
        &RASTER_KEYS,
        &[
            ("max_abs_units", 79),
            ("max_pixel_units", 9),
            ("max_grid_width", 7),
            ("max_grid_height", 7),
            ("max_cells", 63),
        ],
        raster_limits,
    );
    expect_invalid_bounds(
        "holes",
        hole_probe_bounds(),
        &HOLES_KEYS,
        &[
            ("max_grid_width", 5),
            ("max_grid_height", 4),
            ("max_cells", 24),
        ],
        hole_limits,
    );
    expect_invalid_bounds(
        "grid",
        grid_probe_bounds(),
        &GRID_KEYS,
        &[
            ("max_abs_units", 39_999),
            ("max_pixel_units", 4_999),
            ("max_grid_width", 17),
            ("max_grid_height", 5),
            ("max_blocks", 1),
            ("max_cells", 107),
        ],
        grid_limits,
    );
    expect_invalid_bounds(
        "territory",
        territory_probe_bounds(),
        &TERRITORY_KEYS,
        &[
            ("max_abs_units", 99_999),
            ("max_clearance_units", 9_998),
            ("max_clearance_units", 10_001),
            ("max_cells", 120),
        ],
        territory_limits,
    );

    // The witnesses themselves are inside the smallest bounds.
    raster_witness_observations(&rl).expect("raster witnesses");
    hole_witness_observations(&hl).expect("hole witnesses");
    grid_witness_observations(&gl).expect("grid witnesses");
    territory_witness_observations(&tl).expect("territory witnesses");
    println!("traditional_planner_controls: all mutations rejected by the shared checkers");
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

/// Drive real `simplify`/`complicate` transitions from a deterministic,
/// nonminimal start; `validate` independently checks every exposed value
/// (mode survival plus the shared checker on a fresh SUT run).
fn shrink_probe<S>(
    label: &str,
    strategy: &S,
    validate: &dyn Fn(&S::Value),
    complexity: &dyn Fn(&S::Value) -> i128,
) where
    S: Strategy,
    S::Value: Clone + PartialEq + fmt::Debug,
{
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    let initial = tree.current();
    validate(&initial);
    let initial_complexity = complexity(&initial);
    let mut accepted = 0_usize;
    while tree.simplify() {
        accepted += 1;
        assert!(
            accepted < MAX_SHRINK_STEPS,
            "{label}: simplify never exhausted"
        );
        validate(&tree.current());
    }
    let minimal_complexity = complexity(&tree.current());
    assert!(accepted > 0, "{label}: false-only shrink probe");
    assert!(
        minimal_complexity < initial_complexity,
        "{label}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
    );

    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    assert_eq!(
        tree.current(),
        initial,
        "{label}: the deterministic start is stable"
    );
    let (mut simplified, mut complicated, mut changed) = (0_usize, 0_usize, false);
    while tree.simplify() {
        simplified += 1;
        assert!(
            simplified < MAX_SHRINK_STEPS,
            "{label}: alternation never ended"
        );
        let current = tree.current();
        validate(&current);
        changed |= current != initial;
        if tree.complicate() {
            complicated += 1;
        }
        validate(&tree.current());
    }
    assert!(simplified > 0 && changed, "{label}: no real transition");
    let exhausted = tree.current();
    assert!(
        !tree.simplify(),
        "{label}: an exhausted tree simplified again"
    );
    assert_eq!(
        tree.current(),
        exhausted,
        "{label}: a false simplify moved the value"
    );
    let mut restores = 0_usize;
    while tree.complicate() {
        restores += 1;
        assert!(
            restores < MAX_SHRINK_STEPS,
            "{label}: complicate never ended"
        );
        validate(&tree.current());
    }
    let settled = tree.current();
    assert!(!tree.complicate(), "{label}: complicate after exhaustion");
    assert_eq!(
        tree.current(),
        settled,
        "{label}: a false complicate moved the value"
    );
    println!(
        "{label} phase1_simplify={accepted} complexity {initial_complexity}->{minimal_complexity} \
         phase2_simplify={simplified} complicate={complicated} restores={restores}"
    );
}

fn assert_nonshrinking<T: Clone + fmt::Debug + PartialEq + 'static>(label: &str, values: Vec<T>) {
    let mut runner = TestRunner::deterministic();
    let mut tree = prop::sample::select(values)
        .no_shrink()
        .new_tree(&mut runner)
        .expect("selector tree");
    let before = tree.current();
    assert!(!tree.simplify(), "{label} selector simplified");
    assert!(!tree.complicate(), "{label} selector complicated");
    assert_eq!(tree.current(), before);
}

fn shape_complexity(shapes: &[Shape], base: Point2) -> i128 {
    shapes
        .iter()
        .flat_map(|s| {
            s.xs()
                .into_iter()
                .map(move |x| x - base.x)
                .chain(s.ys().into_iter().map(move |y| y - base.y))
        })
        .map(i128::from)
        .sum::<i128>()
        + shapes.len() as i128
}

#[test]
fn traditional_planner_shrink_bounds() {
    let _guard = CaseGuard::new();

    let rl = raster_limits(&raster_probe_bounds()).expect("raster probe bounds");
    for mode in RASTER_MODES {
        let strategy = raster_strategy(&rl, vec![mode], vec![NoWork::ShortRing]);
        let validate = |case: &RasterCase| {
            assert_eq!(case.mode, mode, "the raster mode must survive shrinking");
            let stats = check_raster(case, &rl, &run_raster(case))
                .unwrap_or_else(|e| panic!("shrunk {mode:?} raster case: {e}"));
            if mode != RasterMode::NoWork {
                assert!(stats.set > 0, "{mode:?} lost its material");
            }
        };
        let complexity = |case: &RasterCase| {
            let m = rl.max_abs;
            i128::from(case.pixel + case.origin.x + m + case.origin.y + m)
                + (case.size.0 + case.size.1) as i128
                + shape_complexity(&case.shapes, case.origin)
        };
        shrink_probe(
            &format!("raster {mode:?}"),
            &strategy,
            &validate,
            &complexity,
        );
    }
    for kind in NO_WORKS {
        let strategy = raster_strategy(&rl, vec![RasterMode::NoWork], vec![kind]);
        let validate = |case: &RasterCase| {
            assert_eq!(case.no_work, Some(kind), "the no-work kind must survive");
            check_raster(case, &rl, &run_raster(case)).expect("shrunk no-work case");
        };
        let complexity = |case: &RasterCase| {
            i128::from(case.pixel + case.origin.x + case.origin.y + 2 * rl.max_abs)
                + (case.size.0 + case.size.1) as i128
        };
        shrink_probe(
            &format!("raster {kind:?}"),
            &strategy,
            &validate,
            &complexity,
        );
    }

    let hl = hole_limits(&hole_probe_bounds()).expect("hole probe bounds");
    for mode in HOLE_MODES {
        let strategy = hole_strategy(&hl, vec![mode]);
        let validate = |case: &HoleCase| {
            assert_eq!(case.mode, mode, "the hole mode must survive shrinking");
            check_holes(case, &hl, &fill_grid_holes(&case.grid, case.size))
                .unwrap_or_else(|e| panic!("shrunk {mode:?} hole case: {e}"));
        };
        let complexity = |case: &HoleCase| {
            (case.size.0 + case.size.1 + case.grid.len()) as i128
                + case.grid.iter().map(|&v| i128::from(v)).sum::<i128>()
        };
        shrink_probe(
            &format!("holes {mode:?}"),
            &strategy,
            &validate,
            &complexity,
        );
    }

    let gl = grid_limits(&grid_probe_bounds()).expect("grid probe bounds");
    for mode in GRID_MODES {
        let strategy = grid_strategy(&gl, vec![mode]);
        let validate = |case: &GridCase| {
            assert_eq!(case.mode, mode, "the grid mode must survive shrinking");
            check_grid_case(case, &gl).unwrap_or_else(|e| panic!("shrunk {mode:?} grid case: {e}"));
        };
        let complexity = |case: &GridCase| -> i128 {
            let m = gl.max_abs;
            match &case.body {
                GridBody::Scene(s) => {
                    s.combo as i128
                        + shape_complexity(&s.support, pt(-m, -m))
                        + shape_complexity(&s.trim, pt(-m, -m))
                }
                GridBody::Seed(s) => {
                    (s.os + s.blocks.0 + s.blocks.1) as i128
                        + s.input
                            .iter()
                            .chain(&s.mask)
                            .map(|&v| i128::from(v))
                            .sum::<i128>()
                }
                GridBody::Islands(ic) => {
                    i128::from(ic.pixel + ic.offset.abs() + ic.origin.x + ic.origin.y + 2 * m)
                        + (ic.size.0 + ic.size.1) as i128
                        + ic.islands
                            .iter()
                            .map(|d| {
                                (d.c0 + d.c1 + d.r0 + d.r1) as i128
                                    + i128::from(d.hole)
                                    + i128::from(d.selected)
                                    + i128::from(d.trim_x.map_or(0, |t| t + m + 1))
                            })
                            .sum::<i128>()
                        + i128::from(ic.fill_holes)
                        + i128::from(ic.decoys)
                        + i128::from(!ic.no_samples)
                }
            }
        };
        shrink_probe(&format!("grid {mode:?}"), &strategy, &validate, &complexity);
    }

    let tl = territory_limits(&territory_probe_bounds()).expect("territory probe bounds");
    for mode in TERRITORY_MODES {
        let strategy = territory_strategy(&tl, vec![mode]);
        let validate = |case: &TerritoryCase| {
            assert_eq!(case.mode, mode, "the territory mode must survive shrinking");
            check_territory(case, &tl, &case.run())
                .unwrap_or_else(|e| panic!("shrunk {mode:?} territory case: {e}"));
        };
        let complexity = |case: &TerritoryCase| -> i128 {
            let m = tl.max_abs;
            let rects = case
                .subject
                .iter()
                .chain(case.entries.iter().flat_map(|e| e.rects.iter()));
            rects
                .map(|r| i128::from(r.0 + r.1 + r.2 + r.3 + 4 * m))
                .sum::<i128>()
                + i128::from(case.width_units)
                + i128::from(case.layer)
                + case.query.chain.len() as i128
        };
        shrink_probe(
            &format!("territory {mode:?}"),
            &strategy,
            &validate,
            &complexity,
        );
    }

    // The mode selectors themselves never shrink: a shrinking mode tree could
    // switch a witness's class mid-search.
    assert_nonshrinking("raster mode", RASTER_MODES.to_vec());
    assert_nonshrinking("raster no-work", NO_WORKS.to_vec());
    assert_nonshrinking("hole mode", HOLE_MODES.to_vec());
    assert_nonshrinking("grid mode", GRID_MODES.to_vec());
    assert_nonshrinking("territory mode", TERRITORY_MODES.to_vec());
}
