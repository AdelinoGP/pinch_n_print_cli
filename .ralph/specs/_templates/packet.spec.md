---
status: implemented
packet: [spec-slug]
task_ids:
  - TASK-000
backlog_source: docs/07_implementation_status.md
copy_note: Copy this file into ./.ralph/specs/<spec-slug>/ and change status to draft or active.
---

# Packet Contract: [spec-slug]

Template note: this file is marked `status: implemented` so Ralph preflight ignores the template copy. Change the status after copying it into a real packet folder.

## Goal

State the single remediation slice this packet owns.

## Scope Boundaries

- In scope:
- Out of scope:

## Acceptance Criteria

- **Given** [initial condition], **when** [action], **then** [observable result]. | `verification-command`
- **Given** [second condition], **when** [second action], **then** [second observable result]. | `verification-command`

Each criterion must end with a pipe `|` and a runnable verification command. If multiple criteria share the same verification, repeat the command in each criterion (do not use "see AC-N").

## Verification

- `[supplemental verification commands — only for criteria that share verification from a previous criterion]`

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/<path>`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (optional)
