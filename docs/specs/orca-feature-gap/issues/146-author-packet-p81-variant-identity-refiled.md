# 146 — Author packet P81 (re-filed) — per-extruder / variant identity keys — config-resolution

Type: task
Status: open
Assignee: —
Blocked by: 125
Map: ../map.md

## Question

Re-filed from [ticket 88](88-author-packet-p81-extruder-nozzle-extruder-geometry-mapping-config-resolution.md),
which re-sized P81 at claim time: **all five keys are per-extruder / per-filament-variant /
per-process-variant vector keys whose live consumers all resolve a vector slot onto a printer
inventory this port does not have, and are not authorable under Authoring rule 1 until the
per-tool config model ruling ([125](125-rule-per-tool-config-model.md)) lands.**
**Read ticket 88's answer before starting** — it holds the per-key canonical grounding and the
from-disk tree evidence, and is not restated here.

Keys (5, from P81 after ticket 88's grounding — membership unchanged from
[05-asset-packet-list.md](05-asset-packet-list.md) P81):

`extruder_variant_list`, `filament_extruder_variant`, `filament_self_index`, `print_extruder_id`,
`print_extruder_variant`

Canonical decision points (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` —
re-derive the path at point of use per the map Notes; ticket 88 pins the functions):

- `extruder_variant_list` — `DynamicPrintConfig::support_different_extruders` (variant grouping
  gate) + `DynamicPrintConfig::get_index_for_extruder` (fallback slot resolution).
- `filament_extruder_variant` + `filament_self_index` — `Print::get_filament_config_indx`
  (the shared `get_config_index` kernel), `Print::update_filament_self_index_cache` (self-index
  cache load), `Print::get_filament_unprintable_flow` (variant→volume-type grouping feeding
  tool ordering).
- `print_extruder_id` + `print_extruder_variant` — `Print::get_nozzle_config_index` +
  `get_index_for_extruder` slot resolution.

All other touches (`update_values_to_printer_extruders*`, `Preset`/`PresetBundle` sizing,
`PrintApply` seeding, `ParameterUtils::get_index_for_extruder_parameter`) are preset/config-expansion
plumbing, not slicing decision points. All five pass rule 3 (live in `libslic3r/`, in scope);
zero occurrences of all five spellings under this tree's `crates/` + `modules/` + `xtask/` +
`resources/` (ticket 88, verified by grep).

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the per-extruder / variant-identity
  family again, the honest outcome is to defer this packet again — never declare the keys
  (Authoring rule 1). Per-key re-derive the owner from the *tree's* seams at claim time
  (ticket 27's hazard); the "config-resolution" owner from the tier table names canonical's
  seam, not this port's.
- The packet, when authorable, must either build every missing decision point (the variant-slot
  resolution this family needs — per-extruder machine model, variant index maps) or shed the
  unimplemented keys — it may not record them as declared-with-gap.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked here — or when a
later ruling rules the family out of scope.

Fold candidates: [136](136-author-packet-p32-per-extruder-keys-refiled.md) (six per-extruder P32
keys over the same absent inventory, same 125 blocker), [142](142-author-packet-p63-extruder-ams-count-refiled.md)
(same blocker family, grouping subject), [143](143-author-packet-p64-nozzle-volume-type-tool-ordering-refiled.md)
(per-extruder nozzle-list + `add_volume_type_limits` unprintable-volume arms consuming
`Print::get_filament_unprintable_flow`, one of this family's own readers, same blocker family) —
consider folding at claim time before taking a new packet number.

## Answer