# Implementation Plan: 306-interlocking-beams-slice-prepass

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Port the voxel grid and dilation kernels

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `crates/slicer-core/src/algos/interlocking/voxel.rs` as an ungated module with the grid arithmetic and the three dilation-kernel shapes, proven by unit tests, before any generator code exists.
- Precondition: `crates/slicer-core/src/algos/interlocking/` does not exist; `rg -q '^pub mod interlocking;' crates/slicer-core/src/algos/mod.rs` fails.
- Postcondition: `voxel.rs` exists with the attribution header, `pub mod interlocking;` is declared with no `cfg` in `algos/mod.rs`, and AC-1 – AC-4 and AC-21 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — the `Point2`, `Polygon`, `ExPolygon` declarations and `mm_to_units` / `units_to_mm` only
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` — the module header only, for the ungated-module and attribution-header precedent
  - `crates/slicer-core/Cargo.toml` — the `[features]` block and `[[test]]` targets, to confirm no `required-features` is added
  - `docs/ORCASLICER_ATTRIBUTION.md` — whole file, short
  - `docs/08_coordinate_system.md` — the porting checklist section only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/interlocking/voxel.rs`
  - `crates/slicer-core/src/algos/mod.rs`
  - `crates/slicer-core/tests/algo_interlocking_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` — no wiring until the kernel is correct
  - `crates/slicer-ir/src/resolved_config.rs` — Step 4 owns the keys
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: not applicable — this step adds no struct field to an existing type and bumps no schema or version constant. `DilationKernel` and `VoxelGrid` are new types with no existing literal sites.
- Expected sub-agent dispatches:
  - Question: what are `DilationKernel`'s exact `CUBE` / `DIAMOND` / `PRISM` construction loops and their bounds?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: what is the exact body of `walkLine`?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: what are the exact bodies of `_walkAreas`, `walkAreas`, `walkPolygons` and `dilate`, and what half-cell XY translation does `_walkAreas` assume of its input?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` ≤30 lines per function, one dispatch each
  - Question: what are `toGridCoord`, `toLowerCoord`, `toLowerCorner` and `toPolygon` verbatim?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.hpp`; return: `SNIPPETS` ≤30 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — direct ranged read of the porting checklist
  - `docs/ORCASLICER_ATTRIBUTION.md` — direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp` — delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.hpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail; bounded failure SNIPPETS
  - `rg -q '^pub mod interlocking;' crates/slicer-core/src/algos/mod.rs && ! rg -q 'algo_interlocking_tdd' crates/slicer-core/Cargo.toml; echo "exit=$?"` — FACT exit code, proving the module and its test target are ungated
- Exit condition: AC-1 – AC-4 and AC-21 pass, and a **bare** `cargo test -p slicer-core --test algo_interlocking_tdd` (no `--features`) reports a non-zero passing count. A zero count means the module was gated and the run was blind.

### Step 2: Port the generator core and its enabling gate

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `crates/slicer-core/src/algos/interlocking/mod.rs` with `InterlockingParams`, the four-condition enabling gate, and the non-air-filtering path — `get_shell_voxels`, `add_boundary_cells`, `compute_unioned_volume_regions`, `generate_microstructure`, `apply_microstructure_to_outlines`, plus the local rotation helpers.
- Precondition: Step 1's exit condition holds; `crates/slicer-core/src/algos/interlocking/mod.rs` declares only `pub mod voxel;`.
- Postcondition: `generate_interlocking_structure` interdigitates two tool-distinct regions with `boundary_avoidance_cells = 0`, and AC-5 – AC-8, AC-11, AC-12, AC-N1 – AC-N3 and AC-N5 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/polygon_ops.rs` — `union_ex`, `intersection_ex`, `difference_ex`, `xor`, `offset`, `opening_ex`, `closing_ex`, `OffsetJoinType` signatures only
  - `crates/slicer-ir/src/slice_ir.rs` — `SlicedRegion`, `SliceIR`, `PaintValue`, `MODIFIER_FOOTPRINT_REGION_ID`, `is_modifier_namespace_id` only
  - `docs/21_data_defaults_and_fixtures.md` — the struct-literal churn gate section only
  - `crates/slicer-core/src/algos/interlocking/voxel.rs` — whole file, authored in Step 1
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/interlocking/mod.rs`
  - `crates/slicer-core/tests/algo_interlocking_tdd.rs`
  - `crates/slicer-core/src/algos/interlocking/voxel.rs` (only if a walk signature must widen for the generator's callbacks)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-scheduler/**`
  - `crates/slicer-core/src/algos/paint_segmentation/**` — Step 4 owns the rename
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: `InterlockingParams` is a new `pub` struct with six named fields under `crates/*/src`, which puts it on the `cargo xtask check-literals` watchlist from this step onward. Every **test** literal of it authored here and in Steps 3, 7 and 8 must use a `..` rest or carry an `// exhaustive: <reason>` waiver; production `src/` literals stay exhaustive. Budget the gate run into this step rather than discovering it at closure. There are no pre-existing literal sites to sweep — the type is new.
- Expected sub-agent dispatches:
  - Question: what does `generateMicrostructure` compute for `middle` and `width[2]`, and how is phase 1 derived from phase 0?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: what is the exact sequence of set operations in `applyMicrostructureToOutlines`, and which structure-phase index does each layer read?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: what are `getShellVoxels`, `addBoundaryCells` and `computeUnionedVolumeRegions` verbatim, including the ghost layer and the `ignored_gap_` closing?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines per function, one dispatch each
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` — direct ranged read of the churn gate
  - `docs/08_coordinate_system.md` — direct ranged read; `ignored_gap_` divides by 100
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo xtask check-literals` — FACT exit code; the new watched struct's test literals must already comply
- Exit condition: AC-5 – AC-8, AC-11, AC-12, AC-N1 – AC-N3 and AC-N5 pass, and AC-9 / AC-10 **fail** — the air-filtering branch is Step 3's and must not be silently half-implemented here.

### Step 3: Port air filtering and thin-area handling

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `grow_border_areas_perpendicular` and `handle_thin_areas` and the `air_filtering` branch that `interlocking_boundary_avoidance` selects, so the sixth key drives a behaviour change.
- Precondition: Step 2's exit condition holds — AC-9 and AC-10 currently fail.
- Postcondition: AC-9 and AC-10 pass and every Step 2 AC still passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` — `RoleWidthContext` and `resolve_role_width` only, for the per-region outer-wall width the two functions consume
  - `crates/slicer-core/src/polygon_ops.rs` — `opening_ex`, `closing_ex`, `offset`, `intersection_ex`, `difference_ex` signatures only
  - `crates/slicer-core/src/algos/interlocking/mod.rs` — whole file, authored in Step 2
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/interlocking/mod.rs`
  - `crates/slicer-core/tests/algo_interlocking_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-ir/**`, `crates/slicer-scheduler/**`
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: not applicable — no new struct field, no schema or version constant. New `InterlockingParams` test literals in this step still owe a `..` rest or an `// exhaustive:` waiver per Step 2's gate.
- Expected sub-agent dispatches:
  - Question: what is `growBorderAreasPerpendicular` verbatim, including the loop bound `(detect / min_line) + 2`?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: what is `handleThinAreas` verbatim, including `number_of_beams_detect` / `number_of_beams_expand`, `rounding_errors`, `close_gaps` and the four-way intersection?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: how does `generateInterlockingStructure` sequence `addBoundaryCells` on the air dilation, the `has_all_meshes.erase` loop and `handleThinAreas`?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — direct ranged read; `rounding_errors = 5` is a 1 nm constant and becomes sub-unit here
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo clippy -p slicer-core --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: AC-9 and AC-10 pass, and the `boundary_avoidance_cells = 0` results from Step 2 are unchanged — the air branch must be additive, not a rewrite of the default path.

### Step 4: Rename `interlocking_beam` and declare the other five keys

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: retire the PnP-invented `mmu_segmented_region_interlocking_beam` to canonical `interlocking_beam` with no alias, preserving the Phase-5 suppression read site, and add the five remaining keys as `ResolvedConfig` `cli` fields with canonical defaults and no invented bounds.
- Precondition: Step 3's exit condition holds; `rg -c 'mmu_segmented_region_interlocking_beam' crates` returns 9 across three files.
- Postcondition: the six keys resolve with canonical defaults, the old spelling is gone from `crates/` and `modules/`, and AC-13 – AC-15 and AC-N4 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` — the MMU segmented-region `cli` block, the hand-written `PartialEq` arms for those fields, and the `to_config_map` omission comment only; located by `rg -n 'mmu_segmented_region'`
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — ±40 lines around each of the three `rg -n 'mmu_segmented_region_interlocking_beam'` hits only
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` — whole file, under 200 lines
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`
- Files explicitly out of bounds:
  - `docs/spec_packets/_OLD/96_paint-segmentation-phase5-width-limit.md` — another packet's directory; it mentions the old spelling and stays as the historical record
  - `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` — names only the two keys canonical really does prefix `mmu_segmented_region_`; the rename does not reach it
  - `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING` is map rule 2 non-evidence and carries no twin for any of the six
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: this step renames a field on `ResolvedConfig`, whose `PartialEq` is **hand-written**, not derived. The nine sites are: `crates/slicer-ir/src/resolved_config.rs` ×4 (the `cli` declaration's key string and field name, the `PartialEq` arm, the `to_config_map` omission comment), `crates/slicer-core/src/algos/paint_segmentation/mod.rs` ×3 (the `run_phase5_width_limit` read, its doc comment, the driver test's struct literal), `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` ×2 (the module doc comment and the AC-7 JSON config string) — verified by `rg` at authoring. Adding the five new `cli` fields also adds five `PartialEq` arms in the same file. `ResolvedConfig` literals elsewhere are unaffected because the type derives `Default` and every construction goes through it.
- Expected sub-agent dispatches:
  - Question: what are the `min`, `max`, `sidetext`, `ConfigOption*` type and default of each of the six `interlocking_*` `ConfigOptionDef`s?; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` ≤5 lines
  - Question: after the edits, does `mmu_segmented_region_interlocking_beam` appear anywhere under `crates/`, `modules/` or `docs/`?; scope: `crates/**`, `modules/**`, `docs/**`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the Multimaterial anchor. The doc edit itself is Step 6's.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo test -p slicer-core --features host-algos --lib interlocking_beam_true_skips_phase5_driver 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail; the rename must not change Phase-5 behaviour
  - `! rg -q 'mmu_segmented_region_interlocking_beam' crates modules; echo "exit=$?"` — FACT exit code
  - `cargo check --workspace --all-targets` — FACT pass/fail; the rename touches a test file, so a plain `cargo check` would not compile it
- Exit condition: AC-13 – AC-15 and AC-N4 pass, `rg 'mmu_segmented_region_interlocking_beam' crates modules` returns nothing, and `cargo test -p slicer-runtime --test executor cube_4color_phase5` still passes — proving the Phase-5 end-to-end behaviour survived the rename.

### Step 5: Register the stage in the scheduler

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: add `"PrePass::InterlockingBeams"` to `STAGE_ORDER` and to `HOST_ONLY_STAGES`, deliberately not to `VALID_STAGES`, and pin its positional relations in the stage-list contract test.
- Precondition: Step 4's exit condition holds; `rg -q 'PrePass::InterlockingBeams' crates` fails.
- Postcondition: AC-16 passes and the two stage-list partition tests still pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` — the `STAGE_ORDER` const only
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — whole file, under 200 lines
  - `crates/slicer-schema/src/lib.rs` — the `VALID_STAGES` const only, to confirm the stage stays out of it
  - `crates/slicer-runtime/src/prepass.rs` — the ordered list of `run_builtin_stage` stage-id string literals only (`rg -n '        "PrePass::'`), to confirm the declared slot matches the true execution slot
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/execution_plan.rs`
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/src/lib.rs` — `VALID_STAGES` must **not** gain this stage; it is host-only by design and the contract test asserts its absence
  - `crates/slicer-runtime/**` — Step 6 owns the producer
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: this step inserts a string into `STAGE_ORDER`, which `stage_list_consistency_tdd.rs` partitions against `VALID_STAGES` and `HOST_ONLY_STAGES` — a `STAGE_ORDER` entry in neither list fails `host_only_stages_partition_stage_order_into_valid_stages`. Both edits therefore land together. No struct field and no schema or version constant is touched.
- Expected sub-agent dispatches:
  - Question: which other draft packets insert a new entry into `STAGE_ORDER`, and at which position?; scope: `docs/spec_packets/*/`; return: `LOCATIONS` ≤20; purpose: whichever of packet 305 and this one merges second must re-run the contract test, because it asserts positions rather than membership
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — direct ranged read of the fixed stage-order section; the doc edit itself is Step 10's
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — delegate; confirms the stage sits after mm segmentation and before support generation
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_contract stage_list_consistency_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: AC-16 passes. The `--test scheduler_contract` target name comes from `crates/slicer-scheduler/Cargo.toml`'s `[[test]] name = "scheduler_contract"` — do not guess it from the directory name.

### Step 6: Author the host built-in producer and wire it into the prepass

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `crates/slicer-runtime/src/builtins/interlocking_producer.rs` — region pairing by material tool index, per-object config gate, per-region outer-wall width — and register it with `run_builtin_stage` immediately after the `PrePass::PaintSegmentation` block.
- Precondition: Step 5's exit condition holds.
- Postcondition: the workspace compiles with the producer registered; behaviour is **unproven** until Step 7, which is this step's falsifying validation.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/prepass.rs` — the `PrepassExecutionError` enum, its `Display` arm list, and the `PrePass::PaintSegmentation` `run_builtin_stage` block only
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `commit_shell_classification_builtin` only, as the clone-mutate-`replace_slice_ir` precedent
  - `crates/slicer-runtime/src/blackboard.rs` — `replace_slice_ir` and `BlackboardError` only
  - `crates/slicer-core/src/flow.rs` — `RoleWidthContext` and `resolve_role_width` only; the argument order is `(role, first_layer, bridge, context)`
  - `crates/slicer-ir/src/slice_ir.rs` — `RegionKey`, `RegionMapIR::config_for`, `PaintValue`, `MODIFIER_FOOTPRINT_REGION_ID`, `is_modifier_namespace_id` only
  - `crates/slicer-runtime/src/builtins/mod.rs` — whole file, under 30 lines
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/interlocking_producer.rs` (new)
  - `crates/slicer-runtime/src/builtins/mod.rs`
  - `crates/slicer-runtime/src/prepass.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/interlocking/**` — the kernel is finished; a change here means Steps 1–3 were wrong and the step should stop rather than patch
  - `crates/slicer-scheduler/**` — Step 5 owns the stage list
  - `crates/slicer-runtime/tests/**` — Step 7 owns the tests
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: this step adds a variant to `PrepassExecutionError`, whose `Display` is a hand-written exhaustive `match` in the same file — the arm lands in the same edit or the file does not compile. The enum derives `Clone + PartialEq + Eq`, so `InterlockingBuiltinError` must derive them too; the existing `PaintSegmentation` variant carries a `String` precisely because its source type does not, and that fallback is available if a derive proves impossible. No struct field is added to an existing type and no schema or version constant is bumped.
- Expected sub-agent dispatches:
  - Question: which test/non-test files construct a `RoleWidthContext` literal today, and with which field-init idiom?; scope: `crates/**/*.rs`; return: `LOCATIONS` ≤20; purpose: match the existing construction idiom rather than invent a second
  - Question: does `InterlockingBuiltinError`'s chosen shape satisfy `PrepassExecutionError`'s `Clone + PartialEq + Eq` derives?; scope: `crates/slicer-runtime/src/prepass.rs`; return: `FACT` ≤5 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` — direct ranged read of the prepass stage list; the doc edit itself is Step 10's
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp` — delegate; the region-pair loop's skip-equal-extruder condition is what the producer's pairing mirrors
- Verification:
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo clippy -p slicer-runtime --all-targets -- -D warnings` — FACT pass/fail
  - `cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail; the default-off stage must not disturb the existing painted-print path
- Exit condition: the workspace compiles with `--all-targets`, `rg -q 'PrePass::InterlockingBeams' crates/slicer-runtime/src/prepass.rs` succeeds, and the pre-existing `cube_4color_phase5` tests still pass. Behaviour is not claimed at this step — Step 7 proves or falsifies it.

### Step 7: Prove the seam — execution slot, downstream visibility, per-object gate

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: author `crates/slicer-runtime/tests/executor/prepass_interlocking_stage_order_tdd.rs` and register it, proving the built-in runs in the right slot, that `PrePass::SupportAnalysis` sees the interlocked footprint, and that the gate is per object.
- Precondition: Step 6's exit condition holds.
- Postcondition: AC-17 – AC-19 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/main.rs` — the `mod` declaration list only
  - `crates/slicer-runtime/src/builtins/interlocking_producer.rs` — whole file, authored in Step 6
  - `crates/slicer-runtime/src/prepass.rs` — the `run_builtin_stage` call sequence only, to assert the slot
  - existing blackboard fixture builders located by the dispatch below
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/prepass_interlocking_stage_order_tdd.rs` (new)
  - `crates/slicer-runtime/tests/executor/main.rs`
- Files explicitly out of bounds:
  - every `crates/*/src` file — if a test cannot be written against the producer as it stands, the finding is a Step 6 defect, not a licence to edit `src` here
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: not applicable — no struct field, no schema or version constant. The one hazard is registration: `crates/slicer-runtime/tests/executor/main.rs` aggregates 48 `mod` declarations today (including `mod cube_4color_phase5_tdd;`), and a new file without its `mod` line never compiles while `cargo test --test executor <filter>` reports a clean green on zero tests.
- Expected sub-agent dispatches:
  - Question: which existing test files build a `Blackboard` with a committed `SliceIR` and `RegionMapIR` and then call `run_prepass`?; scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS` ≤20; purpose: reuse an existing fixture builder rather than author a third
  - Question: what does the `PrePass::SupportAnalysis` built-in read off the `SliceIR`, so the AC-18 assertion targets a value it actually derives?; scope: `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`; return: `SUMMARY` ≤200 words
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — direct ranged read of the fixed stage-order section
- OrcaSlicer refs:
  - none — this step writes no ported code
- Verification:
  - `cargo test -p slicer-runtime --test executor prepass_interlocking 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `rg -q 'mod prepass_interlocking_stage_order_tdd;' crates/slicer-runtime/tests/executor/main.rs; echo "exit=$?"` — FACT exit code
- Exit condition: AC-17 – AC-19 pass **with a non-zero passing count**, and the `mod` line is present. A zero count means the file was never registered and the run was blind — check the count, not the exit code.

### Step 8: End-to-end proof on a real painted 3MF

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: author `crates/slicer-runtime/tests/executor/cube_4color_interlocking_tdd.rs` and register it, proving the six keys change a real slice of `resources/cube_4color.3mf` through `run_slice`.
- Precondition: Step 7's exit condition holds.
- Postcondition: AC-20 passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` — whole file, under 200 lines, for the `slice_cube` / `motion` helpers and the JSON-config-override idiom
  - `crates/slicer-runtime/tests/executor/main.rs` — the `mod` declaration list only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/cube_4color_interlocking_tdd.rs` (new)
  - `crates/slicer-runtime/tests/executor/main.rs`
- Files explicitly out of bounds:
  - every `crates/*/src` file
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` — Step 4 already renamed its two sites; do not fold the new test into it
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: not applicable — no struct field, no schema or version constant. Same `mod`-registration hazard as Step 7.
- Expected sub-agent dispatches:
  - Question: does `resources/cube_4color.3mf` produce at least two regions with distinct material tool indices after `PrePass::PaintSegmentation`? If not, the end-to-end AC cannot fire and the fixture must change.; scope: `crates/slicer-runtime/tests/**`, `crates/slicer-core/src/algos/paint_segmentation/**`; return: `FACT` ≤5 lines
- Context cost: `S`
- Authoritative docs:
  - none — this step writes a test against already-decided behaviour
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test executor cube_4color_interlocking 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `rg -q 'mod cube_4color_interlocking_tdd;' crates/slicer-runtime/tests/executor/main.rs; echo "exit=$?"` — FACT exit code
- Exit condition: AC-20 passes with a non-zero passing count. If the two motion-line sequences are equal, the cause is either a fixture with no tool-distinct region pair or a producer gate that never opened — diagnose before weakening the assertion, and never fall back to comparing the CONFIG_BLOCK header, which changes whenever a key is set and would pass vacuously.

### Step 9: Author the ADR and the two deviation rows

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: record the port strategy as an ADR and file the two deviations — region-pair identity taken from the material variant chain, and the `interlocking_beam` rename retiring the PnP spelling.
- Precondition: Step 8's exit condition holds.
- Postcondition: `cargo xtask check-deviations` exits 0 and an ADR filename containing `interlocking` exists.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the header row and the last few rows only, to match the live `DEV-###` row format
  - `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` — the front-matter and headings only, as the ADR format precedent
  - `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — the Status and Context sections only, to state conformance explicitly: ADR-0033 governs **guest** access to host-only algorithms through a bridge, and this packet has no guest, so it conforms by not needing the pattern rather than by contradicting it
- Files allowed to edit (at most 3):
  - `docs/adr/0064-interlocking-beams-host-prepass-voxel-port.md` (new; re-derive the number from disk)
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - every `crates/**` file
  - `docs/spec_packets/**` other than this packet's own directory
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: not applicable — no struct field, no schema or version constant. The ADR number and both `DEV-###` IDs are **ledger facts**: re-derive `max(ADR-*)` over `docs/adr/` and `max(DEV-*)` over `docs/DEVIATION_LOG.md` **plus** `docs/spec_packets/*/` at the moment of writing. `0064` and `DEV-199` were next-free at authoring; packets 303, 304 and 305 each currently intend a row spelled `DEV-198`, so do not assume that one is free either.
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` across `docs/DEVIATION_LOG.md` and every `docs/spec_packets/*/`, and the highest `ADR-####` prefix in `docs/adr/`?; scope: `docs/**`; return: `FACT` ≤5 lines
  - Question: does any existing ADR's normative content govern where a slice-mutating pass may run, or how per-region tool identity is carried?; scope: `docs/adr/`; return: `SUMMARY` ≤200 words; purpose: S8 conformance — conform or amend explicitly, never contradict silently
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — direct ranged read; the single source of truth, CI-checked by `cargo xtask check-deviations`
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask check-deviations` — FACT exit code
  - `ls docs/adr/ | rg -q 'interlocking'; echo "exit=$?"` — FACT exit code
- Exit condition: `cargo xtask check-deviations` exits 0, and the ADR records the two `[FWD]` resolutions from `design.md` §Open Questions (the `stInternal` reclassification question and the absent cancellation token) as decisions rather than leaving them open.

### Step 10: Doc edits

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land the three doc edits so the new stage and the six keys are discoverable from the authoritative docs.
- Precondition: Step 9's exit condition holds.
- Postcondition: AC-22 and AC-23 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/04_host_scheduler.md` — the fixed stage-order section only
  - `docs/01_system_architecture.md` — the prepass stage list only
  - `docs/15_config_keys_reference.md` — the Multimaterial section only, located by the dispatch below
- Files allowed to edit (at most 3):
  - `docs/04_host_scheduler.md`
  - `docs/01_system_architecture.md`
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - every `crates/**` file
  - `docs/07_implementation_status.md` — updated once at the completion gate through a worker dispatch, not here
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: not applicable — no struct field, no schema or version constant.
- Expected sub-agent dispatches:
  - Question: what is the exact anchor and row format of the Multimaterial section in the config-key reference, and where do the existing `mmu_segmented_region_*` rows sit?; scope: `docs/15_config_keys_reference.md`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md`, `docs/01_system_architecture.md` — direct ranged reads; both edited
  - `docs/15_config_keys_reference.md` — over 300 lines; delegated `LOCATIONS`, then a ranged edit
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'PrePass::InterlockingBeams' docs/04_host_scheduler.md && rg -q 'PrePass::InterlockingBeams' docs/01_system_architecture.md && rg -q 'interlocking_beam_width' docs/15_config_keys_reference.md && rg -q 'interlocking_boundary_avoidance' docs/15_config_keys_reference.md; echo "exit=$?"` — FACT exit code
  - `rg -qi 'declaration-only keys: 0' docs/spec_packets/306-interlocking-beams-slice-prepass/requirements.md && rg -qi 'queue count unchanged: 409' docs/spec_packets/306-interlocking-beams-slice-prepass/requirements.md; echo "exit=$?"` — FACT exit code
  - `cargo xtask check-literals` — FACT exit code
- Exit condition: AC-22 and AC-23 pass, and the config-key reference's new rows sit in the Multimaterial section beside the existing `mmu_segmented_region_*` rows rather than in a new section of their own.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Four delegated `SNIPPETS` reads of the voxel walk; the grid arithmetic is small but the walk functions are the fiddliest part of the port |
| Step 2 | M | Five ported functions plus the shared two-tool fixture every later kernel AC reuses |
| Step 3 | M | Two ported functions with dense set algebra; the observable assertions are indirect |
| Step 4 | S | Nine-site rename plus five `cli` declarations; mechanical, but `PartialEq` is hand-written |
| Step 5 | S | Two-file stage registration; the contract test asserts positions, not membership |
| Step 6 | M | New producer, new error variant, prepass registration; behaviour unproven until Step 7 |
| Step 7 | M | Three runtime seam tests plus aggregator registration |
| Step 8 | S | One end-to-end test on `resources/cube_4color.3mf` plus aggregator registration |
| Step 9 | S | One ADR, two deviation rows |
| Step 10 | S | Three doc edits |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate is `M`; no step is L. Every step's "Files allowed to edit" list holds at most 3 files, counted individually — Steps 5–10 exist as separate steps precisely because folding them into two would have busted that cap.

## Packet Completion Gate
- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — it carries two mentions of the retired `mmu_segmented_region_interlocking_beam` spelling that must move to `interlocking_beam`.
- Reconcile reopened/superseded status transitions: none — this packet supersedes no packet. It absorbs wayfinder ticket 97's key list, which is recorded in that ticket's resolution, not in another packet directory.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run the whole suite through the gated entry point — `cargo xtask test --summary --workspace` — because this packet changes a `ResolvedConfig` key name that the guest-facing config path and several e2e fixtures read. Dispatch it to a sub-agent with a `FACT pass/fail` return; never absorb the full output.
- Record remaining packet-local risk: the unmeasured cost of the voxel walk on many-variant painted objects, and the indirectness of AC-10's thin-area assertion.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
