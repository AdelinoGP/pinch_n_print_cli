# Design: 197-literal-sweep-host-runtime

## Controlling Code Paths

- Primary code path: none changed — test-scope construction sites only. Read-only anchors: `SliceRunOptions` + its packet-195 `impl Default` (`crates/slicer-runtime/src/run.rs`), `PipelineConfig` (`crates/slicer-runtime/src/pipeline.rs`), `ExecutionPlan` + `impl Default` (`crates/slicer-scheduler/src/execution_plan.rs`), fixtures in `crates/slicer-sdk/src/test_support/fixtures.rs`, `common::pipeline_config_base` (`crates/slicer-runtime/tests/common/mod.rs`).
- Neighboring tests/fixtures: runtime bucket binaries mount `tests/common/mod.rs` via `#[path = "../common/mod.rs"] mod common;` in each `<bucket>/main.rs` (verified in `tests/integration/main.rs`); the ten top-level runtime test files are their own binaries and can mount `common` the same way if a fixture is needed there.
- OrcaSlicer comparison: not applicable — no parity surface.

## Architecture Constraints

- **No guest-WASM input in scope (grounded 2026-08-07).** `shared_input_paths` in `xtask/src/build_guests.rs` collects `src/`, `Cargo.toml`, `build.rs` of `slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`, `slicer-core` plus WIT and per-guest dirs. None of `slicer-runtime`, `slicer-scheduler`, `slicer-wasm-host` (its host crate), `pnp-cli` is in that set, so this packet cannot make guests stale and carries no `build-guests` gate. The wasm-staleness snippet is deliberately omitted. Two cautions remain: (1) stale guests *from before this packet* can fail runtime `executor`/`e2e` and wasm-host tests — run `cargo xtask build-guests --check` before blaming the sweep (CLAUDE.md rule); (2) `crates/slicer-wasm-host/test-guests/**` IS guest-feeding and rule-exempt — untouched, guarded by AC-N4.
- **Renamed dev-dep.** `slicer-runtime` consumes the sdk as `slicer_sdk = { package = "slicer-sdk", ..., features = ["test"] }` — call sites write `slicer_sdk::test_support::fixtures::...`. The new `pnp-cli` dev-dep uses the plain name `slicer-sdk` (crate ident `slicer_sdk` either way).
- **Class-c helper contract.** `common::pipeline_config_base(mesh_ir, plan, runners) -> PipelineConfig` (packet-195 export: `cancel_flag` `None`, empty `wasm_handles`, empty `resolved_configs`, passed args installed). Call sites that need non-base fields use FRU over the call: `PipelineConfig { support_tools: <x>, ..common::pipeline_config_base(m, p, r) }`. The pnp-cli twin has the same shape, file-local.
- **Marshal carrier tests.** `slicer-wasm-host` tests that assert every field crosses the WIT boundary keep exhaustive literals with waiver reason `// exhaustive: WIT-boundary carrier test asserts every field crosses`. This is the plan's production-checkpoint rationale extended to boundary tests; it is the intended waiver use.
- **Watched types without a base** (surfaced by the report, e.g. mesh/view structs): file-local `fn <type>_base()` with one waivered exhaustive literal, FRU at call sites — packet-195 pnp-cli twin precedent. Never add `Default` here.

## Code Change Surface

- Selected approach: report-driven sweep, one step per crate (runtime split into two steps by bucket to keep step cost ≤M).
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/pnp-cli/Cargo.toml`: add `[dev-dependencies]` `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }`.
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs`: route its `PipelineConfig` sites (6 measured 2026-08-07; re-derive) through the file-local twin; delete the `#[allow(dead_code)]` on `fn pipeline_config_base`.
  - `crates/pnp-cli/tests/visual_debug_overlays_tdd.rs`, `visual_debug_intermediate_renderer_tdd.rs`: `PrintEntity` → `print_entity_base`.
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`: 14 `PipelineConfig` sites → `common::pipeline_config_base`.
  - Runtime buckets + top-level files + `benches/**` + cfg-test mod in `src/layer_executor.rs`: `SliceRunOptions`/`ExecutionPlan`/`Point3WithWidth`/`GlobalLayer`/`LayerCollectionIR` → FRU; `PrintEntity`/`WallLoop` → sdk fixtures via existing `slicer_sdk` dev-dep. The whole `tests/common/` tree (`mod.rs` plus sibling fixture files: `perimeter_harness.rs` — 1 exhaustive `PipelineConfig` literal, measured 2026-08-07 — `ir_builders.rs`, `dispatch_fixture.rs`, etc.) has its own helper fns converted to FRU/`pipeline_config_base` internally where reported.
  - `crates/slicer-scheduler/tests/**` (+ `tests/fixtures/`) and reported cfg-test mods: `ExecutionPlan`/`GlobalLayer` → FRU.
  - `crates/slicer-wasm-host/tests/**` (incl. `tests/common/mod.rs` helper fns) and reported cfg-test mods in `src/host.rs`, `src/marshal/leaf.rs`: `Point3WithWidth`/`GlobalLayer` → FRU; carrier waivers as above.
- Rejected alternatives and reasons:
  - Converting pnp-cli `PipelineConfig` sites to a shared crate helper — rejected; `PipelineConfig` holds trait objects, the plan's locked decision 3(c) picked per-crate/file-local helpers, and packet 195 already landed the twin.
  - Adding `Default` for `PipelineConfig`/`MeshIR`-class types — rejected; trait-object holders and schema-versioned types need constructed bases, not derived zeros.
  - Spell-all-plus-FRU — rejected (`clippy::needless_update`, defeats churn reduction).

## Files in Scope (read + edit)

Sweep packet: per-crate bounded globs replace the 3-file list; each step edits one crate's test surface only.

- `crates/pnp-cli/tests/**/*.rs` + `crates/pnp-cli/Cargo.toml` - role: pnp-cli sweep; expected change: dev-dep, twin activation, FRU/fixture conversions.
- `crates/slicer-scheduler/tests/**/*.rs` + reported `#[cfg(test)]` mods in `crates/slicer-scheduler/src/**` - role: scheduler sweep; expected change: FRU conversions.
- `crates/slicer-wasm-host/tests/**/*.rs` + reported `#[cfg(test)]` mods in `crates/slicer-wasm-host/src/**` - role: wasm-host sweep; expected change: FRU conversions + carrier waivers.
- `crates/slicer-runtime/tests/**/*.rs` + `crates/slicer-runtime/benches/**/*.rs` + reported `#[cfg(test)]` mod in `crates/slicer-runtime/src/layer_executor.rs` - role: runtime sweep (two steps); expected change: FRU/fixture/helper conversions.

## Read-Only Context

- `crates/slicer-runtime/src/run.rs` - `SliceRunOptions` struct + `Default` impl only - purpose: know the quiet baseline per field.
- `crates/slicer-runtime/src/pipeline.rs` - `PipelineConfig` struct only - purpose: field inventory for FRU-over-base calls.
- `crates/slicer-scheduler/src/execution_plan.rs` - `ExecutionPlan` struct + `Default` impl only - purpose: same.
- `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures only.
- `crates/slicer-runtime/tests/common/mod.rs` - >500 lines; ranged reads around `pipeline_config_base` and reported helpers only.

## Out-of-Bounds Files

- `crates/slicer-wasm-host/test-guests/**` - rule-exempt, guest-feeding; never edit, never load (AC-N4).
- Production (non-`cfg(test)`) src in all four crates - exhaustive marshal/producer literals are intentional checkpoints.
- `xtask/src/check_literals.rs` + xtask tests - packet 194 owns; defects are deviations.
- `docs/spec_packets/194-*/`, `docs/spec_packets/195-*/` except `packet.spec.md` - SUMMARY dispatch only.
- `crates/slicer-runtime/tests/common/mod.rs`'s packet-195 `pipeline_config_base` body/signature - consumed, not edited (new sibling helpers allowed).
- `OrcaSlicerDocumented/` - never load. `target/`, `Cargo.lock`, generated code, vendored deps - never load (except `target/sweep-197-*` scratch).

## Expected Sub-Agent Dispatches

- Question: run Step-1 report + baselines; return per-crate violating-file list (path + count) and baseline greenness; scope: commands in `requirements.md` matrix; return: `LOCATIONS` ≤20 entries per crate + `FACT`.
- Question: after a crate's sweep, does its `check-literals` exit 0 and its suite diff clean against `target/sweep-197-<crate>-baseline.txt`?; scope: one crate; return: `FACT` PASS/FAIL + ≤5 lines on failure.
- Question: mid-runtime-sweep, does `cargo test -p slicer-runtime --test <bucket>` pass?; scope: one bucket binary; return: `FACT` + failing test names ≤5 lines.
- Question: workspace gates (`check`/`clippy` `--all-targets`); scope: workspace; return: `FACT` + first error ≤10 lines.

## Data and Contract Notes

- IR/manifest contracts: untouched; conversions are value-identical by construction (omitted fields equal base values).
- WIT boundary: untouched; carrier-test exhaustiveness preserved via waivers, never FRU'd away.
- Determinism/scheduler constraints: `ExecutionPlan` FRU must not alter any plan a test builds — `Default` supplies empty stage vectors only where the test already spelled empties.

## Locked Assumptions and Invariants

- Test counts, assert counts, and every constructed value invariant; construction syntax only.
- The five packet-195 no-`Default` locks hold (AC-N1).
- `test-guests/**` untouched (AC-N4).
- The packet-195 helper signatures (`pipeline_config_base` both homes, sdk fixtures) are consumed as-is.

## Risks and Tradeoffs

- Largest sweep area (sizing estimates in `requirements.md`, re-derive): risk of step overrun — mitigated by splitting runtime into two bucket-scoped steps with per-bucket verification.
- Runtime `executor`/`e2e` buckets execute real WASM slicing: two full-suite runs (baseline, post) are slow but mandatory; failures there first get a `build-guests --check` triage per CLAUDE.md before sweep-blame.
- `pipeline_config_base` FRU can subtly change a test if a spelled field previously differed from the base — conversion reviews must diff field-by-field against the base contract before omitting anything.
- Checker false negatives/positives at this scale are deviations against packet 194, not local patches.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (runtime bucket sweep)
- Highest-risk dispatch and required return format: Step-1 report enumeration — `LOCATIONS` capped at 20 entries per crate; never the raw report body.

## Open Questions

- `[FWD]` If the report surfaces watched types with no base in wasm-host/scheduler tests, choose file-local waivered base fns per the stated precedent; record each new base fn in the close notes for 199's audit.
- `[FWD]` Runtime top-level test binaries that need `common` fixtures may add the `#[path = "../common/mod.rs"] mod common;` mount; do so only where the report demands it (each mount makes that binary compile `common`).
