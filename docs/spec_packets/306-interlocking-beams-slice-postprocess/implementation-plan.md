# Implementation Plan: 306-interlocking-beams-slice-postprocess

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Steps 1–4 land the analysis half and the config; Steps 5–7 land the seam; Steps 8–9 land the module; Steps 10–12 prove and document. No step edits more than three files.

## Steps

### Step 1: Port the voxel grid and dilation kernels

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `crates/slicer-core/src/algos/interlocking/voxel.rs` as an ungated module with the grid arithmetic and the three kernel shapes, proven by unit tests, before any lattice code exists.
- Precondition: `crates/slicer-core/src/algos/interlocking/` does not exist.
- Postcondition: AC-1 – AC-4 and AC-23 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — `Point2`, `Polygon`, `ExPolygon`, `mm_to_units` / `units_to_mm` only
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` — module header only, for the ungated-module and attribution-header precedent
  - `crates/slicer-core/Cargo.toml` — `[features]` and `[[test]]` blocks, to confirm no `required-features` is added
  - `docs/ORCASLICER_ATTRIBUTION.md` — whole file, short
  - `docs/08_coordinate_system.md` — porting checklist section only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/interlocking/voxel.rs`
  - `crates/slicer-core/src/algos/mod.rs`
  - `crates/slicer-core/tests/algo_interlocking_tdd.rs`
- Files explicitly out of bounds: `crates/slicer-runtime/**`, `modules/**`, `crates/slicer-schema/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — no struct field added to an existing type, no schema or version constant. `DilationKernel` and `VoxelGrid` are new with no existing literal sites.
- Expected sub-agent dispatches:
  - Question: `DilationKernel`'s exact `CUBE`/`DIAMOND`/`PRISM` construction loops and bounds?; scope: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: exact bodies of `walkLine`, `_walkAreas`, `walkAreas`, `walkPolygons`, `dilate`, and the half-cell XY translation `_walkAreas` assumes?; scope: same file; return: `SNIPPETS` ≤30 lines per function, one dispatch each
  - Question: `toGridCoord`, `toLowerCoord`, `toLowerCorner`, `toPolygon` verbatim?; scope: `.../VoxelUtils.hpp`; return: `SNIPPETS` ≤30 lines
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (ranged), `docs/ORCASLICER_ATTRIBUTION.md` (direct)
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.{cpp,hpp}` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `rg -q '^pub mod interlocking;' crates/slicer-core/src/algos/mod.rs && ! rg -q 'algo_interlocking_tdd' crates/slicer-core/Cargo.toml; echo "exit=$?"` — FACT exit code
- Exit condition: AC-1 – AC-4 and AC-23 pass, and a **bare** `cargo test -p slicer-core --test algo_interlocking_tdd` reports a non-zero passing count. A zero count means the module was gated and the run was blind.

### Step 2: Port the shell-voxel analysis and the lattice gate

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `crates/slicer-core/src/algos/interlocking/mod.rs` with `InterlockingParams`, `InterlockingPair`, `InterlockingLattice`, the four-condition gate, `get_shell_voxels`, `add_boundary_cells`, the air-cell erase and the rotation helpers.
- Precondition: Step 1's exit condition holds.
- Postcondition: AC-5 – AC-7, AC-N1 – AC-N3 and AC-N5 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/polygon_ops.rs` — `union_ex`, `intersection_ex`, `difference_ex`, `xor`, `offset`, `opening_ex`, `closing_ex`, `OffsetJoinType` signatures only
  - `crates/slicer-ir/src/slice_ir.rs` — `SlicedRegion`, `SliceIR`, `RegionKey`, `PaintValue`, `MODIFIER_FOOTPRINT_REGION_ID`, `is_modifier_namespace_id` only
  - `crates/slicer-core/src/algos/interlocking/voxel.rs` — whole file, authored in Step 1
  - `docs/21_data_defaults_and_fixtures.md` — struct-literal churn gate section only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/interlocking/mod.rs`
  - `crates/slicer-core/tests/algo_interlocking_tdd.rs`
  - `crates/slicer-core/src/algos/interlocking/voxel.rs` (only if a walk signature must widen for the analysis callbacks)
- Files explicitly out of bounds: `crates/slicer-runtime/**`, `modules/**`, `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-core/src/algos/paint_segmentation/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: `InterlockingParams` (six fields) and `InterlockingPair` are new `pub` structs under `crates/*/src`, which puts both on the `cargo xtask check-literals` watchlist from this step onward. Every **test** literal here and in Steps 8–11 needs a `..` rest or an `// exhaustive: <reason>` waiver; production `src/` literals stay exhaustive. No pre-existing literal sites to sweep — the types are new.
- Expected sub-agent dispatches:
  - Question: `getShellVoxels` and `addBoundaryCells` verbatim, including the `xor_ex` skin step and the `opening_ex(skin, cell_size.x()/2)` filter?; scope: `.../InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines each
  - Question: how does `generateInterlockingStructure` sequence the air-dilation `addBoundaryCells`, the `has_all_meshes.erase` loop and the `merge` intersection?; scope: same file; return: `SNIPPETS` ≤30 lines
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (ranged; `ignored_gap_` divides by 100), `docs/21_data_defaults_and_fixtures.md` (ranged)
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo xtask check-literals` — FACT exit code
- Exit condition: AC-5 – AC-7, AC-N1 – AC-N3 and AC-N5 pass, and `cells` is emitted in a deterministic sorted order (assert it; do not assume `HashSet` iteration is stable).

### Step 3: Register `InterlockingLatticeIR` in the schema and over WIT

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: introduce the new IR end to end — Rust type + `CURRENT_INTERLOCKING_LATTICE_SCHEMA_VERSION`, the WIT record at the canonical source, and the host/guest binding — so both the producer and the module can name it.
- Precondition: Step 2's exit condition holds.
- Postcondition: `cargo build --tests` succeeds and `cargo xtask build-guests --check` exits 0.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/` — the `LightningTreeIR` declaration and its schema constant only, as the worked example
  - `crates/slicer-schema/wit/` — the record neighbouring `lightning-tree`, located by the dispatch below
  - `crates/slicer-wasm-host/src/binding.rs` — `LayerStageInput` and its `lightning_tree_ir` field only
  - `docs/02_ir_schemas.md` — over 300 lines; the IR-registration anchor only, located by dispatch
  - `docs/03_wit_and_manifest.md` — the `[ir-access]` grammar section only
- Files allowed to edit (at most 3):
  - the `crates/slicer-ir/src/` file that declares the prepass IRs
  - `crates/slicer-schema/wit/` (canonical source only — both host `bindgen!` and the guest macro read it directly; there is no inline copy to sync)
  - `crates/slicer-wasm-host/src/binding.rs`
- Files explicitly out of bounds: `modules/**` (Step 8 authors the module), `crates/slicer-runtime/**` (Steps 6–7), `OrcaSlicerDocumented/**`
- Blast-radius discipline: **this is the widest step in the packet.** It adds a public IR type and a WIT record. Follow `CLAUDE.md` §"WIT/Type Changes Checklist" in full: search `wit_host.rs`, `dispatch.rs` and `wit_guest` for the affected type; verify type identity across the boundary (a `list<grid-point3>` on one side and a `Vec<GridPoint3>` on the other must resolve to the same record, or linking fails); run `cargo build --tests`. It introduces a **new** schema constant rather than bumping `CURRENT_SLICE_IR_SCHEMA_VERSION`, so no existing test asserting an old version value should be disturbed — confirm that with the dispatch below rather than assuming. If the step exceeds `M`, split it into "Rust type + schema constant" and "WIT record + binding" rather than widening the edit list.
- Expected sub-agent dispatches:
  - Question: how is a prepass IR registered end to end — schema constant, WIT record, `bindgen!` host side, guest macro side, `LayerStageInput` field — using `LightningTreeIR` as the worked example?; scope: `crates/slicer-ir/**`, `crates/slicer-schema/**`, `crates/slicer-wasm-host/**`; return: `LOCATIONS` ≤20
  - Question: does any test hard-assert a count of registered IRs, WIT records, or `LayerStageInput` fields that a new entry would break?; scope: `crates/**/tests/**`; return: `LOCATIONS` ≤20
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` (delegated anchor, then ranged), `docs/03_wit_and_manifest.md` (ranged)
- OrcaSlicer refs: none — this step ports no canonical code
- Verification:
  - `cargo build --tests` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT exit code (`0` fresh, `1` stale, `3` `wasm-tools` missing); rebuild without `--check` if stale
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: the workspace compiles with `--all-targets`, `build-guests --check` exits 0, and the new type is nameable from both a host crate and a guest crate. If any count-assertion test surfaced by the dispatch fails, fix it in this step — do not defer it.

### Step 4: Rename `interlocking_beam` and declare the six keys

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: retire `mmu_segmented_region_interlocking_beam` to canonical `interlocking_beam` with no alias, preserving the Phase-5 suppression read site, and add the five remaining keys as `ResolvedConfig` `cli` fields with canonical defaults and no invented bounds.
- Precondition: Step 3's exit condition holds; `rg -c 'mmu_segmented_region_interlocking_beam' crates` returns 9 across three files.
- Postcondition: AC-19 – AC-21 and AC-N4 pass.
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
  - `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` — names only the two keys canonical really does prefix `mmu_segmented_region_`
  - `crates/slicer-gcode/src/serialize.rs` — rule 2 non-evidence, no twin for any of the six
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: `ResolvedConfig`'s `PartialEq` is **hand-written**, not derived. The nine rename sites are `resolved_config.rs` ×4 (the `cli` key string, the field name, the `PartialEq` arm, the `to_config_map` omission comment), `paint_segmentation/mod.rs` ×3 (the `run_phase5_width_limit` read, its doc comment, the driver test's struct literal) and `cube_4color_phase5_tdd.rs` ×2 (module doc comment, the AC-7 JSON config string) — verified by `rg` at authoring. The five new `cli` fields also add five `PartialEq` arms in the same file. Other `ResolvedConfig` literals are unaffected: the type derives `Default` and every construction goes through it. `docs/07_implementation_status.md` carries two further mentions, updated at the completion gate.
- Expected sub-agent dispatches:
  - Question: `min`, `max`, `sidetext`, `ConfigOption*` type and default of each of the six `interlocking_*` `ConfigOptionDef`s?; scope: `.../PrintConfig.cpp`; return: `FACT` ≤5 lines
  - Question: after the edits, does `mmu_segmented_region_interlocking_beam` appear anywhere under `crates/`, `modules/` or `docs/`?; scope: `crates/**`, `modules/**`, `docs/**`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs: `docs/15_config_keys_reference.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the Multimaterial anchor. The doc edit itself is Step 12's.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate; never load
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_interlocking_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo test -p slicer-core --features host-algos --lib interlocking_beam_true_skips_phase5_driver 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `! rg -q 'mmu_segmented_region_interlocking_beam' crates modules; echo "exit=$?"` — FACT exit code
  - `cargo check --workspace --all-targets` — FACT pass/fail; the rename touches a test file, which a plain `cargo check` would not compile
- Exit condition: AC-19 – AC-21 and AC-N4 pass, the old spelling is gone from `crates/` and `modules/`, and `cargo test -p slicer-runtime --test executor cube_4color_phase5` still passes — proving the Phase-5 end-to-end behaviour survived the rename.

### Step 5: Register `PrePass::InterlockingLattice` and prove the stage can be shared

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: add the new host-only prepass stage to `STAGE_ORDER` and `HOST_ONLY_STAGES`, and prove that a narrow-write module coexists with a coarse `SliceIR` mutator on `Layer::SlicePostProcess`.
- Precondition: Step 4's exit condition holds.
- Postcondition: AC-9 and AC-N6 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` — the `STAGE_ORDER` const only
  - `crates/slicer-scheduler/src/dag.rs` — the `EdgeReason::IrWriteRead` construction and the `ir_reads()` containment test only
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — whole file, under 200 lines
  - `crates/slicer-scheduler/tests/unit/main.rs` — the `mod` declaration list only
  - `modules/core-modules/seam-placer/seam-placer.toml`, `modules/core-modules/part-cooling/part-cooling.toml` — whole files, short
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/execution_plan.rs`
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`
  - `crates/slicer-scheduler/tests/unit/interlocking_stage_sharing_tdd.rs` (new) plus its `mod` line in `crates/slicer-scheduler/tests/unit/main.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/src/lib.rs` — `VALID_STAGES` must **not** gain the prepass stage, and `Layer::SlicePostProcess` must stay in it unchanged
  - `docs/spec_packets/303-elefant-foot-slice-postprocess/**` — do not amend 303's manifest to make the pair fit; the narrow write resolves it from this side
  - `crates/slicer-runtime/**` — Steps 6–7
  - `OrcaSlicerDocumented/**` — delegate
- Blast-radius discipline: inserting into `STAGE_ORDER` obliges a matching `HOST_ONLY_STAGES` entry — `host_only_stages_partition_stage_order_into_valid_stages` fails on a `STAGE_ORDER` entry in neither list, so both edits land together. The new unit test file needs a `mod` line in `crates/slicer-scheduler/tests/unit/main.rs` (5 `mod` declarations today); without it the file never compiles and `cargo test --test scheduler_unit <filter>` reports a clean green on zero tests. No struct field, no version constant.
- Expected sub-agent dispatches:
  - Question: does the stage DAG accept a dotted write path against a coarse read of the same root without emitting a reverse edge — what exactly does `seam-placer` rely on?; scope: `crates/slicer-scheduler/src/dag.rs`; return: `SNIPPETS` ≤30 lines. **If dotted paths do not avoid the reverse edge, AC-N6 is unsatisfiable: stop, and report that this packet is blocked on ticket 148 rather than widening the module's writes.**
  - Question: which other draft packets insert into `STAGE_ORDER`, and at which position?; scope: `docs/spec_packets/*/`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs: `docs/04_host_scheduler.md` (ranged; the doc edit itself is Step 12's), `docs/03_wit_and_manifest.md` (ranged, `[ir-access]` grammar)
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — delegate; confirms the analysis sits after mm segmentation
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_contract stage_list_consistency_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo test -p slicer-scheduler --test scheduler_unit interlocking_and_coarse_slice_mutator_coexist_on_slice_postprocess 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `rg -q 'mod interlocking_stage_sharing_tdd;' crates/slicer-scheduler/tests/unit/main.rs; echo "exit=$?"` — FACT exit code
- Exit condition: AC-9 and AC-N6 pass **with a non-zero passing count**. `--test scheduler_contract` and `--test scheduler_unit` are the real target names from `crates/slicer-scheduler/Cargo.toml`; do not guess them from directory names.

### Step 6: Author the lattice producer and register the prepass

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` — region pairing by material tool index, per-object config gate, per-region outer-wall width — and register it with `run_builtin_stage` immediately after the `PrePass::PaintSegmentation` block.
- Precondition: Step 5's exit condition holds.
- Postcondition: the workspace compiles with the built-in registered; behaviour is proven in Step 10.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/prepass.rs` — the `PrepassExecutionError` enum, its `Display` arm list, and the `PrePass::PaintSegmentation` `run_builtin_stage` block only
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `commit_shell_classification_builtin` only, as the commit precedent
  - `crates/slicer-runtime/src/blackboard.rs` — the commit method for a new prepass slot and `BlackboardError` only
  - `crates/slicer-core/src/flow.rs` — `RoleWidthContext` and `resolve_role_width` only; argument order is `(role, first_layer, bridge, context)`
  - `crates/slicer-ir/src/slice_ir.rs` — `RegionKey`, `RegionMapIR::config_for`, `PaintValue`, `is_modifier_namespace_id` only
  - `crates/slicer-runtime/src/builtins/mod.rs` — whole file, under 30 lines
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` (new)
  - `crates/slicer-runtime/src/builtins/mod.rs`
  - `crates/slicer-runtime/src/prepass.rs`
- Files explicitly out of bounds: `crates/slicer-core/src/algos/interlocking/**` (the kernel is finished; a change there means Steps 1–2 were wrong — stop rather than patch), `crates/slicer-scheduler/**`, `modules/**`, `crates/slicer-runtime/tests/**` (Step 10), `OrcaSlicerDocumented/**`
- Blast-radius discipline: adds `PrepassExecutionError::InterlockingLattice { source: InterlockingLatticeBuiltinError }`. The `Display` impl is a hand-written exhaustive `match` in the same file — the arm lands in the same edit or the file does not compile. The enum derives `Clone + PartialEq + Eq`, so the source type must too; the `PaintSegmentation` variant carries a `String` precisely because its source does not, and that fallback is available. Committing a new IR may also require a new `BlackboardPrepassSlot` variant — check `blackboard.rs` before writing, and if it does, that variant and its match arms are part of this step's budget.
- Expected sub-agent dispatches:
  - Question: which test/non-test files construct a `RoleWidthContext` literal today, and with which field-init idiom?; scope: `crates/**/*.rs`; return: `LOCATIONS` ≤20
  - Question: does committing a new prepass IR require a new `BlackboardPrepassSlot` variant, and which match arms enumerate that enum?; scope: `crates/slicer-runtime/src/blackboard.rs`, `crates/slicer-runtime/src/prepass.rs`; return: `LOCATIONS` ≤20
- Context cost: `M`
- Authoritative docs: `docs/01_system_architecture.md` (ranged; the doc edit itself is Step 12's)
- OrcaSlicer refs: `.../InterlockingGenerator.cpp` — delegate; the region-pair loop's skip-equal-extruder condition is what the pairing mirrors
- Verification:
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo clippy -p slicer-runtime --all-targets -- -D warnings` — FACT pass/fail
  - `cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail; the default-off stage must not disturb the existing painted-print path
- Exit condition: the workspace compiles with `--all-targets` and `rg -q 'PrePass::InterlockingLattice' crates/slicer-runtime/src/prepass.rs` succeeds. Behaviour is not claimed here — Step 10 proves or falsifies it.

### Step 7: Thread the lattice onto `LayerStageInput` at the layer seam

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: make the committed `InterlockingLatticeIR` reach a `Layer::SlicePostProcess` module, mirroring how `lightning_tree_ir` reaches `lightning-infill`.
- Precondition: Step 6's exit condition holds.
- Postcondition: a module on `Layer::SlicePostProcess` receives `Some(lattice)` when the prepass committed one and `None` otherwise; the workspace compiles.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` — over 4000 lines; only the `LayerStageInput` construction site and the `Layer::SlicePostProcess` dispatch entry, located by `rg -n 'SlicePostProcess'` and `rg -n 'lightning_tree_ir'`
  - `crates/slicer-wasm-host/src/binding.rs` — `LayerStageInput` only, as edited in Step 3
  - `crates/slicer-runtime/src/blackboard.rs` — the lattice accessor only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
- Files explicitly out of bounds: `crates/slicer-core/**`, `modules/**`, `crates/slicer-scheduler/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: `LayerStageInput` gained a field in Step 3, so **every** construction site of that struct must already name it. Dispatch a `LOCATIONS` sweep for `LayerStageInput {` before editing and cite the result inline; if Step 3 left any site uncompiled, fix it here rather than at the acceptance ceremony. No schema or version constant is touched.
- Expected sub-agent dispatches:
  - Question: which files construct a `LayerStageInput` literal, and how does each obtain `lightning_tree_ir`?; scope: `crates/**/*.rs`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs: `docs/05_module_sdk.md` — ranged read of the stage-entrypoint signature
- OrcaSlicer refs: none — this step ports no canonical code
- Verification:
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT exit code
- Exit condition: the workspace compiles with `--all-targets` and every `LayerStageInput` construction site names the new field. A `cargo check` that passes only because a construction site sits behind a disabled `cfg` is not an exit — confirm against the dispatch's `LOCATIONS` list.

### Step 8: Scaffold the module and port the microstructure and outline rewrite

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: create `modules/core-modules/interlocking-beams/` with `pnp_cli module new`, declare the manifest, and port `generateMicrostructure` and `applyMicrostructureToOutlines` into the guest, emitting `polygon_updates`.
- Precondition: Step 7's exit condition holds.
- Postcondition: AC-10 – AC-13, AC-15 and AC-16 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/seam-placer.toml` — whole file; the narrow-write `[ir-access]` shape
  - `modules/core-modules/lightning-infill/` — the manifest and entrypoint signature only; how a layer-stage module consumes a prepass IR
  - `crates/slicer-ir/src/stage_io.rs` — the `LayerStageCommit::SlicePostProcess` variant only
  - `crates/slicer-core/src/polygon_ops.rs` — the set-operation signatures only
  - `docs/05_module_sdk.md` — the stage-entrypoint section only
  - `docs/03_wit_and_manifest.md` — the `[ir-access]` grammar and §Known claim IDs only
- Files allowed to edit (at most 3):
  - `modules/core-modules/interlocking-beams/interlocking-beams.toml`
  - `modules/core-modules/interlocking-beams/src/lib.rs`
  - `modules/core-modules/interlocking-beams/Cargo.toml`
- Files explicitly out of bounds: `crates/**` (every host crate is finished by this point), `modules/core-modules/*/` other than the new one, `OrcaSlicerDocumented/**`
- Blast-radius discipline: a new core module is discovered dynamically, so no edition or registry file should need editing — but confirm that with the dispatch below rather than assuming, because `dist/editions.toml` names the natively-integrated modules explicitly. `InterlockingParams` literals in this crate's tests owe a `..` rest or an `// exhaustive:` waiver. **`cargo xtask build-guests --check` must exit 0 at the end of this step**; a new guest that is never built is the classic "unrelated" failure.
- Expected sub-agent dispatches:
  - Question: what does `generateMicrostructure` compute for `middle` and `width[2]`, and how is phase 1 derived from phase 0?; scope: `.../InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: the exact sequence of set operations in `applyMicrostructureToOutlines`, and which structure-phase index each layer reads?; scope: same file; return: `SNIPPETS` ≤30 lines
  - Question: does adding a core module require an edit to `dist/editions.toml` or any registry, or are core modules discovered dynamically?; scope: `xtask/**`, `dist/**`, `crates/slicer-runtime/**`; return: `FACT` ≤5 lines
- Context cost: `M`
- Authoritative docs: `docs/05_module_sdk.md` (ranged), `docs/03_wit_and_manifest.md` (ranged), `docs/ORCASLICER_ATTRIBUTION.md` (direct; the ported file carries the header)
- OrcaSlicer refs: `.../InterlockingGenerator.cpp` — delegate; never load
- Verification:
  - `cargo test -p interlocking-beams 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT exit code
  - `rg -q 'writes = \["SliceIR\.regions\.polygons"\]' modules/core-modules/interlocking-beams/interlocking-beams.toml; echo "exit=$?"` — FACT exit code; a coarse write here fails AC-N6
- Exit condition: AC-10 – AC-13, AC-15 and AC-16 pass, `build-guests --check` exits 0, and AC-14 **fails** — thin-area handling is Step 9's and must not be half-implemented here.

### Step 9: Port thin-area handling into the module

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land `grow_border_areas_perpendicular` and `handle_thin_areas` in the module, reached only when `interlocking_boundary_avoidance > 0`, so the sixth key drives a behaviour change.
- Precondition: Step 8's exit condition holds — AC-14 currently fails.
- Postcondition: AC-14 passes and every Step 8 AC still passes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/interlocking-beams/src/lib.rs` — whole file, authored in Step 8
  - `crates/slicer-core/src/flow.rs` — `RoleWidthContext` and `resolve_role_width` only, for the per-region outer-wall width the two functions consume
  - `crates/slicer-core/src/polygon_ops.rs` — `opening_ex`, `closing_ex`, `offset`, `intersection_ex`, `difference_ex` signatures only
- Files allowed to edit (at most 3):
  - `modules/core-modules/interlocking-beams/src/lib.rs`
  - `modules/core-modules/interlocking-beams/interlocking-beams.toml` (only if a config key must be added to the schema for the width lookup)
- Files explicitly out of bounds: `crates/**`, other modules, `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — no struct field, no schema or version constant. New test literals still owe the churn-gate treatment.
- Expected sub-agent dispatches:
  - Question: `growBorderAreasPerpendicular` verbatim, including the loop bound `(detect / min_line) + 2`?; scope: `.../InterlockingGenerator.cpp`; return: `SNIPPETS` ≤30 lines
  - Question: `handleThinAreas` verbatim, including `number_of_beams_detect` / `number_of_beams_expand`, `rounding_errors`, `close_gaps` and the four-way intersection?; scope: same file; return: `SNIPPETS` ≤30 lines
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` — ranged; `rounding_errors = 5` is a 1 nm constant and becomes sub-unit here
- OrcaSlicer refs: `.../InterlockingGenerator.cpp` — delegate; never load
- Verification:
  - `cargo test -p interlocking-beams 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT exit code
  - `cargo clippy -p interlocking-beams --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: AC-14 passes, and the `boundary_avoidance_cells = 0` results from Step 8 are unchanged — the thin-area branch must be additive, not a rewrite of the default path.

### Step 10: Prove the seam end to end through real dispatch

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: author the three runtime tests — the analysis prepass runs in its slot without mutating slices, the module's `polygon_updates` merge onto the right regions, and the per-object gate holds — plus the `cube_4color.3mf` end-to-end test.
- Precondition: Step 9's exit condition holds.
- Postcondition: AC-8, AC-17, AC-18 and AC-22 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/main.rs` — the `mod` declaration list only (48 declarations today)
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` — whole file, for the `slice_cube` / `motion` helpers and the JSON-override idiom
  - `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` — whole file, authored in Step 6
  - existing blackboard fixture builders located by the dispatch below
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/prepass_interlocking_lattice_tdd.rs` (new)
  - `crates/slicer-runtime/tests/executor/layer_interlocking_beams_tdd.rs` and `cube_4color_interlocking_tdd.rs` (new)
  - `crates/slicer-runtime/tests/executor/main.rs`
- Files explicitly out of bounds: every `crates/*/src` and `modules/*/src` file — if a test cannot be written against the code as it stands, that is an earlier step's defect, not a licence to edit `src` here; `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — no struct field, no schema or version constant. The hazard is registration: three new files need three `mod` lines in `crates/slicer-runtime/tests/executor/main.rs`, and a file without one never compiles while `cargo test --test executor <filter>` reports a clean green on **zero** tests.
- Expected sub-agent dispatches:
  - Question: which existing test files build a `Blackboard` with a committed `SliceIR` and `RegionMapIR` and then call `run_prepass`?; scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS` ≤20
  - Question: which existing test drives a real guest module through a `Layer::` stage, and how does it load the module?; scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS` ≤20
  - Question: does `resources/cube_4color.3mf` produce at least two regions with distinct material tool indices after `PrePass::PaintSegmentation`?; scope: `crates/slicer-runtime/tests/**`, `crates/slicer-core/src/algos/paint_segmentation/**`; return: `FACT` ≤5 lines
- Context cost: `M`
- Authoritative docs: `docs/04_host_scheduler.md` — ranged read of the fixed stage-order section
- OrcaSlicer refs: none — this step writes no ported code
- Verification:
  - `cargo test -p slicer-runtime --test executor prepass_interlocking_lattice 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo test -p slicer-runtime --test executor layer_interlocking_beams 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `cargo test -p slicer-runtime --test executor cube_4color_interlocking 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'` — FACT pass/fail
  - `for m in prepass_interlocking_lattice_tdd layer_interlocking_beams_tdd cube_4color_interlocking_tdd; do rg -q "mod $m;" crates/slicer-runtime/tests/executor/main.rs || exit 1; done; echo "exit=$?"` — FACT exit code
- Exit condition: AC-8, AC-17, AC-18 and AC-22 pass **with non-zero passing counts**, and all three `mod` lines are present. If AC-22's two motion-line sequences are equal, diagnose — a fixture with no tool-distinct region pair, a gate that never opened, or a stale guest (`build-guests --check` first) — and never fall back to comparing the CONFIG_BLOCK header, which changes whenever a key is set and would pass vacuously.

### Step 11: Author the ADR and the two deviation rows

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: record the analysis-in-prepass / apply-in-module split as an ADR and file the two deviations — variant-chain region-pair identity, and the `interlocking_beam` rename.
- Precondition: Step 10's exit condition holds.
- Postcondition: `cargo xtask check-deviations` exits 0 and an ADR filename containing `interlocking` exists.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the header row and the last few rows only, to match the live `DEV-###` format
  - `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` — front-matter and headings only, as the ADR format precedent
  - `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — Status and Context only; state conformance explicitly, since ADR-0033 governs guest access to host-only algorithms and this packet's prepass IR is a different mechanism from the four-layer bridge
- Files allowed to edit (at most 3):
  - `docs/adr/0064-interlocking-beams-analysis-prepass-apply-in-module.md` (new; re-derive the number from disk)
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds: every `crates/**` and `modules/**` file; `docs/spec_packets/**` other than this packet's own directory
- Blast-radius discipline: not applicable — no struct field, no schema or version constant. The ADR number and both `DEV-###` IDs are **ledger facts**: re-derive `max(ADR-*)` over `docs/adr/` and `max(DEV-*)` over `docs/DEVIATION_LOG.md` **plus** `docs/spec_packets/*/` at the moment of writing. `0064` and `DEV-199` were next-free at authoring; packets 303, 304 and 305 each currently intend a row spelled `DEV-198`.
- Expected sub-agent dispatches:
  - Question: highest `DEV-###` across `docs/DEVIATION_LOG.md` and every `docs/spec_packets/*/`, and highest `ADR-####` prefix in `docs/adr/`?; scope: `docs/**`; return: `FACT` ≤5 lines
  - Question: does any existing ADR's normative content govern where a slice-mutating pass may run, or how per-region tool identity is carried?; scope: `docs/adr/`; return: `SUMMARY` ≤200 words
- Context cost: `S`
- Authoritative docs: `docs/DEVIATION_LOG.md` — direct ranged read; CI-checked by `cargo xtask check-deviations`
- OrcaSlicer refs: none
- Verification:
  - `cargo xtask check-deviations` — FACT exit code
  - `ls docs/adr/ | rg -q 'interlocking'; echo "exit=$?"` — FACT exit code
- Exit condition: `cargo xtask check-deviations` exits 0, and the ADR records both `[FWD]` resolutions from `design.md` §Open Questions (the `stInternal` reclassification question and the absent cancellation token) as decisions, **plus the reason the application half is a module rather than a host built-in** — so the next slice-mutating pass inherits the reasoning rather than re-deriving it.

### Step 12: Doc edits

- Task IDs: none (queue packet; backlog row is wayfinder ticket 96)
- Objective: land the doc edits so the new stage, the new IR and the six keys are discoverable from the authoritative docs.
- Precondition: Step 11's exit condition holds.
- Postcondition: AC-24 and AC-25 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/04_host_scheduler.md` — the fixed stage-order section only
  - `docs/01_system_architecture.md` — the prepass stage list only
  - `docs/02_ir_schemas.md` — over 300 lines; the IR-registration anchor only, located by dispatch
  - `docs/15_config_keys_reference.md` — over 300 lines; the Multimaterial section only, located by dispatch
- Files allowed to edit (at most 3):
  - `docs/04_host_scheduler.md` and `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds: every `crates/**` and `modules/**` file; `docs/07_implementation_status.md` (updated once at the completion gate through a worker dispatch, not here)
- Blast-radius discipline: not applicable — no struct field, no schema or version constant.
- Expected sub-agent dispatches:
  - Question: the exact anchor and row format of the Multimaterial section in the config-key reference, and where the existing `mmu_segmented_region_*` rows sit?; scope: `docs/15_config_keys_reference.md`; return: `LOCATIONS` ≤20
  - Question: the exact anchor and format of the IR list in the IR-schemas doc, using `LightningTreeIR` as the example row?; scope: `docs/02_ir_schemas.md`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs: `docs/04_host_scheduler.md`, `docs/01_system_architecture.md` (direct ranged reads); `docs/02_ir_schemas.md`, `docs/15_config_keys_reference.md` (delegated anchors, then ranged edits)
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'PrePass::InterlockingLattice' docs/04_host_scheduler.md && rg -q 'PrePass::InterlockingLattice' docs/01_system_architecture.md && rg -q 'InterlockingLatticeIR' docs/02_ir_schemas.md && rg -q 'interlocking_beam_width' docs/15_config_keys_reference.md; echo "exit=$?"` — FACT exit code
  - `rg -qi 'declaration-only keys: 0' docs/spec_packets/306-interlocking-beams-slice-postprocess/requirements.md && rg -qi 'queue count unchanged: 409' docs/spec_packets/306-interlocking-beams-slice-postprocess/requirements.md; echo "exit=$?"` — FACT exit code
  - `cargo xtask check-literals` — FACT exit code
- Exit condition: AC-24 and AC-25 pass, and the config-key reference's new rows sit in the Multimaterial section beside the existing `mmu_segmented_region_*` rows rather than in a section of their own.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Four delegated `SNIPPETS` reads; the walk functions are the fiddliest part of the port |
| Step 2 | M | The analysis half plus the shared 8-layer two-tool fixture every later kernel AC reuses |
| Step 3 | M | New IR across the WIT boundary — the widest step; split it rather than widen its edit list |
| Step 4 | S | Nine-site rename plus five `cli` declarations; mechanical, but `PartialEq` is hand-written |
| Step 5 | S | Stage registration plus the stage-sharing proof that unblocks coexistence with packet 303 |
| Step 6 | M | The lattice producer, a new error variant, possibly a new blackboard slot |
| Step 7 | S | One threading edit, but every `LayerStageInput` construction site must already name the field |
| Step 8 | M | New guest module: manifest, scaffold, microstructure, outline rewrite |
| Step 9 | M | Two ported functions with dense set algebra; the assertions are indirect |
| Step 10 | M | Four runtime tests plus three aggregator registrations |
| Step 11 | S | One ADR, two deviation rows |
| Step 12 | S | Four doc edits |

Aggregate is `M`; no step is L. Every step's "Files allowed to edit" list holds at most 3 files — the twelve-step shape exists precisely to keep that true across an IR change and a new guest module.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` exits 0 — a new guest module and a WIT change both landed.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — it carries two mentions of the retired `mmu_segmented_region_interlocking_beam` spelling.
- Reconcile reopened/superseded status transitions: none. This packet supersedes no packet; it absorbs wayfinder ticket 97's key list, recorded in that ticket's resolution and in `task-map.md`.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run the whole suite through the gated entry point — `cargo xtask test --summary --workspace` — because this packet changes a `ResolvedConfig` key name, adds an IR across the WIT boundary, and adds a guest module. Dispatch it to a sub-agent with a `FACT pass/fail` return; never absorb the full output.
- Record remaining packet-local risk: the unmeasured cost of the voxel analysis on many-variant painted objects, and the indirectness of AC-14's thin-area assertion.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
