# 9 — Author packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P02 — Multimaterial / Prime tower (1/2) — wipe-tower** — 13 keys, Tier A plumbing, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower):

`enable_filament_ramming`, `enable_tower_interface_cooldown_during_tower`, `enable_tower_interface_features`, `filament_tower_interface_pre_extrusion_dist`, `filament_tower_interface_pre_extrusion_length`, `filament_tower_interface_print_temp`, `filament_tower_interface_purge_volume`, `filament_tower_ironing_area`, `prime_tower_brim_width`, `prime_tower_enable_framework`, `prime_tower_flat_ironing`, `prime_tower_infill_gap`, `prime_tower_skip_points`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
