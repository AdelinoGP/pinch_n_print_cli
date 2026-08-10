# 8 — Author packet P01 — Cooling / Notes — part-cooling

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P01 — Cooling / Notes — part-cooling** — 17 keys, Tier A plumbing, owner part-cooling. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P01 — Cooling / Notes — part-cooling):

`activate_air_filtration`, `activate_chamber_temp_control`, `additional_cooling_fan_speed`, `auxiliary_fan`, `complete_print_exhaust_fan_speed`, `dont_slow_down_outer_wall`, `during_print_exhaust_fan_speed`, `fan_cooling_layer_time`, `fan_kickstart`, `fan_speedup_overhangs`, `fan_speedup_time`, `full_fan_speed_layer`, `internal_bridge_fan_speed`, `ironing_fan_speed`, `overhang_fan_threshold`, `reduce_fan_stop_start_freq`, `support_material_interface_fan_speed`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
