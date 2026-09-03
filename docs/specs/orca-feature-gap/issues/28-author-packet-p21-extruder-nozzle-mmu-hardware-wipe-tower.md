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

**Inherited from ticket 26 (2026-09-03):** `extruder_printable_area` and
`extruder_printable_height` were returned to the queue as unimplemented by P19 —
the emitter is the wrong owner. Both are per-extruder vectors, inert on a
single-extruder printer, whose only behaviour-changing canonical paths are
multi-extruder wipe-tower ones: `Print::get_extruder_shared_printable_polygon`
feeding `WipeTower::set_shared_print_bed` (clamps the tower centre) and
`WipeTower::is_valid_last_layer` (skips finish-layer / purge above an extruder's
height, gated on `m_is_multi_extruder`). Decide here whether this ticket adopts
them or hands them to 29–31; do not leave them unowned. Re-tiered A -> B in
`04-asset-tier-assignment.md`.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
