---
status: draft
packet: 240a-support-raft-substrate
depends_on: 236-support-stabilization
task_ids:
  - TASK-409
  - TASK-410
  - TASK-411
  - TASK-412
  - TASK-413
  - TASK-531
  - TASK-532
  - TASK-533
  - TASK-534
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: L
---

# Packet Contract: 240a-support-raft-substrate

## Goal

Build the signed negative-index substrate a raft consumer needs: migrate the
layer-index surface from `u32` to `i32`, teach `PrePass::LayerPlanning` to emit
a raft PREFIX band at global indices `-N .. -1`, repair every consumer that
today assumes `GlobalLayer.index == Vec position`, add the
`SlicedRegion.raft_fill` carrier plus its WIT accessors, and expose
`SupportPlanIR.raft_plan` to `Layer::*` guests through `paint-region-layer-view`.
No raft geometry is synthesized here — that is 240b.

## Scope Boundaries

This packet owns the substrate only: types, index assignment, transport, and the
consumers that break when indices go negative. The `com.core.raft-default`
module, the `generate_raft_base` geometry port, the raft config keys, the
ADR-0009 amendment, and the Human Validation Gate all belong to
**240b-support-raft-module**. Independent support-Z (239) and the AGG
rasterizer (241) are excluded.

## Prerequisites and Blockers

- Depends on: **236-support-stabilization** — SATISFIED. Re-derive before
  activation (`grep '^status:' docs/spec_packets/236-support-stabilization/packet.spec.md`);
  at authoring time it reads `implemented`, so 236's outcomes (AC-8 per-region
  ruling, G-21 validator update, ADR-0059 acceptance) are shipped facts.
  236's AC-10 has already deleted `docs/spec_packets/215-raft-geometry/`.
- Unblocks: **240b-support-raft-module** (hard dependency — 240b cannot start
  until AC-1..AC-7 here are green), and through it
  242-support-family-orca-closure.
- Activation blockers: none. This packet has no human-gate artifact dependency;
  the visual gate lives in 240b.

## Acceptance Criteria

- **AC-1. Given** the workspace builds, **when** the signed-index migration
  lands, **then** `GlobalLayer.index`, `ObjectLayerRef.local_layer_index`,
  `ObjectLayerRef.global_layer_index`, `SliceIR.global_layer_index`,
  `PerimeterIR.global_layer_index`, `InfillIR.global_layer_index`,
  `SupportIR.global_layer_index`, `LayerCollectionIR.global_layer_index`,
  `RegionKey.global_layer_index`, `SupportCandidateSource.global_layer_index`,
  `SupportGeometryKey.global_support_layer_index`,
  `AnchoredEntity.anchor_global_layer_index`,
  `OrderedEventCollection.anchor_global_layer_index`,
  `SupportPlanEntry.anchor_layer_index`, and `PaintRegionLayerView.layer_index`
  are all `i32`, `LayerModule::run_infill` takes `layer_index: i32`, and a
  `SliceIR` carrying `global_layer_index: -2` round-trips through serde
  unchanged. |
  `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2. Given** the guest bridge, **when** a negative layer index crosses the
  WIT boundary, **then** it arrives unchanged rather than wrapping — the
  `as u32` truncation in the `slicer-macros` paint-view bridge (which turns
  `-1` into `4294967295`) is gone, `PaintRegionLayerView::layer_index()`
  returns `i32`, and a guest dispatched at index `-1` observes `-1`. |
  `mkdir -p target && cargo test -p slicer-macros --test binding_surface_tdd -- negative_layer_index_survives_paint_view_bridge --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3. Given** `index != Vec position` for the first time, **when** the layer
  loop hydrates a raft layer, **then** `hydrate_slice_arena`
  (`crates/slicer-runtime/src/layer_executor.rs`) resolves the `SliceIR` by
  matching `index` rather than by `slice_vec.get(layer.index as usize)`,
  `raw_polygons_by_layer` in
  `crates/slicer-runtime/src/builtins/prepass_slice_producer.rs` is keyed
  `HashMap<i32, _>`, the Z lookup in
  `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` finds by
  `index` identity, and no `FatalLayer` carrying "slice_ir Vec missing entry
  for layer index -1" is produced. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- negative_index_layer_hydrates_slice_arena --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4. Given** `layer-proposal` gains `is-raft-prefix: bool`
  (`crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`),
  **when** a `PrePass::LayerPlanning` guest pushes `N` prefix proposals ahead of
  the model proposals, **then** BOTH harvest legs — `harvest_layer_plan_ir_from`
  (`crates/slicer-wasm-host/src/marshal/in_.rs`) and the
  `PrePass::LayerPlanning` arm of `crates/slicer-wasm-host/src/marshal/native.rs`
  — assign those layers indices `-N .. -1` in push order and the remainder
  `0 ..`, the `MAX_LAYERS` bound is re-derived signed with a matching lower
  bound, and the native arm's positional resolved-config carry-over
  (`input_layer_plan.and_then(|plan| plan.global_layers.get(index))`) is
  re-keyed by `index` so it cannot skew when only one side has a prefix band. |
  `mkdir -p target && cargo test -p slicer-wasm-host --test marshal_layer_plan_prefix_tdd -- prefix_band_indices_are_negative_on_both_legs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5. Given** `support_raft_layers > 0` in config, **when**
  `com.core.layer-planner-default` runs, **then** it pushes exactly
  `support_raft_layers` proposals with `is-raft-prefix: true` before any model
  proposal, each carrying a Z computed in `f64` with a single `as f32` cast at
  the end (matching the existing `generate_object_layers` discipline) and at
  least one `active_regions` entry so the empty-region fallback in
  `derive_layer_output_envelope_from_input` (a hardcoded 0.2 mm envelope) never
  fires for a raft layer; and with `support_raft_layers = 0` it pushes none. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_prefix_band_emitted_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- no_raft_prefix_band_when_raft_layers_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6. Given** the new carrier, **when** the IR is extended, **then**
  `SlicedRegion.raft_fill: Vec<ExPolygon>` exists with `#[serde(default)]`, a
  `raft-fill` accessor returning `list<ex-polygon>` is present on BOTH
  `slice-region-view` and the perimeter region resource in
  `crates/slicer-schema/wit/deps/ir-types.wit`, `region_partition.rs` carries a
  `split_field!(raft_fill);` line so the field survives modifier-region
  splitting, and `CURRENT_SLICE_IR_SCHEMA_VERSION` is `4.9.0` with a
  version-history doc-comment entry. |
  `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_survives_roundtrip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && test "$(rg -c 'raft-fill: func' crates/slicer-schema/wit/deps/ir-types.wit)" -eq 2 && rg -q 'split_field..raft_fill' crates/slicer-runtime/src/region_partition.rs`
- **AC-7. Given** `SupportPlanIR.raft_plan` has no read-side transport today,
  **when** a `Layer::Infill` guest asks for it, **then**
  `paint-region-layer-view` exposes a `raft-plan` accessor returning
  `option<raft-plan-view>` (a `raft-plan-view` record declared in
  `ir-types.wit`, mirroring how `support-plan-entry-view` mirrors the prepass
  `support-plan-entry` rather than importing across worlds), the host
  `PaintRegionLayerData` gains a `raft_plan` field populated in
  `build_paint_layer_data_with_plan` (`crates/slicer-wasm-host/src/dispatch.rs`)
  with `SupportPlanIR` pushed to `runtime_reads`, the `slicer-macros` guest shim
  mirrors it, and the SDK `PaintRegionLayerView::raft_plan()` getter returns it
  on the native leg with no other native change. |
  `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- raft_plan_reaches_layer_infill_guest --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

Every AC names exact fields, paths, counts, errors, variants, or output
fragments and ends with its own runnable command. Repeat shared commands; never
write "see AC-N". Commands that dump more than 200 successful output lines must
be wrapped or filtered so a subagent can return a FACT.

AC verification command rule: every binary named above either exists today
(`slicer-runtime --test executor`, `slicer-runtime --test integration`,
`slicer-macros --test binding_surface_tdd`) or is a new auto-discovered
top-level test file authored by the step that first needs it
(`crates/slicer-ir/tests/signed_layer_indices_tdd.rs`,
`crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs`,
`crates/slicer-wasm-host/tests/marshal_layer_plan_prefix_tdd.rs`,
`crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs`). `slicer-ir`,
`slicer-macros`, and `slicer-wasm-host` auto-discover `tests/*.rs` with no
`required-features`; the aggregated `slicer-runtime` binaries (`executor`,
`integration`) need an explicit `mod` registration, called out in the owning
step. Note `executor` is auto-discovered from `tests/executor/main.rs` rather
than declared as a `[[test]]` in `crates/slicer-runtime/Cargo.toml` (only
`integration` and five others are declared); it is nonetheless a real runnable
binary today.

## Negative Test Cases

- **AC-N1. Given** the signed band, **when** a `LayerPlanIR` is harvested whose
  prefix-marked run is not contiguous at the front (an `is-raft-prefix: true`
  proposal pushed AFTER a model proposal), **then** harvest rejects it with a
  typed error naming the offending push position rather than silently
  interleaving negative and positive indices. |
  `mkdir -p target && cargo test -p slicer-wasm-host --test marshal_layer_plan_prefix_tdd -- noncontiguous_prefix_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2. Given** the finalization monotonic gate in
  `execute_layer_finalization_with_instrumentation` (which rejects a reversal
  with "layer indices must be monotonic"), **when** the raft band is present,
  **then** the emitted `LayerCollectionIR` sequence is still monotonic
  non-decreasing across the `-N .. -1 .. 0 ..` boundary and G-code Vec order
  matches index order. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band_satisfies_finalization_monotonic_gate --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3. Given** a raft prefix layer whose Z lies below all object geometry,
  **when** `PrePass::Slice` runs, **then** it yields a `SliceIR` with zero
  region polygons (canonical `slice_mesh_ex` returns an empty `Vec<ExPolygon>`
  for a non-intersecting Z) and NOT a `FatalLayer`. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_layer_below_geometry_slices_empty_not_fatal --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief
  "240-support-raft", §10 absorption mapping, §13 traps T1/T4/T5/T8/T9;
  direct range read.
- `docs/adr/0009-raft-as-layer-infill-role.md` - the negative-prefix contract
  this packet implements; direct read (93 lines).
- `docs/02_ir_schemas.md` - sections edited by this packet; delegated SUMMARY.
- `docs/21_data_defaults_and_fixtures.md` - literal-gate rules for the
  migration's test-literal fallout; delegated SUMMARY.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "IR 6 — SliceIR" (schema bump to 4.9.0 + `SlicedRegion.raft_fill`) - `rg -q 'raft_fill' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` signed-layer-index semantics (the `-N .. -1` raft prefix band and the `index != Vec position` consequence) - `rg -q 'raft prefix band' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` - the new `layer-proposal.is-raft-prefix` field, the `raft-plan-view` record, and the `raft-fill` accessors - `rg -q 'is-raft-prefix' docs/03_wit_and_manifest.md && rg -q 'raft-plan-view' docs/03_wit_and_manifest.md`
- `docs/DEVIATION_LOG.md` gains the DEV-124 reopen row (the shipped `layer_index == support_raft_layers` clamp is wrong under a negative prefix band — see `requirements.md` §DEV-124 Reopen). Re-derive the next free ID at write time (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`) rather than trusting any ID written here - `rg -q 'raft prefix band' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`, specifically its `object.add_support_layer(...)` insertion of `SupportLayers` at print_z BELOW layer 0. This is the canonical analogue of the signed negative prefix band and the only OrcaSlicer behaviour this packet needs. Cite functions, never line numbers.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `generate_object_layers`: the f64-until-the-final-cast Z discipline that AC-5's raft Z generator must match.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost is `L` because the u32→i32 migration is inherently wide.
`design.md` §Why This Packet Carries An L justifies why it cannot be split
further, and Step 2 is pre-split into 2a/2b for that reason. Do not activate
this packet on a standard band without the swarm escalation.

## Human Validation Gate

**None.** This packet ships no printable geometry — it is types, index
assignment, and transport. Visual verification of raft output is 240b's gate,
which cannot sign until this packet is green. Recording a visual verdict here
would be inspection of an artifact this packet does not produce.
