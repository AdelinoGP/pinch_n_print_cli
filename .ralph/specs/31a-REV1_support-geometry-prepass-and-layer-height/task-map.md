# Task Map: 31a-REV1_support-geometry-prepass-and-layer-height

## docs/07 Task IDs Covered

| TASK | Title | Packet 31a Status | Packet 31a-REV1 Action |
|------|-------|-------------------|------------------------|
| `TASK-163` | Close the five algorithmic v1 limitations on the foundation established by packet 31a | Partially done (implementation present but broken) | Fix execution order bug blocking 31b |

## Backlog Source

`docs/07_implementation_status.md` — Workstream 3, `TASK-163`.

## Step-to-Task Mapping

| Step | Task ID | Action |
|------|---------|--------|
| Step 1 | TASK-163 | Read broken code in `prepass.rs` |
| Step 2 | TASK-163 | Revert execution order |
| Step 3 | TASK-163 | Verify build |
| Step 4 | TASK-163 | Check/update test expectations |
| Step 5 | TASK-163 | Run full verification matrix |
| Step 6 | TASK-163 | Packet completion gate |

## Relationship to Prior Packet

Packet `31a_support-geometry-prepass-and-layer-height` (Steps 1–17, partially complete) introduced an execution order bug in `execute_prepass_with_builtins`. This packet (`31a-REV1`) is a targeted fix that:

1. **Fixes:** Execution order in `execute_prepass_with_builtins` (`prepass.rs`).
2. **Preserves:** All other 31a work (types, WIT, SDK, blackboard, manifests, planner interpolation).
3. **Unblocks:** Packet `31b_support-planner-algorithmic-parity`.

## Supersession

This packet supersedes the execution-order portion of `31a_support-geometry-prepass-and-layer-height`. The prior packet's `packet.spec.md` status remains `draft` with a note that it was partially implemented and superseded by `31a-REV1`. The type/WIT/SDK work from 31a is **not** reverted.
