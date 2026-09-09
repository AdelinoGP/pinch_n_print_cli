# 70 — Author packet P63 — Extruder / Nozzle / Extruder geometry / mapping — tool-ordering

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260908_P63) — claimed 2026-09-08
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P63 — Extruder / Nozzle / Extruder geometry / mapping — tool-ordering** — 1 keys, Tier B new logic, owner tool-ordering. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P63 — Extruder / Nozzle / Extruder geometry / mapping — tool-ordering):

`extruder_ams_count`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, re-filed, no packet, no code change**
(the ticket-28/39 shape). `extruder_ams_count` is a machine-inventory key whose
every live consumer rides the absent filament-grouping feature — not a
declare-and-wire key.

Claim-time grounding (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — paths re-derived at
point of use per the map Notes; OrcaSlicer cited by file + function, never line
numbers):

- Canonical declaration: `coStrings`, default `{}`, internal-use (`PrintConfig.cpp`;
  reference snapshot says `comDevelop`, tree says internal-use — the tree wins for
  scope, neither changes the sizing). Value shape is per-extruder: one
  `"<slots>#<count>"`-token string per extruder joined by `"|"`
  (`get_extruder_ams_count` / `save_extruder_ams_count_to_string`).
- Live slicing reads, all inside `ToolOrdering.cpp::build_filament_group_context`
  (called from `ToolOrdering::get_recommended_filament_maps`): the key is parsed
  and feeds `FilamentGroupUtils::calc_max_group_size` (per-extruder group-slot
  capacity) plus `build_machine_filaments` (machine-side filament inventory),
  subject to the `has_filament_switcher` capacity override. The capacity then
  feeds the grouping scorer (`FilamentGroup.cpp` — size-limit reward, k-medoids
  cluster size limits, master-extruder under-representation penalty) — the same
  absent grouping feature ticket 39 grounds for `master_extruder_id`. Rule 3
  passes (live in `libslic3r/`); the tier-table owner stands as far as it goes
  ("tool-ordering" names a canonical file — the tree has no such module, so the
  packet-time owner must be re-derived from the tree's seams, ticket-27 hazard).
- Named non-borrows, not decision points: `Print.cpp`'s re-slice gate entry
  (invalidation bookkeeping alongside `filament_map*`, not a behaviour);
  `PrintApply.cpp`'s diff-erase in non-auto map modes (GUI/apply plumbing);
  `PresetBundle.cpp`'s printer-preset round-trip (`get`/`save` helpers,
  `extruder_ams_counts.resize` on extruder-count change).
- Tree state: **zero occurrences** under `crates/` / `modules/` / `xtask/` (the
  one map-prose hit is this queue's own ledger). The port has no
  filament-grouping engine, no `build_filament_group_context` equivalent, no
  per-extruder AMS-slot inventory, and no per-extruder vector model
  (`nozzle_diameter` is an `extensions` scalar) — the only tool model is runtime
  entity `tool_index` ordering. A packet today would have to build the grouping
  feature plus the per-extruder axis — the very thing ticket 125 rules on —
  which is 100% declaration-only prohibited by rule 1.

### Disposition

- **Re-filed as
  [142](142-author-packet-p63-extruder-ams-count-refiled.md), blocked on 06 + 125**
  — the same blocker family the map already uses for ticket 136's six-key
  per-extruder re-file (which shares the grouping subject via `extruder_type`'s
  `prefer_non_model_filament` arm and `master_extruder_id`'s grouping scorer)
  and ticket 119 / the Tier D fog. 142's body carries this ticket's per-key
  canonical grounding. Consider folding with 136 at claim time (same subject,
  same blocker) before taking a new packet number.

No production code changed; no new deviation rows; no new spec packet.
