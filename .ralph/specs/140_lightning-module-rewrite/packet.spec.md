---
status: active
packet: 140_lightning-module-rewrite
task_ids:
  - TASK-265
backlog_source: docs/07_implementation_status.md
context_cost_estimate: L
---

# Packet Contract: 140_lightning-module-rewrite

## Goal

Rewrite `modules/core-modules/lightning-infill` as a per-layer sampler: read the layer's
tree segments from the packet-137 / packet-139 `LightningTreeIR` view (accessed via
the `PaintRegionLayerView` SDK accessor `lightning_tree_segments_for(object_id,
region_id)`, which 139 upgrades to per-region keying), emit them as raw
`ExtrusionPath3D` polylines with `ExtrusionRole::SparseInfill` and the config-derived
`speed_factor`, delete the single-layer stub (the `build_branches` function at
`lib.rs:234` and the inline grid-sampling machinery in `run_infill`/`fill_expolygon`),
**port the full `getBestGroundingLocation` grounding search into
`crates/slicer-core/src/algos/lightning/layer.rs` (closing the 139 Step-2
`D-139-LAYER-GROUNDING-SEARCH-STUB` stub — `wall_supporting_radius` becomes a
load-bearing parameter)**, close DEV-081, run the contained lightning re-bless +
roadmap-close workspace ceremony, and **close the D-137-WIT-RUN-INFILL-NO-PAINT-VIEW
deviation** by extending the WIT `run-infill` signature with a
`paint: paint-region-layer-view` argument, bumping `slicer:world-layer@2.2.0` →
`@2.3.0`, threading the paint view through the SDK trait + macro glue + host
dispatch + the four `run_infill`-implementing core modules, and adding a real
`Layer::Infill` test-guest that calls `lightning-tree-segments` through the WIT
boundary.

## Scope Boundaries

140 is **the lightning packet** — module rewrite + WIT closure + grounding-search
refinement in `slicer-core`. The boundary between generation (host-side, lives in
`slicer-core/src/algos/lightning/`) and sampling (module-side, lives in
`modules/core-modules/lightning-infill/`) stays the ADR-0029 seam, but 140 owns
both sides: the generation side gets the full grounding search (Step 0) so that the
sampling side samples higher-quality trees. The 138/139 producer surface is no
longer "defects routed, not patched" — 140 patches `crates/slicer-core/src/algos/lightning/{layer,tree_node}.rs`
specifically for the grounding search and records any further 138/139 surface
changes as deviations. Manifest claims stay `["claim:sparse-fill"]`; the WIT
`run-infill` signature is extended (one additive argument) but no other WIT
change is in scope. The four `run_infill`-implementing core modules
(rectilinear/gyroid/lightning/top-surface-ironing) are updated to take the new
paint-view arg (only `lightning-infill` calls it; the other three take `_paint` and
ignore it). `support-surface-ironing` implements only `run_infill_postprocess` and
is NOT in scope. DEV-081 flips to Closed, `D-139-LAYER-GROUNDING-SEARCH-STUB` flips
to Closed, `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` flips to Closed, and
lightning-affected expectations are re-blessed in one justified event.

## Prerequisites and Blockers

- Depends on: `137_lightning-prepass-contract` (view, `LightningTreeIR` —
  `status: implemented`), `138_lightning-distancefield-treenode` (primitives —
  `status: implemented`), `139_lightning-layer-generator` (real trees committed,
  per-region keying — `status: implemented`), `133_infill-linker-module` (the
  linker connects the emission — `status: implemented`).
- **DEVIATION-CLOSURE DEP on packet 137's review** — this packet must extend
  the WIT `run-infill` signature with `paint: paint-region-layer-view`,
  bump `slicer:world-layer@2.2.0` → `@2.3.0`, extend the SDK trait
  `LayerModule::run_infill` at `crates/slicer-sdk/src/traits.rs:369`, update
  the slicer-macros `infill_arm` at
  `crates/slicer-macros/src/lib.rs:1779-1794` and the macro-emitted glue at
  `:2804-2809`, update the host dispatch `Layer::Infill` arm at
  `crates/slicer-wasm-host/src/dispatch.rs:442-465` to mirror the
  `Layer::Support` arm at `:584-619`, update the four
  `run_infill`-implementing core modules (rectilinear/gyroid/lightning/top-
  surface-ironing) to take the new arg, extend the
  `layer-infill-guest` test-guest to call `lightning-tree-segments` through
  the WIT seam, and re-baseline the `wit_drift_detection_tdd` test that
  pins the `run-infill` signature string. Closes
  `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` in `docs/DEVIATION_LOG.md`.
- **DEVIATION-CLOSURE DEP on packet 139** — this packet must port the full
  `getBestGroundingLocation` (Orca `Layer.cpp::getBestGroundingLocation`, the
  TBB-style parallel grid scan + tree-node locator + `wall_supporting_radius`
  exclusion) into `crates/slicer-core/src/algos/lightning/layer.rs`,
  remove the 139 Step-2 stub comment from `Layer::generate_new_trees`, and
  co-update the 139 test home (`crates/slicer-core/tests/algo_lightning_tdd.rs`)
  with the new AC-G1 + AC-G2 tests in the same step. Closes
  `D-139-LAYER-GROUNDING-SEARCH-STUB` in `docs/DEVIATION_LOG.md`.
- Unblocks: — (roadmap end).
- Activation blockers: 137 and 139 must both be `status: implemented`
  (forward-deps above). Packet cost is L (justified unsplittable — generation
  + sampling + WIT closure are tightly coupled at the per-layer seam; the
  swarm will run in extended band per the escalation protocol).

## Acceptance Criteria

- **AC-1. Given** a lightning-configured object with committed `LightningTreeIR` trees,
  **when** `run_infill` dispatches, **then** the module emits exactly the layer's tree
  segments as raw `ExtrusionPath3D` polylines with `role == ExtrusionRole::SparseInfill`
  and the config-derived `speed_factor` — count and endpoint equality against the IR
  view (the module adds NO geometry of its own). | `cargo test -p lightning-infill -- samples_tree_ir_raw_emit 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the module source, **when** grepped, **then** `build_branches` and the
  inline grid-sampling machinery are deleted (zero definitions) and no
  `clip_polyline`/`connect_branches` call remains. | `rg -c 'fn build_branches|fn nearest_boundary_point|fn polygon_bbox_mm|fn point_in_expolygon|fn point_in_polygon|fn sample_grid|clip_polyline|connect_branches' modules/core-modules/lightning-infill/src/lib.rs | grep -q '^0$' && echo STUB-GONE`
- **AC-3. Given** an end-to-end lightning-configured slice with the linker active, **when**
  `Layer::InfillPostProcess` commits, **then** the sparse bucket contains linked multi-point
  polylines derived from tree segments (mean points-per-path > 2) — lightning flows through
  Architecture A like every other module. | `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3a. Given** the WIT canonical `world-layer.wit`, **when** grepped, **then** the
  `run-infill` export's signature includes `paint: paint-region-layer-view` and the
  package version reads `slicer:world-layer@2.3.0` (the D-137-WIT-RUN-INFILL-NO-PAINT-VIEW
  closure). | `rg -n 'run-infill: func\(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view' crates/slicer-schema/wit/deps/world-layer/world-layer.wit && rg -n 'package slicer:world-layer@2.3.0;' crates/slicer-schema/wit/deps/world-layer/world-layer.wit`
- **AC-3b. Given** the `layer-infill-guest` test-guest under
  `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs`, **when** rebuilt
  via `cargo xtask build-guests`, **then** the guest's `fn run_infill` accepts the
  `paint: PaintRegionLayerView` argument, calls
  `paint.lightning_tree_segments(object_id, region_id)` for each region in the
  per-layer loop, and emits a witness path encoding the segment count
  (`width == count_marker, x == 137.0`). The host pipeline reaches the test guest
  end-to-end through the WIT boundary (this satisfies D-137's original AC-4
  wording — "a `Layer::Infill` test guest calling the new read-view method
  lightning-tree-segments"). | `cargo test -p slicer-wasm-host --test contract -- lightning_infill_guest_calls_lightning_tree_segments 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3c. Given** the four `run_infill`-implementing core modules
  (rectilinear-infill, gyroid-infill, lightning-infill, top-surface-ironing),
  **when** compiled via `cargo check --workspace --all-targets`, **then** each
  module's `fn run_infill` signature accepts the new
  `paint: &PaintRegionLayerView` argument; only `lightning-infill` actually
  calls it (the other three bind `_paint` and ignore it). | `rg -n 'fn run_infill\(' modules/core-modules/{rectilinear,gyroid,lightning,top-surface-ironing}-infill/src/lib.rs`
- **AC-3d. Given** the `wit_drift_detection_tdd` suite, **when** run, **then** the
  new assertions for the `run-infill` paint-view signature AND the
  `world-layer@2.3.0` package version both pass. | `cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** `docs/DEVIATION_LOG.md`, **when** grepped, **then** the DEV-081 row's
  status column reads `Closed` (or the open status is replaced by a `Closed … packet
  140` suffix per the live log's convention — FACT at the time of editing). |
  `rg -q 'DEV-081.*[Cc]losed.*140|DEV-081.*140.*[Cc]losed' docs/DEVIATION_LOG.md && echo DEV-CLOSED`
- **AC-5. Given** lightning-affected test expectations, **when** this packet closes,
  **then** each re-bless carries a closure-log justification and was captured from two
  consecutive identical runs (contained lightning bless — the roadmap's second and final
  bless event). | `cargo test -p lightning-infill 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-G1. Given** a synthetic overhang point at exactly `wall_supporting_radius` distance
  from a wall, **when** `Layer::get_best_grounding_location` runs, **then** the chosen
  grounding location is NOT that wall (the radius is an exclusionary distance — walls
  within `wall_supporting_radius` are skipped to avoid spurious reattachment, per Orca
  `Layer.cpp::getBestGroundingLocation` semantics). | `cargo test -p slicer-core -- lightning_layer_wall_supporting_radius 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-G2. Given** the 139 Step-3 prism with a single internal overhang (AC-2 fixture),
  **when** the full grounding search replaces the 139 Step-2 stub, **then** the per-layer
  continuity invariant still holds — every layer's tree endpoints lie within
  `prune_length` of the layer below's trees or outline (no continuity regression from the
  grounding refinement). | `cargo test -p slicer-core -- lightning_generator_tree_continuity 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a default-config slice (no lightning holder) of
  `resources/regression_wedge.stl`, **when** run, **then** the g-code SHA is byte-identical
  (the rewrite touches nothing outside the lightning path). | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** a lightning-configured layer whose committed `LightningTreeIR` has zero
  segments (e.g. no overhangs), **when** the module runs, **then** it emits nothing for
  that layer and the slice completes (no panic, no fallback to the deleted stub). |
  `cargo test -p lightning-infill -- empty_trees_emit_nothing 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p lightning-infill 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo xtask test --workspace --summary` (roadmap-close ceremony — dispatch; FACT
  verdict only)
- `cargo xtask build-guests --check` (rebuild if STALE)

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §Phase L4 — full read (short).
- `docs/adr/0029-lightning-prepass-tree-generator.md` — module-sampler contract;
  delegate SUMMARY.
- `docs/DEVIATION_LOG.md` — DEV-081 row (the closure target) AND
  `D-139-LAYER-GROUNDING-SEARCH-STUB` row (also closed by this packet).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (37 lines) / `.hpp` (42 lines) — `Filler::_fill_surface_single`: what per-layer transformation (if any) Orca applies between `getTreesForLayer` and emission — the module must mirror only the sampling side (generation is host-side per ADR-0029; linking is the 133 linker's).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` — DEV-081 status → `Closed` (packet 140) —
  `rg -q 'DEV-081.*[Cc]losed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` status →
  `Closed` (packet 140) —
  `rg -q 'D-137-WIT-RUN-INFILL-NO-PAINT-VIEW.*[Cc]losed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — `D-139-LAYER-GROUNDING-SEARCH-STUB` status →
  `Closed` (packet 140 Step 0 — full `getBestGroundingLocation` ported) —
  `rg -q 'D-139-LAYER-GROUNDING-SEARCH-STUB.*[Cc]losed.*140' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` — TASK-262…TASK-265 closure sweep —
  `rg -q 'TASK-265.*[Cc]losed' docs/07_implementation_status.md`
- `docs/03_wit_and_manifest.md` §`world-layer.wit` — update the package version
  `2.2.0` → `2.3.0` line and add a `run-infill` paint-view bullet (the WIT
  signature change is load-bearing for this packet's deviation closure).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
