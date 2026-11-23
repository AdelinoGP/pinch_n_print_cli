//! Construction-first integer geometry and witness-preserving value trees.

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{Reason, TestRunner};
use slicer_ir::{ExPolygon, Point2, Polygon};

use super::{
    DomainError, GeometryBounds, GeometryCase, GeometryComplexity, GeometryDomain,
    GeometryStrategy, GeometryValueTree, InvalidGeometryCase, InvalidGeometryKind, TopologyClass,
};
use crate::test_support::property_testing::oracles::validate_geometry;

// Bound allocation and the quadratic independent ring checks. A larger maximum
// does not force that many rings, but required minima must fit this catalog.
const CATALOG_RING_CAP: usize = 64;
const MAX_UNITS: i64 = 1_000_000;

#[derive(Debug, Clone)]
pub(super) struct Params {
    components: usize,
    holes: usize,
    width: i64,
    height: i64,
    x: i64,
    y: i64,
    extra_vertices: usize,
    chamfers: usize,
    triangle: bool,
}

fn invalid_bounds(message: &str) -> DomainError {
    DomainError::InvalidBounds(message.into())
}

fn minima(domain: &GeometryDomain) -> (usize, usize) {
    (
        domain
            .min_components
            .max(if domain.class == TopologyClass::Disconnected {
                2
            } else {
                1
            }),
        domain
            .min_holes
            .max(usize::from(domain.class == TopologyClass::Holed)),
    )
}

fn primary_vertices(class: TopologyClass) -> usize {
    match class {
        TopologyClass::Empty => 0,
        TopologyClass::Convex => 3,
        TopologyClass::Concave => 6,
        TopologyClass::NonStarShaped | TopologyClass::Thin | TopologyClass::NearDegenerate => 8,
        TopologyClass::Holed | TopologyClass::Disconnected => 4,
    }
}

// Returns the minimum primary width, height and total vertices of this layout.
fn layout(domain: &GeometryDomain, components: usize, holes: usize) -> Option<(i64, i64, usize)> {
    let (mut width, mut height) = match domain.class {
        TopologyClass::Empty => (0, 0),
        TopologyClass::Convex | TopologyClass::Disconnected => (1, 1),
        TopologyClass::Concave => (2, 2),
        TopologyClass::NonStarShaped | TopologyClass::NearDegenerate => (3, 2),
        TopologyClass::Thin => (16, 16),
        TopologyClass::Holed => (3, 3),
    };
    if holes != 0 {
        width = width.max(i64::try_from(holes.checked_mul(2)?.checked_add(1)?).ok()?);
        // Notches leave a bottom strip [0,3] to hold the [1,2] holes.
        height = height.max(
            if matches!(
                domain.class,
                TopologyClass::Concave
                    | TopologyClass::NonStarShaped
                    | TopologyClass::Thin
                    | TopologyClass::NearDegenerate
            ) {
                4
            } else {
                3
            },
        );
    }
    let vertices = primary_vertices(domain.class)
        .checked_add(components.checked_sub(1)?.checked_mul(4)?)?
        .checked_add(holes.checked_mul(4)?)?;
    let span =
        width.checked_add(i64::try_from(components.checked_sub(1)?.checked_mul(2)?).ok()?)?;
    let available = domain.bounds.max_abs_units.checked_mul(2)?;
    (vertices <= domain.bounds.max_vertices && span <= available && height <= available)
        .then_some((width, height, vertices))
}

fn choose(raw: u32, maximum: usize) -> usize {
    (raw as usize) % (maximum + 1)
}

fn parameters(domain: &GeometryDomain, raw: [u32; 8]) -> Params {
    if domain.class == TopologyClass::Empty {
        return Params {
            components: 0,
            holes: 0,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            extra_vertices: 0,
            chamfers: 0,
            triangle: false,
        };
    }
    let (min_components, min_holes) = minima(domain);
    let max_components = (min_components..=domain.bounds.max_components.min(CATALOG_RING_CAP))
        .take_while(|&c| layout(domain, c, min_holes).is_some())
        .last()
        .expect("minimum layout was checked");
    let components = min_components + choose(raw[0], max_components - min_components);
    let holes_cap = if domain.class == TopologyClass::Convex {
        0
    } else {
        domain.bounds.max_holes.min(CATALOG_RING_CAP)
    };
    let max_holes = (min_holes..=holes_cap)
        .take_while(|&h| layout(domain, components, h).is_some())
        .last()
        .expect("minimum layout was checked");
    let holes = min_holes + choose(raw[1], max_holes - min_holes);
    let (min_width, min_height, minimum_vertices) =
        layout(domain, components, holes).expect("selected layout fits");
    let span = 2 * domain.bounds.max_abs_units;
    let extra_components = i64::try_from(2 * (components - 1)).expect("catalog cap");
    let width_room = (span - extra_components - min_width).min(32);
    let height_room = (span - min_height).min(32);
    let width = min_width + i64::from(raw[2] % (u32::try_from(width_room).unwrap() + 1));
    let height = min_height + i64::from(raw[3] % (u32::try_from(height_room).unwrap() + 1));
    let triangle =
        domain.class == TopologyClass::Convex && minimum_vertices == domain.bounds.max_vertices;
    let base_vertices =
        minimum_vertices + usize::from(domain.class == TopologyClass::Convex && !triangle);
    let remaining = domain.bounds.max_vertices - base_vertices;
    let chamfers =
        if domain.class == TopologyClass::Convex && !triangle && width >= 3 && height >= 2 {
            choose(raw[5], remaining.min(2))
        } else {
            0
        };
    let extra_vertices = if width >= 3 && !triangle {
        choose(raw[6], (remaining - chamfers).min(2))
    } else {
        0
    };
    let x_room = u32::try_from(span - width - extra_components).expect("bounded span");
    let y_room = u32::try_from(span - height).expect("bounded span");
    let x = -domain.bounds.max_abs_units + i64::from(raw[4] % (x_room + 1));
    let y = -domain.bounds.max_abs_units + i64::from(raw[7] % (y_room + 1));
    Params {
        components,
        holes,
        width,
        height,
        x,
        y,
        extra_vertices,
        chamfers,
        triangle,
    }
}

pub(super) fn geometry_cases(domain: GeometryDomain) -> Result<GeometryStrategy, DomainError> {
    // The oracle's validation distinguishes invalid domain bounds from an
    // empty case that merely lacks the requested nonempty topology.
    if let Err(DomainError::InvalidBounds(message)) =
        validate_geometry(&GeometryCase { polygons: vec![] }, &domain)
    {
        return Err(DomainError::InvalidBounds(message));
    }
    if domain.class != TopologyClass::Empty {
        let (components, holes) = minima(&domain);
        if components > CATALOG_RING_CAP
            || holes > CATALOG_RING_CAP
            || layout(&domain, components, holes).is_none()
        {
            return Err(invalid_bounds(
                "required catalog geometry cannot fit the bounds",
            ));
        }
    }
    let parameter_domain = domain.clone();
    let params = any::<[u32; 8]>()
        .prop_map(move |raw| parameters(&parameter_domain, raw))
        .boxed();
    Ok(GeometryStrategy { domain, params })
}

fn polygon(points: Vec<(i64, i64)>, x: i64, y: i64) -> Polygon {
    Polygon {
        points: points
            .into_iter()
            .map(|(px, py)| Point2 {
                x: x + px,
                y: y + py,
            })
            .collect(),
    }
}

fn rectangle(x: i64, y: i64, width: i64, height: i64) -> Polygon {
    polygon(vec![(0, 0), (width, 0), (width, height), (0, height)], x, y)
}

fn make_case(domain: &GeometryDomain, p: &Params) -> GeometryCase {
    if p.components == 0 {
        return GeometryCase { polygons: vec![] };
    }
    let (w, h) = (p.width, p.height);
    let mut points = vec![(0, 0)];
    if p.triangle {
        points.extend([(w, 0), (0, h)]);
    } else {
        for i in 1..=p.extra_vertices {
            points.push((i64::try_from(i).expect("at most two"), 0));
        }
        points.push((w, 0));
        match domain.class {
            TopologyClass::Convex if p.chamfers > 0 => {
                points.extend([(w, h - 1), (w - 1, h)]);
                if p.chamfers == 2 {
                    points.extend([(1, h), (0, h - 1)]);
                } else {
                    points.push((0, h));
                }
            }
            TopologyClass::Concave => {
                let bottom = if p.holes > 0 { 3 } else { 1 };
                points.extend([(w, bottom), (w - 1, bottom), (w - 1, h), (0, h)]);
            }
            TopologyClass::NonStarShaped | TopologyClass::Thin | TopologyClass::NearDegenerate => {
                let left = (w - 1) / 2;
                let bottom = if p.holes > 0 { 3 } else { 1 };
                // Right wall is downward and imposes x >= left+1; left wall
                // is upward and imposes x <= left. Their kernel is empty.
                points.extend([
                    (w, h),
                    (left + 1, h),
                    (left + 1, bottom),
                    (left, bottom),
                    (left, h),
                    (0, h),
                ]);
            }
            _ => points.extend([(w, h), (0, h)]),
        }
    }
    let holes = (0..p.holes)
        .map(|index| {
            let hx = 1 + i64::try_from(index * 2).expect("catalog cap");
            // Clockwise unit squares; adjacent closures have one unit clearance.
            polygon(vec![(hx, 1), (hx, 2), (hx + 1, 2), (hx + 1, 1)], p.x, p.y)
        })
        .collect();
    let mut polygons = vec![ExPolygon {
        contour: polygon(points, p.x, p.y),
        holes,
    }];
    for index in 1..p.components {
        let x = p.x + w + 1 + i64::try_from((index - 1) * 2).expect("catalog cap");
        polygons.push(ExPolygon {
            contour: rectangle(x, p.y, 1, 1),
            holes: vec![],
        });
    }
    GeometryCase { polygons }
}

impl Strategy for GeometryStrategy {
    type Tree = GeometryValueTree;
    type Value = GeometryCase;

    fn new_tree(&self, runner: &mut TestRunner) -> Result<Self::Tree, Reason> {
        let params = self.params.new_tree(runner)?.current();
        let current = make_case(&self.domain, &params);
        validate_geometry(&current, &self.domain).map_err(|error| {
            Reason::from(format!("constructed invalid geometry: {error}; {params:?}"))
        })?;
        Ok(GeometryValueTree {
            domain: self.domain.clone(),
            current,
            history: Vec::new(),
            rejected: Vec::new(),
        })
    }
}

pub(super) fn geometry_complexity(case: &GeometryCase) -> Result<GeometryComplexity, DomainError> {
    let mut holes = 0_usize;
    let mut vertices = 0_usize;
    let mut sum = 0_u128;
    for poly in &case.polygons {
        holes = holes
            .checked_add(poly.holes.len())
            .ok_or(DomainError::ArithmeticOverflow)?;
        for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
            vertices = vertices
                .checked_add(ring.points.len())
                .ok_or(DomainError::ArithmeticOverflow)?;
            for p in &ring.points {
                sum = sum
                    .checked_add(i128::from(p.x).unsigned_abs())
                    .and_then(|s| s.checked_add(i128::from(p.y).unsigned_abs()))
                    .ok_or(DomainError::ArithmeticOverflow)?;
            }
        }
    }
    Ok((case.polygons.len(), holes, vertices, sum))
}

fn for_each_point_mut(case: &mut GeometryCase, mut f: impl FnMut(&mut Point2)) {
    for poly in &mut case.polygons {
        for ring in std::iter::once(&mut poly.contour).chain(poly.holes.iter_mut()) {
            for point in &mut ring.points {
                f(point);
            }
        }
    }
}

fn coordinates(case: &GeometryCase, x_axis: bool) -> Vec<i64> {
    case.polygons
        .iter()
        .flat_map(|p| std::iter::once(&p.contour).chain(p.holes.iter()))
        .flat_map(|r| r.points.iter())
        .map(|p| if x_axis { p.x } else { p.y })
        .collect()
}

impl GeometryValueTree {
    // Candidate generation is lazy: the bound limits actual validation attempts,
    // rather than allocating an unbounded list of cloned polygon sets.
    fn consider(&mut self, candidate: GeometryCase, attempts: &mut u32) -> Option<bool> {
        if *attempts >= self.domain.bounds.max_shrink_candidates {
            eprintln!(
                "candidate_limit_reached {:?}",
                geometry_complexity(&self.current).expect("bounded catalog complexity fits")
            );
            return Some(false);
        }
        *attempts += 1;
        if self
            .rejected
            .iter()
            .any(|(from, to)| *from == self.current && *to == candidate)
        {
            return None;
        }
        if geometry_complexity(&candidate).ok()? >= geometry_complexity(&self.current).ok()?
            || validate_geometry(&candidate, &self.domain).is_err()
        {
            return None;
        }
        self.history
            .push(std::mem::replace(&mut self.current, candidate));
        Some(true)
    }
}

impl ValueTree for GeometryValueTree {
    type Value = GeometryCase;

    fn current(&self) -> GeometryCase {
        self.current.clone()
    }

    fn simplify(&mut self) -> bool {
        let mut attempts = 0;
        // Reverse order makes the last removable vertex the first candidate.
        for component in (0..self.current.polygons.len()).rev() {
            let ring_count = 1 + self.current.polygons[component].holes.len();
            for ring in (0..ring_count).rev() {
                let length = if ring == 0 {
                    self.current.polygons[component].contour.points.len()
                } else {
                    self.current.polygons[component].holes[ring - 1]
                        .points
                        .len()
                };
                if length <= 3 {
                    continue;
                }
                for vertex in (0..length).rev() {
                    let points = if ring == 0 {
                        &self.current.polygons[component].contour.points
                    } else {
                        &self.current.polygons[component].holes[ring - 1].points
                    };
                    let (a, b, c) = (
                        points[(vertex + length - 1) % length],
                        points[vertex],
                        points[(vertex + 1) % length],
                    );
                    let collinear = i128::from(b.x - a.x) * i128::from(c.y - b.y)
                        == i128::from(b.y - a.y) * i128::from(c.x - b.x);
                    // Collinear subdivisions are optional in every family;
                    // convex outers may also remove a chamfer or corner.
                    if !collinear && !(self.domain.class == TopologyClass::Convex && ring == 0) {
                        continue;
                    }
                    let mut candidate = self.current.clone();
                    let selected = if ring == 0 {
                        &mut candidate.polygons[component].contour
                    } else {
                        &mut candidate.polygons[component].holes[ring - 1]
                    };
                    selected.points.remove(vertex);
                    if let Some(result) = self.consider(candidate, &mut attempts) {
                        return result;
                    }
                }
            }
        }
        let holes = self
            .current
            .polygons
            .iter()
            .map(|p| p.holes.len())
            .sum::<usize>();
        let required_holes = self
            .domain
            .min_holes
            .max(usize::from(self.domain.class == TopologyClass::Holed));
        if holes > required_holes {
            for component in (0..self.current.polygons.len()).rev() {
                if !self.current.polygons[component].holes.is_empty() {
                    let mut candidate = self.current.clone();
                    candidate.polygons[component].holes.pop();
                    if let Some(result) = self.consider(candidate, &mut attempts) {
                        return result;
                    }
                    break;
                }
            }
        }
        let required_components = self
            .domain
            .min_components
            .max(usize::from(self.domain.class == TopologyClass::Disconnected) * 2);
        if self.current.polygons.len() > required_components {
            let mut candidate = self.current.clone();
            candidate.polygons.pop();
            if let Some(result) = self.consider(candidate, &mut attempts) {
                return result;
            }
        }
        for axis in [true, false] {
            let mut values = coordinates(&self.current, axis);
            values.sort_unstable();
            values.dedup();
            for pair in values.windows(2).rev() {
                let gap = pair[1] - pair[0];
                if gap <= 1 {
                    continue;
                }
                let mut candidate = self.current.clone();
                for_each_point_mut(&mut candidate, |p| {
                    let coordinate = if axis { &mut p.x } else { &mut p.y };
                    if *coordinate >= pair[1] {
                        *coordinate -= gap / 2;
                    }
                });
                if let Some(result) = self.consider(candidate, &mut attempts) {
                    return result;
                }
            }
        }
        for axis in [true, false] {
            let mut values = coordinates(&self.current, axis);
            if values.is_empty() {
                continue;
            }
            values.sort_unstable();
            let median = values[values.len() / 2];
            // Clamp translation so no point can leave the coordinate bound.
            let lower = -self.domain.bounds.max_abs_units - values[0];
            let upper = self.domain.bounds.max_abs_units - values[values.len() - 1];
            let shift = (-median).clamp(lower, upper);
            if shift == 0 {
                continue;
            }
            let mut candidate = self.current.clone();
            for_each_point_mut(&mut candidate, |p| {
                if axis {
                    p.x += shift
                } else {
                    p.y += shift
                }
            });
            if let Some(result) = self.consider(candidate, &mut attempts) {
                return result;
            }
        }
        false
    }

    fn complicate(&mut self) -> bool {
        if let Some(previous) = self.history.pop() {
            let rejected = std::mem::replace(&mut self.current, previous);
            self.rejected.push((self.current.clone(), rejected));
            true
        } else {
            false
        }
    }
}

fn check_units(bounds: &GeometryBounds) -> Result<(), DomainError> {
    if !(1..=MAX_UNITS).contains(&bounds.max_abs_units) {
        Err(invalid_bounds("max_abs_units must be in 1..=1,000,000"))
    } else {
        Ok(())
    }
}

pub(super) fn line_segments(
    bounds: GeometryBounds,
) -> Result<BoxedStrategy<[Point2; 2]>, DomainError> {
    check_units(&bounds)?;
    let m = bounds.max_abs_units;
    let arbitrary = (-m..=m, -m..=m, -m..=m, -m..=m)
        .prop_map(|(x, y, u, v)| [Point2 { x, y }, Point2 { x: u, y: v }]);
    let witnesses = proptest::sample::select(vec![
        [Point2 { x: 0, y: 0 }, Point2 { x: 0, y: 0 }],
        [Point2 { x: -m, y: 0 }, Point2 { x: m, y: 0 }],
        [Point2 { x: 0, y: -m }, Point2 { x: 0, y: m }],
        [Point2 { x: -m, y: -m }, Point2 { x: m, y: m }],
        [Point2 { x: -m, y: m }, Point2 { x: m, y: -m }],
        [Point2 { x: 0, y: 0 }, Point2 { x: m, y: 0 }],
    ]);
    Ok(prop_oneof![4 => arbitrary, 2 => witnesses].boxed())
}

pub(super) fn invalid_geometry_cases(
    kind: InvalidGeometryKind,
    bounds: GeometryBounds,
) -> Result<BoxedStrategy<InvalidGeometryCase>, DomainError> {
    check_units(&bounds)?;
    let (width, height, vertices, holes) = match kind {
        InvalidGeometryKind::Crossing => (2, 2, 4, 0),
        InvalidGeometryKind::DuplicateVertex => (2, 2, 5, 0),
        InvalidGeometryKind::Collapsed => (2, 0, 3, 0),
        InvalidGeometryKind::TouchingHole => (3, 3, 8, 1),
        InvalidGeometryKind::MisnestedHole => (3, 1, 8, 1),
    };
    let span = 2 * bounds.max_abs_units;
    if width > span
        || height > span
        || bounds.max_vertices < vertices
        || bounds.max_holes < holes
        || bounds.max_components < 1
        || [bounds.max_vertices, bounds.max_holes, bounds.max_components]
            .into_iter()
            .any(|n| u32::try_from(n).is_err())
    {
        return Err(invalid_bounds("malformed witness cannot fit the bounds"));
    }
    let m = bounds.max_abs_units;
    Ok((0..=span - width, 0..=span - height)
        .prop_map(move |(dx, dy)| {
            let (x, y) = (dx - m, dy - m);
            let (contour, holes) = match kind {
                InvalidGeometryKind::Crossing => {
                    (polygon(vec![(0, 0), (2, 2), (0, 2), (2, 0)], x, y), vec![])
                }
                InvalidGeometryKind::DuplicateVertex => (
                    polygon(vec![(0, 0), (2, 0), (2, 2), (0, 2), (0, 0)], x, y),
                    vec![],
                ),
                InvalidGeometryKind::Collapsed => {
                    (polygon(vec![(0, 0), (1, 0), (2, 0)], x, y), vec![])
                }
                InvalidGeometryKind::TouchingHole => (
                    rectangle(x, y, 3, 3),
                    vec![polygon(vec![(0, 1), (0, 2), (1, 2), (1, 1)], x, y)],
                ),
                InvalidGeometryKind::MisnestedHole => (
                    rectangle(x, y, 1, 1),
                    vec![polygon(vec![(2, 0), (2, 1), (3, 1), (3, 0)], x, y)],
                ),
            };
            InvalidGeometryCase {
                polygons: vec![ExPolygon { contour, holes }],
                kind,
            }
        })
        .boxed())
}
