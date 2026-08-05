---
status: implemented
packet: 167-config-block-viewer-keys
task_ids:
  - TASK-273
---

# 167-config-block-viewer-keys

## Goal

Purge every speed/acceleration/jerk-valued key from `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs` (replacing them with neutral cosmetic keys so OrcaSlicer's ~80-key CONFIG_BLOCK minimum gate still passes), synthesize a safe non-Bambu `printer_model` when the fork's raw_config omits it, and document the fork-facing required-key contract in `docs/02_ir_schemas.md`.

## Problem Statement

OrcaSlicer's viewer trusts CONFIG_BLOCK values: `ConfigBase::load_from_gcode_file` rejects blocks under ~80 key=value pairs, and `GCodeProcessor::apply_config` feeds machine limits and speed/accel/jerk settings into the time estimator and config panel. `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs:402-475`, 72 entries) pads the block past that gate with fabricated values — and 34 of those entries are speed/acceleration/jerk-valued print-profile keys (`travel_speed`, `default_acceleration`, `travel_jerk`, `sparse_infill_speed`, …) that actively mislead the viewer whenever the fork does not override them. **Grounding correction to the wave-1 plan**: the table contains no literal `machine_max_*` keys today; the misleading class is the print-profile speed/accel/jerk family, and the packet must additionally guarantee `machine_max_*` keys are never introduced as padding. Separately, when `printer_model` is absent, OrcaSlicer's `s_IsBBLPrinter` heuristic can default to Bambu-printer behavior on drag-in; PNP emits no `printer_model` anywhere today (grounded: zero occurrences in `crates/slicer-gcode`). The fork supplies real values via the already-verbatim raw_config passthrough (`serialize_config_block`, serialize.rs:283-382, dedup via `emit_config_kv`'s `BTreeSet` at serialize.rs:386-395; padding loop gated by `emitted.len() >= 96` at serialize.rs:373-379); this packet fixes the PNP-side defaults and documents the contract.

## Architecture Constraints

- CONFIG_BLOCK is part of the normative G-code envelope contract (`docs/02_ir_schemas.md`, "G-code envelope blocks (Normative — packet 55)"): `CONFIG_BLOCK_*` stays the final semicolon-prefixed content; block ordering must not change.
- Golden-output hazard: `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` contains a CONFIG_BLOCK; changing padding changes its bytes. The golden must be re-blessed in this packet with the diff reviewed (only CONFIG_BLOCK lines may differ — motion lines byte-identical). Any motion-line diff falsifies the packet.
- No guest-WASM impact: `crates/slicer-gcode` is not in CLAUDE.md's guest-input path list; no `cargo xtask build-guests --check` obligation beyond normal hygiene.

## Data and Contract Notes

- IR/manifest contracts: none. CONFIG_BLOCK is a wire-format (G-code text) contract documented in `docs/02_ir_schemas.md`.
- WIT boundary: none.
- Determinism: key emission stays sorted/deterministic (`BTreeSet` + sorted raw keys + fixed table order). The `emitted.len() >= 96` stop condition is retained unchanged.

## Locked Assumptions and Invariants

- `emit_config_kv`'s insert-or-skip dedup is the single shadowing guard; the printer_model synthesis must run through it (and, like the filament synthesis, is additionally guarded by `raw_config.contains_key`).
- Grounded 2026-07-17: `ORCA_CONFIG_PADDING` has 72 entries at serialize.rs:402-475; the padding loop gate is `emitted.len() >= 96` at serialize.rs:374; `printer_model` occurs nowhere in `crates/slicer-gcode`. Re-verify before editing if the file has moved.
- The wave-1 plan's claim that padding emits `machine_max_*` keys was falsified; the packet's contract is strengthened to "never emit them" rather than "remove them".

## Risks and Tradeoffs

- Risk: a "neutral" replacement key is actually consumed by the viewer's processor. Mitigation: every candidate is checked against `docs/ORCA_CONFIG_REFERENCE.md` via dispatch and excluded if speed/accel/jerk/machine-limit-typed; AC-1's grep enforces the name classes mechanically.
- Risk: golden `.gcode` fixtures churn. Mitigation: Step 4 inventories and re-blesses with a motion-lines-identical check.
- Tradeoff: `Generic PNP Printer` appears in the viewer's config panel for non-fork CLI users; accepted as strictly better than an absent key triggering Bambu-mode heuristics.
