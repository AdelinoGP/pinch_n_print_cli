---
status: draft
packet: 80
task_ids: [TASK-229, TASK-230]
requires: [79]
backlog_source: docs/07_implementation_status.md
---

# Packet 80 — Relocate Misplaced Module Tests; Annotate Legitimately-Runtime Module-Referencing Tests

## Goal

Move the two `slicer-runtime/tests/executor/` files whose system-under-test is a core-module (`wipe_tower_bed_bounds.rs` → `wipe-tower/tests/`, `prepass_support_generation_orca_parity_tdd.rs` → `support-planner/tests/`) into their respective module crates, switching the support-planner test from manual `host::test_support::install_log_capture` setup to `#[module_test]` so the relocation directly exercises the seam packet 77 created. Annotate the three other runtime-located tests that import a module by name (`slicing_promotion_e2e_regression_tdd`, `gcode_part_cooling_emission_tdd`, `gcode_skirt_brim_emission_tdd`) with `// NOT RELOCATABLE — SUT is <runtime symbol>, module is fixture input` comments so future agents do not re-litigate.

## Scope Boundaries

Two relocations + three annotations. The relocation targets were identified by recon: both import only `slicer_ir` + `slicer_sdk` (+ the module) — no `slicer_runtime::*` symbols. After relocation they use packet 79's `LayerCollectionFixtureBuilder` + `tool_change(...)` (`wipe-tower`) and packet 77's `#[module_test]` macro (`support-planner`). The three annotated tests use real `slicer_runtime::*` symbols (`commit_*_builtin`, `Blackboard`, `DefaultGCodeEmitter`, `DefaultGCodeSerializer`) and legitimately belong where they are; the annotation prevents future "relocate everything" sweeps. The runtime test-bucket aggregator files at `crates/slicer-runtime/tests/executor/main.rs:36,42` lose two `mod` declarations (lines confirmed by recon). The integration aggregator at `crates/slicer-runtime/tests/integration/main.rs:21,23` is untouched. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 79 implemented**. The wipe-tower relocation depends on `LayerCollectionFixtureBuilder` + `tool_change(...)` from packet 79; the support-planner relocation uses `#[module_test]` from packet 77 and `slicer_sdk::test_prelude::*` from packet 78, but its closure is paired with the wipe-tower relocation here for cohesion.
- Closure requires `cargo xtask build-guests --check` clean (rebuild if stale).

## Acceptance Criteria

### AC-1 — `wipe_tower_bed_bounds.rs` no longer exists in `slicer-runtime/tests/executor/`; exists in `wipe-tower/tests/`

**Given** the relocation,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs` is true; `test -f modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` is true; the relocated file declares `use slicer_sdk::test_prelude::*;` (or imports the specific helpers it needs from there) AND does NOT import any `slicer_runtime::*` symbol.

| `test ! -f crates/slicer-runtime/tests/executor/wipe_tower_bed_bounds.rs && test -f modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs && grep -qE 'use slicer_sdk::test_prelude|use slicer_sdk::test_support' modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs && ! grep -qE 'use slicer_runtime::' modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`

### AC-2 — `prepass_support_generation_orca_parity_tdd.rs` no longer exists in `slicer-runtime/tests/executor/`; exists in `support-planner/tests/` and uses `#[module_test]`

**Given** the relocation,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs` is true; `test -f modules/core-modules/support-planner/tests/orca_parity_tdd.rs` is true; the relocated file uses `#[module_test]` on at least the one test that previously did `log_test_support::install_log_capture()` manually (a `grep -c '#\[module_test\]'` returns ≥ 1); the manual `install_log_capture` setup is removed (because `#[module_test]`'s `mock_host_setup` hook from packet 77 covers it).

| `test ! -f crates/slicer-runtime/tests/executor/prepass_support_generation_orca_parity_tdd.rs && test -f modules/core-modules/support-planner/tests/orca_parity_tdd.rs && [ $(grep -c '#\[module_test\]' modules/core-modules/support-planner/tests/orca_parity_tdd.rs) -ge 1 ] && ! grep -qE 'log_test_support::install_log_capture|test_support::install_log_capture' modules/core-modules/support-planner/tests/orca_parity_tdd.rs`

### AC-3 — `crates/slicer-runtime/tests/executor/main.rs` aggregator no longer declares the two moved tests

**Given** the aggregator update,
**When** `crates/slicer-runtime/tests/executor/main.rs` is grepped,
**Then** it contains no `mod wipe_tower_bed_bounds;` line and no `mod prepass_support_generation_orca_parity_tdd;` line. Other `mod` declarations in the file are unchanged.

| `! grep -qE '^mod wipe_tower_bed_bounds;' crates/slicer-runtime/tests/executor/main.rs && ! grep -qE '^mod prepass_support_generation_orca_parity_tdd;' crates/slicer-runtime/tests/executor/main.rs`

### AC-4 — `cargo test -p wipe-tower` passes with the relocated test included

**Given** the wipe-tower relocation,
**When** `cargo test -p wipe-tower --test bed_bounds_tdd` runs,
**Then** the relocated test's assertions (bed-containment per packet 58 AC6, the deferred object-footprint scope) pass. The pre-relocation test count in `slicer-runtime/tests/executor/` decreases by N tests; the wipe-tower per-package test count increases by N tests. The implementation log records both counts.

| `cargo test -p wipe-tower --test bed_bounds_tdd`

### AC-5 — `cargo test -p support-planner` passes — first test in this module

**Given** the support-planner relocation (note: `support-planner` previously had no tests),
**When** `cargo test -p support-planner` runs,
**Then** the relocated test (which previously lived as `prepass_support_generation_orca_parity_tdd.rs` in runtime) passes with all original assertions preserved AND `support-planner/Cargo.toml` now contains the dev-dep `slicer-sdk = { ..., features = ["test"] }` (because the test now uses `slicer_sdk::test_prelude::*` and `#[module_test]`).

| `cargo test -p support-planner && grep -A5 '^\[dev-dependencies\]' modules/core-modules/support-planner/Cargo.toml | grep -qE 'slicer-sdk.*features = \[.*"test".*\]'`

### AC-6 — Three legitimately-runtime tests each carry a top-of-file `NOT RELOCATABLE` comment naming the runtime SUT

**Given** the three remaining tests that import a module by name (`slicing_promotion_e2e_regression_tdd.rs`, `gcode_part_cooling_emission_tdd.rs`, `gcode_skirt_brim_emission_tdd.rs`),
**When** each file's first 25 lines are inspected,
**Then** each contains a comment matching the pattern `// NOT RELOCATABLE — SUT is <runtime symbol>, module <name> is fixture input` (or equivalent — the literal substring `NOT RELOCATABLE` and the named runtime symbol must appear; one-line variants are acceptable). Specifically: `slicing_promotion_e2e_regression_tdd` names `commit_shell_classification_builtin` or `commit_slice_builtin` or `Blackboard`; `gcode_part_cooling_emission_tdd` and `gcode_skirt_brim_emission_tdd` each name `DefaultGCodeEmitter` or `DefaultGCodeSerializer` or `Blackboard`.

| `for f in slicing_promotion_e2e_regression_tdd gcode_part_cooling_emission_tdd gcode_skirt_brim_emission_tdd; do head -25 crates/slicer-runtime/tests/executor/$f.rs crates/slicer-runtime/tests/integration/$f.rs 2>/dev/null | grep -qE 'NOT RELOCATABLE' || exit 1; done; head -25 crates/slicer-runtime/tests/executor/slicing_promotion_e2e_regression_tdd.rs | grep -qE 'commit_(shell_classification|slice)_builtin|Blackboard' && head -25 crates/slicer-runtime/tests/integration/gcode_part_cooling_emission_tdd.rs | grep -qE 'DefaultGCodeEmitter|DefaultGCodeSerializer|Blackboard' && head -25 crates/slicer-runtime/tests/integration/gcode_skirt_brim_emission_tdd.rs | grep -qE 'DefaultGCodeEmitter|DefaultGCodeSerializer|Blackboard'`

### AC-7 — `cargo test -p slicer-runtime` still passes — the moved tests are gone but nothing else regresses

**Given** the moves and aggregator updates,
**When** `cargo test -p slicer-runtime` runs (one of the largest test sweeps in the workspace because it bundles unit + contract + executor + integration + e2e),
**Then** all remaining `slicer-runtime` tests pass with zero regressions vs the pre-packet-80 count. The implementation log records the pre/post test counts; the delta is `-N_wipe_tower - N_orca_parity` (the count of tests that moved out).

| `cargo test -p slicer-runtime`

## Negative Test Cases

### AC-N1 — No `use wipe_tower::` or `use support_planner::` imports remain in `crates/slicer-runtime/tests/`

**Given** the relocations,
**When** `rg "use (wipe_tower|support_planner)::" crates/slicer-runtime/tests/` runs,
**Then** the result is empty. This is the structural signal that the relocation is complete — if any test file in runtime/tests/ still imports those modules, it should either (a) have been relocated too, or (b) use a different runtime SUT and import the module legitimately as fixture input (in which case it gets the AC-6 annotation).

| `! rg "use (wipe_tower|support_planner)::" crates/slicer-runtime/tests/ 2>/dev/null`

### AC-N2 — The relocated support-planner test would NOT compile without packet 77's `#[module_test]` wiring

**Given** the support-planner test uses `#[module_test]`,
**When** the implementer temporarily reverts `slicer_sdk::test_support::reset_global_state` to a stub (in a working-tree-only experiment), the relocated test compiles AND runs but its assertion-based check on `take_log_messages()` returns the prior test's leftover logs instead of empty,
**Then** the implementer reverts the experiment; the actual relocated test relies on `reset_global_state`'s implementation from packet 77 step 2 to drain log capture between tests. This is documented in `implementation-plan.md` step "Verify packet 77 hook is load-bearing".

| (Manual implementer ceremony documented in `implementation-plan.md`. Not CI-gated.)

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p wipe-tower -p support-planner -p slicer-runtime`
4. `cargo xtask build-guests --check` (rebuild if STALE)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/05_module_sdk.md` (post-packets-77/78/79) — §Test Support documents `#[module_test]` and the test prelude that the relocated support-planner test uses. No change in this packet.
- `docs/02_ir_schemas.md` — IR-12 LayerCollectionIR (the wipe-tower test's fixture). Read only for the field list when reviewing the relocation; no change.
- `CLAUDE.md` (project root) — §Test Discipline. No change.

## Doc Impact Statement

No doc files are edited by this packet. The relocation is purely structural; the test files themselves move, the aggregator loses two lines, three runtime tests get header comments. No section in `docs/05_module_sdk.md` describes the runtime/test layout that's being adjusted. The user has flagged that `GCodeEmitter` may move to its own crate in a future packet; that future packet would relocate the two remaining `gcode_*_emission` runtime tests, but P80 documents the current state without pre-empting.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
