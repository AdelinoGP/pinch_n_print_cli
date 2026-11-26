# Requirements: 276-retraction-toolchange-restart-lift-emitter

## Packet Metadata

- Grouped task IDs: none - queue packet; implementation is recorded against [43 - Author packet P36 - Extruder / Nozzle / Retraction (1/2) - emitter](../specs/orca-feature-gap/issues/43-author-packet-p36-extruder-nozzle-retraction-emitter.md).
- Backlog source: `docs/specs/orca-feature-gap/issues/43-author-packet-p36-extruder-nozzle-retraction-emitter.md` (P36 in the wayfinder map "Close the OrcaSlicer FFF feature gap").
- Packet number: 276, allocated from disk at authoring time per [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md) (highest committed packet dir is 275).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P36's ten retraction keys are Tier B new logic owned by the host emitter, and all ten are zero-occurrence in `crates/`, `modules/`, and `xtask/` — the "Tier B new logic" sizing survived grounding, but the owner needs narrowing: seven keys decide inside `DefaultGCodeEmitter::emit_gcode` (toolchange synthesis, unretract rendering, ZHop execution), while `long_retractions_when_cut` publishes through the `machine-gcode-emit` placeholder seam that already mirrors canonical's `update_placeholder_parser_with_variant_params`. Two keys leave the packet at authoring (human-approved scope ruling): `retract_before_wipe` needs wipe moves that land in P37, and `long_retractions_when_ec` has no geometric decision point anywhere (placeholder-only, nullable per-filament) so it returns to the queue with that named. Canonical spells six of the eight kept keys per-filament; this packet declares scalar-globals per the ticket-04 precedent and records DEV-171, leaving the vector model to the ticket-125 ruling.

## In Scope

- `retract_length_toolchange` (float, canonical default `10.0`): toolchange-synthesis length selection in `emit_gcode`, honouring `tool_config:<idx>:` overrides exactly like `retract_length_for_tool`.
- `retract_restart_extra` / `retract_restart_extra_toolchange` (floats, default `0.0`): added to `Unretract.length` at the entity-unretract site (normal) and the post-toolchange site (toolchange variant).
- `deretraction_speed` (float mm/s, default `0.0` = same-as-retraction): overrides `Unretract.speed` at render as `value * 60.0` (GCodeCommand speeds are mm/min at the serialize boundary: emitter synthesis uses `2400.0`, serialize passes through).
- `retract_lift_above` / `retract_lift_below` (floats mm, default `0.0` = bound disabled): gate ZHop execution arms on `layer_z`.
- `retract_lift_enforce` (string-parsed enum, default `"all_surfaces"`): layer-position gating (`top_only` = topmost layer, `bottom_only` = layer 0, `top_and_bottom` = both); unknown values fail the slice (AC-N1).
- `long_retractions_when_cut` (bool, default `false`): eighth `ResolvedConfig` field + `to_config_map` insert (ticket-42 shape — the macro default is what the module receives) plus the `machine-gcode-emit.toml` declaration, published through the module's existing placeholder substitution with a key-specific canonical `1`/`0` rendering guarantee (the generic formatter renders word-form `Bool`).
- DEV-171 (scalar-global vs canonical per-filament vectors), generated config reference + `host-keys.toml` entries, guest rebuild for the one manifest touched.

## Out of Scope

- `retract_before_wipe` — shed to P37 (ticket 44): it partitions retraction around wipe moves this tree does not emit yet; wiring it here would be declaration-only under Authoring rule 1.
- `long_retractions_when_ec` — returned to the queue as unimplemented (tier-table row annotated): `ConfigOptionBoolsNullable` with no geometric read site; the `_cut` placeholder wiring names its landing pattern when claimed.
- Per-filament vectors for any kept key — stays with the [125 - Rule on the port's per-tool config model](../specs/orca-feature-gap/issues/125-rule-per-tool-config-model.md) ruling; DEV-171 records the scalar choice.
- Firmware-mode (`G10`/`G11`) speed behaviour: `G11` carries no speed in this port's serializer, so `deretraction_speed` only affects `Gcode`-mode unretracts (known limitation, not a divergence row).
- The suspected missing `mm/s -> mm/min` conversion on the `TravelRetract -> GCodeCommand` path (path-opt-originated retracts may emit `F30` where synthesis emits `F2400`): Step 1 confirms or refutes; if confirmed it files a follow-up ticket and is not fixed here.
- `ORCA_CONFIG_PADDING` edits of any kind (Authoring rule 2); new keys reach the CONFIG_BLOCK only via `to_config_map` as a side effect of being live.
- P37 wipe/travel/cut keys; the `retraction_speed` / `retraction_length` normal-path keys (ticket-101 rename set, already live in path-opt).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (rule-4 modular-pipeline constraint only).
- `docs/03_wit_and_manifest.md` - manifest `[config.schema]` key contract section only (the single module-manifest declaration).
- `docs/DEVIATION_LOG.md` - direct read of the last 20 lines only (DEV-171 format + number confirmation; highest at authoring is DEV-170).
- `docs/ORCA_CONFIG_REFERENCE.md` - P36 rows as the key list (never the status column).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - pin the exact `retract_lift_enforce` enum key strings and the `deretraction_speed` / lift-bound / restart-extra defaults this packet adopts.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - pin `lazy_lift` / `eager_lift` bound-comparison shape (strict vs inclusive, zero-as-sentinel) that Step 3 conforms to or records as a divergence.
- `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` - pin the `deretraction_speed <= 0` fallback and the restart-extra selection the emitter mirrors.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - pin the `long_retractions_when_cut` placeholder publication this packet mirrors in `machine-gcode-emit`.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`; each asserts a behaviour change at a non-default value plus the default-path identity arm (map Authoring rules gate (b)).
- Negative: `AC-N1` (unknown `retract_lift_enforce` value is fatal).
- Cross-packet impact: P37 (ticket 44) gains `retract_before_wipe` (ticket-closure edit, not this packet's code); packet 267's envelope/estimator work is untouched; the ticket-125 vector model is unblocked, not pre-empted.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test gcode_toolchange_wrapping -- toolchange_retract_uses_toolchange_length 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 toolchange length + per-tool override | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- restart_extra_adds_to_unretract 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2 normal restart extra + default identity | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_toolchange_wrapping -- restart_extra_toolchange_only_after_toolchange 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-3 toolchange restart extra selectivity | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- deretraction_speed_overrides_unretract 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-4 deretraction speed + zero fallback | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_above_suppresses 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-5 lift upper bound | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_below_suppresses 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-6 lift lower bound | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_enforce_selects_layers 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-7 enforce layer selection | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- long_retraction_placeholder 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-8 placeholder render with canonical `1`/`0` spelling | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test retraction_keys_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-8 manifest declaration (new guard binary authored by Step 4) | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- lift_enforce_rejects_unknown 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-N1 enum rejection | FACT pass/fail |
| `cargo check --workspace --all-targets` | struct-literal blast radius of new `ResolvedConfig` fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after the one manifest edit | FACT exit code (0 fresh / 1 stale / 3 infra) |

## Step Completion Expectations

- Step 1's canonical pin-downs (enum strings, bound comparison, placeholder name) are binding inputs to Steps 3-4; a Step 1 finding that contradicts a design assumption redesigns that step before code, never after.
- The `to_config_map` inserts and the `tool_config:` override path land in the same step as the field declarations (Step 2); no later step re-opens field plumbing.
- `retract_before_wipe` and `long_retractions_when_ec` are never declared by any step; a step that cannot wire a kept key without one of them returns the key to the queue with a tier-table annotation instead of a gap record.

## Context Discipline Notes

- Packet-specific hazard: `crates/slicer-gcode/src/emit.rs` is large — Steps read only the cited sites (entity retract/unretract loops, toolchange synthesis, ZHop arms); never load the whole file.
- `OrcaSlicerDocumented/` is delegate-only; the implementer never loads it.
- Heavy dispatches (canonical grounding, struct-literal survey) carry the bounded return formats in `design.md`; reject oversized returns and redispatch narrowly.
