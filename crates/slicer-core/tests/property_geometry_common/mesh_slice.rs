//! Charter `H-MESH-SLICE`: analytical solids sliced at independently known planes.
//!
//! `slicer_core::slice_mesh_ex` is compared against cross-sections derived from
//! the solid's own dimensions, never against a second `slice_mesh_ex` or Clipper
//! call:
//!
//! * a box interior section is the rectangle `[x0,x0+w] x [y0,y0+d]`;
//! * a right-triangular prism interior section is the XY triangle
//!   `(x0,y0), (x0+w,y0), (x0,y0+d)` extruded over `z0..=z0+h`.
//!
//! Both solids are built here, so every vertex is known before the SUT runs, and
//! every side face is a **vertical** plane. A slice plane therefore meets each
//! intersected edge at a point whose XY coordinates are exactly two of the
//! solid's corner coordinates, so the section is exact on the integer grid and
//! the double area is compared exactly rather than through a float envelope.
//!
//! Boundary convention (pinned by `triangle_mesh_slicer_tdd`, and re-asserted
//! here as deterministic witnesses rather than assumed): a plane exactly at the
//! bottom is **empty** because horizontal facets are ignored, while a plane
//! exactly at the top yields the full section through upper-edge ownership.
//!
//! Membership is sampled only at points whose clearance from the analytic
//! boundary is proved exactly in integer arithmetic here, so a disagreement is
//! never explained by sampling too close to an edge.

use proptest::prelude::*;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_core::slice_mesh_ex;
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Point3, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, signed_area2, validate_indexed_mesh, validate_ring, PointLocation,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{at_least, in_range, read, to_i64, to_usize, BoundsError};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-MESH-SLICE";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_slice_analytical_solids";

/// Sampling clearance from the analytic boundary, in canonical units.
pub const SAMPLE_CLEARANCE: i128 = 2;

/// Plane-count floor: the generated family must be able to emit a multi-plane
/// vector, and the deterministic witnesses use several distinct planes.
pub const MIN_PLANES: u64 = 4;
/// Whole-millimetre extent floor: below 2 mm a solid has no strictly interior
/// whole-millimetre plane.
pub const MIN_EXTENT_MM: u64 = 2;
/// Lattice divisions per axis. A 9x9 lattice per layer keeps `BigRational`
/// membership queries bounded while still covering every face region.
pub const LATTICE_DIVISIONS: i64 = 8;

/// Raw `H-MESH-SLICE` profile keys, before validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSliceBounds {
    /// `max_extent_mm`.
    pub max_extent_mm: u64,
    /// `max_translation_mm`.
    pub max_translation_mm: u64,
    /// `max_planes`.
    pub max_planes: u64,
}

impl RawSliceBounds {
    /// Read every `H-MESH-SLICE` key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_extent_mm: read(run, "max_extent_mm")?,
            max_translation_mm: read(run, "max_translation_mm")?,
            max_planes: read(run, "max_planes")?,
        })
    }
}

/// Validated `H-MESH-SLICE` limits, converted without narrowing loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceLimits {
    /// Largest solid dimension, whole millimetres.
    pub max_extent_mm: i64,
    /// Largest absolute origin coordinate, whole millimetres.
    pub max_translation_mm: i64,
    /// Largest generated `zs` length.
    pub max_planes: usize,
}

/// Reject zero, overflowing and out-of-range slice bounds before sampling.
pub fn checked_limits(raw: RawSliceBounds) -> Result<SliceLimits, BoundsError> {
    in_range("max_extent_mm", raw.max_extent_mm, 1..=64)?;
    at_least(
        "max_extent_mm",
        raw.max_extent_mm,
        "strictly interior whole-millimetre plane",
        MIN_EXTENT_MM,
    )?;
    in_range("max_translation_mm", raw.max_translation_mm, 0..=64)?;
    in_range("max_planes", raw.max_planes, 1..=32)?;
    at_least(
        "max_planes",
        raw.max_planes,
        "multi-plane scene",
        MIN_PLANES,
    )?;
    Ok(SliceLimits {
        max_extent_mm: to_i64("max_extent_mm", raw.max_extent_mm)?,
        max_translation_mm: to_i64("max_translation_mm", raw.max_translation_mm)?,
        max_planes: to_usize("max_planes", raw.max_planes)?,
    })
}

/// Where a slice plane sits relative to the solid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    /// Below or above the solid.
    Outside,
    /// Exactly the bottom plane: empty by the horizontal-facet rule.
    Bottom,
    /// Strictly between bottom and top.
    Interior,
    /// Exactly the top plane: the full section by upper-edge ownership.
    Top,
}

/// One analytical solid, in whole millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Solid {
    /// `true` for a box, `false` for a right-triangular prism.
    pub box_solid: bool,
    /// Minimum X, millimetres.
    pub x0: i64,
    /// Minimum Y, millimetres.
    pub y0: i64,
    /// Minimum Z, millimetres.
    pub z0: i64,
    /// X extent, millimetres.
    pub w: i64,
    /// Y extent, millimetres.
    pub d: i64,
    /// Z extent, millimetres.
    pub h: i64,
}

fn vertex(x: i64, y: i64, z: i64) -> Point3 {
    // `IndexedTriangleSet` vertices are millimetres, not canonical units.
    Point3 {
        x: x as f32,
        y: y as f32,
        z: z as f32,
    }
}

/// Whole millimetres to canonical units, exactly.
///
/// `slicer_ir::mm_to_units` takes an `f32`; every coordinate here is an integer
/// millimetre, so the exact integer product is used instead of a float round
/// trip.
fn units(mm: i64) -> i64 {
    mm * 10_000
}

impl Solid {
    /// The closed, outward-wound indexed triangle set for this solid.
    ///
    /// Every side face is vertical, which is what makes the slice section exact.
    #[must_use]
    pub fn mesh(&self) -> IndexedTriangleSet {
        let (x0, y0, z0) = (self.x0, self.y0, self.z0);
        let (x1, y1, z1) = (x0 + self.w, y0 + self.d, z0 + self.h);
        if self.box_solid {
            IndexedTriangleSet {
                vertices: vec![
                    vertex(x0, y0, z0),
                    vertex(x1, y0, z0),
                    vertex(x1, y1, z0),
                    vertex(x0, y1, z0),
                    vertex(x0, y0, z1),
                    vertex(x1, y0, z1),
                    vertex(x1, y1, z1),
                    vertex(x0, y1, z1),
                ],
                indices: vec![
                    0, 2, 1, 0, 3, 2, // bottom, -Z
                    4, 5, 6, 4, 6, 7, // top, +Z
                    0, 1, 5, 0, 5, 4, // front, -Y
                    1, 2, 6, 1, 6, 5, // right, +X
                    2, 3, 7, 2, 7, 6, // back, +Y
                    3, 0, 4, 3, 4, 7, // left, -X
                ],
            }
        } else {
            IndexedTriangleSet {
                vertices: vec![
                    vertex(x0, y0, z0),
                    vertex(x1, y0, z0),
                    vertex(x0, y1, z0),
                    vertex(x0, y0, z1),
                    vertex(x1, y0, z1),
                    vertex(x0, y1, z1),
                ],
                indices: vec![
                    0, 2, 1, // bottom, -Z
                    3, 4, 5, // top, +Z
                    0, 1, 4, 0, 4, 3, // y = y0 side, -Y
                    0, 3, 5, 0, 5, 2, // x = x0 side, -X
                    1, 2, 5, 1, 5, 4, // hypotenuse, +X +Y
                ],
            }
        }
    }

    /// The CCW analytic cross-section of a strictly interior plane.
    #[must_use]
    pub fn section(&self) -> Vec<ExPolygon> {
        let (x0, y0) = (units(self.x0), units(self.y0));
        let (x1, y1) = (x0 + units(self.w), y0 + units(self.d));
        let points = if self.box_solid {
            vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x1, y: y1 },
                Point2 { x: x0, y: y1 },
            ]
        } else {
            vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x0, y: y1 },
            ]
        };
        vec![ExPolygon {
            contour: Polygon { points },
            holes: Vec::new(),
        }]
    }

    /// Absolute double-area envelope for this solid's section, in squared units.
    ///
    /// A box section is proved exact: every output edge is an input coordinate
    /// and no rounded operation stands between the plane and the corner.
    ///
    /// A prism section has one diagonal edge, and its chained vertices pass
    /// through an `f32` millimetre intersection plus integer-grid rounding.
    /// Moving every vertex of a convex ring by at most one unit changes its
    /// area by at most `perimeter * 1`, so the double area moves by at most
    /// `2 * perimeter`. The perimeter is bounded above by `2 * (w + d)` in
    /// units, because the hypotenuse is shorter than the sum of the legs.
    /// Hence the envelope is `4 * (w + d) * 10_000` squared units, which at the
    /// largest allowed 64 mm side is `5_120_000` squared units, far below one
    /// square millimetre of double area (`2e8`).
    #[must_use]
    pub fn section_double_area_envelope(&self) -> i128 {
        if self.box_solid {
            0
        } else {
            4 * (i128::from(self.w) + i128::from(self.d)) * 10_000
        }
    }

    /// The exact analytic double area of the section, in squared canonical units.
    #[must_use]
    pub fn section_double_area(&self) -> i128 {
        let width = i128::from(units(self.w));
        let depth = i128::from(units(self.d));
        if self.box_solid {
            2 * width * depth
        } else {
            width * depth
        }
    }

    /// The analytic section for one plane, or an empty vector when it is empty.
    ///
    /// A plane below the solid, exactly at its bottom, or above its top yields
    /// nothing; an interior plane and the exact top plane yield the section.
    #[must_use]
    pub fn expected_section(&self, plane_mm: i64) -> Vec<ExPolygon> {
        if plane_mm > self.z0 && plane_mm <= self.z0 + self.h {
            self.section()
        } else {
            Vec::new()
        }
    }

    /// Where `plane_mm` sits relative to the solid.
    #[must_use]
    pub fn boundary_kind(&self, plane_mm: i64) -> BoundaryKind {
        if plane_mm == self.z0 {
            BoundaryKind::Bottom
        } else if plane_mm == self.z0 + self.h {
            BoundaryKind::Top
        } else if plane_mm > self.z0 && plane_mm < self.z0 + self.h {
            BoundaryKind::Interior
        } else {
            BoundaryKind::Outside
        }
    }

    /// Whether `point` lies inside the section with `SAMPLE_CLEARANCE` to spare.
    ///
    /// Exact integer arithmetic: for the prism the hypotenuse test squares both
    /// sides instead of dividing, so no float or rational appears.
    #[must_use]
    pub fn is_interior_clear(&self, point: Point2) -> bool {
        let (x0, y0) = (units(self.x0), units(self.y0));
        let (x1, y1) = (x0 + units(self.w), y0 + units(self.d));
        let (x, y) = (i128::from(point.x), i128::from(point.y));
        let (x0, y0, x1, y1) = (
            i128::from(x0),
            i128::from(y0),
            i128::from(x1),
            i128::from(y1),
        );
        if x < x0 + SAMPLE_CLEARANCE
            || x > x1 - SAMPLE_CLEARANCE
            || y < y0 + SAMPLE_CLEARANCE
            || y > y1 - SAMPLE_CLEARANCE
        {
            return false;
        }
        if self.box_solid {
            return true;
        }
        // Hypotenuse from (x1,y0) to (x0,y1). `numer / sqrt(w^2+d^2)` is the
        // signed distance, positive on the interior side: at (x0,y0) it equals
        // `d * w > 0`.
        let width = x1 - x0;
        let depth = y1 - y0;
        let numer = depth * (x1 - x) - width * (y - y0);
        numer > 0
            && numer * numer
                >= SAMPLE_CLEARANCE * SAMPLE_CLEARANCE * (width * width + depth * depth)
    }

    /// Whether `point` lies outside the section with `SAMPLE_CLEARANCE` to spare.
    #[must_use]
    pub fn is_exterior_clear(&self, point: Point2) -> bool {
        let (x0, y0) = (units(self.x0), units(self.y0));
        let (x1, y1) = (x0 + units(self.w), y0 + units(self.d));
        let (x, y) = (i128::from(point.x), i128::from(point.y));
        let (x0, y0, x1, y1) = (
            i128::from(x0),
            i128::from(y0),
            i128::from(x1),
            i128::from(y1),
        );
        if x <= x0 - SAMPLE_CLEARANCE
            || x >= x1 + SAMPLE_CLEARANCE
            || y <= y0 - SAMPLE_CLEARANCE
            || y >= y1 + SAMPLE_CLEARANCE
        {
            return true;
        }
        if self.box_solid {
            return false;
        }
        // Outside the hypotenuse half-plane by at least the clearance.
        let width = x1 - x0;
        let depth = y1 - y0;
        let numer = depth * (x1 - x) - width * (y - y0);
        numer < 0
            && numer * numer
                >= SAMPLE_CLEARANCE * SAMPLE_CLEARANCE * (width * width + depth * depth)
    }
}

/// Counts proving the checked population was not empty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SliceStats {
    /// Layers the model expects to be nonempty.
    pub expected_nonempty: usize,
    /// Proven-clear interior sample points.
    pub inside_samples: usize,
    /// Proven-clear exterior sample points.
    pub outside_samples: usize,
}

fn pt(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

/// The guaranteed interior/exterior samples plus a bounded lattice.
///
/// The lattice alone can be lattice-thin: dividing the *longest* axis into
/// `LATTICE_DIVISIONS` steps leaves a thin prism's only candidate exactly on
/// its hypotenuse, which made the population check fail on a legitimate case.
/// One analytically guaranteed interior point and four guaranteed exterior
/// points are therefore always included, so "nonempty layer" always yields a
/// nonempty inside population regardless of the lattice spacing.
fn sample_points(solid: &Solid, divisions: i64) -> Vec<Point2> {
    let x0 = units(solid.x0);
    let y0 = units(solid.y0);
    let w = units(solid.w);
    let d = units(solid.d);
    let mut points = Vec::new();
    // Rectangle: the centre. Prism: the centroid, which is at least one third
    // of each leg from the two axis edges and at inradius/3 from the hypotenuse.
    if solid.box_solid {
        points.push(pt(x0 + w / 2, y0 + d / 2));
    } else {
        points.push(pt(x0 + w / 3, y0 + d / 3));
    }
    // Four points eight units outside the bounding box, so every exterior
    // sample passes `is_exterior_clear` without depending on the lattice.
    let pad = 8;
    for (x, y) in [
        (x0 - pad, y0 - pad),
        (x0 + w + pad, y0 - pad),
        (x0 + w + pad, y0 + d + pad),
        (x0 - pad, y0 + d + pad),
    ] {
        points.push(pt(x, y));
    }
    points.extend(lattice(solid, divisions));
    points
}

/// A bounded integer lattice over the section's bounding box.
fn lattice(solid: &Solid, divisions: i64) -> Vec<Point2> {
    let x0 = units(solid.x0) - 2 * SAMPLE_CLEARANCE as i64;
    let y0 = units(solid.y0) - 2 * SAMPLE_CLEARANCE as i64;
    let x1 = x0 + units(solid.w) + 4 * SAMPLE_CLEARANCE as i64;
    let y1 = y0 + units(solid.d) + 4 * SAMPLE_CLEARANCE as i64;
    let step = ((x1 - x0).max(y1 - y0) / divisions.max(1)).max(1);
    let mut out = Vec::new();
    let mut y = y0;
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            out.push(pt(x, y));
            x += step;
        }
        y += step;
    }
    out
}

/// Check one layer against the analytic section.
fn check_layer(
    solid: &Solid,
    plane_mm: i64,
    emitted: &[ExPolygon],
    divisions: i64,
    stats: &mut SliceStats,
) -> Result<(), String> {
    let kind = solid.boundary_kind(plane_mm);
    let expected = solid.expected_section(plane_mm);
    if expected.is_empty() {
        if !emitted.is_empty() {
            return Err(format!(
                "layer z={plane_mm} mm ({kind:?}) must be empty, got {} polygon(s)",
                emitted.len()
            ));
        }
        return Ok(());
    }
    if emitted.is_empty() {
        return Err(format!(
            "layer z={plane_mm} mm ({kind:?}) must be nonempty, got nothing"
        ));
    }
    if emitted.len() != 1 {
        return Err(format!(
            "layer z={plane_mm} mm ({kind:?}) must be one component, got {}",
            emitted.len()
        ));
    }
    for polygon in emitted {
        validate_ring(&polygon.contour).map_err(|error| format!("contour ring: {error}"))?;
        let contour = signed_area2(&polygon.contour).map_err(|error| format!("area: {error}"))?;
        if contour <= 0 {
            return Err(format!(
                "layer z={plane_mm} mm contour is not CCW (double area {contour})"
            ));
        }
        for hole in &polygon.holes {
            validate_ring(hole).map_err(|error| format!("hole ring: {error}"))?;
            let area = signed_area2(hole).map_err(|error| format!("area: {error}"))?;
            if area >= 0 {
                return Err(format!(
                    "layer z={plane_mm} mm hole is not CW (double area {area})"
                ));
            }
        }
    }
    let mut total = 0_i128;
    for polygon in emitted {
        total += signed_area2(&polygon.contour).map_err(|error| format!("area: {error}"))?;
        for hole in &polygon.holes {
            total += signed_area2(hole).map_err(|error| format!("area: {error}"))?;
        }
    }
    let want = solid.section_double_area();
    let envelope = solid.section_double_area_envelope();
    let error = (total - want).abs();
    if error > envelope {
        return Err(format!(
            "layer z={plane_mm} mm double area {total} differs from the dimension-derived {want} by {error}, beyond the derived envelope {envelope}"
        ));
    }
    stats.expected_nonempty += 1;
    for point in sample_points(solid, divisions) {
        if solid.is_interior_clear(point) {
            stats.inside_samples += 1;
            match point_in_region(point, emitted) {
                Ok(PointLocation::Inside) => {}
                other => {
                    return Err(format!(
                    "layer z={plane_mm} mm: interior sample {point:?} is {other:?} in the output"
                ))
                }
            }
        } else if solid.is_exterior_clear(point) {
            stats.outside_samples += 1;
            match point_in_region(point, emitted) {
                Ok(PointLocation::Outside) => {}
                Ok(other) => {
                    return Err(format!(
                        "layer z={plane_mm} mm: exterior sample {point:?} is {other:?} in the output"
                    ))
                }
                Err(error) => {
                    return Err(format!(
                        "layer z={plane_mm} mm: exterior sample {point:?} could not be classified: {error}"
                    ))
                }
            }
        }
    }
    Ok(())
}

/// The independent full check of one `slice_mesh_ex` call.
pub fn check_slice(
    solid: &Solid,
    planes_mm: &[i64],
    emitted: &[Vec<ExPolygon>],
    divisions: i64,
) -> Result<SliceStats, String> {
    if planes_mm.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(format!(
            "model precondition: planes {planes_mm:?} are not strictly increasing"
        ));
    }
    if emitted.len() != planes_mm.len() {
        return Err(format!(
            "slice_mesh_ex returned {} layers for {} planes",
            emitted.len(),
            planes_mm.len()
        ));
    }
    let mut stats = SliceStats::default();
    for (index, plane) in planes_mm.iter().enumerate() {
        check_layer(solid, *plane, &emitted[index], divisions, &mut stats)?;
    }
    Ok(stats)
}

/// A generated slice scene: one solid and its strictly increasing whole-mm planes.
#[derive(Debug, Clone, PartialEq)]
pub struct SliceScene {
    /// The analytical solid.
    pub solid: Solid,
    /// Strictly increasing whole-millimetre planes.
    pub planes_mm: Vec<i64>,
}

/// The `H-MESH-SLICE` scene strategy, exposed so the owned-shrink control can
/// drive the exact tree this property consumes.
pub fn scene_strategy(limits: SliceLimits) -> impl Strategy<Value = SliceScene> {
    let extent = MIN_EXTENT_MM as i64..=limits.max_extent_mm;
    let origin = -limits.max_translation_mm..=limits.max_translation_mm;
    let max_planes = limits.max_planes;
    (
        any::<bool>(),
        extent.clone(),
        extent.clone(),
        extent,
        origin.clone(),
        origin.clone(),
        origin,
        1_usize..=max_planes,
        -2_i64..=4_i64,
        1_i64..=2,
    )
        .prop_map(move |(box_solid, w, d, h, x0, y0, z0, count, base, gap)| {
            let planes_mm = (0..count)
                .map(|index| z0 + base + i64::try_from(index).unwrap_or(0) * gap)
                .collect();
            SliceScene {
                solid: Solid {
                    box_solid,
                    x0,
                    y0,
                    z0,
                    w,
                    d,
                    h,
                },
                planes_mm,
            }
        })
}

/// Check one generated scene end to end.
pub fn check_scene(scene: &SliceScene) -> TestCaseResult {
    let mesh = scene.solid.mesh();
    if let Err(error) = validate_indexed_mesh(&mesh, true) {
        return Err(TestCaseError::fail(format!(
            "the constructed solid violates its own closed-mesh precondition: {error}"
        )));
    }
    let planes: Vec<f32> = scene.planes_mm.iter().map(|mm| *mm as f32).collect();
    if planes.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(TestCaseError::fail(
            "generated planes are not strictly increasing after narrowing to f32",
        ));
    }
    let emitted = slice_mesh_ex(&mesh, &planes);
    match check_slice(&scene.solid, &scene.planes_mm, &emitted, LATTICE_DIVISIONS) {
        Ok(stats) => {
            // A scene whose planes are all outside legitimately has no nonempty
            // layer; when it does have one, the sampled population must be real.
            if stats.expected_nonempty > 0 && stats.inside_samples == 0 {
                return Err(TestCaseError::fail(format!(
                    "vacuous inside population for {} nonempty layers: {stats:?}",
                    stats.expected_nonempty
                )));
            }
            Ok(())
        }
        Err(error) => Err(TestCaseError::fail(error)),
    }
}

fn assert_layer(solid: &Solid, plane_mm: i64) {
    let mesh = solid.mesh();
    validate_indexed_mesh(&mesh, true)
        .unwrap_or_else(|error| panic!("fixture {solid:?} is not a closed mesh: {error}"));
    let emitted = slice_mesh_ex(&mesh, &[plane_mm as f32]);
    check_slice(solid, &[plane_mm], &emitted, LATTICE_DIVISIONS).unwrap_or_else(|error| {
        panic!("H-MESH-SLICE witness {solid:?} z={plane_mm} mm failed: {error}")
    });
}

/// The pre-existing vertex-touching tetrahedron, with its analytic triangle.
///
/// This fixture is pinned by `triangle_mesh_slicer_tdd`; the plane passes
/// exactly through one of its vertices, a different regime from a
/// whole-millimetre interior plane.
fn vertex_touching_tetrahedron() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 2.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
        ],
        indices: vec![0, 1, 2, 0, 3, 1, 0, 2, 3, 1, 3, 2],
    }
}

/// Deterministic witnesses: every boundary convention, both solids.
///
/// Run before `PropertyRun::new`, so no convention depends on RNG frequency.
pub fn deterministic_witnesses(limits: SliceLimits) {
    let extent = limits.max_extent_mm.clamp(MIN_EXTENT_MM as i64, 4);
    for box_solid in [true, false] {
        let solid = Solid {
            box_solid,
            x0: -1,
            y0: 0,
            z0: -2,
            w: extent,
            d: (extent - 1).max(2),
            h: (extent - 1).max(2),
        };
        for plane in [
            solid.z0 - 1,
            solid.z0,
            solid.z0 + 1,
            solid.z0 + solid.h - 1,
            solid.z0 + solid.h,
            solid.z0 + solid.h + 1,
        ] {
            assert_layer(&solid, plane);
        }
        let mesh = solid.mesh();
        assert!(
            slice_mesh_ex(&mesh, &[]).is_empty(),
            "H-MESH-SLICE: an empty plane vector must return an empty layer vector"
        );
        let empty = slice_mesh_ex(&IndexedTriangleSet::default(), &[0.0]);
        assert_eq!(
            empty.len(),
            1,
            "H-MESH-SLICE: an empty mesh still yields one (empty) layer per plane"
        );
        assert!(
            empty[0].is_empty(),
            "H-MESH-SLICE: an empty mesh layer must be empty"
        );
    }

    // The through-vertex witness: an analytically expected triangle.
    let emitted = slice_mesh_ex(&vertex_touching_tetrahedron(), &[0.5]);
    assert_eq!(emitted.len(), 1, "H-MESH-SLICE: one plane, one layer");
    let layer = &emitted[0];
    assert_eq!(
        layer.len(),
        1,
        "H-MESH-SLICE: the vertex-touching tetrahedron must yield one component"
    );
    let half = units(1) / 2;
    let expected = Polygon {
        points: vec![pt(0, 0), pt(half, units(1)), pt(-half, units(1))],
    };
    let want = signed_area2(&expected).expect("analytic triangle area");
    let got = signed_area2(&layer[0].contour).expect("emitted contour area");
    assert_eq!(
        got, want,
        "H-MESH-SLICE: the through-vertex triangle double area must be {want}, got {got}"
    );
    validate_ring(&layer[0].contour).expect("H-MESH-SLICE: through-vertex ring is valid");
}

/// Run the registry-selected `H-MESH-SLICE` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-MESH-SLICE registry context");
    let raw = RawSliceBounds::read(&run).expect("H-MESH-SLICE bound keys");
    let limits = checked_limits(raw).expect("H-MESH-SLICE checked profile bounds");
    deterministic_witnesses(limits);
    let strategy = scene_strategy(limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene))
        .expect("H-MESH-SLICE analytical cross-sections");
}
