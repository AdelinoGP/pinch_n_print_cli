//! Typed preparation diagnostics and checked, non-normalizing geometry validation.

use slicer_ir::{ExPolygon, Point2};
use slicer_schema::preparation::PreparationProjectionError as Error;
use slicer_sdk::preparation::{
    PreparationProjectionGeometry as Geometry, PreparationProjectionScene,
};
use std::collections::{BTreeMap, BTreeSet};

/// Exact requested owner-local diagnostic views.
#[derive(Debug, Clone)]
pub struct PreparationProjectionRequest {
    views: Vec<String>,
}

impl PreparationProjectionRequest {
    /// Reject empty or duplicate identities, without changing UTF-8 names.
    pub fn new(views: Vec<String>) -> Result<Self, Error> {
        let mut seen = BTreeSet::new();
        if views
            .iter()
            .any(|view| view.is_empty() || !seen.insert(view))
        {
            return Err(Error::InvalidReference);
        }
        Ok(Self { views })
    }

    pub(crate) fn views(&self) -> &[String] {
        &self.views
    }
}

/// Immutable validated public geometry; contains no private plan authority.
#[derive(Debug, PartialEq)]
pub struct CommittedPreparationProjection {
    scene: PreparationProjectionScene,
    owner_module_id: String,
    diagnostic_bytes: u64,
    layer_z: BTreeMap<u32, f32>,
}

impl CommittedPreparationProjection {
    /// Return only the public typed diagnostic DTO.
    pub fn scene(&self) -> &PreparationProjectionScene {
        &self.scene
    }
    /// Return the frozen owning module identity.
    pub fn owner_module_id(&self) -> &str {
        &self.owner_module_id
    }
    /// Return the exact requested view identity.
    pub fn view(&self) -> &str {
        &self.scene.view
    }
    /// Return logical diagnostic bytes, excluding allocator overhead.
    pub fn diagnostic_bytes(&self) -> u64 {
        self.diagnostic_bytes
    }
    /// Return host-attached committed schedule Z, never module-authored Z.
    pub fn layer_z(&self, index: u32) -> Option<f32> {
        self.layer_z.get(&index).copied()
    }
}

/// Diagnostic counters independent of opaque piece accounting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreparationDiagnosticAccounting {
    /// Logical bytes currently in unpublished staging.
    pub staged_bytes: u64,
    /// Logical bytes retained by the committed store (not external snapshots).
    pub retained_bytes: u64,
    /// Number of accepted scene publications.
    pub published_scenes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectionFault {
    pub(crate) view: String,
    pub(crate) error: Error,
    pub(crate) field: String,
}

pub(crate) fn checked_add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::ArithmeticOverflow)
}

fn bytes(value: usize) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::ArithmeticOverflow)
}

pub(crate) fn validate_scene(
    scene: &PreparationProjectionScene,
    owner_module_id: &str,
    layers: &BTreeMap<u32, f32>,
) -> Result<CommittedPreparationProjection, ProjectionFault> {
    let fault = |error, field: String| ProjectionFault {
        view: scene.view.clone(),
        error,
        field,
    };
    let validate = || -> Result<u64, (Error, String)> {
        let reference = |field: &str| (Error::InvalidReference, field.to_owned());
        if scene.scene_id.is_empty() {
            return Err(reference("scene_id"));
        }
        if scene.view.is_empty() {
            return Err(reference("view"));
        }
        let mut classes = BTreeSet::new();
        for class in &scene.classes {
            if class.is_empty() || !classes.insert(class) {
                return Err(reference("classes"));
            }
        }
        let mut ids = BTreeSet::new();
        let mut total = 0;
        let mut string = |s: &str| -> Result<(), (Error, String)> {
            total = bytes(s.len())
                .and_then(|n| checked_add(total, n))
                .map_err(|e| (e, "diagnostic_bytes".into()))?;
            Ok(())
        };
        string(&scene.scene_id)?;
        string(&scene.view)?;
        for class in &scene.classes {
            string(class)?;
        }
        for primitive in &scene.primitives {
            let field = || format!("primitive={}", primitive.id);
            if primitive.id.is_empty() || !ids.insert(&primitive.id) {
                return Err(reference(&format!("{}.id", field())));
            }
            if !classes.contains(&primitive.class)
                || !layers.contains_key(&primitive.global_layer_index)
            {
                return Err(reference(&format!("{}.class/global_layer_index", field())));
            }
            let z = layers[&primitive.global_layer_index];
            if !z.is_finite() || z < 0.0 {
                return Err((Error::InvalidCoordinate, format!("{}.schedule_z", field())));
            }
            string(&primitive.id)?;
            string(&primitive.class)?;
            if let Some(label) = &primitive.label {
                string(label)?;
            }
        }
        for primitive in &scene.primitives {
            let n = validate_geometry(&primitive.geometry)
                .map_err(|e| (e, format!("primitive={}.geometry", primitive.id)))?;
            total = checked_add(
                total,
                checked_add(n, 4).map_err(|e| (e, "global_layer_index".into()))?,
            )
            .map_err(|e| (e, "diagnostic_bytes".into()))?;
        }
        Ok(total)
    };
    let total = validate().map_err(|(e, field)| fault(e, field))?;
    Ok(CommittedPreparationProjection {
        scene: scene.clone(),
        owner_module_id: owner_module_id.into(),
        diagnostic_bytes: total,
        // The whole committed schedule, not only primitive-referenced layers:
        // an explicitly completed empty scene still needs the real schedule Z
        // for every requested rendered layer (AC-6), and the host is the only
        // authority that attaches schedule Z.
        layer_z: layers.clone(),
    })
}

fn point(point: &Point2) -> Result<(), Error> {
    // Integer coordinates are retained unchanged; only check the float boundary.
    let (x, y) = point.to_mm();
    if x.is_finite() && y.is_finite() {
        Ok(())
    } else {
        Err(Error::InvalidCoordinate)
    }
}

fn point_bytes(points: &[Point2]) -> Result<u64, Error> {
    for p in points {
        point(p)?;
    }
    bytes(points.len())?
        .checked_mul(16)
        .ok_or(Error::ArithmeticOverflow)
}

fn validate_geometry(geometry: &Geometry) -> Result<u64, Error> {
    match geometry {
        Geometry::Point(p) => {
            point(p)?;
            Ok(17)
        }
        Geometry::Polyline(points) => {
            if points.len() < 2
                || points.first() == points.last()
                || points.windows(2).any(|w| w[0] == w[1])
            {
                return Err(Error::InvalidShape);
            }
            checked_add(1, point_bytes(points)?)
        }
        Geometry::Polygon { polygon, .. } => {
            validate_polygon(polygon)?;
            let mut n = checked_add(2, point_bytes(&polygon.contour.points)?)?;
            for hole in &polygon.holes {
                n = checked_add(n, point_bytes(&hole.points)?)?;
            }
            Ok(n)
        }
    }
}

fn cross(a: Point2, b: Point2, c: Point2) -> Result<i128, Error> {
    let sub = |a: i64, b: i64| {
        i128::from(a)
            .checked_sub(i128::from(b))
            .ok_or(Error::InvalidCoordinate)
    };
    let x = sub(b.x, a.x)?
        .checked_mul(sub(c.y, a.y)?)
        .ok_or(Error::InvalidCoordinate)?;
    let y = sub(b.y, a.y)?
        .checked_mul(sub(c.x, a.x)?)
        .ok_or(Error::InvalidCoordinate)?;
    x.checked_sub(y).ok_or(Error::InvalidCoordinate)
}

fn on_segment(a: Point2, b: Point2, p: Point2) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

fn intersects(a: Point2, b: Point2, c: Point2, d: Point2) -> Result<bool, Error> {
    let ab_c = cross(a, b, c)?;
    let ab_d = cross(a, b, d)?;
    let cd_a = cross(c, d, a)?;
    let cd_b = cross(c, d, b)?;
    Ok((ab_c == 0 && on_segment(a, b, c))
        || (ab_d == 0 && on_segment(a, b, d))
        || (cd_a == 0 && on_segment(c, d, a))
        || (cd_b == 0 && on_segment(c, d, b))
        || (ab_c.signum() * ab_d.signum() < 0 && cd_a.signum() * cd_b.signum() < 0))
}

fn edges(points: &[Point2]) -> impl Iterator<Item = (Point2, Point2)> + '_ {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
}

fn ring(points: &[Point2], ccw: bool) -> Result<(), Error> {
    if points.len() < 3 {
        return Err(Error::InvalidShape);
    }
    let mut distinct = BTreeSet::new();
    for p in points {
        point(p)?;
        if !distinct.insert((p.x, p.y)) {
            return Err(Error::InvalidShape);
        }
    }
    let mut area = 0_i128;
    for (a, b) in edges(points) {
        let term = i128::from(a.x)
            .checked_mul(i128::from(b.y))
            .and_then(|x| {
                i128::from(a.y)
                    .checked_mul(i128::from(b.x))
                    .and_then(|y| x.checked_sub(y))
            })
            .ok_or(Error::InvalidCoordinate)?;
        area = area.checked_add(term).ok_or(Error::InvalidCoordinate)?;
    }
    // Check predicates before winding so extreme-coordinate predicate failures
    // cannot be disguised as a mere winding error.
    for (i, (a, b)) in edges(points).enumerate() {
        let c = points[(i + 2) % points.len()];
        if cross(a, b, c)? == 0 && (on_segment(a, b, c) || on_segment(b, c, a)) {
            return Err(Error::InvalidShape);
        }
        for (j, (c, d)) in edges(points).enumerate().skip(i + 1) {
            if j == i + 1 || (i == 0 && j == points.len() - 1) {
                continue;
            }
            if intersects(a, b, c, d)? {
                return Err(Error::InvalidShape);
            }
        }
    }
    if area == 0 || (area > 0) != ccw {
        return Err(Error::InvalidShape);
    }
    Ok(())
}

fn rings_intersect(a: &[Point2], b: &[Point2]) -> Result<bool, Error> {
    for (p, q) in edges(a) {
        for (c, d) in edges(b) {
            if intersects(p, q, c, d)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn strictly_inside(p: Point2, polygon: &[Point2]) -> Result<bool, Error> {
    let mut winding = 0_i64;
    for (a, b) in edges(polygon) {
        let side = cross(a, b, p)?;
        if side == 0 && on_segment(a, b, p) {
            return Ok(false);
        }
        if a.y <= p.y && b.y > p.y && side > 0 {
            winding = winding.checked_add(1).ok_or(Error::InvalidCoordinate)?;
        }
        if a.y > p.y && b.y <= p.y && side < 0 {
            winding = winding.checked_sub(1).ok_or(Error::InvalidCoordinate)?;
        }
    }
    Ok(winding != 0)
}

fn validate_polygon(polygon: &ExPolygon) -> Result<(), Error> {
    let contour = &polygon.contour.points;
    ring(contour, true)?;
    for (i, hole) in polygon.holes.iter().enumerate() {
        let hole = &hole.points;
        ring(hole, false)?;
        if rings_intersect(contour, hole)? || !strictly_inside(hole[0], contour)? {
            return Err(Error::InvalidShape);
        }
        for other in &polygon.holes[..i] {
            let other = &other.points;
            if rings_intersect(other, hole)?
                || strictly_inside(hole[0], other)?
                || strictly_inside(other[0], hole)?
            {
                return Err(Error::InvalidShape);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::Polygon;
    use slicer_sdk::preparation::PreparationProjectionPrimitive;

    fn points(values: &[(i64, i64)]) -> Vec<Point2> {
        values.iter().map(|&(x, y)| Point2 { x, y }).collect()
    }
    fn scene(polygon: ExPolygon) -> PreparationProjectionScene {
        PreparationProjectionScene {
            scene_id: "final".into(),
            view: "view".into(),
            classes: vec!["domains".into()],
            // exhaustive: this fixture pins every public primitive field for validator identity checks.
            primitives: vec![PreparationProjectionPrimitive {
                id: "domain".into(),
                global_layer_index: 1,
                class: "domains".into(),
                label: Some("π".into()),
                geometry: Geometry::Polygon {
                    polygon,
                    filled: true,
                },
            }],
        }
    }
    fn valid() -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: points(&[(0, 0), (40000, 0), (40000, 40000), (0, 40000)]),
            },
            holes: vec![Polygon {
                points: points(&[
                    (10000, 10000),
                    (10000, 30000),
                    (30000, 30000),
                    (30000, 10000),
                ]),
            }],
        }
    }
    fn validate(
        value: &PreparationProjectionScene,
    ) -> Result<CommittedPreparationProjection, Error> {
        validate_scene(value, "owner", &BTreeMap::from([(0, 0.2), (1, 0.4)])).map_err(|e| e.error)
    }

    #[test]
    fn preparation_projection_valid_ring_hole_and_exact_bytes() {
        let input = scene(valid());
        let snapshot = validate(&input).expect("independently authored CCW contour/CW hole");
        assert_eq!(snapshot.scene(), &input);
        assert_eq!(snapshot.layer_z(1), Some(0.4));
        assert_eq!(snapshot.owner_module_id(), "owner");
        // Strings: final(5), view(4), domains twice(14), domain(6), π(2).
        // Geometry: 8 vertices * 16, layer index(4), tag(1), filled(1).
        assert_eq!(snapshot.diagnostic_bytes(), 165);
    }

    #[test]
    fn preparation_projection_rejects_ring_and_hole_topology_without_repair() {
        for ring in [
            vec![(0, 0), (1, 0)],
            vec![(0, 0), (1, 0), (2, 0)],
            vec![(0, 0), (0, 10), (10, 10), (10, 0)], // wrong winding
            vec![(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)], // terminal repeat
            vec![(0, 0), (10, 10), (0, 10), (10, 0)], // self-crossing
            vec![(0, 0), (10, 0), (5, 0), (10, 10), (0, 10)], // adjacent overlap
        ] {
            let value = scene(ExPolygon {
                contour: Polygon {
                    points: points(&ring),
                },
                holes: vec![],
            });
            assert_eq!(
                validate(&value).unwrap_err(),
                Error::InvalidShape,
                "{ring:?}"
            );
        }
        for hole in [
            vec![(0, 10000), (0, 30000), (30000, 30000), (30000, 10000)], // touches
            vec![
                (30000, 10000),
                (30000, 30000),
                (50000, 30000),
                (50000, 10000),
            ], // crosses
            vec![
                (50000, 10000),
                (50000, 30000),
                (60000, 30000),
                (60000, 10000),
            ], // outside
            vec![
                (10000, 10000),
                (30000, 10000),
                (30000, 30000),
                (10000, 30000),
            ], // CCW
        ] {
            let mut polygon = valid();
            polygon.holes = vec![Polygon {
                points: points(&hole),
            }];
            assert_eq!(
                validate(&scene(polygon)).unwrap_err(),
                Error::InvalidShape,
                "{hole:?}"
            );
        }
        let mut polygon = valid();
        polygon.holes.push(polygon.holes[0].clone());
        assert_eq!(validate(&scene(polygon)).unwrap_err(), Error::InvalidShape);
        let mut polygon = valid();
        polygon.holes.push(Polygon {
            points: points(&[
                (15000, 15000),
                (15000, 20000),
                (20000, 20000),
                (20000, 15000),
            ]),
        });
        assert_eq!(validate(&scene(polygon)).unwrap_err(), Error::InvalidShape);
    }

    #[test]
    fn preparation_projection_extreme_checked_predicates_and_finite_z() {
        let polygon = ExPolygon {
            contour: Polygon {
                points: points(&[
                    (i64::MIN, i64::MIN),
                    (i64::MAX, i64::MIN),
                    (i64::MAX, i64::MAX),
                    (i64::MIN, i64::MAX),
                ]),
            },
            holes: vec![],
        };
        assert_eq!(
            validate(&scene(polygon)).unwrap_err(),
            Error::InvalidCoordinate
        );
        for z in [f32::NAN, f32::INFINITY, -0.1] {
            assert_eq!(
                validate_scene(&scene(valid()), "owner", &BTreeMap::from([(1, z)]))
                    .unwrap_err()
                    .error,
                Error::InvalidCoordinate
            );
        }
        let mut value = scene(valid());
        value.primitives[0].geometry = Geometry::Point(Point2 {
            x: i64::MAX,
            y: i64::MIN,
        });
        assert!(
            validate(&value).is_ok(),
            "no arbitrary coordinate quota for points"
        );
    }

    #[test]
    fn preparation_projection_references_lines_and_exact_utf8() {
        let base = scene(valid());
        let mut value = base.clone();
        value.scene_id.clear();
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        let mut value = base.clone();
        value.classes.push("domains".into());
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        let mut value = base.clone();
        value.classes.push(String::new());
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        let mut value = base.clone();
        value.primitives.push(value.primitives[0].clone());
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        let mut value = base.clone();
        value.primitives[0].id.clear();
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        let mut value = base.clone();
        value.primitives[0].global_layer_index = 2;
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        let mut value = base.clone();
        value.primitives[0].class = "Domains".into();
        assert_eq!(validate(&value).unwrap_err(), Error::InvalidReference);
        for line in [
            vec![],
            vec![(0, 0)],
            vec![(0, 0), (0, 0), (1, 1)],
            vec![(0, 0), (1, 1), (0, 0)],
        ] {
            let mut value = base.clone();
            value.primitives[0].geometry = Geometry::Polyline(points(&line));
            assert_eq!(validate(&value).unwrap_err(), Error::InvalidShape);
        }
        let mut value = base;
        value.primitives[0].geometry = Geometry::Polyline(points(&[(0, 0), (20000, 0)]));
        assert!(validate(&value).is_ok());
        assert_eq!(
            PreparationProjectionRequest::new(vec!["view".into(), "view".into()]).unwrap_err(),
            Error::InvalidReference
        );
        assert_eq!(
            PreparationProjectionRequest::new(vec![String::new()]).unwrap_err(),
            Error::InvalidReference
        );
        assert!(PreparationProjectionRequest::new(vec!["é".into(), "e\u{301}".into()]).is_ok());
    }
}
