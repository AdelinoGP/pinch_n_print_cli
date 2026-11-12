//! Checked conversion from raw engine geometry to PnP IR geometry.
//!
//! This module never repairs. Every function either returns geometry that has
//! passed every declared check or a [`ConversionFailure`] naming the check that
//! failed. It is engine-neutral on purpose: it consumes plain `f64` rings and a
//! plain triangle soup, so the private unit tests below can drive
//! otherwise-unconstructible damage without making any upstream type public and
//! without requiring the `openrscad` feature.
//!
//! # Coordinate policy for the external `f64` boundary
//!
//! The engine hands over millimetres as `f64`. PnP 2D geometry is `i64` scaled by
//! [`slicer_ir::slice_ir::UNITS_PER_MM`] (1 unit = 100 nm = 10⁻⁴ mm) and PnP 3D
//! geometry is millimetre `f32`. The canonical rounding rule from
//! `docs/08_coordinate_system.md` is round-half-away-from-zero, and the existing
//! `mm_to_units` helper cannot enforce it here: it takes `f32` and its
//! `as i64` cast saturates silently. This module therefore applies the same
//! normative rule directly to `f64`, in five checked steps:
//!
//! 1. the input is finite;
//! 2. `|input| <= limits.max_abs_coordinate_mm`;
//! 3. the scaled product `input * UNITS_PER_MM` is finite;
//! 4. the rounded value lies inside the half-open representable `i64` range;
//! 5. only then is it cast.
//!
//! Steps 3–5 are what a saturating cast hides: `(f64::MAX * 1e4).round() as i64`
//! is `i64::MAX`, i.e. a plausible-looking coordinate at the far edge of the
//! grid, not an error. The f64 multiply/round error is recorded separately from
//! the integer half-unit quantization: rounding to the nearest grid point
//! contributes at most 0.5 unit (0.00005 mm) when the product itself is exact,
//! and the product is not exact for an arbitrary `f64`. Neither is claimed to
//! make an arbitrary float round trip exact.
//!
//! # Contour contract
//!
//! Input is the engine's flat, unordered, arbitrarily wound set of closed rings
//! under an even-odd fill interpretation. Output is the IR's
//! [`slicer_ir::slice_ir::ExPolygon`] forest: outer contours CCW, holes CW,
//! nested islands as separate components.
//!
//! Reconstruction is independent of input order and input winding. Even nesting
//! depth becomes an outer contour, odd depth becomes a hole of its immediate
//! container. A ring may be closed by an exactly equal terminal point; that
//! redundant representation is removed. Nothing else is removed or merged.
//!
//! Contacts are checked twice. Before quantization, self-contact and cross-ring
//! contact are validated on the **original millimetre `f64` rings** with
//! `robust::orient2d`, an adaptive-precision predicate that returns the exact
//! orientation sign (and exactly `0.0` for collinear points). After quantization
//! they are validated again on the 100 nm output grid with exact `i128`
//! predicates. Rounding is the hazard: two rings that cross in `f64` can round
//! into two disjoint integer rings, so a grid-only check can accept a crossing.
//! No rounded surrogate is used, at any resolution.
//!
//! The raw pass has no coordinate-magnitude precondition: `robust` is exact for
//! every finite input. The integer pass enforces the documented `2^40`-unit bound
//! so its own `i128` arithmetic is provably safe.
//!
//! # Mesh contract
//!
//! Input is the engine's indexed triangle soup in millimetres. Output is the
//! IR's flat `Vec<u32>` triangle stream with `f32` millimetre vertices. Exact
//! coincident-position deduplication is allowed only as representation
//! normalisation that preserves the triangle mapping; there is no epsilon
//! welding, face deletion, hole filling, auto-repair or fallback tessellation.
//! Edge-incidence validation does not claim arbitrary 3D self-intersection
//! freedom.

use crate::{ExPolygon, IndexedTriangleSet, ScadErrorKind, ScadLimits};

/// Raw triangle soup as the engine hands it over, in millimetres.
///
/// Indices are a **flat** stream, three per triangle, mirroring the IR output.
/// The flat shape is deliberate: it is the shape in which a truncated index
/// stream is representable at all, so the `len % 3 == 0` check below is a real
/// check rather than a vacuous one on an already-grouped `[u32; 3]` array.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawMesh {
    /// One `[x, y, z]` per vertex, in millimetres.
    pub vertices: Vec<[f64; 3]>,
    /// Flat index stream, three indices per triangle, indexing `vertices`.
    pub indices: Vec<u32>,
}

/// A rejected conversion: the error taxonomy kind plus the exact check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConversionFailure {
    /// Which [`ScadErrorKind`] the failed check maps to.
    pub kind: ScadErrorKind,
    /// The failed check, with the offending values.
    pub message: String,
}

impl ConversionFailure {
    /// Build a failure for `kind`.
    pub(crate) fn new(kind: ScadErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Convert flat engine rings into a checked even-odd [`ExPolygon`] forest.
///
/// # Errors
///
/// [`ScadErrorKind::NonFiniteCoordinate`] for a NaN/infinite coordinate,
/// [`ScadErrorKind::CoordinateOutOfRange`] when a coordinate, its scaled
/// product or its rounded integer is outside the configured or representable
/// range, [`ScadErrorKind::ComplexityLimit`] when the declared vertex bound is
/// exceeded, and [`ScadErrorKind::InvalidContours`] for any topology damage:
/// a ring with fewer than three distinct vertices, a zero-area ring, a
/// self-intersecting or self-touching ring, two rings that intersect or touch,
/// a ring that quantization collapses, or nesting that quantization changes.
/// Exact orientation sign of three raw millimetre points.
///
/// `robust::orient2d` is an adaptive-precision predicate: for any finite `f64`
/// input it returns the **exact** sign, and exactly `0.0` for collinear points.
/// That is what makes the pre-quantization contact check sound — a crossing cannot
/// be rounded out of existence because nothing is rounded.
fn raw_orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    robust::orient2d(
        robust::Coord { x: a[0], y: a[1] },
        robust::Coord { x: b[0], y: b[1] },
        robust::Coord { x: c[0], y: c[1] },
    )
}

/// Whether `p` lies on segment `a..b`, decided exactly.
fn raw_on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    raw_orient(a, b, p) == 0.0
        && a[0].min(b[0]) <= p[0]
        && p[0] <= a[0].max(b[0])
        && a[1].min(b[1]) <= p[1]
        && p[1] <= a[1].max(b[1])
}

/// Whether two raw segments intersect or touch, decided exactly.
fn raw_intersects(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let ab_c = raw_orient(a, b, c);
    let ab_d = raw_orient(a, b, d);
    let cd_a = raw_orient(c, d, a);
    let cd_b = raw_orient(c, d, b);
    let straddles = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    (straddles(ab_c, ab_d) && straddles(cd_a, cd_b))
        || raw_on_segment(a, b, c)
        || raw_on_segment(a, b, d)
        || raw_on_segment(c, d, a)
        || raw_on_segment(c, d, b)
}

/// Validate self-contact and cross-ring contact on the ORIGINAL raw rings.
///
/// This is the pre-quantization half of the contour contract. It runs on the
/// millimetre `f64` values the engine produced, before any rounding, so a crossing
/// that rounding would separate is still seen. The post-quantization pass
/// ([`validate_contacts`]) remains the authoritative check on the emitted grid.
fn check_raw_contacts(rings: &[&[[f64; 2]]]) -> Result<(), ConversionFailure> {
    #[derive(Clone, Copy)]
    struct RawEdge {
        ring: usize,
        index: usize,
        a: [f64; 2],
        b: [f64; 2],
    }

    let mut edges: Vec<RawEdge> = Vec::new();
    for (ring_index, ring) in rings.iter().enumerate() {
        for index in 0..ring.len() {
            edges.push(RawEdge {
                ring: ring_index,
                index,
                a: ring[index],
                b: ring[(index + 1) % ring.len()],
            });
        }
    }
    // Output-sensitive x-interval sweep: only x-overlapping edges are candidates.
    edges.sort_by(|left, right| {
        left.a[0]
            .min(left.b[0])
            .total_cmp(&right.a[0].min(right.b[0]))
            .then(left.ring.cmp(&right.ring))
            .then(left.index.cmp(&right.index))
    });

    let mut active: Vec<RawEdge> = Vec::new();
    for edge in edges.iter().copied() {
        let min_x = edge.a[0].min(edge.b[0]);
        active.retain(|other| other.a[0].max(other.b[0]) >= min_x);
        for other in &active {
            if edge.ring == other.ring {
                let n = rings[edge.ring].len();
                if (edge.index + 1) % n == other.index || (other.index + 1) % n == edge.index {
                    continue;
                }
            }
            if edge.a[1].max(edge.b[1]) < other.a[1].min(other.b[1])
                || other.a[1].max(other.b[1]) < edge.a[1].min(edge.b[1])
            {
                continue;
            }
            if raw_intersects(edge.a, edge.b, other.a, other.b) {
                return Err(ConversionFailure::new(
                    ScadErrorKind::InvalidContours,
                    format!(
                        "raw rings {} and {} intersect or touch: edges {} and {}",
                        edge.ring, other.ring, edge.index, other.index
                    ),
                ));
            }
        }
        active.push(edge);
    }
    Ok(())
}

pub(crate) fn convert_contours(
    raw: &[Vec<[f64; 2]>],
    limits: &ScadLimits,
) -> Result<Vec<ExPolygon>, ConversionFailure> {
    use slicer_ir::slice_ir::{Point2, Polygon, UNITS_PER_MM};
    use std::collections::BTreeSet;

    fn invalid(message: impl Into<String>) -> ConversionFailure {
        ConversionFailure::new(ScadErrorKind::InvalidContours, message)
    }

    fn quantize(value: f64, units_per_mm: f64) -> Result<i64, ConversionFailure> {
        let scaled = value * units_per_mm;
        let rounded = scaled.round();
        // i64::MAX rounds UP to 2^63 as f64, hence the exclusive upper end.
        if !scaled.is_finite() || !((i64::MIN as f64)..(-(i64::MIN as f64))).contains(&rounded) {
            return Err(ConversionFailure::new(
                ScadErrorKind::CoordinateOutOfRange,
                format!("coordinate {value} has an unrepresentable scaled product {scaled}"),
            ));
        }
        let units = rounded as i64;
        // With |coordinate| <= 2^40, each shoelace term is at most 2^81
        // and each difference-coordinate cross product is at most 2^83.
        // For at most 2^20 distinct points, shoelace partial sums stay below
        // 2^101, safely inside i128. Larger caller-provided vertex budgets
        // are also safe: area accumulation below additionally uses checked_add.
        if !(-(1_i64 << 40)..=(1_i64 << 40)).contains(&units) {
            return Err(ConversionFailure::new(
                ScadErrorKind::CoordinateOutOfRange,
                format!("coordinate {value} exceeds the 2^40-unit i128 safety bound"),
            ));
        }
        Ok(units)
    }

    fn orient(a: Point2, b: Point2, c: Point2) -> i128 {
        (i128::from(b.x) - i128::from(a.x)) * (i128::from(c.y) - i128::from(a.y))
            - (i128::from(b.y) - i128::from(a.y)) * (i128::from(c.x) - i128::from(a.x))
    }

    fn on_segment(a: Point2, b: Point2, p: Point2) -> bool {
        orient(a, b, p) == 0
            && (a.x.min(b.x)..=a.x.max(b.x)).contains(&p.x)
            && (a.y.min(b.y)..=a.y.max(b.y)).contains(&p.y)
    }

    fn intersects(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
        let ab_c = orient(a, b, c);
        let ab_d = orient(a, b, d);
        let cd_a = orient(c, d, a);
        let cd_b = orient(c, d, b);
        (ab_c.signum() * ab_d.signum() < 0 && cd_a.signum() * cd_b.signum() < 0)
            || on_segment(a, b, c)
            || on_segment(a, b, d)
            || on_segment(c, d, a)
            || on_segment(c, d, b)
    }

    #[derive(Clone, Copy)]
    struct Edge {
        ring: usize,
        index: usize,
        a: Point2,
        b: Point2,
    }

    fn check_contacts(
        edges: &mut [Edge],
        rings: &[Vec<Point2>],
        cross_ring: bool,
    ) -> Result<(), ConversionFailure> {
        edges.sort_unstable_by_key(|e| (e.a.x.min(e.b.x), e.ring, e.index));
        let mut active: Vec<Edge> = Vec::new();
        // Output-sensitive x-interval sweep: only overlapping x intervals are
        // candidates, with an additional y-interval prune. Its cost is
        // O(E log E + K), where K counts x-overlapping edge pairs, not just
        // actual contacts. max_vertices bounds the quadratic worst case.
        for edge in edges.iter().copied() {
            let min_x = edge.a.x.min(edge.b.x);
            active.retain(|other| other.a.x.max(other.b.x) >= min_x);
            for other in &active {
                if (edge.ring != other.ring) != cross_ring
                    || edge.a.y.max(edge.b.y) < other.a.y.min(other.b.y)
                    || other.a.y.max(other.b.y) < edge.a.y.min(edge.b.y)
                {
                    continue;
                }
                if !cross_ring {
                    let n = rings[edge.ring].len();
                    if (edge.index + 1) % n == other.index || (other.index + 1) % n == edge.index {
                        // Adjacent overlap is checked separately below; their
                        // common endpoint alone is a valid ring connection.
                        continue;
                    }
                }
                if intersects(edge.a, edge.b, other.a, other.b) {
                    return Err(invalid(format!(
                        "rings {} and {} have intersecting or touching edges {} and {}",
                        edge.ring, other.ring, edge.index, other.index
                    )));
                }
            }
            active.push(edge);
        }
        Ok(())
    }

    fn validate_contacts(rings: &[Vec<Point2>]) -> Result<(), ConversionFailure> {
        let mut edges = Vec::new();
        for (ring_index, ring) in rings.iter().enumerate() {
            let mut own_edges = Vec::with_capacity(ring.len());
            for index in 0..ring.len() {
                let a = ring[index];
                let b = ring[(index + 1) % ring.len()];
                let c = ring[(index + 2) % ring.len()];
                if orient(a, b, c) == 0 && (on_segment(a, b, c) || on_segment(b, c, a)) {
                    return Err(invalid(format!(
                        "ring {ring_index} has overlapping adjacent edges at {index}"
                    )));
                }
                own_edges.push(Edge {
                    ring: ring_index,
                    index,
                    a,
                    b,
                });
            }
            check_contacts(&mut own_edges, rings, false)?;
            edges.extend(own_edges);
        }
        check_contacts(&mut edges, rings, true)
    }

    fn quantized_rings(
        raw_rings: &[&[[f64; 2]]],
        units_per_mm: f64,
    ) -> Result<Vec<Vec<Point2>>, ConversionFailure> {
        raw_rings
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|point| {
                        Ok(Point2 {
                            x: quantize(point[0], units_per_mm)?,
                            y: quantize(point[1], units_per_mm)?,
                        })
                    })
                    .collect()
            })
            .collect()
    }

    fn contains_integer(ring: &[Point2], point: Point2) -> bool {
        let mut inside = false;
        for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            if (a.y > point.y) != (b.y > point.y) && (orient(*a, *b, point) > 0) == (b.y > a.y) {
                inside = !inside;
            }
        }
        inside
    }

    /// Exact even-odd containment of `point` in a raw millimetre ring.
    ///
    /// The crossing test is the exact orientation sign ([`raw_orient`]), not a
    /// division-derived intersection: the previous float form could round the
    /// intersection onto the query point's own X, flipping "inside" to "outside" and
    /// hiding a nesting change introduced by quantization.
    fn contains_float(ring: &[[f64; 2]], point: [f64; 2]) -> bool {
        let mut inside = false;
        for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            if (a[1] > point[1]) != (b[1] > point[1]) {
                let orient = raw_orient(*a, *b, point);
                if (orient > 0.0) == (b[1] > a[1]) {
                    inside = !inside;
                }
            }
        }
        inside
    }

    type Forest = (Vec<usize>, Vec<Option<usize>>);

    fn nesting(
        count: usize,
        contains: impl Fn(usize, usize) -> bool,
    ) -> Result<Forest, ConversionFailure> {
        let depths: Vec<_> = (0..count)
            .map(|child| {
                (0..count)
                    .filter(|&outer| outer != child && contains(outer, child))
                    .count()
            })
            .collect();
        let mut parents = vec![None; count];
        for child in 0..count {
            let mut container_depths = Vec::with_capacity(depths[child]);
            for outer in 0..count {
                if outer != child && contains(outer, child) {
                    container_depths.push(depths[outer]);
                    if depths[outer].checked_add(1) == Some(depths[child]) {
                        parents[child] = Some(outer);
                    }
                }
            }
            container_depths.sort_unstable();
            if !container_depths.iter().copied().eq(0..depths[child]) {
                return Err(invalid(format!(
                    "ring {child} is not in a proper nesting forest"
                )));
            }
        }
        for child in 0..count {
            let mut ancestor = parents[child];
            let mut seen = 0;
            while let Some(outer) = ancestor {
                if seen >= depths[child] || !contains(outer, child) {
                    return Err(invalid(format!(
                        "ring {child} has inconsistent nesting parents"
                    )));
                }
                seen += 1;
                ancestor = parents[outer];
            }
            if seen != depths[child] {
                return Err(invalid(format!(
                    "ring {child} has an incomplete nesting chain"
                )));
            }
        }
        Ok((depths, parents))
    }

    let total = raw.iter().try_fold(0_usize, |total, ring| {
        total.checked_add(ring.len()).ok_or_else(|| {
            ConversionFailure::new(
                ScadErrorKind::ComplexityLimit,
                "contour vertex count overflow",
            )
        })
    })?;
    if total > limits.max_vertices {
        return Err(ConversionFailure::new(
            ScadErrorKind::ComplexityLimit,
            format!(
                "{total} contour vertices exceed max_vertices {}",
                limits.max_vertices
            ),
        ));
    }

    let mut raw_rings = Vec::with_capacity(raw.len());
    for (ring_index, ring) in raw.iter().enumerate() {
        for (point_index, point) in ring.iter().enumerate() {
            for &coordinate in point {
                if !coordinate.is_finite() {
                    return Err(ConversionFailure::new(
                        ScadErrorKind::NonFiniteCoordinate,
                        format!(
                            "ring {ring_index} vertex {point_index} is non-finite: {coordinate}"
                        ),
                    ));
                }
                if coordinate.abs() > limits.max_abs_coordinate_mm {
                    return Err(ConversionFailure::new(
                        ScadErrorKind::CoordinateOutOfRange,
                        format!("ring {ring_index} vertex {point_index} exceeds the coordinate bound: {coordinate}"),
                    ));
                }
            }
        }
        let mut points = ring.as_slice();
        if points.len() > 1 && points.first() == points.last() {
            points = &points[..points.len() - 1];
        }
        if points.len() < 3 {
            return Err(invalid(format!(
                "ring {ring_index} has fewer than three vertices"
            )));
        }
        // Translate to the first vertex to avoid avoidable cancellation from
        // a large common origin. This is the same signed shoelace area.
        let origin = points[0];
        let area: f64 = points[1..]
            .windows(2)
            .map(|pair| {
                (pair[0][0] - origin[0]) * (pair[1][1] - origin[1])
                    - (pair[0][1] - origin[1]) * (pair[1][0] - origin[0])
            })
            .sum();
        if area == 0.0 {
            return Err(invalid(format!(
                "ring {ring_index} has zero raw signed area"
            )));
        }
        raw_rings.push(points);
    }

    // Pre-quantization contact validation runs on the ORIGINAL millimetre `f64`
    // rings with exact adaptive predicates. A fixed-resolution surrogate cannot do
    // this job: rounding the input to any grid can turn a crossing into two
    // disjoint rings, so a finer grid only moves the threshold at which a crossing
    // is hidden instead of removing it.
    check_raw_contacts(&raw_rings)?;
    let mut rings = quantized_rings(&raw_rings, UNITS_PER_MM)?;
    let mut areas = Vec::with_capacity(rings.len());
    for (ring_index, ring) in rings.iter().enumerate() {
        if ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .any(|(a, b)| a == b)
        {
            return Err(invalid(format!(
                "ring {ring_index}: quantization collapsed an edge"
            )));
        }
        let distinct: BTreeSet<_> = ring.iter().copied().collect();
        if distinct.len() < 3 {
            return Err(invalid(format!(
                "ring {ring_index} has fewer than three distinct integer points"
            )));
        }
        if distinct.len() != ring.len() {
            return Err(invalid(format!("ring {ring_index} repeats an integer point (raw repetition or quantization collision)")));
        }
        let area =
            ring.iter()
                .zip(ring.iter().cycle().skip(1))
                .try_fold(0_i128, |sum, (a, b)| {
                    sum.checked_add(
                        i128::from(a.x) * i128::from(b.y) - i128::from(a.y) * i128::from(b.x),
                    )
                    .ok_or_else(|| {
                        invalid(format!("ring {ring_index} exceeds the i128 area bound"))
                    })
                })?;
        if area == 0 {
            return Err(invalid(format!(
                "ring {ring_index} has zero quantized signed area"
            )));
        }
        areas.push(area);
    }

    validate_contacts(&rings).map_err(|failure| {
        invalid(format!("quantized grid: {}", failure.message))
    })?;

    let (depths, parents) = nesting(rings.len(), |outer, child| {
        contains_integer(&rings[outer], rings[child][0])
    })?;
    let raw_forest = nesting(raw_rings.len(), |outer, child| {
        contains_float(raw_rings[outer], raw_rings[child][0])
    })?;
    if raw_forest.0 != depths || raw_forest.1 != parents {
        return Err(invalid(
            "quantization changed nesting (depth or immediate parent)",
        ));
    }

    let mut output = Vec::new();
    for index in 0..rings.len() {
        if depths[index] % 2 != 0 {
            continue;
        }
        if areas[index] < 0 {
            rings[index].reverse();
        }
        let contour = Polygon {
            points: std::mem::take(&mut rings[index]),
        };
        let mut holes = Vec::new();
        for hole in 0..rings.len() {
            if parents[hole] == Some(index) && depths[hole] % 2 == 1 {
                if areas[hole] > 0 {
                    rings[hole].reverse();
                }
                holes.push(Polygon {
                    points: std::mem::take(&mut rings[hole]),
                });
            }
        }
        output.push(ExPolygon { contour, holes });
    }
    Ok(output)
}

/// Convert a raw engine triangle soup into a checked [`IndexedTriangleSet`].
///
/// # Errors
///
/// [`ScadErrorKind::ComplexityLimit`] when the declared vertex or triangle
/// bound is exceeded, [`ScadErrorKind::NonFiniteCoordinate`] for a NaN/infinite
/// vertex, [`ScadErrorKind::CoordinateOutOfRange`] when a vertex or its `f32`
/// narrows out of the configured or representable range,
/// [`ScadErrorKind::InvalidMesh`] for a bad or truncated index, a degenerate or
/// inverted triangle, a non-distinct triangle, or edge incidence that is not
/// closed and consistently oriented.
pub(crate) fn convert_mesh(
    raw: &RawMesh,
    limits: &ScadLimits,
) -> Result<IndexedTriangleSet, ConversionFailure> {
    use slicer_ir::slice_ir::Point3;
    use std::collections::BTreeMap;

    fn invalid(message: impl Into<String>) -> ConversionFailure {
        ConversionFailure::new(ScadErrorKind::InvalidMesh, message)
    }

    fn normal64(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    fn normal32(a: Point3, b: Point3, c: Point3) -> [f32; 3] {
        let u = [b.x - a.x, b.y - a.y, b.z - a.z];
        let v = [c.x - a.x, c.y - a.y, c.z - a.z];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    if raw.vertices.len() > limits.max_vertices || raw.indices.len() / 3 > limits.max_triangles {
        return Err(ConversionFailure::new(
            ScadErrorKind::ComplexityLimit,
            format!(
                "mesh has {} vertices and {} triangles, exceeding the declared limits",
                raw.vertices.len(),
                raw.indices.len() / 3
            ),
        ));
    }
    if !raw.indices.len().is_multiple_of(3) {
        return Err(invalid(format!(
            "truncated index stream: {} indices",
            raw.indices.len()
        )));
    }
    if raw.indices.is_empty() {
        return Ok(IndexedTriangleSet {
            vertices: Vec::new(),
            indices: Vec::new(),
        });
    }

    let mut vertices = Vec::with_capacity(raw.vertices.len());
    let mut canonical = BTreeMap::new();
    let mut position_ids = Vec::with_capacity(raw.vertices.len());
    for (index, position) in raw.vertices.iter().enumerate() {
        let mut narrowed = [0.0_f32; 3];
        for (axis, &value) in position.iter().enumerate() {
            if !value.is_finite() {
                return Err(ConversionFailure::new(
                    ScadErrorKind::NonFiniteCoordinate,
                    format!("mesh vertex {index} axis {axis} is non-finite: {value}"),
                ));
            }
            if value.abs() > limits.max_abs_coordinate_mm {
                return Err(ConversionFailure::new(
                    ScadErrorKind::CoordinateOutOfRange,
                    format!(
                        "mesh vertex {index} axis {axis} exceeds the coordinate bound: {value}"
                    ),
                ));
            }
            let value32 = value as f32;
            if !value32.is_finite() {
                return Err(ConversionFailure::new(
                    ScadErrorKind::CoordinateOutOfRange,
                    format!("mesh vertex {index} axis {axis} overflows f32: {value}"),
                ));
            }
            narrowed[axis] = if value32.is_subnormal() { 0.0 } else { value32 };
        }
        // Signed zeros are the same exact position. Canonicalize their key,
        // not the output vertex: the 1:1 vertex mapping remains untouched.
        let key = narrowed.map(|value| if value == 0.0 { 0 } else { value.to_bits() });
        let next_id = canonical.len();
        position_ids.push(*canonical.entry(key).or_insert(next_id));
        vertices.push(Point3 {
            x: narrowed[0],
            y: narrowed[1],
            z: narrowed[2],
        });
    }
    for (offset, &index) in raw.indices.iter().enumerate() {
        if index as usize >= vertices.len() {
            return Err(invalid(format!(
                "bad or truncated index {index} at stream offset {offset}"
            )));
        }
    }

    let mut incidence: BTreeMap<(usize, usize), (usize, usize)> = BTreeMap::new();
    for (triangle_index, triangle) in raw.indices.chunks_exact(3).enumerate() {
        let [a, b, c] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        if a == b
            || b == c
            || c == a
            || vertices[a] == vertices[b]
            || vertices[b] == vertices[c]
            || vertices[c] == vertices[a]
        {
            return Err(invalid(format!(
                "degenerate triangle {triangle_index}: repeated index or normalized position"
            )));
        }
        let before = normal64(raw.vertices[a], raw.vertices[b], raw.vertices[c]);
        let after = normal32(vertices[a], vertices[b], vertices[c]);
        // hypot avoids a spurious zero from squaring a small nonzero normal.
        let area = after[0].hypot(after[1]).hypot(after[2]);
        let dot = before
            .iter()
            .zip(after)
            .map(|(&x, y)| x * f64::from(y))
            .sum::<f64>();
        if !before.iter().all(|v| v.is_finite())
            || before.iter().all(|&v| v == 0.0)
            || !area.is_finite()
            || area <= 0.0
            || !dot.is_finite()
            || dot < 0.0
        {
            return Err(invalid(format!("inverted or collapsed triangle {triangle_index}: normalized area {area}, normal dot {dot}")));
        }
        let ids = [position_ids[a], position_ids[b], position_ids[c]];
        for (from, to) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
            let counts = incidence.entry((from.min(to), from.max(to))).or_default();
            if from < to {
                counts.0 += 1;
            } else {
                counts.1 += 1;
            }
        }
    }
    for (edge, counts) in incidence {
        if counts != (1, 1) {
            return Err(invalid(format!("edge {edge:?} is not closed and consistently oriented: directed incidences {counts:?}")));
        }
    }
    Ok(IndexedTriangleSet {
        vertices,
        indices: raw.indices.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::slice_ir::{Point2, Point3, Polygon, UNITS_PER_MM};

    fn square(min: f64, max: f64) -> Vec<[f64; 2]> {
        vec![[min, min], [max, min], [max, max], [min, max]]
    }

    fn area_mm2(polygon: &Polygon) -> f64 {
        let sum: i128 = polygon
            .points
            .iter()
            .zip(polygon.points.iter().cycle().skip(1))
            .map(|(a, b)| i128::from(a.x) * i128::from(b.y) - i128::from(a.y) * i128::from(b.x))
            .sum();
        sum as f64 / (2.0 * UNITS_PER_MM * UNITS_PER_MM)
    }

    fn canonical_forest(mut forest: Vec<ExPolygon>) -> Vec<ExPolygon> {
        for component in &mut forest {
            for polygon in std::iter::once(&mut component.contour).chain(&mut component.holes) {
                let first = polygon
                    .points
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, p)| **p)
                    .map(|(index, _)| index)
                    .expect("test polygons must be nonempty");
                polygon.points.rotate_left(first);
            }
            component.holes.sort_by_key(|hole| hole.points[0]);
        }
        forest.sort_by_key(|component| component.contour.points[0]);
        forest
    }

    fn tetrahedron() -> RawMesh {
        RawMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            // Opposite the origin points toward +x,+y,+z; the other faces
            // point toward -z, -y and -x respectively.
            indices: vec![1, 2, 3, 0, 2, 1, 0, 1, 3, 0, 3, 2],
        }
    }

    #[test]
    fn scad_conversion_contour_forest() {
        let mut input = vec![
            square(45.0, 55.0),
            square(200.0, 300.0),
            square(20.0, 80.0),
            square(0.0, 100.0),
            square(40.0, 60.0),
        ];
        input[0].reverse();
        input[3].reverse();
        let output = convert_contours(&input, &ScadLimits::default())
            .expect("disjoint nested squares must convert");
        // Depths 0, 2 and 0 give THREE components, including the disjoint
        // 200..300 outer. Two would silently drop one requested component.
        assert_eq!(output.len(), 3);
        let actual = canonical_forest(output);
        assert_eq!(area_mm2(&actual[0].contour), 10_000.0);
        assert_eq!(actual[0].holes.len(), 1);
        assert_eq!(area_mm2(&actual[0].holes[0]), -3_600.0);
        assert_eq!(area_mm2(&actual[1].contour), 400.0);
        assert_eq!(actual[1].holes.len(), 1);
        assert_eq!(area_mm2(&actual[1].holes[0]), -100.0);
        assert_eq!(area_mm2(&actual[2].contour), 10_000.0);
        assert!(actual[2].holes.is_empty());
        for component in &actual {
            assert!(area_mm2(&component.contour) > 0.0);
            for hole in &component.holes {
                assert!(area_mm2(hole) < 0.0);
            }
        }
        let polygon = |coordinates: &[(f32, f32)]| Polygon {
            points: coordinates
                .iter()
                .map(|&(x, y)| Point2::from_mm(x, y))
                .collect(),
        };
        // Independent, explicitly enumerated topology and winding oracle.
        let expected = vec![
            ExPolygon {
                contour: polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]),
                holes: vec![polygon(&[
                    (20.0, 20.0),
                    (20.0, 80.0),
                    (80.0, 80.0),
                    (80.0, 20.0),
                ])],
            },
            ExPolygon {
                contour: polygon(&[(40.0, 40.0), (60.0, 40.0), (60.0, 60.0), (40.0, 60.0)]),
                holes: vec![polygon(&[
                    (45.0, 45.0),
                    (45.0, 55.0),
                    (55.0, 55.0),
                    (55.0, 45.0),
                ])],
            },
            ExPolygon {
                contour: polygon(&[
                    (200.0, 200.0),
                    (300.0, 200.0),
                    (300.0, 300.0),
                    (200.0, 300.0),
                ]),
                holes: vec![],
            },
        ];
        assert_eq!(actual, expected);
        input.rotate_left(2);
        for ring in &mut input {
            ring.reverse();
        }
        // test-quality: determinism pin for input-order/winding independence;
        // the independently enumerated forest above supplies correctness.
        let shuffled = convert_contours(&input, &ScadLimits::default())
            .expect("shuffled reversed rings must convert");
        assert_eq!(canonical_forest(shuffled), expected);
    }

    #[test]
    fn scad_rejects_conversion_damage() {
        fn contour_case(
            name: &str,
            rings: Vec<Vec<[f64; 2]>>,
            limits: ScadLimits,
            kind: ScadErrorKind,
        ) {
            let error = convert_contours(&rings, &limits).expect_err(name);
            assert_eq!(error.kind, kind, "{name}: {}", error.message);
        }
        fn mesh_case(name: &str, mesh: RawMesh, limits: ScadLimits, kind: ScadErrorKind) {
            let error = convert_mesh(&mesh, &limits).expect_err(name);
            assert_eq!(error.kind, kind, "{name}: {}", error.message);
        }
        let limits = ScadLimits::default();
        let generous = ScadLimits {
            max_abs_coordinate_mm: f64::MAX,
            ..ScadLimits::default()
        };
        contour_case(
            "NaN contour coordinate",
            vec![vec![[f64::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]]],
            limits,
            ScadErrorKind::NonFiniteCoordinate,
        );
        contour_case(
            "infinite contour coordinate",
            vec![vec![[f64::INFINITY, 0.0], [1.0, 0.0], [0.0, 1.0]]],
            limits,
            ScadErrorKind::NonFiniteCoordinate,
        );
        contour_case(
            "coordinate beyond configured bound",
            vec![square(0.0, limits.max_abs_coordinate_mm + 1.0)],
            limits,
            ScadErrorKind::CoordinateOutOfRange,
        );
        contour_case(
            "scaled coordinate overflows i64",
            vec![square(0.0, 1.0e15)],
            generous,
            ScadErrorKind::CoordinateOutOfRange,
        );
        contour_case(
            "distinct first vertices quantize together",
            vec![vec![[0.0, 0.0], [0.00001, 0.0], [1.0, 1.0], [0.0, 1.0]]],
            limits,
            ScadErrorKind::InvalidContours,
        );
        contour_case(
            "zero-area collinear ring",
            vec![vec![[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]],
            limits,
            ScadErrorKind::InvalidContours,
        );
        contour_case(
            "self-intersecting bowtie",
            vec![vec![[0.0, 0.0], [3.0, 3.0], [0.0, 2.0], [2.0, 0.0]]],
            limits,
            ScadErrorKind::InvalidContours,
        );
        contour_case(
            "two squares touching at one point",
            vec![square(0.0, 1.0), square(1.0, 2.0)],
            limits,
            ScadErrorKind::InvalidContours,
        );
        let mut mesh = tetrahedron();
        mesh.indices[0] = 4;
        mesh_case(
            "out-of-range mesh index",
            mesh,
            limits,
            ScadErrorKind::InvalidMesh,
        );
        let mut mesh = tetrahedron();
        mesh.indices.push(0);
        mesh_case(
            "index stream truncated with remainder one",
            mesh,
            limits,
            ScadErrorKind::InvalidMesh,
        );
        mesh_case(
            "zero-area triangle",
            RawMesh {
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
                indices: vec![0, 1, 2],
            },
            limits,
            ScadErrorKind::InvalidMesh,
        );
        mesh_case(
            "open two-triangle surface",
            RawMesh {
                vertices: vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                ],
                indices: vec![0, 1, 2, 0, 2, 3],
            },
            limits,
            ScadErrorKind::InvalidMesh,
        );
        let mut mesh = tetrahedron();
        mesh.vertices[0][0] = f64::NAN;
        mesh_case(
            "NaN mesh vertex",
            mesh,
            limits,
            ScadErrorKind::NonFiniteCoordinate,
        );
        let mut mesh = tetrahedron();
        mesh.vertices[0][0] = f64::from(f32::MAX) * 2.0;
        mesh_case(
            "finite vertex overflows f32",
            mesh,
            generous,
            ScadErrorKind::CoordinateOutOfRange,
        );
        let vertex_limit = ScadLimits {
            max_vertices: 3,
            ..ScadLimits::default()
        };
        contour_case(
            "contour vertex count exceeds limit",
            vec![square(0.0, 1.0)],
            vertex_limit,
            ScadErrorKind::ComplexityLimit,
        );
        mesh_case(
            "mesh vertex count exceeds limit",
            tetrahedron(),
            vertex_limit,
            ScadErrorKind::ComplexityLimit,
        );
        mesh_case(
            "triangle count exceeds limit",
            tetrahedron(),
            ScadLimits {
                max_triangles: 3,
                ..ScadLimits::default()
            },
            ScadErrorKind::ComplexityLimit,
        );
    }

    #[test]
    fn scad_conversion_mesh_tetrahedron() {
        let raw = tetrahedron();
        let result =
            convert_mesh(&raw, &ScadLimits::default()).expect("outward tetrahedron must convert");
        assert_eq!(result.indices, raw.indices);
        assert_eq!(result.indices.len(), 12);
        assert_eq!(
            result.vertices,
            vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0
                }
            ]
        );
    }

    #[test]
    fn scad_conversion_rejects_raw_crossing_hidden_by_output_quantization() {
        let input: Vec<Vec<[f64; 2]>> = vec![
            vec![[0.0, -0.49], [200.0, 1.51], [200.0, 200.0], [0.0, 200.0]],
            vec![[10.0, -0.49], [90.0, 0.49], [110.0, -1.49]],
        ]
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(|p| [p[0] / UNITS_PER_MM, p[1] / UNITS_PER_MM])
                .collect()
        })
        .collect();
        let error = convert_contours(&input, &ScadLimits::default())
            .expect_err("raw rings cross even though output-grid rings are disjoint");
        assert_eq!(error.kind, ScadErrorKind::InvalidContours);
        assert!(
            error.message.contains("intersect or touch"),
            "{}",
            error.message
        );
    }

    #[test]
    fn scad_conversion_rejects_a_crossing_below_any_surrogate_grid() {
        // A crossing whose excursion is far below both the 100 nm output grid and
        // the old 1 nm surrogate: the apex clears the outer's lower edge
        // `y = x/100` by about 2e-6 grid units. Only exact predicates see it, which
        // is why the contact check runs on the original f64 values rather than on
        // any rounded grid.
        let input: Vec<Vec<[f64; 2]>> = vec![
            vec![
                [0.0, 0.0],
                [200.0, 2.0],
                [200.0, 200.0],
                [0.0, 200.0],
            ],
            vec![[10.0, -0.49], [49.00501, 0.49005011], [110.0, -1.49]],
        ]
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(|p| [p[0] / UNITS_PER_MM, p[1] / UNITS_PER_MM])
                .collect()
        })
        .collect();
        let error = convert_contours(&input, &ScadLimits::default())
            .expect_err("an exact predicate must see a sub-nanometre crossing");
        assert_eq!(error.kind, ScadErrorKind::InvalidContours);
    }

    #[test]
    fn scad_conversion_rejects_changed_nesting_after_quantization() {
        // In grid-unit coordinates, the raw outer's lower edge is
        // y = x/100 - 0.49. Every hole vertex is strictly ABOVE it:
        // y - (x/100 - 0.49) is 0.88, 0.08 and 0.88 respectively.
        // Quantization moves that outer edge to y = x/100, while the hole
        // becomes (10,0), (90,0), (110,1): all strictly BELOW that edge.
        // Neither integer ring collapses or touches the other, so only the
        // raw-vs-integer nesting guard can reject this valid raw holed set.
        let input: Vec<Vec<[f64; 2]>> = vec![
            vec![[0.0, -0.49], [200.0, 1.51], [200.0, 200.0], [0.0, 200.0]],
            vec![[10.0, 0.49], [90.0, 0.49], [110.0, 1.49]],
        ]
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(|p| [p[0] / UNITS_PER_MM, p[1] / UNITS_PER_MM])
                .collect()
        })
        .collect();
        let error = convert_contours(&input, &ScadLimits::default())
            .expect_err("quantization moves the hole outside its parent");
        assert_eq!(error.kind, ScadErrorKind::InvalidContours);
        assert!(
            error.message.contains("quantization changed nesting"),
            "unexpected earlier rejection: {}",
            error.message
        );
    }

    #[test]
    fn scad_conversion_mesh_exact_position_incidence_preserves_mapping() {
        let shared = tetrahedron();
        let raw = RawMesh {
            vertices: shared
                .indices
                .iter()
                .map(|&index| shared.vertices[index as usize])
                .collect(),
            indices: (0..12).collect(),
        };
        let result = convert_mesh(&raw, &ScadLimits::default())
            .expect("exact coincident face vertices form a closed tetrahedron");
        assert_eq!(result.indices, raw.indices);
        assert_eq!(result.vertices.len(), 12);
        for (actual, expected) in result.vertices.iter().zip(&raw.vertices) {
            assert_eq!(
                [
                    f64::from(actual.x),
                    f64::from(actual.y),
                    f64::from(actual.z)
                ],
                *expected
            );
        }
    }

    #[test]
    fn scad_conversion_mesh_rejects_normalization_damage_and_bad_winding() {
        let mut raw = tetrahedron();
        raw.vertices[1][0] = f64::from(f32::from_bits(1));
        let error = convert_mesh(&raw, &ScadLimits::default())
            .expect_err("subnormal normalization coincides two triangle positions");
        assert_eq!(error.kind, ScadErrorKind::InvalidMesh);
        assert!(error.message.contains("normalized position"));
        let mut raw = tetrahedron();
        raw.indices.swap(0, 1);
        let error = convert_mesh(&raw, &ScadLimits::default())
            .expect_err("one reversed tetrahedron face violates oriented incidence");
        assert_eq!(error.kind, ScadErrorKind::InvalidMesh);
        assert!(error.message.contains("directed incidences"));
    }

    #[test]
    fn scad_conversion_contour_contacts_and_collisions_are_not_repaired() {
        let cases = [
            (
                "nonadjacent distinct vertices quantize to one point",
                vec![vec![
                    [0.0, 0.0],
                    [2.0, 0.0],
                    [1.0, 1.0],
                    [0.00001, 0.00001],
                    [0.0, 2.0],
                ]],
                "repeats an integer point",
            ),
            (
                "nonzero raw triangle becomes integer-collinear",
                vec![vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.00001]]],
                "zero quantized signed area",
            ),
            (
                "adjacent collinear segments backtrack and overlap",
                vec![vec![
                    [0.0, 0.0],
                    [2.0, 0.0],
                    [1.0, 0.0],
                    [1.0, 1.0],
                    [0.0, 1.0],
                ]],
                // Edge 2 starts on edge 0, so the exact raw pass names the contact
                // before the integer pass can report the overlap.
                "intersect or touch",
            ),
            (
                "nonadjacent edge touches the interior of another edge",
                vec![vec![
                    [0.0, 0.0],
                    [2.0, 0.0],
                    [2.0, 2.0],
                    [1.0, 0.0],
                    [0.0, 2.0],
                ]],
                // The exact raw pass sees this before quantization, so its message
                // is the one that surfaces.
                "intersect or touch",
            ),
            (
                "two rings cross without a shared vertex",
                vec![
                    square(0.0, 2.0),
                    vec![[1.0, -1.0], [3.0, -1.0], [3.0, 1.0], [1.0, 1.0]],
                ],
                "intersect or touch",
            ),
            (
                "raw-separated rings touch after quantization",
                vec![square(0.0, 1.0), square(1.00001, 2.0)],
                // Raw rings are 1e-5 mm apart, so only the quantized pass fires.
                "intersecting or touching edges",
            ),
        ];
        for (name, input, check) in cases {
            let error = convert_contours(&input, &ScadLimits::default()).expect_err(name);
            assert_eq!(error.kind, ScadErrorKind::InvalidContours, "{name}");
            assert!(error.message.contains(check), "{name}: {}", error.message);
        }
    }

    #[test]
    fn scad_conversion_rejects_changed_nesting_hidden_by_float_containment() {
        // The child triangle is strictly inside the convex raw outer, but after
        // quantization it rounds entirely outside. A float ray-cast rounded the
        // intersection onto the query point's own X and reported "outside" for the
        // raw ring too, so the damaged forest looked unchanged. Exact orientation
        // containment makes the change visible, so the result is rejected.
        let outer = vec![
            [0.0, -0.000049],
            [0.02, 0.000151],
            [0.02, 0.02],
            [0.0, 0.02],
        ];
        let child = vec![
            [0.0009999999999999998, -0.000039],
            [0.009, 0.000049],
            [0.011, 0.000149],
        ];
        for rotation in 0..child.len() {
            let mut rotated = child.clone();
            rotated.rotate_left(rotation);
            let input = vec![outer.clone(), rotated];
            let error = convert_contours(&input, &ScadLimits::default()).expect_err(
                "a nesting change introduced by quantization must be rejected, whatever                  vertex the child ring starts at",
            );
            assert_eq!(error.kind, ScadErrorKind::InvalidContours);
            assert!(
                error.message.contains("changed nesting"),
                "rotation {rotation}: {}",
                error.message
            );
        }
    }

    #[test]
    fn scad_conversion_rounding_and_integer_safety_bound() {
        let output = convert_contours(&[square(-0.00005, 0.00005)], &ScadLimits::default())
            .expect("half units round away from zero on both signs");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].contour.points,
            // One grid unit = 0.0001 mm, not raw millimetre coordinates.
            vec![
                Point2 { x: -1, y: -1 },
                Point2 { x: 1, y: -1 },
                Point2 { x: 1, y: 1 },
                Point2 { x: -1, y: 1 },
            ]
        );
        // The raw pass uses exact predicates and needs no magnitude bound. The
        // output-grid pass does, so it alone sets the accepted millimetre
        // boundary: 2^40 units `= 2^40 / UNITS_PER_MM` mm.
        let bound_mm = (1_u64 << 40) as f64 / UNITS_PER_MM;
        let limits = ScadLimits {
            max_abs_coordinate_mm: bound_mm * 2.0,
            ..ScadLimits::default()
        };
        let output = convert_contours(&[square(-bound_mm, bound_mm)], &limits)
            .expect("inclusive arithmetic bound must fit exact i128 geometry");
        assert_eq!(output.len(), 1);
        let output_bound = (bound_mm * UNITS_PER_MM).round() as i64;
        assert_eq!(output[0].contour.points[0].x, -output_bound);
        assert_eq!(output[0].contour.points[2].y, output_bound);
        let error = convert_contours(&[square(0.0, bound_mm + 1.0)], &limits).expect_err(
            "representable i64 coordinates beyond the i128 safety envelope are rejected",
        );
        assert_eq!(error.kind, ScadErrorKind::CoordinateOutOfRange);
        assert!(error.message.contains("i128 safety bound"));
    }

    #[test]
    fn scad_conversion_mesh_narrowing_rejects_flips_and_area_collapse() {
        // At 1.5, one f32 ULP is 2^-23. Relative to that origin, these raw
        // xy points in ULPs are (-.49,.49), (1.49,.51), (1.51,1.49).
        // Their signed double area is 1.94 ULP^2, but the narrowed points
        // (0,0), (1,1), (2,1) have signed double area -1 ULP^2.
        let step = 2.0_f64.powi(-23);
        let raw = RawMesh {
            vertices: vec![
                [1.5 - 0.49 * step, 1.5 + 0.49 * step, 0.0],
                [1.5 + 1.49 * step, 1.5 + 0.51 * step, 0.0],
                [1.5 + 1.51 * step, 1.5 + 1.49 * step, 0.0],
            ],
            indices: vec![0, 1, 2],
        };
        let error = convert_mesh(&raw, &ScadLimits::default())
            .expect_err("f32 narrowing reverses a nonzero triangle normal");
        assert_eq!(error.kind, ScadErrorKind::InvalidMesh);
        assert!(error.message.contains("inverted or collapsed triangle"));
        assert!(error.message.contains("normal dot -"), "{}", error.message);
        let raw = RawMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 2.0 + 2.0_f64.powi(-25), 0.0],
            ],
            indices: vec![0, 1, 2],
        };
        let error = convert_mesh(&raw, &ScadLimits::default())
            .expect_err("three still-distinct f32 positions become collinear");
        assert_eq!(error.kind, ScadErrorKind::InvalidMesh);
        assert!(error.message.contains("normalized area 0"));
    }

    #[test]
    fn scad_conversion_empty_and_single_closure() {
        let limits = ScadLimits::default();
        assert_eq!(
            convert_contours(&[], &limits).expect("empty contours are valid"),
            Vec::<ExPolygon>::new()
        );
        assert_eq!(
            convert_mesh(
                &RawMesh {
                    vertices: vec![],
                    indices: vec![]
                },
                &limits
            )
            .expect("empty mesh is valid"),
            IndexedTriangleSet {
                vertices: vec![],
                indices: vec![]
            }
        );
        let mut closed = square(0.0, 1.0);
        closed.push(closed[0]);
        let result = convert_contours(&[closed.clone()], &limits)
            .expect("one exact closure point is allowed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].contour.points.len(), 4);
        assert_eq!(area_mm2(&result[0].contour), 1.0);
        closed.push(closed[0]);
        let error = convert_contours(&[closed], &limits)
            .expect_err("a second repeated closure point must not be silently removed");
        assert_eq!(error.kind, ScadErrorKind::InvalidContours);
    }
}
