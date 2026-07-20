# Implementation Plan: support-plan-raft-plan-and-raftinfill-role

## Execution Rules

- One atomic step at a time.
- Maps to `TASK-289` (renumbered from source-plan `TASK-265` + `TASK-266`; both IDs are reused by unrelated work in `docs/07_implementation_status.md`).
- IR change is load-bearing first; subsequent steps depend on it being durable.

## Steps

### Step 1: Confirm post-74710fa state of dispatcher + IR + WIT mirror

- Task IDs: `TASK-289`
- Files allowed to read:
  - `docs/specs/support-modules-orca-port.md` §C6, §C7
  - `docs/adr/0009-raft-as-layer-infill-role.md` — directly
  - `docs/specs/raft-default-module.md` — directly
- Sub-agent dispatches:
  - "Locate `should_emit` function in `crates/slicer-sdk/src/views.rs`; return SNIPPETS ≤ 30 lines showing the `match role` arms and surrounding context." — purpose: confirm Step 3 edit site.
  - "Locate `SupportPlanIR.schema_version` literal value in `crates/slicer-ir/src/slice_ir.rs`; return FACT (the SemVer value)." — purpose: Step 2 bump arithmetic.
  - "Locate `SupportPlanEntry`, `SupportPlanIR`, `Point3WithWidth`, `ExtrusionRole` definitions in `crates/slicer-ir/src/slice_ir.rs`; return LOCATIONS file:line." — purpose: edit targets.
  - "Confirm `ExtrusionRole` is mirrored in `crates/slicer-schema/wit/deps/types.wit` (NOT `ir-types.wit`); return SNIPPETS ≤ 20 lines showing the WIT variant." — purpose: Step 3 WIT edit site.
  - "Locate the degenerate raft block in `modules/core-modules/support-planner/src/lib.rs` (lines 442-491); return LOCATIONS ≤ 5 entries." — purpose: confirm Step 4 edit site.
  - "Summarize OrcaSlicer `generate_raft_base` from `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp`; return SUMMARY ≤ 200 words." — purpose: confirm data field semantics.
- Files allowed to edit: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C6, §C7
  - `docs/adr/0009-raft-as-layer-infill-role.md`
- OrcaSlicer refs: delegate per obligations.
- Verification: implementer can recite (a) the dispatcher location, (b) the current schema_version, (c) the degenerate block location, (d) the WIT mirror file.
- Exit condition: discovery captured.

### Step 2: Add `RaftPlan`, `RaftLayerSpec`, `ExtrusionRole::RaftInfill` to `slicer-ir`; bump schema_version

- Task IDs: `TASK-289`
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — at the located lines.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` (new)
- Files out-of-bounds: SDK, planner, runtime tests (later steps).
- Sub-agent dispatches:
  - "Run `cargo build -p slicer-ir`; return FACT pass/fail."
  - "Run `cargo test -p slicer-ir --test support_plan_ir_schema_version_bumped`; return FACT pass/fail."
  - "Run `cargo build --workspace`; return FACT pass/fail; SNIPPETS ≤ 30 lines FIRST error."
- Context cost: `M`
- Verification: AC-1, AC-2, AC-7 PASS; workspace compiles.
- Exit condition: IR additions durable.

### Step 3: Add `claim:raft-fill` arm to `should_emit`; update WIT `ExtrusionRole` mirror; rebuild guests

- Task IDs: `TASK-289`
- Files allowed to read:
  - `crates/slicer-sdk/src/views.rs` — `should_emit` location
  - `crates/slicer-schema/wit/deps/types.wit` — the WIT mirror
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-schema/wit/deps/types.wit`
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` (new; or extension)
- Files out-of-bounds: planner, runtime tests.
- Sub-agent dispatches:
  - "Locate every `match role` site that switches on `ExtrusionRole` workspace-wide; return LOCATIONS ≤ 20 entries." — purpose: exhaustive update audit.
  - "Run `cargo xtask build-guests`; return FACT pass/fail. Do NOT paste rebuild log." — purpose: rebuild after WIT enum addition.
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE."
  - "Run `cargo test -p slicer-sdk --test should_emit_raft_fill_claim`; return FACT pass/fail."
  - "Run `cargo build --workspace`; return FACT pass/fail."
- Context cost: `M`
- Verification: AC-3, AC-11, AC-N2 PASS; workspace compiles; guests fresh.
- Exit condition: role/claim machinery live.

### Step 4: Replace degenerate raft block with `RaftPlan` emission in `support-planner`

- Task IDs: `TASK-289`
- Files allowed to read:
  - `modules/core-modules/support-planner/src/lib.rs` — degenerate block + `plan_for_object` context
  - existing config-read site for `support_raft_layers` and friends (line 160)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files out-of-bounds: traditional-support (Step 6), docs (Step 7).
- Sub-agent dispatches:
  - "Find tests that reference the degenerate raft single-point in `modules/core-modules/support-planner/tests/`; return LOCATIONS." — purpose: migrate or invalidate.
  - "Run `cargo build -p support-planner`; return FACT pass/fail."
  - "Run `cargo test -p support-planner`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: existing tests don't regress.
  - "Run AC-6 grep; return FACT pass/fail." — purpose: degenerate gone.
- Context cost: `M`
- Verification: AC-6 PASS; planner compiles; existing tests pass.
- Exit condition: degenerate gone; new emission in place.

### Step 5: Author `raft_plan_emission_tdd.rs` integration tests (AC-4, AC-5, AC-N1) + AC-N3 WIT round-trip; register new test in the integration aggregator

- Task IDs: `TASK-289`
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/` patterns
  - existing integration test setup
  - `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs` (range-read; 410 lines)
  - `crates/slicer-runtime/tests/integration/main.rs` (the aggregator)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs` (add `mod raft_plan_emission_tdd;` after the existing `support_golden_regression_wedge_tdd` and `support_invariants_wedge_tdd` lines; the new test would otherwise compile in isolation but never run via `cargo test --test integration raft_plan_*` — silent 0-tests-run false pass)
- Files out-of-bounds: planner (Step 4 owns).
- Sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test raft_plan_emission_tdd`; return FACT per-test pass/fail; SNIPPETS ≤ 30 lines on failure."
  - "Run `cargo test -p slicer-wasm-host --test wit_boundary_tdd`; return FACT pass/fail." — purpose: AC-N3 WIT round-trip.
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE."
- Context cost: `M`
- Verification: AC-4, AC-5, AC-N1, AC-N3 PASS.
- Exit condition: integration coverage in place; WIT round-trip green; aggregator registers the new test (otherwise the test is dead code per S7 test-target-wiring rule).

### Step 6: Add traditional-support non-consumption sentence; verify manifest clean

- Task IDs: `TASK-289`
- Files allowed to read:
  - `modules/core-modules/traditional-support/src/lib.rs` lead `//!` block
  - `modules/core-modules/traditional-support/traditional-support.toml`
- Files allowed to edit (≤ 3):
  - `modules/core-modules/traditional-support/src/lib.rs`
- Files out-of-bounds: other modules.
- Sub-agent dispatches:
  - "Run AC-8 grep; return FACT pass/fail."
  - "Run AC-9 grep; return FACT pass/fail." — purpose: confirm manifest already clean.
  - "Run `cargo build -p traditional-support`; return FACT pass/fail."
- Context cost: `S`
- Verification: AC-8, AC-9 PASS.
- Exit condition: doc + manifest aligned.

### Step 7: Update `docs/02_ir_schemas.md` §SupportPlanIR per Doc Impact

- Task IDs: `TASK-289`
- Files allowed to read:
  - `docs/02_ir_schemas.md` §SupportPlanIR
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`
- Files out-of-bounds: other docs.
- Sub-agent dispatches:
  - "Run AC-10 grep; return FACT pass/fail."
- Context cost: `S`
- Verification: AC-10 PASS.
- Exit condition: docs updated.

### Step 8: Final packet verification + close

- Task IDs: `TASK-289`
- Files allowed to read: none beyond prior.
- Files allowed to edit: none.
- Sub-agent dispatches:
  - "Run all AC commands sequentially; return FACT (PASS / FAIL list)."
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail; SNIPPETS ≤ 20 lines FIRST error."
  - "Run `cargo xtask build-guests --check`; return FACT clean."
- Context cost: `S`
- Verification: all ACs PASS; clippy clean; guests fresh.
- Exit condition: closure summary recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery dispatches. |
| Step 2 | M | IR additions + schema bump. |
| Step 3 | M | SDK + WIT + guest rebuild. |
| Step 4 | M | Planner degenerate block removal + new emission. |
| Step 5 | M | Integration tests + WIT round-trip. |
| Step 6 | S | Doc + manifest verification. |
| Step 7 | S | Docs/02 update. |
| Step 8 | S | Final verification. |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All eight steps complete; every exit condition met.
- AC-1 through AC-11 + AC-N1 + AC-N2 + AC-N3 all PASS.
- `docs/02_ir_schemas.md` updated; AC-10 grep PASS.
- `docs/07_implementation_status.md` marks the new `TASK-289` row as `[x]` (via worker dispatch).
- `cargo xtask build-guests --check` clean.
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands green: `cargo xtask build-guests --check`, `cargo build --workspace`, the three new test files, `cargo clippy --workspace --all-targets -- -D warnings`.
- Mark `TASK-289` `[x]`; transition `packet.spec.md` to `status: implemented`.
