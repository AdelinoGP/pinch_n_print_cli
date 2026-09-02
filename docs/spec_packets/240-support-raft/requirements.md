# Requirements: 240-support-raft

## Packet Metadata

- Grouped task IDs: `TASK-409`..`TASK-418`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` (§11 queue row 7, §12 brief "240-support-raft"); gap register row G-06
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The raft transport exists but nothing consumes it. `RaftPlan`
(`crates/slicer-ir/src/slice_ir.rs`, produced by the tree planner's
`push_raft_plan` when `support_raft_layers > 0`) already flows through the full
prepass chain — SDK (`crates/slicer-sdk/src/prepass_builders.rs`), macro glue
(`crates/slicer-macros/src/lib.rs`), wasm host (`crates/slicer-wasm-host/src/host.rs`),
both marshal legs (`crates/slicer-wasm-host/src/marshal/in_.rs`,
`.../native.rs`), and the blackboard merge (`raft_plan_min` in
`crates/slicer-runtime/src/blackboard.rs`) — and is then rendered by nothing
(G-06: "the IR exists, the consumer does not"). Every raft config key is dead
in the four support modules.

Two structural facts block even writing the consumer:

1. **Layer indices are unsigned.** `GlobalLayer.index`,
   `ObjectLayerRef.local_layer_index` / `.global_layer_index`,
   `SliceIR.global_layer_index`, `InfillIR.global_layer_index`, and
   `SupportIR.global_layer_index` are all `u32`; only
   `SupportPlanEntry.global_layer_index` is `i32`. Canonical inserts raft
   layers at print_z **below** layer 0 (`SupportCommon.cpp::generate_support_layers`
   → `object.add_support_layer(...)`), which PnP represents as signed negative
   global-layer prefix entries — unrepresentable in a u32-keyed IR.
2. **No raft fill carrier or claim holder exists.** Verified absent on
   2026-08-22 (plan §10): no `SlicedRegion.raft_fill` field, no
   `com.core.raft-default` module, no `claim:raft-fill` in any manifest.
   The role/claim plumbing half-exists: `ExtrusionRole::RaftInfill` is an
   enum variant and `SliceRegionView::should_emit` maps it to
   `"claim:raft-fill"`, but no module declares that claim, so raft emission
   would be suppressed everywhere.

This packet closes G-06 by building the consumer, the signed-index substrate,
and the key surface as one coherent slice. It absorbs ALL of deleted-draft
215-raft-geometry per plan §10; that directory is deleted by 236 (not this
packet) and its scope mapping is recorded below rather than re-litigated.

### Absorption mapping from 215-raft-geometry (plan §10, verbatim obligations)

- New module `com.core.raft-default` (`Layer::Infill` synthesizer) holding
  `claim:raft-fill`; reads `SupportPlanIR.raft_plan`, `SliceIR`,
  `LayerPlanIR`; writes the new `SlicedRegion.raft_fill` with deterministic
  fill polygons (extrusion-path conversion happens downstream under the
  claim-holder path — see design.md §ADR-0009 Reconciliation). ADR-0009
  contract preserved — rafts stay signed negative global-layer prefix
  entries, never anchored entities.
- Signed-index migration u32→i32 for the six Rust fields + one trait method:
  `GlobalLayer.index`, `ObjectLayerRef.local_layer_index` /
  `global_layer_index`, `SliceIR.global_layer_index`,
  `InfillIR.global_layer_index`, `SupportIR.global_layer_index`
  (`crates/slicer-ir/src/slice_ir.rs`), `LayerModule::run_infill`.
  `SupportPlanEntry.global_layer_index` is already `i32` — the pattern to follow.
- Issue-19/20 raft keys: `raft_contact_distance`, `raft_expansion`,
  `raft_first_layer_expansion`; wire the existing dead raft keys in the four
  support modules' manifests or record why each stays dead.
- DEV-124 check while the raft path is open (see §DEV-124 Verify-Record).

## In Scope

- New guest module `modules/core-modules/raft-default/`: manifest TOML,
  WIT world choice, guest src implementing `LayerModule::run_infill`,
  holding `claim:raft-fill`, synthesizing deterministic raft polygons
  (boundaries + inflation staging only) into `SlicedRegion.raft_fill`;
  extrusion-path/flow/speed rendering stays downstream under the
  claim-holder emit path per design.md §ADR-0009 Reconciliation.
- `SlicedRegion.raft_fill: Vec<ExPolygon>` new field (+ serde default), the
  `slice-region-view` WIT accessor, host projection in both marshal legs, and
  the resulting minor schema bump with recorded version-history doc comment.
- Signed-index migration u32→i32 across the enumerated blast radius
  (table in `design.md` §Migration Table), including the WIT `layer-idx`
  boundary review and every struct-literal/test site.
- Raft geometry semantics ported from canonical: first-layer expansion
  (`raft_first_layer_expansion`), contact/inflate staging ("inflate in
  multiple steps to avoid leaking"), base-raft vs interface-raft layer loops,
  expansion via `raft_expansion`, honoring `RaftPlan.raft_layers` /
  `base_raft_layers` / `interface_raft_layers` counts.
- Emission of raft entries as signed negative global-layer prefix layers that
  sort before model layer 0, through ordinary ordered-entity output (never
  anchored events).
- **Absorbed from the deleted packet 261-raft-keys (2026-09-01).** The
  OrcaSlicer feature-gap queue routed `raft_contact_distance` and
  `raft_expansion` to their own packet, which declared both with-gap because
  no raft generator existed. Under the map's Authoring rule 1 that
  disposition is prohibited, and this packet already builds the generator the
  keys need — so 261 was deleted and its two keys belong here. What 240 was
  missing was behaviour evidence at a non-default value (rule 6b): AC-5
  asserted declaration and defaults only. **AC-7** (raft Z gap) and **AC-8**
  (footprint XY expansion) supply it. Canonical decision points carried
  forward from 261's dispatched reads (2026-08-31): the raft Z gap in
  `SlicingParameters::SlicingParameters` (`gap_raft_object` →
  `object_print_z_min`, forced to 0 when `raft_z_gap == 0.0` or the contact
  is zero-`topZ`); the `layer_id == 0` XY expansion in
  `SupportMaterial::generate_contact_polygons`; `TreeSupport3D::generate_raft_contact`
  / `finalize_raft_contact`; and the `GCode.cpp` `_print_z` warning. The
  "ignored for soluble interface" rule canonical applies to
  `raft_contact_distance` is a wire-or-record decision for this packet, not a
  separate key.
- Config keys `raft_contact_distance` / `raft_expansion` /
  `raft_first_layer_expansion` declared in `com.core.raft-default`'s manifest;
  wire-or-record decisions for the existing dead raft keys in the four support
  modules; regenerate `docs/15_config_keys_reference.md` (T8).
- Claim-conflict behavior for a second `claim:raft-fill` holder (AC-N1).
- DEV-124 verify-record pass (AC-6).
- ADR-0009 formal amendment (status → Accepted, additive Amendments section):
  Decision 5's claim assignment (pattern module `rectilinear-infill` holds
  `claim:raft-fill`) is superseded by the support-families completion plan
  (§12 240 brief), reassigning the claim to `com.core.raft-default`; the
  zero-pattern-algorithm / no-renderer Future-Reviewer constraint is
  preserved unchanged — the module synthesizes `raft_fill` polygons only,
  with extrusion-path conversion happening downstream under the claim-holder
  emit path.
- Human Validation Gate artifacts (packet.spec.md §Human Validation Gate).

## Out of Scope

- Independent support-layer Z (G-02) — owned by 239-support-independent-layer-z.
- AGG rasterizer / `SupportGridPattern` (G-07) — owned by
  241-support-agg-rasterizer.
- Tree/traditional planner algorithm fidelity (238b) and renderer flow/
  density/interface semantics (238c) — raft reuses their outputs, fixes none.
- Replacing signed negative raft-prefix layers with anchored entities
  (plan §15 prohibition; ADR-0009 preserved).
- Pattern variety beyond v1 rectilinear (grid/honeycomb/lightning raft via
  alternative `claim:raft-fill` holders stays future work).
- Deleting `docs/spec_packets/215-raft-geometry/` — 236-owned work; this
  packet only records the absorption.
- DEV-124's deliberately-unported `has_bottom_shell_layers` residual — verify
  and record, do not port (see below).
- Ironing/filament feature-gap keys — separate track.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~750 lines; direct range reads of §7, §8, §9, §10, §12 brief only.
- `docs/adr/0009-raft-as-layer-infill-role.md` - <110 lines; direct read.
- `docs/specs/support-parity-gap-register.md` - G-06 row only; direct range read.
- `docs/02_ir_schemas.md` - SliceIR/schema-version sections; delegated SUMMARY of current SliceIR section before editing.
- `docs/15_config_keys_reference.md` - regenerated, not read in bulk.
- `docs/19_visual_debug.md` / `docs/17_agent_debugging.md` - human-gate bundle only; delegated SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base`: object-independent raft construction; first-layer expansion (`inflate_factor_1st_layer` = `raft_first_layer_expansion`), contact/inflate staging, base/interface loops, multi-step inflation, classic vs organic branches; consumes `brim_type`, `brim_object_gap`, `slicing_params.raft_layers()`/`base_raft_layers`/`interface_raft_layers`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers` → `object.add_support_layer(...)`: SupportLayers installed at print_z BELOW layer 0 (canonical analogue of PnP signed negative prefix indices).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: defaults `raft_contact_distance = 0.1`, `raft_expansion = 1.5`, `raft_first_layer_expansion = 2.0`.

## DEV-124 Verify-Record

DEV-124 closed 2026-08-07: both perimeter generators now gate
`only_one_wall_first_layer` on `layer_index == support_raft_layers` (pinned by
`classic_clamp_follows_raft_layers_not_layer_zero` and
`classic_clamp_unchanged_when_no_raft_configured` in
`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`). With
this packet making rafts actually printable, AC-6 re-runs those tests under a
live raft path and records the outcome here:

- Expected verdict: both tests green unchanged; the clamp now fires on the real
  first printed layer under raft because `support_raft_layers` shifts the gate.
- Residual recorded (deliberately not ported): canonical's
  `has_bottom_shell_layers` conjunct is unconditionally true under PnP's
  `ResolvedConfig` range [1, 10]; revisit only if that range ever admits 0.
- If either test fails under a live raft, that is a NEW finding: file a
  deviation row and route it — do not silently widen the test.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (signed migration + round-trip), `AC-2` (`raft_fill` field
  + determinism + WIT accessor + schema bump), `AC-3` (module manifest +
  freshness + claim dispatch), `AC-4` (negative-prefix ordering, expansions
  honored, zero anchored entities), `AC-5` (keys declared/wired + four-module
  wire-or-record), `AC-6` (DEV-124 verify-record), `AC-7`
  (`raft_contact_distance` moves the raft/object Z gap at a non-default
  value, forced to zero at 0.0), `AC-8` (`raft_expansion` grows the raft
  footprint at a non-default value, un-expanded at 0.0). AC-7 and AC-8 are
  the absorbed packet-261 keys' behaviour evidence — see §In Scope.
- Negative: `AC-N1` (double-holder `ClaimConflict`), `AC-N2` (out-of-band
  negative index rejected), `AC-N3` (undeclared-key manifest lint).
- Cross-packet impact: consumes 236's G-21 validator update and ADR-0059
  acceptance (forward dependency); feeds 242's closure evidence; leaves 239/241
  untouched.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1 migration | FACT pass/fail |
| `cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_deterministic --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 field/determinism | FACT pass/fail |
| `cargo xtask build-guests --check` | AC-3 guest freshness precondition (exit 0 required) | FACT exit code |
| `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- raft_infill_claim_emits --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 claim dispatch | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- raft_prefix_orders_before_model_layers --exact --nocapture && cargo test -p slicer-runtime --test integration -- raft_mints_no_anchored_entities --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 ordering/no-anchored | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- raft_keys_declared_and_wired --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 keys wired | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture && cargo test -p slicer-runtime --test contract -- classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6 DEV-124 record | FACT pass/fail |
| `cargo test -p slicer-scheduler --test validation_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 conflict advisory | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- raft_index_outside_band_rejected --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 bounds rejection | FACT pass/fail |
| `rg -q 'raft_contact_distance' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_expansion' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_first_layer_expansion' modules/core-modules/raft-default/raft-default.toml` | AC-N3 manifest lint | FACT exit code |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

All commands satisfy invariant 16 (explicit `--exact` names plus a non-zero
matched-count guard); none invokes `cargo test --workspace`.

## Step Completion Expectations

- Steps land in order Step 1 → Step 8; the signed-index migration step must
  complete `cargo check --workspace --all-targets` green before the module
  steps begin (later steps compile against migrated types).
- Guest-facing edits require `cargo xtask build-guests --check` before
  attributing any test result (T4/E4); a new guest artifact requires an actual
  rebuild (drop `--check`) inside the creating step.
- WIT edits always end with `cargo build --tests` in the same step.

## Context Discipline Notes

- Never open `OrcaSlicerDocumented/` directly (E7/T1): it is gitignored, so
  glob tools miss it — verify by direct listing before claiming absence.
- `modules/core-modules/tree-support-planner/src/lib.rs` is ~5.9k lines:
  ranged reads only; never load in full.
- The u32→i32 migration touches many test files: run the LOCATIONS dispatch
  listed in `design.md` instead of grepping serially in-context.
