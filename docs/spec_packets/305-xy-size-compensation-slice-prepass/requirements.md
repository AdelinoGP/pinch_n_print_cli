# Requirements: 305-xy-size-compensation-slice-prepass

## Packet Metadata

- Grouped task IDs: none — this packet is queued by the wayfinder map, not by `docs/07_implementation_status.md`. Its backlog identity is wayfinder ticket 95 (P88).
- Backlog source: `docs/specs/orca-feature-gap/issues/95-author-packet-p88-quality-precision-new-contour-compensation.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer lets a user nudge every printed contour and every hole in the XY plane by an independent signed millimetre amount, so a part that comes off the bed a few hundredths oversized can be corrected without re-modelling it. Canonical does this in `PrintObject::_shrink_contour_holes`, applied from the compensation block inside `PrintObject::slice_volumes` (`PrintObjectSlice.cpp`), driven by `xy_contour_compensation` and `xy_hole_compensation` — both `PrintObjectConfig` members (`PrintConfig.hpp`), both `coFloat` defaulting to `0`, neither carrying a `min` or a `max`. This tree implements none of it: the only occurrence of either key in any **code** path is the pair of `ORCA_CONFIG_PADDING` twins in `crates/slicer-gcode/src/serialize.rs`, which map rule 2 makes non-evidence. Every other occurrence is documentation — `docs/ORCA_CONFIG_REFERENCE.md`, the gap-inventory assets, and packet 303's cross-reference — none of which is a read site. A user setting either key today gets a padded CONFIG_BLOCK line and no geometry change.

The slice is coherent because both keys are read by the same canonical function, in the same pass, over the same geometry, in the same two-pass positive-then-negative order, and neither means anything without the other: `_shrink_contour_holes` takes them as one `(contour_delta, hole_delta)` pair and the branch that fires depends on the sign of both.

**The one design decision this packet had to make is where the pass lives, and the answer overturns the tier table.** `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` assigns both keys to a "new contour-compensation module" (Tier C), and wayfinder ticket 93 handed this ticket an ordering obligation on the assumption it would land on `Layer::SlicePostProcess` beside packet 303's elephant-foot module. That owner cannot work, for three reasons, any one of which is sufficient:

1. **Every prepass consumer of the slice footprint would see uncompensated geometry.** `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) runs `PrePass::OverhangAnnotation`, `PrePass::ShellClassification`, `PrePass::SupportAnalysis`, `PrePass::SupportGeometry` and `PrePass::LightningTreeGen` **before** `Layer::SlicePostProcess`, and all of them read the committed `SliceIR`. Canonical applies XY compensation inside `slice_volumes`, before `detect_surfaces_type` and before support generation, so every downstream consumer sees the compensated footprint. A module at `Layer::SlicePostProcess` would leave overhang bands, top/bottom shell classification, support analysis, support geometry and lightning trees all computed against the uncompensated outline — on **every** layer, at a magnitude the user chooses. This is materially worse than the same concern for elephant-foot, which is bounded to the first `elefant_foot_compensation_layers` layers and to a shrink small enough that canonical itself limits it by contour width. AC-12 pins the fix and is unsatisfiable at the module seam.
2. **A second coarse `SliceIR` mutator cannot be ordered against packet 303's on that stage — it deadlocks the scheduler.** The per-stage DAG in `crates/slicer-scheduler/src/dag.rs` emits an `EdgeReason::IrWriteRead` edge from a writer to a reader whenever `reader.ir_reads()` contains the writer's exact write path. Packet 303's `elefant-foot` manifest declares `reads = ["SliceIR"]` and `writes = ["SliceIR"]`. A contour-compensation module with the same honest access mask produces edges in **both** directions, and `validate_cycles` (`crates/slicer-scheduler/src/validation.rs`) then fails `topological_sort` with `SchedulerError::CyclicDependency`. No pair of modules in this tree does that today: `seam-placer` sits beside `fuzzy-skin` on `Layer::PerimetersPostProcess` and avoids it only by declaring narrow writes (`PerimeterIR.resolved-seam`, `PerimeterIR.regions.walls`) against `fuzzy-skin`'s coarse `PerimeterIR` read, and `top-surface-ironing` establishes the same lesson from the other side, ordering itself after the infill modules with `[compatibility].requires` rather than with a fake `InfillIR` read (`modules/core-modules/top-surface-ironing/top-surface-ironing.toml`, and `docs/04_host_scheduler.md`). That comment cites `DEV-065`, which no longer exists as a row in `docs/DEVIATION_LOG.md` — it was removed with the closed entries — so treat the manifest and the doc as the sources, not the retired ID. Escaping the cycle would mean rewriting packet 303's manifest, which this packet must not do.
3. **Canonical's positive-growth branch is per object, not per region.** When a layer carries more than one region, canonical merges the whole object's layer, runs `_shrink_contour_holes` once on the merged expolygons, then re-derives each region as `intersection(offset(region, max_growth), merged)` and trims it by the union of the regions already processed, giving priority to the lower region index. The key itself is a `PrintObjectConfig` key — one value per object, not per region. A host prepass built-in holds the whole `Vec<SliceIR>` and can group by `object_id` directly.

The owner is therefore corrected to a host prepass built-in on a new host-only stage — the same correction ticket 94 made for packet 304, and the same staging shape packet 297 chose, with the reason differing (297 and 304 needed cross-layer reads; this pass needs to run **upstream of the prepass consumers**).

No packet is superseded or reopened.

## In Scope

- **Ungated pure kernel** `crates/slicer-core/src/algos/xy_size_compensation.rs`, opening with the standard porting header from `docs/ORCASLICER_ATTRIBUTION.md` (original C++ source path `src/libslic3r/PrintObjectSlice.cpp`), declared as `pub mod xy_size_compensation;` in `crates/slicer-core/src/algos/mod.rs` beside the ungated `bridge_over_infill`. Ungated deliberately: the kernel needs neither `rayon` nor `boostvoronoi`, `polygon_ops` is itself ungated, and an ungated module cannot fall into the CLAUDE.md silent-zero-tests trap.
  - `pub struct XySizeCompensationParams { contour_delta_mm: f32, hole_delta_mm: f32 }` — the per-object resolution of the two keys and nothing else.
  - `pub fn shrink_contour_holes(polys: &[ExPolygon], contour_delta_mm: f32, hole_delta_mm: f32) -> Vec<ExPolygon>` — canonical's `_shrink_contour_holes`: per input expolygon, offset the contour by `contour_delta_mm`, offset each hole so that a **positive** `hole_delta_mm` enlarges the hole aperture, difference the offset holes out of the offset contour, and union the accumulated result. A contour whose offset yields nothing is dropped whole, with its holes (AC-6).
  - `pub fn compensate_object_layer(regions: &mut [(RegionId, Vec<ExPolygon>)], params: &XySizeCompensationParams) -> bool` — canonical's control flow for one object on one layer: early-out when both deltas are zero; the single-printable-region branch; the multi-region branch with its positive merge-and-re-split pass and its negative merged-trimming pass; `MODIFIER_FOOTPRINT_REGION_ID` regions passed through untouched and excluded from every merge. Returns whether any region changed.
- **New host-only stage** `PrePass::XySizeCompensation`: one entry in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) between `PrePass::Slice` and `PrePass::OverhangAnnotation`; one entry in `HOST_ONLY_STAGES` (`crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`); one arm in `required_slots` (`crates/slicer-runtime/src/prepass.rs`) listing `LayerPlan`, `RegionMap` and `SliceIR`. `slicer_schema::VALID_STAGES` and `slicer_schema::STAGES` are deliberately **not** touched, which is what keeps the WIT surface, the `slicer-macros` glue match, the `pnp_cli module new` scaffold arm and `stage_io.rs`'s commit match out of scope.
- **Producer** `crates/slicer-runtime/src/builtins/xy_size_compensation_producer.rs` — `pub fn commit_xy_size_compensation_builtin(blackboard: &mut Blackboard) -> Result<(), XySizeCompensationBuiltinError>`, declared in `crates/slicer-runtime/src/builtins/mod.rs`, registered by one `run_builtin_stage` call in `prepass.rs` with `should_run = |bb| bb.slice_ir().is_some() && bb.region_map().is_some()`, resolving the pair through `RegionMapIR::config_for` per `RegionKey`, grouping each layer's regions by `object_id`, and writing back through `Blackboard::replace_slice_ir`. One new `PrepassExecutionError::XySizeCompensation` variant.
- **Config plumbing** in `crates/slicer-ir/src/resolved_config.rs`: two `cli` rows plus their `to_config_map` emissions, at canonical defaults — `xy_contour_compensation: f32 = 0.0` and `xy_hole_compensation: f32 = 0.0`. No bounds: canonical declares none and ticket 113 forbids inventing them.
- **Tests**: `crates/slicer-core/tests/algo_xy_size_compensation_tdd.rs` (auto-discovered, no `Cargo.toml` entry, no `required-features`, no file-level `cfg`), `crates/slicer-ir/tests/resolved_config_xy_size_compensation_tdd.rs`, and `crates/slicer-runtime/tests/executor/prepass_xy_size_compensation_stage_order_tdd.rs` plus its one `mod` line in `crates/slicer-runtime/tests/executor/main.rs`.
- **Docs and ledger**: the stage list in `docs/04_host_scheduler.md` and `docs/01_system_architecture.md` (numbered list and Data Dependency Matrix); `docs/15_config_keys_reference.md` regenerated via `cargo xtask gen-config-docs`; one `docs/DEVIATION_LOG.md` row with the ID re-derived at write time; the P88 rows in `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (owner corrected) and `05-asset-packet-list.md` (P88 linked to this packet).

### Key disposition table

Declaration-only keys: 0

| Key | Canonical type / default | Owner after this packet | Decision point it drives | Disposition |
| --- | --- | --- | --- | --- |
| `xy_contour_compensation` | `coFloat`, default `0`, no `min`, no `max` | `host:xy_size_compensation` built-in | the contour offset inside `shrink_contour_holes`, and the sign that selects canonical's positive-growth merge branch vs. its negative trimming branch | wired — AC-1 / AC-3 / AC-4 / AC-6 / AC-7 / AC-8 / AC-11 / AC-12 / AC-13 |
| `xy_hole_compensation` | `coFloat`, default `0`, no `min`, no `max` | `host:xy_size_compensation` built-in | the per-hole offset inside `shrink_contour_holes`, and its contribution to `max_growth` / `min_growth` in the multi-region branch | wired — AC-2 / AC-3 / AC-4 / AC-7 / AC-8 |

Both keys are P88 queue keys and both become live in this packet. No supporting non-queue key is carried, and no key is added to or removed from the queue. **Queue count unchanged: 409.**

## Out of Scope

- **Canonical's two painted-object suppressions and their warnings.** `slice_volumes` zeroes both deltas and raises a `WarningLevel::CRITICAL` `active_step_add_warning` when the object is multi-material painted, and raises a second warning when it is fuzzy-skin painted. Neither is ported. The mm-painted arm is conjoined with `num_extruders > 1`, an extruder count this tree cannot resolve at prepass (wayfinder ticket 118 measured `tool-count()` returning 1 because no core module declares `filament_density`); and a paint predicate here has to cover two channels, because a semantic declared `[[region_split]]` surfaces through `SlicedRegion.variant_chain` while an undeclared one surfaces through `SlicedRegion.segment_annotations` (`crates/slicer-ir/src/slice_ir.rs`). Recorded as a deviation clause, not smuggled in as a guess.
- **Canonical's region-assignment bbox growth.** `PrintApply.cpp` passes `xy_contour_compensation` into `update_volume_bboxes` / `generate_print_object_regions` so region assignment stays correct under a positive contour compensation. This tree assigns regions at `PrePass::RegionMapping`, before `PrePass::Slice`, and grows no bbox. Recorded as a deviation clause; the observable effect is confined to objects with modifier volumes or layer-config ranges **and** a positive contour compensation.
- **Compensating the modifier footprint.** `MODIFIER_FOOTPRINT_REGION_ID` regions are host-internal bookkeeping consumed by `sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`) after `Layer::Perimeters`, i.e. long after this stage. They are passed through untouched (AC-N1) and the resulting split-against-uncompensated-bounds gap is a deviation clause.
- **The elephant-foot seam question.** Packet 303 places elephant-foot on `Layer::SlicePostProcess`; whether that is the right home is an open wayfinder question. This packet neither depends on the answer nor moves anything: prepass precedes every `Layer::` stage, so XY compensation runs before elephant-foot under either resolution, which is canonical's order.
- **Making the stage module-targetable.** No `VALID_STAGES` entry, no `slicer_schema::STAGES` row, no WIT world, no SDK trait, no `pnp_cli module new` scaffold arm. A fork wanting to replace this pass changes the built-in, not a manifest.
- **Visual-debug taps for the new stage.** `SILHOUETTE_TAP_STAGE_IDS` (`crates/pnp-cli/src/visual_debug.rs`) and the `BLACKBOARD_TAP_STAGE_IDS` list (`crates/slicer-runtime/src/layer_executor.rs`) are not extended; the pass's effect is observable on the existing post-prepass `SliceIR` taps.
- **Any `crates/slicer-gcode/src/serialize.rs` edit.** `ORCA_CONFIG_PADDING` is map rule 2 non-evidence; CONFIG_BLOCK emission for these keys rides as a side effect of the `ResolvedConfig` fields being live, and the padding-twin spelling stays with ticket 132.
- **Range validation.** Canonical declares no `min` and no `max` on either key. Under the ticket-113 rule this packet invents neither (AC-N3).
- **The per-tool axis.** Both keys are per-object in canonical and are carried by the existing per-object overlay. This packet builds no second per-tool mechanism and is not blocked on ticket 125.

## Authoritative Docs

- `docs/04_host_scheduler.md` - over 300 lines; delegate a SUMMARY of the "Fixed Stage Order" section, then edit that list.
- `docs/01_system_architecture.md` - over 300 lines; delegate a SUMMARY of the numbered stage list and the Data Dependency Matrix; edit only those two places.
- `docs/08_coordinate_system.md` - direct read; 1 unit = 100 nm. Every canonical constant divides by 100, and `slicer_core::polygon_ops::offset` takes **millimetres**, not internal units — the most likely arithmetic slip in this packet.
- `docs/ORCASLICER_ATTRIBUTION.md` - direct read of the "Standard Porting Header" block; the kernel file must open with it verbatim.
- `docs/02_ir_schemas.md` - the `SliceIR` / `SlicedRegion` section only; the fields this pass mutates and the ones it must leave alone.

## Parity Evidence Standard

Ticket 02's standard applies: canonical is **readable, not runnable**, so parity is evidenced by a canonical function read plus invariant tests, never by a golden fixture captured from a run that cannot be reproduced. Bit-identity with canonical is **not** claimed — this tree's Clipper2 offset, its arc tolerance and its 100 nm quantisation all differ from canonical's ClipperLib path. The invariants that carry the parity claim are the bounding-box deltas (AC-1 through AC-4), the drop-on-annihilation rule (AC-6), the region-priority non-overlap property (AC-7), the area-monotonicity of the negative branch (AC-8), and default-path bitwise identity (AC-5). Any behaviour that cannot be verified against the canonical read surfaces to the human before it is guessed at; it never blocks the packet.

## OrcaSlicer Reference Obligations

Delegated per `packet.spec.md` §OrcaSlicer Reference Obligations. The implementer never loads `OrcaSlicerDocumented/` directly. Citations in this packet and in any code comment it produces name the **file and function** — `PrintObject::_shrink_contour_holes` (`PrintObjectSlice.cpp`) — never a line number, per CLAUDE.md.

## Acceptance Summary

| AC | What it proves | Step |
| --- | --- | --- |
| AC-1, AC-2, AC-3 | `shrink_contour_holes` moves contours and holes independently, in the right direction, by the right amount — including the clockwise-hole sign trap | Step 1 |
| AC-6 | the empty-offset-result drop rule | Step 1 |
| AC-4, AC-5 | the two-pass positive-then-negative order, and default-path bitwise identity | Step 2 |
| AC-7, AC-8 | canonical's multi-region merge-and-re-split and merged-trimming branches | Step 2 |
| AC-N1, AC-N2 | modifier footprints excluded; annihilation is empty, not invalid | Step 2 |
| AC-10 | the stage exists, is ordered correctly, and is host-only | Step 3 |
| AC-9, AC-N3 | both keys are real `ResolvedConfig` fields at canonical defaults, unbounded | Step 4 |
| AC-11, AC-12, AC-13 | the config reaches the kernel, the blackboard is updated, shell classification consumes the compensated footprint, and the gate is per object | Step 5 |
| AC-16 | docs record the new stage and the deviation is discoverable | Step 6 |
| AC-14 | the kernel and its test binary are ungated | Steps 1-2, re-checked at the gate |
| AC-15 | zero declaration-only keys; queue count unchanged | Step 7 |

## Verification Commands

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p slicer-core --test algo_xy_size_compensation_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"`
- `cargo test -p slicer-ir --test resolved_config_xy_size_compensation_tdd 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"`
- `cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"`
- `cargo test -p slicer-scheduler --test scheduler_contract 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"`
- `cargo xtask gen-config-docs --check`
- `cargo xtask build-guests --check` — expect exit `0`. This packet adds no guest and edits no guest input, so a stale report means the tree was already stale; reconcile before blaming this work.

Every run tees to `target/test-output.log` per CLAUDE.md, and the log is read rather than the tests re-run. Note the log is overwritten each run: capture findings before launching the next.

## Step Completion Expectations

Each step ends green on its own AC commands plus `cargo check --workspace --all-targets`. No step may be reported complete on a narrow run alone where the crate has non-default features — `slicer-core` does (`host-algos`), which is exactly why AC-14 pins this kernel and its test binary as ungated: the bare `-p slicer-core` run must report a non-zero test count, not a silent `ok` on zero tests.

## Context Discipline Notes

Per `packet.spec.md` §Context Discipline Note. The kernel is the only genuinely large read in this packet and it is split across Steps 1 and 2; the canonical control-flow block is quoted in bounded snippets rather than summarised, because the branch structure is the parity claim.
