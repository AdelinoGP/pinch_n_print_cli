# Task Map: 307-prime-tower-body-parity

This packet has no `docs/07_implementation_status.md` task IDs — it is a queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap". The crosswalk below maps the map's **queue keys** to packet steps instead, and records the explicit mapping needs that require this file: the packet dissolves two queue entries (P22 / ticket 29's key, P24 / ticket 31's key) into the body work, consumes three draft producers without re-declaring their keys, and splits one canonical function set (smooth mode) across two modules.

| Queue key (map row) | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `wipe_tower_bridging` (P03 residue) | `Step 3` (spacing) | `docs/08_coordinate_system.md` | `modules/core-modules/wipe-tower/src/lib.rs`, `wipe-tower.toml` | `WipeTower2.cpp`, `PrintConfig.cpp` | `M` | Sparse-line cap; inert at default 10.0 for sub-10 mm towers (AC-6) |
| `wipe_tower_no_sparse_layers` (P03 residue) | `Step 2` (idle gate) | `docs/04_host_scheduler.md` | `modules/core-modules/wipe-tower/src/lib.rs`, `wipe-tower.toml` | `WipeTower2.cpp`, `WipeTower.cpp`, `GCode.cpp` | `M` | Idle-layer existence switch (AC-4) |
| `prime_tower_skip_points` (P02 residue, 254a-returned) | `Step 6` (gap gate) | `docs/08_coordinate_system.md` | `modules/core-modules/wipe-tower/src/lib.rs`, `wipe-tower.toml` | `WipeTower2.cpp` | `S` | Tower-side `use_gap_wall` only; emitter routing is DEV-203 (AC-11) |
| `wipe_tower_filament` (P22, ticket 29 dissolved) | `Step 4` (tool select + fatal) | `docs/03_wit_and_manifest.md` | `modules/core-modules/wipe-tower/src/lib.rs`, `wipe-tower.toml` | `ToolOrdering.cpp`, `WipeTower2.cpp`, `Print.cpp` | `S` | Forced-filament short-circuit; Tier-D fog not engaged (masking) |
| `timelapse_type` (P24, ticket 31 dissolved) | `Step 5` (tower half + ingest arm) + suppression half | `docs/00_project_overview.md` | `modules/core-modules/wipe-tower/src/lib.rs`, `wipe-tower.toml`, `modules/core-modules/machine-gcode-emit/src/lib.rs`, `machine-gcode-emit.toml` | `Print.cpp`, `ToolOrdering.cpp`, `WipeTower.cpp`, `GCode.cpp`, `PrintConfig.cpp` | `M` | Smooth planning + DEV-168 clause (d); AC-14 maps canonical `"0"`/`"1"` spellings; DEV-202 records the observability limit |
| — (planning, not a key) | `Step 2` | `docs/04_host_scheduler.md` | `modules/core-modules/wipe-tower/src/lib.rs` | `WipeTower2.cpp` | `M` | Backward max-propagation + tower max + idle set (AC-3) |
| — (assembly, not a key) | `Step 3` | `docs/08_coordinate_system.md` | `modules/core-modules/wipe-tower/src/lib.rs` | `WipeTower2.cpp` | `M` | `finish_layer` order over 254a/255 builders (AC-5, AC-7) |
| — (validation, not a key) | `Step 7` | `docs/04_host_scheduler.md` | `modules/core-modules/wipe-tower/src/lib.rs` | `Print.cpp` | `S` | Bed-bounds at planned max; purge/interface/body coexistence |
| — (evidence, not a key) | `Step 8` | `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md` | scheduler + runtime + emitter test arms | — | `S` | AC-10, AC-12, AC-N1, each in its pre-existing file |
| — (closure, not a key) | `Step 9` | `docs/DEVIATION_LOG.md` | DEV rows + generated doc + gates | — | `S` | AC-13 + DEV-201/202/203 + check/clippy/guests |
| — (consumed, not declared) | `Steps 1–3` | producer specs | 254a depth/pitch/brim, 254b interface coexistence, 255 wall helpers | `WipeTower.cpp`, `WipeTower2.cpp` | `S` | FORWARD-DEPs; landing order 254a → 254b → 255 → 307 |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate is `M`; no row is L, so no split is required before activation.

## Absorbed queue entries

- P22 (`wipe_tower_filament`, ticket 29): dissolved — a few lines of tool selection with no meaning absent the body; splitting it would produce the declaration-only packet rule 1 forbids.
- P24 (`timelapse_type`, ticket 31): dissolved — smooth mode is a body selector (single-filament tower, every-layer entries, equalised depth, wall-only deliverable) plus the DEV-168 suppression clause, which lands here.
