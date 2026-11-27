# Implementation Plan: 304-polyhole-slice-prepass

## Execution Rules

- Work one atomic step at a time; this packet has no `docs/07` task IDs — its backlog identity is wayfinder ticket 94 (P87), recorded in `packet.spec.md` `backlog_source`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Port `create_polyholes` and pin its geometry

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: land the polygon constructor — edge count, circumscribed radius, twist set, canonical output interleave, clockwise winding — as an ungated `slicer-core` kernel function with tests, before any detection logic exists.
- Precondition: `crates/slicer-core/src/algos/polyhole.rs` does not exist; `rg polyhole crates modules xtask` returns nothing.
- Postcondition: `cargo test -p slicer-core --test algo_polyhole_tdd` compiles and reports a non-zero test count with AC-1 and AC-2 green; `crates/slicer-core/src/algos/mod.rs` declares `pub mod polyhole;` with no `cfg` attribute.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mod.rs` - whole file (~26 lines)
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - the module header and its file-private epsilon declaration only (long file)
  - `crates/slicer-ir/src/slice_ir.rs` - the `Point2` and `Polygon` definitions and `Point2::from_mm` only (very long file)
  - `docs/ORCASLICER_ATTRIBUTION.md` - the "Standard Porting Header" block only
  - `docs/08_coordinate_system.md` - whole file
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/polyhole.rs`
  - `crates/slicer-core/src/algos/mod.rs`
  - `crates/slicer-core/tests/algo_polyhole_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-scheduler/**`, `crates/slicer-schema/**`, `modules/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline: not applicable — no struct field is added to an existing type and no schema or version constant moves.
- Expected sub-agent dispatches:
  - Question: Return the free function `create_polyholes` verbatim; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SNIPPETS` (1 snippet, 30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct read; every canonical constant divides by 100
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read of the porting-header block
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (`create_polyholes`) - delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_polyhole_tdd 2>&1 | tee target/test-output.log | rg 'test result'` - FACT pass/fail plus the test count
  - `rg -q '^pub mod polyhole;' crates/slicer-core/src/algos/mod.rs; echo "exit=$?"` - FACT pass/fail
- Exit condition: AC-1 and AC-2 pass, the run reports at least two tests, and the module declaration carries no `cfg` attribute. If the run prints `ok` with `0 passed`, the file or the target is gated — stop and remove the gate before proceeding.

### Step 2: Port detection, grouping and replacement

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: land `PolyholeParams` and `transform_holes_to_polyholes` over `&mut [SliceIR]`, reproducing canonical's candidacy test, radius statistics, both spread tests, per-region timelines, contiguous upward walk, group-keep rule, and per-layer rotation indexing.
- Precondition: Step 1 complete — `create_polyholes` exists and is green.
- Postcondition: AC-3 through AC-7 and AC-N1 through AC-N4 are green; `ExPolygon.contour` is provably untouched by the pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/polyhole.rs` - whole file (as written in Step 1)
  - `crates/slicer-ir/src/slice_ir.rs` - the `ExPolygon`, `SlicedRegion`, `SliceIR` definitions only (very long file)
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - `build_region_timelines` only (long file) - the per-`(object_id, region_id)` timeline shape to mirror inside the kernel
  - `crates/slicer-ir/src/resolved_config.rs` - the `ResolvedFloatOrPercent` definition only (~3000 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/polyhole.rs`
  - `crates/slicer-core/tests/algo_polyhole_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` except the single read above; `crates/slicer-scheduler/**`; `modules/**`; `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline: not applicable — `PolyholeParams` is a new type with no existing literal sites.
- Expected sub-agent dispatches:
  - Question: In `PrintObject::_transform_hole_to_polyholes`, return the candidacy test, the centroid/radius statistics and the two spread tests verbatim; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SNIPPETS` (<=2 snippets, 30 lines each)
  - Question: In `PrintObject::_transform_hole_to_polyholes`, return the vertical-grouping loop verbatim — the contiguity break, the match predicate, the consume, and the keep-group condition; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SNIPPETS` (1 snippet, 30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct read; the threshold's mm-to-unit conversion
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (`PrintObject::_transform_hole_to_polyholes`) - delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_polyhole_tdd 2>&1 | tee target/test-output.log | rg 'test result'` - FACT pass/fail plus the test count
  - `cargo clippy -p slicer-core --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
- Exit condition: AC-3 through AC-7 and AC-N1 through AC-N4 pass. If AC-N1 fails only for the reflex-vertex case, the convexity predicate's sign is written for a counter-clockwise ring — invert it for the clockwise hole winding rather than loosening the assertion.

### Step 3: Register the host-only stage

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: add `PrePass::PolyholeTransform` to `STAGE_ORDER` between `PrePass::Slice` and `PrePass::OverhangAnnotation`, classify it host-only, and declare its blackboard prerequisites — with no code yet registered against it.
- Precondition: Step 2 complete; `rg -q 'PolyholeTransform' crates` returns nothing.
- Postcondition: AC-9 green; `cargo check --workspace --all-targets` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` - the `STAGE_ORDER` constant only (long file)
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` - whole file (~90 lines)
  - `crates/slicer-runtime/src/prepass.rs` - the `required_slots` function only (long file)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/execution_plan.rs`
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`
  - `crates/slicer-runtime/src/prepass.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/**` and `crates/slicer-macros/**` - the stage is host-only; an edit here means it was accidentally made module-targetable
  - `crates/slicer-ir/src/stage_io.rs`, `crates/slicer-wasm-host/src/dispatch.rs`, `crates/pnp-cli/src/module_new.rs` - no new IR commit type and no scaffold arm
- Blast-radius discipline: mandatory here. Adding a `STAGE_ORDER` entry without a matching `HOST_ONLY_STAGES` entry fails `host_only_stages_partition_stage_order_into_valid_stages`; both edits land in this step. Two neighbouring assertions were checked at authoring and need **no** edit under this design: `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` asserts *containment* of six built-in stage ids rather than equality, and `crates/slicer-runtime/tests/unit/builtin_producers_tdd.rs` asserts a producer count of 7, which holds because this stage registers via `run_builtin_stage` and mints no `Producer`. The verification below re-runs both so the claim is proven, not assumed.
- Expected sub-agent dispatches:
  - none — the two neighbouring assertions were resolved at authoring and are re-proven by this step's verification commands.
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY of the "Fixed Stage Order" section (edited in Step 6, read here for the classification convention)
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo test -p slicer-scheduler --test contract stage_list_consistency_tdd 2>&1 | tee target/test-output.log | rg 'test result'` - FACT pass/fail
  - `cargo test -p slicer-runtime --test unit builtin_producers 2>&1 | tee target/test-output.log | rg 'test result'` - FACT pass/fail; proves the producer count is undisturbed
  - `cargo check --workspace --all-targets 2>&1 | tail -3` - FACT pass/fail
- Exit condition: AC-9 passes, the producer-count test is still green, and the workspace check is clean. If the partition test names the new stage as "missing from VALID_STAGES", the `HOST_ONLY_STAGES` entry was not added — add it rather than adding the stage to `VALID_STAGES`.

### Step 4: Plumb the four config keys through `ResolvedConfig`

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: declare `hole_to_polyhole`, `hole_to_polyhole_threshold`, `hole_to_polyhole_twisted` and `hole_to_polyhole_max_edges` as real `ResolvedConfig` fields at canonical defaults so CLI, overlay and per-region resolution all carry them.
- Precondition: Step 3 complete.
- Postcondition: AC-8 green; the four keys round-trip through `to_config_map`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - one existing `cli` row per extractor (`extract_bool`, `extract_float_or_percent`, `extract_int_as_u32`) and the `ResolvedFloatOrPercent` definition only (~3000 lines) - locate by grep, never read top to bottom
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-ir/tests/resolved_config_polyhole_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` - `ORCA_CONFIG_PADDING` is rule 2 non-evidence and rides ticket 132
  - `modules/**` - no manifest declares these keys
- Blast-radius discipline: mandatory check. Adding fields to `ResolvedConfig` can break exhaustive struct literals. Before editing, dispatch a `LOCATIONS` sweep for `ResolvedConfig {` literal sites without a `..` rest and add any that appear to this step's edit list. The `cargo xtask check-literals` gate below is the backstop, not the plan.
- Expected sub-agent dispatches:
  - Question: List every `ResolvedConfig {` struct-literal site in the workspace that does not use a `..` rest; scope: `crates/**`, `modules/**`, `xtask/**`; return: `LOCATIONS` (<=20 entries)
  - Question: Confirm the coBool/coFloatOrPercent/coInt declarations and defaults of the four `hole_to_polyhole*` keys; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of the struct-literal churn gate rule
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_polyhole_tdd 2>&1 | tee target/test-output.log | rg 'test result'` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: AC-8 passes and `check-literals` exits 0. If an empty raw source yields anything other than `false` / `{ value: 0.01, is_percent: false }` / `true` / `50`, the defaults are wrong — fix them here, not in the producer.

### Step 5: Land the producer and register the built-in

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: resolve per-region params through `RegionMapIR::config_for`, call the kernel, write back with `replace_slice_ir`, and register the built-in between the `PrePass::Slice` and `PrePass::OverhangAnnotation` calls.
- Precondition: Steps 1-4 complete; the kernel is green and the stage exists.
- Postcondition: AC-10 and AC-11 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - `commit_shell_classification_builtin` and the helpers `build_region_timelines`, `find_region_mut`, `clone_region_polys` only (long file)
  - `crates/slicer-runtime/src/prepass.rs` - the `run_builtin_stage` definition, the `PrepassExecutionError` enum, and the calls around `PrePass::Slice` / `PrePass::OverhangAnnotation` only (long file)
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - the `config.extensions.get("nozzle_diameter")` read only
  - `crates/slicer-runtime/src/blackboard.rs` - `replace_slice_ir` only (long file)
  - `crates/slicer-ir/src/slice_ir.rs` - `RegionKey`, `RegionPlan`, `RegionMapIR::config_for` only (very long file)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/polyhole_producer.rs`
  - `crates/slicer-runtime/src/prepass.rs`
  - `crates/slicer-runtime/tests/executor/prepass_polyhole_stage_order_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/polyhole.rs` - frozen after Step 2; a producer that needs a kernel change means the kernel signature was wrong, so reopen Step 2 deliberately rather than editing both at once
  - `crates/slicer-schema/**`, `crates/slicer-macros/**`, `modules/**`
- Blast-radius discipline: `crates/slicer-runtime/src/builtins/mod.rs` (one `pub mod` line) and `crates/slicer-runtime/tests/executor/main.rs` (one `mod` line) are mechanical companions of the two new files and land with them; count them against this step's budget.
- Expected sub-agent dispatches:
  - Question: Where exactly is `_transform_hole_to_polyholes` called from `PrintObject::slice`, and what runs immediately before and after it?; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SUMMARY` (<=200 words)
  - Question: Which `ResolvedConfig` field or `extensions` entry carries the outer-wall filament id for a region in this tree?; scope: `crates/slicer-runtime/src/run.rs`, `crates/slicer-ir/src/resolved_config.rs`; return: `FACT` (<=5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY of the prepass built-in execution contract
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` (`PrintObject::slice`) - delegate; never load
- Verification:
  - `cargo test -p slicer-runtime --test executor prepass_polyhole 2>&1 | tee target/test-output.log | rg 'test result'` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
- Exit condition: AC-10 and AC-11 pass. If AC-11 fails because both regions were converted, the producer resolved one global config instead of calling `config_for` per `RegionKey` — fix the resolution, do not relax the test.

### Step 6: Docs and the deviation row

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: add the new stage to both stage lists, make the deviation discoverable, and file one `DEVIATION_LOG.md` row with the three clauses named in `design.md` §Data and Contract Notes.
- Precondition: Steps 1-5 complete and green.
- Postcondition: AC-14 green; `cargo xtask check-deviations` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/04_host_scheduler.md` - the "Fixed Stage Order" section only (over 300 lines)
  - `docs/01_system_architecture.md` - the numbered stage list and the Data Dependency Matrix only (over 300 lines)
  - `docs/DEVIATION_LOG.md` - the table header and the last two rows only (very long file)
- Files allowed to edit (at most 3 hand-edited):
  - `docs/04_host_scheduler.md`
  - `docs/01_system_architecture.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - Every `docs/spec_packets/*/` directory other than this packet's own - never modify another packet, including 297 and 303
- Blast-radius discipline: mandatory here, and it is **generator** blast radius rather than struct-literal blast radius. `cargo xtask check-deviations` (run without `--check`) regenerates the Open Deviation Map in `docs/07_implementation_status.md` and the generated config tables in `docs/15_config_keys_reference.md` from `docs/DEVIATION_LOG.md`. Those two files are therefore modified by this step even though they are not hand-edited — run the regenerator after filing the row, commit its output with the row, and never hand-edit a generated section. AC-14's `docs/15_config_keys_reference.md` grep passes only after the regenerator has run.
- Expected sub-agent dispatches:
  - Question: Return the highest `DEV-###` present anywhere under `docs/DEVIATION_LOG.md` and `docs/spec_packets/`; scope: those paths; return: `FACT` (<=5 lines) - re-derive at write time; never trust the ID quoted in this packet
  - Question: List the exact section heading and surrounding lines of the fixed stage order in `docs/04_host_scheduler.md` and the numbered stage list plus Data Dependency Matrix in `docs/01_system_architecture.md`; scope: those two files; return: `LOCATIONS` (<=20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md`, `docs/01_system_architecture.md` - delegated SUMMARY, edit only the located sections
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `rg -q 'PrePass::PolyholeTransform' docs/04_host_scheduler.md && rg -q 'PrePass::PolyholeTransform' docs/01_system_architecture.md && rg -q 'hole_to_polyhole' docs/15_config_keys_reference.md; echo "exit=$?"` - FACT pass/fail
  - `cargo xtask check-deviations` - FACT pass/fail
- Exit condition: AC-14 passes and the deviation gate is clean. `docs/15_config_keys_reference.md` §Deviations from OrcaSlicer is the section the new row's "Affected section" field points at; if the grep fails there, the row's affected-section field was not written.

### Step 7: Annotate the wayfinder assets

- Task IDs: none (wayfinder ticket 94 / P87)
- Objective: record the owner correction and the packet linkage in the map's assets, so the tier table stops asserting an owner this packet disproved.
- Precondition: Steps 1-6 complete and green.
- Postcondition: AC-13 green; the three P87 rows in the tier table name the host built-in, and the P87 heading in the packet list links this packet.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - the three `hole_to_polyhole*` rows and the `hole_to_polyhole_max_edges` note only (over 300 lines)
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - the P87 section only (over 300 lines)
- Files allowed to edit (at most 3):
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `docs/specs/orca-feature-gap/map.md` - the map's Decisions-so-far entry is the wayfinder session's to write, not the implementer's
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` §Notes "Authoring rules" - delegated SUMMARY; the rules the annotation must not contradict
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `rg -qi 'declaration-only keys: 0' docs/spec_packets/304-polyhole-slice-prepass/requirements.md && rg -qi 'queue count unchanged: 409' docs/spec_packets/304-polyhole-slice-prepass/requirements.md; echo "exit=$?"` - FACT pass/fail
  - `rg -c 'host:polyhole' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - FACT: expect 3
- Exit condition: the tier table's three P87 rows no longer say `new polyhole module`, and the packet list's P87 heading names `304-polyhole-slice-prepass`. Do not change the queue count anywhere — it stays 409.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One delegated snippet; three small files |
| Step 2 | M | The largest step: two delegated snippets and the whole detection/grouping port |
| Step 3 | S | Three one-line edits plus one dispatch |
| Step 4 | S | Four macro rows; the struct-literal sweep is the only variable cost |
| Step 5 | M | Producer plus registration; two long files read in narrow ranges |
| Step 6 | S | Two doc sections and one ledger row |
| Step 7 | S | Two asset annotations |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` gains **no task row**: this packet carries no `TASK-###` and is tracked by wayfinder ticket 94 instead. Do not invent one. Its Open Deviation Map section *is* regenerated by Step 6's `cargo xtask check-deviations` run; that is generator output, not a backlog edit, and must never be hand-written.
- No reopened or superseded packet to reconcile.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk — in particular whether packet 297 or 303 landed in the meantime, which would change the built-in's neighbours in `prepass.rs`.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
