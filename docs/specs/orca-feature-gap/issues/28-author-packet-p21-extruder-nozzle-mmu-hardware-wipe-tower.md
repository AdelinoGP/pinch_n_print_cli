# 28 — Author packet P21 — Extruder / Nozzle / MMU Hardware — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet for **P21 — Extruder / Nozzle / MMU Hardware — wipe-tower** — 5 keys, Tier B new logic, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P21 — Extruder / Nozzle / MMU Hardware — wipe-tower):

`cooling_tube_length`, `cooling_tube_retraction`, `extra_loading_move`, `high_current_on_filament_swap`, `parking_pos_retraction`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

**Inherited from ticket 26 (2026-09-03):** `extruder_printable_area` and
`extruder_printable_height` were returned to the queue as unimplemented by P19 —
the emitter is the wrong owner. Both are per-extruder vectors, inert on a
single-extruder printer, whose only behaviour-changing canonical paths are
multi-extruder wipe-tower ones: `Print::get_extruder_shared_printable_polygon`
feeding `WipeTower::set_shared_print_bed` (clamps the tower centre) and
`WipeTower::is_valid_last_layer` (skips finish-layer / purge above an extruder's
height, gated on `m_is_multi_extruder`). Decide here whether this ticket adopts
them or hands them to 29–31; do not leave them unowned. Re-tiered A -> B in
`04-asset-tier-assignment.md`.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Adjudication (2026-09-03): P21 is not authorable now.** All five keys are
live in canonical, but only on a code path this port does not have, and every
one of them is arithmetically fused with keys the queue has already deferred
(Tier D). Authoring the packet today would produce five declaration-only keys,
which Authoring rule 1 prohibits. The packet is re-filed as
[119 — Author packet P21 (re-filed, blocked on the per-tool config model)](./119-author-packet-p21-mmu-hardware-wipe-tower-refiled.md),
blocked on the new research ticket
[118 — Inventory the port's per-tool (filament/extruder) config mechanism](./118-inventory-per-tool-config-mechanism.md).

### Canonical grounding (rule 3 check: the keys are live, not dead)

All five keys pass the dead-in-canonical check, but the read site matters:

- In the **BBS tower** (`WipeTower.cpp`) they are **entirely dead**. The block
  that assigns them (`m_cooling_tube_retraction` … `m_set_extruder_trimpot`,
  in the `if (m_semm)` arm of the constructor) sits inside `#if 0 // BBS:
  remove useless config`, the matching members are commented out in
  `WipeTower.hpp`, and both consumers — `WipeTower::toolchange_Unload` and
  `WipeTower::toolchange_Load` — have `#if 0` bodies carrying the comments
  *"BBS: toolchange unload is done in change_filament_gcode"* and *"BBS: tool
  load is done in change_filament_gcode"*.
- In the **Prusa-style tower** (`WipeTower2.cpp`) they are **live**:
  `WipeTower2::toolchange_Unload` (cooling-tube retraction ladder, the
  `load_move_x_advanced` cooling shuttle, the parking-position retract) and
  `WipeTower2::toolchange_Load` (the `parking_pos_retraction +
  extra_loading_move` load ladder, plus `set_extruder_trimpot` under
  `high_current_on_filament_swap`).
- Which tower runs is `Print::wipe_tower_type` — `is_BBL_printer() ? Type1 :
  wipe_tower_type`, whose config default is `type2`. So on a non-BBL printer
  the live path is `WipeTower2`, and the keys matter there.
- Two further canonical gates sit above them: `m_semm`
  (`single_extruder_multi_material`) and, in `WipeTower2`,
  `m_enable_filament_ramming` (`enable_filament_ramming`).

**Parity note worth carrying forward:** canonical's own Type1 path delegates
the whole unload/load choreography to `change_filament_gcode`. This port
already has that hook — `change_filament_gcode` is a registered custom-G-code
point in `machine-gcode-emit`'s manifest, emitted from its `GCodeCommand::ToolChange`
arm. So the port is at Type1-equivalent parity today; P21 is Type2-only work.

### Why the packet cannot be authored: the Tier D fusion

Every use site multiplies a P21 key by a **per-filament** value, all of which
[`04-asset-tier-assignment.md`](./04-asset-tier-assignment.md) marks **Tier D —
deferred (per-filament config model)**:

| P21 key | canonical use site | Tier D values it is fused with |
|---|---|---|
| `cooling_tube_retraction`, `cooling_tube_length` | `WipeTower2::toolchange_Unload` retraction ladder | `filament_unloading_speed_start`, `filament_unloading_speed` |
| `cooling_tube_length` | the cooling shuttle (`load_move_x_advanced`) | `filament_cooling_moves` (loop count — **zero disables the only use of the key**), `filament_cooling_initial_speed`, `filament_cooling_final_speed` |
| `parking_pos_retraction` | park retract after cooling | `filament_cooling_moves`, `filament_stamping_distance` |
| `parking_pos_retraction`, `extra_loading_move` | `WipeTower2::toolchange_Load` load ladder | `filament_loading_speed_start`, `filament_loading_speed` |
| `high_current_on_filament_swap` | `set_extruder_trimpot(550)` | fires only inside that same load block |

Neither `single_extruder_multi_material` nor `enable_filament_ramming` exists in
this tree. `single_extruder_multi_material` appears **only** as a hardcoded
`ORCA_CONFIG_PADDING` row in `crates/slicer-gcode/src/serialize.rs`, which
Authoring rule 2 explicitly rules out as evidence; `enable_filament_ramming` has
zero occurrences in `crates/` or `modules/`.

The port's wipe-tower module is also not a G-code writer: `wipe-tower.toml`
declares `[stage] id = "PostPass::LayerFinalization"` and the module emits purge
geometry (`generate_purge_paths` → `ExtrusionPath3D`), with the pre-toolchange
retract synthesised host-side in `crates/slicer-gcode/src/emit.rs`. A ramming /
unload / load choreography is a G-code sequence interleaved with tower
geometry, so P21 needs an emitter seam that does not exist yet — the owner
column's "wipe-tower" is right about the *feature* and wrong about the *seam*
(the ticket-27 owner-re-derivation hazard again).

**Net:** P21 is a Tier B+ feature — "SEMM filament unload/load choreography on
the Type2 tower" — gated on a config-model decision, not five keys of plumbing.

### The two inherited keys: adopted, not handed to 29–31

`extruder_printable_area` and `extruder_printable_height` are **adopted by the
re-filed ticket 119**, not handed to tickets 29–31 (their subjects —
filament-for-features, flush options, special mode — are all further from the
behaviour than P21's own). They are per-**extruder** vectors gated on
`m_is_multi_extruder`, which is the same missing subsystem seen from the
extruder side rather than the filament side: this port's `nozzle_diameter` is a
**scalar** `f32` in `ResolvedConfig`, not canonical's per-extruder `coFloats`.
They are therefore blocked on ticket 118 alongside the five MMU keys, and stay
recorded as unimplemented in the tier table.

### What the tree already has (feeds ticket 118)

The per-tool config model is **not** absent — it is present in one instance and
unproven at scale. `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs`)
carries `filament_density: Vec<f64>` under a `@filament` scope marker with a
tool-indexed accessor `ResolvedConfig::filament_density_for(tool_index)` that
falls back to the first entry. That is exactly the shape canonical's `coFloats`
need. Whether it generalises — through `to_config_map` to a module's
`ConfigView`, and whether a module can index it by tool id (the wipe-tower
module does see each `ToolChange`'s `from`/`to`) — is what ticket 118 measures.

Status: **not authored.** Re-filed as ticket 119; prerequisite filed as ticket 118.
