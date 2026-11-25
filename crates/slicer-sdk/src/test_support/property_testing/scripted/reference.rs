//! Independent reference checks for controlled recipe families.
//!
//! Nothing here is a second renderer or a self-union: the engine output is
//! compared against *intended* geometry derived from the recipe's own integral
//! dimensions, using the prerequisite exact predicates
//! ([`point_in_region`], [`validate_geometry`], [`validate_indexed_mesh`]) and a
//! small axis-aligned interval model. These are bounded witnesses, not general
//! proofs: closed edge incidence does not prove arbitrary self-intersection
//! freedom, and a cell occupancy model only describes axis-aligned recipes.

use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Polygon};
use slicer_scad::{ScadEvalOutput, ScadGeometry};

use super::super::domains::{DomainError, GeometryCase, GeometryDomain};
use super::super::oracles::{
    mesh_signed_volume_mm3, point_in_region, validate_geometry, validate_indexed_mesh, PointLocation,
};
use super::{
    invalid_bounds, invalid_input, Recipe2d, Recipe3d, ScadRecipe, ScriptedError, SolidWitness,
};

const UNITS_PER_MM: f64 = 10_000.0;
/// Bound on the cell model, so one witness check stays finite.
const MAX_CELLS: usize = 4096;

/// One axis-aligned cell in canonical units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cell {
    min: [i64; 3],
    max: [i64; 3],
}

impl Cell {
    fn volume_units3(self) -> i128 {
        (i128::from(self.max[0]) - i128::from(self.min[0]))
            * (i128::from(self.max[1]) - i128::from(self.min[1]))
            * (i128::from(self.max[2]) - i128::from(self.min[2]))
    }

    /// Strict interior test with doubled coordinates, so half-unit probes are exact.
    fn contains_strict(self, point: [i128; 3]) -> bool {
        (0..3).all(|axis| {
            let min = i128::from(self.min[axis]) * 2;
            let max = i128::from(self.max[axis]) * 2;
            min < point[axis] && point[axis] < max
        })
    }
}

fn contours(output: &ScadEvalOutput) -> Result<Vec<ExPolygon>, ScriptedError> {
    match &output.geometry {
        ScadGeometry::Contours(contours) => Ok(contours.clone()),
        ScadGeometry::Empty => Ok(Vec::new()),
        other => Err(invalid_input(format!(
            "a contour recipe produced {other:?}"
        ))),
    }
}

fn mesh(output: &ScadEvalOutput) -> Result<IndexedTriangleSet, ScriptedError> {
    match &output.geometry {
        ScadGeometry::Mesh(mesh) => Ok(mesh.clone()),
        other => Err(invalid_input(format!("a solid recipe produced {other:?}"))),
    }
}

/// Certify a planar recipe output against its domain and, where the recipe is
/// an exactly modellable polygon family, against independent point
/// observations of the intended region.
pub(super) fn certify_contours(
    recipe: &ScadRecipe,
    output: &ScadEvalOutput,
    domain: &GeometryDomain,
) -> Result<GeometryCase, ScriptedError> {
    let case = GeometryCase {
        polygons: contours(output)?,
    };
    validate_geometry(&case, domain).map_err(ScriptedError::Domain)?;
    if let Some(intended) = intended_polygons(recipe) {
        compare_regions(&intended, &case.polygons)?;
    }
    Ok(case)
}

/// The recipe's exact intended polygon set, when it needs no clipping step.
fn intended_polygons(recipe: &ScadRecipe) -> Option<Vec<ExPolygon>> {
    let ScadRecipe::Contours(recipe) = recipe else {
        return None;
    };
    polygons2d(recipe)
}

fn polygons2d(recipe: &Recipe2d) -> Option<Vec<ExPolygon>> {
    match recipe {
        Recipe2d::Rectangle { size_units } => Some(vec![rectangle(
            0,
            0,
            i64::from(size_units[0]),
            i64::from(size_units[1]),
        )]),
        Recipe2d::Polygon { polygons } => Some(polygons.clone()),
        Recipe2d::Translate {
            offset_units,
            child,
        } => Some(
            polygons2d(child)?
                .into_iter()
                .map(|polygon| translate_expolygon(polygon, offset_units[0], offset_units[1]))
                .collect(),
        ),
        Recipe2d::QuarterTurn { turns, child } => Some(
            polygons2d(child)?
                .into_iter()
                .map(|polygon| quarter_turn_expolygon(polygon, *turns))
                .collect(),
        ),
        Recipe2d::Union(children) => {
            // Concatenation is only the intended region when the children are
            // disjoint; an overlapping union needs real clipping, so it is
            // certified by the domain validator instead of a double-counted
            // area.
            let mut polygons = Vec::new();
            let mut boxes = Vec::new();
            for child in children {
                for polygon in polygons2d(child)? {
                    let (min_x, min_y, max_x, max_y) = expolygon_bbox(&polygon);
                    if boxes
                        .iter()
                        .any(|(a, b, c, d)| min_x <= *c && *a <= max_x && min_y <= *d && *b <= max_y)
                    {
                        return None;
                    }
                    boxes.push((min_x, min_y, max_x, max_y));
                    polygons.push(polygon);
                }
            }
            Some(polygons)
        }
        // A subtraction or an intersection needs real clipping; a faceted disk
        // is only comparable through its analytic regular-polygon properties.
        // Those families are certified by the domain validator plus the
        // analytic area/vertex checks the contract tests derive themselves.
        Recipe2d::Difference { .. } | Recipe2d::Intersection(_) | Recipe2d::Disk { .. } => None,
    }
}

fn expolygon_bbox(polygon: &ExPolygon) -> (i64, i64, i64, i64) {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for point in &polygon.contour.points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x, max_y)
}

fn rectangle(x: i64, y: i64, width: i64, height: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x, y },
                Point2 { x: x + width, y },
                Point2 {
                    x: x + width,
                    y: y + height,
                },
                Point2 {
                    x,
                    y: y + height,
                },
            ],
        },
        holes: Vec::new(),
    }
}

fn translate_expolygon(mut polygon: ExPolygon, dx: i64, dy: i64) -> ExPolygon {
    for ring in std::iter::once(&mut polygon.contour).chain(polygon.holes.iter_mut()) {
        for point in &mut ring.points {
            point.x += dx;
            point.y += dy;
        }
    }
    polygon
}

fn rotate_point(point: Point2, turns: u8) -> Point2 {
    match turns % 4 {
        0 => point,
        1 => Point2 {
            x: -point.y,
            y: point.x,
        },
        2 => Point2 {
            x: -point.x,
            y: -point.y,
        },
        _ => Point2 {
            x: point.y,
            y: -point.x,
        },
    }
}

fn quarter_turn_expolygon(mut polygon: ExPolygon, turns: u8) -> ExPolygon {
    for ring in std::iter::once(&mut polygon.contour).chain(polygon.holes.iter_mut()) {
        for point in &mut ring.points {
            *point = rotate_point(*point, turns);
        }
    }
    polygon
}

fn probe_points(polygons: &[ExPolygon]) -> Vec<Point2> {
    let mut probes = Vec::new();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for polygon in polygons {
        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            let count = ring.points.len();
            for (index, point) in ring.points.iter().enumerate() {
                probes.push(*point);
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
                let next = ring.points[(index + 1) % count];
                probes.push(Point2 {
                    x: i64::midpoint(point.x, next.x),
                    y: i64::midpoint(point.y, next.y),
                });
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    probes.push(Point2 {
                        x: point.x + dx,
                        y: point.y + dy,
                    });
                }
            }
        }
    }
    if probes.is_empty() {
        return probes;
    }
    // A coarse interior/exterior grid, deliberately off the integer lattice.
    let span_x = (max_x - min_x).max(1);
    let span_y = (max_y - min_y).max(1);
    for ix in 0..=6 {
        for iy in 0..=6 {
            probes.push(Point2 {
                x: min_x + span_x * ix / 6 + 1,
                y: min_y + span_y * iy / 6 + 1,
            });
        }
    }
    probes
}

/// Compare the converted region with the intended region at every probe, and
/// require the total signed area to agree.
fn compare_regions(intended: &[ExPolygon], converted: &[ExPolygon]) -> Result<(), ScriptedError> {
    let intended_area = signed_area_mm2(intended)?;
    let converted_area = signed_area_mm2(converted)?;
    let scale = intended_area.abs().max(1.0);
    if (intended_area - converted_area).abs() > 1e-9 * scale {
        return Err(invalid_input(format!(
            "converted area {converted_area} does not match the intended {intended_area}"
        )));
    }
    for probe in probe_points(intended) {
        let expected = point_in_region(probe, intended).map_err(ScriptedError::Domain)?;
        let observed = point_in_region(probe, converted).map_err(ScriptedError::Domain)?;
        if expected == observed {
            continue;
        }
        // An intended boundary point may legitimately quantize onto a slightly
        // different boundary; an interior/outside disagreement is a failure.
        if expected == PointLocation::Boundary || observed == PointLocation::Boundary {
            continue;
        }
        return Err(invalid_input(format!(
            "converted region disagrees with the intended region at ({}, {}): {expected:?} vs {observed:?}",
            probe.x, probe.y
        )));
    }
    Ok(())
}

/// Signed area of a polygon forest in square millimetres, from exact canonical
/// integer area with one division per forest.
fn signed_area_mm2(polygons: &[ExPolygon]) -> Result<f64, ScriptedError> {
    let mut total: i128 = 0;
    for polygon in polygons {
        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            total = total
                .checked_add(signed_area2(ring)?)
                .ok_or(ScriptedError::Domain(DomainError::ArithmeticOverflow))?;
        }
    }
    Ok(total as f64 / (2.0 * UNITS_PER_MM * UNITS_PER_MM))
}

fn signed_area2(ring: &Polygon) -> Result<i128, ScriptedError> {
    let overflow = || ScriptedError::Domain(DomainError::ArithmeticOverflow);
    let mut total: i128 = 0;
    let count = ring.points.len();
    for index in 0..count {
        let a = ring.points[index];
        let b = ring.points[(index + 1) % count];
        let cross = i128::from(a.x)
            .checked_mul(i128::from(b.y))
            .and_then(|value| {
                value.checked_sub(i128::from(a.y).checked_mul(i128::from(b.x))?)
            })
            .ok_or_else(overflow)?;
        total = total.checked_add(cross).ok_or_else(overflow)?;
    }
    Ok(total)
}

// ---------------------------------------------------------------------------
// Solids
// ---------------------------------------------------------------------------

/// Certify a solid recipe output: structural mesh validity, then the requested
/// witness against an independent axis-aligned occupancy model.
pub(super) fn certify_solid(
    recipe: &ScadRecipe,
    output: &ScadEvalOutput,
    witness: SolidWitness,
) -> Result<IndexedTriangleSet, ScriptedError> {
    let mesh = mesh(output)?;
    validate_indexed_mesh(&mesh, true).map_err(ScriptedError::Domain)?;
    let volume = mesh_signed_volume_mm3(&mesh).map_err(ScriptedError::Domain)?;
    if !(volume > 0.0) {
        return Err(invalid_input(format!(
            "a {witness:?} scene must have positive volume, got {volume}"
        )));
    }
    match occupancy(recipe) {
        Some(cells) => certify_against_cells(&mesh, volume, &cells, witness)?,
        None if is_single_cylinder(recipe) => certify_cylinder(recipe, &mesh, volume, witness)?,
        None if witness == SolidWitness::ClosedSolid => (),
        None => {
            return Err(invalid_input(format!(
                "{witness:?} needs a controlled axis-aligned scene; this recipe has no occupancy model"
            )))
        }
    }
    Ok(mesh)
}

/// Whether the recipe is exactly one faceted cylinder (the only non-orthogonal
/// primitive the generator emits).
fn is_single_cylinder(recipe: &ScadRecipe) -> bool {
    matches!(
        recipe,
        ScadRecipe::Solid(Recipe3d::Cylinder { .. })
    )
}

/// A faceted cylinder has an exact regular-polygon reference: volume is the
/// n-gon area times the height, and every vertex lies on the radius at z=0 or
/// z=height. Ideal-circle or sphere equality is never used.
fn certify_cylinder(
    recipe: &ScadRecipe,
    mesh: &IndexedTriangleSet,
    volume: f64,
    witness: SolidWitness,
) -> Result<(), ScriptedError> {
    if witness != SolidWitness::ClosedSolid {
        return Err(invalid_input(
            "a faceted cylinder certifies only the closed-solid witness",
        ));
    }
    let ScadRecipe::Solid(Recipe3d::Cylinder {
        radius_units,
        height_units,
        facets,
    }) = recipe
    else {
        return Err(invalid_input("not a single cylinder recipe"));
    };
    let radius = f64::from(*radius_units) / UNITS_PER_MM;
    let height = f64::from(*height_units) / UNITS_PER_MM;
    let facets_f = f64::from(*facets);
    let analytic = 0.5 * facets_f * radius * radius * (std::f64::consts::TAU / facets_f).sin() * height;
    // The same f32 narrowing budget as the cell scenes: a few times 2^-24 per
    // edge of the largest dimension.
    let budget = 1e-5 * radius.max(height).powi(3) + 1e-9;
    if (analytic - volume).abs() > budget {
        return Err(invalid_input(format!(
            "faceted cylinder volume {volume} disagrees with the regular-polygon reference {analytic}"
        )));
    }
    let (min, max) = mesh_bounds(mesh)?;
    let coordinate_budget = 1e-5 * radius.max(height) + 1e-6;
    // A faceted circle is not an ideal circle, and an odd regular polygon is not
    // symmetric about the Y axis. Take the signed minima and maxima of the actual
    // regular-polygon vertices instead of assuming ±radius.
    let angle = std::f64::consts::TAU / facets_f;
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for index in 0..*facets {
        let theta = angle * f64::from(index);
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if (min[0] - min_x).abs() > coordinate_budget
        || (max[0] - max_x).abs() > coordinate_budget
        || (min[1] - min_y).abs() > coordinate_budget
        || (max[1] - max_y).abs() > coordinate_budget
        || min[2].abs() > coordinate_budget
        || (max[2] - height).abs() > coordinate_budget
    {
        return Err(invalid_input(format!(
            "faceted cylinder bounds {min:?}..{max:?} disagree with the {facets}-gon extents ({min_x}..{max_x}, {min_y}..{max_y}) and height {height}"
        )));
    }
    let mut on_radius = 0_usize;
    for vertex in &mesh.vertices {
        let x = f64::from(vertex.x);
        let y = f64::from(vertex.y);
        let r = (x * x + y * y).sqrt();
        if (r - radius).abs() <= coordinate_budget {
            on_radius += 1;
        }
    }
    if on_radius < usize::from(*facets) {
        return Err(invalid_input(format!(
            "a {facets}-facet cylinder must keep at least {facets} vertices on its radius, found {on_radius}"
        )));
    }
    Ok(())
}

fn certify_against_cells(
    mesh: &IndexedTriangleSet,
    volume: f64,
    cells: &[Cell],
    witness: SolidWitness,
) -> Result<(), ScriptedError> {
    if cells.is_empty() {
        return Err(invalid_input("a certified solid scene has no cells"));
    }
    if cells.len() > MAX_CELLS {
        return Err(invalid_bounds("the occupancy model exceeded its cell bound"));
    }
    let units3: i128 = cells.iter().map(|cell| cell.volume_units3()).sum();
    let cell_volume_mm3 = units3 as f64 / (UNITS_PER_MM * UNITS_PER_MM * UNITS_PER_MM);
    let (min, max) = cells_bounds(cells);
    let largest = (0..3)
        .map(|axis| (max[axis] - min[axis]).abs() as f64 / UNITS_PER_MM)
        .fold(0.0_f64, f64::max);
    // f32 vertex narrowing has relative error below 2^-24. A closed triangulated
    // axis-aligned cell scene therefore loses at most a few times that per edge:
    // 1e-5 * M^3 is a conservative absolute volume budget, and 1e-5 * M + 1e-7 a
    // conservative coordinate budget for these bounded scenes.
    let volume_budget = 1e-5 * largest.powi(3) + 1e-9;
    if (cell_volume_mm3 - volume).abs() > volume_budget {
        return Err(invalid_input(format!(
            "mesh volume {volume} disagrees with the {cell_volume_mm3} mm³ occupancy reference (budget {volume_budget})"
        )));
    }
    let (mesh_min, mesh_max) = mesh_bounds(mesh)?;
    let coordinate_budget = 1e-5 * largest + 1e-7;
    for axis in 0..3 {
        let expected_min = min[axis] as f64 / UNITS_PER_MM;
        let expected_max = max[axis] as f64 / UNITS_PER_MM;
        if (mesh_min[axis] - expected_min).abs() > coordinate_budget
            || (mesh_max[axis] - expected_max).abs() > coordinate_budget
        {
            return Err(invalid_input(format!(
                "mesh bounds {mesh_min:?}..{mesh_max:?} disagree with the occupancy reference"
            )));
        }
    }
    // The model says what the scene must contain; the converted mesh must show
    // it at the same designated points. Volume and bounds agreement alone cannot
    // transfer a cavity, span or body separation to the output.
    match witness {
        SolidWitness::ClosedSolid => Ok(()),
        SolidWitness::Cavity => {
            let probe = enclosed_void_probe(cells, min, max)
                .ok_or_else(|| invalid_input("the scene has no enclosed void"))?;
            if point_inside_mesh(mesh, probe_mm(probe)) {
                return Err(invalid_input(
                    "the model's enclosed void centre is inside the converted mesh",
                ));
            }
            for axis in 0..3 {
                for sign in [-1_i128, 1] {
                    let support = blocked_by(cells, probe, axis, sign).ok_or_else(|| {
                        invalid_input("the model's void is not closed on six sides")
                    })?;
                    if !point_inside_mesh(mesh, probe_mm(support)) {
                        return Err(invalid_input(format!(
                            "the converted mesh has no material on axis {axis} sign {sign} around the void"
                        )));
                    }
                }
            }
            Ok(())
        }
        SolidWitness::Bridge => {
            let span = unsupported_span_probe(cells, min, max)
                .ok_or_else(|| invalid_input("the scene has no unsupported span"))?;
            if !(span.supported_left.is_some() && span.supported_right.is_some()) {
                return Err(invalid_input("the unsupported span is not bridged"));
            }
            verify_span(mesh, &span)
        }
        SolidWitness::Overhang => {
            let span = unsupported_span_probe(cells, min, max)
                .ok_or_else(|| invalid_input("the scene has no unsupported span"))?;
            if span.supported_left.is_some() && span.supported_right.is_some() {
                return Err(invalid_input("the unsupported span is fully bridged"));
            }
            verify_span(mesh, &span)
        }
        SolidWitness::SeparatedBodies => {
            if mesh_triangle_components(mesh) < 2 {
                return Err(invalid_input(
                    "the converted mesh is one connected surface, not separated bodies",
                ));
            }
            let probes = separated_body_probes(cells, min, max)
                .ok_or_else(|| invalid_input("the scene has no separated bodies"))?;
            let midpoint = separated_gap_probe(cells, min, max)
                .ok_or_else(|| invalid_input("the scene has no gap between its bodies"))?;
            if !point_inside_mesh(mesh, probe_mm(probes.0))
                || !point_inside_mesh(mesh, probe_mm(probes.1))
            {
                return Err(invalid_input(
                    "a model-separated body is missing from the converted mesh",
                ));
            }
            if point_inside_mesh(mesh, probe_mm(midpoint)) {
                return Err(invalid_input(
                    "the converted mesh fills the gap between the separated bodies",
                ));
            }
            Ok(())
        }
    }
}

/// The converted mesh must show the span exactly where the model says it is.
fn verify_span(mesh: &IndexedTriangleSet, span: &UnsupportedProbe) -> Result<(), ScriptedError> {
    if !point_inside_mesh(mesh, probe_mm(span.point)) {
        return Err(invalid_input(
            "the model's unsupported span is missing from the converted mesh",
        ));
    }
    let mut below = span.point;
    below[2] -= 2;
    if point_inside_mesh(mesh, probe_mm(below)) {
        return Err(invalid_input(
            "the converted mesh supports the span that the model leaves open",
        ));
    }
    for support in [span.supported_left, span.supported_right]
        .into_iter()
        .flatten()
    {
        if !point_inside_mesh(mesh, probe_mm(support)) {
            return Err(invalid_input(
                "the converted mesh is missing a support the model claims",
            ));
        }
    }
    Ok(())
}

fn cells_bounds(cells: &[Cell]) -> ([i64; 3], [i64; 3]) {
    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for cell in cells {
        for axis in 0..3 {
            min[axis] = min[axis].min(cell.min[axis]);
            max[axis] = max[axis].max(cell.max[axis]);
        }
    }
    (min, max)
}

/// Independent point-in-mesh test by ray casting, using the converted vertices.
///
/// The ray direction is deliberately skew so an axis-aligned box face or edge is
/// not grazed. This is the bounded witness check for the designated interior
/// points the occupancy model asks about; it is not a general solid classifier.
fn point_inside_mesh(mesh: &IndexedTriangleSet, point: [f64; 3]) -> bool {
    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
    // A skew ray can still pass exactly through a shared triangle edge or vertex,
    // where naive counting would count both incident triangles once. Each
    // direction reports whether it hit such a degenerate point; an ambiguous
    // direction is discarded rather than trusted.
    const DIRECTIONS: [[f64; 3]; 5] = [
        [1.0, 0.373_1, 0.114_9],
        [0.721_3, 1.0, -0.413_7],
        [-0.317_9, 0.611_3, 1.0],
        [1.0, -0.907_3, 0.523_1],
        [0.431_9, 0.276_7, 1.0],
    ];
    for direction in DIRECTIONS {
        let mut crossings = 0_usize;
        let mut ambiguous = false;
        for triangle in mesh.indices.chunks_exact(3) {
            let mut vertices = [[0.0_f64; 3]; 3];
            for (slot, index) in vertices.iter_mut().zip(triangle) {
                let Some(vertex) = mesh.vertices.get(*index as usize) else {
                    return false;
                };
                *slot = [
                    f64::from(vertex.x),
                    f64::from(vertex.y),
                    f64::from(vertex.z),
                ];
            }
            let a = vertices[0];
            let edge1 = [
                vertices[1][0] - a[0],
                vertices[1][1] - a[1],
                vertices[1][2] - a[2],
            ];
            let edge2 = [
                vertices[2][0] - a[0],
                vertices[2][1] - a[1],
                vertices[2][2] - a[2],
            ];
            let pvec = cross(direction, edge2);
            let determinant = dot(edge1, pvec);
            if determinant.abs() < 1e-12 {
                continue;
            }
            let inverse = 1.0 / determinant;
            let tvec = [point[0] - a[0], point[1] - a[1], point[2] - a[2]];
            let u = dot(tvec, pvec) * inverse;
            let qvec = cross(tvec, edge1);
            let v = dot(direction, qvec) * inverse;
            let t = dot(edge2, qvec) * inverse;
            if t <= 1e-9 {
                continue;
            }
            let w = 1.0 - u - v;
            if u < -1e-9 || v < -1e-9 || w < -1e-9 {
                continue;
            }
            // Within a hair of any triangle edge: this direction is unreliable.
            if u.abs() < 1e-7 || v.abs() < 1e-7 || w.abs() < 1e-7 {
                ambiguous = true;
                break;
            }
            crossings += 1;
        }
        if !ambiguous {
            return crossings % 2 == 1;
        }
    }
    // Every tried direction was degenerate for this point: fail closed for a
    // witness check rather than claiming the point is outside.
    true
}

/// Number of edge-connected triangle components in a converted mesh.
///
/// A closed manifold surface's triangles are connected through shared undirected
/// edges, so two disjoint bodies have two components. This is a topology
/// observation on the output itself, independent of volume or bounds.
fn mesh_triangle_components(mesh: &IndexedTriangleSet) -> usize {
    use std::collections::BTreeMap;
    let triangles = mesh.indices.len() / 3;
    if triangles == 0 {
        return 0;
    }
    // Weld vertices by position first: a connected surface can be expressed with
    // duplicated coincident vertices, and two bodies joined through a coincident
    // face must not read as two components just because their indices differ.
    let mut welded: BTreeMap<(u32, u32, u32), u32> = BTreeMap::new();
    let mut remap = Vec::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        let key = (vertex.x.to_bits(), vertex.y.to_bits(), vertex.z.to_bits());
        let next = welded.len() as u32;
        remap.push(*welded.entry(key).or_insert(next));
    }
    let mut parent: Vec<usize> = (0..triangles).collect();
    fn find(parent: &mut [usize], index: usize) -> usize {
        if parent[index] == index {
            index
        } else {
            let root = find(parent, parent[index]);
            parent[index] = root;
            root
        }
    }
    let mut edges: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for (triangle, chunk) in mesh.indices.chunks_exact(3).enumerate() {
        for pair in 0..3 {
            let a = remap[chunk[pair] as usize];
            let b = remap[chunk[(pair + 1) % 3] as usize];
            if a == b {
                continue;
            }
            let key = if a < b { (a, b) } else { (b, a) };
            match edges.get(&key) {
                Some(&other) => {
                    let (left, right) = (find(&mut parent, triangle), find(&mut parent, other));
                    if left != right {
                        parent[left] = right;
                    }
                }
                None => {
                    edges.insert(key, triangle);
                }
            }
        }
    }
    let roots: std::collections::BTreeSet<_> =
        (0..triangles).map(|i| find(&mut parent, i)).collect();
    roots.len()
}

/// Convert a doubled canonical-unit probe to millimetres.
fn probe_mm(point: [i128; 3]) -> [f64; 3] {
    [
        point[0] as f64 / 2.0 / UNITS_PER_MM,
        point[1] as f64 / 2.0 / UNITS_PER_MM,
        point[2] as f64 / 2.0 / UNITS_PER_MM,
    ]
}

fn mesh_bounds(mesh: &IndexedTriangleSet) -> Result<([f64; 3], [f64; 3]), ScriptedError> {
    if mesh.vertices.is_empty() {
        return Err(invalid_input("a certified mesh has no vertices"));
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for (axis, value) in [vertex.x, vertex.y, vertex.z].into_iter().enumerate() {
            let value = f64::from(value);
            if !value.is_finite() {
                return Err(invalid_input("a certified mesh has a non-finite vertex"));
            }
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    Ok((min, max))
}

fn occupancy(recipe: &ScadRecipe) -> Option<Vec<Cell>> {
    match recipe {
        ScadRecipe::Solid(recipe) => cells3d(recipe),
        ScadRecipe::Contours(_) => None,
    }
}

fn cells3d(recipe: &Recipe3d) -> Option<Vec<Cell>> {
    match recipe {
        Recipe3d::Box { size_units } => Some(vec![Cell {
            min: [0; 3],
            max: [
                i64::from(size_units[0]),
                i64::from(size_units[1]),
                i64::from(size_units[2]),
            ],
        }]),
        Recipe3d::LinearExtrude {
            height_units,
            profile,
        } => Some(
            rect2d(profile)?
                .into_iter()
                .map(|(min, max)| Cell {
                    min: [min[0], min[1], 0],
                    max: [max[0], max[1], i64::from(*height_units)],
                })
                .collect(),
        ),
        Recipe3d::Translate {
            offset_units,
            child,
        } => Some(
            cells3d(child)?
                .into_iter()
                .map(|cell| translate_cell(cell, *offset_units))
                .collect(),
        ),
        Recipe3d::QuarterTurnZ { turns, child } => Some(
            cells3d(child)?
                .into_iter()
                .map(|cell| quarter_turn_cell(cell, *turns))
                .collect(),
        ),
        Recipe3d::Union(children) => {
            let mut cells: Vec<Cell> = Vec::new();
            for child in children {
                for cell in cells3d(child)? {
                    // Keep the model disjoint so volumes never double count.
                    let remainder = subtract_all(&[cell], &cells);
                    cells.extend(remainder);
                }
            }
            Some(cells)
        }
        Recipe3d::Difference { base, cuts } => {
            let mut cells = cells3d(base)?;
            for cut in cuts {
                let cut_cells = cells3d(cut)?;
                cells = subtract_all(&cells, &cut_cells);
                if cells.is_empty() {
                    break;
                }
            }
            Some(cells)
        }
        Recipe3d::Intersection(children) => {
            let mut cells: Option<Vec<Cell>> = None;
            for child in children {
                let child_cells = cells3d(child)?;
                cells = Some(match cells {
                    None => child_cells,
                    Some(current) => intersect_all(&current, &child_cells),
                });
            }
            cells
        }
        Recipe3d::Cylinder { .. } => None,
    }
}

fn rect2d(recipe: &Recipe2d) -> Option<Vec<([i64; 2], [i64; 2])>> {
    match recipe {
        Recipe2d::Rectangle { size_units } => Some(vec![(
            [0, 0],
            [i64::from(size_units[0]), i64::from(size_units[1])],
        )]),
        Recipe2d::Translate {
            offset_units,
            child,
        } => Some(
            rect2d(child)?
                .into_iter()
                .map(|(min, max)| {
                    (
                        [min[0] + offset_units[0], min[1] + offset_units[1]],
                        [max[0] + offset_units[0], max[1] + offset_units[1]],
                    )
                })
                .collect(),
        ),
        Recipe2d::QuarterTurn { turns, child } => Some(
            rect2d(child)?
                .into_iter()
                .map(|(min, max)| quarter_turn_rect(min, max, *turns))
                .collect(),
        ),
        Recipe2d::Union(children) => {
            let mut rects: Vec<([i64; 2], [i64; 2])> = Vec::new();
            for child in children {
                for rect in rect2d(child)? {
                    let remainder = subtract_rect_all(vec![rect], &rects);
                    rects.extend(remainder);
                }
            }
            Some(rects)
        }
        Recipe2d::Difference { base, cuts } => {
            let mut rects = rect2d(base)?;
            for cut in cuts {
                let cut_rects = rect2d(cut)?;
                rects = subtract_rect_all(rects, &cut_rects);
            }
            Some(rects)
        }
        Recipe2d::Intersection(children) => {
            let mut rects: Option<Vec<([i64; 2], [i64; 2])>> = None;
            for child in children {
                let child_rects = rect2d(child)?;
                rects = Some(match rects {
                    None => child_rects,
                    Some(current) => {
                        let mut next = Vec::new();
                        for a in &current {
                            for b in &child_rects {
                                if let Some(overlap) = intersect_rect(*a, *b) {
                                    next.push(overlap);
                                }
                            }
                        }
                        next
                    }
                });
            }
            rects
        }
        Recipe2d::Polygon { .. } | Recipe2d::Disk { .. } => None,
    }
}

fn quarter_turn_rect(min: [i64; 2], max: [i64; 2], turns: u8) -> ([i64; 2], [i64; 2]) {
    let corners = [
        Point2 {
            x: min[0],
            y: min[1],
        },
        Point2 {
            x: max[0],
            y: min[1],
        },
        Point2 {
            x: max[0],
            y: max[1],
        },
        Point2 {
            x: min[0],
            y: max[1],
        },
    ];
    let mut min_out = [i64::MAX; 2];
    let mut max_out = [i64::MIN; 2];
    for point in corners.map(|point| rotate_point(point, turns)) {
        min_out[0] = min_out[0].min(point.x);
        min_out[1] = min_out[1].min(point.y);
        max_out[0] = max_out[0].max(point.x);
        max_out[1] = max_out[1].max(point.y);
    }
    (min_out, max_out)
}

fn translate_cell(mut cell: Cell, offset: [i64; 3]) -> Cell {
    for axis in 0..3 {
        cell.min[axis] += offset[axis];
        cell.max[axis] += offset[axis];
    }
    cell
}

fn quarter_turn_cell(cell: Cell, turns: u8) -> Cell {
    let (min, max) = quarter_turn_rect(
        [cell.min[0], cell.min[1]],
        [cell.max[0], cell.max[1]],
        turns,
    );
    Cell {
        min: [min[0], min[1], cell.min[2]],
        max: [max[0], max[1], cell.max[2]],
    }
}

fn subtract_all(cells: &[Cell], cuts: &[Cell]) -> Vec<Cell> {
    let mut remaining = cells.to_vec();
    for cut in cuts {
        let mut next = Vec::new();
        for cell in &remaining {
            next.extend(subtract_cell(*cell, *cut));
        }
        remaining = next;
        if remaining.is_empty() {
            break;
        }
    }
    remaining
}

/// Six maximal slabs of `base \ cut`, filtered to positive volume.
fn subtract_cell(base: Cell, cut: Cell) -> Vec<Cell> {
    let mut pieces = vec![base];
    for axis in 0..3 {
        let mut next = Vec::new();
        for piece in pieces {
            if piece.max[axis] <= cut.min[axis] || cut.max[axis] <= piece.min[axis] {
                next.push(piece);
                continue;
            }
            let low = cut.min[axis].clamp(piece.min[axis], piece.max[axis]);
            let high = cut.max[axis].clamp(piece.min[axis], piece.max[axis]);
            if low > piece.min[axis] {
                let mut lower = piece;
                lower.max[axis] = low;
                next.push(lower);
            }
            if high < piece.max[axis] {
                let mut upper = piece;
                upper.min[axis] = high;
                next.push(upper);
            }
            if high > low {
                let mut middle = piece;
                middle.min[axis] = low;
                middle.max[axis] = high;
                next.push(middle);
            }
        }
        pieces = next;
    }
    pieces
        .into_iter()
        .filter(|piece| !cell_contains(cut, *piece))
        .filter(|piece| piece.volume_units3() > 0)
        .collect()
}

fn cell_contains(outer: Cell, inner: Cell) -> bool {
    (0..3).all(|axis| outer.min[axis] <= inner.min[axis] && inner.max[axis] <= outer.max[axis])
}

fn intersect_all(left: &[Cell], right: &[Cell]) -> Vec<Cell> {
    let mut out = Vec::new();
    for a in left {
        for b in right {
            let mut min = [0_i64; 3];
            let mut max = [0_i64; 3];
            let mut valid = true;
            for axis in 0..3 {
                min[axis] = a.min[axis].max(b.min[axis]);
                max[axis] = a.max[axis].min(b.max[axis]);
                if min[axis] >= max[axis] {
                    valid = false;
                }
            }
            if valid {
                out.push(Cell { min, max });
            }
        }
    }
    out
}

fn subtract_rect_all(
    base: Vec<([i64; 2], [i64; 2])>,
    cuts: &[([i64; 2], [i64; 2])],
) -> Vec<([i64; 2], [i64; 2])> {
    let mut remaining = base;
    for cut in cuts {
        let mut next = Vec::new();
        for rect in remaining {
            next.extend(subtract_rect(rect, *cut));
        }
        remaining = next;
        if remaining.is_empty() {
            break;
        }
    }
    remaining
}

fn subtract_rect(
    base: ([i64; 2], [i64; 2]),
    cut: ([i64; 2], [i64; 2]),
) -> Vec<([i64; 2], [i64; 2])> {
    let (min, max) = base;
    let (cut_min, cut_max) = cut;
    if cut_max[0] <= min[0] || max[0] <= cut_min[0] || cut_max[1] <= min[1] || max[1] <= cut_min[1] {
        return vec![base];
    }
    let low_x = cut_min[0].clamp(min[0], max[0]);
    let high_x = cut_max[0].clamp(min[0], max[0]);
    let low_y = cut_min[1].clamp(min[1], max[1]);
    let high_y = cut_max[1].clamp(min[1], max[1]);
    let mut out = Vec::new();
    if low_x > min[0] {
        out.push(([min[0], min[1]], [low_x, max[1]]));
    }
    if high_x < max[0] {
        out.push(([high_x, min[1]], [max[0], max[1]]));
    }
    if high_x > low_x && low_y > min[1] {
        out.push(([low_x, min[1]], [high_x, low_y]));
    }
    if high_x > low_x && high_y < max[1] {
        out.push(([low_x, high_y], [high_x, max[1]]));
    }
    out
}

fn intersect_rect(
    a: ([i64; 2], [i64; 2]),
    b: ([i64; 2], [i64; 2]),
) -> Option<([i64; 2], [i64; 2])> {
    let min = [a.0[0].max(b.0[0]), a.0[1].max(b.0[1])];
    let max = [a.1[0].min(b.1[0]), a.1[1].min(b.1[1])];
    (min[0] < max[0] && min[1] < max[1]).then_some((min, max))
}

/// Probe points as doubled canonical coordinates: cell face centres, cell
/// centres, and just-inside cell faces are exactly representable this way.
fn probe_points_3d(cells: &[Cell]) -> Vec<[i128; 3]> {
    let doubled = |value: i64| i128::from(value) * 2;
    let mut points = Vec::new();
    for cell in cells {
        let mut xs = vec![
            doubled(cell.min[0]) + 1,
            doubled(i64::midpoint(cell.min[0], cell.max[0])),
            doubled(cell.max[0]) - 1,
        ];
        let mut ys = vec![
            doubled(cell.min[1]) + 1,
            doubled(i64::midpoint(cell.min[1], cell.max[1])),
            doubled(cell.max[1]) - 1,
        ];
        let mut zs = vec![
            doubled(cell.min[2]) + 1,
            doubled(i64::midpoint(cell.min[2], cell.max[2])),
            doubled(cell.max[2]) - 1,
        ];
        xs.sort_unstable();
        ys.sort_unstable();
        zs.sort_unstable();
        xs.dedup();
        ys.dedup();
        zs.dedup();
        for &x in &xs {
            for &y in &ys {
                for &z in &zs {
                    points.push([x, y, z]);
                }
            }
        }
    }
    points
}

fn inside(cells: &[Cell], point: [i128; 3]) -> bool {
    cells.iter().any(|cell| cell.contains_strict(point))
}

/// Find a void that is closed on all six sides, as a doubled-unit probe point.
///
/// A candidate empty point must be strictly inside the overall box and every axis
/// direction must reach solid material before leaving the box. The direction scan
/// uses per-axis cell boundaries offset by one unit, so a material interval of at
/// least two units always contains a probe.
fn enclosed_void_probe(cells: &[Cell], min: [i64; 3], max: [i64; 3]) -> Option<[i128; 3]> {
    let doubled = |value: i64| i128::from(value) * 2;
    let span = (0..3)
        .map(|axis| max[axis].saturating_sub(min[axis]))
        .max()
        .unwrap_or(0);
    let step = (span / 12).max(1);
    let counts = [
        (max[0] - min[0]) / step + 1,
        (max[1] - min[1]) / step + 1,
        (max[2] - min[2]) / step + 1,
    ];
    for ix in 0..counts[0] {
        for iy in 0..counts[1] {
            for iz in 0..counts[2] {
                let point = [
                    doubled(min[0] + ix * step) + 1,
                    doubled(min[1] + iy * step) + 1,
                    doubled(min[2] + iz * step) + 1,
                ];
                let interior = (0..3).all(|axis| {
                    let low = doubled(min[axis]) + 2;
                    let high = doubled(max[axis]) - 2;
                    low < point[axis] && point[axis] < high
                });
                if !interior || inside(cells, point) {
                    continue;
                }
                let closed = (0..3).all(|axis| {
                    blocked_by(cells, point, axis, -1).is_some()
                        && blocked_by(cells, point, axis, 1).is_some()
                });
                if closed {
                    return Some(point);
                }
            }
        }
    }
    None
}

fn axis_samples(cells: &[Cell], axis: usize) -> Vec<i128> {
    let mut samples = Vec::new();
    for cell in cells {
        let low = i128::from(cell.min[axis]) * 2;
        let high = i128::from(cell.max[axis]) * 2;
        samples.push(low - 1);
        samples.push(low + 1);
        samples.push(high - 1);
        samples.push(high + 1);
        samples.push((low + high) / 2);
    }
    samples.sort_unstable();
    samples.dedup();
    samples
}

/// The nearest material sample beyond `point` along one axis, if any.
fn blocked_by(cells: &[Cell], point: [i128; 3], axis: usize, sign: i128) -> Option<[i128; 3]> {
    axis_samples(cells, axis).into_iter().find_map(|sample| {
        let beyond = if sign < 0 {
            sample < point[axis]
        } else {
            sample > point[axis]
        };
        if !beyond {
            return None;
        }
        let mut probe = point;
        probe[axis] = sample;
        inside(cells, probe).then_some(probe)
    })
}

/// A designated unsupported-span probe and the supports the model claims.
struct UnsupportedProbe {
    point: [i128; 3],
    supported_left: Option<[i128; 3]>,
    supported_right: Option<[i128; 3]>,
}

/// Prefer a bridged probe when the scene has one, so a bridge is not classified
/// by an incidental free edge and vice versa.
fn unsupported_span_probe(
    cells: &[Cell],
    min: [i64; 3],
    _max: [i64; 3],
) -> Option<UnsupportedProbe> {
    let doubled = |value: i64| i128::from(value) * 2;
    let floor = doubled(min[2]);
    let mut free_edge = None;
    for point in probe_points_3d(cells) {
        if !inside(cells, point) {
            continue;
        }
        let mut below = point;
        below[2] -= 2;
        if below[2] <= floor || inside(cells, below) {
            continue;
        }
        let probe = UnsupportedProbe {
            point,
            supported_left: supported_on_side(cells, point, -1),
            supported_right: supported_on_side(cells, point, 1),
        };
        if probe.supported_left.is_some() && probe.supported_right.is_some() {
            return Some(probe);
        }
        if free_edge.is_none() {
            free_edge = Some(probe);
        }
    }
    free_edge
}

/// Whether material at the probe's level, strictly on one side in X, continues
/// one unit below it. Returns the support sample so the converted mesh can be
/// checked at the same point.
///
/// The cell model is axis-aligned, so the side test walks the level footprint's
/// own X samples (cell boundaries and centres) rather than stepping one unit at a
/// time across an arbitrarily large gap.
fn supported_on_side(cells: &[Cell], point: [i128; 3], direction: i128) -> Option<[i128; 3]> {
    let mut samples: Vec<i128> = Vec::new();
    for cell in cells {
        samples.push(i128::from(cell.min[0]) * 2);
        samples.push(i128::from(cell.max[0]) * 2);
        samples.push(i128::from(i64::midpoint(cell.min[0], cell.max[0])) * 2);
    }
    samples.sort_unstable();
    samples.dedup();
    for x in samples {
        let on_side = if direction < 0 {
            x < point[0]
        } else {
            x > point[0]
        };
        if !on_side {
            continue;
        }
        let at_level = [x, point[1], point[2]];
        if !inside(cells, at_level) {
            continue;
        }
        let mut below = at_level;
        below[2] -= 2;
        if inside(cells, below) {
            return Some(below);
        }
    }
    None
}

/// Centres of two separated bodies at an interior slice, in doubled units.
fn separated_body_probes(
    cells: &[Cell],
    min: [i64; 3],
    max: [i64; 3],
) -> Option<([i128; 3], [i128; 3])> {
    let z = i64::midpoint(min[2], max[2]);
    let rects: Vec<([i64; 2], [i64; 2])> = cells
        .iter()
        .filter(|cell| cell.min[2] < z && z < cell.max[2])
        .map(|cell| ([cell.min[0], cell.min[1]], [cell.max[0], cell.max[1]]))
        .collect();
    if rects.len() < 2 {
        return None;
    }
    let mut parent: Vec<usize> = (0..rects.len()).collect();
    fn find(parent: &mut [usize], index: usize) -> usize {
        if parent[index] == index {
            index
        } else {
            let root = find(parent, parent[index]);
            parent[index] = root;
            root
        }
    }
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            if rect_touches(rects[i], rects[j]) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a] = b;
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for index in 0..rects.len() {
        let root = find(&mut parent, index);
        match groups
            .iter_mut()
            .find(|group| find(&mut parent, group[0]) == root)
        {
            Some(group) => group.push(index),
            None => groups.push(vec![index]),
        }
    }
    let first = *groups.first()?.first()?;
    let second = groups.get(1)?.first().copied()?;
    let centre = |index: usize| {
        let (low, high) = rects[index];
        [
            i128::from(i64::midpoint(low[0], high[0])) * 2,
            i128::from(i64::midpoint(low[1], high[1])) * 2,
            i128::from(z) * 2,
        ]
    };
    Some((centre(first), centre(second)))
}

/// A point in the empty gap between the two body groups, in doubled units.
///
/// The centre-of-centres is only in the gap when both bodies have the same size,
/// so the gap probe uses the facing edges on the separated axis.
fn separated_gap_probe(
    cells: &[Cell],
    min: [i64; 3],
    max: [i64; 3],
) -> Option<[i128; 3]> {
    let z = i64::midpoint(min[2], max[2]);
    let rects: Vec<([i64; 2], [i64; 2])> = cells
        .iter()
        .filter(|cell| cell.min[2] < z && z < cell.max[2])
        .map(|cell| ([cell.min[0], cell.min[1]], [cell.max[0], cell.max[1]]))
        .collect();
    if rects.len() < 2 {
        return None;
    }
    let mut parent: Vec<usize> = (0..rects.len()).collect();
    fn find(parent: &mut [usize], index: usize) -> usize {
        if parent[index] == index {
            index
        } else {
            let root = find(parent, parent[index]);
            parent[index] = root;
            root
        }
    }
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            if rect_touches(rects[i], rects[j]) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a] = b;
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for index in 0..rects.len() {
        let root = find(&mut parent, index);
        match groups
            .iter_mut()
            .find(|group| find(&mut parent, group[0]) == root)
        {
            Some(group) => group.push(index),
            None => groups.push(vec![index]),
        }
    }
    let group_a = groups.first()?;
    let group_b = groups.get(1)?;
    let bounds_of = |group: &Vec<usize>| {
        let mut low = [i64::MAX; 2];
        let mut high = [i64::MIN; 2];
        for &index in group {
            let (rect_low, rect_high) = rects[index];
            for axis in 0..2 {
                low[axis] = low[axis].min(rect_low[axis]);
                high[axis] = high[axis].max(rect_high[axis]);
            }
        }
        (low, high)
    };
    let (a_low, a_high) = bounds_of(group_a);
    let (b_low, b_high) = bounds_of(group_b);
    let overlap_mid = |axis: usize| i128::from(i64::midpoint(a_low[axis].max(b_low[axis]), a_high[axis].min(b_high[axis]))) * 2;
    for axis in 0..2 {
        let gap = if a_high[axis] < b_low[axis] {
            Some((a_high[axis], b_low[axis]))
        } else if b_high[axis] < a_low[axis] {
            Some((b_high[axis], a_low[axis]))
        } else {
            None
        };
        if let Some((left, right)) = gap {
            // The separated axis carries the gap midpoint; the other axis keeps
            // the overlap midpoint so the probe is between the bodies.
            let gap_mid = i128::from(i64::midpoint(left, right)) * 2;
            let probe = if axis == 0 {
                [gap_mid, overlap_mid(1), i128::from(z) * 2]
            } else {
                [overlap_mid(0), gap_mid, i128::from(z) * 2]
            };
            return Some(probe);
        }
    }
    None
}

fn rect_touches(a: ([i64; 2], [i64; 2]), b: ([i64; 2], [i64; 2])) -> bool {
    a.0[0] <= b.1[0] && b.0[0] <= a.1[0] && a.0[1] <= b.1[1] && b.0[1] <= a.1[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_cell(min: [i64; 3], max: [i64; 3]) -> Cell {
        Cell { min, max }
    }

    #[test]
    fn reference_bridge_scene_is_bridged() {
        let cells = vec![
            box_cell([0, 0, 0], [2000, 2000, 2000]),
            box_cell([4000, 0, 0], [6000, 2000, 2000]),
            box_cell([0, 0, 2000], [6000, 2000, 3000]),
        ];
        let (min, max) = cells_bounds(&cells);
        assert!(probe_points_3d(&cells).iter().any(|point| {
            if !inside(&cells, *point) {
                return false;
            }
            let mut below = *point;
            below[2] -= 2;
            below[2] > 0 && !inside(&cells, below)
        }));
        let bridge = unsupported_span_probe(&cells, min, max)
            .expect("the bridge scene has an unsupported span");
        assert!(
            bridge.supported_left.is_some() && bridge.supported_right.is_some(),
            "the span must be bridged on both sides"
        );
        assert!(enclosed_void_probe(&cells, min, max).is_none());
        // A scene with the beam removed has no unsupported span at all.
        let mut without_beam = cells.clone();
        without_beam.pop();
        let (min, max) = cells_bounds(&without_beam);
        assert!(unsupported_span_probe(&without_beam, min, max).is_none());
        // A cantilever plate is a free-edge span, not a bridge.
        let cantilever = vec![
            box_cell([0, 0, 0], [2000, 2000, 2000]),
            box_cell([0, 0, 2000], [4000, 2000, 3000]),
        ];
        let (min, max) = cells_bounds(&cantilever);
        let overhang = unsupported_span_probe(&cantilever, min, max)
            .expect("the cantilever has an unsupported span");
        assert!(
            !(overhang.supported_left.is_some() && overhang.supported_right.is_some()),
            "a cantilever must not be classified as bridged"
        );
        // A cavity scene has an enclosed void and no unsupported span.
        let cavity = vec![
            box_cell([0, 0, 0], [2000, 2000, 500]),
            box_cell([0, 0, 1500], [2000, 2000, 2000]),
            box_cell([0, 0, 500], [500, 2000, 1500]),
            box_cell([1500, 0, 500], [2000, 2000, 1500]),
            box_cell([500, 0, 500], [1500, 500, 1500]),
            box_cell([500, 1500, 500], [1500, 2000, 1500]),
        ];
        let (min, max) = cells_bounds(&cavity);
        let void = enclosed_void_probe(&cavity, min, max).expect("the cavity is enclosed");
        assert!(!inside(&cavity, void));
    }

    fn box_mesh(min: [f64; 3], max: [f64; 3]) -> slicer_ir::IndexedTriangleSet {
        let vertices = vec![
            [min[0], min[1], min[2]],
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            0, 1, 5, 0, 5, 4, // front
            1, 2, 6, 1, 6, 5, // right
            2, 3, 7, 2, 7, 6, // back
            3, 0, 4, 3, 4, 7, // left
        ];
        slicer_ir::IndexedTriangleSet {
            vertices: vertices
                .into_iter()
                .map(|[x, y, z]| slicer_ir::Point3 {
                    x: x as f32,
                    y: y as f32,
                    z: z as f32,
                })
                .collect(),
            indices,
        }
    }

    #[test]
    fn reference_ray_casting_handles_shared_diagonals() {
        let cube = box_mesh([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        validate_indexed_mesh(&cube, true).expect("the hand-built cube is closed");
        // The reviewer probe that exited through a shared diagonal: a naive
        // counting rule reports outside here.
        assert!(point_inside_mesh(&cube, [0.5, 0.313_45, 0.442_55]));
        for probe in [
            [0.5, 0.5, 0.5],
            [0.01, 0.5, 0.99],
            [0.5, 0.01, 0.5],
            [0.999, 0.5, 0.5],
        ] {
            assert!(point_inside_mesh(&cube, probe), "inside {probe:?}");
        }
        for probe in [
            [1.5, 0.5, 0.5],
            [-0.01, 0.5, 0.5],
            [0.5, 1.01, 0.5],
            [0.5, 0.5, 1.01],
        ] {
            assert!(!point_inside_mesh(&cube, probe), "outside {probe:?}");
        }
    }

    fn adapt(text: &str) -> slicer_ir::IndexedTriangleSet {
        use slicer_scad::{ScadEvalOptions, ScadSource};
        let source = ScadSource {
            label: "reference-connectivity".to_string(),
            text: text.to_string(),
        };
        let output = slicer_scad::evaluate(&source, &ScadEvalOptions::default())
            .expect("the adapter renders the connectivity fixtures");
        let slicer_scad::ScadGeometry::Mesh(mesh) = output.geometry else {
            panic!("a solid fixture is a mesh");
        };
        mesh
    }

    #[test]
    fn reference_mesh_connectivity_separates_bodies() {
        // A real converted single body is one surface component.
        let single = adapt("cube(0.2);");
        assert_eq!(mesh_triangle_components(&single), 1);
        // A real converted pair of disjoint bodies is two surface components.
        let separated = adapt("union(){ cube(0.2); translate([0.4,0,0]) cube(0.2); }");
        assert_eq!(mesh_triangle_components(&separated), 2);
        // A converted scene that joins the bodies is one component even though
        // both bodies survive and their centres are far apart.
        let joined =
            adapt("union(){ cube(0.2); translate([0.4,0,0]) cube(0.2); translate([0.2,0.09,0.09]) cube([0.2,0.02,0.02]); }");
        assert_eq!(mesh_triangle_components(&joined), 1);
    }

    #[test]
    fn reference_separated_bodies_rejects_a_connected_mesh() {
        let cells = vec![
            box_cell([0, 0, 0], [2000, 2000, 2000]),
            box_cell([4000, 0, 0], [6000, 2000, 2000]),
        ];
        let separated = adapt("union(){ cube(0.2); translate([0.4,0,0]) cube(0.2); }");
        let volume = mesh_signed_volume_mm3(&separated).expect("volume");
        certify_against_cells(&separated, volume, &cells, SolidWitness::SeparatedBodies)
            .expect("two disjoint bodies certify");
        // Joining the bodies makes the converted surface one component. The
        // certification rejects it even though both body centres are inside and
        // the gap between them is still empty.
        let joined =
            adapt("union(){ cube(0.2); translate([0.4,0,0]) cube(0.2); translate([0.2,0.09,0.09]) cube([0.2,0.02,0.02]); }");
        assert_eq!(mesh_triangle_components(&joined), 1);
        let joined_volume = mesh_signed_volume_mm3(&joined).expect("volume");
        assert!(
            certify_against_cells(&joined, joined_volume, &cells, SolidWitness::SeparatedBodies)
                .is_err(),
            "a surface-connected mesh must not satisfy the separated-body witness"
        );
    }

    #[test]
    fn reference_cylinder_uses_its_regular_polygon_reference() {
        use slicer_scad::{ScadEvalOptions, ScadSource};
        let recipe = ScadRecipe::Solid(Recipe3d::Cylinder {
            radius_units: 4_000,
            height_units: 2_000,
            facets: 12,
        });
        let source = ScadSource {
            label: "cylinder-reference".to_string(),
            text: "cylinder(h = 0.2, r = 0.4, $fn = 12);".to_string(),
        };
        let output = slicer_scad::evaluate(&source, &ScadEvalOptions::default())
            .expect("the adapter renders a faceted cylinder");
        let slicer_scad::ScadGeometry::Mesh(mesh) = &output.geometry else {
            panic!("a cylinder is a mesh");
        };
        let volume = mesh_signed_volume_mm3(mesh).expect("volume");
        certify_solid(&recipe, &output, SolidWitness::ClosedSolid)
            .expect("the cylinder matches its regular-polygon reference");
        let analytic = 0.5 * 12.0 * 0.4 * 0.4 * (std::f64::consts::TAU / 12.0).sin() * 0.2;
        assert!(
            (volume - analytic).abs() < 1e-5 * 0.2_f64.powi(3) + 1e-9,
            "measured {volume} vs analytic {analytic}"
        );
        // Odd facet counts are asymmetric about the Y axis and must still
        // certify; this is the case the symmetric-extrema version got wrong.
        for facets in [10_u16, 13, 15] {
            let recipe = ScadRecipe::Solid(Recipe3d::Cylinder {
                radius_units: 4_000,
                height_units: 2_000,
                facets,
            });
            let source = ScadSource {
                label: format!("cylinder-{facets}"),
                text: format!("cylinder(h = 0.2, r = 0.4, $fn = {facets});"),
            };
            let output = slicer_scad::evaluate(&source, &ScadEvalOptions::default())
                .expect("the adapter renders the odd-facet cylinder");
            certify_solid(&recipe, &output, SolidWitness::ClosedSolid)
                .unwrap_or_else(|error| panic!("{facets}-facet cylinder must certify: {error}"));
        }
        // A recipe claiming a different facet count must not certify.
        let wrong = ScadRecipe::Solid(Recipe3d::Cylinder {
            radius_units: 4_000,
            height_units: 2_000,
            facets: 24,
        });
        assert!(certify_solid(&wrong, &output, SolidWitness::ClosedSolid).is_err());
    }
}
