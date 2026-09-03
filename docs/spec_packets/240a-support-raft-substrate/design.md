# Design: 240a-support-raft-substrate

## Controlling Code Paths

- Index assignment: `com.core.layer-planner-default` (`push-layer`) →
  `harvest_layer_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
  and the `PrePass::LayerPlanning` arm of
  `crates/slicer-wasm-host/src/marshal/native.rs` → `LayerPlanIR.global_layers`
  → `promote_global_layers` (`crates/slicer-runtime/src/layer_executor.rs`).
- Index consumption: `hydrate_slice_arena`
  (`crates/slicer-runtime/src/layer_executor.rs`),
  `execute_prepass_slice_all_layers`
  (`crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`),
  `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`.
- Raft-plan read path: `SupportPlanIR.raft_plan` (blackboard) →
  `build_paint_layer_data_with_plan` (`crates/slicer-wasm-host/src/dispatch.rs`)
  → host `PaintRegionLayerData.raft_plan` → WIT
  `paint-region-layer-view.raft-plan` → `slicer-macros` guest shim → SDK
  `PaintRegionLayerView::raft_plan()`.
- Neighboring tests/fixtures:
  `crates/slicer-macros/tests/binding_surface_tdd.rs`,
  `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`,
  fixture `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference
  Obligations; do not repeat delegation rules.

## Architecture Constraints

- **ADR-0009 preserved:** rafts are signed negative global-layer PREFIX entries
  (`-N .. -1`, sorting before model layer 0). No raft geometry may be minted as
  an `AnchoredEntity`, routed through
  `execute_per_layer_with_anchored_events`, or carried by any anchored-event
  structure (plan §15 prohibition). The `layer-idx = s32` WIT type in
  `ir-types.wit` and the signed doc comment on
  `SupportPlanEntry.global_layer_index` already anticipate this — the Rust IR
  must match them.
- **`index` becomes an identity, not a position.** After this packet,
  `GlobalLayer.index` no longer equals the element's position in
  `LayerPlanIR.global_layers`. Every lookup keyed by a layer index must resolve
  by identity (find / HashMap), and every genuinely positional zip must stay
  positional. `design.md` §Positional Consumer Ruling fixes which is which; do
  not decide case-by-case at the keyboard.
- **Single-writer per IR is unchanged.** This packet adds fields and accessors;
  it does not change any module's `writes` set.
- **Determinism:** raft-band Z generation is a pure function of
  (`support_raft_layers`, `first_layer_height`, `layer_height`) computed in
  `f64` with one terminal `as f32`, mirroring the existing
  `generate_object_layers` discipline in
  `modules/core-modules/layer-planner-default/src/lib.rs`. Deviating from
  f64-until-the-end reintroduces the documented z=18.8 topology regression.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants: this packet bumps
  `CURRENT_SLICE_IR_SCHEMA_VERSION` from `4.8.0` to `4.9.0` for
  `SlicedRegion.raft_fill` plus the signed indices. Re-derive the live value
  before editing rather than trusting `4.8.0` written here. The bump and every
  test asserting the old value land in the same step (Step 7); only two test
  files assert the literal today (`crates/slicer-ir/tests/ir_tests.rs` and
  `crates/slicer-ir/tests/material_boundary_widening_tdd.rs`) — confirm that
  count at edit time.

## Code Change Surface

### Selected approach

1. **Signed-index migration first** (substrate before everything): retype the
   fields in §Migration Table, change `LayerModule::run_infill`'s parameter to
   `i32`, retype `PaintRegionLayerView.layer_index`, delete the `as u32`
   truncation in the `slicer-macros` paint-view bridge, re-derive `MAX_LAYERS`
   signed with a lower bound, and fix every struct literal / assertion site
   from the pre-baked LOCATIONS sweep.
2. **Repair the positional consumers** per §Positional Consumer Ruling before
   any negative index can exist, so the fix and the thing it protects against
   never coexist in a broken state.
3. **WIT prefix marking:** add `is-raft-prefix: bool` to `layer-proposal` in
   `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`;
   both harvest legs count the leading prefix run of length `N` and assign
   `-N .. -1` to it, `0 ..` to the rest, rejecting a non-contiguous run.
4. **Producer:** `com.core.layer-planner-default` reads `support_raft_layers`
   (a config key it can read directly — the raft band must exist before
   `PrePass::SupportGeometry`, so it cannot wait for `RaftPlan`) and pushes the
   band first.
5. **Carrier + accessors:** `SlicedRegion.raft_fill`, both `ir-types.wit`
   resources, `split_field!(raft_fill);`, host/SDK/macro/fixture projection,
   schema bump.
6. **Raft-plan read accessor:** `raft-plan-view` record + accessor + host field
   + dispatch population + macro shim + SDK getter.

### Why the raft band is produced at `PrePass::LayerPlanning`

`VALID_STAGES` (`crates/slicer-schema/src/lib.rs`) orders
`PrePass::LayerPlanning` before `PrePass::SupportGeometry`, and both support
planners declare `LayerPlanIR` in their `reads`. Producing the band at layer
planning therefore makes it visible to the support planners, which is required
for 240b. `RaftPlan` is not available that early — but it does not need to be:
`RaftPlan` is itself derived from the same config keys, so the planner reads
`support_raft_layers` directly. The counts in `RaftPlan`
(`base_raft_layers`, `interface_raft_layers`) matter only to 240b's geometry,
not to band creation.

### Positional Consumer Ruling (normative; do not re-decide per site)

| Site | Ruling |
| --- | --- |
| `hydrate_slice_arena` `slice_vec.get(layer.index as usize)` (`crates/slicer-runtime/src/layer_executor.rs`) | **Convert to identity lookup.** This is the highest-risk site: a negative index becomes a huge `usize`, missing, and raising `FatalLayer` "slice_ir Vec missing entry for layer index -1" on every raft layer. |
| `raw_polygons_by_layer: HashMap<u32, _>` (`crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`) | **Re-key to `HashMap<i32, _>`.** Already keyed by `gl.index`, so only the key type changes. |
| `prepass_slice_producer.rs` prev-layer lookup `i.checked_sub(1).map(\|prev_i\| layer_plan.global_layers[prev_i].index)` | **Leave.** Position→index conversion is already correct by construction. |
| `support_analysis_producer.rs` `plan.global_layers.get(layer_index as usize)` Z lookup | **Convert to identity lookup** (`.iter().find(\|l\| l.index == layer_index)`). Note the same site feeds `SupportGeometryKey.global_support_layer_index`, which the migration retypes. |
| `support_analysis_producer.rs` `global_layers.get(position)` inside the `slice_modifier_volumes(...).enumerate()` zip | **Leave positional.** It zips against `layer_zs`, which is built from `global_layers` in order; the in-code comment already says so. Converting it to find-by-index would be a regression. |
| `native.rs` `input_layer_plan.and_then(\|plan\| plan.global_layers.get(index))` resolved-config carry-over | **Re-key by `index`.** Otherwise the carry-over skews whenever only one side of the round-trip has a prefix band. |

### Rejected alternatives and reasons

- **Positive prefix band (raft at `0..N-1`, model shifted to `N..`):**
  rejected by explicit decision. It needs no type change and matches DEV-124's
  shipped clamp, but it contradicts ADR-0009, plan §15, and the `layer-idx = s32`
  contract, and would require amending all three. The signed band was chosen to
  keep those authorities intact; the cost is this packet's `L` and the DEV-124
  reopen recorded in `requirements.md` §DEV-124 Reopen.
- **Host-side prefix injection after harvest** (synthesize the band in
  `promote_global_layers` instead of at the planner): rejected — the band would
  not exist during `PrePass::Slice` or `PrePass::SupportGeometry`, so raft
  layers would have no `SliceIR` slot and the support planners could not see
  them.
- **Importing the prepass `raft-plan` record into `ir-handles`:** rejected —
  cross-world record import risks a world-satisfaction failure (the same reason
  `finalization-layer-finalization.wit` keeps its own `layer-idx`). Declare a
  `raft-plan-view` record in `ir-types.wit` instead, exactly mirroring how
  `support-plan-entry-view` mirrors the prepass `support-plan-entry`.
- **Anchored entities for raft layers:** rejected — plan §15 prohibition;
  ADR-0009 contract.

## Migration Table (u32 → i32)

| Field / method | File | Note |
| --- | --- | --- |
| `GlobalLayer.index` | `crates/slicer-ir/src/slice_ir.rs` | serde round-trip test |
| `ObjectLayerRef.local_layer_index` | same | |
| `ObjectLayerRef.global_layer_index` | same | |
| `SliceIR.global_layer_index` | same | schema minor bump |
| `PerimeterIR.global_layer_index` | same | |
| `InfillIR.global_layer_index` | same | consumed by `run_infill` callers |
| `SupportIR.global_layer_index` | same | |
| `LayerCollectionIR.global_layer_index` | same | finalization monotonic gate reads this |
| `RegionKey.global_layer_index` | same | |
| `SupportCandidateSource.global_layer_index` | same | |
| `SupportGeometryKey.global_support_layer_index` | same | doc warns consumers index a `Vec<LayerCollisionCache>` with it — audit that use |
| `AnchoredEntity.anchor_global_layer_index` | same | |
| `OrderedEventCollection.anchor_global_layer_index` | same | sorted in `layer_executor.rs` merge walk |
| `SupportPlanEntry.anchor_layer_index` | same **and** `crates/slicer-sdk/src/prepass_types.rs` | the struct is DUPLICATED in the SDK with identical fields; both copies must move together or the legs skew |
| `LayerModule::run_infill(layer_index)` | `crates/slicer-sdk/src/traits.rs` | all impls + macro glue |
| `PaintRegionLayerView.layer_index` + `layer_index()` | `crates/slicer-sdk/src/traits.rs` | closes the s32/u32 boundary mismatch |
| `raw_polygons_by_layer` key | `crates/slicer-runtime/src/builtins/prepass_slice_producer.rs` | `HashMap<u32,_>` → `HashMap<i32,_>` |
| `MAX_LAYERS` bound | `crates/slicer-wasm-host/src/marshal/in_.rs` | signed re-derivation + lower bound for the negative band |

**Already signed — the pattern to follow, do not touch:**
`SupportPlanEntry.global_layer_index: i32`,
`LightningTreeEntry.global_layer_index: i32`,
`StageIoError::DuplicateSupportPlanEntry.global_layer_index: i32`.

**Deliberately NOT migrated:** `StageIoError::DuplicateLayerCommit.layer_index`
and `StageIoError::LayerSlotOutOfRange.layer_index` stay `usize` (true slot
positions); WIT `finalization-layer-finalization.wit`'s `layer-idx = u32` stays
`u32` (see `requirements.md` §Out of Scope).

**Already correctly signed on the wire:** `ir-types.wit` `layer-idx = s32`,
`paint-region-layer-view.layer-index`, `support-plan-entry-view.global-layer-index: s32`,
`support-plan-entry.global-layer-index: s32`
(`crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`
— nested dir, not a flat `deps/*.wit`), and the generated guest `run`
signatures. The only wire-side defect is the SDK-side `as u32` truncation.

### Enumerated blast radius (pre-baked; Step 2 executes, never discovers)

The implementing worker MUST dispatch this LOCATIONS sweep first and paste the
result into Step 2a's edit list before editing:

> Question: enumerate every file containing a struct literal or field
> assignment of `GlobalLayer {`, `ObjectLayerRef {`, `SliceIR {`,
> `PerimeterIR {`, `InfillIR {`, `SupportIR {`, `LayerCollectionIR {`,
> `RegionKey {`, `SupportCandidateSource {`, `SupportGeometryKey {`,
> `AnchoredEntity {`, `OrderedEventCollection {`, plus every
> `global_layer_index:` / `local_layer_index:` / `anchor_layer_index:` /
> `global_support_layer_index:` / `anchor_global_layer_index:` occurrence and
> every `fn run_infill(` impl; scope: `crates/ modules/`; return: LOCATIONS
> ≤20 entries, aggregated per file with per-file counts.

Known hot files from grounding (worker verifies counts): `slice_ir.rs` itself,
`crates/slicer-sdk/src/views.rs`, `crates/slicer-sdk/src/traits.rs`,
`crates/slicer-sdk/src/test_support/fixtures.rs` (two builder types),
`crates/slicer-wasm-host/src/marshal/{in_,out,native}.rs`,
`crates/slicer-wasm-host/src/host.rs`, `crates/slicer-macros/src/lib.rs` and
its `slicer_module_tdd.rs` / `binding_surface_tdd.rs` test files,
`crates/slicer-runtime/src/{blackboard.rs, layer_executor.rs,
region_partition.rs, layer_finalization.rs, visual_debug_render.rs}`,
`crates/slicer-runtime/src/builtins/{prepass_slice_producer,support_analysis_producer}.rs`,
`crates/slicer-gcode` consumers, `crates/pnp-cli/src/visual_debug.rs`, plus
executor/integration/contract tests constructing these structs. Literal-gate
rule applies: any watched-type literal in TEST code gains a `..` rest or an
`// exhaustive: <reason>` waiver per `docs/21_data_defaults_and_fixtures.md`;
production literals stay exhaustive.

## `raft_fill` Carrier Footprint (pre-baked)

A new `Vec<ExPolygon>` field on `SlicedRegion` touches ~14 non-test files. Use
the shipped `sparse_infill_area` / `internal_bridge_areas` fields as the exact
template; the sites are:

- `crates/slicer-ir/src/slice_ir.rs` — field decl (`#[serde(default)]`) +
  version-history doc entry.
- `crates/slicer-schema/wit/deps/ir-types.wit` — accessor on `slice-region-view`
  **and** on the perimeter region resource (two sites; the second is easy to
  miss, which is why AC-6 asserts a count of exactly 2).
- `crates/slicer-wasm-host/src/host.rs` — one accessor impl per resource.
- `crates/slicer-macros/src/lib.rs` — WIT→SDK marshal for both view legs.
- `crates/slicer-sdk/src/views.rs` — field, `Default`, `from_ir` clone, setter,
  and getters on both view types.
- `crates/slicer-sdk/src/test_support/fixtures.rs` — both builder types.
- `crates/slicer-core/src/algos/prepass_slice.rs` — exhaustive production
  literal.
- `crates/slicer-runtime/src/region_partition.rs` — **`split_field!(raft_fill);`**
  (omitting this silently drops the field on modifier-region splits).
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs`,
  `crates/slicer-runtime/src/layer_executor.rs` — population / consumption.
- `crates/slicer-runtime/src/visual_debug_render.rs` and
  `crates/pnp-cli/src/visual_debug.rs` — overlay + manifest emission, so 240b's
  visual gate can see raft fill.

## `raft_plan` Read-Path Footprint (pre-baked)

- `crates/slicer-schema/wit/deps/ir-types.wit` — new `raft-plan-view` record
  (mirror of the prepass `raft-plan`: `raft-layers: u32`,
  `raft-first-layer-density: f32`, `base-raft-layers: u32`,
  `interface-raft-layers: u32`) + `raft-plan` accessor on
  `paint-region-layer-view`.
- `crates/slicer-wasm-host/src/host.rs` — `PaintRegionLayerData.raft_plan`
  field + accessor impl that pushes `"SupportPlanIR"` to `runtime_reads`,
  matching the existing `support_plan_entries` impl. The 8 existing
  `PaintRegionLayerData` construction sites must move to FRU or `Default`.
- `crates/slicer-wasm-host/src/dispatch.rs` — populate in
  `build_paint_layer_data_with_plan` directly after the struct literal; raft is
  layer-independent, so unlike `support_plan_entries` it takes no
  `anchor_layer_index` filter.
- `crates/slicer-macros/src/lib.rs` — guest shim mirror beside the
  `support_plan_entries` reconstruction loop.
- `crates/slicer-sdk/src/traits.rs` — `PaintRegionLayerView::raft_plan()`
  beside `support_plan()`. **The native leg needs no other change**: it already
  hands the whole `Arc<SupportPlanIR>` to the view via `with_support_plan`, so
  `raft_plan` is already in hand there.

## Why This Packet Carries An L

The packet's own rule says an `L` step forces a split. This packet is `L` in
aggregate but has no `L` step: the migration is pre-split into 2a (crates) and
2b (modules + tests), and every other step is `S` or `M`. The aggregate cannot
be reduced further without producing a tree that does not compile between
packets — the retype, the consumers that break because of it, and the index
assignment that exercises it are a single compile unit. Activating this packet
requires the swarm extended band (240k reading / 300k hard stop) with a logged
ESCALATION.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - IR retypes + `raft_fill` + schema bump.
- `crates/slicer-sdk/src/{traits.rs, views.rs, test_support/fixtures.rs}` - trait signature, view retype, `raft_plan()` getter, `raft_fill` plumbing.
- `crates/slicer-macros/src/lib.rs` - stage-method signatures, truncation removal, both shims.
- `crates/slicer-schema/wit/deps/ir-types.wit` - `raft-fill` (×2), `raft-plan-view`, `raft-plan` accessor.
- `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit` - `is-raft-prefix`.
- `crates/slicer-wasm-host/src/{host.rs, dispatch.rs, marshal/in_.rs, marshal/native.rs, marshal/out.rs}` - index assignment, both legs, raft-plan projection.
- `crates/slicer-runtime/src/{layer_executor.rs, region_partition.rs, blackboard.rs}` and `crates/slicer-runtime/src/builtins/{prepass_slice_producer.rs, support_analysis_producer.rs}` - positional-consumer repair.
- `modules/core-modules/layer-planner-default/{src/lib.rs, layer-planner-default.toml}` - raft band emission.
- New test files listed in `packet.spec.md` §AC verification command rule, plus `mod` registrations in `crates/slicer-runtime/tests/{executor,integration}/main.rs`.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/DEVIATION_LOG.md`.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - symbol-scoped reads only; locate each
  with `rg -n 'pub struct <Name>'` at the moment of reading, never by a stored
  line pin (the file is >3k lines and pins rot). Structs needed: `GlobalLayer`,
  `ObjectLayerRef`, `SupportPlanEntry`, `RaftPlan`, `SupportPlanIR`,
  `SlicedRegion`, `SliceIR`, plus `CURRENT_SLICE_IR_SCHEMA_VERSION` and its
  version-history doc comment.
- `modules/core-modules/tree-support-planner/src/lib.rs` - the range around
  `push_raft_plan` only (locate with `rg -n push_raft_plan`); never load the
  ~5.9k-line file.
- `crates/slicer-runtime/src/blackboard.rs` - the range around `raft_plan_min`.
- `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` - 20 lines,
  full read; confirms which imports a `Layer::Infill` guest gets.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (T1: gitignored, glob-blind).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/tree-support-planner/src/lib.rs` beyond the cited
  range - planner algorithms are 238b's surface.
- `crates/slicer-core/src/algos/support_geometry.rs` and other
  238a/238b/238c/239/241-owned files - delegate symbol lookups; do not browse.
- `modules/core-modules/raft-default/**` - 240b's surface; does not exist yet
  and must not be created here.
- `crates/slicer-runtime/src/perimeter*` and the perimeter generators - the
  DEV-124 clamp is recorded here, never edited here.

## Expected Sub-Agent Dispatches

- LOCATIONS sweep for the migration blast radius (question verbatim in
  §Enumerated Blast Radius); scope `crates/ modules/`; return LOCATIONS;
  purpose: Step 2a edit list.
- LOCATIONS sweep for `CURRENT_SLICE_IR_SCHEMA_VERSION` assertion sites; scope
  `crates/`; return LOCATIONS ≤20; purpose: Step 7 bump fallout.
- FACT: does `ir-types.wit` compile with a locally-declared `raft-plan-view`
  record (no cross-world import)? scope `crates/slicer-schema/wit/`; return
  FACT; purpose: Step 8.
- OrcaSlicer SUMMARY: `generate_support_layers` below-zero print_z insertion;
  return SUMMARY; purpose: Step 5 semantics check.

## Data and Contract Notes

- IR/manifest contracts: `SliceIR` schema minor-bumped to `4.9.0`;
  `SlicedRegion.raft_fill` serde-defaulted so old JSON loads; config keys
  snake_case (E9).
- WIT boundary: canonical sources live at `crates/slicer-schema/wit/` (both
  host `bindgen!` and guest `include_str!` read them); after any WIT edit run
  `cargo build --tests`, then rebuild guests (T4).
- `layer-planner-default`'s manifest must declare `support_raft_layers` in
  `[config.schema]` or the module config view will silently resolve an in-code
  default (E9).

## Locked Assumptions and Invariants

- Rafts remain signed negative global-layer prefix entries; never anchored
  entities (plan §15; ADR-0009).
- `SupportPlanEntry.global_layer_index: i32` semantics (`-N..-1` raft band,
  `0..` model) extend unchanged to the newly-signed fields.
- After this packet, `GlobalLayer.index != Vec position` is a permanent
  property of the IR. Any future code indexing a layer-parallel Vec by `index`
  is a bug.
- The two WIT `layer-idx` aliases stay divergent: `ir-handles` `s32`,
  `finalization-layer-finalization` `u32`. This asymmetry is deliberate and
  documented in that file's own comment.
- Invariant 16: every acceptance command names `--exact` tests or asserts a
  non-zero matched count in the same run.

## Risks and Tradeoffs

- **Migration breadth:** the retype ripples through call-site `as` casts and
  test literals well beyond the field list. Mitigated by the pre-baked
  LOCATIONS sweep and the 2a/2b split; if the sweep exceeds ~20 files in either
  half, split again before editing.
- **Silent sign truncation:** any residual `as u32` on a layer index converts
  `-1` to `4294967295` without a compile error. Step 3's exit condition
  requires a repo-wide grep for `as u32` on layer-index expressions, not just a
  green build.
- **Schema bump fallout:** tests hard-asserting the old SliceIR schema version
  fail loudly; bump + fallout land in one step by design.
- **wasm/native leg skew (T9):** `raft_fill` must be projected in BOTH marshal
  legs and `is-raft-prefix` handled in both harvest legs; AC-4 asserts both
  legs explicitly for exactly this reason.
- **DEV-124 reopen:** the chosen indexing convention invalidates a shipped fix.
  Recorded and routed in `requirements.md` §DEV-124 Reopen rather than silently
  absorbed; 240b re-verifies under a live raft.

## Context Cost Estimate

- Aggregate: `L` (justified in §Why This Packet Carries An L)
- Largest step: `M` (Steps 2a, 2b, 7, 8)
- Highest-risk dispatch and required return format: LOCATIONS blast-radius
  sweep (must aggregate per-file counts, not raw hits)

## Open Questions

- [FWD] Does `SupportGeometryKey.global_support_layer_index` index a
  `Vec<LayerCollisionCache>` directly anywhere (its doc comment warns that it
  might)? If so that site joins §Positional Consumer Ruling. Worker resolves in
  Step 2a via the LOCATIONS result; no activation blocker.
- [FWD] Exact `active_regions` seeding for a raft layer — one synthetic region
  per object, or a single sentinel region? Worker decides in Step 6 against the
  `derive_layer_output_envelope_from_input` fallback behaviour and records the
  choice in a code comment; no activation blocker.
- None [BLOCK].
