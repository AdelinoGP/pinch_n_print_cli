# Implementation Plan: 278-gcode-output-emitter-modes

## Execution Rules

- Work one atomic step at a time; every step maps to wayfinder ticket 51 and has `task_ids: []`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every cargo command tees combined output to `target/test-output.log`; inspect that file rather than rerunning truncated output.
- Delegate canonical reads, authoritative-doc checks, cargo runs, and broad symbol inventories using the bounded formats in `design.md`.

## Steps

### Step 1: Declare and carry the emitter settings

- Task IDs: `[]` (wayfinder ticket 51)
- Objective: add exact manifest schemas for `exclude_object`, `gcode_comments`, `gcode_label_objects`, and `gcode_flavor`; add the three booleans to `ResolvedConfig` and host-key documentation; write schema/lock/bounds tests first.
- Precondition: re-derive that no retained bool is already a `ResolvedConfig` field and no manifest declares `gcode_flavor`.
- Postcondition: the four schema tables have exact types/defaults/values; all three bools flow through `to_config_map`, `PartialEq`, and `Hash`; invalid enum/bool forms reject with `ConfigResolutionError::TypeMismatch`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - macro field list and manual `to_config_map`/`PartialEq`/`Hash` symbols only.
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - config schema only.
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - bool arms only.
- Files allowed to edit (at most 3 per atomic sub-change):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_config_schema_tdd.rs` (net-new standalone test target).
  - Separate sub-change: `docs/config/host-keys.toml`, `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, and `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`, `crates/pnp-cli/**`, `OrcaSlicerDocumented/**`.
- Blast-radius discipline:
  - Dispatch a `LOCATIONS` inventory of every `ResolvedConfig {` literal before editing. Add any exhaustive literal and every manual equality/hash/map assertion to this step; do not defer fallout to compilation.
- Expected sub-agent dispatches:
  - Question: list `ResolvedConfig` literals and manual field coverage sites; scope: `crates/**`, `modules/**`; return: `LOCATIONS` <=20.
  - Question: confirm canonical declaration forms; scope: delegated `PrintConfig.cpp`; return: `SUMMARY` <=120 words.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated `[config.schema]` summary.
  - `docs/21_data_defaults_and_fixtures.md` - targeted FRU rule.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegated only.
- Verification:
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-scheduler --test scheduler_integration gcode_output_modes_bounds_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test unit host_keys_doc_lock_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: all four schemas and three typed fields are exact, invalid spellings reject, literals/manual impls are complete, and the three tests pass.

### Step 2: Implement object runs, exclusion markers, and labels

- Task IDs: `[]` (wayfinder ticket 51)
- Objective: test-first implementation of deterministic object-run transitions, safe names/comments, independent human labels, and the flavor marker matrix in `DefaultGCodeEmitter::emit_gcode`.
- Precondition: Step 1 resolves the three booleans and strict flavor values; runtime already supplies `with_flavor`.
- Postcondition: AC-1 through AC-3 pass for repeated runs, every flavor, disabled gates, and CR/LF injection input.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - `DefaultGCodeEmitter` fields/builders, `emit_gcode` layer/entity loops, and tail only.
  - `crates/slicer-gcode/src/flavor.rs` - enum and config strings.
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - fixture constructors only.
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_output_modes_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`, all IR definitions, module source, CLI source, Orca source.
- Blast-radius discipline: no struct/schema field is added; helpers are private and test construction uses existing FRU-compatible `LayerCollectionIR` fixtures.
- Expected sub-agent dispatches:
  - Question: confirm canonical flavor/object start-end matrix and label ordering; scope: delegated `GCode.cpp::process_layer`; return: `SUMMARY` <=150 words.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` - `PostPass::GCodeEmit` and flavor-resolution sections, targeted.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::apply_print_config` and `GCode::process_layer`, delegated only.
- Verification:
  - `cargo test -p slicer-gcode --test gcode_output_modes_tdd exclude_object 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-gcode --test gcode_output_modes_tdd human_object_labels_and_injection_guard 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: exact AC marker fragments/order pass; sorted IDs produce stable ordinals; malicious IDs cannot create a second output line; default exclusion-off emits no firmware marker.

### Step 3: Gate verbose extrusion diagnostics

- Task IDs: `[]` (wayfinder ticket 51)
- Objective: add one opt-in `; filament:` diagnostic per extrusion entity without gating structural viewer comments.
- Precondition: Step 2's object transition placement is stable and Step 1 exposes `gcode_comments`.
- Postcondition: AC-4 passes bidirectionally and no travel-only or zero-extrusion entity receives the diagnostic.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - entity move loop and `orca_type_label` only.
  - `crates/slicer-gcode/tests/gcode_output_modes_tdd.rs` - packet fixture/helper only.
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-gcode/tests/gcode_output_modes_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`, structural comment helpers, Orca source.
- Blast-radius discipline: none; no public data shape changes.
- Expected sub-agent dispatches:
  - Question: summarize what canonical `gcode_comments` gates versus structural preview tags; scope: delegated `GCode.cpp::_extrude` and `GCodeWriter.cpp`; return: `FACT` <=5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - G-code emitter responsibility, targeted.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude`; delegated only.
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - verbose comment gates; delegated only.
- Verification:
  - `cargo test -p slicer-gcode --test gcode_output_modes_tdd verbose_extrusion_comments_gate 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: enabled emits exactly one named diagnostic at the asserted position; disabled output lacks it and preserves all four structural comment families.

### Step 4: Correct the internal-travel retract policy owner

- Task IDs: `[]` (wayfinder ticket 51)
- Objective: declare/read `reduce_infill_retraction` in `path-optimization-default` and build consecutive-entity travel classification using exact `RegionKey`, destination role, and complete-segment containment in `sparse_infill_area`; preserve external/perimeter-bound behavior.
- Precondition: re-derive the current `run_path_optimization` same-region behavior and how its views expose infill presence; if the view cannot prove sparse infill, stop and record `[BLOCK]` rather than substituting region sameness alone.
- Postcondition: AC-6 passes both values and all four rejecting premises; true suppresses retract/unretract/Z-hop only for a qualifying contained non-perimeter travel, while false emits the matched sequence.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/path-optimization-default/src/lib.rs` - module state, `from_config`, and retract policy in `run_path_optimization` only.
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs` - fixture and three policy tests only.
  - SDK `PerimeterRegionView` definition located by symbol - only methods proving sparse infill/perimeter destination classification.
- Files allowed to edit (at most 3):
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (serialization only), WIT/IR schemas, Orca source.
- Blast-radius discipline: adding a private module field requires updating only its in-file constructor/default literals; re-derive before editing. No public struct or schema constant.
- Expected sub-agent dispatches:
  - Question: can `PerimeterRegionView` prove sparse-infill presence without a WIT change, and where?; scope: SDK view plus current WIT record only; return: `FACT` <=5 lines.
  - Question: summarize canonical qualifying/rejecting conditions; scope: delegated `GCode.cpp::needs_retraction`; return: `SUMMARY` <=120 words.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` - Layer PathOptimization/GCodeEmit ownership, targeted.
  - `docs/08_coordinate_system.md` - mm/internal-unit conversion checklist, targeted.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::needs_retraction`, delegated only.
- Verification:
  - `cargo test -p path-optimization-default --test travel_policy_tdd reduce_infill_retraction 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT exit code; rebuild without `--check` if stale before trusting failures.
- Exit condition: the test proves exact false/true, different-region, perimeter-destination, empty-area, and segment-escapes-area cases; no emitter filtering exists; guest freshness exits 0. If full segment containment is not expressible with existing view/helper APIs, Step 4 stops blocked rather than weakening AC-6.

### Step 5: Prove the real runtime flavor/config path

- Task IDs: `[]` (wayfinder ticket 51)
- Objective: add an aggregated runtime integration case proving non-default Klipper config reaches both object marker and pressure-advance decisions and differs from Marlin.
- Precondition: Steps 1-3 pass; locate an existing integration helper that runs the real pipeline and allows config overrides.
- Postcondition: AC-5 passes through `run_slice`, not a direct helper-only test; exact flavor spellings remain in the CONFIG_BLOCK as an incidental existing behavior, not acceptance evidence.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/main.rs` - module registration only.
  - Existing integration flavor/pressure test helpers located by symbol - fixture setup only.
  - `crates/slicer-runtime/src/run.rs` - flavor resolution and emitter construction only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/slicer-runtime/tests/integration/gcode_output_modes_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/run.rs` unless grounding proves a retained key is not delivered; CONFIG_BLOCK assertions; CLI output routing.
- Blast-radius discipline: test-only module registration; no schema/struct change.
- Expected sub-agent dispatches:
  - Question: identify a working real-pipeline integration helper and its minimum fixture; scope: runtime integration tests; return: `LOCATIONS` <=20.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` - flavor resolution, targeted.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - flavor pressure syntax, delegated only.
- Verification:
  - `cargo test -p slicer-runtime --test integration gcode_output_modes_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: one real-pipeline test proves Klipper and Marlin produce the exact paired syntax differences and invalid values are already rejected by Step 1.

### Step 6: Regenerate docs and record queue dispositions

- Task IDs: `[]` (wayfinder ticket 51)
- Objective: regenerate config docs; annotate 04/05 with the `filename_format`/retraction owner corrections and `support_object_skip_flush` sequencing decision; run closure gates without changing ticket 51's header.
- Precondition: Steps 1-5 pass and guest artifacts are fresh.
- Postcondition: generated docs include retained declarations; queue assets accurately state five retained P44 keys, returned `filename_format`, corrected retraction owner, and sequenced support rider; all gates pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - Others/G-code output table only.
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P41/P44/P84 rows only.
  - `docs/15_config_keys_reference.md` - grep probes only; never full read.
- Files allowed to edit (at most 3 per atomic sub-change):
  - `docs/15_config_keys_reference.md` - generated only via xtask.
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - Ticket 51 header, ticket 48 history, `ORCA_CONFIG_PADDING`, production code.
- Blast-radius discipline: none; documentation/generated-output step.
- Expected sub-agent dispatches:
  - Question: re-derive current P41/P44/P84 rows immediately before editing; scope: 04/05 targeted sections; return: `SNIPPETS` <=3 snippets.
  - Question: run every gate and return only pass/fail; scope: commands below; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` Authoring rules 1-6 and Notes, targeted.
- OrcaSlicer refs: none; prior steps own canonical evidence.
- Verification:
  - `cargo xtask gen-config-docs --check` - FACT exit code.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT exit code.
  - Full matrix in `requirements.md` - FACT per command.
- Exit condition: all ACs/gates pass, 04/05 dispositions are explicit, ticket 51's header is untouched, and `git diff --stat -- crates/slicer-gcode/src/serialize.rs` is empty.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Manifest, typed fields, blast-radius inventory, rejection tests |
| Step 2 | M | Object-run state machine and dialect matrix |
| Step 3 | S | One verbose diagnostic gate |
| Step 4 | M | Existing module-owned retract policy and guest rebuild |
| Step 5 | M | Real runtime integration proof |
| Step 6 | S | Generated docs, queue annotations, gates |

Aggregate: `M`; no step is `L`, so no split is required before activation.

## Packet Completion Gate

- All steps and falsifying exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is N/A for this queue packet (`task_ids: []`); re-derive whether a task mapping was added before closure.
- `filename_format` and `support_object_skip_flush` remain queued and undeclared; five retained keys have tested non-default decisions.
- `packet.spec.md` remains draft until explicit activation and becomes `implemented` only after the acceptance ceremony.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm guest freshness by exit code and inspect `target/test-output.log` for any failure rather than rerunning.
- Record remaining risks: object-level rather than instance-level cancellation, Klipper definitions without polygon metadata, and deferred Bambu M624 flush labeling.
- Confirm context stayed within the standard band; no extended-band escalation is permitted for this packet.

All cargo check/clippy commands use `--all-targets`; each cargo test tees combined output to `target/test-output.log`.
