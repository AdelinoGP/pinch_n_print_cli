# Design: 240a-support-raft-substrate

## Controlling Code Paths

- Band creation: `com.core.layer-planner-default` (`push-layer`) →
  `harvest_layer_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
  and the `PrePass::LayerPlanning` arm of
  `crates/slicer-wasm-host/src/marshal/native.rs` → `LayerPlanIR.global_layers`
  → `promote_global_layers` (`crates/slicer-runtime/src/layer_executor.rs`).
- Band consumption (unchanged, guarded by AC-4): `hydrate_slice_arena`
  (`crates/slicer-runtime/src/layer_executor.rs`),
  `execute_prepass_slice_all_layers`
  (`crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`),
  `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`.
- Object-bottom predicates: `detect_support_contacts`
  (`crates/slicer-core/src/algos/overhang_annotation.rs`, fed the global index
  by `support_analysis_producer.rs`), `run_perimeters`
  (`modules/core-modules/classic-perimeters/src/lib.rs`).
- Raft-plan read path: `SupportPlanIR.raft_plan` (blackboard) →
  `build_paint_layer_data_with_plan` (`crates/slicer-wasm-host/src/dispatch.rs`)
  → host `PaintRegionLayerData.raft_plan` → WIT
  `paint-region-layer-view.raft-plan` → `slicer-macros` guest shim → SDK
  `PaintRegionLayerView::raft_plan()`.
- Raft-marker read path (parallel, same files): `GlobalLayer.is_raft`
  (`LayerPlanIR`) → `build_paint_layer_data_with_plan` → host
  `PaintRegionLayerData.is_raft` → WIT `paint-region-layer-view.is-raft` →
  `slicer-macros` guest shim → SDK `PaintRegionLayerView::is_raft()`. This is
  the path 240b's `Layer::Infill` raft module depends on.
- Neighboring tests/fixtures:
  `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`
  (DEV-124 pins; must stay unmodified), fixture
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.
- OrcaSlicer comparison: see `requirements.md` section "OrcaSlicer Reference
  Obligations"; do not repeat delegation rules.

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

## Code Change Surface

### Selected approach

1. **Marker first:** add `GlobalLayer.is_raft: bool` (`#[serde(default)]`) and
   WIT `layer-proposal.is-raft-prefix: bool`; wire both harvest legs to copy
   the flag through; reject a raft-marked run that is not contiguous at the
   front. Index assignment is untouched — it is already correct.
2. **Producer:** `com.core.layer-planner-default` reads `support_raft_layers`
   (declared in its manifest so E9 cannot fire) and pushes the band first.
3. **Audit:** apply section "First-Model-Layer Audit" verbatim — three
   conversions, everything else explicitly untouched.
4. **Carrier:** `SlicedRegion.raft_fill`, both `ir-types.wit` resources,
   `split_field!(raft_fill);`, host/SDK/macro/fixture projection, schema bump.
5. **Read accessors:** `raft-plan-view` record + BOTH the `raft-plan` and
   `is-raft` accessors on `paint-region-layer-view` + host fields + dispatch
   population + macro shim + SDK getters. `is-raft` is what lets a
   `Layer::Infill` guest identify a raft layer; without it 240b cannot work.

### Why the raft band is produced at `PrePass::LayerPlanning`

`VALID_STAGES` (`crates/slicer-schema/src/lib.rs`) orders
`PrePass::LayerPlanning` before `PrePass::SupportGeometry`, and both support
planners declare `LayerPlanIR` in their `reads`. Producing the band at layer
planning therefore makes it visible to the support planners, which 240b needs.
`RaftPlan` is not available that early and does not need to be: it is derived
from the same config keys, so the planner reads `support_raft_layers` directly.
`RaftPlan`'s `base_raft_layers` / `interface_raft_layers` matter only to 240b's
geometry, not to band creation.

### First-Model-Layer Audit (normative; do not re-decide per site)

The discriminator: **"object bottom geometry" means `support_raft_layers`;
"physical first layer on the plate" stays `0`, because under this band the raft
IS the physical first layer.**

**Config reach is the hard part, not the comparison.** Two of the three sites
live in `detect_support_contacts`, which reads only `SupportContactParams`. That
struct has no raft field, and its ONLY bridge from `ResolvedConfig` is
`resolve_contact_params`
(`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`), a PRIVATE
`fn` which today hardcodes `enforce_support_layers: 0` and `layer_id: 0` and
contains zero `raft` references. Adding a raft field without editing that bridge leaves it at its
default: the conversion compiles, the test can still pass, and the behaviour is
inert. This is the same failure DEV-124 hit from the other direction (raft keys
invisible to the perimeter modules until their manifests declared them). The
bridge is therefore in Step 4's edit surface and AC-5 asserts it explicitly.

**Convert to raft-aware (3 sites):**

| Site | Expression today | Ruling |
| --- | --- | --- |
| `detect_support_contacts` sharp-tail gate (`crates/slicer-core/src/algos/overhang_annotation.rs`) | `params.support_sharp_tails && params.layer_id == 0` | **Convert.** `params.layer_id` is fed the GLOBAL index (`layer_id: *layer_index` in `support_analysis_producer.rs`). Sharp tails are an object-bottom-geometry exception; under a raft this fires on the raft. |
| `detect_support_contacts` enforce window (same file) | `params.layer_id < params.enforce_support_layers` | **Convert to a shifted window** (`raft .. raft + enforce_support_layers`). Canonical counts these object-relative. NOTE: `resolve_contact_params` hardcodes `enforce_support_layers: 0`, so under `u32` this predicate is **always false today** — the conversion is correct-by-construction but unobservable until that key is wired. Do not chase a behavioural difference here; assert the shifted arithmetic directly. |
| `run_perimeters` overlap-key selection (`modules/core-modules/classic-perimeters/src/lib.rs`) | `if layer_index == 0 \|\| top_shell == Some(0) { "top_bottom_infill_wall_overlap" }` | **Convert.** The in-code comment says "layer zero is always bottom-surface context" — that is the object's bottom. |

**Leave alone (explicitly ruled; changing any of these is a regression):**

| Site class | Reason |
| --- | --- |
| `resolve_role_width(..., layer_index == 0, ...)` in `classic-perimeters` (`run_perimeters`), `rectilinear-infill`, `wave-overhangs` | First-layer LINE WIDTH. Canonical applies `initial_layer_line_width` at the physical first layer, which under this band is the raft. Correct at `0`. |
| `arachne_params_from_config(config, layer_index == 0)` (`arachne-perimeters`) | Same: first-layer flow flag, physical layer. |
| `params.is_initial_layer = layer_index == 0` (`run_perimeters`, `arachne-perimeters`) | Deliberate. The DEV-124 comment above it states `is_initial_layer` is the physical layer 0 while `is_bottom_layer` carries object-bottom meaning. |
| `layer_height_mm`'s `if layer_index == 0 { return layer.z; }` (`support_analysis_producer.rs`) | "No predecessor layer" guard. Layer 0 is still layer 0. |
| `if layer_id == 0 { continue; }` (`crates/slicer-core/src/algos/lightning/generator.rs`, both sites) | Local Vec index guarding `[idx - 1]`. |
| `idx == 0` guards in `overhang-classifier-default`, `tree-support-planner`, `part-cooling` | Local slice/enumerate indices, not global layer identity. |
| Medial-axis log-once guard (`run_perimeters`, `classic-perimeters`) | Logging only; no geometry effect. |

**Deferred with a `[FWD]` note, not converted here (see section "Open
Questions"):** the arachne sandwich-order override, the `rectilinear-infill`
bottom-solid-fill width flag, and `part-cooling`'s
`close_fan_the_first_x_layers`. Each is defensible as physical-layer semantics;
converting them without a canonical ruling risks a parity regression. 240b
re-examines them under a live raft.

### Rejected alternatives and reasons

- **Signed negative band (`-N..-1`) with a `u32` to `i32` migration:**
  withdrawn. See `requirements.md` section "Banding Decision" — canonical does
  the opposite, the cited ADR-0009 authority does not exist, and it would
  reopen DEV-124. Cost was a ~15-field type migration rippling through
  hundreds of call sites; benefit was index stability that the explicit
  `is_raft` marker delivers instead.
- **Deriving raft-ness from `index < support_raft_layers`:** rejected —
  requires config reach at every consumer, the exact problem DEV-124
  documented (`ConfigView::from_declared` drops undeclared keys).
- **`LayerPlanIR.raft_layer_count: u32` instead of a per-layer flag:**
  rejected — several consumers hold a `GlobalLayer` without the plan.
- **Importing the prepass `raft-plan` record into `ir-handles`:** rejected —
  cross-world record import risks a world-satisfaction failure (the same reason
  `finalization-layer-finalization.wit` keeps its own `layer-idx`). Declare a
  `raft-plan-view` record in `ir-types.wit`, exactly mirroring how
  `support-plan-entry-view` mirrors the prepass `support-plan-entry`.
- **Anchored entities for raft layers:** rejected — plan section 15 prohibition.

## `raft_fill` Carrier Footprint (pre-baked)

Use the shipped `sparse_infill_area` / `internal_bridge_areas` fields as the
exact template; the sites are:

- `crates/slicer-ir/src/slice_ir.rs` — field decl (`#[serde(default)]`) plus
  the version-history doc entry (shared with `is_raft`).
- `crates/slicer-schema/wit/deps/ir-types.wit` — accessor on `slice-region-view`
  **and** on the perimeter region resource (two sites; the second is easy to
  miss, which is why AC-6 asserts a count of exactly 2).
- `crates/slicer-wasm-host/src/host.rs` — one accessor impl per resource.
- `crates/slicer-macros/src/lib.rs` — WIT to SDK marshal for both view legs.
- `crates/slicer-sdk/src/views.rs` — field, `Default`, `from_ir` clone, setter,
  and getters on both view types.
- `crates/slicer-sdk/src/test_support/fixtures.rs` — both builder types.
- `crates/slicer-core/src/algos/prepass_slice.rs` — exhaustive production
  literal.
- `crates/slicer-runtime/src/region_partition.rs` — **`split_field!(raft_fill);`**
  beside `split_field!(internal_bridge_areas);` (omitting this silently drops
  the field on modifier-region splits).
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs`,
  `crates/slicer-runtime/src/layer_executor.rs` — population / consumption.
- `crates/slicer-runtime/src/visual_debug_render.rs` and
  `crates/pnp-cli/src/visual_debug.rs` — overlay + manifest emission, so 240b's
  visual gate can see raft fill.

## `raft_plan` Read-Path Footprint (pre-baked)

- `crates/slicer-schema/wit/deps/ir-types.wit` — new `raft-plan-view` record
  mirroring `RaftPlan`'s four verified fields (`raft-layers: u32`,
  `raft-first-layer-density: f32`, `base-raft-layers: u32`,
  `interface-raft-layers: u32`) plus TWO accessors on
  `paint-region-layer-view`: `raft-plan: func() -> option<raft-plan-view>` and
  `is-raft: func() -> bool`. The `is-raft` accessor is mandatory, not optional:
  240b's `com.core.raft-default` is a `Layer::Infill` guest and has no other
  way to tell a raft layer from a model layer. Omitting it reproduces the
  "declared read with no WIT accessor" trap — validates clean, fails at runtime.
  This file also carries a STALE header comment claiming raft entries "carry
  negative `global_layer_index`"; Step 5 corrects it.
- `crates/slicer-wasm-host/src/host.rs` — `PaintRegionLayerData.raft_plan` and
  `.is_raft` fields plus accessor impls; the `raft_plan` impl pushes
  `"SupportPlanIR"` to `runtime_reads` and the `is_raft` impl pushes
  `"LayerPlanIR"`,
  matching the existing `support_plan_entries` impl. Two
  `PaintRegionLayerData` struct literals exist crate-wide — one in
  `paint_region_ir_to_layer_data` (`crates/slicer-wasm-host/src/host.rs`) and
  one in `crates/slicer-wasm-host/src/dispatch.rs` — and both must move to FRU
  or `Default`. Several further functions merely *return*
  `PaintRegionLayerData` and need no change; re-derive the exact set at edit
  time (`rg -n 'PaintRegionLayerData' crates/slicer-wasm-host/src`).
- `crates/slicer-wasm-host/src/dispatch.rs` — populate in
  `build_paint_layer_data_with_plan` directly after the struct literal; raft is
  layer-independent, so unlike `support_plan_entries` it takes no
  `anchor_layer_index` filter.
- `crates/slicer-macros/src/lib.rs` — guest shim mirror beside the
  `support_plan_entries` reconstruction loop.
- `crates/slicer-sdk/src/traits.rs` — `PaintRegionLayerView::raft_plan()` and
  `::is_raft()` beside `support_plan()`. **The native leg needs no other
  change** for `raft_plan`: it already hands the whole `Arc<SupportPlanIR>` to
  the view via `with_support_plan`. `is_raft` is carried on the same view
  struct and set where the view is built.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - `GlobalLayer.is_raft`, `SlicedRegion.raft_fill`, schema bump, and BOTH `SupportPlanEntry` negative-band doc promises (struct-level and field-level).
- `crates/slicer-sdk/src/{traits.rs, views.rs, test_support/fixtures.rs}` - `raft_plan()` / `is_raft()` getters, `raft_fill` plumbing.
- `crates/slicer-sdk/src/prepass_types.rs` - `LayerProposal.is_raft` mirror. The native leg reads the SDK `LayerProposal` (fields `z`, `active_regions` today), so without this mirror the native leg cannot carry the flag at all and AC-2's both-legs equality is unsatisfiable.
- `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` - must actually CALL the new `raft-plan` / `is-raft` accessors, following its own shipped `paint.lightning_tree_segments(...)` precedent. Adding accessors without a caller makes AC-7 vacuous.
- `crates/slicer-macros/src/lib.rs` - both shims, AND the WIT `LayerProposal { z, active_regions }` literal in the layer-planning glue emitter, which the `is-raft-prefix` record change breaks.
- `crates/slicer-schema/wit/deps/ir-types.wit` - `raft-fill` (x2), `raft-plan-view`, `raft-plan` and `is-raft` accessors, plus (Step 5) correcting its stale header comment claiming raft entries carry negative `global_layer_index`.
- `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit` - `is-raft-prefix`.
- `crates/slicer-wasm-host/src/{host.rs, dispatch.rs, marshal/in_.rs, marshal/native.rs}` - marker on both legs, contiguity rejection, raft-plan projection.
- `crates/slicer-runtime/src/region_partition.rs` - `split_field!(raft_fill);`.
- `crates/slicer-core/src/algos/overhang_annotation.rs` - two audit conversions plus the new raft-boundary field on `SupportContactParams`.
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - `resolve_contact_params` must populate that field from `support_raft_layers`; without this edit the conversion is inert (see §First-Model-Layer Audit). Its unit test goes in this file's own `#[cfg(test)] mod tests` and runs under `--lib`, because `resolve_contact_params` is private and `tests/` is a separate crate. Filter it by the FULL module path (`builtins::support_analysis_producer::tests::<name>`) — with `--lib`, `--exact` matches the complete path and a bare name matches nothing (measured).
- `modules/core-modules/classic-perimeters/src/lib.rs` - one audit conversion (overlap key ONLY; the DEV-124 wall clamp is untouched).
- `modules/core-modules/layer-planner-default/{src/lib.rs, layer-planner-default.toml}` - raft band emission + key declaration.
- `crates/slicer-core/src/algos/prepass_slice.rs`, `crates/slicer-runtime/src/{slice_postprocess_prepass.rs, layer_executor.rs, visual_debug_render.rs}`, `crates/pnp-cli/src/visual_debug.rs` - the `raft_fill` carrier footprint (Step 6). Listed here explicitly so this section stays authoritative: the Context Discipline Note declares Files in Scope binding, so a footprint file absent from it is a contradiction.
- `crates/slicer-wasm-host/tests/contract/prepass_output_builder_validation_tdd.rs` (11 WIT literals), `crates/slicer-sdk/tests/prepass_module_tdd.rs` (10 SDK literals), `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` (2 SDK literals) - exhaustive `LayerProposal` literals broken by the `is-raft-prefix` / `is_raft` additions (Step 2). Neither `LayerProposal` type has a `Default` or FRU escape, and at 2 fields they sit below the >=5 struct-literal-gate watchlist threshold, so nothing forced `..` on them.
- `crates/slicer-wasm-host/src/marshal/native.rs` - also Step 7: the native `PaintRegionLayerView` is constructed here and `is_raft` must be set at construction.
- New test files listed in `packet.spec.md` section "AC verification command rule", plus `mod` registrations in `crates/slicer-runtime/tests/{executor,integration,contract}/main.rs`.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/DEVIATION_LOG.md`.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - symbol-scoped reads only; locate each
  with `rg -n 'pub struct <Name>'` at the moment of reading, never by a stored
  line pin (the file is >3k lines and pins rot). Structs needed: `GlobalLayer`,
  `RaftPlan`, `SupportPlanIR`, `SlicedRegion`, plus
  `CURRENT_SLICE_IR_SCHEMA_VERSION` and its version-history doc comment.
- `modules/core-modules/arachne-perimeters/src/lib.rs` - the `run_perimeters`
  range around `is_bottom_layer` only; it is the DEV-124 template to copy, and
  is NOT edited by this packet.
- `crates/slicer-runtime/src/blackboard.rs` - the range around `raft_plan_min`.
- `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` - 20 lines,
  full read; confirms which imports a `Layer::Infill` guest gets.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (T1: gitignored, glob-blind).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/tree-support-planner/src/lib.rs` beyond a cited range -
  planner algorithms are 238b's surface.
- `modules/core-modules/raft-default/**` - 240b's surface; does not exist yet
  and must not be created here.
- `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` -
  READ-ONLY. AC-N4 asserts `git diff --quiet` on it.
- The DEV-124 wall clamps in both perimeter generators - correct as shipped;
  read as a template, never edit.
- `crates/slicer-core/src/algos/support_geometry.rs` and other
  238a/238b/238c/239/241-owned files - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- LOCATIONS: `CURRENT_SLICE_IR_SCHEMA_VERSION` assertion sites; scope
  `crates/`; return LOCATIONS ≤20; purpose: Step 6 bump fallout.
- FACT: does `ir-types.wit` resolve with a locally-declared `raft-plan-view`
  record (no cross-world import)? scope `crates/slicer-schema/wit/`; return
  FACT; purpose: Step 7.
- SNIPPETS ≤10 lines: what does `derive_layer_output_envelope_from_input`
  return for a layer with empty `active_regions`? scope
  `crates/slicer-wasm-host/src/dispatch.rs`; purpose: Step 3 seeding.
- OrcaSlicer SUMMARY: `new_layers` (`PrintObjectSlice.cpp`) raft id offset and
  `generate_object_layers` (`Slicing.cpp`) Z discipline; purpose: Steps 2-3.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Steps 2, 3, 6, 7)
- Highest-risk dispatch and required return format: the
  `CURRENT_SLICE_IR_SCHEMA_VERSION` LOCATIONS sweep (must aggregate per-file,
  not raw hits)

## Open Questions

- [FWD] Arachne sandwich-order override
  (`wall_sequence == WallSequence::InnerOuterInner && layer_index == 0`,
  `modules/core-modules/arachne-perimeters/src/lib.rs`): canonical disables
  sandwich ordering at physical `layer_id == 0` for adhesion, which suggests
  leaving it; but if the intent is object-bottom it needs `support_raft_layers`.
  Resolve against a canonical read in 240b under a live raft; not an activation
  blocker.
- [FWD] `rectilinear-infill` bottom-solid-fill width flag: currently ruled a
  physical-layer line-width flag (leave alone). Revisit only if PnP wants
  object-bottom flow on the first model layer.
- [FWD] `part-cooling`'s `close_fan_the_first_x_layers`: canonical counts these
  physically (raft included), so ruled leave-alone; confirm in 240b.
- None [BLOCK].
