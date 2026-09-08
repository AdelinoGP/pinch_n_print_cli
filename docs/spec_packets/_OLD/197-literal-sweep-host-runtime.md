---
status: implemented
packet: 197-literal-sweep-host-runtime
task_ids:
  - TASK-319
---

# 197-literal-sweep-host-runtime

## Goal

Convert every `cargo xtask check-literals` violation in `slicer-runtime`, `slicer-scheduler`, `slicer-wasm-host`, and `pnp-cli` test code to FRU-over-a-base — including the 14 `PipelineConfig` sites in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` routed through `common::pipeline_config_base` and the `pnp-cli` e2e sites routed through their packet-195 file-local twin — so `cargo xtask check-literals crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli` exits 0 with every suite green and test counts unchanged.

## Problem Statement

Queue row #4 of `docs/specs/struct-literal-churn-gate-plan.md`. The host-side crates carry the largest violation surface of the three sweep areas (sizing estimates measured 2026-08-07, re-derive from the Step-1 report: 39 `slicer-runtime` test/bench files with `Point3WithWidth` literals, 63 with `GlobalLayer`, 21 with `LayerCollectionIR`, 20 with `PrintEntity`, 11 with `WallLoop`; plus `slicer-wasm-host` ~6, `pnp-cli` ~5, `slicer-scheduler` ~2). This is also where the three packet-195 base classes all converge: `SliceRunOptions::default()` (class a), sdk fixtures (class b, dev-dep already present in `slicer-runtime` as the renamed `slicer_sdk`), and the `PipelineConfig` helpers (class c). One coherent slice: all four crates sit on the host side of the WIT boundary and share the runtime test-fixture vocabulary.

## Architecture Constraints

- **No guest-WASM input in scope (grounded 2026-08-07).** `shared_input_paths` in `xtask/src/build_guests.rs` collects `src/`, `Cargo.toml`, `build.rs` of `slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`, `slicer-core` plus WIT and per-guest dirs. None of `slicer-runtime`, `slicer-scheduler`, `slicer-wasm-host` (its host crate), `pnp-cli` is in that set, so this packet cannot make guests stale and carries no `build-guests` gate. The wasm-staleness snippet is deliberately omitted. Two cautions remain: (1) stale guests *from before this packet* can fail runtime `executor`/`e2e` and wasm-host tests — run `cargo xtask build-guests --check` before blaming the sweep (CLAUDE.md rule); (2) `crates/slicer-wasm-host/test-guests/**` IS guest-feeding and rule-exempt — untouched, guarded by AC-N4.
- **Renamed dev-dep.** `slicer-runtime` consumes the sdk as `slicer_sdk = { package = "slicer-sdk", ..., features = ["test"] }` — call sites write `slicer_sdk::test_support::fixtures::...`. The new `pnp-cli` dev-dep uses the plain name `slicer-sdk` (crate ident `slicer_sdk` either way).
- **Class-c helper contract.** `common::pipeline_config_base(mesh_ir, plan, runners) -> PipelineConfig` (packet-195 export: `cancel_flag` `None`, empty `wasm_handles`, empty `resolved_configs`, passed args installed). Call sites that need non-base fields use FRU over the call: `PipelineConfig { support_tools: <x>, ..common::pipeline_config_base(m, p, r) }`. The pnp-cli twin has the same shape, file-local.
- **Marshal carrier tests.** `slicer-wasm-host` tests that assert every field crosses the WIT boundary keep exhaustive literals with waiver reason `// exhaustive: WIT-boundary carrier test asserts every field crosses`. This is the plan's production-checkpoint rationale extended to boundary tests; it is the intended waiver use.
- **Watched types without a base** (surfaced by the report, e.g. mesh/view structs): file-local `fn <type>_base()` with one waivered exhaustive literal, FRU at call sites — packet-195 pnp-cli twin precedent. Never add `Default` here.

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
