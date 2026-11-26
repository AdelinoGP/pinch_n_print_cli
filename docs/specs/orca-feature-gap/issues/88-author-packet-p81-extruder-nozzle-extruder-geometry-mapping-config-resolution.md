# 88 — Author packet P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260910_P81) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06, 104, 105
Map: ../map.md

## Question

Author the spec packet for **P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution** — 5 keys, Tier B new logic, owner config-resolution. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution):

`extruder_variant_list`, `filament_extruder_variant`, `filament_self_index`, `print_extruder_id`, `print_extruder_variant`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, all five keys re-filed, no packet, no code change** (the ticket-28/39 shape).

Claim-time grounding, verified against the oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) and re-derived from this tree:

- **All five pass rule 3** (live in `libslic3r/`, stay in scope). `extruder_variant_list` (`coStrings`, per-extruder, default `{"Direct Drive Standard"}` — dynamic-only, no `PrintConfig` static member) gates variant grouping in `DynamicPrintConfig::support_different_extruders` (`PrintConfig.cpp`) and resolves the fallback slot in `DynamicPrintConfig::get_index_for_extruder`. `filament_extruder_variant` (`coStrings`, per-filament-variant) + `filament_self_index` (`coInts`, per-filament-variant, default `{1}`) feed `Print::get_filament_config_indx` (`Print.cpp`, variant-list + self-index-list args to the shared `get_config_index` kernel), with `filament_self_index` loaded by `Print::update_filament_self_index_cache` (full → ori → base priority, sized/clamped by the variant length) and grouped by `Print::get_filament_unprintable_flow`. `print_extruder_id` (`coInts`, per-process-variant, default `{1}`) + `print_extruder_variant` (`coStrings`, per-process-variant) feed `Print::get_nozzle_config_index` (variant-list + id-list args) and the `get_index_for_extruder` slot resolution. Everything else touching these spellings (`update_values_to_printer_extruders*`, `Preset`/`PresetBundle` sizing, `PrintApply` seeding, `ParameterUtils::get_index_for_extruder_parameter` plumbing) is preset/config-expansion machinery, not a slicing decision point.
- **Zero occurrences of all five spellings** under this tree's `crates/` + `modules/` + `xtask/` + `resources/` (verified by grep; no `ORCA_CONFIG_PADDING` twin for any of the five either, so rule 2 is not even engaged). The tier table's `config-resolution` owner names the canonical seam, not a tree seam (ticket-27 hazard): neither the per-tool mechanism (`resolve_per_tool_configs` — `crates/slicer-scheduler/src/config_resolution.rs`, generic over the `tool_config:<idx>:` prefix, zero hits for all five spellings) nor runtime entity assembly (`FeatureFilamentSelection` — six `Option<u32>` feature→tool overrides, `assemble_ordered_entities_with_support_identities` — `crates/slicer-runtime/src/layer_executor.rs`, zero hits) consumes or produces any of them.
- **A packet today would be 100% declaration-only** (rule 1). Every live read above resolves a per-extruder / per-filament-variant / per-process-variant *vector slot* onto a printer inventory this port does not have: no per-extruder machine model (`extruder_offset`, `extruder_type`, `nozzle_diameter` scalar, no grouping engine), no `FilamentGroup` scorer, no physical/logical tool split. That is exactly what [125](125-rule-per-tool-config-model.md) rules on — open at claim time, and its sub-questions name this family explicitly ("Is an extruder axis separate from the tool axis in scope?", per-filament vs per-extruder vector distinction unsettled, "Declare[s] no key as part of this ticket"). Declaring five vector keys onto a scalar `ResolvedConfig` before 125 lands is the ticket-39/41 shape.
- **Re-filed as [146](146-author-packet-p81-variant-identity-refiled.md), blocked on 125** (the 28→119 / 39→136 / 41→137 / 69→141 pattern). 146 carries this ticket's per-key grounding and names the fold candidates: 136 (already blocked on 125 — six per-extruder P32 keys over the same absent inventory), 142/143 (same blocker family; 143's `add_volume_type_limits` unprintable-volume marking consumes `Print::get_filament_unprintable_flow`, one of this family's own readers).

No packet number taken, no deviation rows, no `ORCA_CONFIG_PADDING` edit. No code change; no queue-count change. 04/05 rows annotate the re-file.
