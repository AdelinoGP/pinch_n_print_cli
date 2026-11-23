//! Independent elementary integer/rational geometry for small test inputs.
//!
//! Coordinates are canonical 100 nm units. In a validated domain, |x| and |y|
//! are at most 1,000,000 and the total vertex bound fits in u32. An orientation
//! determinant has magnitude at most 8*M^2; a shoelace term has magnitude at
//! most 2*M^2, so even u32::MAX terms total less than 2^74. Both fit i128.
//! Public standalone orientation/area still check every operation for arbitrary
//! i64 inputs. Intersection parameters, clipping, and distance comparisons use
//! arbitrary precision: their intermediate rational products have no i128 bound.
//! This module uses only `slicer-ir` data types, `num-bigint`, and
//! `num-rational` for geometry. Although the SDK also depends on `slicer-core`,
//! no production geometry, Clipper, or geo predicates are used here.

use std::cmp::{max, min, Ordering};
use std::collections::BTreeSet;

use num_bigint::BigInt;
use num_rational::BigRational;
use slicer_ir::{ExPolygon, Point2, Polygon};

use super::super::domains::{
    BoundaryRule, DomainError, GeometryCase, GeometryDomain, TopologyClass,
};
use super::{PointLocation, SegmentRelation};

fn invalid_input(message: &str) -> DomainError {
    DomainError::InvalidInput(message.into())
}

fn invalid_bounds(message: &str) -> DomainError {
    DomainError::InvalidBounds(message.into())
}

fn edges(ring: &Polygon) -> impl Iterator<Item = [Point2; 2]> + '_ {
    ring.points
        .iter()
        .copied()
        .zip(ring.points.iter().copied().cycle().skip(1))
        .take(ring.points.len())
        .map(|(a, b)| [a, b])
}

fn sign(order: Ordering) -> i8 {
    match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub(super) fn checked_orientation(a: Point2, b: Point2, c: Point2) -> Result<i8, DomainError> {
    let sub = |a: i64, b: i64| {
        i128::from(a)
            .checked_sub(i128::from(b))
            .ok_or(DomainError::ArithmeticOverflow)
    };
    let abx = sub(b.x, a.x)?;
    let aby = sub(b.y, a.y)?;
    let acx = sub(c.x, a.x)?;
    let acy = sub(c.y, a.y)?;
    let left = abx
        .checked_mul(acy)
        .ok_or(DomainError::ArithmeticOverflow)?;
    let right = aby
        .checked_mul(acx)
        .ok_or(DomainError::ArithmeticOverflow)?;
    let cross = left
        .checked_sub(right)
        .ok_or(DomainError::ArithmeticOverflow)?;
    Ok(sign(cross.cmp(&0)))
}

pub(super) fn checked_signed_area2(ring: &Polygon) -> Result<i128, DomainError> {
    edges(ring).try_fold(0_i128, |sum, [a, b]| {
        let left = i128::from(a.x)
            .checked_mul(i128::from(b.y))
            .ok_or(DomainError::ArithmeticOverflow)?;
        let right = i128::from(b.x)
            .checked_mul(i128::from(a.y))
            .ok_or(DomainError::ArithmeticOverflow)?;
        let term = left
            .checked_sub(right)
            .ok_or(DomainError::ArithmeticOverflow)?;
        sum.checked_add(term).ok_or(DomainError::ArithmeticOverflow)
    })
}

fn exact_orientation(a: Point2, b: Point2, c: Point2) -> i8 {
    let abx = BigInt::from(b.x) - BigInt::from(a.x);
    let aby = BigInt::from(b.y) - BigInt::from(a.y);
    let acx = BigInt::from(c.x) - BigInt::from(a.x);
    let acy = BigInt::from(c.y) - BigInt::from(a.y);
    sign((abx * acy - aby * acx).cmp(&BigInt::from(0)))
}

fn integer_on_segment(p: Point2, [a, b]: [Point2; 2]) -> bool {
    exact_orientation(a, b, p) == 0
        && (min(a.x, b.x)..=max(a.x, b.x)).contains(&p.x)
        && (min(a.y, b.y)..=max(a.y, b.y)).contains(&p.y)
}

pub(super) fn exact_segment_relation(a: [Point2; 2], b: [Point2; 2]) -> SegmentRelation {
    use SegmentRelation::{CollinearOverlap, Disjoint, EndpointTouch, ProperCrossing};
    if a[0] == a[1] {
        return if integer_on_segment(a[0], b) {
            EndpointTouch
        } else {
            Disjoint
        };
    }
    if b[0] == b[1] {
        return if integer_on_segment(b[0], a) {
            EndpointTouch
        } else {
            Disjoint
        };
    }
    let o1 = exact_orientation(a[0], a[1], b[0]);
    let o2 = exact_orientation(a[0], a[1], b[1]);
    let o3 = exact_orientation(b[0], b[1], a[0]);
    let o4 = exact_orientation(b[0], b[1], a[1]);
    if o1 * o2 < 0 && o3 * o4 < 0 {
        return ProperCrossing;
    }
    if o1 == 0 && o2 == 0 && o3 == 0 && o4 == 0 {
        let (a0, a1, b0, b1) = if a[0].x != a[1].x {
            (a[0].x, a[1].x, b[0].x, b[1].x)
        } else {
            (a[0].y, a[1].y, b[0].y, b[1].y)
        };
        let lo = max(min(a0, a1), min(b0, b1));
        let hi = min(max(a0, a1), max(b0, b1));
        return match lo.cmp(&hi) {
            Ordering::Less => CollinearOverlap,
            Ordering::Equal => EndpointTouch,
            Ordering::Greater => Disjoint,
        };
    }
    if (o1 == 0 && integer_on_segment(b[0], a))
        || (o2 == 0 && integer_on_segment(b[1], a))
        || (o3 == 0 && integer_on_segment(a[0], b))
        || (o4 == 0 && integer_on_segment(a[1], b))
    {
        EndpointTouch
    } else {
        Disjoint
    }
}

pub(super) fn validate_ring(ring: &Polygon) -> Result<(), DomainError> {
    let n = ring.points.len();
    if n < 3 {
        return Err(invalid_input("a ring needs at least three vertices"));
    }
    if ring.points.iter().copied().collect::<BTreeSet<_>>().len() != n {
        return Err(invalid_input("a ring cannot repeat a vertex"));
    }
    if edges(ring).any(|[a, b]| a == b) {
        return Err(invalid_input("a ring cannot have a zero-length edge"));
    }
    if checked_signed_area2(ring)? == 0 {
        return Err(invalid_input("a ring cannot have zero signed area"));
    }
    let all_edges: Vec<_> = edges(ring).collect();
    for i in 0..n {
        for j in i + 1..n {
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            let relation = exact_segment_relation(all_edges[i], all_edges[j]);
            if (adjacent && relation != SegmentRelation::EndpointTouch)
                || (!adjacent && relation != SegmentRelation::Disjoint)
            {
                return Err(invalid_input(
                    "ring edges intersect beyond their shared endpoint",
                ));
            }
        }
    }
    Ok(())
}

fn rational(value: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RationalPoint {
    x: BigRational,
    y: BigRational,
}

impl From<Point2> for RationalPoint {
    fn from(point: Point2) -> Self {
        Self {
            x: rational(point.x),
            y: rational(point.y),
        }
    }
}

impl RationalPoint {
    fn sub(&self, other: &Self) -> Self {
        Self {
            x: &self.x - &other.x,
            y: &self.y - &other.y,
        }
    }

    fn along(&self, direction: &Self, t: &BigRational) -> Self {
        Self {
            x: &self.x + &direction.x * t,
            y: &self.y + &direction.y * t,
        }
    }
}

fn cross(a: &RationalPoint, b: &RationalPoint) -> BigRational {
    &a.x * &b.y - &a.y * &b.x
}

fn side(a: &RationalPoint, b: &RationalPoint, p: &RationalPoint) -> BigRational {
    cross(&b.sub(a), &p.sub(a))
}

fn exact_point_on_segment(p: &RationalPoint, a: &RationalPoint, b: &RationalPoint) -> bool {
    side(a, b, p) == rational(0)
        && p.x >= min(&a.x, &b.x).clone()
        && p.x <= max(&a.x, &b.x).clone()
        && p.y >= min(&a.y, &b.y).clone()
        && p.y <= max(&a.y, &b.y).clone()
}

fn on_ring(p: &RationalPoint, ring: &Polygon) -> bool {
    edges(ring).any(|[a, b]| exact_point_on_segment(p, &a.into(), &b.into()))
}

// Called after the boundary check. Horizontal ray intersections use an exact
// half-open Y interval so a ray through a vertex is counted just once.
fn inside_ring(p: &RationalPoint, ring: &Polygon) -> bool {
    let mut inside = false;
    for [a, b] in edges(ring) {
        let a = RationalPoint::from(a);
        let b = RationalPoint::from(b);
        if (a.y > p.y) != (b.y > p.y) {
            let x = &a.x + (&p.y - &a.y) * (&b.x - &a.x) / (&b.y - &a.y);
            if x > p.x {
                inside = !inside;
            }
        }
    }
    inside
}

fn exact_point_location(p: &RationalPoint, polygons: &[ExPolygon]) -> PointLocation {
    if polygons
        .iter()
        .any(|poly| on_ring(p, &poly.contour) || poly.holes.iter().any(|hole| on_ring(p, hole)))
    {
        return PointLocation::Boundary;
    }
    if polygons.iter().any(|poly| {
        inside_ring(p, &poly.contour) && !poly.holes.iter().any(|hole| inside_ring(p, hole))
    }) {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}

pub(super) fn point_in_region(point: Point2, polygons: &[ExPolygon]) -> PointLocation {
    exact_point_location(&point.into(), polygons)
}

// The query segment is nonzero. Collinear overlap contributes both clipped
// endpoints; a point-edge contributes a single parameter if it is on the query.
fn exact_segment_intersection_params(query: [Point2; 2], edge: [Point2; 2]) -> Vec<BigRational> {
    let p = RationalPoint::from(query[0]);
    let r = RationalPoint::from(query[1]).sub(&p);
    let q = RationalPoint::from(edge[0]);
    let s = RationalPoint::from(edge[1]).sub(&q);
    let qp = q.sub(&p);
    let denominator = cross(&r, &s);
    let zero = rational(0);
    let one = rational(1);
    if denominator != zero {
        let t = cross(&qp, &s) / &denominator;
        let u = cross(&qp, &r) / denominator;
        if zero <= t && t <= one && zero <= u && u <= one {
            return vec![t];
        }
        return Vec::new();
    }
    if cross(&qp, &r) != zero {
        return Vec::new();
    }
    let t0;
    let t1;
    if r.x != zero {
        t0 = (&q.x - &p.x) / &r.x;
        t1 = (rational(edge[1].x) - &p.x) / &r.x;
    } else {
        t0 = (&q.y - &p.y) / &r.y;
        t1 = (rational(edge[1].y) - &p.y) / &r.y;
    }
    let lo = max(min(t0.clone(), t1.clone()), zero);
    let hi = min(max(t0, t1), one);
    if lo <= hi {
        vec![lo, hi]
    } else {
        Vec::new()
    }
}

pub(super) fn segment_in_region(
    segment: [Point2; 2],
    polygons: &[ExPolygon],
    boundary: BoundaryRule,
) -> bool {
    let start = RationalPoint::from(segment[0]);
    let direction = RationalPoint::from(segment[1]).sub(&start);
    let accepted = |p: &RationalPoint| {
        matches!(
            (boundary, exact_point_location(p, polygons)),
            (_, PointLocation::Inside) | (BoundaryRule::Inclusive, PointLocation::Boundary)
        )
    };
    if segment[0] == segment[1] {
        return accepted(&start);
    }
    let mut params = vec![rational(0), rational(1)];
    for poly in polygons {
        for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
            for edge in edges(ring) {
                params.extend(exact_segment_intersection_params(segment, edge));
            }
        }
    }
    params.sort();
    params.dedup();
    if params
        .iter()
        .any(|t| !accepted(&start.along(&direction, t)))
    {
        return false;
    }
    params.windows(2).all(|pair| {
        let midpoint = (&pair[0] + &pair[1]) / rational(2);
        accepted(&start.along(&direction, &midpoint))
    })
}

fn rings_intersect(a: &Polygon, b: &Polygon) -> bool {
    edges(a)
        .any(|ea| edges(b).any(|eb| exact_segment_relation(ea, eb) != SegmentRelation::Disjoint))
}

fn ring_point_location(point: Point2, ring: &Polygon) -> PointLocation {
    let p = RationalPoint::from(point);
    if on_ring(&p, ring) {
        PointLocation::Boundary
    } else if inside_ring(&p, ring) {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}

fn disjoint_closures(a: &Polygon, b: &Polygon) -> bool {
    !rings_intersect(a, b)
        && ring_point_location(a.points[0], b) == PointLocation::Outside
        && ring_point_location(b.points[0], a) == PointLocation::Outside
}

fn validate_domain(domain: &GeometryDomain) -> Result<(), DomainError> {
    let bounds = &domain.bounds;
    if !(1..=1_000_000).contains(&bounds.max_abs_units) {
        return Err(invalid_bounds("max_abs_units must be in 1..=1,000,000"));
    }
    // A portable count bound also makes the checked-area proof above explicit.
    if [bounds.max_vertices, bounds.max_holes, bounds.max_components]
        .into_iter()
        .any(|value| u32::try_from(value).is_err())
    {
        return Err(invalid_bounds("geometry count bound exceeds u32"));
    }
    if domain.class == TopologyClass::Empty {
        if domain.min_components != 0 || domain.min_holes != 0 {
            return Err(invalid_bounds("empty geometry requires zero minima"));
        }
        return Ok(());
    }
    if domain.min_components == 0
        || bounds.max_vertices == 0
        || bounds.max_holes == 0
        || bounds.max_components == 0
        || (domain.class == TopologyClass::Convex && domain.min_holes > 0)
        || (domain.class == TopologyClass::Thin && bounds.max_abs_units < 8)
    {
        return Err(invalid_bounds(
            "inconsistent nonempty geometry bounds or minima",
        ));
    }
    let components = max(
        domain.min_components,
        if domain.class == TopologyClass::Disconnected {
            2
        } else {
            1
        },
    );
    let holes = max(
        domain.min_holes,
        usize::from(domain.class == TopologyClass::Holed),
    );
    if components > bounds.max_components || holes > bounds.max_holes {
        return Err(invalid_bounds("topology minima exceed count bounds"));
    }
    let min_vertices = components.checked_add(holes).and_then(|n| n.checked_mul(3));
    // All simple polygons with at most five vertices are star-shaped: a
    // triangulated pentagon's two noncrossing diagonals share an endpoint,
    // which belongs to every triangle. A non-star ring therefore needs six.
    let extra_vertices = match domain.class {
        TopologyClass::NonStarShaped => 3,
        TopologyClass::Concave | TopologyClass::Thin | TopologyClass::NearDegenerate => 1,
        _ => 0,
    };
    let min_vertices = min_vertices.and_then(|n| n.checked_add(extra_vertices));
    let grid_width = 2 * i128::from(bounds.max_abs_units) + 1;
    let grid_points = grid_width * grid_width;
    if min_vertices.is_none_or(|n| {
        n > bounds.max_vertices || !i128::try_from(n).is_ok_and(|count| count <= grid_points)
    }) {
        return Err(invalid_bounds(
            "vertex bound cannot hold the required rings and witness",
        ));
    }
    Ok(())
}

// Squared distance from a point to a closed integer segment. Projection is
// clamped to its endpoints; the interior perpendicular case is cross^2/|d|^2.
fn point_segment_distance_sq(p: Point2, [a, b]: [Point2; 2]) -> BigRational {
    let dx = BigInt::from(b.x) - BigInt::from(a.x);
    let dy = BigInt::from(b.y) - BigInt::from(a.y);
    let px = BigInt::from(p.x) - BigInt::from(a.x);
    let py = BigInt::from(p.y) - BigInt::from(a.y);
    let length_sq = &dx * &dx + &dy * &dy;
    let projection = &px * &dx + &py * &dy;
    if projection <= BigInt::from(0) || length_sq == BigInt::from(0) {
        return BigRational::from_integer(&px * &px + &py * &py);
    }
    if projection >= length_sq {
        let bx = BigInt::from(p.x) - BigInt::from(b.x);
        let by = BigInt::from(p.y) - BigInt::from(b.y);
        return BigRational::from_integer(&bx * &bx + &by * &by);
    }
    let cross = px * dy - py * dx;
    BigRational::new(&cross * &cross, length_sq)
}

fn exact_segment_distance_sq(a: [Point2; 2], b: [Point2; 2]) -> BigRational {
    if exact_segment_relation(a, b) != SegmentRelation::Disjoint {
        return rational(0);
    }
    [
        point_segment_distance_sq(a[0], b),
        point_segment_distance_sq(a[1], b),
        point_segment_distance_sq(b[0], a),
        point_segment_distance_sq(b[1], a),
    ]
    .into_iter()
    .min()
    .expect("four endpoint distances")
}

fn edge_clearance_witness(ring: &Polygon, near_degenerate: bool) -> bool {
    let min_x = ring.points.iter().map(|p| p.x).min().expect("valid ring");
    let max_x = ring.points.iter().map(|p| p.x).max().expect("valid ring");
    let min_y = ring.points.iter().map(|p| p.y).min().expect("valid ring");
    let max_y = ring.points.iter().map(|p| p.y).max().expect("valid ring");
    let min_dim = min(
        i128::from(max_x) - i128::from(min_x),
        i128::from(max_y) - i128::from(min_y),
    );
    if !near_degenerate && min_dim < 16 {
        return false;
    }
    let threshold = if near_degenerate {
        rational(1)
    } else {
        // min_dim >= 16, so min_dim / 16 is already at least one. Keep
        // fractional units: flooring here rejects genuinely thin witnesses.
        BigRational::new(BigInt::from(min_dim), BigInt::from(16))
    };
    let threshold_sq = &threshold * &threshold;
    let all_edges: Vec<_> = edges(ring).collect();
    let n = all_edges.len();
    for i in 0..n {
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let distance_sq = exact_segment_distance_sq(all_edges[i], all_edges[j]);
            if (near_degenerate && distance_sq == threshold_sq)
                || (!near_degenerate && distance_sq <= threshold_sq)
            {
                return true;
            }
        }
    }
    false
}

// Clip the bounding rectangle by every directed edge's CLOSED left half-plane.
// Preserve point and line kernels: only an empty vertex list means empty kernel.
fn exact_kernel_is_empty(ring: &Polygon) -> bool {
    let min_x = ring.points.iter().map(|p| p.x).min().expect("valid ring");
    let max_x = ring.points.iter().map(|p| p.x).max().expect("valid ring");
    let min_y = ring.points.iter().map(|p| p.y).min().expect("valid ring");
    let max_y = ring.points.iter().map(|p| p.y).max().expect("valid ring");
    let mut clipped: Vec<RationalPoint> = [
        Point2 { x: min_x, y: min_y },
        Point2 { x: max_x, y: min_y },
        Point2 { x: max_x, y: max_y },
        Point2 { x: min_x, y: max_y },
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    let zero = rational(0);
    for [a, b] in edges(ring) {
        if clipped.is_empty() {
            return true;
        }
        let a = RationalPoint::from(a);
        let b = RationalPoint::from(b);
        let mut next = Vec::new();
        let mut previous = clipped.last().expect("nonempty clip").clone();
        let mut previous_side = side(&a, &b, &previous);
        for current in &clipped {
            let current_side = side(&a, &b, current);
            if (previous_side >= zero) != (current_side >= zero) {
                let t = &previous_side / (&previous_side - &current_side);
                next.push(previous.along(&current.sub(&previous), &t));
            }
            if current_side >= zero {
                next.push(current.clone());
            }
            previous = current.clone();
            previous_side = current_side;
        }
        next.dedup();
        if next.len() > 1 && next.first() == next.last() {
            next.pop();
        }
        clipped = next;
    }
    clipped.is_empty()
}

fn has_reflex_turn(ring: &Polygon) -> bool {
    let n = ring.points.len();
    (0..n).any(|i| {
        exact_orientation(
            ring.points[i],
            ring.points[(i + 1) % n],
            ring.points[(i + 2) % n],
        ) < 0
    })
}

pub(super) fn validate_geometry(
    case: &GeometryCase,
    domain: &GeometryDomain,
) -> Result<(), DomainError> {
    validate_domain(domain)?;
    if domain.class == TopologyClass::Empty {
        return if case.polygons.is_empty() {
            Ok(())
        } else {
            Err(invalid_input("empty geometry has components"))
        };
    }
    if case.polygons.len() < domain.min_components
        || case.polygons.len() > domain.bounds.max_components
    {
        return Err(invalid_input("component count is outside the domain"));
    }
    let mut hole_count = 0_usize;
    let mut vertex_count = 0_usize;
    for poly in &case.polygons {
        hole_count = hole_count
            .checked_add(poly.holes.len())
            .ok_or_else(|| invalid_input("too many holes"))?;
        for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
            vertex_count = vertex_count
                .checked_add(ring.points.len())
                .ok_or_else(|| invalid_input("too many vertices"))?;
            if ring.points.iter().any(|p| {
                i128::from(p.x).abs() > i128::from(domain.bounds.max_abs_units)
                    || i128::from(p.y).abs() > i128::from(domain.bounds.max_abs_units)
            }) {
                return Err(invalid_input("coordinate exceeds the domain bound"));
            }
            validate_ring(ring)?;
        }
        if checked_signed_area2(&poly.contour)? <= 0 {
            return Err(invalid_input("outer winding must be counterclockwise"));
        }
        for (i, hole) in poly.holes.iter().enumerate() {
            if checked_signed_area2(hole)? >= 0 {
                return Err(invalid_input("hole winding must be clockwise"));
            }
            if hole
                .points
                .iter()
                .any(|&p| ring_point_location(p, &poly.contour) != PointLocation::Inside)
                || rings_intersect(hole, &poly.contour)
            {
                return Err(invalid_input("hole must be strictly inside its outer ring"));
            }
            if poly.holes[..i]
                .iter()
                .any(|other| !disjoint_closures(hole, other))
            {
                return Err(invalid_input("hole closures must be disjoint"));
            }
        }
    }
    if hole_count < domain.min_holes
        || hole_count > domain.bounds.max_holes
        || vertex_count > domain.bounds.max_vertices
    {
        return Err(invalid_input("hole or vertex count is outside the domain"));
    }
    for (i, poly) in case.polygons.iter().enumerate() {
        if case.polygons[..i]
            .iter()
            .any(|other| !disjoint_closures(&poly.contour, &other.contour))
        {
            return Err(invalid_input("outer component closures must be disjoint"));
        }
    }
    let witness = match domain.class {
        TopologyClass::Empty => unreachable!("empty case already handled"),
        TopologyClass::Convex => case.polygons.iter().all(|p| !has_reflex_turn(&p.contour)),
        TopologyClass::Concave => case.polygons.iter().any(|p| has_reflex_turn(&p.contour)),
        TopologyClass::NonStarShaped => case
            .polygons
            .iter()
            .any(|p| exact_kernel_is_empty(&p.contour)),
        TopologyClass::Holed => hole_count >= 1,
        TopologyClass::Disconnected => case.polygons.len() >= 2,
        TopologyClass::Thin => case
            .polygons
            .iter()
            .any(|p| edge_clearance_witness(&p.contour, false)),
        TopologyClass::NearDegenerate => case
            .polygons
            .iter()
            .any(|p| edge_clearance_witness(&p.contour, true)),
    };
    if !witness {
        return Err(invalid_input(
            "geometry lacks its requested topology witness",
        ));
    }
    Ok(())
}
