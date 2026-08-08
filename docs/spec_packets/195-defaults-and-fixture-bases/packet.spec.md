---
status: implemented
packet: 195-defaults-and-fixture-bases
task_ids:
  - TASK-317
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 195-defaults-and-fixture-bases

## Goal

Give every watched type that sweep packets 196–198 must convert a usable FRU base: a manual `impl Default` for `SliceRunOptions` (class a), `slicer_sdk::test_support` fixture bases for the unsafe-default IR types `PrintEntity`, `WallLoop`, and `OrderedEntityView` plus the ADR-0054/ADR-0004 addenda naming `sdk::test_support` the single IR-fixture home (class b), and per-crate `pipeline_config_base` helpers for the trait-object holder `PipelineConfig` (class c), with guest WASM rebuilt.

## Scope Boundaries

Three additive change classes plus two ADR amendments and one crate-header touch; no struct shapes change and no existing call site is converted (sweeps are packets 196–198). Downstream `Cargo.toml`s are untouched — the `slicer-sdk` dev-dep with `feature = "test"` is added by the sweep packets to the crates they convert; this packet edits only `crates/slicer-sdk/Cargo.toml` (a `[[test]]` entry for its own new test file).

## Prerequisites and Blockers

- Depends on: packet 194 (`cargo xtask check-literals --report` enumerates the audit in Step 1; the path filter proves this packet's new test files clean).
- Unblocks: packets 196, 197, 198 (they consume the exports below), transitively 199.
- Activation blockers: packet 194 not yet `implemented`.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** `SliceRunOptions` (`crates/slicer-runtime/src/run.rs`), **when** `SliceRunOptions::default()` is called, **then** it returns the quiet test baseline: `mesh` is `Arc<MeshIR>` with `schema_version == CURRENT_MESH_IR_SCHEMA_VERSION` and empty `objects`; `model_label` empty; `config_path`, `output_path`, `thumbnail`, `report`, `cancel_flag` all `None`; `module_dirs` and `config_overrides` empty; `no_default_module_paths`, `report_verbose`, `instrument_stderr`, `profile`, `profile_verbose`, and `progress_events` all `false`; and `SliceRunOptions { profile: true, ..Default::default() }` compiles (FRU usable). | `mkdir -p target && cargo test -p slicer-runtime --test unit slice_run_options_default 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** `slicer_sdk::test_support::fixtures::print_entity_base`, **when** called with `ExtrusionRole::SparseInfill`, **then** the returned `PrintEntity` has `entity_id == 0` (documented uninitialized sentinel), `path.points.len() == 1` (non-empty-path invariant), `path.role == ExtrusionRole::SparseInfill`, `role == ExtrusionRole::SparseInfill`, `path.speed_factor == 1.0`, `region_key == RegionKey::default()`, `topo_order == 0`, `tool_index == 0`. | `mkdir -p target && cargo test -p slicer-sdk --test test_support_fixture_bases_tdd print_entity_base 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** `slicer_sdk::test_support::fixtures::wall_loop_base`, **when** called with `(LoopType::Outer, WallBoundaryType::ExteriorSurface)` and with `(LoopType::Inner, WallBoundaryType::Interior)`, **then** each returned `WallLoop` has `perimeter_index == 0`, the given `loop_type` and `boundary_type` stored, a non-empty `path.points`, `width_profile.widths.len() == path.points.len()` (the "one width per vertex" convention used by the existing `PerimeterRegionViewBuilder::add_outer_wall`), empty `feature_flags`, and `path.role` mapped `Outer → OuterWall`, `ThinWall → ThinWall`, all other variants → `InnerWall`. | `mkdir -p target && cargo test -p slicer-sdk --test test_support_fixture_bases_tdd wall_loop_base 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** `slicer_sdk::test_support::fixtures::ordered_entity_view_base`, **when** called with `ExtrusionRole::OuterWall`, **then** the returned `slicer_sdk::views::OrderedEntityView` has `original_index == 0`, `tool_index == 0`, `region_key == RegionKey::default()`, `role == ExtrusionRole::OuterWall`, `start_point == Point3WithWidth::default()`, `end_point == Point3WithWidth::default()`, `point_count == 2`. | `mkdir -p target && cargo test -p slicer-sdk --test test_support_fixture_bases_tdd ordered_entity_view_base 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** `common::pipeline_config_base(mesh_ir, plan, runners)` in `crates/slicer-runtime/tests/common/mod.rs`, **when** the smoke test builds one with `ExecutionPlan::default()` and the Noop stage runners already defined in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`, **then** the returned `PipelineConfig` has `cancel_flag` `None`, empty `wasm_handles`, empty `resolved_configs`, and the passed `mesh_ir`/`plan`/`runners` installed. | `mkdir -p target && cargo test -p slicer-runtime --test integration pipeline_config_base_smoke 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** `crates/pnp-cli/tests/e2e_integration_tdd.rs`, **when** grepped and compiled, **then** it contains a file-local `fn pipeline_config_base` (marked `#[allow(dead_code)]` until sweep packet 197 converts the file's 6 `PipelineConfig` sites) whose single exhaustive `PipelineConfig` literal carries a `// exhaustive:` waiver, and `cargo check -p pnp-cli --tests` passes. | `rg -q 'fn pipeline_config_base' crates/pnp-cli/tests/e2e_integration_tdd.rs && rg -q '// exhaustive:' crates/pnp-cli/tests/e2e_integration_tdd.rs && cargo check -p pnp-cli --tests && echo PASS`
- **AC-7. Given** the ADR addenda, **when** grepped, **then** `docs/adr/0054-host-side-test-support-crate.md` and `docs/adr/0004-test-support-lives-in-slicer-sdk.md` each contain an amendment section with the phrase `single IR-fixture home`, and the header rustdoc of `crates/pnp-cli-locator/src/lib.rs` no longer scopes `slicer_sdk::test_support` as guest-side-only (it names it the IR-fixture home for host and guest tests while keeping the locator disjoint). | `rg -q 'single IR-fixture home' docs/adr/0054-host-side-test-support-crate.md && rg -q 'single IR-fixture home' docs/adr/0004-test-support-lives-in-slicer-sdk.md && rg -q 'IR-fixture home' crates/pnp-cli-locator/src/lib.rs && echo PASS`
- **AC-8. Given** the packet-194 gate, **when** run with path filters naming this packet's new test files, **then** it exits 0 (the new files contain zero violations). | `cargo xtask check-literals crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs crates/slicer-runtime/tests/unit/slice_run_options_default_tdd.rs; test $? -eq 0 && echo PASS`
- **AC-9. Given** the `crates/slicer-sdk/src/test_support/fixtures.rs` edit (a guest-feeding path), **when** the freshness gate runs at close, **then** it reports clean. | `cargo xtask build-guests --check; test $? -eq 0 && echo PASS`

## Negative Test Cases

- **AC-N1. Given** the unsafe-default rule (`docs/specs/_OLD/default-builder-migration.md` §3.6 rejects `#[default]` for `ExtrusionRole`, `LoopType`, `WallBoundaryType`), **when** the tree is grepped after this packet, **then** `PrintEntity` and `WallLoop` still have no `Default` (impl or derive) and `PrintEntity`'s "intentionally has no `Default` derive" doc comment survives. | `! rg -q 'impl Default for PrintEntity|impl Default for WallLoop' crates && rg -q 'intentionally has no .Default. derive' crates/slicer-ir/src/slice_ir.rs && echo PASS`
- **AC-N2. Given** the re-derived audit (2026-08-07: zero test-code construction sites), **when** the tree is grepped after this packet, **then** `Diagnostic` (both `crates/slicer-ir/src/stage_io.rs` and `crates/slicer-sdk/src/prepass_types.rs`), `DeferredRetract`, and `DeferredTravelMove` (`crates/slicer-runtime/src/blackboard.rs`) have gained no `Default`. | `! rg -q 'impl Default for Diagnostic|impl Default for DeferredRetract|impl Default for DeferredTravelMove' crates && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (must be clean at close; rebuild without `--check` first if `STALE:`)

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - 89 lines; direct read (locked decision 3 governs the three classes).
- `docs/specs/_OLD/default-builder-migration.md` - 1449 lines; ranged reads ONLY: §3.6 (enum-default safety, lines ~176-198) and §5 intro (manual-`impl Default` pattern pinning `CURRENT_*` consts, lines ~308-330); line hints re-verified 2026-08-07.
- `docs/adr/0054-host-side-test-support-crate.md` - 186 lines; direct read (amendment target).
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - 72 lines; direct read (amendment target).
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; read to keep fixture-policy wording consistent.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/adr/0054-host-side-test-support-crate.md` new amendment section (sdk::test_support becomes the single IR-fixture home for host- and guest-side tests; locator stays binary-location-only) - `rg -q 'single IR-fixture home' docs/adr/0054-host-side-test-support-crate.md`
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` new amendment section (scope extension to host-side IR fixtures; guests still never enable `feature = "test"`) - `rg -q 'single IR-fixture home' docs/adr/0004-test-support-lives-in-slicer-sdk.md`

Doc greps are appended to the ACs (AC-7). No other docs change: the rule page `docs/21_data_defaults_and_fixtures.md` already points here by design (packet 194 authored the pointer).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
