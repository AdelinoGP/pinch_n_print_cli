# 63 — Author packet P56 — Speed / Acceleration — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P56 — Speed / Acceleration — emitter** — 11 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P56 — Speed / Acceleration — emitter):

`accel_to_decel_enable`, `accel_to_decel_factor`, `bridge_acceleration`, `default_acceleration`, `initial_layer_acceleration`, `inner_wall_acceleration`, `internal_solid_infill_acceleration`, `outer_wall_acceleration`, `sparse_infill_acceleration`, `top_surface_acceleration`, `travel_acceleration`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
