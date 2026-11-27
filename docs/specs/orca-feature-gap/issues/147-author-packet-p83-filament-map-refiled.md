# 147 — Author packet P83 (re-filed) — filament map / map mode — config-resolution

Type: task
Status: open
Assignee: —
Blocked by: 125
Map: ../map.md

## Question

Re-filed from [ticket 90](90-author-packet-p83-multimaterial-filament-for-features-config-resolution.md),
which re-sized P83 at claim time: **both keys are the filament-grouping
engine's input (the map) and dispatch selector (the mode), and every live
slicing consumer resolves a filament slot onto a per-extruder / nozzle
inventory this port does not have — not authorable under Authoring rule 1
until the per-tool config model ruling ([125](125-rule-per-tool-config-model.md))
lands.** **Read ticket 90's answer before starting** — it holds the per-key
canonical grounding and the from-disk tree evidence, and is not restated here.

Keys (2, from P83 — membership unchanged from
[05-asset-packet-list.md](05-asset-packet-list.md) P83):

`filament_map`, `filament_map_mode`

Canonical decision points (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` —
re-derive the path at point of use per the map Notes; ticket 90 pins the functions):

- `filament_map` (`coInts`, default `{1}`, 1-based filament→extruder) —
  `Extruder::extruder_id` (`Extruder.cpp`), `Print::get_extruder_id` /
  `Print::get_filament_maps` (`Print.cpp`, feeding `GCode::get_extruder_id`,
  the `Brim.cpp` per-object extruder-area select, and the `GCode.cpp`
  multi-extruder validity check), and the `fmmManual` / `fmmNozzleManual`
  direct-wrap branches of `ToolOrdering::get_recommended_filament_maps`
  (`GCode/ToolOrdering.cpp`). The auto-mode write-back
  (`Print::update_filament_maps_to_config`) makes it an engine *output* there,
  not an input. Preset reshaping (`update_values_to_printer_extruders*`) is
  plumbing, not a decision point.
- `filament_map_mode` (`coEnum` over `FilamentMapMode`, default
  `fmmAutoForFlush`) — `Print::get_filament_map_mode` /
  `Print::is_dynamic_group_reorder` plus the static/dynamic branch and the
  `map_mode < fmmManual` write-back gate (`Print.cpp`), and the full mode
  dispatch inside `ToolOrdering::get_recommended_filament_maps` (Flush vs Match
  `FGMode`, manual / nozzle-manual wraps, manual verification throw,
  multi-nozzle branches, TPU split). 3MF plate IO, the legacy
  `"Auto"` migration spelling, and `AppConfig` preferred-mode state are
  file-IO / app-state non-borrows.

All other touches are preset/config-expansion plumbing. Both keys pass rule 3
(live in `libslic3r/`, in scope); zero occurrences of either spelling under
this tree's `crates/` + `modules/` + `xtask/` + `resources/` (ticket 90,
verified by grep — the only `filament_map*` hits are the reference snapshot
rows, the sidecar's plural `filament_maps` plate-metadata passthrough test,
and map prose).

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the per-extruder /
  grouping family again, the honest outcome is to defer this packet again —
  never declare the keys (Authoring rule 1). Per-key re-derive the owner from
  the *tree's* seams at claim time (ticket 27's hazard); the
  "config-resolution" owner from the tier table names canonical's seam, not
  this port's.
- The packet, when authorable, must either build every missing decision point
  (the filament→extruder/nozzle slot resolution this pair needs — per-extruder
  machine model, nozzle list, grouping engine, `LayeredNozzleGroupResult`
  table) or shed the unimplemented keys — it may not record them as
  declared-with-gap.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the family out of scope.

Fold candidates: [136](136-author-packet-p32-per-extruder-keys-refiled.md) (the
grouping scorer + per-extruder inventory this dispatch drives, same 125
blocker), [142](142-author-packet-p63-extruder-ams-count-refiled.md) /
[143](143-author-packet-p64-nozzle-volume-type-tool-ordering-refiled.md) (same
grouping subject and blocker), [146](146-author-packet-p81-variant-identity-refiled.md)
(variant-identity family whose `get_filament_config_indx` /
`get_nozzle_config_index` readers consume the same grouping result, same
blocker) — consider folding at claim time before taking a new packet number.
Adjacent, not folded: [124](124-author-packet-sequential-printing-and-toolhead-clearance.md)
consumes the grouping result on its sequential path but does not own the
grouping subject.

## Answer
