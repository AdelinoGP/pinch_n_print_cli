# Implementation Plan: 205e-integrated-parity-harness

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-331`.
- Preserve all current test names and failure assertions unless a mechanical helper migration requires a documented path-only rename.

## Steps

### Step 1: Inventory parity tests and comparator self-tests

- Task IDs: `TASK-331`
- Objective: record the 21 module tests, stage families, helper duplication, comparator families, and negative self-test names.
- Precondition: 205c and 205d are implemented.
- Postcondition: the migration matrix distinguishes module-specific setup from shared setup.
- Files allowed to read: `tests/contract/main.rs:20-50`; representative parity files; `common/mod.rs:1-40,455-495`; comparator/self-test locations.
- Files allowed to edit: none.
- Files explicitly out of bounds: production code, registry, WIT, Orca source.
- Expected sub-agent dispatches: structural inventory `SUMMARY`; self-test inventory `LOCATIONS`.
- Context cost: `S`.
- Authoritative docs: ADR-0042 and ADR-0056.
- Verification: `rg -c '^mod integrated_parity_.*_tdd;' crates/slicer-runtime/tests/contract/main.rs` and `rg -n 'parity_comparator_rejects|accepts_ulp' crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`.
- Exit condition: the pre-migration inventory is recorded in the implementation worktree before helper edits.

### Step 2: Add the family-aware harness and migrate representative families

- Task IDs: `TASK-331`
- Objective: add shared execution setup and migrate one layer, one prepass, one finalization, and one postpass parity file.
- Precondition: Step 1 matrix is complete.
- Postcondition: representative tests retain their module-specific fixture/config/claims and pass unchanged comparators.
- Files allowed to read: Step 1 files plus representative WASM fixture helpers.
- Files allowed to edit: `crates/slicer-runtime/tests/common/mod.rs`; a new `tests/common/integrated_parity_harness.rs`; four representative parity files.
- Files explicitly out of bounds: production crates and remaining parity files until the helper shape is proven.
- Expected sub-agent dispatches: targeted contract tests; return `FACT`.
- Context cost: `M`.
- Authoritative docs: ADR-0042/0056.
- Verification: targeted representative test names and `cargo check -p slicer-runtime --tests`.
- Exit condition: the helper reduces repeated setup without changing output or comparator assertions.

### Step 3: Migrate remaining parity files and comparator scaffolding

- Task IDs: `TASK-331`
- Objective: migrate the remaining 17 parity tests and factor repeated family container scaffolding while retaining diagnostics.
- Precondition: Step 2 representative tests pass.
- Postcondition: all 21 parity tests and all comparator self-tests pass; no test is ignored or removed.
- Files allowed to read: all 21 parity files and comparator/self-test files, bounded per file.
- Files allowed to edit: remaining 17 parity files; `parity_invariants.rs`; `parity_invariants_selftest_tdd.rs` only for mechanical fixture/helper signatures.
- Files explicitly out of bounds: production code, registry, edition config.
- Expected sub-agent dispatches: full contract test, return `FACT`; source inventory, return `LOCATIONS`.
- Context cost: `M`.
- Authoritative docs: ADR-0042 and ADR-0056.
- Verification: `cargo test -p slicer-runtime --test contract --all-targets`.
- Exit condition: AC-1 through AC-N1 pass and the final inventory confirms all 21 tests, six comparator families, and negative self-tests remain.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| 1 | S | inventory |
| 2 | M | harness design proof |
| 3 | M | full migration and comparator cleanup |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch.
