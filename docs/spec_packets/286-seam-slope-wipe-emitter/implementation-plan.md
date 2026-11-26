# Implementation Plan: 286-seam-slope-wipe-emitter

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs (wayfinder queue packet — no `TASK-###` slice applies).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Config surface + guard binary skeleton (AC-1)

- Task IDs: none (wayfinder queue packet 286, ticket 60)
- Objective: declare all eight keys end-to-end (schema → resolved config → manifests → twin shadowing) with a failing-then-passing schema test.
- Precondition: packet 285's field/table names known (SUMMARY dispatch) so no key or arm collides.
- Postcondition: `schema_declares_all_eight_keys` passes; defaults movement byte-identical except the single `wipe_on_loops` shadow value.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `[1290-1360]` (`declare_resolved_config!` invocation region; re-derive at use)
  - `crates/slicer-gcode/src/serialize.rs` - lines `[515-560]` (padding twins + 284-precedent arms)
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs` (new: closed-loop emit fixture + AC-1 test; fixture reused by Steps 2–4)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate; never load)
  - `crates/slicer-gcode/src/emit.rs` (Step 2 owns it)
  - `ORCA_CONFIG_PADDING` table in `serialize.rs` (rule 2: shadow, never edit)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New `ResolvedConfig` fields ride the `declare_resolved_config!` macro (overlay arms auto-covered per ticket 126), but struct-literal sites compiling against the struct must still be inventoried.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
- Expected sub-agent dispatches:
  - Question: every struct-literal site compiling against `ResolvedConfig` + tests asserting field counts; scope: `crates/ modules/ xtask/`; return: `LOCATIONS` (≤20 entries, one context line each)
  - Question: 285's field/table/arm names for collision check; scope: `docs/spec_packets/285-seam-scarf-joint-emitter/`; return: `SUMMARY` (≤150 words, names only)
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - range read only if the fixture bakes mm constants (else skip)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; confirm the eight declared types/defaults/bounds (do not write the tables without this read)
- Verification:
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail (struct-literal blast radius)
- Exit condition: schema test passes AND workspace check passes AND `git diff --stat` shows no `emit.rs` change AND no padding-table line changed.

### Step 1b: Manifest rows + doc regen (AC-1)

- Task IDs: none (wayfinder queue packet 286, ticket 60)
- Objective: land the eight schema rows/tables outside the Rust sources and prove the regen is fresh.
- Precondition: Step 1 green.
- Postcondition: `gen-config-docs --check` passes with the eight keys present in both generated outputs.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/config/host-keys.toml` - `[resolved_config]` section only
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - `[config.schema.*]` region only (re-derive range at use)
- Files allowed to edit (at most 3):
  - `docs/config/host-keys.toml`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate; never load)
  - `ORCA_CONFIG_PADDING` table (rule 2: untouched)
  - `crates/slicer-gcode/src/emit.rs` (Step 2 owns it)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no struct field or constant in this step (manifest rows only).
- Expected sub-agent dispatches:
  - None. No authoritative-doc fact-check or cargo run in this step needs delegation beyond the verification commands below.
- Context cost: `S`
- Authoritative docs:
  - None beyond the Step 1 table (no new doc facts needed).
- OrcaSlicer refs:
  - None — canonical table fixed in Step 1.
- Verification:
  - `rg -q 'seam_slope_type' docs/config/host-keys.toml` - FACT pass/fail
  - `cargo xtask gen-config-docs --check` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code only (rebuild without `--check` if stale — the manifest schema edit feeds guest WASM per ticket 101)
- Exit condition: both greps pass AND `--check` passes AND guests fresh (rebuilt if the manifest edit staled them).

### Step 2: Slope ramp generalising 285's scarf stage (AC-2–AC-5)

- Task IDs: none (wayfinder queue packet 286, ticket 60)
- Objective: build the slope-gate chain and steps-modulated ramp as an additive generalisation of 285's scarf block, with geometry two-run comparisons per AC.
- Precondition: Steps 1–1b green; packet 285 landed (activation blocker) — re-run dispatch 4 first and reconcile anchor names if 285's block drifted at its swarm.
- Postcondition: AC-2–AC-5 tests pass; 285's scarf tests still pass (re-run its guard binary when landed, else record the skip).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - `DefaultGCodeEmitter::emit_gcode` loop region + 285's scarf block only (re-derive ranges at use; never the whole file)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate; never load)
  - `docs/spec_packets/285-seam-scarf-joint-emitter/` beyond the anchor SUMMARY (no cross-packet edits)
  - `crates/slicer-ir/src/resolved_config.rs`, manifests (Step 1 owns them)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no new field or constant in this step (pure `emit.rs` logic + tests).
- Expected sub-agent dispatches:
  - Question: slope gating order + `ExtrusionLoopSloped` ramp construction inputs; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (≤200 words, no code unless asked)
  - Question: 285's scarf block anchor names as landed; scope: `docs/spec_packets/285-seam-scarf-joint-emitter/`; return: `SUMMARY` (≤150 words, names only)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct range read for `mm_to_units` at the min-length/start-height boundary
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; `GCode::extrude_loop` gate order + ramp shape only
  - `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` - delegate; `Layer::is_perimeter_compatible` grouping need only (drop recorded as divergence)
- Verification:
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd default_path_no_slope_no_wipe 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd slope_type_gates_which_loops_ramp 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd steps_length_and_entire_loop_modulate_ramp 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd inner_walls_and_start_height_modulate 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: all four slope tests pass AND default output is geometry-identical to Step 1's baseline except the pinned shadow value AND no new `order_lock` handling yet (Step 4 owns it).

### Step 3: Loop-end wipe site + retract coincidence (AC-6)

- Task IDs: none (wayfinder queue packet 286, ticket 60)
- Objective: emit the pre-leave and pre-external inward wipe moves with the at-most-one-wipe precedence over 277's retract site.
- Precondition: Step 2 green; 277's retract-wipe `Move` trigger shape known (SUMMARY dispatch — read the shape, never the draft's steps).
- Postcondition: AC-6 passes; Steps 1–2 tests still pass (no slope regression from the wipe site).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - loop-end region of `emit_gcode` only (re-derive range at use)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate; never load)
  - `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/` beyond the trigger-shape SUMMARY (no cross-packet edits; 277 stays untouched)
  - Retract emission arms in `emit.rs` outside the coincidence guard (no behaviour change to retracts)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no new field or constant in this step.
- Expected sub-agent dispatches:
  - Question: loop-wipe trigger sites + neighbour-qualification shape; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` (≤200 words)
  - Question: 277's retract-wipe `Move` trigger shape for the coincidence guard; scope: `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/packet.spec.md`; return: `SUMMARY` (≤150 words)
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - range read only if the wipe vector bakes mm constants (else skip)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; loop-wipe sites + neighbour qualification only
- Verification:
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd loop_wipes_emit_at_loop_boundaries 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT full-binary green (no slope regression)
- Exit condition: AC-6 passes AND full guard binary green AND retract-only fixtures emit zero loop-wipe moves.

### Step 4: Bounds rejection, order_lock bypass, DEV-178, docs (AC-N1, AC-N2)

- Task IDs: none (wayfinder queue packet 286, ticket 60)
- Objective: close the packet — validation gate, lock bypass, deviation row, doc regen, full gates.
- Precondition: Steps 1–3 green.
- Postcondition: every pipe-suffixed AC passes; DEV-178 written collision-free; docs fresh; workspace gates green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail rows only (re-derive `max(DEV-*)` over LOG + `docs/spec_packets/*/` before writing)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs` (bounds gate + lock bypass)
  - `crates/slicer-gcode/tests/seam_slope_wipe_emission_tdd.rs` (AC-N1 + AC-N2)
  - `docs/DEVIATION_LOG.md` (DEV-178 row)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate; never load)
  - `ORCA_CONFIG_PADDING` table (rule 2: untouched)
  - Any other packet directory (packet safety)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not applicable — no new field or constant in this step (DEV rows are docs).
- Expected sub-agent dispatches:
  - Question: current `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/`; scope: those paths; return: `FACT` (one number)
- Context cost: `S`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - delegated SUMMARY only if the bounds errors need new codes beyond the ticket-113 pattern (else skip)
- OrcaSlicer refs:
  - None — no new canonical reads in this step (bounds/divergence positions fixed in Steps 1–3).
- Verification:
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd locked_paths_bypass_slope_and_wipe 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask gen-config-docs --check` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code only (rebuild without `--check` if stale — the manifest schema edit feeds guest WASM per ticket 101)
- Exit condition: all AC commands PASS AND `rg -q 'DEV-178' docs/DEVIATION_LOG.md` AND `rg -q 'seam_slope_type' docs/15_config_keys_reference.md` AND `rg -q 'seam_slope_type' docs/config/host-keys.toml` AND guests fresh.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Schema + fixture; two bounded dispatches |
| Step 1b | S | Manifest rows + regen freshness |
| Step 2 | M | Slope stage; largest step, still splittable-neutral (one function region) |
| Step 3 | S | Wipe site; two SUMMARY dispatches |
| Step 4 | S | Gates + docs; one FACT dispatch |

Aggregate `M`; no step is L — no split required before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
