//! Charter `H-ARACHNE-GRAPH`: real skeletal-graph construction relations.
//!
//! # What is called
//!
//! Every generated and deterministic case drives the **production**
//! `slicer_core::skeletal_trapezoidation::SkeletalTrapezoidationGraph::from_polygons`
//! (`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`). No local
//! construction, mirrored pipeline graph or production compactor is used as
//! the oracle.
//!
//! # Input model (shared with `H-ARACHNE-PIPELINE`)
//!
//! Inputs come from #2 `domains::geometry_cases` (independently validated by
//! `oracles::validate_geometry` inside `GeometryStrategy::new_tree`), are
//! scaled by the exact integer [`SCALE`] (1 catalog unit = 2_500 canonical
//! units = 0.25 mm) and validated **again** against a realized
//! [`GeometryDomain`] whose `max_abs_units` is the live registry bound. The
//! second validation is a precondition assertion, never a rejection that
//! consumes the property budget: an invalid scaled case is an authoring bug,
//! so it panics in the strategy map rather than shrinking forever.
//!
//! Classes sampled: `Empty`, `Convex`, `Concave`, `Holed`, `Thin` and
//! `Disconnected`. `NonStarShaped` is not sampled (its kernel witness is
//! scale-invariant but adds no graph relation), and `NearDegenerate` is
//! **excluded by derivation**, not by observation: `edge_clearance_witness`
//! (`crates/slicer-sdk/src/test_support/property_testing/oracles/exact.rs`)
//! requires a non-adjacent edge pair at distance exactly one *catalog* unit,
//! so the `1 x 1 -> SCALE x SCALE` map invalidates its witness.
//!
//! # Declared outcomes
//!
//! The contract is *measured*, not assumed. [`CAMPAIGN_CLASSES`] is exactly
//! the accepted-class set asserted by [`check_case`]; `Empty` must return
//! `SktError::EmptyInput`, and every non-empty class must return a graph with
//! a non-empty vertex/edge population (asserted, so a constructor regression
//! that silently returns an empty graph fails here). `NearDegenerate` would
//! be the only class with a scale-incompatible witness, and it is not
//! sampled. This table is re-measured by the deterministic witnesses and by
//! the registry pilot recorded in the packet evidence; a class that starts
//! returning an error is a regression, not an accepted outcome.
//!
//! # Independent relation checker
//!
//! [`check_relations`] indexes `vertices`/`edges` itself and never calls the
//! production compactor. It proves:
//!
//! * every `start_vertex` is `NO_INDEX` or `< vertices.len()`;
//! * every non-sentinel `twin`/`next`/`prev` is `< edges.len()`;
//! * `twin.twin == e`, `next.prev == e`, `prev.next == e` whenever the link
//!   exists (reciprocity, not just range);
//! * a twin's `start_vertex` resolves the edge endpoint (non-sentinel and in
//!   range);
//! * the two construction-stage side tables are vertex-parallel;
//! * `0 <= r_min <= r_max` whenever both endpoint radii are finite and
//!   nonnegative; a non-finite `distance_to_boundary` is an allowed sentinel
//!   and is never required to be finite;
//! * every finite `distance_to_boundary` is nonnegative.
//!
//! # Bounds (authoring values, to be re-measured in Step 7)
//!
//! | key | minimum | routine | extended |
//! |---|---:|---:|---:|
//! | `max_abs_units` | 100_000 (10 mm) | 400_000 | 1_000_000 |
//! | `max_vertices` | 8 | 24 | 32 |
//! | `max_holes` | 1 | 2 | 3 |
//! | `max_components` | 2 | 3 | 4 |
//! | `max_shrink_candidates` | 1 | 64 | 256 |
//!
//! `max_abs_units` must admit the deterministic 10 mm square (its far corner
//! is 100_000 units) and the `1_000_000`-unit bound of the #2
//! `validate_geometry` domain. `max_vertices = 8` is the Holed (`4 + 4`) and
//! Thin (`8`) template minimum; `max_components = 2` is the Disconnected
//! template minimum; `max_holes = 1` is the Holed minimum. The catalog bound
//! `max_abs_units / SCALE` must stay at or above the Thin template's `8`
//! catalog-unit span.

use proptest::prelude::*;
use proptest::strategy::Union;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::skeletal_trapezoidation::{STVertex, SkeletalTrapezoidationGraph, SktError};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};
use slicer_sdk::test_support::property_testing::domains::{
    geometry_cases, BoundaryRule, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::validate_geometry;
use slicer_sdk::test_support::property_testing::PropertyRun;

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-ARACHNE-GRAPH";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_arachne_graph_topology";

/// Exact integer scale from #2 catalog units to canonical units.
///
/// One catalog unit becomes 2_500 units = 0.25 mm. A catalog `1 x 1` feature
/// (the smallest hole or gap the catalog emits) is therefore 0.25 mm across:
/// comfortably above the 11.5 µm preprocessing epsilon, while a `32`-unit
/// catalog extent becomes 8 mm. The map is an integer multiplication, so
/// winding, topology witnesses, disjointness and positive area are preserved
/// exactly (the one scale-variant witness, `NearDegenerate`, is excluded).
pub const SCALE: i64 = 2_500;

/// Smallest accepted `max_abs_units`: the deterministic 10 mm square reaches
/// 100_000 units, and the #2 catalog places coordinates within this bound.
pub const MIN_ABS_UNITS: u64 = 100_000;
/// Largest accepted `max_abs_units`: the #2 `validate_geometry` domain limit.
pub const MAX_ABS_UNITS: u64 = 1_000_000;
/// Smallest accepted `max_vertices`: Holed needs `4 + 4`, Thin needs `8`.
pub const MIN_VERTICES: u64 = 8;
/// Largest accepted `max_vertices`: the #2 catalog's own ring cap.
pub const MAX_VERTICES: u64 = 64;
/// Smallest accepted `max_holes`: the Holed class needs one.
pub const MIN_HOLES: u64 = 1;
/// Largest accepted `max_holes`: the #2 catalog's own ring cap.
pub const MAX_HOLES: u64 = 64;
/// Smallest accepted `max_components`: Disconnected needs two.
pub const MIN_COMPONENTS: u64 = 2;
/// Largest accepted `max_components`: the #2 catalog's own ring cap.
pub const MAX_COMPONENTS: u64 = 64;
/// Largest accepted `max_shrink_candidates` (an allocation guard, not a
/// coverage claim).
pub const MAX_SHRINK_CANDIDATES: u64 = 1_000_000;
/// Smallest catalog half-span that still fits the Thin template's 16-unit
/// extent (`span == 2 * catalog bound`).
pub const MIN_CATALOG_ABS_UNITS: u64 = 8;

/// Smallest guaranteed-output rectangle edge (4 mm).
pub const ROBUST_RECT_MIN_MM: i64 = 4;
/// Largest guaranteed-output rectangle edge (8 mm).
pub const ROBUST_RECT_MAX_MM: i64 = 8;

/// The topology classes the measured campaign samples, in fixed order.
///
/// `Empty` is the declared `EmptyInput` outcome; every other entry is a
/// declared `Ok` class. This is the accepted/rejected table the charter asks
/// for, and [`check_case`] asserts it entry by entry.
pub const CAMPAIGN_CLASSES: [TopologyClass; 6] = [
    TopologyClass::Empty,
    TopologyClass::Convex,
    TopologyClass::Concave,
    TopologyClass::Holed,
    TopologyClass::Thin,
    TopologyClass::Disconnected,
];

// ─────────────────────────────────────────────────────────────────────────────
// Checked profile bounds.
// ─────────────────────────────────────────────────────────────────────────────

/// A rejected profile bound or a failed precondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundsError {
    message: String,
}

impl BoundsError {
    /// Wrap a profile/authoring diagnostic.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BoundsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BoundsError {}

/// Raw `H-ARACHNE-GRAPH` profile keys, before validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawGraphBounds {
    /// `max_abs_units`.
    pub max_abs_units: u64,
    /// `max_vertices`.
    pub max_vertices: u64,
    /// `max_holes`.
    pub max_holes: u64,
    /// `max_components`.
    pub max_components: u64,
    /// `max_shrink_candidates`.
    pub max_shrink_candidates: u64,
}

impl RawGraphBounds {
    /// Read every `H-ARACHNE-GRAPH` key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        let read = |name: &str| {
            run.bound(name)
                .map_err(|error| BoundsError::new(format!("bound {name}: {error}")))
        };
        Ok(Self {
            max_abs_units: read("max_abs_units")?,
            max_vertices: read("max_vertices")?,
            max_holes: read("max_holes")?,
            max_components: read("max_components")?,
            max_shrink_candidates: read("max_shrink_candidates")?,
        })
    }
}

/// Validated `H-ARACHNE-GRAPH` limits, converted without narrowing loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphLimits {
    /// Largest absolute coordinate, canonical units.
    pub max_abs_units: i64,
    /// Largest total vertex count across all rings.
    pub max_vertices: usize,
    /// Largest total hole count across all components.
    pub max_holes: usize,
    /// Largest outer-component count.
    pub max_components: usize,
    /// Largest shrink-candidate count.
    pub max_shrink_candidates: u32,
    /// Catalog half-span, `max_abs_units / SCALE`.
    pub catalog_abs_units: i64,
}

/// A checked integer conversion that keeps the value in `range`.
fn in_range(name: &str, value: u64, range: (u64, u64)) -> Result<u64, BoundsError> {
    if !(range.0..=range.1).contains(&value) {
        return Err(BoundsError::new(format!(
            "bound {name} = {value} is outside the accepted range {}..={}",
            range.0, range.1
        )));
    }
    Ok(value)
}

fn to_i64(name: &str, value: u64) -> Result<i64, BoundsError> {
    i64::try_from(value)
        .map_err(|_| BoundsError::new(format!("bound {name} = {value} does not fit i64")))
}

fn to_usize(name: &str, value: u64) -> Result<usize, BoundsError> {
    usize::try_from(value)
        .map_err(|_| BoundsError::new(format!("bound {name} = {value} does not fit usize")))
}

fn to_u32(name: &str, value: u64) -> Result<u32, BoundsError> {
    u32::try_from(value)
        .map_err(|_| BoundsError::new(format!("bound {name} = {value} does not fit u32")))
}

/// Validate a raw profile before any strategy exists.
pub fn validate_graph_bounds(raw: RawGraphBounds) -> Result<GraphLimits, BoundsError> {
    let max_abs_units = to_i64(
        "max_abs_units",
        in_range(
            "max_abs_units",
            raw.max_abs_units,
            (MIN_ABS_UNITS, MAX_ABS_UNITS),
        )?,
    )?;
    let max_vertices = to_usize(
        "max_vertices",
        in_range(
            "max_vertices",
            raw.max_vertices,
            (MIN_VERTICES, MAX_VERTICES),
        )?,
    )?;
    let max_holes = to_usize(
        "max_holes",
        in_range("max_holes", raw.max_holes, (MIN_HOLES, MAX_HOLES))?,
    )?;
    let max_components = to_usize(
        "max_components",
        in_range(
            "max_components",
            raw.max_components,
            (MIN_COMPONENTS, MAX_COMPONENTS),
        )?,
    )?;
    let max_shrink_candidates = to_u32(
        "max_shrink_candidates",
        in_range(
            "max_shrink_candidates",
            raw.max_shrink_candidates,
            (1, MAX_SHRINK_CANDIDATES),
        )?,
    )?;
    let catalog_abs_units = max_abs_units / SCALE;
    if catalog_abs_units < MIN_CATALOG_ABS_UNITS as i64 {
        return Err(BoundsError::new(format!(
            "max_abs_units = {max_abs_units} leaves a catalog half-span of {catalog_abs_units} \
             units; the Thin template needs at least {MIN_CATALOG_ABS_UNITS} (SCALE = {SCALE})"
        )));
    }
    Ok(GraphLimits {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
        catalog_abs_units,
    })
}

/// A valid reference profile used by the deterministic controls (authoring
/// tier, not a measured registry profile).
pub fn reference_limits() -> GraphLimits {
    validate_graph_bounds(RawGraphBounds {
        max_abs_units: 400_000,
        max_vertices: 24,
        max_holes: 2,
        max_components: 3,
        max_shrink_candidates: 64,
    })
    .expect("reference H-ARACHNE-GRAPH profile")
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared input model.
// ─────────────────────────────────────────────────────────────────────────────

/// A realized (scaled, validated) catalog case tagged with its class.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedCase {
    /// Producing topology class.
    pub class: TopologyClass,
    /// Realized geometry in canonical units.
    pub case: GeometryCase,
}

/// Build the `GeometryDomain` whose template the catalog strategy samples.
///
/// The catalog half-span is deliberately much smaller than the realized
/// bound: the exact [`SCALE`] map then puts every coordinate within the live
/// `max_abs_units` without a second placement pass.
pub fn catalog_domain(
    class: TopologyClass,
    catalog_abs_units: i64,
    limits: &GraphLimits,
) -> GeometryDomain {
    // exhaustive: every field is a deliberate charter choice, never a helper default
    GeometryDomain {
        class,
        min_components: if class == TopologyClass::Empty {
            0
        } else if class == TopologyClass::Disconnected {
            2
        } else {
            1
        },
        min_holes: if class == TopologyClass::Holed { 1 } else { 0 },
        boundary: BoundaryRule::Strict,
        // exhaustive: live profiles must supply every measured bound, never the helper default
        bounds: GeometryBounds {
            max_abs_units: catalog_abs_units,
            max_vertices: limits.max_vertices,
            max_holes: limits.max_holes,
            max_components: limits.max_components,
            max_shrink_candidates: limits.max_shrink_candidates,
        },
    }
}

/// The domain a realized case is validated against.
pub fn realized_domain(class: TopologyClass, limits: &GraphLimits) -> GeometryDomain {
    // exhaustive: the realized bound is the live registry bound
    GeometryDomain {
        class,
        min_components: if class == TopologyClass::Empty {
            0
        } else if class == TopologyClass::Disconnected {
            2
        } else {
            1
        },
        min_holes: if class == TopologyClass::Holed { 1 } else { 0 },
        boundary: BoundaryRule::Strict,
        // exhaustive: the realized domain is the live registry bound, not a helper default
        bounds: GeometryBounds {
            max_abs_units: limits.max_abs_units,
            max_vertices: limits.max_vertices,
            max_holes: limits.max_holes,
            max_components: limits.max_components,
            max_shrink_candidates: limits.max_shrink_candidates,
        },
    }
}

/// Exact integer scaling of every ring coordinate.
fn scale_case(case: &GeometryCase) -> GeometryCase {
    let scale_ring = |ring: &Polygon| Polygon {
        points: ring
            .points
            .iter()
            .map(|p| Point2 {
                x: p.x
                    .checked_mul(SCALE)
                    .expect("scaled x coordinate fits i64"),
                y: p.y
                    .checked_mul(SCALE)
                    .expect("scaled y coordinate fits i64"),
            })
            .collect(),
    };
    GeometryCase {
        polygons: case
            .polygons
            .iter()
            .map(|polygon| ExPolygon {
                contour: scale_ring(&polygon.contour),
                holes: polygon.holes.iter().map(scale_ring).collect(),
            })
            .collect(),
    }
}

/// The shared bounded strategy: one independently validated, exactly scaled
/// catalog case per generated value, tagged with its declared class.
///
/// One extra branch generates the guaranteed-output rectangle family directly
/// (4..=8 mm on each axis, origin-anchored, exact integer coordinates). It is a
/// valid `Convex` realized case, and it makes the guaranteed-output class of
/// `H-ARACHNE-PIPELINE` non-vacuous by construction instead of relying on the
/// catalog RNG to emit a large plain rectangle.
pub fn campaign_strategy(limits: &GraphLimits) -> Result<BoxedStrategy<TaggedCase>, BoundsError> {
    let mut branches = Vec::with_capacity(CAMPAIGN_CLASSES.len() + 1);
    for class in CAMPAIGN_CLASSES {
        let catalog = catalog_domain(class, limits.catalog_abs_units, limits);
        let realized = realized_domain(class, limits);
        let strategy = geometry_cases(catalog).map_err(|error| {
            BoundsError::new(format!("{class:?} catalog domain is unusable: {error}"))
        })?;
        branches.push(
            strategy
                .prop_map(move |case| {
                    let realized_case = scale_case(&case);
                    if let Err(error) = validate_geometry(&realized_case, &realized) {
                        panic!(
                            "{class:?} scaled case violated its realized domain: {error}; \
                             case = {realized_case:?}"
                        );
                    }
                    TaggedCase {
                        class,
                        case: realized_case,
                    }
                })
                .boxed(),
        );
    }
    let realized = realized_domain(TopologyClass::Convex, limits);
    branches.push(
        (
            ROBUST_RECT_MIN_MM..=ROBUST_RECT_MAX_MM,
            ROBUST_RECT_MIN_MM..=ROBUST_RECT_MAX_MM,
        )
            .prop_map(move |(width_mm, height_mm)| {
                let width = mm_units(width_mm);
                let height = mm_units(height_mm);
                let case = GeometryCase {
                    polygons: vec![ExPolygon {
                        contour: Polygon {
                            points: vec![
                                Point2 { x: 0, y: 0 },
                                Point2 { x: width, y: 0 },
                                Point2 {
                                    x: width,
                                    y: height,
                                },
                                Point2 { x: 0, y: height },
                            ],
                        },
                        holes: Vec::new(),
                    }],
                };
                if let Err(error) = validate_geometry(&case, &realized) {
                    panic!("guaranteed rectangle violated its realized domain: {error}");
                }
                TaggedCase {
                    class: TopologyClass::Convex,
                    case,
                }
            })
            .boxed(),
    );
    Ok(Union::new(branches).boxed())
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent indexed relation checker.
// ─────────────────────────────────────────────────────────────────────────────

/// Observed populations, returned so callers can assert non-vacuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphPopulations {
    /// `graph.vertices.len()`.
    pub vertices: usize,
    /// `graph.edges.len()`.
    pub edges: usize,
    /// Edges carrying a non-sentinel `twin`.
    pub twins: usize,
    /// Edges whose twin's `start_vertex` resolves the edge endpoint.
    pub resolved_endpoints: usize,
}

fn vertex_index(vertices: &[STVertex], index: usize) -> Option<&STVertex> {
    if index == NO_INDEX {
        return None;
    }
    vertices.get(index)
}

/// Prove the construction-stage relations on a graph, using only its public
/// index fields.
pub fn check_relations(graph: &SkeletalTrapezoidationGraph) -> Result<GraphPopulations, String> {
    let vertex_count = graph.vertices.len();
    let edge_count = graph.edges.len();
    let mut populations = GraphPopulations {
        vertices: vertex_count,
        edges: edge_count,
        twins: 0,
        resolved_endpoints: 0,
    };

    for (idx, vertex) in graph.vertices.iter().enumerate() {
        if vertex.distance_to_boundary.is_finite() && vertex.distance_to_boundary < 0.0 {
            return Err(format!(
                "vertex {idx}: finite distance_to_boundary {} is negative",
                vertex.distance_to_boundary
            ));
        }
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        if edge.start_vertex != NO_INDEX && edge.start_vertex >= vertex_count {
            return Err(format!(
                "edge {idx}: start_vertex {} is neither NO_INDEX nor < vertices.len() ({vertex_count})",
                edge.start_vertex
            ));
        }
        for (name, value) in [
            ("twin", edge.twin),
            ("next", edge.next),
            ("prev", edge.prev),
        ] {
            if value != NO_INDEX && value >= edge_count {
                return Err(format!(
                    "edge {idx}: {name} {value} is neither NO_INDEX nor < edges.len() ({edge_count})"
                ));
            }
        }
        if edge.twin != NO_INDEX {
            populations.twins += 1;
            let twin = &graph.edges[edge.twin];
            if twin.twin != idx {
                return Err(format!(
                    "edge {idx}: twin reciprocity broken (edges[{0}].twin = {1}, expected {idx})",
                    edge.twin, twin.twin
                ));
            }
            if twin.start_vertex == NO_INDEX || twin.start_vertex >= vertex_count {
                return Err(format!(
                    "edge {idx}: twin {} start_vertex {} does not resolve the edge endpoint",
                    edge.twin, twin.start_vertex
                ));
            }
            populations.resolved_endpoints += 1;
        }
        if edge.next != NO_INDEX && graph.edges[edge.next].prev != idx {
            return Err(format!(
                "edge {idx}: next {} does not point back via prev ({})",
                edge.next, graph.edges[edge.next].prev
            ));
        }
        if edge.prev != NO_INDEX && graph.edges[edge.prev].next != idx {
            return Err(format!(
                "edge {idx}: prev {} does not point back via next ({})",
                edge.prev, graph.edges[edge.prev].next
            ));
        }

        // Radius relation only in the declared subdomain where both endpoint
        // radii are finite and nonnegative. A NaN `distance_to_boundary` is an
        // allowed sentinel and is never required to be finite.
        let from = vertex_index(&graph.vertices, edge.start_vertex);
        let to = if edge.twin == NO_INDEX {
            None
        } else {
            graph
                .edges
                .get(edge.twin)
                .and_then(|twin| vertex_index(&graph.vertices, twin.start_vertex))
        };
        if let (Some(from), Some(to)) = (from, to) {
            let a = from.distance_to_boundary;
            let b = to.distance_to_boundary;
            if a.is_finite() && a >= 0.0 && b.is_finite() && b >= 0.0 {
                if !edge.r_min.is_finite() || !edge.r_max.is_finite() {
                    return Err(format!(
                        "edge {idx}: finite endpoint radii ({a}, {b}) but non-finite bounds \
                         r_min = {}, r_max = {}",
                        edge.r_min, edge.r_max
                    ));
                }
                if edge.r_min > edge.r_max {
                    return Err(format!(
                        "edge {idx}: r_min {} > r_max {} for finite endpoint radii ({a}, {b})",
                        edge.r_min, edge.r_max
                    ));
                }
                if edge.r_min < 0.0 || edge.r_max < 0.0 {
                    return Err(format!(
                        "edge {idx}: negative radius bounds r_min = {}, r_max = {}",
                        edge.r_min, edge.r_max
                    ));
                }
            }
        }
    }

    if graph.beading_propagation.len() != vertex_count {
        return Err(format!(
            "construction-stage side table: beading_propagation.len() = {} != vertices.len() = \
             {vertex_count}",
            graph.beading_propagation.len()
        ));
    }
    if graph.beading_dist_to_bottom_source.len() != vertex_count {
        return Err(format!(
            "construction-stage side table: beading_dist_to_bottom_source.len() = {} != \
             vertices.len() = {vertex_count}",
            graph.beading_dist_to_bottom_source.len()
        ));
    }

    Ok(populations)
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses and per-case checking.
// ─────────────────────────────────────────────────────────────────────────────

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

/// Whole millimetres to canonical units (1 unit = 100 nm; 1 mm = 10_000
/// units).
fn mm_units(mm: i64) -> i64 {
    (mm as f64 * UNITS_PER_MM) as i64
}

fn expoly(points: Vec<Point2>, holes: Vec<Vec<Point2>>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: holes.into_iter().map(|points| Polygon { points }).collect(),
    }
}

/// A 4 mm square, wholly inside the minimum accepted profile.
fn rectangle_witness() -> ExPolygon {
    let side = mm_units(4);
    expoly(
        vec![p(0, 0), p(side, 0), p(side, side), p(0, side)],
        Vec::new(),
    )
}

/// A 4 mm square with a centred 1 mm hole (a genuine holed outline).
fn holed_witness() -> ExPolygon {
    let side = mm_units(4);
    let hole_side = mm_units(1);
    let offset = (side - hole_side) / 2;
    expoly(
        vec![p(0, 0), p(side, 0), p(side, side), p(0, side)],
        vec![vec![
            p(offset, offset),
            p(offset + hole_side, offset),
            p(offset + hole_side, offset + hole_side),
            p(offset, offset + hole_side),
        ]],
    )
}

/// A 4 mm L-shaped concave outline.
fn concave_witness() -> ExPolygon {
    let side = mm_units(4);
    let leg = mm_units(2);
    expoly(
        vec![
            p(0, 0),
            p(side, 0),
            p(side, leg),
            p(leg, leg),
            p(leg, side),
            p(0, side),
        ],
        Vec::new(),
    )
}

/// Prove the documented accepted outcomes on named deterministic witnesses.
///
/// These run before any generated case, so the mandatory rectangle and
/// holed/concave populations cannot be absent merely because the RNG did not
/// sample them.
pub fn deterministic_witnesses(limits: &GraphLimits) {
    let rectangle = rectangle_witness();
    let graph = SkeletalTrapezoidationGraph::from_polygons(&[rectangle.clone()])
        .expect("H-ARACHNE-GRAPH: the documented 4 mm rectangle must construct");
    let populations =
        check_relations(&graph).expect("H-ARACHNE-GRAPH: rectangle relations must hold");
    assert!(
        populations.vertices > 0 && populations.edges > 0,
        "H-ARACHNE-GRAPH: the documented rectangle must construct a nonempty graph, got \
         {populations:?}"
    );
    assert!(
        populations.twins > 0 && populations.resolved_endpoints == populations.twins,
        "H-ARACHNE-GRAPH: every rectangle twin must resolve its endpoint, got {populations:?}"
    );
    assert_eq!(
        graph.beading_propagation.len(),
        graph.vertices.len(),
        "H-ARACHNE-GRAPH: beading_propagation is vertex-parallel at construction"
    );
    assert_eq!(
        graph.beading_dist_to_bottom_source.len(),
        graph.vertices.len(),
        "H-ARACHNE-GRAPH: beading_dist_to_bottom_source is vertex-parallel at construction"
    );

    for (label, witness) in [("holed", holed_witness()), ("concave", concave_witness())] {
        let graph =
            SkeletalTrapezoidationGraph::from_polygons(&[witness]).unwrap_or_else(|error| {
                panic!(
                    "H-ARACHNE-GRAPH: the documented {label} witness must construct, got {error}"
                )
            });
        let populations = check_relations(&graph)
            .unwrap_or_else(|error| panic!("H-ARACHNE-GRAPH: {label} relations: {error}"));
        assert!(
            populations.vertices > 0 && populations.edges > 0,
            "H-ARACHNE-GRAPH: the documented {label} witness must be nonempty, got {populations:?}"
        );
    }

    // A class-appropriate input is validated before construction; an input
    // outside the declared profile is not a graph case at all.
    let domain = realized_domain(TopologyClass::Convex, limits);
    validate_geometry(
        &GeometryCase {
            polygons: vec![rectangle],
        },
        &domain,
    )
    .expect("H-ARACHNE-GRAPH: the rectangle witness is inside the realized domain");
}

/// Check one generated case against its declared class contract.
pub fn check_case(input: &TaggedCase, limits: &GraphLimits) -> Result<(), String> {
    let domain = realized_domain(input.class, limits);
    validate_geometry(&input.case, &domain).map_err(|error| {
        format!(
            "H-ARACHNE-GRAPH precondition: {:?} case is not valid for the realized domain \
             (authoring bug, not a shrinkable rejection): {error}",
            input.class
        )
    })?;

    match input.class {
        TopologyClass::Empty => {
            let empty = SkeletalTrapezoidationGraph::from_polygons(&input.case.polygons);
            match empty {
                Err(SktError::EmptyInput) => Ok(()),
                other => Err(format!(
                    "H-ARACHNE-GRAPH: the Empty class must return SktError::EmptyInput, got \
                     {other:?}"
                )),
            }
        }
        TopologyClass::Convex
        | TopologyClass::Concave
        | TopologyClass::Holed
        | TopologyClass::Thin
        | TopologyClass::Disconnected => {
            let graph = SkeletalTrapezoidationGraph::from_polygons(&input.case.polygons).map_err(
                |error| {
                    format!(
                        "H-ARACHNE-GRAPH: accepted class {:?} returned an unexpected error: \
                         {error:?} (declared outcome table: Empty -> EmptyInput, all other \
                         sampled classes -> Ok with a nonempty graph)",
                        input.class
                    )
                },
            )?;
            let populations = check_relations(&graph)?;
            if populations.vertices == 0 || populations.edges == 0 {
                return Err(format!(
                    "H-ARACHNE-GRAPH: accepted class {:?} returned an empty graph \
                     {populations:?}",
                    input.class
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "H-ARACHNE-GRAPH: class {other:?} is outside the measured accepted-class table"
        )),
    }
}

/// Run the registry-selected `H-ARACHNE-GRAPH` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME)
        .expect("use the registry-backed property command for H-ARACHNE-GRAPH");
    let raw = RawGraphBounds::read(&run).expect("H-ARACHNE-GRAPH bounds");
    let limits = validate_graph_bounds(raw).expect("H-ARACHNE-GRAPH profile");
    deterministic_witnesses(&limits);
    let strategy = campaign_strategy(&limits).expect("H-ARACHNE-GRAPH strategy");
    run.check(&strategy, |input, _guard| -> TestCaseResult {
        check_case(&input, &limits).map_err(TestCaseError::fail)
    })
    .expect("H-ARACHNE-GRAPH charter");
}
