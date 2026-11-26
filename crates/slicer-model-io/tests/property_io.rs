//! Opt-in property tests for `slicer-model-io`'s public placement and extent
//! seams (packet `proptest-openrscad_16_pbt-io-gcode`, charters `H-IO-EXTENT`
//! and `H-IO-PLACEMENT`). The generated properties live here; deterministic
//! controls share the same checkers.
//!
//! Gated by the `property_testing` selection feature through the manifest
//! `[[test]]` entry, and skipped entirely by an ordinary build.
//!
//! # H-IO-EXTENT oracle
//!
//! The generated domain is dyadic: every vertex coordinate is a multiple of
//! `1/16` mm with `|coordinate| < 4096` mm, and the affine Z row
//! `m[2], m[6], m[10], m[14]` is a multiple of `1/4` mm. In units of `2^-6` mm
//! every product and the four-term sum are exact integers, so the expected
//! world Z is derived with integer arithmetic and narrowed once to `f32`.
//! That independently matches the production's exact `f64` sum plus one
//! narrowing cast, and proves the cast is exact on this domain. The all-zero
//! matrix is the documented identity convention; empty, flat and non-finite
//! outcomes are reduced through a filter/reduce model of the documented
//! NaN-ignore and infinity-rejects policy.
//!
//! # H-IO-PLACEMENT oracle
//!
//! Each object's world AABB is recomputed from its vertices and the documented
//! zero-matrix-is-identity convention; the union gives
//! `dx = bed_center.x - (min.x + max.x) / 2`, `dy` likewise and `dz = -min.z`.
//! After placement the checker requires one rigid translation for every
//! object, world `z_min = 0`, the whole XY AABB centre on the bed within two
//! ulps, every object-pair AABB difference preserved within
//! `4 * f32::EPSILON * max_abs`, and `build_volume` equal to the post-placement
//! **world-space** AABB union for every scene, identity and live transform
//! alike. That is the documented `MeshIR::build_volume` contract: the pnp-cli
//! visual-debug viewport reads it as the model-wide world-space extent, and
//! `bed_placement_tdd::build_volume_tracks_the_placed_model` requires it to
//! follow the placed model. Identity scenes additionally require the shifted
//! raw-vertex union, which the world union equals there.
//!
//! A fixture may pre-seed any `build_volume`; the function must refresh it to
//! the post-placement world union on every successful placement, including an
//! exactly-zero shift. The zero-shift witness starts from a deliberately stale
//! box and requires the recompute.

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestCaseError, TestRunner};
use slicer_ir::{BoundingBox3, IndexedTriangleSet, MeshIR, ObjectMesh, Point3, Transform3d};
use slicer_model_io::loader::object_world_z_extent;
use slicer_model_io::{place_bare_mesh_on_bed, BedPlacement};
use slicer_sdk::test_support::property_testing::domains::{
    analytical_meshes, rigid_transforms, MeshCase, SolidKind,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

/// Registry identity of the `H-IO-EXTENT` charter.
const EXTENT_PROPERTY_ID: &str = "H-IO-EXTENT";
/// Exact registry test name (must match `test-properties.toml`).
const EXTENT_TEST_NAME: &str = "proptest_object_world_z_extent";
/// Registry identity of the `H-IO-PLACEMENT` charter.
const PLACEMENT_PROPERTY_ID: &str = "H-IO-PLACEMENT";
/// Exact registry test name (must match `test-properties.toml`).
const PLACEMENT_TEST_NAME: &str = "proptest_place_bare_mesh_on_bed";

/// `2^4` vertices per millimetre: the dyadic `2^-4` mm coordinate grid.
const VERTEX_GRID_PER_MM: f32 = 16.0;
/// `2^2` affine-row units per millimetre: the dyadic `2^-2` mm matrix grid.
const ROW_GRID_PER_MM: f64 = 4.0;
/// Bounded affine row entries in `[-16, 16]` mm.
const ROW_BOUND_MM: i64 = 16;
/// Bounded non-zero `m[10]` scale in `[-8, -0.25] U [0.25, 8]` mm.
const SCALE_BOUND_MM: i64 = 8;

// ===========================================================================
// H-IO-EXTENT.
// ===========================================================================

fn zero_transform() -> Transform3d {
    Transform3d { matrix: [0.0; 16] }
}

fn point3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

/// Structural mode of a generated extent case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtentMode {
    /// All-zero matrix, the documented identity convention.
    Identity,
    /// Live affine Z row with at least one non-zero entry.
    Affine,
    /// Constant transformed Z: a degenerate extent.
    Flat,
    /// A NaN vertex beside finite ones: the reduction ignores it.
    NonFiniteNan,
    /// An infinite vertex: the reduced extent is non-finite.
    NonFiniteInfinity,
}

const EXTENT_MODES: [ExtentMode; 5] = [
    ExtentMode::Identity,
    ExtentMode::Affine,
    ExtentMode::Flat,
    ExtentMode::NonFiniteNan,
    ExtentMode::NonFiniteInfinity,
];

#[derive(Debug, Clone)]
struct ExtentCase {
    mode: ExtentMode,
    vertices: Vec<Point3>,
    transform: Transform3d,
}

/// Validated `H-IO-EXTENT` registry bounds.
#[derive(Debug, Clone, Copy)]
struct ExtentLimits {
    max_vertices: usize,
    max_abs_units: i64,
}

impl ExtentLimits {
    fn from_bounds(max_vertices: u64, max_abs_mm: u64) -> Result<Self, String> {
        let max_vertices = usize::try_from(max_vertices)
            .map_err(|_| "max_vertices does not fit usize".to_string())?;
        if max_vertices == 0 {
            return Err("max_vertices must be positive".to_string());
        }
        if max_vertices > 4096 {
            return Err(format!(
                "max_vertices {max_vertices} exceeds the 4096 allocation guard"
            ));
        }
        let max_abs_mm = f32::from(
            u16::try_from(max_abs_mm)
                .map_err(|_| "max_abs_mm does not fit the bounded grid".to_string())?,
        );
        if !(max_abs_mm.is_finite() && max_abs_mm > 0.0 && max_abs_mm < 4096.0) {
            return Err(format!(
                "max_abs_mm {max_abs_mm} must be finite, positive and below 2^12 mm"
            ));
        }
        let units = max_abs_mm * VERTEX_GRID_PER_MM;
        if units.fract() != 0.0 {
            return Err(format!(
                "max_abs_mm {max_abs_mm} must be a multiple of 1/16 mm to stay on the dyadic \
                 coordinate grid"
            ));
        }
        Ok(Self {
            max_vertices,
            max_abs_units: units as i64,
        })
    }

    fn witness() -> Self {
        Self {
            max_vertices: 16,
            max_abs_units: 1024,
        }
    }
}

fn vertex_strategy(limits: ExtentLimits) -> impl Strategy<Value = Point3> {
    let units = limits.max_abs_units;
    (-units..=units, -units..=units, -units..=units).prop_map(|(x, y, z)| Point3 {
        x: x as f32 / VERTEX_GRID_PER_MM,
        y: y as f32 / VERTEX_GRID_PER_MM,
        z: z as f32 / VERTEX_GRID_PER_MM,
    })
}

fn row_strategy() -> impl Strategy<Value = f64> {
    (-ROW_BOUND_MM..=ROW_BOUND_MM).prop_map(|units| units as f64 / ROW_GRID_PER_MM)
}

fn scale_strategy() -> impl Strategy<Value = f64> {
    prop_oneof![(-SCALE_BOUND_MM..=-1), (1..=SCALE_BOUND_MM)]
        .prop_map(|units| units as f64 / ROW_GRID_PER_MM)
}

fn affine_transform_strategy() -> impl Strategy<Value = Transform3d> {
    (
        row_strategy(),
        row_strategy(),
        scale_strategy(),
        row_strategy(),
    )
        .prop_map(|(m2, m6, m10, m14)| {
            let mut matrix = [0.0_f64; 16];
            matrix[2] = m2;
            matrix[6] = m6;
            matrix[10] = m10;
            matrix[14] = m14;
            Transform3d { matrix }
        })
}

fn extent_case_strategy(mode: ExtentMode, limits: ExtentLimits) -> BoxedStrategy<ExtentCase> {
    match mode {
        ExtentMode::Identity => {
            prop::collection::vec(vertex_strategy(limits), 0..=limits.max_vertices)
                .prop_map(move |vertices| ExtentCase {
                    mode,
                    vertices,
                    transform: zero_transform(),
                })
                .boxed()
        }
        ExtentMode::Affine => (
            prop::collection::vec(vertex_strategy(limits), 0..=limits.max_vertices),
            affine_transform_strategy(),
        )
            .prop_map(move |(vertices, transform)| ExtentCase {
                mode,
                vertices,
                transform,
            })
            .boxed(),
        ExtentMode::Flat => (
            (-limits.max_abs_units..=limits.max_abs_units),
            prop::collection::vec(
                (
                    -limits.max_abs_units..=limits.max_abs_units,
                    -limits.max_abs_units..=limits.max_abs_units,
                ),
                1..=limits.max_vertices,
            ),
        )
            .prop_map(move |(z_units, xy)| {
                let z = z_units as f32 / VERTEX_GRID_PER_MM;
                let vertices = xy
                    .into_iter()
                    .map(|(x, y)| {
                        point3(
                            x as f32 / VERTEX_GRID_PER_MM,
                            y as f32 / VERTEX_GRID_PER_MM,
                            z,
                        )
                    })
                    .collect();
                ExtentCase {
                    mode,
                    vertices,
                    transform: zero_transform(),
                }
            })
            .boxed(),
        ExtentMode::NonFiniteNan => (0..=(limits.max_abs_units - 1))
            .prop_flat_map(move |base| {
                (Just(base), 1..=(limits.max_abs_units - base)).prop_map(move |(base, rise)| {
                    let z0 = base as f32 / VERTEX_GRID_PER_MM;
                    let z1 = (base + rise) as f32 / VERTEX_GRID_PER_MM;
                    ExtentCase {
                        mode,
                        vertices: vec![
                            point3(0.0, 0.0, z0),
                            point3(1.0, 0.0, z1),
                            point3(0.0, 1.0, f32::NAN),
                        ],
                        transform: zero_transform(),
                    }
                })
            })
            .boxed(),
        ExtentMode::NonFiniteInfinity => {
            prop::sample::select(vec![f32::INFINITY, f32::NEG_INFINITY])
                .no_shrink()
                .prop_map(move |infinity| ExtentCase {
                    mode,
                    vertices: vec![
                        point3(0.0, 0.0, 0.0),
                        point3(0.0, 0.0, 0.5),
                        point3(1.0, 1.0, infinity),
                    ],
                    transform: zero_transform(),
                })
                .boxed()
        }
    }
}

fn extent_strategy(limits: ExtentLimits) -> BoxedStrategy<ExtentCase> {
    prop::sample::select(EXTENT_MODES.to_vec())
        .no_shrink()
        .prop_flat_map(move |mode| extent_case_strategy(mode, limits))
        .boxed()
}

/// Affine Z row in integer `1/4` mm units.
#[derive(Debug, Clone, Copy)]
struct RowUnits {
    m2: i64,
    m6: i64,
    m10: i64,
    m14: i64,
}

fn grid_i64(value: f64, per_mm: f64, label: &str) -> Result<i64, String> {
    let scaled = value * per_mm;
    if !scaled.is_finite() {
        return Err(format!("{label} entry {value} is not finite"));
    }
    let rounded = scaled.round();
    if scaled != rounded {
        return Err(format!(
            "{label} entry {value} is not a multiple of 1/{per_mm} mm"
        ));
    }
    if rounded < i64::MIN as f64 || rounded > i64::MAX as f64 {
        return Err(format!(
            "{label} entry {value} does not fit an integer grid"
        ));
    }
    Ok(rounded as i64)
}

fn row_units(matrix: &[f64; 16]) -> Result<RowUnits, String> {
    Ok(RowUnits {
        m2: grid_i64(matrix[2], ROW_GRID_PER_MM, "m[2]")?,
        m6: grid_i64(matrix[6], ROW_GRID_PER_MM, "m[6]")?,
        m10: grid_i64(matrix[10], ROW_GRID_PER_MM, "m[10]")?,
        m14: grid_i64(matrix[14], ROW_GRID_PER_MM, "m[14]")?,
    })
}

fn vertex_units(value: f32) -> Result<i64, String> {
    let scaled = f64::from(value) * f64::from(VERTEX_GRID_PER_MM);
    if !scaled.is_finite() {
        return Err(format!("coordinate {value} is not finite"));
    }
    let rounded = scaled.round();
    if scaled != rounded {
        return Err(format!("coordinate {value} is not a multiple of 1/16 mm"));
    }
    Ok(rounded as i64)
}

/// Exact world Z in units of `2^-6` mm, narrowed once to `f32`.
fn exact_world_z(vertex: &Point3, row: RowUnits) -> Result<f32, String> {
    let kx = vertex_units(vertex.x)?;
    let ky = vertex_units(vertex.y)?;
    let kz = vertex_units(vertex.z)?;
    let numerator = kx * row.m2 + ky * row.m6 + kz * row.m10 + 16 * row.m14;
    let value = numerator as f64 / 64.0;
    if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(format!("world z {value} does not fit f32"));
    }
    Ok(value as f32)
}

/// Filter/reduce model of the documented reduction policy: NaN vertices are
/// ignored, and an infinity that remains in the set makes the reduced extremum
/// non-finite, so the extent is `None`.
fn reduce_extent(z_values: &[f32]) -> Option<(f32, f32)> {
    let non_nan = z_values.iter().copied().filter(|z| !z.is_nan());
    let min = non_nan.clone().reduce(f32::min)?;
    let max = non_nan.reduce(f32::max)?;
    (min.is_finite() && max.is_finite() && max > min).then_some((min, max))
}

fn extent_oracle(case: &ExtentCase) -> Result<Option<(f32, f32)>, String> {
    let matrix = &case.transform.matrix;
    let identity = matrix.iter().all(|value| *value == 0.0);
    let z_values = if identity {
        case.vertices.iter().map(|vertex| vertex.z).collect()
    } else {
        let row = row_units(matrix)?;
        case.vertices
            .iter()
            .map(|vertex| exact_world_z(vertex, row))
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(reduce_extent(&z_values))
}

/// The shared extent checker: the property and `io_extent_controls` both call
/// this with the real (or deliberately mutated) observation.
fn check_extent_case(case: &ExtentCase, actual: Option<(f32, f32)>) -> Result<(), String> {
    let expected = extent_oracle(case)?;
    match (expected, actual) {
        (None, None) => Ok(()),
        (Some((expected_min, expected_max)), Some((actual_min, actual_max)))
            if expected_min.to_bits() == actual_min.to_bits()
                && expected_max.to_bits() == actual_max.to_bits() =>
        {
            Ok(())
        }
        (expected, actual) => Err(format!(
            "object_world_z_extent({:?}) = {actual:?}, independent oracle = {expected:?}",
            case.mode
        )),
    }
}

fn object_from_extent_case(case: &ExtentCase) -> ObjectMesh {
    ObjectMesh {
        id: "extent-witness".to_string(),
        mesh: IndexedTriangleSet {
            vertices: case.vertices.clone(),
            indices: Vec::new(),
        },
        transform: case.transform,
        ..Default::default()
    }
}

/// Mode-specific structural witness used by the shrink probe.
fn validate_extent_case(case: &ExtentCase, limits: ExtentLimits) -> Result<(), String> {
    if case.vertices.len() > limits.max_vertices {
        return Err(format!(
            "generated {} vertices exceeds max_vertices {}",
            case.vertices.len(),
            limits.max_vertices
        ));
    }
    let matrix = &case.transform.matrix;
    let identity = matrix.iter().all(|value| *value == 0.0);
    match case.mode {
        ExtentMode::Identity => {
            if !identity {
                return Err("identity mode must use the all-zero matrix".to_string());
            }
        }
        ExtentMode::Affine => {
            if identity {
                return Err("affine mode lost its live transform".to_string());
            }
            let row = row_units(matrix)?;
            if row.m10 == 0 {
                return Err("affine mode lost its non-zero m[10] scale".to_string());
            }
        }
        ExtentMode::Flat => {
            if !identity {
                return Err("flat mode must use the all-zero identity matrix".to_string());
            }
            if case.vertices.is_empty() {
                return Err("flat mode lost its degenerate witness".to_string());
            }
            let z = case.vertices[0].z;
            if !case.vertices.iter().all(|vertex| vertex.z == z) {
                return Err("flat mode lost its constant transformed Z".to_string());
            }
        }
        ExtentMode::NonFiniteNan => {
            if !case.vertices.iter().any(|vertex| vertex.z.is_nan()) {
                return Err("non-finite NaN mode lost its NaN vertex".to_string());
            }
            if !case.vertices.iter().any(|vertex| vertex.z.is_finite()) {
                return Err("non-finite NaN mode needs a finite vertex too".to_string());
            }
        }
        ExtentMode::NonFiniteInfinity => {
            if !case.vertices.iter().any(|vertex| vertex.z.is_infinite()) {
                return Err("non-finite infinity mode lost its infinite vertex".to_string());
            }
        }
    }
    for vertex in &case.vertices {
        if case.mode != ExtentMode::NonFiniteNan
            && case.mode != ExtentMode::NonFiniteInfinity
            && (!vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite())
        {
            return Err("finite mode generated a non-finite coordinate".to_string());
        }
        if vertex.x.is_finite() && vertex_units(vertex.x)?.abs() > limits.max_abs_units {
            return Err(format!(
                "coordinate {} exceeds max_abs_mm {}",
                vertex.x,
                limits.max_abs_units as f32 / VERTEX_GRID_PER_MM
            ));
        }
        if vertex.y.is_finite() && vertex_units(vertex.y)?.abs() > limits.max_abs_units {
            return Err(format!(
                "coordinate {} exceeds max_abs_mm {}",
                vertex.y,
                limits.max_abs_units as f32 / VERTEX_GRID_PER_MM
            ));
        }
        if vertex.z.is_finite() && vertex_units(vertex.z)?.abs() > limits.max_abs_units {
            return Err(format!(
                "coordinate {} exceeds max_abs_mm {}",
                vertex.z,
                limits.max_abs_units as f32 / VERTEX_GRID_PER_MM
            ));
        }
    }
    Ok(())
}

fn run_extent_shrink_probe(limits: ExtentLimits) {
    for mode in EXTENT_MODES {
        let strategy = extent_case_strategy(mode, limits);
        let shrink_expected = mode != ExtentMode::NonFiniteInfinity;
        shrink_witness_probe(
            &format!("H-IO-EXTENT {mode:?}"),
            mode,
            &strategy,
            |case: &ExtentCase| case.mode,
            |case| validate_extent_case(case, limits),
            shrink_expected,
        );
    }
}

#[test]
fn proptest_object_world_z_extent() {
    let run = PropertyRun::new(EXTENT_PROPERTY_ID, EXTENT_TEST_NAME)
        .expect("H-IO-EXTENT registry context");
    let limits = ExtentLimits::from_bounds(
        run.bound("max_vertices").expect("max_vertices"),
        run.bound("max_abs_mm").expect("max_abs_mm"),
    )
    .expect("H-IO-EXTENT domain bounds");

    run_extent_shrink_probe(limits);
    let strategy = extent_strategy(limits);
    run.check(&strategy, |case, _guard| {
        let actual = object_world_z_extent(&object_from_extent_case(&case));
        check_extent_case(&case, actual).map_err(TestCaseError::fail)
    })
    .expect("H-IO-EXTENT exact affine Z-row contract");
}

// ===========================================================================
// H-IO-PLACEMENT.
// ===========================================================================

/// One object's raw vertices plus its authored transform.
#[derive(Debug, Clone)]
struct ObjectFixture {
    vertices: Vec<Point3>,
    transform: Transform3d,
}

/// Structural mode of a generated placement scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlacementMode {
    /// No objects at all: an unplaceable world.
    EmptyWorld,
    /// One or more zero-matrix boxes.
    Identity,
    /// One or more boxes with a bounded live rigid transform.
    LiveTransform,
    /// One empty object beside at least one nonempty object.
    MixedWithEmpty,
    /// A scene whose world AABB is already on the bed and on the plate.
    AlreadyPlaced,
    /// A scene sunk below the plate and off-centre.
    BelowBed,
    /// A non-printable world whose every object is rejected by the finite-X
    /// AABB gate (a negative mode: `place_bare_mesh_on_bed` must return `None`).
    NonFiniteXWorld,
}

const PLACEMENT_MODES: [PlacementMode; 7] = [
    PlacementMode::EmptyWorld,
    PlacementMode::Identity,
    PlacementMode::LiveTransform,
    PlacementMode::MixedWithEmpty,
    PlacementMode::AlreadyPlaced,
    PlacementMode::BelowBed,
    PlacementMode::NonFiniteXWorld,
];

#[derive(Debug, Clone)]
struct PlacementCase {
    mode: PlacementMode,
    objects: Vec<ObjectFixture>,
    bed_center: (f32, f32),
}

/// Validated `H-IO-PLACEMENT` registry bounds.
#[derive(Debug, Clone, Copy)]
struct PlacementLimits {
    max_objects: usize,
    max_vertices_per_object: usize,
    max_abs_mm: f32,
    max_translation_mm: f32,
}

impl PlacementLimits {
    fn from_bounds(
        max_objects: u64,
        max_vertices_per_object: u64,
        max_abs_mm: u64,
        max_translation_mm: u64,
    ) -> Result<Self, String> {
        let max_objects = usize::try_from(max_objects)
            .map_err(|_| "max_objects does not fit usize".to_string())?;
        if max_objects == 0 || max_objects > 1024 {
            return Err(format!("max_objects {max_objects} must be in 1..=1024"));
        }
        if max_objects < 2 {
            return Err(
                "max_objects must be at least 2 to host the empty-object-beside-nonempty \
                 witness"
                    .to_string(),
            );
        }
        let max_vertices_per_object = usize::try_from(max_vertices_per_object)
            .map_err(|_| "max_vertices_per_object does not fit usize".to_string())?;
        if !(8..=4096).contains(&max_vertices_per_object) {
            return Err(format!(
                "max_vertices_per_object {max_vertices_per_object} must be in 8..=4096 for the \
                 box family"
            ));
        }
        let max_abs_mm = f32::from(
            u16::try_from(max_abs_mm)
                .map_err(|_| "max_abs_mm does not fit the bounded grid".to_string())?,
        );
        if !(max_abs_mm.is_finite() && max_abs_mm > 2.0 && max_abs_mm < 4096.0) {
            return Err(format!(
                "max_abs_mm {max_abs_mm} must be finite, above the 2 mm box-origin offset and \
                 below 2^12 mm"
            ));
        }
        let max_translation_mm = f32::from(
            u16::try_from(max_translation_mm)
                .map_err(|_| "max_translation_mm does not fit the bounded grid".to_string())?,
        );
        if !(max_translation_mm.is_finite() && (0.0..=10_000.0).contains(&max_translation_mm)) {
            return Err(format!(
                "max_translation_mm {max_translation_mm} must be finite and in [0, 10000] mm"
            ));
        }
        if max_translation_mm * 8.0 > 1_000_000.0 {
            return Err(format!(
                "max_translation_mm {max_translation_mm} is too large for the 1/8 mm bed-centre \
                 grid"
            ));
        }
        Ok(Self {
            max_objects,
            max_vertices_per_object,
            max_abs_mm,
            max_translation_mm,
        })
    }

    fn witness() -> Self {
        Self {
            max_objects: 4,
            max_vertices_per_object: 8,
            max_abs_mm: 64.0,
            max_translation_mm: 200.0,
        }
    }

    /// Largest absolute world coordinate after placement: the box bound plus
    /// the bounded origin offset (2 mm) plus two bounded translations.
    fn max_abs_after(self) -> f32 {
        self.max_abs_mm + 2.0 + 2.0 * self.max_translation_mm
    }
}

fn box_strategy(limits: PlacementLimits) -> BoxedStrategy<MeshCase> {
    analytical_meshes(SolidKind::Box, limits.max_abs_mm - 2.0)
        .expect("validated box extent is finite and in (0, 10000] mm")
}

fn bed_center_strategy(limits: PlacementLimits) -> impl Strategy<Value = (f32, f32)> {
    let cap = (limits.max_translation_mm * 8.0) as i64;
    (-cap..=cap, -cap..=cap).prop_map(|(x, y)| (x as f32 / 8.0, y as f32 / 8.0))
}

fn identity_fixture(case: &MeshCase) -> ObjectFixture {
    ObjectFixture {
        vertices: case.mesh.vertices.clone(),
        transform: zero_transform(),
    }
}

fn live_fixture(case: &MeshCase, transform: Transform3d) -> ObjectFixture {
    ObjectFixture {
        vertices: case.mesh.vertices.clone(),
        transform,
    }
}

fn empty_fixture() -> ObjectFixture {
    ObjectFixture {
        vertices: Vec::new(),
        transform: zero_transform(),
    }
}

fn placement_strategy(limits: PlacementLimits) -> BoxedStrategy<PlacementCase> {
    prop::sample::select(PLACEMENT_MODES.to_vec())
        .no_shrink()
        .prop_flat_map(move |mode| placement_case_strategy(mode, limits))
        .boxed()
}

fn placement_case_strategy(
    mode: PlacementMode,
    limits: PlacementLimits,
) -> BoxedStrategy<PlacementCase> {
    let boxes = box_strategy(limits);
    let bed = bed_center_strategy(limits);
    match mode {
        PlacementMode::EmptyWorld => bed
            .prop_map(move |bed_center| PlacementCase {
                mode,
                objects: Vec::new(),
                bed_center,
            })
            .boxed(),
        PlacementMode::Identity => (prop::collection::vec(boxes, 1..=limits.max_objects), bed)
            .prop_map(move |(boxes, bed_center)| PlacementCase {
                mode,
                objects: boxes.iter().map(identity_fixture).collect(),
                bed_center,
            })
            .boxed(),
        PlacementMode::LiveTransform => {
            let rigid = rigid_transforms(limits.max_translation_mm)
                .expect("validated translation bound is finite and in [0, 10000] mm");
            (
                prop::collection::vec((boxes, rigid), 1..=limits.max_objects),
                bed,
            )
                .prop_map(move |(pairs, bed_center)| PlacementCase {
                    mode,
                    objects: pairs
                        .iter()
                        .map(|(case, rigid)| live_fixture(case, rigid.transform))
                        .collect(),
                    bed_center,
                })
                .boxed()
        }
        PlacementMode::MixedWithEmpty => (
            prop::collection::vec(boxes, 1..=(limits.max_objects - 1)),
            bed,
        )
            .prop_map(move |(boxes, bed_center)| {
                let mut objects = vec![empty_fixture()];
                objects.extend(boxes.iter().map(identity_fixture));
                PlacementCase {
                    mode,
                    objects,
                    bed_center,
                }
            })
            .boxed(),
        PlacementMode::AlreadyPlaced => (prop::collection::vec(boxes, 1..=limits.max_objects), bed)
            .prop_map(move |(boxes, bed_center)| {
                let mut objects: Vec<ObjectFixture> = boxes.iter().map(identity_fixture).collect();
                centre_fixtures_on_bed(&mut objects, bed_center);
                PlacementCase {
                    mode,
                    objects,
                    bed_center,
                }
            })
            .boxed(),
        PlacementMode::BelowBed => (
            prop::collection::vec(boxes, 1..=limits.max_objects),
            bed,
            (1_i64..=16, 1_i64..=16, 1_i64..=16),
        )
            .prop_map(move |(boxes, bed_center, (shift_x, shift_y, shift_z))| {
                let mut objects: Vec<ObjectFixture> = boxes.iter().map(identity_fixture).collect();
                let shift = (
                    -(shift_x as f32) / 8.0,
                    -(shift_y as f32) / 8.0,
                    -(shift_z as f32) / 8.0,
                );
                for object in &mut objects {
                    for vertex in &mut object.vertices {
                        vertex.x += shift.0;
                        vertex.y += shift.1;
                        vertex.z += shift.2;
                    }
                }
                PlacementCase {
                    mode,
                    objects,
                    bed_center,
                }
            })
            .boxed(),
        PlacementMode::NonFiniteXWorld => (
            prop::sample::select(vec![f32::INFINITY, f32::NEG_INFINITY]).no_shrink(),
            bed,
        )
            .prop_map(move |(infinity, bed_center)| {
                // Negative mode: not a printable object. One infinite X makes
                // the object's finite-X AABB gate fail, so no object remains.
                let mut vertices = control_box([0.0, 0.0, 0.0], [4.0, 3.0, 2.0]);
                vertices[0].x = infinity;
                PlacementCase {
                    mode,
                    objects: vec![ObjectFixture {
                        vertices,
                        transform: zero_transform(),
                    }],
                    bed_center,
                }
            })
            .boxed(),
    }
}

/// An optional finite world AABB.
type Aabb = Option<(Point3, Point3)>;

/// The documented transform application: the all-zero matrix is identity,
/// otherwise the column-major matrix is applied to each vertex in `f64` and
/// narrowed once. The function returns `None` under the production finite-X
/// gate (empty or non-finite-X geometry), which is the gate that decides
/// whether an object participates in placement.
fn world_aabb_of(vertices: &[Point3], transform: &Transform3d) -> Aabb {
    let matrix = &transform.matrix;
    let identity = matrix.iter().all(|value| *value == 0.0);
    let mut min = point3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = point3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for vertex in vertices {
        let world = if identity {
            *vertex
        } else {
            let (x, y, z) = (
                f64::from(vertex.x),
                f64::from(vertex.y),
                f64::from(vertex.z),
            );
            point3(
                (matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12]) as f32,
                (matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13]) as f32,
                (matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14]) as f32,
            )
        };
        min.x = min.x.min(world.x);
        min.y = min.y.min(world.y);
        min.z = min.z.min(world.z);
        max.x = max.x.max(world.x);
        max.y = max.y.max(world.y);
        max.z = max.z.max(world.z);
    }
    (min.x.is_finite() && max.x.is_finite()).then_some((min, max))
}

fn object_world_aabb_oracle(object: &ObjectMesh) -> Aabb {
    world_aabb_of(&object.mesh.vertices, &object.transform)
}

fn fixture_world_aabb(object: &ObjectFixture) -> Aabb {
    world_aabb_of(&object.vertices, &object.transform)
}

fn merge_aabb(accumulator: Aabb, next_aabb: Aabb) -> Aabb {
    match (accumulator, next_aabb) {
        (None, other) | (other, None) => other,
        (Some((left_min, left_max)), Some((right_min, right_max))) => Some((
            point3(
                left_min.x.min(right_min.x),
                left_min.y.min(right_min.y),
                left_min.z.min(right_min.z),
            ),
            point3(
                left_max.x.max(right_max.x),
                left_max.y.max(right_max.y),
                left_max.z.max(right_max.z),
            ),
        )),
    }
}

fn union_of_aabbs(aabbs: &[Aabb]) -> Aabb {
    aabbs.iter().copied().fold(None, merge_aabb)
}

fn union_aabb(objects: &[ObjectMesh]) -> Aabb {
    objects
        .iter()
        .map(object_world_aabb_oracle)
        .fold(None, merge_aabb)
}

fn fixtures_union(objects: &[ObjectFixture]) -> Aabb {
    objects
        .iter()
        .map(fixture_world_aabb)
        .fold(None, merge_aabb)
}

/// Documented placement formula on the `f32` world AABB.
fn placement_from_union(min: Point3, max: Point3, bed_center: (f32, f32)) -> BedPlacement {
    BedPlacement {
        dx: bed_center.0 - (min.x + max.x) / 2.0,
        dy: bed_center.1 - (min.y + max.y) / 2.0,
        dz: -min.z,
    }
}

fn placement_formula(mesh: &MeshIR, bed_center: (f32, f32)) -> Option<BedPlacement> {
    let (min, max) = union_aabb(&mesh.objects)?;
    Some(placement_from_union(min, max, bed_center))
}

/// Shift every fixture so the union AABB is centred on `bed_center` and rests
/// on the plate. Callers only pass zero-matrix fixtures.
fn centre_fixtures_on_bed(objects: &mut [ObjectFixture], bed_center: (f32, f32)) {
    let Some((min, max)) = fixtures_union(objects) else {
        return;
    };
    let placement = placement_from_union(min, max, bed_center);
    for object in objects.iter_mut() {
        for vertex in &mut object.vertices {
            vertex.x += placement.dx;
            vertex.y += placement.dy;
            vertex.z += placement.dz;
        }
    }
}

/// Union of the stored raw vertices (not world AABBs): the production
/// `compute_bounding_box_union` shape.
fn raw_vertex_union(objects: &[ObjectMesh]) -> Aabb {
    let mut min = point3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = point3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut any = false;
    for object in objects {
        for vertex in &object.mesh.vertices {
            any = true;
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }
    }
    any.then_some((min, max))
}

fn points_equal(left: Point3, right: Point3) -> bool {
    left.x == right.x && left.y == right.y && left.z == right.z
}

fn within_envelope(actual: f32, expected: f32, allowed: f32) -> bool {
    (actual - expected).abs() <= allowed
}

/// One ulp of `value`, derived with the neighbour operations rather than a
/// hard-coded exponent shift.
fn f32_ulp(value: f32) -> f32 {
    let magnitude = value.abs();
    if !magnitude.is_finite() {
        return f32::INFINITY;
    }
    if magnitude == 0.0 {
        return f32::from_bits(1);
    }
    let next = magnitude.next_up();
    if next.is_finite() {
        next - magnitude
    } else {
        magnitude - magnitude.next_down()
    }
}

fn check_pair_differences(before: &[Aabb], after: &[Aabb], allowed: f32) -> Result<(), String> {
    for left in 0..before.len() {
        let (Some((before_left_min, before_left_max)), Some((after_left_min, after_left_max))) =
            (before[left], after[left])
        else {
            continue;
        };
        for (right, (before_right, after_right)) in
            before.iter().zip(after).enumerate().skip(left + 1)
        {
            let (
                Some((before_right_min, before_right_max)),
                Some((after_right_min, after_right_max)),
            ) = (before_right, after_right)
            else {
                continue;
            };
            for (label, before_difference, after_difference) in [
                (
                    "min.x",
                    before_left_min.x - before_right_min.x,
                    after_left_min.x - after_right_min.x,
                ),
                (
                    "min.y",
                    before_left_min.y - before_right_min.y,
                    after_left_min.y - after_right_min.y,
                ),
                (
                    "min.z",
                    before_left_min.z - before_right_min.z,
                    after_left_min.z - after_right_min.z,
                ),
                (
                    "max.x",
                    before_left_max.x - before_right_max.x,
                    after_left_max.x - after_right_max.x,
                ),
                (
                    "max.y",
                    before_left_max.y - before_right_max.y,
                    after_left_max.y - after_right_max.y,
                ),
                (
                    "max.z",
                    before_left_max.z - before_right_max.z,
                    after_left_max.z - after_right_max.z,
                ),
            ] {
                if (after_difference - before_difference).abs() > allowed {
                    return Err(format!(
                        "object pair ({left}, {right}) {label} difference changed from \
                         {before_difference:?} to {after_difference:?} beyond the derived envelope \
                         {allowed:?}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// The shared placement checker. The property calls it with the real output;
/// `io_placement_controls` calls it with one deliberately mutated decision.
fn check_placement(
    before: &MeshIR,
    bed_center: (f32, f32),
    after: &MeshIR,
    placement: Option<BedPlacement>,
    limits: PlacementLimits,
) -> Result<(), String> {
    let before_aabbs: Vec<Aabb> = before
        .objects
        .iter()
        .map(object_world_aabb_oracle)
        .collect();
    let Some((min, max)) = union_of_aabbs(&before_aabbs) else {
        if placement.is_some() {
            return Err("an empty/unplaceable world must return None, not a placement".to_string());
        }
        if after != before {
            return Err("a None placement must not mutate the mesh".to_string());
        }
        return Ok(());
    };
    let expected = placement_from_union(min, max, bed_center);
    let Some(actual) = placement else {
        return Err(format!(
            "a placeable world returned None; the documented formula is {expected:?}"
        ));
    };
    for (label, actual_value, expected_value) in [
        ("dx", actual.dx, expected.dx),
        ("dy", actual.dy, expected.dy),
        ("dz", actual.dz, expected.dz),
    ] {
        if actual_value != expected_value {
            return Err(format!(
                "BedPlacement.{label} = {actual_value:?} ({:08x}), documented formula expects \
                 {expected_value:?} ({:08x})",
                actual_value.to_bits(),
                expected_value.to_bits()
            ));
        }
    }

    let envelope = 4.0 * f32::EPSILON * limits.max_abs_after();
    if !envelope.is_finite() || envelope <= 0.0 {
        return Err(format!(
            "the derived placement envelope {envelope:?} is not finite and positive"
        ));
    }
    let after_aabbs: Vec<Aabb> = after.objects.iter().map(object_world_aabb_oracle).collect();
    if after_aabbs.len() != before_aabbs.len() {
        return Err(format!(
            "placement changed the object count: {} -> {}",
            before_aabbs.len(),
            after_aabbs.len()
        ));
    }

    // One rigid translation for every object: no per-object re-centring, no
    // per-object re-drop, no per-object scale.
    for (index, (before_aabb, after_aabb)) in before_aabbs.iter().zip(&after_aabbs).enumerate() {
        match (before_aabb, after_aabb) {
            (Some((before_min, before_max)), Some((after_min, after_max))) => {
                for (label, actual_value, expected_value) in [
                    ("min.x", after_min.x, before_min.x + expected.dx),
                    ("min.y", after_min.y, before_min.y + expected.dy),
                    ("min.z", after_min.z, before_min.z + expected.dz),
                    ("max.x", after_max.x, before_max.x + expected.dx),
                    ("max.y", after_max.y, before_max.y + expected.dy),
                    ("max.z", after_max.z, before_max.z + expected.dz),
                ] {
                    if !within_envelope(actual_value, expected_value, envelope) {
                        return Err(format!(
                            "object {index} {label} = {actual_value:?}, but the whole-scene \
                             translation predicts {expected_value:?} (envelope {envelope:?})"
                        ));
                    }
                }
            }
            (None, None) => {}
            _ => {
                return Err(format!(
                    "object {index} changed its finite-X AABB presence during placement"
                ))
            }
        }
    }

    let Some((after_min, after_max)) = union_of_aabbs(&after_aabbs) else {
        return Err("a placed world lost its finite world AABB".to_string());
    };

    let z_allowed = 4.0 * f32::EPSILON * limits.max_abs_after();
    if after_min.z.abs() > z_allowed {
        return Err(format!(
            "placed world z_min = {:?}, expected 0 within {z_allowed:?}",
            after_min.z
        ));
    }

    let centre_x = (after_min.x + after_max.x) / 2.0;
    let centre_y = (after_min.y + after_max.y) / 2.0;
    for (axis, actual_value, expected_value) in
        [("x", centre_x, bed_center.0), ("y", centre_y, bed_center.1)]
    {
        let allowed = 2.0 * f32_ulp(actual_value.abs().max(expected_value.abs()));
        if (actual_value - expected_value).abs() > allowed {
            return Err(format!(
                "placed world {axis} centre = {actual_value:?}, bed_center = \
                 {expected_value:?}, allowed |delta| <= 2 ulp = {allowed:?}"
            ));
        }
    }

    check_pair_differences(&before_aabbs, &after_aabbs, envelope)?;

    // `build_volume` contract: it is the model-wide **world-space** AABB
    // union. `crates/pnp-cli/src/visual_debug.rs` reads it as the world-space
    // model extent (`mesh_xy_bounds` / `mesh_silhouette_plane_extent`), and
    // `bed_placement_tdd::build_volume_tracks_the_placed_model` requires it to
    // follow the placed model. Identity scenes additionally equal the shifted
    // raw-vertex union because a zero matrix stores world coordinates in the
    // vertices; the production's former raw-vertex recompute was a confirmed
    // defect for loader-assembled (real identity matrix) objects.
    let build_volume = after.build_volume;
    if !points_equal(build_volume.min, after_min) || !points_equal(build_volume.max, after_max) {
        return Err(format!(
            "post-placement build_volume {:?}..{:?} must equal the world-space AABB union \
             {after_min:?}..{after_max:?}",
            build_volume.min, build_volume.max
        ));
    }
    let all_identity = before
        .objects
        .iter()
        .all(|object| object.transform.matrix.iter().all(|value| *value == 0.0));
    if all_identity {
        let raw_union_after = raw_vertex_union(&after.objects);
        if !raw_union_after.is_some_and(|(raw_min, raw_max)| {
            points_equal(raw_min, build_volume.min) && points_equal(raw_max, build_volume.max)
        }) {
            return Err(format!(
                "identity-scene build_volume {:?} = the world union must also equal the union of \
                 the shifted raw vertices {raw_union_after:?}",
                (build_volume.min, build_volume.max)
            ));
        }
    }
    Ok(())
}

fn mesh_from_parts(parts: impl IntoIterator<Item = (Vec<Point3>, Transform3d)>) -> MeshIR {
    let objects: Vec<ObjectMesh> = parts
        .into_iter()
        .enumerate()
        .map(|(index, (vertices, transform))| ObjectMesh {
            id: format!("object-{index}"),
            mesh: IndexedTriangleSet {
                vertices,
                indices: Vec::new(),
            },
            transform,
            ..Default::default()
        })
        .collect();
    let build_volume = union_aabb(&objects)
        .map(|(min, max)| BoundingBox3 { min, max })
        .unwrap_or_default();
    MeshIR {
        objects,
        build_volume,
        ..Default::default()
    }
}

fn mesh_from_case(case: &PlacementCase) -> MeshIR {
    mesh_from_parts(
        case.objects
            .iter()
            .map(|fixture| (fixture.vertices.clone(), fixture.transform)),
    )
}

/// Mode-specific structural witness used by the shrink probe.
fn validate_placement_case(case: &PlacementCase, limits: PlacementLimits) -> Result<(), String> {
    if case.objects.len() > limits.max_objects {
        return Err(format!(
            "generated {} objects exceeds max_objects {}",
            case.objects.len(),
            limits.max_objects
        ));
    }
    if !case.bed_center.0.is_finite()
        || !case.bed_center.1.is_finite()
        || case.bed_center.0.abs() > limits.max_translation_mm
        || case.bed_center.1.abs() > limits.max_translation_mm
    {
        return Err(format!(
            "bed_center {:?} is outside the bounded domain",
            case.bed_center
        ));
    }
    let mut empty_objects = 0_usize;
    let mut nonempty_objects = 0_usize;
    for object in &case.objects {
        if object.vertices.len() > limits.max_vertices_per_object {
            return Err(format!(
                "generated {} vertices exceeds max_vertices_per_object {}",
                object.vertices.len(),
                limits.max_vertices_per_object
            ));
        }
        if object.vertices.is_empty() {
            empty_objects += 1;
        } else {
            nonempty_objects += 1;
        }
        for vertex in &object.vertices {
            if case.mode != PlacementMode::NonFiniteXWorld
                && (!vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite())
            {
                return Err("placement fixture generated a non-finite coordinate".to_string());
            }
        }
        let matrix = &object.transform.matrix;
        let identity = matrix.iter().all(|value| *value == 0.0);
        if !identity && !is_rigid_z(matrix) {
            return Err("placement fixture holds a non-rigid live transform".to_string());
        }
    }
    match case.mode {
        PlacementMode::EmptyWorld => {
            if !case.objects.is_empty() {
                return Err("empty-world mode generated objects".to_string());
            }
        }
        PlacementMode::Identity | PlacementMode::AlreadyPlaced | PlacementMode::BelowBed => {
            if nonempty_objects == 0
                || case
                    .objects
                    .iter()
                    .any(|object| !object.transform.matrix.iter().all(|value| *value == 0.0))
            {
                return Err(format!(
                    "{:?} mode needs nonempty zero-matrix objects",
                    case.mode
                ));
            }
        }
        PlacementMode::LiveTransform => {
            if nonempty_objects == 0
                || case
                    .objects
                    .iter()
                    .any(|object| object.transform.matrix.iter().all(|value| *value == 0.0))
            {
                return Err("live-transform mode needs nonempty live-matrix objects".to_string());
            }
        }
        PlacementMode::MixedWithEmpty => {
            if empty_objects == 0 || nonempty_objects == 0 {
                return Err(
                    "mixed-with-empty mode needs one empty and one nonempty object".to_string(),
                );
            }
        }
        PlacementMode::NonFiniteXWorld => {
            if nonempty_objects == 0 {
                return Err("non-finite-X mode needs a nonempty object".to_string());
            }
            let any_nonfinite_x = case
                .objects
                .iter()
                .flat_map(|object| object.vertices.iter())
                .any(|vertex| !vertex.x.is_finite());
            if !any_nonfinite_x
                || !case
                    .objects
                    .iter()
                    .all(|object| fixture_world_aabb(object).is_none())
            {
                return Err(
                    "non-finite-X mode must leave every object without a finite-X world AABB"
                        .to_string(),
                );
            }
        }
    }
    match case.mode {
        PlacementMode::EmptyWorld => {
            if fixtures_union(&case.objects).is_some() {
                return Err("empty-world mode has a finite world AABB".to_string());
            }
        }
        PlacementMode::NonFiniteXWorld => {
            if fixtures_union(&case.objects).is_some() {
                return Err(
                    "non-finite-X mode has an object accepted by the finite-X gate".to_string(),
                );
            }
        }
        PlacementMode::AlreadyPlaced => {
            let placement = placement_formula(&mesh_from_case(case), case.bed_center)
                .ok_or_else(|| "already-placed mode lost its world AABB".to_string())?;
            if placement.dx != 0.0 || placement.dy != 0.0 || placement.dz != 0.0 {
                return Err(format!(
                    "already-placed mode is not already placed: {placement:?}"
                ));
            }
        }
        PlacementMode::BelowBed => {
            let (min, max) = fixtures_union(&case.objects)
                .ok_or_else(|| "below-bed mode lost its world AABB".to_string())?;
            let placement = placement_from_union(min, max, case.bed_center);
            if placement.dz <= 0.0 {
                return Err(format!(
                    "below-bed mode must sit below the plate, got dz {:?}",
                    placement.dz
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

/// A matrix of one of the four Z quarter-turns plus a translation.
fn is_rigid_z(matrix: &[f64; 16]) -> bool {
    let linear = matches!(
        (matrix[0], matrix[1], matrix[4], matrix[5]),
        (1.0, 0.0, 0.0, 1.0)
            | (0.0, 1.0, -1.0, 0.0)
            | (-1.0, 0.0, 0.0, -1.0)
            | (0.0, -1.0, 1.0, 0.0)
    );
    linear
        && matrix[2] == 0.0
        && matrix[3] == 0.0
        && matrix[6] == 0.0
        && matrix[7] == 0.0
        && matrix[8] == 0.0
        && matrix[9] == 0.0
        && matrix[10] == 1.0
        && matrix[11] == 0.0
        && matrix[15] == 1.0
        && matrix[12].is_finite()
        && matrix[13].is_finite()
        && matrix[14].is_finite()
}

fn probe_placement_case(mode: PlacementMode, count: usize, offset: i64) -> PlacementCase {
    let bed_center = (offset as f32 / 8.0, -(offset as f32) / 8.0);
    let box_fixture = |position: usize| ObjectFixture {
        vertices: control_box(
            [
                position as f32 * 5.0 + offset as f32 / 8.0,
                offset as f32 / 8.0,
                0.0,
            ],
            [4.0, 3.0, 2.0],
        ),
        transform: zero_transform(),
    };
    let objects = match mode {
        PlacementMode::EmptyWorld => Vec::new(),
        PlacementMode::Identity => (0..count.max(1)).map(box_fixture).collect(),
        PlacementMode::LiveTransform => (0..count.max(1))
            .map(|position| {
                let mut matrix = [0.0_f64; 16];
                matrix[0] = 1.0;
                matrix[5] = 1.0;
                matrix[10] = 1.0;
                matrix[12] = position as f64 * 5.0 + offset as f64 / 8.0;
                matrix[13] = offset as f64 / 8.0;
                matrix[15] = 1.0;
                ObjectFixture {
                    vertices: control_box([0.0, 0.0, 0.0], [4.0, 3.0, 2.0]),
                    transform: Transform3d { matrix },
                }
            })
            .collect(),
        PlacementMode::MixedWithEmpty => {
            let mut objects = vec![empty_fixture()];
            objects.extend((0..count.max(1)).map(box_fixture));
            objects
        }
        PlacementMode::AlreadyPlaced => {
            let mut objects: Vec<ObjectFixture> = (0..count.max(1)).map(box_fixture).collect();
            centre_fixtures_on_bed(&mut objects, bed_center);
            objects
        }
        PlacementMode::BelowBed => {
            let mut objects: Vec<ObjectFixture> = (0..count.max(1)).map(box_fixture).collect();
            let shift = -(1.0 + offset as f32 / 8.0);
            for object in &mut objects {
                for vertex in &mut object.vertices {
                    vertex.z += shift;
                }
            }
            objects
        }
        PlacementMode::NonFiniteXWorld => vec![ObjectFixture {
            vertices: vec![point3(f32::INFINITY, 0.0, 0.0), point3(0.0, 1.0, 1.0)],
            transform: zero_transform(),
        }],
    };
    PlacementCase {
        mode,
        objects,
        bed_center,
    }
}

fn probe_placement_strategy(
    mode: PlacementMode,
    limits: PlacementLimits,
) -> BoxedStrategy<PlacementCase> {
    if matches!(
        mode,
        PlacementMode::EmptyWorld | PlacementMode::NonFiniteXWorld
    ) {
        return Just(probe_placement_case(mode, 0, 0)).boxed();
    }
    let max_count = if mode == PlacementMode::MixedWithEmpty {
        limits.max_objects - 1
    } else {
        limits.max_objects
    };
    (1..=max_count, 0..=8_i64)
        .prop_map(move |(count, offset)| probe_placement_case(mode, count, offset))
        .boxed()
}

fn run_placement_shrink_probe(limits: PlacementLimits) {
    for mode in PLACEMENT_MODES {
        let strategy = probe_placement_strategy(mode, limits);
        shrink_witness_probe(
            &format!("H-IO-PLACEMENT {mode:?}"),
            mode,
            &strategy,
            |case: &PlacementCase| case.mode,
            |case| validate_placement_case(case, limits),
            !matches!(
                mode,
                PlacementMode::EmptyWorld | PlacementMode::NonFiniteXWorld
            ),
        );
    }
}

/// Deterministic structural witnesses inside the property run: the charter
/// names empty, single, multi, already-placed, below-bed/off-centre and
/// empty-beside-nonempty scenes explicitly, so each is driven through the real
/// seam and the shared checker on every registry execution.
fn run_placement_production_witness(
    label: &str,
    before: MeshIR,
    bed_center: (f32, f32),
    limits: PlacementLimits,
) {
    let mut after = before.clone();
    let placement = place_bare_mesh_on_bed(&mut after, bed_center);
    check_placement(&before, bed_center, &after, placement, limits)
        .unwrap_or_else(|error| panic!("H-IO-PLACEMENT {label} witness: {error}"));
}

fn placement_structural_witnesses(limits: PlacementLimits) {
    let bed_center = (25.0, 30.0);
    run_placement_production_witness("empty world", MeshIR::default(), bed_center, limits);

    let single = mesh_from_parts(vec![(
        control_box([5.0, 7.0, 3.0], [6.0, 4.0, 2.0]),
        zero_transform(),
    )]);
    run_placement_production_witness("single object", single, bed_center, limits);

    let multi = mesh_from_parts(vec![
        (
            control_box([-10.0, 0.0, 2.0], [5.0, 5.0, 5.0]),
            zero_transform(),
        ),
        (
            control_box([12.0, 8.0, 0.0], [6.0, 4.0, 3.0]),
            zero_transform(),
        ),
    ]);
    run_placement_production_witness("multiple objects", multi.clone(), bed_center, limits);

    let mixed = mesh_from_parts(vec![
        (Vec::new(), zero_transform()),
        (
            control_box([3.0, 4.0, 9.0], [7.0, 6.0, 5.0]),
            zero_transform(),
        ),
    ]);
    run_placement_production_witness(
        "empty object beside a nonempty one",
        mixed,
        bed_center,
        limits,
    );

    let mut already = multi.clone();
    let (min, max) = union_aabb(&already.objects).expect("the multi-object witness is placeable");
    let shift = placement_from_union(min, max, bed_center);
    apply_translation(&mut already, shift.dx, shift.dy, shift.dz);
    run_placement_production_witness("already placed", already, bed_center, limits);

    let mut below = multi;
    apply_translation(&mut below, 3.0, -2.0, -5.0);
    run_placement_production_witness("below bed and off centre", below, bed_center, limits);

    // Live transform: the fixture's build_volume is the **pre-placement world
    // union** (not the former raw-space box), so any stale pre-placement value
    // is observable here. This is the deterministic witness for the fixed
    // `build_volume` contract; the bed-placement regression pins the true
    // former raw output.
    let mut matrix = [0.0_f64; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[10] = 1.0;
    matrix[12] = 11.0;
    matrix[13] = -7.0;
    matrix[14] = 3.5;
    matrix[15] = 1.0;
    let live = mesh_from_parts(vec![(
        control_box([0.0, 0.0, 0.0], [4.0, 3.0, 2.0]),
        Transform3d { matrix },
    )]);
    run_placement_production_witness("live transform", live, bed_center, limits);

    // All-nonfinite-X world: every object fails the finite-X AABB gate, so the
    // seam must return None and leave the mesh untouched.
    let mut nonfinite_x = control_box([0.0, 0.0, 0.0], [4.0, 3.0, 2.0]);
    nonfinite_x[0].x = f32::INFINITY;
    let nonfinite_x = mesh_from_parts(vec![(nonfinite_x, zero_transform())]);
    run_placement_production_witness("all-nonfinite-X world", nonfinite_x, bed_center, limits);
}

#[test]
fn proptest_place_bare_mesh_on_bed() {
    let run = PropertyRun::new(PLACEMENT_PROPERTY_ID, PLACEMENT_TEST_NAME)
        .expect("H-IO-PLACEMENT registry context");
    let limits = PlacementLimits::from_bounds(
        run.bound("max_objects").expect("max_objects"),
        run.bound("max_vertices_per_object")
            .expect("max_vertices_per_object"),
        run.bound("max_abs_mm").expect("max_abs_mm"),
        run.bound("max_translation_mm").expect("max_translation_mm"),
    )
    .expect("H-IO-PLACEMENT domain bounds");

    run_placement_shrink_probe(limits);
    placement_structural_witnesses(limits);
    let live_transform_cases = std::cell::Cell::new(0_usize);
    let strategy = placement_strategy(limits);
    run.check(&strategy, |case, _guard| {
        if case.mode == PlacementMode::LiveTransform {
            live_transform_cases.set(live_transform_cases.get() + 1);
        }
        let before = mesh_from_case(&case);
        let mut after = before.clone();
        let placement = place_bare_mesh_on_bed(&mut after, case.bed_center);
        check_placement(&before, case.bed_center, &after, placement, limits)
            .map_err(TestCaseError::fail)
    })
    .expect("H-IO-PLACEMENT rigid whole-scene translation contract");
    assert!(
        live_transform_cases.get() > 0,
        "the generated run must execute at least one live-transform scene so the world-space \
         build_volume contract is exercised through the property, not only the witness"
    );
}

// ===========================================================================
// Shared shrink / complicate witness probe.
// ===========================================================================

fn probe_runner(seed: u64) -> TestRunner {
    TestRunner::new(Config {
        cases: 1,
        max_local_rejects: 0,
        max_global_rejects: 0,
        max_flat_map_regens: 0,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..Config::default()
    })
}

/// Obtain a nonminimal tree, simplify to exhaustion, validate every state,
/// then complicate and validate again. `shrink_expected` is false for modes
/// whose strategy is entirely `Just`/`no_shrink`.
fn shrink_witness_probe<S, T, M>(
    label: &str,
    mode: M,
    strategy: &S,
    mode_of: impl Fn(&T) -> M,
    validate: impl Fn(&T) -> Result<(), String>,
    shrink_expected: bool,
) where
    S: Strategy<Value = T>,
    T: std::fmt::Debug,
    M: PartialEq + std::fmt::Debug + Copy,
{
    let mut obtained_nonminimal = false;
    for seed in 0..16_u64 {
        let mut runner = probe_runner(0x5eed_0000 + seed);
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("probe strategy builds a tree");
        assert_eq!(
            mode_of(&tree.current()),
            mode,
            "{label}: strategy produced the wrong mode"
        );
        let mut steps = 0_usize;
        while steps < 512 && tree.simplify() {
            steps += 1;
            assert_eq!(
                mode_of(&tree.current()),
                mode,
                "{label}: shrinking changed the required mode"
            );
            validate(&tree.current())
                .unwrap_or_else(|error| panic!("{label}: shrink step {steps}: {error}"));
        }
        validate(&tree.current()).unwrap_or_else(|error| panic!("{label}: minimal state: {error}"));
        for _ in 0..8 {
            if !tree.complicate() {
                break;
            }
            assert_eq!(
                mode_of(&tree.current()),
                mode,
                "{label}: complicating changed the required mode"
            );
            validate(&tree.current())
                .unwrap_or_else(|error| panic!("{label}: complicate step: {error}"));
        }
        if steps > 0 {
            obtained_nonminimal = true;
            break;
        }
        if !shrink_expected {
            break;
        }
    }
    if shrink_expected {
        assert!(
            obtained_nonminimal,
            "{label}: probe failed to obtain a nonminimal value tree, so the shrink audit \
             certifies nothing"
        );
    } else {
        assert!(
            !obtained_nonminimal,
            "{label}: a no-payload mode simplified unexpectedly"
        );
    }
}

// ===========================================================================
// Deterministic controls (no `PropertyRun`, no `PNP_PROPERTY_CONTEXT`).
// ===========================================================================

#[test]
fn io_extent_controls() {
    let limits = ExtentLimits::witness();

    let affine = |m2: f64, m6: f64, m10: f64, m14: f64, vertices: Vec<Point3>| {
        let mut matrix = [0.0_f64; 16];
        matrix[2] = m2;
        matrix[6] = m6;
        matrix[10] = m10;
        matrix[14] = m14;
        ExtentCase {
            mode: ExtentMode::Affine,
            vertices,
            transform: Transform3d { matrix },
        }
    };
    let expect = |case: &ExtentCase, expected: Option<(f32, f32)>| {
        let actual = object_world_z_extent(&object_from_extent_case(case));
        assert_eq!(actual, expected, "hand-computed {:?} witness", case.mode);
        check_extent_case(case, actual).expect("production must match the exact integer oracle");
    };

    // Each row term gets its own witness, so a dropped term produces a
    // distinguishable (and rejected) observation.
    let m2_case = affine(
        1.0,
        0.0,
        0.0,
        0.0,
        vec![point3(1.0, 0.0, 0.0), point3(0.0, 0.0, 2.0)],
    );
    expect(&m2_case, Some((0.0, 1.0)));
    assert!(
        check_extent_case(&m2_case, None).is_err(),
        "the checker must reject a missing m[2]*x term"
    );

    let m6_case = affine(
        0.0,
        1.0,
        0.0,
        0.0,
        vec![point3(0.0, 1.0, 0.0), point3(0.0, 0.0, 2.0)],
    );
    expect(&m6_case, Some((0.0, 1.0)));
    assert!(
        check_extent_case(&m6_case, None).is_err(),
        "the checker must reject a missing m[6]*y term"
    );

    let m10_case = affine(
        0.0,
        0.0,
        2.0,
        0.0,
        vec![point3(0.0, 0.0, 1.0), point3(0.0, 0.0, 2.0)],
    );
    expect(&m10_case, Some((2.0, 4.0)));
    assert!(
        check_extent_case(&m10_case, None).is_err(),
        "the checker must reject a missing m[10]*z term"
    );

    let m14_case = affine(
        0.0,
        0.0,
        2.0,
        1.0,
        vec![point3(0.0, 0.0, 0.0), point3(0.0, 0.0, 1.0)],
    );
    expect(&m14_case, Some((1.0, 3.0)));
    assert!(
        check_extent_case(&m14_case, Some((0.0, 2.0))).is_err(),
        "the checker must reject a missing m[14] translation"
    );

    // Combined witness for the transform-aware versus raw-vertex distinction
    // and the ordered extrema.
    let combined = affine(
        1.0,
        1.0,
        2.0,
        1.0,
        vec![
            point3(1.0, 0.0, 0.0),
            point3(0.0, 0.0, 3.0),
            point3(0.0, 1.0, 0.0),
        ],
    );
    expect(&combined, Some((2.0, 7.0)));
    assert!(
        check_extent_case(&combined, Some((0.0, 3.0))).is_err(),
        "the checker must reject a transform-blind raw-vertex extent"
    );
    assert!(
        check_extent_case(&combined, Some((7.0, 2.0))).is_err(),
        "the checker must reject swapped extrema"
    );
    assert!(
        check_extent_case(&combined, Some((1.0, 7.0))).is_err(),
        "the checker must reject a missing m[2]*x term in the combined witness"
    );

    // Degenerate: constant transformed Z must be None, and a fabricated Some
    // is rejected.
    let flat = ExtentCase {
        mode: ExtentMode::Flat,
        vertices: vec![point3(0.0, 0.0, 5.0), point3(3.0, 4.0, 5.0)],
        transform: zero_transform(),
    };
    assert_eq!(object_world_z_extent(&object_from_extent_case(&flat)), None);
    check_extent_case(&flat, None).expect("degenerate extent must be None");
    assert!(
        check_extent_case(&flat, Some((5.0, 5.0))).is_err(),
        "the checker must reject a fabricated Some for a degenerate extent"
    );

    // Empty: None, and a fabricated Some is rejected.
    let empty = ExtentCase {
        mode: ExtentMode::Identity,
        vertices: Vec::new(),
        transform: zero_transform(),
    };
    assert_eq!(
        object_world_z_extent(&object_from_extent_case(&empty)),
        None
    );
    check_extent_case(&empty, None).expect("empty mesh must be None");
    assert!(
        check_extent_case(&empty, Some((0.0, 1.0))).is_err(),
        "the checker must reject a fabricated Some for an empty mesh"
    );

    // The all-zero matrix is identity, not a collapse to the origin.
    let zero_matrix = ExtentCase {
        mode: ExtentMode::Identity,
        vertices: vec![point3(0.0, 0.0, -2.0), point3(0.0, 0.0, 6.0)],
        transform: zero_transform(),
    };
    assert_eq!(
        object_world_z_extent(&object_from_extent_case(&zero_matrix)),
        Some((-2.0, 6.0))
    );
    check_extent_case(&zero_matrix, Some((-2.0, 6.0)))
        .expect("zero matrix must be treated as identity");

    // Non-finite policy, separately labelled: NaN vertices are ignored, an
    // infinity makes the reduced extent non-finite and returns None. Neither
    // is a valid printable object.
    let nan_ignored = ExtentCase {
        mode: ExtentMode::NonFiniteNan,
        vertices: vec![
            point3(0.0, 0.0, 0.0),
            point3(0.0, 0.0, 2.0),
            point3(0.0, 0.0, f32::NAN),
        ],
        transform: zero_transform(),
    };
    assert_eq!(
        object_world_z_extent(&object_from_extent_case(&nan_ignored)),
        Some((0.0, 2.0)),
        "NaN vertices are ignored by the comparison-based reduction"
    );
    check_extent_case(&nan_ignored, Some((0.0, 2.0))).expect("NaN-ignore policy");
    let infinity_rejected = ExtentCase {
        mode: ExtentMode::NonFiniteInfinity,
        vertices: vec![
            point3(0.0, 0.0, 0.0),
            point3(0.0, 0.0, 2.0),
            point3(0.0, 0.0, f32::INFINITY),
        ],
        transform: zero_transform(),
    };
    assert_eq!(
        object_world_z_extent(&object_from_extent_case(&infinity_rejected)),
        None,
        "an infinity reaching an extremum yields a non-finite extent"
    );
    check_extent_case(&infinity_rejected, None).expect("infinity-rejects policy");

    run_extent_shrink_probe(limits);
}

fn control_box(origin: [f32; 3], dims: [f32; 3]) -> Vec<Point3> {
    let [x, y, z] = origin;
    let [a, b, c] = dims;
    vec![
        point3(x, y, z),
        point3(x + a, y, z),
        point3(x + a, y + b, z),
        point3(x, y + b, z),
        point3(x, y, z + c),
        point3(x + a, y, z + c),
        point3(x + a, y + b, z + c),
        point3(x, y + b, z + c),
    ]
}

/// Test-local mirror of the production rigid-application and `build_volume`
/// recompute, used only to apply a deliberate mutation to the after-mesh.
fn apply_translation(mesh: &mut MeshIR, dx: f32, dy: f32, dz: f32) {
    for object in &mut mesh.objects {
        if object.transform.matrix.iter().all(|value| *value == 0.0) {
            for vertex in &mut object.mesh.vertices {
                vertex.x += dx;
                vertex.y += dy;
                vertex.z += dz;
            }
        } else {
            let matrix = &mut object.transform.matrix;
            matrix[12] += f64::from(dx);
            matrix[13] += f64::from(dy);
            matrix[14] += f64::from(dz);
        }
    }
    mesh.build_volume = raw_vertex_union(&mesh.objects)
        .map(|(min, max)| BoundingBox3 { min, max })
        .unwrap_or_default();
}

#[test]
fn io_placement_controls() {
    let limits = PlacementLimits::witness();
    assert_eq!(limits.max_abs_after(), 466.0);

    let bed_center = (50.0, 60.0);
    // Two distinct boxes: one below the plate (z_min = 5) and one higher, at
    // different XY extents, so per-object mutations are observable.
    let before = mesh_from_parts(vec![
        (
            control_box([-12.0, -4.0, 5.0], [10.0, 8.0, 6.0]),
            zero_transform(),
        ),
        (
            control_box([28.0, 6.0, 7.0], [12.0, 8.0, 6.0]),
            zero_transform(),
        ),
    ]);
    let expected = placement_formula(&before, bed_center).expect("placeable control scene");
    assert_eq!(expected.dz, -5.0, "the control scene sits below the plate");

    // The real production output must pass the shared checker.
    let mut production_after = before.clone();
    let production_placement = place_bare_mesh_on_bed(&mut production_after, bed_center);
    assert_eq!(production_placement, Some(expected));
    check_placement(
        &before,
        bed_center,
        &production_after,
        production_placement,
        limits,
    )
    .expect("the true placement must pass the shared checker");

    // Mutation 1: per-object centring/re-dropping instead of one translation.
    let mut per_object = before.clone();
    for object in &mut per_object.objects {
        let (min, max) = object_world_aabb_oracle(object).expect("nonempty control object");
        let dx = bed_center.0 - (min.x + max.x) / 2.0;
        let dy = bed_center.1 - (min.y + max.y) / 2.0;
        let dz = -min.z;
        for vertex in &mut object.mesh.vertices {
            vertex.x += dx;
            vertex.y += dy;
            vertex.z += dz;
        }
    }
    per_object.build_volume = raw_vertex_union(&per_object.objects)
        .map(|(min, max)| BoundingBox3 { min, max })
        .unwrap_or_default();
    assert!(
        check_placement(&before, bed_center, &per_object, Some(expected), limits).is_err(),
        "the checker must reject per-object centring"
    );

    // Mutation 2: translation-only XY, without the Z drop.
    let mut xy_only = before.clone();
    apply_translation(&mut xy_only, expected.dx, expected.dy, 0.0);
    assert!(
        check_placement(&before, bed_center, &xy_only, Some(expected), limits).is_err(),
        "the checker must reject a translation-only XY shift without the Z drop"
    );

    // Mutation 3: a Z-only drop of an off-centre world (no XY centring).
    let mut z_only = before.clone();
    apply_translation(&mut z_only, 0.0, 0.0, expected.dz);
    assert!(
        check_placement(&before, bed_center, &z_only, Some(expected), limits).is_err(),
        "the checker must reject a Z-only drop that leaves the XY AABB off the bed centre"
    );

    // Mutation 4: a non-rigid per-object scale about each object's centre.
    let mut scaled = before.clone();
    apply_translation(&mut scaled, expected.dx, expected.dy, expected.dz);
    for object in &mut scaled.objects {
        let (min, max) = object_world_aabb_oracle(object).expect("nonempty control object");
        let centre = point3(
            (min.x + max.x) / 2.0,
            (min.y + max.y) / 2.0,
            (min.z + max.z) / 2.0,
        );
        for vertex in &mut object.mesh.vertices {
            vertex.x = centre.x + 1.5 * (vertex.x - centre.x);
            vertex.y = centre.y + 1.5 * (vertex.y - centre.y);
            vertex.z = centre.z + 1.5 * (vertex.z - centre.z);
        }
    }
    scaled.build_volume = raw_vertex_union(&scaled.objects)
        .map(|(min, max)| BoundingBox3 { min, max })
        .unwrap_or_default();
    assert!(
        check_placement(&before, bed_center, &scaled, Some(expected), limits).is_err(),
        "the checker must reject a non-rigid per-object scale"
    );

    // Mutation 5: fabricated placements for unplaceable worlds.
    let empty = MeshIR::default();
    check_placement(&empty, bed_center, &empty, None, limits)
        .expect("an empty world legitimately returns None");
    assert!(
        check_placement(
            &empty,
            bed_center,
            &empty,
            Some(BedPlacement {
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
            }),
            limits,
        )
        .is_err(),
        "the checker must reject a fabricated placement for an empty world"
    );
    let all_empty = mesh_from_parts(vec![(Vec::new(), zero_transform())]);
    check_placement(&all_empty, bed_center, &all_empty, None, limits)
        .expect("a world of empty objects legitimately returns None");
    assert!(check_placement(
        &all_empty,
        bed_center,
        &all_empty,
        Some(BedPlacement {
            dx: 1.0,
            dy: 0.0,
            dz: 0.0,
        }),
        limits,
    )
    .is_err());

    // Mutation 6: the all-nonfinite-X world is unplaceable even though it has
    // a nonempty object, because no object passes the finite-X AABB gate.
    let mut nonfinite_x = control_box([0.0, 0.0, 0.0], [4.0, 3.0, 2.0]);
    nonfinite_x[0].x = f32::NEG_INFINITY;
    let nonfinite_x = mesh_from_parts(vec![(nonfinite_x, zero_transform())]);
    assert!(
        place_bare_mesh_on_bed(&mut nonfinite_x.clone(), bed_center).is_none(),
        "an all-nonfinite-X world must return None"
    );
    check_placement(&nonfinite_x, bed_center, &nonfinite_x, None, limits)
        .expect("an all-nonfinite-X world legitimately returns None");
    assert!(
        check_placement(
            &nonfinite_x,
            bed_center,
            &nonfinite_x,
            Some(BedPlacement {
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
            }),
            limits,
        )
        .is_err(),
        "the checker must reject a fabricated placement for an all-nonfinite-X world"
    );

    // Mutation 7: a live-transform scene with the **pre-placement world union**
    // as `build_volume` is rejected. `mesh_from_parts` seeds the box from
    // `union_aabb`, which already applies the live transform, so this is not
    // the former raw-space box (the bed-placement regression
    // `identity_matrix_placement_updates_build_volume_to_the_world_union` pins
    // that). Both stale boxes are wrong after placement; this one proves the
    // world-space contract independently of the identity convention.
    let mut live_matrix = [0.0_f64; 16];
    live_matrix[0] = 1.0;
    live_matrix[5] = 1.0;
    live_matrix[10] = 1.0;
    live_matrix[12] = 9.0;
    live_matrix[13] = -4.0;
    live_matrix[14] = 2.5;
    live_matrix[15] = 1.0;
    let live_before = mesh_from_parts(vec![(
        control_box([0.0, 0.0, 0.0], [4.0, 3.0, 2.0]),
        Transform3d {
            matrix: live_matrix,
        },
    )]);
    let mut live_after = live_before.clone();
    let live_placement = place_bare_mesh_on_bed(&mut live_after, bed_center);
    check_placement(
        &live_before,
        bed_center,
        &live_after,
        live_placement,
        limits,
    )
    .expect("the fixed world-space build_volume must be accepted for a live transform");
    let mut stale = live_after.clone();
    stale.build_volume = live_before.build_volume;
    assert!(
        check_placement(&live_before, bed_center, &stale, live_placement, limits).is_err(),
        "a stale pre-placement build_volume must be rejected for a live transform"
    );

    // A zero-shift placement must still refresh `build_volume`: seed a stale
    // box and require the recompute (the old early return kept it).
    let mut already_placed = before.clone();
    let (min, max) = union_aabb(&already_placed.objects).expect("placeable");
    let shift = placement_from_union(min, max, bed_center);
    apply_translation(&mut already_placed, shift.dx, shift.dy, shift.dz);
    let stale_box = BoundingBox3 {
        min: Point3 {
            x: -9.0e3,
            y: -9.0e3,
            z: -9.0e3,
        },
        max: Point3 {
            x: 9.0e3,
            y: 9.0e3,
            z: 9.0e3,
        },
    };
    already_placed.build_volume = stale_box;
    let mut zero_shift_after = already_placed.clone();
    let zero_shift_placement = place_bare_mesh_on_bed(&mut zero_shift_after, bed_center);
    assert_eq!(
        zero_shift_placement,
        Some(BedPlacement {
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
        }),
        "the already-placed scene must produce an exactly-zero shift"
    );
    assert_ne!(
        zero_shift_after.build_volume, stale_box,
        "a zero-shift placement must still refresh a stale build_volume"
    );
    check_placement(
        &already_placed,
        bed_center,
        &zero_shift_after,
        zero_shift_placement,
        limits,
    )
    .expect("the refreshed build_volume must satisfy the identity-scene contract");

    run_placement_shrink_probe(limits);
}
