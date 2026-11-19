//! Charter `H-MESH-SCAD`: scripted SCAD solids sliced at independent planes.
//!
//! `scripted_cases` (feature `slicer-sdk/property_testing_scad`) converts a
//! typed solid recipe to an **actual** [`slicer_ir::IndexedTriangleSet`]. That
//! mesh is stimulus, never an expected answer. It is validated with #2
//! `validate_indexed_mesh(.., true)` and required to have a positive
//! `mesh_signed_volume_mm3`, then sliced with the production
//! `slicer_core::slice_mesh_ex` at a small number of strictly increasing finite
//! planes chosen inside the mesh's Z range.
//!
//! ## The independent model
//!
//! The reference is a small ray-parity model computed **only from the input
//! triangles**, never from `AabbTree`, `slice_mesh_ex`, Clipper or the slicer's
//! own chaining algorithm:
//!
//! * **Membership.** For a query point `(x, y, z)` on the plane, cast the
//!   vertical `+Z` ray and count the triangles whose projected XY triangle
//!   contains `(x, y)` strictly and whose own plane is crossed strictly above
//!   `z`. An odd count means the point is inside the solid.
//! * **Structure.** Each triangle that straddles the plane contributes one
//!   section segment whose endpoints lie on two of the mesh's *edges*. A
//!   straight edge crosses a plane at most once, and the chosen planes are clear
//!   of every vertex, so two section segments belong to the same closed section
//!   curve exactly when they share such a crossed edge. A union-find over
//!   mesh-edge identities therefore counts the closed curves exactly, without
//!   chaining and without any orientation assembly. Each segment is also
//!   oriented from its triangle's outward normal (`d = (-n_y, n_x)` puts material
//!   on the left for a counter-clockwise-outward surface), so the *per-curve*
//!   signed double area is a sum over its own unordered segments, and its sign
//!   says whether that curve is a material boundary (positive) or a hole
//!   (negative).
//!
//! The emitted layer must therefore have exactly `outer_curves` components,
//! exactly `hole_curves` total holes, and a total double area matching the
//! model's signed sum. A filled cavity, a dropped body or an all-outside answer
//! breaks one of those identities, and the membership samples besides.
//!
//! ## Documented limitations
//!
//! * The ray-parity model runs on `f64` widened from the mesh's `f32`
//!   millimetre vertices and is exact only away from ambiguous configurations.
//!   It explicitly **excludes** rather than resolves them: a query whose XY lies
//!   within [`EDGE_CLEARANCE_UNITS`] canonical units of any *projected* triangle
//!   edge is dropped (the ray would graze a triangle edge or vertex), a query
//!   whose XY lies within [`SECTION_CLEARANCE_UNITS`] of any model section
//!   segment is dropped (the emitted boundary is a grid rounding of that
//!   segment), and a query is dropped when a triangle is coplanar with the plane
//!   to within [`COPLANAR_CLEARANCE_MM`]. Excluded queries are counted in
//!   [`PlaneStats::excluded`], never silently accepted.
//! * Planes are the midpoints of the largest vertex-Z gaps, so no plane passes
//!   through a vertex. The through-vertex and exact top/bottom regimes belong to
//!   the direct `H-MESH-SLICE` charter, which pins them analytically; this model
//!   does not claim them.
//! * Parity over a closed surface proves membership, not the *location* of the
//!   boundary between samples. The boundary is pinned to a band of
//!   [`PROBE_OFFSET_UNITS`] units by the perpendicular probe pairs placed on both
//!   sides of every model section segment.
//! * The section model assumes a consistently outward-oriented closed surface
//!   (checked by `validate_indexed_mesh` plus the positive signed volume) and
//!   non-degenerate triangle projections. A triangle whose projection is exactly
//!   degenerate cannot be crossed by the vertical ray at all, so it is skipped.
//! * `f32` plane values are checked to keep [`PLANE_VERTEX_CLEARANCE_MM`] from
//!   every distinct vertex Z, so an ambiguous coplanar or through-vertex hit is
//!   never manufactured by the plane choice.
//!
//! ## Why the two clearances are 8 units
//!
//! Both are sized against the largest coordinate the registry profiles admit,
//! not against the sizes a particular seed happens to draw. `checked_limits`
//! caps `max_abs_units` at 1_000_000 canonical units, i.e. **M = 100 mm** at
//! 10⁴ units/mm; the measured profiles are far inside that (routine
//! 20_000 units = 2 mm, extended 40_000 = 4 mm). With `u = 2⁻⁵³` and
//! `γₙ = n·u/(1 − n·u)`, and with every operand of a difference bounded by the
//! segment's diagonal `2√2·M`:
//!
//! * **Barycentric weights and crossing heights.** `ray_parity` evaluates a
//!   determinant and a numerator that are each a sum of two products of
//!   coordinate differences, then one quotient: `n = 15` rounding operations,
//!   so each weight carries a relative error of at most `γ₁₅ ≈ 1.7e-15` and its
//!   *sign* is exact whenever the exact weight is nonzero. The resulting
//!   crossing height is a convex combination of three vertex Z values, each at
//!   most `M`, so its absolute error is at most `3·γ₁₅·M ≈ 5e-13 mm` — six
//!   orders of magnitude below [`COPLANAR_CLEARANCE_MM`]. `crossing` adds two
//!   operations to a quotient and three to a multiply-add (`n = 6`), so a model
//!   section endpoint moves by at most `γ₆·2M ≈ 1.3e-13 mm`.
//! * **[`EDGE_CLEARANCE_UNITS`].** `distance_to_segment` is a clamped projection
//!   plus a `hypot` (`n ≤ 15`, a conservative count of its subexpressions), and
//!   every intermediate is bounded by the segment's diagonal, so the computed
//!   distance differs from the exact one by at most
//!   `γ₁₅·2√2·M ≈ 4.7e-13 mm = 4.7e-9 units`. **8 units = 8.0e-4 mm**
//!   exceeds that by a factor of 1.7e9, so no query that passed the filter is
//!   within arithmetically detectable distance of a projected edge. The same
//!   clearance covers projection degeneracy: a triangle whose projection is
//!   thinner than the clearance has no interior point farther than half its own
//!   altitude from its boundary, so every query inside it would be excluded,
//!   and a projection with exactly zero area is skipped by the
//!   `determinant == 0.0` guard.
//! * **[`SECTION_CLEARANCE_UNITS`].** The emitted layer boundary is a grid
//!   rounding of the exact section by at most **one unit** per vertex (see
//!   `super::envelopes`), which dominates: the model segment is within
//!   `1.3e-13 mm = 1.3e-9 units` of the exact section and the query's own
//!   conversion to `f64` millimetres is within `u·M ≈ 1.1e-14 mm`. The bound is
//!   therefore `1 + 1.3e-9` units, so **8 units exceeds it by a factor of 8**
//!   and leaves about 7 units between the query and the emitted boundary — far
//!   beyond what the exact `BigRational` predicate in `point_in_region` needs to
//!   avoid reporting `Boundary`.
//!
//! Neither number is a tolerance applied to a result: both are *exclusions*. A
//! query inside either band is dropped and counted in [`PlaneStats::excluded`],
//! never accepted with a widened expectation.
//!
//! `max_queries` bounds the samples checked per plane and `max_planes` bounds
//! the probed planes; both are read from the registry profile, as the packet's
//! two scripted records carry both keys.

use std::collections::HashMap;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::slice_mesh_ex;
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Polygon};
use slicer_scad::ScadGeometry;
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, GeometryBounds, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    mesh_signed_volume_mm3, point_in_region, signed_area2, validate_indexed_mesh, PointLocation,
};
use slicer_sdk::test_support::property_testing::scripted::{
    emit_scad, evaluate_recipe, scripted_cases, Recipe3d, RecipeBounds, ScadRecipe, ScriptedCase,
    ScriptedDomain, SolidWitness,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{at_least, in_range, BoundsError};
use super::envelopes::{region_double_area, UNITS_PER_MM};
use super::scad_boolean::RawScadBounds;

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-MESH-SCAD";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_scripted_mesh_cross_sections";

/// Canonical units per millimetre, as a float for the mesh-side arithmetic.
pub const UNITS_PER_MM_F64: f64 = UNITS_PER_MM as f64;

/// Clearance, in canonical units, of a query from every *projected* triangle
/// edge. Below it the `+Z` ray could graze a triangle edge or vertex and the
/// crossing count would be ambiguous, so the query is excluded.
pub const EDGE_CLEARANCE_UNITS: f64 = 8.0;

/// Clearance, in canonical units, of a query from every model section segment.
///
/// The emitted cross-section boundary is a grid rounding of the exact section
/// (at most one unit per vertex; see [`super::envelopes`]), so a query at this
/// distance can never be a boundary question for `point_in_region`.
pub const SECTION_CLEARANCE_UNITS: f64 = 8.0;

/// Perpendicular probe offset, in canonical units, used to place samples on
/// both sides of every model section segment.
pub const PROBE_OFFSET_UNITS: f64 = 64.0;

/// A triangle is treated as coplanar with the query plane when all three of its
/// vertices are within this distance of it; such a triangle is excluded, not
/// resolved. It is far below [`PLANE_VERTEX_CLEARANCE_MM`], so it never fires for
/// a plane this module chooses.
pub const COPLANAR_CLEARANCE_MM: f64 = 1e-6;

/// Minimum distance from a chosen plane to every vertex Z, in millimetres.
pub const PLANE_VERTEX_CLEARANCE_MM: f64 = 1e-5;

/// Lattice divisions per axis for the far-field query candidates.
pub const LATTICE_DIVISIONS: i64 = 8;

/// Smallest accepted `max_planes`: the two-gap solid families (bridge,
/// overhang) need two probes to reach both their lower and upper regions.
pub const MIN_PLANES: u64 = 2;
/// Safety ceiling on `max_planes`.
pub const PLANES_CEILING: u64 = 64;

/// Validated `H-MESH-SCAD` limits.
#[derive(Debug, Clone)]
pub struct MeshLimits {
    /// Domain bounds for the convex constituent profile.
    pub geometry: GeometryBounds,
    /// Recipe bounds for emission and evaluation accounting.
    pub recipe: RecipeBounds,
    /// Adapter options derived from [`MeshLimits::recipe`].
    pub options: slicer_scad::ScadEvalOptions,
    /// Largest sample vector per probed plane.
    pub max_queries: usize,
    /// Largest number of probed planes.
    pub max_planes: usize,
}

/// Reject zero, overflowing and out-of-range scripted bounds before sampling.
///
/// The shared [`RawScadBounds`] reader and validator are reused verbatim, so both
/// scripted records check the identical key set and fail an invalid profile
/// before a strategy or a value tree exists.
pub fn checked_limits(raw: RawScadBounds) -> Result<MeshLimits, BoundsError> {
    let contour = super::scad_boolean::checked_limits(raw)?;
    // Mesh-specific: the plane-count bound must admit the multi-plane probes
    // every two-region solid witness needs.
    in_range("max_planes", raw.max_planes, 1..=PLANES_CEILING)?;
    at_least(
        "max_planes",
        raw.max_planes,
        "two-region plane probe",
        MIN_PLANES,
    )?;
    at_least("max_queries", raw.max_queries, "plane sample population", 4)?;
    Ok(MeshLimits {
        geometry: contour.geometry,
        recipe: contour.recipe,
        options: contour.options,
        max_queries: contour.max_queries,
        max_planes: contour.max_planes,
    })
}

/// The scripted solid domain for one requested witness.
///
/// `scripted_cases` requires a constituent planar domain that is convex, has no
/// required holes and at most one required component; the requested
/// [`SolidWitness`] carries the actual solid structure.
pub fn domain(witness: SolidWitness, limits: &MeshLimits) -> ScriptedDomain {
    ScriptedDomain {
        // exhaustive: pins the constituent profile the solid families require
        geometry: GeometryDomain {
            class: TopologyClass::Convex,
            min_holes: 0,
            min_components: 1,
            boundary: BoundaryRule::Strict,
            bounds: limits.geometry.clone(),
        },
        solid_witness: Some(witness),
        bounds: limits.recipe,
    }
}

/// The five solid witnesses, in a fixed order.
pub const WITNESSES: [SolidWitness; 5] = [
    SolidWitness::ClosedSolid,
    SolidWitness::Cavity,
    SolidWitness::Bridge,
    SolidWitness::Overhang,
    SolidWitness::SeparatedBodies,
];

/// Build the `H-MESH-SCAD` strategy: one certified solid per sample, tagged with
/// the witness whose branch produced it.
///
/// The tag is fixed per branch, and proptest's `Union` value tree stays inside
/// the branch that produced the value, so the requested witness survives
/// shrinking; each exposed value is re-certified by `scripted_cases` because the
/// retained case carries its own recipe, source and adapter output.
pub fn strategy(
    limits: &MeshLimits,
) -> Result<BoxedStrategy<(SolidWitness, ScriptedCase)>, BoundsError> {
    let mut branches = Vec::new();
    for witness in WITNESSES {
        let cases =
            scripted_cases(domain(witness, limits), limits.options.clone()).map_err(|error| {
                BoundsError::Arithmetic {
                    what: format!("scripted {witness:?} solid domain: {error}"),
                }
            })?;
        branches.push(cases.prop_map(move |case| (witness, case)).boxed());
    }
    Ok(Union::new(branches).boxed())
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent section model.
// ─────────────────────────────────────────────────────────────────────────────

/// One triangle widened to `f64`, in millimetres.
#[derive(Debug, Clone, Copy)]
struct Triangle {
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
}

fn triangle_at(mesh: &IndexedTriangleSet, corners: &[u32; 3]) -> Result<Triangle, String> {
    let mut widened = [[0.0_f64; 3]; 3];
    for (slot, index) in corners.iter().enumerate() {
        let vertex = mesh
            .vertices
            .get(*index as usize)
            .ok_or_else(|| format!("triangle index {index} is outside the vertex array"))?;
        widened[slot] = [
            f64::from(vertex.x),
            f64::from(vertex.y),
            f64::from(vertex.z),
        ];
        if widened[slot].iter().any(|value| !value.is_finite()) {
            return Err(format!("triangle {corners:?} has a non-finite vertex"));
        }
    }
    Ok(Triangle {
        a: widened[0],
        b: widened[1],
        c: widened[2],
    })
}

/// One section segment in the XY plane, in millimetres.
#[derive(Debug, Clone, Copy)]
pub struct SectionSegment {
    /// First endpoint.
    pub a: [f64; 2],
    /// Second endpoint: walking `a -> b` keeps the material on the left.
    pub b: [f64; 2],
    /// Mesh edge (sorted vertex indices) crossed at `a`.
    pub from: u64,
    /// Mesh edge (sorted vertex indices) crossed at `b`.
    pub to: u64,
}

/// The independent planar section of a closed triangle mesh at one Z.
#[derive(Debug, Clone, Default)]
pub struct SectionModel {
    /// Oriented section segments, in millimetres.
    pub segments: Vec<SectionSegment>,
    /// Number of closed section curves.
    pub curves: usize,
    /// Curves with positive signed area: material boundaries.
    pub outer_curves: usize,
    /// Curves with negative signed area: holes.
    pub hole_curves: usize,
    /// Signed double area of the whole section, in squared canonical units.
    pub signed_double_area_units2: f64,
    /// Total section length, in canonical units.
    pub perimeter_units: f64,
}

fn edge_key(left: u32, right: u32) -> u64 {
    let (low, high) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    (u64::from(low) << 32) | u64::from(high)
}

#[derive(Debug, Default)]
struct DisjointSet {
    parent: Vec<usize>,
    depth: Vec<u8>,
}

impl DisjointSet {
    fn insert(&mut self) -> usize {
        let index = self.parent.len();
        self.parent.push(index);
        self.depth.push(0);
        index
    }

    fn find(&mut self, mut node: usize) -> usize {
        while self.parent[node] != node {
            let grandparent = self.parent[self.parent[node]];
            self.parent[node] = grandparent;
            node = grandparent;
        }
        node
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left == right {
            return;
        }
        let (deep, shallow) = if self.depth[left] >= self.depth[right] {
            (left, right)
        } else {
            (right, left)
        };
        self.parent[shallow] = deep;
        if self.depth[deep] == self.depth[shallow] {
            self.depth[deep] += 1;
        }
    }
}

/// Interpolate the crossing of the edge `p -> q` with the plane `z`.
fn crossing(p: [f64; 3], q: [f64; 3], z: f64) -> [f64; 2] {
    let t = (z - p[2]) / (q[2] - p[2]);
    [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]
}

/// Build the independent section of `mesh` at `z`, in millimetres.
///
/// # Errors
/// Returns a message when the plane is too close to a vertex, a segment's
/// direction cannot be resolved, or a crossing count is not two; those are model
/// preconditions and are reported, never silently skipped.
pub fn section_model(mesh: &IndexedTriangleSet, z: f64) -> Result<SectionModel, String> {
    let mut set = DisjointSet::default();
    let mut edge_slots: HashMap<u64, usize> = HashMap::new();
    let mut segments: Vec<SectionSegment> = Vec::new();
    for chunk in mesh.indices.chunks_exact(3) {
        let corners = [chunk[0], chunk[1], chunk[2]];
        let triangle = triangle_at(mesh, &corners)?;
        let points = [triangle.a, triangle.b, triangle.c];
        let heights = points.map(|point| point[2] - z);
        if heights
            .iter()
            .any(|height| height.abs() < PLANE_VERTEX_CLEARANCE_MM)
        {
            return Err(format!(
                "model precondition: plane z={z} mm is within {PLANE_VERTEX_CLEARANCE_MM} mm of a \
                 vertex of triangle {corners:?}"
            ));
        }
        if heights.iter().all(|height| *height > 0.0) || heights.iter().all(|height| *height < 0.0)
        {
            continue;
        }
        // Mixed signs: exactly two of the three edges cross the plane.
        let mut crossed: Vec<([f64; 3], [f64; 3], u64)> = Vec::new();
        for (slot, other) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
            if (heights[slot] > 0.0) != (heights[other] > 0.0) {
                crossed.push((
                    points[slot],
                    points[other],
                    edge_key(corners[slot], corners[other]),
                ));
            }
        }
        if crossed.len() != 2 {
            return Err(format!(
                "triangle {corners:?} has {} plane crossings, expected exactly two",
                crossed.len()
            ));
        }
        let first = crossing(crossed[0].0, crossed[0].1, z);
        let second = crossing(crossed[1].0, crossed[1].1, z);
        // Outward normal from the counter-clockwise-outward winding.
        let edge1 = [
            triangle.b[0] - triangle.a[0],
            triangle.b[1] - triangle.a[1],
            triangle.b[2] - triangle.a[2],
        ];
        let edge2 = [
            triangle.c[0] - triangle.a[0],
            triangle.c[1] - triangle.a[1],
            triangle.c[2] - triangle.a[2],
        ];
        let normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];
        // `d = (-n_y, n_x)` runs along the section with material on its left.
        let direction = [-normal[1], normal[0]];
        let along = [second[0] - first[0], second[1] - first[1]];
        let dot = along[0] * direction[0] + along[1] * direction[1];
        let magnitude = along[0].hypot(along[1]) * direction[0].hypot(direction[1]);
        if magnitude == 0.0 || dot.abs() <= 1e-12 * magnitude {
            return Err(format!(
                "triangle {corners:?} yields an ambiguous section direction (|e||d|={magnitude}, \
                 dot={dot})"
            ));
        }
        let (start, start_edge, end, end_edge) = if dot > 0.0 {
            (first, crossed[0].2, second, crossed[1].2)
        } else {
            (second, crossed[1].2, first, crossed[0].2)
        };
        let start_slot = *edge_slots.entry(start_edge).or_insert_with(|| set.insert());
        let end_slot = *edge_slots.entry(end_edge).or_insert_with(|| set.insert());
        set.union(start_slot, end_slot);
        segments.push(SectionSegment {
            a: start,
            b: end,
            from: start_edge,
            to: end_edge,
        });
    }
    let mut per_curve: HashMap<usize, f64> = HashMap::new();
    let mut perimeter_units = 0.0_f64;
    for segment in &segments {
        let root = set.find(
            *edge_slots
                .get(&segment.from)
                .ok_or_else(|| String::from("a section segment lost its mesh-edge identity"))?,
        );
        // Twice the signed area of the triangle (origin, a, b).
        let double_area_mm2 = segment.a[0] * segment.b[1] - segment.b[0] * segment.a[1];
        *per_curve.entry(root).or_insert(0.0) += double_area_mm2;
        perimeter_units +=
            (segment.b[0] - segment.a[0]).hypot(segment.b[1] - segment.a[1]) * UNITS_PER_MM_F64;
    }
    let mut model = SectionModel {
        segments,
        perimeter_units,
        ..SectionModel::default()
    };
    for area_mm2 in per_curve.values() {
        // A closed curve accumulates twice its signed region area.
        let signed = area_mm2 * UNITS_PER_MM_F64 * UNITS_PER_MM_F64;
        model.signed_double_area_units2 += signed;
        if signed > 0.0 {
            model.outer_curves += 1;
        } else if signed < 0.0 {
            model.hole_curves += 1;
        } else {
            return Err(String::from(
                "a section curve has zero signed area, so it is neither material nor hole",
            ));
        }
    }
    model.curves = model.outer_curves + model.hole_curves;
    Ok(model)
}

/// Distance from `point` to the closed segment `[a, b]`, in the segments' unit.
fn distance_to_segment(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length_sq = dx * dx + dy * dy;
    if length_sq == 0.0 {
        return (point[0] - a[0]).hypot(point[1] - a[1]);
    }
    let t = (((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / length_sq).clamp(0.0, 1.0);
    (point[0] - (a[0] + t * dx)).hypot(point[1] - (a[1] + t * dy))
}

// ─────────────────────────────────────────────────────────────────────────────
// Ray-parity membership model.
// ─────────────────────────────────────────────────────────────────────────────

/// The vertical-ray membership verdict for one query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// An odd number of upward crossings.
    Inside,
    /// An even number of upward crossings.
    Outside,
}

/// Count the vertical `+Z` ray crossings of `mesh` from `(x, y, z)`.
///
/// `None` means an ambiguous configuration was met (a triangle coplanar with the
/// query plane); the caller excludes the query instead of resolving it.
pub fn ray_parity(
    mesh: &IndexedTriangleSet,
    x: f64,
    y: f64,
    z: f64,
) -> Result<Option<Verdict>, String> {
    let mut crossings = 0_usize;
    for chunk in mesh.indices.chunks_exact(3) {
        let triangle = triangle_at(mesh, &[chunk[0], chunk[1], chunk[2]])?;
        let points = [triangle.a, triangle.b, triangle.c];
        if points
            .iter()
            .all(|point| (point[2] - z).abs() < COPLANAR_CLEARANCE_MM)
        {
            return Ok(None);
        }
        // Barycentric weights of `(x, y)` in the projected triangle.
        let (ax, ay) = (triangle.a[0], triangle.a[1]);
        let (bx, by) = (triangle.b[0], triangle.b[1]);
        let (cx, cy) = (triangle.c[0], triangle.c[1]);
        let determinant = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
        if determinant == 0.0 {
            // The projection is a segment, so the vertical ray cannot cross the
            // triangle's interior: it can only graze its plane.
            continue;
        }
        let wa = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / determinant;
        let wb = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / determinant;
        let wc = 1.0 - wa - wb;
        if wa <= 0.0 || wb <= 0.0 || wc <= 0.0 {
            continue;
        }
        let crossing_z = wa * triangle.a[2] + wb * triangle.b[2] + wc * triangle.c[2];
        if (crossing_z - z).abs() < COPLANAR_CLEARANCE_MM {
            return Ok(None);
        }
        if crossing_z > z {
            crossings += 1;
        }
    }
    Ok(Some(if crossings % 2 == 1 {
        Verdict::Inside
    } else {
        Verdict::Outside
    }))
}

/// Whether `(x, y)` lies within `clearance` of any projected triangle edge.
fn near_projected_edge(
    mesh: &IndexedTriangleSet,
    point: [f64; 2],
    clearance: f64,
) -> Result<bool, String> {
    let clearance_sq = clearance * clearance;
    for chunk in mesh.indices.chunks_exact(3) {
        let triangle = triangle_at(mesh, &[chunk[0], chunk[1], chunk[2]])?;
        let projected = [
            [triangle.a[0], triangle.a[1]],
            [triangle.b[0], triangle.b[1]],
            [triangle.c[0], triangle.c[1]],
        ];
        for (slot, other) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
            let distance = distance_to_segment(point, projected[slot], projected[other]);
            if distance * distance < clearance_sq {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Plane selection.
// ─────────────────────────────────────────────────────────────────────────────

/// Choose up to `max_planes` strictly increasing planes inside the mesh's Z
/// range, each the midpoint of one of the largest vertex-Z gaps.
///
/// A vertex gap at or below `2 * PLANE_VERTEX_CLEARANCE_MM` cannot host an
/// unambiguous plane and is not a candidate; running out of candidates is
/// reported, not silently skipped.
pub fn planes(mesh: &IndexedTriangleSet, max_planes: usize) -> Result<Vec<f32>, String> {
    let mut heights: Vec<f64> = mesh.vertices.iter().map(|v| f64::from(v.z)).collect();
    if heights.iter().any(|height| !height.is_finite()) {
        return Err(String::from("the mesh has a non-finite vertex Z"));
    }
    heights.sort_by(|left, right| left.partial_cmp(right).expect("finite Z"));
    heights.dedup();
    if heights.len() < 2 {
        return Err(format!(
            "the mesh has {} distinct vertex Z value(s), so it has no sliceable extent",
            heights.len()
        ));
    }
    let mut gaps: Vec<(usize, f64)> = heights
        .windows(2)
        .enumerate()
        .map(|(index, pair)| (index, pair[1] - pair[0]))
        .filter(|(_, gap)| *gap > 2.0 * PLANE_VERTEX_CLEARANCE_MM)
        .collect();
    if gaps.is_empty() {
        return Err(String::from(
            "no vertex-Z gap is wide enough to host a plane clear of both its vertices",
        ));
    }
    gaps.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .expect("finite gap")
            .then(left.0.cmp(&right.0))
    });
    let mut selected: Vec<f64> = gaps
        .iter()
        .take(max_planes)
        .map(|(index, gap)| heights[*index] + gap / 2.0)
        .collect();
    selected.sort_by(|left, right| left.partial_cmp(right).expect("finite plane"));
    let planes: Vec<f32> = selected.iter().map(|z| *z as f32).collect();
    for plane in &planes {
        if !plane.is_finite() {
            return Err(format!("plane {plane} is not finite"));
        }
        for height in &heights {
            if (f64::from(*plane) - height).abs() < PLANE_VERTEX_CLEARANCE_MM {
                return Err(format!(
                    "plane {plane} mm is within {PLANE_VERTEX_CLEARANCE_MM} mm of the vertex Z \
                     {height}"
                ));
            }
        }
    }
    if planes.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(format!(
            "planes {planes:?} are not strictly increasing after narrowing to f32"
        ));
    }
    Ok(planes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-plane check.
// ─────────────────────────────────────────────────────────────────────────────

/// Observed populations of one probed plane.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlaneStats {
    /// The plane, in millimetres.
    pub plane_z: f32,
    /// Model closed curves.
    pub curves: usize,
    /// Model material curves.
    pub outer_curves: usize,
    /// Model hole curves.
    pub hole_curves: usize,
    /// Emitted layer components.
    pub layer_components: usize,
    /// Emitted layer holes.
    pub layer_holes: usize,
    /// Queries the model classified as inside.
    pub inside_samples: usize,
    /// Queries the model classified as outside.
    pub outside_samples: usize,
    /// Queries excluded as ambiguous (edge, section or coplanar clearance).
    pub excluded: usize,
}

fn signed_double_area(ring: &Polygon) -> Result<i128, String> {
    signed_area2(ring).map_err(|error| format!("ring double area: {error}"))
}

/// A model query in canonical units.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Query {
    point: Point2,
    verdict: Verdict,
}

/// Integer canonical-unit span of the mesh's XY bounds, padded.
fn padded_axes(mesh: &IndexedTriangleSet) -> Result<[[i64; 2]; 2], String> {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for vertex in &mesh.vertices {
        for (axis, value) in [(0_usize, vertex.x), (1, vertex.y)] {
            min[axis] = min[axis].min(f64::from(value));
            max[axis] = max[axis].max(f64::from(value));
        }
    }
    if min.iter().chain(max.iter()).any(|value| !value.is_finite()) {
        return Err(String::from("the mesh has non-finite bounds"));
    }
    let mut axes = [[0_i64; 2]; 2];
    for axis in 0..2 {
        let lowest = (min[axis] * UNITS_PER_MM_F64).round();
        let highest = (max[axis] * UNITS_PER_MM_F64).round();
        let pad = ((highest - lowest).abs() * 0.125).max(4.0 * SECTION_CLEARANCE_UNITS);
        axes[axis] = [(lowest - pad) as i64, (highest + pad) as i64];
    }
    Ok(axes)
}

/// Build the model queries for one plane, excluding every ambiguous point.
fn queries(
    mesh: &IndexedTriangleSet,
    model: &SectionModel,
    plane_z: f32,
    max_queries: usize,
) -> Result<(Vec<Query>, usize), String> {
    let z = f64::from(plane_z);
    let edge_clearance = EDGE_CLEARANCE_UNITS / UNITS_PER_MM_F64;
    let section_clearance = SECTION_CLEARANCE_UNITS / UNITS_PER_MM_F64;
    let axes = padded_axes(mesh)?;
    let mut candidates: Vec<Point2> = Vec::new();
    for step_y in 0..=LATTICE_DIVISIONS {
        for step_x in 0..=LATTICE_DIVISIONS {
            let x = axes[0][0] + (axes[0][1] - axes[0][0]) * step_x / LATTICE_DIVISIONS;
            let y = axes[1][0] + (axes[1][1] - axes[1][0]) * step_y / LATTICE_DIVISIONS;
            candidates.push(Point2 { x, y });
        }
    }
    // Perpendicular probes on both sides of every model section segment, so the
    // emitted boundary is pinned from both sides at every section segment.
    for segment in &model.segments {
        let (dx, dy) = (segment.b[0] - segment.a[0], segment.b[1] - segment.a[1]);
        let length = dx.hypot(dy);
        if length == 0.0 {
            continue;
        }
        let normal = [-dy / length, dx / length];
        let middle = [
            (segment.a[0] + segment.b[0]) / 2.0,
            (segment.a[1] + segment.b[1]) / 2.0,
        ];
        let offset = PROBE_OFFSET_UNITS / UNITS_PER_MM_F64;
        for sign in [1.0_f64, -1.0] {
            let x = (middle[0] + sign * offset * normal[0]) * UNITS_PER_MM_F64;
            let y = (middle[1] + sign * offset * normal[1]) * UNITS_PER_MM_F64;
            if x.abs() < 9.0e15 && y.abs() < 9.0e15 {
                candidates.push(Point2 {
                    x: x.round() as i64,
                    y: y.round() as i64,
                });
            }
        }
    }
    let mut excluded = 0_usize;
    let mut accepted: Vec<Query> = Vec::new();
    let mut seen: Vec<Point2> = Vec::new();
    for point in candidates {
        if seen.contains(&point) {
            continue;
        }
        seen.push(point);
        let xy = [
            point.x as f64 / UNITS_PER_MM_F64,
            point.y as f64 / UNITS_PER_MM_F64,
        ];
        if near_projected_edge(mesh, xy, edge_clearance)? {
            excluded += 1;
            continue;
        }
        if model
            .segments
            .iter()
            .any(|segment| distance_to_segment(xy, segment.a, segment.b) < section_clearance)
        {
            excluded += 1;
            continue;
        }
        match ray_parity(mesh, xy[0], xy[1], z)? {
            None => excluded += 1,
            Some(verdict) => accepted.push(Query { point, verdict }),
        }
    }
    if accepted.is_empty() {
        return Err(String::from(
            "no query clears the stated projected-edge, section and coplanar ambiguity",
        ));
    }
    // Mandatory populations first, then a deterministic strided remainder.
    let mut chosen: Vec<Query> = Vec::new();
    for verdict in [Verdict::Inside, Verdict::Outside] {
        if let Some(found) = accepted.iter().find(|query| query.verdict == verdict) {
            if !chosen.contains(found) {
                chosen.push(*found);
            }
        }
    }
    let rest: Vec<Query> = accepted
        .iter()
        .copied()
        .filter(|query| !chosen.contains(query))
        .collect();
    let room = max_queries.saturating_sub(chosen.len());
    if rest.len() <= room {
        chosen.extend(rest);
    } else if room > 0 {
        for index in 0..room {
            chosen.push(rest[index * rest.len() / room]);
        }
    }
    Ok((chosen, excluded))
}

/// Check one plane's emitted layer against the independent section model.
pub fn check_plane(
    mesh: &IndexedTriangleSet,
    model: &SectionModel,
    plane_z: f32,
    layer: &[ExPolygon],
    max_queries: usize,
) -> Result<PlaneStats, String> {
    if model.curves == 0 {
        return Err(format!(
            "the independent model found no section at z={plane_z} mm inside the mesh's Z range"
        ));
    }
    if layer.is_empty() {
        return Err(format!(
            "z={plane_z} mm: the layer is empty (an all-outside answer) but the input triangles \
             imply {} material curve(s) and {} hole curve(s)",
            model.outer_curves, model.hole_curves
        ));
    }
    let holes: usize = layer.iter().map(|polygon| polygon.holes.len()).sum();
    if model.hole_curves > 0 && holes == 0 {
        return Err(format!(
            "z={plane_z} mm: the input triangles imply {} hole curve(s) but the layer has none, so \
             a void was filled or lost",
            model.hole_curves
        ));
    }
    if layer.len() != model.outer_curves {
        return Err(format!(
            "z={plane_z} mm: the layer has {} component(s) but the input triangles imply {} \
             material curve(s) (and {} hole curve(s))",
            layer.len(),
            model.outer_curves,
            model.hole_curves
        ));
    }
    if holes != model.hole_curves {
        return Err(format!(
            "z={plane_z} mm: the layer has {holes} hole(s) but the input triangles imply {}",
            model.hole_curves
        ));
    }
    for polygon in layer {
        let contour = signed_double_area(&polygon.contour)?;
        if contour <= 0 {
            return Err(format!(
                "z={plane_z} mm: a contour is not counter-clockwise (double area {contour})"
            ));
        }
        for hole in &polygon.holes {
            let area = signed_double_area(hole)?;
            if area >= 0 {
                return Err(format!(
                    "z={plane_z} mm: a hole is not clockwise (double area {area})"
                ));
            }
        }
    }
    // Derived envelope: each emitted section vertex is a grid rounding of an
    // exact model vertex (at most one unit; see `super::envelopes`), and moving
    // every vertex of a closed curve by at most one unit changes its double area
    // by at most twice the perimeter.
    let want = model.signed_double_area_units2;
    let got: f64 = region_double_area(layer)? as f64;
    let envelope = 2.0 * model.perimeter_units;
    if (got - want).abs() > envelope {
        return Err(format!(
            "z={plane_z} mm: layer double area {got} differs from the input-triangle model {want} \
             by {}, beyond the derived envelope {envelope}",
            (got - want).abs()
        ));
    }
    let (selected, excluded) = queries(mesh, model, plane_z, max_queries)?;
    let mut stats = PlaneStats {
        plane_z,
        curves: model.curves,
        outer_curves: model.outer_curves,
        hole_curves: model.hole_curves,
        layer_components: layer.len(),
        layer_holes: holes,
        excluded,
        ..PlaneStats::default()
    };
    for query in &selected {
        let got = point_in_region(query.point, layer)
            .map_err(|error| format!("point_in_region: {error}"))?;
        let want = match query.verdict {
            Verdict::Inside => PointLocation::Inside,
            Verdict::Outside => PointLocation::Outside,
        };
        if got != want {
            return Err(format!(
                "z={plane_z} mm: query {:?} is {want:?} by the independent ray-parity model but \
                 {got:?} in the emitted cross-section",
                query.point
            ));
        }
        match query.verdict {
            Verdict::Inside => stats.inside_samples += 1,
            Verdict::Outside => stats.outside_samples += 1,
        }
    }
    // Nonempty populations: a nonempty section must be observed from both sides.
    if stats.inside_samples == 0 || stats.outside_samples == 0 {
        return Err(format!(
            "z={plane_z} mm has a section but the sample population {stats:?} is one-sided"
        ));
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Witness-implied structure.
// ─────────────────────────────────────────────────────────────────────────────

/// The component and hole counts a [`SolidWitness`] implies for a sliced plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WitnessRule {
    /// Smallest material curve count.
    pub min_outer: usize,
    /// Largest material curve count.
    pub max_outer: usize,
    /// Smallest hole curve count.
    pub min_holes: usize,
    /// Largest hole curve count.
    pub max_holes: usize,
    /// Whether at least one probed plane must show a hole.
    pub requires_a_hole: bool,
}

/// The witness-implied structure of one probed plane.
///
/// These are properties of the #4 controlled solid families, re-derived here so
/// the independent model is itself checked: a solid box, cylinder or orthogonal
/// intersection is one body with no void; a cavity box is one body whose probes
/// must include a plane through the void; a bridge is two pillars joined by a
/// beam, so one or two bodies and never a void; an overhang is a pillar with a
/// wider cap, so always one body; and separated bodies are two boxes spanning the
/// same Z range, so always two.
pub fn witness_rule(witness: SolidWitness) -> WitnessRule {
    match witness {
        SolidWitness::ClosedSolid => WitnessRule {
            min_outer: 1,
            max_outer: 1,
            min_holes: 0,
            max_holes: 0,
            requires_a_hole: false,
        },
        SolidWitness::Cavity => WitnessRule {
            min_outer: 1,
            max_outer: 1,
            min_holes: 0,
            max_holes: 1,
            requires_a_hole: true,
        },
        SolidWitness::Bridge => WitnessRule {
            min_outer: 1,
            max_outer: 2,
            min_holes: 0,
            max_holes: 0,
            requires_a_hole: false,
        },
        SolidWitness::Overhang => WitnessRule {
            min_outer: 1,
            max_outer: 1,
            min_holes: 0,
            max_holes: 0,
            requires_a_hole: false,
        },
        SolidWitness::SeparatedBodies => WitnessRule {
            min_outer: 2,
            max_outer: 2,
            min_holes: 0,
            max_holes: 0,
            requires_a_hole: false,
        },
    }
}

fn check_witness(witness: SolidWitness, model: &SectionModel, plane_z: f32) -> Result<(), String> {
    let rule = witness_rule(witness);
    if model.outer_curves < rule.min_outer
        || model.outer_curves > rule.max_outer
        || model.hole_curves < rule.min_holes
        || model.hole_curves > rule.max_holes
    {
        return Err(format!(
            "z={plane_z} mm: the {witness:?} witness implies {}-{} material curve(s) and {}-{} \
             hole curve(s), but the input triangles give {} and {}",
            rule.min_outer,
            rule.max_outer,
            rule.min_holes,
            rule.max_holes,
            model.outer_curves,
            model.hole_curves
        ));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Property body.
// ─────────────────────────────────────────────────────────────────────────────

/// Check one certified scripted solid end to end.
pub fn check_case(
    witness: SolidWitness,
    case: &ScriptedCase,
    limits: &MeshLimits,
) -> TestCaseResult {
    // A retained case must reproduce its own source, so the witness survives
    // shrinking as re-emittable input rather than as a remembered string.
    match emit_scad(&case.recipe, &case.recipe_bounds) {
        Ok(source) if source == case.source => {}
        Ok(_) => {
            return Err(TestCaseError::fail(
                "a retained scripted case no longer re-emits its own source",
            ))
        }
        Err(error) => {
            return Err(TestCaseError::fail(format!(
                "a retained scripted case no longer re-emits: {error}"
            )))
        }
    }
    let mesh = match &case.output.geometry {
        ScadGeometry::Mesh(mesh) => mesh.clone(),
        ScadGeometry::Empty => {
            return Err(TestCaseError::fail(format!(
                "a certified {witness:?} solid evaluated to no geometry"
            )))
        }
        ScadGeometry::Contours(contours) => {
            return Err(TestCaseError::fail(format!(
                "a certified {witness:?} solid converted to {} contour component(s)",
                contours.len()
            )))
        }
    };
    check_solid(witness, &mesh, limits).map_err(TestCaseError::fail)
}

/// The full independent check of one validated solid at its chosen planes.
pub fn check_solid(
    witness: SolidWitness,
    mesh: &IndexedTriangleSet,
    limits: &MeshLimits,
) -> Result<(), String> {
    if let Err(error) = validate_indexed_mesh(mesh, true) {
        return Err(format!(
            "the {witness:?} solid is not a closed indexed mesh: {error}"
        ));
    }
    match mesh_signed_volume_mm3(mesh) {
        Ok(volume) if volume > 0.0 => {}
        Ok(volume) => {
            return Err(format!(
                "the {witness:?} solid has signed volume {volume} mm^3, which is not \
                 outward-oriented"
            ))
        }
        Err(error) => {
            return Err(format!(
                "the {witness:?} solid's signed volume is unavailable: {error}"
            ))
        }
    }
    let planes = planes(mesh, limits.max_planes)?;
    let layers = slice_mesh_ex(mesh, &planes);
    if layers.len() != planes.len() {
        return Err(format!(
            "{witness:?}: slice_mesh_ex returned {} layers for {} planes",
            layers.len(),
            planes.len()
        ));
    }
    let mut saw_hole = false;
    for (index, plane) in planes.iter().enumerate() {
        let model = section_model(mesh, f64::from(*plane))?;
        check_witness(witness, &model, *plane)?;
        saw_hole |= model.hole_curves > 0;
        let stats = check_plane(mesh, &model, *plane, &layers[index], limits.max_queries)?;
        println!("H-MESH-SCAD {witness:?} {stats:?}");
    }
    if witness_rule(witness).requires_a_hole && !saw_hole {
        return Err(format!(
            "the {witness:?} witness requires a probed plane through its cavity, but none of the \
             probed planes {planes:?} showed a hole curve"
        ));
    }
    Ok(())
}

/// A deterministic solid recipe for one witness, in canonical units.
///
/// Every dimension is a quarter, eighth or half of the profile envelope, so the
/// witness is RNG-independent and stays inside the adapter's coordinate bound.
fn witness_recipe(witness: SolidWitness, max_abs_units: i64) -> ScadRecipe {
    let units = |value: i64| u32::try_from(value.max(1)).unwrap_or(1);
    let half = (max_abs_units / 2).max(4);
    let quarter = (max_abs_units / 4).max(2);
    let eighth = (max_abs_units / 8).max(2);
    let cuboid = |size: [i64; 3]| Recipe3d::Box {
        size_units: size.map(units),
    };
    let solid = match witness {
        SolidWitness::ClosedSolid => cuboid([half, half, half]),
        SolidWitness::Cavity => {
            let inner = (half / 2).max(2);
            let inset = ((half - inner) / 2).max(1);
            Recipe3d::Difference {
                base: Box::new(cuboid([half, half, half])),
                cuts: vec![Recipe3d::Translate {
                    offset_units: [inset, inset, inset],
                    child: Box::new(cuboid([inner, inner, inner])),
                }],
            }
        }
        SolidWitness::Bridge => Recipe3d::Union(vec![
            cuboid([quarter, quarter, quarter]),
            Recipe3d::Translate {
                offset_units: [quarter + eighth, 0, 0],
                child: Box::new(cuboid([quarter, quarter, quarter])),
            },
            Recipe3d::Translate {
                offset_units: [0, 0, quarter],
                child: Box::new(cuboid([2 * quarter + eighth, quarter, eighth])),
            },
        ]),
        SolidWitness::Overhang => Recipe3d::Union(vec![
            cuboid([quarter, quarter, quarter]),
            Recipe3d::Translate {
                offset_units: [0, 0, quarter],
                child: Box::new(cuboid([quarter + eighth, quarter, eighth])),
            },
        ]),
        SolidWitness::SeparatedBodies => Recipe3d::Union(vec![
            cuboid([quarter, quarter, quarter]),
            Recipe3d::Translate {
                offset_units: [quarter + eighth, 0, 0],
                child: Box::new(cuboid([eighth, eighth, quarter])),
            },
        ]),
    };
    // The witnesses here are primitive cuboids, so every dimension is exact and
    // no planar profile or extrusion is needed.
    ScadRecipe::Solid(solid)
}

/// Require `check_plane` to reject a named mutant layer.
///
/// The rejection must name the structural fault the mutation introduces
/// (`component` for a dropped body, `hole` for a filled cavity, `all-outside`
/// for an empty layer), so the arm cannot pass because of an unrelated model
/// fault. The unmutated layer of the
/// same plane is checked against the same reference in the same arm, which is
/// what makes the arm non-vacuous.
fn expect_layer_rejected(
    arm: &str,
    expected: &str,
    result: Result<PlaneStats, String>,
) -> Result<(), String> {
    match result {
        Ok(stats) => Err(format!(
            "H-MESH-SCAD arm '{arm}': the mutant layer passed the plane check: {stats:?}"
        )),
        Err(reason) => {
            if !reason.contains(expected) {
                return Err(format!(
                    "H-MESH-SCAD arm '{arm}': the mutant layer was rejected for the wrong reason \
                     (expected the message to name '{expected}'): {reason}"
                ));
            }
            println!("H-MESH-SCAD arm '{arm}' rejected the mutant layer: {reason}");
            Ok(())
        }
    }
}

/// Fault-detection arms on the emitted layers of the deterministic witnesses.
///
/// Every witness runs the all-outside arm on every probed plane (the real layer
/// replaced by an empty one, rejected by name as `all-outside`).
/// `SeparatedBodies` additionally supplies the dropped-body arm and `Cavity` the
/// filled-hole arm. Each arm first re-checks the
/// real emitted layer with the same [`check_plane`] call, so a passing arm is
/// always accompanied by a passing correct result on the identical model.
fn layer_mutant_arms(
    witness: SolidWitness,
    mesh: &IndexedTriangleSet,
    limits: &MeshLimits,
) -> Result<(), String> {
    let planes = planes(mesh, limits.max_planes)?;
    let layers = slice_mesh_ex(mesh, &planes);
    if layers.len() != planes.len() {
        return Err(format!(
            "{witness:?} arm: slice_mesh_ex returned {} layers for {} planes",
            layers.len(),
            planes.len()
        ));
    }
    if planes.is_empty() {
        return Err(format!(
            "H-MESH-SCAD arm 'all-outside' for {witness:?}: the deterministic witness has no \
             probed plane, so the arm would be vacuous"
        ));
    }
    // All-outside arm, on every witness: replace each probed plane's real layer
    // with the empty answer an all-outside SUT would emit. Every probed plane
    // lies inside the solid's Z range, so the model has a section there and the
    // empty layer must be rejected by name.
    for (index, plane) in planes.iter().enumerate() {
        let model = section_model(mesh, f64::from(*plane))?;
        let arm = format!("all-outside {witness:?} z={plane}");
        check_plane(mesh, &model, *plane, &layers[index], limits.max_queries)
            .map_err(|error| format!("H-MESH-SCAD arm '{arm}' unmutated: {error}"))?;
        expect_layer_rejected(
            &arm,
            "all-outside",
            check_plane(mesh, &model, *plane, &[], limits.max_queries),
        )?;
    }
    if !matches!(
        witness,
        SolidWitness::SeparatedBodies | SolidWitness::Cavity
    ) {
        return Ok(());
    }
    let mut performed = false;
    for (index, plane) in planes.iter().enumerate() {
        let layer = &layers[index];
        let model = section_model(mesh, f64::from(*plane))?;
        let holes: usize = layer.iter().map(|polygon| polygon.holes.len()).sum();
        match witness {
            SolidWitness::SeparatedBodies => {
                if layer.len() < 2 || holes != 0 {
                    continue;
                }
                let arm = format!("dropped-body z={plane}");
                check_plane(mesh, &model, *plane, layer, limits.max_queries)
                    .map_err(|error| format!("H-MESH-SCAD arm '{arm}' unmutated: {error}"))?;
                let mut mutant = layer.clone();
                mutant.remove(0);
                expect_layer_rejected(
                    &arm,
                    "component",
                    check_plane(mesh, &model, *plane, &mutant, limits.max_queries),
                )?;
                performed = true;
                break;
            }
            SolidWitness::Cavity => {
                if holes != 1 || model.hole_curves != 1 {
                    continue;
                }
                let arm = format!("filled-cavity z={plane}");
                check_plane(mesh, &model, *plane, layer, limits.max_queries)
                    .map_err(|error| format!("H-MESH-SCAD arm '{arm}' unmutated: {error}"))?;
                let mut mutant = layer.clone();
                for polygon in &mut mutant {
                    polygon.holes.clear();
                }
                expect_layer_rejected(
                    &arm,
                    "hole",
                    check_plane(mesh, &model, *plane, &mutant, limits.max_queries),
                )?;
                performed = true;
                break;
            }
            _ => {}
        }
    }
    if !performed {
        return Err(format!(
            "H-MESH-SCAD arm for {witness:?}: no probed plane of the deterministic witness \
             carries the structure the arm mutates, so the arm would be vacuous"
        ));
    }
    Ok(())
}

/// Deterministic, RNG-independent witnesses evaluated through the real adapter.
///
/// One concrete solid per witness, checked by the same independent model the
/// generated cases use, so the witness-implied component and hole counts — and
/// in particular a cavity or a separated body that disappears — are covered
/// even when the seeded generated draws happen not to reach that branch. The
/// structural witnesses then run the mutant arms of [`layer_mutant_arms`], so
/// the checker is proved to *reject* a dropped body and a filled cavity rather
/// than only to accept the correct layer.
pub fn deterministic_witnesses(limits: &MeshLimits) -> Result<(), String> {
    for witness in WITNESSES {
        let recipe = witness_recipe(witness, limits.geometry.max_abs_units);
        let output = evaluate_recipe(&recipe, &limits.options, &limits.recipe)
            .map_err(|error| format!("{witness:?} witness evaluation: {error}"))?;
        let mesh = match output.geometry {
            ScadGeometry::Mesh(mesh) => mesh,
            other => {
                return Err(format!(
                    "{witness:?} witness converted to {other:?}, not a triangle mesh"
                ))
            }
        };
        check_solid(witness, &mesh, limits)
            .map_err(|error| format!("{witness:?} deterministic witness: {error}"))?;
        layer_mutant_arms(witness, &mesh, limits)?;
    }
    Ok(())
}

/// Run the registry-selected `H-MESH-SCAD` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME)
        .expect("H-MESH-SCAD requires the registry-backed property command");
    let raw = RawScadBounds::read(&run).expect("H-MESH-SCAD bound keys");
    let limits = checked_limits(raw).expect("H-MESH-SCAD checked profile bounds");
    deterministic_witnesses(&limits).expect("H-MESH-SCAD deterministic scripted witnesses");
    let strategy = strategy(&limits).expect("H-MESH-SCAD strategy");
    run.check(&strategy, |(witness, case), _guard| {
        check_case(witness, &case, &limits)
    })
    .expect("H-MESH-SCAD scripted mesh cross-sections");
}
