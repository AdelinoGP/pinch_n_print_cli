# 90 — Author packet P83 — Multimaterial / Filament for Features — config-resolution

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-10) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06, 104, 105
Map: ../map.md

## Question

Author the spec packet for **P83 — Multimaterial / Filament for Features — config-resolution** — 2 keys, Tier B new logic, owner config-resolution. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P83 — Multimaterial / Filament for Features — config-resolution):

`filament_map`, `filament_map_mode`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, both keys re-filed, no packet, no code change** (the ticket-28/39 shape).

Claim-time grounding, verified against the oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) and re-derived from this tree:

- **Both pass rule 3** (live in `libslic3r/`, stay in scope). `filament_map` (`coInts`, default `{1}`, 1-based filament→extruder) is the grouping engine's manual input *and* its auto-mode write-back output: `Extruder::extruder_id` (`Extruder.cpp`) indexes it (`get_at(m_id) - 1`); `Print::get_extruder_id` / `Print::get_filament_maps` (`Print.cpp`) feed `GCode::get_extruder_id` (the emission-time filament→extruder resolver, degenerate `return 0` without a `Print`), the `Brim.cpp` per-object extruder printable-area select, and the `GCode.cpp` multi-extruder g-code validity check; the `fmmManual` / `fmmNozzleManual` branches of `ToolOrdering::get_recommended_filament_maps` (`GCode/ToolOrdering.cpp`) wrap the user's map directly (1-based → 0-based), with the multi-nozzle manual path verifying the engine's result against it (`result_map[fid] != filament_map - 1` → fatal `RuntimeError`); the auto modes derive the map from the engine and write it back through `Print::update_filament_maps_to_config` (used-only merge onto the config base, with the 1-element-default re-size guard). `filament_map_mode` (`coEnum` over `FilamentMapMode`, default `fmmAutoForFlush` — "Auto For Flush" / "Auto For Match" / "Manual" / "Nozzle Manual" / "Default", `PrintConfig.cpp`) is the dispatch that selects which of those behaviours runs: `Print::get_filament_map_mode` / `Print::is_dynamic_group_reorder` plus the static-vs-dynamic branch and the `map_mode < fmmManual` auto-only write-back gate (`Print.cpp`), and inside the grouping function the Flush-vs-Match `FGMode` select, the two manual wraps, the manual verification throw, the multi-nozzle manual/match branches, and the TPU split. The remaining touches (`bbs_3mf.cpp` plate `FILAMENT_MAP_MODE_ATTR` IO incl. the legacy `"Auto"` migration arm, `AppConfig` preferred-mode default, `update_values_to_printer_extruders*` preset reshaping) are file IO / app state / preset plumbing, not slicing decision points.
- **Zero occurrences of either spelling** under this tree's `crates/` + `modules/` + `xtask/` + `resources/` (verified by grep; the only `filament_map*` hits in-tree are the `03-asset-scoped-gap.md` class row, the reference snapshot rows, and the `slicer-model-io` sidecar's *plural* `filament_maps` plate-metadata passthrough test — a 3MF metadata capture, not a config read — so rule 2 is not even engaged). The tier table's `config-resolution` owner names the canonical seam, not a tree seam (ticket-27 hazard): neither the per-tool mechanism (`resolve_per_tool_configs` — `crates/slicer-scheduler/src/config_resolution.rs`, generic over the `tool_config:<idx>:` prefix) nor runtime entity assembly (`FeatureFilamentSelection` — six `Option<u32>` feature→tool overrides, `SupportToolSelection.tool_count` from `filament_density`'s list length — `crates/slicer-runtime/src/run.rs` / `layer_executor.rs`) consumes or produces either spelling, and no `ResolvedConfig` field, module manifest, or emitter site names them.
- **A packet today would be 100% declaration-only** (rule 1). Every live read above resolves a filament slot onto a per-extruder / nozzle inventory this port does not have: no per-extruder machine model (`nozzle_diameter` is an `extensions` scalar, not canonical's `coFloats` vector), no `build_default_nozzle_list` / `NozzleInfo` list, no `FilamentGroup` k-medoids scorer, no `LayeredNozzleGroupResult` table, no physical/logical tool split (all live on the single-tool default — 1-entry `filament_density` list → `tool_count = 1`). The single-extruder degenerate (`ret = all-master`, `create` wrap, `get_extruder_id` → 0) is what this port already *is* by construction — declaring the input for it would pin behaviour that is already the only behaviour. That is exactly what [125](125-rule-per-tool-config-model.md) rules on — open at claim time, and its sub-questions name the extruder-vs-tool axis and the ingest/vector-shape choices this pair needs answered. Declaring the dispatch enum before 125 lands is the ticket-39/41/88 shape.
- **Re-filed as [147](147-author-packet-p83-filament-map-refiled.md), blocked on 125** (the 28→119 / 39→136 / 41→137 / 69→141 / 88→146 pattern). 147 carries this ticket's per-key grounding and names the fold candidates: 136 (grouping scorer + per-extruder inventory this dispatch drives), 142/143 (same grouping subject, same blocker), 146 (variant-identity readers of the same grouping result). Adjacent, not folded: 124 consumes the grouping result on its sequential path but does not own the grouping subject.

No packet number taken, no deviation rows, no `ORCA_CONFIG_PADDING` edit. No code change; no queue-count change. 04/05 rows annotate the re-file.
