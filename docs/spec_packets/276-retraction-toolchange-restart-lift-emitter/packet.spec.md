---
status: draft
packet: 276-retraction-toolchange-restart-lift-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/43-author-packet-p36-extruder-nozzle-retraction-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
---

# Packet Contract: 276-retraction-toolchange-restart-lift-emitter

## Goal

Make eight P36 retraction keys drive real emission behaviour: toolchange-length selection, normal/toolchange restart-extra, deretraction-speed fallback, Z-lift bound and surface gating in the host emitter, and the long-retraction placeholder publication through the module seam.

## Scope Boundaries

This packet adds eight scalar-global `ResolvedConfig` fields plus their `to_config_map` inserts: seven wire into `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) at the sites that already decide retract output (entity retract/unretract loops, toolchange synthesis, ZHop execution arms), and the eighth (`long_retractions_when_cut`) is additionally declared in `machine-gcode-emit.toml` — the ticket-42 shape, where the manifest declaration closes the 04-contract and the macro default is what the module's placeholder seam receives — and published there. It does not touch `ORCA_CONFIG_PADDING` (Authoring rule 2), does not build per-filament vectors (ticket-125 ruling), and does not declare `retract_before_wipe` (shed to P37) or `long_retractions_when_ec` (returned to the queue) anywhere.

## Prerequisites and Blockers

- Depends on [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md), [05 - Decide packet granularity and grouping](../specs/orca-feature-gap/issues/05-packet-granularity.md), [04 - Define the cost rubric that makes "cheapest-first" decidable](../specs/orca-feature-gap/issues/04-cost-tiering-rubric.md), [101 - Rename path-optimization keys to Orca names](../specs/orca-feature-gap/issues/101-rename-path-optimization-keys.md), and [107 - Collapse infill duplicate spellings to Orca names](../specs/orca-feature-gap/issues/107-collapse-infill-duplicate-spellings.md); all are resolved map decisions. This queue packet carries `task_ids: []` (packet 267 precedent).
- Related fog, not a blocker: [125 - Rule on the port's per-tool config model](../specs/orca-feature-gap/issues/125-rule-per-tool-config-model.md) owns the per-filament vector model; this packet records the scalar-global divergence (DEV-171) and does not wait for it.
- Unblocks [43 - Author packet P36 - Extruder / Nozzle / Retraction (1/2) - emitter](../specs/orca-feature-gap/issues/43-author-packet-p36-extruder-nozzle-retraction-emitter.md) for ticket closure.
- Activation blockers: none for the draft packet; activation remains a separate explicit `/swarm` decision.

## Acceptance Criteria

- **AC-1. Given** a two-tool fixture where the config sets `retract_length_toolchange = 15.0` (global `retract_length` stays `2.0`), **when** `DefaultGCodeEmitter::emit_gcode` synthesizes the pre-`T<n>` retract, **then** the synthesized `GCodeCommand::Retract.length` is `15.0`, and **when** `tool_config:1:retract_length_toolchange = 6.0` is set, **then** tool 1's synthesized retract length is `6.0` while tool 0's stays `15.0`. | `cargo test -p slicer-gcode --test gcode_toolchange_wrapping -- toolchange_retract_uses_toolchange_length 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** a single-tool travel fixture with `retract_restart_extra = 0.5` and base retract length `0.8`, **when** the entity unretract loop emits the post-travel `Unretract`, **then** its length is `1.3` (`0.8 + 0.5`), and **when** `retract_restart_extra = 0.0` (canonical default), **then** the unretract length is byte-identical to today (`0.8`). | `cargo test -p slicer-gcode --test gcode_emit_tdd -- restart_extra_adds_to_unretract 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** a two-tool fixture with `retract_restart_extra_toolchange = 0.4` (base `0.8`), **when** the post-`T<n>` unretract emits, **then** its length is `1.2`, while a mid-print non-toolchange unretract in the same run stays `0.8`. | `cargo test -p slicer-gcode --test gcode_toolchange_wrapping -- restart_extra_toolchange_only_after_toolchange 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** an unretract whose incoming speed is `1800.0` mm/min, **when** `deretraction_speed = 40.0` (mm/s), **then** the emitted `Unretract.speed` is `2400.0` (`40.0 * 60.0`), and **when** `deretraction_speed = 0.0` (canonical default), **then** the speed passes through unchanged (`1800.0`). | `cargo test -p slicer-gcode --test gcode_emit_tdd -- deretraction_speed_overrides_unretract 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** a fixture whose layers sit below `Z = 1.0`, **when** `retract_lift_above = 10.0`, **then** no Z-hop lift move is emitted for any `ZHop` in the layer, and **when** `retract_lift_above = 0.0` (canonical default), **then** every `ZHop` lifts exactly as today. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_above_suppresses 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** a fixture whose layers sit above `Z = 5.0`, **when** `retract_lift_below = 1.0`, **then** no Z-hop lift move is emitted for any `ZHop` in the layer, and **when** `retract_lift_below = 0.0` (canonical default), **then** every `ZHop` lifts exactly as today. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_below_suppresses 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** a three-layer fixture with `ZHop`s on every layer, **when** `retract_lift_enforce = "top_only"`, **then** lifts emit only on the topmost layer; **when** `"bottom_only"`, only on layer 0; **when** `"top_and_bottom"`, on layers 0 and 2 but not 1; and **when** `"all_surfaces"` (canonical default), on all three layers exactly as today. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_enforce_selects_layers 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-8. Given** `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` after this packet, **when** its `[config.schema]` is parsed, **then** it declares `long_retractions_when_cut` (bool, default `false`) with a `display` and `group`, and **when** a `change_filament_gcode` template references `[long_retractions_when_cut]` with the key set `true`, **then** the rendered G-code contains the canonical `1` spelling (and `0` at default). | `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- long_retraction_placeholder 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Negative Test Cases

- **AC-N1. Given** `retract_lift_enforce = "sometimes"` (not one of `all_surfaces`, `top_only`, `bottom_only`, `top_and_bottom`), **when** the slice runs, **then** it fails with a fatal config error naming the key and the four legal values, and no G-code is emitted. | `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_enforce_rejects_unknown 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular-pipeline + community-extensibility goals constraining rule 4; implementer dispatches, never loads directly).
- `docs/03_wit_and_manifest.md` - manifest `[config.schema]` key contract for the one module-manifest declaration (`long_retractions_when_cut`); implementer reads only the schema-key section.
- `docs/DEVIATION_LOG.md` - direct read of the last 20 lines only (DEV-171 row format + next free number confirmation).
- `docs/ORCA_CONFIG_REFERENCE.md` - P36 rows only, as the gap-source key list (never sized off its status column per map Notes).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - pin the exact `retract_lift_enforce` enum key strings and the `deretraction_speed` / lift-bound / restart-extra defaults this packet adopts.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - pin `lazy_lift` / `eager_lift` bound-comparison shape (strict vs inclusive, zero-as-sentinel) that Step 3 conforms to or records as a divergence.
- `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` - pin the `deretraction_speed <= 0` fallback and the restart-extra selection the emitter mirrors.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - pin the `long_retractions_when_cut` placeholder publication this packet mirrors in `machine-gcode-emit`.

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-171" - `rg -q 'DEV-171' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` (generated) entries for the eight new `ResolvedConfig` keys - `rg -q 'retract_length_toolchange' docs/15_config_keys_reference.md`.
- `docs/config/host-keys.toml` entries for the eight new host keys - `rg -q 'retract_length_toolchange' docs/config/host-keys.toml`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
