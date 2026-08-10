# 88 — Author packet P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution** — 5 keys, Tier B new logic, owner config-resolution. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution):

`extruder_variant_list`, `filament_extruder_variant`, `filament_self_index`, `print_extruder_id`, `print_extruder_variant`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
