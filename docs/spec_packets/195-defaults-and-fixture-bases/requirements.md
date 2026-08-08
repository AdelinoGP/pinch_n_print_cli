# Requirements: 195-defaults-and-fixture-bases

## Packet Metadata

- Grouped task IDs: `TASK-317` (new row; registered in `docs/07_implementation_status.md` by the implementing swarm — see `task-map.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 194's gate demands that test-code literals of watched types carry a `..` rest — but a rest needs a base expression, and several heavily-constructed watched types have none today. Re-derived 2026-08-07 against the tree:

- `SliceRunOptions` (`crates/slicer-runtime/src/run.rs`, 15 fields) has no `Default`; it is constructed in ≥ 10 `slicer-runtime` test files plus `crates/pnp-cli/src/main.rs` (production — stays exhaustive).
- `PrintEntity` and `WallLoop` (`crates/slicer-ir/src/slice_ir.rs`) *cannot* safely gain `Default`: `docs/specs/_OLD/default-builder-migration.md` §3.6 explicitly rejects `#[default]` for `ExtrusionRole`, `LoopType`, and `WallBoundaryType`, and `PrintEntity`'s rustdoc states it intentionally has no `Default` derive. They need fixture bases that take the unsafe enums explicitly.
- `OrderedEntityView` (`crates/slicer-sdk/src/views.rs`, 7 fields, carries `ExtrusionRole`) was on the orchestrator's class-(a) candidate list but fails the same §3.6 criterion — re-classified into class (b); it has 3 exhaustive literals in `crates/slicer-sdk/tests/layer_module_tdd.rs`.
- `PipelineConfig` (`crates/slicer-runtime/src/pipeline.rs`, 9 fields, holds `PipelineStageRunners` trait objects — §7.6 of the old spec) cannot have `Default` at all; ~14 literals in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`, more across `slicer-runtime` tests, and 6 in `crates/pnp-cli/tests/e2e_integration_tdd.rs`. It needs per-crate helper fns concentrating one waivered exhaustive literal each.
- Candidates dropped by the audit (zero test-code construction sites, 2026-08-07): `Diagnostic` (both the `slicer-ir` `stage_io.rs` and `slicer-sdk` `prepass_types.rs` types — all literals are production/guest src), `DeferredRetract`, `DeferredTravelMove` (only the `blackboard.rs` definitions and two production sites in `layer_executor.rs`).

Without this packet, sweep packets 196–198 would have nothing to write on the right of `..` for these types.

## In Scope

- **Class (a) — safe `Default`:** manual `impl Default for SliceRunOptions` in `crates/slicer-runtime/src/run.rs`, following the old spec's §5 Bucket-B pattern (explicit per-field values; `mesh: Arc::new(MeshIR::default())` pins `CURRENT_MESH_IR_SCHEMA_VERSION` via the existing `impl Default for MeshIR`). `progress_events` defaults to `false` — the quiet test baseline — with a rustdoc note that this deliberately differs from `pnp_cli slice`'s CLI default (`true`); `pnp_cli` sets every field explicitly and is unaffected. A Step-1 audit re-derives the class-(a) list from `cargo xtask check-literals --report`; any additional no-`Default` watched type with test sites is admitted only if it passes §3.6/§5 criteria, otherwise it is recorded for the sweeps to waiver.
- **Class (b) — fixture bases** in `crates/slicer-sdk/src/test_support/fixtures.rs` (gated module; feature `test`), alongside the existing `print_entity`/`tool_change`/`seam_candidate` helpers:
  - `pub fn print_entity_base(role: ExtrusionRole) -> PrintEntity`
  - `pub fn wall_loop_base(loop_type: LoopType, boundary_type: WallBoundaryType) -> WallLoop`
  - `pub fn ordered_entity_view_base(role: ExtrusionRole) -> OrderedEntityView`
  Signatures take the unsafe-enum values explicitly; everything else is defaulted (exact field values in the ACs). New gated test file `crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs` plus its `[[test]] required-features = ["test"]` entry in `crates/slicer-sdk/Cargo.toml`.
- **ADR addenda (class b policy):** amendment sections in ADR-0054 and ADR-0004 naming `slicer_sdk::test_support` the **single IR-fixture home** for both host- and guest-side tests (host crates consume it via a `slicer-sdk` dev-dep with `feature = "test"`, added by the sweep packets); `pnp-cli-locator` stays std-only, dev-dep-only, binary-location-only. Touch the header rustdoc of `crates/pnp-cli-locator/src/lib.rs` (its "lives guest-side" / "disjoint surfaces" wording) to match. The existing host-side `WallLoopBuilder` in `crates/slicer-runtime/tests/common/ir_builders.rs` is noted in the ADR-0054 amendment as a consolidation target for the sweeps — not migrated here.
- **Class (c) — per-crate `PipelineConfig` helpers:**
  - `pub fn pipeline_config_base(mesh_ir: Arc<MeshIR>, plan: ExecutionPlan, runners: PipelineStageRunners) -> PipelineConfig` in `crates/slicer-runtime/tests/common/mod.rs` (the mod already opens with `#![allow(dead_code)]`), defaulting `cancel_flag: None`, `support_tools: SupportToolSelection::default()`, `resolved_configs`/`wasm_handles` empty, `default_resolved_config: Arc::new(ResolvedConfig::default())`, `bounds: Arc::new(ConfigBoundsIndex::empty())`; its single exhaustive `PipelineConfig` literal carries a `// exhaustive: <reason>` waiver. Smoke test `pipeline_config_base_smoke` appended to `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`, reusing that file's existing Noop stage runners.
  - A file-local twin in `crates/pnp-cli/tests/e2e_integration_tdd.rs` (`#[allow(dead_code)]` until sweep packet 197 converts the file's sites), mirroring the imports already present in that file.
- Guest WASM rebuild: the `fixtures.rs` edit touches `crates/slicer-sdk/src/**`, a guest-feeding path — `cargo xtask build-guests --check` and rebuild are owned by the step that edits it.
- Compile blast radius: `cargo check --workspace --all-targets` at close (adding `Default` does not change struct shape, but every new fn and impl must compile everywhere).

## Out of Scope

- Converting any existing call site to FRU or to the new helpers/bases (packets 196–198), including the 3 `OrderedEntityView` literals in `layer_module_tdd.rs` and all `PipelineConfig` literals.
- Adding the `slicer-sdk` dev-dep (`feature = "test"`) to any downstream crate (sweep packets own it for the crates they touch).
- `Default` impls for `PrintEntity`, `WallLoop`, `OrderedEntityView`, `Diagnostic` (either crate), `DeferredRetract`, `DeferredTravelMove`, `PipelineConfig`, or any enum `#[default]` rejected by §3.6.
- Migrating or deleting `WallLoopBuilder` (`crates/slicer-runtime/tests/common/ir_builders.rs`).
- Wiring/enforcement changes (packet 199); edits to `docs/21_data_defaults_and_fixtures.md` (packet 194 authored it with the fixture-policy pointer already in place).

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - 89 lines; direct read (locked decision 3).
- `docs/specs/_OLD/default-builder-migration.md` - 1449 lines; ranged reads only (§3.6 ~176-198, §5 intro ~308-330); never open in full.
- `docs/adr/0054-host-side-test-support-crate.md` - 186 lines; direct read.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - 72 lines; direct read.
- `docs/02_ir_schemas.md` - delegate a FACT check only if an IR field meaning is in doubt; not expected.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`.
- Negative: `AC-N1` (unsafe types stay `Default`-less), `AC-N2` (dropped candidates stay untouched).
- Cross-packet impact: packets 196–198 consume the exact exported names/signatures (`print_entity_base`, `wall_loop_base`, `ordered_entity_view_base`, `pipeline_config_base`, `SliceRunOptions::default`) and the ADR policy; renaming any of them later invalidates those packets.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-runtime --test unit slice_run_options_default 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-sdk --test test_support_fixture_bases_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2/3/4 (whole new file) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration pipeline_config_base_smoke 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-5 | FACT pass/fail |
| `rg -q 'fn pipeline_config_base' crates/pnp-cli/tests/e2e_integration_tdd.rs && rg -q '// exhaustive:' crates/pnp-cli/tests/e2e_integration_tdd.rs && cargo check -p pnp-cli --tests && echo OK` | AC-6 | FACT pass/fail |
| `rg -q 'single IR-fixture home' docs/adr/0054-host-side-test-support-crate.md && rg -q 'single IR-fixture home' docs/adr/0004-test-support-lives-in-slicer-sdk.md && rg -q 'IR-fixture home' crates/pnp-cli-locator/src/lib.rs && echo OK` | AC-7 | FACT pass/fail |
| `cargo xtask check-literals crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs crates/slicer-runtime/tests/unit/slice_run_options_default_tdd.rs; echo "exit=$?"` | AC-8 (expect `exit=0`) | FACT single line |
| `cargo xtask build-guests --check; echo "exit=$?"` | AC-9 (expect `exit=0`; rebuild first if `STALE:`) | FACT single line |
| `! rg -q 'impl Default for PrintEntity\|impl Default for WallLoop' crates && rg -q 'intentionally has no .Default. derive' crates/slicer-ir/src/slice_ir.rs && echo OK` | AC-N1 | FACT pass/fail |
| `! rg -q 'impl Default for Diagnostic\|impl Default for DeferredRetract\|impl Default for DeferredTravelMove' crates && echo OK` | AC-N2 | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile blast radius | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- Step 1's audit (via packet 194's `--report`) must confirm or amend the class lists **before** any code step; a genuinely new class-(a) admission requires the §3.6/§5 check recorded in the step's output.
- The `fixtures.rs` edit (Step 3) is the only guest-feeding change; its step owns `cargo xtask build-guests --check` + rebuild, and AC-9 re-confirms at close.
- AC-8 runs only after packet 194's binary exists in the tree (it does — 194 precedes this packet); if `check-literals` is missing, the dependency order was violated: stop and report.

## Context Discipline Notes

- `docs/specs/_OLD/default-builder-migration.md` is 1449 lines: ranged reads of §3.6 and §5 only; delegate anything else about it as a SUMMARY dispatch.
- `crates/slicer-ir/src/slice_ir.rs` is >2300 lines and `crates/slicer-sdk/src/test_support/fixtures.rs` is 1136 lines: ranged reads only (struct definitions and the helper-fn region around the existing `print_entity`).
- `cargo xtask check-literals --report` output may exceed 200 lines: always filter (`grep -c`, `grep '^crates/<crate>'`, `tail -1`) — never absorb the full list.
- All `cargo test` runs tee to `target/test-output.log`; read the log on failure instead of re-running.
