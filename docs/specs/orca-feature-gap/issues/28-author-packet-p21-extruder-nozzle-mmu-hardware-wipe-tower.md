# 28 — Author packet P21 — Extruder / Nozzle / MMU Hardware — wipe-tower

Type: task
Status: open
Assignee: —
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet for **P21 — Extruder / Nozzle / MMU Hardware — wipe-tower** — 5 keys, Tier B new logic, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P21 — Extruder / Nozzle / MMU Hardware — wipe-tower):

`cooling_tube_length`, `cooling_tube_retraction`, `extra_loading_move`, `high_current_on_filament_swap`, `parking_pos_retraction`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
