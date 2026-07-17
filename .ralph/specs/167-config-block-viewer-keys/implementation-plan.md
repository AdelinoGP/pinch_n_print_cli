# Implementation Plan: 167-config-block-viewer-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Write the RED integration tests

- Task IDs: `TASK-273`
- Objective: append `config_block_meets_orca_minimum_key_gate`, `config_block_synthesizes_non_bbl_printer_model`, and `config_block_fork_keys_never_shadowed` to the CONFIG_BLOCK integration test file; the printer_model and no-shadowing tests are RED (synthesis does not exist yet), the ≥80-count test is GREEN pre-change and must stay GREEN post-change.
- Precondition: clean working tree on the packet branch.
- Postcondition: three tests exist; RED/GREEN state recorded in `.ralph/specs/167-config-block-viewer-keys/closure-log.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — lines 1-120 and 420-500 only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  - `.ralph/specs/167-config-block-viewer-keys/closure-log.md`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no production edits this step); `docs/ORCA_CONFIG_REFERENCE.md`; `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "run the three new tests"; scope: `cargo test -p slicer-runtime --test integration -- config_block`; return: `FACT` per-test pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — lines 1660-1720 only (envelope contract the assertions encode)
- OrcaSlicer refs:
  - upstream behavior cited by function name only (`ConfigBase::load_from_gcode_file`, `GCodeProcessor::apply_config`, `s_IsBBLPrinter`); no OrcaSlicerDocumented reads required
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block 2>&1 | tee target/test-output.log | grep "^test result"` — FACT pass/fail per test
- Exit condition: printer_model + no-shadowing tests fail for the expected reason (missing `printer_model` line / absent key), count test passes; any other failure mode falsifies the step.

### Step 2: Rework ORCA_CONFIG_PADDING

- Task IDs: `TASK-273`
- Objective: remove the 34 speed/accel/jerk-valued entries enumerated in `design.md` and add ~45 neutral replacement entries (upstream-default values, name classes `machine_max_*`/`*speed*`/`*acceleration*`/`*jerk*` forbidden); update the table and loop doc comments.
- Precondition: Step 1 tests in place.
- Postcondition: AC-1 grep PASS; `config_block_meets_orca_minimum_key_gate` still GREEN.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` — lines 200-480 only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` (delegate lookups); test files (no edits this step); `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "For candidate neutral padding keys <batch>, what are upstream defaults per docs/ORCA_CONFIG_REFERENCE.md, and are any speed/accel/jerk/machine-limit-typed?"; scope: `docs/ORCA_CONFIG_REFERENCE.md`; return: `FACT` ≤5 lines per batch
- Context cost: `S`
- Authoritative docs:
  - `docs/ORCA_CONFIG_REFERENCE.md` — delegated only
- OrcaSlicer refs:
  - none beyond the delegated reference-doc lookups
- Verification:
  - `cd F:/slicerProject/pinch_n_print && awk '/^const ORCA_CONFIG_PADDING/,/^\];/' crates/slicer-gcode/src/serialize.rs | grep -E '"(machine_max_[a-z_]*|[a-z_]*speed[a-z_]*|[a-z_]*acceleration[a-z_]*|[a-z_]*jerk[a-z_]*)"' ; test $? -eq 1 && echo PASS || echo FAIL` — FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_meets_orca_minimum_key_gate 2>&1 | tee target/test-output.log | grep "^test result"` — FACT pass/fail
- Exit condition: both commands PASS; a count under 80 means more neutral keys are required before exiting.

### Step 3: Synthesize printer_model

- Task IDs: `TASK-273`
- Objective: add the `printer_model = Generic PNP Printer` synthesis branch in `serialize_config_block` (guarded by `raw_config.contains_key`, emitted via `emit_config_kv`).
- Precondition: Step 2 exit met.
- Postcondition: `config_block_synthesizes_non_bbl_printer_model` and `config_block_fork_keys_never_shadowed` GREEN.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` — lines 283-400 only
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
- Files explicitly out of bounds:
  - test files (assertions frozen from Step 1); docs; `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "run the config_block test filter"; scope: `cargo test -p slicer-runtime --test integration -- config_block`; return: `FACT` per-test pass/fail
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block 2>&1 | tee target/test-output.log | grep "^test result"` — FACT pass/fail
- Exit condition: all config_block-filtered tests GREEN, including the two previously-RED tests.

### Step 4: Golden re-bless inventory + pre-existing invariants

- Task IDs: `TASK-273`
- Objective: inventory tests asserting CONFIG_BLOCK bytes (golden `precision_legacy_20mmbox.gcode` suspect), re-bless any golden whose diff is confined to CONFIG_BLOCK lines (motion lines byte-identical), and re-run the pre-existing header/config-block invariant tests.
- Precondition: Step 3 exit met.
- Postcondition: AC-N2 GREEN; goldens re-blessed with a reviewed CONFIG_BLOCK-only diff logged in the closure log.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` — grep-driven reads only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` (re-bless only if its comparison covers CONFIG_BLOCK)
  - `.ralph/specs/167-config-block-viewer-keys/closure-log.md`
- Files explicitly out of bounds:
  - production source (no code edits this step); `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "Which golden/e2e tests assert CONFIG_BLOCK bytes or line counts?"; scope: `crates/slicer-runtime/tests`, `crates/slicer-gcode/tests`; return: `LOCATIONS` ≤20
  - Question: "diff the failing golden — are all changed lines within CONFIG_BLOCK_START..END?"; scope: the golden file + regenerated output; return: `FACT` yes/no + ≤10-line SNIPPETS of any motion-line diff
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_header 2>&1 | tee target/test-output.log | grep "^test result"` — FACT pass/fail
  - the affected golden test binary re-run — FACT pass/fail
- Exit condition: invariant tests and golden tests GREEN; any motion-line diff falsifies the packet (stop and diagnose).

### Step 5: Document the fork-facing contract + crosswalk

- Task IDs: `TASK-273`
- Objective: append the "CONFIG_BLOCK viewer-key contract" subsection to `docs/02_ir_schemas.md` (fork-required keys: `printer_model`, `filament_density`, `filament_cost`, `printable_area`, `nozzle_diameter`, `machine_max_*` family; padding exclusion invariant; `Generic PNP Printer` synthesis rule) and mint TASK-273 in `docs/07_implementation_status.md`.
- Precondition: Steps 2-4 exits met.
- Postcondition: AC-4 grep PASS; docs/07 row exists; clippy clean.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` — lines 1660-1720 only
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/07_implementation_status.md` (via worker dispatch appending the TASK-273 row; never a full read)
  - `.ralph/specs/167-config-block-viewer-keys/closure-log.md`
- Files explicitly out of bounds:
  - all source files; `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "append the TASK-273 crosswalk row per task-map.md"; scope: `docs/07_implementation_status.md`; return: `FACT` done + the appended line
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — lines 1660-1720 only
- OrcaSlicer refs:
  - cite upstream consumers by function name in the new subsection; no reads
- Verification:
  - `cd F:/slicerProject/pinch_n_print && grep -q "CONFIG_BLOCK viewer-key contract" docs/02_ir_schemas.md && grep -q "machine_max_" docs/02_ir_schemas.md && echo PASS || echo FAIL` — FACT PASS/FAIL
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: doc grep PASS, `grep -c "TASK-273" docs/07_implementation_status.md` ≥ 1, clippy/check clean.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | three appended tests |
| Step 2 | S | table rework; reference lookups delegated |
| Step 3 | S | one synthesis branch |
| Step 4 | S | delegated golden inventory + re-bless |
| Step 5 | S | doc subsection + crosswalk |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
