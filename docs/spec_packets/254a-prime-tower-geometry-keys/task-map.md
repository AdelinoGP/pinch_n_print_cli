# Task Map: prime-tower-geometry-keys

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. **This packet emits the template's own skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253, 255, 256, 257a, 258), so the `docs/07` crosswalk is N-A. Implementation is recorded against wayfinder ticket 09 (`docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md`).

## Crosswalk

| Packet step | Task IDs | Wayfinder ticket / notes |
| --- | --- | --- |
| Whole packet (Steps 1–8) | — | 09 — Author packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower. Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; no TASK rows; re-derive the crosswalk question at completion time per the ledger-fact rule. |

## Split provenance

This directory is **half A** of former packet `254-prime-tower-keys-wipe-tower`, which is superseded in place (the directory was renamed, keeping its git history). The split is required by ticket 09's own rule — "split by feature if the packet exceeds the B ceiling of 12 keys" — because the re-authored scope covers 13 keys.

| Former 254 key | Now owned by |
| --- | --- |
| `prime_tower_infill_gap` | **254a** (this packet) |
| `prime_tower_brim_width` | **254a** (this packet) |
| `prime_tower_enable_framework` | **254a** (this packet) |
| `enable_filament_ramming` | `254b-prime-tower-interface-and-ramming` |
| `enable_tower_interface_features` | `254b` |
| `enable_tower_interface_cooldown_during_tower` | `254b` |
| `filament_tower_interface_pre_extrusion_dist` | `254b` |
| `filament_tower_interface_pre_extrusion_length` | `254b` |
| `filament_tower_interface_print_temp` | `254b` |
| `filament_tower_interface_purge_volume` | `254b` |
| `filament_tower_ironing_area` | `254b` |
| `prime_tower_flat_ironing` | `254b` |
| `prime_tower_skip_points` | **returned to the queue** — needs a travel-avoid-perimeter facility (`requirements.md` §Returned to Queue) |
