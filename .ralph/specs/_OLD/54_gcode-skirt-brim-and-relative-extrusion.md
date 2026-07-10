---
status: implemented
packet: 54_gcode-skirt-brim-and-relative-extrusion
task_ids:
  - TASK-142a
  - TASK-183

---

# 54_gcode-skirt-brim-and-relative-extrusion

## Goal

Two independent, narrowly-scoped tracks delivered in one packet:

- **Track A — Skirt/Brim emission gating (TASK-142a).** The live `SkirtBrim` finalization module (`modules/core-modules/skirt-brim/src/lib.rs`, dispatched at `crates/slicer-host/src/dispatch.rs:2854`) was ported in the closed TASK-142, yet Benchy live output contains zero `;TYPE:Skirt` / `;TYPE:Brim` blocks. Diagnose the gap (Step 1) and close it so a config with `skirt_brim_enabled = true` and `skirt_loops > 0` produces at least one Skirt block before the first model extrusion.
- **Track B — Relative-extrusion toggle (TASK-183).** Add `use_relative_e_distances: ConfigValue::Bool` (default `true`) to `crates/slicer-host/src/config_schema.rs`; read it from `config_source` at `PipelineConfig` construction in `crates/slicer-host/src/main.rs` and forward to a new `DefaultGCodeSerializer::with_extrusion_mode(...)` constructor in `crates/slicer-host/src/gcode_emit.rs`; emit `M83` + per-move E deltas when true (default) or `M82` + absolute E values when false. `GCodeIR` E values remain absolute in both modes; only the serialized text differs. X/Y/Z/F/S/T tokens are byte-identical between modes.

The two tracks share no source files: Track A edits skirt-brim module + possibly dispatch wiring; Track B edits `gcode_emit.rs`, `main.rs`, `config_schema.rs`. Each track has its own TDD test file. They are bundled in one packet at the user's explicit request because both are small.

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

## Architecture Constraints

- `GCodeIR` E values remain absolute. `docs/02_ir_schemas.md` contract unchanged.
- Serializer is the ONLY place that converts to text. Both modes produce identical X/Y/Z/F/S/T tokens; the only differences are: (a) one preamble directive (`M82` vs `M83`) and (b) the formatted `E` value (delta vs absolute) on `G1`/`G0` and `Retract`/`Unretract` lines.
- The serializer keeps an internal `f64 e_accumulator`. On `G92 E0` the accumulator resets. On every `Move`/`Retract`/`Unretract` carrying an `E`, the emitted delta is `move.e - e_accumulator`; then `e_accumulator = move.e`.
- The flag flows: `config_source.get("use_relative_e_distances")` (read at `PipelineConfig` construction in `main.rs`) → `bool` → `DefaultGCodeSerializer::with_extrusion_mode(bool)` → stored as a field on the serializer. The flag NEVER touches the IR.

## Data and Contract Notes

- IR contracts touched: NONE. The `Move`/`Retract`/`Unretract` IR types remain absolute. `GCodeIR` preamble representation is unchanged.
- WIT boundary: NONE for Track B. Track A *may* touch the manifest TOML (`skirt-brim.toml`) if the diagnosis is "config key not declared".
- Determinism: the relative-mode accumulator is per-serializer-instance and deterministic.
- The accumulator starts at `0.0`. A `G92 E0` directive resets it to `0.0`. Any other `G92 E<value>` resets it to `<value>`.

## Locked Assumptions and Invariants

- Track A's fix size is bounded: ONE source file + the new test file. Anything larger is an escalation, not a silent scope creep.
- Track B never modifies the IR. Both modes start from the same `GCodeIR` instance.
- X/Y/Z/F/S/T tokens are formatted by the same code path in both modes. Only the E formatting branches.
- `M83` (relative) emit happens exactly once in the preamble; the serializer never re-emits it. `M82` (absolute) similarly emit-once.

## Risks and Tradeoffs

- Risk: Track A diagnosis returns ambiguous result. Mitigated by the explicit "ESCALATE" escape hatch — the implementer surfaces a hand-off rather than guessing.
- Risk: Per-move E delta rounding differs from OrcaSlicer (floor vs round). Mitigated by the FACT dispatch on `GCodeWriter.cpp` extracting the rounding rule (likely `format!("{:.5}", delta)`).
- Risk: `G92 E0` may appear mid-line (in a multi-command line) and the parser might miss it. Mitigated by treating `G92 E0` ONLY as standalone `GCodeCommand::SetExtruderPosition`-equivalent variants in `GCodeIR`; the serializer hooks the reset at the IR-variant level, not via text-matching.
- Tradeoff: bundling two unrelated tracks in one packet. Accepted at the user's explicit instruction; mitigated by independent step-tracks, independent test files, and the Track A split hatch.
