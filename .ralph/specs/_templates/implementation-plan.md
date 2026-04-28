# Implementation Plan: [spec-slug]

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: [short step title]

- Task IDs:
  - `TASK-000`
- Objective:
- Precondition:
- Postcondition:
- Files allowed to read (with line-range hints when > 300 lines):
  - `[path/to/file]` — lines `[N-M]`
- Files allowed to edit (≤ 3):
  - `[path/to/file]`
- Files explicitly out-of-bounds for this step:
  - `[anything that's tempting but must be delegated]`
- Expected sub-agent dispatches:
  - `[question; scope; return-format]`
- Context cost: `S | M` (never L; if L, split this step)
- Authoritative docs:
  - `[docs/01_system_architecture.md]` — read lines `[N-M]` or delegate a SUMMARY
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]` — delegate; never load
- Verification:
  - `[targeted command]` — dispatch as FACT pass/fail or SNIPPETS on failure
- Exit condition:

### Step 2: [short step title]

- Task IDs:
  - `TASK-000`
- Objective:
- Precondition:
- Postcondition:
- Files allowed to read:
  - `[path/to/file]`
- Files allowed to edit (≤ 3):
  - `[path/to/file]`
- Files explicitly out-of-bounds for this step:
  -
- Expected sub-agent dispatches:
  -
- Context cost: `S | M`
- Authoritative docs:
  - `[docs/01_system_architecture.md]`
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]`
- Verification:
  - `[targeted command]`
- Exit condition:

For read-only discovery steps, state the expected inventory, decision, or count that proves the step is complete. Read-only discovery steps are usually pure-dispatch steps (the implementer does no direct reading, only adjudicates returned LOCATIONS or FACTs).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S/M | |
| Step 2 | S/M | |

If the sum exceeds M aggregate, or any single step is L, the packet must be split before activation.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for the packet task IDs (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
