//! Opt-in property tests for the infill linker's graph, connection and chain
//! seams.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_13_pbt-infill`):
//!
//! * `M-LINKER-GRAPH` — independently computed ring identity, perimeter,
//!   shortest arc (strictly shorter and tie), connector corner walks and
//!   bounded stubs on the real `BoundaryInfillGraph`, `BoundaryRing::
//!   directed_distance`, `contour_connector` and `contour_stub`.
//! * `M-LINKER-CONNECT` — every eligible source path preserved as a contiguous
//!   ordered-or-reversed subsequence of exactly one `connect_infill` output,
//!   added connector/stub geometry walked on the constructed ring, an
//!   independent compatibility predicate and no cross-ring extrusion.
//! * `M-LINKER-CHAIN` — `chain_or_connect_infill` on both sides of
//!   `AnchorParams::dont_connect`, and the actual `orchestrate_infill` identity,
//!   ownership, pass-through and locked-path seams.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! The linker's submodules are already public, so these properties call the
//! same seams the module's own driver calls. They do not widen any API. The
//! `proptest_` bodies require the foundation runner's `PNP_PROPERTY_CONTEXT`;
//! `linker_controls` and `linker_shrink_bounds` need no context.
//!
//! # Independent oracles
//!
//! Every fixture ring is an axis-aligned integer polygon (`RingModel`). Its
//! perimeter, vertex arc offsets, shortest forward/backward arc (ties go
//! `Forward`, the documented `directed_distance` contract), the corner list of
//! a walk and a stub's corners/end point are computed by exact `i64`
//! arithmetic from the fixture dimensions. `project`, `directed_distance`,
//! `vertices_between`, `clip_batched` and every SUT walk are **never** used to
//! build an expected value; `project` is only compared against the model to
//! prove the SUT sees constructed endpoints on the constructed ring.
//!
//! Exact ring-walk claims are restricted to endpoints constructed on a ring
//! edge at least `VERTEX_MARGIN_UNITS` (3 units) from every vertex: production
//! deliberately merges arc positions closer than `ARC_EPSILON_UNITS` (1 unit,
//! `modules/core-modules/infill-linker/src/graph.rs`), so a coincident-vertex
//! endpoint is a different, deliberately excluded contract.
//!
//! # Tolerances (derived, not observed)
//!
//! * Grid: `units_to_mm(u)` rounds one exact `i64 -> f32` (|u| <= 2^24) and one
//!   division, so `|f64(x)·1e4 − u| <= |u|·2^-24`; `grid_tol_units` uses
//!   `2·(max_abs+1)·2^-24` (0.12 units at the 1 000 000-unit cap).
//! * Stub lerp: `contour_stub` rounds two vertex conversions, one difference,
//!   one f32 fraction, one product and one sum, each bounded by
//!   `2·max_abs·2^-24` units; `lerp_tol_units` uses `16·(max_abs+1)·2^-24`
//!   (< 1 unit), below the 3-unit vertex margin and the 1 000-unit cell.
//! * Width compatibility: widths stay in `[0.25, 0.5)` mm or are the exact
//!   `c`, `2c`, `next_up(2c)` control values (`c = 0.000001_f32`), where f32
//!   subtraction is exact, so the independent f64 comparison equals the
//!   production f32 one. Compatible/incompatible sides are 33/34 ulps
//!   (`33·2^-25 <= c < 34·2^-25`).
//! * Orchestrate segments: `ORCH_SEG_TOL_UNITS = 4` covers the documented
//!   one-unit clip inflation, the grid error and the stub lerp error; rows are
//!   separated from every other horizontal feature by >= 10 000 units.
//!
//! ## Strict-anchor band (`ANCHOR_BAND_UNITS = 1.0`)
//!
//! Production decides a direct join with `candidate.distance <
//! anchor_length_max_units` (strict). `candidate.distance` is
//! `BoundaryRing::directed_distance` evaluated at the arc positions from
//! `boundary_position`, and `project_on_ring`
//! (`modules/core-modules/infill-linker/src/graph.rs`) derives each of those
//! from `segment_start + parameter * segment_len` with a f64 quotient and
//! product. On a constructed on-ring integer endpoint that recovers the exact
//! arc only up to f64 rounding: the retained extended-tier seed
//! 12768350826874075087 produced `project((13518, -1)) = 13519.000000000002`
//! and therefore `directed_distance = 3109.999999999998` for an exact arc of
//! 3110 units, so the strict `<` flipped against an anchor of exactly 3110
//! units. The measured projection error is tiny: `project_on_ring` rounds
//! `point_dx*dx`, `segment_len^2`, their quotient, `parameter*segment_len` and
//! the sum (five relative errors of at most `2^-53`), i.e.
//! `|prod - exact| <= 5·max_abs·2^-53` (< 1e-10 units at the 1 000 000-unit
//! cap), so the comparison is effectively an **equality tolerance for off-by-one
//! arc rounding**, not a continuation of a geometric epsilon. It is deliberately
//! not derived from `ARC_EPSILON_UNITS` (which governs walk vertex inclusion, a
//! different question): `ANCHOR_BAND_UNITS = 1.0` is the smallest band that
//! admits integer arcs of `anchor ± 1`, and an off-by-one-unit join in the SUT
//! would not be distinguished at the boundary. That residual is accepted and
//! openly recorded rather than dressed up as a derivation. `mm_to_units(units_to_mm(anchor))`
//! round-trips exactly because every anchor is checked with `units_round_trip`.
//! When `|exact_arc - anchor| <= 1` either the join or the stub branch is
//! accepted. Outside it the oracle is strict —
//! `exact_arc < anchor - 1` must join, `exact_arc > anchor + 1` must not — and
//! every observed junction still has to be on-ring, compatible and no longer
//! than `anchor + 1`. `linker_connect_controls` pins both edges and the
//! retained case.
//!
//! `DEV-110` (stub neighbour bookkeeping) stays a recorded residual: stub
//! runs are checked to lie on their ring and to stay within the anchor length,
//! never against a copied budget algorithm.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Re-derived watched types
//! (`pub` structs with >= 5 named fields under `crates/*/src`) used here:
//!
//! * `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) — `endpoint_point`
//!   (FRU over `Default::default()`), `with_xy` (FRU over the copied point).
//! * `ExtrusionPath3D` (same file) — `make_path` (FRU over
//!   `extrusion_path3d_base`) and `locked_path` (FRU over `make_path`).
//! * `InfillRegion` (same file) — `prior_region` (FRU over
//!   `Default::default()`).
//!
//! `GeometryBounds` is not constructed. `RegionOrigin` (3 fields),
//! `AnchorParams` (2 fields, module-local), `ExPolygon`/`Polygon`/`Point2`
//! (< 5 fields) and every private fixture struct are not watched.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::Debug;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    mm_to_units, units_to_mm, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillRegion,
    PaintValue, Point2, Point3WithWidth, Polygon,
};
use slicer_sdk::builders::{InfillOutputBuilder, RegionOrigin};
use slicer_sdk::test_support::fixtures::{
    extrusion_path3d_base, ConfigViewBuilder, PerimeterRegionViewBuilder,
};
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::views::PerimeterRegionView;

use crate::connect::{chain_or_connect_infill, connect_infill, AnchorParams};
use crate::graph::{contour_connector, contour_stub, BoundaryInfillGraph, RingDirection};
use crate::orchestrate::orchestrate_infill;

const GRAPH_ID: &str = "M-LINKER-GRAPH";
const GRAPH_TEST: &str = "property_tests::proptest_linker_ring_walks";
const CONNECT_ID: &str = "M-LINKER-CONNECT";
const CONNECT_TEST: &str = "property_tests::proptest_linker_connection_compatibility";
const CHAIN_ID: &str = "M-LINKER-CHAIN";
const CHAIN_TEST: &str = "property_tests::proptest_linker_chain_and_origins";

const GRAPH_BOUND_KEYS: [&str; 5] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
];
const CONNECT_BOUND_KEYS: [&str; 4] = [
    "max_abs_units",
    "max_paths",
    "max_points",
    "max_anchor_units",
];
const CHAIN_BOUND_KEYS: [&str; 5] = [
    "max_abs_units",
    "max_paths",
    "max_points",
    "max_regions",
    "max_anchor_units",
];

/// Smallest fixture cell (0.1 mm). Rings are at least three cells per side.
const MIN_CELL_UNITS: i64 = 1_000;
/// Widest shape (`HoleAndComponent` / `TwoComponents`) in cells: 6 + 3 + 6.
const SHAPE_SPAN_CELLS: i64 = 15;
/// f32 grid round-trip domain (100 mm).
const MAX_GRID_ABS_UNITS: i64 = 1_000_000;
/// Every constructed endpoint stays this far (circularly) from every vertex.
const VERTEX_MARGIN_UNITS: i64 = 3;
/// Cross-endpoint separation for the strict-anchor and stub classes, so the
/// shortest joining arc is at least `DONT_CONNECT_UNITS`.
const STRICT_SEPARATION_UNITS: i64 = 600;
/// `AnchorParams::DONT_CONNECT_MAX_MM` (0.05 mm) in units.
const DONT_CONNECT_UNITS: i64 = 500;
const MAX_QUERIES: usize = 4;
const WIDTH_LIMIT_MM: f32 = 0.000001;
const BASE_WIDTH_MM: f32 = 0.4;
const WIDTH_ULPS_COMPATIBLE: u32 = 33;
const WIDTH_ULPS_INCOMPATIBLE: u32 = 34;
const ROW_PITCH_UNITS: i64 = 10_000;
const ROW_OVERSHOOT_UNITS: i64 = 5_000;
const MIN_REGION_WIDTH_UNITS: i64 = 60_000;
const REGION_WIDTH_SPAN_UNITS: i64 = 40_000;
const ORCH_SEG_TOL_UNITS: f64 = 4.0;
const INFILL_OVERLAP: f32 = 0.45;
const DEFAULT_LINE_WIDTH: f32 = 0.4;
const PROBE_TREES: usize = 16;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

fn inc(cell: &Cell<u64>) {
    cell.set(cell.get() + 1);
}

fn add(cell: &Cell<u64>, value: u64) {
    cell.set(cell.get() + value);
}

fn bits(value: f32) -> u32 {
    value.to_bits()
}

fn grid_tol_units(max_abs: i64) -> f64 {
    2.0 * (max_abs as f64 + 1.0) * f64::powi(2.0, -24)
}

fn lerp_tol_units(max_abs: i64) -> f64 {
    16.0 * (max_abs as f64 + 1.0) * f64::powi(2.0, -24)
}

fn units_of(point: &Point3WithWidth) -> (f64, f64) {
    (f64::from(point.x) * 10_000.0, f64::from(point.y) * 10_000.0)
}

fn grid_of(point: &Point3WithWidth, tol: f64) -> Option<(i64, i64)> {
    let (x, y) = units_of(point);
    let (rx, ry) = (x.round(), y.round());
    ((x - rx).abs() <= tol && (y - ry).abs() <= tol).then_some((rx as i64, ry as i64))
}

fn point_bits_eq(left: &Point3WithWidth, right: &Point3WithWidth) -> bool {
    bits(left.x) == bits(right.x)
        && bits(left.y) == bits(right.y)
        && bits(left.z) == bits(right.z)
        && bits(left.width) == bits(right.width)
        && bits(left.flow_factor) == bits(right.flow_factor)
        && left.overhang_quartile == right.overhang_quartile
        && bits(left.dist_to_top_mm) == bits(right.dist_to_top_mm)
        && left.overhang_distance_mm.map(f32::to_bits)
            == right.overhang_distance_mm.map(f32::to_bits)
}

/// Every non-planar field of a point equals the anchor's (stub contract).
fn same_except_xy(left: &Point3WithWidth, anchor: &Point3WithWidth) -> bool {
    point_bits_eq(&with_xy(left, anchor.x, anchor.y), anchor)
}

fn path_meta_eq(left: &ExtrusionPath3D, right: &ExtrusionPath3D) -> bool {
    left.role == right.role
        && bits(left.speed_factor) == bits(right.speed_factor)
        && left.tool_index == right.tool_index
        && left.order_lock == right.order_lock
}

fn path_bits_eq(left: &ExtrusionPath3D, right: &ExtrusionPath3D) -> bool {
    path_meta_eq(left, right)
        && left.points.len() == right.points.len()
        && left
            .points
            .iter()
            .zip(&right.points)
            .all(|(a, b)| point_bits_eq(a, b))
}

fn reversed(path: &ExtrusionPath3D) -> ExtrusionPath3D {
    let mut copy = path.clone();
    copy.points.reverse();
    copy
}

fn endpoint_point(units: (i64, i64), z: f32, width: f32, flow_factor: f32) -> Point3WithWidth {
    Point3WithWidth {
        x: units_to_mm(units.0),
        y: units_to_mm(units.1),
        z,
        width,
        flow_factor,
        ..Default::default()
    }
}

fn with_xy(point: &Point3WithWidth, x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth { x, y, ..*point }
}

fn grid_round_trips(units: (i64, i64)) -> bool {
    Point2::from_mm(units_to_mm(units.0), units_to_mm(units.1))
        == Point2 {
            x: units.0,
            y: units.1,
        }
}

fn units_round_trip(units: i64) -> bool {
    mm_to_units(units_to_mm(units)) == units
}

fn bump_width(width: f32, ulps: u32) -> f32 {
    f32::from_bits(width.to_bits() + ulps)
}

/// Independent compatibility predicate (charter): equal role, bit-equal
/// speed, equal authored tool and both corresponding endpoint width
/// differences at most 0.000001 mm, compared exactly in f64.
fn independent_compatible(left: &ExtrusionPath3D, right: &ExtrusionPath3D) -> bool {
    let close = |a: f32, b: f32| (f64::from(a) - f64::from(b)).abs() <= f64::from(WIDTH_LIMIT_MM);
    let (Some(lf), Some(ll), Some(rf), Some(rl)) = (
        left.points.first(),
        left.points.last(),
        right.points.first(),
        right.points.last(),
    ) else {
        return false;
    };
    left.role == right.role
        && bits(left.speed_factor) == bits(right.speed_factor)
        && left.tool_index == right.tool_index
        && close(lf.width, rf.width)
        && close(ll.width, rl.width)
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent integer ring model.
// ─────────────────────────────────────────────────────────────────────────────

fn opposite(direction: RingDirection) -> RingDirection {
    match direction {
        RingDirection::Forward => RingDirection::Backward,
        RingDirection::Backward => RingDirection::Forward,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RingModel {
    points: Vec<(i64, i64)>,
    starts: Vec<i64>,
    length: i64,
    global_start: i64,
    polygon_index: usize,
    hole_index: Option<usize>,
}

impl RingModel {
    fn new(
        points: Vec<(i64, i64)>,
        global_start: i64,
        polygon_index: usize,
        hole_index: Option<usize>,
    ) -> Result<Self, String> {
        if points.len() < 4 {
            return Err(format!(
                "ring needs at least four vertices, got {}",
                points.len()
            ));
        }
        let count = points.len();
        let mut starts = Vec::with_capacity(count);
        let mut length: i64 = 0;
        for index in 0..count {
            let (a, b) = (points[index], points[(index + 1) % count]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            if (dx == 0) == (dy == 0) {
                return Err(format!(
                    "edge {index} {a:?}->{b:?} is not a nonzero axis-aligned edge"
                ));
            }
            starts.push(length);
            length = length
                .checked_add(dx.abs() + dy.abs())
                .ok_or_else(|| "ring perimeter overflow".to_owned())?;
        }
        Ok(Self {
            points,
            starts,
            length,
            global_start,
            polygon_index,
            hole_index,
        })
    }

    fn at(&self, arc: i64) -> (i64, i64) {
        let arc = arc.rem_euclid(self.length);
        let edge = self
            .starts
            .iter()
            .rposition(|start| *start <= arc)
            .expect("arc zero is the first vertex");
        let (a, b) = (
            self.points[edge],
            self.points[(edge + 1) % self.points.len()],
        );
        let t = arc - self.starts[edge];
        (
            a.0 + (b.0 - a.0).signum() * t,
            a.1 + (b.1 - a.1).signum() * t,
        )
    }

    fn offset(&self, from: i64, to: i64, direction: RingDirection) -> i64 {
        match direction {
            RingDirection::Forward => (to - from).rem_euclid(self.length),
            RingDirection::Backward => (from - to).rem_euclid(self.length),
        }
    }

    fn shortest(&self, from: i64, to: i64) -> (i64, RingDirection) {
        let forward = self.offset(from, to, RingDirection::Forward);
        let backward = self.offset(from, to, RingDirection::Backward);
        if forward <= backward {
            (forward, RingDirection::Forward)
        } else {
            (backward, RingDirection::Backward)
        }
    }

    fn is_tie(&self, from: i64, to: i64) -> bool {
        self.offset(from, to, RingDirection::Forward)
            == self.offset(from, to, RingDirection::Backward)
    }

    /// Vertices strictly inside `(0, span)` of a walk, in walk order.
    fn corners_within(&self, from: i64, direction: RingDirection, span: i64) -> Vec<(i64, i64)> {
        let mut corners = self
            .starts
            .iter()
            .enumerate()
            .filter_map(|(index, start)| {
                let offset = self.offset(from, *start, direction);
                (offset > 0 && offset < span).then_some((offset, self.points[index]))
            })
            .collect::<Vec<_>>();
        corners.sort();
        corners.into_iter().map(|(_, point)| point).collect()
    }

    fn walk(&self, from: i64, to: i64, direction: RingDirection) -> Vec<(i64, i64)> {
        self.corners_within(from, direction, self.offset(from, to, direction))
    }

    fn circular(&self, a: i64, b: i64) -> i64 {
        self.offset(a, b, RingDirection::Forward)
            .min(self.offset(a, b, RingDirection::Backward))
    }

    fn clear_of_vertices(&self, arc: i64, margin: i64) -> bool {
        self.starts
            .iter()
            .all(|start| self.circular(arc, *start) >= margin)
    }

    fn signed_area2(&self) -> i128 {
        let count = self.points.len();
        (0..count)
            .map(|index| {
                let (a, b) = (self.points[index], self.points[(index + 1) % count]);
                i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1)
            })
            .sum()
    }

    fn is_reflex(&self, point: (i64, i64)) -> bool {
        let count = self.points.len();
        let Some(index) = self.points.iter().position(|p| *p == point) else {
            return false;
        };
        let a = self.points[(index + count - 1) % count];
        let b = self.points[index];
        let c = self.points[(index + 1) % count];
        let cross = i128::from(b.0 - a.0) * i128::from(c.1 - b.1)
            - i128::from(b.1 - a.1) * i128::from(c.0 - b.0);
        cross.signum() * self.signed_area2().signum() < 0
    }

    fn distance_to_edge(&self, point: (f64, f64), edge: usize) -> f64 {
        let (a, b) = (
            self.points[edge],
            self.points[(edge + 1) % self.points.len()],
        );
        let cx = point.0.clamp(a.0.min(b.0) as f64, a.0.max(b.0) as f64);
        let cy = point.1.clamp(a.1.min(b.1) as f64, a.1.max(b.1) as f64);
        (point.0 - cx).hypot(point.1 - cy)
    }

    /// A segment lies on the ring when one ring edge holds both endpoints.
    /// Axis-aligned edges are convex, so the whole segment is on that edge.
    fn segment_on_ring(&self, a: (f64, f64), b: (f64, f64), tol: f64) -> bool {
        (0..self.points.len()).any(|edge| {
            self.distance_to_edge(a, edge) <= tol && self.distance_to_edge(b, edge) <= tol
        })
    }
}

fn check_walk_on_ring(ring: &RingModel, points: &[(f64, f64)], tol: f64) -> Result<(), String> {
    for (index, pair) in points.windows(2).enumerate() {
        if !ring.segment_on_ring(pair[0], pair[1], tol) {
            return Err(format!(
                "segment {index} {:?} -> {:?} does not lie on ring (polygon {}, hole {:?})",
                pair[0], pair[1], ring.polygon_index, ring.hole_index
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeKind {
    Rectangle,
    Notch,
    Hole,
    TwoComponents,
    HoleAndComponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShapeParams {
    ox: i64,
    oy: i64,
    cell: i64,
    cw: i64,
    ch: i64,
    p: [i64; 6],
}

impl ShapeParams {
    const ZERO: Self = Self {
        ox: 0,
        oy: 0,
        cell: 0,
        cw: 0,
        ch: 0,
        p: [0; 6],
    };

    fn complexity(&self) -> u128 {
        let mut total = u128::from(self.ox.unsigned_abs())
            + u128::from(self.oy.unsigned_abs())
            + u128::from(self.cell.unsigned_abs())
            + u128::from(self.cw.unsigned_abs())
            + u128::from(self.ch.unsigned_abs());
        for value in self.p {
            total += u128::from(value.unsigned_abs());
        }
        total
    }
}

#[derive(Debug, Clone)]
struct ExSpec {
    contour: Vec<(i64, i64)>,
    holes: Vec<Vec<(i64, i64)>>,
}

fn rect_ccw(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<(i64, i64)> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn rect_cw(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<(i64, i64)> {
    vec![(x0, y0), (x0, y1), (x1, y1), (x1, y0)]
}

fn shape_spec(kind: ShapeKind, s: &ShapeParams) -> Result<Vec<ExSpec>, String> {
    if s.cell < MIN_CELL_UNITS || !(3..=6).contains(&s.cw) || !(3..=6).contains(&s.ch) {
        return Err(format!(
            "shape parameters outside the fixture domain: {s:?}"
        ));
    }
    let x = |c: i64| s.ox + c * s.cell;
    let y = |c: i64| s.oy + c * s.cell;
    let p = |index: usize| s.p[index].unsigned_abs() as i64;
    let (cw, ch) = (s.cw, s.ch);
    let hole = |p0: i64, p1: i64, p2: i64, p3: i64| {
        let hx = 1 + p0 % (cw - 2);
        let hw = 1 + p1 % (cw - 1 - hx);
        let hy = 1 + p2 % (ch - 2);
        let hh = 1 + p3 % (ch - 1 - hy);
        rect_cw(x(hx), y(hy), x(hx + hw), y(hy + hh))
    };
    let outer = rect_ccw(x(0), y(0), x(cw), y(ch));
    Ok(match kind {
        ShapeKind::Rectangle => vec![ExSpec {
            contour: outer,
            holes: Vec::new(),
        }],
        ShapeKind::Notch => {
            let nx = 1 + p(0) % (cw - 2);
            let nd = 1 + p(1) % (ch - 1);
            vec![ExSpec {
                contour: vec![
                    (x(0), y(0)),
                    (x(cw), y(0)),
                    (x(cw), y(ch)),
                    (x(nx + 1), y(ch)),
                    (x(nx + 1), y(ch - nd)),
                    (x(nx), y(ch - nd)),
                    (x(nx), y(ch)),
                    (x(0), y(ch)),
                ],
                holes: Vec::new(),
            }]
        }
        ShapeKind::Hole => vec![ExSpec {
            contour: outer,
            holes: vec![hole(p(0), p(1), p(2), p(3))],
        }],
        ShapeKind::TwoComponents | ShapeKind::HoleAndComponent => {
            let gap = 1 + p(4) % 3;
            let cw2 = 3 + p(5) % 4;
            let ch2 = 3 + (p(4) + p(5)) % 4;
            let second = rect_ccw(x(cw + gap), y(0), x(cw + gap + cw2), y(ch2));
            let holes = if kind == ShapeKind::HoleAndComponent {
                vec![hole(p(0), p(1), p(2), p(3))]
            } else {
                Vec::new()
            };
            vec![
                ExSpec {
                    contour: outer,
                    holes,
                },
                ExSpec {
                    contour: second,
                    holes: Vec::new(),
                },
            ]
        }
    })
}

#[derive(Debug, Clone, PartialEq)]
struct ShapeModel {
    expolygons: Vec<ExPolygon>,
    rings: Vec<RingModel>,
    total: i64,
}

fn polygon_of(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| Point2 { x, y }).collect(),
    }
}

fn shape_model(specs: &[ExSpec]) -> Result<ShapeModel, String> {
    let mut rings = Vec::new();
    let mut expolygons = Vec::new();
    let mut position = 0_i64;
    for (polygon_index, spec) in specs.iter().enumerate() {
        let ring = RingModel::new(spec.contour.clone(), position, polygon_index, None)?;
        position += ring.length;
        rings.push(ring);
        for (hole_index, hole) in spec.holes.iter().enumerate() {
            let ring = RingModel::new(hole.clone(), position, polygon_index, Some(hole_index))?;
            position += ring.length;
            rings.push(ring);
        }
        expolygons.push(ExPolygon {
            contour: polygon_of(&spec.contour),
            holes: spec.holes.iter().map(|hole| polygon_of(hole)).collect(),
        });
    }
    for ring in &rings {
        if ring.points.iter().any(|p| {
            p.0.abs() > MAX_GRID_ABS_UNITS
                || p.1.abs() > MAX_GRID_ABS_UNITS
                || !grid_round_trips(*p)
        }) {
            return Err("ring vertex outside the f32 grid round-trip domain".to_owned());
        }
    }
    Ok(ShapeModel {
        expolygons,
        rings,
        total: position,
    })
}

fn place(ring: &RingModel, request: u64, taken: &[i64], separation: i64) -> Result<i64, String> {
    let length = ring.length;
    let start = (request % length as u64) as i64;
    for step in 0..length {
        let arc = (start + step) % length;
        if ring.clear_of_vertices(arc, VERTEX_MARGIN_UNITS)
            && taken
                .iter()
                .all(|other| ring.circular(arc, *other) >= separation)
        {
            return Ok(arc);
        }
    }
    Err(format!(
        "no admissible endpoint on a ring of length {length}"
    ))
}

fn place_tie(ring: &RingModel, request: u64) -> Result<(i64, i64), String> {
    let length = ring.length;
    if length % 2 != 0 {
        return Err("rectilinear ring perimeter must be even".to_owned());
    }
    let start = (request % length as u64) as i64;
    for step in 0..length {
        let from = (start + step) % length;
        let to = (from + length / 2) % length;
        if ring.clear_of_vertices(from, VERTEX_MARGIN_UNITS)
            && ring.clear_of_vertices(to, VERTEX_MARGIN_UNITS)
        {
            return Ok((from, to));
        }
    }
    Err("no admissible antipodal endpoint pair".to_owned())
}

// ─────────────────────────────────────────────────────────────────────────────
// Checked bound builders.
// ─────────────────────────────────────────────────────────────────────────────

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    match bounds.get(key).copied() {
        None => Err(DomainError::InvalidBounds(format!(
            "missing bound key `{key}`"
        ))),
        Some(0) => Err(DomainError::InvalidBounds(format!(
            "bound `{key}` must be positive"
        ))),
        Some(value) => Ok(value),
    }
}

fn checked_i64(bounds: &BTreeMap<String, u64>, key: &str) -> Result<i64, DomainError> {
    i64::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
}

fn checked_u32(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u32, DomainError> {
    u32::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
}

fn invalid(message: impl Into<String>) -> DomainError {
    DomainError::InvalidBounds(message.into())
}

fn checked_max_abs(bounds: &BTreeMap<String, u64>) -> Result<(i64, i64), DomainError> {
    let max_abs = checked_i64(bounds, "max_abs_units")?;
    if max_abs > MAX_GRID_ABS_UNITS {
        return Err(invalid(format!(
            "max_abs_units {max_abs} exceeds the {MAX_GRID_ABS_UNITS}-unit f32 grid round-trip domain"
        )));
    }
    let max_cell = (max_abs / 2) / SHAPE_SPAN_CELLS;
    if max_cell < MIN_CELL_UNITS {
        return Err(invalid(format!(
            "max_abs_units {max_abs} cannot hold a {SHAPE_SPAN_CELLS}-cell fixture of {MIN_CELL_UNITS}-unit cells"
        )));
    }
    Ok((max_abs, max_cell))
}

#[derive(Debug, Clone, Copy)]
struct GraphLimits {
    max_abs: i64,
    max_cell: i64,
    max_shrink_candidates: u32,
}

fn checked_graph_limits(bounds: &BTreeMap<String, u64>) -> Result<GraphLimits, DomainError> {
    let (max_abs, max_cell) = checked_max_abs(bounds)?;
    let max_vertices = checked_u32(bounds, "max_vertices")?;
    let max_holes = checked_u32(bounds, "max_holes")?;
    let max_components = checked_u32(bounds, "max_components")?;
    let max_shrink_candidates = checked_u32(bounds, "max_shrink_candidates")?;
    if max_vertices < 8 {
        return Err(invalid(
            "max_vertices must admit the 8-vertex notch/hole/two-component fixtures",
        ));
    }
    if max_holes < 1 {
        return Err(invalid("max_holes must admit the rectangular-hole fixture"));
    }
    if max_components < 2 {
        return Err(invalid(
            "max_components must admit the two-component fixture",
        ));
    }
    Ok(GraphLimits {
        max_abs,
        max_cell,
        max_shrink_candidates,
    })
}

#[derive(Debug, Clone, Copy)]
struct ConnectLimits {
    max_abs: i64,
    max_cell: i64,
    max_paths: usize,
    max_points: usize,
    max_anchor_units: i64,
}

fn checked_connect_limits(bounds: &BTreeMap<String, u64>) -> Result<ConnectLimits, DomainError> {
    let (max_abs, max_cell) = checked_max_abs(bounds)?;
    let max_paths = checked_u32(bounds, "max_paths")? as usize;
    let max_points = checked_u32(bounds, "max_points")? as usize;
    let max_anchor_units = checked_i64(bounds, "max_anchor_units")?;
    if max_paths < 3 {
        return Err(invalid("max_paths must admit the three-path multi class"));
    }
    if max_points < 2 {
        return Err(invalid("max_points must admit two-point paths"));
    }
    if max_anchor_units > MAX_GRID_ABS_UNITS {
        return Err(invalid(
            "max_anchor_units exceeds the f32 round-trip domain",
        ));
    }
    // 2·(3 + max_paths) endpoints on the 12-cell minimum outer ring.
    if (2 * max_paths as i64 + 2) * (2 * VERTEX_MARGIN_UNITS + 1)
        + 8 * (2 * VERTEX_MARGIN_UNITS + 1)
        >= 12 * MIN_CELL_UNITS
    {
        return Err(invalid("max_paths cannot be placed on the minimum ring"));
    }
    Ok(ConnectLimits {
        max_abs,
        max_cell,
        max_paths,
        max_points,
        max_anchor_units,
    })
}

#[derive(Debug, Clone, Copy)]
struct ChainLimits {
    connect: ConnectLimits,
    max_regions: usize,
    slot_height: i64,
}

fn checked_chain_limits(bounds: &BTreeMap<String, u64>) -> Result<ChainLimits, DomainError> {
    let (max_abs, max_cell) = checked_max_abs(bounds)?;
    let max_paths = checked_u32(bounds, "max_paths")? as usize;
    let max_points = checked_u32(bounds, "max_points")? as usize;
    let max_regions = checked_u32(bounds, "max_regions")? as usize;
    let max_anchor_units = checked_i64(bounds, "max_anchor_units")?;
    if max_paths < 2 {
        return Err(invalid("max_paths must admit two paths"));
    }
    if max_points < 2 {
        return Err(invalid("max_points must admit two-point paths"));
    }
    if max_regions < 2 {
        return Err(invalid("max_regions must admit the A/B wall-group pair"));
    }
    if max_anchor_units > MAX_GRID_ABS_UNITS {
        return Err(invalid(
            "max_anchor_units exceeds the f32 round-trip domain",
        ));
    }
    if max_paths > 16 || max_regions > 16 {
        return Err(invalid(
            "max_paths/max_regions exceed the bounded orchestrate layout",
        ));
    }
    let slot_height = (max_paths as i64 + 1) * ROW_PITCH_UNITS;
    let layout_height = slot_height
        .checked_mul(max_regions as i64)
        .ok_or(DomainError::ArithmeticOverflow)?;
    if layout_height > max_abs
        || MIN_REGION_WIDTH_UNITS + REGION_WIDTH_SPAN_UNITS + ROW_OVERSHOOT_UNITS > max_abs
    {
        return Err(invalid(format!(
            "max_abs_units {max_abs} cannot hold {max_regions} stacked {slot_height}-unit region slots"
        )));
    }
    Ok(ChainLimits {
        connect: ConnectLimits {
            max_abs,
            max_cell,
            max_paths,
            max_points,
            max_anchor_units,
        },
        max_regions,
        slot_height,
    })
}

fn live_bounds(run: &PropertyRun, keys: &[&str]) -> BTreeMap<String, u64> {
    keys.iter()
        .map(|key| {
            let value = run
                .bound(key)
                .unwrap_or_else(|error| panic!("linker bound `{key}`: {error}"));
            ((*key).to_owned(), value)
        })
        .collect()
}

fn graph_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 200_000),
        ("max_vertices".to_owned(), 16),
        ("max_holes".to_owned(), 2),
        ("max_components".to_owned(), 2),
        ("max_shrink_candidates".to_owned(), 4_096),
    ])
}

fn connect_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 200_000),
        ("max_paths".to_owned(), 4),
        ("max_points".to_owned(), 5),
        ("max_anchor_units".to_owned(), 100_000),
    ])
}

fn chain_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 400_000),
        ("max_paths".to_owned(), 4),
        ("max_points".to_owned(), 4),
        ("max_regions".to_owned(), 4),
        ("max_anchor_units".to_owned(), 100_000),
    ])
}

fn shape_strategy(max_abs: i64, max_cell: i64) -> BoxedStrategy<ShapeParams> {
    let half = max_abs / 2;
    (
        -half..=0_i64,
        -half..=0_i64,
        MIN_CELL_UNITS..=max_cell,
        3_i64..=6,
        3_i64..=6,
        prop::array::uniform6(0_i64..=1_000),
    )
        .prop_map(|(ox, oy, cell, cw, ch, p)| ShapeParams {
            ox,
            oy,
            cell,
            cw,
            ch,
            p,
        })
        .boxed()
}

// ─────────────────────────────────────────────────────────────────────────────
// M-LINKER-GRAPH
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RingClass {
    Rectangle,
    Notch,
    Hole,
    TwoComponents,
    Empty,
}

const RING_CLASSES: [RingClass; 5] = [
    RingClass::Rectangle,
    RingClass::Notch,
    RingClass::Hole,
    RingClass::TwoComponents,
    RingClass::Empty,
];

impl RingClass {
    fn shape(self) -> Option<ShapeKind> {
        match self {
            Self::Rectangle => Some(ShapeKind::Rectangle),
            Self::Notch => Some(ShapeKind::Notch),
            Self::Hole => Some(ShapeKind::Hole),
            Self::TwoComponents => Some(ShapeKind::TwoComponents),
            Self::Empty => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArcMode {
    Free,
    Tie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryRaw {
    ring: u32,
    from: u64,
    to: u64,
    stub_forward: bool,
    budget_permille: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphRaw {
    class: RingClass,
    arc: ArcMode,
    shape: ShapeParams,
    queries: Vec<QueryRaw>,
}

impl GraphRaw {
    fn complexity(&self) -> u128 {
        let mut total = self.shape.complexity() + 1_000 * self.queries.len() as u128;
        for query in &self.queries {
            total += u128::from(query.ring)
                + u128::from(query.from)
                + u128::from(query.to)
                + u128::from(query.stub_forward)
                + u128::from(query.budget_permille);
        }
        total
    }
}

#[derive(Debug, Clone)]
struct GraphQuery {
    ring: usize,
    from: i64,
    to: i64,
    stub_direction: RingDirection,
    budget: i64,
    start: Point3WithWidth,
    end: Point3WithWidth,
}

#[derive(Debug, Clone)]
struct GraphCase {
    class: RingClass,
    arc: ArcMode,
    model: ShapeModel,
    queries: Vec<GraphQuery>,
    max_abs: i64,
}

fn query_points(ring: &RingModel, from: i64, to: i64) -> (Point3WithWidth, Point3WithWidth) {
    (
        endpoint_point(ring.at(from), 0.2, 0.4, 1.0),
        endpoint_point(ring.at(to), 0.3, 0.5, 1.0),
    )
}

fn build_graph_case(raw: &GraphRaw, max_abs: i64) -> Result<GraphCase, String> {
    let Some(kind) = raw.class.shape() else {
        return Ok(GraphCase {
            class: raw.class,
            arc: raw.arc,
            model: shape_model(&[])?,
            queries: Vec::new(),
            max_abs,
        });
    };
    let model = shape_model(&shape_spec(kind, &raw.shape)?)?;
    if raw.queries.is_empty() || raw.queries.len() > MAX_QUERIES {
        return Err(format!(
            "query count {} outside 1..={MAX_QUERIES}",
            raw.queries.len()
        ));
    }
    let mut queries = Vec::new();
    for query in &raw.queries {
        let ring_index = query.ring as usize % model.rings.len();
        let ring = &model.rings[ring_index];
        let (from, to) = match raw.arc {
            ArcMode::Free => {
                let from = place(ring, query.from, &[], VERTEX_MARGIN_UNITS)?;
                (from, place(ring, query.to, &[from], VERTEX_MARGIN_UNITS)?)
            }
            ArcMode::Tie => place_tie(ring, query.from)?,
        };
        let budget = (ring.length * i64::from(query.budget_permille) / 1_000).max(1);
        let (start, end) = query_points(ring, from, to);
        queries.push(GraphQuery {
            ring: ring_index,
            from,
            to,
            stub_direction: if query.stub_forward {
                RingDirection::Forward
            } else {
                RingDirection::Backward
            },
            budget,
            start,
            end,
        });
    }
    Ok(GraphCase {
        class: raw.class,
        arc: raw.arc,
        model,
        queries,
        max_abs,
    })
}

fn graph_strategy(
    limits: &GraphLimits,
    classes: &[RingClass],
    arcs: &[ArcMode],
) -> BoxedStrategy<GraphRaw> {
    let (max_abs, max_cell) = (limits.max_abs, limits.max_cell);
    (
        prop::sample::select(classes.to_vec()).no_shrink(),
        prop::sample::select(arcs.to_vec()).no_shrink(),
    )
        .prop_ind_flat_map(move |(class, arc)| {
            if class == RingClass::Empty {
                return Just(GraphRaw {
                    class,
                    arc,
                    shape: ShapeParams::ZERO,
                    queries: Vec::new(),
                })
                .boxed();
            }
            let query = (
                any::<u32>(),
                any::<u64>(),
                any::<u64>(),
                any::<bool>(),
                1_u32..=1_500,
            )
                .prop_map(|(ring, from, to, stub_forward, budget_permille)| QueryRaw {
                    ring,
                    from,
                    to,
                    stub_forward,
                    budget_permille,
                });
            (
                shape_strategy(max_abs, max_cell),
                prop::collection::vec(query, 1..=MAX_QUERIES),
            )
                .prop_map(move |(shape, queries)| GraphRaw {
                    class,
                    arc,
                    shape,
                    queries,
                })
                .boxed()
        })
        .boxed()
}

fn check_graph_structure(model: &ShapeModel, graph: &BoundaryInfillGraph) -> Result<(), String> {
    if graph.polygons_outer() != model.expolygons.as_slice() {
        return Err("graph does not retain the source expolygons".to_owned());
    }
    if graph.rings().len() != model.rings.len() {
        return Err(format!(
            "graph has {} rings, the fixture has {}",
            graph.rings().len(),
            model.rings.len()
        ));
    }
    if graph.total_len() != model.total as f64 {
        return Err(format!(
            "graph total {} differs from the independent perimeter sum {}",
            graph.total_len(),
            model.total
        ));
    }
    for (index, (ring, expected)) in graph.rings().iter().zip(&model.rings).enumerate() {
        if ring.polygon != polygon_of(&expected.points)
            || ring.polygon_index != expected.polygon_index
            || ring.hole_index != expected.hole_index
        {
            return Err(format!(
                "ring {index} has the wrong identity or traversal order"
            ));
        }
        if ring.pos_of_first_point != expected.global_start as f64
            || ring.length != expected.length as f64
        {
            return Err(format!(
                "ring {index} arc table ({}, {}) differs from the independent ({}, {})",
                ring.pos_of_first_point, ring.length, expected.global_start, expected.length
            ));
        }
        if graph.pos_of_first_point(expected.polygon_index, expected.hole_index)
            != Some(expected.global_start as f64)
        {
            return Err(format!(
                "pos_of_first_point lookup for ring {index} is wrong"
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArcKind {
    ShorterForward,
    ShorterBackward,
    Tie,
}

fn check_directed(
    ring: &RingModel,
    from: i64,
    to: i64,
    actual: (f64, RingDirection),
) -> Result<ArcKind, String> {
    let (distance, direction) = ring.shortest(from, to);
    if actual.0 != distance as f64 || actual.1 != direction {
        return Err(format!(
            "directed_distance {from}->{to} = {actual:?}, the independent shortest arc is ({distance}, {direction:?})"
        ));
    }
    Ok(if ring.is_tie(from, to) {
        ArcKind::Tie
    } else if direction == RingDirection::Forward {
        ArcKind::ShorterForward
    } else {
        ArcKind::ShorterBackward
    })
}

fn check_lerped_metadata(
    point: &Point3WithWidth,
    start: &Point3WithWidth,
    end: &Point3WithWidth,
) -> Result<(), String> {
    let within = |value: f32, a: f32, b: f32| {
        let eps = 4.0 * f32::EPSILON * a.abs().max(b.abs()).max(1.0);
        value >= a.min(b) - eps && value <= a.max(b) + eps
    };
    if bits(point.flow_factor) != bits(start.flow_factor)
        || point.overhang_quartile != start.overhang_quartile
        || bits(point.dist_to_top_mm) != bits(start.dist_to_top_mm)
        || point.overhang_distance_mm.map(f32::to_bits)
            != start.overhang_distance_mm.map(f32::to_bits)
    {
        return Err("connector point does not inherit the start point's metadata".to_owned());
    }
    if !within(point.z, start.z, end.z) || !within(point.width, start.width, end.width) {
        return Err("connector z/width is not interpolated between its endpoints".to_owned());
    }
    Ok(())
}

/// Whole-walk connector checker: exact independent corner list, every
/// segment on the given ring model, and inherited/lerped metadata.
#[allow(clippy::too_many_arguments)]
fn check_connector(
    ring: &RingModel,
    from: i64,
    to: i64,
    start: &Point3WithWidth,
    end: &Point3WithWidth,
    connector: &[Point3WithWidth],
    allow_either_on_tie: bool,
    tol: f64,
) -> Result<Vec<(i64, i64)>, String> {
    let (_, direction) = ring.shortest(from, to);
    let mut candidates = vec![ring.walk(from, to, direction)];
    if allow_either_on_tie && ring.is_tie(from, to) {
        candidates.push(ring.walk(from, to, opposite(direction)));
    }
    let actual = connector
        .iter()
        .map(|point| grid_of(point, tol))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "connector point is off the integer grid".to_owned())?;
    if !candidates.contains(&actual) {
        return Err(format!(
            "connector {actual:?} is not the independent shortest ring walk {:?}",
            candidates[0]
        ));
    }
    let mut walk = vec![units_of(start)];
    walk.extend(connector.iter().map(units_of));
    walk.push(units_of(end));
    check_walk_on_ring(ring, &walk, tol)?;
    for point in connector {
        check_lerped_metadata(point, start, end)?;
    }
    Ok(actual)
}

#[allow(clippy::too_many_arguments)]
fn check_stub(
    ring: &RingModel,
    from: i64,
    direction: RingDirection,
    budget: i64,
    anchor: &Point3WithWidth,
    stub: &[Point3WithWidth],
    grid_tol: f64,
    lerp_tol: f64,
) -> Result<usize, String> {
    if budget <= 0 {
        return if stub.is_empty() {
            Ok(0)
        } else {
            Err("nonpositive stub budget produced geometry".to_owned())
        };
    }
    let walked = budget.min(ring.length);
    let corners = ring.corners_within(from, direction, walked);
    let end = ring.at(match direction {
        RingDirection::Forward => from + walked,
        RingDirection::Backward => from - walked,
    });
    if stub.len() != corners.len() + 1 {
        return Err(format!(
            "stub has {} points, the independent walk of {walked} has {} corners plus its end",
            stub.len(),
            corners.len()
        ));
    }
    for (point, corner) in stub.iter().zip(&corners) {
        if grid_of(point, grid_tol) != Some(*corner) {
            return Err(format!(
                "stub corner {:?} differs from {corner:?}",
                units_of(point)
            ));
        }
    }
    let last = units_of(&stub[stub.len() - 1]);
    if (last.0 - end.0 as f64).abs() > lerp_tol || (last.1 - end.1 as f64).abs() > lerp_tol {
        return Err(format!(
            "stub end {last:?} differs from the independent end {end:?}"
        ));
    }
    let mut walk = vec![units_of(anchor)];
    walk.extend(stub.iter().map(units_of));
    check_walk_on_ring(ring, &walk, lerp_tol)?;
    if !stub.iter().all(|point| same_except_xy(point, anchor)) {
        return Err("stub point metadata differs from its anchor".to_owned());
    }
    Ok(corners.len())
}

#[derive(Default)]
struct GraphCounters {
    cases: Cell<u64>,
    queries: Cell<u64>,
    shorter_forward: Cell<u64>,
    shorter_backward: Cell<u64>,
    ties: Cell<u64>,
    corner_walks: Cell<u64>,
    reflex_walks: Cell<u64>,
    hole_walks: Cell<u64>,
    second_component_walks: Cell<u64>,
    stubs: Cell<u64>,
    clamped_stubs: Cell<u64>,
    empty_graphs: Cell<u64>,
}

impl GraphCounters {
    fn report(&self, label: &str) -> String {
        format!(
            "{label} cases={} queries={} shorter_forward={} shorter_backward={} ties={} corner_walks={} \
             reflex_walks={} hole_walks={} second_component_walks={} stubs={} clamped_stubs={} empty_graphs={}",
            self.cases.get(),
            self.queries.get(),
            self.shorter_forward.get(),
            self.shorter_backward.get(),
            self.ties.get(),
            self.corner_walks.get(),
            self.reflex_walks.get(),
            self.hole_walks.get(),
            self.second_component_walks.get(),
            self.stubs.get(),
            self.clamped_stubs.get(),
            self.empty_graphs.get()
        )
    }

    fn assert_nonvacuous(&self) {
        for (name, cell) in [
            ("shorter_forward", &self.shorter_forward),
            ("shorter_backward", &self.shorter_backward),
            ("ties", &self.ties),
            ("corner_walks", &self.corner_walks),
            ("reflex_walks", &self.reflex_walks),
            ("hole_walks", &self.hole_walks),
            ("second_component_walks", &self.second_component_walks),
            ("stubs", &self.stubs),
            ("clamped_stubs", &self.clamped_stubs),
            ("empty_graphs", &self.empty_graphs),
        ] {
            assert!(
                cell.get() > 0,
                "M-LINKER-GRAPH population `{name}` is empty"
            );
        }
    }
}

fn run_graph_case(case: &GraphCase, counters: &GraphCounters) -> Result<(), String> {
    inc(&counters.cases);
    let graph = BoundaryInfillGraph::new(&case.model.expolygons);
    check_graph_structure(&case.model, &graph)?;
    if case.class == RingClass::Empty {
        if !graph.rings().is_empty()
            || graph.total_len() != 0.0
            || graph.pos_of_first_point(0, None).is_some()
        {
            return Err("empty graph is not the distinct empty mode".to_owned());
        }
        inc(&counters.empty_graphs);
        return Ok(());
    }
    let (grid, lerp) = (grid_tol_units(case.max_abs), lerp_tol_units(case.max_abs));
    for query in &case.queries {
        inc(&counters.queries);
        let model = &case.model.rings[query.ring];
        let ring = &graph.rings()[query.ring];
        for point in [&query.start, &query.end] {
            let Some(units) = grid_of(point, grid) else {
                return Err("constructed endpoint is off the grid".to_owned());
            };
            if !grid_round_trips(units)
                || point_in_region(
                    Point2 {
                        x: units.0,
                        y: units.1,
                    },
                    &case.model.expolygons,
                ) != Ok(PointLocation::Boundary)
            {
                return Err(format!(
                    "constructed endpoint {units:?} is not on the region boundary"
                ));
            }
        }
        let from = (model.global_start + query.from) as f64;
        let to = (model.global_start + query.to) as f64;
        match check_directed(
            model,
            query.from,
            query.to,
            ring.directed_distance(from, to),
        )? {
            ArcKind::ShorterForward => inc(&counters.shorter_forward),
            ArcKind::ShorterBackward => inc(&counters.shorter_backward),
            ArcKind::Tie => inc(&counters.ties),
        }
        let connector = contour_connector(ring, from, to, &query.start, &query.end);
        let corners = check_connector(
            model,
            query.from,
            query.to,
            &query.start,
            &query.end,
            &connector,
            false,
            grid,
        )?;
        if !corners.is_empty() {
            inc(&counters.corner_walks);
            if model.hole_index.is_some() {
                inc(&counters.hole_walks);
            }
            if model.polygon_index > 0 {
                inc(&counters.second_component_walks);
            }
            if corners.iter().any(|corner| model.is_reflex(*corner)) {
                inc(&counters.reflex_walks);
            }
        }
        let stub = contour_stub(
            ring,
            from,
            query.stub_direction,
            query.budget as f64,
            &query.start,
        );
        check_stub(
            model,
            query.from,
            query.stub_direction,
            query.budget,
            &query.start,
            &stub,
            grid,
            lerp,
        )?;
        inc(&counters.stubs);
        if query.budget > model.length {
            inc(&counters.clamped_stubs);
        }
    }
    Ok(())
}

fn witness_query(
    model: &ShapeModel,
    ring: usize,
    from: i64,
    to: i64,
    stub_direction: RingDirection,
    budget: i64,
) -> GraphQuery {
    let (start, end) = query_points(&model.rings[ring], from, to);
    GraphQuery {
        ring,
        from,
        to,
        stub_direction,
        budget,
        start,
        end,
    }
}

fn witness_graph(
    class: RingClass,
    arc: ArcMode,
    specs: &[ExSpec],
    queries: &[(usize, i64, i64, bool, i64)],
) -> GraphCase {
    let model = shape_model(specs).expect("witness model");
    let queries = queries
        .iter()
        .map(|&(ring, from, to, forward, budget)| {
            let direction = if forward {
                RingDirection::Forward
            } else {
                RingDirection::Backward
            };
            witness_query(&model, ring, from, to, direction, budget)
        })
        .collect();
    GraphCase {
        class,
        arc,
        model,
        queries,
        max_abs: 200_000,
    }
}

fn notch_spec() -> Vec<ExSpec> {
    vec![ExSpec {
        contour: vec![
            (0, 0),
            (90_000, 0),
            (90_000, 60_000),
            (60_000, 60_000),
            (60_000, 30_000),
            (30_000, 30_000),
            (30_000, 60_000),
            (0, 60_000),
        ],
        holes: Vec::new(),
    }]
}

fn holed_square_spec() -> Vec<ExSpec> {
    vec![ExSpec {
        contour: rect_ccw(0, 0, 100_000, 100_000),
        holes: vec![rect_cw(30_000, 30_000, 70_000, 70_000)],
    }]
}

/// Deterministic graph witnesses with hand-derived literal expectations; they
/// pin the integer model itself before it judges the SUT.
fn graph_witnesses(counters: &GraphCounters) {
    // 10 mm × 6 mm rectangle: perimeter 320 000 units.
    let rectangle = witness_graph(
        RingClass::Rectangle,
        ArcMode::Free,
        &[ExSpec {
            contour: rect_ccw(0, 0, 100_000, 60_000),
            holes: Vec::new(),
        }],
        &[
            (0, 20_000, 130_000, true, 100_000),
            (0, 130_000, 20_000, false, 30_000),
            (0, 20_000, 130_000, true, 400_000),
        ],
    );
    let ring = &rectangle.model.rings[0];
    assert_eq!(ring.length, 320_000);
    assert_eq!(
        ring.shortest(20_000, 130_000),
        (110_000, RingDirection::Forward)
    );
    assert_eq!(
        ring.walk(20_000, 130_000, RingDirection::Forward),
        vec![(100_000, 0)]
    );
    assert_eq!(
        ring.shortest(130_000, 20_000),
        (110_000, RingDirection::Backward)
    );
    assert_eq!(
        ring.corners_within(20_000, RingDirection::Backward, 30_000),
        vec![(0, 0)]
    );
    assert_eq!(ring.at(20_000 - 30_000), (0, 10_000));
    run_graph_case(&rectangle, counters).expect("rectangle witness");

    // Tie: (50 000, 0) and its antipode (50 000, 60 000) are 160 000 both ways.
    let tie = witness_graph(
        RingClass::Rectangle,
        ArcMode::Tie,
        &[ExSpec {
            contour: rect_ccw(0, 0, 100_000, 60_000),
            holes: Vec::new(),
        }],
        &[(0, 50_000, 210_000, true, 1_000)],
    );
    assert!(tie.model.rings[0].is_tie(50_000, 210_000));
    assert_eq!(
        tie.model.rings[0].walk(50_000, 210_000, RingDirection::Forward),
        vec![(100_000, 0), (100_000, 60_000)]
    );
    run_graph_case(&tie, counters).expect("tie witness");

    // Notch: (80 000, 60 000) -> (10 000, 60 000) walks down through the notch
    // (130 000 forward versus 230 000 back), passing both reflex corners.
    let notch = witness_graph(
        RingClass::Notch,
        ArcMode::Free,
        &notch_spec(),
        &[(0, 160_000, 290_000, true, 60_000)],
    );
    let ring = &notch.model.rings[0];
    assert_eq!(ring.length, 360_000);
    assert_eq!(
        ring.shortest(160_000, 290_000),
        (130_000, RingDirection::Forward)
    );
    assert_eq!(
        ring.walk(160_000, 290_000, RingDirection::Forward),
        vec![
            (60_000, 60_000),
            (60_000, 30_000),
            (30_000, 30_000),
            (30_000, 60_000)
        ]
    );
    assert!(ring.is_reflex((60_000, 30_000)) && ring.is_reflex((30_000, 30_000)));
    assert!(!ring.is_reflex((60_000, 60_000)) && !ring.is_reflex((0, 0)));
    run_graph_case(&notch, counters).expect("notch witness");

    // Hole ring [30 000, 70 000]² (CW, global start 400 000, length 160 000):
    // local 10 000 -> 100 000 is 90 000 forward, 70 000 backward.
    let hole = witness_graph(
        RingClass::Hole,
        ArcMode::Free,
        &holed_square_spec(),
        &[(1, 10_000, 100_000, false, 20_000)],
    );
    let ring = &hole.model.rings[1];
    assert_eq!((ring.global_start, ring.length), (400_000, 160_000));
    assert_eq!(
        ring.shortest(10_000, 100_000),
        (70_000, RingDirection::Backward)
    );
    assert_eq!(
        ring.walk(10_000, 100_000, RingDirection::Backward),
        vec![(30_000, 30_000), (70_000, 30_000)]
    );
    run_graph_case(&hole, counters).expect("hole witness");

    // Second component starts after the first ring's 120 000 units.
    let two = witness_graph(
        RingClass::TwoComponents,
        ArcMode::Free,
        &[
            ExSpec {
                contour: rect_ccw(0, 0, 30_000, 30_000),
                holes: Vec::new(),
            },
            ExSpec {
                contour: rect_ccw(50_000, 0, 90_000, 30_000),
                holes: Vec::new(),
            },
        ],
        &[(1, 10_000, 50_000, true, 45_000)],
    );
    assert_eq!(two.model.rings[1].global_start, 120_000);
    assert_eq!(
        two.model.rings[1].walk(10_000, 50_000, RingDirection::Forward),
        vec![(90_000, 0)]
    );
    run_graph_case(&two, counters).expect("two-component witness");

    let empty = witness_graph(RingClass::Empty, ArcMode::Free, &[], &[]);
    run_graph_case(&empty, counters).expect("empty graph witness");
}

#[test]
fn proptest_linker_ring_walks() {
    let run = PropertyRun::new(GRAPH_ID, GRAPH_TEST).expect("M-LINKER-GRAPH registry context");
    let limits = checked_graph_limits(&live_bounds(&run, &GRAPH_BOUND_KEYS)).expect("graph bounds");
    let witness = GraphCounters::default();
    graph_witnesses(&witness);
    witness.assert_nonvacuous();
    let generated = GraphCounters::default();
    let strategy = graph_strategy(&limits, &RING_CLASSES, &[ArcMode::Free, ArcMode::Tie]);
    run.check(&strategy, |raw, _guard| {
        let case = match build_graph_case(&raw, limits.max_abs) {
            Ok(case) => case,
            Err(message) => return fail(format!("graph fixture domain: {message}")),
        };
        match run_graph_case(&case, &generated) {
            Ok(()) => Ok(()),
            Err(message) => fail(format!("{:?}/{:?}: {message}", case.class, case.arc)),
        }
    })
    .expect("M-LINKER-GRAPH ring walks");
    assert!(
        generated.cases.get() > 0 && generated.queries.get() + generated.empty_graphs.get() > 0
    );
    // Per-case invariants only: the graph mode is drawn per case, so a
    // particular ring class, arc direction or stub class can legitimately be
    // absent from a 64-case run. `linker_controls`' deterministic witness
    // asserts every one of those populations on every run. `queries +
    // empty_graphs` is guaranteed positive because every case is one or the
    // other.
    assert_generated_populations(
        "M-LINKER-GRAPH",
        &[(
            "queries+empty_graphs",
            generated.queries.get() + generated.empty_graphs.get(),
        )],
    );
    println!("{}", witness.report("M-LINKER-GRAPH witness"));
    println!("{}", generated.report("M-LINKER-GRAPH generated"));
}

// ─────────────────────────────────────────────────────────────────────────────
// M-LINKER-CONNECT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectClass {
    Merge,
    WidthAtBoundary,
    RoleMismatch,
    SpeedMismatch,
    ToolMismatch,
    WidthFirstAbove,
    WidthLastAbove,
    AnchorAtDistance,
    OuterToHole,
    Disconnected,
    Stub,
    DontConnect,
    ShortDiscarded,
    Multi,
}

const CONNECT_CLASSES: [ConnectClass; 14] = [
    ConnectClass::Merge,
    ConnectClass::WidthAtBoundary,
    ConnectClass::RoleMismatch,
    ConnectClass::SpeedMismatch,
    ConnectClass::ToolMismatch,
    ConnectClass::WidthFirstAbove,
    ConnectClass::WidthLastAbove,
    ConnectClass::AnchorAtDistance,
    ConnectClass::OuterToHole,
    ConnectClass::Disconnected,
    ConnectClass::Stub,
    ConnectClass::DontConnect,
    ConnectClass::ShortDiscarded,
    ConnectClass::Multi,
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathRaw {
    ring: u32,
    start: u64,
    end: u64,
    interior: Vec<(i64, i64)>,
    meta: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectRaw {
    class: ConnectClass,
    shape: ShapeParams,
    paths: Vec<PathRaw>,
    slack: i64,
    anchor_len: i64,
    first_ulps: u32,
    last_ulps: u32,
    above_ulps: u32,
    alt: u32,
    dont_connect_units: i64,
}

fn paths_complexity(paths: &[PathRaw]) -> u128 {
    let mut total = 1_000_000 * paths.len() as u128;
    for path in paths {
        total += u128::from(path.ring)
            + u128::from(path.start)
            + u128::from(path.end)
            + u128::from(path.meta)
            + 1_000 * path.interior.len() as u128;
        for (x, y) in &path.interior {
            total += u128::from(x.unsigned_abs()) + u128::from(y.unsigned_abs());
        }
    }
    total
}

impl ConnectRaw {
    fn complexity(&self) -> u128 {
        self.shape.complexity()
            + paths_complexity(&self.paths)
            + u128::from(self.slack.unsigned_abs())
            + u128::from(self.anchor_len.unsigned_abs())
            + u128::from(self.first_ulps)
            + u128::from(self.last_ulps)
            + u128::from(self.above_ulps)
            + u128::from(self.alt)
            + u128::from(self.dont_connect_units.unsigned_abs())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Endpoint {
    ring: usize,
    local: i64,
    units: (i64, i64),
}

#[derive(Debug, Clone)]
struct ConnectFixture {
    model: ShapeModel,
    inputs: Vec<ExtrusionPath3D>,
    endpoints: Vec<Option<[Endpoint; 2]>>,
    anchor_max_units: i64,
    anchor_len_units: i64,
    max_abs: i64,
}

impl ConnectFixture {
    fn anchor(&self) -> AnchorParams {
        AnchorParams {
            anchor_length_mm: units_to_mm(self.anchor_len_units),
            anchor_length_max_mm: units_to_mm(self.anchor_max_units),
        }
    }

    fn graph(&self) -> BoundaryInfillGraph {
        BoundaryInfillGraph::new(&self.model.expolygons)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MergeExpectation {
    Free,
    /// The exact shortest arc `d` must be joined. Strict when the arc is
    /// outside the anchor band; inside the band either branch is legal.
    MustMerge(i64),
    /// Never extruded together (incompatible metadata or a different ring).
    /// Independent of the anchor: no band applies.
    MustNotMerge,
    /// A compatible same-ring pair whose exact arc sits inside the derived
    /// strict-anchor band (the `Stub` and `AnchorAtDistance` classes): either
    /// the join or the stub branch is accepted, while every observed junction
    /// and stub still passes its own ring-walk and legality checks.
    EitherBranch,
}

/// Legal band at the strict `candidate.distance < anchor_length_max_units`
/// boundary. See the header derivation: `ARC_EPSILON_UNITS` (1.0 unit,
/// `modules/core-modules/infill-linker/src/graph.rs`) plus the sub-1e-10-unit
/// `project_on_ring` f64 rounding term.
const ANCHOR_BAND_UNITS: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnchorSide {
    /// Strictly more than one unit below the anchor: the join is required.
    Below,
    /// Within one unit of the anchor: either branch is legal.
    WithinBand,
    /// Strictly more than one unit above the anchor: a join is a failure.
    Above,
}

fn anchor_side(exact_arc: i64, anchor_units: i64) -> AnchorSide {
    if (exact_arc as f64) < anchor_units as f64 - ANCHOR_BAND_UNITS {
        AnchorSide::Below
    } else if (exact_arc as f64) > anchor_units as f64 + ANCHOR_BAND_UNITS {
        AnchorSide::Above
    } else {
        AnchorSide::WithinBand
    }
}

#[derive(Debug, Clone)]
struct ConnectCase {
    class: ConnectClass,
    fixture: ConnectFixture,
    expectation: MergeExpectation,
}

#[derive(Debug, Clone)]
struct PathMeta {
    role: ExtrusionRole,
    speed: f32,
    tool: Option<u32>,
    first_width: f32,
    last_width: f32,
}

fn base_meta() -> PathMeta {
    PathMeta {
        role: ExtrusionRole::SparseInfill,
        speed: 1.0,
        tool: Some(0),
        first_width: BASE_WIDTH_MM,
        last_width: BASE_WIDTH_MM,
    }
}

/// Half of the selector range (`< 16`, and every value shrinks toward 0) is the
/// compatible base metadata, so generated multi-path cases merge often; the
/// other half varies role, speed, authored tool and first width bit-wise.
fn pool_meta(selector: u32) -> PathMeta {
    if selector < 16 {
        return base_meta();
    }
    let selector = selector - 16;
    PathMeta {
        role: if selector & 1 == 0 {
            ExtrusionRole::SparseInfill
        } else {
            ExtrusionRole::TopSolidInfill
        },
        speed: if selector & 2 == 0 { 1.0 } else { 0.9 },
        tool: Some((selector >> 2) & 1),
        first_width: bump_width(
            BASE_WIDTH_MM,
            if selector & 8 == 0 {
                0
            } else {
                WIDTH_ULPS_INCOMPATIBLE + 6
            },
        ),
        last_width: BASE_WIDTH_MM,
    }
}

fn make_path(points: Vec<Point3WithWidth>, meta: &PathMeta) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points,
        speed_factor: meta.speed,
        tool_index: meta.tool,
        ..extrusion_path3d_base(meta.role.clone())
    }
}

fn path_from(
    start: (i64, i64),
    interior: &[(i64, i64)],
    end: (i64, i64),
    meta: &PathMeta,
) -> ExtrusionPath3D {
    let mut points = vec![endpoint_point(start, 0.2, meta.first_width, 1.0)];
    points.extend(
        interior
            .iter()
            .map(|point| endpoint_point(*point, 0.2, BASE_WIDTH_MM, 1.0)),
    );
    points.push(endpoint_point(end, 0.2, meta.last_width, 1.0));
    make_path(points, meta)
}

fn connect_strategy(limits: &ConnectLimits, classes: &[ConnectClass]) -> BoxedStrategy<ConnectRaw> {
    let limits = *limits;
    prop::sample::select(classes.to_vec())
        .no_shrink()
        .prop_ind_flat_map(move |class| {
            let count = if class == ConnectClass::Multi {
                3..=limits.max_paths
            } else {
                2..=2
            };
            (
                Just(class),
                shape_strategy(limits.max_abs, limits.max_cell),
                prop::collection::vec(path_raw_strategy(&limits), count),
                0..=limits.max_anchor_units,
                1..=limits.max_anchor_units,
                0..=WIDTH_ULPS_COMPATIBLE,
                0..=WIDTH_ULPS_COMPATIBLE,
                0_u32..=30,
                any::<u32>(),
                0_i64..DONT_CONNECT_UNITS,
            )
        })
        .prop_map(
            |(
                class,
                shape,
                paths,
                slack,
                anchor_len,
                first_ulps,
                last_ulps,
                above_ulps,
                alt,
                dont_connect_units,
            )| {
                ConnectRaw {
                    class,
                    shape,
                    paths,
                    slack,
                    anchor_len,
                    first_ulps,
                    last_ulps,
                    above_ulps,
                    alt,
                    dont_connect_units,
                }
            },
        )
        .boxed()
}

fn path_raw_strategy(limits: &ConnectLimits) -> BoxedStrategy<PathRaw> {
    let half = limits.max_abs / 2;
    (
        any::<u32>(),
        any::<u64>(),
        any::<u64>(),
        prop::collection::vec((-half..=half, -half..=half), 0..=limits.max_points - 2),
        0_u32..=31,
    )
        .prop_map(|(ring, start, end, interior, meta)| PathRaw {
            ring,
            start,
            end,
            interior,
            meta,
        })
        .boxed()
}

/// Rings of `HoleAndComponent`: 0 = outer A, 1 = hole of A, 2 = outer B.
fn class_rings(class: ConnectClass, index: usize, selector: u32) -> usize {
    let outer = if selector.is_multiple_of(2) { 0 } else { 2 };
    match class {
        ConnectClass::OuterToHole => index.min(1),
        ConnectClass::Disconnected => {
            if index == 0 {
                (selector % 2) as usize
            } else {
                2
            }
        }
        ConnectClass::Multi => (selector % 3) as usize,
        _ => outer,
    }
}

fn min_cross_distance(model: &ShapeModel, a: &[Endpoint; 2], b: &[Endpoint; 2]) -> Option<i64> {
    let mut best = None;
    for left in a {
        for right in b {
            if left.ring == right.ring {
                let distance = model.rings[left.ring].shortest(left.local, right.local).0;
                best = Some(best.map_or(distance, |current: i64| current.min(distance)));
            }
        }
    }
    best
}

fn build_connect_case(raw: &ConnectRaw, limits: &ConnectLimits) -> Result<ConnectCase, String> {
    let model = shape_model(&shape_spec(ShapeKind::HoleAndComponent, &raw.shape)?)?;
    let class = raw.class;
    let pair = class != ConnectClass::Multi;
    if pair && raw.paths.len() != 2 {
        return Err("pair classes carry exactly two paths".to_owned());
    }
    if !pair && (raw.paths.len() < 3 || raw.paths.len() > limits.max_paths) {
        return Err("multi class carries 3..=max_paths paths".to_owned());
    }
    let separation = if matches!(class, ConnectClass::AnchorAtDistance | ConnectClass::Stub) {
        STRICT_SEPARATION_UNITS
    } else {
        VERTEX_MARGIN_UNITS
    };
    let selector = raw.paths[0].ring;
    let mut taken: Vec<Vec<i64>> = vec![Vec::new(); model.rings.len()];
    let mut inputs = Vec::new();
    let mut endpoints = Vec::new();
    for (index, path) in raw.paths.iter().enumerate() {
        if path.interior.len() + 2 > limits.max_points {
            return Err("path exceeds max_points".to_owned());
        }
        let ring_index = if pair {
            class_rings(class, index, selector)
        } else {
            class_rings(class, index, path.ring)
        };
        let ring = &model.rings[ring_index];
        let start = place(ring, path.start, &taken[ring_index], separation)?;
        taken[ring_index].push(start);
        let end = place(ring, path.end, &taken[ring_index], separation)?;
        taken[ring_index].push(end);
        let pair_endpoints = [
            Endpoint {
                ring: ring_index,
                local: start,
                units: ring.at(start),
            },
            Endpoint {
                ring: ring_index,
                local: end,
                units: ring.at(end),
            },
        ];
        let meta = if pair {
            let mut meta = base_meta();
            if index == 1 {
                match class {
                    ConnectClass::Merge | ConnectClass::ShortDiscarded => {
                        meta.first_width = bump_width(BASE_WIDTH_MM, raw.first_ulps);
                        meta.last_width = bump_width(BASE_WIDTH_MM, raw.last_ulps);
                    }
                    ConnectClass::WidthAtBoundary => {
                        meta.first_width = bump_width(BASE_WIDTH_MM, WIDTH_ULPS_COMPATIBLE);
                        meta.last_width = bump_width(BASE_WIDTH_MM, WIDTH_ULPS_COMPATIBLE);
                    }
                    ConnectClass::RoleMismatch => meta.role = ExtrusionRole::TopSolidInfill,
                    ConnectClass::SpeedMismatch => {
                        meta.speed = f32::from_bits(1.0_f32.to_bits() + 1 + raw.alt % 4);
                    }
                    ConnectClass::ToolMismatch => {
                        meta.tool = if raw.alt.is_multiple_of(2) {
                            Some(1)
                        } else {
                            None
                        };
                    }
                    ConnectClass::WidthFirstAbove => {
                        meta.first_width =
                            bump_width(BASE_WIDTH_MM, WIDTH_ULPS_INCOMPATIBLE + raw.above_ulps);
                        meta.last_width = bump_width(BASE_WIDTH_MM, raw.last_ulps);
                    }
                    ConnectClass::WidthLastAbove => {
                        meta.first_width = bump_width(BASE_WIDTH_MM, raw.first_ulps);
                        meta.last_width =
                            bump_width(BASE_WIDTH_MM, WIDTH_ULPS_INCOMPATIBLE + raw.above_ulps);
                    }
                    _ => {}
                }
            }
            meta
        } else {
            pool_meta(path.meta)
        };
        inputs.push(path_from(
            pair_endpoints[0].units,
            &path.interior,
            pair_endpoints[1].units,
            &meta,
        ));
        endpoints.push(Some(pair_endpoints));
    }
    if matches!(
        class,
        ConnectClass::DontConnect | ConnectClass::ShortDiscarded
    ) {
        // A one-point path inside material, on no ring.
        let first = &model.rings[0].points[0];
        let short = endpoint_point(
            (first.0 + raw.shape.cell / 2, first.1 + raw.shape.cell / 2),
            0.2,
            BASE_WIDTH_MM,
            1.0,
        );
        inputs.insert(1, make_path(vec![short], &base_meta()));
        endpoints.insert(1, None);
    }
    let distance = if pair {
        let a = endpoints[0].ok_or("pair endpoint")?;
        let b = endpoints.last().copied().flatten().ok_or("pair endpoint")?;
        min_cross_distance(&model, &a, &b)
    } else {
        None
    };
    let joinable_anchor = |d: i64| (d + 1 + raw.slack).max(DONT_CONNECT_UNITS);
    let (anchor_max_units, anchor_len_units, expectation) = match class {
        ConnectClass::Merge | ConnectClass::WidthAtBoundary | ConnectClass::ShortDiscarded => {
            let d = distance.ok_or("same-ring pair")?;
            (joinable_anchor(d), 0, MergeExpectation::MustMerge(d))
        }
        ConnectClass::RoleMismatch
        | ConnectClass::SpeedMismatch
        | ConnectClass::ToolMismatch
        | ConnectClass::WidthFirstAbove
        | ConnectClass::WidthLastAbove => {
            let d = distance.ok_or("same-ring pair")?;
            (joinable_anchor(d), 0, MergeExpectation::MustNotMerge)
        }
        ConnectClass::AnchorAtDistance => {
            let d = distance.ok_or("same-ring pair")?;
            (d, 0, MergeExpectation::EitherBranch)
        }
        ConnectClass::Stub => {
            let d = distance.ok_or("same-ring pair")?;
            (d, raw.anchor_len, MergeExpectation::EitherBranch)
        }
        ConnectClass::OuterToHole | ConnectClass::Disconnected => {
            if distance.is_some() {
                return Err("cross-ring classes must not share a ring".to_owned());
            }
            (
                joinable_anchor(model.total),
                0,
                MergeExpectation::MustNotMerge,
            )
        }
        ConnectClass::DontConnect => (
            raw.dont_connect_units,
            raw.anchor_len,
            MergeExpectation::Free,
        ),
        ConnectClass::Multi => (
            DONT_CONNECT_UNITS + raw.slack,
            if raw.alt.is_multiple_of(2) {
                0
            } else {
                raw.anchor_len
            },
            MergeExpectation::Free,
        ),
    };
    if matches!(class, ConnectClass::AnchorAtDistance | ConnectClass::Stub)
        && anchor_max_units < DONT_CONNECT_UNITS
    {
        return Err("strict anchor class must stay in the connecting branch".to_owned());
    }
    if !units_round_trip(anchor_max_units) || !units_round_trip(anchor_len_units) {
        return Err("anchor lengths do not round-trip through f32 mm".to_owned());
    }
    // Fixture self-check against the independent predicate.
    if pair {
        let compatible = independent_compatible(&inputs[0], inputs.last().ok_or("pair")?);
        let want = !matches!(
            class,
            ConnectClass::RoleMismatch
                | ConnectClass::SpeedMismatch
                | ConnectClass::ToolMismatch
                | ConnectClass::WidthFirstAbove
                | ConnectClass::WidthLastAbove
        );
        if compatible != want {
            return Err(format!(
                "{class:?} fixture compatibility is {compatible}, expected {want}"
            ));
        }
    }
    Ok(ConnectCase {
        class,
        fixture: ConnectFixture {
            model,
            inputs,
            endpoints,
            anchor_max_units,
            anchor_len_units,
            max_abs: limits.max_abs,
        },
        expectation,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Source {
        index: usize,
        reversed: bool,
        start: usize,
        end: usize,
    },
    Added {
        start: usize,
        end: usize,
    },
}

fn scan_output(
    path: &ExtrusionPath3D,
    inputs: &[ExtrusionPath3D],
    eligible: &[usize],
) -> Vec<Block> {
    let points = &path.points;
    let mut blocks = Vec::new();
    let mut added: Option<usize> = None;
    let mut position = 0;
    while position < points.len() {
        let hit = eligible.iter().find_map(|&index| {
            let source = &inputs[index].points;
            let count = source.len();
            if position + count > points.len() {
                return None;
            }
            let forward = (0..count).all(|k| point_bits_eq(&points[position + k], &source[k]));
            let backward =
                (0..count).all(|k| point_bits_eq(&points[position + k], &source[count - 1 - k]));
            if forward {
                Some((index, false, count))
            } else if backward {
                Some((index, true, count))
            } else {
                None
            }
        });
        if let Some((index, reversed, count)) = hit {
            if let Some(start) = added.take() {
                blocks.push(Block::Added {
                    start,
                    end: position,
                });
            }
            blocks.push(Block::Source {
                index,
                reversed,
                start: position,
                end: position + count,
            });
            position += count;
        } else {
            added.get_or_insert(position);
            position += 1;
        }
    }
    if let Some(start) = added {
        blocks.push(Block::Added {
            start,
            end: points.len(),
        });
    }
    blocks
}

#[derive(Debug, Default, Clone, Copy)]
struct ConnectSummary {
    sources: u64,
    junctions: u64,
    connector_corners: u64,
    stub_runs: u64,
    identity: bool,
}

fn exit_endpoint(fx: &ConnectFixture, index: usize, reversed: bool) -> Result<Endpoint, String> {
    let pair =
        fx.endpoints[index].ok_or_else(|| format!("source {index} has no ring endpoints"))?;
    Ok(if reversed { pair[0] } else { pair[1] })
}

fn entry_endpoint(fx: &ConnectFixture, index: usize, reversed: bool) -> Result<Endpoint, String> {
    let pair =
        fx.endpoints[index].ok_or_else(|| format!("source {index} has no ring endpoints"))?;
    Ok(if reversed { pair[1] } else { pair[0] })
}

fn check_stub_run(
    fx: &ConnectFixture,
    output: usize,
    endpoint: Endpoint,
    anchor: &Point3WithWidth,
    walk: &[Point3WithWidth],
) -> Result<(), String> {
    if fx.anchor_len_units <= 0 {
        return Err(format!(
            "output {output} carries added end geometry although anchor_length is zero"
        ));
    }
    let tol = lerp_tol_units(fx.max_abs);
    let ring = &fx.model.rings[endpoint.ring];
    let mut points = vec![units_of(anchor)];
    points.extend(walk.iter().map(units_of));
    check_walk_on_ring(ring, &points, tol)
        .map_err(|error| format!("output {output} stub: {error}"))?;
    let length: f64 = points
        .windows(2)
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum();
    // Each walk point is within `tol` units of the exact ring (`check_walk_on_ring`
    // uses the same `lerp_tol_units` envelope), so the polyline length measured
    // from those points can exceed the exact arc length by at most `tol` per
    // segment. `points.len()` bounds the segment count (it is one more than it).
    if length > fx.anchor_len_units as f64 + tol * points.len() as f64 {
        return Err(format!(
            "output {output} stub walks {length} units, over the {} anchor length",
            fx.anchor_len_units
        ));
    }
    if !walk.iter().all(|point| same_except_xy(point, anchor)) {
        return Err(format!(
            "output {output} stub metadata differs from its anchor"
        ));
    }
    Ok(())
}

/// The connect checker shared by `M-LINKER-CONNECT`, the chain-connect side of
/// `M-LINKER-CHAIN` and `linker_controls`.
fn check_connect_output(
    fx: &ConnectFixture,
    output: &[ExtrusionPath3D],
    expectation: MergeExpectation,
) -> Result<ConnectSummary, String> {
    let mut summary = ConnectSummary::default();
    if fx.anchor_max_units < DONT_CONNECT_UNITS {
        if output.len() != fx.inputs.len()
            || !output
                .iter()
                .zip(&fx.inputs)
                .all(|(a, b)| path_bits_eq(a, b))
        {
            return Err(
                "dont_connect early return must return its input unchanged, short paths included"
                    .to_owned(),
            );
        }
        summary.identity = true;
        summary.sources = fx.inputs.len() as u64;
        return Ok(summary);
    }
    let eligible = (0..fx.inputs.len())
        .filter(|&index| fx.inputs[index].points.len() >= 2)
        .collect::<Vec<_>>();
    let grid = grid_tol_units(fx.max_abs);
    let mut seen = vec![0_usize; fx.inputs.len()];
    let mut joins: Vec<i64> = Vec::new();
    let mut multi_source_outputs = 0;
    for (o, path) in output.iter().enumerate() {
        if path.points.len() < 2 {
            return Err(format!("output {o} has fewer than two points"));
        }
        let blocks = scan_output(path, &fx.inputs, &eligible);
        let sources = blocks
            .iter()
            .filter_map(|block| match *block {
                Block::Source {
                    index,
                    reversed,
                    start,
                    end,
                } => Some((index, reversed, start, end)),
                Block::Added { .. } => None,
            })
            .collect::<Vec<_>>();
        if sources.is_empty() {
            return Err(format!(
                "output {o} contains no source path: added geometry without a source"
            ));
        }
        if sources.len() > 1 {
            multi_source_outputs += 1;
        }
        for pair in sources.windows(2) {
            let (a, a_rev, _, a_end) = pair[0];
            let (b, b_rev, b_start, _) = pair[1];
            let ja = exit_endpoint(fx, a, a_rev)?;
            let jb = entry_endpoint(fx, b, b_rev)?;
            if ja.ring != jb.ring {
                return Err(format!(
                    "cross-ring extrusion in output {o}: source {a} (ring {}) joined to source {b} (ring {})",
                    ja.ring, jb.ring
                ));
            }
            if !independent_compatible(&fx.inputs[a], &fx.inputs[b]) {
                return Err(format!(
                    "incompatible sources {a} and {b} extruded together in output {o}"
                ));
            }
            let ring = &fx.model.rings[ja.ring];
            let (distance, _) = ring.shortest(ja.local, jb.local);
            // Strict only outside the derived anchor band: inside one unit of
            // the anchor either branch is legal (see `ANCHOR_BAND_UNITS`).
            if anchor_side(distance, fx.anchor_max_units) == AnchorSide::Above {
                return Err(format!(
                    "output {o} joins {a} and {b} across {distance} units, more than one unit above the {} anchor",
                    fx.anchor_max_units
                ));
            }
            let corners = check_connector(
                ring,
                ja.local,
                jb.local,
                &path.points[a_end - 1],
                &path.points[b_start],
                &path.points[a_end..b_start],
                true,
                grid,
            )
            .map_err(|error| format!("output {o} junction {a}->{b}: {error}"))?;
            summary.junctions += 1;
            summary.connector_corners += corners.len() as u64;
            joins.push(distance);
        }
        let (first, first_rev, first_start, _) = sources[0];
        if first_start > 0 {
            let endpoint = entry_endpoint(fx, first, first_rev)?;
            let walk = path.points[..first_start]
                .iter()
                .rev()
                .copied()
                .collect::<Vec<_>>();
            check_stub_run(fx, o, endpoint, &path.points[first_start], &walk)?;
            summary.stub_runs += 1;
        }
        let (last, last_rev, _, last_end) = sources[sources.len() - 1];
        if last_end < path.points.len() {
            let endpoint = exit_endpoint(fx, last, last_rev)?;
            check_stub_run(
                fx,
                o,
                endpoint,
                &path.points[last_end - 1],
                &path.points[last_end..],
            )?;
            summary.stub_runs += 1;
        }
        for &(index, _, _, _) in &sources {
            if !path_meta_eq(path, &fx.inputs[index]) {
                return Err(format!(
                    "source path {index} metadata changed in output {o}"
                ));
            }
            seen[index] += 1;
        }
    }
    for &index in &eligible {
        match seen[index] {
            0 => return Err(format!("source path {index} was dropped")),
            1 => {}
            count => return Err(format!("source path {index} appears {count} times")),
        }
    }
    summary.sources = eligible.len() as u64;
    match expectation {
        MergeExpectation::Free | MergeExpectation::EitherBranch => {}
        MergeExpectation::MustMerge(d) => match anchor_side(d, fx.anchor_max_units) {
            AnchorSide::Below => {
                if output.len() != 1 || joins != [d] {
                    return Err(format!(
                        "the compatible same-ring pair must merge across its {d}-unit shortest arc, strictly below the {} anchor; got {} outputs, joins {joins:?}",
                        fx.anchor_max_units,
                        output.len()
                    ));
                }
            }
            // Inside the band production may legally take either branch; the
            // per-output junction, stub and preservation checks above already
            // validate whichever one it took.
            AnchorSide::WithinBand => {}
            AnchorSide::Above => {
                return Err(format!(
                    "oracle expectation is inconsistent: exact arc {d} is above the {} anchor band",
                    fx.anchor_max_units
                ))
            }
        },
        MergeExpectation::MustNotMerge => {
            if multi_source_outputs > 0 || output.len() != eligible.len() {
                return Err("paths that must stay separate were extruded together".to_owned());
            }
        }
    }
    Ok(summary)
}

/// Proves the SUT's own projection puts every constructed endpoint on its
/// constructed ring (compared, never used to build an expectation).
fn check_projection(fx: &ConnectFixture, graph: &BoundaryInfillGraph) -> Result<(), String> {
    for endpoint in fx.endpoints.iter().flatten().flatten() {
        let point = Point2 {
            x: endpoint.units.0,
            y: endpoint.units.1,
        };
        if !grid_round_trips(endpoint.units)
            || point_in_region(point, &fx.model.expolygons) != Ok(PointLocation::Boundary)
        {
            return Err(format!(
                "endpoint {:?} is not a grid boundary point",
                endpoint.units
            ));
        }
        let expected = (fx.model.rings[endpoint.ring].global_start + endpoint.local) as f64;
        let projected = graph
            .project(point)
            .ok_or_else(|| "nonempty graph projected nothing".to_owned())?;
        if (projected - expected).abs() > 1e-6 {
            return Err(format!(
                "SUT projects {:?} to arc {projected}, constructed ring {} arc {expected}",
                endpoint.units, endpoint.ring
            ));
        }
    }
    Ok(())
}

#[derive(Default)]
struct ConnectCounters {
    cases: Cell<u64>,
    merges: Cell<u64>,
    separated: Cell<u64>,
    sources: Cell<u64>,
    junctions: Cell<u64>,
    connector_corners: Cell<u64>,
    stub_runs: Cell<u64>,
    cross_ring_pairs: Cell<u64>,
    mismatch_pairs: Cell<u64>,
    strict_anchor: Cell<u64>,
    identity: Cell<u64>,
    short_discarded: Cell<u64>,
    multi: Cell<u64>,
}

impl ConnectCounters {
    fn report(&self, label: &str) -> String {
        format!(
            "{label} cases={} merges={} separated={} sources={} junctions={} connector_corners={} stub_runs={} \
             cross_ring_pairs={} mismatch_pairs={} strict_anchor={} identity={} short_discarded={} multi={}",
            self.cases.get(),
            self.merges.get(),
            self.separated.get(),
            self.sources.get(),
            self.junctions.get(),
            self.connector_corners.get(),
            self.stub_runs.get(),
            self.cross_ring_pairs.get(),
            self.mismatch_pairs.get(),
            self.strict_anchor.get(),
            self.identity.get(),
            self.short_discarded.get(),
            self.multi.get()
        )
    }

    fn assert_nonvacuous(&self) {
        for (name, cell) in [
            ("merges", &self.merges),
            ("separated", &self.separated),
            ("sources", &self.sources),
            ("junctions", &self.junctions),
            ("connector_corners", &self.connector_corners),
            ("stub_runs", &self.stub_runs),
            ("cross_ring_pairs", &self.cross_ring_pairs),
            ("mismatch_pairs", &self.mismatch_pairs),
            ("strict_anchor", &self.strict_anchor),
            ("identity", &self.identity),
            ("short_discarded", &self.short_discarded),
            ("multi", &self.multi),
        ] {
            assert!(
                cell.get() > 0,
                "M-LINKER-CONNECT population `{name}` is empty"
            );
        }
    }
}

fn run_connect_case(
    case: &ConnectCase,
    counters: &ConnectCounters,
) -> Result<ConnectSummary, String> {
    inc(&counters.cases);
    let fx = &case.fixture;
    let graph = fx.graph();
    check_projection(fx, &graph)?;
    let output = connect_infill(fx.inputs.clone(), &graph, fx.anchor());
    let summary = check_connect_output(fx, &output, case.expectation)?;
    add(&counters.sources, summary.sources);
    add(&counters.junctions, summary.junctions);
    add(&counters.connector_corners, summary.connector_corners);
    add(&counters.stub_runs, summary.stub_runs);
    match case.expectation {
        MergeExpectation::MustMerge(_) => inc(&counters.merges),
        MergeExpectation::MustNotMerge => inc(&counters.separated),
        MergeExpectation::Free | MergeExpectation::EitherBranch => {}
    }
    match case.class {
        ConnectClass::OuterToHole | ConnectClass::Disconnected => inc(&counters.cross_ring_pairs),
        ConnectClass::RoleMismatch
        | ConnectClass::SpeedMismatch
        | ConnectClass::ToolMismatch
        | ConnectClass::WidthFirstAbove
        | ConnectClass::WidthLastAbove => inc(&counters.mismatch_pairs),
        ConnectClass::AnchorAtDistance => inc(&counters.strict_anchor),
        ConnectClass::DontConnect if summary.identity => inc(&counters.identity),
        ConnectClass::ShortDiscarded => inc(&counters.short_discarded),
        ConnectClass::Multi => inc(&counters.multi),
        _ => {}
    }
    Ok(summary)
}

fn witness_connect_raw(class: ConnectClass) -> ConnectRaw {
    let path = |ring: u32, start: u64, end: u64| PathRaw {
        ring,
        start,
        end,
        interior: vec![(1_500, 2_500)],
        meta: 0,
    };
    // Outer ring A is 88 000 units (bottom edge [0, 24 000)): P0 sits on the
    // bottom edge and P1 on the right edge, so the 21 000-unit shortest join
    // turns the (24 000, 0)-arc corner.
    let mut paths = vec![path(0, 3_000, 9_000), path(0, 30_000, 36_000)];
    if class == ConnectClass::Multi {
        paths.push(path(1, 4_000, 9_000));
        paths.push(path(2, 30_000, 31_000));
    }
    ConnectRaw {
        class,
        shape: ShapeParams {
            ox: -20_000,
            oy: -10_000,
            cell: 4_000,
            cw: 6,
            ch: 5,
            p: [1, 1, 1, 1, 0, 2],
        },
        paths,
        slack: 2_000,
        anchor_len: 3_000,
        first_ulps: 5,
        last_ulps: 9,
        above_ulps: 0,
        alt: 1,
        dont_connect_units: 0,
    }
}

fn connect_witnesses(limits: &ConnectLimits, counters: &ConnectCounters) {
    for class in CONNECT_CLASSES {
        let case = build_connect_case(&witness_connect_raw(class), limits)
            .unwrap_or_else(|error| panic!("{class:?} witness fixture: {error}"));
        run_connect_case(&case, counters)
            .unwrap_or_else(|error| panic!("{class:?} witness: {error}"));
    }
}

#[test]
fn proptest_linker_connection_compatibility() {
    let run =
        PropertyRun::new(CONNECT_ID, CONNECT_TEST).expect("M-LINKER-CONNECT registry context");
    let limits =
        checked_connect_limits(&live_bounds(&run, &CONNECT_BOUND_KEYS)).expect("connect bounds");
    let witness = ConnectCounters::default();
    connect_witnesses(&limits, &witness);
    witness.assert_nonvacuous();
    let generated = ConnectCounters::default();
    let strategy = connect_strategy(&limits, &CONNECT_CLASSES);
    run.check(&strategy, |raw, _guard| {
        let case = match build_connect_case(&raw, &limits) {
            Ok(case) => case,
            Err(message) => {
                return fail(format!(
                    "connect fixture domain ({:?}): {message}",
                    raw.class
                ))
            }
        };
        match run_connect_case(&case, &generated) {
            Ok(_) => Ok(()),
            Err(message) => fail(format!("{:?}: {message}", case.class)),
        }
    })
    .expect("M-LINKER-CONNECT compatibility and preservation");
    assert!(generated.cases.get() > 0 && generated.sources.get() > 0);
    // Only the population every accepted case forces is asserted on the
    // generated run. Class-specific coverage (`merges`, `stubs`, `identity`,
    // `multi`, …) depends on which of the 14 `ConnectClass` arms the
    // non-shrinking selector drew, and the charter forbids relying on a random
    // mode being selected; `connect_witnesses` + `assert_nonvacuous` above give
    // each of those a deterministic witness on every run.
    assert_generated_populations("M-LINKER-CONNECT", &[("sources", generated.sources.get())]);
    println!("{}", witness.report("M-LINKER-CONNECT witness"));
    println!("{}", generated.report("M-LINKER-CONNECT generated"));
}

// ─────────────────────────────────────────────────────────────────────────────
// M-LINKER-CHAIN — direct chain_or_connect_infill
// ─────────────────────────────────────────────────────────────────────────────

/// Chain-only checker: a bitwise permutation of the inputs, each possibly
/// reversed, with no added or removed point. Returns the reversed count.
fn check_chain_only(
    inputs: &[ExtrusionPath3D],
    output: &[ExtrusionPath3D],
) -> Result<(usize, bool), String> {
    if output.len() != inputs.len() {
        return Err(format!(
            "chain-only returned {} paths for {} inputs",
            output.len(),
            inputs.len()
        ));
    }
    let mut used = vec![false; inputs.len()];
    let mut reversed_count = 0;
    let mut reordered = false;
    for (o, path) in output.iter().enumerate() {
        let hit = (0..inputs.len()).find_map(|index| {
            if used[index] {
                None
            } else if path_bits_eq(path, &inputs[index]) {
                Some((index, false))
            } else if path_bits_eq(path, &reversed(&inputs[index])) {
                Some((index, true))
            } else {
                None
            }
        });
        let Some((index, was_reversed)) = hit else {
            return Err(format!(
                "chain-only output {o} is not an input path: it gained, lost or changed geometry"
            ));
        };
        used[index] = true;
        reordered |= index != o;
        reversed_count += usize::from(was_reversed && inputs[index].points.len() > 1);
    }
    Ok((reversed_count, reordered))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainTop {
    ChainOnly,
    ChainConnect,
    Orchestrate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectRaw {
    top: ChainTop,
    shape: ShapeParams,
    paths: Vec<PathRaw>,
    anchor_units: i64,
    anchor_len: i64,
}

fn build_direct_fixture(raw: &DirectRaw, limits: &ConnectLimits) -> Result<ConnectFixture, String> {
    let model = shape_model(&shape_spec(ShapeKind::HoleAndComponent, &raw.shape)?)?;
    if raw.paths.len() < 2 || raw.paths.len() > limits.max_paths {
        return Err("direct chain carries 2..=max_paths paths".to_owned());
    }
    let mut taken: Vec<Vec<i64>> = vec![Vec::new(); model.rings.len()];
    let mut inputs = Vec::new();
    let mut endpoints = Vec::new();
    for path in &raw.paths {
        if path.interior.len() + 2 > limits.max_points {
            return Err("path exceeds max_points".to_owned());
        }
        let ring_index = (path.ring % 3) as usize;
        let ring = &model.rings[ring_index];
        let start = place(ring, path.start, &taken[ring_index], VERTEX_MARGIN_UNITS)?;
        taken[ring_index].push(start);
        let end = place(ring, path.end, &taken[ring_index], VERTEX_MARGIN_UNITS)?;
        taken[ring_index].push(end);
        let pair = [
            Endpoint {
                ring: ring_index,
                local: start,
                units: ring.at(start),
            },
            Endpoint {
                ring: ring_index,
                local: end,
                units: ring.at(end),
            },
        ];
        inputs.push(path_from(
            pair[0].units,
            &path.interior,
            pair[1].units,
            &pool_meta(path.meta),
        ));
        endpoints.push(Some(pair));
    }
    let connecting = raw.top == ChainTop::ChainConnect;
    if connecting != (raw.anchor_units >= DONT_CONNECT_UNITS) {
        return Err("anchor does not match the selected dont_connect side".to_owned());
    }
    if !units_round_trip(raw.anchor_units) || !units_round_trip(raw.anchor_len) {
        return Err("anchor does not round-trip".to_owned());
    }
    Ok(ConnectFixture {
        model,
        inputs,
        endpoints,
        anchor_max_units: raw.anchor_units,
        anchor_len_units: raw.anchor_len,
        max_abs: limits.max_abs,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// M-LINKER-CHAIN — orchestrate_infill identity / ownership
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PairClass {
    Union,
    ObjectMismatch,
    RegionToolMismatch,
    PathToolMismatch,
    RoleMismatch,
    SpeedMismatch,
    WidthMismatch,
    ChainMismatch,
    Owned,
}

const PAIR_CLASSES: [PairClass; 9] = [
    PairClass::Union,
    PairClass::ObjectMismatch,
    PairClass::RegionToolMismatch,
    PairClass::PathToolMismatch,
    PairClass::RoleMismatch,
    PairClass::SpeedMismatch,
    PairClass::WidthMismatch,
    PairClass::ChainMismatch,
    PairClass::Owned,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockMode {
    Unlocked,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookupMode {
    Exact,
    Missing,
    Reversed,
    EmptyPriorChainView,
    ChainPriorEmptyView,
}

const LOOKUP_MODES: [LookupMode; 5] = [
    LookupMode::Exact,
    LookupMode::Missing,
    LookupMode::Reversed,
    LookupMode::EmptyPriorChainView,
    LookupMode::ChainPriorEmptyView,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseRole {
    Sparse,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainChoice {
    Empty,
    MaterialFuzzy,
}

/// Legal priority/name-ordered chain: `material` then `fuzzy_skin`.
fn legal_chain() -> Vec<(String, PaintValue)> {
    vec![
        ("material".to_owned(), PaintValue::ToolIndex(1)),
        ("fuzzy_skin".to_owned(), PaintValue::Flag(true)),
    ]
}

/// A distinct legal chain (different material value).
fn distinct_chain() -> Vec<(String, PaintValue)> {
    vec![
        ("material".to_owned(), PaintValue::ToolIndex(2)),
        ("fuzzy_skin".to_owned(), PaintValue::Flag(true)),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrchRaw {
    pair: PairClass,
    lock: LockMode,
    base: BaseRole,
    chain: ChainChoice,
    width: i64,
    a_rows: usize,
    b_rows: usize,
    points: usize,
    a_lower: bool,
    extras: Vec<(LookupMode, usize)>,
}

impl OrchRaw {
    fn complexity(&self) -> u128 {
        let mut total = u128::from(self.width.unsigned_abs())
            + 1_000 * (self.a_rows + self.b_rows + self.points) as u128
            + u128::from(self.a_lower)
            + 1_000_000 * self.extras.len() as u128;
        for (_, rows) in &self.extras {
            total += 1_000 * *rows as u128;
        }
        total
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChainRaw {
    Direct(DirectRaw),
    Orchestrate(OrchRaw),
}

impl ChainRaw {
    fn complexity(&self) -> u128 {
        match self {
            Self::Direct(raw) => {
                raw.shape.complexity()
                    + paths_complexity(&raw.paths)
                    + u128::from(raw.anchor_units.unsigned_abs())
                    + u128::from(raw.anchor_len.unsigned_abs())
            }
            Self::Orchestrate(raw) => raw.complexity(),
        }
    }
}

#[derive(Debug, Clone)]
struct OrchRegion {
    object_id: String,
    region_id: u64,
    prior_chain: Vec<(String, PaintValue)>,
    view_chain: Option<Vec<(String, PaintValue)>>,
    wall_source: Option<u64>,
    region_tool: u32,
    role: ExtrusionRole,
    speed: f32,
    path_tool: Option<u32>,
    width: f32,
    rows: usize,
    points: usize,
    slot: usize,
    locked: bool,
    lookup: Option<LookupMode>,
}

impl OrchRegion {
    fn matched(&self) -> bool {
        self.view_chain.as_ref() == Some(&self.prior_chain)
    }

    fn sparse(&self) -> bool {
        self.role == ExtrusionRole::SparseInfill
    }
}

#[derive(Debug, Clone)]
struct OrchCase {
    raw: OrchRaw,
    width: i64,
    slot_height: i64,
    regions: Vec<OrchRegion>,
}

fn base_role(base: BaseRole) -> ExtrusionRole {
    match base {
        BaseRole::Sparse => ExtrusionRole::SparseInfill,
        BaseRole::Solid => ExtrusionRole::TopSolidInfill,
    }
}

fn build_orch_case(raw: &OrchRaw, limits: &ChainLimits) -> Result<OrchCase, String> {
    let max_paths = limits.connect.max_paths;
    if !(MIN_REGION_WIDTH_UNITS..=MIN_REGION_WIDTH_UNITS + REGION_WIDTH_SPAN_UNITS)
        .contains(&raw.width)
        || !(1..=max_paths).contains(&raw.a_rows)
        || !(1..=max_paths).contains(&raw.b_rows)
        || !(2..=limits.connect.max_points).contains(&raw.points)
        || raw.extras.len() + 2 > limits.max_regions
        || raw
            .extras
            .iter()
            .any(|(_, rows)| !(1..=max_paths).contains(rows))
    {
        return Err(format!("orchestrate raw outside its bounds: {raw:?}"));
    }
    let chain = match raw.chain {
        ChainChoice::Empty => Vec::new(),
        ChainChoice::MaterialFuzzy => legal_chain(),
    };
    let wall = if raw.pair == PairClass::Owned {
        None
    } else {
        Some(100)
    };
    let template = OrchRegion {
        object_id: "obj-a".to_owned(),
        region_id: 0,
        prior_chain: chain.clone(),
        view_chain: Some(chain.clone()),
        wall_source: wall,
        region_tool: 0,
        role: base_role(raw.base),
        speed: 1.0,
        path_tool: Some(3),
        width: BASE_WIDTH_MM,
        rows: raw.a_rows,
        points: raw.points,
        slot: 0,
        locked: raw.lock == LockMode::Locked,
        lookup: None,
    };
    let a = OrchRegion {
        region_id: if raw.a_lower { 11 } else { 13 },
        ..template.clone()
    };
    let mut b = OrchRegion {
        region_id: 12,
        rows: raw.b_rows,
        slot: 1,
        locked: false,
        ..template.clone()
    };
    match raw.pair {
        PairClass::Union | PairClass::Owned => {}
        PairClass::ObjectMismatch => b.object_id = "obj-b".to_owned(),
        PairClass::RegionToolMismatch => b.region_tool = 1,
        PairClass::PathToolMismatch => b.path_tool = Some(4),
        PairClass::RoleMismatch => {
            b.role = match raw.base {
                BaseRole::Sparse => ExtrusionRole::TopSolidInfill,
                BaseRole::Solid => ExtrusionRole::BottomSolidInfill,
            }
        }
        PairClass::SpeedMismatch => b.speed = 0.9,
        PairClass::WidthMismatch => b.width = 0.45,
        PairClass::ChainMismatch => {
            let other = if chain.is_empty() {
                legal_chain()
            } else {
                distinct_chain()
            };
            b.prior_chain = other.clone();
            b.view_chain = Some(other);
        }
    }
    let mut regions = vec![a, b];
    for (index, (lookup, rows)) in raw.extras.iter().enumerate() {
        let legal = legal_chain();
        let mut reversed_chain = legal.clone();
        reversed_chain.reverse();
        let (prior_chain, view_chain) = match lookup {
            LookupMode::Exact => (legal.clone(), Some(legal)),
            LookupMode::Missing => (legal, None),
            LookupMode::Reversed => (legal, Some(reversed_chain)),
            LookupMode::EmptyPriorChainView => (Vec::new(), Some(legal)),
            LookupMode::ChainPriorEmptyView => (legal, Some(Vec::new())),
        };
        regions.push(OrchRegion {
            region_id: 21 + index as u64,
            prior_chain,
            view_chain,
            wall_source: None,
            rows: *rows,
            slot: 2 + index,
            locked: false,
            lookup: Some(*lookup),
            ..template.clone()
        });
    }
    let case = OrchCase {
        raw: raw.clone(),
        width: raw.width,
        slot_height: limits.slot_height,
        regions,
    };
    // Fixture self-check: only the `Union` pair class is jointly linkable.
    if orch_joinable(&case.regions[0], &case.regions[1]) != (raw.pair == PairClass::Union) {
        return Err(format!(
            "{:?} fixture disagrees with the ownership table",
            raw.pair
        ));
    }
    Ok(case)
}

/// Independent ownership table: two regions are linked as one union group
/// exactly when both resolve an exact view and share object, region tool,
/// ordered chain, wall source, role, speed bits, authored tool, endpoint
/// width and config (all fixture configs are identical).
fn orch_joinable(a: &OrchRegion, b: &OrchRegion) -> bool {
    a.matched()
        && b.matched()
        && a.object_id == b.object_id
        && a.region_tool == b.region_tool
        && a.prior_chain == b.prior_chain
        && a.wall_source.is_some()
        && a.wall_source == b.wall_source
        && a.role == b.role
        && bits(a.speed) == bits(b.speed)
        && a.path_tool == b.path_tool
        && (f64::from(a.width) - f64::from(b.width)).abs() <= f64::from(WIDTH_LIMIT_MM)
}

fn orch_group(case: &OrchCase, index: usize) -> Vec<usize> {
    (0..case.regions.len())
        .filter(|&other| {
            other == index || orch_joinable(&case.regions[index], &case.regions[other])
        })
        .collect()
}

fn row_y(case: &OrchCase, region: &OrchRegion, row: usize) -> i64 {
    region.slot as i64 * case.slot_height + (row as i64 + 1) * ROW_PITCH_UNITS
}

fn region_meta(region: &OrchRegion) -> PathMeta {
    PathMeta {
        role: region.role.clone(),
        speed: region.speed,
        tool: region.path_tool,
        first_width: region.width,
        last_width: region.width,
    }
}

fn row_path(case: &OrchCase, region: &OrchRegion, row: usize) -> ExtrusionPath3D {
    let y = row_y(case, region, row);
    let (x0, x1) = (-ROW_OVERSHOOT_UNITS, case.width + ROW_OVERSHOOT_UNITS);
    let count = region.points as i64;
    let points = (0..count)
        .map(|index| {
            let x = if index == count - 1 {
                x1
            } else {
                x0 + (x1 - x0) * index / (count - 1)
            };
            endpoint_point((x, y), 0.2, region.width, 1.0)
        })
        .collect();
    make_path(points, &region_meta(region))
}

fn locked_path(case: &OrchCase, region: &OrchRegion) -> ExtrusionPath3D {
    let y = region.slot as i64 * case.slot_height + case.slot_height - ROW_PITCH_UNITS / 2;
    let mid = case.width / 2;
    ExtrusionPath3D {
        order_lock: Some(7),
        ..make_path(
            vec![
                endpoint_point((mid - 10_000, y), 0.2, region.width, 1.0),
                endpoint_point((mid + 10_000, y), 0.2, region.width, 1.0),
            ],
            &region_meta(region),
        )
    }
}

fn region_paths(case: &OrchCase, region: &OrchRegion) -> Vec<ExtrusionPath3D> {
    let mut paths = (0..region.rows)
        .map(|row| row_path(case, region, row))
        .collect::<Vec<_>>();
    if region.locked {
        // Authored last: the orchestrator must still emit it first.
        paths.push(locked_path(case, region));
    }
    paths
}

fn prior_region(case: &OrchCase, region: &OrchRegion) -> InfillRegion {
    let paths = region_paths(case, region);
    let (sparse_infill, solid_infill) = if region.sparse() {
        (paths, Vec::new())
    } else {
        (Vec::new(), paths)
    };
    InfillRegion {
        object_id: region.object_id.clone(),
        region_id: region.region_id,
        variant_chain: region.prior_chain.clone(),
        sparse_infill,
        solid_infill,
        ..Default::default()
    }
}

fn region_config() -> ConfigView {
    ConfigViewBuilder::new()
        .float("line_width", 0.4)
        .float("sparse_infill_density", 20.0)
        .float("layer_height", 0.2)
        .float("infill_overlap", f64::from(INFILL_OVERLAP))
        .float_or_percent("infill_anchor_max", 20.0, false)
        .build()
}

fn region_rect(case: &OrchCase, slot: usize) -> ExPolygon {
    let y0 = slot as i64 * case.slot_height;
    ExPolygon {
        contour: polygon_of(&rect_ccw(0, y0, case.width, y0 + case.slot_height)),
        holes: Vec::new(),
    }
}

fn region_view(case: &OrchCase, region: &OrchRegion) -> Option<PerimeterRegionView> {
    let chain = region.view_chain.clone()?;
    let area = region_rect(case, region.slot);
    let mut view = PerimeterRegionViewBuilder::new()
        .object_id(region.object_id.clone())
        .region_id(region.region_id)
        .add_infill_area(area.clone())
        .sparse_infill_area(vec![area.clone()])
        .top_solid_fill(vec![area.clone()])
        .bottom_solid_fill(vec![area])
        .wall_source_region_id(region.wall_source)
        .tool_index(region.region_tool)
        .build();
    view.set_config(region_config());
    view.set_variant_chain(chain);
    Some(view)
}

fn run_orchestrate(case: &OrchCase) -> Result<InfillOutputBuilder, String> {
    let prior = case
        .regions
        .iter()
        .map(|region| prior_region(case, region))
        .collect::<Vec<_>>();
    let views = case
        .regions
        .iter()
        .filter_map(|region| region_view(case, region))
        .collect::<Vec<_>>();
    let mut output = InfillOutputBuilder::new();
    orchestrate_infill(
        &prior,
        &views,
        INFILL_OVERLAP,
        DEFAULT_LINE_WIDTH,
        &mut output,
    )?;
    Ok(output)
}

#[derive(Debug, Default, Clone, Copy)]
struct OrchSummary {
    outputs: u64,
    passthrough_regions: u64,
    locked_bypass: u64,
    cross_region_paths: u64,
    majority_unequal: u64,
    majority_tie: u64,
    rows: u64,
}

fn origin_index(case: &OrchCase, origin: &Option<RegionOrigin>, k: usize) -> Result<usize, String> {
    let origin = origin
        .as_ref()
        .ok_or_else(|| format!("output path {k} has no origin"))?;
    case.regions
        .iter()
        .position(|region| {
            region.object_id == origin.object_id
                && region.region_id == origin.region_id
                && region.prior_chain == origin.variant_chain
        })
        .ok_or_else(|| format!("origin {origin:?} matches no prior region's full ordered identity"))
}

/// Per-owner row length (units) of one linked output path, after checking its
/// segments are rows of the group or axis-aligned boundary walks inside it.
fn path_ownership(
    case: &OrchCase,
    group: &[usize],
    path: &ExtrusionPath3D,
) -> Result<BTreeMap<usize, (f64, Vec<usize>)>, String> {
    let tol = ORCH_SEG_TOL_UNITS;
    let y_min = group
        .iter()
        .map(|&i| case.regions[i].slot)
        .min()
        .unwrap_or(0) as f64
        * case.slot_height as f64;
    let y_max = (group
        .iter()
        .map(|&i| case.regions[i].slot)
        .max()
        .unwrap_or(0)
        + 1) as f64
        * case.slot_height as f64;
    let mut owners: BTreeMap<usize, (f64, Vec<usize>)> =
        group.iter().map(|&i| (i, (0.0, Vec::new()))).collect();
    for point in &path.points {
        let (x, y) = units_of(point);
        if x < -tol || x > case.width as f64 + tol || y < y_min - tol || y > y_max + tol {
            return Err(format!(
                "point ({x}, {y}) lies outside its ownership group's area"
            ));
        }
    }
    for pair in path.points.windows(2) {
        let (a, b) = (units_of(&pair[0]), units_of(&pair[1]));
        let (dx, dy) = ((b.0 - a.0).abs(), (b.1 - a.1).abs());
        if dx > tol && dy > tol {
            return Err(format!(
                "segment {a:?} -> {b:?} is a diagonal chord, not a row or ring walk"
            ));
        }
        if dy > tol {
            continue;
        }
        let row = case.regions.iter().enumerate().find_map(|(index, region)| {
            (0..region.rows)
                .find(|&row| (a.1 - row_y(case, region, row) as f64).abs() <= tol)
                .map(|row| (index, row))
        });
        if let Some((owner, row)) = row {
            let Some(entry) = owners.get_mut(&owner) else {
                return Err(format!(
                    "row {row} of region {} is extruded in a path of another ownership group",
                    case.regions[owner].region_id
                ));
            };
            entry.0 += dx;
            if !entry.1.contains(&row) {
                entry.1.push(row);
            }
        }
    }
    if owners.values().all(|(length, _)| *length <= 0.0) {
        return Err("linked output path covers no source row".to_owned());
    }
    Ok(owners)
}

fn expected_owner(case: &OrchCase, owners: &BTreeMap<usize, (f64, Vec<usize>)>) -> usize {
    let mut ranked = owners
        .iter()
        .map(|(index, (length, _))| (*index, *length))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| {
                case.regions[left.0]
                    .region_id
                    .cmp(&case.regions[right.0].region_id)
            })
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked[0].0
}

/// Orchestrate checker over raw output slices (so controls can mutate them).
fn check_orchestrate(
    case: &OrchCase,
    sparse: (&[ExtrusionPath3D], &[Option<RegionOrigin>]),
    solid: (&[ExtrusionPath3D], &[Option<RegionOrigin>]),
) -> Result<OrchSummary, String> {
    let mut summary = OrchSummary::default();
    for (is_sparse, (paths, origins)) in [(true, sparse), (false, solid)] {
        if paths.len() != origins.len() {
            return Err("output paths and origins differ in length".to_owned());
        }
        summary.outputs += paths.len() as u64;
        let owners = origins
            .iter()
            .enumerate()
            .map(|(k, origin)| origin_index(case, origin, k))
            .collect::<Result<Vec<_>, _>>()?;
        if owners.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err("outputs are not emitted in prior-region order".to_owned());
        }
        let mut row_hits: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (index, region) in case.regions.iter().enumerate() {
            let outs = paths
                .iter()
                .zip(&owners)
                .filter(|(_, owner)| **owner == index)
                .map(|(path, _)| path)
                .collect::<Vec<_>>();
            let priors = if region.sparse() == is_sparse {
                region_paths(case, region)
            } else {
                Vec::new()
            };
            if !region.matched() {
                if outs.len() != priors.len()
                    || !outs.iter().zip(&priors).all(|(a, b)| path_bits_eq(a, b))
                {
                    return Err(format!(
                        "region {} without an exact view was not passed through unchanged",
                        region.region_id
                    ));
                }
                summary.passthrough_regions += u64::from(!priors.is_empty());
                continue;
            }
            for locked in priors.iter().filter(|path| path.order_lock.is_some()) {
                let positions = outs
                    .iter()
                    .enumerate()
                    .filter(|(_, path)| path_bits_eq(path, locked))
                    .map(|(position, _)| position)
                    .collect::<Vec<_>>();
                if positions.len() != 1 {
                    return Err(format!(
                        "locked path of region {} appears {} times, not once unchanged",
                        region.region_id,
                        positions.len()
                    ));
                }
                if outs
                    .iter()
                    .take(positions[0])
                    .any(|path| path.order_lock.is_none() && path.role == locked.role)
                {
                    return Err(format!(
                        "locked path of region {} is not emitted first",
                        region.region_id
                    ));
                }
                summary.locked_bypass += 1;
            }
            let group = orch_group(case, index);
            for path in outs.iter().filter(|path| path.order_lock.is_none()) {
                if path.role != region.role
                    || bits(path.speed_factor) != bits(region.speed)
                    || path.tool_index != region.path_tool
                {
                    return Err(format!(
                        "linked path of region {} lost its authored role/speed/tool",
                        region.region_id
                    ));
                }
                let ownership = path_ownership(case, &group, path)
                    .map_err(|error| format!("region {}: {error}", region.region_id))?;
                let owner = expected_owner(case, &ownership);
                if owner != index {
                    return Err(format!(
                        "path attributed to region {} but its independent majority owner is region {}",
                        region.region_id, case.regions[owner].region_id
                    ));
                }
                let positive = ownership
                    .values()
                    .filter(|(length, _)| *length > 0.0)
                    .collect::<Vec<_>>();
                if positive.len() > 1 {
                    summary.cross_region_paths += 1;
                    let mut lengths = positive
                        .iter()
                        .map(|(length, _)| *length)
                        .collect::<Vec<_>>();
                    lengths.sort_by(|a, b| b.total_cmp(a));
                    if lengths[0] == lengths[1] {
                        summary.majority_tie += 1;
                    } else {
                        summary.majority_unequal += 1;
                    }
                }
                for (owner, (length, rows)) in &ownership {
                    if *length > 0.0 {
                        for row in rows {
                            *row_hits.entry((*owner, *row)).or_default() += 1;
                        }
                    }
                }
            }
        }
        for (index, region) in case.regions.iter().enumerate() {
            if !region.matched() || region.sparse() != is_sparse {
                continue;
            }
            for row in 0..region.rows {
                match row_hits.get(&(index, row)).copied().unwrap_or(0) {
                    1 => summary.rows += 1,
                    hits => {
                        return Err(format!(
                            "row {row} of region {} is extruded by {hits} output paths, not exactly one",
                            region.region_id
                        ))
                    }
                }
            }
        }
    }
    Ok(summary)
}

fn check_orchestrate_output(
    case: &OrchCase,
    output: &InfillOutputBuilder,
) -> Result<OrchSummary, String> {
    check_orchestrate(
        case,
        (output.sparse_paths(), output.sparse_path_origins()),
        (output.solid_paths(), output.solid_path_origins()),
    )
}

#[derive(Default)]
struct ChainCounters {
    cases: Cell<u64>,
    chain_only: Cell<u64>,
    chain_only_reversed: Cell<u64>,
    chain_only_reordered: Cell<u64>,
    chain_connect: Cell<u64>,
    chain_connect_junctions: Cell<u64>,
    orchestrate: Cell<u64>,
    union_pairs: Cell<u64>,
    separated_pairs: Cell<u64>,
    cross_region_paths: Cell<u64>,
    majority_unequal: Cell<u64>,
    majority_tie: Cell<u64>,
    passthrough_regions: Cell<u64>,
    locked_bypass: Cell<u64>,
    legal_chain_matches: Cell<u64>,
    rows: Cell<u64>,
}

impl ChainCounters {
    fn report(&self, label: &str) -> String {
        format!(
            "{label} cases={} chain_only={} chain_only_reversed={} chain_only_reordered={} chain_connect={} \
             chain_connect_junctions={} orchestrate={} union_pairs={} separated_pairs={} cross_region_paths={} \
             majority_unequal={} majority_tie={} passthrough_regions={} locked_bypass={} legal_chain_matches={} rows={}",
            self.cases.get(),
            self.chain_only.get(),
            self.chain_only_reversed.get(),
            self.chain_only_reordered.get(),
            self.chain_connect.get(),
            self.chain_connect_junctions.get(),
            self.orchestrate.get(),
            self.union_pairs.get(),
            self.separated_pairs.get(),
            self.cross_region_paths.get(),
            self.majority_unequal.get(),
            self.majority_tie.get(),
            self.passthrough_regions.get(),
            self.locked_bypass.get(),
            self.legal_chain_matches.get(),
            self.rows.get()
        )
    }

    fn assert_nonvacuous(&self) {
        for (name, cell) in [
            ("chain_only", &self.chain_only),
            ("chain_only_reversed", &self.chain_only_reversed),
            ("chain_only_reordered", &self.chain_only_reordered),
            ("chain_connect", &self.chain_connect),
            ("chain_connect_junctions", &self.chain_connect_junctions),
            ("union_pairs", &self.union_pairs),
            ("separated_pairs", &self.separated_pairs),
            ("cross_region_paths", &self.cross_region_paths),
            ("majority_unequal", &self.majority_unequal),
            ("majority_tie", &self.majority_tie),
            ("passthrough_regions", &self.passthrough_regions),
            ("locked_bypass", &self.locked_bypass),
            ("legal_chain_matches", &self.legal_chain_matches),
            ("rows", &self.rows),
        ] {
            assert!(
                cell.get() > 0,
                "M-LINKER-CHAIN population `{name}` is empty"
            );
        }
    }
}

fn run_direct_chain(
    raw: &DirectRaw,
    limits: &ConnectLimits,
    counters: &ChainCounters,
) -> Result<(), String> {
    let fx = build_direct_fixture(raw, limits)?;
    let graph = fx.graph();
    check_projection(&fx, &graph)?;
    let anchor = fx.anchor();
    if anchor.dont_connect() != (raw.top == ChainTop::ChainOnly) {
        return Err("selected chain side disagrees with AnchorParams::dont_connect".to_owned());
    }
    let output = chain_or_connect_infill(fx.inputs.clone(), &graph, anchor);
    if raw.top == ChainTop::ChainOnly {
        let (reversed_count, reordered) = check_chain_only(&fx.inputs, &output)?;
        inc(&counters.chain_only);
        add(&counters.chain_only_reversed, reversed_count as u64);
        add(&counters.chain_only_reordered, u64::from(reordered));
    } else {
        let summary = check_connect_output(&fx, &output, MergeExpectation::Free)?;
        inc(&counters.chain_connect);
        add(&counters.chain_connect_junctions, summary.junctions);
    }
    Ok(())
}

fn run_orch_case(case: &OrchCase, counters: &ChainCounters) -> Result<OrchSummary, String> {
    let output = run_orchestrate(case)?;
    let summary = check_orchestrate_output(case, &output)?;
    inc(&counters.orchestrate);
    if case.raw.pair == PairClass::Union {
        inc(&counters.union_pairs);
    } else {
        inc(&counters.separated_pairs);
    }
    add(&counters.cross_region_paths, summary.cross_region_paths);
    add(&counters.majority_unequal, summary.majority_unequal);
    add(&counters.majority_tie, summary.majority_tie);
    add(&counters.passthrough_regions, summary.passthrough_regions);
    add(&counters.locked_bypass, summary.locked_bypass);
    add(
        &counters.legal_chain_matches,
        case.regions
            .iter()
            .filter(|region| region.matched() && region.prior_chain == legal_chain())
            .count() as u64,
    );
    add(&counters.rows, summary.rows);
    Ok(summary)
}

fn run_chain_raw(
    raw: &ChainRaw,
    limits: &ChainLimits,
    counters: &ChainCounters,
) -> Result<(), String> {
    inc(&counters.cases);
    match raw {
        ChainRaw::Direct(direct) => run_direct_chain(direct, &limits.connect, counters),
        ChainRaw::Orchestrate(orch) => {
            run_orch_case(&build_orch_case(orch, limits)?, counters).map(|_| ())
        }
    }
}

fn chain_strategy(
    limits: &ChainLimits,
    tops: &[ChainTop],
    pairs: &[PairClass],
    locks: &[LockMode],
) -> BoxedStrategy<ChainRaw> {
    let limits = *limits;
    let pairs = pairs.to_vec();
    let locks = locks.to_vec();
    prop::sample::select(tops.to_vec())
        .no_shrink()
        .prop_ind_flat_map(move |top| match top {
            ChainTop::ChainOnly | ChainTop::ChainConnect => {
                let anchor = if top == ChainTop::ChainOnly {
                    (0..DONT_CONNECT_UNITS).boxed()
                } else {
                    (DONT_CONNECT_UNITS..=DONT_CONNECT_UNITS + limits.connect.max_anchor_units)
                        .boxed()
                };
                (
                    shape_strategy(limits.connect.max_abs, limits.connect.max_cell),
                    prop::collection::vec(
                        path_raw_strategy(&limits.connect),
                        2..=limits.connect.max_paths,
                    ),
                    anchor,
                    prop::option::of(1..=limits.connect.max_anchor_units),
                )
                    .prop_map(move |(shape, paths, anchor_units, anchor_len)| {
                        ChainRaw::Direct(DirectRaw {
                            top,
                            shape,
                            paths,
                            anchor_units,
                            anchor_len: anchor_len.unwrap_or(0),
                        })
                    })
                    .boxed()
            }
            ChainTop::Orchestrate => orch_strategy(&limits, &pairs, &locks)
                .prop_map(ChainRaw::Orchestrate)
                .boxed(),
        })
        .boxed()
}

fn orch_strategy(
    limits: &ChainLimits,
    pairs: &[PairClass],
    locks: &[LockMode],
) -> BoxedStrategy<OrchRaw> {
    let limits = *limits;
    (
        prop::sample::select(pairs.to_vec()).no_shrink(),
        prop::sample::select(locks.to_vec()).no_shrink(),
        prop::sample::select(vec![BaseRole::Sparse, BaseRole::Solid]).no_shrink(),
        prop::sample::select(vec![ChainChoice::Empty, ChainChoice::MaterialFuzzy]).no_shrink(),
    )
        .prop_ind_flat_map(move |(pair, lock, base, chain)| {
            let max_paths = limits.connect.max_paths;
            (
                MIN_REGION_WIDTH_UNITS..=MIN_REGION_WIDTH_UNITS + REGION_WIDTH_SPAN_UNITS,
                1..=max_paths,
                1..=max_paths,
                2..=limits.connect.max_points,
                any::<bool>(),
                prop::collection::vec(
                    (
                        prop::sample::select(LOOKUP_MODES.to_vec()).no_shrink(),
                        1..=max_paths,
                    ),
                    0..=limits.max_regions - 2,
                ),
            )
                .prop_map(move |(width, a_rows, b_rows, points, a_lower, extras)| {
                    OrchRaw {
                        pair,
                        lock,
                        base,
                        chain,
                        width,
                        a_rows,
                        b_rows,
                        points,
                        a_lower,
                        extras,
                    }
                })
        })
        .boxed()
}

fn witness_orch(
    pair: PairClass,
    lock: LockMode,
    base: BaseRole,
    a_rows: usize,
    b_rows: usize,
    a_lower: bool,
    extras: Vec<(LookupMode, usize)>,
) -> OrchRaw {
    OrchRaw {
        pair,
        lock,
        base,
        chain: ChainChoice::MaterialFuzzy,
        width: 80_000,
        a_rows,
        b_rows,
        points: 3,
        a_lower,
        extras,
    }
}

fn chain_only_witness() -> (ConnectFixture, Vec<ExtrusionPath3D>) {
    // P0 (2,2)->(8,2) mm and P1 (1,5)->(9,5) mm on a 10 mm square.
    let model = shape_model(&[ExSpec {
        contour: rect_ccw(0, 0, 100_000, 100_000),
        holes: Vec::new(),
    }])
    .expect("chain-only model");
    let meta = base_meta();
    let inputs = vec![
        path_from((20_000, 20_000), &[], (80_000, 20_000), &meta),
        path_from(
            (10_000, 50_000),
            &[(50_000, 50_000)],
            (90_000, 50_000),
            &meta,
        ),
    ];
    let fixture = ConnectFixture {
        model,
        inputs: inputs.clone(),
        endpoints: vec![None, None],
        anchor_max_units: 0,
        anchor_len_units: 0,
        max_abs: 200_000,
    };
    let output = chain_or_connect_infill(inputs, &fixture.graph(), fixture.anchor());
    (fixture, output)
}

fn chain_witnesses(limits: &ChainLimits, counters: &ChainCounters) {
    inc(&counters.cases);
    let (fixture, output) = chain_only_witness();
    let (reversed_count, reordered) =
        check_chain_only(&fixture.inputs, &output).expect("chain-only witness");
    inc(&counters.chain_only);
    add(&counters.chain_only_reversed, reversed_count as u64);
    add(&counters.chain_only_reordered, u64::from(reordered));

    let direct = DirectRaw {
        top: ChainTop::ChainConnect,
        shape: witness_connect_raw(ConnectClass::Merge).shape,
        paths: witness_connect_raw(ConnectClass::Multi).paths,
        anchor_units: 50_000,
        anchor_len: 2_000,
    };
    run_direct_chain(&direct, &limits.connect, counters).expect("chain-connect witness");

    // Unequal majority: A (id 13) has two rows, B (id 12) one; A must own the
    // merged path even though B has the lower region id.
    let unequal = build_orch_case(
        &witness_orch(
            PairClass::Union,
            LockMode::Unlocked,
            BaseRole::Sparse,
            2,
            1,
            false,
            Vec::new(),
        ),
        limits,
    )
    .expect("unequal witness");
    let summary = run_orch_case(&unequal, counters).expect("unequal majority witness");
    assert!(
        summary.majority_unequal > 0,
        "unequal witness merged no cross-region path: {summary:?}"
    );
    // Tie: one row each, A (id 13) first in prior order; the tie goes to the
    // lower region id B (12), not to the first point's owner.
    let tie = build_orch_case(
        &witness_orch(
            PairClass::Union,
            LockMode::Unlocked,
            BaseRole::Solid,
            1,
            1,
            false,
            Vec::new(),
        ),
        limits,
    )
    .expect("tie witness");
    let output = run_orchestrate(&tie).expect("tie orchestrate");
    let summary = check_orchestrate_output(&tie, &output).expect("tie majority witness");
    assert!(
        summary.majority_tie > 0,
        "tie witness merged no cross-region path: {summary:?}"
    );
    assert!(output
        .solid_path_origins()
        .iter()
        .all(|origin| origin.as_ref().map(|origin| origin.region_id) == Some(12)));
    run_orch_case(&tie, counters).expect("tie witness counters");
    for pair in PAIR_CLASSES {
        let lock = if pair == PairClass::Owned {
            LockMode::Locked
        } else {
            LockMode::Unlocked
        };
        let case = build_orch_case(
            &witness_orch(
                pair,
                lock,
                BaseRole::Sparse,
                2,
                2,
                true,
                vec![(LookupMode::Exact, 1)],
            ),
            limits,
        )
        .unwrap_or_else(|error| panic!("{pair:?} witness: {error}"));
        run_orch_case(&case, counters).unwrap_or_else(|error| panic!("{pair:?} witness: {error}"));
    }
    let lookups = build_orch_case(
        &witness_orch(
            PairClass::Union,
            LockMode::Locked,
            BaseRole::Solid,
            1,
            2,
            true,
            vec![(LookupMode::Missing, 1), (LookupMode::Reversed, 1)],
        ),
        limits,
    )
    .expect("lookup witness");
    run_orch_case(&lookups, counters).expect("lookup witness");
    let empty_chain = build_orch_case(
        &witness_orch(
            PairClass::Owned,
            LockMode::Unlocked,
            BaseRole::Sparse,
            1,
            1,
            true,
            vec![
                (LookupMode::EmptyPriorChainView, 1),
                (LookupMode::ChainPriorEmptyView, 2),
            ],
        ),
        limits,
    )
    .expect("empty-chain witness");
    run_orch_case(&empty_chain, counters).expect("empty chain is not a wildcard");
}

#[test]
fn proptest_linker_chain_and_origins() {
    let run = PropertyRun::new(CHAIN_ID, CHAIN_TEST).expect("M-LINKER-CHAIN registry context");
    let limits = checked_chain_limits(&live_bounds(&run, &CHAIN_BOUND_KEYS)).expect("chain bounds");
    let witness = ChainCounters::default();
    chain_witnesses(&limits, &witness);
    witness.assert_nonvacuous();
    let generated = ChainCounters::default();
    let strategy = chain_strategy(
        &limits,
        &[
            ChainTop::ChainOnly,
            ChainTop::ChainConnect,
            ChainTop::Orchestrate,
        ],
        &PAIR_CLASSES,
        &[LockMode::Unlocked, LockMode::Locked],
    );
    run.check(&strategy, |raw, _guard| {
        match run_chain_raw(&raw, &limits, &generated) {
            Ok(()) => Ok(()),
            Err(message) => fail(message),
        }
    })
    .expect("M-LINKER-CHAIN chain and origins");
    assert!(generated.cases.get() > 0);
    // Structural invariant: every generated case takes exactly one chain mode.
    // A specific mode can legitimately be absent from a 64-case run, so the
    // per-mode populations are proven by the deterministic witness above
    // (`chain_witnesses` + `ChainCounters::assert_nonvacuous`) rather than by
    // sampling.
    assert_eq!(
        generated.chain_only.get() + generated.chain_connect.get() + generated.orchestrate.get(),
        generated.cases.get(),
        "M-LINKER-CHAIN every generated case must take exactly one chain mode"
    );
    println!("{}", witness.report("M-LINKER-CHAIN witness"));
    println!("{}", generated.report("M-LINKER-CHAIN generated"));
}

/// Assert the generated-run populations that every accepted call must exercise.
///
/// Only per-case invariants that hold for every accepted case belong here.
/// Mode-sampled populations (a specific ring class, chain mode, pair class or
/// majority case) can legitimately be absent from a 64-case run, and the
/// deterministic witnesses in `linker_controls` assert every one of them on
/// every run instead.
fn assert_generated_populations(property: &str, populations: &[(&str, u64)]) {
    for (name, value) in populations {
        assert!(
            *value > 0,
            "{property} generated population `{name}` is empty"
        );
    }
}

fn expect_rejected<T: Debug>(label: &str, result: Result<T, String>, needle: &str) {
    match result {
        Ok(value) => panic!("{label} was accepted: {value:?}"),
        Err(message) => assert!(
            message.contains(needle),
            "{label} was rejected for the wrong reason: {message}"
        ),
    }
}

fn linker_graph_controls() {
    let counters = GraphCounters::default();
    graph_witnesses(&counters);
    counters.assert_nonvacuous();

    let notch = witness_graph(
        RingClass::Notch,
        ArcMode::Free,
        &notch_spec(),
        &[(0, 160_000, 290_000, false, 200_000)],
    );
    let query = &notch.queries[0];
    let ring = &notch.model.rings[0];
    let graph = BoundaryInfillGraph::new(&notch.model.expolygons);
    let real = contour_connector(
        &graph.rings()[0],
        160_000.0,
        290_000.0,
        &query.start,
        &query.end,
    );
    check_connector(
        ring,
        160_000,
        290_000,
        &query.start,
        &query.end,
        &real,
        false,
        grid_tol_units(200_000),
    )
    .expect("real notch connector");
    expect_rejected(
        "notch chord",
        check_connector(
            ring,
            160_000,
            290_000,
            &query.start,
            &query.end,
            &[],
            false,
            grid_tol_units(200_000),
        ),
        "not the independent shortest ring walk",
    );
    expect_rejected(
        "notch chord walk",
        check_walk_on_ring(
            ring,
            &[units_of(&query.start), units_of(&query.end)],
            grid_tol_units(200_000),
        ),
        "does not lie on ring",
    );
    // Keeping the corner count but skipping the reflex corners is also a chord.
    let mut shortcut = real.clone();
    shortcut[1] = with_xy(&shortcut[1], 6.0, 6.0);
    shortcut[2] = with_xy(&shortcut[2], 3.0, 6.0);
    expect_rejected(
        "notch shortcut",
        check_connector(
            ring,
            160_000,
            290_000,
            &query.start,
            &query.end,
            &shortcut,
            false,
            grid_tol_units(200_000),
        ),
        "not the independent shortest ring walk",
    );

    // A connector attributed to a different ring must fail.
    let hole = witness_graph(
        RingClass::Hole,
        ArcMode::Free,
        &holed_square_spec(),
        &[(1, 10_000, 100_000, false, 20_000)],
    );
    let hole_graph = BoundaryInfillGraph::new(&hole.model.expolygons);
    let hq = &hole.queries[0];
    let hole_connector = contour_connector(
        &hole_graph.rings()[1],
        410_000.0,
        500_000.0,
        &hq.start,
        &hq.end,
    );
    check_connector(
        &hole.model.rings[1],
        10_000,
        100_000,
        &hq.start,
        &hq.end,
        &hole_connector,
        false,
        grid_tol_units(200_000),
    )
    .expect("real hole connector");
    expect_rejected(
        "hole connector judged on the outer ring",
        check_walk_on_ring(
            &hole.model.rings[0],
            &std::iter::once(units_of(&hq.start))
                .chain(hole_connector.iter().map(units_of))
                .chain(std::iter::once(units_of(&hq.end)))
                .collect::<Vec<_>>(),
            grid_tol_units(200_000),
        ),
        "does not lie on ring",
    );
    let outer_corners = hole.model.rings[0]
        .walk(10_000, 100_000, RingDirection::Forward)
        .into_iter()
        .map(|units| endpoint_point(units, 0.2, 0.4, 1.0))
        .collect::<Vec<_>>();
    expect_rejected(
        "outer-ring corners as the hole connector",
        check_connector(
            &hole.model.rings[1],
            10_000,
            100_000,
            &hq.start,
            &hq.end,
            &outer_corners,
            false,
            grid_tol_units(200_000),
        ),
        "not the independent shortest ring walk",
    );

    // Empty graph is a distinct legal mode, and a nonempty model never matches it.
    let empty = BoundaryInfillGraph::new(&[]);
    check_graph_structure(&shape_model(&[]).expect("empty model"), &empty)
        .expect("empty graph structure");
    assert!(check_graph_structure(&notch.model, &empty).is_err());
    assert!(empty.rings().is_empty() && empty.project(Point2 { x: 0, y: 0 }).is_none());

    // Stub controls: nonpositive budget is empty; a truncated stub is rejected.
    let rect = witness_graph(
        RingClass::Rectangle,
        ArcMode::Free,
        &[ExSpec {
            contour: rect_ccw(0, 0, 100_000, 60_000),
            holes: Vec::new(),
        }],
        &[(0, 20_000, 130_000, true, 100_000)],
    );
    let rect_graph = BoundaryInfillGraph::new(&rect.model.expolygons);
    let anchor = &rect.queries[0].start;
    for budget in [0.0, -1.0, f64::NAN] {
        assert!(contour_stub(
            &rect_graph.rings()[0],
            20_000.0,
            RingDirection::Forward,
            budget,
            anchor
        )
        .is_empty());
    }
    let stub = contour_stub(
        &rect_graph.rings()[0],
        20_000.0,
        RingDirection::Forward,
        100_000.0,
        anchor,
    );
    let (grid, lerp) = (grid_tol_units(200_000), lerp_tol_units(200_000));
    check_stub(
        &rect.model.rings[0],
        20_000,
        RingDirection::Forward,
        100_000,
        anchor,
        &stub,
        grid,
        lerp,
    )
    .expect("real stub");
    expect_rejected(
        "stub missing its corner",
        check_stub(
            &rect.model.rings[0],
            20_000,
            RingDirection::Forward,
            100_000,
            anchor,
            &stub[1..],
            grid,
            lerp,
        ),
        "stub has",
    );
}

fn two_path_fixture(first: PathMeta, second: PathMeta) -> ConnectFixture {
    // 10 mm square; P0 on the left edge (0,2)->(0,4) mm, P1 on the left edge
    // (0,6)->(0,8) mm: the 2 mm shortest arc (0,4)->(0,6) has no corner.
    let model = shape_model(&[ExSpec {
        contour: rect_ccw(0, 0, 100_000, 100_000),
        holes: Vec::new(),
    }])
    .expect("two-path model");
    let ring = &model.rings[0];
    let endpoint = |local: i64| Endpoint {
        ring: 0,
        local,
        units: ring.at(local),
    };
    let e = [
        endpoint(380_000),
        endpoint(360_000),
        endpoint(340_000),
        endpoint(320_000),
    ];
    let inputs = vec![
        path_from(e[0].units, &[(30_000, 30_000)], e[1].units, &first),
        path_from(e[2].units, &[(30_000, 70_000)], e[3].units, &second),
    ];
    ConnectFixture {
        model,
        inputs,
        endpoints: vec![Some([e[0], e[1]]), Some([e[2], e[3]])],
        anchor_max_units: 50_000,
        anchor_len_units: 0,
        max_abs: 200_000,
    }
}

/// Fabricates the merge the SUT would emit if the two sources were joined.
fn fabricated_merge(fx: &ConnectFixture) -> Vec<ExtrusionPath3D> {
    let mut merged = fx.inputs[0].clone();
    merged.points.extend(fx.inputs[1].points.iter().copied());
    vec![merged]
}

/// The retained extended-tier `Stub` case (seed 12768350826874075087), whose
/// exact shortest arc equals its anchor at 3110 units. See the header's
/// strict-anchor derivation and `linker_connect_controls`.
fn retained_stub_case() -> ConnectCase {
    let raw = ConnectRaw {
        class: ConnectClass::Stub,
        shape: ShapeParams {
            ox: -1,
            oy: -1,
            cell: 6001,
            cw: 4,
            ch: 6,
            p: [0, 0, 0, 0, 0, 0],
        },
        paths: vec![
            PathRaw {
                ring: 205_935_360,
                start: 5_888_230_514_469_562_159,
                end: 765_669_561_936_661_080,
                interior: vec![(-14_984, 77_860), (-77_094, 23_491)],
                meta: 22,
            },
            PathRaw {
                ring: 149_400_712,
                start: 2_245_242_238_599_216_109,
                end: 848_144_915_981_946_281,
                interior: vec![(33_024, -90_375)],
                meta: 23,
            },
        ],
        slack: 21_004,
        anchor_len: 41_057,
        first_ulps: 5,
        last_ulps: 28,
        above_ulps: 30,
        alt: 1_333_532_815,
        dont_connect_units: 235,
    };
    let limits = checked_connect_limits(&connect_probe_bounds()).expect("connect limits");
    build_connect_case(&raw, &limits).expect("retained stub case")
}

fn linker_connect_controls() {
    let limits = checked_connect_limits(&connect_probe_bounds()).expect("connect limits");
    let counters = ConnectCounters::default();
    connect_witnesses(&limits, &counters);
    counters.assert_nonvacuous();

    // Compatible pair: real merge passes; dropping a source fails.
    let fx = two_path_fixture(base_meta(), base_meta());
    let output = connect_infill(fx.inputs.clone(), &fx.graph(), fx.anchor());
    check_connect_output(&fx, &output, MergeExpectation::MustMerge(20_000))
        .expect("compatible merge");
    let mut dropped = output.clone();
    let cut = dropped[0].points.len() - fx.inputs[1].points.len();
    dropped[0].points.truncate(cut);
    expect_rejected(
        "dropped source",
        check_connect_output(&fx, &dropped, MergeExpectation::Free),
        "dropped",
    );
    expect_rejected(
        "dropped whole output",
        check_connect_output(&fx, &[], MergeExpectation::Free),
        "dropped",
    );
    // Path order alone is not a merge: the unmerged inputs fail MustMerge.
    expect_rejected(
        "unmerged compatible pair",
        check_connect_output(&fx, &fx.inputs, MergeExpectation::MustMerge(20_000)),
        "must merge",
    );
    // A chord-routed join between the pair (skipping the left edge) fails.
    let mut chord = output.clone();
    chord[0].points.insert(
        fx.inputs[0].points.len(),
        endpoint_point((50_000, 50_000), 0.2, BASE_WIDTH_MM, 1.0),
    );
    expect_rejected(
        "interior chord connector",
        check_connect_output(&fx, &chord, MergeExpectation::Free),
        "junction",
    );

    // Each individual incompatibility, SUT-separated and fabricated-merge rejected.
    let c = WIDTH_LIMIT_MM;
    let mut mismatches = Vec::new();
    let mut role = base_meta();
    role.role = ExtrusionRole::TopSolidInfill;
    mismatches.push(("role", base_meta(), role));
    let mut speed = base_meta();
    speed.speed = f32::from_bits(1.0_f32.to_bits() + 1);
    mismatches.push(("speed", base_meta(), speed));
    let mut tool = base_meta();
    tool.tool = Some(1);
    mismatches.push(("tool", base_meta(), tool));
    let mut tool_none = base_meta();
    tool_none.tool = None;
    mismatches.push(("tool none", base_meta(), tool_none));
    let mut first = base_meta();
    first.first_width = bump_width(BASE_WIDTH_MM, WIDTH_ULPS_INCOMPATIBLE);
    mismatches.push(("first width 34 ulps", base_meta(), first));
    let mut last = base_meta();
    last.last_width = bump_width(BASE_WIDTH_MM, WIDTH_ULPS_INCOMPATIBLE);
    mismatches.push(("last width 34 ulps", base_meta(), last));
    let mut exact_low = base_meta();
    exact_low.first_width = c;
    let mut above = base_meta();
    above.first_width = f32::from_bits((2.0 * c).to_bits() + 1);
    mismatches.push(("first width just above 1e-6", exact_low.clone(), above));
    for (label, a, b) in mismatches {
        let fx = two_path_fixture(a, b);
        assert!(
            !independent_compatible(&fx.inputs[0], &fx.inputs[1]),
            "{label} predicate"
        );
        let output = connect_infill(fx.inputs.clone(), &fx.graph(), fx.anchor());
        check_connect_output(&fx, &output, MergeExpectation::MustNotMerge)
            .unwrap_or_else(|error| panic!("{label} real separation: {error}"));
        expect_rejected(
            label,
            check_connect_output(&fx, &fabricated_merge(&fx), MergeExpectation::Free),
            "incompatible",
        );
    }
    // Width boundary exactly at 1e-6 (c versus 2c) and 33 ulps are compatible.
    let mut exact_high = base_meta();
    exact_high.first_width = 2.0 * c;
    assert_eq!(2.0 * c - c, c);
    let mut boundary = base_meta();
    boundary.first_width = bump_width(BASE_WIDTH_MM, WIDTH_ULPS_COMPATIBLE);
    boundary.last_width = bump_width(BASE_WIDTH_MM, WIDTH_ULPS_COMPATIBLE);
    for (label, a, b) in [
        ("width exactly 1e-6", exact_low, exact_high),
        ("width 33 ulps", base_meta(), boundary),
    ] {
        let fx = two_path_fixture(a, b);
        assert!(
            independent_compatible(&fx.inputs[0], &fx.inputs[1]),
            "{label} predicate"
        );
        let output = connect_infill(fx.inputs.clone(), &fx.graph(), fx.anchor());
        check_connect_output(&fx, &output, MergeExpectation::MustMerge(20_000))
            .unwrap_or_else(|error| panic!("{label} real merge: {error}"));
    }

    // Strict-anchor band. The pair's exact shortest arc is 20 000 units.
    let mut at = two_path_fixture(base_meta(), base_meta());
    // More than one unit above the arc: the join must not happen, and a
    // fabricated join is rejected by the same checker.
    at.anchor_max_units = 20_000 - 2;
    let output = connect_infill(at.inputs.clone(), &at.graph(), at.anchor());
    check_connect_output(&at, &output, MergeExpectation::MustNotMerge)
        .expect("anchor two units below the arc keeps the pair separate");
    expect_rejected(
        "merge two units above the anchor",
        check_connect_output(&at, &fabricated_merge(&at), MergeExpectation::Free),
        "more than one unit above",
    );
    // Inside the band (arc == anchor): both branches are legal, and the
    // fabricated opposite branch is accepted by the same banded expectation.
    at.anchor_max_units = 20_000;
    assert_eq!(
        anchor_side(20_000, at.anchor_max_units),
        AnchorSide::WithinBand
    );
    let output = connect_infill(at.inputs.clone(), &at.graph(), at.anchor());
    check_connect_output(&at, &output, MergeExpectation::EitherBranch)
        .expect("the band accepts the branch production took");
    let alternative = if output.len() == 1 {
        at.inputs.clone()
    } else {
        fabricated_merge(&at)
    };
    check_connect_output(&at, &alternative, MergeExpectation::EitherBranch)
        .expect("the band accepts the other branch as well");
    // More than one unit below the arc: the join is required.
    at.anchor_max_units = 20_000 + 2;
    assert_eq!(anchor_side(20_000, at.anchor_max_units), AnchorSide::Below);
    let output = connect_infill(at.inputs.clone(), &at.graph(), at.anchor());
    check_connect_output(&at, &output, MergeExpectation::MustMerge(20_000))
        .expect("anchor two units above the arc must merge");
    expect_rejected(
        "separation two units below the anchor",
        check_connect_output(&at, &at.inputs, MergeExpectation::MustMerge(20_000)),
        "must merge",
    );

    // Retained extended-tier regression (seed 12768350826874075087): the exact
    // arc is 3110 units and the anchor is exactly 3110 units, so the pre-band
    // oracle demanded the stub branch. Production's `directed_distance` reads
    // `project((13518, -1)) = 13519.000000000002`, i.e. 3109.999999999998, so
    // the strict `<` joined. Both branches are legal inside the 1-unit band;
    // the join itself is still reported and still fails the strict separation
    // expectation.
    let retained = retained_stub_case();
    let fixture = &retained.fixture;
    assert_eq!(fixture.anchor_max_units, 3_110);
    assert_eq!(
        anchor_side(3_110, fixture.anchor_max_units),
        AnchorSide::WithinBand
    );
    assert_eq!(anchor_side(3_112, 3_110), AnchorSide::Above);
    let output = connect_infill(fixture.inputs.clone(), &fixture.graph(), fixture.anchor());
    check_connect_output(fixture, &output, MergeExpectation::EitherBranch)
        .expect("retained band case passes the banded checker");
    assert_eq!(
        output.len(),
        1,
        "the retained case must still be a real join, not a silent pass-through"
    );
    expect_rejected(
        "retained join under the strict separation expectation",
        check_connect_output(fixture, &output, MergeExpectation::MustNotMerge),
        "must stay separate",
    );

    // Cross-ring connector on a holed square: real separation passes, a
    // fabricated outer-to-hole merge is rejected as cross-ring.
    let model = shape_model(&holed_square_spec()).expect("holed model");
    let outer = &model.rings[0];
    let hole = &model.rings[1];
    let endpoint = |ring: usize, model_ring: &RingModel, local: i64| Endpoint {
        ring,
        local,
        units: model_ring.at(local),
    };
    let (o0, o1, h0, h1) = (
        endpoint(0, outer, 10_000),
        endpoint(0, outer, 20_000),
        endpoint(1, hole, 10_000),
        endpoint(1, hole, 20_000),
    );
    let cross = ConnectFixture {
        inputs: vec![
            path_from(o0.units, &[(20_000, 20_000)], o1.units, &base_meta()),
            path_from(h0.units, &[(20_000, 25_000)], h1.units, &base_meta()),
        ],
        endpoints: vec![Some([o0, o1]), Some([h0, h1])],
        model,
        anchor_max_units: 1_000_000,
        anchor_len_units: 0,
        max_abs: 200_000,
    };
    let graph = cross.graph();
    check_projection(&cross, &graph).expect("outer/hole endpoints project to their own rings");
    let output = connect_infill(cross.inputs.clone(), &graph, cross.anchor());
    check_connect_output(&cross, &output, MergeExpectation::MustNotMerge)
        .expect("outer-to-hole stays separate");
    expect_rejected(
        "outer-to-hole merge",
        check_connect_output(&cross, &fabricated_merge(&cross), MergeExpectation::Free),
        "cross-ring",
    );

    // Disconnected components: same rejection, different components. Two far
    // apart squares put the two paths on different rings, so a fabricated merge
    // must be rejected as cross-ring rather than stitched across the gap.
    let two_comp = shape_model(&[
        ExSpec {
            contour: rect_ccw(0, 0, 60_000, 60_000),
            holes: Vec::new(),
        },
        ExSpec {
            contour: rect_ccw(100_000, 0, 160_000, 60_000),
            holes: Vec::new(),
        },
    ])
    .expect("two-component model");
    let first = &two_comp.rings[0];
    let second = &two_comp.rings[1];
    let (c0, c1, d0, d1) = (
        endpoint(0, first, 10_000),
        endpoint(0, first, 20_000),
        endpoint(1, second, 10_000),
        endpoint(1, second, 20_000),
    );
    let disconnected = ConnectFixture {
        inputs: vec![
            path_from(c0.units, &[(20_000, 20_000)], c1.units, &base_meta()),
            path_from(d0.units, &[(140_000, 20_000)], d1.units, &base_meta()),
        ],
        endpoints: vec![Some([c0, c1]), Some([d0, d1])],
        model: two_comp,
        anchor_max_units: 1_000_000,
        anchor_len_units: 0,
        max_abs: 200_000,
    };
    let disconnected_graph = disconnected.graph();
    check_projection(&disconnected, &disconnected_graph)
        .expect("disconnected endpoints project to their own components");
    let output = connect_infill(
        disconnected.inputs.clone(),
        &disconnected_graph,
        disconnected.anchor(),
    );
    check_connect_output(&disconnected, &output, MergeExpectation::MustNotMerge)
        .expect("disconnected components stay separate");
    expect_rejected(
        "disconnected component merge",
        check_connect_output(
            &disconnected,
            &fabricated_merge(&disconnected),
            MergeExpectation::Free,
        ),
        "cross-ring",
    );

    // dont_connect preserves its input (a one-point path included); the
    // connecting branch discards that path.
    let mut short = two_path_fixture(base_meta(), base_meta());
    short.inputs.insert(
        1,
        make_path(
            vec![endpoint_point((50_000, 50_000), 0.2, 0.4, 1.0)],
            &base_meta(),
        ),
    );
    short.endpoints.insert(1, None);
    short.anchor_max_units = 400;
    let output = connect_infill(short.inputs.clone(), &short.graph(), short.anchor());
    assert!(short.anchor().dont_connect());
    let summary = check_connect_output(&short, &output, MergeExpectation::Free)
        .expect("dont_connect identity");
    assert!(summary.identity && output.len() == 3);
    let mut altered = output.clone();
    altered.remove(1);
    expect_rejected(
        "dont_connect dropping the short path",
        check_connect_output(&short, &altered, MergeExpectation::Free),
        "unchanged",
    );
    short.anchor_max_units = 50_000;
    let output = connect_infill(short.inputs.clone(), &short.graph(), short.anchor());
    check_connect_output(&short, &output, MergeExpectation::MustMerge(20_000))
        .expect("short path discarded");
    assert!(output.iter().all(|path| path.points.len() >= 2));

    // Interior endpoints are not unanchorable: two scan lines inset 1 mm from
    // the square boundary are still joined by the live nearest-ring anchoring.
    let interior = vec![
        path_from((10_000, 20_000), &[], (90_000, 20_000), &base_meta()),
        path_from((10_000, 40_000), &[], (90_000, 40_000), &base_meta()),
    ];
    let graph = BoundaryInfillGraph::new(&[ExPolygon {
        contour: polygon_of(&rect_ccw(0, 0, 100_000, 100_000)),
        holes: Vec::new(),
    }]);
    let joined = connect_infill(
        interior,
        &graph,
        AnchorParams {
            anchor_length_mm: 0.0,
            anchor_length_max_mm: 10.0,
        },
    );
    assert_eq!(
        joined.len(),
        1,
        "interior endpoints were treated as unanchorable"
    );
}

fn linker_chain_controls() {
    let limits = checked_chain_limits(&chain_probe_bounds()).expect("chain limits");
    let counters = ChainCounters::default();
    chain_witnesses(&limits, &counters);
    counters.assert_nonvacuous();

    // Chain-only reordering adds no connector geometry.
    let (fixture, output) = chain_only_witness();
    let (reversed_count, reordered) =
        check_chain_only(&fixture.inputs, &output).expect("chain-only");
    assert!(
        reversed_count > 0 && reordered,
        "witness must reorder and reverse"
    );
    let mut with_connector = output.clone();
    let joint = with_connector[0].points.last().copied().expect("point");
    with_connector[0].points.push(with_xy(&joint, 9.9, 9.9));
    expect_rejected(
        "chain-only connector",
        check_chain_only(&fixture.inputs, &with_connector),
        "gained, lost or changed",
    );
    let mut merged = output.clone();
    let tail = merged.remove(1);
    merged[0].points.extend(tail.points);
    expect_rejected(
        "chain-only merge",
        check_chain_only(&fixture.inputs, &merged),
        "chain-only returned",
    );

    // Wrong exact chain and wrong majority owner, through the same checker.
    let unequal = build_orch_case(
        &witness_orch(
            PairClass::Union,
            LockMode::Unlocked,
            BaseRole::Sparse,
            2,
            1,
            false,
            vec![(LookupMode::Reversed, 1)],
        ),
        &limits,
    )
    .expect("unequal case");
    let output = run_orchestrate(&unequal).expect("unequal orchestrate");
    let sparse = output.sparse_paths().to_vec();
    let origins = output.sparse_path_origins().to_vec();
    let summary = check_orchestrate(
        &unequal,
        (&sparse, &origins),
        (output.solid_paths(), output.solid_path_origins()),
    )
    .expect("real unequal output");
    assert!(summary.majority_unequal > 0 && summary.passthrough_regions > 0);
    let merged_index = sparse
        .iter()
        .position(|path| path.points.len() > 3)
        .expect("a merged cross-region path");
    let mut wrong_chain = origins.clone();
    if let Some(origin) = wrong_chain[merged_index].as_mut() {
        origin.variant_chain.reverse();
    }
    expect_rejected(
        "reversed-chain origin",
        check_orchestrate(
            &unequal,
            (&sparse, &wrong_chain),
            (output.solid_paths(), output.solid_path_origins()),
        ),
        "full ordered identity",
    );
    let mut empty_chain = origins.clone();
    if let Some(origin) = empty_chain[merged_index].as_mut() {
        origin.variant_chain.clear();
    }
    expect_rejected(
        "empty-chain wildcard origin",
        check_orchestrate(
            &unequal,
            (&sparse, &empty_chain),
            (output.solid_paths(), output.solid_path_origins()),
        ),
        "full ordered identity",
    );
    // Moving the merged path to the minority owner B (region 12, lower id):
    // its slot must stay in prior order, so rebuild B's bucket with it.
    let b_origin = Some(RegionOrigin {
        object_id: "obj-a".to_owned(),
        region_id: 12,
        variant_chain: legal_chain(),
    });
    let mut wrong_owner_paths = Vec::new();
    let mut wrong_owner_origins = Vec::new();
    for (index, (path, origin)) in sparse.iter().zip(&origins).enumerate() {
        if index != merged_index {
            wrong_owner_paths.push(path.clone());
            wrong_owner_origins.push(origin.clone());
        }
    }
    let b_position = wrong_owner_origins
        .iter()
        .position(|origin| origin.as_ref().map(|origin| origin.region_id) != Some(13))
        .unwrap_or(wrong_owner_origins.len());
    wrong_owner_paths.insert(b_position, sparse[merged_index].clone());
    wrong_owner_origins.insert(b_position, b_origin);
    expect_rejected(
        "minority owner",
        check_orchestrate(
            &unequal,
            (&wrong_owner_paths, &wrong_owner_origins),
            (output.solid_paths(), output.solid_path_origins()),
        ),
        "majority owner",
    );
    // A reversed-chain lookup must pass through: a linked (clipped) version fails.
    let reversed_index = sparse
        .iter()
        .zip(&origins)
        .position(|(_, origin)| origin.as_ref().map(|origin| origin.region_id) == Some(21))
        .expect("pass-through output");
    let mut linked_passthrough = sparse.clone();
    linked_passthrough[reversed_index].points[0].x += 0.1;
    expect_rejected(
        "reversed lookup treated as matched",
        check_orchestrate(
            &unequal,
            (&linked_passthrough, &origins),
            (output.solid_paths(), output.solid_path_origins()),
        ),
        "passed through unchanged",
    );
}

#[test]
fn linker_controls() {
    linker_graph_controls();
    linker_connect_controls();
    linker_chain_controls();
}

// ─────────────────────────────────────────────────────────────────────────────
// Bound matrix and shrink probes.
// ─────────────────────────────────────────────────────────────────────────────

fn assert_bound_matrix<T: Debug>(
    label: &str,
    valid: BTreeMap<String, u64>,
    keys: &[&str],
    build: fn(&BTreeMap<String, u64>) -> Result<T, DomainError>,
    i64_keys: &[&str],
    semantic: &[(&str, u64)],
) {
    build(&valid).unwrap_or_else(|error| panic!("{label} valid bounds: {error}"));
    for key in keys {
        let mut bounds = valid.clone();
        bounds.remove(*key);
        assert!(
            matches!(build(&bounds), Err(DomainError::InvalidBounds(_))),
            "{label}: missing `{key}` accepted"
        );
        let mut bounds = valid.clone();
        bounds.insert((*key).to_owned(), 0);
        assert!(
            matches!(build(&bounds), Err(DomainError::InvalidBounds(_))),
            "{label}: zero `{key}` accepted"
        );
        let mut bounds = valid.clone();
        let overflow = if i64_keys.contains(key) {
            i64::MAX as u64 + 1
        } else {
            u64::from(u32::MAX) + 1
        };
        bounds.insert((*key).to_owned(), overflow);
        assert!(
            matches!(build(&bounds), Err(DomainError::ArithmeticOverflow)),
            "{label}: overflowing `{key}` accepted"
        );
    }
    for (key, value) in semantic {
        let mut bounds = valid.clone();
        bounds.insert((*key).to_owned(), *value);
        assert!(
            matches!(build(&bounds), Err(DomainError::InvalidBounds(_))),
            "{label}: invalid minimum `{key}={value}` accepted"
        );
    }
}

#[derive(Debug, Default)]
struct ProbeStats {
    simplify_true: u32,
    simplify_false: u32,
    complicate_true: u32,
    complicate_false: u32,
    /// Complicate calls made by the bounded *repeated* loop that reported false.
    repeated_complicate_false: u32,
    /// A false simplify left the current value bit-identical.
    false_simplify_stable: bool,
    reduced: bool,
}

fn probe_strategy<S, V>(
    runner: &mut TestRunner,
    strategy: &S,
    label: &str,
    steps: u32,
    validate: &dyn Fn(&V) -> Result<(), String>,
    complexity: &dyn Fn(&V) -> u128,
) -> ProbeStats
where
    S: Strategy<Value = V>,
    V: Debug,
{
    for _ in 0..PROBE_TREES {
        let mut tree = strategy.new_tree(runner).expect("probe tree");
        let initial = tree.current();
        validate(&initial).unwrap_or_else(|error| panic!("{label} initial: {error}"));
        let start = complexity(&initial);
        let mut stats = ProbeStats::default();
        for step in 0..steps {
            let before_simplify = format!("{:?}", tree.current());
            if !tree.simplify() {
                stats.simplify_false += 1;
                // A false simplify must not move the value.
                stats.false_simplify_stable = format!("{:?}", tree.current()) == before_simplify;
                break;
            }
            stats.simplify_true += 1;
            let current = tree.current();
            validate(&current).unwrap_or_else(|error| panic!("{label} simplified: {error}"));
            stats.reduced |= complexity(&current) < start;
            if step % 3 == 0 {
                // Repeated complicate until it reports false (bounded).
                for _ in 0..64 {
                    let complicated = tree.complicate();
                    validate(&tree.current())
                        .unwrap_or_else(|error| panic!("{label} complicated: {error}"));
                    if complicated {
                        stats.complicate_true += 1;
                    } else {
                        stats.complicate_false += 1;
                        stats.repeated_complicate_false += 1;
                        break;
                    }
                }
            }
        }
        // One more complicate after the loop; this alone must not be what proves
        // the repeated loop reached false.
        if tree.complicate() {
            stats.complicate_true += 1;
        } else {
            stats.complicate_false += 1;
        }
        validate(&tree.current()).unwrap_or_else(|error| panic!("{label} final: {error}"));
        if stats.reduced {
            assert!(stats.simplify_true > 0, "{label}: false-only probe");
            assert!(
                stats.repeated_complicate_false > 0,
                "{label}: the repeated complicate loop never reached false"
            );
            assert!(
                stats.false_simplify_stable,
                "{label}: a false simplify changed the probed value"
            );
            assert!(
                stats.simplify_false > 0,
                "{label}: simplify never exhausted within {steps} steps"
            );
            println!("{label}: {stats:?}");
            return stats;
        }
    }
    panic!("{label}: no probe tree produced a real reduction");
}

fn assert_no_shrink<S: Strategy>(runner: &mut TestRunner, strategy: &S, label: &str)
where
    S::Value: Debug,
{
    let mut tree = strategy.new_tree(runner).expect("no-shrink tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "{label} simplified");
    assert!(!tree.complicate(), "{label} complicated");
    assert_eq!(format!("{:?}", tree.current()), before, "{label} changed");
}

fn validate_direct(raw: &DirectRaw, top: ChainTop, limits: &ChainLimits) -> Result<(), String> {
    if raw.top != top {
        return Err("chain side changed".to_owned());
    }
    let fx = build_direct_fixture(raw, &limits.connect)?;
    if fx.anchor().dont_connect() != (top == ChainTop::ChainOnly) {
        return Err("dont_connect side changed".to_owned());
    }
    if fx
        .inputs
        .iter()
        .any(|path| path.points.len() < 2 || path.order_lock.is_some())
    {
        return Err("direct inputs must be unlocked and at least two points".to_owned());
    }
    Ok(())
}

fn validate_orch(
    raw: &OrchRaw,
    pair: PairClass,
    lock: LockMode,
    limits: &ChainLimits,
) -> Result<(), String> {
    if raw.pair != pair || raw.lock != lock {
        return Err("pair class or lock mode changed".to_owned());
    }
    let case = build_orch_case(raw, limits)?;
    for region in &case.regions[2..] {
        let lookup = region.lookup.ok_or("extra without lookup mode")?;
        let matched = region.matched();
        if matched != (lookup == LookupMode::Exact)
            || (lookup == LookupMode::Missing) != region.view_chain.is_none()
        {
            return Err(format!("{lookup:?} lookup structure lost"));
        }
    }
    if case.regions[0].locked != (lock == LockMode::Locked) {
        return Err("locked witness lost".to_owned());
    }
    Ok(())
}

#[test]
fn linker_shrink_bounds() {
    assert_bound_matrix(
        "graph",
        graph_probe_bounds(),
        &GRAPH_BOUND_KEYS,
        checked_graph_limits,
        &["max_abs_units"],
        &[
            ("max_abs_units", 29_999),
            ("max_abs_units", MAX_GRID_ABS_UNITS as u64 + 1),
            ("max_vertices", 7),
        ],
    );
    assert_bound_matrix(
        "connect",
        connect_probe_bounds(),
        &CONNECT_BOUND_KEYS,
        checked_connect_limits,
        &["max_abs_units", "max_anchor_units"],
        &[
            ("max_paths", 2),
            ("max_points", 1),
            ("max_anchor_units", MAX_GRID_ABS_UNITS as u64 + 1),
            ("max_abs_units", 29_999),
        ],
    );
    assert_bound_matrix(
        "chain",
        chain_probe_bounds(),
        &CHAIN_BOUND_KEYS,
        checked_chain_limits,
        &["max_abs_units", "max_anchor_units"],
        &[
            ("max_paths", 1),
            ("max_points", 1),
            ("max_regions", 1),
            ("max_regions", 17),
            ("max_abs_units", 100_000),
        ],
    );

    let mut runner = TestRunner::deterministic();
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(RING_CLASSES.to_vec()).no_shrink(),
        "ring class",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(CONNECT_CLASSES.to_vec()).no_shrink(),
        "connect class",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(PAIR_CLASSES.to_vec()).no_shrink(),
        "pair class",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(LOOKUP_MODES.to_vec()).no_shrink(),
        "lookup mode",
    );

    let graph = checked_graph_limits(&graph_probe_bounds()).expect("graph limits");
    for class in RING_CLASSES {
        for arc in [ArcMode::Free, ArcMode::Tie] {
            let strategy = graph_strategy(&graph, &[class], &[arc]);
            let label = format!("graph {class:?}/{arc:?}");
            if class == RingClass::Empty {
                assert_no_shrink(&mut runner, &strategy, &label);
                let raw = strategy
                    .new_tree(&mut runner)
                    .expect("empty tree")
                    .current();
                let case = build_graph_case(&raw, graph.max_abs).expect("empty case");
                run_graph_case(&case, &GraphCounters::default()).expect("parameterless empty mode");
                continue;
            }
            let validate = move |raw: &GraphRaw| -> Result<(), String> {
                if raw.class != class || raw.arc != arc {
                    return Err("ring class or arc mode changed".to_owned());
                }
                let case = build_graph_case(raw, graph.max_abs)?;
                if arc == ArcMode::Tie
                    && !case
                        .queries
                        .iter()
                        .all(|query| case.model.rings[query.ring].is_tie(query.from, query.to))
                {
                    return Err("tie witness lost".to_owned());
                }
                Ok(())
            };
            let stats = probe_strategy(
                &mut runner,
                &strategy,
                &label,
                graph.max_shrink_candidates,
                &validate,
                &|raw: &GraphRaw| raw.complexity(),
            );
            // The final probed value still passes the SUT checker.
            let raw = strategy.new_tree(&mut runner).expect("tree").current();
            run_graph_case(
                &build_graph_case(&raw, graph.max_abs).expect("case"),
                &GraphCounters::default(),
            )
            .unwrap_or_else(|error| panic!("{label} SUT: {error}"));
            assert!(stats.reduced);
        }
    }

    let connect = checked_connect_limits(&connect_probe_bounds()).expect("connect limits");
    for class in CONNECT_CLASSES {
        let strategy = connect_strategy(&connect, &[class]);
        let label = format!("connect {class:?}");
        let validate = move |raw: &ConnectRaw| -> Result<(), String> {
            if raw.class != class {
                return Err("connect class changed".to_owned());
            }
            let case = build_connect_case(raw, &connect)?;
            let want = match class {
                ConnectClass::Merge
                | ConnectClass::WidthAtBoundary
                | ConnectClass::ShortDiscarded => {
                    matches!(case.expectation, MergeExpectation::MustMerge(_))
                }
                ConnectClass::DontConnect => case.fixture.anchor().dont_connect(),
                ConnectClass::Multi => case.fixture.inputs.len() >= 3,
                ConnectClass::Stub | ConnectClass::AnchorAtDistance => {
                    matches!(case.expectation, MergeExpectation::EitherBranch)
                }
                _ => case.expectation == MergeExpectation::MustNotMerge,
            };
            if !want {
                return Err(format!("{class:?} mode witness lost"));
            }
            Ok(())
        };
        probe_strategy(
            &mut runner,
            &strategy,
            &label,
            4_096,
            &validate,
            &|raw: &ConnectRaw| raw.complexity(),
        );
        let raw = strategy.new_tree(&mut runner).expect("tree").current();
        run_connect_case(
            &build_connect_case(&raw, &connect).expect("case"),
            &ConnectCounters::default(),
        )
        .unwrap_or_else(|error| panic!("{label} SUT: {error}"));
    }

    let chain = checked_chain_limits(&chain_probe_bounds()).expect("chain limits");
    for top in [ChainTop::ChainOnly, ChainTop::ChainConnect] {
        let strategy = chain_strategy(&chain, &[top], &PAIR_CLASSES, &[LockMode::Unlocked]);
        let label = format!("chain {top:?}");
        let validate = move |raw: &ChainRaw| -> Result<(), String> {
            match raw {
                ChainRaw::Direct(direct) => validate_direct(direct, top, &chain),
                ChainRaw::Orchestrate(_) => Err("direct mode switched to orchestrate".to_owned()),
            }
        };
        probe_strategy(
            &mut runner,
            &strategy,
            &label,
            4_096,
            &validate,
            &|raw: &ChainRaw| raw.complexity(),
        );
        let raw = strategy.new_tree(&mut runner).expect("tree").current();
        run_chain_raw(&raw, &chain, &ChainCounters::default())
            .unwrap_or_else(|error| panic!("{label} SUT: {error}"));
    }
    for pair in PAIR_CLASSES {
        for lock in [LockMode::Unlocked, LockMode::Locked] {
            let strategy = chain_strategy(&chain, &[ChainTop::Orchestrate], &[pair], &[lock]);
            let label = format!("orchestrate {pair:?}/{lock:?}");
            let validate = move |raw: &ChainRaw| -> Result<(), String> {
                match raw {
                    ChainRaw::Orchestrate(orch) => validate_orch(orch, pair, lock, &chain),
                    ChainRaw::Direct(_) => Err("orchestrate mode switched to direct".to_owned()),
                }
            };
            probe_strategy(
                &mut runner,
                &strategy,
                &label,
                4_096,
                &validate,
                &|raw: &ChainRaw| raw.complexity(),
            );
            let raw = strategy.new_tree(&mut runner).expect("tree").current();
            run_chain_raw(&raw, &chain, &ChainCounters::default())
                .unwrap_or_else(|error| panic!("{label} SUT: {error}"));
        }
    }
}
