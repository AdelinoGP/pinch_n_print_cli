# Swarm Execution Report — Output Format

**When to read this:** at the end of a swarm run, when emitting the Swarm Execution Report (Phase 5 completion, or earlier when emitting `Status: PARTIAL` from a checkpoint stop — 150k standard band / 300k extended band).

**Topics:** report layout, status fields, per-step results, review verdict, docs/status impact, handoff for partial/deferred runs.

## Template

```text
## Swarm Execution Report: <packet-slug>

Status: COMPLETED | PARTIAL | BLOCKED
Mode: implement | refine-draft | review-only
Starting packet status: draft | active | implemented | superseded
Iterations: N
Workers used: N
Planner state backend: session-memory | packet-checkpoint
Planner context cost (peak): <%>; remaining at report time: <%>

Packet-quality preflight:
- PASS / FAIL
- Notes: ...

Execution manifest:
- built: yes / no
- delta scope for this iteration: changed_steps=<...>, changed_files=<...>

Step results:
- Step N: DONE / PARTIAL / SKIPPED
  - Evidence: ...
  - Validation: ...

Review verdict:
- APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED / BLOCKED / DEFERRED

Docs and status impact:
- packet status change: ...
- docs/07 change: yes / no
- superseded packet reconciliation: ...

Remaining issues:
1. ...

Verification commands run:
- ...

Handoff (if Status: PARTIAL or DEFERRED):
- next concrete dispatch: ...
- files to reopen: ...
- recommended fresh-session entry point: ...
```

## Fill-in notes

- `Planner context cost (peak)` is the highest budget reading observed during the run; `remaining at report time` is the budget left when the report is emitted.
- `Docs and status impact / docs/07 change: no` is correct for retrofit packets correcting already-closed work — emit it explicitly rather than omitting the line.
- `Handoff` is omitted only on `Status: COMPLETED` runs; on PARTIAL/BLOCKED/DEFERRED the section is mandatory.