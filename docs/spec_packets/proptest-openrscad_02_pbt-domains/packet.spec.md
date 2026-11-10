---
packet: proptest-openrscad_02_pbt-domains
status: draft
---

# Direct property domains, independent oracles and useful shrinking

## Goal

Provide bounded direct scalar, graph, IR, integer-geometry and analytical-mesh strategies with independently tested predicates, concrete regression formatting and witness-preserving shrinking, accepted through measured Windows pilots.

## Why

The approved plan's §§5–6 and G-GEOMETRY require much more than random data that happens not to panic. Later owner packets need valid domains, independent mathematical/reference predicates and shrinks that retain the reason a case matters. `validate_polygon_simplicity` (`crates/slicer-core/src/polygon_ops.rs`) delegates to Clipper self-union, so it cannot be the sole independent validity oracle for tests of Clipper operations.

This packet supplies direct inputs and oracle building blocks without blocking host work on OpenRSCAD. Its bounded template families are deliberately not advertised as a general simple-polygon generator; the combined direct/scripted coverage decision remains with the scripted packet.

## Scope

- In: shared direct strategies for finite/exceptional scalars, scalar config values and ID text; tiny graph models; travel anchors, entity/layer sequences and locked permutations; integer rings/holes/sets/segments with named topology classes; analytical indexed solids and rigid transforms; independent predicates/reference models; concrete Rust fixture formatting; shrink protocol probes; registry entries, measured profiles and evidence for these helpers.
- Out: OpenRSCAD/recipes/SCAD features; a mandatory arbitrary/general-polygon generator; new production fields, validators or visibility; full host/module charter coverage; a live OrcaSlicer differential harness; blanket `Arbitrary` derives; random module-entrypoint/E2E quotas; automatic production fixes or fake committed regressions when no SUT defect was found.

## Depends on

- `docs/spec_packets/proptest-openrscad_01_pbt-foundation/` (**draft at authoring**): consumes its planned native SDK `property_testing`, `PropertyRun::{new,bound,check}`, `CaseGuard`, target-local artifacts and `test-properties.toml` schema. This packet may be authored now but implementation/closure waits for that packet to be implemented and its exports rechecked on disk. No prerequisite is claimed satisfied.

## Design notes

### Existing contracts and independence boundary

- `Point2 { x: i64, y: i64 }`, `Polygon { points: Vec<Point2> }`, `ExPolygon { contour: Polygon, holes: Vec<Polygon> }`, `IndexedTriangleSet { vertices: Vec<Point3>, indices: Vec<u32> }`, `BoundingBox3`, `Transform3d`, `ConfigValue` and the string aliases `ObjectId`/`ModuleId` were checked in `crates/slicer-ir/src/slice_ir.rs`. `ExPolygon` outer winding is CCW, holes CW; indices are a flat triangle stream, not `Vec<[u32; 3]>`. `Transform3d::default()` is all zeros, not identity; explicit column-major matrices are required.
- `ConfigValue` float equality compares `to_bits()` for `Float`, `Percent` and `FloatOrPercent`; signed zero and NaN payloads must not be erased by fixture formatting. Typed scalar-config domains certify only their requested scalar range/type, not the scheduler's cross-field validation. Role/config combinations must be explicit compatible fixtures; never call an arbitrary `ResolvedConfig` valid because each isolated field looks valid.
- `topological_sort` (`crates/slicer-scheduler/src/topology.rs`) returns unsorted IDs, including acyclic downstream tails blocked by cycles. The graph oracle here uses exhaustive small-graph path reachability, not a copy of Kahn's algorithm, and imports no scheduler code. Graph-to-`ModuleNode` conversion belongs to the scheduler owner packet.
- `validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`) checks only membership in `ordered_entities`, not global entity validity, lock semantics or other side tables. Preserve the foundation property's identity and exact membership charter when replacing its local strategy.
- ADR-0062 (`docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md`) makes equal `order_lock` tags within a layer/object/region atomic authored-order/direction blocks. Abstract lock-index strategies here represent those blocks; they do not prove the production host applies them. The later `pbt-layers-seams` packet will exercise actual mutation paths.
- Put all new support under the foundation's native-only `slicer_sdk::test_support::property_testing` gate. Predicates must not call `slicer-core`, Clipper, `geo`, production geometry validators or production slicing/containment helpers. SDK already depends on core; dependency-package separation alone is not oracle independence. Record the actual implementation dependencies of every oracle in its charter. No OrcaSlicer behavior is borrowed or translated in this packet; do not invent canonical citations or attribution headers for original mathematics.

<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`).

Direct integer strategies intentionally construct `Point2` coordinates in canonical units, with unit comments: do not introduce a float round trip to create exact test geometry. Float-boundary probes derive the f32 multiplication/rounding error separately; finite does not imply overflow-free. Z and mesh coordinates remain millimeters.

<!-- snippet: wasm-staleness -->
- Feeds guest WASM: run `cargo xtask build-guests --check` after edits and judge by exit code (0 fresh, 1 stale, 3 infrastructure error) before interpreting any guest test failure (AGENTS.md §Guest WASM Staleness).

### New shared exports: exact downstream contract

Everything in this section is **new planned support**, crate `slicer-sdk`, under `slicer_sdk::test_support::property_testing`. Submodule names are part of the handoff: `domains`, `oracles`, `regression`; implementation may use private subfiles below them. Types/functions below are public only within that feature-gated test-support surface. Existing production IR types retain their names/shapes. `BoxedStrategy` means `proptest::strategy::BoxedStrategy`; no blanket IR `Arbitrary` implementation is added.

Common errors, in the `domains` submodule:

```rust
pub enum DomainError {
    InvalidBounds(String),
    InvalidInput(String),
    ArithmeticOverflow,
    UnsupportedDomain(String),
}
// DomainError: Debug + Display + std::error::Error.
```

Scalar/config/ID/graph exports, all in `domains` (the `use` lines below are documentation shorthand, not new re-exports):

```rust
use std::ops::RangeInclusive;
use std::collections::BTreeSet;
use proptest::strategy::BoxedStrategy;
use slicer_ir::*;

pub fn finite_f32(range: RangeInclusive<f32>) -> Result<BoxedStrategy<f32>, DomainError>;
pub fn finite_f64(range: RangeInclusive<f64>) -> Result<BoxedStrategy<f64>, DomainError>;
pub fn nonfinite_f32() -> BoxedStrategy<f32>;
pub fn nonfinite_f64() -> BoxedStrategy<f64>;
pub fn rounding_boundary_mm(max_abs_units: i64) -> Result<BoxedStrategy<f32>, DomainError>;
pub enum ScalarConfigDomain {
    Bool,
    Int { min: i64, max: i64 },
    Float { min: f64, max: f64 },
    Percent { min: f64, max: f64 },
    FloatOrPercent { min: f64, max: f64, is_percent: bool },
    StringChoices(Vec<String>),
}
pub fn scalar_config_values(domain: ScalarConfigDomain)
    -> Result<BoxedStrategy<ConfigValue>, DomainError>;
pub enum IdTextKind { UuidSyntax, OtherText }
pub fn id_text(kind: IdTextKind, max_bytes: usize)
    -> Result<BoxedStrategy<String>, DomainError>;
pub struct SmallGraph {
    pub node_ids: Vec<u16>,
    pub edges: Vec<(u16, u16)>,
}
pub enum GraphKind { Dag, CycleWithTail }
pub enum MalformedGraphKind { DuplicateNode, UnknownEndpoint }
pub fn graphs(kind: GraphKind, max_nodes: u16)
    -> Result<BoxedStrategy<SmallGraph>, DomainError>;
pub fn malformed_graphs(kind: MalformedGraphKind, max_nodes: u16)
    -> Result<BoxedStrategy<SmallGraph>, DomainError>;
```

Finite strategies mix bounded interior values and reachable IEEE witnesses (both zeros, subnormals and neighboring boundary floats when inside range). Nonfinite families contain infinities and multiple NaN payload/sign encodings. An out-of-range domain is assembled as an explicit union of finite ranges outside an owner's accepted interval plus nonfinite values, not silently fed to valid-domain properties. `rounding_boundary_mm` emits representable f32 neighbors of half-unit conversion boundaries within a checked bound; it does not promise universal exact conversion. `UuidSyntax` means lowercase ASCII `8-4-4-4-12` hex grammar, not object uniqueness, version semantics or a validator the string aliases do not have; `OtherText` is its complement within `max_bytes`, including deterministic empty/non-ASCII witnesses. Reject too-small UUID bounds instead of an empty strategy.

Valid graph IDs are unique; all endpoints are members; DAGs are constructed from ordered nodes and forward edges. `CycleWithTail` requires enough nodes for a real cycle and at least one acyclic node downstream, preserving both witnesses during shrinking. Malformed graph strategies preserve their named duplicate/unknown-endpoint witness and are used only at APIs that accept raw graphs and promise validation. They are not inputs to the already-validated `topological_sort` contract.

IR/permutation exports in `domains`:

```rust
pub enum AnchorDomain { Valid, Dangling }
pub struct AnchorCase { pub layer: slicer_ir::LayerCollectionIR }
pub fn anchor_cases(domain: AnchorDomain, max_entities: usize, max_travels: usize)
    -> Result<BoxedStrategy<AnchorCase>, DomainError>;
pub struct EntityDomain {
    pub roles: Vec<slicer_ir::ExtrusionRole>,
    pub max_entities: usize,
    pub max_points: usize,
    pub max_layers: usize,
}
pub fn entity_layers(domain: EntityDomain)
    -> Result<BoxedStrategy<Vec<slicer_ir::LayerCollectionIR>>, DomainError>;
pub fn permutations(len: usize) -> BoxedStrategy<Vec<usize>>;
pub fn locked_orders(len: usize, groups: Vec<Vec<usize>>)
    -> Result<BoxedStrategy<Vec<usize>>, DomainError>;
```

Anchor cases allow empty valid layers, repeated anchors and explicit `0`/`u64::MAX` membership witnesses; `Dangling` always has a missing reference, retaining it during shrinking. These are membership fixtures, not complete printing-validity certification. `entity_layers` has nonempty role choices, bounded paths with finite dyadic mm coordinates, positive widths/flow/speed factors, fresh per-layer IDs, matching entity/path roles and consistent layer indices with strictly increasing finite Z. Use `print_entity_base` and `extrusion_path3d_base` (`crates/slicer-sdk/src/test_support/fixtures.rs`), FRU, and the existing ID generator inside case construction; no production-wide defaults change. Empty layer/entity sequences are explicitly permitted when size bounds allow them. Cross-field role/config fixtures are built by owner packets from the supplied scalar domains and documented compatible bases, not by a blanket whole-config generator here. `groups` are disjoint nonempty authored index sequences within `0..len`; outputs are complete index permutations with each group contiguous and forward. Invalid group requests return `InvalidInput` rather than producing a partial permutation.

Integer geometry exports in `domains`:

```rust
pub enum TopologyClass {
    Empty, Convex, Concave, NonStarShaped, Holed, Disconnected, Thin, NearDegenerate,
}
pub enum BoundaryRule { Strict, Inclusive }
pub struct GeometryBounds {
    pub max_abs_units: i64,
    pub max_vertices: usize,
    pub max_holes: usize,
    pub max_components: usize,
    pub max_shrink_candidates: u32,
}
pub struct GeometryDomain {
    pub class: TopologyClass,
    pub min_holes: usize,
    pub min_components: usize,
    pub boundary: BoundaryRule,
    pub bounds: GeometryBounds,
}
pub struct GeometryCase { pub polygons: Vec<slicer_ir::ExPolygon> }
pub struct GeometryStrategy { /* private fields */ }
pub struct GeometryValueTree { /* private fields */ }
// GeometryStrategy: Strategy<Value = GeometryCase, Tree = GeometryValueTree>.
// GeometryValueTree: ValueTree<Value = GeometryCase>, implementing the actual
// current(&self), simplify(&mut self), complicate(&mut self) protocol.
pub fn geometry_cases(domain: GeometryDomain) -> Result<GeometryStrategy, DomainError>;
pub fn line_segments(bounds: GeometryBounds)
    -> Result<BoxedStrategy<[slicer_ir::Point2; 2]>, DomainError>;
pub type GeometryComplexity = (usize, usize, usize, u128);
pub fn geometry_complexity(case: &GeometryCase) -> Result<GeometryComplexity, DomainError>;
pub enum InvalidGeometryKind { Crossing, DuplicateVertex, Collapsed, TouchingHole, MisnestedHole }
pub struct InvalidGeometryCase {
    pub polygons: Vec<slicer_ir::ExPolygon>,
    pub kind: InvalidGeometryKind,
}
pub fn invalid_geometry_cases(kind: InvalidGeometryKind, bounds: GeometryBounds)
    -> Result<BoxedStrategy<InvalidGeometryCase>, DomainError>;
```

`GeometryBounds` and `GeometryDomain` provide documented safe `Default` implementations to support FRU, but live property profiles must supply their measured bounds explicitly. Reject inconsistent bounds/minima before sampling. `max_vertices` bounds the total across all rings, `min_holes` the total across components, and `max_components` all outers. Nonempty classes require `min_components >= 1`; `Holed` requires at least one hole, `Disconnected` at least two components, `Empty` requires zero minima. `boundary` controls containment-query expectations, not permission for rings/components/holes to touch: valid domain rings never self-touch; holes are strictly internal and mutually disjoint; disconnected outers have disjoint closures. Collinear consecutive edges may occur when nonzero-length, nonoverlapping and consistent with simplicity; no repeated closing endpoint is stored.

Reachable deterministic fixtures plus parametrized families must cover: convex rings; genuine concavity; a non-star-shaped orthogonal notch/U-shaped witness with an independently justified empty visibility kernel; multiple holes; multiple disjoint components; narrow corridors/necks, short edges and small gaps; one-unit clearances/almost-collinear turns. This is a bounded catalog plus varying dimensions/translations/optional vertices/components, not a promise of uniform or unrestricted simple polygons. Deliberate touching/crossing/duplicate/collapsed/misnested records live in separate invalid strategies. Quantization collapse is an explicit negative oracle fixture; do not call a collapsed post-conversion polygon valid because its pre-conversion coordinates were separated.

Mesh/transform exports in `domains`:

```rust
pub enum SolidKind { Box, TriangularPrism }
pub struct MeshCase {
    pub mesh: slicer_ir::IndexedTriangleSet,
    pub expected_bounds: slicer_ir::BoundingBox3,
    pub expected_volume_mm3: f64,
}
pub fn analytical_meshes(kind: SolidKind, max_extent_mm: f32)
    -> Result<BoxedStrategy<MeshCase>, DomainError>;
pub struct RigidTransformCase {
    pub transform: slicer_ir::Transform3d,
    pub translation_mm: [f64; 3],
    pub quarter_turns_z: u8,
}
pub fn rigid_transforms(max_translation_mm: f32)
    -> Result<BoxedStrategy<RigidTransformCase>, DomainError>;
pub enum InvalidMeshKind { BadIndex, TruncatedTriangle, NonFiniteVertex, DegenerateTriangle }
pub fn invalid_meshes(kind: InvalidMeshKind)
    -> BoxedStrategy<slicer_ir::IndexedTriangleSet>;
```

Analytical solids are small, closed, consistently outward-oriented indexed surfaces with positive bounded dyadic dimensions. Their bounds/volume derive from dimensions (`xyz` or triangle-base-area times prism depth), not a production mesh routine or the triangle-sum oracle. Transform families are rigid Z quarter-turns plus bounded translation, enabling a sign/swap coordinate oracle independently of matrix multiplication; arbitrary-rotation invariance is not claimed. Derive finite-intermediate and f32/f64 error limits for the selected bounds. Invalid mesh shrinkers retain the designated malformed record; validating APIs decide their product error contract later.

Independent exports in `oracles` (unqualified types are from `domains`, and `Point2`/`Polygon`/`ExPolygon`/`IndexedTriangleSet` are existing `slicer_ir` types):

```rust
pub fn graph_reachability(graph: &SmallGraph)
    -> Result<std::collections::BTreeSet<(u16, u16)>, DomainError>;
pub fn graph_cycle_remainder(graph: &SmallGraph)
    -> Result<std::collections::BTreeSet<u16>, DomainError>;
pub fn first_missing_anchor(layer: &slicer_ir::LayerCollectionIR) -> Option<u64>;
pub fn is_permutation(order: &[usize], len: usize) -> bool;
pub fn locked_groups_are_contiguous(order: &[usize], groups: &[Vec<usize>]) -> bool;
pub enum SegmentRelation { Disjoint, ProperCrossing, EndpointTouch, CollinearOverlap }
pub enum PointLocation { Outside, Boundary, Inside }
pub fn orientation(a: Point2, b: Point2, c: Point2) -> Result<i8, DomainError>;
pub fn signed_area2(ring: &Polygon) -> Result<i128, DomainError>;
pub fn segment_relation(a: [Point2; 2], b: [Point2; 2])
    -> Result<SegmentRelation, DomainError>;
pub fn validate_ring(ring: &Polygon) -> Result<(), DomainError>;
pub fn point_in_region(point: Point2, polygons: &[ExPolygon])
    -> Result<PointLocation, DomainError>;
pub fn segment_in_region(segment: [Point2; 2], polygons: &[ExPolygon], boundary: BoundaryRule)
    -> Result<bool, DomainError>;
pub fn validate_geometry(case: &GeometryCase, domain: &GeometryDomain)
    -> Result<(), DomainError>;
pub fn validate_indexed_mesh(mesh: &IndexedTriangleSet, require_closed: bool)
    -> Result<(), DomainError>;
pub fn mesh_signed_volume_mm3(mesh: &IndexedTriangleSet) -> Result<f64, DomainError>;
```

`graph_reachability` includes pairs connected by a **nonempty** path, so `(v,v)` means a cycle, not reflexivity. `graph_cycle_remainder` is every cycle node plus all nodes reachable from one, derived from that path model. Both reject malformed raw graphs. The anchor oracle uses ordered sets/sorted vectors, not `validate_travel_anchors`. The lock predicate requires authored member order as well as contiguity; permutation completeness is a separate check.

`orientation` returns exactly `-1`, `0`, or `1`. `signed_area2` is signed canonical-unit squared double-area; use checked widened arithmetic and return `ArithmeticOverflow` rather than wrapping for inputs outside the proven bound. Prove each configured coordinate/vertex bound against all intermediate expressions. Full-segment containment partitions at **all** exact boundary intersection parameters and classifies endpoints and every open interval, handling tangencies, collinear overlap, zero-length queries, multiple holes and disconnected components. Inclusive permits outer or hole boundary points, Strict permits only region interior. A segment crossing a hole is false in both modes even if endpoints and its single midpoint are inside.

Do not force rational intersection arithmetic into unproven `i128` products. Optional property-only exact rational/big-integer dependencies are permitted for this oracle; verify compatible workspace-managed versions and keep production/guest graphs clean. Alternatively demonstrate checked bounds for every rational operation. No public upstream numeric type leaks from these functions. `validate_ring` checks distinct vertices/zero-length edges, area and all nonadjacent edge relations, allowing only the documented adjacent endpoint sharing. `validate_geometry` additionally checks winding, strict hole containment/disjointness and component relationships plus structural class/minimum witnesses. These are small-input test predicates, not production repair logic.

`validate_indexed_mesh` checks finite coordinates, triangle-stream divisibility, index range, distinct triangle vertices and nonzero triangle area; closed mode additionally requires each undirected edge exactly twice with opposite directions. It does not claim absence of arbitrary 3D self-intersections. Signed volume uses the mathematical oriented tetrahedron sum with finite-intermediate checks, compared to independently dimension-derived solids with a derived mm³ error budget, not an arbitrary epsilon. No blanket mesh-repair-preserves-geometry assertion is introduced.

Concrete-regression exports in `regression`:

```rust
pub enum ConcreteInput {
    F32Bits(u32),
    F64Bits(u64),
    Graph(super::domains::SmallGraph),
    Layer(slicer_ir::LayerCollectionIR),
    Geometry(super::domains::GeometryCase),
    Mesh(slicer_ir::IndexedTriangleSet),
}
pub fn format_regression_fixture(name: &str, root_cause: &str, input: &ConcreteInput)
    -> Result<String, super::domains::DomainError>;
```

Formatting returns an escaped Rust **fixture statement block**, binding `input`, preceded by the supplied case name/root-cause comment. Emitted `input` types are `f32`, `f64`, `(Vec<u16>, Vec<(u16, u16)>)` for Graph, `LayerCollectionIR` for Layer, `Vec<ExPolygon>` for Geometry, and `IndexedTriangleSet` for Mesh. Do not emit references to feature-gated `SmallGraph`, `GeometryCase` or `ConcreteInput` itself. It is not a generated passing `#[test]`: the owner must add an independently justified assertion against the real SUT. Preserve float bit patterns via `from_bits`, integer IDs, ring winding/topology and index order; use fixture bases/FRU for watched IR structs. Reject an empty name/root cause; sanitize comment/newline/identifier/string content without executing it. A formatter probe compiles and evaluates representative emitted statements and compares concrete data/float bits; a string snapshot alone is insufficient. The concrete fixture must run without Proptest RNG, SDK `PropertyRun` context or an evaluator. Existing SDK fixture bases may be referenced by the fixture and enabled with `test` only.

### Shrinking protocol and measurement

Complexity is the lexicographic tuple `(component_count, hole_count, total_vertex_count, sum_abs_xy_units)`, computed with checked arithmetic. A valid accepted simplification must become strictly smaller in that ordering when the geometric value changes; do not claim global minimality. Try bounded component/hole/vertex deletion, parameter reduction and translation simplification only when the domain permits them. Geometry `current()` after **every** successful or unsuccessful `simplify`/`complicate` call must be valid and retain the required topology/minimum witnesses. A class such as NonStarShaped/Holed/Disconnected must not silently shrink to a convex/hole-free/connected case. `complicate` may restore larger values; repeated complicate returns to the pre-simplification value according to Proptest's real protocol. Revalidate candidates, bound candidate retries, and explicitly report exhausted limits.

The custom `GeometryValueTree` serves these bounded families only; it is not the conditional full general-polygon implementation. Use construction-first models rather than making a broad `prop_filter` the only generator. Graph/anchor/invalid-domain strategies also retain the relevant witnesses during shrinking. When candidate retries hit `max_shrink_candidates`, record the domain diagnostic `candidate_limit_reached` through private helper telemetry, retain the best known failing input, and report incomplete shrinking; returning `false` alone must not be described as proven minimality. Initial generation exhaustion reports an exhausted harness outcome. Controlled failure probes demonstrate fewer vertices and, where permitted, fewer holes/components, and demonstrate retention when minima prohibit removal. Start probes with nonminimal known witnesses; fail if the probe population is empty or no structural reduction occurs. A trivial initially minimal fixture cannot prove shrinking useful.

Add **new** SDK targets `property_domains` and `property_domain_contracts`, both with `required-features = ["property_testing"]` and actual files under `crates/slicer-sdk/tests/`. `property_domains` contains registry-backed generative helper contracts; `property_domain_contracts` contains deterministic counterexamples, compile/evaluate formatter probes and controlled known-failing shrink harnesses. All new test names in ACs below are planned, not existing coverage claims.

Append these records to the foundation registry, all package `slicer-sdk`, kind `test`, target `property_domains`, features `["slicer-sdk/property_testing"]`, `requires_scad = false`, `requires_host_algos = false`, `generator_version = 1`, and both measured tiers:

| ID | Exact test name | Required `bounds` keys |
|---|---|---|
| `D-SCALARS` | `proptest_direct_scalar_domains` | `max_scalar_units`, `max_id_bytes` |
| `D-GRAPHS` | `proptest_direct_graph_domains` | `max_nodes` |
| `D-IR` | `proptest_direct_ir_domains` | `max_entities`, `max_travels`, `max_points`, `max_layers` |
| `D-GEOMETRY` | `proptest_direct_geometry_domains` | `max_abs_units`, `max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates` |
| `D-MESH` | `proptest_direct_mesh_domains` | `max_extent_mm`, `max_translation_mm` |

Each record's `production_path` is the **new** `crates/slicer-sdk/src/test_support/property_testing/domains/mod.rs`, where the public entry functions are defined (private helpers may be split into subfiles). These are helper-validation properties, not credits toward host/module SUT coverage. Exact `production_symbol` and `charter` values:

| ID | `production_symbol` | `charter` |
|---|---|---|
| `D-SCALARS` | `slicer_sdk::test_support::property_testing::domains::finite_f32` | `docs/testing/properties.md#d-scalars` |
| `D-GRAPHS` | `slicer_sdk::test_support::property_testing::domains::graphs` | `docs/testing/properties.md#d-graphs` |
| `D-IR` | `slicer_sdk::test_support::property_testing::domains::anchor_cases` | `docs/testing/properties.md#d-ir` |
| `D-GEOMETRY` | `slicer_sdk::test_support::property_testing::domains::geometry_cases` | `docs/testing/properties.md#d-geometry` |
| `D-MESH` | `slicer_sdk::test_support::property_testing::domains::analytical_meshes` | `docs/testing/properties.md#d-mesh` |

Bounds remain u64 in the registry; checked conversions feed usize/u16/i64/f32 as appropriate. Mesh mm bound keys are whole millimeters; dyadic subdimensions come from the strategy. Never cast an excessive bound to a smaller integer/float silently.

Create **new** `docs/testing/proptest-domain-evidence.md`: record exact source/dirty state, feature/profile/toolchain/Windows context, sampled topology witnesses, generation attempts/rejections, callback counts, candidate retries, actual shrink traces/complexity, compile and execution measurements separately and selected profile values with reasons. Every direct family has deterministic rare witnesses in addition to fresh routine/extended exploration. Run the entire direct-domain routine/extended selection plus `H-IR-ANCHORS` after adopting shared anchor helpers. The existing foundation Windows routine job automatically picks up the new registry records; closure verifies this actual execution, not just registry editing.

The direct G-GEOMETRY disposition records which obligations this packet proves, and that scripted operations/cavities/bridges/engine conversion plus the **combined** generality decision remain with `pbt-scripted`. If measured direct families have a bounded direct-scope gap, fill it here. If a broad general generator is demonstrably necessary, record it as blocking fix work for the affected requirement; do not restore an unconditional general-shrinker prerequisite or declare SCAD complete.

## Acceptance criteria

`C`, `P` and logging rules are defined in Verification. Every assertion names a falsifiable domain or independent reference, not the generator's tag alone.

- **AC-1** `domain_scalar_edges_and_config_types` under `C`, plus `D-SCALARS` under `P`, exercise both zero signs, reachable subnormals, rounding-boundary neighbors, finite range ends, infinities and distinct NaN payloads, requested `ConfigValue` variants and UUID-syntax/complement witnesses. Finite helpers never expose nonfinite/out-of-range values during generation or shrinking; scalar-config validity is not misreported as whole-config validity.
- **AC-2** `domain_graph_models_and_cycle_tail` under `C`, plus `D-GRAPHS`, show unique endpoints/DAG precedence by an independent path model; a concrete cycle-plus-tail includes the acyclic tail in `graph_cycle_remainder`; malformed duplicate/unknown-endpoint cases stay separate and retain their witness through shrinks. The graph oracle never calls `topological_sort` or clones its Kahn implementation.
- **AC-3** `domain_ir_anchors_permutations_and_layers` under `C`, plus `D-IR` and `H-IR-ANCHORS`, prove anchor mode preservation against `first_missing_anchor`, full index bijections and forward-contiguous disjoint lock groups, matching role/path metadata, fresh per-layer IDs and finite ordered layer Z. The owning IR property still calls `validate_travel_anchors`; helper tests alone do not replace it.
- **AC-4** `domain_exact_predicates_counterexamples` under `C` pins orientation/area/segment classifications, adjacent-edge exceptions, crossing/repeated/collapsed rings, hole containment/winding/disjointness and disconnected components to independently worked integer cases. A concave segment with inside endpoints/midpoint but an outside interval, and a separate hole-crossing segment with inside endpoints/midpoint, are both rejected by `segment_in_region`; boundary-inclusive versus strict, tangency/collinear overlap and zero-length queries differ exactly as documented. A float-separated ring that collapses after canonical quantization is not certified valid.
- **AC-5** `domain_geometry_topology_witnesses` under `C`, plus `D-GEOMETRY`, constructs and independently validates every nonempty `TopologyClass`, multiple holes/components, an empty class, non-star-shaped witness, thin corridor/neck, short edge/gap and one-unit near-degenerate cases. At least one genuinely non-star-shaped witness is mathematically justified; a radial generator/tag is not accepted as evidence.
- **AC-6** `domain_shrink_protocol_preserves_witnesses` and `domain_known_failures_shrink_structurally` under `C` drive `current`/`simplify`/repeated `complicate` including false-return states. Every exposed valid value is valid; known failing rings/holed/disconnected cases strictly reduce structural complexity, removable holes/components disappear, required ones remain, and targeted invalid cases keep their named fault. No initially empty/minimal probe or unchecked shrink candidate can satisfy the gate.
- **AC-7** `domain_analytical_mesh_oracles` under `C`, plus `D-MESH`, compares closed edge incidence/orientation, finite/index validity, dimension-derived bounds/volume of boxes/prisms and independent sign/swap transform observations. Bad indices, truncated triangle stream, nonfinite vertices and degenerate triangles are rejected by the helper oracle with the promised classification. Derived finite-intermediate and mm³ tolerance policies are recorded, not tuned to random output.
- **AC-8** `domain_regression_fixture_compiles_and_preserves_bits` under `C` compiles/evaluates emitted scalar, graph, layer, geometry and mesh statements, preserving all float bits/IDs/topology/indices; generated fixture use needs no Proptest RNG/context/SCAD engine. Name/root cause and previously failing input are readable. The probe adds a genuine observed assertion; formatter-only output is never counted as a concrete SUT regression.
- **AC-9** `docs/testing/proptest-domain-evidence.md` records measured Windows routine/extended pilots for all five `D-` IDs, generation/rejection/shrink behavior, actual class witnesses, selected limits/bounds and separate compile/execution context. `cargo xtask test --summary --properties --tier routine` and its extended form execute all five plus `H-IR-ANCHORS`; the Windows PR job includes the delivered family without a new hand-maintained roster.
- **AC-N1** `domain_rejects_invalid_bounds_and_witness_loss` under `C` rejects nonfinite/reversed scalar ranges, too-small graph/UUID domains, excessive/narrowing bounds, impossible hole/component/vertex minima and overlapping/out-of-range lock groups. Injected overflowing arithmetic yields `DomainError::ArithmeticOverflow`, never wrapped valid geometry; a candidate losing the required hole/component/non-star/cycle/missing-anchor witness is not exposed as a valid shrink.
- **AC-N2** `domain_exhaustion_is_not_success` under `C` uses controlled rejection/candidate exhaustion through the real foundation harness; outcome is exhausted/failed with the configured limit and retained input, not passed or globally minimized. Existing foundation zero-execution/feature-composition/status tests still pass, and normal production/guest graphs remain Proptest/engine-free.

## Verification

Implementation, not authoring, performs these executions. Every plain `cargo test` below runs with Bash `set -o pipefail`, an existing `target/`, and suffix `2>&1 | tee target/test-output.log`; read/archive this file before overwriting it. On PowerShell preserve the native child status across `Tee-Object`. Registry commands reuse xtask's single combined-log/freshness policy.

- `C`: `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts -- --nocapture` with that wrapper.
- `P`: `cargo xtask test --summary --properties --property D-SCALARS --property D-GRAPHS --property D-IR --property D-GEOMETRY --property D-MESH --tier routine`; repeat with `--tier extended`, archiving each log/run first.
- `cargo xtask test --summary --properties --property H-IR-ANCHORS --tier routine`; repeat explicit-seed reproduction using the retained run seed. Run `cargo xtask test --summary --properties --tier routine` and `--tier extended` as the combined incremental gate.
- Foundation regression tests: `cargo test -p slicer-sdk --features property_testing --test property_harness -- --nocapture` and `cargo test -p xtask --bin xtask -- property_ --nocapture`, each with the wrapper. Existing `cargo test -p slicer-ir --test ir_validation_tdd` uses the same capture rule.
- `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo check -p slicer-sdk --all-targets --features property_testing`; `cargo clippy -p slicer-sdk --all-targets --features property_testing -- -D warnings`; `cargo check -p slicer-ir --all-targets --features property_testing`.
- `cargo xtask check-literals`; `cargo xtask check-test-quality --report`; `cargo xtask build-guests --check` (handle `1`/`3` per AGENTS.md). No workspace test run is required. Oracles do not need new `slicer-core` tests, and this packet must not use a bare core test run as evidence for gated host algorithms.

## Docs impact

- `docs/05_module_sdk.md`, `## Test Support (slicer-sdk feature)` / `### IR Fixture Builders`: add direct-domain imports/ownership, exact validity/shrink obligations, kernel-test placement and default-off behavior. Check: `rg -n 'GeometryDomain|anchor_cases|property_testing' docs/05_module_sdk.md`.
- `docs/21_data_defaults_and_fixtures.md`, `## 5. Fixture policy`: document strategy bases/FRU and compiled concrete fixture formatting with root-cause assertions, not committed seed histories. Check: `rg -n 'format_regression_fixture|concrete regression' docs/21_data_defaults_and_fixtures.md`.
- `docs/testing/properties.md` (**planned prerequisite document**, not existing at authoring): add all five complete `D-` charters, oracle independence dependencies, bounded topology catalog, exact arithmetic/tolerance derivations and direct-to-scripted disposition. Check: `rg -n '^## D-|NonStarShaped|ArithmeticOverflow|G-GEOMETRY' docs/testing/properties.md`.
- **New** `docs/testing/proptest-domain-evidence.md`: actual pilot/source/profile/shrink-trace evidence and unresolved combined-scripted obligations. Check: `rg -n 'Windows|G-GEOMETRY|source_revision|shrink' docs/testing/proptest-domain-evidence.md`.
- `docs/08_coordinate_system.md` needs no normative conversion change: consume its current `Conversion & Determinism (Normative)` contract and put test-domain-specific error proofs in the contributor charters. No IR/WIT/schema or scheduler contract changes.

## Open questions

- [FWD] Measure the direct-family pilots on Windows, choose routine/extended counts, size bounds, rejection/candidate/shrink limits, and safe documented helper defaults. Derive arithmetic/float error policies before marking the profiles accepted; no runtime budgets or rejection percentages are prescribed during authoring.
- [FWD] Choose checked bounded rational arithmetic or optional exact rational/big-integer dependencies for full-segment partitioning after proving all intermediate ranges. Any added numeric dependencies stay under SDK `property_testing`; this is not permission to use production Clipper/geo predicates as the oracle.
- [FWD] Record the direct-only G-GEOMETRY coverage/disposition and precise remaining hybrid gaps for `pbt-scripted`. A full custom general-polygon strategy remains conditional on that combined feasibility result, not silently mandatory here.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | Existing IR/config/fixture/validator/topology shapes inspected at crate-qualified paths; all new domains/oracles/targets are explicitly planned. |
| 2 | WIT/IR identifiers | PASS | `Point2`, `Polygon`, `ExPolygon`, flat `IndexedTriangleSet.indices`, `Transform3d.matrix`, actual `ConfigValue`/role variants and entity anchor fields confirmed; no WIT edit. |
| 3 | Schema versions | PASS | No production IR/WIT version added or bumped; foundation test-protocol schema unchanged. |
| 4 | ADRs | PASS | Fixture-home/whole-module gating and ADR-0062 lock direction/contiguity preserved; no new ADR slot allocated. |
| 5 | Deviation IDs | PASS | No deviation ID allocated, closed or relied on. |
| 6 | Prerequisites | PASS | Foundation is explicitly draft, not falsely satisfied; its planned module, methods, feature, registry and context shapes agree exactly. |
| 7 | Test targets | PASS | New SDK standalone targets require `property_testing`; existing baseline target verified; planned prerequisite targets called out; no unregistered aggregated module. |
| 8 | Criteria | PASS | Concrete counterexamples, shrink transitions, measured profiles and negative controls; no workspace criterion command. |
| 9 | Fallout | PASS | No existing struct field/schema change. New helper structs have no old literal sites; GeometryBounds/GeometryDomain default/FRU requirements and formatter watched-type fallouts are explicit. |
| 10 | Docs impact | PASS | Actual canonical headings and planned prerequisite/new docs identified with checks. |
| 11 | Completeness | PASS | No blocking scope question or placeholder; measurement, numeric implementation and combined-scripted decisions have bounded forward ownership. |

**Verdict: PREFLIGHT PASS (authoring only; direct G-GEOMETRY/pilots not executed; implementation waits for foundation).**
