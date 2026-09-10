# 143 — Author packet P64 (re-filed) — nozzle_volume_type — tool-ordering

Type: task
Status: open
Assignee: —
Blocked by: 06, 125
Map: ../map.md

## Question

Re-filed from [ticket 71](./71-author-packet-p64-extruder-nozzle-nozzle-tool-ordering.md),
which re-sized P64 at claim time: **`nozzle_volume_type` is a per-extruder
machine-inventory key whose every live slicing consumer rides the absent
multi-nozzle filament-grouping feature, and is not authorable under Authoring
rule 1 until the per-tool config model ruling
([125](./125-rule-per-tool-config-model.md)) lands.** **Read ticket 71's
answer before starting** — it holds the per-key canonical grounding and the
from-disk tree evidence, and is not restated here.

Key (1, Tier B, owner tool-ordering):

`nozzle_volume_type`

Canonical decision points (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path
at point of use per the map Notes; ticket 71 pins the functions):

- `ToolOrdering.cpp::build_nozzle_groups` — reads the per-extruder vector
  (`nozzle_volume_type.values`, short-vector fallback `nvtStandard`) into
  `MultiNozzleUtils::NozzleGroupInfo` (with the `nvtHybrid` multi-group fan-out
  over `extruder_nozzle_counts`), feeding `build_filament_group_context`'s
  nozzle list and the grouping engine — the same absent grouping subject as
  tickets 136 (`master_extruder_id`) and 142 (`extruder_ams_count`).
- `ToolOrdering.cpp::build_default_nozzle_list` — the single-nozzle-per-extruder
  path's identical read (same fallback), feeding the manual/`fmmManual`
  grouping wraps and the sequential-path result.
- `add_volume_type_limits` (inside `build_filament_group_context`) — matches
  each nozzle's `volume_type` against the per-filament `unprintable_volumes` to
  mark filaments unprintable on single-nozzle-per-extruder machines.
- Named non-borrows, not decision points: `PrintConfig.cpp`'s legacy
  `"Normal" → "Standard"` / `"Big Traffic" → "High Flow"` migration spelling;
  `is_using_different_extruders`'s dirty-check comparison (config-equality
  bookkeeping, there is no port dirty-check seam); the `PresetBundle.cpp` /
  `PrintConfig.cpp` `update_values_to_printer_extruders` variant-slot reshaping
  (preset plumbing, not slicing behaviour); `GCode.cpp`'s
  `nozzle_volume_types[]` placeholder publication (derived FROM the grouping
  result, which does not exist here); `MultiNozzleUtils.cpp`'s
  `volume_type="…"` gcode.3mf serialization surface (same absent subject);
  `bbs_3mf.cpp`'s project read/write of the key and the per-filament
  `nozzle_volume_type` attribute (file IO, no slicing behaviour).

Why it is blocked:

- **Absent grouping subject** — the port has no filament-grouping engine
  (`FilamentGroup.cpp` k-medoids / exhaustive scorer), no
  `build_filament_group_context` equivalent, no `NozzleGroupInfo` /
  `NozzleInfo` model, no unprintable-volume marking, and no per-extruder
  `nozzle_diameter`/`nozzle_volume_type` vectors (`nozzle_diameter` is an
  `extensions` scalar here; `nozzle_volume_type` is zero-occurrence under
  `crates/` / `modules/` / `xtask/`). A packet today would have to build the
  grouping feature plus the per-extruder axis — the very thing ticket 125
  rules on — which is 100% declaration-only prohibited by rule 1.
- **Per-extruder axis shape** — canonical declares `coEnums` (one entry per
  extruder, short-vector fallback to `nvtStandard`); this port has no
  per-extruder vector model and whether an *extruder* axis is distinct from the
  *tool* axis is the open question ticket 125 rules on. The natural siblings
  are ticket 136's six-key per-extruder re-file and ticket 142's AMS-count
  re-file (which shares the same grouping subject and blocker).

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the per-extruder
  family again, the honest outcome is to defer this packet again — never declare
  the key (Authoring rule 1). (06 resolved at ticket 71's claim time and rides
  125 transitively.)
- Per-key re-derive the owner from the *tree's* seams at claim time (ticket 27's
hazard); "tool-ordering" names a canonical file, not a tree module — consider
folding with tickets 136 and/or 142, which own the same grouping subject and
blocker, before taking a new packet number. The sibling
`default_nozzle_volume_type` ruled out of scope by ticket 89 (P82) —
preset-management, not grouping — is not a fold candidate.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the family out of scope.

## Answer
