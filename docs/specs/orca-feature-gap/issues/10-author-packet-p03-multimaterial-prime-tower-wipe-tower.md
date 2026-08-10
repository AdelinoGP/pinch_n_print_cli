# 10 — Author packet P03 — Multimaterial / Prime tower (2/2) — wipe-tower

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P03 — Multimaterial / Prime tower (2/2) — wipe-tower** — 13 keys, Tier A plumbing, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P03 — Multimaterial / Prime tower (2/2) — wipe-tower):

`purge_in_prime_tower`, `single_extruder_multi_material`, `wipe_tower_bridging`, `wipe_tower_cone_angle`, `wipe_tower_extra_flow`, `wipe_tower_extra_rib_length`, `wipe_tower_extra_spacing`, `wipe_tower_fillet_wall`, `wipe_tower_max_purge_speed`, `wipe_tower_no_sparse_layers`, `wipe_tower_rib_width`, `wipe_tower_rotation_angle`, `wipe_tower_wall_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
