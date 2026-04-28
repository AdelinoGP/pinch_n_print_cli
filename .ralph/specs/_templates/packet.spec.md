---
status: implemented
packet: [spec-slug]
task_ids:
  - TASK-000
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S | M    # aggregate cost; never L for an active packet
copy_note: Copy this file into ./.ralph/specs/<spec-slug>/ and change status to draft or active.
---

# Packet Contract: [spec-slug]

Template note: this file is marked `status: implemented` so Ralph preflight ignores the template copy. Change the status after copying it into a real packet folder.

## Goal

State the single remediation slice this packet owns.

## Scope Boundaries

- In scope:
- Out of scope:

## Prerequisites and Blockers

- Depends on:
- Unblocks:
- Activation blockers:

## Acceptance Criteria

- **Given** [initial condition], **when** [action], **then** [observable result]. | `verification-command`
- **Given** [second condition], **when** [second action], **then** [second observable result]. | `verification-command`

Each criterion must end with a pipe `|` and a runnable verification command. If multiple criteria share the same verification, repeat the command in each criterion (do not use "see AC-N").

Name exact assertion content in the criterion text. Prefer exact fields, paths, counts, error variants, or output fragments over phrases like "all required fields" or "correct diagnostics".

Each verification command must be **delegation-friendly**: it produces a small, parseable output (exit code, single assertion, JSON path). Commands that dump > 200 lines of log on success should be wrapped or filtered so a sub-agent can return a FACT.

## Negative Test Cases

- **Given** [rejection or failure condition], **when** [action], **then** [observable failure or validation result]. | `verification-command`

Include this section whenever the packet changes validation, enforcement, contract boundaries, or error-handling behavior.

## Verification

- `[supplemental packet-level or workspace verification commands only — not a replacement for per-criterion commands]`

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`

For each doc, note whether the implementer should load it directly or delegate the read (delegate when the doc is > 300 lines or only one section is needed).

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/<path>` — always delegate the read; never load OrcaSlicer source into the implementer's own context.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (required when the packet spans more than one task ID or corrects a prior packet)

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
