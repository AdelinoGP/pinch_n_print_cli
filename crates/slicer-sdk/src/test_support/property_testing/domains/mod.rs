//! Shared bounded input domains for native property tests.

use std::fmt;
use std::ops::RangeInclusive;

use proptest::strategy::BoxedStrategy;
use slicer_ir::{ConfigValue, LayerCollectionIR, Point2};

mod geometry;
mod graphs;
mod ir;
mod mesh;
mod scalars;

/// Finite values within the inclusive range, including reachable IEEE witnesses.
pub fn finite_f32(range: RangeInclusive<f32>) -> Result<BoxedStrategy<f32>, DomainError> {
    scalars::finite_f32(range)
}

/// Finite values within the inclusive range, including reachable IEEE witnesses.
pub fn finite_f64(range: RangeInclusive<f64>) -> Result<BoxedStrategy<f64>, DomainError> {
    scalars::finite_f64(range)
}

/// Infinities and distinguishable NaN bit patterns.
pub fn nonfinite_f32() -> BoxedStrategy<f32> {
    scalars::nonfinite_f32()
}

/// Infinities and distinguishable NaN bit patterns.
pub fn nonfinite_f64() -> BoxedStrategy<f64> {
    scalars::nonfinite_f64()
}

/// Float neighbors of half-unit conversion boundaries in millimeters.
///
/// Probes `(k + 0.5) / 10_000` for `-max_abs_units <= k < max_abs_units`.
/// One unit is 100 nm; probes do not guarantee exact conversion. The bound
/// must be in `1..=1_000_000`.
pub fn rounding_boundary_mm(max_abs_units: i64) -> Result<BoxedStrategy<f32>, DomainError> {
    scalars::rounding_boundary_mm(max_abs_units)
}

/// Generate one requested scalar type and range, not a valid whole configuration.
pub fn scalar_config_values(
    domain: ScalarConfigDomain,
) -> Result<BoxedStrategy<ConfigValue>, DomainError> {
    scalars::scalar_config_values(domain)
}

/// Generate lowercase UUID-shaped text, or its complement within the byte limit.
///
/// The byte limit must not exceed 4096. UUID syntax requires at least 36 bytes;
/// the non-ASCII complement witness requires at least two bytes.
pub fn id_text(kind: IdTextKind, max_bytes: usize) -> Result<BoxedStrategy<String>, DomainError> {
    scalars::id_text(kind, max_bytes)
}

/// Generate small well-formed DAGs or directed cycles with acyclic tails.
///
/// The node limit is at most 32, at least one for DAGs and three for cycles.
pub fn graphs(kind: GraphKind, max_nodes: u16) -> Result<BoxedStrategy<SmallGraph>, DomainError> {
    graphs::graphs(kind, max_nodes)
}

/// Generate raw graphs with a fault that survives shrinking.
///
/// These are only for APIs validating raw input, never already-validated
/// scheduler contracts. The node-entry limit is at most 32; a duplicate needs
/// at least two entries and an unknown endpoint needs at least one node.
pub fn malformed_graphs(
    kind: MalformedGraphKind,
    max_nodes: u16,
) -> Result<BoxedStrategy<SmallGraph>, DomainError> {
    graphs::malformed_graphs(kind, max_nodes)
}

/// Generate travel-anchor membership fixtures, not complete printable layers.
///
/// Each count is at most 4096; a dangling case requires at least one travel.
pub fn anchor_cases(
    domain: AnchorDomain,
    max_entities: usize,
    max_travels: usize,
) -> Result<BoxedStrategy<AnchorCase>, DomainError> {
    ir::anchor_cases(domain, max_entities, max_travels)
}

/// Generate bounded layers with fresh IDs and matching role/path metadata.
///
/// Role choices must be nonempty. Each dimension is at most 128, and their
/// product is at most 100,000 points. If `max_points` is zero, layers have no
/// entities. These limits are allocation guards, not printing validation.
pub fn entity_layers(
    domain: EntityDomain,
) -> Result<BoxedStrategy<Vec<LayerCollectionIR>>, DomainError> {
    ir::entity_layers(domain)
}

/// Generate a complete permutation by sorting shrinkable random keys.
pub fn permutations(len: usize) -> BoxedStrategy<Vec<usize>> {
    ir::permutations(len)
}

/// Generate permutations of authored-order contiguous, disjoint lock groups.
///
/// Groups must be nonempty, disjoint and in range; the length is at most 4096.
pub fn locked_orders(
    len: usize,
    groups: Vec<Vec<usize>>,
) -> Result<BoxedStrategy<Vec<usize>>, DomainError> {
    ir::locked_orders(len, groups)
}

/// A bounded catalog strategy whose value tree retains valid topology witnesses.
#[derive(Debug, Clone)]
pub struct GeometryStrategy {
    domain: GeometryDomain,
    params: BoxedStrategy<geometry::Params>,
}

/// A reversible, bounded exploration of simpler valid geometry cases.
///
/// Exhausting the candidate limit does not prove global minimality.
#[derive(Debug)]
pub struct GeometryValueTree {
    domain: GeometryDomain,
    current: GeometryCase,
    history: Vec<GeometryCase>,
    rejected: Vec<(GeometryCase, GeometryCase)>,
}

/// Construct a strategy for the bounded integer-geometry catalog.
///
/// Minimum primary-ring vertex counts are: Empty 0, Convex 3 (a triangle),
/// Concave 6, NonStarShaped 8, Holed 4 plus four per hole, Disconnected 4
/// per component, and Thin and NearDegenerate 8. Other required components
/// and holes add four vertices each. The catalog supports at most 64 required
/// components and 64 required holes; coordinates are canonical 100 nm units.
/// Requests whose required template cannot fit the supplied bounds are rejected.
pub fn geometry_cases(domain: GeometryDomain) -> Result<GeometryStrategy, DomainError> {
    geometry::geometry_cases(domain)
}

/// Count components, holes, vertices, and checked absolute coordinate units.
pub fn geometry_complexity(case: &GeometryCase) -> Result<GeometryComplexity, DomainError> {
    geometry::geometry_complexity(case)
}

/// Generate bounded integer segments, including axis, diagonal and point witnesses.
///
/// `max_abs_units` must be in `1..=1_000_000`; shrinking stays within that bound.
pub fn line_segments(bounds: GeometryBounds) -> Result<BoxedStrategy<[Point2; 2]>, DomainError> {
    geometry::line_segments(bounds)
}

/// Generate malformed geometry whose named fault survives shrinking.
///
/// These cases are only for APIs that promise to validate raw geometry.
pub fn invalid_geometry_cases(
    kind: InvalidGeometryKind,
    bounds: GeometryBounds,
) -> Result<BoxedStrategy<InvalidGeometryCase>, DomainError> {
    geometry::invalid_geometry_cases(kind, bounds)
}

/// Generate closed, outward-oriented boxes or right triangular prisms.
///
/// Extents must be finite and in `(0, 10_000]` mm. Dimensions and origins
/// lie on a power-of-two grid: 1/8 mm for bounds at least 1/8 mm, and a
/// smaller power of two otherwise. Thus even positive subnormal bounds work.
/// `max_extent_mm` bounds each solid dimension; the origin is chosen from a
/// separate grid limited to 16 grid units, so an absolute vertex coordinate can
/// exceed `max_extent_mm` by that bounded offset (at most 2 mm at the 1/8 mm
/// grid).
///
/// For these solids, let `M` be the largest absolute vertex coordinate in mm.
/// Comparing the independent tetrahedron sum with the dimension-derived volume
/// has a conservative absolute budget of `128 * f64::EPSILON * M^3` mm³.
/// With `u = f64::EPSILON / 2`, the sum has at most 72 signed monomials of
/// magnitude at most `M^3`; each passes through at most 17 rounded operations
/// including the final division by six. Their absolute error is at most
/// `(72/6)*gamma(17)*M^3`, where `gamma(17) = 17u/(1-17u) < 18u`; this is
/// below `108 * f64::EPSILON * M^3`. The dimensional volume is exact: grid
/// indices are at most 80,016 and the product of the three dimensions fits
/// in 53 bits. Nonzero intermediates remain normal in f64 even at the smallest
/// positive f32 grid, and f32 vertex coordinates are exactly representable.
pub fn analytical_meshes(
    kind: SolidKind,
    max_extent_mm: f32,
) -> Result<BoxedStrategy<MeshCase>, DomainError> {
    mesh::analytical_meshes(kind, max_extent_mm)
}

/// Generate explicit column-major Z quarter-turns and bounded translations.
///
/// The finite translation bound must be in `[0, 10_000]` mm. Rotations and
/// dyadic translations are exactly representable in f64; a sign/swap oracle
/// on suitably bounded points on the same grid needs no tolerance. Arbitrary
/// rotations and their floating-point error are outside this domain.
pub fn rigid_transforms(
    max_translation_mm: f32,
) -> Result<BoxedStrategy<RigidTransformCase>, DomainError> {
    mesh::rigid_transforms(max_translation_mm)
}

/// Generate a malformed indexed mesh whose named fault survives shrinking.
pub fn invalid_meshes(kind: InvalidMeshKind) -> BoxedStrategy<slicer_ir::IndexedTriangleSet> {
    mesh::invalid_meshes(kind)
}

/// Failure to construct or operate on a requested property-test domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// A requested bound is invalid or inconsistent.
    InvalidBounds(
        /// Description of the invalid bounds.
        String,
    ),
    /// An input does not satisfy a domain's requirements.
    InvalidInput(
        /// Description of the invalid input.
        String,
    ),
    /// Checked domain arithmetic exceeded its representable range.
    ArithmeticOverflow,
    /// The requested domain is not supported.
    UnsupportedDomain(
        /// Description of the unsupported domain.
        String,
    ),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds(message) => write!(f, "invalid domain bounds: {message}"),
            Self::InvalidInput(message) => write!(f, "invalid domain input: {message}"),
            Self::ArithmeticOverflow => write!(f, "domain arithmetic overflow"),
            Self::UnsupportedDomain(message) => write!(f, "unsupported domain: {message}"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Requested scalar configuration value family and bounds.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarConfigDomain {
    /// Boolean configuration values.
    Bool,
    /// Integer configuration values in the requested range.
    Int {
        /// Inclusive minimum integer.
        min: i64,
        /// Inclusive maximum integer.
        max: i64,
    },
    /// Floating-point configuration values in the requested range.
    Float {
        /// Inclusive minimum value.
        min: f64,
        /// Inclusive maximum value.
        max: f64,
    },
    /// Percentage configuration values in the requested range.
    Percent {
        /// Inclusive minimum percentage.
        min: f64,
        /// Inclusive maximum percentage.
        max: f64,
    },
    /// Floating-point or percentage configuration values in the requested range.
    FloatOrPercent {
        /// Inclusive minimum value.
        min: f64,
        /// Inclusive maximum value.
        max: f64,
        /// Whether the value represents a percentage.
        is_percent: bool,
    },
    /// Configuration strings chosen from the supplied values.
    StringChoices(
        /// Available string values.
        Vec<String>,
    ),
}

/// Requested syntax family for identifier text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdTextKind {
    /// Lowercase UUID-shaped hexadecimal text.
    UuidSyntax,
    /// Text outside the UUID syntax family.
    OtherText,
}

/// A small directed graph represented by node IDs and endpoint pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmallGraph {
    /// Node identifiers in the graph.
    pub node_ids: Vec<u16>,
    /// Directed edges as source and destination identifiers.
    pub edges: Vec<(u16, u16)>,
}

/// Requested well-formed graph family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphKind {
    /// A directed acyclic graph.
    Dag,
    /// A directed cycle with an acyclic downstream tail.
    CycleWithTail,
}

/// Requested malformed graph witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedGraphKind {
    /// A node identifier appears more than once.
    DuplicateNode,
    /// An edge refers to an unknown node identifier.
    UnknownEndpoint,
}

/// Requested travel-anchor membership family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorDomain {
    /// All travel anchors belong to the layer's ordered entities.
    Valid,
    /// At least one travel anchor is absent from the ordered entities.
    Dangling,
}

/// A layer containing a travel-anchor membership case.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorCase {
    /// Layer containing the ordered entities and travel moves.
    pub layer: slicer_ir::LayerCollectionIR,
}

/// Requested role choices and size limits for entity layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDomain {
    /// Permitted extrusion roles.
    pub roles: Vec<slicer_ir::ExtrusionRole>,
    /// Maximum entity count.
    pub max_entities: usize,
    /// Maximum path point count.
    pub max_points: usize,
    /// Maximum layer count.
    pub max_layers: usize,
}

/// Requested bounded integer-geometry topology family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyClass {
    /// No polygon components.
    Empty,
    /// A convex polygon.
    Convex,
    /// A concave polygon.
    Concave,
    /// A polygon with an empty visibility kernel.
    NonStarShaped,
    /// Geometry containing at least one hole.
    Holed,
    /// Geometry containing multiple disconnected components.
    Disconnected,
    /// Geometry with a thin feature such as a corridor or neck.
    Thin,
    /// Geometry with a near-degenerate feature such as a one-unit clearance.
    NearDegenerate,
}

/// Whether containment queries include region boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryRule {
    /// Only region interior is included.
    Strict,
    /// Region interior and boundaries are included.
    Inclusive,
}

/// Limits for bounded integer geometry and shrink exploration.
///
/// The default is a safe helper base only; live property profiles must supply
/// their measured bounds explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryBounds {
    /// Maximum absolute coordinate in canonical units, each 100 nm.
    pub max_abs_units: i64,
    /// Maximum total vertices across all rings.
    pub max_vertices: usize,
    /// Maximum total holes across all components.
    pub max_holes: usize,
    /// Maximum number of outer components.
    pub max_components: usize,
    /// Maximum number of shrink candidates to consider.
    pub max_shrink_candidates: u32,
}

/// Provides safe helper defaults only; live property profiles must supply
/// their measured bounds explicitly.
impl Default for GeometryBounds {
    fn default() -> Self {
        Self {
            max_abs_units: 10_000,
            max_vertices: 64,
            max_holes: 4,
            max_components: 4,
            max_shrink_candidates: 64,
        }
    }
}

/// Requested topology, minimum structure, containment rule, and bounds.
///
/// The default is a safe helper base only; live property profiles must supply
/// their measured bounds explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryDomain {
    /// Requested topology family.
    pub class: TopologyClass,
    /// Minimum total holes across all components.
    pub min_holes: usize,
    /// Minimum number of outer components.
    pub min_components: usize,
    /// Rule for region-boundary containment queries.
    pub boundary: BoundaryRule,
    /// Size and shrink exploration limits.
    pub bounds: GeometryBounds,
}

/// Provides a safe helper default only; live property profiles must supply
/// their measured bounds explicitly.
impl Default for GeometryDomain {
    fn default() -> Self {
        Self {
            class: TopologyClass::Convex,
            min_holes: 0,
            min_components: 1,
            boundary: BoundaryRule::Strict,
            bounds: GeometryBounds::default(),
        }
    }
}

/// A polygon-set geometry case.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryCase {
    /// Outer polygons and their holes.
    pub polygons: Vec<slicer_ir::ExPolygon>,
}

/// Component, hole, and vertex counts followed by the sum of absolute XY units.
pub type GeometryComplexity = (usize, usize, usize, u128);

/// Requested malformed integer-geometry witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidGeometryKind {
    /// Ring edges cross.
    Crossing,
    /// A ring contains a duplicate vertex.
    DuplicateVertex,
    /// A ring has collapsed geometry.
    Collapsed,
    /// A hole touches a boundary.
    TouchingHole,
    /// A hole is nested incorrectly.
    MisnestedHole,
}

/// Polygon-set geometry with a named malformed witness.
#[derive(Debug, Clone, PartialEq)]
pub struct InvalidGeometryCase {
    /// Outer polygons and their holes.
    pub polygons: Vec<slicer_ir::ExPolygon>,
    /// Malformation the case is intended to exhibit.
    pub kind: InvalidGeometryKind,
}

/// Requested analytically described solid family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidKind {
    /// Rectangular box.
    Box,
    /// Triangular prism.
    TriangularPrism,
}

/// A mesh paired with independently expected bounds and volume.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshCase {
    /// Indexed triangle mesh.
    pub mesh: slicer_ir::IndexedTriangleSet,
    /// Bounds expected from the solid's dimensions.
    pub expected_bounds: slicer_ir::BoundingBox3,
    /// Volume expected from the solid's dimensions, in cubic millimeters.
    pub expected_volume_mm3: f64,
}

/// A rigid Z quarter-turn and translation with independent parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct RigidTransformCase {
    /// Column-major transformation matrix.
    pub transform: slicer_ir::Transform3d,
    /// Translation along X, Y, and Z in millimeters.
    pub translation_mm: [f64; 3],
    /// Number of quarter-turns around the Z axis.
    pub quarter_turns_z: u8,
}

/// Requested malformed indexed-mesh witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidMeshKind {
    /// An index refers beyond the vertex array.
    BadIndex,
    /// The index stream ends with an incomplete triangle.
    TruncatedTriangle,
    /// At least one vertex coordinate is nonfinite.
    NonFiniteVertex,
    /// At least one triangle has degenerate geometry.
    DegenerateTriangle,
}
