# 142 — Author packet P63 (re-filed) — extruder_ams_count — tool-ordering

Type: task
Status: open
Assignee: —
Blocked by: 06, 125
Map: ../map.md

## Question

Re-filed from [ticket 70](./70-author-packet-p63-extruder-nozzle-extruder-geometry-mapping-tool-ordering.md),
which re-sized P63 at claim time: **`extruder_ams_count` is a machine-inventory
key whose every live consumer rides the absent filament-grouping feature, and is
not authorable under Authoring rule 1 until the per-tool config model ruling
([125](./125-rule-per-tool-config-model.md)) lands.** **Read ticket 70's answer
before starting** — it holds the per-key canonical grounding and the from-disk
tree evidence, and is not restated here.

Key (1, Tier B, owner tool-ordering):

`extruder_ams_count`

Canonical decision points (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path
at point of use per the map Notes; ticket 70 pins the functions):

- `ToolOrdering.cpp::build_filament_group_context` — parses the key via
  `get_extruder_ams_count` (`PrintConfig.cpp`) into per-extruder slot maps and
  sizes the grouping engine through
  `FilamentGroupUtils::calc_max_group_size` (group-slot capacity per extruder)
  plus `build_machine_filaments` (the machine-side filament inventory the
  grouping scores against), subject to the `has_filament_switcher` capacity
  override. The capacity feeds the grouping scorer (`FilamentGroup.cpp` —
  size-limit reward, k-medoids cluster size limits, master-extruder
  under-representation penalty) — the same absent grouping feature ticket 39
  grounds for `master_extruder_id` (re-filed as ticket 136).
- `Print.cpp` (re-slice gate) — the key sits in the invalidation list alongside
  `filament_map*` / `filament_volume_map` / `filament_adhesiveness_category` and
  the wipe-tower cluster; that is orchestration bookkeeping, not a slicing
  behaviour. `PrintApply.cpp` erases it from the diff in non-auto map modes
  (GUI/apply plumbing), not a decision point.
- `PresetBundle.cpp` — printer-preset round-trip (`get_extruder_ams_count` /
  `save_extruder_ams_count_to_string`, `extruder_ams_counts.resize` on extruder
  count change): preset plumbing, not slicing behaviour.

Why it is blocked:

- **Absent grouping subject** — the port has no filament-grouping engine
  (`FilamentGroup.cpp` k-medoids / exhaustive scorer), no
  `build_filament_group_context` equivalent, and no per-extruder
  AMS-slot inventory; the only tool model is runtime entity `tool_index`
  ordering (`crates/slicer-runtime`), not printer extruders with AMS slots.
  The key's value shape (`"<slots>#<count>"` tokens joined by `"|"`, one
  entry per extruder; default `{}`) has no ingest, no owner seam, and zero
  occurrences under `crates/` / `modules/` / `xtask/`.
- **Per-extruder axis shape** — canonical declares `coStrings` (one AMS-count
  string per extruder); this port has no per-extruder vector model
  (`nozzle_diameter` is an `extensions` scalar, not a `ResolvedConfig` field)
  and whether an *extruder* axis is distinct from the *tool* axis is the open
  question ticket 125 rules on. The natural sibling is ticket 136's
  six-key per-extruder re-file (which shares the same grouping subject via
  `extruder_type`'s `prefer_non_model_filament` arm and `master_extruder_id`'s
  grouping scorer).

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the per-extruder
  family again, the honest outcome is to defer this packet again — never declare
  the key (Authoring rule 1). (06 resolved at ticket 70's claim time and rides
  136/125 transitively.)
- Per-key re-derive the owner from the *tree's* seams at claim time (ticket 27's
  hazard); "tool-ordering" names a canonical file, not a tree module — consider
  folding with ticket 136's per-extruder re-file, which owns the same grouping
  subject and blocker, before taking a new packet number.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the family out of scope.

## Answer
