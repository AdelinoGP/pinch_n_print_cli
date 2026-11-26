# 10 — Author packet P03 — Multimaterial / Prime tower (2/2) — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (ses_fb4e119c6ffeBSxr93q95x8BKn)
Blocked by: 06, 100
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

Packet `docs/spec_packets/255-wipe-tower-geometry-keys/` authored (`draft`), preflight **PASS** (S0–S8 all green; one dispatch claim retracted via re-check — scheduler flat tests auto-discover, no registration needed).

Grounding findings, all verified in code at authoring time:

- **One live decision point among the 13:** `wipe_tower_extra_flow` (canonical `coPercent` 100%, [100, 300], consumed in `WipeTower2.cpp::toolchange_Wipe`) wires to the port's hardcoded scan-line `flow_factor: 1.0` in `generate_purge_paths` (`modules/core-modules/wipe-tower/src/lib.rs`) — the emitter multiplies extrusion E by `flow_factor` (`crates/slicer-gcode/src/emit.rs`). Identity at defaults (no output change); `"200%"` doubles purge extrusion. Both module execution paths (`process()`, `run_finalization()`) share the wiring site.
- **Alias finding (grounding discovery):** `wipe_tower_max_purge_speed` needs no new key — host key `wipe_tower_speed` (`crates/slicer-ir/src/feedrate.rs::FeedrateConfig`, default 90.0 = canonical's 90; consumed at `ExtrusionRole::WipeTower` in `resolve_feedrate`) already drives the same decision. Excluded from the packet (duplicate-spelling class per ticket 107); rename question filed as new ticket 108. `wipe_tower_speed` is not in the 99–107 rename set.
- **10 declared-with-gaps keys:** cone/rib/fillet walls, bridging pass, rotation, wall types, flush routing, ramming spacing, sparse layers — per-key canonical consumers cited (file + function) in `requirements.md` §Per-key parity evidence. All scalar-typed canonically, so the Tier-D per-filament fog is NOT engaged (packet 254's fog note does not inherit here).
- **Three keys already reach CONFIG_BLOCK** as `ORCA_CONFIG_PADDING` literals (`single_extruder_multi_material`, `wipe_tower_rotation_angle`, `wipe_tower_no_sparse_layers`), probed to match Orca defaults; padding dedups against user-supplied values.
- **Output change at defaults is exactly +2 CONFIG_BLOCK lines** (the two percent-typed defaults `wipe_tower_extra_flow`/`wipe_tower_extra_spacing` thread via the packet-185 transport, spelled `100%`); geometry is byte-identical at defaults.
- Guest-staleness baseline: `cargo xtask build-guests --check` exit 1 at authoring time, `STALE: tree-support-planner-guest` only — pre-existing (docs-only working tree, same hazard as ticket 99's finding); the wipe-tower guest is fresh.

## Answer
