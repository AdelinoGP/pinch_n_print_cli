# 39 — Author packet P32 — Extruder / Nozzle / Extruder geometry / mapping — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8c83b46cffebu86slqC198Lmi)
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P32 — Extruder / Nozzle / Extruder geometry / mapping — emitter** — 7 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P32 — Extruder / Nozzle / Extruder geometry / mapping — emitter):

`extruder_colour`, `extruder_offset`, `extruder_type`, `master_extruder_id`, `physical_extruder_map`, `printer_extruder_id`, `printer_extruder_variant`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable as a standalone packet, and one of
the seven keys is already covered.** The "7 keys, Tier B new logic, host
emitter" sizing from 04/05 does not survive contact with the tree or with
canonical. Verdict per key:

### `extruder_colour` — already covered; no queue work remains

Canonical's only pipeline read of `extruder_colour` is `GCode::append_full_config`
(CONFIG_BLOCK writer): it is emitted as an alias of `filament_colour`
(`opt_serialize("filament_colour")`); its only other hit (`Print.cpp`'s
`steps_gcode` set) is a re-slice gate, not a decision point. The port already
implements that behaviour: `; extruder_colour = …` is emitted in both the
HEADER_BLOCK and the CONFIG_BLOCK
(`crates/slicer-gcode/src/serialize.rs`: `ThumbnailAwareSerializer::serialize_gcode`
rewrites the header lines to the authored palette when the config carries one;
`serialize_config_block` emits it with the same resolved list, falling back to
`filament_colour_csv`). Behaviour at a non-default value is pinned by
`cube_4color_gcode_output_tdd` (`; extruder_colour =` directive, multiple
distinct colours). Recorded divergence, rationale: when a 3MF supplies an
`extruder_colour` distinct from `filament_colour`, the port honours the
3MF value while canonical always aliases to the filament value — real
Orca-authored files carry identical values, and the port's choice is strictly
closer to the authored data.

### The other six — blocked on the per-tool config model ruling (125)

`extruder_offset`, `extruder_type`, `master_extruder_id`, `physical_extruder_map`,
`printer_extruder_id`, `printer_extruder_variant` have **zero occurrences in
this tree**. Their canonical decision points are per-extruder vectors or whole
absent features, not gaps inside an existing emitter seam:

- `extruder_offset` — `GCode::point_to_gcode` / `GCode::gcode_to_point`
  subtract the active extruder's XY offset from every emitted coordinate, and
  `WipeTowerIntegration::post_process_wipe_tower_moves` /
  `WipeTowerIntegration::transform_gcode` add a bridging move when the offset
  changes at toolchange. Requires per-extruder offset vectors + emission
  geometry; the port's `crates/slicer-gcode` emitter has no offset term and the
  port's tool model is runtime entity `tool_index` ordering, not printer
  extruders.
- `extruder_type` — `ToolOrdering.cpp::build_filament_group_context`
  (`prefer_non_model_filament` bowden/direct preferencing) and
  `Print::update_filament_maps_to_config`'s variant backfill. No filament
  grouping or variant machinery exists.
- `master_extruder_id` — the `FilamentGroup.cpp` grouping algorithm
  (`FilamentGroup::calc_group_by_kmedoids` and the exhaustive scorer before
  it) — a whole absent feature.
- `physical_extruder_map` — internal→physical T-index mapping consumed at
  emission (`GCode::_do_export` first-filament reorder;
  `GCode::generate_timelapse_gcode` and the layer/wrapping-gcode placeholder
  seeds `curr_physical_extruder_id` / `most_used_physical_extruder_id`;
  `WipeTower`'s M104/M109 `T{map[…]}` targets). The port emits plain tool
  indices — `ToolIndexOutOfRange`-checked entity `tool_index` values — with no
  logical/physical split anywhere.
- `printer_extruder_id` / `printer_extruder_variant` — the *shape* keys of
  per-extruder option arrays (`ParameterUtils.cpp::get_index_for_extruder_parameter`,
  `update_values_to_printer_extruders`, `Print::get_filament_unprintable_flow`).
  This port has no per-extruder vector model at all (`nozzle_diameter` is a
  scalar `f32` in `ResolvedConfig`; `tool_config:<tool_index>:<key>` covers
  CLI-bound fields and manifest keys, not printer-extruder arrays), and
  whether an *extruder* axis is even distinct from the *tool* axis is open fog
  pending ticket 125's ruling.

Under Authoring rules 1–6 this is **not authorable now**: a packet that must
end with every key driving a behaviour-changing decision point would have to
build the per-extruder config model (the very thing 125/126 rule on), the
filament-grouping feature, the physical tool map, and offset-aware emission —
the missing decision points are the per-extruder model, not seven small
plumbing gaps.

### Disposition

- `extruder_colour`: **covered**; the queue carries no work for it.
- The other six: **re-filed as
  [136](136-author-packet-p32-per-extruder-keys-refiled.md)**, blocked on 125
  (the per-tool config model ruling, itself gated on 126) — the same
  blocker family the map already uses for ticket 119's keys and the Tier D
  fog. 136's body carries this ticket's per-key canonical grounding.

No production code changed; no new deviation rows; no new spec packet.
