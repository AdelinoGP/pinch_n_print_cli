# Implementation Plan: 52_gcode-feedrate-emission

## Execution Rules

- One atomic step at a time.
- Each step must map back to `TASK-153`.
- TDD first: failing test, then implementation, then the narrowest falsifying validation.
- Honor the context-discipline preamble: stop reading at 60%, hand off at 85%.

## Steps

### Step 1: Discovery — confirm config threading and OrcaSlicer defaults

- Task IDs: `TASK-153`
- Objective: Establish (a) what handle (`&ConfigView` / `&ResolvedConfig` / `&HashMap<ConfigKey, ConfigValue>`) the gcode-emit builder receives today, and (b) the OrcaSlicer default values + rounding rule for the eight speed keys.
- Precondition: Packet activated; reconnaissance dispatches in `design.md` not yet executed.
- Postcondition: Both questions answered as FACTs; values recorded in this packet's `design.md` "Locked Assumptions" section by editing that file.
- Files allowed to read: none directly. Pure-dispatch step.
- Files allowed to edit (≤ 3): `.ralph/specs/52_gcode-feedrate-emission/design.md` (append FACT block).
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/*` (delegated), `crates/slicer-host/src/dispatch.rs`.
- Expected sub-agent dispatches:
  - "What concrete type is passed as the config handle to the function that builds `GCodeCommand`s from `LayerCollectionIR` in `crates/slicer-host/src/gcode_emit.rs`? Return: FACT, one line with the type name and the function signature."
  - "Return verbatim the OrcaSlicer default values (mm/s) for the eight speed keys named in `requirements.md`. Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`. Return: FACT, one row per key (`key = <number> mm/s`), ≤ 12 lines."
  - "What rounding rule does `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp::set_speed` use to convert mm/s to mm/min G-code value? Return: FACT, ≤ 3 lines."
- Context cost: S.
- Authoritative docs: none in this step.
- OrcaSlicer refs: as above, delegated.
- Verification: dispatches return well-formed FACT blocks; if any return SUMMARY or SNIPPETS instead, re-dispatch with tightened scope.
- Exit condition: design.md has the eight default values written in, plus the rounding rule (e.g. "round-half-to-even, integer mm/min").

### Step 2: Add failing TDD tests

- Task IDs: `TASK-153`
- Objective: Write `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` containing all 8 acceptance tests + 3 negative tests from `packet.spec.md`. Tests must fail at red, not panic.
- Precondition: Step 1 complete; defaults known.
- Postcondition: `cargo test -p slicer-host --test gcode_feedrate_emission_tdd` runs and every assertion fails with a clear message (no panics from missing types).
- Files allowed to read: `crates/slicer-host/src/gcode_emit.rs` (range `:200-:320` and `:380-:480`); `crates/slicer-host/src/config_schema.rs` (full, < 300 lines); `crates/slicer-ir/src/slice_ir.rs` (range `:1280-:1330`, `:1460-:1530`); `crates/slicer-host/tests/orca_comment_contract_tdd.rs` (full — small reference test for the IR-construction idiom).
- Files allowed to edit (≤ 3): `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` (new).
- Files explicitly out-of-bounds for this step: `crates/slicer-host/src/pipeline.rs`, `crates/slicer-host/src/dispatch.rs`, all of `modules/`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_feedrate_emission_tdd --no-run`; return FACT (compiles? yes/no) and SNIPPETS for any compile error (≤ 20 lines)."
  - "Run `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`; return FACT — expected: every test fails with a single named assertion."
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` — delegate a SUMMARY of `GCodeCommand::Move` and `ExtrusionPath3D`.
- OrcaSlicer refs: none in this step.
- Verification: `cargo test -p slicer-host --test gcode_feedrate_emission_tdd` runs to red. No panic stacks.
- Exit condition: every named test from `packet.spec.md` exists and is at red.

### Step 3: Register the eight speed config keys

- Task IDs: `TASK-153`
- Objective: Add eight `ConfigField` entries to `config_schema.rs` with the OrcaSlicer defaults from Step 1's FACT. Each registered as `ConfigValue::Float`. Validation rejects non-float supplied values with `ConfigValidationError` naming the key.
- Precondition: Step 2 complete; specifically `speed_keys_registered_with_defaults` and `rejects_non_float_speed_config` tests are red.
- Postcondition: Those two tests pass; the rest still fail (until Step 4 wires the resolver).
- Files allowed to read: `crates/slicer-host/src/config_schema.rs` (full).
- Files allowed to edit (≤ 3): `crates/slicer-host/src/config_schema.rs`.
- Files explicitly out-of-bounds for this step: `gcode_emit.rs`, all module crates.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- speed_keys_registered_with_defaults rejects_non_float_speed_config`; return FACT pass/fail."
  - "Run `cargo check -p slicer-host`; return FACT pass/fail and SNIPPETS of any error."
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: defaults already extracted in Step 1.
- Verification: `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- speed_keys_registered_with_defaults rejects_non_float_speed_config` → pass.
- Exit condition: two named tests green; clippy clean for `config_schema.rs`.

### Step 4: Implement `resolve_feedrate` and wire the three call sites

- Task IDs: `TASK-153`
- Objective: Add the `resolve_feedrate(role, speed_factor, &config) -> Option<f32>` helper to `gcode_emit.rs`; replace the three `f: None` literals at `:228`, `:282`, `:309` with calls to the helper. Travel-move builder gains a fallback to `resolve_feedrate(&ExtrusionRole::Custom("Travel"), 1.0, &config)` when `tm.f` is `None`.
- Precondition: Step 3 complete; config keys are registered.
- Postcondition: All eight acceptance tests + three negative tests pass; `orca_comment_contract_tdd` still passes.
- Files allowed to read: `crates/slicer-host/src/gcode_emit.rs` (range `:200-:480`); `crates/slicer-host/src/config_schema.rs` (full); `crates/slicer-ir/src/slice_ir.rs` (`:1280-:1330`, `:1460-:1530`).
- Files allowed to edit (≤ 3): `crates/slicer-host/src/gcode_emit.rs`.
- Files explicitly out-of-bounds for this step: pipeline.rs, dispatch.rs, all module crates.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`; return FACT (all pass) or SNIPPETS for any failing test (≤ 20 lines)."
  - "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail."
  - "Run `cargo clippy -p slicer-host -- -D warnings`; return FACT pass/fail and SNIPPETS for any lint."
- Context cost: M.
- Authoritative docs: `docs/08_coordinate_system.md` — read directly, confirm mm/min convention.
- OrcaSlicer refs: rounding rule already extracted in Step 1.
- Verification: `cargo test -p slicer-host --test gcode_feedrate_emission_tdd` → all green; `cargo test -p slicer-host --test orca_comment_contract_tdd` → green.
- Exit condition: every test from this packet passes; regression test green; clippy clean.

### Step 5: Backlog hygiene and deviation note

- Task IDs: `TASK-153`
- Objective: Insert TASK-153 row in `docs/07_implementation_status.md` under Phase H; append a DEV-009 remediation progress note in `docs/DEVIATION_LOG.md`. Mark this packet's `packet.spec.md` status from `draft` to `implemented` only after the acceptance ceremony in the gate below.
- Precondition: Step 4 complete and all tests green.
- Postcondition: docs/07 carries TASK-153 with status `[x] Closed YYYY-MM-DD`; DEV-009 has a remediation row referencing this packet.
- Files allowed to read: `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` (load directly, small).
- Files allowed to edit (≤ 3): `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `.ralph/specs/52_gcode-feedrate-emission/packet.spec.md` (status flip — only after gate).
- Files explicitly out-of-bounds for this step: any source code (no edits in this step).
- Expected sub-agent dispatches:
  - "Append a new row in the Phase H table of `docs/07_implementation_status.md` for TASK-153; return EDITED/NOT-EDITED and the resulting row."
  - "Append a DEV-009 progress entry in `docs/DEVIATION_LOG.md`; return EDITED/NOT-EDITED."
- Context cost: S.
- Authoritative docs: as above.
- OrcaSlicer refs: none.
- Verification: row exists in both docs.
- Exit condition: both docs updated; ready for the Packet Completion Gate.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Pure-dispatch discovery; three FACT returns. |
| Step 2 | M | Author the failing test file; range-read only. |
| Step 3 | S | Single-file edit in `config_schema.rs`. |
| Step 4 | M | Resolver + three call-site changes in `gcode_emit.rs`. |
| Step 5 | S | Docs hygiene only. |

Aggregate: M. No step is L.

## Packet Completion Gate

- All five steps complete with exit conditions met.
- `cargo test -p slicer-host --test gcode_feedrate_emission_tdd` — every test green (dispatched as FACT).
- `cargo test -p slicer-host --test orca_comment_contract_tdd` — green (regression).
- `cargo check --workspace` — green.
- `cargo clippy -p slicer-host -- -D warnings` — green.
- `docs/07_implementation_status.md` updated for TASK-153 (via worker dispatch — never load the full backlog).
- `docs/DEVIATION_LOG.md` has DEV-009 remediation progress note.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (a single sub-agent call running them all and returning a FACT block of pass/fail per command is acceptable).
- Confirm packet-level verification commands are green.
- Record peak implementer context usage; if > 70%, log it as a lesson for `spec-packet-generator` future runs.
- Only AFTER the above: flip `packet.spec.md` status to `implemented`.
