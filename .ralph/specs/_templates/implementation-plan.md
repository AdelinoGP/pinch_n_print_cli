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
- Files expected to change:
  - `[path/to/file]`
- Authoritative docs:
  - `[docs/01_system_architecture.md]`
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]`
- Verification:
  - `[targeted command]`

### Step 2: [short step title]

- Task IDs:
  - `TASK-000`
- Objective:
- Files expected to change:
  - `[path/to/file]`
- Authoritative docs:
  - `[docs/01_system_architecture.md]`
- OrcaSlicer refs:
  - `[OrcaSlicerDocumented/path]`
- Verification:
  - `[targeted command]`

## Packet Completion Gate

- All steps complete.
- Packet acceptance criteria green.
- `docs/07_implementation_status.md` updated for the packet task IDs.
- `packet.spec.md` ready to move to `status: implemented`.
