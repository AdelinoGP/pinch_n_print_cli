---
status: draft
packet: 277-retraction-wipe-travel-firmware-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/44-author-packet-p37-extruder-nozzle-retraction-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
---

# Packet Contract: 277-retraction-wipe-travel-firmware-emitter

## Goal

Make eleven P37 retraction keys drive real emission behaviour: minimum-travel and layer-change retract gates, wipe-move emission with before-wipe split, firmware G10/G11 mode, Z-hop style selection with slope angle, Z-offset shift, and the cut/EC distance placeholder publication through the module seam.

## Scope Boundaries

This packet adds eleven scalar-global `ResolvedConfig` fields plus their `to_config_map` inserts: nine wire into `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) at the sites that already decide retract/travel/Z output (entity retract/unretract loops, layer-change path, ZHop execution arms, preamble Z), and two (`retraction_distances_when_cut`, `retraction_distances_when_ec`) are additionally declared in `machine-gcode-emit.toml` — the ticket-42/276 shape, where the manifest declaration closes the 04-contract and the macro default is what the module's placeholder seam receives — and published there. It does not touch `ORCA_CONFIG_PADDING` (Authoring rule 2), does not build per-filament vectors (ticket-125 ruling), and declares no key outside the eleven.

## Prerequisites and Blockers

- Depends on [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md), [05 - Decide packet granularity and grouping](../specs/orca-feature-gap/issues/05-packet-granularity.md), [04 - Define the cost rubric that makes "cheapest-first" decidable](../specs/orca-feature-gap/issues/04-cost-tiering-rubric.md), [101 - Rename path-optimization keys to Orca names](../specs/orca-feature-gap/issues/101-rename-path-optimization-keys.md), and [107 - Collapse infill duplicate spellings to Orca names](../specs/orca-feature-gap/issues/107-collapse-infill-duplicate-spellings.md); all are resolved map decisions. This queue packet carries `task_ids: []` (packet 267 precedent).
- Related fog, not a blocker: [125 - Rule on the port's per-tool config model](../specs/orca-feature-gap/issues/125-rule-per-tool-config-model.md) owns the per-filament vector model; this packet records the scalar-global divergence (DEV-172) and does not wait for it.
- Unblocks [44 - Author packet P37 - Extruder / Nozzle / Retraction (2/2) - emitter](../specs/orca-feature-gap/issues/44-author-packet-p37-extruder-nozzle-retraction-emitter.md) for ticket closure.
- Activation blockers: none for the draft packet; activation remains a separate explicit `/swarm` decision.

## Acceptance Criteria

- **AC-1. Given** a single-tool fixture with two travels (XY lengths `1.0` mm and `5.0` mm) and `retraction_minimum_travel = 2.0` (canonical default), **when** `DefaultGCodeEmitter::emit_gcode` renders the entity retract loop, **then** the `1.0` mm travel emits no `GCodeCommand::Retract` while the `5.0` mm travel emits one, and **when** `retraction_minimum_travel = 0.0`, **then** both travels emit retracts exactly as today. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- minimum_travel_gates_retract 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** a two-layer single-tool fixture with `retract_when_changing_layer = true`, **when** the layer-change path emits, **then** a `GCodeCommand::Retract` precedes the Z move to layer 1, and **when** `retract_when_changing_layer = false` (canonical default), **then** no layer-change retract emits (byte-identical to today). | `cargo test -p slicer-gcode --test gcode_emit_tdd -- retract_on_layer_change 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** a single-tool travel fixture with `wipe = true` and `wipe_distance = 2.0` (canonical defaults `false` / `1.0`), **when** the retract site emits, **then** a wipe `GCodeCommand::Move` with XY extent `2.0` mm follows the retract and precedes the travel, and **when** `wipe = false` (default), **then** no wipe move emits (byte-identical to today). | `cargo test -p slicer-gcode --test gcode_emit_tdd -- wipe_emits_move 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** a wipe-enabled fixture with total retract length `2.0` and `retract_before_wipe = 50.0` (percent, canonical default `100.0`), **when** the split site emits, **then** the pre-wipe `Retract.length` is `1.0` and a post-wipe `Retract.length` of `1.0` follows the wipe move, and **when** `retract_before_wipe = 100.0` (default), **then** the full `2.0` retracts before the wipe with no post-wipe remainder. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- before_wipe_splits_retract 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** a single-tool travel fixture with `use_firmware_retraction = true`, **when** the retract/unretract sites emit, **then** the commands carry `slicer_ir::RetractMode::Firmware` (serializing to `G10` / `G11`), and **when** `use_firmware_retraction = false` (canonical default), **then** the commands carry `slicer_ir::RetractMode::Gcode` exactly as today. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- firmware_mode_selects_g10_g11 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** a fixture with a `ZHop` of height `0.4` and `z_hop_types = "normal"`, **when** the ZHop arm emits, **then** the lift is a vertical-only `Move` (Z set, X/Y `None`), and **when** `z_hop_types = "slope"` with `travel_slope = 3.0` (canonical defaults `slope` / `3.0`), **then** the lift is a diagonal `Move` (X/Y lead-in plus Z) whose XY extent equals `0.4 / tan(3.0°)` within `1e-3`. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- hop_type_selects_shape 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** a two-layer fixture with `z_offset = 0.5` (canonical default `0.0`), **when** the preamble and layer-change paths emit, **then** every emitted Z target equals `layer_z + 0.5`, and **when** `z_offset = 0.0` (default), **then** every Z target equals `layer_z` exactly as today. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- z_offset_shifts_targets 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-8. Given** `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` after this packet, **when** its `[config.schema]` is parsed, **then** it declares `retraction_distances_when_cut` (float, default `18.0`) with a `display` and `group`, and **when** a `change_filament_gcode` template references `[retraction_distances_when_cut]` with the key set `25.0`, **then** the rendered G-code contains `25.0` (and `18.0` at default). | `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- cut_distance_placeholder 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-9. Given** the same manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares `retraction_distances_when_ec` (float, default `10.0`) with a `display` and `group`, and **when** a `change_filament_gcode` template references `[retraction_distances_when_ec]` with the key set `12.0`, **then** the rendered G-code contains `12.0` (and `10.0` at default). | `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- ec_distance_placeholder 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Negative Test Cases

- **AC-N1. Given** `z_hop_types = "helicopter"` (not one of `auto`, `normal`, `slope`, `spiral`), **when** the slice runs, **then** it fails with a fatal config error naming the key and the four legal values, and no G-code is emitted. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- hop_type_rejects_unknown 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular-pipeline + community-extensibility goals constraining rule 4; implementer dispatches, never loads directly).
- `docs/03_wit_and_manifest.md` - manifest `[config.schema]` key contract for the two module-manifest declarations (`retraction_distances_when_cut`, `retraction_distances_when_ec`); implementer reads only the schema-key section.
- `docs/DEVIATION_LOG.md` - direct read of the last 20 lines only (DEV-172 row format + next free number confirmation; re-derive at implementation time, never trust this packet's number).
- `docs/ORCA_CONFIG_REFERENCE.md` - P37 rows only, as the gap-source key list (never sized off its status column per map Notes).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - pin the eleven P37 defaults and types (`retract_before_wipe` coPercents 100, `retract_when_changing_layer` coBools false, `_cut`/`_ec` coFloats 18/10, `retraction_minimum_travel` 2, `travel_slope` 3, `use_firmware_retraction` false, `wipe` false, `wipe_distance` 1, `z_hop_types` zhtSlope, `z_offset` 0) plus the `z_hop_types` enum strings this packet normalizes to snake-case.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - pin `needs_retraction` minimum-travel gate, `retract` wipe gate, `change_layer` retract + `z_offset` adjustment, `Wipe::calculateWipeRetractionLengths` split arithmetic, and the `_cut`/`_ec` placeholder publication (`append_tcr`, `update_placeholder_parser_with_variant_params`, `do_export` flag) this packet mirrors.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - pin `_retract`/`unretract` firmware G10/G11 shape, `retract` before-wipe split call, and `travel_to_xyz` slope-lift math (`atan2(dz, xy) < travel_slope`, XY extent `dz / tan`) that Step 5 conforms to or records as a divergence.
- `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` - pin `retract_before_wipe` percent-to-factor clamp and `travel_slope` degrees-to-radians conversion the emitter mirrors.

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-172" - `rg -q 'DEV-172' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` (generated) entries for the eleven new `ResolvedConfig` keys - `rg -q 'retract_before_wipe' docs/15_config_keys_reference.md`.
- `docs/config/host-keys.toml` entries for the eleven new host keys - `rg -q 'retract_before_wipe' docs/config/host-keys.toml`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
