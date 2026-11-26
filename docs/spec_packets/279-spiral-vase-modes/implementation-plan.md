# Implementation Plan: 279-spiral-vase-modes

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Config surface — fields, schemas, strict spelling

- Task IDs: `[]` (wayfinder ticket 52)
- Objective: declare all five keys so invalid spellings cannot reach behavior, with canonical defaults.
- Precondition: packet number 279 is the next free number (re-derived 2026-09-07, MAX=278).
- Postcondition: `spiral_mode`/`spiral_mode_smooth` bools, two flow-ratio floats, and the percent-kind cap resolve with canonical defaults; word-form bools and non-numeric cap strings are rejected; `cargo xtask check-literals` is clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - ranged around `gcode_resolution` and `bridge_no_support` declaration sites only
  - `docs/config/host-keys.toml` - ranged around the `gcode_resolution` row only
- Files allowed to edit (at most 3 per atomic sub-change):
  - Sub-change A (host fields): `crates/slicer-ir/src/resolved_config.rs`, `docs/config/host-keys.toml` (plus blast-radius production sites from the LOCATIONS re-derivation, each gaining the five fields here)
  - Sub-change B (manifest alias rows): the two perimeter manifests' `spiral_mode` rows
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`
  - `OrcaSlicerDocumented/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the `ResolvedConfig` struct-literal sites before editing; cite the result inline below.
  - Blast-radius measurement (authoring time, 2026-09-07): 44 files contain `ResolvedConfig {` literals; test-code sites are covered by the check-literals FRU rule while every production exhaustive site gains the five fields inside this step. Worker re-derives with `rg -l 'ResolvedConfig\s*\{' crates modules --glob '*.rs'` at implementation time.
- Expected sub-agent dispatches:
  - Question: enumerate `ResolvedConfig` struct-literal sites affected by five new fields; scope: `crates/ modules/`; return: `LOCATIONS` (≤20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated targeted summary for `[config.schema]` rows
  - `docs/21_data_defaults_and_fixtures.md` - targeted struct-literal rule
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask check-literals` - FACT exit code
- Exit condition: new bounds tests fail before the declarations and pass after; no other test changes behavior.

### Step 2: Unified dispatch and orchestration validation

- Task IDs: `[]` (wayfinder ticket 52)
- Objective: one mode spelling drives dispatch and validation, with no second mechanism.
- Precondition: Step 1 exit holds; `spiral_mode` resolves.
- Postcondition: `spiral_mode=true` (or legacy `spiral_vase=true` fallback) forces classic perimeters; copies>1 without ByObject, incompatible materials, and absolute extrusion each fail fatal with a spiral-named error; AC-1/AC-2/AC-3/AC-N1 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` - ranged around `dedup_same_claim_modules_with_wall_generator` and `SPIRAL_VASE_CONFIG_KEY` only
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` - ranged around the `spiral_vase` read only
- Files allowed to edit (at most 3 per atomic sub-change):
  - Sub-change A (code): `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - Sub-change B (tests): `crates/slicer-scheduler/tests/integration/spiral_vase_modes_tdd.rs` (new), `crates/slicer-scheduler/tests/integration/main.rs` (mod registration only)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`
  - `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: do canonical slicing reads change anything beyond classic-forcing and validation; scope: oracle only; return: `SUMMARY` (≤200 words)
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - targeted orchestration-validation section
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Print.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-1, AC-2, AC-3, AC-N1 all pass; default-path dispatch unchanged (existing scheduler tests green).

### Step 3: SpiralVase emitter stage

- Task IDs: `[]` (wayfinder ticket 52)
- Objective: build the SpiralVase post-processor and hook it into emission.
- Precondition: Step 2 exit holds; mode resolution is available to the emitter.
- Postcondition: enabled layers get continuous-Z ramp, gated XY smoothing under the cap with extrusion rescale, first/last flow ramps (`0` disables its end), and tiny-move removal; disabled mode emits byte-identical output to today; AC-4/AC-5/AC-6 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - ranged around `DefaultGCodeEmitter::emit_gcode` only
  - `docs/08_coordinate_system.md` - targeted mm/unit checklist
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/spiral_vase.rs` (new)
  - `crates/slicer-gcode/src/emit.rs` (one hook + module declaration)
  - `crates/slicer-gcode/tests/spiral_vase_modes_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`
  - `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: confirm `LayerCollectionIR` extrusion-move shape the stage consumes; scope: `crates/slicer-ir/`; return: `SNIPPETS` (≤3 snippets, 30 lines each)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - targeted mm/internal-unit checklist
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SpiralVase.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --test spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-4, AC-5, AC-6 pass; existing `slicer-gcode` test files stay green.

### Step 4: Time-lapse gate and guest freshness

- Task IDs: `[]` (wayfinder ticket 52)
- Objective: extend the TimeLapse gate with the missing `!spiral` clause without breaking guest freshness.
- Precondition: Step 3 exit holds; mode reaches module config.
- Postcondition: TimeLapse injection is suppressed while spiral is on and intact otherwise; guest artifacts are fresh; AC-7 passes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - ranged around `run_gcode_postprocess` TimeLapse gate only
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/machine-gcode-emit/tests/spiral_timelapse_gate_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`
  - `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: confirm module bool-declaration and test-binary pattern; scope: `modules/core-modules/machine-gcode-emit/`; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated targeted summary for module config declaration
- OrcaSlicer refs: none beyond requirements.md (gate logic is port-side; canonical reference is the `(is_i3 && !spiral) || multi_extruder` shape already recorded).
- Verification:
  - `cargo test -p machine-gcode-emit --test spiral_timelapse_gate_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask build-guests --check` - FACT exit code (rebuild without `--check` if stale, then re-run the test)
- Exit condition: AC-7 passes with fresh guests; existing machine-gcode-emit tests stay green.

### Step 5: Docs, ledger, and end-to-end boundary

- Task IDs: `[]` (wayfinder ticket 52)
- Objective: close out docs, annotations, and the runtime boundary proof.
- Precondition: Steps 1–4 exits hold.
- Postcondition: generated config docs match, 04/05 record the spelling adoption with no count change, runtime integration proves the relative-extrusion boundary end to end, and the workspace gates are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - ranged around P45 rows only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - ranged around P45 rows only
- Files allowed to edit (at most 3 per atomic sub-change):
  - Sub-change A (boundary test): `crates/slicer-runtime/tests/integration/spiral_vase_modes_tdd.rs` (new), `crates/slicer-runtime/tests/integration/main.rs` (mod registration only)
  - Sub-change B (ledger): `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`, `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`
  - `docs/15_config_keys_reference.md` (regenerated only, never hand-edited)
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` Notes authoring rules - targeted read
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test integration spiral_vase_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask gen-config-docs --check` - FACT exit code
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: AC-N2 passes; every pipe-suffixed AC in `packet.spec.md` returns PASS; preflight is ready to run.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Config surface + blast-radius dispatch |
| Step 2 | S | Alias + validation + FWD oracle summary |
| Step 3 | M | New stage; largest step |
| Step 4 | S | One gate clause + freshness check |
| Step 5 | S | Docs + ledger + runtime boundary |

Split before activation if aggregate cost exceeds M or any step is L.

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
