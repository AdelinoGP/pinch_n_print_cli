# 69 — Author packet P62 — Cooling / Notes — tool-ordering

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-08) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P62 — Cooling / Notes — tool-ordering** — 1 keys, Tier B new logic, owner tool-ordering. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P62 — Cooling / Notes — tool-ordering):

`max_layer_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, re-filed, no packet, no code
change** (the ticket-28/39 shape).

`max_layer_height` (Tier B, 1 key) passes Authoring rule 3 — it is live in
canonical's slicing pipeline, not dead — but every live consumer rides a
subsystem this tree does not have, so a packet today would be 100%
declaration-only, prohibited by rule 1. Verified against the canonical oracle
(`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, re-derived
`ToolOrdering.cpp` under `src/libslic3r/GCode/` at claim time — the map Notes'
path spelling `src/libslic3r/ToolOrdering.cpp` is stale) and against the tree
(zero occurrences of the key or any consumer under `crates/`/`modules/`/`xtask/`):

- **Canonical declaration:** `coFloats`, default `{0}` (`PrintConfig.cpp` —
  per-extruder vector: listed in the extruder option keys, the filament option
  keys, and `printer_extruder_options`). `0` is auto, not absent: both readers
  substitute `0.75 × nozzle_diameter[i]` (`ToolOrdering.cpp`'s
  `calc_max_layer_height`, `Slicing.cpp`'s `max_layer_height_from_nozzle`).
- **Consumer 1 — tower partitions** (`calc_max_layer_height` →
  `fill_wipe_tower_partitions`): inserts intermediate tower layers at the raft
  gap and between partitions, and forces tower marking while the next tower
  layer exceeds `max + EPSILON` — all threaded through both
  `sort_and_build_data` overloads and the by-object/by-print layer-set
  constructors. The port's tower is purge-only with no partitions, no idle
  layers, and no marking (ticket 29's census) — the subject does not exist.
  **Sequences after [122](122-author-packet-prime-tower-body-parity.md)**; not
  one of 122's census keys (named `prime_tower_*`/`wipe_tower_*` only), so it
  waits on 122 rather than folding into it.
- **Consumer 2 — skirt marking** (`calc_max_layer_height` →
  `mark_skirt_layers`): marks intermediate layers for skirt on the same
  `> max + EPSILON` threshold. The port's skirt emits on the first N layers by
  count (`skirt-brim`), with no z-gap intermediate marking — the subject does
  not exist.
- **Consumer 3 — slicing envelope** (`SlicingParameters::create_from_config`):
  the same nozzle formula, clamped against `min_layer_height_from_nozzle`,
  bounds the variable-layer-height profile. The port's layer planner emits
  uniform `layer_height` steps (`layer-planner-default`) with no variable
  profile to clamp — and the adaptive enable key the tooltip names is commented
  out of canonical itself, so the envelope currently has no live switch on
  either side.
- **Non-consumer — `Print.cpp::object_skirt_offset`:** takes `max_element` of
  the vector but the offset never reaches skirt generation (ticket 32's
  finding: its `libslic3r/` caller is the sequential-clearance validator owned
  by [124](124-author-packet-sequential-printing-and-toolhead-clearance.md)).
  Named non-borrow.
- **Vector shape:** canonical reads per-extruder (`get_at` with clamping, `max_element`).
  This port has no per-extruder vector model — `nozzle_diameter` is an
  `extensions` scalar, not a `ResolvedConfig` field, and first-wins ingestion
  keeps element 0. Owned by [125](125-rule-per-tool-config-model.md).

Tier-table owner "tool-ordering" names a canonical file, not a tree module
(ticket-27 hazard): the partition logic likely lands with the tower body, the
skirt marking with skirt-brim, the envelope with the layer planner — the
re-file leaves the seam open for claim time.

**Re-filed as
[141](141-author-packet-p62-max-layer-height-tool-ordering-refiled.md),
blocked on 122 + 125.** Tier table + packet-list P62 rows annotate the re-file;
no queue-count change (unimplemented keys stay in scope).
