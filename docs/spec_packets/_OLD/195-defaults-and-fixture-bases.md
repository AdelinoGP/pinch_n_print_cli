---
status: implemented
packet: 195-defaults-and-fixture-bases
task_ids:
  - TASK-317
---

# 195-defaults-and-fixture-bases

## Goal

Give every watched type that sweep packets 196–198 must convert a usable FRU base: a manual `impl Default` for `SliceRunOptions` (class a), `slicer_sdk::test_support` fixture bases for the unsafe-default IR types `PrintEntity`, `WallLoop`, and `OrderedEntityView` plus the ADR-0054/ADR-0004 addenda naming `sdk::test_support` the single IR-fixture home (class b), and per-crate `pipeline_config_base` helpers for the trait-object holder `PipelineConfig` (class c), with guest WASM rebuilt.

## Problem Statement

Packet 194's gate demands that test-code literals of watched types carry a `..` rest — but a rest needs a base expression, and several heavily-constructed watched types have none today. Re-derived 2026-08-07 against the tree:

- `SliceRunOptions` (`crates/slicer-runtime/src/run.rs`, 15 fields) has no `Default`; it is constructed in ≥ 10 `slicer-runtime` test files plus `crates/pnp-cli/src/main.rs` (production — stays exhaustive).
- `PrintEntity` and `WallLoop` (`crates/slicer-ir/src/slice_ir.rs`) *cannot* safely gain `Default`: `docs/specs/_OLD/default-builder-migration.md` §3.6 explicitly rejects `#[default]` for `ExtrusionRole`, `LoopType`, and `WallBoundaryType`, and `PrintEntity`'s rustdoc states it intentionally has no `Default` derive. They need fixture bases that take the unsafe enums explicitly.
- `OrderedEntityView` (`crates/slicer-sdk/src/views.rs`, 7 fields, carries `ExtrusionRole`) was on the orchestrator's class-(a) candidate list but fails the same §3.6 criterion — re-classified into class (b); it has 3 exhaustive literals in `crates/slicer-sdk/tests/layer_module_tdd.rs`.
- `PipelineConfig` (`crates/slicer-runtime/src/pipeline.rs`, 9 fields, holds `PipelineStageRunners` trait objects — §7.6 of the old spec) cannot have `Default` at all; ~14 literals in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`, more across `slicer-runtime` tests, and 6 in `crates/pnp-cli/tests/e2e_integration_tdd.rs`. It needs per-crate helper fns concentrating one waivered exhaustive literal each.
- Candidates dropped by the audit (zero test-code construction sites, 2026-08-07): `Diagnostic` (both the `slicer-ir` `stage_io.rs` and `slicer-sdk` `prepass_types.rs` types — all literals are production/guest src), `DeferredRetract`, `DeferredTravelMove` (only the `blackboard.rs` definitions and two production sites in `layer_executor.rs`).

Without this packet, sweep packets 196–198 would have nothing to write on the right of `..` for these types.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Of this packet's edits, only `crates/slicer-sdk/src/test_support/fixtures.rs` feeds the guest build (`crates/slicer-sdk/**` is a universal guest dep; the `test_support` module is cfg-gated out of guest code, but the freshness gate is mtime-based, so the rebuild is still mandatory). `slicer-runtime`, `pnp-cli`, docs, and ADRs are host-only.
- ADR-0004's negative consequence stands unchanged: guest builds must never enable `slicer-sdk`'s `test` feature; the new fixture fns live behind the existing gate and add nothing to production or guest surfaces.
- ADR-0054's constraints on `pnp-cli-locator` (std-only, dev-dep-only, host-side-only, exactly four functions) are untouched — this packet edits only its header rustdoc wording.
- Schema/version constants: `SliceRunOptions::default()` pins nothing new — `MeshIR::default()` already pins `CURRENT_MESH_IR_SCHEMA_VERSION` (existing manual impl in `crates/slicer-ir/src/slice_ir.rs`). No version constant is bumped anywhere in this packet.

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
