---
packet: proptest-openrscad_03_scad-adapter
status: implemented
---

# Optional exact SCAD adapter with Windows feasibility proof

## Goal

Provide an engine-optional, runtime/SDK-independent `slicer-scad` library that evaluates self-contained SCAD with expression overrides and a budget, rejects untrusted diagnostic outcomes, and returns checked PnP contours or an indexed mesh, proved on Windows.

## Why

The approved input is `docs/specs/proptest-openrscad-adoption.md` §§4, 7–8 and G-SCAD. Product ingestion and scripted property inputs must use the same strict boundary, not separate evaluator/conversion implementations. This packet establishes that boundary and feasibility evidence; it does not route CLI input or claim complete OpenSCAD language parity.

Source inspection finds integration gaps that a mesh-returned-success check would hide: the inspected upstream budget entry lacks overrides, constant-expression evaluation is unbudgeted, and the contour entry discards its collected diagnostic vectors. These are implementation obligations for a selected immutable revision, not permission to omit the budget or accept partial/approximate geometry.

## Scope

- In: new optional adapter crate/workspace member; immutable OpenRSCAD dependency selection; focused pinned upstream/fork patches when required; source/parameter validation; exact evaluation/rendering; deny-all resource boundary; strict diagnostic classification; checked 2D/3D conversion and explicit 2D extrusion; evaluation-local state/cache; bounded Windows contract/process tests, CI lane, provenance and measured adapter-prerequisite evidence for G-SCAD.
- Out: Proptest, SDK helpers, runtime dependencies, model-I/O/CLI routing, `ObjectMesh`/`MeshIR` construction, object IDs, bed placement, material/color assignment, persistent caching, preview/fallback modes, broad language fuzzing, general mesh repair, recipe generation/shrink measurements, final eligible property profiles, full G-SCAD closure, and an enforceable in-process native render deadline. No IR/WIT/schema version change or OrcaSlicer port is required.

## Depends on

None. This independent source-baseline branch does not wait for the property foundation. Engine pinning/build/compatibility execution remains implementation work; none is claimed complete here.

## Design notes

### Grounded ownership and upstream evidence

- Existing `Point2`, `Polygon`, `ExPolygon`, `Point3` and `IndexedTriangleSet` (`crates/slicer-ir/src/slice_ir.rs`) are the output types. Mesh indices are a flat `Vec<u32>` triangle stream. Outer contours are CCW, holes CW. `UNITS_PER_MM` is the authoritative f64 conversion constant; `mm_to_units` accepts f32 and uses a saturating cast, so it cannot itself validate an arbitrary upstream f64.
- `assemble_object` (`crates/slicer-model-io/src/loader.rs`) remains the assembly owner under `docs/02_ir_schemas.md` § `ObjectMesh` Assembly Contract. The adapter returns geometry only. Host mesh ownership in `docs/01_system_architecture.md` § Data Ownership Rules is unchanged.
- The following primary sources were inspected during authoring. Links to `main` are **navigation, not pins**; recheck these symbols and their complete call paths against the selected revision before depending on them:

| Source | Observed seam / consequence |
|---|---|
| [syntax `lib.rs`](https://github.com/matthova/openrscad/blob/main/crates/openrscad-syntax/src/lib.rs), `parse` | Parses source to a spanned program. |
| [eval `lib.rs`](https://github.com/matthova/openrscad/blob/main/crates/openrscad-eval/src/lib.rs), `eval_const_expr`, `eval_program_with_budget`, `eval_program_with_mode`, private `eval_program_impl` | Expressions use unlimited fuel in the inspected implementation. Budget and override/exact-mode entrypoints are separate; the combined implementation is private. No public combined API is assumed. |
| Same file, `FileResolver::{load,load_bytes}`, `Interp::eval_stmts_core`, `Interp::report_recursion` | Missing resources may become warnings. Root recursion can retain earlier geometry while returning `Ok(EvalOutput)` with a warning. Reject actual resolver requests and partial results independently of a nonempty mesh. |
| [geom `lib.rs`](https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/src/lib.rs), `render_cached_diag`, `render_contours_cached`, `is_2d` | Mesh exact rendering returns diagnostic vectors; the inspected contour entry collects but discards them. `is_2d` uses `any` for mixed containers and is not a sufficient mixed-dimension validator. |
| Same file, `RenderDiagnostics`, `boolean_or_fallback`, `render_preview_cached_diag` | Errors can accompany nonempty fallback meshes; warnings can describe approximation. Exactness is not an `errors.is_empty()` predicate. Never select the preview-concatenation entry. |
| [mesh `mesh.rs`](https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/src/mesh.rs), `Mesh` | Upstream fields are `verts: Vec<[f64; 3]>`, `tris: Vec<[u32; 3]>`; these types stay private to the adapter. |
| [geom manifest](https://github.com/matthova/openrscad/blob/main/crates/openrscad-geom/Cargo.toml), native dependency table | Native `manifold-csg` is unconditional inside the inspected geometry crate, even though Rust kernels also exist. Optionalize the entire adapter engine edge, not merely its caller. |
| [COMPAT.md](https://github.com/matthova/openrscad/blob/main/COMPAT.md), warned divergences | Concave/holed `roof()` can omit geometry; concave-leaf 3D Minkowski can approximate. Keep real negative fixtures tied to the adopted revision. This is not a claim that every SCAD construct is certified compatible. |
| [release workflow](https://github.com/matthova/openrscad/blob/main/.github/workflows/release.yml), Windows matrix and CMake setup | An upstream Windows build path exists; this project's Windows integration must still execute. |

Implementation selects a real compatible `matthova/openrscad` commit and records it in Cargo `rev` entries/lock resolution, or a documented immutable fork revision with its upstream base and patch digest. All four engine packages resolve coherently. Prefer contributing patches upstream; do not depend on moving branches or invent an SHA. The inspected upstream manifest declares Rust 1.85; the current root package MSRV and checked-in toolchain are distinct policies (`Cargo.toml`, `rust-toolchain.toml`). Verify selected sources and dependencies under the actual supported configuration; do not change the repository toolchain to obtain an unnoticed pass.

### Planned public adapter contract

**Every name and path introduced in the remainder of this section is new/planned.** Crate `slicer-scad`, library path `slicer_scad`, definitions/re-exports in `crates/slicer-scad/src/lib.rs`; private engine code may live in `engine.rs`, diagnostic policy in `diagnostics.rs`, conversion in `conversion.rs`. No upstream implementation type is public.

```rust
pub struct ScadSource { pub label: String, pub text: String }
pub struct ScadParameter { pub name: String, pub expression: String }
pub struct ScadLimits {
    pub evaluation_steps: u64,
    pub max_source_bytes: usize,
    pub max_ast_nodes: usize,
    pub max_ast_depth: usize,
    pub max_geometry_nodes: usize,
    pub max_facets: u32,
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_abs_coordinate_mm: f64,
    pub worker_stack_bytes: usize,
}
pub struct ScadEvalOptions {
    pub parameters: Vec<ScadParameter>,
    pub extrude_height_mm: Option<f64>,
    pub limits: ScadLimits,
    pub random_seed: Option<u64>,
}
pub enum ScadGeometry {
    Empty,
    Contours(Vec<slicer_ir::ExPolygon>),
    Mesh(slicer_ir::IndexedTriangleSet),
}
pub enum ScadDiagnosticStage { Parse, Parameter, Evaluation, Render, Conversion }
pub enum ScadDiagnosticClass { Information, Error, Partial, Degraded, Approximate, Unknown }
pub struct ScadDiagnostic {
    pub stage: ScadDiagnosticStage,
    pub class: ScadDiagnosticClass,
    pub code: String,
    pub message: String,
    pub span: Option<std::ops::Range<usize>>,
}
pub struct ScadEngineInfo {
    pub revision: String,
    pub patch_digest: Option<String>,
    pub backend: String,
}
pub struct ScadProvenance {
    pub engine: ScadEngineInfo,
    pub source_label: String,
    pub source_digest: String,
    pub parameters: Vec<ScadParameter>,
    pub random_seed: Option<u64>,
}
pub struct ScadEvalOutput {
    pub geometry: ScadGeometry,
    pub diagnostics: Vec<ScadDiagnostic>,
    pub provenance: ScadProvenance,
}
pub enum ScadErrorKind {
    BackendDisabled, InvalidOptions, Parse, InvalidParameter,
    EvaluationBudgetExceeded, Evaluation, ExternalResourceDenied,
    PartialOutput, DegradedOutput, ApproximateOutput, UnknownDiagnostic,
    MixedDimensions, ExtrusionHeightOn3d, NonFiniteCoordinate,
    CoordinateOutOfRange, InvalidContours, InvalidMesh, ComplexityLimit,
    WorkerFailure,
}
pub struct ScadError {
    pub kind: ScadErrorKind,
    pub source_label: String,
    pub parameter: Option<String>,
    pub message: String,
    pub diagnostics: Vec<ScadDiagnostic>,
}
pub fn engine_info() -> Option<ScadEngineInfo>;
pub fn evaluate(source: &ScadSource, options: &ScadEvalOptions)
    -> Result<ScadEvalOutput, ScadError>;
```

All public data types implement `Debug + Clone + serde::Serialize + serde::Deserialize`; `ScadError` additionally implements `Display + std::error::Error`. These are data serialization derives, not a geometry wire/schema version change. `ScadLimits` and `ScadEvalOptions` implement documented bounded `Default` for fixture/FRU use; defaults are chosen and recorded during Windows pilots, never unlimited. Defaults have empty parameters, no extrusion and no forced random seed. Production callers may supply limits explicitly; serializing/deserializing an options record never bypasses `evaluate` validation. Other records need no fabricated default. New watched literal sites use FRU or a justified exhaustive waiver.

- Add `default = []` and `openrscad` to the new crate's manifest. Only `openrscad` activates optional native engine dependencies. Core public types, `engine_info` and `evaluate` compile without the feature; `engine_info()` is `None` and `evaluate` returns `BackendDisabled` before attempting parsing/resource access. Enabled Windows initially selects native `ManifoldKernel` with provenance `backend = "manifold_cpp"`; there is no runtime preview/backend selector. Unsupported platforms are not acceptance claims. Do not add an adapter dev-dependency on SDK or Proptest.
- `source.label` is diagnostic data, never a path to load or a resource base directory. `source_digest` is lowercase SHA-256 hex of source UTF-8 bytes. `ScadProvenance.parameters` preserves the ordered raw input override list, including duplicates; effective last-write-wins values are derived from it. `ScadEngineInfo.revision` is the actual immutable engine/fork revision; `patch_digest` is lowercase SHA-256 of an ordered applied patch set when present, otherwise `None`. Record backend and engine information **before** dangerous test execution using `engine_info`.
- Identifiers are case-sensitive SCAD identifiers, including supported `$` specials, validated using the adopted grammar. Do not normalize them or put them in slicer config. Every expression is parsed as exactly one expression, cannot inject extra statements, and is evaluated in the engine's constant/base scope, not as a string, JSON value or arbitrary script concatenation. Process overrides in input order; the last occurrence of a name wins. Evaluate each supplied expression under the shared budget, including overwritten occurrences; record the failed parameter name. A later parameter does not implicitly see earlier override bindings. Accept nested vectors, booleans, strings and arithmetic supported by the selected evaluator. Undefined/malformed/unsupported parameter outcomes cannot silently turn into empty geometry.
- `max_source_bytes` bounds the checked total UTF-8 byte length of source plus all override names/expressions; AST node/depth capacities apply across the program and parsed override expressions. Many small overrides cannot evade aggregate size/fuel limits. `ScadDiagnostic.span` indexes source text except at `stage = Parameter`, where it indexes the failing expression identified by `ScadError.parameter`; unavailable spans are `None`.
- Exact evaluation seeds `$preview = false`; explicit source or parameter `$preview` follows SCAD semantics and may choose a different model, but **never** selects the preview renderer. Add combined override/budget/mode support and budgeted constant-expression support to the pinned upstream when absent. One finite `evaluation_steps` allowance covers override evaluation plus program evaluation; resetting the allowance per override is forbidden. Zero/unlimited sentinels, invalid capacities, nonfinite/nonpositive coordinate bounds or an unsafe stack setting yield `InvalidOptions`. Budget exhaustion has its own `EvaluationBudgetExceeded` classification, including when it happens in a parameter.
- `extrude_height_mm = None`: a purely 2D result returns `Contours`; a purely 3D result returns `Mesh`. `Some(h)` requires finite, strictly positive h within declared bounds; extrude a 2D result from Z=0 to Z=h via the same exact engine boundary and return a checked `Mesh`. A 3D result with height returns `ExtrusionHeightOn3d`; mixed nonempty dimensions return `MixedDimensions`. Classify evaluated dimensional structure correctly, including wrappers and excluded background geometry; do not trust upstream `is_2d` alone. Empty results return `Empty` (also with a valid explicit height); future import/slice callers must reject them as empty models, while an explicitly empty property domain may use them. Invalid height fails before rendering, never silently becomes `Empty`.

### Resource, state and diagnostic policy

Implement a production deny-all resolver that records every `load`/`load_bytes` request, returns no content, and makes the overall evaluation fail with `ExternalResourceDenied`, even if upstream only warns and retains a sibling cube. Audit all native resource seams at the pin: includes/use, mesh/vector imports, surface/dxf functions, font discovery and any direct filesystem/network bypass. Do not register system fonts or start background discovery. Only bundled/in-memory font data may be used; patch a bypass or reject the unsupported operation before it can access a resource. Actual attempted requests are rejected; an unreachable disabled branch need not be rejected merely because its text contains `import`. No caller-supplied resolver and no working-directory fallback exist in this API.

Create all evaluator/kernel/cache state per `evaluate`, on a joined worker with the validated `worker_stack_bytes`; parse and parameter evaluation also run under that stack policy. No detached timed-out workers, process-global per-case panic-hook churn, persistent geometry cache or cross-case random state. Verify explicit seeded programs and `random_seed = Some(s)` repeat independently of prior calls/concurrent callers; add a focused seed/reset seam if required. Unseeded programs are not promised deterministic random output. Audit font/global initialization for races without inventing a new shared cache.

All diagnostic-producing paths, **including contour projection/lowering and optional extrusion**, feed one fail-closed classifier. If the selected pin cannot return those diagnostics, add a focused upstream diagnostic-returning entrypoint; rendering the same tree again through a different path is not proof the contour path was clean. Success contains only `Information` diagnostics. Initial planned informational codes are `scad_echo` for evaluated echoes and `assignment_overwritten` for the verified overwritten-assignment lint; match the latter only after pin-specific semantic/source verification. No generic substring such as `warning` or empty-error-list rule is an allowlist. Store the revision-bound mapping, origin/function and real reproducer for each admitted warning in **new** `docs/scad/openrscad-compatibility.md`.

| Observed outcome | Required `ScadErrorKind` / behavior |
|---|---|
| Syntax error | `Parse`, source label and byte span retained. |
| Invalid expression/name | `InvalidParameter`, offending `parameter` retained; budget exhaustion remains distinct. |
| Program eval error / exhausted fuel | `Evaluation` / `EvaluationBudgetExceeded`. |
| Any actual external request | `ExternalResourceDenied`, even with nonempty remaining output. |
| Recursion-aborted or otherwise partial output | `PartialOutput`, no usable geometry returned. |
| Render fallback/recoverable geometry error | `DegradedOutput`, even with a nonempty mesh. |
| Approximation or geometry-skipping warning | `ApproximateOutput`, even when other geometry remains. |
| Unknown warning, severity or diagnostic channel | `UnknownDiagnostic`, until deliberately classified at the pin. |
| Recognized information | Preserve stage/code/message/span and continue. |

Retain all available diagnostics on failures; deterministic primary-error precedence is external request, partial/degraded/approximate/unknown outcome, then conversion. A fatal parse/parameter/evaluation failure ends before render. An output cannot become acceptable because geometry conversion subsequently succeeds.

### Checked conversion, not repair

<!-- snippet: coord-system -->
- Units: 1 unit = 100 nm (10⁻⁴ mm), not OrcaSlicer's 1 nm; divide OrcaSlicer constants by 100 and convert with `Point2::from_mm` / `mm_to_units()` (`docs/08_coordinate_system.md`).

For this external **f64** boundary, use the same normative half-away-from-zero policy directly with `slicer_ir::UNITS_PER_MM`: check finite input, configured coordinate bound, finite scaled product and rounded result in the half-open representable i64 range **before** casting. Do not narrow through f32 or accept saturation. Record the f64 multiplication/rounding error separately from integer half-unit quantization; do not claim a universal exact float round trip. Document this checked external-f64 use of the existing policy in `docs/08_coordinate_system.md`, without changing existing f32 helpers.

Validate raw contour rings, reconstruct an even-odd nesting forest independently of input order/winding, then validate again after quantization. Even depths become CCW outer components; odd depths become CW holes of their immediate containing outer; nested islands become separate components. A redundant exactly equal terminal closure point may be removed as representation normalization; do not drop quantization-collapsed vertices, zero-area rings, crossing/touching boundaries, changed nesting/contacts or lost components/holes to rescue a result. Use checked/exact predicates or prove finite intermediate bounds; unrestricted i64 geometry is not automatically safe in i128. Contour validation lives in the adapter, not SDK, Clipper/boolean self-union or a dependency on runtime/core. Tests use independently worked integer/rational/nesting fixtures, not the production grouping routine as their oracle.

For mesh output check f64 finite/range values and source indices before access, output cardinalities and checked triangle flattening, then checked f64→f32 conversion into `Point3` mm coordinates. Apply the IR's documented subnormal boundary policy explicitly; reject if narrowing/normalization creates degeneracy, orientation inversion or other invalidity. Check triangle distinctness/nonzero area, finite intermediates, and closed consistently oriented edge incidence by exact-position identity, with analytically justified bounds/volume for designated fixtures. Exact coincident vertex deduplication is allowed only as representation normalization with the triangle mapping preserved; no epsilon welding, face deletion, hole filling, auto-repair or fallback tessellation. Arbitrary 3D self-intersection freedom is not claimed by edge-incidence checks. Preserve disconnected bodies within one indexed mesh and do not expose color groups/materials.

Enforce AST/source/node/depth/facet bounds before expensive traversal/render allocation where possible, output count/coordinate bounds before conversion allocation, and explicit extrusion growth bounds. These limits and evaluation fuel do **not** interrupt synchronous native rendering or promise bounded native memory/time. Tests needing deadlines/abort containment supervise a killable child process covering parse/evaluation/render and join/kill it on Windows; a thread timeout or `catch_unwind` is insufficient.

### Windows evidence and dependency gates

**Gate ownership/order:** this packet proves the adapter-prerequisite portion of G-SCAD: Windows build, evaluation/exact render, checked conversion, diagnostics, reproducibility, process feasibility and representative adapter evaluation/render costs. It does not close full G-SCAD or select final eligible property profiles. `docs/spec_packets/proptest-openrscad_04_pbt-scripted/` consumes that adapter prerequisite, measures recipe generation/shrink costs, finalizes eligible property profiles and records full G-SCAD evidence together with the combined G-GEOMETRY decision. This packet may close after its own acceptance criteria pass; it does not wait for #4. Full G-SCAD remains incomplete until #4 supplies its remaining obligations.

Create **new** `docs/scad/windows-feasibility.md` with the adapter-prerequisite disposition and explicitly pending #4 obligations, actual engine/fork/base/patch identity, selected native backend, toolchain/target/MSRV check, CMake/MSVC identities, commands and exit statuses, revision/dirty content provenance, `CARGO_TARGET_DIR`, build profile/features and full logs. Measure representative adapter evaluation and exact-render costs for the holed-2D and indexed-3D fixtures in AC-3, separately from build costs; derive bounded adapter defaults and contract-test settings, not property profiles. Record limitations honestly; a failed adapter prerequisite leaves dependent SCAD/scripted adoption incomplete, while direct property work may continue.

Add **new** `jobs.scad-adapter-windows` in `.github/workflows/ci.yml`, on `windows-latest`, with the checked-in Rust policy plus CMake/MSVC prerequisites. Run feature-off and enabled adapter contract/process checks; preserve logs and repro files under `target/scad-adapter/` on failure. Do not depend on the foundation job existing. Later ingestion/scripted packets extend required SCAD Windows coverage rather than deferring all wiring to final rollout.

Retain real Cargo metadata/tree evidence for default workspace, default adapter, enabled adapter, default CLI and discovered guest normal/build graphs. A default workspace build must not compile engine/native dependencies merely because the adapter is a member; adapter normal/build graph includes neither SDK/runtime nor Proptest, even with `openrscad`. Inspect normal/build edges rather than confusing test feature unification with a product dependency. SCAD-enabled CLI graph proof belongs to the ingestion packet once that feature exists. No SDK/IR source edit is planned here; if implementation unexpectedly edits guest inputs, apply the guest freshness discipline before interpreting guest failures.

## Acceptance criteria

All test targets/names below are **new planned tests**; `F`, `A`, `U` and `L` are defined in Verification. Tests assert actual outputs/classifications and nonempty populations where needed.

- **AC-1** `scad_backend_feature_contract` under `F` checks `engine_info().is_none()` and `BackendDisabled`; enabled under `A` it observes the actual pinned `revision`, `patch_digest` and `backend = "manifold_cpp"`, and a nonempty cube mesh. Recorded Cargo graphs prove the normal/build isolation described above; metadata-driven graph assertions do not merely grep manifest source.
- **AC-2** `scad_parameters_budget_and_exact_mode` under `A` exercises arithmetic, nested vectors, duplicate last-wins overrides, case-sensitive and `$` names, source diagnostics and `$preview` branch selection. A finite shared allowance covers both parameter and main evaluation; a runaway expression and a nested-loop program each produce `EvaluationBudgetExceeded`. Exact union geometry has analytic bounds/volume and no internal preview-concatenation surfaces, including when `$preview=true` is explicitly supplied.
- **AC-3** `scad_even_odd_contours_and_explicit_extrusion` under `A` and `scad_conversion_contour_forest` under `U` prove a holed profile, shuffled/reversed raw contours, multiple holes, disjoint outers, nested islands and deeper nesting against independently enumerated expected regions/area/winding. No implicit thickness: None returns contours; positive explicit height returns a closed mesh with dimension-derived Z bounds/volume. Pure 3D returns a checked flat-index mesh preserving separated bodies.
- **AC-4** `scad_informational_diagnostics_and_state_isolation` under `A` observes actual `scad_echo`/verified `assignment_overwritten` diagnostics, and repeat seeded evaluation before/after another source and concurrent calls. Geometry/provenance and accepted diagnostics satisfy the declared repeatability, without retaining prior resolver/cache/random state. Font fixtures use bundled data only.
- **AC-5** `scad_process_supervision_retains_repro_and_joins` under `L` proves real adapter execution in a supervised Windows child, plus controlled hard-deadline and abnormal-abort child probes: nonzero outcome, no surviving worker, source/options/engine identity saved before execution, and timeout/abort versus evaluator-budget classifications distinct. Worker stack policy is tested against a non-tail-recursion fixture on the selected Windows backend; no in-process render deadline is advertised.
- **AC-6** `docs/scad/windows-feasibility.md` records a passing actual Windows feature-off/enabled build and all narrow tests, immutable dependency/patch lock provenance, no-engine default-build evidence, safe measured adapter defaults and representative evaluation/exact-render measurements for AC-3's holed-2D/indexed-3D fixtures, separate from build measurements. It records only the adapter-prerequisite portion of G-SCAD as passed and lists recipe generation/shrink costs, final eligible property profiles and full G-SCAD closure as pending #4 obligations. The required `scad-adapter-windows` PR lane and artifact upload execute. Upstream CI or source inspection alone does not satisfy this adapter prerequisite.
- **AC-N1** `scad_rejects_resources_even_with_remaining_geometry` under `A` drives real include/use, import/surface/dxf and native font-resource seams applicable at the pin, with a valid sibling primitive and existing tempting files in different working directories. It returns `ExternalResourceDenied`, never accesses those resources and never returns sibling geometry as success; disabled/unreached requests do not become spurious text-scan errors.
- **AC-N2** `scad_rejects_partial_approximate_degraded_and_unknown` under `A`, plus `scad_diagnostic_channels_fail_closed` under `U`, reject nonempty root-recursion partial output, concave/holed roof omission, concave-leaf Minkowski approximation, recoverable backend fallback and an injected unknown diagnostic. Exercise real selected-pin reproducers where available and production classifier injection for otherwise unreachable channel variants. Both mesh **and contour/projection** paths propagate diagnostic information; a deliberately failed projected 3D child cannot be accepted as clean 2D contours. No `errors.is_empty()` shortcut or blanket warning allowlist passes.
- **AC-N3** `scad_rejects_conversion_damage` under `U` checks NaN/infinity, configured/range overflow, i64 cast boundaries, quantization-collapsed edges/holes, changed contact/nesting, bad/truncated indices, f32 overflow/subnormal-induced collapse, degenerate/inverted triangles and invalid closed-edge incidence. Expected errors are `NonFiniteCoordinate`, `CoordinateOutOfRange`, `InvalidContours` or `InvalidMesh` as appropriate, never repaired success. Run public-path small post-quantization failure witnesses under `A` as well.
- **AC-N4** `scad_rejects_invalid_options_and_dimensions` under `A` checks malformed/statement-injecting parameters (`InvalidParameter`), invalid limits/height (`InvalidOptions`), height on 3D (`ExtrusionHeightOn3d`), mixed dimensions (`MixedDimensions`), and exceeded source/AST/node/facet/output bounds (`ComplexityLimit`). Empty results are explicitly `Empty`, not fabricated thickness or a successful import claim.

## Verification

Authoring is read-only source/document inspection plus packet creation, not a G-SCAD execution pass. Every plain Cargo test below uses Bash `set -o pipefail`, an existing `target/` and suffix `2>&1 | tee target/test-output.log`; read/archive the log before the next invocation. PowerShell must preserve `$LASTEXITCODE` across `Tee-Object`. CI archives each narrow run before the next overwrites the canonical log.

- `F`: `cargo test -p slicer-scad --no-default-features --test scad_feature_contract -- --nocapture` with the wrapper. This standalone target is ungated and asserts the appropriate feature state in both configurations; no feature-off zero-test binary counts as proof.
- `A`: `cargo test -p slicer-scad --features openrscad --test scad_adapter_contracts -- --nocapture` with the wrapper; also run `scad_feature_contract` with `--features openrscad`.
- `U`: `cargo test -p slicer-scad --features openrscad --lib -- scad_ --nocapture` with the wrapper. Register private conversion/classifier test modules in `crates/slicer-scad/src/lib.rs` or their actual owning modules.
- `L`: `cargo test -p slicer-scad --features openrscad --test scad_process_limits -- --nocapture` with the wrapper. `scad_adapter_contracts` and `scad_process_limits` are explicit standalone manifest targets with `required-features = ["openrscad"]`. Dangerous fixtures execute in supervised children, not the parent test thread. Verify actual named test discovery/execution; no silent feature/fixture skip.
- `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo build -p slicer-scad --features openrscad`; `cargo check -p slicer-scad --all-targets --features openrscad`; `cargo clippy -p slicer-scad --all-targets --features openrscad -- -D warnings`.
- `cargo tree -p slicer-scad --no-default-features -e normal,build,features`; repeat with `--features openrscad`; retain default CLI/workspace and discovered guest graph evidence in the feasibility record.
- `cargo xtask check-literals`; `cargo xtask check-test-quality --report`. No workspace test run is required. Any multi-crate/guest test run uses `cargo xtask test` and actual guest freshness exit handling, not a bare broad Cargo test.

## Docs impact

- **New** `docs/scad/adapter.md`, public API/feature/resource/diagnostic/conversion/limit contract and examples of contours versus extrusion. Check: `rg -n 'ScadEvalOptions|BackendDisabled|ExternalResourceDenied|Contours|deadline' docs/scad/adapter.md`.
- **New** `docs/scad/openrscad-compatibility.md`, immutable engine/patch provenance, complete diagnostic-channel audit, warning classification table and real reproducers. Check: `rg -n 'revision|patch|render_contours|PartialOutput|UnknownDiagnostic' docs/scad/openrscad-compatibility.md`.
- **New** `docs/scad/windows-feasibility.md`, G-SCAD adapter-prerequisite Windows toolchain/build/execute/process/measurement evidence and explicit pending scripted/profile obligations. Check: `rg -n 'G-SCAD|Windows|source_revision|CMake|MSVC|measurement|pending|proptest-scripted-evidence' docs/scad/windows-feasibility.md`.
- `docs/08_coordinate_system.md` § Conversion & Determinism (Normative): checked external-f64 conversion and post-narrowing validation, unchanged canonical unit/rounding rule. Check: `rg -n 'external.*f64|saturat|post.*quant' docs/08_coordinate_system.md`.
- `docs/01_system_architecture.md` § Data Ownership Rules: link the new optional geometry-only ingress boundary; retain host assembly ownership. Check: `rg -n 'slicer-scad|scad/adapter' docs/01_system_architecture.md`.

## Open questions

- [FWD] Select/verify/pin the actual compatible OpenRSCAD revision, coherent engine dependencies, native Manifold backend and any immutable fork/patch provenance; record Windows build and supported Rust/MSRV evidence. No speculative SHA or build pass is supplied by this packet.
- [FWD] At that pin, implement/test missing shared budget + override + exact-mode and budgeted-expression seams, diagnostic-returning contour paths, and seed/resource seams if needed. Audit every diagnostic/resource channel; unknown or unobservable degradation blocks exact adoption rather than weakening policy.
- [FWD] Derive checked conversion/intermediate bounds, error envelopes, bounded defaults, worker stack policy and test process limits from source and Windows measurements. Rendering time/memory impact is unmeasured during authoring; no hard in-process render deadline is promised.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | Existing IR/conversion/assembly names read at crate-qualified paths; upstream entrypoints read from primary sources and explicitly reverified at the future pin; every adapter name is planned. |
| 2 | WIT/IR identifiers | PASS | `ExPolygon.contour/holes`, integer `Point2`, millimeter f32 `Point3`, flat `IndexedTriangleSet.indices` confirmed; no WIT change. |
| 3 | Schema versions | PASS | No IR/WIT version bump or invented version assertion. |
| 4 | ADRs | PASS | Geometry-only adapter preserves host data/assembly ownership; no SDK fixture duplication and no ADR allocation/amendment. |
| 5 | Deviation IDs | PASS | No deviation ID allocated, consumed or closed. |
| 6 | Prerequisites | PASS | Independent baseline branch; no draft packet claimed implemented. |
| 7 | Test targets | PASS | Crate/standalone targets are explicitly new with actual registration/feature requirements in the plan; private tests registered; process fixtures supplied here. |
| 8 | Criteria | PASS | Exact variants, real output oracles, strict negative paths and process/Windows evidence; AC-6 closes only the measured adapter prerequisite, with generation/shrink/profile/full-gate ownership explicitly assigned to #4; no workspace criterion command. |
| 9 | Fallout | PASS | No existing fields/schema changed. New limits/options/diagnostic/provenance/error literal sites and default/FRU handling listed in plan. |
| 10 | Docs impact | PASS | Canonical ownership/unit sections plus new adapter/compatibility/evidence docs have checks. |
| 11 | Completeness | PASS | No placeholder or scope-changing blocker; pin/patch/numeric/build feasibility are explicit forward implementation obligations. |

**Verdict: PREFLIGHT PASS (authoring only; adapter-prerequisite Windows execution not performed; full G-SCAD requires #4's remaining measurements/profiles).**

## Deviations

Recorded during implementation against `vendor/openrscad@1a45bd33f2` on Windows
(rustc 1.96.0). Each is a deliberate, evidenced choice, not a loosened criterion.

1. **Engine hosting: vendored patched subset, not a Cargo `rev` or a public fork.**
   The packet permits "a documented immutable fork revision with its upstream base
   and patch digest". The patched engine lives at `vendor/openrscad/` as its own
   workspace (excluded from the root workspace) with the upstream base revision,
   two ordered patch files and a recomputed digest in `vendor/openrscad/PATCHES.md`
   and a committed `Cargo.lock`. Rationale: hermetic CI (no network fetch of a
   fork), a pin held by this repository's own history, and the existing
   `vendor/clipper2-rust` precedent. The vendored tree reproduces byte-for-byte
   from base + patches (verified by applying the patches to a pristine checkout).
2. **Patch scope is wider than the two named gaps.** Besides the combined
   budget/override/mode entrypoint, budgeted expression evaluation and the
   diagnostic-returning contour entrypoint, the patch also adds structured
   `Warning::code`, `EvalError::budget_exhausted`, `EvalOutput::recursion_aborted`,
   `seed_random_state`, `eval_const_expr_with_budget_and_resolver` (an override
   expression's own warnings/echoes and the host resolver), and
   `Mesh::welded_exact`. Each closes a criterion this packet owns: fail-closed
   classification without prose matching (AC-N2), a budget exhaustion that is not a
   message (AC-2), a partial result that is not a nonempty mesh (AC-N2),
   reproducible seeded evaluation (AC-4), an override that cannot reach a resource
   or smuggle an unclassified warning (AC-N1, AC-N2), and a kernel boundary that
   cannot delete a thin body silently (AC-N3). Justification per patch is in
   `vendor/openrscad/PATCHES.md`.
3. **`MIN_WORKER_STACK_BYTES` is 64 MiB, and it is measured, not derived.** The
   packet's "unsafe stack setting" is an acceptance criterion, so it needed a real
   bound. Measured with the `cum` non-tail-recursion fixture in a `dev` profile on
   Windows: 16/24/32/36/40/44 MiB hit `STATUS_STACK_OVERFLOW` before the engine's
   6 000-frame guard could fire; 48/64/96 MiB reach the guard and return
   `PartialOutput`. Upstream's ≈ 1.6 KiB/frame release estimate would have implied
   ~10 MiB and produced a crash instead of a classification. 64 MiB is the measured
   debug floor plus margin.
4. **`RawMesh` carries a flat `Vec<u32>` index stream, not grouped `[u32; 3]`.**
   AC-N3 requires rejecting a *truncated* index stream. On an already-grouped array
   that check is vacuous; on a flat stream it is real. The engine adapter flattens
   the engine's `Vec<[u32; 3]>` at the boundary.
5. **`max_facets` bounds explicit `$fn` and derived counts for the primitives that
   expose their radius.** `circle`/`sphere`/`cylinder` carry `r` and `FragmentSpec`,
   so the engine's own `fragments` formula is evaluated before rendering and
   bounded. `offset`, extrusion and `import` nodes are bounded by their explicit
   `$fn` only; their derived counts are bounded after the fact by `max_vertices` and
   `max_triangles`. This is a narrower claim than "facet bounds are complete" and is
   recorded as a limitation in `docs/scad/windows-feasibility.md`. (An earlier
   revision of this entry claimed derived counts were never visible on the tree.
   That was wrong for the three primitives, and it is corrected here.)
6. **`DegradedOutput` is reachable from source, and is now tested that way.** An
   earlier revision claimed the pinned engine only degrades a boolean on an operand
   the public SCAD surface cannot construct without an `import`. That claim was
   false: a `polyhedron` whose faces do not form a closed manifold is itself such an
   operand, so
   `union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }`
   and its `projection()` form both return `DegradedOutput` with a
   `boolean_fallback` diagnostic. Both are committed fixtures.
7. **No `Default` for `ScadDiagnostic`, `ScadProvenance` or `ScadError`.** The
   packet says not to invent unsafe defaults for these. The single exhaustive
   literal in the classifier's test module therefore carries an
   `// exhaustive: <reason>` waiver naming the pinned data contract, per
   `docs/21_data_defaults_and_fixtures.md`.
8. **AST depth counts operator nesting, not parentheses.** `(( (1) ))` parses to a
   single literal, so the AC-N4 depth fixture uses real `+` nesting. The bound is
   unchanged; only the fixture was corrected.
9. **AC-6's CI-execution clause is satisfied on the delivered head.**
   `.github/workflows/ci.yml` gains the required self-contained
   `scad-adapter-windows` job. It executed on `800aabde` — the head carrying the
   final engine patch set, patch digest, `robust` dependency and adapter source — in
   run `43131961268` (job `130898010397`), passed every step (`F` 1, `F`+`A` identity
   1, `A` 9 + 1 ignored, `U` 22, `L` 3, narrow clippy) and uploaded artifact
   `scad-adapter-windows` (id `13906171026`). The superseded run `43113651822` on
   `65f6acf` is kept as history. Evidence, and the attribution of the unrelated
   pre-existing `Test`-job failure, are recorded in
   `docs/scad/windows-feasibility.md`.
10. **The process-supervision deadline probe blocks deliberately.** A real adapter
    run long enough to outlast a fixed deadline depends on host speed, which would
    make AC-5's probe flaky rather than informative. The controlled input is a
    blocked child; the supervision result is a distinct enum
    (`Exited`/`TimedOut`/`Terminated`), and the assertions are per case: the hang
    case is exactly `TimedOut`, the abort case is a failed exit that is **not** a
    timeout, and the run/budget cases exit successfully with their markers. The
    minimum-stack recursion fixture also runs in a supervised child, not in the
    parent test process.
11. **Post-review corrections (round 1).** An independent cold review reproduced six
    behavioural gaps, all now closed with the reviewer's witnesses kept as
    regressions: override expressions share the recording resolver and the
    classifier (and `undef` is `InvalidParameter`); raw rings get a 1 nm
    pre-quantization contact pass in addition to the 1e-4 mm pass; AST counting is
    aggregate across the program and every override, `projection()` children are
    traversed by both the node and facet walks, and derived primitive facet counts
    are preflighted; dimension-changing wrappers validate their child's dimension;
    `polyhedron` face indices and arity are validated before rendering; and
    parameter diagnostic spans are reported in expression coordinates. Isolation
    assertions now compare geometry as well as diagnostics. This item is recorded
    because those gaps shipped in the first implementation pass and the review —
    not the tests — found them.
12. **Wrapper fix, second delta (round 2).** Validating a dimension-changing wrapper's
    operand initially classified a wrapper around `Node::Empty` as `Mixed`, so
    `linear_extrude(1){}` and `projection(){}` were rejected instead of returning
    `Empty`. The empty operand now propagates as `Empty` through `Projection`,
    `Offset`, `LinearExtrude`, `RotateExtrude` and `Roof`, with public-path
    regressions for the empty wrapper alone and beside valid sibling geometry. The
    same delta corrected two documentation defects the first review had flagged but
    the fix pass missed: the first `adapter.md` example now matches
    `&out.geometry`, and the feasibility record no longer repeats the retracted
    `DegradedOutput` claim.

13. **Root-cause corrections from the third review.** Each of these fixes the cause
    rather than the symptom, and each keeps the reviewer's witness as a regression:
    - **Empty 2D wrappers.** The fix is in the engine's dispatch, not in the
      adapter's classifier: `is_2d` now ignores a `Projection`/`Offset` whose operand
      is structurally empty (`is_empty_tree`, patch 0002), so
      `union(){ projection(){}; cube(1); }` renders the cube instead of taking the 2D
      path and dropping it. The adapter additionally rejects any tree where its own
      dimensional analysis and `is_2d` disagree, because it must not depend on engine
      dispatch being right.
    - **Raw ring contact.** The 1 nm surrogate grid is gone. Contact is validated on
      the original `f64` rings with `robust::orient2d`, an adaptive-precision
      predicate with an exact sign for every finite input, so a crossing cannot be
      rounded out of existence. A sub-nanometre crossing witness is a committed
      regression.
    - **Post-quantization witness.** The public-path witness is now a pair of unit
      squares 2e-5 mm apart, which the exact raw pass accepts and the 100 nm grid
      rejects; the quantized pass names itself in the error so the two rejections are
      distinguishable.
    - **Diagnostic retention.** Evaluation warnings and echoes are classified before
      primary-error precedence, so an `ExternalResourceDenied` still carries the echo
      and the unclassified warning the source produced.
    - **Identifier validation.** Override names are validated by the adopted lexer
      (exactly one `Ident` token whose text is the whole name), not by a character
      shape plus a keyword list that could drift. Reserved keywords are rejected.
    - **Documentation examples.** `docs/scad/adapter.md` is included as crate
      documentation, so its `no_run` examples are compiled by `cargo test --doc`; the
      result-reading example now has a coherent `Result<(), Box<dyn Error>>` signature.
    - **AC-6 record.** The feasibility record now states the measurement context
      (`CARGO_TARGET_DIR`, profile, features, lockfiles) and where the full logs are
      retained, and the lane tees every test to `target/test-output.log` before
      archiving it, per the packet's canonical-log rule.
    - **Raw nesting containment.** The raw-versus-quantized forest comparison used a
      float ray-cast, whose intersection could round onto the query point's own X and
      report a strictly-contained child as outside — hiding a nesting change that
      quantization introduced. `contains_float` now uses the same exact orientation
      predicate as the contact check, with a witness that is rejected for every
      rotation of the child ring.
    - **Local `rustfmt` gate.** `cargo fmt --all` cannot run on the development host:
      rustfmt is invoked with a single over-long argument list and fails with
      `os error 206` before checking anything, so the workspace-wide Format job is the
      first thing to see drift. The adapter is therefore checked per file with
      `rustfmt --edition 2021 --check crates/slicer-scad/**/*.rs` before each push.
    - **AC-N1 coverage.** The named resource regression now also drives program-phase
      `dxf_dim` and `dxf_cross` (the `load_bytes` seam) and a relative `include`
      resolved from a child process whose working directory is the one holding a
      readable `tempting.scad`, so a working-directory fallback would be observed.
