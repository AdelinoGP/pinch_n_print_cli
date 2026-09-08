# Requirements: 197-literal-sweep-host-runtime

## Packet Metadata

- Grouped task IDs: `TASK-319`
- Backlog source: `docs/07_implementation_status.md` (new row added at completion; TASK-319 allocated by `docs/specs/struct-literal-churn-gate-plan.md` — re-derive the highest existing TASK id before writing the row)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Queue row #4 of `docs/specs/struct-literal-churn-gate-plan.md`. The host-side crates carry the largest violation surface of the three sweep areas (sizing estimates measured 2026-08-07, re-derive from the Step-1 report: 39 `slicer-runtime` test/bench files with `Point3WithWidth` literals, 63 with `GlobalLayer`, 21 with `LayerCollectionIR`, 20 with `PrintEntity`, 11 with `WallLoop`; plus `slicer-wasm-host` ~6, `pnp-cli` ~5, `slicer-scheduler` ~2). This is also where the three packet-195 base classes all converge: `SliceRunOptions::default()` (class a), sdk fixtures (class b, dev-dep already present in `slicer-runtime` as the renamed `slicer_sdk`), and the `PipelineConfig` helpers (class c). One coherent slice: all four crates sit on the host side of the WIT boundary and share the runtime test-fixture vocabulary.

## In Scope

- Convert exhaustive watched-type struct literals to FRU (`..Default::default()` or `..<fixture>()`) in:
  - `crates/slicer-runtime/tests/**` (five bucket binaries `unit|contract|executor|integration|e2e` driven by `<bucket>/main.rs`, ten top-level test files, and the whole shared `tests/common/` tree — `mod.rs` plus its sibling fixture files, incl. `perimeter_harness.rs` which holds 1 exhaustive `PipelineConfig` literal, measured 2026-08-07), `crates/slicer-runtime/benches/**`, and the `#[cfg(test)]` mod in `crates/slicer-runtime/src/layer_executor.rs` (candidate measured 2026-08-07; re-derive);
  - `crates/slicer-scheduler/tests/**` (incl. `tests/fixtures/`) and any reported `#[cfg(test)]` mods (candidate: `src/execution_plan.rs`);
  - `crates/slicer-wasm-host/tests/**` (incl. `tests/common/mod.rs` with its `fn point3` helper — the only construction helper fn there, verified 2026-08-07) and reported `#[cfg(test)]` mods (candidates: `src/host.rs`, `src/marshal/leaf.rs`);
  - `crates/pnp-cli/tests/**`.
- Conversion rule (locked decision 2; `docs/21_data_defaults_and_fixtures.md` normative): keep explicitly-meaningful non-default fields spelled, OMIT fields equal to the base's value, append `..Default::default()` / `..<fixture>()`; never spell-all-plus-FRU (`clippy::needless_update`); never change what a test asserts.
- Class-specific conversions (packet-195 exports, consumed not re-decided):
  - `SliceRunOptions` literals → `SliceRunOptions { <meaningful fields>, ..Default::default() }` (quiet baseline: all bools false including `progress_events`, all `Option`s `None`, empty collections, empty `MeshIR` pinned to `CURRENT_MESH_IR_SCHEMA_VERSION`).
  - `ExecutionPlan` literals → FRU over its existing `impl Default` (`crates/slicer-scheduler/src/execution_plan.rs`).
  - `PipelineConfig` in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` (14 sites measured 2026-08-07; re-derive) → `common::pipeline_config_base(mesh_ir, plan, runners)`; other runtime `PipelineConfig` sites likewise where the binary can reach `common`.
  - `PipelineConfig` in `crates/pnp-cli/tests/e2e_integration_tdd.rs` (6 sites measured 2026-08-07; re-derive) → the packet-195 file-local twin; remove its `#[allow(dead_code)]`.
  - `PrintEntity`/`WallLoop` sites → `slicer_sdk::test_support::fixtures::{print_entity_base, wall_loop_base}`; `slicer-runtime` already dev-deps the sdk (renamed `slicer_sdk`, `features = ["test"]`, verified 2026-08-07); `pnp-cli` gains `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }` in `[dev-dependencies]` (its 2 `PrintEntity` files measured 2026-08-07: `visual_debug_overlays_tdd.rs`, `visual_debug_intermediate_renderer_tdd.rs`).
  - Default-able types (`Point3WithWidth`, `GlobalLayer`, `LayerCollectionIR` — all verified to have `Default`) → plain FRU.
- Watched types with neither `Default` nor a 195 fixture (e.g. `MeshIR`-adjacent or view structs surfaced by the report): file-local `fn <type>_base()` with one waivered exhaustive literal (packet-195 pnp-cli twin precedent), FRU at call sites; never add `Default` in this packet.
- Waivers where exhaustiveness IS the test's intent (marshal/WIT carrier tests asserting every field crosses the boundary), each with a mandatory reason.
- Baseline capture before any edit (per-crate suite summaries, assert-macro count, `#[test]` count) into `target/sweep-197-*`; post-sweep invariance proof.

## Out of Scope

- Any assertion, expected-value, tolerance, or fixture-semantic change.
- Adding `Default` to `PrintEntity`, `WallLoop`, `Diagnostic` (both crates), `DeferredRetract`, `DeferredTravelMove` (packet-195 locks, re-guarded by AC-N1).
- `crates/slicer-wasm-host/test-guests/**` — rule-exempt (WIT adapter shims must break loudly) and guest-feeding; untouched (AC-N4).
- Production `src/` literals outside `#[cfg(test)]` mods (marshal exhaustive literals are deliberate propagation checkpoints — the plan's counter-evidence case).
- Sweeps of `slicer-ir`/`slicer-core`/`slicer-gcode` (packet 196), `slicer-sdk`/`modules` (packet 198), and residue outside all sweep areas (packet 199 absorbs; reviewer-verified 2026-08-07: `slicer-helpers` 0, `slicer-model-io` 1, `slicer-macros` 1 files — earlier broader-grep figures of 3/2/1 counted unwatched `MeshIR`-style matches).
- Enforcement wiring (packet 199); editing the checker (`xtask/src/check_literals.rs` — packet 194 owns it; defects are deviations).
- Editing `crates/slicer-runtime/tests/common/mod.rs`'s `pipeline_config_base` signature (packet-195 contract; call sites adapt to it, not vice versa). Adding new helper fns to `common/mod.rs` is allowed; changing existing 195-authored ones is not.

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time; delegate SUMMARY if over 300 lines.
- `CLAUDE.md` §Test Discipline, §Guest WASM Staleness - named sections only.
- `docs/adr/0054-host-side-test-support-crate.md` / `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - "single IR-fixture home" amendments (packet 195); FACT-check via dispatch only if fixture consumption is questioned.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: AC-1's clean area is one third of packet 199's enforcement precondition; AC-5 completes the packet-195 deferred obligation (twin goes live, allow removed); waiver inventory feeds 199's audit.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask check-literals --report crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli \| tee target/sweep-197-report.txt \| tail -1` | Step-1 enumeration (report mode exits 0) | FACT: summary line |
| `cargo xtask check-literals crates/pnp-cli; test $? -eq 0 && echo PASS` | per-crate green after the pnp-cli step | FACT PASS/FAIL |
| `cargo xtask check-literals crates/slicer-scheduler; test $? -eq 0 && echo PASS` | per-crate green after the scheduler step | FACT PASS/FAIL |
| `cargo xtask check-literals crates/slicer-wasm-host; test $? -eq 0 && echo PASS` | per-crate green after the wasm-host step | FACT PASS/FAIL |
| `cargo xtask check-literals crates/slicer-runtime; test $? -eq 0 && echo PASS` | per-crate green after the runtime steps | FACT PASS/FAIL |
| AC-1 through AC-7, AC-N1 through AC-N4 pipe-suffixed commands (see `packet.spec.md`) | packet acceptance | FACT PASS/FAIL each; on suite FAIL, `grep -E 'FAILED\|panicked at' -C 5 target/test-output.log` SNIPPETS ≤20 lines |
| `cargo test -p slicer-runtime --test integration 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | narrow mid-sweep check for the pipeline_tdd conversions | FACT: summary lines |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate; catches `needless_update` misconversions | FACT pass/fail |

All `cargo test` invocations tee to `target/test-output.log`; inspect the log, never re-run for output.

## Step Completion Expectations

- Step 1 writes the shared scratch state: `target/sweep-197-report.txt`, `target/sweep-197-{slicer-runtime,slicer-scheduler,slicer-wasm-host,pnp-cli}-baseline.txt`, `target/sweep-197-assert-baseline.txt`, `target/sweep-197-testattr-baseline.txt`. Implementation-time scratch, never committed, never quoted into the packet as frozen counts.
- A pre-existing red baseline test halts the packet (blocker), it is never swept over.
- If any runtime `executor`/`e2e` or wasm-host test fails at baseline or post, run `cargo xtask build-guests --check` (and rebuild if `STALE:`) before attributing the failure to the sweep — CLAUDE.md §Guest WASM Staleness. This packet's own edits cannot make guests stale (no guest-input path in scope), but predecessor packets' artifacts can be stale on disk.
- Steps 2-5 (per-crate sweeps) are order-independent; all follow Step 1 and precede Step 6.

## Context Discipline Notes

- `target/sweep-197-report.txt` may be large (biggest area): grep per crate/binary, never read whole.
- `crates/slicer-runtime` suite runs are the slowest in this batch (real WASM slicing in `executor`/`e2e` buckets); exactly two full runs are budgeted (baseline, post) — mid-sweep checks use single buckets (`--test integration` etc.).
- `crates/slicer-runtime/tests/common/mod.rs` is >500 lines: ranged reads around `pipeline_config_base` and the helpers the report names.
- Do not open packets 194/195's `design.md`/`implementation-plan.md`; their exports are restated here and in `design.md`.
