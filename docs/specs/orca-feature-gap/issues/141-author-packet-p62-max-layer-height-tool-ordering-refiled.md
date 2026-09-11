# 141 — Author packet P62 (re-filed) — max_layer_height — tool-ordering

Type: task
Status: open
Assignee: —
Blocked by: 122, 125
Map: ../map.md

## Question

Re-filed from [ticket 69](./69-author-packet-p62-cooling-notes-tool-ordering.md),
which re-sized P62 at claim time: **`max_layer_height` is a per-extruder vector
key whose every live consumer rides a missing subsystem, and is not authorable
under Authoring rule 1 until those land.** **Read ticket 69's answer before
starting** — it holds the per-key canonical grounding and the from-disk tree
evidence, and is not restated here.

Key (1, Tier B, owner tool-ordering):

`max_layer_height`

Canonical decision points (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path
at point of use per the map Notes; ticket 69 pins the functions):

- `ToolOrdering.cpp::calc_max_layer_height` — min over the per-extruder vector
  (`0` = auto → `0.75 × nozzle_diameter[i]`, `get_at` clamps short vectors),
  floored by the object's `layer_height`. Feeds `fill_wipe_tower_partitions`
  (intermediate tower-layer insertion + next-tower-layer spacing, both on a
  `gap > max + EPSILON` threshold) and `mark_skirt_layers` (intermediate skirt
  marking on the same threshold), called from both `sort_and_build_data`
  overloads and both layer-set constructors.
- `Slicing.cpp::max_layer_height_from_nozzle` +
  `SlicingParameters::create_from_config` — the same `0 → 0.75 × nozzle`
  formula, clamped against `min_layer_height_from_nozzle`, feeding the
  `SlicingParameters` min/max envelope that clamps the variable-layer-height
  profile. The enable key the tooltip names (`adaptive_layer_height`) is
  commented out of `PrintConfig.cpp` — the envelope is live, the switch is not.
- `Print.cpp::object_skirt_offset` — `max_element` of the vector in the
  per-object skirt-offset math. Dead end for this packet: the offset never
  reaches skirt generation (ticket 32's finding — neither `_make_skirt` nor
  `_make_brim` calls it; its `libslic3r/` caller is the sequential-clearance
  validator owned by ticket 124). Named non-borrow, not a decision point.

Why it is blocked:

- **Per-extruder vector shape** — canonical declares `coFloats` (extruder +
  filament option lists, `printer_extruder_options`); this port has no
  per-extruder vector model (`nozzle_diameter` is an `extensions` scalar, not a
  `ResolvedConfig` field) and first-wins ingestion keeps element 0 only. Owned
  by [125](./125-rule-per-tool-config-model.md).
- **Tower-partition subject** — `fill_wipe_tower_partitions` inserts idle tower
  layers and spaces partitions; this port's tower is purge-only with no
  partitions, no idle layers, and no `has_wipe_tower` marking. Owned by
  [122](./122-author-packet-prime-tower-body-parity.md). (The key is not in
  ticket 29's census, so it sequences *after* 122 rather than folding into it.)
  **Update (2026-09-11): 122 resolved as packet
  [307](../../../spec_packets/307-prime-tower-body-parity/) (`draft`) — the idle-layer
  and planned-depth subjects this key needs are that packet's Steps 2–3. They
  exist only once 307 is implemented (off-map), so this subject stays closed
  until then; re-check 307's status at claim time rather than assuming it.**
- **Skirt-marking / slicing-envelope subjects** — the port's skirt emits on the
  first N layers by count (no z-gap intermediate marking) and its layer planner
  emits uniform `layer_height` steps (no variable profile to clamp). Whichever
  consumer the packet wires, it builds the subject first or sheds the key —
  declaration-only is prohibited by rule 1.

Authoring obligations:

- **Do not start until 122 and 125 land.** If either ruling defers its family
  again, the honest outcome is to defer this packet again — never declare the
  key (Authoring rule 1). (06 resolved at ticket 69's claim time and rides
  122 transitively.)
- Per-key re-derive the owner from the *tree's* seams at claim time (ticket 27's
  hazard); "tool-ordering" names a canonical file, not a tree module — the
  partition logic may belong with the tower body, the skirt marking with
  skirt-brim, the envelope with the layer planner. The packet, when authorable,
  must either build every missing decision point or shed the unimplemented
  behaviour — it may not record it as declared-with-gap.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the family out of scope.

## Answer
