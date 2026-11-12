---
packet: proptest-openrscad_04_pbt-scripted
status: implemented
---

# Typed scripted geometry strategies and the hybrid geometry decision

## Goal

Add native-only SDK scripted property support with bounded typed SCAD recipes, independently witnessed geometry, useful valid shrinking, reproducible artifacts and measured Windows profiles, then resolve the combined direct/scripted G-GEOMETRY coverage decision.

## Why

`docs/specs/proptest-openrscad-adoption.md` §§5–7, 12–13 requires rich scripted stimulus without treating a successful engine render as a correctness oracle. Direct bounded catalogs from the domains packet deliberately do not claim unrestricted polygon generality; this packet measures the hybrid, fills bounded direct gaps and identifies any genuinely blocking general-generator work rather than silently reinstating an unconditional custom shrinker prerequisite.

Native evaluation can happen during `Strategy::new_tree` and `ValueTree::simplify`, not only inside the SUT callback. A callback-only guard or timeout therefore cannot certify the generator lifecycle. Artifacts and hard process supervision must cover generation, candidate rendering, shrinking and replay as well as property callbacks.

## Scope

- In: SDK `property_testing_scad`; bounded typed 2D/3D recipe AST/emission; adapter-backed evaluated strategies/value trees; topology/solid witnesses and independent validity/reference checks; pre-execution source/recipe/options/engine artifacts; replay and concrete regression extraction; whole-property process supervision for SCAD registry records; measured routine/extended profiles; incremental required Windows SCAD-property CI; combined G-GEOMETRY evidence/conditional gap handling.
- Out: Proptest in product graphs, another SCAD evaluator/converter, arbitrary source fuzzing as the valid strategy, claiming OpenRSCAD as the slicer's oracle, host/module property completion, a mandatory full general-polygon generator regardless of feasibility, randomized guest/E2E quotas, CLI ingestion, global minimum claims, persistent geometry caching, or a hard in-process native render deadline.

## Depends on

- `docs/spec_packets/proptest-openrscad_02_pbt-domains/` (**draft at authoring**), transitively #1: consumes planned SDK `property_testing`, `PropertyRun::{new,bound,check}`, `CaseGuard`, `domains::{DomainError,GeometryBounds,GeometryDomain,GeometryCase,TopologyClass,BoundaryRule,GeometryStrategy,GeometryValueTree,geometry_cases,geometry_complexity}`, independent `oracles::{validate_geometry,validate_indexed_mesh,mesh_signed_volume_mm3,point_in_region,segment_in_region}`, and `regression::{ConcreteInput,format_regression_fixture}`. Preserve their exact packet shapes; no prerequisite is asserted satisfied.
- `docs/spec_packets/proptest-openrscad_03_scad-adapter/` (**draft at authoring**): consumes its exact planned `slicer_scad::{ScadSource,ScadEvalOptions,ScadLimits,ScadGeometry,ScadEvalOutput,ScadEngineInfo,ScadError,evaluate,engine_info}` contract, strict diagnostic/conversion boundary and passed adapter-prerequisite portion of G-SCAD, not full G-SCAD. This packet owns the remaining recipe generation/shrink cost measurements, final eligible property profiles and full G-SCAD evidence, plus combined G-GEOMETRY. Implementation and closure wait for both prerequisites to be implemented and exports/feature graphs rechecked on disk. Authoring may proceed now.

## Design notes

### Ownership, gates and independence

`crates/slicer-sdk/Cargo.toml` currently has only `test` and a self dev-dependency enabling it. The foundation's planned `property_testing` enables optional Proptest but not an engine. Add **new** `property_testing_scad = ["property_testing", "dep:slicer-scad", "slicer-scad/openrscad"]`, with `slicer-scad` an optional dependency under `cfg(not(target_arch = "wasm32"))`. Gate the entire **new** `slicer_sdk::test_support::property_testing::scripted` module with `all(feature = "property_testing_scad", not(target_arch = "wasm32"))`. Do not change SDK `test`, defaults, self dev-dependency, production prelude or guest manifests to enable this feature. Normal product CLI builds, including the later SCAD-enabled CLI, must remain Proptest-free. Ordinary property support remains engine-free.

This follows ADR-0004, ADR-0016 and the fixture-home amendment to ADR-0054 (`docs/adr/0004-test-support-lives-in-slicer-sdk.md`, `docs/adr/0016-test-prelude-whole-module-gate.md`, `docs/adr/0054-host-side-test-support-crate.md`). No new fixture crate, normal-dependency cycle, production visibility widening, ADR reservation or deviation ID is required.

The domains packet's predicates may certify small converted inputs; they do not prove the engine modeled the intended recipe. Add independently derived reference observations for controlled recipe families. Against Clipper production code, no Clipper/self-union validity oracle; against another algorithm using `geo` or Manifold, disclose shared paths before calling OpenRSCAD differential evidence. A generated mesh is stimulus, not an expected slicer answer. No OrcaSlicer behavior is borrowed/ported by these helper charters.

<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`).

Recipe integral dimensions/coordinates below are **canonical units**, explicitly commented at raw integer construction sites. Emit millimeter expressions using the canonical scale and deterministic escaping/number formatting; do not narrow through f32 before source emission. All evaluated output still crosses the adapter's checked f64/grid/f32 boundaries. A source-valid recipe is not necessarily valid after conversion.

<!-- snippet: wasm-staleness -->
- Feeds guest WASM: run `cargo xtask build-guests --check` after edits and judge by exit code (0 fresh, 1 stale, 3 infrastructure error) before interpreting any guest test failure (AGENTS.md §Guest WASM Staleness).

### Planned SDK exports: exact downstream surface

All names below are **new/planned**, crate `slicer-sdk`, in `slicer_sdk::test_support::property_testing::scripted`, with public definitions in **new** `crates/slicer-sdk/src/test_support/property_testing/scripted/mod.rs`; private emission/strategy/artifact/reference subfiles may be split below it. Unqualified `GeometryDomain` and `DomainError` refer to the domains packet's existing planned exports, not new substitutes. Adapter names retain the #3 shapes.

```rust
pub enum Recipe2d {
    Rectangle { size_units: [u32; 2] },
    Disk { radius_units: u32, facets: u16 },
    Polygon { polygons: Vec<slicer_ir::ExPolygon> },
    Translate { offset_units: [i64; 2], child: Box<Recipe2d> },
    QuarterTurn { turns: u8, child: Box<Recipe2d> },
    Union(Vec<Recipe2d>),
    Difference { base: Box<Recipe2d>, cuts: Vec<Recipe2d> },
    Intersection(Vec<Recipe2d>),
}
pub enum Recipe3d {
    Box { size_units: [u32; 3] },
    Cylinder { radius_units: u32, height_units: u32, facets: u16 },
    LinearExtrude { height_units: u32, profile: Box<Recipe2d> },
    Translate { offset_units: [i64; 3], child: Box<Recipe3d> },
    QuarterTurnZ { turns: u8, child: Box<Recipe3d> },
    Union(Vec<Recipe3d>),
    Difference { base: Box<Recipe3d>, cuts: Vec<Recipe3d> },
    Intersection(Vec<Recipe3d>),
}
pub enum ScadRecipe { Contours(Recipe2d), Solid(Recipe3d) }
pub enum SolidWitness { ClosedSolid, Cavity, Bridge, Overhang, SeparatedBodies }
pub struct RecipeBounds {
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_facets: u16,
    pub max_source_bytes: usize,
    pub max_evaluations: u32,
}
pub struct ScriptedDomain {
    pub geometry: super::domains::GeometryDomain,
    pub solid_witness: Option<SolidWitness>,
    pub bounds: RecipeBounds,
}
pub struct ScriptedCase {
    pub recipe: ScadRecipe,
    pub recipe_bounds: RecipeBounds,
    pub source: slicer_scad::ScadSource,
    pub options: slicer_scad::ScadEvalOptions,
    pub output: slicer_scad::ScadEvalOutput,
}
pub enum ScriptedError {
    Domain(super::domains::DomainError),
    Adapter(slicer_scad::ScadError),
    Artifact(String),
    ReplayMismatch(String),
}
pub struct ScriptedStrategy { /* private fields */ }
pub struct ScriptedValueTree { /* private fields */ }
// ScriptedStrategy: Strategy<Value = ScriptedCase, Tree = ScriptedValueTree>.
// ScriptedValueTree: ValueTree<Value = ScriptedCase>, with the actual
// current(&self), simplify(&mut self), complicate(&mut self) protocol.
pub type RecipeComplexity = (usize, usize, u128);
pub fn recipe_complexity(recipe: &ScadRecipe) -> Result<RecipeComplexity, ScriptedError>;
pub fn emit_scad(recipe: &ScadRecipe, bounds: &RecipeBounds)
    -> Result<slicer_scad::ScadSource, ScriptedError>;
pub fn evaluate_recipe(
    recipe: &ScadRecipe,
    options: &slicer_scad::ScadEvalOptions,
    bounds: &RecipeBounds,
) -> Result<slicer_scad::ScadEvalOutput, ScriptedError>;
pub fn scripted_cases(domain: ScriptedDomain, options: slicer_scad::ScadEvalOptions)
    -> Result<ScriptedStrategy, ScriptedError>;
pub fn write_scripted_artifacts(directory: &std::path::Path, case: &ScriptedCase)
    -> Result<(), ScriptedError>;
pub fn replay_scripted_case(directory: &std::path::Path)
    -> Result<ScriptedCase, ScriptedError>;
```

Recipe enums, `SolidWitness`, `RecipeBounds` and `ScriptedCase` implement `Debug + Clone + serde::Serialize + serde::Deserialize`. `ScriptedDomain` and strategy/tree implement `Debug` as required; no new trait requirement is imposed on the prerequisite's public domain types. `ScriptedError` implements `Debug + Display + std::error::Error`. `RecipeBounds` has a documented bounded `Default` for FRU, chosen from pilots. `ScriptedCase` has no artificial default; exhaustive test waivers identify the complete replay-record contract. No new variant is added to prerequisite `ConcreteInput`.

- `solid_witness = None` selects `ScadRecipe::Contours`; `geometry` describes the **converted final contour** domain with its exact #2 class/minima/boundary semantics. `Some(w)` selects `ScadRecipe::Solid`; `geometry` describes the bounded constituent planar profile domain, while `w` is independently checked on the resulting solid scene. It is not a claim that arbitrary 3D output inherits a 2D topology label. `SeparatedBodies` retains at least two disjoint positive-volume bodies, `Cavity` a nonzero void, and Bridge/Overhang retain independently measured positive unsupported spans in controlled scenes; `ClosedSolid` may simplify away those optional structures. Empty contours require the explicit `TopologyClass::Empty`; every solid witness is nonempty.
- Constructor validation rejects inconsistent geometry/recipe/adapter capacities before generation. Use checked conversions from registry u64 bounds to u16/u32/usize/i64. Positive primitive dimensions, `turns in 0..=3`, minimum polygonal facet counts, total nodes/depth/source length, sum of explicit vertices/facets, all coordinate/translation/extrusion intermediates and adapter mesh-output counts are bounded. `max_evaluations` caps actual candidate evaluations per tree, including initial retries and shrinking; `geometry.bounds.max_shrink_candidates` caps candidate proposals. Limits cannot be reset per failed proposal to obtain an effectively unlimited shrink.
- `emit_scad` emits a readable self-contained program with a stable diagnostic label `recipe.scad`, explicit facet counts and no resource loads, ambient variables, text/font requests, loops, recursion or random programs. Explicit polygon leaves preserve arbitrary **already validated bounded** non-star-shaped/holed shapes, not just radial vertices. Fixed typed operations prevent 2D/3D mixing; CSG trees can still be empty/invalid and must be evaluated/validated. Statement/source injection is not an AST variant. `evaluate_recipe` emits and calls the public `slicer_scad::evaluate` with the supplied options; it is not another renderer, and its caller must supply process containment when needed. It guarantees adapter acceptance, not an owner's full property preconditions or witness certification without a `ScriptedDomain`.
- Build valid cases primarily by constructive templates, bounded combinations and known geometric clearances. Reuse #2 `geometry_cases` for explicit leaves and independent validators for output; do not implement a second direct polygon domain with different winding/contact/minimum semantics. General 2D witnesses include convex, concave, genuinely non-star-shaped, multiple holes, disconnected sets, thin corridors/necks/gaps/short edges and quantization-scale/almost-collinear cases. At least one non-star-shaped witness has an independently justified empty visibility kernel. 3D witnesses include extrusions, cavities, bridges, overhangs, separated bodies and varying controlled cross-sections.

### Independent witness and shrink obligations

Reference checks must exercise actual converted geometry, not just AST tags. For contours use #2 independent ring/region predicates, exact region point/segment observations and analytical rectangle/polygon set areas. For controlled orthogonal box/extrusion CSG scenes derive a small cell-occupancy reference from dimension/boundary intervals, independently of engine/kernel code; confirm cavity/bridge/overhang/body relationships at designated interior planes/volumes. For faceted disks/cylinders use regular-polygon geometry, not ideal-circle/sphere equality. Validate mesh finite/index/area/closed-edge conditions with `oracles::validate_indexed_mesh` and dimension-derived bounds/volume with derived mm/mm³ error envelopes. Document which 3D facts these bounded witnesses establish; edge incidence alone does not establish arbitrary self-intersection freedom or a cavity. Add negative controls by removing a hole/body/span from otherwise valid outputs and by supplying quantization-collapse counterexamples.

`RecipeComplexity` is lexicographic `(node_count, explicit_polygon_vertices_plus_facets, sum_abs_integral_parameters)`, all computed with checked arithmetic over dimensions/translations/turns/facets and polygon coordinates. Accepted value-changing simplifications strictly decrease this tuple; `geometry_complexity` from #2 additionally records contour structural changes. Do not claim global minimality. Delete subtrees/Boolean operands, simplify operations, remove optional components/holes/vertices, shrink translations/dimensions/facets and simplify profiles only if the requested domain and solid witness survive.

`ScriptedValueTree::current` returns the cached certified case and performs no evaluation or I/O. Evaluate/revalidate each proposed candidate before exposing it; validate after adapter quantization/narrowing, not just after AST editing. `current()` remains valid after every successful **and false-returning** `simplify`/`complicate` transition. Repeated `complicate` restores the pre-simplification value according to actual Proptest `ValueTree` semantics, verified against the adopted API ([1.11 interface navigation](https://docs.rs/proptest/1.11.0/proptest/strategy/trait.ValueTree.html)); it need not implement numeric binary search. Restore certified cached states rather than re-rendering in `current`. Hole-/disconnected-/non-star-/solid-specific properties retain their required witness, not merely nonzero output.

Expected candidate-domain failures (for example a Boolean proposal that becomes empty or loses required clearance) may be rejected and counted within finite limits. Parse/evaluation/unknown/partial/degraded/approximate/resource errors from emitted valid recipes are **adapter/generator failures**, not an ordinary discarded sample and never a SUT success. Bound initial retries and shrink candidate/evaluation counts, record reasons, and do not solve a rejection-heavy strategy merely by raising limits.

Because `ValueTree` methods cannot return `Result`, integrate private per-run/per-process generator telemetry with the foundation's `PropertyRun::check` before it emits controller completion. A fatal candidate/adapter/artifact error or exhausted correctness generation must fail the run even when it happened outside a callback; preserve the last certified case, attach failure classification `generator_or_oracle`/`exhausted`/`harness` as appropriate and do not emit `passed`. Reaching only the shrink-candidate/evaluation limit on a known failure retains that failure with `shrink_completed = false`. Do not change the public `PropertyRun::check` callback shape or fabricate Proptest counters. Fork workers report their actual attempt observations without impersonating controller completion. No case-local SDK state is assumed during strategy generation; adapter state is independently fresh, and SUT callbacks still receive the fresh foundation `CaseGuard`.

Controlled known-failing probes begin with nonminimal nonempty rings, holed/disconnected recipes and 3D CSG scenes. They must exhibit real operation/vertex reduction, removal of optional holes/bodies, and retention of required holes/bodies/cavities/spans. A no-op shrinker, all-rejected population, initially minimal fixture or shrinking from a hole-specific failure to a plain box cannot satisfy this gate.

### Failure, replay and hard containment

Use the foundation's existing context/event/version/seed/artifact schemas and run directory; no parallel registry or alternate seed history. Preserve `generator_version` and source-state content digest. Add these **new planned artifact files** under `target/property-testing/<run_token>/<property_id>/attempts/<process-role>-<pid>-<sequence>/`:

- Before engine execution: `recipe.json`, `recipe-bounds.json`, `source.scad`, `options.json`, `engine.json`, and `state.json`. The state object has `schema_version: 1`, `stage: "prepared" | "evaluated" | "rejected" | "failed" | "timeout" | "aborted"`, `generator_version: positive integer`, `source_digest: string`, `input_digest: string`, `reason: string | null`, `shrink_completed: bool | null`. Both digests are lowercase SHA-256 hex. `source_digest` covers source UTF-8 bytes; `input_digest` covers the ordered fixed-filename inputs (`recipe.json`, `recipe-bounds.json`, `source.scad`, `options.json`, `engine.json`) as filename/byte-length/content records, plus generator version, with documented unambiguous framing. Changing options or bounds must change this digest as well as changing recipe/source. Save/flush recipe before emission if emission itself may fail; never execute the adapter before readable source/options/engine identity exist. An emission failure before a complete input set has a failure record, not a falsely complete `prepared` state.
- After evaluation where available: `geometry.json` (serialized `ScadGeometry`), `diagnostics.json` (the adapter diagnostics), final `state.json`; preserve provenance in the serialized full output `output.json`. A failure during emission keeps its recipe plus honest failure state, not a nonexistent source claim. Artifacts remain target-local; no large committed ad hoc JSON fixture.
- At a retained failure: foundation `failure.json` links the actual attempt directory and records the existing failure classification, context/seed/profile/features, evaluator pin/backend/patch, source/options, retained geometry, shrink status and reproduction command. No minimization completion claim after timeout/abort/limit or unknown completion. A process-killed prepared attempt is evidence of an interrupted input, not proof it was the globally smallest failure.

`write_scripted_artifacts` writes a complete evaluated case using the same representation, fixed filenames and safe atomic/error handling; an artifact failure fails the diagnostic-producing run. `replay_scripted_case` reads these fixed files, checks schema/generator/source/input digests, re-emits and compares source, validates the active engine revision/backend/patch identity, then evaluates the retained concrete recipe/options and returns a full case. Mismatches return `ReplayMismatch`, missing/damaged files `Artifact`, engine errors `Adapter`; no seed regeneration is required. It reuses the original bounds and refuses to silently evaluate with different defaults. Treat the caller-supplied directory as data: never derive file paths from source labels, recipe strings or manifest references; reject symlink/reparse-point escapes from the chosen replay directory and oversized input files before loading them. These helper calls are library operations, not a promise of in-process cancellation. Replay contract tests run them inside the same killable supervision used for native work. The recorded seed command remains a second reproduction route, for example `cargo xtask test --summary --properties --property D-SCAD-2D --tier routine --with-scad --seed 42`, with the **actual retained seed**, not a hardcoded substitute, in artifacts.

Produce concrete owning-SUT regressions through the unchanged #2 formatter: contours map to `ConcreteInput::Geometry(GeometryCase { polygons })`, meshes to `ConcreteInput::Mesh(mesh)`, empty contours only where the owner accepts them. Compile/evaluate emitted `fixture.rs` without engine/Proptest/PropertyRun context and add the real independently justified SUT assertion only after diagnosing a confirmed defect. Preserve source/options alongside geometry for engine defects; never commit seeds as the durable regression. A controlled injected failure proves artifacts/shrinking but is not claimed as a discovered production defect.

Extend the foundation's planned `xtask/src/properties.rs` executor for every selected `requires_scad = true` record: require a positive checked `bounds.test_process_timeout_ms`, supervise the **whole exact libtest execution child**, including strategy generation and shrink rendering, and kill/join on deadline. Keep Proptest's profile `timeout_ms`/`isolation` meanings unchanged; all new SCAD profiles use measured `isolation = "fork"` and positive callback `timeout_ms` in addition to whole-process supervision. Preserve original native status; timeout/abort yields nonzero run status and retained artifacts, never a green missing-evidence summary. The supervisor must contain/terminate Proptest fork descendants on Windows too; prove actual process-tree cleanup using supported Windows process containment rather than assuming killing only the controller kills its workers. Discovery may compile/list, but is not accepted execution evidence. No new CLI selectors are required.

### Planned registry records, bounds and Windows profiles

Append to the foundation's root `test-properties.toml`, preserving its exact schema:

| ID | `test_name` | `charter` |
|---|---|---|
| `D-SCAD-2D` | `proptest_scripted_contour_domains` | `docs/testing/properties.md#d-scad-2d` |
| `D-SCAD-3D` | `proptest_scripted_solid_domains` | `docs/testing/properties.md#d-scad-3d` |

Both have `package = "slicer-sdk"`, `target_kind = "test"`, `target_name = "property_scripted"`, `features = ["slicer-sdk/property_testing_scad"]`, `requires_host_algos = false`, `requires_scad = true`, `generator_version = 1`, `production_symbol = "slicer_sdk::test_support::property_testing::scripted::scripted_cases"`, `production_path = "crates/slicer-sdk/src/test_support/property_testing/scripted/mod.rs"`, and measured routine **and** extended profiles. These validate helper contracts and do not credit host/module SUT coverage.

Required `bounds` keys for both records are exactly `max_abs_units`, `max_vertices`, `max_holes`, `max_components`, `max_shrink_candidates`, `max_recipe_nodes`, `max_recipe_depth`, `max_facets`, `max_source_bytes`, `max_evaluations`, `max_scad_vertices`, `max_scad_triangles`, `evaluation_steps`, `worker_stack_bytes`, `test_process_timeout_ms`. The geometry keys feed the existing domain; recipe keys map to their named fields; adapter options derive their checked coordinate bound from canonical units and use explicit mesh/fuel/stack/source capacities. Derive/check `max_ast_nodes`, `max_ast_depth` and `max_geometry_nodes` conservatively from the emitter/recipe bound, rather than using an unmeasured permissive fallback. `max_facets` must fit both recipe u16 and adapter u32. Height/translation intermediates fit the same `max_abs_units` coordinate envelope. Profile/timeout/case/rejection/shrink values are chosen after Windows measurements, not prescribed numerically here.

Add **new** SDK standalone targets `property_scripted` and `property_scripted_contracts`, both `required-features = ["property_testing_scad"]`, with no hidden unregistered aggregated modules. Add **new** `jobs.properties-scad-windows` to `.github/workflows/ci.yml`, using the checked-in Rust and native CMake/MSVC plus wasm freshness prerequisites, running `cargo xtask test --summary --properties --tier routine --with-scad`. Preserve the ordinary non-SCAD lane and #3 adapter gate. Failure uploads include the canonical combined log and all `target/property-testing/` attempts, including aborted pre-render cases. Verify the foundation feature union keeps required owner host algorithms enabled with real scripted features. Extended execution is manually reproducible and required for this packet's closure; final manual-dispatch UX remains with rollout closure.

### Combined G-GEOMETRY decision and conditional gap ownership

Create **new** `docs/testing/proptest-scripted-evidence.md`: source/dirty-content/toolchain/Windows/features/pin/backend/profile context, actual witness reachability, independent reference derivations/negative controls, generation/rejection/evaluation attempts, candidate retry/rejection reasons, structural shrink traces, interrupted versus completed minimization, separate compilation/generation/render/shrink measurements and final eligible property-profile reasons. Combine these generation/shrink/profile results with #3's adapter-prerequisite evidence from `docs/scad/windows-feasibility.md` to record full G-SCAD as passed only when all its obligations are met; missing evidence keeps that gate incomplete. No full G-SCAD pass is a prerequisite for starting this packet. Use deterministic rare topology fixtures plus fresh routine/extended sampling, never a flaky statistical requirement that every run happens to sample every rare class.

Reconcile the #2 direct evidence with scripted evidence against **every row** of the plan's combined geometry obligation table and G-GEOMETRY, with explicit valid and invalid domains. Record exactly one disposition and its evidence:

1. **`hybrid_sufficient`**: bounded direct + scripted families reach all required classes, including non-star-shaped/multiple-hole/disconnected/thin/near-degenerate and layered/3D witnesses, and shrink meaningful failures while retaining required witnesses. State the bounded domain; this is not unrestricted polygon generality.
2. **`bounded_direct_gap_filled`**: implement the measured missing bounded direct family in this packet behind existing #2 `geometry_cases`/`GeometryValueTree` contracts, add independent witnesses/shrink probes and rerun the affected `D-GEOMETRY` tiers plus hybrid gate. Do not edit the earlier packet artifacts or silently change their public shapes. Record the exact gap, added reachable family and measurements.
3. **`general_generator_blocked`**: broad unresolved gaps require a dedicated blocking generator/fix packet with independent predicate/witness/ValueTree/measurement obligations. Author it through the spec-packet workflow using identifiers re-derived then; do not reserve a directory/number now or implement an unreviewed broad redesign inside this packet. Identify the affected downstream charters and keep this packet's combined G-GEOMETRY/closure incomplete until the gap is resolved or the user explicitly approves a scope disposition. Direct work whose requirements are already met continues; do not mark SCAD/scripted requirements complete merely because they are optional features.

## Acceptance criteria

All named targets/tests below are **new planned tests** except prerequisite regressions identified in Verification. `C`, `P`, `X` and log rules are defined there.

- **AC-1** `scripted_feature_isolation_and_union` under `C`/`X` proves actual default/SDK-test/property-only normal/build graphs exclude the engine, while `property_testing_scad` enables native adapter plus property support without changing production/guest graphs. The selected real property target discovers/executes both `D-SCAD-2D` and `D-SCAD-3D`; `--with-scad` now has eligible records and no longer reports the foundation's honest unavailable selector.
- **AC-2** `scripted_emission_and_independent_recipe_references` under `C`, plus both records under `P`, drive typed primitives, explicit non-star-shaped/holed polygon leaves, transforms, union/difference/intersection and extrusion through `emit_scad` and the public adapter. Output regions/analytical bounds/volume match independently derived bounded references, not emitter tags or engine self-comparison. Readable source is self-contained and escaped deterministically.
- **AC-3** `scripted_topology_and_solid_witnesses` under `C` proves every required nonempty contour class, explicit Empty, multiple holes/components, thin/one-unit near-degenerate witnesses, and closed solid/cavity/bridge/overhang/separated-body scenes. Independent geometry observations reject deliberately lost holes/bodies/voids/spans; neither a valid AST nor closed-edge incidence alone certifies them.
- **AC-4** `scripted_value_tree_protocol_and_useful_shrinking` under `C` drives actual `new_tree`/`current`/successful and false `simplify`/repeated `complicate`. Every exposed value remains valid; nonminimal controlled failures demonstrably reduce nodes/vertices/facets and optional components/holes, required witnesses remain, and emitted source still reproduces the retained geometry. No unrestricted vertex-deletion validity or global minimality claim is made.
- **AC-5** `scripted_artifacts_replay_and_concrete_fixture` under `C` verifies pre-evaluation files and complete evaluated artifacts, reproducible explicit-seed sequences, direct retained-case replay with the same source/options/engine identity, and compiled/evaluated concrete geometry/mesh fixture statements with no Proptest context or engine. Failure metadata identifies actual features/profiles/versions/pin, classification and shrink status; no source-adjacent seed file is durable regression history.
- **AC-6** `property_scad_supervision_covers_generation_shrink_and_descendants` under `X`, plus `scripted_abort_and_timeout_keep_pre_execution_case` under `C`, exercise the real executor with a blocked/aborted child before a callback and during a candidate render, and with a live fork descendant. All fail nonzero, retain prepared inputs/native status, terminate and join the process tree on Windows, and do not claim minimization complete. A real bounded adapter-generated property passes under the same supervision path.
- **AC-7** `docs/testing/proptest-scripted-evidence.md` contains actual passing Windows routine/extended selected runs, positive executed-property evidence, measured accepted/rejected/evaluation/shrink behavior, separated compilation/recipe-generation/render/shrink costs and final eligible finite property-profile values. It combines those results with #3's passed adapter-prerequisite evidence to close full G-SCAD; any missing required measurement/profile evidence leaves this criterion and full G-SCAD incomplete. The required `properties-scad-windows` lane executes the delivered registry records and uploads interrupted attempts. No source-inspection assertion is substituted for execution.
- **AC-8** The same evidence records `hybrid_sufficient` or `bounded_direct_gap_filled`, covering every required geometry-table row and G-GEOMETRY proof with direct+scripted sources and useful retained-witness shrinks. A `general_generator_blocked` finding instead leaves this acceptance criterion/closure unmet and names blocking follow-up work; it is not an acceptable green completion disposition.
- **AC-N1** `scripted_rejects_bounds_invalid_output_and_fatal_adapter_diagnostics` under `C` rejects invalid recipe/domain/adapter bounds before sampling and never exposes post-quantization collapsed/crossing/misnested geometry, lost required topology or invalid mesh as a valid case. Real partial/approximate/degraded/unclassified adapter outcomes fail as `ScriptedError::Adapter`/generator failures, not passing discarded SUT input. Existing adapter negative tests still pass.
- **AC-N2** `scripted_generation_and_shrink_exhaustion_are_not_success` under `C` drives the real foundation completion path with fatal errors in `new_tree`/`simplify`, retry/evaluation/candidate limits and artifact-write failure. No latent telemetry error can end in `outcome = "passed"`; rejection/exhaustion is distinct from an assertion failure and retained known failures report `shrink_completed = false` at limits.
- **AC-N3** `scripted_replay_refuses_changed_provenance_and_damaged_artifacts` under `C` detects changed source bytes, recipe/options/bounds, incompatible generator version, engine revision/backend/patch, missing/truncated files and unsafe replay paths. It returns `ReplayMismatch`/`Artifact`, never silently replays current defaults or a different pin.

## Verification

Implementation executes these after prerequisites; authoring does not claim profiles, G-SCAD or G-GEOMETRY passed. Every plain Cargo test uses Bash `set -o pipefail`, an existing `target/` and suffix `2>&1 | tee target/test-output.log`; read/archive before the next invocation. PowerShell preserves native status across `Tee-Object`. Xtask selected runs reuse the combined log/freshness policy, retaining all child output.

- `C`: `cargo test -p slicer-sdk --features property_testing_scad --test property_scripted_contracts -- --nocapture` with that wrapper. Native-risk fixture/replay calls execute in supervised children; no parent-thread timeout approximation.
- `P`: `cargo xtask test --summary --properties --property D-SCAD-2D --property D-SCAD-3D --tier routine --with-scad`; repeat `--tier extended`, archive each run, then replay each retained explicit seed. Run the full incremental `cargo xtask test --summary --properties --tier routine --with-scad` and its extended form after narrow checks pass.
- `X`: `cargo test -p xtask --bin xtask -- property_ --nocapture` with the wrapper; extend the prerequisite's registered `properties` internal tests, not an imagined xtask library target.
- Prerequisite regressions: `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts -- --nocapture`; `cargo test -p slicer-sdk --features property_testing --test property_harness -- --nocapture`, each separately captured. Rerun affected `D-GEOMETRY`/`D-MESH` routine/extended registry selections and `H-IR-ANCHORS` through xtask.
- Adapter regressions: `cargo test -p slicer-scad --features openrscad --test scad_adapter_contracts -- --nocapture` and `cargo test -p slicer-scad --features openrscad --test scad_process_limits -- --nocapture`, separately captured. Multi-crate/guest orchestration must use `cargo xtask test`, not bare broad Cargo.
- `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo check -p slicer-sdk --all-targets --features property_testing_scad`; `cargo clippy -p slicer-sdk --all-targets --features property_testing_scad -- -D warnings`; `cargo check -p slicer-scad --all-targets --features openrscad`.
- `cargo xtask check-literals`; `cargo xtask check-test-quality --report`; `cargo xtask build-guests --check` and required rebuild/recheck on exit `1`, stop on `3`. No workspace test run is required; optional-feature targets need their explicit commands and verified nonzero execution.

## Docs impact

- `docs/05_module_sdk.md` § Test Support (slicer-sdk feature): native-only `property_testing_scad`, typed recipe/witness ownership, real callback isolation versus generation supervision, default-off/guest policy. Check: `rg -n 'property_testing_scad|ScriptedDomain|ScriptedValueTree' docs/05_module_sdk.md`.
- `docs/21_data_defaults_and_fixtures.md` § 5. Fixture policy: scripted source versus concrete-regression lifecycle and FRU-safe bounds. Check: `rg -n 'scripted|format_regression_fixture|RecipeBounds' docs/21_data_defaults_and_fixtures.md`.
- Planned prerequisite `docs/testing/properties.md`: complete `D-SCAD-2D`/`D-SCAD-3D` charters, all plan §5 fields, exact registry bound keys, oracle independence/tolerances, supervision, fresh profiles/replay/artifact/conditional-gap policy. Check: `rg -n 'D-SCAD-2D|D-SCAD-3D|test_process_timeout_ms|ReplayMismatch' docs/testing/properties.md`.
- **New** `docs/testing/proptest-scripted-evidence.md`: measured Windows profiles, real shrink/witness/negative-control traces and combined disposition. Check: `rg -n 'Windows|G-GEOMETRY|hybrid_sufficient|bounded_direct_gap_filled|general_generator_blocked|source_revision' docs/testing/proptest-scripted-evidence.md`.
- Planned prerequisite `docs/scad/windows-feasibility.md`: link scripted measured render/shrink extension without rewriting #3 engine acceptance as an oracle. Check: `rg -n 'proptest-scripted-evidence|scripted' docs/scad/windows-feasibility.md`.

## Open questions

- [FWD] Measure Windows recipe generation/evaluation/rejection/shrink behavior and complete-process/fork containment; select finite routine/extended bounds, defaults, callback timeout and whole-process deadline from evidence. Do not invent budgets or weaken witness coverage to fit a convenient runtime.
- [FWD] Finalize checked arithmetic and dimensionally correct grid/f32/mm³ error envelopes for the bounded recipe reference families; audit actual shared `geo`/Manifold/Clipper implementation paths before claiming oracle independence.
- [FWD] Execute and record the combined G-GEOMETRY disposition after direct+scripted pilots. Fill a measured bounded direct gap under existing public contracts here; if broad generality remains missing, author explicitly blocking follow-up generator work and leave affected coverage/this closure incomplete.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | Existing SDK manifest/module/IR types read; all consumed planned #1/#2/#3 names/shapes checked against their specs; all scripted additions marked planned. |
| 2 | WIT/IR identifiers | PASS | Canonical-unit `Point2`, `ExPolygon.contour/holes`, f32-mm mesh vertices/flat indices unchanged; no WIT edit. |
| 3 | Schema versions | PASS | No IR/WIT bump; foundation registry/context/event schema preserved; new artifact state protocol explicitly local. |
| 4 | ADRs | PASS | ADR-0004/0016/0054 fixture-home/whole-module/locator boundaries inspected and preserved; no new slot. |
| 5 | Deviation IDs | PASS | No mutable ID allocated, reserved, consumed or closed. |
| 6 | Prerequisites | PASS | Both direct and adapter prerequisites explicitly draft; implementation waits, exact planned exports/features agree. |
| 7 | Test targets | PASS | New standalone SDK targets require native scripted feature; xtask private tests registered; prerequisite targets explicitly planned in those packets. |
| 8 | Criteria | PASS | Exact IDs/bounds/errors, actual evaluated witnesses, negative controls/exhaustion/whole-process probes and Windows execution; AC-7 closes full G-SCAD only from combined #3 adapter and #4 generation/shrink/profile evidence; no workspace criterion command. |
| 9 | Fallout | PASS | New bounds/case literal/default sites and unchanged prerequisite schemas tracked in plan; no production struct/version additions. |
| 10 | Docs impact | PASS | Existing canonical SDK/fixture headings plus prerequisite/new workflow/evidence docs have checks. |
| 11 | Completeness | PASS | No unresolved scope question or placeholder; conditional generator outcome has explicit ownership and cannot be mistaken for green closure. |

**Verdict: PREFLIGHT PASS (authoring only; implementation waits for #2/#3; combined G-GEOMETRY and Windows pilots not executed).**

## Deviations

Recorded during implementation on the delivered tree. None changes scope.

1. **Dimensionality mismatch maps to `ScriptedError::Domain`.** The planned
   `ScriptedError` variants cannot construct a `slicer_scad::ScadError` (its
   constructor is crate-private), so a recipe whose converted dimensionality
   contradicts its recipe variant is reported as
   `Domain(DomainError::InvalidInput)` and treated as a generator/domain failure.
   Adapter rejections still surface as `ScriptedError::Adapter`.
2. **Adapter AST/geometry capacity is derived with a source-byte floor.** The
   structural estimate alone is not a bound: a polygon leaf contributes one AST
   node per vertex, and a holed leaf expands to extra evaluated CSG nodes.
   Measured while implementing: a 96-vertex leaf needed 208 AST nodes against an
   `8 * 24 + 16 = 208` structural estimate, and a holed leaf needed 28 evaluated
   CSG nodes against `max_nodes = 24`. `required_adapter_capacity` therefore uses
   `max(8 * max_nodes + 16, max_source_bytes)` for both AST nodes and geometry
   nodes, and `2 * max_depth + 8` for AST depth. `scripted_cases` and
   `evaluate_recipe` reject a smaller adapter capacity before any render.
3. **Attempt journaling happens for every candidate, not only failures.** The
   pre-execution input set must exist before the adapter call, so a passing run
   also keeps an attempt directory per evaluated candidate. With
   `isolation = "fork"` the controller and the worker each generate the certified
   tree, so the measured runs record two attempt directories per accepted case
   (48 routine, 192 extended per record); both record `stage = "evaluated"`.
4. **The foundation's `no_eligible_scad_properties` selector test changed
   premise.** That assertion was only true while no `requires_scad` record
   existed, and this packet deliberately makes `--with-scad` eligible. The test
   now asserts the honest refusal against a SCAD-free registry (including an
   empty one) and adds a positive control that the live registry selects exactly
   `D-SCAD-2D` and `D-SCAD-3D`.
5. **Windows process-tree containment uses `taskkill /T /F`.** No new dependency
   was added to `xtask`. The observable guarantee — a live descendant PID is gone
   after the deadline kill — is tested rather than assumed.
6. **Delegated geometry shrinking is undone on rejection.**
   `ScriptedValueTree` reuses `GeometryValueTree` for explicit polygon leaves and
   only calls its `simplify` while the current recipe is tree-derived. A
   delegated step whose candidate fails certification is undone with
   `complicate`, so the delegated tree never drifts from the certified value.
   Boolean/CSG simplifications then run only from a value the tree can still
   explain.
7. **Registered properties select among per-class strategies.** A single
   `ScriptedDomain` cannot reach every planar class, so `property_scripted` builds
   one strategy per class/witness and combines them with `prop_oneof!`. The SDK
   surface stays `scripted_cases(domain, options)` as planned; the union lives in
   the registry test.
8. **No extra public helpers were added to produce `ConcreteInput`.** The planned
   export block is implemented exactly; tests build `ConcreteInput::Geometry` /
   `ConcreteInput::Mesh` from the certified output themselves.
9. **`docs/testing/properties.md` carries the two charters and the supervision
   rule.** It is the planned prerequisite document; the `D-SCAD-2D` / `D-SCAD-3D`
   charters, the exact bound keys and the positive
   `test_process_timeout_ms` requirement are documented there, and the measured
   evidence is in the new `docs/testing/proptest-scripted-evidence.md`.

Combined `G-GEOMETRY` disposition: **`hybrid_sufficient`** for the stated bounded
domain. Full `G-SCAD` is recorded as passed by combining the adapter-prerequisite
evidence in `docs/scad/windows-feasibility.md` with the generation, render and
shrink measurements and the eligible profiles in
`docs/testing/proptest-scripted-evidence.md`.

10. **Supervision fault-injection seam.** `PNP_SCRIPTED_PAUSE_BEFORE_RENDER`
    pauses inside `evaluate_certified` between the complete, readable input set
    and the adapter call. It lives in the test-only
    `test_support::property_testing::scripted` module (no guest or product graph
    enables that module), is default-off, and only the abort fixture in
    `property_scripted_contracts` sets it. It is how a kill is proven to retain a
    real `prepared` attempt rather than a fabricated `evaluated` one.
11. **Windows containment is a Job Object plus `taskkill /T`.** The supervisor
    creates a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and assigns the
    execution child to it, so a descendant left behind by an abnormal controller
    exit is terminated when the job handle closes; the deadline path additionally
    runs `taskkill /T /F`. `windows-sys` 0.59 was already in the workspace lock,
    so this only adds an `xtask` dependency edge. Assignment can be refused when
    the runner already holds a non-nestable job; the deadline kill remains the
    fallback and the job handle is still the abnormal-exit guarantee. Containment
    is established **before execution is accepted**: a failed job creation or
    assignment refuses to run the child, and the deadline path kills the direct
    child on every platform before waiting so a blocked fixture cannot outlive its
    deadline. Both paths are tested, including a fixture that exits `9` with a
    live descendant.
12. **Solid domains accept only convex rectangular constituent profiles.**
    `geometry` on a solid domain describes the bounded constituent planar
    domain. The bounded solid families build convex rectangular profiles, so a
    request for another constituent topology (or `min_holes > 0`,
    `min_components > 1`) is rejected as `DomainError::UnsupportedDomain` before
    sampling instead of being silently ignored. Both registry records request
    `TopologyClass::Convex`.
13. **Shrink rejection memory is keyed by the transition.** A rejected
    `(base, candidate)` pair is never re-proposed from the same base, and a
    `complicate` records the just-undone transition as rejected. Without the
    second half, a candidate that makes the property pass was re-accepted and
    re-complicated forever, retaining the original non-minimal value; the
    committed selective-failure regression now proves the search continues with
    the other dimensions.
14. **Recipe parameters span 0..4096.** The earlier 0..64 range never reached the
    template jitter spans, so every generated dimension sat in its first few
    units. The wider range is what makes dimension shrinking meaningful; the
    reachability and selective-failure tests depend on it.
15. **Attempt journaling is thread-local, not environment-global.** `PropertyRun`
    publishes its artifact directory to the scripted generator through a
    thread-local slot; `prepare_attempt` no longer reads
    `PNP_PROPERTY_CONTEXT`. A process-global read let one contract test's blocked
    attempts path corrupt a sibling test that happened to run concurrently. The
    context environment variable is still read by `PropertyRun::new` itself.
16. **Native-risk contract bodies run in supervised children.** The artifact,
    concrete-fixture and replay-refusal bodies each execute in a child of the
    test binary with a deadline kill, so no engine call happens on the parent
    test thread.
17. **The render-pause probe is phase-selectable.** `PNP_SCRIPTED_PAUSE_AFTER=<n>`
    pauses on the n-th candidate evaluation of the thread
    (`PNP_SCRIPTED_PAUSE_BEFORE_RENDER` is the n = 1 form), so the supervision
    test kills the child once before any callback and once during a shrink
    candidate render. Both killed runs retain a `prepared` attempt.
## Closure review status

Four independent closure reviews were run. Every substantive finding was fixed
and re-verified:

- shrink progress after a passing candidate (rejection memory keyed by
  transition, plus a committed selective-failure regression);
- fatal shrink errors retaining incomplete status;
- converted-mesh witness loss (occupancy probes plus surface connectivity, with
  a converted-beam counterexample);
- ray casting shared-edge double counting;
- cylinder regular-polygon extrema (odd facet counts);
- generator reachability of every advertised CSG/solid family;
- prepared-only abort retention and kills at both generation and shrink-render
  phases;
- proposal/evaluation limits and their failure reporting;
- the named AC-N1 test, constructor capacity/limit validation, and the solid
  constituent-domain restriction;
- `failure.json` attempt links and the `--with-scad` reproduction command;
- the CI log-ownership collision;
- Windows Job Object containment (Windows-only, fail-closed, with abnormal-exit
  and deadline tests) and the non-Windows deadline path;
- the contract-test process-global context race (attempt journaling is
  thread-local);
- supervised children for the native-risk fixture/replay bodies.

One witness remains **unverified on this machine**: creating a Windows directory
junction or file symlink is refused by the workspace volume ("Local volumes are
required", "The specified path is invalid"). The refusal itself is implemented
(symlink metadata plus canonical containment) and the probe reports the
environment limitation instead of silently passing. The final review recorded
that item plus the completed fixes; the delivered tree's executable gates (SDK
contracts, reference/fatal-shrink units, xtask property tests, workspace
clippy/build/check, literals, test-quality, guest freshness, and the full
incremental routine and extended `--with-scad` runs) are green.

The packet keeps `status: implemented` because the implementation and all
executable acceptance evidence are delivered; closure review approval is
recorded here rather than implied.
