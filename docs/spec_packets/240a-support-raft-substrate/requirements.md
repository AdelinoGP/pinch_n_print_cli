# Requirements: 240a-support-raft-substrate

## Packet Metadata

- Grouped task IDs: `TASK-409`..`TASK-413`, `TASK-531`..`TASK-534`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` (§11 queue row 7, §12 brief "240-support-raft"); gap register row G-06
- Packet status: `draft`
- Aggregate context cost: `L` (justified in `design.md` §Why This Packet Carries An L)

## Problem Statement

The raft transport exists but nothing consumes it, and the substrate a consumer
would need does not exist either. `RaftPlan` (`crates/slicer-ir/src/slice_ir.rs`,
produced by the tree planner's `push_raft_plan` when `support_raft_layers > 0`)
flows through the prepass write chain — SDK
(`crates/slicer-sdk/src/prepass_builders.rs`), macro glue
(`crates/slicer-macros/src/lib.rs`), wasm host
(`crates/slicer-wasm-host/src/host.rs`), the native marshal leg
(`crates/slicer-wasm-host/src/marshal/native.rs`), and the blackboard merge
(`raft_plan_min` in `crates/slicer-runtime/src/blackboard.rs`) — and is then
read by nothing (G-06: "the IR exists, the consumer does not").

Four structural facts, each verified against the tree at authoring time, block
writing any consumer. This packet exists to remove all four.

1. **Layer indices are unsigned.** `GlobalLayer.index`,
   `ObjectLayerRef.local_layer_index` / `.global_layer_index`,
   `SliceIR.global_layer_index`, `PerimeterIR.global_layer_index`,
   `InfillIR.global_layer_index`, `SupportIR.global_layer_index`,
   `LayerCollectionIR.global_layer_index`, `RegionKey.global_layer_index`,
   `SupportCandidateSource.global_layer_index`,
   `SupportGeometryKey.global_support_layer_index`,
   `AnchoredEntity.anchor_global_layer_index`,
   `OrderedEventCollection.anchor_global_layer_index`, and
   `SupportPlanEntry.anchor_layer_index` are all `u32`. Only
   `SupportPlanEntry.global_layer_index` and `LightningTreeEntry.global_layer_index`
   are already `i32` — those two are the pattern to follow, and
   `SupportPlanEntry.global_layer_index`'s doc comment already reserves
   negatives for raft prefix layers. Canonical inserts raft layers at print_z
   below layer 0 (`SupportCommon.cpp::generate_support_layers` →
   `object.add_support_layer(...)`), which PnP represents as signed negative
   global-layer prefix entries — unrepresentable today.

2. **Nothing can create a prefix band.** The WIT `layer-proposal`
   (`crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`)
   carries only `z` and `active-regions`; it has no index field, and
   `layer-plan-output.push-layer` is append-only. Both harvest legs assign
   `GlobalLayer.index` purely from `.enumerate()` push position —
   `harvest_layer_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
   and the `PrePass::LayerPlanning` arm of
   `crates/slicer-wasm-host/src/marshal/native.rs`. A guest therefore cannot
   express "this layer is a raft prefix layer" at all.

3. **Three consumers assume `index == Vec position`.** `hydrate_slice_arena`
   (`crates/slicer-runtime/src/layer_executor.rs`) does
   `slice_vec.get(layer.index as usize)` and raises a `FatalLayer` on a miss;
   `slice_vec` is built positionally by `execute_prepass_slice_all_layers`
   (`crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`), which also
   keys `raw_polygons_by_layer` as `HashMap<u32, _>`; and
   `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` looks up Z
   with `plan.global_layers.get(layer_index as usize)`. A negative index turns
   into a huge `usize` and every raft layer dies with "slice_ir Vec missing
   entry for layer index -1". (A second site in the same producer, the
   `global_layers.get(position)` zip against `layer_zs`, is genuinely
   positional and must be left alone.)

4. **The guest bridge truncates.** The wire is already signed —
   `ir-types.wit` declares `layer-idx = s32`, `paint-region-layer-view.layer-index`
   returns it, and the generated guest `run` signatures already take
   `layer_index: i32` — but the `slicer-macros` paint-view bridge does
   `paint.layer_index() as u32` before constructing the SDK
   `PaintRegionLayerView` (whose `layer_index` field is `u32`). A `-1` becomes
   `4294967295` at that cast.

Separately, `SupportPlanIR.raft_plan` has **no read-side transport**: the
`ir-handles` interface exposes SupportPlanIR only as
`paint-region-layer-view.support-plan-entries` / `.support-plan-segments`, and
`build_paint_layer_data_with_plan` (`crates/slicer-wasm-host/src/dispatch.rs`)
projects only `plan.entries`. And `SlicedRegion` has no `raft_fill` field.

## In Scope

- Signed-index migration `u32` → `i32` across the enumerated blast radius
  (table in `design.md` §Migration Table), including every struct literal and
  assertion site, the `MAX_LAYERS` bound, and the `raw_polygons_by_layer` key
  type.
- Removal of the `as u32` truncation in the `slicer-macros` paint-view bridge
  and the matching `PaintRegionLayerView.layer_index` retype, closing the
  existing s32/u32 boundary mismatch.
- Repair of the three `index == Vec position` consumers, and of the native
  marshal arm's positional resolved-config carry-over.
- WIT `layer-proposal.is-raft-prefix: bool` plus negative-index assignment for
  a leading prefix run in BOTH harvest legs, with a typed rejection for a
  non-contiguous prefix run.
- `com.core.layer-planner-default` emitting the raft prefix band from config
  (`support_raft_layers`, `first_layer_height`, `layer_height`), in `f64` with a
  single terminal `as f32`, seeding at least one active region per raft layer.
- `SlicedRegion.raft_fill: Vec<ExPolygon>` (serde default), `raft-fill`
  accessors on BOTH `ir-types.wit` region resources, the
  `split_field!(raft_fill);` line in
  `crates/slicer-runtime/src/region_partition.rs`, host/SDK/macro/fixture
  projection, and the `CURRENT_SLICE_IR_SCHEMA_VERSION` minor bump to `4.9.0`
  with its version-history doc-comment entry.
- `raft-plan-view` record in `ir-types.wit`, `paint-region-layer-view.raft-plan`
  accessor, host `PaintRegionLayerData.raft_plan` + population in
  `build_paint_layer_data_with_plan`, macro guest shim, and the SDK
  `PaintRegionLayerView::raft_plan()` getter.
- The DEV-124 reopen deviation row (see §DEV-124 Reopen).

## Out of Scope

- The `com.core.raft-default` module, `claim:raft-fill`, the
  `generate_raft_base` geometry port, the three raft config keys, the
  four-manifest wire-or-record decisions, the ADR-0009 amendment, and the Human
  Validation Gate — all owned by **240b-support-raft-module**.
- Making the `finalization-layer-finalization.wit` `layer-idx = u32` signed. It
  is deliberately distinct from the `ir-handles` `s32 layer-idx` (its own
  comment says a shared type would fail world satisfaction). Recorded as a
  known asymmetry in `design.md` §Locked Assumptions; revisit only if a raft
  layer must reach `PostPass::LayerFinalization` with its sign intact.
- `StageIoError::DuplicateLayerCommit.layer_index` and
  `LayerSlotOutOfRange.layer_index` stay `usize` — they are true slot
  positions, not layer identities.
- The second `global_layers.get(position)` site in
  `support_analysis_producer.rs` — genuinely positional (it zips against
  `layer_zs`), must NOT be converted to a find-by-index.
- Independent support-layer Z (G-02, 239) and the AGG rasterizer (G-07, 241).
- Replacing signed negative raft-prefix layers with anchored entities (plan §15
  prohibition; ADR-0009).

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~750 lines; direct range reads of §10, §12, §13, §15 only.
- `docs/adr/0009-raft-as-layer-infill-role.md` - 93 lines; direct read.
- `docs/specs/support-parity-gap-register.md` - G-06 row only; direct range read.
- `docs/02_ir_schemas.md` - SliceIR / schema-version sections; delegated SUMMARY before editing.
- `docs/03_wit_and_manifest.md` - WIT contract sections; delegated SUMMARY.
- `docs/21_data_defaults_and_fixtures.md` - literal gate; delegated SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers` → `object.add_support_layer(...)`: SupportLayers installed at print_z BELOW layer 0, the canonical analogue of the signed negative prefix band.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `generate_object_layers`: f64-until-the-final-cast Z discipline.

## DEV-124 Reopen

DEV-124 was closed 2026-08-07 (`docs/DEVIATION_LOG.md`, Status column: "Closed
— 2026-08-07: fixed the same day"). Its fix makes both perimeter generators
gate `only_one_wall_first_layer` on `layer_index == support_raft_layers`,
pinned by `classic_clamp_follows_raft_layers_not_layer_zero` and
`classic_clamp_unchanged_when_no_raft_configured` in
`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`.

**That clamp is only correct under a POSITIVE prefix band.** Under this
packet's signed negative band the first printed model layer is index `0`, not
index `support_raft_layers`, so once the raft path is live the clamp fires on
the wrong layer — the exact defect DEV-124 was filed for, reintroduced from the
other direction.

Obligation on this packet (Step 9): file a new deviation row recording that
DEV-124's remedy is index-convention-dependent and that the negative-band
decision reopens it. Re-derive the next free ID at write time
(`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`); do not
trust any ID written in this packet. The row must name the two pinning tests
and state the corrected predicate (`layer_index == 0` for the first model layer
under a negative prefix band). Do NOT change the perimeter generators here —
the clamp becomes wrong only when a raft is actually emitted, which is 240b;
this packet records the finding and routes it, 240b re-verifies it under a live
raft.

Residual carried forward unchanged: canonical's `has_bottom_shell_layers`
conjunct stays deliberately unported (PnP's `ResolvedConfig` range is [1, 10],
so it is unconditionally true); revisit only if that range ever admits 0.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (signed migration across 15 fields + `run_infill`), `AC-2`
  (no truncation at the paint-view bridge), `AC-3` (three positional consumers
  repaired), `AC-4` (prefix band assigned negative indices on both harvest
  legs), `AC-5` (layer planner emits the band from config), `AC-6`
  (`raft_fill` carrier + both WIT accessors + region split + 4.9.0 bump),
  `AC-7` (`raft_plan` read accessor reaches a `Layer::Infill` guest).
- Negative: `AC-N1` (non-contiguous prefix run rejected), `AC-N2` (finalization
  monotonic gate satisfied across the sign boundary), `AC-N3` (raft layer below
  all geometry slices empty, not fatal).
- Cross-packet impact: hard-blocks 240b; feeds 242's closure evidence; leaves
  239/241 untouched. 236 is `implemented`, so nothing here is a forward
  dependency.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure-gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1 migration | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-macros --test binding_surface_tdd -- negative_layer_index_survives_paint_view_bridge --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 no truncation | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor -- negative_index_layer_hydrates_slice_arena --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 positional consumers | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-wasm-host --test marshal_layer_plan_prefix_tdd -- prefix_band_indices_are_negative_on_both_legs --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 both harvest legs | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_prefix_band_emitted_before_model_layers --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- no_raft_prefix_band_when_raft_layers_zero --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 planner emits band (each command tees and is guarded separately) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_survives_roundtrip --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6 carrier | FACT pass/fail |
| `test "$(rg -c 'raft-fill: func' crates/slicer-schema/wit/deps/ir-types.wit)" -eq 2 && rg -q 'split_field..raft_fill' crates/slicer-runtime/src/region_partition.rs` | AC-6 both WIT resources + region split | FACT exit code |
| `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- raft_plan_reaches_layer_infill_guest --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-7 raft_plan read path | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-wasm-host --test marshal_layer_plan_prefix_tdd -- noncontiguous_prefix_band_rejected --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 rejection | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band_satisfies_finalization_monotonic_gate --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 monotonic gate | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_layer_below_geometry_slices_empty_not_fatal --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3 empty slice tolerated | FACT pass/fail |
| `cargo xtask build-guests --check; echo EXIT:$?` | guest freshness (exit 0 required before attributing any guest failure) | FACT exit code |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

All commands name `--exact` tests plus a non-zero matched-count guard, or are
pure exit-code checks; none invokes `cargo test --workspace`.

## Step Completion Expectations

- Steps land in order Step 1 → Step 9. The migration (Steps 2a/2b) must reach
  `cargo check --workspace --all-targets` green before the consumer-repair and
  transport steps begin.
- Guest-facing edits require `cargo xtask build-guests --check` before
  attributing any test result (T4/E4); the `layer-planner-default` change in
  Step 6 rebuilds guests in-step.
- WIT edits always end with `cargo build --tests` in the same step.
- Every new test file under an aggregated `slicer-runtime` binary carries its
  `mod` registration in the same step as the file (T2 blindness).

## Context Discipline Notes

- Never open `OrcaSlicerDocumented/` directly (E7/T1): it is gitignored, so
  glob tools miss it — verify by direct listing before claiming absence.
- `crates/slicer-ir/src/slice_ir.rs` is >3k lines: locate each struct with
  `rg -n 'pub struct <Name>'` at read time and range-read around the hit.
  Never store or reuse a line pin.
- `modules/core-modules/tree-support-planner/src/lib.rs` is ~5.9k lines: ranged
  reads only; never load in full.
- The `u32`→`i32` migration touches many test files: run the LOCATIONS dispatch
  in `design.md` §Enumerated Blast Radius instead of grepping serially
  in-context.
