//! Charter `H-BOOL-SET`: the four production Boolean wrappers
//! (`slicer_core::polygon_ops::{union, intersection, difference, xor}`)
//! against independent set-algebra references.
//!
//! Two reference families, both independent of Clipper:
//!
//! * **Rectangles (complete, exact).** Pairs of axis-aligned rectangles with
//!   whole-millimetre edges. The plane is partitioned at every input X/Y edge;
//!   every open cell is compared with the Boolean membership truth table at its
//!   midpoint (whole-mm edges make every midpoint an integer grid point), every
//!   output vertex must lie on the partition grid with axis-aligned edges, and
//!   the output double area must equal the checked `i128` sum of the expected
//!   cells. Together these determine the output region exactly; no tolerance.
//! * **General valid sets (bounded sample).** #2 `geometry_cases` for
//!   `Empty`, `Convex`, `Concave`, `Holed` and `Disconnected`, scaled by
//!   [`SCALE`] so every ring encloses lattice points, then placed by checked
//!   translations. A deterministic lattice of integer samples, filtered to
//!   points at Euclidean distance at least [`MARGIN`] from **every** input
//!   edge, is classified with #2 `point_in_region` on both inputs and on each
//!   output; per-ring witnesses guarantee each component and hole is observed.
//!   A witness only needs [`WITNESS_MARGIN`] clearance, because a scaled
//!   catalogue feature can be too small to contain any point at [`MARGIN`] (the
//!   `1 x 1` triangle at [`SCALE`] has inradius `approx 1.72` units), which
//!   would make the witness requirement unsatisfiable rather than unlucky.
//!   Clipper rounds new intersection vertices by at most `sqrt(1/2) < 1` unit
//!   (see `envelopes` and the proof on [`WITNESS_MARGIN`]), so the expected
//!   membership is still exact at either margin and a `Boundary` output
//!   classification is a failure. When both operands are
//!   rectilinear, all intersections are exact integers and the area identities
//!   `|A∪B|+|A∩B| = |A|+|B|`, `|A\B| = |A|-|A∩B|`, `|A⊕B| = |A∪B|-|A∩B|` are
//!   checked exactly.
//!
//! Same-set identities (`G∪G = G∩G = G`, `G\G = G⊕G = ∅`) and a translated
//! copy pair are checked for every case. Output rings are validated with #2
//! `validate_ring` and `signed_area2` winding; strict hole/component structure
//! (#2 `validate_geometry` with the operand's class) is demanded only for the
//! identities, where the expected result *is* the operand.
//!
//! Limitation: the general family proves membership only at the sampled
//! points; unsampled regions of non-rectilinear outputs are not certified.

use std::cmp::Ordering;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::polygon_ops::{difference, intersection, union, xor};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::domains::{
    geometry_cases, BoundaryRule, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    orientation, point_in_region, segment_relation, signed_area2, validate_geometry, validate_ring,
    PointLocation, SegmentRelation,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{validate_geometry_limits, BoundsError, GeometryLimits, RawGeometryBounds};
use super::envelopes::{point_clears_segment, region_double_area, UNITS_PER_MM};
use super::rect::{
    cell_partition, cells_double_area, edges, is_rectilinear, rings, scale_polygons, sorted_unique,
    translate_polygons, BoolOp, Rect,
};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-BOOL-SET";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_boolean_set_algebra";

/// Scale applied to #2 catalog geometry. Catalog vertices are integers and its
/// smallest features (unit-square holes, unit gaps) become `SCALE`-unit
/// squares, so quarter-gap lattice points sit `SCALE / 4 = MARGIN` units from
/// every axis-aligned edge.
pub const SCALE: i64 = 8;
/// Minimum Euclidean clearance, in units, of every general-family sample from
/// every input edge. Exceeds the `sqrt(1/2)`-unit Clipper rounding envelope.
pub const MARGIN: i64 = 2;
/// Minimum Euclidean clearance, in units, of a **witness** sample from every
/// input edge (the general population keeps [`MARGIN`]).
///
/// A witness exists to observe a ring at all, and a scaled catalogue feature can
/// be too small to contain any point at [`MARGIN`]: the `1 x 1` catalogue
/// triangle at [`SCALE`] `= 8` has inradius `8 * (2 - sqrt(3)) / 2 approx 1.72`
/// units, so **no** lattice point in it can sit 2 units from all three edges.
/// Demanding [`MARGIN`] of a witness therefore made the requirement
/// unsatisfiable, not unlucky, and failed with "no lattice witness strictly
/// inside ring".
///
/// One unit is enough, and the proof is the same rounding envelope:
///
/// * every output vertex is either an exact input vertex or an exact input-edge
///   intersection point rounded to the lattice, so it moves by at most
///   `sqrt(1/2)` units (`ROUNDED_POINT_MAX_EUCLIDEAN_SQ_NUMERATOR` /
///   `..._DENOMINATOR`, i.e. `1/2` squared length);
/// * distance to a segment is a convex function of the segment's endpoints, so
///   every output edge stays within `sqrt(1/2) < 0.71` units of the exact input
///   boundary;
/// * hence an integer point at distance `>= 1` from the exact input boundary is
///   at distance `>= 1 - sqrt(1/2) > 0.29` units from the rounded output
///   boundary, and rounding cannot flip its inside/outside classification (nor
///   make that classification `Boundary`).
///
/// The requirement stays a hard error when even a `1`-unit lattice point does
/// not exist: an unobservable ring is a construction failure, never a silent
/// skip.
pub const WITNESS_MARGIN: i64 = 1;
/// Relative placement range of operand B and of the translated copy, in
/// catalog units (`SCALE * REL_CATALOG` canonical units).
pub const REL_CATALOG: i64 = 48;
/// Smallest accepted `max_abs_units`: rectangles need at least 2 whole mm of
/// half-span, which also leaves the catalog its required room.
pub const MIN_ABS_UNITS: u64 = 20_000;
/// Rectangle cells: four distinct edges per axis give at most `3 x 3` cells.
pub const RECT_CELLS: u64 = 9;

/// The five topology classes this charter names.
pub const CLASSES: [TopologyClass; 5] = [
    TopologyClass::Empty,
    TopologyClass::Convex,
    TopologyClass::Concave,
    TopologyClass::Holed,
    TopologyClass::Disconnected,
];

// ─────────────────────────────────────────────────────────────────────────────
// Profile validation.
// ─────────────────────────────────────────────────────────────────────────────

/// Queries needed so every ring of both operands and one exterior point fit,
/// and the full rectangle partition fits. Only reachable with overflowing
/// count bounds, which [`validate_limits`] rejects first, does this saturate.
pub fn min_queries(raw: RawGeometryBounds) -> u64 {
    raw.max_components
        .checked_add(raw.max_holes)
        .and_then(|rings| rings.checked_mul(2))
        .and_then(|rings| rings.checked_add(1))
        .map_or(u64::MAX, |witnesses| witnesses.max(RECT_CELLS))
}

/// Validate the `H-BOOL-SET` profile before any strategy exists.
pub fn validate_limits(raw: RawGeometryBounds) -> Result<GeometryLimits, BoundsError> {
    validate_geometry_limits(
        raw,
        MIN_ABS_UNITS,
        // Holed needs 4 + 4, Disconnected 2 * 4 vertices.
        8,
        1,
        2,
        min_queries(raw),
        "per-ring sample witness",
    )
}

/// A valid profile used by the controls (not a measured registry profile).
pub fn reference_raw_bounds() -> RawGeometryBounds {
    RawGeometryBounds {
        max_abs_units: 200_000,
        max_vertices: 16,
        max_holes: 2,
        max_components: 3,
        max_shrink_candidates: 64,
        max_queries: 160,
    }
}

/// Catalog coordinate bound: `max_abs_units / (4 * SCALE)`, so the scaled
/// normalised catalog spans at most half of `max_abs_units`.
fn catalog_units(limits: &GeometryLimits) -> i64 {
    limits.max_abs_units / (4 * SCALE)
}

/// Placement base range: every realised coordinate stays within
/// `max_abs_units` (proof in [`realize`]).
fn base_range(limits: &GeometryLimits) -> Result<(i64, i64), String> {
    let half = limits.max_abs_units / 2;
    let rel = SCALE * REL_CATALOG;
    let low = -limits.max_abs_units + rel;
    let high = half - rel;
    if low > high {
        return Err(format!(
            "max_abs_units {} leaves no placement room",
            limits.max_abs_units
        ));
    }
    Ok((low, high))
}

fn domain(class: TopologyClass, bounds: GeometryBounds) -> GeometryDomain {
    // exhaustive: every domain field is a deliberate charter choice, never a helper default
    GeometryDomain {
        class,
        min_components: usize::from(class != TopologyClass::Empty),
        min_holes: 0,
        boundary: BoundaryRule::Strict,
        bounds,
    }
}

fn geometry_bounds(limits: &GeometryLimits, max_abs_units: i64) -> GeometryBounds {
    // exhaustive: live profiles must supply every measured bound, never the helper default
    GeometryBounds {
        max_abs_units,
        max_vertices: limits.max_vertices,
        max_holes: limits.max_holes,
        max_components: limits.max_components,
        max_shrink_candidates: limits.max_shrink_candidates,
    }
}

/// The validation domain for a realised (scaled, placed) operand.
pub fn realised_domain(class: TopologyClass, limits: &GeometryLimits) -> GeometryDomain {
    domain(class, geometry_bounds(limits, limits.max_abs_units))
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy.
// ─────────────────────────────────────────────────────────────────────────────

/// A catalog case tagged with the class whose domain produced it.
#[derive(Debug, Clone)]
pub struct Tagged {
    /// Producing class.
    pub class: TopologyClass,
    /// Raw #2 catalog case (catalog coordinates).
    pub case: GeometryCase,
}

/// One generated `H-BOOL-SET` input.
#[derive(Debug, Clone)]
pub struct BoolInput {
    /// Whole-mm rectangle pair (overlapping, disjoint, touching or nested).
    pub rects: (Rect, Rect),
    /// Operand A.
    pub a: Tagged,
    /// Operand B.
    pub b: Tagged,
    /// Placement of B relative to A, catalog units.
    pub rel_b: (i64, i64),
    /// Translation of the copy of A, catalog units.
    pub copy: (i64, i64),
    /// Common absolute placement, canonical units.
    pub base: (i64, i64),
}

fn rect_strategy(max_mm: i64) -> BoxedStrategy<Rect> {
    (
        -max_mm..max_mm,
        1..=2 * max_mm,
        -max_mm..max_mm,
        1..=2 * max_mm,
    )
        .prop_map(move |(x0, w, y0, h)| {
            // x0 < max_mm and w >= 1, so x1 > x0 even after clamping; the map
            // is applied to every shrunk value, so the invariant survives.
            let x1 = (x0 + w).min(max_mm);
            let y1 = (y0 + h).min(max_mm);
            Rect::from_mm(x0, y0, x1, y1).expect("bounded whole-mm rectangle")
        })
        .boxed()
}

fn tagged_strategy(limits: &GeometryLimits) -> Result<BoxedStrategy<Tagged>, BoundsError> {
    let bounds = geometry_bounds(limits, catalog_units(limits));
    let mut branches = Vec::new();
    for class in CLASSES {
        let strategy = geometry_cases(domain(class, bounds.clone())).map_err(|error| {
            BoundsError::Arithmetic {
                what: format!("{class:?} catalog domain: {error}"),
            }
        })?;
        branches.push(
            strategy
                .prop_map(move |case| Tagged { class, case })
                .boxed(),
        );
    }
    Ok(Union::new(branches).boxed())
}

/// Build the `H-BOOL-SET` strategy. Every bound failure surfaces here, before
/// any value is sampled.
pub fn strategy(limits: &GeometryLimits) -> Result<BoxedStrategy<BoolInput>, BoundsError> {
    let max_mm = limits.max_abs_units / UNITS_PER_MM;
    if max_mm < 1 {
        return Err(BoundsError::Arithmetic {
            what: "rectangle family needs at least one whole mm".into(),
        });
    }
    let (base_low, base_high) =
        base_range(limits).map_err(|what| BoundsError::Arithmetic { what })?;
    let tagged = tagged_strategy(limits)?;
    Ok((
        (rect_strategy(max_mm), rect_strategy(max_mm)),
        tagged.clone(),
        tagged,
        (-REL_CATALOG..=REL_CATALOG, -REL_CATALOG..=REL_CATALOG),
        (-REL_CATALOG..=REL_CATALOG, -REL_CATALOG..=REL_CATALOG),
        (base_low..=base_high, base_low..=base_high),
    )
        .prop_map(|(rects, a, b, rel_b, copy, base)| BoolInput {
            rects,
            a,
            b,
            rel_b,
            copy,
            base,
        })
        .boxed())
}

/// Realised operands in canonical units.
#[derive(Debug, Clone)]
pub struct Realized {
    /// Operand A.
    pub a: Vec<ExPolygon>,
    /// Operand B.
    pub b: Vec<ExPolygon>,
    /// Translated copy of A.
    pub copy: Vec<ExPolygon>,
}

fn normalized(case: &GeometryCase) -> Result<Vec<ExPolygon>, String> {
    let points: Vec<Point2> = rings(&case.polygons)
        .flat_map(|ring| ring.points.iter().copied())
        .collect();
    let (Some(min_x), Some(min_y)) = (
        points.iter().map(|p| p.x).min(),
        points.iter().map(|p| p.y).min(),
    ) else {
        return Ok(Vec::new());
    };
    translate_polygons(
        &case.polygons,
        min_x.checked_neg().ok_or("negate overflow")?,
        min_y.checked_neg().ok_or("negate overflow")?,
    )
}

/// Place one operand: `SCALE * (normalised + rel) + base`, checked.
///
/// Bound proof: catalog coordinates are within `±c` with
/// `c = max_abs_units / (4 * SCALE)`, so normalised ones are in `[0, 2c]` and
/// `SCALE * (normalised + rel)` lies in
/// `[-SCALE*REL, max_abs_units/2 + SCALE*REL]`. `base_range` keeps the base
/// in `[-max_abs_units + SCALE*REL, max_abs_units/2 - SCALE*REL]`, so every
/// coordinate is within `[-max_abs_units, max_abs_units]`. The caller also
/// re-validates with #2 `validate_geometry` at that bound.
fn place(case: &GeometryCase, rel: (i64, i64), base: (i64, i64)) -> Result<Vec<ExPolygon>, String> {
    let shifted = translate_polygons(&normalized(case)?, rel.0, rel.1)?;
    let scaled = scale_polygons(&shifted, SCALE)?;
    translate_polygons(&scaled, base.0, base.1)
}

/// Realise one generated input and validate every operand against its own
/// class domain at the registry coordinate bound.
pub fn realize(input: &BoolInput, limits: &GeometryLimits) -> Result<Realized, String> {
    let a = place(&input.a.case, (0, 0), input.base)?;
    let b = place(&input.b.case, input.rel_b, input.base)?;
    let copy = place(&input.a.case, input.copy, input.base)?;
    for (label, class, polygons) in [
        ("A", input.a.class, &a),
        ("B", input.b.class, &b),
        ("copy", input.a.class, &copy),
    ] {
        validate_geometry(
            &GeometryCase {
                polygons: polygons.clone(),
            },
            &realised_domain(class, limits),
        )
        .map_err(|error| format!("generator invariant: realised {label} ({class:?}): {error}"))?;
    }
    Ok(Realized { a, b, copy })
}

// ─────────────────────────────────────────────────────────────────────────────
// SUT outputs and independent checkers.
// ─────────────────────────────────────────────────────────────────────────────

/// The four production results for one ordered operand pair.
#[derive(Debug, Clone, PartialEq)]
pub struct BoolOutputs {
    /// `union(a, b)`.
    pub union: Vec<ExPolygon>,
    /// `intersection(a, b)`.
    pub intersection: Vec<ExPolygon>,
    /// `difference(a, b)`.
    pub difference: Vec<ExPolygon>,
    /// `xor(a, b)`.
    pub xor: Vec<ExPolygon>,
}

impl BoolOutputs {
    /// Call all four real production wrappers.
    pub fn compute(a: &[ExPolygon], b: &[ExPolygon]) -> Self {
        Self {
            union: union(a, b),
            intersection: intersection(a, b),
            difference: difference(a, b),
            xor: xor(a, b),
        }
    }

    /// The result of one operation.
    pub fn get(&self, op: BoolOp) -> &[ExPolygon] {
        match op {
            BoolOp::Union => &self.union,
            BoolOp::Intersection => &self.intersection,
            BoolOp::Difference => &self.difference,
            BoolOp::Xor => &self.xor,
        }
    }
}

/// Independent ring and winding validation of one output set: every ring is
/// simple (#2 `validate_ring`), contours are CCW and holes CW by exact
/// `signed_area2`. No class or nesting claim is made here.
pub fn validate_output_rings(label: &str, polygons: &[ExPolygon]) -> Result<(), String> {
    for (index, polygon) in polygons.iter().enumerate() {
        validate_ring(&polygon.contour)
            .map_err(|error| format!("{label}[{index}] contour invalid: {error}"))?;
        let area = signed_area2(&polygon.contour).map_err(|error| error.to_string())?;
        if area <= 0 {
            return Err(format!(
                "{label}[{index}] contour is not counter-clockwise (double area {area})"
            ));
        }
        for (hole_index, hole) in polygon.holes.iter().enumerate() {
            validate_ring(hole)
                .map_err(|error| format!("{label}[{index}].hole[{hole_index}] invalid: {error}"))?;
            let area = signed_area2(hole).map_err(|error| error.to_string())?;
            if area >= 0 {
                return Err(format!(
                    "{label}[{index}].hole[{hole_index}] is not clockwise (double area {area})"
                ));
            }
        }
    }
    Ok(())
}

/// Weakly-simple ring validation for **general** Boolean outputs.
///
/// Clipper's `NonZero` output may legitimately **pinch**: a single ring can
/// pass through one point twice, because two lobes of the region meet only
/// there (an arch, a bar joined to a plate at one corner). Such a ring repeats
/// a vertex non-consecutively, so the strict simple-ring predicate (#2
/// `validate_ring`, which rejects any repeated vertex) does not apply to this
/// family; it is used only where the expected result is a single simple
/// rectangle. The predicate below is the **weakly simple** ring predicate:
/// self-touching at points is allowed, everything else that `validate_ring`
/// rejected is still rejected, and a self-crossing that hides at a repeated
/// vertex is rejected too.
///
/// Accepted:
/// * a ring with at least three vertices, no *consecutive* duplicate, and
///   nonzero signed double area whose sign matches the ring's role (contour
///   CCW, hole CW);
/// * a non-consecutive vertex revisit at which the two incident edges of **every
///   visit are adjacent in the cyclic angular order** of all the incident edges
///   at that vertex — no edge of another visit lies strictly between them, so
///   each visit leaves and re-enters through the same angular wedge and the
///   passes touch there instead of crossing. Exactly collinear incident
///   directions (two edges along one ray) share a single angular position and
///   are left to the collinear-overlap rule below;
/// * any other single-point meeting between two non-adjacent edges: adjacent
///   edges share exactly their common endpoint, and non-adjacent edges either
///   are disjoint or touch at exactly one point (the pinch).
///
/// Rejected (each is what the strict predicate rejected, kept here):
/// * a **proper crossing** between two non-adjacent edges (a bowtie), which
///   makes the ring traverse a region twice with opposite winding;
/// * a **vertex crossing**: a vertex visited more than once at which some other
///   visit's incident edge lies strictly between the two edges of a visit in the
///   cyclic angular order around that vertex. The strands interleave, which is
///   a transverse self-intersection of the boundary **at** the vertex — the
///   figure-8 a per-edge-pair test cannot see, because every one of its edge
///   pairs only shares that vertex;
/// * a **collinear overlap** between two edges — adjacent or not — where the
///   shared set has positive length, so the boundary is not a curve;
/// * a **backtracking spike**: three consecutive vertices `a, b, c` with zero
///   turn (`orientation(a, b, c) == 0`) and reversal (`(b-a)·(c-b) < 0`), i.e.
///   the boundary runs out to `b` and comes straight back over itself.
///
/// What this certifies: every ring is a closed curve with no transverse
/// self-intersection — neither between two edges nor at a repeated vertex —
/// and no positive-length doubled boundary, and every repeated vertex is a
/// *touch* whose two passes occupy disjoint angular wedges there. It does not
/// prove that the two lobes are disjoint anywhere away from the shared point:
/// whether the region is semantically right is decided by the membership
/// oracle, not by ring shape, and this predicate only certifies the listed
/// shape properties with the demanded winding.
///
/// **Known limitation, deliberate and recorded rather than implied:** a vertex
/// lying on the *interior* of another edge is a T-touch, which
/// `segment_relation` reports as an endpoint touch, and the vertex-interleave
/// rule above only inspects vertices the ring visits more than once. A ring
/// whose crossing happens to sit exactly at such a T-touch is therefore
/// accepted. Closing it needs the edges split at every T-touch before the
/// interleave test; until then the guarantee is "no crossing between edges, and
/// no interleaved crossing at a repeated vertex". Such a ring is not rejected
/// structurally; whether its region is right is certified only by the
/// membership oracle.
pub fn validate_general_output_rings(label: &str, polygons: &[ExPolygon]) -> Result<(), String> {
    /// Zero turn and a reversal at `here`: `cross == 0` and `dot < 0`.
    fn backtracks(prev: Point2, here: Point2, next: Point2) -> Result<bool, String> {
        if orientation(prev, here, next).map_err(|error| error.to_string())? != 0 {
            return Ok(false);
        }
        let sub = |to: i64, from: i64| i128::from(to).checked_sub(i128::from(from));
        let product = |a: i128, b: i128| {
            a.checked_mul(b)
                .ok_or_else(|| String::from("spike dot product overflow"))
        };
        let (in_x, in_y) = (
            sub(here.x, prev.x).ok_or("spike delta overflow")?,
            sub(here.y, prev.y).ok_or("spike delta overflow")?,
        );
        let (out_x, out_y) = (
            sub(next.x, here.x).ok_or("spike delta overflow")?,
            sub(next.y, here.y).ok_or("spike delta overflow")?,
        );
        let dot = product(in_x, out_x)?
            .checked_add(product(in_y, out_y)?)
            .ok_or_else(|| String::from("spike dot product overflow"))?;
        Ok(dot < 0)
    }

    /// Exact cyclic angular comparison of two edges incident at `centre`, each
    /// named by its far endpoint. The half-plane is decided by coordinate
    /// comparison alone, and the order inside a half is #2 `orientation`'s
    /// checked sign of `(a - centre) x (b - centre)`, so the comparison is
    /// integer-exact and never produces a float.
    fn incident_angle_cmp(centre: Point2, a: Point2, b: Point2) -> Result<Ordering, String> {
        /// `0` for the closed upper half-plane starting at `+x`, `1` below it.
        fn half(centre: Point2, far: Point2) -> u8 {
            if far.y > centre.y || (far.y == centre.y && far.x > centre.x) {
                0
            } else {
                1
            }
        }
        let (half_a, half_b) = (half(centre, a), half(centre, b));
        if half_a != half_b {
            return Ok(half_a.cmp(&half_b));
        }
        let turn = orientation(centre, a, b).map_err(|error| error.to_string())?;
        Ok(turn.cmp(&0))
    }

    /// Rejects every **vertex crossing** of the ring: a vertex the ring visits
    /// more than once whose incident edges interleave in cyclic angular order.
    ///
    /// At a vertex visited `k` times there are `2k` incident edge ends — for
    /// each visit, the direction back along `prev -> P` and forward along
    /// `P -> next`. Sort them by exact angle, collapse exactly collinear
    /// directions into one angular position, and pair each visit's two ends by
    /// position. A pair is crossed by another visit exactly when an odd number
    /// of that visit's ends lies strictly inside the arc between the pair's two
    /// positions (the standard chord-crossing test on a circle): the two
    /// passes then alternate, so the curve crosses at `P` instead of touching
    /// it. Ties are only possible for two edges along one ray, which overlap
    /// and are rejected by the collinear-overlap rule; they are still ordered
    /// deterministically by visit index.
    ///
    /// Two cyclically adjacent visits of the same point are one zero-length
    /// pass, not a pinch, and are skipped here: the duplicate/consecutive rules
    /// own that degenerate shape.
    fn reject_vertex_crossings(label: &str, points: &[Point2]) -> Result<(), String> {
        let n = points.len();
        let mut inspected: Vec<Point2> = Vec::new();
        for index in 0..n {
            let centre = points[index];
            if inspected.contains(&centre) {
                continue;
            }
            inspected.push(centre);
            let visits: Vec<usize> = (0..n).filter(|&other| points[other] == centre).collect();
            if visits.len() < 2 {
                continue;
            }
            if visits
                .iter()
                .any(|&first| visits.iter().any(|&second| (first + 1) % n == second))
            {
                continue;
            }
            let mut incident: Vec<(Point2, usize)> = Vec::with_capacity(2 * visits.len());
            for (visit, &occurrence) in visits.iter().enumerate() {
                incident.push((points[(occurrence + n - 1) % n], visit));
                incident.push((points[(occurrence + 1) % n], visit));
            }
            // Insertion sort: at most a handful of entries, and the comparator
            // is fallible, so `sort_by` cannot be used.
            for write in 1..incident.len() {
                let mut probe = write;
                while probe > 0 {
                    if incident_angle_cmp(centre, incident[probe - 1].0, incident[probe].0)?
                        == Ordering::Greater
                    {
                        incident.swap(probe - 1, probe);
                        probe -= 1;
                    } else {
                        break;
                    }
                }
            }
            let mut slot = vec![0usize; incident.len()];
            for position in 1..incident.len() {
                let same =
                    incident_angle_cmp(centre, incident[position - 1].0, incident[position].0)?
                        == Ordering::Equal;
                slot[position] = if same {
                    slot[position - 1]
                } else {
                    slot[position - 1] + 1
                };
            }
            let mut positions = vec![[0usize; 2]; visits.len()];
            let mut filled = vec![0usize; visits.len()];
            for (position, &(_, visit)) in incident.iter().enumerate() {
                if filled[visit] == 2 {
                    return Err(format!(
                        "{label} passes through {centre:?} more than twice in one visit"
                    ));
                }
                positions[visit][filled[visit]] = slot[position];
                filled[visit] += 1;
            }
            for (visit, &[first, second]) in positions.iter().enumerate() {
                if first == second {
                    return Err(format!(
                        "{label} runs out to {centre:?} and back along the same ray, so the \
                         boundary is doubled there"
                    ));
                }
                let (low, high) = (first.min(second), first.max(second));
                for (other, &[left, right]) in positions.iter().enumerate() {
                    if other == visit {
                        continue;
                    }
                    let between = usize::from(low < left && left < high)
                        + usize::from(low < right && right < high);
                    if between % 2 == 1 {
                        return Err(format!(
                            "{label} crosses at vertex {centre:?}: the edges of visits {visit} \
                             and {other} interleave in the cyclic angular order around it, so the \
                             boundary passes through the vertex instead of touching it"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn weakly_simple(label: &str, ring: &Polygon, want_positive: bool) -> Result<(), String> {
        let points = &ring.points;
        let n = points.len();
        if n < 3 {
            return Err(format!("{label} has fewer than three points: {points:?}"));
        }
        if points.windows(2).any(|pair| pair[0] == pair[1]) || points[0] == points[n - 1] {
            return Err(format!(
                "{label} repeats a vertex consecutively, including across the closing seam: {points:?}"
            ));
        }
        let area = signed_area2(ring).map_err(|error| error.to_string())?;
        if area == 0 {
            return Err(format!("{label} has zero double area"));
        }
        if want_positive && area < 0 {
            return Err(format!(
                "{label} is not counter-clockwise (double area {area})"
            ));
        }
        if !want_positive && area > 0 {
            return Err(format!("{label} is not clockwise (double area {area})"));
        }
        for index in 0..n {
            let (prev, here) = (points[(index + n - 1) % n], points[index]);
            let next = points[(index + 1) % n];
            if backtracks(prev, here, next)? {
                return Err(format!(
                    "{label} backtracks at vertex {here:?}: the boundary runs out to a \
                     collinear spike and returns over itself"
                ));
            }
        }
        // Runs before the pairwise loop so that a ring which both crosses at a
        // repeated vertex and (as a consequence) crosses two edges elsewhere is
        // reported for the vertex, which is the defect a per-edge-pair test
        // cannot see.
        reject_vertex_crossings(label, points)?;
        let edge = |index: usize| [points[index], points[(index + 1) % n]];
        // Exact closed bounding-box overlap: a pair that fails it is #2
        // `SegmentRelation::Disjoint`, so skipping it here changes no verdict
        // and keeps the pairwise check cheap on large Clipper rings.
        let may_meet = |p: [Point2; 2], q: [Point2; 2]| {
            let span =
                |a: Point2, b: Point2| (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
            let (px0, px1, py0, py1) = span(p[0], p[1]);
            let (qx0, qx1, qy0, qy1) = span(q[0], q[1]);
            px0 <= qx1 && qx0 <= px1 && py0 <= qy1 && qy0 <= py1
        };
        for i in 0..n {
            for j in (i + 1)..n {
                let adjacent = j == i + 1 || (i == 0 && j == n - 1);
                if !adjacent && !may_meet(edge(i), edge(j)) {
                    continue;
                }
                let relation =
                    segment_relation(edge(i), edge(j)).map_err(|error| error.to_string())?;
                match relation {
                    SegmentRelation::ProperCrossing => {
                        return Err(format!(
                            "{label} edges {i} and {j} properly cross: {:?} and {:?}",
                            edge(i),
                            edge(j)
                        ));
                    }
                    SegmentRelation::CollinearOverlap => {
                        return Err(format!(
                            "{label} edges {i} and {j} overlap collinearly instead of meeting at \
                             a single point: {:?} and {:?}",
                            edge(i),
                            edge(j)
                        ));
                    }
                    SegmentRelation::EndpointTouch => {}
                    SegmentRelation::Disjoint => {
                        // Adjacent edges always share their common endpoint, so a
                        // disjoint adjacent pair is a construction error, not the
                        // pinch this predicate exists to accept.
                        if adjacent {
                            return Err(format!(
                                "{label} adjacent edges {i} and {j} do not share their common \
                                 endpoint: {:?} and {:?}",
                                edge(i),
                                edge(j)
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    for (index, polygon) in polygons.iter().enumerate() {
        weakly_simple(&format!("{label}[{index}] contour"), &polygon.contour, true)?;
        for (hole_index, hole) in polygon.holes.iter().enumerate() {
            weakly_simple(&format!("{label}[{index}].hole[{hole_index}]"), hole, false)?;
        }
    }
    Ok(())
}

fn location(point: Point2, polygons: &[ExPolygon]) -> Result<PointLocation, String> {
    point_in_region(point, polygons).map_err(|error| format!("point_in_region: {error}"))
}

/// Observed counts for the rectangle reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectStats {
    /// Open cells compared per operation.
    pub cells: usize,
    /// Expected-inside cells per operation, in `BoolOp::ALL` order.
    pub inside_cells: [usize; 4],
}

/// The complete exact rectangle reference for all four operations.
pub fn check_rect_pair(a: Rect, b: Rect, out: &BoolOutputs) -> Result<RectStats, String> {
    let xs = sorted_unique(vec![a.x0, a.x1, b.x0, b.x1]);
    let ys = sorted_unique(vec![a.y0, a.y1, b.y0, b.y1]);
    let cells = cell_partition(&xs, &ys)?;
    if cells.is_empty() {
        return Err(String::from("rectangle partition produced no cell"));
    }
    let mut inside_cells = [0_usize; 4];
    for (slot, op) in BoolOp::ALL.into_iter().enumerate() {
        let result = out.get(op);
        let label = format!("{op:?}");
        validate_output_rings(&label, result)?;
        for ring in rings(result) {
            let n = ring.points.len();
            for index in 0..n {
                let p = ring.points[index];
                let q = ring.points[(index + 1) % n];
                if xs.binary_search(&p.x).is_err() || ys.binary_search(&p.y).is_err() {
                    return Err(format!(
                        "{label}: vertex {p:?} is not on the input edge grid x={xs:?} y={ys:?}"
                    ));
                }
                if p.x != q.x && p.y != q.y {
                    return Err(format!("{label}: edge {p:?}->{q:?} is not axis-aligned"));
                }
            }
        }
        let mut expected = Vec::new();
        for cell in &cells {
            let want = op.truth(a.contains_cell(cell), b.contains_cell(cell));
            let mid = cell.midpoint().ok_or_else(|| {
                format!("cell {cell:?} has no integer midpoint: whole-mm precondition violated")
            })?;
            let got = location(mid, result)?;
            let ok = matches!(
                (want, got),
                (true, PointLocation::Inside) | (false, PointLocation::Outside)
            );
            if !ok {
                return Err(format!(
                    "{label}: cell {cell:?} midpoint {mid:?} expected {} but output is {got:?}; \
                     A={a:?} B={b:?} output={result:?}",
                    if want { "Inside" } else { "Outside" }
                ));
            }
            if want {
                expected.push(*cell);
            }
        }
        inside_cells[slot] = expected.len();
        let want_area = cells_double_area(&expected)?;
        let got_area = region_double_area(result)?;
        if want_area != got_area {
            return Err(format!(
                "{label}: double area {got_area} != expected cell sum {want_area}; A={a:?} B={b:?}"
            ));
        }
    }
    Ok(RectStats {
        cells: cells.len(),
        inside_cells,
    })
}

/// Observed populations of the general sample reference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SampleStats {
    /// Samples classified (after the margin filter and the query cap).
    pub accepted: usize,
    /// Samples strictly inside A.
    pub in_a: usize,
    /// Samples strictly inside B.
    pub in_b: usize,
    /// Samples outside both operands.
    pub outside_both: usize,
    /// Samples inside both operands.
    pub in_both: usize,
    /// Rings (components and holes of both operands) with a witness sample.
    pub ring_witnesses: usize,
}

fn axis_samples(values: &[i64]) -> Vec<i64> {
    let values = sorted_unique(values.to_vec());
    let (Some(&first), Some(&last)) = (values.first(), values.last()) else {
        return vec![0];
    };
    let mut out = vec![first - 2 * MARGIN];
    for pair in values.windows(2) {
        let gap = pair[1] - pair[0];
        out.push(pair[0] + gap / 4);
        out.push(pair[0] + (3 * gap) / 4);
    }
    out.push(last + 2 * MARGIN);
    sorted_unique(out)
}

fn ring_bbox(ring: &Polygon) -> Option<(i64, i64, i64, i64)> {
    Some((
        ring.points.iter().map(|p| p.x).min()?,
        ring.points.iter().map(|p| p.y).min()?,
        ring.points.iter().map(|p| p.x).max()?,
        ring.points.iter().map(|p| p.y).max()?,
    ))
}

/// Deterministic, nonempty, bounded sample of points clear of every edge of
/// both operands, with one witness per component and per hole first.
///
/// The general population clears every input edge by [`MARGIN`]; a witness only
/// has to clear them by [`WITNESS_MARGIN`] (see that constant's proof), so a
/// small catalogue feature is still observable. If not even a
/// [`WITNESS_MARGIN`]-clear lattice point lies inside a ring, that is a hard
/// error: an unobservable ring must not be skipped silently.
pub fn sample_points(
    a: &[ExPolygon],
    b: &[ExPolygon],
    max_queries: usize,
) -> Result<(Vec<Point2>, usize), String> {
    let all: Vec<ExPolygon> = a.iter().chain(b.iter()).cloned().collect();
    let all_edges = edges(&all);
    let vertices: Vec<Point2> = rings(&all)
        .flat_map(|ring| ring.points.iter().copied())
        .collect();
    let xs = axis_samples(&vertices.iter().map(|p| p.x).collect::<Vec<_>>());
    let ys = axis_samples(&vertices.iter().map(|p| p.y).collect::<Vec<_>>());
    let clears = |p: Point2, margin: i64| -> Result<bool, String> {
        for &(s, t) in &all_edges {
            if !point_clears_segment(p, s, t, margin)? {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let mut candidates = Vec::new();
    let mut witness_candidates = Vec::new();
    for &y in &ys {
        for &x in &xs {
            let p = Point2 { x, y };
            if clears(p, MARGIN)? {
                candidates.push(p);
                witness_candidates.push(p);
            } else if clears(p, WITNESS_MARGIN)? {
                witness_candidates.push(p);
            }
        }
    }
    if candidates.is_empty() {
        return Err(String::from("no lattice sample clears every input edge"));
    }
    // One witness per component (strictly in its material) and per hole
    // (strictly inside the hole ring). Only candidates in the ring's bounding
    // box are probed; the classification itself is #2 `point_in_region`.
    let mut chosen: Vec<Point2> = Vec::new();
    let mut ring_witnesses = 0;
    for polygon in &all {
        let mut targets: Vec<(ExPolygon, Polygon)> =
            vec![(polygon.clone(), polygon.contour.clone())];
        for hole in &polygon.holes {
            targets.push((
                ExPolygon {
                    contour: hole.clone(),
                    holes: Vec::new(),
                },
                hole.clone(),
            ));
        }
        for (region, ring) in targets {
            let (x0, y0, x1, y1) = ring_bbox(&ring).ok_or("empty ring")?;
            let inside = |pool: &[Point2]| -> Result<Option<Point2>, String> {
                for p in pool
                    .iter()
                    .filter(|p| x0 < p.x && p.x < x1 && y0 < p.y && p.y < y1)
                {
                    if location(*p, std::slice::from_ref(&region))? == PointLocation::Inside {
                        return Ok(Some(*p));
                    }
                }
                Ok(None)
            };
            // Prefer a witness clearing every input edge by `MARGIN`, exactly as
            // before; only a ring too small to contain one falls back to
            // `WITNESS_MARGIN`.
            let witness = match inside(&candidates)? {
                Some(point) => point,
                None => inside(&witness_candidates)?.ok_or_else(|| {
                    format!(
                        "no lattice witness at Euclidean distance >= {WITNESS_MARGIN} from every \
                         input edge and strictly inside ring {ring:?}; sample construction failed"
                    )
                })?,
            };
            ring_witnesses += 1;
            if !chosen.contains(&witness) {
                chosen.push(witness);
            }
        }
    }
    // The first candidate is the exterior lower-left corner point.
    if !chosen.contains(&candidates[0]) {
        chosen.push(candidates[0]);
    }
    if chosen.len() > max_queries {
        return Err(format!(
            "{} ring witnesses exceed max_queries {max_queries}",
            chosen.len()
        ));
    }
    let rest: Vec<Point2> = candidates
        .iter()
        .copied()
        .filter(|p| !chosen.contains(p))
        .collect();
    let room = max_queries - chosen.len();
    if rest.len() <= room {
        chosen.extend(rest);
    } else if room > 0 {
        // Evenly strided deterministic subset.
        for index in 0..room {
            chosen.push(rest[index * rest.len() / room]);
        }
    }
    Ok((chosen, ring_witnesses))
}

/// The general bounded-sample reference for all four operations.
pub fn check_general_pair(
    a: &[ExPolygon],
    b: &[ExPolygon],
    out: &BoolOutputs,
    max_queries: usize,
) -> Result<SampleStats, String> {
    for op in BoolOp::ALL {
        validate_general_output_rings(&format!("{op:?}"), out.get(op))?;
    }
    let (samples, ring_witnesses) = sample_points(a, b, max_queries)?;
    let mut stats = SampleStats {
        ring_witnesses,
        ..SampleStats::default()
    };
    for p in samples {
        let in_a = match location(p, a)? {
            PointLocation::Inside => true,
            PointLocation::Outside => false,
            PointLocation::Boundary => {
                return Err(format!(
                    "sample {p:?} on A's boundary: margin filter failed"
                ))
            }
        };
        let in_b = match location(p, b)? {
            PointLocation::Inside => true,
            PointLocation::Outside => false,
            PointLocation::Boundary => {
                return Err(format!(
                    "sample {p:?} on B's boundary: margin filter failed"
                ))
            }
        };
        stats.accepted += 1;
        stats.in_a += usize::from(in_a);
        stats.in_b += usize::from(in_b);
        stats.in_both += usize::from(in_a && in_b);
        stats.outside_both += usize::from(!in_a && !in_b);
        for op in BoolOp::ALL {
            let want = op.truth(in_a, in_b);
            let got = location(p, out.get(op))?;
            let ok = matches!(
                (want, got),
                (true, PointLocation::Inside) | (false, PointLocation::Outside)
            );
            if !ok {
                return Err(format!(
                    "{op:?}: sample {p:?} (in A: {in_a}, in B: {in_b}) expected {} but output is \
                     {got:?}; A={a:?} B={b:?} output={:?}",
                    if want { "Inside" } else { "Outside" },
                    out.get(op)
                ));
            }
        }
    }
    // Nonempty populations: the check can never pass by observing nothing.
    if stats.accepted == 0 || stats.outside_both == 0 {
        return Err(format!("vacuous sample population {stats:?}"));
    }
    if (!a.is_empty() && stats.in_a == 0) || (!b.is_empty() && stats.in_b == 0) {
        return Err(format!(
            "a nonempty operand was never observed inside: {stats:?}"
        ));
    }
    let expected_rings = rings(a).count() + rings(b).count();
    if stats.ring_witnesses != expected_rings {
        return Err(format!(
            "{} of {expected_rings} rings have a witness sample",
            stats.ring_witnesses
        ));
    }
    if is_rectilinear(a) && is_rectilinear(b) {
        check_area_identities(a, b, out)?;
    }
    Ok(stats)
}

/// Exact inclusion–exclusion identities; valid only for rectilinear operands,
/// whose Boolean intersection vertices are exact integers.
pub fn check_area_identities(
    a: &[ExPolygon],
    b: &[ExPolygon],
    out: &BoolOutputs,
) -> Result<(), String> {
    let area_a = region_double_area(a)?;
    let area_b = region_double_area(b)?;
    let u = region_double_area(&out.union)?;
    let i = region_double_area(&out.intersection)?;
    let d = region_double_area(&out.difference)?;
    let x = region_double_area(&out.xor)?;
    let add = |p: i128, q: i128| p.checked_add(q).ok_or("area sum overflow");
    let sub = |p: i128, q: i128| p.checked_sub(q).ok_or("area difference overflow");
    if add(u, i)? != add(area_a, area_b)? {
        return Err(format!(
            "|A∪B|+|A∩B| = {} != |A|+|B| = {}",
            u + i,
            area_a + area_b
        ));
    }
    if d != sub(area_a, i)? {
        return Err(format!("|A\\B| = {d} != |A|-|A∩B| = {}", area_a - i));
    }
    if x != sub(u, i)? {
        return Err(format!("|A⊕B| = {x} != |A∪B|-|A∩B| = {}", u - i));
    }
    Ok(())
}

/// Same-set identities for one operand `g` and the SUT results of `(g, g)`.
pub fn check_identities(
    g: &[ExPolygon],
    out: &BoolOutputs,
    class_domain: &GeometryDomain,
    max_queries: usize,
) -> Result<(), String> {
    if !out.difference.is_empty() {
        return Err(format!("G \\ G is not empty: {:?}", out.difference));
    }
    if !out.xor.is_empty() {
        return Err(format!("G xor G is not empty: {:?}", out.xor));
    }
    check_general_pair(g, g, out, max_queries)?;
    // Here the expected result is G itself, so G's own strict structure
    // (disjoint component closures, strictly nested holes, class witness)
    // is proved for union and intersection.
    for (label, result) in [("G∪G", &out.union), ("G∩G", &out.intersection)] {
        validate_geometry(
            &GeometryCase {
                polygons: result.clone(),
            },
            class_domain,
        )
        .map_err(|error| format!("{label} lost the operand's structure: {error}"))?;
        if is_rectilinear(g) {
            let want = region_double_area(g)?;
            let got = region_double_area(result)?;
            if want != got {
                return Err(format!("{label} double area {got} != |G| {want}"));
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Property body.
// ─────────────────────────────────────────────────────────────────────────────

fn check_input(input: &BoolInput, limits: &GeometryLimits) -> Result<(), String> {
    let (ra, rb) = input.rects;
    let rect_stats = check_rect_pair(
        ra,
        rb,
        &BoolOutputs::compute(&[ra.to_expolygon()], &[rb.to_expolygon()]),
    )
    .map_err(|error| format!("rectangle family: {error}"))?;
    if rect_stats.inside_cells[0] == 0 {
        return Err(String::from("rectangle union observed no inside cell"));
    }
    let realized = realize(input, limits)?;
    check_general_pair(
        &realized.a,
        &realized.b,
        &BoolOutputs::compute(&realized.a, &realized.b),
        limits.max_queries,
    )
    .map_err(|error| format!("general pair: {error}"))?;
    check_identities(
        &realized.a,
        &BoolOutputs::compute(&realized.a, &realized.a),
        &realised_domain(input.a.class, limits),
        limits.max_queries,
    )
    .map_err(|error| format!("identities: {error}"))?;
    check_general_pair(
        &realized.a,
        &realized.copy,
        &BoolOutputs::compute(&realized.a, &realized.copy),
        limits.max_queries,
    )
    .map_err(|error| format!("translated copy: {error}"))?;
    Ok(())
}

/// Square helper for the deterministic witnesses, in canonical units.
fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
    Rect::new(x0, y0, x1, y1).expect("witness rectangle")
}

/// Holed square `[0,80]²` minus `[24,56]²` (canonical units).
pub fn holed_witness() -> ExPolygon {
    ExPolygon {
        contour: square(0, 0, 80, 80).ring(),
        holes: vec![square(24, 24, 56, 56).hole_ring()],
    }
}

/// L-shaped concave witness `[0,64]² \ [32,64]x[32,64]`.
pub fn concave_witness() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: [(0, 0), (64, 0), (64, 32), (32, 32), (32, 64), (0, 64)]
                .into_iter()
                .map(|(x, y)| Point2 { x, y })
                .collect(),
        },
        holes: Vec::new(),
    }
}

/// Chamfered convex octagon with diagonal edges.
pub fn convex_witness() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: [
                (8, 0),
                (56, 0),
                (64, 8),
                (64, 56),
                (56, 64),
                (8, 64),
                (0, 56),
                (0, 8),
            ]
            .into_iter()
            .map(|(x, y)| Point2 { x, y })
            .collect(),
        },
        holes: Vec::new(),
    }
}

/// Deterministic, RNG-independent witnesses run before the generated cases.
pub fn deterministic_witnesses() {
    let mm = |x0, y0, x1, y1| Rect::from_mm(x0, y0, x1, y1).expect("whole-mm witness");
    let rect_pairs = [
        ("overlapping", mm(0, 0, 4, 3), mm(2, 1, 6, 5)),
        ("disjoint", mm(0, 0, 1, 1), mm(3, 3, 5, 6)),
        (
            "nested (difference has a hole)",
            mm(0, 0, 10, 10),
            mm(2, 2, 4, 4),
        ),
        ("edge-touching", mm(0, 0, 2, 2), mm(2, 0, 4, 2)),
        ("identical", mm(-3, -3, 3, 3), mm(-3, -3, 3, 3)),
    ];
    for (label, a, b) in rect_pairs {
        let out = BoolOutputs::compute(&[a.to_expolygon()], &[b.to_expolygon()]);
        let stats = check_rect_pair(a, b, &out)
            .unwrap_or_else(|error| panic!("rectangle witness {label}: {error}"));
        assert!(stats.inside_cells[0] > 0, "{label}: union has inside cells");
    }
    // The nested pair's difference must be one component with one hole.
    let nested = BoolOutputs::compute(
        &[mm(0, 0, 10, 10).to_expolygon()],
        &[mm(2, 2, 4, 4).to_expolygon()],
    );
    assert_eq!(
        nested.difference.len(),
        1,
        "nested difference is one component"
    );
    assert_eq!(
        nested.difference[0].holes.len(),
        1,
        "nested difference keeps the hole"
    );

    let bar = square(40, 32, 120, 48).to_expolygon();
    let pair_a = vec![
        square(0, 0, 16, 16).to_expolygon(),
        square(32, 0, 48, 16).to_expolygon(),
    ];
    let bridge = square(8, 4, 40, 12).to_expolygon();
    let general = [
        (
            "holed vs bar through the hole",
            vec![holed_witness()],
            vec![bar.clone()],
        ),
        ("disconnected vs bridge", pair_a.clone(), vec![bridge]),
        (
            "concave vs bar",
            vec![concave_witness()],
            vec![square(16, 16, 48, 48).to_expolygon()],
        ),
        (
            "convex (diagonal edges) vs bar",
            vec![convex_witness()],
            vec![bar.clone()],
        ),
        ("empty vs holed", Vec::new(), vec![holed_witness()]),
        ("holed vs empty", vec![holed_witness()], Vec::new()),
    ];
    for (label, a, b) in general {
        let out = BoolOutputs::compute(&a, &b);
        let stats = check_general_pair(&a, &b, &out, 4096)
            .unwrap_or_else(|error| panic!("general witness {label}: {error}"));
        assert!(
            stats.accepted > 0 && stats.outside_both > 0,
            "{label}: {stats:?}"
        );
    }
    let bounds = GeometryBounds {
        max_abs_units: 1_000,
        ..GeometryBounds::default()
    };
    for (label, class, g) in [
        ("holed", TopologyClass::Holed, vec![holed_witness()]),
        ("disconnected", TopologyClass::Disconnected, pair_a),
        ("concave", TopologyClass::Concave, vec![concave_witness()]),
        ("convex", TopologyClass::Convex, vec![convex_witness()]),
    ] {
        check_identities(
            &g,
            &BoolOutputs::compute(&g, &g),
            &domain(class, bounds.clone()),
            4096,
        )
        .unwrap_or_else(|error| panic!("identity witness {label}: {error}"));
    }
}

/// Registry entry point for `proptest_boolean_set_algebra`.
pub fn run_property() {
    deterministic_witnesses();
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("use the registry-backed property command");
    let raw = RawGeometryBounds::read(&run).expect("H-BOOL-SET bounds");
    let limits = validate_limits(raw).expect("H-BOOL-SET profile");
    let strategy = strategy(&limits).expect("H-BOOL-SET strategy");
    run.check(&strategy, |input, _guard| -> TestCaseResult {
        check_input(&input, &limits).map_err(TestCaseError::fail)
    })
    .expect("H-BOOL-SET charter");
}
