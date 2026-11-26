# Requirements: 277-retraction-wipe-travel-firmware-emitter

## Packet Metadata

- Grouped task IDs: none - queue packet; implementation is recorded against [44 - Author packet P37 - Extruder / Nozzle / Retraction (2/2) - emitter](../specs/orca-feature-gap/issues/44-author-packet-p37-extruder-nozzle-retraction-emitter.md).
- Backlog source: `docs/specs/orca-feature-gap/issues/44-author-packet-p37-extruder-nozzle-retraction-emitter.md` (P37 in the wayfinder map "Close the OrcaSlicer FFF feature gap").
- Packet number: 277, allocated from disk at authoring time per [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md) (highest committed packet dir is 276).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P37's eleven retraction keys are Tier B new logic owned by the host emitter, and all eleven are zero-occurrence as configuration-driven behaviour in `crates/`, `modules/`, and `xtask/` — the "Tier B new logic" sizing survived claim-time re-derivation, so no packet shed and no queue return. The owner needs the 276 narrowing: nine keys decide inside `DefaultGCodeEmitter::emit_gcode` (travel gate, layer-change gate, wipe emission + split, firmware mode, hop style + slope, Z offset), while `retraction_distances_when_cut` / `_ec` publish through the `machine-gcode-emit` placeholder seam that already mirrors canonical's `update_placeholder_parser_with_variant_params` (the 276 `_cut`-bool precedent, here with float distances). The wipe trio is the load-bearing build: this tree emits no wipe move at all, so `retract_before_wipe` / `wipe` / `wipe_distance` land together as one new emission site, not three declarations. Canonical spells nine of the eleven keys per-filament; this packet declares scalar-globals per the ticket-04 precedent and records DEV-172, leaving the vector model to the ticket-125 ruling.

## In Scope

- `retraction_minimum_travel` (float mm, canonical default `2.0`): `needs_retraction`-style gate in `emit_gcode` — skip the retract/unretract pair when the travel XY length is below the minimum.
- `retract_when_changing_layer` (bool, default `false`): layer-change retract gate — emit a `Retract` before the Z move to the next layer only when true.
- `wipe` (bool, default `false`) + `wipe_distance` (float mm, default `1.0`): new wipe `Move` emission after the retract and before the travel, with XY extent `wipe_distance`.
- `retract_before_wipe` (percent 0–100, canonical `coPercents` default `100.0`): split factor — pre-wipe `Retract.length = total * value / 100`, remainder as post-wipe `Retract`; default `100.0` retracts fully before the wipe.
- `use_firmware_retraction` (bool, default `false`): select `slicer_ir::RetractMode::Firmware` (serializes to `G10`/`G11`) vs `RetractMode::Gcode` on every emitted retract/unretract.
- `z_hop_types` (string-parsed enum `auto`/`normal`/`slope`/`spiral`, default `"slope"` = canonical `zhtSlope`, snake-case normalization of canonical `"Auto Lift"` etc.) + `travel_slope` (float degrees, default `3.0`): hop-style selection on the ZHop arm; `slope` emits a diagonal lead-in with XY extent `hop_height / tan(travel_slope)`; unknown values fail the slice (AC-N1).
- `z_offset` (float mm, default `0.0`): additive shift on every emitted Z target (preamble initial Z + layer-change Z).
- `retraction_distances_when_cut` (float mm, default `18.0`) + `retraction_distances_when_ec` (float mm, default `10.0`): eleventh/twelfth `ResolvedConfig` fields in scalar-global form plus `machine-gcode-emit.toml` declarations, published through the module's existing placeholder substitution with float spelling (the 276 bool-`1`/`0` precedent generalized to distances; canonical's EC-null state is not represented — unset means default).
- DEV-172 (scalar-global vs canonical per-filament vectors for the nine vector keys), generated config reference + `host-keys.toml` entries, guest rebuild for the one manifest touched.

## Out of Scope

- Per-filament vectors for any kept key — stays with the [125 - Rule on the port's per-tool config model](../specs/orca-feature-gap/issues/125-rule-per-tool-config-model.md) ruling; DEV-172 records the scalar choice. No `tool_config:` override is honoured by these eleven keys (the 276 `retract_length_toolchange` override is not generalized here).
- Canonical min/max ranges as validation (ticket-113 ruling: GUI hints, not pipeline rules) — defaults are adopted, bounds are not enforced beyond non-negativity where the type requires it; the `z_hop_types` strict-parse rejection (AC-N1) is the only new fatal.
- `ORCA_CONFIG_PADDING` edits of any kind (Authoring rule 2); new keys reach the CONFIG_BLOCK only via `to_config_map` as a side effect of being live.
- Spiral-hop geometry beyond style selection (canonical `travel_to_xyz` spiral path is a lift-shape variant of the same arm; this packet selects the style and emits the slope-correct diagonal, it does not port a helical path generator).
- The suspected missing `mm/s -> mm/min` conversion on the `TravelRetract -> GCodeCommand` path (ticket-43 incidental finding): Step 1 confirms or refutes; if confirmed it files a follow-up ticket and is not fixed here.
- P38+ queue keys; the `retraction_speed` / `retraction_length` normal-path keys (ticket-101 rename set, already live).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (rule-4 modular-pipeline constraint only; these are in-module emitter branches, not cross-module algorithm selection, so no claim holders).
- `docs/03_wit_and_manifest.md` - manifest `[config.schema]` key contract section only (the two module-manifest declarations).
- `docs/DEVIATION_LOG.md` - direct read of the last 20 lines only (DEV-172 format + number confirmation; highest committed at authoring is DEV-170 with DEV-171 planned by unmerged packet 276 — re-derive at implementation time).
- `docs/ORCA_CONFIG_REFERENCE.md` - P37 rows as the key list (never the status column).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - pin the eleven P37 defaults and types plus the `z_hop_types` enum strings this packet normalizes to snake-case.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - pin `needs_retraction` minimum-travel gate, `retract` wipe gate, `change_layer` retract + `z_offset` adjustment, `Wipe::calculateWipeRetractionLengths` split arithmetic, and the `_cut`/`_ec` placeholder publication this packet mirrors.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - pin `_retract`/`unretract` firmware G10/G11 shape, `retract` before-wipe split call, and `travel_to_xyz` slope-lift math that Step 5 conforms to or records as a divergence.
- `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` - pin `retract_before_wipe` percent-to-factor clamp and `travel_slope` degrees-to-radians conversion the emitter mirrors.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`; each asserts a behaviour change at a non-default value plus the default-path identity arm (map Authoring rules gate (b)). AC-4 covers `retract_before_wipe` with the wipe fixture it partitions; AC-6 covers both `z_hop_types` and `travel_slope` as one style decision; AC-8/AC-9 cover the two placeholder keys.
- Negative: `AC-N1` (`z_hop_types` unknown value is fatal).
- Cross-packet impact: packet 276's `retract_before_wipe` handoff lands here (no 276 edit needed); the ticket-125 vector model is unblocked, not pre-empted; packet 267's envelope work is untouched.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- minimum_travel_gates_retract 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 travel gate + default identity | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- retract_on_layer_change 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2 layer-change gate | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- wipe_emits_move 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-3 wipe emission | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- before_wipe_splits_retract 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-4 split arithmetic | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- firmware_mode_selects_g10_g11 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-5 firmware mode | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- hop_type_selects_shape 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-6 hop style + slope math | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- z_offset_shifts_targets 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-7 Z offset | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- cut_distance_placeholder 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-8 cut placeholder | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- ec_distance_placeholder 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-9 EC placeholder | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd -- hop_type_rejects_unknown 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-N1 enum rejection | FACT pass/fail |
| `cargo test -p slicer-ir --test resolved_config_defaults_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Step 3 eleven-key declaration case (existing binary, never a new binary for schema alone) | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test retraction_keys_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | manifest declaration guard (shared binary 276 authors; Step 6 extends with the two distance cases, queue-order merge churn) | FACT pass/fail |
| `cargo check --workspace --all-targets` | struct-literal blast radius of new `ResolvedConfig` fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after the one manifest edit | FACT exit code (0 fresh / 1 stale / 3 infra) |

## Step Completion Expectations

- Step 1's canonical pin-downs (hop enum strings, percent clamp, slope math, placeholder names, G10/G11 spelling, Z-offset sites) are binding inputs to Steps 3-6; a Step 1 finding that contradicts a design assumption redesigns that step before code, never after.
- The `to_config_map` inserts land in the same step as the field declarations (Step 2); no later step re-opens field plumbing.
- A step that cannot wire a kept key to a behaviour-changing decision point returns the key to the queue with a tier-table annotation instead of a gap record (Authoring rule 1); no declaration-only key survives.
- The next free `DEV-###` is re-derived at implementation time (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`); this packet's DEV-172 is the authoring-time expectation, not a frozen ledger fact.

## Context Discipline Notes

- Packet-specific hazard: `crates/slicer-gcode/src/emit.rs` is large — Steps read only the cited sites (entity retract/unretract loops, layer-change path, ZHop arms, preamble Z); never load the whole file.
- `OrcaSlicerDocumented/` is delegate-only; the implementer never loads it.
- Heavy dispatches (canonical grounding, struct-literal survey) carry the bounded return formats in `design.md`; reject oversized returns and redispatch narrowly.
