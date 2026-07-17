# Requirements: 167-config-block-viewer-keys

## Packet Metadata

- Grouped task IDs: `TASK-273`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

OrcaSlicer's viewer trusts CONFIG_BLOCK values: `ConfigBase::load_from_gcode_file` rejects blocks under ~80 key=value pairs, and `GCodeProcessor::apply_config` feeds machine limits and speed/accel/jerk settings into the time estimator and config panel. `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs:402-475`, 72 entries) pads the block past that gate with fabricated values — and 34 of those entries are speed/acceleration/jerk-valued print-profile keys (`travel_speed`, `default_acceleration`, `travel_jerk`, `sparse_infill_speed`, …) that actively mislead the viewer whenever the fork does not override them. **Grounding correction to the wave-1 plan**: the table contains no literal `machine_max_*` keys today; the misleading class is the print-profile speed/accel/jerk family, and the packet must additionally guarantee `machine_max_*` keys are never introduced as padding. Separately, when `printer_model` is absent, OrcaSlicer's `s_IsBBLPrinter` heuristic can default to Bambu-printer behavior on drag-in; PNP emits no `printer_model` anywhere today (grounded: zero occurrences in `crates/slicer-gcode`). The fork supplies real values via the already-verbatim raw_config passthrough (`serialize_config_block`, serialize.rs:283-382, dedup via `emit_config_kv`'s `BTreeSet` at serialize.rs:386-395; padding loop gated by `emitted.len() >= 96` at serialize.rs:373-379); this packet fixes the PNP-side defaults and documents the contract.

## In Scope

- Remove every speed/acceleration/jerk-valued entry from `ORCA_CONFIG_PADDING` (the 34-entry removal list is enumerated in `design.md`).
- Add enough neutral, preview-cosmetic replacement keys (patterns, toggles, counts — nothing the viewer feeds into motion/time computation) to keep the padded CONFIG_BLOCK at ≥80 `; key = value` lines with a minimal raw_config; keep the `emitted.len() >= 96` stop condition.
- Guarantee no `machine_max_*` key ever appears in the padding table (AC-1 grep class).
- Synthesize `; printer_model = Generic PNP Printer` when `raw_config` lacks `printer_model`, via the existing `emit_config_kv` dedup path so a fork-supplied value always wins.
- Add integration tests: minimum-key-gate count (AC-2), printer_model synthesis (AC-3), fork-key no-shadowing (AC-N1).
- Document the fork-facing required-key contract (`printer_model`, `filament_density`, `filament_cost`, `printable_area`, `nozzle_diameter`, `machine_max_*` family) as a new "CONFIG_BLOCK viewer-key contract" subsection in `docs/02_ir_schemas.md` under "G-code envelope blocks".
- Add the completed `TASK-273` crosswalk to `docs/07_implementation_status.md`.

## Out of Scope

- Any change to the raw_config passthrough serialization, value formatting, key ordering, or the `emit_config_kv` dedup mechanism.
- Emitting real machine limits from PNP config (that is the fork's job per the contract; PNP-side estimator work is packet 169).
- HEADER_BLOCK, THUMBNAIL_BLOCK, and machine start/end G-code.
- OrcaSlicer-side (fork frontend) changes.
- `docs/15_config_keys_reference.md` regeneration (padding keys are not PNP config keys).

## Authoritative Docs

- `docs/02_ir_schemas.md` — 1811 lines; direct read of the "G-code envelope blocks" section (~lines 1660-1720) only; this packet appends there.
- `docs/ORCA_CONFIG_REFERENCE.md` — 2404 lines; delegate LOCATIONS lookups for neutral replacement key candidates and their upstream defaults.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (padding purged of speed/accel/jerk/machine_max classes), `AC-2` (≥80-line gate still passes), `AC-3` (non-BBL printer_model synthesis), `AC-4` (doc contract present).
- Negative: `AC-N1` (fork-supplied machine_max_* and printer_model never shadowed/duplicated), `AC-N2` (pre-existing duplicate-key and block-ordering invariants hold).
- Cross-packet impact: packet 169-time-estimator-slice-stats depends on this packet's contract for fork-realistic machine-limit fixtures; the doc subsection added here is what 169's fixtures cite.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `awk '/^const ORCA_CONFIG_PADDING/,/^\];/' crates/slicer-gcode/src/serialize.rs | grep -E '"(machine_max_[a-z_]*|[a-z_]*speed[a-z_]*|[a-z_]*acceleration[a-z_]*|[a-z_]*jerk[a-z_]*)"' ; test $? -eq 1 && echo PASS || echo FAIL` | AC-1: misleading classes absent from padding | FACT PASS/FAIL |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_meets_orca_minimum_key_gate 2>&1 | tee target/test-output.log | grep "^test result"` | AC-2: ≥80-line minimum gate | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_synthesizes_non_bbl_printer_model 2>&1 | tee target/test-output.log | grep "^test result"` | AC-3: printer_model synthesis | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_fork_keys_never_shadowed 2>&1 | tee target/test-output.log | grep "^test result"` | AC-N1: fork keys win | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_header 2>&1 | tee target/test-output.log | grep "^test result"` | AC-N2: pre-existing block invariants | FACT pass/fail |
| `grep -q "CONFIG_BLOCK viewer-key contract" docs/02_ir_schemas.md && grep -q "machine_max_" docs/02_ir_schemas.md && echo PASS || echo FAIL` | AC-4: doc contract grep | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | commit gate | FACT pass/fail |

## Step Completion Expectations

The padding-table rework (Step 2) and printer_model synthesis (Step 3) both edit `serialize_config_block`'s emission path; Step 3 must run after Step 2 so AC-2's key count is measured against the final table. No other cross-step state.

## Context Discipline Notes

`crates/slicer-gcode/src/serialize.rs` is 807 lines — open only the grounded windows (`serialize_config_block` 283-400, padding table 402-475). `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` is 765 lines — read only its helper functions (`region_between`, slicing harness) and append tests; do not read all existing tests. `docs/ORCA_CONFIG_REFERENCE.md` is delegation-only.
