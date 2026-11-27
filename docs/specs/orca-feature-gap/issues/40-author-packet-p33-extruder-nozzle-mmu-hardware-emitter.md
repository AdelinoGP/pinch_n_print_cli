# 40 — Author packet P33 — Extruder / Nozzle / MMU Hardware — emitter

Type: task
Status: closed
Assignee: wayfinder session (ses_f8c6d54beffeU5nyqU5sbYdSmo) — claimed 2026-09-05
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P33 — Extruder / Nozzle / MMU Hardware — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P33 — Extruder / Nozzle / MMU Hardware — emitter):

`grab_length`, `start_end_points`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation of one key; the other returned to the queue
as unimplemented** — the ticket-22 (P15) shape, re-sized at claim time from the
tree.

### `grab_length` — implemented directly (no packet)

The purge-volume decision point already existed: `WipeTower::purge_volume_for`
(`modules/core-modules/wipe-tower/src/lib.rs`), wired by ticket 30, feeds both
the purge box's scan-line depth and the prime entity's extruded length. The
tier table's owner (`crates/slicer-gcode (toolchange)`) was wrong for this
tree — the port computes the purge volume once, in the wipe-tower module, and
the emitter only emits what the module produced (the ticket-27 lesson again).

- Declared `grab_length` on `wipe-tower.toml` (float, default 0, min 0 —
  canonical `PrintConfig.cpp` declares `coFloats` min 0 default `{0}`).
- `purge_volume_for` now subtracts `grab_length × 2.4` (the `(diameter/2)^2*PI`
  cross-section both canonical read sites hardcode — `GCode.cpp` toolchange
  path and `Print.cpp` wipe-tower planning) and clamps at 0, matching
  canonical's `std::max(0.f, wipe_volume - grab_purge_volume)`.
- 4 new tests: default identity, fallback + matrix reduction, zero clamp,
  emitted-geometry shrink (scan-line count and prime length).
- **DEV-170** records the two divergences: scalar where canonical is a
  per-extruder `coFloats` (the `flush_multiplier` precedent, DEV-169 (c)), and
  the reduction applying to the `prime_volume` fallback (canonical has no
  fallback opinion; default 0 keeps every existing print unchanged).

### `start_end_points` — returned to the queue as unimplemented

Canonical's only read site is `get_path_of_change_filament` (`GCode.cpp`),
which computes the three `travel_point_*` placeholders consumed by the user's
`change_filament_gcode` template. It is **not** wireable in this tree today:

1. **It requires `bed_exclude_area` to be live.** The algorithm returns the
   safe default path when `bed_exclude_area.size() != 4` — the cutter area is
   derived from the exclusion polygon. `bed_exclude_area` is packet 256's
   scope (authored, preflighted, **not implemented**); nothing declares it yet.
   Wiring `start_end_points` alone would be declaration-only (Authoring rule 1).
2. **The path computation needs object bounding boxes**, which exist only in
   the model (prepass/host), not at the `PostPass::GCodePostProcess` seam where
   the `change_filament_gcode` substitution runs (`machine-gcode-emit`), and
   the computed points would need a transport into that module's `ConfigView`
   (new `ResolvedConfig` fields or an extensions mechanism) plus
   `travel_point_*` schema declarations on `machine-gcode-emit.toml`.

Missing feature named (tier table): the filament-change travel path
(`get_path_of_change_filament` equivalent) and the `travel_point_*`
placeholders in the `change_filament_gcode` substitution. Re-file when packet
256's implementation lands (or fold into it). No key declared, no packet
number taken, no code change for this key.

### Records

- `04-asset-tier-assignment.md`: `grab_length` row annotated live (ticket 40);
  `start_end_points` row annotated returned-to-queue, blocked on packet 256.
- `05-asset-packet-list.md`: P33 now covers 1 key (`grab_length`);
  `start_end_points` returned to the queue.
- `docs/DEVIATION_LOG.md`: DEV-170.
- Gates: `cargo test -p wipe-tower` 36/36, `cube_4color_gcode_output_tdd`
  9/9, clippy + check-literals + gen-config-docs clean, all 46 guests rebuilt
  fresh.
