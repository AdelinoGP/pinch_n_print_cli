# 59 — Author packet P52 — Quality / Seam (1/2) — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P52 — Quality / Seam (1/2) — emitter** — 8 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P52 — Quality / Seam (1/2) — emitter):

`has_scarf_joint_seam`, `role_based_wipe_speed`, `scarf_angle_threshold`, `scarf_joint_flow_ratio`, `scarf_joint_speed`, `scarf_overhang_threshold`, `seam_gap`, `seam_slope_conditional`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
