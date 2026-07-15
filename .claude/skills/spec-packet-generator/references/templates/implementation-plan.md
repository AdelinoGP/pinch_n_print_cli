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
- Expected sub-agent dispatches:
  - Question: [question]; scope: `[path/glob]`; return: `[bounded format]`
- Context cost: `S | M` (split an L step)
- Authoritative docs:
  - `[docs/path.md]` - range or delegated SUMMARY
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]` - delegate; never load
- Verification:
  - `[targeted command]` - FACT pass/fail or bounded failure SNIPPETS
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
