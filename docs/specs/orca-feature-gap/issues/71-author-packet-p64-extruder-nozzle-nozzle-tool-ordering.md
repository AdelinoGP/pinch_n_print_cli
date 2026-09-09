# 71 — Author packet P64 — Extruder / Nozzle / Nozzle — tool-ordering

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260908_P64) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P64 — Extruder / Nozzle / Nozzle — tool-ordering** — 1 keys, Tier B new logic, owner tool-ordering. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P64 — Extruder / Nozzle / Nozzle — tool-ordering):

`nozzle_volume_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, re-filed, no packet, no code change**
(the ticket-28/39 shape). `nozzle_volume_type` is a per-extruder
machine-inventory key whose every live slicing consumer rides the absent
multi-nozzle filament-grouping feature — not a declare-and-wire key.

Claim-time grounding (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — paths re-derived at
point of use per the map Notes; OrcaSlicer cited by file + function, never line
numbers):

- Canonical declaration: `coEnums`, default `nvtStandard`, four values
  (`Standard` / `High Flow` / `Hybrid` / `TPU High Flow`), `comSimple`,
  per-extruder vector (`PrintConfig.cpp`). Rule 3 passes — live in `libslic3r/`,
  stays in scope. The tier-table owner stands as far as it goes
  ("tool-ordering" names a canonical file — the tree has no such module, so the
  packet-time owner must be re-derived from the tree's seams, ticket-27 hazard).
- Live slicing reads, all inside the multi-nozzle grouping subject:
  `ToolOrdering.cpp::build_nozzle_groups` (per-extruder read with the
  short-vector `nvtStandard` fallback, feeding `build_filament_group_context`'s
  nozzle list; `nvtHybrid` fans out over `extruder_nozzle_counts`) and
  `ToolOrdering.cpp::build_default_nozzle_list` (the same read on the
  single-nozzle-per-extruder path), plus the `add_volume_type_limits` match of
  nozzle `volume_type` against per-filament `unprintable_volumes` for
  single-nozzle-per-extruder machines. All feed the `FilamentGroup.cpp`
  grouping scorer — the same absent feature tickets 136 and 142 ground.
- Named non-borrows, not decision points: the `PrintConfig.cpp` legacy
  `"Normal" → "Standard"` / `"Big Traffic" → "High Flow"` migration spelling;
  `is_using_different_extruders`'s dirty-check (config-equality bookkeeping, no
  port seam); the `PresetBundle.cpp` / `PrintConfig.cpp`
  `update_values_to_printer_extruders` variant-slot reshaping (preset
  plumbing); `GCode.cpp`'s `nozzle_volume_types[]` placeholder publication
  (derived from the grouping result, which does not exist here);
  `MultiNozzleUtils.cpp`'s gcode.3mf serialization surface; `bbs_3mf.cpp`'s
  project read/write of the key and the per-filament attribute (file IO).
- Tree state: **zero occurrences** under `crates/` / `modules/` / `xtask/` (only
  map-prose ledger hits — 04/05/ticket bodies — plus the reference snapshot rows
  and tickets 89's sibling key). The port has no grouping engine, no
  `NozzleGroupInfo` / `NozzleInfo` model, no unprintable-volume marking, and no
  per-extruder vector model (`nozzle_diameter` is an `extensions` scalar) — the
  only tool model is runtime entity `tool_index` ordering. A packet today would
  have to build the grouping feature plus the per-extruder axis — the very thing
  ticket 125 rules on — which is 100% declaration-only prohibited by rule 1.

### Disposition

- **Re-filed as
  [143](143-author-packet-p64-nozzle-volume-type-tool-ordering-refiled.md), blocked on 06 + 125**
  — the same blocker family the map already uses for ticket 136's six-key
  per-extruder re-file and ticket 142's AMS-count re-file (which shares this
  ticket's grouping subject). 143's body carries this ticket's per-key
  canonical grounding. Consider folding with 136 and/or 142 at claim time
  (same subject, same blocker) before taking a new packet number. The sibling
  `default_nozzle_volume_type` (ticket 89, P82) stays separate — it is
  config-resolution preset plumbing, not tool-ordering grouping.

No production code changed; no new deviation rows; no new spec packet.
