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
  - TASK-533
  - TASK-534
  - TASK-535
  - TASK-536
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 240a-support-raft-substrate

## Goal

Build the substrate a raft consumer needs, on a **positive offset band**: teach
`PrePass::LayerPlanning` to emit `N = support_raft_layers` raft layers at global
indices `0 .. N-1` with model layers shifted to `N ..`, mark them with a new
`GlobalLayer.is_raft` flag carried over WIT as `layer-proposal.is-raft-prefix`,
make the object-bottom-geometry predicates raft-aware, add the
`SlicedRegion.raft_fill` carrier plus its WIT accessors, and expose
`SupportPlanIR.raft_plan` to `Layer::*` guests through `paint-region-layer-view`.
No raft geometry is synthesized here — that is 240b.

## Scope Boundaries

This packet owns types, index assignment, transport, and the object-bottom
predicates that change meaning when the band exists. The `com.core.raft-default`
module, the `generate_raft_base` geometry port, the raft config keys, the
ADR-0009 amendment, and the Human Validation Gate all belong to
**240b-support-raft-module**. Independent support-Z (239) and the AGG
rasterizer (241) are excluded.

**Layer indices stay `u32`.** An earlier revision of this packet specified a
signed negative band (`-N..-1`) and a `u32` to `i32` migration of ~15 IR fields.
That is abandoned; see `requirements.md` section "Banding Decision". No
layer-index field changes type in this packet.

## Prerequisites and Blockers

- Depends on: **236-support-stabilization** — SATISFIED. Re-derive before
  activation (`grep '^status:' docs/spec_packets/236-support-stabilization/packet.spec.md`);
  at authoring time it reads `implemented`.
- Unblocks: **240b-support-raft-module** (hard dependency — 240b cannot start
  until AC-1..AC-7 here are green), and through it
  242-support-family-orca-closure.
- Activation blockers: none. The visual gate lives in 240b.

## Acceptance Criteria

- **AC-1. Given** the IR, **when** the raft marker lands, **then**
  `GlobalLayer` gains `pub is_raft: bool` with `#[serde(default)]` (alongside the
  existing `has_nonplanar` / `is_sync_layer` bools — note the struct derives
  `Default` and those two carry no `#[serde(default)]` of their own, so the
  attribute is new here and is what preserves backward compat), every layer-index field remains `u32`
  with no `i32` migration, and a `LayerPlanIR` whose first two `GlobalLayer`s
  carry `is_raft: true` round-trips through serde unchanged. |
  `mkdir -p target && cargo test -p slicer-ir --test raft_band_ir_tdd -- is_raft_defaults_false_and_survives_roundtrip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2. Given** `layer-proposal` gains `is-raft-prefix: bool`
  (`crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`),
  **when** a `PrePass::LayerPlanning` guest pushes `N` raft proposals ahead of
  the model proposals, **then** BOTH harvest legs — `harvest_layer_plan_ir_from`
  (`crates/slicer-wasm-host/src/marshal/in_.rs`) and the
  `PrePass::LayerPlanning` arm of `crates/slicer-wasm-host/src/marshal/native.rs`
  — set `GlobalLayer.is_raft` from it, assign indices `0 ..` in push order
  unchanged, and produce identical `(index, is_raft)` pairs for the same push
  sequence. `harvest_layer_plan_ir_from` is `pub(crate)`, so this test CANNOT
  live in a `tests/*.rs` file (separate crate); it goes in a
  `#[cfg(test)] mod tests` inside `crates/slicer-wasm-host/src/marshal/in_.rs`
  and runs under `--lib` with the full module path. The native leg
  (`commit_native_prepass_response_with_inputs`) is `pub` and reachable from the
  same in-crate module, which is what makes the both-legs comparison possible.
  The SDK `LayerProposal` (`crates/slicer-sdk/src/prepass_types.rs`) must mirror
  the flag too — the native leg reads that type, whose fields today are only
  `z` and `active_regions`. |
  `mkdir -p target && cargo test -p slicer-wasm-host --lib -- marshal::in_::tests::raft_marker_identical_on_both_legs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3. Given** `support_raft_layers > 0` in config, **when**
  `com.core.layer-planner-default` runs, **then** it pushes exactly
  `support_raft_layers` proposals with `is-raft-prefix: true` before any model
  proposal, each carrying a Z computed in `f64` with a single `as f32` cast at
  the end (matching the existing `generate_object_layers` discipline) and at
  least one `active_regions` entry so the empty-region fallback in
  `derive_layer_output_envelope_from_input` never fires for a raft layer;
  `support_raft_layers` is declared in
  `modules/core-modules/layer-planner-default/layer-planner-default.toml`
  under `[config.schema]` so E9's silent-default trap cannot fire; and with
  `support_raft_layers = 0` it pushes none. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::raft_band_emitted_before_model_layers --exact --nocapture 2>&1 | tee target/ac3-a.log && test "$(grep -c '^test .* ok$' target/ac3-a.log)" -gt 0 && rg -q 'config\.schema\.support_raft_layers' modules/core-modules/layer-planner-default/layer-planner-default.toml && cargo test -p slicer-runtime --test integration -- raft_band::no_raft_band_when_raft_layers_zero --exact --nocapture 2>&1 | tee target/ac3-b.log && test "$(grep -c '^test .* ok$' target/ac3-b.log)" -gt 0`
- **AC-4. Given** the band is contiguous from index 0, **when** a raft layer is
  hydrated, **then** `GlobalLayer.index` still equals its position in
  `LayerPlanIR.global_layers`, `hydrate_slice_arena`
  (`crates/slicer-runtime/src/layer_executor.rs`) resolves its `SliceIR` without
  producing a `FatalLayer`, and `raw_polygons_by_layer`
  (`crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`) remains
  `HashMap<u32, _>` — the positional-consumer contract is UPHELD, not repaired.
  Note `hydrate_slice_arena` is a private `fn`; the test exercises it
  indirectly through the executor entry point rather than calling it. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_positional_tdd::raft_layer_index_equals_vec_position --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5. Given** the first printed MODEL layer is now `support_raft_layers`
  rather than `0`, **when** the object-bottom audit lands, **then** the three
  sites ruled raft-aware in `design.md` section "First-Model-Layer Audit" —
  the sharp-tail gate `params.layer_id == 0` in `detect_support_contacts`
  (`crates/slicer-core/src/algos/overhang_annotation.rs`), the
  `enforce_support_layers` window in the same function, and the
  `top_bottom_infill_wall_overlap` selection in `run_perimeters`
  (`modules/core-modules/classic-perimeters/src/lib.rs`) — resolve the boundary
  from `support_raft_layers` following the shipped DEV-124 template, while
  every site the audit rules "leave alone" (all first-layer LINE-WIDTH flags
  and all local-Vec bounds guards) is unchanged in `git diff`. **The boundary
  must actually reach `SupportContactParams`**: `resolve_contact_params`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) — today
  the sole `ResolvedConfig` bridge into that struct, and today hardcoding
  `enforce_support_layers: 0` — must populate the new raft-boundary field from
  `support_raft_layers` rather than a literal, so that a config with
  `support_raft_layers = 2` yields params carrying `2`. Without this the
  conversion rides at its default and is inert while still compiling.
  `resolve_contact_params` is a PRIVATE `fn`, so this test belongs in that
  file's existing `#[cfg(test)] mod tests` (beside
  `resolve_contact_params_uses_typed_threshold_overlap_percent_and_literal`)
  and runs under `--lib`, NOT in the `contract` binary, which is a separate
  crate and cannot name it. **The filter must be the FULL module path** —
  `--exact` matches a unit test's complete path, so the bare test name matches
  nothing and the run reports `0 passed; … 100 filtered out`. Measured
  2026-09-04: the bare name yielded `running 0 tests`; the path-qualified form
  yielded `1 passed`. The `git diff --quiet` leg is the machine check for
  the "leave alone" clause — without it the leave-alone ruling is prose only,
  and silent semantic drift is this packet's primary risk. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_object_bottom_tdd::object_bottom_predicates_are_raft_aware --exact --nocapture 2>&1 | tee target/ac5-a.log && test "$(grep -c '^test .* ok$' target/ac5-a.log)" -gt 0 && cargo test -p slicer-runtime --lib -- builtins::support_analysis_producer::tests::resolve_contact_params_carries_raft_boundary_from_config --exact --nocapture 2>&1 | tee target/ac5-b.log && test "$(grep -c '^test .* ok$' target/ac5-b.log)" -gt 0 && git diff --quiet -- modules/core-modules/arachne-perimeters/src/lib.rs modules/core-modules/rectilinear-infill/src/lib.rs modules/core-modules/wave-overhangs/src/lib.rs modules/core-modules/overhang-classifier-default/src/lib.rs modules/core-modules/part-cooling/src/lib.rs modules/core-modules/tree-support-planner/src/lib.rs crates/slicer-core/src/algos/lightning/generator.rs`
- **AC-6. Given** the new carrier, **when** the IR is extended, **then**
  `SlicedRegion.raft_fill: Vec<ExPolygon>` exists with `#[serde(default)]`, a
  `raft-fill` accessor returning `list<ex-polygon>` is present on BOTH
  `resource slice-region-view` and `resource perimeter-region-view` in
  `crates/slicer-schema/wit/deps/ir-types.wit`, `region_partition.rs` carries a
  `split_field!(raft_fill);` line beside `split_field!(internal_bridge_areas);`
  so the field survives modifier-region splitting, and
  `CURRENT_SLICE_IR_SCHEMA_VERSION` has been minor-bumped to the next minor
  above its live value (re-derived from `crates/slicer-ir/src/slice_ir.rs` at
  the moment of the edit; `4.8.0` at authoring, so `4.9.0` unless something
  bumped first) with a version-history doc-comment entry naming both
  `is_raft` and `raft_fill`. |
  `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_survives_roundtrip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && test "$(rg -c 'raft-fill: func' crates/slicer-schema/wit/deps/ir-types.wit)" -eq 2 && rg -q 'split_field..raft_fill' crates/slicer-runtime/src/region_partition.rs`
- **AC-7. Given** neither `SupportPlanIR.raft_plan` nor the raft marker has
  read-side transport today, **when** a `Layer::Infill` guest asks for them,
  **then** `paint-region-layer-view` exposes an `is-raft: func() -> bool`
  accessor returning the current layer's `GlobalLayer.is_raft` (without which a
  `Layer::Infill` guest such as 240b's `com.core.raft-default` cannot tell a
  raft layer from a model layer), AND a `raft-plan` accessor returning
  `option<raft-plan-view>` — a `raft-plan-view` record declared in
  `ir-types.wit` mirroring `RaftPlan`'s four fields (`raft-layers: u32`,
  `raft-first-layer-density: f32`, `base-raft-layers: u32`,
  `interface-raft-layers: u32`) rather than importing across worlds — the host
  `PaintRegionLayerData` gains a `raft_plan` field populated in
  `build_paint_layer_data_with_plan` (`crates/slicer-wasm-host/src/dispatch.rs`)
  with `SupportPlanIR` pushed to `runtime_reads` (`build_paint_layer_data_with_plan`
  is a private `fn`; the test reaches it INDIRECTLY by dispatching a real
  `Layer::Infill` test-guest through the public dispatch entry point, never by
  calling it), the `slicer-macros` guest shim mirrors both, and the SDK `PaintRegionLayerView::raft_plan()` and
  `::is_raft()` getters return them on BOTH legs. `raft_plan()` needs no further
  native change (the view already holds the `Arc<SupportPlanIR>` via
  `with_support_plan`), but **`is_raft` does**: the native view is constructed in
  `crates/slicer-wasm-host/src/marshal/native.rs` and `is_raft` must be set there
  from `GlobalLayer.is_raft`, or native `is_raft()` compiles and silently returns
  `false` forever. This AC therefore asserts the NATIVE leg explicitly, not only
  the wasm one. |
  `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- raft_plan_reaches_layer_infill_guest --exact --nocapture 2>&1 | tee target/ac7-a.log && test "$(grep -c '^test .* ok$' target/ac7-a.log)" -gt 0 && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- is_raft_reaches_layer_infill_guest --exact --nocapture 2>&1 | tee target/ac7-b.log && test "$(grep -c '^test .* ok$' target/ac7-b.log)" -gt 0 && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- is_raft_set_on_native_leg --exact --nocapture 2>&1 | tee target/ac7-c.log && test "$(grep -c '^test .* ok$' target/ac7-c.log)" -gt 0`

Every AC names exact fields, paths, counts, errors, variants, or output
fragments and ends with its own runnable command. Repeat shared commands; never
write "see AC-N". Commands that dump more than 200 successful output lines must
be wrapped or filtered so a subagent can return a FACT.

AC verification command rule: `slicer-runtime --test executor`,
`--test integration`, and `--test contract` exist today; `slicer-ir`,
`slicer-macros`, and `slicer-wasm-host` auto-discover `tests/*.rs` with no
`required-features`, so the new top-level files
(`crates/slicer-ir/tests/raft_band_ir_tdd.rs`,
`crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs`,
`crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs`) need no
registration. The aggregated `slicer-runtime` binaries DO need an explicit
`mod` line, called out in the owning step.

## Negative Test Cases

- **AC-N1. Given** the raft band must be a contiguous run at the FRONT of the
  push sequence, **when** a `LayerPlanIR` is harvested whose `is-raft-prefix`
  proposals are not contiguous at the front (a raft-marked proposal pushed
  AFTER a model proposal), **then** harvest rejects it with a typed error
  naming the offending push position rather than silently accepting an
  interleaved band. |
  `mkdir -p target && cargo test -p slicer-wasm-host --lib -- marshal::in_::tests::noncontiguous_raft_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2. Given** the finalization monotonic gate in
  `execute_layer_finalization_with_instrumentation`, which rejects a reversal
  with "layer indices must be monotonic", **when** the raft band is present,
  **then** the emitted `LayerCollectionIR` sequence is still monotonic
  non-decreasing across the raft/model boundary and G-code Vec order matches
  index order. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::raft_band_satisfies_finalization_monotonic_gate --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3. Given** a raft layer whose Z lies below all object geometry,
  **when** `PrePass::Slice` runs, **then** it yields a `SliceIR` with zero
  region polygons — canonical `slice_mesh_ex`
  (`crates/slicer-core/src/triangle_mesh_slicer.rs`) returns
  `Vec<Vec<ExPolygon>>`, one inner `Vec` per requested z, so a
  non-intersecting Z yields an EMPTY inner entry, not a missing one — and NOT
  a `FatalLayer`. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_positional_tdd::raft_layer_below_geometry_slices_empty_not_fatal --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N4. Given** DEV-124's shipped clamp gates `only_one_wall_first_layer` on
  `layer_index == support_raft_layers`, **when** a raft band is live, **then**
  the clamp fires on the first MODEL layer (index `support_raft_layers`) and
  NOT on raft layer 0, and the two pinning tests
  `classic_clamp_follows_raft_layers_not_layer_zero` and
  `classic_clamp_unchanged_when_no_raft_configured`
  (`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`)
  still pass with that file unmodified — DEV-124 stays CLOSED. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && git diff --quiet -- crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - section 12 brief
  "240-support-raft" (amended 2026-09-04 to the positive band), section 10
  absorption mapping, section 13 traps T1/T4/T5/T8/T9, section 15 prohibitions;
  direct range read.
- `docs/02_ir_schemas.md` - sections edited by this packet; delegated SUMMARY.
- `docs/21_data_defaults_and_fixtures.md` - literal-gate rules; delegated SUMMARY.
- `docs/DEVIATION_LOG.md` - the DEV-124 row, whose remedy this packet UPHOLDS;
  read that row only.
- `docs/adr/0009-raft-as-layer-infill-role.md` - **NOT a layer-index contract.**
  It concerns where raft pattern algorithms live (`Layer::Infill` role/claim
  reuse) and mentions no index, signedness, or band; its Status is `Proposed`.
  Relevant to 240b only.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "IR 6 — SliceIR" (schema minor bump to the next minor above the live `CURRENT_SLICE_IR_SCHEMA_VERSION`, re-derived from `crates/slicer-ir/src/slice_ir.rs` at the moment of the edit, plus `SlicedRegion.raft_fill` and `GlobalLayer.is_raft`) - `rg -q 'raft_fill' docs/02_ir_schemas.md && rg -q 'is_raft' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` positive-band semantics (raft occupies `0..N-1`, the first printed model layer is `support_raft_layers`, and `index == Vec position` is preserved) - `rg -q 'raft offset band' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` - the new `layer-proposal.is-raft-prefix` field, the `raft-plan-view` record, and the `raft-fill` accessors - `rg -q 'is-raft-prefix' docs/03_wit_and_manifest.md && rg -q 'raft-plan-view' docs/03_wit_and_manifest.md`
- `docs/DEVIATION_LOG.md` gains a row recording that PnP's raft band is a positive offset band matching canonical, that this packet's earlier signed-negative specification was withdrawn, and that DEV-124's remedy is upheld rather than reopened. Re-derive the next free ID at write time (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`) - `rg -q 'raft offset band' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`: raft `SupportGeneratorLayer`s appended at positive print_z within `[0, object_print_z_min]`, sorted by print_z, id taken from a dense non-negative counter. Cite functions, never line numbers.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `new_layers`: object `Layer` ids start at `slicing_parameters().raft_layers()` and print_z is offset by `object_print_z_min`. This is the canonical positive offset band this packet ports.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `generate_object_layers`: the `coordf_t`-throughout Z discipline that AC-3's raft Z generator must match.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard)

Aggregate context cost is `M`. The `u32` to `i32` migration that made the
earlier revision an `L` is gone; no step exceeds `M`. This packet runs on the
standard swarm band with no escalation.

## Human Validation Gate

**None.** This packet ships no printable geometry — it is types, index
assignment, and transport. Visual verification of raft output is 240b's gate,
which cannot sign until this packet is green.
