# Requirements: 54_gcode-skirt-brim-and-relative-extrusion

## Packet Metadata

- Grouped task IDs:
  - `TASK-142a` (new — Track A — diagnose & close the live SkirtBrim emit gap left by TASK-142)
  - `TASK-183` (new — Track B — relative-extrusion toggle via `use_relative_e_distances`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Two independent gaps in the live G-code emit path, bundled in one packet at the user's explicit request:

**Track A.** TASK-142 (Closed 2026-04-25) ported `SkirtBrim` to `run_finalization()` using the `LayerCollectionView`. Reconnaissance confirms:
- The module exists at `modules/core-modules/skirt-brim/src/lib.rs` with `from_config` and `run_finalization`.
- It reads `skirt_brim_enabled`, `skirt_loops`, `skirt_distance`, `skirt_height`, `brim_width`, `line_width` from `ConfigView`.
- It is dispatched at `crates/slicer-host/src/dispatch.rs:2854` (`dispatch_finalization_call`).
- The emit-side `;TYPE:Skirt` label match exists at `crates/slicer-host/src/gcode_emit.rs:89` (`ExtrusionRole::Skirt` → `orca_type_label`).

Despite all of the above being in place, the user's Benchy emit output contains zero `;TYPE:Skirt|;TYPE:Brim` blocks. The diagnostic question — "where does the chain break?" — is the first deliverable. The fix is the second; its size and surface are bounded to the smallest correct change matching the diagnosis.

**Track B.** No `M82`/`M83` references exist anywhere in the workspace. `GCodeIR` E values are written absolute. The user wants a `use_relative_e_distances` toggle, default `true` (firmware-relative). When `true`, the serializer emits one `M83` in the preamble and writes per-move E deltas; when `false`, it emits one `M82` and writes the existing absolute values verbatim. X/Y/Z/F/S/T tokens are byte-identical between modes. `GCodeIR` E semantics in `docs/02_ir_schemas.md` are NOT changed; only the serialized text differs.

The two tracks share no source files. They are bundled because both are small and the user requested it; the implementation plan keeps them as independent step-tracks. If Track A's diagnosis surfaces a larger fix than budgeted, Track A is split off as packet 54a before continuing — not absorbed.

This packet is the third remediation against DEV-009.

## In Scope

Track A:
- Discovery step producing a SUMMARY ≤ 100 words naming the root cause + the smallest fix.
- Minimal fix implementing the diagnosed cause (config default, dispatch wiring, OR role tagging — exactly one).
- New test file `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs`.

Track B:
- `use_relative_e_distances` registered in `crates/slicer-host/src/config_schema.rs` (`ConfigValue::Bool`, default `true`).
- `DefaultGCodeSerializer::with_extrusion_mode(relative: bool)` added in `crates/slicer-host/src/gcode_emit.rs`. `DefaultGCodeSerializer::new()` becomes a shim that calls `with_extrusion_mode(true)`.
- Threading the flag at the `PipelineConfig` construction site in `crates/slicer-host/src/main.rs` (caller of `run_pipeline_with_raw_config`) into the serializer.
- New test file `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs`.
- DEV-009 progress entries; TASK-142a + TASK-183 rows in `docs/07_implementation_status.md`.

## Out of Scope

- Raft generation.
- Skirt/brim geometry changes (already delivered in TASK-142).
- Multi-extruder / per-tool E accumulators in relative mode.
- Changes to `GCodeCommand::Move`, `Retract`, `Unretract` IR types — the IR remains absolute.
- Cooling (packet 53) and feedrate (packet 52).
- Any other unrelated emit-path work.

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage; SUMMARY.
- `docs/02_ir_schemas.md` — `GCodeCommand::Move`, `GCodeIR` preamble; SUMMARY (E remains absolute in IR).
- `docs/03_wit_and_manifest.md` — relevant only if Track A's diagnosis points at manifest config-key propagation; load directly the schema section in that case.
- `docs/05_module_sdk.md` — finalization-stage section; load directly the relevant ≤ 40 lines.
- `docs/07_implementation_status.md` — delegate; insert TASK-142a + TASK-183 rows.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — load directly; DEV-009 progress entries.

## OrcaSlicer Reference Obligations

All reads delegated.

- Track A: `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`, `OrcaSlicerDocumented/src/libslic3r/Print.cpp` (skirt loop integration) — SUMMARY ≤ 200 words confirming role-tagging contract.
- Track B: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — FACT, ≤ 8 lines, on `M82`/`M83` and the running E accumulator pattern. `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — FACT, one line, default value of `use_relative_e_distances`.

## Acceptance Summary

Positive outcomes:

Track A:
- `skirt_brim_enabled = true`, `skirt_loops = 1` → ≥ 1 `;TYPE:Skirt` block before any model extrusion.
- `skirt_brim_enabled = false` → zero `;TYPE:Skirt`/`;TYPE:Brim`.
- `skirt_loops = 3` → ≥ 3 distinct closed skirt loops.
- `brim_width > 0.0` → ≥ 1 `;TYPE:Brim` block before model extrusion.

Track B:
- Default (`use_relative_e_distances` unset) → `M83` present exactly once; `M82` absent.
- Explicit `Bool(false)` → `M82` present exactly once; `M83` absent; every `G1` E byte-identical to absolute IR.
- Relative mode → E values are deltas (typically < 5 mm).
- Modes differ only in M82/M83 and E text; X/Y/Z/F identical.
- Sum of deltas between `G92 E0` resets equals the corresponding absolute final E within 1e-3 mm.
- `use_relative_e_distances` registered as `Bool` with default `true`; non-bool rejected with `ConfigValidationError`.

Negative outcomes:

- Track A: a file with `skirt_brim_enabled = true` but zero `;TYPE:Skirt|;TYPE:Brim` blocks fails.
- Track B: M82 in relative mode → fail. M83 or deltas in absolute mode → fail. Monotonic E run in relative mode → fail. X/Y/Z/F drift across modes → fail.

Measurable outcomes:

- Track A: new test file with ≥ 5 tests (4 ACs + 1 negative).
- Track B: new test file with ≥ 10 tests (6 ACs + 4 negative).
- `gcode_emit.rs`: `DefaultGCodeSerializer::with_extrusion_mode` exists; `new()` is a shim.
- `main.rs`: the `PipelineConfig` construction site reads `use_relative_e_distances` from `config_source` and forwards it to `DefaultGCodeSerializer::with_extrusion_mode(...)` before `run_pipeline_with_raw_config` is called.
- `config_schema.rs`: one new bool field registered.
- `skirt-brim/src/lib.rs` (or one other file) edited only at the location indicated by Step 1's diagnosis FACT.

Cross-packet impact:

- Closes the skirt/brim and relative-E subsets of DEV-009.
- Does not block any other packet.
- TASK-142 is NOT reopened; TASK-142a is a new follow-up that cites the predecessor.

## Verification Commands

- `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd` — Track A acceptance.
- `cargo test -p slicer-host --test gcode_relative_extrusion_tdd` — Track B acceptance.
- `./modules/core-modules/build-core-modules.sh` — only if Track A touches `skirt-brim/src/lib.rs`. Dispatch as FACT pass/fail.
- `cargo check --workspace` — fast type-check.
- `cargo clippy --workspace -- -D warnings`.

## Step Completion Expectations

See `implementation-plan.md` for per-step Precondition / Postcondition / Falsifying check / Files-allowed / Files-edit / Expected dispatches / Cost. No step is L.

## Context Discipline Notes

- `crates/slicer-host/src/gcode_emit.rs` is > 600 lines — range-read `:200-:480` only.
- `crates/slicer-host/src/main.rs` is large — range-read around the `PipelineConfig` construction site (~`:230-:280`) only.
- `crates/slicer-host/src/dispatch.rs` is > 2800 lines — only range-read if Track A's diagnosis says the dispatcher arm is the bug; default `:2840-:2900`.
- `modules/core-modules/skirt-brim/src/lib.rs` is small per reconnaissance — load directly.
- Likely temptation reads to skip: `.ralph/specs/16_skirt-brim-finalization-live-path/` (packet 16; closed; do NOT re-read — its conclusion is TASK-142); the full `docs/07_implementation_status.md`; OrcaSlicer source.
- Sub-agent return formats:
  - Track A diagnosis: SUMMARY ≤ 100 words IDENTIFYING ONE cause + ONE smallest fix. If two causes are needed, escalate — do not bundle them silently.
  - OrcaSlicer Brim/Print SUMMARY: ≤ 200 words.
  - OrcaSlicer GCodeWriter M82/M83 FACT: ≤ 8 lines.
  - cargo runs: FACT pass/fail; SNIPPETS on failure.
