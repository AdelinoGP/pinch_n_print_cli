# 44 — Author packet P37 — Extruder / Nozzle / Retraction (2/2) — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P37 — Extruder / Nozzle / Retraction (2/2) — emitter** — 10 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P37 — Extruder / Nozzle / Retraction (2/2) — emitter):

`retract_when_changing_layer`, `retraction_distances_when_cut`, `retraction_distances_when_ec`, `retraction_minimum_travel`, `travel_slope`, `use_firmware_retraction`, `wipe`, `wipe_distance`, `z_hop_types`, `z_offset`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
