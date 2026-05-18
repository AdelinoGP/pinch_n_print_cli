# Requirements: [spec-slug]

## Packet Metadata

- Grouped task IDs:
  - `TASK-000`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft | active | implemented`
- Aggregate context cost: `S | M` (must not be L)

## Problem Statement

Describe the concrete gap this packet closes and why it is a coherent slice. Motivation-shaped (what is wrong, why it matters), not solution-shaped (the solution is the Goal in `packet.spec.md`).

If this packet reopens, supersedes, or narrows a prior packet, name the earlier packet and the exact gap it left behind.

## In Scope

Full bullet list of what this packet does. `packet.spec.md` §Scope Boundaries carries a prose summary; this section is the authoritative list.

-

## Out of Scope

-

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`
- Add any packet-specific docs from `docs/04` through `docs/16`

For each doc, note size and whether the implementer should load it directly or delegate. Default rule: delegate any doc > 300 lines.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

(Include the verbatim opening paragraph from `references/snippets/orca-delegation.md` and list the OrcaSlicer files. Skip if no parity is involved.)

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` through `AC-N` from `packet.spec.md`. Add any packet-specific refinements here that didn't fit the Given/When/Then form (measurable outcomes, exact field names, count thresholds).
- Negative cases: `AC-N1` through `AC-NM` from `packet.spec.md`.
- Cross-packet impact: note any blocked or unblocked packets when relevant.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the 2–3 gate commands; this section is the authoritative list with delegation hints.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `[targeted test or command]` | [what it proves] | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `[workspace gate if required]` | [why workspace-wide] | FACT pass/fail |

All verification commands must be delegation-friendly (small, parseable output) so the implementer and reviewer can dispatch them to a sub-agent and consume only a FACT or SNIPPETS return.

## Step Completion Expectations

For each step in `implementation-plan.md`, the canonical fields (precondition, postcondition, files, dispatches, cost) live in that file. This section calls out only **cross-step** expectations that the step list cannot express:

- Cross-step invariants (e.g., "no step may regress the existing AC-3 test even if the AC-3 file is not edited by that step")
- Step ordering rationale (when not obvious from preconditions)
- Cross-step shared scratch state (rare; usually a red flag)

If none apply, write `None.` and move on. Do not restate per-step preconditions/postconditions here — they live in `implementation-plan.md`.

## Context Discipline Notes

Document any context-budget hazards **specific to this packet**. Workspace-wide discipline lives in the `context-discipline` snippet in `packet.spec.md`; do not restate it here.

- Large files in the read-only path that MUST be ranged or delegated:
- Likely temptation reads (files the implementer might curiosity-open) and why they should be skipped:
- Sub-agent return-format hints for the heaviest dispatches:

If none apply, write `None packet-specific.` rather than copying the snippet text.
