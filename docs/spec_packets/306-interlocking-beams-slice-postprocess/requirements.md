# Requirements: 306-interlocking-beams-slice-postprocess

## Packet Metadata

- Grouped task IDs: none — this is a queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap", not a `docs/07` slice. Its backlog row is wayfinder ticket 96 (P89), with ticket 97 (P90) dissolved into it.
- Backlog source: `docs/specs/orca-feature-gap/issues/96-author-packet-p89-multimaterial-multimaterial-advanced-new-interlocking.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's beam interlocking generates a physical dovetail lattice where two filaments meet inside one object, so a multi-material part does not delaminate along the colour boundary. This port has none of it. Of the six keys, five — `interlocking_beam_layer_count`, `interlocking_beam_width`, `interlocking_depth`, `interlocking_orientation`, `interlocking_boundary_avoidance` — have **zero occurrences** anywhere in `crates/` or `modules/`, including `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`), which carries no twin for any of them. The sixth, `interlocking_beam`, is live but **misnamed and half-implemented**.

**The six are one packet, not two.** Canonical's enabling gate reads four of them in a single condition — `!interlocking_beam || interlocking_beam_layer_count < 1 || interlocking_depth < 1 || interlocking_beam_width < EPSILON` — and `interlocking_depth` sits in the P90 trio. The remaining two P90 keys are no more separable: `interlocking_orientation` is applied before the voxel walk and unapplied on every output; `interlocking_boundary_avoidance` selects the `air_filtering` branch that owns the air dilation **and** the whole of `handleThinAreas` / `growBorderAreasPerpendicular`. A P89-only packet would hardcode all three and a P90 packet would unpick them, rewriting every acceptance test for no gain. Ticket 97 is dissolved into this packet on the ticket-31 (P24) precedent.

**`interlocking_beam` is already in this tree under a name canonical does not have.** `crates/slicer-ir/src/resolved_config.rs` declares `mmu_segmented_region_interlocking_beam`; canonical `PrintConfig.cpp` / `PrintConfig.hpp` declare `interlocking_beam` as a `PrintObjectConfig` member and no `mmu_segmented_region_interlocking_beam` (whereas `mmu_segmented_region_interlocking_depth` and `mmu_segmented_region_max_width` genuinely do carry that prefix). Under the invented name the key's **second** canonical read site is already correct: `multi_material_segmentation_by_painting` (`MultiMaterialSegmentation.cpp`) suppresses `cut_segmented_layers` when it is set, and `run_phase5_width_limit` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) does exactly that. What is missing is the **first** read site — the generator gate — and the generator behind it. This packet renames the key and adds the missing read site; it does not re-home Phase 5.

### Ticket 04's owner is held: the microstructure is a module, on `Layer::SlicePostProcess`

Ticket 04 assigns these keys to a `new interlocking module`, Tier C. Re-derived at claim time per ticket 27, **that owner is correct**, and the seam is `Layer::SlicePostProcess`:

- **The stage's commit type is a precise fit.** `LayerStageCommit::SlicePostProcess { polygon_updates, path_z_updates }` (`crates/slicer-ir/src/stage_io.rs`, merged in `crates/slicer-runtime/src/layer_executor.rs`) carries `polygon_updates: Vec<(RegionKey, Vec<ExPolygon>)>`, and the host replaces `existing.regions[ridx].polygons` with each entry. Canonical's two write points — `applyMicrostructureToOutlines`' `slices.set(union_ex(diff_ex(polys, areas_other), areas_here))` and `handleThinAreas`' `slices.set(closing_ex(...))` — are exactly "replace this region's polygons". The `RegionKey` even carries `variant_chain`, which is how the two material variants being interlocked are told apart.
- **`Layer::SlicePostProcess` is module-targetable and empty.** It is present in both `STAGE_ORDER` and `VALID_STAGES` (`crates/slicer-schema/src/lib.rs`) and carries **zero** production modules today. Ticket 148 asks what the seam is for and notes packet 303 would be its first occupant; this packet supplies the second and a worked answer.
- **Rule 4 points here, not away.** The swappable part of interlocking is the beam **pattern** — canonical ships one (alternating half-cell beams on orthogonal phases). Putting the pattern in a guest module is what makes a community dovetail variant possible; freezing it in host code is what makes it impossible. The map's own rule is to use the mechanics this tree has, and the module seam is one of them.

### Only the analysis is host-side, and that is an established idiom

The algorithm splits cleanly, and canonical's own function boundaries are the split:

| canonical function | needs | lands |
| --- | --- | --- |
| `getShellVoxels`, `addBoundaryCells` | every layer — `skin = xor_ex(layers[n], layers[n-1])` reads the layer below | host prepass |
| air-cell erase (`addBoundaryCells` on `air_dilation` + `has_all_meshes.erase`) | every layer | host prepass |
| `generateMicrostructure` | nothing but config | module |
| `applyMicrostructureToOutlines` | layer N's polygons + the cell set | module |
| `growBorderAreasPerpendicular`, `handleThinAreas` | layer N's polygons + the cell set + both regions' wall widths | module |

`computeUnionedVolumeRegions` looks global but is not: `layer_regions[N]` is the union of that layer's two regions closed by `ignored_gap_`, derivable by the module from its own layer. The ghost layer canonical allocates exists only so the topmost **skin** computes, which is analysis.

So: a host built-in `host:interlocking_lattice` on a new host-only stage `PrePass::InterlockingLattice` does the voxel analysis and commits a new `InterlockingLatticeIR`; the guest module reads it and does everything else per layer. **This is not a new pattern.** `LightningTreeIR` is a global prepass product (`PrePass::LightningTreeGen`) threaded to a per-layer guest module through `LayerStageInput.lightning_tree_ir` (`crates/slicer-wasm-host/src/binding.rs`); `SeamPlanIR`, `SupportPlanIR` and `SurfaceClassificationIR` are the same shape. Interlocking is the fifth instance, not an exception.

**The honest cost of this design, stated rather than hidden:** `InterlockingLatticeIR` is a new IR type crossing the WIT boundary, so it needs a schema registration, a WIT record, a `LayerStageInput` field and a guest rebuild. A host-only design would need none of that. The packet pays it because the alternative freezes a swappable algorithm in the host and leaves the module seam unoccupied.

### The DAG cycle is a narrow-writes problem, and it is solved in this packet

Packet 303 plans `elefant-foot` on the same stage with coarse `reads = ["SliceIR"]` / `writes = ["SliceIR"]`. The per-stage DAG (`crates/slicer-scheduler/src/dag.rs`) emits an `EdgeReason::IrWriteRead` edge whenever a reader's `ir_reads()` contains a writer's write path by exact `Vec<String>::contains`, so two coarse `SliceIR` reader-writers produce edges both ways and `validate_cycles` (`crates/slicer-scheduler/src/validation.rs`) → `topological_sort` (`crates/slicer-scheduler/src/topology.rs`) fails with `SchedulerError::CyclicDependency`. The escape is the one already shipping: **narrow declared writes.** `seam-placer.toml` declares `writes = ["PerimeterIR.resolved-seam", "PerimeterIR.regions.walls"]` beside `fuzzy-skin`'s coarse `PerimeterIR`; `part-cooling.toml` declares `LayerCollectionIR.cooling`; `skirt-brim.toml` declares `LayerCollectionIR.skirt-brim`. This module declares `writes = ["SliceIR.regions.polygons"]`, which is both honest (it writes nothing else) and orderable against 303 unchanged. AC-N6 asserts the pair validates.

## In Scope

- Six queue keys as `ResolvedConfig` `cli` fields with canonical types and defaults — `interlocking_beam: bool = false`, `interlocking_beam_width: f32 = 0.8` (mm), `interlocking_beam_layer_count: i32 = 2`, `interlocking_depth: i32 = 2` (cells), `interlocking_orientation: f32 = 22.5` (degrees), `interlocking_boundary_avoidance: i32 = 2` (cells) — **and** as `[config.schema]` rows in the new module's manifest. Both are required: the host prepass reads five of them for the analysis, and `ConfigView::from_declared` (`crates/slicer-ir/src/slice_ir.rs`) filters the raw source to the module's declared keys, so an undeclared key silently loses to the guest's `unwrap_or` fallback.
- Retiring `mmu_segmented_region_interlocking_beam` to canonical `interlocking_beam` — a straight rename with no alias, across its nine code sites in three files, preserving the Phase-5 suppression behaviour unchanged.
- A new ungated **analysis** kernel `crates/slicer-core/src/algos/interlocking/` with `voxel.rs` (`GridPoint3`, `Vec3Units`, `DilationKernel`, `DilationKernelType::{Cube, Diamond, Prism}`, `VoxelGrid` and its walk functions) and `mod.rs` (`InterlockingParams`, `InterlockingLattice`, `compute_interlocking_lattice`, `get_shell_voxels`, `add_boundary_cells`). Both carry the porting header from `docs/ORCASLICER_ATTRIBUTION.md`.
- A new IR `InterlockingLatticeIR` — `cell_size: Vec3Units`, `rotation_rad: f32`, and per `object_id` a `Vec<InterlockingPair { region_key_a, region_key_b, cells: Vec<GridPoint3> }>` — registered in the schema, exposed over WIT, and threaded onto `LayerStageInput`.
- A new host built-in `crates/slicer-runtime/src/builtins/interlocking_lattice_producer.rs` on a new host-only stage `PrePass::InterlockingLattice`, registered after the `PrePass::PaintSegmentation` built-in, plus a `PrepassExecutionError::InterlockingLattice` variant and its `Display` arm.
- A new guest module `modules/core-modules/interlocking-beams/` on `Layer::SlicePostProcess`, scaffolded with `pnp_cli module new`, owning `generate_microstructure`, `apply_microstructure_to_layer`, `grow_border_areas_perpendicular` and `handle_thin_areas`, and returning `polygon_updates`.
- Per-region external-perimeter width from `resolve_role_width(ExtrusionRole::OuterWall, false, false, &ctx)` (`crates/slicer-core/src/flow.rs`), standing in for canonical's `printing_region.flow(print_object, frExternalPerimeter, 0.1).scaled_width()`.
- Region-pair identity from each region's `variant_chain` `("material", PaintValue::ToolIndex(n))` entry; regions with no material entry take the object's base tool; pairs with equal tool index are skipped, as canonical skips equal-extruder pairs.
- Two `docs/DEVIATION_LOG.md` rows and one ADR (analysis-in-prepass / apply-in-module split; variant-chain region-pair identity).
- Doc edits per `packet.spec.md` §Doc Impact Statement.

### Key disposition table

| Queue key | Canonical type / default | Canonical read site | Decision point after this packet | Disposition |
| --- | --- | --- | --- | --- |
| `interlocking_beam` | `coBool` / `false` | `generate_interlocking_structure` gate; `multi_material_segmentation_by_painting`'s `!interlocking_beam` guard | the lattice prepass gate, the module's emit gate, **and** the existing `run_phase5_width_limit` suppression, now under the canonical name | live (renamed + read site added) |
| `interlocking_beam_width` | `coFloat` / `0.8` mm | `scaled(...)` into `cell_width` and the microstructure split | `VoxelGrid` cell XY size (prepass) and `generate_microstructure`'s `middle` (module) | live |
| `interlocking_beam_layer_count` | `coInt` / `2` | `cell_size.z()` and the structure-layer stride | cell Z size (prepass) and the module's `(layer / blc) % 2` phase select | live |
| `interlocking_depth` | `coInt` / `2` cells | `interface_dilation` kernel size | the `Prism` interface kernel in `get_shell_voxels` (prepass) | live |
| `interlocking_orientation` | `coFloat` / `22.5`° | applied and unapplied around the walk | the walk rotation (prepass) and the apply/unapply pair around the intersection (module) | live |
| `interlocking_boundary_avoidance` | `coInt` / `2` cells | `air_filtering` switch and `air_dilation` kernel size | the air-cell erase (prepass) and the `handle_thin_areas` branch (module) | live |

**Declaration-only keys: 0.** Every one of the six drives a behaviour-changing decision point proven by an AC at a non-default value (AC-5 – AC-7, AC-10 – AC-15, AC-N1 – AC-N3). **Queue count unchanged: 409** — six keys move from unimplemented to implemented; none is ruled out of scope and no supporting non-queue key is added.

## Out of Scope

- **Re-siting packet 303's `elefant-foot`.** It stays on `Layer::SlicePostProcess` with its coarse manifest; this packet's narrow writes make the pair orderable without amending it. Where the other canonical slice-mutating passes belong remains ticket 148's ruling.
- **Canonical's XY-size-compensation suppression for mm-painted objects.** Packet 305 owns those keys and deliberately does not port it; this packet does not add it.
- **Reconciling `PrePass::PaintSegmentation`'s `STAGE_ORDER` slot with its true execution position.** Pre-existing and unrelated; the new prepass stage is placed where the declared and executed orders agree.
- **Re-homing Phase 5 or its two `mmu_segmented_region_*` keys** — wayfinder ticket 98's (P91).
- **Emitting any of the six keys into the CONFIG_BLOCK.** `to_config_map` already omits the `mmu_segmented_region_*` trio to keep G-code bytes stable; the six stay on the same footing. Ticket 132 owns the reader contract.
- **Adopting canonical's `min` / `max` on the five bounded keys** — ticket 113 measured that canonical never enforces them.
- **`DilationKernelType::Cube` / `Diamond` beyond construction.** Canonical's interlocking path only builds `PRISM`; the other two are ported for shape fidelity and covered by AC-1's structure assertion only.
- **Shipping a second beam pattern.** The module makes one possible; this packet ports canonical's and no other.

## Authoritative Docs

- `docs/01_system_architecture.md` — direct ranged read of the prepass stage list; edited.
- `docs/02_ir_schemas.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the IR-registration anchor, then a ranged edit.
- `docs/03_wit_and_manifest.md` — direct ranged read of the `[ir-access]` grammar and §Known claim IDs.
- `docs/04_host_scheduler.md` — direct ranged read of the fixed stage-order section; edited.
- `docs/05_module_sdk.md` — direct ranged read of the stage-entrypoint signature.
- `docs/08_coordinate_system.md` — direct ranged read of the porting checklist.
- `docs/15_config_keys_reference.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the Multimaterial anchor.
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

- `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/InterlockingGenerator.cpp` — the whole algorithm. Note which functions are **analysis** (`getShellVoxels`, `addBoundaryCells`, the air-cell erase) and which are **application** (`generateMicrostructure`, `applyMicrostructureToOutlines`, `growBorderAreasPerpendicular`, `handleThinAreas`) — this packet's seam runs exactly along that line.
- `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.cpp` — `DilationKernel`'s three kernel shapes and the `walkLine` / `walkPolygons` / `walkAreas` / `walkDilatedPolygons` / `walkDilatedAreas` / `dilate` grid walk, including the half-cell XY translation `_walkAreas` assumes.
- `OrcaSlicerDocumented/src/libslic3r/Feature/Interlocking/VoxelUtils.hpp` — `toGridCoord`'s negative-coordinate floor and `toLowerCorner` / `toPolygon`; borrowed exactly.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — the single call site of `generate_interlocking_structure` and its position relative to `apply_mm_segmentation`.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` — `multi_material_segmentation_by_painting`'s `!interlocking_beam` guard on `cut_segmented_layers`; the key's **second** read site.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the six `ConfigOptionDef`s: types, defaults, and the `min` / `max` values this packet deliberately does **not** adopt (ticket 113).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-25`. Refinements absent from their Given/When/Then text: AC-1 – AC-4 are pure grid-arithmetic unit assertions runnable before any lattice exists, so they are the TDD front of Step 1; AC-5 – AC-7 and AC-N1 – AC-N3, AC-N5 share the 8-layer two-tool 20 mm fixture that Step 2 authors once; AC-10 – AC-15 are module-crate tests that need no host at all and can run before the seam is wired; AC-8, AC-9, AC-17, AC-18, AC-22 and AC-N6 need the full wiring and cannot pass before Step 8.
- Negative: `AC-N1` through `AC-N6`.
- Cross-packet impact: **AC-N6 is the load-bearing one for packet 303.** It asserts that a narrow-write module and a coarse `SliceIR` mutator coexist on `Layer::SlicePostProcess`. If it fails, either this module's write path is not actually narrow or the DAG does not honour dotted paths the way `seam-placer` implies — and in that case ticket 148's ruling becomes a blocker for this packet rather than a neighbour. Packet 305's `PrePass::XySizeCompensation` and this packet's `PrePass::InterlockingLattice` both insert into `STAGE_ORDER`; whichever merges second re-runs the scheduler contract test, which asserts positions rather than membership.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --test algo_interlocking_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-1 – AC-7, AC-23, AC-N1 – AC-N3, AC-N5: the analysis kernel | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p interlocking-beams 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-10 – AC-15: the module's microstructure and set algebra | FACT pass/fail |
| `cargo test -p slicer-ir --test resolved_config_interlocking_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-19, AC-N4: the six keys resolve with canonical defaults and no invented bounds | FACT pass/fail |
| `! rg -q 'mmu_segmented_region_interlocking_beam' crates modules && rg -q 'mmu_segmented_region_max_width' crates/slicer-ir/src/resolved_config.rs; echo "exit=$?"` | AC-20: the PnP spelling is retired and the genuinely-prefixed keys survive | FACT exit code |
| `cargo test -p slicer-core --features host-algos --lib interlocking_beam_true_skips_phase5_driver 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-21: the rename preserves the Phase-5 suppression read site | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_contract stage_list_consistency_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-9: stage-order and host-only placement of the new prepass stage | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_unit interlocking_and_coarse_slice_mutator_coexist_on_slice_postprocess 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-N6: narrow writes let this module share the stage with packet 303 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor prepass_interlocking_lattice 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-8: the analysis prepass runs in the right slot and does not mutate slices | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor layer_interlocking_beams 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-17, AC-18: `polygon_updates` merge onto the right regions; per-object gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_interlocking 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | AC-22: end-to-end through real guest dispatch | FACT pass/fail |
| `cargo xtask build-guests --check` | A new guest module is added; exit `0` fresh, `1` stale, `3` `wasm-tools` missing | FACT exit code |
| `cargo check --workspace --all-targets` | Compile gate across every target | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate — `InterlockingParams` and `InterlockingPair` are new watched types | FACT exit code |

## Step Completion Expectations

- **The new IR is the widest blast radius in this packet, and it is a WIT change.** `InterlockingLatticeIR` touches the schema, the canonical WIT under `crates/slicer-schema/wit/`, the host `bindgen!` side, the guest macro side, `LayerStageInput`, and every guest that must rebuild. Follow `CLAUDE.md` §"WIT/Type Changes Checklist" — search `wit_host.rs`, `dispatch.rs` and `wit_guest` for the affected type, verify type identity across the boundary, and run `cargo build --tests` after. Step 5 owns this and nothing else.
- **`cargo xtask build-guests --check` must pass at the end of every step from Step 5 onward**, judged by exit code. A new guest module plus a WIT change is the exact combination that produces "unrelated-looking" test failures.
- **The two new watched structs are watched from the moment they exist.** `InterlockingParams` and `InterlockingPair` are `pub` with ≥5 named fields under `crates/*/src`; from Step 2 onward every test literal needs a `..` rest or an `// exhaustive: <reason>` waiver, and `cargo xtask check-literals` must pass at the end of each step, not only at closure.
- **The rename in Step 4 is atomic across three files.** A partial rename leaves the workspace non-compiling in two of them and silently non-functional in the third (`resolved_config.rs`'s hand-written `PartialEq`). `docs/spec_packets/_OLD/96_paint-segmentation-phase5-width-limit.md` also mentions the old spelling and must **not** be edited — it is another packet's directory.
- **Steps 1–3 must not touch `crates/slicer-runtime` or `modules/`.** The analysis kernel is provable standalone.
- **`docs/07_implementation_status.md` is updated once, at the completion gate, through a worker dispatch** — it also carries two mentions of the retired key spelling.

## Context Discipline Notes

- `crates/slicer-ir/src/slice_ir.rs` and `crates/slicer-ir/src/resolved_config.rs` are both far over 600 lines. Never open either in full: `slice_ir.rs` only for the `ExPolygon` / `Polygon` / `Point2` / `SlicedRegion` / `SliceIR` / `RegionKey` / `RegionMapIR` / `ConfigView::from_declared` declarations and `mm_to_units`; `resolved_config.rs` only for the MMU segmented-region `cli` block and the `to_config_map` omission comment.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` is very large. The rename touches three sites; locate them with `rg -n 'mmu_segmented_region_interlocking_beam'` and open ±40 lines around each. Do not read the module.
- `crates/slicer-runtime/src/layer_executor.rs` is over 4000 lines. Open only the `LayerStageCommit::SlicePostProcess` merge arm and the `Layer::SlicePostProcess` dispatch entry; both are locatable by `rg -n 'SlicePostProcess'`.
- `crates/slicer-runtime/src/prepass.rs` is over 1000 lines. Open only the `PrepassExecutionError` declaration, its `Display` arm list, and the `PrePass::PaintSegmentation` `run_builtin_stage` block.
- The canonical algorithm is 946 lines across four C++ files. Dispatch it function by function (`SUMMARY`, or a ≤30-line `SNIPPETS` return per function); a single "summarise the interlocking generator" dispatch will overflow its return budget and lose the details AC-1 – AC-4 and AC-10 – AC-14 assert.
- Use an existing core module as the manifest and entrypoint template — `seam-placer` for the narrow-write `[ir-access]` shape, `lightning-infill` for consuming a prepass IR in a layer stage. Read those two manifests, not the whole module tree.
