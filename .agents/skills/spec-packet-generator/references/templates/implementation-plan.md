# Implementation Plan: [spec-slug]

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: [title]

- Task IDs: `TASK-000`
- Objective:
- Precondition:
- Postcondition:
- Files allowed to read, with ranges when over 300 lines:
  - `[path]` - lines `[N-M]`
- Files allowed to edit (at most 3):
  - `[path]`
- Files explicitly out of bounds:
  - `[path/category]`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
- Expected sub-agent dispatches:
  - Question: [question]; scope: `[path/glob]`; return: `[bounded format]`
- Context cost: `S | M` (split an L step)
- Authoritative docs:
  - `[docs/path.md]` - range or delegated SUMMARY
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]` - delegate; never load
- Verification:
  - `[targeted command]` - FACT pass/fail or bounded failure SNIPPETS
  - When the step bumps a schema/version constant: include the test binary that asserts on the old constant value, e.g. `cargo test -p <crate> --test <file>`. Do not defer this to the acceptance ceremony — the bump and its test fallout land in the same step.
- Exit condition:

Repeat this complete field set for every step. A read-only discovery step states the inventory, decision, or count proving completion and normally delegates the read.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S/M | |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
