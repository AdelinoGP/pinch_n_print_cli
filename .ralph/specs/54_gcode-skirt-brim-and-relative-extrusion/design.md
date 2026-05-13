# Design: 54_gcode-skirt-brim-and-relative-extrusion

## Controlling Code Paths

### Track A — Skirt/Brim emission gating

- `modules/core-modules/skirt-brim/src/lib.rs` — public entry `from_config` + `run_finalization`. Reads `skirt_brim_enabled` (default `true`), `skirt_loops` (default `1`), `skirt_distance` (default `6.0`), `skirt_height` (default `1`), `brim_width` (default `0.0`), `line_width` (default `0.4`). Output: `LayerCollectionIR` mutation via `push_entity_to_layer()` setting `ExtrusionRole::Skirt` on each entity.
- `crates/slicer-host/src/dispatch.rs:2854` — `dispatch_finalization_call` site that loads `modules/core-modules/skirt-brim/skirt-brim.wasm`.
- `crates/slicer-host/src/gcode_emit.rs:89` — `orca_type_label` matches `ExtrusionRole::Skirt` → emits `;TYPE:Skirt`.
- Diagnostic candidates (Step 1 will discriminate):
  1. `skirt_brim_enabled` config default not applied (e.g. ConfigView falls back to a host-side default of `false` because the manifest key isn't registered in `config_schema.rs`).
  2. Dispatcher does not actually call the module's `run_finalization` (silent skip).
  3. Module runs but produces zero entities because `skirt_loops` is read as `0` (config-resolution bug).
  4. Module produces entities, but they are not on a layer that the serializer iterates (off-by-one layer index).
  5. Entities flow but `ExtrusionRole::Skirt` is stripped/replaced before reaching `orca_type_label`.

### Track B — Relative-extrusion toggle

- `crates/slicer-host/src/gcode_emit.rs:378` — `DefaultGCodeSerializer` struct. Constructor `new()` at `:382`. Serializer logic at `:394-:480`. Key call sites:
  - `:399-:430` — `GCodeCommand::Move` match arm. Writes `G1` line with X/Y/Z/F/E.
  - `:424-:426` — F token write.
  - The E token write (immediately after the F token in the same match arm — exact line to be confirmed by Step 1 range-read; based on reconnaissance, `E` is written within the same `if let Some(e_val) = e {...}` block).
  - `Retract` / `Unretract` variants — also write `E`.
  - The preamble emit (where `M82`/`M83` directive should be added) — discovery needed; likely a `serialize_preamble` method or the start of `serialize_gcode`.
- `crates/slicer-host/src/pipeline.rs:217` — `run_pipeline_with_raw_config(_, raw_config_source: &HashMap<ConfigKey, ConfigValue>, _)`. Construct site for `DefaultGCodeSerializer`. After this packet, the construction uses `with_extrusion_mode(resolved_flag)`.
- `crates/slicer-host/src/config_schema.rs:104-176` — `ConfigValue::Bool` exists; eighth-of-`ConfigField` registration shape already used elsewhere; register the new key here.

## Architecture Constraints

- `GCodeIR` E values remain absolute. `docs/02_ir_schemas.md` contract unchanged.
- Serializer is the ONLY place that converts to text. Both modes produce identical X/Y/Z/F/S/T tokens; the only differences are: (a) one preamble directive (`M82` vs `M83`) and (b) the formatted `E` value (delta vs absolute) on `G1`/`G0` and `Retract`/`Unretract` lines.
- The serializer keeps an internal `f64 e_accumulator`. On `G92 E0` the accumulator resets. On every `Move`/`Retract`/`Unretract` carrying an `E`, the emitted delta is `move.e - e_accumulator`; then `e_accumulator = move.e`.
- The flag flows: `raw_config_source.get(&"use_relative_e_distances")` → `bool` → `DefaultGCodeSerializer::with_extrusion_mode(bool)` → stored as a field on the serializer. The flag NEVER touches the IR.

## Code Change Surface

### Track A

- Selected approach: **diagnose first, then make the smallest correct fix.** Pre-committing to a specific file is unsafe because reconnaissance has narrowed the cause to five candidates but cannot pick one without the diagnosis dispatch.
- Files expected to change:
  - **EXACTLY ONE** of: `modules/core-modules/skirt-brim/src/lib.rs`, `crates/slicer-host/src/dispatch.rs` (range `:2840-:2900`), `crates/slicer-host/src/config_schema.rs`, `modules/core-modules/skirt-brim/skirt-brim.toml`.
  - **PLUS** `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs` (new).
  - If the fix requires more than one file (escalation), Track A is SPLIT OUT to a new packet 54a and Track B is delivered alone in this packet. The implementer must surface the escalation as a hand-off rather than silently expanding scope.

### Track B

- Selected approach: **constructor + per-mode formatter branch + per-instance accumulator.**
- Files expected to change:
  - `crates/slicer-host/src/gcode_emit.rs` — add `with_extrusion_mode`, add `e_accumulator: f64` field, add `relative: bool` field, branch the E formatting in the `Move`/`Retract`/`Unretract` arms, emit `M82`/`M83` in the preamble.
  - `crates/slicer-host/src/pipeline.rs` (range `:200-:280`) — read `use_relative_e_distances` from `raw_config_source`, pass to constructor.
  - `crates/slicer-host/src/config_schema.rs` — register `use_relative_e_distances` as `ConfigValue::Bool` default `true`.
  - `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` (new).

- Rejected alternatives:
  - **Two separate `DefaultGCodeSerializer` types (`Relative` vs `Absolute`).** Rejected: doubles maintenance; the user's task description explicitly wants one constructor with a parameter.
  - **Convert IR to relative at the builder.** Rejected: violates the IR-stays-absolute invariant.
  - **Post-process the produced G-code text.** Rejected: fragile, breaks the byte-identical X/Y/Z/F invariant under whitespace edge cases.

## Files in Scope (read + edit)

Primary (≤ 3):

- `crates/slicer-host/src/gcode_emit.rs` — Track B primary. Range-read `:200-:480`. Edits: 4 small additions (preamble emit, field, constructor, branch in Move/Retract/Unretract arms).
- `crates/slicer-host/src/pipeline.rs` — Track B threading. Range-read `:200-:280`. Edit: one line in the serializer construction.
- `crates/slicer-host/src/config_schema.rs` — Track B + (possibly) Track A. Edits: register `use_relative_e_distances`; possibly register `skirt_brim_enabled` etc. depending on Track A diagnosis.

Auxiliary (small, mechanical):

- `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` — Track B test file (new).
- `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs` — Track A test file (new).
- Track A's "one fix file" — selected by Step 1 diagnosis.

## Read-Only Context

- `modules/core-modules/skirt-brim/src/lib.rs` — load directly (small per reconnaissance).
- `crates/slicer-host/src/gcode_emit.rs:60-:130` — confirm the `orca_type_label` match arms for `ExtrusionRole::Skirt`/`Brim`.
- `crates/slicer-host/src/dispatch.rs:2840-:2900` — confirm the `dispatch_finalization_call` arm for `SkirtBrim`.
- `crates/slicer-ir/src/slice_ir.rs:1280-:1330`, `:1460-:1530` — `ExtrusionRole`, `PrintEntity`, `LayerCollectionIR`.
- `docs/02_ir_schemas.md` — delegate SUMMARY of E semantics + preamble.
- `docs/05_module_sdk.md` — load directly the Finalization Stage section (≤ 40 lines).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate every read.
- `target/`, `Cargo.lock`, generated bindings — never load.
- `.ralph/specs/16_skirt-brim-finalization-live-path/` — DO NOT reopen. The predecessor packet's conclusion is preserved.
- Full `crates/slicer-host/src/dispatch.rs` — out of range unless diagnosis forces it.
- Full `crates/slicer-host/src/pipeline.rs` outside `:200-:280`.
- Full `docs/07_implementation_status.md` — delegate row insertion.
- Other module crates (`wipe-tower`, `tree-support`, etc.) — not relevant.

## Expected Sub-Agent Dispatches

- **Track A diagnosis (the heaviest dispatch):** "In `modules/core-modules/skirt-brim/src/lib.rs`, `crates/slicer-host/src/dispatch.rs:2840-:2900`, and `crates/slicer-host/src/config_schema.rs`, identify EXACTLY ONE root cause for why the live SkirtBrim finalization module produces zero `;TYPE:Skirt|;TYPE:Brim` blocks in Benchy output. Examine these candidate causes in order: (1) `skirt_brim_enabled` config default not applied; (2) dispatcher does not call the module; (3) `skirt_loops` read as 0; (4) entities produced but on wrong layer; (5) `ExtrusionRole::Skirt` stripped before emit. Return: SUMMARY ≤ 100 words naming ONE cause + ONE smallest fix (one file, one change). If two causes are present, escalate — return 'ESCALATE: two-cause diagnosis required, recommend Track A split to packet 54a'."
- "Run `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd`; return FACT pass/fail; SNIPPETS for failing tests."
- "Run `cargo test -p slicer-host --test gcode_relative_extrusion_tdd`; return FACT pass/fail; SNIPPETS for failing tests."
- "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail."
- "OrcaSlicer M82/M83 emission pattern in `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — FACT, ≤ 8 lines."
- "OrcaSlicer Brim.cpp / Print.cpp role-tagging contract — SUMMARY ≤ 200 words."
- "Append TASK-142a and TASK-155 rows to `docs/07_implementation_status.md`; append DEV-009 progress to `docs/DEVIATION_LOG.md`; return EDITED/NOT-EDITED."

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

## Context Cost Estimate

- Aggregate: M.
- Largest single step: Step 1 (Track A diagnosis) — S but RISKY. The FACT must come back as a SINGLE cause; ambiguity escalates rather than expands.
- Highest-risk dispatch: Track A diagnosis. Required return format = SUMMARY ≤ 100 words with one cause + one fix, OR `ESCALATE: ...` literal.

## Open Questions

- None at draft time. The escalation hatch (Track A → packet 54a) is the explicit answer to "what if Step 1 finds a bigger problem".
