---
status: implemented
packet: 240a-support-raft-substrate
task_ids:
  - TASK-409
  - TASK-410
  - TASK-411
  - TASK-412
  - TASK-413
  - TASK-533
  - TASK-534
  - TASK-535
  - TASK-536
---

# 240a-support-raft-substrate

## Goal

Build the substrate a raft consumer needs, on a **positive offset band**: teach
`PrePass::LayerPlanning` to emit `N = support_raft_layers` raft layers at global
indices `0 .. N-1` with model layers shifted to `N ..`, mark them with a new
`GlobalLayer.is_raft` flag carried over WIT as `layer-proposal.is-raft-prefix`,
make the object-bottom-geometry predicates raft-aware, add the
`SlicedRegion.raft_fill` carrier plus its WIT accessors, and expose
`SupportPlanIR.raft_plan` to `Layer::*` guests through `paint-region-layer-view`.
No raft geometry is synthesized here — that is 240b.

## Problem Statement

The raft transport exists but nothing consumes it, and the substrate a consumer
would need does not exist either. `RaftPlan` (`crates/slicer-ir/src/slice_ir.rs`
— fields `raft_layers: u32`, `raft_first_layer_density: f32`,
`base_raft_layers: u32`, `interface_raft_layers: u32`; produced by the tree
planner's `push_raft_plan` when `support_raft_layers > 0`) flows through the
prepass write chain — SDK (`crates/slicer-sdk/src/prepass_builders.rs`), macro
glue (`crates/slicer-macros/src/lib.rs`), wasm host
(`crates/slicer-wasm-host/src/host.rs`), the native marshal leg
(`crates/slicer-wasm-host/src/marshal/native.rs`), and the blackboard merge
(`raft_plan_min` in `crates/slicer-runtime/src/blackboard.rs`) — and is then
read by nothing (G-06: "the IR exists, the consumer does not").

Four structural facts block writing any consumer. This packet removes all four.

1. **Nothing can create a raft band.** The WIT `layer-proposal`
   (`crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`)
   carries only `z` and `active-regions`, and `layer-plan-output.push-layer` is
   append-only. Both harvest legs assign `GlobalLayer.index` purely from
   `.enumerate()` push position — `harvest_layer_plan_ir_from`
   (`crates/slicer-wasm-host/src/marshal/in_.rs`) and the
   `PrePass::LayerPlanning` arm of `crates/slicer-wasm-host/src/marshal/native.rs`.
   A guest cannot express "this layer is a raft layer" at all. Note the index
   assignment itself is already correct for a positive band; only the MARKER
   is missing.

2. **Nothing carries raft-ness on the IR.** `GlobalLayer` has exactly five
   fields (`index`, `z`, `active_regions`, `has_nonplanar`, `is_sync_layer`)
   and none of them distinguishes a raft layer from a model layer.

3. **Object-bottom predicates hardcode layer zero.** Three non-test sites mean
   "the object's bottom" but test `== 0`: the sharp-tail gate and the
   `enforce_support_layers` window in `detect_support_contacts`
   (`crates/slicer-core/src/algos/overhang_annotation.rs`, whose `layer_id` is
   fed the GLOBAL index by `support_analysis_producer.rs`), and the
   `top_bottom_infill_wall_overlap` selection in `run_perimeters`
   (`modules/core-modules/classic-perimeters/src/lib.rs`). Under a raft each
   fires on a raft layer instead of the object's first printed layer — the
   DEV-124 bug class.

4. **`SupportPlanIR.raft_plan` has no read-side transport, and `SlicedRegion`
   has no `raft_fill` field.** The `ir-handles` interface exposes SupportPlanIR
   only as `paint-region-layer-view.support-plan-entries` and
   `.support-plan-segments`, and `build_paint_layer_data_with_plan`
   (`crates/slicer-wasm-host/src/dispatch.rs`) projects only `plan.entries`.

## Architecture Constraints

- **Positive offset band (plan section 12/15 authority, matching canonical).**
  Raft layers occupy global indices `0 .. N-1` where `N = support_raft_layers`;
  model layers occupy `N ..`. This mirrors canonical, where `new_layers`
  (`PrintObjectSlice.cpp`) starts object `Layer` ids at
  `slicing_parameters().raft_layers()`. No raft geometry may be minted as an
  `AnchoredEntity` or routed through `execute_per_layer_with_anchored_events`
  (plan section 15 prohibition).
- **`index` remains a position.** `GlobalLayer.index` still equals the
  element's position in `LayerPlanIR.global_layers`. Every existing positional
  lookup stays correct and MUST NOT be converted to a find-by-identity. AC-4 is
  the regression guard. This is the single largest simplification versus the
  withdrawn signed-band revision.
- **Raft-ness is explicit, never inferred from the index.** Consumers read
  `GlobalLayer.is_raft`. Do not reintroduce `index < support_raft_layers` at
  call sites that already hold a `GlobalLayer`; that reintroduces the config
  reach problem DEV-124 documented.
- **ADR-0009 is not a layer-index authority.** It concerns where raft pattern
  algorithms live (`Layer::Infill` role/claim reuse versus a shared pattern
  library), mentions no index or signedness, and its Status is `Proposed`.
  Cite plan section 12/15 instead. ADR-0009 belongs to 240b.
- **Single-writer per IR is unchanged.** This packet adds fields and accessors;
  it does not change any module's `writes` set.
- **Determinism:** raft-band Z generation is a pure function of
  (`support_raft_layers`, `first_layer_height`, `layer_height`) computed in
  `f64` with one terminal `as f32`, mirroring the existing
  `generate_object_layers` discipline in
  `modules/core-modules/layer-planner-default/src/lib.rs`. Deviating from
  f64-until-the-end reintroduces the documented z=18.8 topology regression.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` section "Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants: this packet minor-bumps
  `CURRENT_SLICE_IR_SCHEMA_VERSION` for `GlobalLayer.is_raft` plus
  `SlicedRegion.raft_fill`. The target is **the next MINOR above the live
  value, re-derived from `crates/slicer-ir/src/slice_ir.rs` at the moment of
  the edit** — never a literal written here. The live value was `4.8.0` at
  authoring, so `4.9.0` unless another in-flight packet bumped first. The bump
  and every test asserting the old value land in the same step (Step 6);
  `crates/slicer-ir/tests/ir_tests.rs` and
  `crates/slicer-ir/tests/material_boundary_widening_tdd.rs` assert the literal
  today and `crates/slicer-ir/tests/extrusion_line_roundtrip.rs` asserts
  `.major`/`.minor` — confirm the set at edit time.

## Data and Contract Notes

- IR/manifest contracts: `SliceIR` schema minor-bumped to the next minor above
  the live `CURRENT_SLICE_IR_SCHEMA_VERSION` (re-derived at the moment of the
  edit); `GlobalLayer.is_raft` and `SlicedRegion.raft_fill` serde-defaulted so
  old JSON loads; config keys snake_case (E9).
- WIT boundary: canonical sources live at `crates/slicer-schema/wit/` (both
  host `bindgen!` and guest `include_str!` read them); after any WIT edit run
  `cargo build --tests`, then rebuild guests (T4).
- `layer-planner-default`'s manifest must declare `support_raft_layers` in
  `[config.schema]` or the module config view will silently resolve an in-code
  default (E9). It currently declares `layer_height`, `first_layer_height`, and
  the `object_height:*` / `layer_height:*` wildcard rows — but no raft key.
  Verified at authoring.

## Locked Assumptions and Invariants

- Rafts occupy a positive `0..N-1` global-layer band; never anchored entities
  (plan section 15).
- `GlobalLayer.index == its position in LayerPlanIR.global_layers` remains
  TRUE after this packet. Any future code that breaks it is a bug.
- The first printed MODEL layer is `support_raft_layers`. The first PHYSICAL
  layer is `0`. Predicates must state which they mean.
- DEV-124 stays closed; its `layer_index == support_raft_layers` clamp is
  correct under this band and its pinning test file is unmodified.
- Layer-index fields stay `u32`; `SupportPlanEntry.global_layer_index` stays
  `i32` (already shipped) with a corrected doc comment.
- Invariant 16: every acceptance command names `--exact` tests or asserts a
  non-zero matched count in the same run.

## Risks and Tradeoffs

- **Silent semantic drift.** Unlike the withdrawn signed-band revision, nothing
  here fails to compile if an object-bottom predicate is missed — it simply
  clamps the wrong layer. Mitigated by the pre-baked audit table (which rules
  every site explicitly, including non-conversions) and by AC-5 plus AC-N4.
  This is the primary risk of the positive band and must not be under-weighted.
- **Deferred audit sites.** Three sites are ruled `[FWD]` rather than decided.
  If any proves to need conversion, it is a follow-up, not a silent
  reinterpretation at the keyboard.
- **Schema bump fallout:** tests hard-asserting the old SliceIR schema version
  fail loudly; bump plus fallout land in one step by design.
- **wasm/native leg skew (T9):** `raft_fill` must be projected in BOTH marshal
  legs and `is-raft-prefix` handled in both harvest legs; AC-2 asserts both
  legs explicitly for exactly this reason.
