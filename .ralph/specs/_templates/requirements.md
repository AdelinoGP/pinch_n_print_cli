# Requirements: [spec-slug]

## Packet Metadata

- Grouped task IDs:
  - `TASK-000`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft | active | implemented`

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

## OrcaSlicer Reference Obligations

- List the exact `OrcaSlicerDocumented/` files the implementation must inspect.
- State what behavior, constants, or edge cases are being borrowed or deliberately not borrowed.

## Acceptance Summary

- Positive cases: copy the packet-level Given/When/Then criteria from `packet.spec.md` and add packet-specific refinements.
- Negative cases: restate the rejection or failure criteria that must hold.
- Measurable outcomes: name the exact outputs, counts, empty/non-empty conditions, or diagnostics that define done.
- Cross-packet impact: note any blocked or unblocked packets when relevant.

## Verification Commands

- `[targeted test or command]`
- `[workspace gate if required]`

## Step Completion Expectations

For each step in `implementation-plan.md`, capture or link the following:

- Precondition:
- Postcondition:
- Falsifying check:
