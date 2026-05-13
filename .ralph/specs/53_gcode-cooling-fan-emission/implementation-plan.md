# Implementation Plan: 53_gcode-cooling-fan-emission

## Execution Rules

- One atomic step at a time.
- Each step maps to either `TASK-154` (new cooling module) or `TASK-152d` (supersession metadata).
- TDD first.
- Honor context-discipline: stop reading at 60%, hand off at 85%.

## Steps

### Step 1: Discovery — capability surface, OrcaSlicer defaults, dispatcher arm

- Task IDs: `TASK-154`
- Objective: Get FACTs for (a) skirt-brim's manifest capability set + crate-type, (b) OrcaSlicer cooling defaults for the eight keys, (c) the existing dispatcher arm template at `dispatch.rs:2854`, (d) the OrcaSlicer cooling algorithm SUMMARY.
- Precondition: Packet activated; design's Open Question on doc style resolved by user.
- Postcondition: FACTs and SUMMARY recorded inside `design.md` under "Locked Assumptions" by appending a discovery block.
- Files allowed to read: none directly (pure-dispatch).
- Files allowed to edit (≤ 3): `.ralph/specs/53_gcode-cooling-fan-emission/design.md` (append).
- Files explicitly out-of-bounds for this step: every source file (Step 1 is dispatch-only).
- Expected sub-agent dispatches: the four listed in `design.md` § "Expected Sub-Agent Dispatches".
- Context cost: S.
- Authoritative docs: none in this step.
- OrcaSlicer refs: as listed; delegated.
- Verification: four well-formed FACT/SUMMARY returns. Re-dispatch any that exceed the agreed format.
- Exit condition: design.md updated; ready to scaffold the module.

### Step 2: Register cooling config keys (TDD-first)

- Task IDs: `TASK-154`
- Objective: Add eight `ConfigField` registrations to `config_schema.rs`, with the OrcaSlicer defaults from Step 1. Add a failing TDD test `cooling_keys_registered` and `rejects_malformed_cooling_config` first; turn them green.
- Precondition: Step 1 complete; defaults known.
- Postcondition: Two named tests green; rest still red (no module yet).
- Files allowed to read: `crates/slicer-host/src/config_schema.rs` (full).
- Files allowed to edit (≤ 3): `crates/slicer-host/src/config_schema.rs`; `crates/slicer-host/tests/gcode_part_cooling_emission_tdd.rs` (new — start the file here with the two tests).
- Files explicitly out-of-bounds for this step: `dispatch.rs`, any module crate.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_part_cooling_emission_tdd -- cooling_keys_registered rejects_malformed_cooling_config`; return FACT pass/fail."
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: defaults already extracted in Step 1.
- Verification: the two named tests pass.
- Exit condition: two tests green; clippy clean for `config_schema.rs`.

### Step 3: Scaffold the new `cooling` module crate

- Task IDs: `TASK-154`
- Objective: Create `modules/core-modules/part-cooling/` with `Cargo.toml`, `part-cooling.toml`, `src/lib.rs`. The crate compiles to a wasm artefact (empty `run_finalization` that returns `Ok(())`). Add the crate to `build-core-modules.sh`. Add the dispatcher arm in `dispatch.rs` around `:2854`.
- Precondition: Step 2 complete.
- Postcondition: `./modules/core-modules/build-core-modules.sh` builds the new module successfully. The dispatcher loads it (test `cooling_module_invoked_in_finalization` still red — module has no behaviour yet).
- Files allowed to read: `modules/core-modules/skirt-brim/Cargo.toml`, `modules/core-modules/skirt-brim/skirt-brim.toml`, `modules/core-modules/skirt-brim/src/lib.rs` (all small per reconnaissance); `crates/slicer-host/src/dispatch.rs` range `:2840-:2900`; `docs/03_wit_and_manifest.md` (manifest schema section); `modules/core-modules/build-core-modules.sh`.
- Files allowed to edit: `modules/core-modules/part-cooling/src/lib.rs` (new), `modules/core-modules/part-cooling/Cargo.toml` (new), `modules/core-modules/part-cooling/part-cooling.toml` (new), `modules/core-modules/part-cooling/wit-guest/Cargo.toml` (new), `modules/core-modules/part-cooling/wit-guest/src/lib.rs` (new), `Crates.toml` (add workspace member), `crates/slicer-host/Cargo.toml` (add dev-dependency on part-cooling), `modules/core-modules/build-core-modules.sh` (one-line addition).
- Files explicitly out-of-bounds for this step: full dispatch.rs outside the range, full pipeline.rs, every doc except 03.
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail and SNIPPETS of error if any."
  - "Run `cargo build -p slicer-host`; return FACT pass/fail."
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` (manifest schema).
- OrcaSlicer refs: none in this step.
- Verification: build script succeeds; `part-cooling.wasm` artefact present.
- Exit condition: empty cooling module loaded by host; build script green.

Note: if this step trends toward L during implementation (e.g. manifest-format complications surface > 30 minutes of digging), split into 3a (manifest + crate scaffolding) and 3b (dispatcher wiring) before continuing.

### Step 4: Implement the cooling algorithm (RED → GREEN)

- Task IDs: `TASK-154`
- Objective: Implement `run_finalization` in `modules/core-modules/part-cooling/src/lib.rs`. Algorithm: first-layer-disable, max-speed, overhang-bump, end-gcode-off. Write the remaining TDD tests first (red); implement until green.
- Precondition: Step 3 complete; cooling module loads but does nothing.
- Postcondition: All six positive ACs + three negative cases pass.
- Files allowed to read: `modules/core-modules/skirt-brim/src/lib.rs` (template); `crates/slicer-ir/src/slice_ir.rs` range `:1460-:1530` and `:1280-:1330` (LayerCollectionIR + PrintEntity); `crates/slicer-host/src/gcode_emit.rs` range `:460-:480` (FanSpeed serializer).
- Files allowed to edit (≤ 4): `modules/core-modules/part-cooling/src/lib.rs`, `crates/slicer-host/tests/gcode_part_cooling_emission_tdd.rs`, `crates/slicer-sdk/src/traits.rs` (push_fan_speed helper), `Crates.toml` (workspace member if not added in Step 3).
- Files explicitly out-of-bounds for this step: dispatch.rs (already wired in Step 3), pipeline.rs, all docs.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_part_cooling_emission_tdd`; return FACT pass/fail; SNIPPETS for failing tests."
  - "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail."
  - "Run `cargo clippy -p slicer-host -p cooling -- -D warnings`; return FACT pass/fail."
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` (delegate SUMMARY of `GCodeCommand::FanSpeed` and `LayerCollectionIR`).
- OrcaSlicer refs: CoolingBuffer SUMMARY from Step 1 (already in design.md).
- Verification: all tests in `gcode_part_cooling_emission_tdd.rs` pass; `orca_comment_contract_tdd` still passes.
- Exit condition: all packet ACs and negative cases green.

### Step 5: Docs hygiene — TASK-152c supersession + DEV-009 progress

- Task IDs: `TASK-152d`
- Objective: Edit `docs/05_module_sdk.md` to remove the cooling rejection snippet from the Rejections section (cooling is now supported via the finalization-stage module). Mark TASK-152c as `Superseded by TASK-152d` in `docs/07_implementation_status.md`. Append TASK-152d and TASK-154 rows. Append a supersession entry + DEV-009 progress entry in `docs/DEVIATION_LOG.md` and `docs/14_deviation_audit_history.md`.
- Precondition: Step 4 complete; all tests green.
- Postcondition: All four docs updated.
- Files allowed to read: `docs/05_module_sdk.md` (range — the Rejections section only), `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`.
- Files allowed to edit: `docs/05_module_sdk.md`, `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`.
- Files explicitly out-of-bounds for this step: every source file.
- Expected sub-agent dispatches:
  - "Append a TASK-152d row + TASK-154 row in the Phase H table of `docs/07_implementation_status.md`; mark TASK-152c row with `Superseded by TASK-152d`. Return EDITED/NOT-EDITED with the resulting rows."
  - "Append a supersession entry + DEV-009 progress entry in `docs/DEVIATION_LOG.md`. Return EDITED/NOT-EDITED."
- Context cost: S.
- Authoritative docs: as above.
- OrcaSlicer refs: none.
- Verification: rows visible in `docs/07`; entries visible in `DEVIATION_LOG.md` and `docs/14_deviation_audit_history.md`; `docs/05_module_sdk.md` Rejections section no longer contains the cooling rejection snippet.
- Exit condition: docs updated; ready for the Packet Completion Gate.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Pure-dispatch discovery. |
| Step 2 | S | Config keys + two failing-then-green tests. |
| Step 3 | M | New module scaffold + dispatcher arm. Can split into 3a/3b if it trends to L. |
| Step 4 | M | Algorithm + remaining tests. |
| Step 5 | S | Docs hygiene. |

Aggregate: M. No step is L.

## Packet Completion Gate

- All five steps complete with exit conditions met.
- `cargo test -p slicer-host --test gcode_part_cooling_emission_tdd` — all green (FACT dispatch).
- `cargo test -p slicer-host --test orca_comment_contract_tdd` — green.
- `./modules/core-modules/build-core-modules.sh` — green (the new `part-cooling.wasm` artefact exists).
- `cargo check --workspace` — green.
- `cargo clippy --workspace -- -D warnings` — green.
- `docs/07_implementation_status.md` shows TASK-152c as `Superseded by TASK-152d`; new TASK-152d + TASK-154 rows present.
- `docs/05_module_sdk.md` Rejections section carries the new pointer.
- `docs/DEVIATION_LOG.md` carries the supersession + DEV-009 progress entries.
- `packet.spec.md` ready to flip to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (a single sub-agent call returning FACT block of pass/fail per command).
- Confirm packet-level verification commands green.
- Record implementer peak context usage; if > 70%, log as a packet-authoring lesson.
- Flip `packet.spec.md` status to `implemented`.
