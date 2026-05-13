---
status: draft
packet: 54_gcode-skirt-brim-and-relative-extrusion
task_ids:
  - TASK-142a   # Track A — diagnose & fix why TASK-142's live SkirtBrim port produces zero ;TYPE:Skirt
  - TASK-155    # Track B — relative-extrusion mode toggle (M82/M83) via use_relative_e_distances
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 54_gcode-skirt-brim-and-relative-extrusion

## Goal

Two independent, narrowly-scoped tracks delivered in one packet:

- **Track A — Skirt/Brim emission gating (TASK-142a).** The live `SkirtBrim` finalization module (`modules/core-modules/skirt-brim/src/lib.rs`, dispatched at `crates/slicer-host/src/dispatch.rs:2854`) was ported in the closed TASK-142, yet Benchy live output contains zero `;TYPE:Skirt` / `;TYPE:Brim` blocks. Diagnose the gap (Step 1) and close it so a config with `skirt_brim_enabled = true` and `skirt_loops > 0` produces at least one Skirt block before the first model extrusion.
- **Track B — Relative-extrusion toggle (TASK-155).** Add `use_relative_e_distances: ConfigValue::Bool` (default `true`) to `crates/slicer-host/src/config_schema.rs`; thread it through `crates/slicer-host/src/pipeline.rs::run_pipeline_with_raw_config` to a new `DefaultGCodeSerializer::with_extrusion_mode(...)` constructor in `crates/slicer-host/src/gcode_emit.rs`; emit `M83` + per-move E deltas when true (default) or `M82` + absolute E values when false. `GCodeIR` E values remain absolute in both modes; only the serialized text differs. X/Y/Z/F/S/T tokens are byte-identical between modes.

The two tracks share no source files: Track A edits skirt-brim module + possibly dispatch wiring; Track B edits `gcode_emit.rs`, `pipeline.rs`, `config_schema.rs`. Each track has its own TDD test file. They are bundled in one packet at the user's explicit request because both are small.

## Scope Boundaries

- In scope (Track A):
  - Discovery step that produces a FACT explaining why TASK-142's live geometry produces zero emit output today (likely candidates: config key not propagated; default `skirt_brim_enabled = false`; module loaded but its output channel mis-wired; entities produced but `ExtrusionRole::Skirt` not flowing through to the emit-side label match).
  - Minimal fix matching the diagnosed cause — exactly one of: config-default flip, dispatch wiring, or role tagging.
  - TDD test `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs` covering positive + negative cases.
- In scope (Track B):
  - Register `use_relative_e_distances` key with default `true`.
  - `DefaultGCodeSerializer::with_extrusion_mode(mode)` constructor; `DefaultGCodeSerializer::new()` becomes a shim that calls `with_extrusion_mode(true)`.
  - Emit `M83` or `M82` in the preamble; running absolute accumulator that resets on `G92 E0`; per-move E deltas in relative mode.
  - Thread the flag from `run_pipeline_with_raw_config`'s `raw_config_source` into the serializer.
  - TDD test `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` covering positive + four negative cases.
- Out of scope:
  - Skirt/brim geometry changes (already delivered in TASK-142).
  - Raft generation (separate concern; not requested in the user's task 3).
  - Multi-extruder / per-tool E accumulators in relative mode (single accumulator is assumed sufficient; multi-tool E semantics defer to a follow-up).
  - Changes to `GCodeIR` E semantics — they remain absolute; the IR contract in `docs/02_ir_schemas.md` is unchanged.
  - Cooling (packet 53) and feedrate (packet 52).

## Prerequisites and Blockers

- Depends on:
  - None for Track B.
  - Track A depends on the discovery FACT in Step 1; the actual fix branches off that FACT, so the implementer must commit to one of the diagnosed-cause branches before Step 2.
- Unblocks:
  - DEV-009 subsets covering skirt/brim and relative-E.
- Activation blockers:
  - None at draft time. Discovery FACT is a pre-step inside the packet, not an activation gate.

## Acceptance Criteria

### Track A — Skirt/Brim

- **Given** a `ConfigView` with `skirt_brim_enabled = true` and `skirt_loops = 1`, **when** the live emit path runs, **then** at least one `;TYPE:Skirt` block exists in the produced G-code before the first `;TYPE:Outer wall` or `;TYPE:Inner wall` block. | `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd -- skirt_block_before_model --nocapture`
- **Given** a `ConfigView` with `skirt_brim_enabled = false`, **when** the live emit path runs, **then** zero `;TYPE:Skirt` AND zero `;TYPE:Brim` blocks appear. | `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd -- skirt_disabled_emits_nothing --nocapture`
- **Given** `skirt_brim_enabled = true` and `skirt_loops = 3`, **when** the live emit path runs, **then** at least 3 distinct closed loops with role `Skirt` are produced (counted via `;TYPE:Skirt` followed by extrusion lines and a return-to-start within positional tolerance). | `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd -- skirt_loops_count_honored --nocapture`
- **Given** a `ConfigView` with `brim_width > 0.0`, **when** the live emit path runs, **then** at least one `;TYPE:Brim` block exists in the produced G-code before the first model extrusion. | `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd -- brim_block_before_model --nocapture`

### Track B — Relative-Extrusion Toggle

- **Given** a `DefaultGCodeSerializer` constructed via `DefaultGCodeSerializer::new()` or via `run_pipeline_with_raw_config` with `use_relative_e_distances` unset, **when** any non-empty `GCodeIR` is serialized, **then** `M83` is present exactly once in the preamble and `M82` does not appear anywhere in the output. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- default_is_relative_m83 --nocapture`
- **Given** `raw_config_source` contains `("use_relative_e_distances", ConfigValue::Bool(false))`, **when** the same `GCodeIR` is serialized through the pipeline, **then** `M82` is present exactly once in the preamble, `M83` does not appear, and every `G1` `E` value matches the original absolute IR value byte-for-byte. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- absolute_mode_when_flag_false --nocapture`
- **Given** relative mode active and any serialized `G1` move carrying an `E` field, **when** the emitted `E` value is read, **then** it is a per-move delta (magnitude < 5 mm in typical cases). | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- e_values_are_per_move_deltas --nocapture`
- **Given** the same `GCodeIR` serialized once with `use_relative_e_distances = true` and once with `use_relative_e_distances = false`, **when** all `G0`/`G1` lines are compared field-by-field, **then** `X`, `Y`, `Z`, and `F` token text matches exactly on every corresponding line. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- xyzf_unchanged_across_modes --nocapture`
- **Given** any extrusion segment between two `G92 E0` resets in the absolute-mode output, **when** the corresponding per-move E deltas in relative-mode output are summed, **then** the total equals the original final absolute E within 1e-3 mm. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- delta_sum_matches_absolute_per_g92_block --nocapture`
- **Given** `crates/slicer-host/src/config_schema.rs` queried for `use_relative_e_distances`, **when** the registration is inspected, **then** it is registered as `ConfigValue::Bool` with default `true` and is rejected with `ConfigValidationError` if supplied a non-bool. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- config_schema_registers_bool_default_true --nocapture`

## Negative Test Cases

### Track A

- **Given** a config requesting a skirt (`skirt_brim_enabled = true`, `skirt_loops > 0`) but a produced file emitting zero `;TYPE:Skirt|;TYPE:Brim`, **when** validated, **then** the test fails. | `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd -- rejects_no_skirt_when_required --nocapture`

### Track B

- **Given** a serializer in relative mode that still emits `M82` anywhere in the output, **when** validated, **then** it fails. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- rejects_m82_in_relative_mode --nocapture`
- **Given** a serializer in absolute mode that emits `M83` or writes per-move E deltas, **when** validated, **then** it fails. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- rejects_m83_or_deltas_in_absolute_mode --nocapture`
- **Given** a serializer in relative mode whose emitted `G1` `E` values keep growing monotonically over hundreds of moves, **when** validated, **then** it fails. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- rejects_monotonic_e_run --nocapture`
- **Given** any `G1` line whose `X`, `Y`, `Z`, or `F` token differs between the two modes for the same input IR, **when** validated, **then** it fails. | `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- rejects_xyzf_drift_across_modes --nocapture`

## Verification

- `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd`
- `cargo test -p slicer-host --test gcode_relative_extrusion_tdd`
- `cargo test -p slicer-host --test orca_comment_contract_tdd` (regression)
- `./modules/core-modules/build-core-modules.sh` if Track A modifies `skirt-brim/src/lib.rs`.
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage role; delegate a SUMMARY.
- `docs/02_ir_schemas.md` — `GCodeCommand::Move`, `GCodeIR` preamble; delegate a SUMMARY (especially the E-value contract — the packet preserves IR-absolute).
- `docs/03_wit_and_manifest.md` — only relevant if Track A's diagnosis points at manifest config-key propagation. Load directly the schema section in that case.
- `docs/05_module_sdk.md` — finalization-stage section; load directly the relevant ≤ 40 lines.
- `docs/07_implementation_status.md` — delegate; insert TASK-142a and TASK-155 rows; TASK-142 reference (do NOT reopen).
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — load directly; DEV-009 progress entries.

## OrcaSlicer Reference Obligations

All reads delegated.

- Track A: `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` and `OrcaSlicerDocumented/src/libslic3r/Print.cpp` (skirt loop integration) — SUMMARY ≤ 200 words confirming the role-tagging contract this packet must match.
- Track B: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — FACT, ≤ 8 lines, on M82/M83 emission and the per-move E accumulator pattern. `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — FACT, one line, default for `use_relative_e_distances`.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

- `OrcaSlicerDocumented/` MUST be delegated.
- The skirt-brim module file is small (per reconnaissance); load directly.
- `crates/slicer-host/src/gcode_emit.rs` is > 600 lines — range-read `:200-:480` only.
- `crates/slicer-host/src/pipeline.rs` is large — range-read around `run_pipeline_with_raw_config` (`:217 ±60`) only.
- Sub-agent return formats:
  - Track A diagnosis: SUMMARY ≤ 100 words IDENTIFYING the cause + the smallest fix; do NOT request full skirt-brim source dump.
  - Brim.cpp / Print.cpp SUMMARY: ≤ 200 words, no code.
  - GCodeWriter M82/M83 FACT: ≤ 8 lines.
  - cargo runs: FACT pass/fail; SNIPPETS on failure.

Aggregate context cost: M. No step is L. The bundled-tracks structure is justified at the user's request; if Track A diagnosis returns a fix significantly larger than expected (e.g. requires changes outside skirt-brim + dispatch), Track A will be SPLIT OUT into its own packet 54a before continuing — surfaced as a hand-off, not absorbed.
