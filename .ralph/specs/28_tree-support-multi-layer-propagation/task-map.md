# Task Map: 28_tree-support-multi-layer-propagation → docs/07

This packet introduces `PrePass::SupportGeneration` + `SupportPlanIR` plus a new core module (`support-planner`) and updates `tree-support` to consume the plan. It does not supersede any prior packet.

## Task ID Mapping

| Packet Step | docs/07 Task | Notes |
|---|---|---|
| Step 1 | TASK-161 | Read-only discovery of PrePass precedent (packet `23-rev1`) and OrcaSlicer `TreeSupport::drop_nodes`. |
| Step 2 | TASK-161 | Add `SupportPlanIR`, `SupportPlanEntry` to `slicer-ir`. |
| Step 3 | TASK-161 | Extend `wit/world-prepass.wit` with `run-support-generation`. |
| Step 4 | TASK-161 | Host plumbing in `prepass.rs` + `blackboard.rs`. |
| Step 5 | TASK-161 | Host WIT dispatcher for the new export. |
| Step 6 | TASK-161 | SDK `PrepassModule` trait + macro stage map. |
| Step 7 | TASK-161 | Scaffold `modules/core-modules/support-planner/` crate. |
| Step 8 | TASK-161 | Add host prepass tests (TDD; expected to fail at this step). |
| Step 9 | TASK-161 | Implement simplified `detect_overhangs` in `support-planner`. |
| Step 10 | TASK-161 | Implement simplified `drop_nodes` propagation + per-layer MST merging. |
| Step 11 | TASK-161, TASK-120b | Update `tree-support` manifest + `run_support` to consume `SupportPlanIR`; preserve grid-MST fallback. Re-exercises the packet-26 live-dispatch tier under the planner-consuming path. |
| Step 12 | TASK-161 | Document and regression-assert traditional-support's per-layer nature (does not consume `SupportPlanIR`). |
| Step 13 | TASK-161 | Rebuild all affected `.wasm` artifacts. |
| Step 14 | TASK-161 | Add TASK-161 row to `docs/07_implementation_status.md`. |
| Step 15 | TASK-161 | Packet completion gate. |

## Superseding Relationship

Packet 28 does **not** supersede any prior packet.

- Packet `26_live-support-module-evidence` (status: `draft`; its benchy support-enabled ACs are blocked on the packet-26 percent-unit fix already landed — unrelated to this packet). This packet extends `live_support_generation_tdd.rs` with a new Section C rather than overwriting Sections A or B.
- Packet `23-rev1_prepass-seam-planning-orca-parity` (status: `implemented`). It is the structural precedent for this packet. No files belonging to packet 23-rev1 are modified.

## docs/07 Reconciliation Note

TASK-161 is a new task added specifically for this packet's deliverable. Draft line to paste into `docs/07_implementation_status.md` under Workstream 3:

```
- [ ] TASK-161 Introduce `PrePass::SupportGeneration` plus a canonical `SupportPlanIR` blackboard contract so tree-support branches can be planned across layers (simplified port of OrcaSlicer `TreeSupport::drop_nodes`) and emitted by `Layer::Support` from pre-planned geometry. Continues DEV-009, deepens Orca parity, and supports TASK-120.
```

TASK-120 and TASK-120b are not modified by this packet:

- **TASK-120** remains `[~]` (Phase H acceptance umbrella is broader than this packet).
- **TASK-120b** was re-closed by packet 26 citing grid-MST live-dispatch evidence; this packet upgrades the underlying algorithm but does not reopen the task. A short parenthetical note may optionally be appended to the TASK-120b closure block after Step 15 passes (e.g. "Further upgraded 2026-04-XX by packet `28_tree-support-multi-layer-propagation`: tree-support now emits branches from `SupportPlanIR` when a `support-planner` module is installed, falling back to the grid-MST filler otherwise.") — this is **optional**, since the existing TASK-120b evidence is still accurate for the fallback path.

## Parallelism Note

Packet 28 is serial by construction — a single active packet at a time per the Ralph runtime rules. No parallel tracks.
