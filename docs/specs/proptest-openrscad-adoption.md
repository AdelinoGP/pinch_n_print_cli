# Proptest Adoption and Optional OpenRSCAD Ingestion — Detailed Design and Packet-Generation Brief

Status: **design approved by the user; implementation and feasibility gates not executed**.

Intended repository destination: `docs/specs/proptest-openrscad-adoption.md`.

- Original authoring copy: `C:\Users\agpen\.opencode\plan\proptest-openrscad-adoption.md` (2026-09-09). This repository copy is now the durable source; the plan copy can be discarded.

This is the input specification for packet generation, not an active spec packet or an implementation claim.

Authoring context: user-approved grilling session `ses_f7bc84401ffedWDR5wJB2XsCCZ`, 2026-09-09.

## 1. Result and purpose

Introduce meaningful property-based testing with Rust Proptest across the host and important native module kernels. Build reusable, validity-aware input strategies, independent correctness oracles, useful shrinking, and reproducible failure handling. Wire the properties into required Windows checks and manual/completion exploration.

Use a hybrid geometry strategy: direct integer-grid inputs for precise slicer cases, and embedded OpenRSCAD for rich scripted 2D/3D inputs. Reuse the OpenRSCAD adapter to offer an initially opt-in user-facing `.scad` input feature.

The next session should generate a dependency-ordered packet queue from this document. It must preserve the decisions below rather than reopening the interview. Source inspection, baseline execution, numeric-domain derivations, and feasibility measurements remain implementation obligations; they are not unanswered user-preference questions.

### Success means

- Properties assert independently justified behavior of production code, rather than restating its implementation or merely asserting that random inputs do not panic.
- Each adopted property has an explicit input domain, oracle, tolerance policy, shrinking contract, execution profile, and reproducible command.
- The host and all selected module families have a reviewed coverage matrix; delegated logic is credited to its actual owner.
- Feature-gated tests demonstrably execute. A successful process that ran no selected properties is a failed gate.
- Confirmed failures become fixes and named concrete regression tests.
- The optional SCAD importer and scripted generators share one evaluation/diagnostics/conversion boundary.
- Windows-native feasibility and execution are demonstrated before SCAD adoption is declared complete.

## 2. Locked decisions

These decisions were explicitly approved. Packet generation may refine implementation details consistently with them, but must return to the user for a material scope or architectural change.

| Topic | Approved decision |
|---|---|
| Initial scope | Broad adoption, rather than a small demonstration or infrastructure-only exercise. |
| Host scope | IR/configuration/coordinates, scheduling and claims, runtime bookkeeping, geometry/mesh algorithms, model I/O, and G-code. |
| Module scope | Walls/Arachne; infill/bridging/surfaces; traditional/tree supports; layer planning/seams; toolpath/finishing. |
| Module depth | **Kernels first.** Existing example tests cover entrypoint wiring; no mandatory new randomized entrypoint test quota. |
| Defects | Include focused fixes. Larger defects become explicit blocking fix work for affected coverage. |
| Input domains | Valid-input correctness and invalid-input behavior at APIs that promise validation/rejection. |
| Oracles | Independent mix: contracts, mathematics, metamorphic relations, small reference models, traceable canonical formulas/fixtures. No new live OrcaSlicer differential harness is required. |
| Sampling | Fresh exploration in both routine and extended runs; explicit replay supported. |
| Committed failures | Named, minimized **concrete** regressions. Proptest seeds are diagnostic artifacts, not committed regression history. |
| Execution | Routine required PR checks; extended runs manually and at completion gates. No scheduled/nightly rollout requirement. |
| Runtime budgets | Measure baseline and pilots, then set case counts, bounds, and shrink limits. No invented runtime budget. |
| SDK support | Dedicated default-off property-testing feature, alongside the existing SDK test-support home. |
| Geometry approach | Hybrid OpenRSCAD-backed generation plus direct integer-grid strategies. |
| Geometry breadth | Convex/concave simple rings, multiple valid holes, disconnected polygon sets, thin and near-degenerate cases. |
| General shrinker | Demonstrate whether scripted shrinking meets the general-geometry requirement. A full custom polygon shrinker is conditional on the feasibility result. |
| Shrink topology | Remove vertices, operations, holes, or components when the property domain permits it. Retain a required witness, such as a hole in a hole-specific property. |
| SCAD engine | **`matthova/openrscad`**, embedded as Rust libraries. This is not `timschmidt/openscad-rs`, which is parser-only. |
| Native dependencies | Accept CMake/C++ compiler prerequisites for OpenRSCAD's native dependency configuration. |
| SCAD ownership | Dedicated `slicer-scad` adapter crate, depending on neither the runtime nor SDK, with an optional engine backend. |
| SCAD distribution | Initially default-off/opt-in. |
| SCAD user commands | `slice` and `mesh import`. |
| SCAD source scope | Self-contained scripts plus parameter overrides. External-resource requests are rejected. |
| Parameters | SCAD expressions; keep these separate from slicer configuration overrides. |
| 2D source | Permit explicit positive extrusion height. No implicit thickness. |
| SCAD object semantics | One fused geometry object, possibly with disconnected components. Display colors do not assign printer materials. |
| Placement | Normal bed placement for slicing, preserving relative arrangement. |
| Diagnostics | Reject partial, approximate, degraded, or unclassified diagnostic outcomes. Allow explicitly recognized informational warnings. |
| Cache | Evaluation-local caching is allowed; no persistent geometry cache initially. |
| SCAD testing | Required bounded SCAD-enabled Windows checks, plus larger manual/completion exploration. |
| Platform acceptance | **Windows only for this rollout.** |
| Upstream changes | Prefer upstream contributions; focused, pinned, documented patches/forks are allowed when needed. |
| Failed SCAD feasibility | Continue core Proptest work with direct strategies. Keep unmet SCAD/scripted-generation obligations blocked. |
| Test UX | Extend `cargo xtask test`, reusing existing gate/logging/summary behavior. |

### Superseded decisions and scope boundaries

The earlier preference for a full custom general-polygon generator was revised after investigating OpenRSCAD. Do not silently restore that as an unconditional prerequisite; prove the hybrid's coverage and shrinking first.

The user selected host-wide plus native module internals. A broad new guest-boundary property campaign or generated full-pipeline E2E campaign is not required. The new SCAD product feature does require focused CLI/import/slice acceptance tests, using existing infrastructure and freshness gates wherever guests are involved.

Real community modules remain external pinned submodules under repository policy. The labeled dragon-curve example is not an additional shipped module target for this rollout.

## 3. Verified baseline and important corrections

### 3.1 Repository baseline

- Searches of manifests and Rust source found no current Proptest integration. Existing fixture-based invariants provide starting points, not a generative harness.
- The root `[workspace.package]` in `Cargo.toml` declares Rust `1.91.0`. Proptest's inspected `1.11.0` release declares Rust `1.85`; OpenRSCAD's inspected manifest also declares Rust `1.85`. These declared versions are compatible. A successful integrated build has **not** been demonstrated in this session.
- `slicer_sdk::test_support` is the designated shared-fixture home in `docs/21_data_defaults_and_fixtures.md`; `docs/05_module_sdk.md` describes its mock host, output captures, and native test ownership.
- `slicer-sdk` has an existing `test` feature and a self dev-dependency enabling it (`crates/slicer-sdk/Cargo.toml`). Its native dependency on `slicer-core` enables `host-algos`.
- Root module crates are workspace members. Their `wit-guest` wrappers are distinct guest workspaces; native module tests can exercise the real module algorithms.
- `topological_sort` (`crates/slicer-scheduler/src/topology.rs`) returns deterministic ordering or the remaining unsorted IDs. Those IDs can include acyclic nodes downstream of a cycle.
- `validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`) checks membership specifically against `LayerCollectionIR.ordered_entities`, not an arbitrary list of entities elsewhere in the IR.
- `test_command` (`xtask/src/test.rs`) already runs literal checks, guest freshness, CLI freshness, and Cargo tests with output retained in `target/test-output.log`.
- **Feature-composition hazard:** `test_command` adds `slicer-core/host-algos` only when the caller has not supplied feature arguments. Adding property/SCAD feature arguments must not accidentally disable that protection.
- `.github/workflows/ci.yml` exists. Its inspected jobs use Ubuntu, and `jobs.test` explicitly tests selected crates rather than every proposed property owner. Windows-only acceptance for this work therefore needs explicit new Windows checks.
- `load_model` and `assemble_object` (`crates/slicer-model-io/src/loader.rs`) own existing STL/OBJ/3MF ingestion. `SliceRunOptions::mesh` (`crates/slicer-runtime/src/run.rs`) is already a preloaded `Arc<MeshIR>`.
- `Cmd::Slice` (`crates/pnp-cli/src/main.rs`) currently accepts `--model`; `MeshCmd::Import` accepts `--input` and handles STEP/STP. Do not infer flag names from stale command examples.

### 3.2 Feature gates must be read from actual targets

`crates/slicer-core/src/lib.rs` gates `medial_axis`, `skeletal_trapezoidation`, and `voronoi` on `host-algos`. Its `polygon_ops`, `smooth_outward`, and `triangle_mesh_slicer` declarations are unconditional.

`crates/slicer-core/src/algos/mod.rs` individually gates `lightning`, `mesh_analysis`, `mesh_cross_section`, `overhang_annotation`, `paint_segmentation`, `prepass_slice`, `region_mapping`, and `support_geometry` on `host-algos`. Do not call every `algos` function ungated merely because the parent module declaration is unconditional.

Test targets also have `required-features` and source-level `cfg` gates. The authoritative execution plan must derive the necessary feature union from the selected targets. Do not freeze a test-binary count into this specification.

### 3.3 Claims this specification deliberately does not treat as facts

- The current pass/fail state of the Arachne invariant suite is unmeasured. A stale comment describing a `transition_dist = 4.0` failure is not current test evidence. `propagate_beadings_downward` now delegates through the corrected transition-distance path in `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`.
- Neither coordinate conversion is a universal exact round trip. Even exact representability of an integer as `f32` does not prove that division, multiplication, and rounding recover it exactly.
- Finite floating-point inputs alone do not guarantee finite arithmetic outputs; intermediate overflow matters.
- A segment midpoint inside an outline does not prove that the full segment stays inside a concave outline or avoids every hole.
- Not every Arachne output is necessarily a closed contour; contracts must distinguish closed walls and legitimate open paths.
- Positive offset followed by negative offset is not a universal identity, and geometric heuristics need not be invariant under arbitrary rotations.
- A hash/UUID construction cannot be specified as mathematically collision-free over an unrestricted input space. Test determinism, documented construction, and collision-handling behavior where applicable.
- A test comment, a self-captured baseline, or an upstream compatibility claim is not proof of canonical correctness.

## 4. Architecture and dependency boundaries

The names below are the **planned new surface**, not claims that these files/APIs already exist.

```text
production:
  pnp-cli [scad]
      -> slicer-model-io [scad]
          -> slicer-scad [openrscad]
              -> openrscad-syntax / ir / eval / geom
              -> slicer-ir (PnP geometry types and conversion contract)

tests:
  owning crate's tests
      -> slicer-sdk [property_testing]
          -> existing test_support + Proptest strategies/oracles/case guard
      -> optionally slicer-sdk [property_testing_scad]
          -> slicer-scad [openrscad]

  basic owning-crate properties may use Proptest directly as a dev-dependency
  when they do not need shared SDK fixtures or geometry strategies.
```

### 4.1 Proposed feature contract

| Package | Feature | Meaning |
|---|---|---|
| `slicer-sdk` | `property_testing` | Default-off; implies existing `test`; enables optional Proptest dependency and shared property helpers. |
| `slicer-sdk` | `property_testing_scad` | Default-off; implies `property_testing`; enables scripted strategies through the SCAD adapter. |
| `slicer-scad` | `openrscad` | Default-off; activates the engine dependencies and implementation. |
| `slicer-model-io` | `scad` | Default-off; enables the explicit SCAD ingestion entry. |
| `pnp-cli` | `scad` | Default-off; enables the importer and SCAD CLI options. |

Feature names may be adjusted during packet preflight only for a real collision or established naming constraint, with all documented commands updated together. The feature semantics are locked.

- A default workspace build must not compile OpenRSCAD merely because `slicer-scad` is a workspace member. Its engine dependencies themselves must be optional, not just its callers.
- A production CLI build, including a SCAD-enabled one, must not acquire Proptest through normal dependency edges.
- Ordinary property helpers must not activate OpenRSCAD. Existing SDK `test` consumers must not automatically acquire Proptest.
- A SCAD-enabled SDK test graph must not leak either engine or property helpers into production guest builds.
- Keep the adapter independent of SDK/runtime ownership. Avoid introducing a normal-dependency cycle to share fixtures.
- Use workspace-managed Proptest dependency versions. Start from the inspected `1.11` API line and commit the resulting lock resolution; verify the current package/MSRV again at dependency selection.
- Retain Proptest's fork/timeout capability for tests that need process isolation. Forking is a per-profile choice, not a requirement for every cheap property.
- Custom strategies are preferred over blanket `Arbitrary` derives on domain structs with semantic validity constraints. Reuse safe fixture bases and FRU per `docs/21_data_defaults_and_fixtures.md`.

### 4.2 SDK helper organization

Use `slicer_sdk::test_support` as the home, with a dedicated property-testing submodule and narrowly curated re-exports. Separate:

1. profile/configuration resolution;
2. per-case state guard;
3. scalar/config/ID/graph/IR strategies;
4. integer geometry strategies and validators;
5. independent oracles;
6. optional scripted geometry strategies;
7. reproducible failure formatting.

These are cohesive test helpers, not a second test framework. Actual properties stay beside their production code. Private and `pub(crate)` kernels are tested inside the owning crate; do not widen public APIs solely for an integration-test file.

## 5. Property charter and oracle standard

Every property added to the inventory must have:

| Field | Required content |
|---|---|
| Stable property ID | A semantic identifier independent of packet numbers. |
| Owner | Cargo package, test target, production symbol, workspace-relative source path. |
| Contract | A precise claim supported by a normative document, mathematical law, independently verified reference, or verified canonical behavior. |
| Domain | Validity requirements, units, bounds, semantic modes, meaningful empty/degenerate cases, and any excluded cases. |
| Strategy | Generator family, mandatory structural witnesses, size dimensions, and conversion boundaries. |
| Oracle | Independent reference calculation/model/predicate, or a justified metamorphic relation. Identify shared implementation dependencies. |
| Tolerance | Exact integer/set comparison when appropriate; otherwise a derived dimensionally correct error budget. |
| Shrinking | What may simplify, what must remain valid, and which witness must remain present. |
| Execution | Routine/extended eligibility, required features, optional SCAD dependency, isolation, and measured profile. |
| Failure record | Concrete input, reproduction command/configuration, diagnostics, and root-cause regression destination. |
| Acceptance | Target actually executed, oracle checked, generator/shrink validity demonstrated, and relevant existing tests passed. |

### 5.1 Independence requirements

- A copy of the production algorithm is not an independent reference model.
- The production function's own validator is not automatically an independent oracle. In particular, `validate_polygon_simplicity` (`crates/slicer-core/src/polygon_ops.rs`) uses Clipper self-union and must not be the sole validity oracle for Clipper properties.
- Independence is evaluated against the actual code path under test, not just Cargo package names. OpenRSCAD uses `geo` for 2D booleans and Manifold backends for 3D geometry. Identify overlap with the particular PnP algorithm before using either as differential evidence.
- A generated OpenRSCAD mesh is a stimulus. It does not by itself establish that the slicer's output is correct, nor that OpenRSCAD rendered the intended SCAD semantics.
- Canonical fixtures/formulas are allowed when provenance is real. Verify OrcaSlicer behavior by file and function name; preserve attribution requirements for any actual port.
- Do not weaken canonical behavior or adjust tolerance until a randomized test passes. A false property or bad generator is fixed as such; a real production defect is fixed in production.

### 5.2 Geometry oracle primitives

Build a bounded-domain, independently tested set of predicates suitable for small test inputs:

- exact orientation and segment intersection with explicitly bounded/widened arithmetic;
- signed area/orientation and duplicate/zero-length edge detection;
- ring simplicity from segment-pair relations, including adjacency and boundary-contact rules;
- strict containment versus boundary-inclusive containment;
- hole/outer and hole/hole disjointness and containment;
- polygon-set component relationships;
- full segment-in-region checking by partitioning at boundary intersections and classifying the resulting intervals;
- mesh index validity, finite coordinates, edge-incidence/orientation, and analytical bounds/volume for designated simple solids.

Test the predicates against deliberate counterexamples before using them to certify a generator. Use checked arithmetic or prove the declared coordinate bound fits intermediate arithmetic. `i128` does not justify unrestricted `i64` geometry automatically.

For booleans, use exact analytical rectangle/polygon cases and independent interior point-membership/set algebra away from boundaries, with topology/area checks where justified. Do not equate different contour start indices or output ordering with different geometric regions.

For path containment, test the entire segment or declared swept footprint; checking only vertices or a single midpoint is insufficient.

## 6. Generator and shrinker design

### 6.1 Direct strategies

Build bounded strategies for:

- finite scalars, signed zero, subnormals, rounding boundaries, validated config values, and separately non-finite/out-of-range values;
- canonical/noncanonical IDs and serialized values;
- small graphs with unique node IDs and valid endpoints, DAGs by construction, cycles with downstream tails, and distinct malformed graph domains where a validator accepts raw data;
- entity collections, travel anchors, permutations, locked groups, role/config combinations, and multi-layer sequences;
- integer-grid polygons and line segments, with named topology classes;
- small indexed meshes, analytical solids, transformations, and intentionally invalid mesh records at validation boundaries.

Keep explicit edge-case strategies in the mix. Random sampling alone is unlikely to hit a single exact sentinel, extreme, or rounding boundary reliably.

### 6.2 Combined geometry coverage obligations

| Class | Required witness and shrinking behavior |
|---|---|
| Convex ring | Valid nonzero-area ring; shrink vertices/dimensions within domain. |
| Concave ring | Include genuinely concave and non-star-shaped witnesses where the tested contract applies; a convex-only or radial-only population is insufficient. |
| Holes | Strictly contained, mutually disjoint holes with the winding expected by the target API. Hole-specific properties retain a required hole. |
| Disconnected sets | Multiple disjoint outer components; relationship-specific properties retain the relevant components. |
| Thin features | Controlled narrow corridors, necks, small gaps, short edges, and aspect ratios in the target's valid domain. |
| Near degeneracy | Quantization-scale separations, almost-collinear edges, proximity to contacts; classify validity after conversion. |
| Contact/invalid cases | Separate strategies for touching, crossing, duplicate, collapsed, self-intersecting, or misnested records, according to the validator's documented contract. |
| Layered/3D geometry | Extrusions, cavities, bridges/overhangs, separated bodies, and dimension-controlled cross-sections. |

The generator does not need a uniform distribution over all polygons. It must state its bounded domain and demonstrate reachable diversity. Do not infer generality from implementing Proptest's `Strategy` trait or from a large random case count.

### 6.3 Scripted strategy representation

Generate a bounded, typed model recipe rather than arbitrary strings as the primary valid-geometry strategy. A recipe records primitive/contour parameters, transforms, Boolean operations, and extrusions, and can emit a self-contained SCAD source file. Permit explicit polygon leaves for shapes not well represented by primitive CSG.

Reasons for a recipe layer:

- parameter/operation shrinking is controllable and reproducible;
- a failing case can be preserved as readable `.scad` plus explicit parameters;
- syntactic validity is separated from geometric validity;
- size, depth, facet count, coordinates, and topology witnesses can be bounded;
- the same recipe can be regenerated without a filesystem dependency.

Arbitrary program/parser fuzzing is not the primary deliverable. Invalid-script tests may generate bounded malformed source for the importer contract separately.

### 6.4 Shrinker obligations

- Every value exposed as a valid-domain case remains valid, including during `simplify`/`complicate` exploration.
- Candidate operations include deleting subtrees/components, simplifying Boolean trees, reducing contour vertices, shrinking dimensions/translation, and removing holes when permitted.
- Revalidate after each candidate evaluation and after PnP quantization. A valid AST is not sufficient.
- Define a deterministic complexity ordering, such as topology/operation count, vertex count, then coordinate/size magnitude. Do not claim global minimality.
- Bound candidate retries and shrinking. Exhausting rejection or shrink limits is reported distinctly and is not a passing correctness result.
- Preserve required witnesses. An invalid-input shrinker must preserve the targeted invalid condition when that is necessary to interpret the failure.
- Avoid rejection-heavy `prop_filter` as the sole general generator strategy. Prove acceptance and shrinking behavior with measurements; do not solve exhaustion merely by raising rejection limits.
- For synthetic known-failing properties, show that inputs become structurally smaller while the independent validity predicates continue to pass. Include holes and disconnected cases.

### 6.5 Conditional general-polygon work

The geometry feasibility checkpoint must produce a recorded decision:

1. the hybrid strategies reach the required topology classes and shrink useful failures: proceed;
2. a bounded gap can be filled with direct strategies: implement that gap;
3. broad gaps require a custom general polygon `Strategy`/`ValueTree`: author a blocking generator packet with the same independent-validation obligations.

Do not call a restricted radial/star-shaped family a general simple-polygon generator. Do not promise that arbitrary vertex deletion or coordinate shrinking preserves simplicity. Preserve Proptest's actual `ValueTree` interface semantics; a custom tree is not required to emulate a particular numeric binary-search implementation.

## 7. Per-case isolation and failure handling

### 7.1 SDK state

`generate_module_test_impl` (`crates/slicer-macros/src/lib.rs`) performs setup/teardown once per Rust test. Proptest runs multiple cases and shrink attempts inside a test.

Add a per-case RAII guard, using the existing public seams in `crates/slicer-sdk/src/test_support/mod.rs` and `mock_host.rs`:

- reset logs and mesh source;
- install a fresh case-specific mock/capture context when required;
- construct fresh output captures, fixtures, and entity-ID generators;
- tear down on normal return, assertion failure, and unwind;
- verify isolation across consecutive cases and after a deliberate panic.

Do not install/churn the process-global panic hook on every case. `install_panic_handler` wraps `std::panic::take_hook/set_hook`; repeated use is not case-local isolation. Capture diagnostic data explicitly and let the test runner report failures. Audit interaction with existing parallel module tests without redesigning unrelated test machinery.

### 7.2 Native evaluation limits

OpenRSCAD's evaluation budget covers evaluation work, not its synchronous geometry backend. A worker-thread timeout neither cancels nor joins an uninterruptible render. `catch_unwind` does not contain native stack-overflow aborts.

- Bound generated recipe complexity before evaluation/rendering.
- Use an evaluation budget through the adapter; add a focused upstream API extension if needed to combine budget, overrides, and exact mode.
- Establish an evaluator worker-stack policy from upstream's native entrypoint and verify it on Windows. Do not simply assume Rust test-thread defaults are sufficient.
- Where a hard per-case deadline or abort containment is required, use Proptest's process isolation or equivalent killable test-process supervision.
- Keep the product integration library-based. Do not advertise an enforceable in-process render deadline that the library cannot provide.
- Save the case before risky execution so an abort/timeout retains a reproduction input. If minimization did not finish, say so rather than calling the retained case minimal.

### 7.3 Regression lifecycle

1. Capture failure classification, selected property/profile/features, seed/configuration, source/recipe, and concrete geometry as needed.
2. Reproduce with the recorded invocation.
3. Determine whether the cause is the SUT, the generator/adapter, the oracle, or a contract misunderstanding.
4. For a confirmed SUT bug, retain a concrete regression that bypasses unnecessary generator/evaluator work when possible.
5. Name the root cause and previously failing input in the regression test/comment.
6. Apply the canonical-correct fix and run the property plus relevant existing tests.

Persist diagnostics under `target/` and upload them on CI failure. Configure Proptest persistence intentionally so ordinary runs do not create source-adjacent seed files that are expected to be committed. Local seed replay may be retained under `target/`; committed history consists of concrete regressions.

If an upstream engine bug is involved, retain both source/parameters and converted geometry where possible, so PnP correctness can be diagnosed independently of evaluator changes. Follow repository rules for recording large fixtures; do not load or commit large ad hoc JSON blobs as a shortcut.

## 8. SCAD adapter contract

### 8.1 Responsibilities and planned public shape

`crates/slicer-scad/` owns a small source-to-geometry API. Proposed conceptual types are `ScadSource`, `ScadEvalOptions`, `ScadGeometry`, `ScadDiagnostic`, and `ScadError`; packet preflight should specify their final Rust representation using repository default/fixture conventions.

Inputs:

- source text and a source label for diagnostics;
- ordered parameter overrides as names and expression source;
- exact evaluation/render policy;
- optional explicit 2D extrusion height in millimeters;
- evaluation/complexity limits;
- deterministic test settings where needed.

Outputs:

- validated PnP-compatible contours (`ExPolygon` sets) for direct 2D consumers, or an indexed triangle mesh for model ingestion;
- accepted informational diagnostics;
- sufficient engine/backend and source/parameter provenance for reproduction.

The adapter performs engine setup, parameter evaluation, no-external-resource resolution, diagnostic classification, exact rendering, and conversion validation. `slicer-model-io` retains ownership of `ObjectMesh`/`MeshIR` assembly, source-derived identity, and frontend placement policy.

Do not expose upstream implementation types throughout the runtime or module APIs. The adapter must not depend on Proptest. Recipe strategies and failure minimization live in test support.

### 8.2 Verified upstream API facts to account for

Source: the linked OpenRSCAD repository and its `COMPAT.md`, evaluated by source inspection only.

- `openrscad_syntax::parse` produces the program AST.
- `openrscad_eval::eval_program_with_mode` takes typed parameter values and evaluation mode.
- `eval_const_expr` evaluates expression source for parameter values.
- `eval_program_with_budget` is a separate public entrypoint. The shared `eval_program_impl` that combines overrides, fuel, and mode is private in the inspected source. Do not invent a public combined call.
- `FileResolver` is supplied by the host; a no-resource implementation can deny external loads.
- `render_cached_diag` is the exact geometry-render path; the preview path is a different API.
- `Mesh` exposes indexed vertices and triangles. `render_contours_with` provides 2D contours using an even-odd interpretation; conversion must reconstruct nesting/winding correctly.
- `RenderDiagnostics` has warnings/errors, not a trustworthy universal `is_exact` flag. Geometry warnings can themselves describe approximations or omitted geometry.
- Evaluation can return successful partial output for a root recursion abort, with a warning rather than a structured partial-result flag in the inspected API.
- Native `openrscad-geom` currently depends on `manifold-csg` unconditionally for non-WASM targets, even though a Rust backend is also present.
- The evaluator has thread-local random state and shared font facilities. Use evaluation-local state, explicit seeds for generated random programs, and bundled fonts for reproducible fixtures.
- Upstream's Windows release workflow demonstrates a Windows build path. This does not replace this project's own integration acceptance.

Pin the engine revision before writing adapter-specific compatibility assertions. If a required diagnostic channel or combined evaluation-options API is absent, implement a focused upstream extension/patch and test it; do not bypass the approved failure policy.

### 8.3 Diagnostic acceptance table

| Outcome | Product importer | Valid-geometry generator |
|---|---|---|
| Parse/evaluation error | Fail with source/parameter context. | Fail/classify generator input; no SUT pass. |
| External-resource request | Fail clearly. | Recipe domain violation; no hidden filesystem access. |
| Recursion-aborted partial result | Reject partial geometry. | Report adapter/generator failure. |
| Renderer fallback/degraded output | Reject, even if a mesh was returned. | Do not use as certified-valid input. |
| Known approximate or geometry-skipping warning | Reject. | Reject from exact valid domain. |
| Unknown warning/diagnostic | Reject until classified in the pinned adapter. | Surface a failure requiring review. |
| Recognized informational warning | Preserve/display and continue. | Retain in reproduction metadata. |
| Empty evaluated geometry | Clear empty-model failure for import/slice. | Allowed only in an explicitly empty-result domain. |
| Non-finite/out-of-range/bad-index converted geometry | Reject. | Report conversion/validity failure. |

Do not infer exactness from `diag.errors.is_empty()` alone. Do not equate all upstream warnings with harmless messages. In particular, upstream documents geometry omission for concave/holed `roof()` and approximation for some concave-leaf 3D Minkowski inputs.

### 8.4 Conversion requirements

- Geometry units are millimeters at the engine boundary. PnP 2D integer units are `10^-4 mm`; use the canonical conversion policy in `docs/08_coordinate_system.md`.
- Range-check finite values before casts; do not accept saturating casts as successful conversion.
- Account for engine `f64`, PnP mesh `f32`, and integer-grid rounding separately. Derive relevant error bounds for the declared domain.
- Validate after narrowing/quantization, including collapsed vertices, zero-length edges, changed hole contact, degenerate triangles, and index validity.
- Map an even-odd contour forest into outer rings/holes/disconnected components. Nested islands become appropriate components; do not assume upstream contour order identifies ownership.
- The test oracle must independently check the adapter's contour grouping and conversion; using the same grouping routine in the generator and its test would be circular.
- Do not repair or approximate rejected SCAD geometry silently. Existing STEP import repair behavior does not automatically become SCAD policy.

## 9. User-facing SCAD ingestion

### 9.1 Planned CLI

The flags below are approved interface defaults to implement, not existing commands.

```text
pnp_cli slice --model part.scad --scad-param "width=5*2" --output part.gcode
pnp_cli slice --model profile.scad --scad-extrude-height 2 --output profile.gcode
pnp_cli mesh import --input part.scad --output part.stl --scad-param "size=[10,20,5]"
```

- `--scad-param NAME=EXPR` is repeatable. Split on the first `=`. The last occurrence of a name wins. Evaluate with the SCAD expression evaluator and report malformed names/expressions clearly.
- Keep SCAD parameters out of `ResolvedConfig` and the slicer-config merge path. Preserve script variable names case-sensitively, including supported special variables; do not normalize a user's SCAD identifiers. The repository's snake_case rule applies to PnP configuration keys, not to rewriting identifiers in an external language.
- `--scad-extrude-height MM` must be finite and strictly positive. It applies only to a 2D evaluated result; reject it for an already-3D result instead of silently ignoring it.
- A 2D result without explicit extrusion is rejected with an actionable diagnostic.
- Define a clear error for empty or mixed-dimensional results that cannot be converted to one valid input mesh.
- External `include`, `use`, imports, or other file-resource requests are unsupported initially. Reject actual resource requests through the resolver/adapter policy; do not accidentally read relative paths from the process working directory.
- Initialize exact evaluation and use exact rendering. Make explicit special-variable override behavior testable; an explicit source/parameter choice must not accidentally select the geometry preview renderer.
- In feature-disabled builds, `.scad` input produces a clear support-disabled diagnostic. It must not be misrouted to a mesh parser.

### 9.2 Ownership and command routing

- Introduce an explicit SCAD-loading/import entry with options rather than teaching every `load_model` caller to evaluate scripts without parameter/extrusion context.
- Route only `Cmd::Slice` and `MeshCmd::Import` (`crates/pnp-cli/src/main.rs`) through it in this rollout.
- Reuse `assemble_object` and normal mesh assembly in `crates/slicer-model-io/src/loader.rs`.
- Derive identity from the original SCAD source basename and object index, consistent with `path_object_id`. Never derive IDs or labels from random temporary output names.
- Create one geometry object. Disconnected components remain within that mesh; display colors and groups do not create tool/material assignments.
- For slicing, extend `place_model_on_bed` (`crates/pnp-cli/src/main.rs`) to treat SCAD output as a bare model, using the established whole-model translation. Preserve the existing distinction that authored 3MF placement is retained.
- `mesh import` remains a geometry-conversion operation; do not invent a printer-bed configuration for it. Bed placement happens when the model is sliced.
- Resolve how STEP-only import options are rejected or handled on the SCAD branch. Do not silently inherit automatic repair or unrelated STEP semantics.
- Preserve structured errors and the existing CLI error conventions; keep source diagnostics on stderr rather than mixing them into G-code/mesh output.

### 9.3 Product acceptance cases

- Feature-off input rejection.
- Self-contained primitive/Boolean 3D model and parameterized model.
- Repeated expression parameters, nested vector values, malformed expressions, and separation from slicer configuration.
- 2D profile with a hole: explicit extrusion succeeds; missing/nonpositive/nonfinite height fails; height on 3D fails.
- Rejection of attempted external resources.
- Rejection of partial/approximate/degraded results; preservation of recognized informational diagnostics.
- Empty/nonfinite/invalid-conversion failure paths.
- Stable source-derived object identity and bed placement across repeated runs and different directories with the same basename.
- Focused `slice` success and `mesh import` export tests using small fixtures. Compare meaningful geometry/IR/output semantics, not incidental tessellation order unless deterministic ordering has been established as a contract.
- No final output is reported successful when source evaluation/import failed.

## 10. Host property obligation matrix

These are **property charters to implement and prove**, not claims that all listed invariants already hold. Packet preflight must finalize each charter's exact domain/oracle from the source and normative docs. Do not turn the cautions in section 3 into failing assertions.

| ID | Owner and production seam | Required property direction and domain/oracle |
|---|---|---|
| H-COORD | `mm_to_units`, `units_to_mm`, `Point2` (`crates/slicer-ir/src/slice_ir.rs`); SDK coordinate wrappers | Monotonic conversion on bounded finite domains; independently derived rounding/error envelopes; cross-wrapper unit consistency; exact integer identity retained without float round trips. Explicit boundary witnesses. |
| H-IDS | `LayerEntityIdGen` (`crates/slicer-ir/src/entity_id.rs`); modifier-ID helpers (`crates/slicer-ir/src/slice_ir.rs`) | Bounded allocation uniqueness/order, documented namespace/round-trip behavior on representable parents, sentinel handling and collision/error policy rather than universal hash injectivity. |
| H-IR | `validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`); selected IR serialization in `crates/slicer-ir/src/slice_ir.rs` | Anchor membership against `ordered_entities` via independent sets; targeted serde round trips/default fields with explicit nonfinite policy; invalid references rejected with the offending ID. |
| H-CONFIG | `RegionMapIR::intern_config`, `ResolvedConfig` (`crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/resolved_config.rs`); `check_scalar` (`crates/slicer-scheduler/src/config_resolution.rs`) | Hash/Eq consistency under actual equality semantics, idempotent interning and value lookup, config-bound and nonfinite rejection at validating APIs. Preserve cross-field constraints in valid strategies. |
| H-CHAINS | `enumerate_canonical_chains` (`crates/slicer-ir/src/region_split_registry.rs`) | Small independent Cartesian-product enumeration: cardinality, contents, empty choice, and declared canonical ordering. Bound multiplication and input size. |
| H-DAG | `topological_sort` (`crates/slicer-scheduler/src/topology.rs`) | Valid output permutation/edge precedence for generated DAGs; deterministic tie behavior where contracted; cyclic failure against an independent graph model. Include cycle-plus-acyclic-tail witnesses. |
| H-CLAIMS | validation passes (`crates/slicer-scheduler/src/validation.rs`); plan construction (`crates/slicer-scheduler/src/execution_plan.rs`) | Small-model reachability, write ordering/conflicts, claim scopes/exemptions, disabled-module behavior, and unmet dependencies. Encode documented family/fill exceptions explicitly. |
| H-REGISTRY | `aggregate_region_splits` (`crates/slicer-scheduler/src/region_split.rs`) | Declaration-order independence where specified, priority/name ordering, duplicates/invalid priority rejection, stable warning conditions. |
| H-RUNTIME | `module_invocation_allowed_on_layer` (`crates/slicer-runtime/src/layer_executor.rs`); `split_regions_by_modifier_footprints`, `sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`) | Independent semantic-set dispatch model; transactional behavior on invalid modifier operations; documented partition precedence/subset relationships on bounded region inputs. Exercise production bookkeeping without a new guest campaign. |
| H-BOOL | `union`, `intersection`, `difference`, `xor`, `clip_polylines` (`crates/slicer-core/src/polygon_ops.rs`) | Set identities/analytical cases within API fill/winding conventions; full-segment clipping and holes; geometric equivalence rather than raw contour-order equality; independent predicates. |
| H-OFFSET | `offset`, `offset2_ex` (`crates/slicer-core/src/polygon_ops.rs`); `smooth_outward_polygon` (`crates/slicer-core/src/smooth_outward.rs`) | Feature-preservation and valid-output contracts only on explicitly bounded clearance domains; source/canonical-backed special cases. No universal offset round trip or unproved monotonicity. |
| H-MESH | `slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`); `transform_point3` (`crates/slicer-core/src/lib.rs`); AABB queries (`crates/slicer-core/src/aabb_tree.rs`) | Analytical solid cross-sections away from ambiguous vertex planes, explicit vertex-plane cases, independent bounds/ray checks, finite and structurally valid output on valid meshes. Use direct and scripted solids. |
| H-ARACHNE | `run_arachne_pipeline` (`crates/slicer-core/src/arachne/pipeline.rs`); beading and skeletal graph kernels | Proven structural invariants, width/point alignment and role-appropriate open/closed behavior, valid bounded bead parameters, independently justified bead/graph relations. Verify canonical equivalents before generalizing fixtures. |
| H-REGIONMAP | `execute_region_mapping_with_cap` (`crates/slicer-core/src/algos/region_mapping.rs`) | Small independent expansion model, cap-boundary behavior, key uniqueness/error handling, canonical chain contents, default-config interning. |
| H-PAINT | paint-segmentation kernels (`crates/slicer-core/src/algos/paint_segmentation/`); `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`) | Source-backed partition/annotation delivery, stable IDs and namespaces, same-object/previous-layer selection, nonempty-return-path semantics; independent region-key and bounded geometry models. |
| H-SUPPORT | `execute_support_geometry` (`crates/slicer-core/src/algos/support_geometry.rs`); mesh/support analysis kernels | Scheduling/index/role relationships and feasible-envelope/clearance contracts on controlled layered scenes; valid degraded/absent outcomes distinguished from successful geometry. |
| H-LIGHTNING | `Generator` (`crates/slicer-core/src/algos/lightning/generator.rs`) | Deterministic output, valid graph references and source-backed local tree/outline contracts; bounded controlled support scenes. Credit delegated lightning module coverage here. |
| H-IO | `path_object_id`, `check_basename_collisions`, `object_world_z_extent`, `place_bare_mesh_on_bed` (`crates/slicer-model-io/src/loader.rs`) | Source-identity determinism, explicit basename collision handling, transform/extent validation, rigid whole-model placement within derived float tolerances, preservation of relative arrangement. |
| H-GCODE | `DefaultGCodeEmitter::resolve_feedrate`, emission (`crates/slicer-gcode/src/emit.rs`); serializer (`crates/slicer-gcode/src/serialize.rs`) | Bounded finite rate/unit/clamp behavior, role/tool/anchor sequences via a small state model, relative/absolute extrusion semantics, meaningful serialization round trips or parser-backed observations. Account for rounding and overflow. |

For `slicer-helpers`, select meaningful mesh/geometry operation contracts encountered in H-MESH/H-IO and add them to the charter ledger after inspecting their preconditions. Do not claim a blanket repair-preserves-geometry property: repair deliberately changes invalid topology.

Review the schema tables, macro expansion tests, integrated registry, and CLI locator for existing exhaustive/structural coverage. Record the reason where property testing adds no useful signal; do not manufacture random tests for fixed enumerations solely to claim every crate has one.

## 11. Module-kernel obligation matrix

Each row is anchored to actual production symbols. Finish the detailed property charter and canonical checks before coding the assertion. Tests for private kernels live in owning-crate test modules. Existing integration fixtures can be reused for domain witnesses without requiring a randomized entrypoint suite.

| Module | Production kernels and workspace path | Property direction |
|---|---|---|
| `classic-perimeters` | `emit_walls` — `modules/core-modules/classic-perimeters/src/lib.rs` | Role-appropriate closure, parallel metadata alignment, valid wall bands/degenerate handling under documented offset/width preconditions. Do not require configured wall count on a shape too small to hold it. |
| `arachne-perimeters` | `build_walls`, `classify_line`, `commit_wall_sequence` — `modules/core-modules/arachne-perimeters/src/lib.rs` | Classification/sequence preservation and width/point/flag alignment; closed versus open semantics from the source contract. Shared algorithm coverage belongs to H-ARACHNE. |
| `fuzzy-skin` | `apply_fuzzy_skin` — `modules/core-modules/fuzzy-skin/src/lib.rs` | Seeded repeatability, preservation of unselected spans/metadata, independently measured displacement from the corresponding source segment with justified tolerances. |
| `rectilinear-infill` | `scan_expolygon`, `adjust_solid_spacing` — `modules/core-modules/rectilinear-infill/src/lib.rs` | Full-segment inclusion/exclusion across holes, scan interval correctness, positive valid spacing, source/canonical-backed spacing boundaries. |
| `gyroid-infill` | `gyroid_f`, `align_to_grid`, `fill_expolygon` — `modules/core-modules/gyroid-infill/src/lib.rs` | Finite valid-domain evaluation, grid phase/floor behavior including negative coordinates, full-path clipping; implicit-surface residual checks only in the correct coordinate/phase domain with a derived sampling tolerance. |
| `lightning-infill` | Native module implementation — `modules/core-modules/lightning-infill/src/lib.rs`; H-LIGHTNING owns the generator | Reuse H-LIGHTNING for the algorithm; retain/extend focused sampling/unit/metadata tests only where module-owned variable behavior warrants it. |
| `infill-linker` | `connect_infill`, `chain_or_connect_infill` — `modules/core-modules/infill-linker/src/connect.rs`; graph helpers in `modules/core-modules/infill-linker/src/graph.rs` | Original path coverage/metadata compatibility, contour-following connector geometry, no cross-ring shortcuts, width/role/tool compatibility from an independent predicate. |
| `wave-overhangs` | `generate` — `modules/core-modules/wave-overhangs/src/generator.rs` | Determinism and justified trim/coverage contracts on bounded bridge domains; check whole paths/footprints. Generalize verified closure/coverage fixtures without assuming arbitrary rotational symmetry. |
| `traditional-support-planner` | `rasterize_polygons`, `fill_grid_holes`, `SupportGrid` — `modules/core-modules/traditional-support-planner/src/agg_raster.rs` | Compare tiny grids against an independent raster/flood-fill model; test documented extraction/clearance/territory semantics. No unconditional area-monotonicity claim across support layers. |
| `tree-support-planner` | `smooth_nodes`, `carve_emitted_regions`, `build_roles` — `modules/core-modules/tree-support-planner/src/lib.rs` | Valid graph references and role relationships, local smoothing constraints, source-backed carving/clearance behavior in feasible scenes; explicit degraded/impossible outcomes. |
| `traditional-support` | `fill_expolygon` — `modules/core-modules/traditional-support/src/lib.rs` | Whole-segment containment/hole exclusion, metadata and pitch semantics, narrow-region behavior only under the documented fallback preconditions. |
| `tree-support` | `render_polygon_with_wall_count`, `scan_fill_region` — `modules/core-modules/tree-support/src/lib.rs` | Distinct wall bands and fill clearance where guaranteed; whole-segment hole avoidance; small-tip cases with explicit nonempty-domain requirements. |
| `support-surface-ironing` | `fill_expolygon` — `modules/core-modules/support-surface-ironing/src/lib.rs` | Full-stroke containment and hole exclusion; planar coordinates and configured flow metadata for valid regions. |
| `top-surface-ironing` | `generate_zigzag_strokes_for_polygon` — `modules/core-modules/top-surface-ironing/src/lib.rs` | Independent scan-interval model, whole-stroke containment, holes, planar row geometry, legitimate empty/tiny regions. |
| `layer-planner-default` | `generate_object_layers`, `merge_layer_sequences` — `modules/core-modules/layer-planner-default/src/lib.rs` | Ordered/valid layer heights and correct per-object associations against a small independently derived sequence model. Derive first-layer/raft/catch-up/end rules; do not assert a generic `ceil(height/layer_height)` count. |
| `seam-planner-default` | `pick_seam_point` — `modules/core-modules/seam-planner-default/src/comparator.rs`; `fit_cubic_bspline` — `modules/core-modules/seam-planner-default/src/align.rs` | Candidate membership/empty behavior and documented paint priority; finite/appropriate fit behavior on supported point sets, with degeneracy and seeded-choice cases. |
| `seam-placer` | `select_seam_candidate`, `project_onto_wall_segment`, `rotate_wall_loop` — `modules/core-modules/seam-placer/src/lib.rs` | Cyclic order/geometry preservation for rotation, parallel-array correspondence, projection-on-segment, inserted-point metadata interpolation, source-backed fallback behavior. Account for duplicated closing endpoints. |
| `path-optimization-default` | `nearest_neighbor_permutation`, `coalesce_locked_candidates` — `modules/core-modules/path-optimization-default/src/lib.rs` | Index bijection, locked-block contiguity/direction, role grouping where required, deterministic tie behavior. Do not require a greedy route to be globally optimal. |
| `part-cooling` | `layer_fan_speed`, `cooling_decision_for_event` — `modules/core-modules/part-cooling/src/lib.rs` | Independent layer/role/config decision table, valid fan range and boundary transitions. Avoid encoding the same branch sequence as the oracle. |
| `overhang-classifier-default` | `build_speed_sections`, `calculate_speed` — `modules/core-modules/overhang-classifier-default/src/lib.rs` | Piecewise interpolation/clamping under finite bounded inputs. Monotonicity applies only when the supplied section speeds actually satisfy its precondition. |
| `machine-gcode-emit` | `substitute_placeholders` — `modules/core-modules/machine-gcode-emit/src/lib.rs` | Independent template-token model, literal preservation, nonrecursive substitution and unknown-key behavior; command-order preservation in existing entrypoint fixtures. |
| `skirt-brim` | `generate_skirt_entities`, `generate_brim_entities` — `modules/core-modules/skirt-brim/src/lib.rs` | Valid closed output and documented enclosure/spacing semantics on controlled print bounds. Avoid pinning an incidental rectangle implementation as canonical behavior. |
| `wipe-tower` | `generate_purge_paths` — `modules/core-modules/wipe-tower/src/lib.rs` | Purge paths within the declared tower region, positive valid flow/length, independent bed containment for the supported bed shapes. Corners alone do not establish containment in an arbitrary concave bed. |

Re-derive the root workspace module inventory when generating packets. Account for every selected member in the charter ledger. A directory on disk is not proof that it is a shipped/root workspace module; in particular, do not add nonmember experiments to this queue without a scope decision.

## 12. Runner, tiers, and Windows CI

### 12.1 Proposed command contract

Extend the existing runner rather than replacing it. Planned examples:

```text
cargo xtask test --summary --properties --tier routine
cargo xtask test --summary --properties --tier routine --with-scad
cargo xtask test --summary --properties --tier extended --with-scad
cargo xtask test --summary --properties --property H-DAG --seed <seed>
```

The concrete command names above are planned additions. The packet must specify and test parsing, conflicts, feature selection, log output, and exit behavior. Preserve existing `--summary`, `--summary-from`, and ordinary Cargo passthrough behavior.

- Use a central property/target manifest or equivalent typed registry with owner, target kind, features, supported tier, and expected property identifiers.
- Reserve a consistent generated-test naming convention (for example, `proptest_...`) and compare actual test discovery/execution against the registry.
- A named property selection must execute that property, not just compile a target. An empty selection or missing feature-gated property fails.
- Merge selected features explicitly, including `slicer-core/host-algos` wherever required. Do not rely on `test_command`'s current no-explicit-features fallback.
- Explicit property/tier/SCAD flags must not leak through as unknown Cargo/libtest flags.
- Validate conflicts such as execution selectors with `--summary-from` and invalid/unknown property IDs.
- Keep the existing guest/CLI freshness and literal gates in the orchestrated path. Reuse gate implementations rather than duplicating their policy.
- When multiple Cargo invocations are necessary, aggregate combined output into `target/test-output.log` for the whole selected run. Do not overwrite earlier failures with the last target's output.
- Preserve child failure status across logging/tee operations and report missing execution separately from test failure.
- Reject or clearly mark debugging overrides that reduce a run below its accepted profile; such a run must not be reported as completion evidence.

### 12.2 Tier semantics

**Routine:** bounded host/module properties with measured profiles; required on Windows PR checks. Ordinary cheap properties normally run in process. SCAD checks have a distinct enabled lane with bounded representative cases and necessary isolation.

**Extended:** larger domains/case counts and expensive geometry properties, manually invoked and required at relevant packet/rollout completion. The same property may run in both tiers with different profiles; some expensive properties are extended-only.

Property profiles are explicit code/configuration after pilot measurement. Honor Proptest configuration intentionally: setting an unconditional `Config::with_cases` must not silently defeat a documented runner override. Record resolved cases, rejection limits, shrink limits, isolation, seed, and input bounds in execution evidence.

### 12.3 Sampling and replay

- Fresh sampling is the default for each run/tier.
- Resolve and record a concrete run seed/configuration before executing cases; explicit replay selects the recorded seed.
- Include property ID, package/target/features, Proptest version, generator version/source state, and OpenRSCAD revision/backend when relevant in failure metadata.
- A concrete regression is the durable record when generator or engine changes invalidate seed-only replay.
- Record accepted cases, rejections/exhaustion, and whether shrinking completed. Add deterministic witness cases for rare required topology classes rather than relying on a statistical gate that intermittently misses them.

### 12.4 Windows checks

Add Windows-native jobs for this rollout rather than treating the existing Ubuntu jobs as Windows evidence:

1. default/core property build and routine execution;
2. SCAD-enabled build and bounded property/importer execution, with CMake/MSVC prerequisites;
3. manual extended execution with preserved failure artifacts.

Scope Windows CI changes to these gates and any supporting runner fixes. This specification does not require a wholesale migration of existing repository workflows.

Measure compile and execution effects separately. Proptest profile tuning must not disguise a dependency-build cost or silently remove expensive contract coverage. Native build and execution claims require actual Windows evidence, not merely an upstream Windows release.

## 13. Measurement and feasibility gates

### G-BASE — Baseline and inventory

- Record actual selected targets/features and run the narrow existing tests relevant to the first packet.
- Establish default and SCAD-enabled dependency-graph expectations.
- Check existing failures from execution/logs, not comments. Follow guest freshness rules before attributing guest/integration failures.
- Record baseline runtime/build context for comparisons; no numeric claims before measurement.

### G-HARNESS — Runner and per-case correctness

- Demonstrate a real generated property and a controlled failing test-runner fixture.
- Verify missing/zero executed properties are rejected.
- Verify explicit SDK/SCAD feature arguments preserve required host algorithm features.
- Verify seed replay, concrete failure artifacts, combined logs, and original exit statuses.
- Verify SDK state isolation after success, failure, and panic under concurrent tests.
- Verify default production and guest normal dependency graphs remain property/engine-free as specified.

### G-GEOMETRY — Generator/oracle validity

- Independent predicate tests pass, including counterexamples to vertex/midpoint-only containment and quantization-induced collapse.
- Required topology witnesses are reachable in the declared bounded domains.
- Every exposed valid shrink candidate remains valid; targeted invalid shrinkers retain their invalid witness.
- Controlled failure probes demonstrate meaningful complexity reduction for rings, holes, sets, and scripts.
- Measure rejection/generation/shrink behavior and establish profiles. No arbitrary success percentage is prescribed here.
- Record the decision on any custom general-polygon shrinker work.

### G-SCAD — Windows engine/adapter feasibility

- Pin and build the chosen OpenRSCAD revision/backend with the project's supported Rust configuration on Windows.
- Build default-off and enabled feature configurations; demonstrate that an ordinary default workspace build does not pull engine dependencies solely from workspace membership.
- Execute the public parse/evaluate/render pipeline and convert both holed 2D contours and a 3D indexed mesh.
- Demonstrate parameter-expression handling together with the required evaluation budget/exact mode, using a tested focused patch if necessary.
- Exercise strict diagnostic classification, including nonempty partial/degraded outputs.
- Validate finite/range/topology checks after conversion.
- Demonstrate reproducibility for controlled seeded/self-contained programs and fresh case state.
- Demonstrate process-level timeout/abort handling for tests requiring hard bounds; a thread timeout is not sufficient evidence.
- Measure representative generation/render/shrink costs and set eligible profiles.

### G-FAMILY — Coverage completion

- Every charter assigned to the work package has a verified domain/oracle and executable property, or an explicit user-approved disposition.
- Relevant existing example/canonical tests pass after fixes. Do not mask regressions by adding ignores, skip filters, loose tolerances, or weakened assertions.
- Confirmed failures have concrete regression tests.
- Routine and applicable extended profiles pass and executed-property evidence is retained.

### G-ROLLOUT — Final acceptance

- All selected host/module families are accounted for and their required gates pass.
- SCAD product acceptance and scripted-generation obligations pass, or the rollout is explicitly reported incomplete for them.
- Windows routine and SCAD-enabled checks execute in CI; extended runs are available manually and have passed for completion.
- Dependency isolation, documentation, feature discoverability, and reproducible failure workflow are complete.
- Run the required repository build/clippy/literal gates. Include a final workspace acceptance run only through the repository's gated entrypoint, after narrower verification is green and with the packet's explicit acceptance requirement.

## 14. Packet-generation queue

These labels are dependency identifiers, **not allocated packet numbers**. Re-derive free packet/task/deviation IDs at authoring time. Each row may become more than one packet if its coherent acceptance surface is too large; do not split solely by file count.

| Work label | Goal and required deliverables | Depends on | Completion proof |
|---|---|---|---|
| PBT-FOUNDATION | Add dependency/SDK opt-in, per-case guard, profile/replay conventions, property registry, initial xtask selectors, and one meaningful IR property as a vertical slice. | Source baseline | G-BASE and G-HARNESS; actual property execution and failure-path harness tests. |
| PBT-DOMAINS | Direct scalar/graph/IR/geometry strategies; independent predicates/oracles; concrete regression formatting; topology-aware shrinking primitives. | PBT-FOUNDATION | G-GEOMETRY for direct domains, measured pilot profiles. |
| SCAD-ADAPTER | Add optional `slicer-scad` boundary, pin engine, Windows build, combined evaluation options/diagnostic patches if needed, exact conversion. | Source baseline; can proceed alongside direct foundation work | G-SCAD; reusable adapter acceptance, no Proptest in product graph. |
| PBT-SCRIPTED | Add SDK scripted opt-in, typed recipe strategies, topology witnesses, shrink/replay artifacts; decide/fill general-polygon gaps. | PBT-DOMAINS, SCAD-ADAPTER | Combined G-GEOMETRY, measured scripted profiles; no reliance on approximate outputs. |
| SCAD-INGEST | Add optional model-I/O entry, CLI params/extrusion, source identity, strict errors, one-object assembly, slice bed placement, mesh import support. | SCAD-ADAPTER | Product acceptance in section 9; Windows feature-off/on checks and focused CLI tests. |
| PBT-IR-CONFIG | H-COORD, H-IDS, H-IR, H-CONFIG, H-CHAINS. | PBT-FOUNDATION, applicable PBT-DOMAINS | Derived numeric domains and independent data-model properties; relevant narrow tests. |
| PBT-SCHEDULER | H-DAG, H-CLAIMS, H-REGISTRY. | PBT-FOUNDATION, graph/config strategies | Tiny independent graph/claim models, cycle-tail and scope-exception regressions, routine execution. |
| PBT-RUNTIME | H-RUNTIME and selected pure host validation seams. | Relevant IR/scheduler/domain work | Transaction/dispatch/partition contracts and relevant runtime tests; freshness when required. |
| PBT-GEOMETRY | H-BOOL, H-OFFSET, H-MESH and selected helper contracts. | PBT-DOMAINS; scripted enrichment after PBT-SCRIPTED | Independent geometric oracles; direct and available scripted profiles; no unproved numeric identities. |
| PBT-ARACHNE | H-ARACHNE plus Arachne module classification/sequence properties. | PBT-DOMAINS, canonical/source contract audit | Correct host-algos execution, proven invariants, canonical-correct fixes and concrete regressions. |
| PBT-PREPASS | H-REGIONMAP, H-PAINT, H-SUPPORT, H-LIGHTNING. | Relevant IR/domains/geometry | Independent combinatorial and controlled layered-scene models; explicit feature gates. |
| PBT-WALLS | Classic perimeters and fuzzy skin; integrate Arachne-module obligations with PBT-ARACHNE. | PBT-DOMAINS, applicable geometry work | Kernel properties with valid wall/metadata domains; routine/extended profiles. |
| PBT-INFILL | Rectilinear, gyroid, linker, wave overhangs, top-surface ironing; credit lightning generator to PBT-PREPASS. | PBT-DOMAINS, applicable PBT-GEOMETRY | Whole-path oracles, valid spacing/phase domains, source-backed coverage contracts. |
| PBT-SUPPORT-MODULES | Traditional/tree planners and renderers, support-surface ironing. | PBT-DOMAINS, relevant support/prepass contracts | Raster/reference models, feasible-scene and role/clearance properties, no false always-success assertions. |
| PBT-LAYERS-SEAMS | Layer planner, seam planner/placer, path optimization. | PBT-FOUNDATION, IR/graph/geometry strategies as needed | Independent sequence/permutation/selection models and degeneracy witnesses. |
| PBT-IO-GCODE | H-IO/H-GCODE plus machine G-code, cooling, overhang speeds, skirt/brim, wipe tower. | Relevant direct strategies and IR contracts | State/sequence/template and controlled geometry properties, source-derived tolerances. |
| PBT-CI-CLOSURE | Final registry audit, Windows jobs/manual dispatch, measured profiles, docs and whole-rollout acceptance. | All required coverage and SCAD packages | G-ROLLOUT and complete evidence ledger. |

CI wiring is incremental: each accepted routine family enters its required Windows gate when delivered. PBT-CI-CLOSURE audits and completes the wiring; it is not permission to leave earlier properties unexecuted.

If G-SCAD fails, the host/module queue continues wherever direct strategies suffice. Any unmet broad geometry requirement receives an explicit direct-generator/fix packet. The SCAD branch remains blocked, rather than being quietly declared optional-complete.

## 15. Verification commands and repository discipline

### Existing gates

```text
cargo build --workspace
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask build-guests --check
```

Use feature-enabled variants for optional code after those features exist. Build/check/clippy commands must compile the selected test targets; default-only gates do not prove optional SCAD/property targets compile.

### Narrow test templates

```bash
set -o pipefail
mkdir -p target
cargo test -p slicer-ir --test <actual-target> -- <actual-property> --nocapture 2>&1 | tee target/test-output.log
```

```bash
set -o pipefail
mkdir -p target
cargo test -p slicer-core --features host-algos --test <actual-target> -- <actual-property> --nocapture 2>&1 | tee target/test-output.log
```

```bash
set -o pipefail
mkdir -p target
cargo test -p <module-package> --lib -- <actual-property> --nocapture 2>&1 | tee target/test-output.log
```

These are templates, not assertions that a target/property already exists. Derive names from the charter registry and Cargo test discovery. For SDK tests use the explicit relevant SDK feature, including `test`/new property features as applicable.

Every Cargo test/nextest invocation retains combined output in `target/test-output.log`. Inspect that file rather than rerunning tests to recover truncated console output. On Windows PowerShell, preserve the native exit status when using `Tee-Object`; on Bash use `pipefail`.

Broad/multi-crate and guest-dependent runs use `cargo xtask test`; retain freshness semantics and treat infrastructure-error exit codes distinctly. SDK source/manifest changes can affect guest freshness even if test symbols do not ship in guest binaries. Do not claim guest artifacts are unaffected without the actual check.

For a final packet acceptance that explicitly requires the workspace suite, after narrower checks pass:

```text
cargo xtask test --summary --workspace
```

Dispatch that required full run to a subagent for a compact factual pass/fail report according to repository instructions. Optional SCAD/property targets need their separately specified enabled gate as well; the default workspace run alone is insufficient.

Do not expand the existing quarantine/skip policy to hide newly discovered property or parity failures. Do not run the full workspace suite merely to avoid identifying the correct narrow feature-enabled command.

## 16. Documentation and handoff deliverables

Packet implementation should update the relevant canonical documents, not only this master spec:

- `docs/21_data_defaults_and_fixtures.md`: shared strategy home, FRU convention, concrete-regression workflow, property-feature use.
- `docs/05_module_sdk.md`: native per-case property harness, feature ownership, private-kernel test placement, scripted opt-in.
- CLI/model-input documentation and `docs/08_coordinate_system.md` only where the new conversion/placement contract adds normative information.
- Test-command documentation and `AGENTS.md`: short links to the canonical property workflow, correct features, Windows gates, and logging/replay requirements. Read the writing-for-agents skill before editing agent instructions in implementation.
- A durable property charter/profile registry and a compact contributor guide explaining how to add a property without circular oracles or vacuous domains.
- Upstream patch provenance and compatibility diagnostic classification tied to the selected OpenRSCAD version.

### Next-session instructions

1. Read current `AGENTS.md`, `.agents/doc-index.md`, and the relevant canonical docs before authoring packets.
2. Re-derive workspace inventory, active packet state, and free identifiers. Do not treat this document as a reservation of packet/task/deviation numbers.
3. Load `spec-packet-generator` and use its batch protocol to materialize the queue/dependencies. Use `spec-review --preflight` as required by that workflow.
4. Keep approved architecture and user-facing policies fixed. Record feasibility measurements and exact property domains as evidence, not as new user interviews unless a genuine design change is needed.
5. Start with PBT-FOUNDATION and the independent SCAD-ADAPTER feasibility branch as the packet workflow permits. Do not block cheap host correctness work on the evaluator.
6. Report incomplete or blocked gates honestly. In particular, no source-inspection statement in this document is a build/test pass.

## 17. Primary sources and navigation

### Repository sources

- Workspace packages/features: `Cargo.toml`; `crates/slicer-core/Cargo.toml`; `crates/slicer-core/src/lib.rs`; `crates/slicer-core/src/algos/mod.rs`; `crates/slicer-sdk/Cargo.toml`.
- Architecture/ownership: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`.
- Units/placement: `docs/08_coordinate_system.md`; `mm_to_units`, `units_to_mm`, `IndexedTriangleSet`, `Polygon`, `ExPolygon` in `crates/slicer-ir/src/slice_ir.rs`.
- Fixture policy: `docs/21_data_defaults_and_fixtures.md`; `reset_global_state`, `mock_host_setup`, `mock_host_teardown`, `install_panic_handler` in `crates/slicer-sdk/src/test_support/mod.rs`; `MockHost` in `crates/slicer-sdk/src/test_support/mock_host.rs`.
- Macro test setup: `generate_module_test_impl` in `crates/slicer-macros/src/lib.rs`.
- Gated runner: `test_command`, `check_literals_preflight`, `handle_guest_freshness_with`, `ensure_pnp_cli_fresh` in `xtask/src/test.rs`; `jobs.test` and other jobs in `.github/workflows/ci.yml`.
- Import boundary: `load_model`, `assemble_object`, `path_object_id`, `place_bare_mesh_on_bed` in `crates/slicer-model-io/src/loader.rs`; `SliceRunOptions` in `crates/slicer-runtime/src/run.rs`; `Cmd::Slice`, `MeshCmd::Import`, `place_model_on_bed` in `crates/pnp-cli/src/main.rs`.
- Packet-input precedent: `docs/specs/struct-literal-churn-gate-plan.md`.

### Proptest

- Crate and version metadata: <https://crates.io/crates/proptest>
- Configuration: <https://docs.rs/proptest/latest/proptest/test_runner/struct.Config.html>
- Strategies: <https://docs.rs/proptest/latest/proptest/strategy/trait.Strategy.html>
- Shrinking interface: <https://docs.rs/proptest/latest/proptest/strategy/trait.ValueTree.html>
- Filtering: <https://proptest-rs.github.io/proptest/proptest/tutorial/filtering.html>
- Failure persistence: <https://proptest-rs.github.io/proptest/proptest/failure-persistence.html>
- Forking/timeouts: <https://proptest-rs.github.io/proptest/proptest/forking.html>
- Limitations: <https://proptest-rs.github.io/proptest/proptest/limitations.html>

### OpenRSCAD — the selected project

- Project and build prerequisites: <https://github.com/matthova/openrscad>
- Compatibility behavior: <https://github.com/matthova/openrscad/blob/main/COMPAT.md>
- Workspace/MSRV: <https://github.com/matthova/openrscad/blob/main/Cargo.toml>
- Native dependency gating: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/Cargo.toml>
- Parse/AST: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-syntax/src/lib.rs>
- Evaluation/options/resolver: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-eval/src/lib.rs>
- Rendering/diagnostics: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/src/lib.rs>
- Mesh representation: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/src/mesh.rs>
- 2D implementation: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/src/shape2d.rs>
- Backend abstraction: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/src/kernel.rs>
- Native executable/evaluation worker: <https://github.com/matthova/openrscad/blob/main/crates/openrscad-cli/src/main.rs>
- Windows release build: <https://github.com/matthova/openrscad/blob/main/.github/workflows/release.yml>

The `main`/`latest` links are navigation sources, not a dependency pin. The implementation must select and record a concrete compatible revision/version at adoption; do not freeze a speculative SHA here.

## 18. Validation record for this document

Performed in the authoring session: read-only repository/source/documentation inspection, upstream primary-source inspection, review of proposed feature and CLI seams, and explicit user approval of the consolidated design.

Not performed: dependency integration, native builds, Proptest execution, existing test-suite execution, runtime/compile benchmarks, actual generator/shrinker experiments, or Windows SCAD integration tests.

The plan file is the sole authored artifact of this session. Repository implementation is unchanged. All performance impact and feasibility claims remain unmeasured until the named gates execute.
