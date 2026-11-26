# Implementation Plan: 305-xy-size-compensation-slice-prepass

## Execution Rules

- Work one atomic step at a time; this packet has no `docs/07` task IDs — its backlog identity is wayfinder ticket 95 (P88), recorded in `packet.spec.md` `backlog_source`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Port `_shrink_contour_holes` and pin the offset signs

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: land the single-expolygon kernel — contour offset, hole offset, difference, union, and the drop-on-annihilation rule — as an ungated `slicer-core` function with tests, before any control flow exists.
- Precondition: `crates/slicer-core/src/algos/xy_size_compensation.rs` does not exist; `rg xy_size_compensation crates modules xtask` returns nothing.
- Postcondition: `cargo test -p slicer-core --test algo_xy_size_compensation_tdd` compiles and reports a non-zero test count with AC-1, AC-2, AC-3 and AC-6 green; `crates/slicer-core/src/algos/mod.rs` declares `pub mod xy_size_compensation;` with no `cfg` attribute.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mod.rs` - whole file (~26 lines)
  - `crates/slicer-core/src/polygon_ops.rs` - the `offset`, `union_ex`, `difference_ex` and `OffsetJoinType` definitions only (long file)
  - `crates/slicer-ir/src/slice_ir.rs` - the `Point2`, `Polygon` and `ExPolygon` definitions and `Point2::from_mm` only (very long file)
  - `docs/ORCASLICER_ATTRIBUTION.md` - the "Standard Porting Header" block only
  - `docs/08_coordinate_system.md` - whole file
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/xy_size_compensation.rs`
  - `crates/slicer-core/src/algos/mod.rs`
  - `crates/slicer-core/tests/algo_xy_size_compensation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-scheduler/**`, `crates/slicer-schema/**`, `crates/slicer-gcode/**`, `modules/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline: not applicable — no struct field is added to an existing type and no schema or version constant moves.
- Expected sub-agent dispatches:
  - Question: Return `PrintObject::_shrink_contour_holes` verbatim; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SNIPPETS` (1 snippet, 30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct read; note that `polygon_ops::offset` takes millimetres, so canonical's `scaled<>()` call is **not** ported
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read of the porting-header block
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` (`PrintObject::_shrink_contour_holes`) - delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_xy_size_compensation_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"` - FACT pass/fail plus the test count
  - `rg -q '^pub mod xy_size_compensation;' crates/slicer-core/src/algos/mod.rs; echo "exit=$?"` - FACT pass/fail
- Exit condition: AC-1, AC-2, AC-3 and AC-6 pass, the run reports at least four tests, and the module declaration carries no `cfg` attribute. If the run prints `ok` with `0 passed`, the file or the target is gated — stop and remove the gate before proceeding. **Do not proceed while AC-2 is red or absent**: an inverted hole sign is invisible downstream.

### Step 2: Port the two-pass control flow and both region branches

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: land `XySizeCompensationParams` and `compensate_object_layer` over one object's layer, reproducing canonical's zero early-out, its positive-then-negative two-pass order, its single-printable-region branch, its multi-region merge-and-re-split with region priority, and its multi-region merged-trimming branch — with modifier footprints excluded throughout.
- Precondition: Step 1 complete and green; `shrink_contour_holes` exists and is exercised by tests.
- Postcondition: AC-4, AC-5, AC-7, AC-8, AC-N1 and AC-N2 green in the same test binary; `compensate_object_layer` returns `false` and mutates nothing when both deltas are `0.0`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/xy_size_compensation.rs` - whole file as written in Step 1
  - `crates/slicer-core/src/polygon_ops.rs` - the `intersection_ex`, `difference_ex`, `union_ex` and `expolygon_area` definitions only (long file)
  - `crates/slicer-ir/src/slice_ir.rs` - two widely separated ranges, not one: `RegionId` is a bare type alias (`pub type RegionId = u64;`) near the top of the file, and `MODIFIER_FOOTPRINT_REGION_ID` with its doc comment sits roughly 1800 lines later beside the slice types. Read both ranges; grepping for the symbol beats guessing an offset (very long file)
- Files allowed to edit (at most 2):
  - `crates/slicer-core/src/algos/xy_size_compensation.rs`
  - `crates/slicer-core/tests/algo_xy_size_compensation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-scheduler/**`, `modules/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline: not applicable — no existing type gains a field.
- Expected sub-agent dispatches:
  - Question: Return the single-region branch of the compensation block in `PrintObject::slice_volumes`, excluding the elephant-foot arm; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SNIPPETS` (1 snippet, 30 lines)
  - Question: Return the multi-region branch of the same block — the `max_growth` merge-and-re-split and the `min_growth` trimming path; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`; return: `SNIPPETS` (1 snippet, 30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct read; the merge epsilon canonical spells `SCALED_EPSILON` must be re-expressed in this tree's units and declared file-private, never imported (the tree's `SCALED_EPSILON` is a file-private `i128` in `crates/slicer-core/src/smooth_outward.rs` and is not reusable)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` (the compensation block inside `PrintObject::slice_volumes`) - delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_xy_size_compensation_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"` - FACT pass/fail plus the test count
  - `cargo clippy -p slicer-core --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: all of AC-4, AC-5, AC-7, AC-8, AC-N1, AC-N2 pass; the positive and negative passes are two distinct calls with the expolygon set re-derived between them; no `rayon` import appears anywhere in the file.

### Step 3: Register the host-only stage

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: add `PrePass::XySizeCompensation` to `STAGE_ORDER` between `PrePass::Slice` and `PrePass::OverhangAnnotation`, classify it host-only, and declare its blackboard prerequisites — with no code yet registered against it.
- Precondition: Step 2 complete; `rg -q 'XySizeCompensation' crates` returns nothing.
- Postcondition: AC-10 green; the stage appears exactly once in `STAGE_ORDER`, once in `HOST_ONLY_STAGES`, and nowhere in `slicer_schema::VALID_STAGES` or `slicer_schema::STAGES`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` - the `STAGE_ORDER` constant only (long file)
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` - whole file
  - `crates/slicer-runtime/src/prepass.rs` - the `required_slots` function only (long file)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/execution_plan.rs`
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`
  - `crates/slicer-runtime/src/prepass.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/**` (the stage is host-only; touching `VALID_STAGES` or `STAGES` means the design is wrong), `crates/slicer-macros/**`, `crates/slicer-ir/src/stage_io.rs`, `crates/pnp-cli/src/module_new.rs`
- Blast-radius discipline: adding a `STAGE_ORDER` entry is the widest edit in this packet. Before editing, enumerate every consumer of `STAGE_ORDER` and every test that asserts a stage count or a containment relation, and record which ones must change and which must hold unchanged. Two are known from packet 304's analysis — the DAG-CLI containment assertion and the built-in producer count — and both should hold, because this built-in mints no `Producer`. Prove that; do not assume it.
- Expected sub-agent dispatches:
  - Question: List every file that references `STAGE_ORDER` or asserts a per-stage count; scope: `crates/`; return: `LOCATIONS` (≤ 20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegate a SUMMARY of the "Fixed Stage Order" section; do not load the file
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (`PrintObject::slice`) - delegate; confirms the pass sits before `_transform_hole_to_polyholes` and after `apply_conical_overhang`
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_contract stage_list_consistency_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"` - FACT pass/fail
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-10 passes, `cargo check --workspace --all-targets` is clean, and the blast-radius enumeration is recorded with each assertion marked changed or unchanged-and-verified.

### Step 4: Plumb the two config keys through `ResolvedConfig`

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: declare `xy_contour_compensation` and `xy_hole_compensation` as real `ResolvedConfig` fields at canonical default `0.0`, unbounded, so CLI, per-object overlay and per-region resolution all carry them.
- Precondition: Step 3 complete.
- Postcondition: AC-9 and AC-N3 green; `cargo xtask gen-config-docs --check` passes after regeneration; no `min` or `max` is declared for either key anywhere.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - the `cli` macro rows for two neighbouring plain `f32` fields and their `to_config_map` arms only (very long file)
  - `docs/21_data_defaults_and_fixtures.md` - the struct-literal rule and waiver format
- Files allowed to edit (at most 2):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-ir/tests/resolved_config_xy_size_compensation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (rule-2 padding twins; rides ticket 132), `modules/**`
- Blast-radius discipline: adding two fields to `ResolvedConfig` may break exhaustive struct literals in test code. Run `cargo xtask check-literals --report` before and after; every new violation must be fixed with a `..` rest or an `// exhaustive: <reason>` waiver, never by weakening an assertion.
- Expected sub-agent dispatches:
  - Question: Confirm the `coFloat` type, the `0` default, and the absence of `min` and `max` for both `xy_*_compensation` declarations; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (≤ 5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read; the `check-literals` gate fires on this edit
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; type and default only
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_xy_size_compensation_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"` - FACT pass/fail
  - `cargo xtask check-literals 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-9 and AC-N3 pass, `check-literals` exits 0, and the regenerated `docs/15_config_keys_reference.md` carries both keys. If a "Deviations from OrcaSlicer" row appears for either key, that is a real finding — report it rather than suppressing it; both defaults are `0.0` and should match.

### Step 5: Land the producer and register the built-in

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: resolve the per-object pair through `RegionMapIR::config_for`, group each layer's regions by `object_id`, call the kernel, write back with `replace_slice_ir`, and register the built-in between the `PrePass::Slice` and `PrePass::OverhangAnnotation` calls.
- Precondition: Steps 2, 3 and 4 complete and green.
- Postcondition: AC-11, AC-12 and AC-13 green; the built-in returns without calling `replace_slice_ir` when every object resolves both deltas to `0.0`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - the `commit_shell_classification_builtin` body only (long file)
  - `crates/slicer-runtime/src/prepass.rs` - the `run_builtin_stage` signature and the `PrePass::Slice` / `PrePass::OverhangAnnotation` registrations only (long file)
  - `crates/slicer-runtime/src/blackboard.rs` - `slice_ir`, `region_map` and `replace_slice_ir` only (long file)
  - `crates/slicer-ir/src/slice_ir.rs` - `RegionKey`, `RegionMapIR::config_for`, `SliceIR`, `SlicedRegion` only (very long file)
  - `crates/slicer-runtime/tests/executor/prepass_executor_tdd.rs` - the blackboard fixture construction only (long file)
- Files allowed to edit (at most 5 — the aggregator `mod` line is a fifth file, counted here rather than hidden inside item 4):
  - `crates/slicer-runtime/src/builtins/xy_size_compensation_producer.rs`
  - `crates/slicer-runtime/src/builtins/mod.rs`
  - `crates/slicer-runtime/src/prepass.rs`
  - `crates/slicer-runtime/tests/executor/prepass_xy_size_compensation_stage_order_tdd.rs` (plus the one `mod` line in `crates/slicer-runtime/tests/executor/main.rs`, which is part of this file's landing and must not be deferred)
- Files explicitly out of bounds:
  - `crates/slicer-core/**` (the kernel is frozen after Step 2), `crates/slicer-schema/**`, `modules/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline: adding a `PrepassExecutionError` variant touches an exhaustive match if any consumer matches on it. Enumerate those consumers before editing and record whether each needed a new arm.
- Expected sub-agent dispatches:
  - Question: Which readers of `Blackboard::slice_ir` are registered before `PrePass::OverhangAnnotation`?; scope: `crates/slicer-runtime/src/`; return: `LOCATIONS` (≤ 20 entries); purpose: resolve the first `[FWD]` in `design.md` §Open Questions
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegate a SUMMARY of the `SliceIR` / `SlicedRegion` section
  - `docs/04_host_scheduler.md` - delegate a SUMMARY of the prepass execution section
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` - delegate; confirm both keys are `PrintObjectConfig` members, which is what makes the gate per object rather than per region
- Verification:
  - `cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"` - FACT pass/fail plus the test count
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-11, AC-12 and AC-13 pass with a non-zero test count (a zero count means the `mod` line is missing), and both `[FWD]` questions in `design.md` §Open Questions are answered in writing — either resolved or promoted to a deviation clause for Step 6.

### Step 6: Docs and the deviation row

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: add the new stage to both stage lists, regenerate the config-key reference, and file one `DEVIATION_LOG.md` row with the four clauses named in `design.md` §Data and Contract Notes.
- Precondition: Step 5 complete and green.
- Postcondition: AC-16 green; `cargo xtask check-deviations --check` passes; the new row's ID was re-derived at write time, not copied from this packet.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - the last few rows only, to derive the next free ID (long file)
  - `docs/04_host_scheduler.md` - the "Fixed Stage Order" section only (long file)
  - `docs/01_system_architecture.md` - the numbered stage list and the Data Dependency Matrix only (long file)
- Files allowed to edit (at most 4):
  - `docs/04_host_scheduler.md`
  - `docs/01_system_architecture.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md` (generator output only — via `cargo xtask gen-config-docs`, never hand-edited)
- Files explicitly out of bounds:
  - Every other `docs/` file; `docs/07_implementation_status.md` gains no task row, and its Open Deviation Map section is generator output
- Blast-radius discipline: the deviation ID is a ledger fact. Re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` **and** `docs/spec_packets/*/` at the moment you write it; packets 303 and 304 each also intend to file one and either may have landed first.
- Expected sub-agent dispatches:
  - Question: Return the current highest `DEV-###` across `docs/DEVIATION_LOG.md` and `docs/spec_packets/`; scope: repo; return: `FACT` (≤ 3 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md`, `docs/01_system_architecture.md` - delegate SUMMARY reads, edit only the named sections
- OrcaSlicer refs: none (citations by file + function only)
- Verification:
  - `rg -q 'PrePass::XySizeCompensation' docs/04_host_scheduler.md && rg -q 'PrePass::XySizeCompensation' docs/01_system_architecture.md && rg -q 'xy_contour_compensation' docs/15_config_keys_reference.md; echo "exit=$?"` - FACT pass/fail
  - `cargo xtask check-deviations --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: AC-16 passes, `check-deviations --check` is clean, and the deviation row carries all four clauses.

### Step 7: Annotate the wayfinder assets

- Task IDs: none (wayfinder ticket 95 / P88)
- Objective: record the owner correction and the packet linkage in the map's assets, so the tier table stops asserting an owner this packet disproved.
- Precondition: Step 6 complete.
- Postcondition: AC-15 green; both P88 rows in the tier table name the host built-in, and the P88 heading in the packet list links this packet.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - the two `xy_*_compensation` rows only (long file)
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - the P88 entry only (long file)
- Files allowed to edit (at most 2):
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `docs/specs/orca-feature-gap/map.md` (the map is the wayfinder session's to edit, not the implementer's), every ticket file under `docs/specs/orca-feature-gap/issues/` other than the two assets above
- Blast-radius discipline: not applicable — two table rows and one heading.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` §Notes - read-only; the authoring rules that bind the disposition table
- OrcaSlicer refs: none
- Verification:
  - `rg -qi 'declaration-only keys: 0' docs/spec_packets/305-xy-size-compensation-slice-prepass/requirements.md && rg -qi 'queue count unchanged: 409' docs/spec_packets/305-xy-size-compensation-slice-prepass/requirements.md; echo "exit=$?"` - FACT pass/fail
  - `rg -q '305-xy-size-compensation-slice-prepass' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md; echo "exit=$?"` - FACT pass/fail
- Exit condition: AC-15 passes and neither asset still names a "contour-compensation module" as the owner.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One delegated snippet; three small files. The sign ACs live here |
| Step 2 | M | The largest step: two delegated snippets and the whole two-branch control flow |
| Step 3 | S | Three small edits plus the `STAGE_ORDER` blast-radius sweep |
| Step 4 | S | Two macro rows; the struct-literal sweep is the only variable cost |
| Step 5 | M | Producer plus registration; four long files read in narrow ranges |
| Step 6 | S | Two doc sections, one generator run, one ledger row |
| Step 7 | S | Two asset annotations |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo xtask check-literals` all clean.
- `cargo xtask build-guests --check` exits `0`. This packet adds no guest; a non-zero exit is a pre-existing condition to reconcile, not a result of this work — prove that by reproducing it with the change stashed.
- `docs/07_implementation_status.md` gains **no task row**: this packet carries no `TASK-###` and is tracked by wayfinder ticket 95 instead. Do not invent one. Its Open Deviation Map section *is* regenerated by Step 6's `cargo xtask check-deviations` run; that is generator output, not a backlog edit, and must never be hand-written.
- No reopened or superseded packet to reconcile.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk — in particular whether packets 297, 303 or 304 landed in the meantime. 297 and 304 change this built-in's neighbours in `prepass.rs`; 303 does not, but if its open seam question resolved toward a prepass elephant-foot, confirm it registered **after** this one.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
