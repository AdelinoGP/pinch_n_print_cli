# Requirements: [spec-slug]

## Packet Metadata

- Grouped task IDs:
  - `TASK-000`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft | active | implemented`
- Aggregate context cost: `S | M` (must not be L)

## Problem Statement

Describe the concrete gap this packet closes and why it is a coherent slice.

If this packet reopens, supersedes, or narrows a prior packet, name the earlier packet and the exact gap it left behind.

## In Scope

-

## Out of Scope

-

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`
- Add any packet-specific docs from `docs/04` through `docs/12`

For each doc, note size and whether the implementer should load it directly or delegate. Default rule: delegate any doc > 300 lines.

## OrcaSlicer Reference Obligations

- List the exact `OrcaSlicerDocumented/` files the implementation must inspect.
- State what behavior, constants, or edge cases are being borrowed or deliberately not borrowed.
- All OrcaSlicer reads MUST be delegated; never load this tree into the implementer's own context.

## Acceptance Summary

- Positive cases: copy the packet-level Given/When/Then criteria from `packet.spec.md` and add packet-specific refinements.
- Negative cases: restate the rejection or failure criteria that must hold.
- Measurable outcomes: name the exact outputs, counts, empty/non-empty conditions, or diagnostics that define done.
- Cross-packet impact: note any blocked or unblocked packets when relevant.

## Verification Commands

- `[targeted test or command]`
- `[workspace gate if required]`

All verification commands listed here must be delegation-friendly (small, parseable output) so the implementer and reviewer can dispatch them to a sub-agent and consume only a FACT or SNIPPETS return.

## Step Completion Expectations

For each step in `implementation-plan.md`, capture or link the following:

- Precondition:
- Postcondition:
- Falsifying check:
- Files allowed to read (with line-range hints when > 300 lines):
- Files allowed to edit (≤ 3):
- Expected sub-agent dispatches:
- Step context cost: `S | M` (never L)

## Context Discipline Notes

Document any context-budget hazards specific to this packet:

- Large files in the read-only path that MUST be ranged or delegated:
- OrcaSlicer trees the implementer must NOT load directly:
- Likely temptation reads (files the implementer might curiosity-open) and why they should be skipped:
- Sub-agent return-format hints for the heaviest dispatches:
