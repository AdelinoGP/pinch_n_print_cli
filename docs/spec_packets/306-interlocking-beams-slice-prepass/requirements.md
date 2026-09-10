# Requirements: 306-interlocking-beams-slice-prepass

## Packet Metadata

- Grouped task IDs: none — this is a queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap", not a `docs/07` slice. Its backlog row is wayfinder ticket 96 (P89), with ticket 97 (P90) dissolved into it.
- Backlog source: `docs/specs/orca-feature-gap/issues/96-author-packet-p89-multimaterial-multimaterial-advanced-new-interlocking.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's beam interlocking generates a physical dovetail lattice where two filaments meet inside one object, so a multi-material part does not delaminate along the colour boundary. This port has none of it. Of the six keys, five — `interlocking_beam_layer_count`, `interlocking_beam_width`, `interlocking_depth`, `interlocking_orientation`, `interlocking_boundary_avoidance` — have **zero occurrences** anywhere in `crates/` or `modules/`, including `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`), which carries no twin for any of them. The sixth, `interlocking_beam`, is live but **misnamed and half-implemented**.

**The six are one packet, not two.** Canonical's enabling gate reads four of them in a single condition — `!interlocking_beam || interlocking_beam_layer_count < 1 || interlocking_depth < 1 || interlocking_beam_width < EPSILON` — and `interlocking_depth` sits in the P90 trio. The remaining two P90 keys are no more separable: `interlocking_orientation` is applied to every polygon before the voxel walk and unapplied on every output, so it threads through `getShellVoxels`, `computeUnionedVolumeRegions`, `handleThinAreas` and `applyMicrostructureToOutlines`; `interlocking_boundary_avoidance` selects the `air_filtering` branch that owns `addBoundaryCells` on the air dilation **and** the whole of `handleThinAreas` / `growBorderAreasPerpendicular`. A P89-only packet would hardcode all three inside the kernel and a P90 packet would unpick them, rewriting every acceptance test for no gain: the work is the voxel generator, and the generator needs all six. Ticket 97 is dissolved into this packet on the ticket-31 (P24) precedent.

**`interlocking_beam` is already in this tree under a name canonical does not have.** `crates/slicer-ir/src/resolved_config.rs` declares `mmu_segmented_region_interlocking_beam`; canonical `PrintConfig.cpp` / `PrintConfig.hpp` declare `interlocking_beam` and no `mmu_segmented_region_interlocking_beam` (it is a `PrintObjectConfig` member, whereas `mmu_segmented_region_interlocking_depth` and `mmu_segmented_region_max_width` genuinely do carry that prefix). The PnP spelling is an invention that landed with the Phase-5 work. Under it, the key's **second** canonical read site is already correct: `multi_material_segmentation_by_painting` (`MultiMaterialSegmentation.cpp`) suppresses `cut_segmented_layers` when `interlocking_beam` is set, and `run_phase5_width_limit` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) does exactly that. What is missing is the **first** read site — the generator gate — and the generator behind it. This packet renames the key to the canonical spelling and adds the missing read site; it does not re-home Phase 5.

### The tier table's owner cannot work — three independently sufficient reasons

Ticket 04 assigns all six keys to a `new interlocking module`, Tier C. Re-derived at claim time per ticket 27, that owner fails:

1. **A guest module cannot write slices at all.** `PrepassStageOutput` — defined in `crates/slicer-core/src/stage_io.rs` and re-exported by `crates/slicer-runtime/src/prepass.rs`, so do not go looking for it in the runtime crate — has exactly the variants `None`, `SurfaceClassification`, `LayerPlan`, `SeamPlan`, `SupportPlan`, `RegionMap`, `SupportGeometry` — **no `SliceIR` variant**. A prepass module receives `slice_ir` on `PrepassStageInput` (`crates/slicer-wasm-host/src/binding.rs` — a different crate from `PrepassStageOutput`, despite the paired names) and can read it, but has no channel to return a mutated one. Interlocking's entire output is rewritten `SlicedRegion.polygons`. This is also why `PrePass::PaintSegmentation`, the one module-targetable prepass stage in this neighbourhood, is itself implemented as a host built-in.
2. **The algorithm is whole-object, not per-layer.** The voxel cell is `Vec3crd(2 * beam_width, 2 * beam_width, 2 * beam_layer_count)` — a single cell spans `2 * beam_layer_count` layers in Z, so `addBoundaryCells` walks a stack of layers at once, `computeUnionedVolumeRegions` allocates a **ghost layer** above the top so the topmost skin computes, and `handleThinAreas` builds one `near_interlock_per_layer` vector across the whole object before touching any layer. A `Layer::` stage sees one layer and cannot express any of it.
3. **A second coarse `SliceIR` mutator on a shared stage deadlocks the scheduler.** The per-stage DAG (`crates/slicer-scheduler/src/dag.rs`) emits an `EdgeReason::IrWriteRead` edge whenever a reader's `ir_reads()` contains a writer's write path, by exact `Vec<String>::contains`. An honest interlocking manifest is `reads = ["SliceIR"]`, `writes = ["SliceIR"]` — the same shape as packet 303's — so the pair produces edges in both directions and `validate_cycles` (`crates/slicer-scheduler/src/validation.rs`) → `topological_sort` (`crates/slicer-scheduler/src/topology.rs` — a different file from `validate_cycles`, despite its caller living in `validation.rs`) fails with `SchedulerError::CyclicDependency` (`crates/slicer-scheduler/src/validation.rs`).

Corrected to `host:interlocking_beams` on a new **host-only** stage `PrePass::InterlockingBeams`, on packet 297's staging shape (kernel in `slicer-core/algos`, producer in `slicer-runtime/builtins`, one `run_builtin_stage` registration).

### Where the stage goes, and one ordering hazard the implementer must not "fix"

Canonical calls `generate_interlocking_structure` inside `PrintObject::slice_volumes`, after `apply_mm_segmentation` and before the XY-size / elephant-foot compensation block. In this tree the material-split regions the generator pairs over are produced by the `PrePass::PaintSegmentation` built-in, so the stage is registered immediately after it and before `PrePass::SupportAnalysis` — which matches canonical, where support generation happens well after `slice_volumes` returns.

**`STAGE_ORDER` does not describe the order host built-ins actually execute in, and this packet must not try to correct that.** `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) lists `PrePass::PaintSegmentation` fourth, ahead of `PrePass::RegionMapping` and `PrePass::Slice`; `run_prepass` (`crates/slicer-runtime/src/prepass.rs`) actually runs it **seventh**, after `PrePass::ShellClassification`. `STAGE_ORDER` governs guest prepass-module dispatch order and visual-debug tap ordering; the built-in sequence is the hardcoded call order in `run_prepass`. The new stage is inserted in `STAGE_ORDER` between `"PrePass::ShellClassification"` and `"PrePass::SupportAnalysis"`, which is both its true execution slot **and** still after `"PrePass::PaintSegmentation"` in the declared list, so the declared order stays self-consistent for this stage without reopening the pre-existing discrepancy. Reconciling `PrePass::PaintSegmentation`'s own declared slot is out of scope here.

## In Scope

- Six queue keys added or renamed as `ResolvedConfig` `cli` fields, with canonical types and defaults: `interlocking_beam: bool = false`, `interlocking_beam_width: f32 = 0.8` (mm), `interlocking_beam_layer_count: i32 = 2`, `interlocking_depth: i32 = 2` (cells), `interlocking_orientation: f32 = 22.5` (degrees), `interlocking_boundary_avoidance: i32 = 2` (cells).
- Retiring `mmu_segmented_region_interlocking_beam` to canonical `interlocking_beam` — a straight rename with no alias, across its nine code sites in three files (`crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`), preserving the Phase-5 suppression behaviour unchanged.
- A new ungated kernel module `crates/slicer-core/src/algos/interlocking/` with two files: `voxel.rs` (`GridPoint3`, `Vec3Units`, `DilationKernel`, `DilationKernelType::{Cube, Diamond, Prism}`, `VoxelGrid` with `to_grid_coord` / `to_lower_corner` / `to_polygon` / `walk_line` / `walk_polygons` / `walk_areas` / `walk_dilated_polygons` / `walk_dilated_areas` / `dilate`) and `mod.rs` (`InterlockingParams`, `generate_interlocking_structure`, `get_shell_voxels`, `add_boundary_cells`, `compute_unioned_volume_regions`, `generate_microstructure`, `apply_microstructure_to_outlines`, `grow_border_areas_perpendicular`, `handle_thin_areas`). Both carry the porting header from `docs/ORCASLICER_ATTRIBUTION.md`.
- A new host built-in producer `crates/slicer-runtime/src/builtins/interlocking_producer.rs` exposing `commit_interlocking_builtin` and `InterlockingBuiltinError`, registered through `run_builtin_stage` and writing back via `Blackboard::replace_slice_ir`.
- A new `PrepassExecutionError::Interlocking { source: InterlockingBuiltinError }` variant with its `Display` arm.
- `"PrePass::InterlockingBeams"` added to `STAGE_ORDER` and to `HOST_ONLY_STAGES`; deliberately **not** added to `VALID_STAGES`.
- Region-pair identity derived from each `SlicedRegion.variant_chain`'s `("material", PaintValue::ToolIndex(n))` entry, with regions carrying no material entry taking the object's base tool index; pairs with equal tool index are skipped exactly as canonical skips equal-extruder region pairs.
- Per-region external-perimeter width taken from `resolve_role_width(ExtrusionRole::OuterWall, false, false, &RoleWidthContext { .. })` (`crates/slicer-core/src/flow.rs`) built from that region's `ResolvedConfig`, standing in for canonical's `printing_region.flow(print_object, frExternalPerimeter, 0.1).scaled_width()`.
- Two `docs/DEVIATION_LOG.md` rows and one ADR (port strategy: voxel grid, region-pair identity, seam placement).
- Doc edits per `packet.spec.md` §Doc Impact Statement.

### Key disposition table

| Queue key | Canonical type / default | Canonical read site | Decision point after this packet | Disposition |
| --- | --- | --- | --- | --- |
| `interlocking_beam` | `coBool` / `false` | `InterlockingGenerator::generate_interlocking_structure` gate; `multi_material_segmentation_by_painting`'s `!interlocking_beam` guard on `cut_segmented_layers` | `generate_interlocking_structure`'s gate **and** the existing `run_phase5_width_limit` suppression, now under the canonical name | live (renamed + second read site added) |
| `interlocking_beam_width` | `coFloat` / `0.8` mm | `generate_interlocking_structure` — `scaled(...)` into `cell_width` and the microstructure split | `InterlockingParams::beam_width_mm` → `VoxelGrid` cell XY size and `generate_microstructure`'s `middle` | live |
| `interlocking_beam_layer_count` | `coInt` / `2` | `generate_interlocking_structure` — `cell_size.z()` and the structure-layer stride | `InterlockingParams::beam_layer_count` → cell Z size and `apply_microstructure_to_outlines`' stride | live |
| `interlocking_depth` | `coInt` / `2` cells | `generate_interlocking_structure` — `interface_dilation` kernel size | `InterlockingParams::depth_cells` → the `DilationKernelType::Prism` interface kernel in `get_shell_voxels` | live |
| `interlocking_orientation` | `coFloat` / `22.5`° | `generate_interlocking_structure` — `Geometry::deg2rad`, applied and unapplied around the walk | `InterlockingParams::orientation_deg` → the rotation in `get_shell_voxels` / `compute_unioned_volume_regions` / `handle_thin_areas` / `apply_microstructure_to_outlines` | live |
| `interlocking_boundary_avoidance` | `coInt` / `2` cells | `generate_interlocking_structure` — `air_filtering` switch and `air_dilation` kernel size | `InterlockingParams::boundary_avoidance_cells` → the air-cell erase branch and `handle_thin_areas` | live |

**Declaration-only keys: 0.** Every one of the six drives a behaviour-changing decision point proven by an AC at a non-default value (AC-7 through AC-12, AC-N1 through AC-N3). **Queue count unchanged: 409** — six keys move from unimplemented to implemented; none is ruled out of scope and no supporting non-queue key is added.

## Out of Scope

- **Canonical's XY-size-compensation suppression for mm-painted objects.** `slice_volumes` zeroes `xy_hole_compensation` / `xy_contour_compensation` when the object is multi-filament and mm-painted, and warns. Packet 305 owns those two keys and deliberately does not port the suppression; this packet does not add it either, and does not amend 305. Consequence, recorded rather than fixed: on a painted object this port applies XY compensation and then interlocking, where canonical applies neither compensation nor that ordering.
- **Reconciling `PrePass::PaintSegmentation`'s `STAGE_ORDER` slot with its true execution position.** Pre-existing, unrelated to these keys, and touching it would move a stage that four other built-ins are ordered against.
- **Re-homing Phase 5 (`cut_segmented_layers`) or its two `mmu_segmented_region_*` keys.** Those are wayfinder ticket 98's (P91). This packet renames only the beam bool and leaves `run_phase5_width_limit`'s behaviour byte-identical.
- **Emitting any of the six keys into the CONFIG_BLOCK.** `to_config_map` (`crates/slicer-ir/src/resolved_config.rs`) already omits the `mmu_segmented_region_*` trio to keep G-code bytes stable; the six stay host-only on the same footing. Ticket 132 owns the reader contract and the padding derivation.
- **Adopting canonical's `min` / `max` on the five bounded keys.** Ticket 113 measured that canonical never enforces them; this port does enforce declared bounds, so adopting them would be a divergence dressed as parity. The runtime gate no-ops every degenerate value instead (AC-N1 – AC-N4).
- **`DilationKernelType::Cube` / `Diamond` code paths beyond construction.** Canonical's interlocking path only ever builds `PRISM` kernels. The other two variants are ported for shape fidelity and are covered by AC-1's structure assertion only; no interlocking behaviour depends on them.
- Any `Layer::` stage, any guest module, any manifest, any WIT change, and any new claim.

## Authoritative Docs

- `docs/01_system_architecture.md` — direct ranged read of the prepass stage list; edited.
- `docs/04_host_scheduler.md` — direct ranged read of the fixed stage-order section; edited.
- `docs/08_coordinate_system.md` — direct ranged read of the porting checklist.
- `docs/15_config_keys_reference.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the Multimaterial anchor, then edit that range only.
- `docs/21_data_defaults_and_fixtures.md` — direct ranged read of the struct-literal churn gate.
- `docs/ORCASLICER_ATTRIBUTION.md` — direct read; short.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp` — the whole algorithm: the six-key enabling gate, the region-pair loop, `getShellVoxels`, `addBoundaryCells`, `computeUnionedVolumeRegions`, `generateMicrostructure`, `applyMicrostructureToOutlines`, `growBorderAreasPerpendicular` and `handleThinAreas`.
- `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp` — `DilationKernel`'s three kernel shapes and the `walkLine` / `walkPolygons` / `walkAreas` / `walkDilatedPolygons` / `walkDilatedAreas` / `dilate` grid walk, including the half-cell XY translation `_walkAreas` assumes.
- `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.hpp` — `toGridCoord`'s negative-coordinate floor and `toLowerCorner` / `toPolygon`; borrowed exactly.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — the single call site of `generate_interlocking_structure` and its position relative to `apply_mm_segmentation` and the XY / elephant-foot compensation block; establishes this packet's stage ordering.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` — `multi_material_segmentation_by_painting`'s `!interlocking_beam` guard on `cut_segmented_layers`; the key's **second** read site, already implemented in this tree under the PnP name being retired.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the six `ConfigOptionDef`s: types, defaults, and the `min` / `max` values this packet deliberately does **not** adopt (ticket 113).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-23`. Refinements absent from their Given/When/Then text: AC-1 – AC-6 are pure kernel unit assertions runnable before any producer exists, so they are the TDD front of Steps 1–2; AC-7 – AC-12 and AC-N1 – AC-N5 all use the same two-tool 20 mm fixture, which Step 2 authors once as a shared test helper; AC-16 – AC-20 require the producer and stage registration and therefore cannot pass before Steps 5–8.
- Negative: `AC-N1` through `AC-N5`.
- Cross-packet impact: packet 305's `PrePass::XySizeCompensation` and this packet's `PrePass::InterlockingBeams` both insert into `STAGE_ORDER`; whichever merges second re-runs `cargo test -p slicer-scheduler --test scheduler_contract` because the stage-list contract test asserts positional relations, not just membership. No other packet's ACs are affected.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-1 – AC-12, AC-N1 – AC-N3, AC-N5, AC-21: the whole kernel | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-ir --test resolved_config_interlocking_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-13, AC-N4: the six keys resolve with canonical defaults and no invented bounds | FACT pass/fail |
| `! rg -q 'mmu_segmented_region_interlocking_beam' crates modules && rg -q 'mmu_segmented_region_max_width' crates/slicer-ir/src/resolved_config.rs && rg -q 'mmu_segmented_region_interlocking_depth' crates/slicer-ir/src/resolved_config.rs; echo "exit=$?"` | AC-14: the PnP spelling is retired and the two genuinely-prefixed keys survive | FACT exit code |
| `cargo test -p slicer-core --features host-algos --lib interlocking_beam_true_skips_phase5_driver 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-15: the rename preserves the Phase-5 suppression read site | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_contract stage_list_consistency_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-16: stage-order, host-only and non-module-targetable placement | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor prepass_interlocking 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-17 – AC-19: execution slot, downstream visibility, per-object gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_interlocking 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-20: end-to-end on a real painted 3MF | FACT pass/fail |
| `rg -q 'PrePass::InterlockingBeams' docs/04_host_scheduler.md && rg -q 'PrePass::InterlockingBeams' docs/01_system_architecture.md && rg -q 'interlocking_beam_width' docs/15_config_keys_reference.md && ls docs/adr/ \| rg -q 'interlocking'; echo "exit=$?"` | AC-23: doc impact | FACT exit code |
| `rg -qi 'declaration-only keys: 0' docs/spec_packets/306-interlocking-beams-slice-prepass/requirements.md && rg -qi 'queue count unchanged: 409' docs/spec_packets/306-interlocking-beams-slice-prepass/requirements.md; echo "exit=$?"` | AC-22: disposition table | FACT exit code |
| `cargo check --workspace --all-targets` | Compile gate across every target, including tests | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate — `InterlockingParams` is a new watched type | FACT exit code |

## Step Completion Expectations

- **The `InterlockingParams` fixture is shared, and it is a watched struct.** `InterlockingParams` is `pub` with six named fields under `crates/*/src`, so from the moment Step 2 introduces it every test literal must use a `..` rest or an `// exhaustive: <reason>` waiver, and `cargo xtask check-literals` must pass at the end of every subsequent step — not only at closure.
- **The rename in Step 4 is atomic across three files.** `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs` and `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs` must change in the same step; a partial rename leaves the workspace non-compiling. `docs/spec_packets/_OLD/96_paint-segmentation-phase5-width-limit.md` also mentions the old spelling and must **not** be edited — it is another packet's directory.
- **Steps 1–3 must not touch `crates/slicer-runtime`.** The kernel is provable standalone; wiring it before it is correct makes every failure ambiguous between kernel and seam.
- **`docs/07_implementation_status.md` is updated once, at the completion gate, through a worker dispatch** — it also carries two mentions of the retired key spelling.

## Context Discipline Notes

- `crates/slicer-ir/src/slice_ir.rs` and `crates/slicer-ir/src/resolved_config.rs` are both far over 600 lines. Never open either in full: `slice_ir.rs` only for the `ExPolygon` / `Polygon` / `Point2` / `SlicedRegion` / `SliceIR` / `RegionKey` / `RegionMapIR` declarations and the `mm_to_units` helpers; `resolved_config.rs` only for the MMU segmented-region `cli` block and the `to_config_map` omission comment.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` is very large. The rename touches three sites; locate them with `rg -n 'mmu_segmented_region_interlocking_beam'` and open ±40 lines around each. Do not read the module.
- `crates/slicer-runtime/src/prepass.rs` is over 1000 lines. Open only the `PrepassExecutionError` declaration, its `Display` impl arm list, and the `run_builtin_stage` call for `PrePass::PaintSegmentation` — the new registration goes immediately after it.
- The canonical algorithm is 946 lines across four C++ files. Dispatch it function by function (`SUMMARY` or a ≤30-line `SNIPPETS` return per function); a single "summarise the interlocking generator" dispatch will overflow its return budget and lose the details AC-1 – AC-6 assert.
