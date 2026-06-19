# Implementation Plan: support-validation-wedge-harness

## Execution Rules

- One atomic step at a time.
- Maps to `TASK-260`.
- Tests authored as RED (failing) only when the invariant fails on current planner output; for invariants where the current planner already satisfies the invariant, the test is written GREEN-on-first-run with a comment marking it as "passive verification" until a future change touches the same surface.
- Honors the context-discipline preamble shared with `swarm` and `spec-review`.

## Steps

### Step 1: Confirm prerequisites + locate fixture helpers + golden recipe convention

- Task IDs: `TASK-260`
- Objective: confirm Packet 2 (`117_support-planner-geometric-correctness`) is closed; locate the runtime tests' fixture-loading helpers; locate or design the golden-regen convention.
- Precondition: spec packets directory in known state.
- Postcondition: implementer has (a) confirmation Packet 2 closed, (b) names of fixture helpers, (c) golden-regen convention (existing xtask command or new subcommand).
- Files allowed to read:
  - `.ralph/specs/117_support-planner-geometric-correctness/packet.spec.md` — confirm `status: implemented` (FACT yes/no via dispatch).
  - `crates/slicer-runtime/tests/integration/main.rs` — fully.
  - `crates/slicer-runtime/tests/common/` — delegated LOCATIONS.
- Files allowed to edit (≤ 3): none in this step.
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` — never load
  - `modules/core-modules/support-planner/src/lib.rs` — out of scope for this packet
- Expected sub-agent dispatches:
  - "Check `status:` field of `.ralph/specs/117_support-planner-geometric-correctness/packet.spec.md`; return FACT (`implemented` / `draft` / other)." — purpose: gate prerequisites.
  - "Locate fixture-loading helpers (`cached_load_model`, `cached_run` or equivalents) in `crates/slicer-runtime/tests/common/`; return LOCATIONS ≤ 5 entries." — purpose: bootstrap pattern.
  - "Find the workspace's golden-regen convention. Either return LOCATIONS for an existing xtask subcommand (e.g., `xtask capture_goldens`), or return FACT `no convention` and suggest a target file path for a new subcommand." — purpose: confirm Step 5 surface.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C1, §Validation Strategy
- OrcaSlicer refs: none.
- Verification:
  - Packet 2 status = `implemented` (FACT pass).
  - Fixture helper names captured.
  - Golden-regen convention recorded.
- Exit condition: prerequisites confirmed.

### Step 2: Author `support_invariants_wedge_tdd.rs` skeleton + introspection helper

- Task IDs: `TASK-260`
- Objective: create the test file with seven `#[test]` functions stubbed + the `reconstruct_chains` helper.
- Precondition: Step 1 complete.
- Postcondition: file compiles; tests run (may fail because the planner currently violates AC-2 due to the geometric-correctness fix landing fresh — that's a real signal that needs resolving, not silenced by widening the assertion).
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanIR`, `SupportPlanEntry`, `Point3WithWidth` definitions only.
  - `docs/specs/support-modules-orca-port.md` §C1, §Validation Strategy.
  - `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs` — pattern reference (range-read).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs` (one-line registration)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-runtime/src/**` — production code; not edited.
  - `modules/core-modules/support-planner/src/lib.rs` — planner internals; not introspected.
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-runtime --tests`; return FACT pass/fail." — purpose: test file compiles.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` §C1
  - `docs/02_ir_schemas.md` §SupportPlanIR
- OrcaSlicer refs: none.
- Verification:
  - File compiles; seven test functions present.
- Exit condition: skeleton ready; runtime + run can be exercised.

### Step 3: Implement the six invariants + AC-N1; iterate to GREEN

- Task IDs: `TASK-260`
- Objective: fill in the seven test function bodies. Run each; if a test fails, decide whether the failure is (a) a real planner regression to surface to a sibling packet, or (b) an invariant misformulation. The packet's HARNESS GOAL is satisfied when each invariant test correctly asserts the right thing, even if the underlying planner produces a regression.
- Precondition: Step 2 complete.
- Postcondition: AC-1 through AC-6 plus AC-N1 are GREEN against the current planner OR a packet-author note in `requirements.md` Step Completion Expectations names the specific invariant that fails because of an upstream planner bug AND a sibling packet that will resolve it.
- Files allowed to read:
  - same as Step 2
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - same as Step 2
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT (per-test pass/fail); SNIPPETS ≤ 30 lines on failure." — purpose: gate each invariant.
- Context cost: `M`
- Authoritative docs: same as Step 2.
- OrcaSlicer refs: none.
- Verification:
  - All seven tests FACT GREEN OR documented upstream-bug exception.
- Exit condition: invariants verified.

### Step 4: Author `support_golden_regression_wedge_tdd.rs` + helpers

- Task IDs: `TASK-260`
- Objective: write the AC-7 + AC-N2 tests with the `endpoints_hausdorff` helper. These will run RED until Step 5 captures the goldens.
- Precondition: Step 3 complete.
- Postcondition: file compiles; tests run with file-not-found errors against the (not-yet-existing) goldens — confirmed RED.
- Files allowed to read:
  - `docs/specs/support-modules-orca-port.md` §Validation Strategy
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs` (one-line registration)
- Files explicitly out-of-bounds for this step:
  - same as Step 2
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT (expected: file-not-found failures)." — purpose: confirm RED state pre-capture.
- Context cost: `S`
- Authoritative docs: `docs/specs/support-modules-orca-port.md` §Validation Strategy
- OrcaSlicer refs: none.
- Verification:
  - File compiles; tests run; failure mode is "golden file missing", not "Hausdorff helper broken" or "tolerance arithmetic wrong".
- Exit condition: test code ready; golden capture is the only thing missing.

### Step 5: Capture initial goldens via the regen recipe

- Task IDs: `TASK-260`
- Objective: run the planner once on `regression_wedge.stl` and write the two golden files.
- Precondition: Step 4 complete; planner is the current post-Packet-2 implementation.
- Postcondition: `resources/golden/support_regression_wedge_branch_count.txt` and `..._endpoints.txt` exist with non-empty content.
- Files allowed to read:
  - `xtask/src/...` (or wherever the golden-regen recipe lands) — existing recipes for pattern reference.
- Files allowed to edit (≤ 3):
  - `xtask/src/...` — add or extend the capture command
  - `resources/golden/support_regression_wedge_branch_count.txt` (new)
  - `resources/golden/support_regression_wedge_endpoints.txt` (new)
- Files explicitly out-of-bounds for this step:
  - The planner — not edited.
- Expected sub-agent dispatches:
  - "Run the new xtask capture command on `regression_wedge.stl`; return FACT (the goldens were created, with branch count and endpoint count summaries). Do NOT paste file contents." — purpose: initial capture.
  - "Run `test -s resources/golden/support_regression_wedge_branch_count.txt && test -s resources/golden/support_regression_wedge_endpoints.txt`; return FACT pass/fail." — purpose: files exist + non-empty.
  - "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: confirm AC-7 GREEN after capture.
- Context cost: `M`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - Golden files present and non-empty.
  - AC-7 FACT GREEN.
- Exit condition: goldens captured; AC-7 passes.

### Step 6: Author AC-N2 drift-detection test against an in-memory mutated golden

- Task IDs: `TASK-260`
- Objective: complete AC-N2 — assert the harness DETECTS intentional drift, not silently accepts it.
- Precondition: Step 5 complete; AC-7 GREEN.
- Postcondition: AC-N2 GREEN.
- Files allowed to read: same as Step 4.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - the golden files — do NOT mutate them on disk; the test overlays an in-memory mutated value.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- detects_intentional_branch_count_drift`; return FACT pass/fail." — purpose: gate AC-N2.
- Context cost: `S`
- Authoritative docs: same as Step 1.
- OrcaSlicer refs: none.
- Verification:
  - AC-N2 FACT GREEN.
- Exit condition: AC-N2 passes.

### Step 7: Guest WASM staleness gate + final packet verification

- Task IDs: `TASK-260`
- Objective: confirm WASM artifacts current (tests need them); run full AC matrix; lint.
- Precondition: Steps 2-6 complete; all ACs GREEN locally.
- Postcondition: AC-1 through AC-7 + AC-N1 + AC-N2 all PASS; workspace clippy clean.
- Files allowed to read: none beyond prior steps.
- Files allowed to edit (≤ 3): none — verification only.
- Files explicitly out-of-bounds for this step:
  - `target/**`
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`)." — purpose: WASM ready.
  - "Run AC-1 through AC-7 + AC-N1 + AC-N2 commands sequentially; return FACT (PASS / FAIL list)." — packet-level gate.
  - "Run `cargo clippy -p slicer-runtime --all-targets -- -D warnings`; return FACT pass/fail." — lint gate.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - All AC commands PASS.
  - Clippy PASS.
- Exit condition: closure summary recorded; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Discovery via dispatches. |
| Step 2 | M | New test file + helper + module registration. |
| Step 3 | M | Iterate to GREEN on six invariants + AC-N1. |
| Step 4 | S | Golden-regression test as RED. |
| Step 5 | M | Initial golden capture via xtask. |
| Step 6 | S | AC-N2 drift detector. |
| Step 7 | S | Final verification. |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All seven steps complete; each exit condition met.
- AC-1 through AC-7 + AC-N1 + AC-N2 all PASS.
- Golden files committed to git.
- `cargo xtask build-guests --check` clean.
- `docs/07_implementation_status.md` marks `TASK-260` `[x]` (via worker dispatch).
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands: `cargo xtask build-guests --check`, the two test files, lint.
- Confirm goldens are committed and not gitignored.
- Confirm implementer's peak context usage stayed under 70%.
- Mark `TASK-260` `[x]`; transition `packet.spec.md` to `status: implemented`.
