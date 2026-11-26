# Task Map: prime-tower-interface-and-ramming

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. **This packet emits the template's own skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253, 255, 256, 257a, 258, 254a), so the `docs/07` crosswalk is N-A. Implementation is recorded against wayfinder ticket 09 (`docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md`).

## Crosswalk

| Packet step | Task IDs | Wayfinder ticket / notes |
| --- | --- | --- |
| Whole packet (Steps 0–8) | — | 09 — Author packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower. Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; no TASK rows; re-derive the crosswalk question at completion time per the ledger-fact rule. |

## Split provenance

This directory is **half B** of former packet `254-prime-tower-keys-wipe-tower`. Half A (`254a-prime-tower-geometry-keys`) inherited the original directory and its git history; this half is a new directory. The split is required by ticket 09's own rule — "split by feature if the packet exceeds the B ceiling of 12 keys" — because the re-authored scope covers 13 keys.

**Ordering is load-bearing, not cosmetic:** `254a` must be implemented and merged before this packet starts (Step 0 gates on it). `254a` builds the per-layer depth model and the `depth_offset` / `block_depth` parameters on `generate_purge_paths` that every interface acceptance criterion here composes with.

| Former 254 key | Now owned by |
| --- | --- |
| `enable_tower_interface_features` | **254b** (this packet) |
| `filament_tower_interface_purge_volume` | **254b** |
| `filament_tower_interface_pre_extrusion_dist` | **254b** |
| `filament_tower_interface_pre_extrusion_length` | **254b** |
| `filament_tower_ironing_area` | **254b** |
| `prime_tower_flat_ironing` | **254b** |
| `enable_filament_ramming` | **254b** |
| `filament_tower_interface_print_temp` | **254b** (new `prime-tower-interface` module) |
| `enable_tower_interface_cooldown_during_tower` | **254b** (new `prime-tower-interface` module) |
| `prime_tower_infill_gap` | `254a-prime-tower-geometry-keys` |
| `prime_tower_brim_width` | `254a` |
| `prime_tower_enable_framework` | `254a` |
| `prime_tower_skip_points` | **returned to the queue** by `254a` — needs a travel-avoid-perimeter facility |

## New module registered by this packet

`prime-tower-interface` (`modules/core-modules/prime-tower-interface/`, stage `PostPass::GCodePostProcess`) is the 24th core module. Its registration surface — the workspace member list, `crates/slicer-integrated-modules/`, `crates/slicer-runtime/`, `crates/pnp-cli/Cargo.toml`, and the hard-asserted core-module count in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` — is enumerated in `implementation-plan.md` §Blast-radius discipline and owned by Steps 5 and 6.
