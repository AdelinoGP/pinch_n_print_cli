# Design: 195-defaults-and-fixture-bases

## Controlling Code Paths

- Primary code path: `crates/slicer-sdk/src/test_support/fixtures.rs` (the gated `#[cfg(any(test, feature = "test"))]` module declared in `crates/slicer-sdk/src/lib.rs`) — the three new fixture bases sit beside the existing `print_entity`, `tool_change`, and `seam_candidate` helpers and the `PerimeterRegionViewBuilder`. Secondary paths: `impl Default for SliceRunOptions` in `crates/slicer-runtime/src/run.rs`; `pipeline_config_base` in `crates/slicer-runtime/tests/common/mod.rs` (mod opens with `#![allow(dead_code)]` — verified 2026-08-07 — so an initially-unused helper cannot trip `-D warnings`).
- Neighboring tests/fixtures: `crates/slicer-sdk/tests/test_support_*.rs` files (each with a `[[test]] required-features = ["test"]` entry and the dev-dependency self-reference `slicer-sdk = { path = ".", features = ["test"] }` already in `crates/slicer-sdk/Cargo.toml`); `crates/slicer-runtime/tests/unit/main.rs` and `tests/integration/main.rs` aggregator binaries (`mod <file>;` lines, `#[path = "../common/mod.rs"] mod common;`); the Noop stage runners (`NoopPrepassRunner`, `NoopLayerRunner`, `NoopFinalizationRunner`, `NoopPostpassRunner`) defined in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`.
- OrcaSlicer comparison: none — no parity surface; the OrcaSlicer sections are intentionally absent from this packet.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Of this packet's edits, only `crates/slicer-sdk/src/test_support/fixtures.rs` feeds the guest build (`crates/slicer-sdk/**` is a universal guest dep; the `test_support` module is cfg-gated out of guest code, but the freshness gate is mtime-based, so the rebuild is still mandatory). `slicer-runtime`, `pnp-cli`, docs, and ADRs are host-only.
- ADR-0004's negative consequence stands unchanged: guest builds must never enable `slicer-sdk`'s `test` feature; the new fixture fns live behind the existing gate and add nothing to production or guest surfaces.
- ADR-0054's constraints on `pnp-cli-locator` (std-only, dev-dep-only, host-side-only, exactly four functions) are untouched — this packet edits only its header rustdoc wording.
- Schema/version constants: `SliceRunOptions::default()` pins nothing new — `MeshIR::default()` already pins `CURRENT_MESH_IR_SCHEMA_VERSION` (existing manual impl in `crates/slicer-ir/src/slice_ir.rs`). No version constant is bumped anywhere in this packet.

## Code Change Surface

- Selected approach: additive-only. No struct shape changes, no call-site conversions; every deliverable is a new fn, a new impl, a new test, or an ADR amendment.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-runtime/src/run.rs`: `impl Default for SliceRunOptions` (manual, §5 Bucket-B style) with rustdoc: "quiet test baseline — `progress_events: false` deliberately differs from `pnp_cli slice`'s CLI default (`true`); production callers set every field explicitly". Field values: `mesh: Arc::new(MeshIR::default())`, `model_label: String::new()`, `config_path/output_path/thumbnail/report/cancel_flag: None`, `module_dirs: Vec::new()`, `config_overrides: HashMap::new()`, all six bools `false`.
  - `crates/slicer-sdk/src/test_support/fixtures.rs`, three new `#[must_use]` fns (naming: `*_base` marks "intended as the expression right of `..`", distinct from the existing fully-parameterized `print_entity`):
    - `pub fn print_entity_base(role: ExtrusionRole) -> PrintEntity` — `entity_id: 0`, `path: ExtrusionPath3D { points: vec![Point3WithWidth::default()], role: role.clone(), speed_factor: 1.0 }`, `role`, `region_key: RegionKey::default()`, `topo_order: 0`, `tool_index: 0`. One point satisfies the non-empty `path.points` invariant asserted by `project_ordered_entities` (`crates/slicer-runtime/src/layer_executor.rs`).
    - `pub fn wall_loop_base(loop_type: LoopType, boundary_type: WallBoundaryType) -> WallLoop` — `perimeter_index: 0`, `loop_type`, `path`: single-default-point path with `speed_factor: 1.0` and `role` mapped `LoopType::Outer → ExtrusionRole::OuterWall`, `LoopType::ThinWall → ExtrusionRole::ThinWall`, `_ → ExtrusionRole::InnerWall` (`LoopType` is `#[non_exhaustive]`, so the wildcard arm is required anyway), `width_profile: WidthProfile { widths: vec![0.0] }` (length-matched to the single point, the "one width per vertex" convention `PerimeterRegionViewBuilder::add_outer_wall` follows — verified 2026-08-07), `feature_flags: Vec::new()`, `boundary_type`.
    - `pub fn ordered_entity_view_base(role: ExtrusionRole) -> OrderedEntityView` — targets `crate::views::OrderedEntityView` (7 fields; the same-named host-side structs in `slicer-runtime`/`slicer-wasm-host` are out of scope): `original_index: 0`, `tool_index: 0`, `region_key: RegionKey::default()`, `role`, `start_point: Point3WithWidth::default()`, `end_point: Point3WithWidth::default()`, `point_count: 2`.
  - `crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs` (new): tests `print_entity_base_*`, `wall_loop_base_*`, `ordered_entity_view_base_*` asserting exactly the AC-2/3/4 field values, plus one FRU usage per base (e.g. `PrintEntity { topo_order: 7, ..print_entity_base(role) }`) proving the base composes with struct-update syntax.
  - `crates/slicer-sdk/Cargo.toml`: one `[[test]] name = "test_support_fixture_bases_tdd" / required-features = ["test"]` entry, matching the existing entries.
  - `crates/slicer-runtime/tests/common/mod.rs`: `pub fn pipeline_config_base(mesh_ir: Arc<MeshIR>, plan: ExecutionPlan, runners: PipelineStageRunners) -> PipelineConfig` — `cancel_flag: None`, `support_tools: SupportToolSelection::default()`, `resolved_configs: Arc::new(BTreeMap::new())`, `default_resolved_config: Arc::new(ResolvedConfig::default())`, `bounds: Arc::new(ConfigBoundsIndex::empty())`, `wasm_handles: HashMap::new()`. Its one exhaustive `PipelineConfig` literal carries `// exhaustive: single per-crate construction point for trait-object holder PipelineConfig` (the packet-194 waiver format).
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`: append `#[test] fn pipeline_config_base_smoke()` using the file's existing Noop runners and `ExecutionPlan::default()`.
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs`: file-local `#[allow(dead_code)] fn pipeline_config_base(...)` twin (mirror the file's existing imports; the allow is removed by sweep packet 197 when it converts the 6 sites).
  - `docs/adr/0054-host-side-test-support-crate.md`: an "Amendment — <date-at-write-time> (packet 195)" section (date it when you write it — ledger-fact discipline): `slicer_sdk::test_support` is the **single IR-fixture home** for host- and guest-side tests; host crates consume it via a `slicer-sdk` dev-dep with `feature = "test"` (added by the sweep packets); Decision item 3's "guest-side surface" wording is superseded accordingly; the locator/test_support disjointness stands; the host-side `WallLoopBuilder` in `crates/slicer-runtime/tests/common/ir_builders.rs` is recorded as a consolidation target for the sweeps.
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md`: matching short amendment (scope extension; the guest-build negative consequence unchanged).
  - `crates/pnp-cli-locator/src/lib.rs`: header rustdoc wording only (two spots currently scope `slicer_sdk::test_support` as guest-side).
- Rejected alternatives and reasons:
  - `#[derive(Default)]` on `SliceRunOptions` — compiles (every field type is `Default` once `Arc<MeshIR>` resolves via `MeshIR: Default`), but a manual impl is chosen so the `progress_events` divergence from the CLI default is spelled and documented per-field (§5 Bucket-B convention).
  - `Default` for `OrderedEntityView` (orchestrator's original classification) — rejected: `role: ExtrusionRole` fails §3.6's safe-variant criterion, the same reason `PrintEntity` is fixture-based. Re-classified to class (b).
  - A new host-side fixture crate — rejected by the plan's locked decision 3(b) (user chose `sdk::test_support`, accepting the guest-staleness rebuild tax) and by ADR-0054's alternative 2 analysis.
  - `pipeline_config_base` taking no `runners` param with internally-built noops — rejected: the Noop runners live in test files, not `src`; hoisting them into `tests/common` is real scope the sweeps don't need (callers always have runners in hand).

## Files in Scope (read + edit)

- `crates/slicer-sdk/src/test_support/fixtures.rs` - role: class (b) home; expected change: three fns (+ rustdoc with examples).
- `crates/slicer-runtime/src/run.rs` - role: class (a); expected change: one manual `impl Default` + rustdoc.
- `crates/slicer-runtime/tests/common/mod.rs` - role: class (c) home; expected change: one helper fn with waivered literal.
- Justified extras (each a single bounded edit, listed per step in `implementation-plan.md`): `crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs` (new), `crates/slicer-sdk/Cargo.toml`, `crates/slicer-runtime/tests/unit/slice_run_options_default_tdd.rs` (new) + `tests/unit/main.rs` (one `mod` line), `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` (append one test), `crates/pnp-cli/tests/e2e_integration_tdd.rs` (one fn), `docs/adr/0054-*.md`, `docs/adr/0004-*.md`, `crates/pnp-cli-locator/src/lib.rs` (header only).

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - only the definition windows of `Point3WithWidth`, `WallLoop`, `PrintEntity`, `LayerCollectionIR`, `RegionKey`, `WallBoundaryType`, `LoopType`, `WidthProfile`, `MeshIR`'s `Default` (symbol-anchored ranged reads; file > 2300 lines).
- `crates/slicer-sdk/src/views.rs` - the `OrderedEntityView` definition window only.
- `crates/slicer-runtime/src/pipeline.rs` - `PipelineConfig` / `PipelineStageRunners` definitions only.
- `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` - the Noop-runner block and one existing `PipelineConfig` literal only.
- `docs/specs/_OLD/default-builder-migration.md` - §3.6 (~176-198) and §5 intro (~308-330) only.
- `crates/pnp-cli/tests/e2e_integration_tdd.rs` - imports block and one existing `PipelineConfig` literal only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - not applicable; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `docs/specs/_OLD/default-builder-migration.md` outside the two cited sections - never load in full (1449 lines).
- Every other test file constructing the watched types (the 103 `Point3WithWidth` files, `layer_module_tdd.rs`, etc.) - sweep-packet territory; do not open or convert.
- `xtask/**` - packet 194's surface; consumed as a binary only.

## Expected Sub-Agent Dispatches

- Question: run `cargo xtask check-literals --report`, and return (1) the distinct watched type names appearing in violations, (2) for each of `SliceRunOptions`, `PrintEntity`, `WallLoop`, `OrderedEntityView`, `PipelineConfig`, `Diagnostic`, `DeferredRetract`, `DeferredTravelMove`: violation count and up to 3 sample `file:line` entries; scope: repo root; return: `LOCATIONS` (≤ 20 entries) + FACT counts; purpose: Step 1 audit.
- Question: does `cargo check --workspace --all-targets` pass?; scope: workspace; return: `FACT` pass/fail, ≤ 20 error lines on failure; purpose: Steps 2-4 gates and close.
- Question: run `cargo xtask build-guests --check` (and, if `STALE:`, the rebuild then re-check); scope: repo root; return: `FACT` clean/stale+rebuilt; purpose: Step 3 and close.

## Data and Contract Notes

- IR/manifest contracts: untouched — no IR struct gains, loses, or changes a field; `SliceRunOptions` is a runtime options struct outside the IR schema docs.
- WIT boundary: untouched. `ordered_entity_view_base` builds the SDK-side mirror of WIT `record ordered-entity-view`; the record itself is unchanged.
- Determinism/scheduler constraints: none — test-only surfaces plus one `Default` impl never invoked by the pipeline.
- Fixture-base contract for sweeps (frozen at close): the three `*_base` names/signatures, `pipeline_config_base`'s three-parameter shape in both crates, and `SliceRunOptions::default()`'s all-quiet field values.

## Locked Assumptions and Invariants

- `PrintEntity`, `WallLoop`, `OrderedEntityView` never gain `Default` (locked by §3.6's rejected-enum list and `PrintEntity`'s rustdoc); AC-N1 enforces it.
- The audit's dropped list (`Diagnostic` ×2, `DeferredRetract`, `DeferredTravelMove`) stays dropped unless Step 1's re-derivation finds test-code construction sites; AC-N2 enforces the default outcome.
- `slicer_sdk::test_support` is the single IR-fixture home (plan decision 3(b)); no parallel host-side fixture crate may be introduced.
- `SliceRunOptions::default().progress_events == false` is a deliberate divergence from the CLI default and is locked by AC-1; changing it later is a behavior change for every FRU-converted test.

## Risks and Tradeoffs

- Guest-staleness tax: every `fixtures.rs` edit invalidates all ~34 guest artifacts (mtime-based gate). Accepted by the user in the plan's locked decision 3(b); the rebuild is budgeted in Step 3.
- `wall_loop_base`'s role mapping (`Outer → OuterWall`, `ThinWall → ThinWall`, else `InnerWall`) is a fixture convention, not an IR invariant; tests that care about `path.role` must override it via FRU. Documented in the fn's rustdoc.
- The pnp-cli helper is dead code until sweep 197 — mitigated with `#[allow(dead_code)]` and an explicit "removed by packet 197" comment; risk is a forgotten allow, caught by 197's per-area zero-violation gate.
- Three same-named `OrderedEntityView` structs exist (`slicer-sdk` views, `slicer-runtime` layer_executor, `slicer-wasm-host` dispatch). The base serves only the SDK one; host-side literals of the other two remain for the sweeps to waiver or FRU via other means. Recorded for packets 197's author.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3: fixtures + gated test file + manifest entry + guest rebuild)
- Highest-risk dispatch and required return format: the Step-1 `--report` audit; `LOCATIONS` ≤ 20 entries + per-type FACT counts (reject full violation dumps).

## Open Questions

- `[FWD]` If Step 1's audit surfaces an additional no-`Default` watched type with test-code violations that passes §3.6/§5 (no unsafe enum, no trait object/`Arc` needing a real value, degenerate zero acceptable), the implementer may add its manual `impl Default` within Step 2's budget and must record the addition in the completion report for the sweep packets; anything failing the criteria is left for sweep-time waivers and listed in the report instead.
- `[FWD]` `wall_loop_base` leaves `feature_flags` empty rather than length-matched to `path.points` — verified 2026-08-07 that `WallLoop` declares no length invariant in its rustdoc, and the existing `PerimeterRegionViewBuilder::add_outer_wall` also pushes empty `feature_flags`. If a consumer assertion during sweeps proves otherwise, adjust the base (not the callers) and note it.
