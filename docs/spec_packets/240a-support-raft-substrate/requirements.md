# Requirements: 240a-support-raft-substrate

## Packet Metadata

- Grouped task IDs: `TASK-409`..`TASK-413`, `TASK-533`..`TASK-536`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` (section 11 queue row 7, section 12 brief "240-support-raft"); gap register row G-06
- Packet status: `draft`
- Aggregate context cost: `M`

## Banding Decision

**The raft band is a POSITIVE OFFSET band: raft layers occupy global indices
`0 .. N-1` where `N = support_raft_layers`, and model layers are shifted to
`N ..`. The first printed MODEL layer is index `support_raft_layers`, not `0`.**

This reverses an earlier revision of this packet, which specified a signed
negative band (`-N .. -1`) and a `u32` to `i32` migration of roughly fifteen
layer-index IR fields. Three findings, each verified against the tree on
2026-09-04, drove the reversal:

1. **Canonical does the opposite of what the earlier revision claimed.** That
   revision asserted "Canonical inserts raft layers at print_z below layer 0".
   A delegated read found the reverse: `generate_support_layers`
   (`SupportCommon.cpp`) appends raft `SupportGeneratorLayer`s at strictly
   POSITIVE print_z in `[0, object_print_z_min]`, sorts the vector by
   increasing print_z so the raft lands first by Z ordering alone, and passes
   `add_support_layer` a plain dense non-negative counter. Object `Layer` ids
   start at `slicing_parameters().raft_layers()` in `new_layers`
   (`PrintObjectSlice.cpp`). Canonical reserves the low non-negative range for
   the raft and shifts the model upward — a positive offset band.

2. **The signed band's cited authority did not exist.** The earlier revision
   attributed the negative-prefix contract to ADR-0009 in four load-bearing
   places. `docs/adr/0009-raft-as-layer-infill-role.md` contains no mention of
   layer indices, signedness, or prefix bands — it decides where raft pattern
   algorithms live — and its Status is `Proposed`, not accepted. The plan spec
   did independently mandate the signed band; that mandate has been amended to
   the positive band (`docs/specs/support-families-anchored-entities-plan.md`,
   Banding decision note dated 2026-09-04).

3. **The signed band reopened a closed deviation; the positive band upholds
   it.** DEV-124 (closed 2026-08-07) fixed `only_one_wall_first_layer` firing
   on the wrong layer under a raft, by gating both perimeter generators on
   `layer_index == support_raft_layers` — explicitly matching canonical's
   `this->layer_id == object_config->raft_layers`. Under a signed negative
   band that fix becomes wrong and must revert to `== 0`. Under the positive
   band it is correct as shipped and stays. The repo has therefore ALREADY
   adopted positive-band semantics at the one site where it mattered, and
   already solved the config-reach problem by declaring `support_raft_layers`
   in both perimeter manifests.

Consequences that this packet must respect everywhere:

- `GlobalLayer.index` remains `u32`. No layer-index field changes type.
- `GlobalLayer.index` still equals its position in `LayerPlanIR.global_layers`.
  The three "positional consumer" repairs the earlier revision planned are
  unnecessary: the contract is upheld, not broken. AC-4 is a regression guard
  on exactly this.
- Raft layers are not self-identifying by index sign, so raft-ness is carried
  explicitly by the new `GlobalLayer.is_raft` flag (see section "Raft Marker").
- Any predicate meaning "the object's bottom geometry" must resolve
  `support_raft_layers`, not `0`. Any predicate meaning "the physical first
  layer on the plate" stays `0`, because under this band the raft IS the
  physical first layer. See `design.md` section "First-Model-Layer Audit".

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

## Raft Marker

Raft-ness is carried by a new `GlobalLayer.is_raft: bool` with
`#[serde(default)]`, joining the existing `has_nonplanar` and `is_sync_layer`
booleans, and set from a new WIT `layer-proposal.is-raft-prefix: bool`.

Chosen over deriving raft-ness from `index < support_raft_layers` because
several consumers hold a `GlobalLayer` without holding `LayerPlanIR` or a
config view, and DEV-124 demonstrated concretely that config reach is the hard
part (raft keys were invisible to the perimeter modules until their manifests
declared them; `ConfigView::from_declared` drops undeclared keys). A per-layer
bool answers locally with no config reach anywhere.

Chosen over a `LayerPlanIR.raft_layer_count: u32` for the same reason. The
count is still derivable where needed as the length of the leading `is_raft`
run, and AC-N1 makes that run's contiguity a validated invariant.

## In Scope

- `GlobalLayer.is_raft: bool` (serde default) and WIT
  `layer-proposal.is-raft-prefix: bool`, set on BOTH harvest legs, with a typed
  rejection when the raft-marked run is not contiguous at the front.
- `com.core.layer-planner-default` emitting the band from config
  (`support_raft_layers`, `first_layer_height`, `layer_height`) in `f64` with a
  single terminal `as f32`, seeding at least one active region per raft layer,
  and declaring `support_raft_layers` in its manifest `[config.schema]`.
- The object-bottom predicate audit: the three raft-aware conversions named in
  `design.md` section "First-Model-Layer Audit", and explicit non-conversion of
  every first-layer LINE-WIDTH flag and local-Vec bounds guard.
- `SlicedRegion.raft_fill: Vec<ExPolygon>` (serde default), `raft-fill`
  accessors on BOTH `ir-types.wit` region resources, the
  `split_field!(raft_fill);` line in
  `crates/slicer-runtime/src/region_partition.rs`, host/SDK/macro/fixture
  projection, and the `CURRENT_SLICE_IR_SCHEMA_VERSION` minor bump with its
  version-history doc-comment entry.
- `raft-plan-view` record in `ir-types.wit`, `paint-region-layer-view.raft-plan`
  AND `paint-region-layer-view.is-raft` accessors, host
  `PaintRegionLayerData.{raft_plan, is_raft}` plus population in
  `build_paint_layer_data_with_plan`, macro guest shim, and the SDK
  `PaintRegionLayerView::{raft_plan(), is_raft()}` getters. The `is-raft`
  accessor is what lets 240b's `Layer::Infill` raft module identify a raft
  layer at all; without it 240b has a declared read with no WIT accessor.
- A regression guard proving `index == Vec position` still holds (AC-4) and
  that DEV-124's clamp still fires on the first model layer (AC-N4).
- Correcting the `SupportPlanEntry.global_layer_index` doc comment, which today
  reserves negative values for raft prefix layers. The field stays `i32` (it
  already is, and churn is not warranted), but the comment must no longer
  promise a negative band.

## Out of Scope

- Any `u32` to `i32` migration of layer-index fields. Explicitly withdrawn; see
  section "Banding Decision".
- The `com.core.raft-default` module, the manifest that HOLDS `claim:raft-fill`,
  the `generate_raft_base` geometry port, the three raft config keys, the
  wire-or-record manifest sweep, the ADR-0009 amendment, and the Human
  Validation Gate — all owned by **240b-support-raft-module**. Note the claim
  STRING itself already ships: `should_emit` maps
  `ExtrusionRole::RaftInfill => "claim:raft-fill"`
  (`crates/slicer-sdk/src/views.rs`), pinned by
  `ac4_raft_fill_claim_emits_raft_infill`
  (`crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`). Neither is
  edited here.
- Changing `SupportGeometryKey.global_support_layer_index`, including its
  `u32::MAX` sentinel. Under the positive band the sentinel does not collide
  with any raft index and needs no change.
- The `finalization-layer-finalization.wit` `layer-idx = u32`. It is
  deliberately distinct from the `ir-handles` `layer-idx = s32`; its own
  comment records that a shared type would fail world satisfaction.
- Every first-layer LINE-WIDTH / flow flag (`resolve_role_width(..., layer_index == 0, ...)`
  and `arachne_params_from_config(config, layer_index == 0)`). Under this band
  the raft IS the physical first layer, so these remain correct at `0`.
- Local-Vec bounds guards of the form `if idx == 0 { continue }`, including
  those in `crates/slicer-core/src/algos/lightning/generator.rs`,
  `modules/core-modules/overhang-classifier-default/src/lib.rs`, and
  `modules/core-modules/tree-support-planner/src/lib.rs`.
- Independent support-layer Z (G-02, 239) and the AGG rasterizer (G-07, 241).
- Replacing raft layers with anchored entities (plan section 15 prohibition).

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~750 lines; direct range reads of sections 10, 12, 13, 15 only.
- `docs/02_ir_schemas.md` - SliceIR / schema-version sections; delegated SUMMARY before editing.
- `docs/03_wit_and_manifest.md` - WIT contract sections; delegated SUMMARY.
- `docs/21_data_defaults_and_fixtures.md` - literal gate; delegated SUMMARY.
- `docs/DEVIATION_LOG.md` - the DEV-124 row only; direct range read.
- `docs/specs/support-parity-gap-register.md` - G-06 row only; direct range read.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`: raft layers appended at positive print_z, sorted by print_z, dense non-negative id counter.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `new_layers`: object `Layer` ids start at `slicing_parameters().raft_layers()`; the canonical positive offset band.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `generate_object_layers`: `coordf_t`-throughout Z discipline.

## DEV-124 Upheld

DEV-124 was closed 2026-08-07. Its fix gates `only_one_wall_first_layer` on
`layer_index == support_raft_layers` in both perimeter generators, pinned by
`classic_clamp_follows_raft_layers_not_layer_zero` and
`classic_clamp_unchanged_when_no_raft_configured`
(`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`).

**Under this packet's positive band that clamp is correct exactly as shipped.**
This packet therefore files no reopen row. It instead adds AC-N4, which proves
under a LIVE raft band that the clamp fires on index `support_raft_layers` and
not on raft layer 0, and that the pinning-test file is unmodified. The earlier
revision's planned "DEV-124 reopen" obligation is withdrawn.

Residual carried forward unchanged: canonical's `has_bottom_shell_layers`
conjunct stays deliberately unported (PnP's `ResolvedConfig` range is [1, 10],
so it is unconditionally true); revisit only if that range ever admits 0.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (`GlobalLayer.is_raft`, indices stay `u32`), `AC-2`
  (marker set identically on both harvest legs), `AC-3` (planner emits the band
  from a declared config key), `AC-4` (`index == Vec position` upheld), `AC-5`
  (object-bottom predicates raft-aware, line-width flags untouched), `AC-6`
  (`raft_fill` carrier + both WIT accessors + region split + schema minor bump),
  `AC-7` (`raft_plan` read accessor reaches a `Layer::Infill` guest).
- Negative: `AC-N1` (non-contiguous raft run rejected), `AC-N2` (finalization
  monotonic gate satisfied across the band boundary), `AC-N3` (raft layer below
  all geometry slices empty, not fatal), `AC-N4` (DEV-124 clamp still correct
  under a live raft, pinning file unmodified).
- Cross-packet impact: hard-blocks 240b; feeds 242's closure evidence; leaves
  239/241 untouched. 236 is `implemented`, so nothing here is a forward
  dependency.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure-gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-ir --test raft_band_ir_tdd -- is_raft_defaults_false_and_survives_roundtrip --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1 marker field | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-wasm-host --lib -- marshal::in_::tests::raft_marker_identical_on_both_legs --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 both harvest legs (in-crate: harvest fn is `pub(crate)`) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::raft_band_emitted_before_model_layers --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 planner emits band | FACT pass/fail |
| `rg -q 'config\.schema\.support_raft_layers' modules/core-modules/layer-planner-default/layer-planner-default.toml` | AC-3 manifest declares the key (E9) | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::no_raft_band_when_raft_layers_zero --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 zero-raft case | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_positional_tdd::raft_layer_index_equals_vec_position --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 positional contract upheld | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_object_bottom_tdd::object_bottom_predicates_are_raft_aware --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 audit applied | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_survives_roundtrip --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6 carrier | FACT pass/fail |
| `test "$(rg -c 'raft-fill: func' crates/slicer-schema/wit/deps/ir-types.wit)" -eq 2 && rg -q 'split_field..raft_fill' crates/slicer-runtime/src/region_partition.rs` | AC-6 both WIT resources + region split | FACT exit code |
| `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- raft_plan_reaches_layer_infill_guest --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-7 raft_plan read path | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-wasm-host --lib -- marshal::in_::tests::noncontiguous_raft_band_rejected --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 rejection (in-crate) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::raft_band_satisfies_finalization_monotonic_gate --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 monotonic gate | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_positional_tdd::raft_layer_below_geometry_slices_empty_not_fatal --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3 empty slice tolerated | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && git diff --quiet -- crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` | AC-N4 DEV-124 upheld | FACT pass/fail |
| `cargo xtask build-guests --check; echo EXIT:$?` | guest freshness (exit 0 required before attributing any guest failure) | FACT exit code |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT exit code |

All commands name `--exact` tests plus a non-zero matched-count guard, or are
pure exit-code checks; none invokes `cargo test --workspace`.

## Step Completion Expectations

- Steps land in order Step 1 through Step 7.
- Guest-facing edits require `cargo xtask build-guests --check` before
  attributing any test result (T4/E4); the `layer-planner-default` change in
  Step 3 rebuilds guests in-step.
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
- The audit in Step 4 is pre-baked in `design.md`; do not re-derive it by
  grepping serially in-context.
