# 8 — Author packet P01 — Cooling / Notes — part-cooling

Type: task
Status: open
Assignee: —
Blocked by: 99, 100, 101, 102, 103, 104, 105, 106, 107
Map: ../map.md

## Question

Author the spec packet for **P01 — Cooling / Notes — part-cooling** — 19 keys (17 Tier A plumbing + 2 Tier B logic), owner part-cooling. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P01 — Cooling / Notes — part-cooling), amended by ticket 99:

`activate_air_filtration`, `activate_chamber_temp_control`, `additional_cooling_fan_speed`, `auxiliary_fan`, `complete_print_exhaust_fan_speed`, `dont_slow_down_outer_wall`, `during_print_exhaust_fan_speed`, `fan_cooling_layer_time`, `fan_kickstart`, `fan_max_speed`, `fan_min_speed`, `fan_speedup_overhangs`, `fan_speedup_time`, `full_fan_speed_layer`, `internal_bridge_fan_speed`, `ironing_fan_speed`, `overhang_fan_threshold`, `reduce_fan_stop_start_freq`, `support_material_interface_fan_speed`

The `fan_max_speed`/`fan_min_speed` keys are the 99 reclassification: the
rename workstream (ticket 99) exposed Orca's percent (0–100) scale vs Pinch's
raw (0–255) scale, and `fan_min_speed` is declared but never read. Packet work
is the scale conversion (Tier B logic) plus wiring `fan_min_speed` to its
consumer alongside `reduce_fan_stop_start_freq`; parity evidence per 02
(canonical: `CoolingBuffer.cpp` fan-speed handling).

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
