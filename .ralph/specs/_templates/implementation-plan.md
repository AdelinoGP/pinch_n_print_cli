# Implementation Plan: [spec-slug]

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: [short step title]

- Task IDs:
  - `TASK-000`
- Objective:
- Precondition:
- Postcondition:
- Files expected to change:
  - `[path/to/file]`
- Authoritative docs:
  - `[docs/01_system_architecture.md]`
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]`
- Verification:
  - `[targeted command]`
- Exit condition:

### Step 2: [short step title]

- Task IDs:
  - `TASK-000`
- Objective:
- Precondition:
- Postcondition:
- Files expected to change:
  - `[path/to/file]`
- Authoritative docs:
  - `[docs/01_system_architecture.md]`
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]`
- Verification:
  - `[targeted command]`
- Exit condition:

For read-only discovery steps, state the expected inventory, decision, or count that proves the step is complete.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green.
- `docs/07_implementation_status.md` updated for the packet task IDs.
- Reopened or superseded packet status transitions reconciled.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
