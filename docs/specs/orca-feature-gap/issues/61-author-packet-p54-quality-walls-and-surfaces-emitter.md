# 61 — Author packet P54 — Quality / Walls and surfaces (1/2) — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P54 — Quality / Walls and surfaces (1/2) — emitter** — 9 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P54 — Quality / Walls and surfaces (1/2) — emitter):

`bottom_solid_infill_flow_ratio`, `first_layer_flow_ratio`, `gap_fill_flow_ratio`, `inner_wall_flow_ratio`, `internal_solid_infill_flow_ratio`, `is_infill_first`, `max_travel_detour_distance`, `outer_wall_flow_ratio`, `overhang_flow_ratio`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
