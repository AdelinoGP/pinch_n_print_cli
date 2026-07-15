# Requirements: [spec-slug]

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft | active | implemented`
- Aggregate context cost: `S | M` (never L)

## Problem Statement

[Motivation-shaped gap and why it is one coherent slice. Name any reopened/superseded packet and its exact missed gap.]

## In Scope

- [Authoritative full scope item]

## Out of Scope

- [Authoritative exclusion]

## Authoritative Docs

- `[docs/path.md]` - [size; direct range or delegation]. Delegate documents over 300 lines.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

[When parity applies, replace with the exact snippet plus paths; otherwise omit.]

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-N`; add only measurable refinements absent from their Given/When/Then text.
- Negative: `AC-N1` through `AC-NM`.
- Cross-packet impact:

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `[targeted command]` | [proof] | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `[workspace gate only if required]` | [why broad] | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants, non-obvious ordering, or shared scratch state. Per-step pre/postconditions belong in `implementation-plan.md`. Write `None.` when absent.

## Context Discipline Notes

Only packet-specific hazards: large ranged/delegated files, tempting reads to skip, and heavy-dispatch return limits. Write `None packet-specific.` when absent.
