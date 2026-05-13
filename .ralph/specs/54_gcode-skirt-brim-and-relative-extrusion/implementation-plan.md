# Implementation Plan: 54_gcode-skirt-brim-and-relative-extrusion

## Execution Rules

- One atomic step at a time.
- Each step maps to either `TASK-142a` (Track A) or `TASK-155` (Track B).
- TDD first.
- Honor context-discipline: stop reading at 60%, hand off at 85%.
- Track A and Track B are independent; if Track A escalates at Step 1, Track B continues alone and Track A is hand-off to a new packet 54a.

## Steps

### Step 1: Track A — Diagnosis of the live SkirtBrim emit gap

- Task IDs: `TASK-142a`
- Objective: Produce a SUMMARY ≤ 100 words naming exactly ONE root cause + ONE smallest fix (one source file, ≤ 30-line edit) for why the live SkirtBrim finalization module produces zero `;TYPE:Skirt|;TYPE:Brim` blocks in Benchy emit output.
- Precondition: Packet activated.
- Postcondition: Diagnosis FACT recorded in `design.md` as a "Diagnosis Outcome" appended section. If ESCALATE returned, Track A is hand-off to packet 54a and Step 2A is skipped.
- Files allowed to read: none directly (pure-dispatch).
- Files allowed to edit (≤ 3): `.ralph/specs/54_gcode-skirt-brim-and-relative-extrusion/design.md` (append diagnosis); `.ralph/specs/54_gcode-skirt-brim-and-relative-extrusion/packet.spec.md` (only if ESCALATE: flip Track A acceptance criteria to a deferred-to-54a note).
- Files explicitly out-of-bounds for this step: every source file (Step 1 is dispatch-only). The implementer does NOT directly read skirt-brim/src/lib.rs.
- Expected sub-agent dispatches: the Track A diagnosis dispatch in `design.md` § "Expected Sub-Agent Dispatches".
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: Brim.cpp / Print.cpp SUMMARY may be requested to validate the diagnosis (delegated).
- Verification: SUMMARY returned in the agreed format; if it exceeds 100 words or fails to name one cause, re-dispatch with tighter scope.
- Exit condition: design.md has one-cause-one-fix diagnosis recorded, OR ESCALATE recorded and Track A handoff initiated.

### Step 2A: Track A — Write failing tests then apply the diagnosed fix

- Task IDs: `TASK-142a`
- Objective: Write `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs` (≥ 5 tests covering the 4 ACs + 1 negative case in `packet.spec.md`). Apply the one-file fix from Step 1. Bring all Track A tests green.
- Precondition: Step 1 returned a non-ESCALATE diagnosis.
- Postcondition: All Track A tests green; `orca_comment_contract_tdd` still green; `./modules/core-modules/build-core-modules.sh` green if the fix touched the module.
- Files allowed to read: `modules/core-modules/skirt-brim/src/lib.rs` (full, small); `crates/slicer-host/src/gcode_emit.rs:60-:130` (label match); the one file named by Step 1's diagnosis; `crates/slicer-ir/src/slice_ir.rs:1280-:1330` and `:1460-:1530`.
- Files allowed to edit (≤ 3): `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs` (new); the ONE source file named by Step 1's diagnosis; if that file is `skirt-brim/src/lib.rs`, also `modules/core-modules/build-core-modules.sh` is implicitly invoked via the verification dispatch (not edited).
- Files explicitly out-of-bounds for this step: any source file NOT named by Step 1's diagnosis; full dispatch.rs; full pipeline.rs.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd`; return FACT pass/fail; SNIPPETS for failing tests."
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail." (only if the fix touched the skirt-brim module)
  - "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail."
- Context cost: M.
- Authoritative docs: `docs/05_module_sdk.md` (Finalization Stage section, ≤ 40 lines).
- OrcaSlicer refs: Brim.cpp / Print.cpp SUMMARY from Step 1.
- Verification: as above.
- Exit condition: all Track A tests green; regression test green.

### Step 2B: Track B — Register `use_relative_e_distances` (TDD-first)

- Task IDs: `TASK-155`
- Objective: Write `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` containing all 6 ACs + 4 negative tests. Add `use_relative_e_distances` to `config_schema.rs` (`ConfigValue::Bool` default `true`). Bring the `config_schema_registers_bool_default_true` test green; rest remain red.
- Precondition: Independent of Track A; can run in parallel.
- Postcondition: One named test green; the other 9 still red.
- Files allowed to read: `crates/slicer-host/src/config_schema.rs` (full).
- Files allowed to edit (≤ 3): `crates/slicer-host/src/config_schema.rs`; `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` (new).
- Files explicitly out-of-bounds for this step: `gcode_emit.rs`, `pipeline.rs`, all of Track A's files.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_relative_extrusion_tdd -- config_schema_registers_bool_default_true`; return FACT pass/fail."
  - "Run `cargo check -p slicer-host`; return FACT pass/fail."
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: GCodeWriter M82/M83 FACT can be requested now to inform Step 3B (cache the FACT inside design.md).
- Verification: one test green.
- Exit condition: schema test green; clippy clean for `config_schema.rs`.

### Step 3B: Track B — Implement `with_extrusion_mode` and the per-mode serializer branch

- Task IDs: `TASK-155`
- Objective: Add `e_accumulator: f64` and `relative: bool` fields to `DefaultGCodeSerializer`. Add `with_extrusion_mode(relative: bool) -> Self`. `new()` calls `with_extrusion_mode(true)`. Emit `M83` or `M82` in the preamble. Branch the E formatting in the `Move`/`Retract`/`Unretract` arms: relative writes `(move.e - accumulator)`, absolute writes `move.e`. `G92 E0` resets the accumulator. X/Y/Z/F/S/T unchanged. All Track B tests pass (except threading test which needs Step 4B).
- Precondition: Step 2B complete.
- Postcondition: Five of the six remaining Track B tests pass (`default_is_relative_m83`, `e_values_are_per_move_deltas`, `xyzf_unchanged_across_modes`, `delta_sum_matches_absolute_per_g92_block`, plus all 4 negative tests). `absolute_mode_when_flag_false` may still be red until Step 4B threads the flag through the pipeline.
- Files allowed to read: `crates/slicer-host/src/gcode_emit.rs:200-:480` (Move/Retract/Unretract arms + preamble emit); `crates/slicer-ir/src/slice_ir.rs:1460-:1530`.
- Files allowed to edit (≤ 3): `crates/slicer-host/src/gcode_emit.rs`; `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` (additions only).
- Files explicitly out-of-bounds for this step: `pipeline.rs` (Step 4B), `dispatch.rs`, all module crates.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_relative_extrusion_tdd`; return FACT — expected 5 of 6 ACs + 4 of 4 negative tests pass."
  - "Run `cargo clippy -p slicer-host -- -D warnings`; return FACT pass/fail."
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` SUMMARY (delegate — confirm IR-absolute invariant for E).
- OrcaSlicer refs: GCodeWriter M82/M83 FACT.
- Verification: as above.
- Exit condition: 5 of 6 ACs + 4 of 4 negative tests green.

### Step 4B: Track B — Thread the flag through `run_pipeline_with_raw_config`

- Task IDs: `TASK-155`
- Objective: In `crates/slicer-host/src/pipeline.rs` (range `:200-:280`), read `use_relative_e_distances` from `raw_config_source.get(&"use_relative_e_distances")` (default `true` if absent). Forward to `DefaultGCodeSerializer::with_extrusion_mode(...)` at the construction site. `absolute_mode_when_flag_false` flips to green.
- Precondition: Step 3B complete.
- Postcondition: All Track B tests green.
- Files allowed to read: `crates/slicer-host/src/pipeline.rs:200-:280`; `crates/slicer-host/src/config_schema.rs` (full).
- Files allowed to edit (≤ 3): `crates/slicer-host/src/pipeline.rs`.
- Files explicitly out-of-bounds for this step: everything outside pipeline.rs `:200-:280`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_relative_extrusion_tdd`; return FACT — expected all pass."
  - "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail."
  - "Run `cargo clippy -p slicer-host -- -D warnings`; return FACT pass/fail."
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: as above.
- Exit condition: all Track B acceptance + negative tests green; regression test green.

### Step 5: Docs hygiene — TASK rows + DEV-009 progress

- Task IDs: `TASK-142a`, `TASK-155`
- Objective: Append TASK-142a and TASK-155 rows to `docs/07_implementation_status.md` under Phase H. Append DEV-009 progress entries for both subsets in `docs/DEVIATION_LOG.md`.
- Precondition: Step 4B complete (and Step 2A complete, if Track A did not escalate).
- Postcondition: Both docs updated.
- Files allowed to read: `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`.
- Files allowed to edit (≤ 3): `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`.
- Files explicitly out-of-bounds for this step: every source file.
- Expected sub-agent dispatches:
  - "Append TASK-142a (Track A) and TASK-155 (Track B) rows in the Phase H table of `docs/07_implementation_status.md`; reference TASK-142 as predecessor for TASK-142a. Return EDITED/NOT-EDITED."
  - "Append DEV-009 progress entries for skirt-brim and relative-E subsets in `docs/DEVIATION_LOG.md`. Return EDITED/NOT-EDITED."
- Context cost: S.
- Authoritative docs: as above.
- OrcaSlicer refs: none.
- Verification: rows + entries visible in both docs.
- Exit condition: docs updated; ready for the Packet Completion Gate.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (Track A diagnosis) | S | Pure-dispatch; escape hatch on ESCALATE. |
| Step 2A (Track A fix) | M | One-file fix + new test file. Skipped if ESCALATE. |
| Step 2B (Track B schema + tests) | S | Single key registration; failing test stub. |
| Step 3B (Track B serializer branch) | M | Core algorithm + 9 tests flip to green. |
| Step 4B (Track B pipeline threading) | S | One-line edit in pipeline.rs `:200-:280`. |
| Step 5 (Docs hygiene) | S | Doc edits only. |

Aggregate: M. No step is L.

If Step 1 returns ESCALATE:
- Track A becomes packet 54a (new packet to be generated by the implementer or by re-running spec-packet-generator).
- This packet 54 reduces to Track B only and is renamed in place to `54_gcode-relative-extrusion`.
- Track A's task ID (TASK-142a) moves to packet 54a; this packet's task_ids becomes `[TASK-155]` only.
- The implementer surfaces this as a hand-off in the Step 1 report; does NOT silently expand scope.

## Packet Completion Gate

- All applicable steps complete (Track A is conditional on no-ESCALATE).
- `cargo test -p slicer-host --test gcode_skirt_brim_emission_tdd` — green (or marked deferred-to-54a).
- `cargo test -p slicer-host --test gcode_relative_extrusion_tdd` — green.
- `cargo test -p slicer-host --test orca_comment_contract_tdd` — green.
- `./modules/core-modules/build-core-modules.sh` — green (only if Track A touched the module).
- `cargo check --workspace` — green.
- `cargo clippy --workspace -- -D warnings` — green.
- `docs/07_implementation_status.md` carries TASK-142a (or hand-off note) + TASK-155 rows.
- `docs/DEVIATION_LOG.md` carries DEV-009 progress entries.
- `packet.spec.md` ready to flip to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (single sub-agent call returning a FACT block of pass/fail per command).
- Confirm packet-level verification commands green.
- Record implementer peak context usage; if > 70%, log it as a packet-authoring lesson.
- Flip `packet.spec.md` status to `implemented`.
