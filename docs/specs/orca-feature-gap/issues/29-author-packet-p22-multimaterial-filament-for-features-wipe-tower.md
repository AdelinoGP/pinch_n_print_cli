# 29 — Author packet P22 — Multimaterial / Filament for Features — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet for **P22 — Multimaterial / Filament for Features — wipe-tower** — 1 keys, Tier B new logic, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P22 — Multimaterial / Filament for Features — wipe-tower):

`wipe_tower_filament`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Adjudication (2026-09-03): P22 does not stand alone — the key's *subject*
does not exist in this port.** `wipe_tower_filament` is a **selector over tower
body geometry**, and this port's prime tower has no body to select for. Rather
than defer, this ticket carried the investigation through — the census, the
canonical body definition, and the port's seam capacity are all below — and the
**user ruled** on the scope question they raise. `wipe_tower_filament` folds into
[122 — Author packet — prime tower body parity](./122-author-packet-prime-tower-body-parity.md).

### What the key does in canonical (rule 3: live, not dead)

`wipe_tower_filament` is a **1-based** filament index, `0` = auto
(`WipeTower2.hpp`'s `m_wipe_tower_filament` comment states the convention). It
does not change the purge; it forces which filament prints the tower's **finish
extrusions**. Three live pipeline consumers, all outside the toolchange wipe:

- `ToolOrdering::insert_wipe_tower_extruder` — early-returns on
  `!enable_prime_tower` or `wipe_tower_filament == 0`, otherwise appends
  `wipe_tower_filament - 1` to the extruder list of **every layer with
  `wipe_tower_partitions > 0`**, so the forced filament is available on that
  layer even if the objects never call for it.
- `WipeTower2::first_toolchange_to_nonsoluble_nonsupport` — chooses which
  toolchange index finishes the layer; its own comment says the finish
  extrusions are *"sparse infill + wall + brim"*, and `-1` means "print them
  with the layer's incoming filament before any toolchange". When
  `m_wipe_tower_filament > 0` it takes a dedicated short-circuit branch.
- The masking hack in `set_extruder` (both `WipeTower.cpp` and
  `WipeTower2.cpp`): `m_filpar[idx].is_soluble = (idx != wipe_tower_filament - 1)`
  — every *other* filament is marked soluble so the branch above lands on the
  forced one. `Print::validate` also asserts the index is in range and
  `Print`'s extruder set gains it.

The masking is why the Tier D per-filament fog is **not** the blocker here: the
forced branch short-circuits `filament_soluble` / `filament_is_support` rather
than reading them. This is a different failure from ticket 28's.

### Why it is unimplementable here: the port's tower is purge-only

`WipeTowerModule` (`modules/core-modules/wipe-tower/src/lib.rs`) emits, per
`ToolChange`, exactly three things — a travel entity, rectilinear purge
scan-lines, and a prime entity — from `generate_purge_paths`. There is:

- **no tower shell, brim, sparse infill, or framework** — nothing canonical
  would call a finish extrusion;
- **no idle-layer tower body.** Both module paths open with the same guard —
  `process` skips a layer when `tool_changes.is_empty()`, and
  `run_finalization` skips when `view.tool_changes().is_empty()`. Canonical
  prints the tower on every layer it spans (that is what
  `wipe_tower_no_sparse_layers` turns off);
- **no free choice of tool.** Every generated path carries `tool_index: None`,
  and both paths then stamp `tool_index = tc.to_tool` — the destination filament
  of the toolchange, which is correct and *forced* for a purge: the point of the
  purge is to flush the incoming material.

So the port has exactly the one extrusion class canonical does **not** let this
key touch, and none of the classes it does. Wiring the key to the purge's tool
would invert its meaning and break the purge; declaring it without wiring is
prohibited by Authoring rule 1. There is no honest third option, and no owner
re-derivation rescues it (ticket 27's hazard was checked: the seam is right, the
geometry is absent).

### The finding generalises past this ticket

The missing tower body is also the unstated blocker under **ten of the thirteen
keys packet 255 declared with-gap** (ticket 10's answer) and under P02's
framework / brim-width / infill-gap / flat-ironing keys. Packets 253–266 are
already marked ⚠ for re-authoring under rules 1–6; this finding says the
re-authoring of the prime-tower packets cannot succeed as *key* work either,
because they are all selectors over the same absent geometry. So the census was
taken once, here, for all of them.

### Census: which keys are body selectors, by body class

Verified against canonical at the constructor / consumer level. The classes are
the four the body has: **shell** (outer wall), **infill** (the sparse "empty
grid"), **brim** (first layer), **idle layer** (a tower layer with no toolchange).

| Key | Class | Canonical consumer |
|---|---|---|
| `wipe_tower_wall_type`, `wipe_tower_rib_width`, `wipe_tower_extra_rib_length`, `wipe_tower_fillet_wall` | shell | `WipeTower2::finish_layer` → `generate_support_rib_wall` / `generate_rib_polygon` |
| `wipe_tower_cone_angle` | shell | `WipeTower2::finish_layer` → `generate_support_cone_wall`; also `WipeTower2::get_wipe_tower_cone_base` from `Print` |
| `wipe_tower_bridging` | infill | `m_bridging`, the sparse-infill line count in `WipeTower2::finish_layer` |
| `prime_tower_infill_gap` | infill | `m_extra_spacing` in `WipeTower::set_extruder`; also read directly in `Print` |
| `prime_tower_brim_width` | brim | `m_wipe_tower_brim_width`, the first-layer brim arm of `finish_layer`; also assigned to `m_wipe_tower_data.brim_width` in `Print` |
| `wipe_tower_no_sparse_layers` | idle layer | `m_no_sparse_layers` — whether idle tower layers exist at all |
| `wipe_tower_rotation_angle` | shell + purge | `m_wipe_tower_rotation_angle` in both towers, plus the tower-placement math in `Print` |
| `prime_tower_enable_framework`, `prime_tower_flat_ironing`, `prime_tower_skip_points` | shell (Type1) | `m_tower_framework`, `m_flat_ironing`, `m_use_gap_wall` in `WipeTower::set_extruder`; `prime_tower_skip_points` also gates travel-avoid in `GCode.cpp` and `WipeTower2::use_gap_wall` |
| `enable_tower_interface_features` + the `filament_tower_interface_*` family | body-adjacent | feedrate clamp in `finish_layer`, contact handling in `GCode.cpp`; the `filament_*` members are per-filament (**Tier D**) |
| `wipe_tower_filament` | selects the tool for **all** of the above | `ToolOrdering::insert_wipe_tower_extruder`, `WipeTower2::first_toolchange_to_nonsoluble_nonsupport` |

Not body selectors, so not blocked by this: `wipe_tower_extra_flow` (already live
per ticket 10), `wipe_tower_max_purge_speed` (alias, ticket 108),
`wipe_tower_extra_spacing` (`m_extra_spacing_wipe` / `m_extra_spacing_ramming`
— purge and ramming side), and P23's `flush_volumes_matrix` /
`flush_multiplier` (purge *volume* routing through `ToolOrdering` and
`WipeTower2::extract_wipe_volumes`).

### What canonical's body is

`WipeTower2::finish_layer` **is** the body, emitted once per tower layer and
merged into that layer's toolchange results by `WipeTower2::generate`. In order it
lays down: the inner perimeter of the sparse section (a `rectangle` on
`fill_box`); the "CP EMPTY GRID" infill — **solid** at `sparse_factor` 1.0–1.5
when the *next* layer rams a soluble filament or when it is the adhesion first
layer, otherwise an inverse-U plus `m_bridging`-spaced sparse lines; the outer
wall via `generate_support_cone_wall` or `generate_support_rib_wall`; and, on the
first layer only, the brim.

Two structural properties the port has no analogue for:

- **The tower is planned globally, not per layer.** `WipeTower2::plan_tower`
  walks the plan **top-down**, propagating each layer's depth downward so the
  tower is a monotone prism, and derives `m_wipe_tower_depth` as the max over all
  layers. `save_on_last_wipe` then re-walks it.
- **Every spanned layer gets a plan entry.** In `Print`'s tower loop, each layer
  with `has_wipe_tower` first calls `plan_toolchange(z, h, cur, cur)` with no
  volume — an *idle* entry — before any real toolchange entries. That is why the
  tower is a solid structure and not a stack of disconnected purges.

### The port's seam can carry it — the gap is geometry, not contract

Checked rather than assumed, and this is the encouraging half:

- **Whole-print visibility already exists.** `run_finalization(&self, layers:
  &[LayerCollectionView], ...)` receives **all** layers, and the manifest sets
  `layer-parallel-safe = false`. Canonical's top-down depth propagation is
  therefore expressible inside the existing module — no prepass, no plan IR.
- **Idle-layer emission already exists.**
  `FinalizationOutputBuilder::push_entity_with_priority(layer_index, path,
  tool_index, region_key, priority)` needs no toolchange anchor, unlike the
  `insert_entity_at` the purge path uses. Body geometry can land on a layer that
  has no tool change.
- **Free tool choice already exists at the seam.** That same call takes an
  explicit `tool_index`, so `wipe_tower_filament` is wireable the moment a body
  exists — the seam was never the obstacle.
- **One real constraint found.** Intra-layer tool changes are emitted **only**
  from `layer.tool_changes` (`crates/slicer-gcode/src/emit.rs`); an entity whose
  `tool_index` differs mid-layer gets no synthesised change. The layer *boundary*
  is different — there the emitter does synthesise one from the first entity's
  `tool_index`. So a forced-filament body must either sit at the layer boundary
  or record a `ToolChange`, which is this port's analogue of canonical's
  `ToolOrdering::insert_wipe_tower_extruder`. Ticket 122 owns this.
- No WIT, schema, or IR change is implied by any of the above.

### Ruling (user, 2026-09-03)

**The port grows a real prime tower body, to full canonical parity.** In the
user's words: *"Full implementation of the canonical functionality, prime tower
here is practically a stub, goal for the packet shall be parity with OrcaSlicer
functionality and behaviour."* Purge-only is **not** accepted as this port's
design, and no census key goes out of scope.

### Disposition of `wipe_tower_filament`

Folded into [122 — Author packet — prime tower body parity](./122-author-packet-prime-tower-body-parity.md)
rather than re-filed as a standalone P22. It is a few lines of tool selection on
top of the body and has no meaning without it; splitting it would produce exactly
the declaration-only packet rule 1 forbids. **P22 is closed as a packet
boundary** — the key travels with the body work.

The Tier D per-filament fog is *not* a blocker for this key: canonical's forced
branch short-circuits `filament_soluble` / `filament_is_support` via the
`set_extruder` masking rather than reading them. The `filament_tower_interface_*`
family *is* Tier D and does not travel with it.

Status: **P22 dissolved into ticket 122.** No packet authored under this ticket.
