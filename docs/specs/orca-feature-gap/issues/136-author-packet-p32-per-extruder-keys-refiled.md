# 136 — Author packet P32 (re-filed) — per-extruder identity / geometry / tool-map — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06, 125
Map: ../map.md

## Question

Re-filed from [ticket 39](./39-author-packet-p32-extruder-nozzle-extruder-geometry-mapping-emitter.md),
which re-sized P32 at claim time: **six of its seven keys are per-extruder-vector
or absent-feature keys and are not authorable under Authoring rule 1 until the
per-tool config model ruling ([125](./125-rule-per-tool-config-model.md), itself
gated on [126](./126-overlay-resolved-field-narrowing.md)) lands.**
**Read ticket 39's answer before starting** — it holds the per-key canonical
grounding, and is not restated here.

Keys (6, from P32 after ticket 39 ruled `extruder_colour` already covered):

`extruder_offset`, `extruder_type`, `master_extruder_id`,
`physical_extruder_map`, `printer_extruder_id`, `printer_extruder_variant`

Per-key canonical decision points (oracle: `GCode.cpp`/`Print.cpp`/
`ToolOrdering.cpp`/`FilamentGroup.cpp`, all per-extruder or absent-feature):

- `extruder_offset` — per-extruder XY offsets applied at emission:
  `GCode::point_to_gcode` / `GCode::gcode_to_point` subtract the active
  extruder's offset from every coordinate, and
  `WipeTowerIntegration::post_process_wipe_tower_moves` /
  `WipeTowerIntegration::transform_gcode` compensate wipe-tower moves and add a
  bridging move when the offset changes at `[change_filament_gcode]`. The port
  has no offset term anywhere in `crates/slicer-gcode` emission.
- `extruder_type` (bowden/direct per extruder) — feeds
  `ToolOrdering.cpp::build_filament_group_context`'s
  `prefer_non_model_filament` (multi-material filament preferencing) and
  `Print::update_filament_maps_to_config`'s variant backfill via
  `get_index_for_extruder`. Neither machinery exists in the port.
- `master_extruder_id` — filament-grouping algorithm
  (`FilamentGroup.cpp`: `FilamentGroup::calc_group_by_kmedoids` plus the
  exhaustive group scorer; seeds the full filament→tool map and the PAM
  starting point, and inflates the objective when the master is
  under-represented). The whole grouping feature is absent.
- `physical_extruder_map` — internal→physical tool index map consumed at
  emission: `GCode::_do_export`'s first-filaments reorder,
  `GCode::generate_timelapse_gcode`'s and the layer/wrapping gcode
  placeholder seeds (`curr_physical_extruder_id`,
  `most_used_physical_extruder_id`), and `WipeTower`'s M104/M109 `T{map[…]}`.
  The port emits plain runtime `tool_index` T values with no logical/physical
  split anywhere.
- `printer_extruder_id` / `printer_extruder_variant` — the *shape* of
  per-extruder option arrays: `ParameterUtils.cpp::get_index_for_extruder_parameter`
  resolves a slot index from both, `update_values_to_printer_extruders`
  (PrintApply/PrintConfig) reshapes arrays to matching extruders, and
  `Print::get_filament_unprintable_flow` reads the variant list directly. The
  port has no per-extruder vector model — `nozzle_diameter` is a scalar in
  `ResolvedConfig`; whether an *extruder* axis is even distinct from the *tool*
  axis is open fog pending 125.

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the per-extruder
  family again, the honest outcome is to defer this packet again — never
  declare the keys (Authoring rule 1). Per-key re-derive the owner from the
  *tree's* seams at claim time (ticket 27's hazard); the "emitter" owner from
  the tier table was reviewed against canonical, not against this port.
- The packet, when authorable, must either build every missing decision point
  (per-extruder vectors at the emission seam, filament grouping, physical tool
  map) or shed the unimplemented keys — it may not record them as
  declared-with-gap.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the per-extruder family out of scope.

Fold candidate: [142](./142-author-packet-p63-extruder-ams-count-refiled.md)
(`extruder_ams_count`, P63) shares this ticket's grouping subject (its capacity
feeds the same `FilamentGroup.cpp` scorer as `master_extruder_id`) and the same
125 blocker — consider folding it in at claim time before taking a new packet
number. Second fold candidate:
[143](./143-author-packet-p64-nozzle-volume-type-tool-ordering-refiled.md)
(`nozzle_volume_type`, P64) — per-extruder nozzle-list + unprintable-volume
arms of the same grouping subject, same 06 + 125 blocker.

## Answer
