# Design: 140_lightning-module-rewrite

## Controlling Code Paths

- Primary code path: `modules/core-modules/lightning-infill/src/lib.rs` — `run_infill`
  keeps its region loop, `should_emit(SparseInfill)` gate, `begin_region` origin
  discipline (at `lib.rs:117`), and config reads (`on_print_start`); the body between
  them becomes: view (via
  `PaintRegionLayerView::lightning_tree_segments_for(object_id, region_id)` in
  `crates/slicer-sdk/src/traits.rs`) → layer tree segments → raw `push_sparse_path`
  per segment/polyline. `build_branches` (at `lib.rs:234`, called from
  `lib.rs:195`) and the grid-sampling helpers `nearest_boundary_point`,
  `polygon_bbox_mm`, `point_in_expolygon`, `point_in_polygon` are deleted.
- Neighboring tests or fixtures: `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`
  (323 lines — pins stub behavior; rewritten), `tests/slicer_module_binding_tdd.rs`
  (kept); `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new)
  + `crates/slicer-runtime/tests/executor/main.rs` mod-line registration.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (one sampling-side SUMMARY; delegate).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- ADR-0029 module-sampler contract: NO generation, NO clipping, NO chaining in the
  module — sample and emit raw; the linker (133) clips and connects.
- **Generation-sampling boundary (revised):** 140 is "the lightning packet"
  and owns both sides of the per-layer seam. The generation side
  (`crates/slicer-core/src/algos/lightning/`) gets the full grounding
  search (Step 0) so the sampling side samples higher-quality trees.
  Cross-layer `Generator`, `DistanceField`, and the producer stay frozen
  for this packet — only `Layer::generate_new_trees` (and the
  `getBestGroundingLocation` helper it delegates to) and the
  `Node`-level surface used by the grounding search are in scope.
  Any change to other 138/139 surface is a recorded deviation.
- Raw-emit uniformity (ADR-0025 + Amendment): this packet removes the roadmap's last
  self-linking exception; nothing may reintroduce path connection here.
- WIT version-bump semantics: `slicer:world-layer@2.2.0` → `@2.3.0` follows the
  2.0.0→2.1.0→2.2.0 chain and is consistent with packet 130's DEV-084 precedent
  (a minor→major correction for an additive export-arg change). The version bump
  is purely advisory under ADR-0044 — no test mechanically detects a missed
  version — so the packet must rely on the doc-update checklist and the explicit
  `wit_drift_detection_tdd` string assertion, not the package version itself.
- The `cargo xtask test --workspace` ceremony only via `--summary` dispatch (CLAUDE.md).

## Code Change Surface

- Selected approach: thin sampler. The view returns the (object, region, layer)'s tree
  segments the 139 producer committed (per-region keying); the module maps each
  segment to a `Point3WithWidth` (z derived from the dispatching `layer_index`;
  width from `self.line_width`; mm conversion at the boundary from integer-unit
  IR segments to `f32` coordinates), tagged `SparseInfill` + the `speed_factor`
  from config, pushed under the correct region origin. The only judgment call —
  whether Orca's Filler applies a sampling-side transform worth mirroring — is
  settled by the single delegated SUMMARY before coding.
- Exact changes:
  - `modules/core-modules/lightning-infill/src/lib.rs` — net −300+ lines expected;
    body swap; deleted functions listed above.
  - `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` — rewritten
    around the sampler contract (AC-1 + AC-N2); the module-binding test
    (`slicer_module_binding_tdd.rs`) is kept verbatim.
  - `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new) +
    `crates/slicer-runtime/tests/executor/main.rs` mod-line — AC-3.
  - `docs/DEVIATION_LOG.md` (DEV-081 row → `Closed` AND
    `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` row → `Closed`) +
    `docs/07_implementation_status.md` (TASK-262…265 closure sweep, via dispatch)
    — closure artifacts.
- **DEVIATION-CLOSURE additions (D-137-WIT-RUN-INFILL-NO-PAINT-VIEW):**
  - WIT edit at `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:25`:
    the `run-infill` export's signature gains a `paint: paint-region-layer-view`
    argument; mirrors the `run-perimeters` and `run-support` signatures at
    `:23` and `:27`. The four other exports that already take a paint view
    are unchanged. The package version at `:1` bumps
    `2.2.0` → `2.3.0`.
  - SDK trait `LayerModule` at
    `crates/slicer-sdk/src/traits.rs:369-377` adds a
    `_paint: &PaintRegionLayerView` parameter to `fn run_infill(...)`. The
    four other trait methods are unchanged. The existing paint-view-bearing
    methods (`run_slice_postprocess`, `run_perimeters`, `run_support`) are
    the right-mirror-shape.
  - slicer-macros `infill_arm` at
    `crates/slicer-macros/src/lib.rs:1779-1794` and the macro-emitted
    `fn run_infill` glue at `:2804-2809` thread the new arg through to
    the module's impl. The macro already does this for the other
    paint-view-bearing stages (perimeters, support); the new arm is a
    parallel addition.
  - Host dispatch `Layer::Infill` arm at
    `crates/slicer-wasm-host/src/dispatch.rs:442-465` is rewritten to
    mirror the existing `Layer::Support` arm at `:584-619`: it builds a
    `PaintRegionLayerData` via the existing
    `build_paint_layer_data_with_plan(paint_ir, layer_index, support_plan_ir, lightning_tree_ir)`
    (the function at `:1335-1404` is already in place from 137), pushes it
    via `push_paint_region_layer_view(paint_data)`, and passes `own(paint)`
    as the 4th argument to `call_run_infill`. The `lightning_tree_ir` field
    is already in `LayerStageInput` (137).
  - Four `run_infill`-implementing core modules update their `fn run_infill`
    signatures: `rectilinear-infill/src/lib.rs:94`, `gyroid-infill/src/lib.rs:138`,
    `lightning-infill/src/lib.rs:97`, `top-surface-ironing/src/lib.rs:305`. The
    first three (rectilinear, gyroid, top-surface-ironing) bind `_paint` and
    ignore it (their algorithm does not read lightning trees); only
    `lightning-infill` actually calls
    `paint.lightning_tree_segments_for(object_id, region_id)` (the existing
    SDK accessor, upgraded by 139 to per-region keying).
  - Test-guest extension at
    `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs:113`:
    the existing `fn run_infill(layer_index, regions, output, config)` gains
    a 5th parameter `_paint: PaintRegionLayerView`; the guest's per-region
    loop calls `paint.lightning_tree_segments(object_id, region_id)` and
    emits a witness path encoding the segment count
    (`width == 137.0, x == count_as_f32`).
  - Re-baseline `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs:592-616`:
    the existing `run-infill-postprocess` signature string (at `:608-612`)
    is preserved (no change to that export); a NEW drift assertion is added
    that pins `run-infill`'s new signature and the `world-layer@2.3.0`
    package version.
  - 33 guest artifacts re-stale (21 core-module + 12 test-guests); one
    `cargo xtask build-guests` rebuild is required.
- **DEVIATION-CLOSURE additions (D-139-LAYER-GROUNDING-SEARCH-STUB) — Step 0:**
  - Port the full `getBestGroundingLocation` (Orca
    `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp::getBestGroundingLocation`)
    into `crates/slicer-core/src/algos/lightning/layer.rs`. The Rust port
    must include:
    - The grid scan over the outline locator (sequential `for` loop
      with `cancel: &dyn Fn()` per 139's convention — TBB/rayon is out
      of scope for this port; record the parallelization as a future
      deviation if needed).
    - The tree-node locator: a `HashMap<(i32, i32), Vec<NodeRef>>` keyed
      by `to_grid_point(location, bbox, locator_cell_size)` (locator
      cell size = 4 in 100nm units, ported from Orca 400 nm).
    - The `wall_supporting_radius` exclusion: outline-candidate points
      within `wall_supporting_radius - tree_connecting_ignore_offset`
      (1 in 100nm units, ported from Orca 100 nm) of a wall are
      skipped.
    - The `getWeightedDistance` ranking (already ported in 139 Step 2;
      reused as-is).
    - Attribution header from `docs/ORCASLICER_ATTRIBUTION.md`. Cite
      by file + function name (NOT by line number) per
      `CLAUDE.md` §OrcaSlicer Citation Style.
  - Remove the 139 Step-2 stub comment from
    `crates/slicer-core/src/algos/lightning/layer.rs:62` (the
    `// 139 deviation: ...` line) once the full search is in place.
  - Co-update the 139 test home
    (`crates/slicer-core/tests/algo_lightning_tdd.rs`) with the new
    `lightning_layer_wall_supporting_radius` test (AC-G1) AND a
    re-assertion of the existing `lightning_generator_tree_continuity`
    test (AC-G2 — must still pass; the grounding refinement must not
    regress continuity).
  - Coordinate system: 1 unit = 100 nm. Divide all OrcaSlicer nm
    constants by 100. Use `slicer_ir::units_to_mm` /
    `Point2::from_mm` / `mm_to_units()` per `docs/08_coordinate_system.md`.
  - `docs/DEVIATION_LOG.md` — `D-139-LAYER-GROUNDING-SEARCH-STUB` row
    → `Closed` (this step).
- Rejected alternatives: (a) keeping the stub as a fallback when trees are empty —
  rejected: empty trees mean nothing to print (AC-N2); a silent stub fallback would
  mask producer bugs; (b) emitting per-tree connected polylines (walking the tree)
  instead of raw segments — rejected: that is self-linking by another name; the
  linker chains; (c) module-side clipping to the region polygon — rejected: linker
  re-clips (ADR-0025); (d) second export `run-infill-with-paint` — rejected
  (permanent desync from perimeters/support pattern); (e) `option<paint>` arg —
  rejected (no WIT tree precedent for `option<>`-as-top-level-arg); (f)
  import-function fetch on demand — rejected (zero surface win, same paint-build
  cost); (g) promoting `lightning-tree-segments` to a top-level resource —
  rejected (breaks symmetry, strictly larger surface).

## Files in Scope (read + edit)

- `modules/core-modules/lightning-infill/src/lib.rs` — role: the rewrite; expected
  change: stub deleted, sampler in.
- `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` — role: TDD;
  expected change: rewritten around the sampler contract.
- `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (new) +
  `crates/slicer-runtime/tests/executor/main.rs` (mod-line registration) — role:
  AC-3.
- `docs/DEVIATION_LOG.md` (DEV-081 row → `Closed` AND
  `D-137-WIT-RUN-INFILL-NO-PAINT-VIEW` row → `Closed`) +
  `docs/07_implementation_status.md` (closure sweep, via dispatch) — role:
  closure artifacts.
- **DEVIATION-CLOSURE additions (D-137-WIT-RUN-INFILL-NO-PAINT-VIEW):**
  - `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` — role: WIT
    signature extension + version bump (one file, two edits).
  - `crates/slicer-sdk/src/traits.rs:369-377` — role: SDK trait extension
    (one method signature).
  - `crates/slicer-macros/src/lib.rs:1779-1794` (the `infill_arm` block) +
    `:2804-2809` (the macro-emitted `fn run_infill` glue) — role: macro
    glue extension.
  - `crates/slicer-wasm-host/src/dispatch.rs:442-465` — role: host dispatch
    arm rewrite (mirror the `Layer::Support` arm at `:584-619`).
  - Four `run_infill`-implementing core modules — role: signature update
    (lightning also uses the paint view):
    `modules/core-modules/rectilinear-infill/src/lib.rs:94`,
    `modules/core-modules/gyroid-infill/src/lib.rs:138`,
    `modules/core-modules/lightning-infill/src/lib.rs:97` (the rewrite
    target — also covers AC-1/AC-N2),
    `modules/core-modules/top-surface-ironing/src/lib.rs:305`.
  - `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs:113` —
    role: test-guest extension (the AC-3b witness).
  - `crates/slicer-wasm-host/tests/contract/lightning_infill_guest_calls_lightning_tree_segments_tdd.rs`
    (new) — role: AC-3b host-side test driver. Instantiates the rebuilt
    `layer-infill-guest.component.wasm` guest against a fixture print
    configured for lightning, drives a `Layer::Infill` dispatch, and
    asserts the guest's emitted witness path encodes the
    `lightning-tree-segments` count for the dispatched region. Registers
    in `crates/slicer-wasm-host/tests/contract/main.rs` (the WIT-contract
    aggregator, currently 12 modules). Without this file, AC-3b's pipe
    command has no driver and is unexercised.
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs:592-616` —
    role: drift re-baseline (new assertions for the run-infill signature and
    the `world-layer@2.3.0` version).
  - `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs` —
    role: re-baseline (the existing 6 `call_run_infill` call sites at lines
    94, 192, 268, 352, 433, 497 will need updating to add the paint arg
    after 140 lands; this file is in scope for re-baseline, the same
    shape as the existing infill_holder_resolution_painted_region_tdd.rs
    re-baseline done in packet 137).
- `docs/03_wit_and_manifest.md` §`world-layer.wit` — role: version + signature
  doc update.
- **Step 0 (D-139-LAYER-GROUNDING-SEARCH-STUB closure):**
  - `crates/slicer-core/src/algos/lightning/layer.rs` — role: Step 0
    port of `getBestGroundingLocation`; removes the 139 Step-2 stub
    comment.
  - `crates/slicer-core/src/algos/lightning/tree_node.rs` — role:
    Step 0 may add a small `to_grid_point(...)` helper (or extend the
    existing 138 surface) used by the tree-node locator; co-updated
    138 tests if so.
  - `crates/slicer-core/tests/algo_lightning_tdd.rs` — role: Step 0
    new test `lightning_layer_wall_supporting_radius` (AC-G1) plus
    continuity re-assertion (AC-G2).
  - `docs/DEVIATION_LOG.md` — `D-139-LAYER-GROUNDING-SEARCH-STUB` row
    status → `Closed` (this step).

## Read-Only Context

- `crates/slicer-sdk/src/traits.rs` — the 137 `lightning_tree_segments_for` accessor
  (ranged; the view-pattern anchor). The 139 refinement upgrades this accessor to
  per-region keying — the shape is the same, the filter is stricter.
- One 134/135-era module test file — the raw-emit test idiom (ranged).
- `crates/slicer-wasm-host/src/dispatch.rs:529-559` (the `Layer::Perimeters` arm)
  and `:584-619` (the `Layer::Support` arm) — the right-mirror-shape for the
  `Layer::Infill` arm at `:442-465`. Both existing arms build a
  `PaintRegionLayerData` via `build_paint_layer_data_with_plan(...)` and push
  via `push_paint_region_layer_view(...)`; the `Layer::Infill` arm mirrors
  exactly.
- `crates/slicer-macros/src/lib.rs:1779-1794` and `:2804-2809` — the existing
  `infill_arm` block + the macro-emitted glue. The new paint-view arg
  parallels the existing perimeters/support macro arms.
- `crates/slicer-sdk/src/lib.rs:39` — the `PaintRegionLayerView` re-export
  (`pub use traits::..., PaintRegionLayerView, ...`). The module's import path
  for the new arg is `use slicer_sdk::traits::PaintRegionLayerView;` (the
  precedent line for the cite-precise path in the implementation plan is the
  `SliceRegionView` import at `lib.rs:38`, which uses `slicer_sdk::views::`,
  not `slicer_sdk::traits::` — the re-export at the root makes both work, but
  the trait path is the right home for the new arg).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — one delegated SUMMARY only; never load.
- `modules/core-modules/infill-linker/**` — triage boundary (requirements §Out of
  Scope).
- `modules/core-modules/support-surface-ironing/**` — implements only
  `run_infill_postprocess`, not `run_infill`; the WIT signature change does
  not reach it.
- `target/`, `Cargo.lock` — never load.
- **Note (revised):** the prior design listed
  `crates/slicer-core/src/algos/lightning/**` here as "138/139's closed
  surface, defects routed, not patched here." That bullet is removed
  per the boundary flip: 140 is the lightning packet. The
  per-file in-scope list above names `layer.rs` and `tree_node.rs`
  explicitly (Step 0 only). Other 138/139 surface (cross-layer
  `Generator`, `DistanceField`, the producer) is still frozen for
  this packet — if 140 needs a change to those, it's a recorded
  deviation, not an in-scope edit.

## Expected Sub-Agent Dispatches

- "SUMMARY ≤ 200 words: `FillLightning.cpp` `Filler::_fill_surface_single` — what
  happens between `getTreesForLayer` and output; is any transform applied the PnP
  sampler must mirror?" — the one Orca question.
- "Run `cargo test -p lightning-infill …`; FACT + counts; SNIPPETS ≤ 20 on failure".
- "Run `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked …`;
  FACT".
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".
- "Run `cargo xtask test --workspace --summary`; verdict block ONLY" — roadmap-close
  ceremony.
- "Flip DEV-081 to Closed + docs/07 TASK-262…265 closure notes; FACT + the two Doc
  Impact greps".

## Data and Contract Notes

- IR/WIT: the view is unchanged from 139's per-region keying; this packet's
  WIT-extension is a transport change (the `run-infill` signature now takes
  a paint view that carries the view methods including
  `lightning-tree-segments`). The IR is unchanged from 139. If the view proves
  insufficient (e.g. missing per-tree grouping the sampler needs), that is a
  137/139-contract deviation — minor bump routed through a recorded deviation,
  not an inline hack.
- Emission: mm at `ExtrusionPath3D` (`f32` `points: Vec<Point3WithWidth>`) from
  integer-unit IR segments via `slicer_ir::units_to_mm(...)` (the one mm↔unit boundary
  in the packet). z derived from the dispatching `layer_index` and the layer Z table.
- Determinism: emission order = IR segment order (frozen by 139).
- WIT version: 2.2.0 → 2.3.0. The bump is purely advisory under ADR-0044;
  re-baseline `wit_drift_detection_tdd` for the new package version AND the new
  `run-infill` signature, do not rely on the package version itself.

## Locked Assumptions and Invariants

- Manifest stays `holds = ["claim:sparse-fill"]`.
- No generation/clipping/chaining in the module — sampler only.
- Empty trees → empty emission, slice completes (AC-N2) — no stub fallback exists.
- Non-lightning output byte-identical (AC-N1).
- DEV-081 closes here or the packet does not close.

## Risks and Tradeoffs

- The old test suite encodes stub semantics wholesale — rewriting it risks losing
  genuine invariants (module binding, role tagging, origin discipline); those
  specific tests are kept/adapted, and each deletion names the stub behavior it
  encoded.
- Linked-lightning visual quality is new territory (no OrcaSlicer golden to compare
  linked output against, since Orca links differently) — the bless justification
  leans on AC-1 (sampling fidelity) + AC-3 (pipeline integrity) + the HTML-report
  visual note.
- The roadmap-close ceremony may surface cross-packet debt; triage per the fence,
  record honestly (the packet-126 lesson: never flip closure before the ceremony).

## Context Cost Estimate

- Aggregate: `L` (justified unsplittable — generation + sampling + WIT closure
  are tightly coupled at the per-layer seam; the swarm runs in extended band
  per the escalation protocol).
- Largest single step: `L` (Step 0 — the WIT + trait + macro + dispatch + four-
  module + test-guest + drift-re-baseline bundle is one atomic coupled change
  set; partial state breaks every infill guest's instantiation at runtime).
  Step 1 (grounding search port) is `M`. Step 2 (the rewrite) is `M`.
- Highest-risk dispatch: the workspace ceremony — summary-only contract.
- Step 0's L rating: the WIT signature change forces the SDK trait extension,
  which forces the macro glue update, which forces the four `run_infill`-
  implementing core modules' signature update, which forces the test-guest
  extension, which forces the drift-re-baseline. Splitting any of these
  sub-steps leaves the workspace un-compiling (the macro-generated glue
  must match the WIT signature; the four modules must match the trait;
  the test-guest must match the dispatch). One atomic step.
- Step 1 (grounding search) is a focused ~150-line port of
  `getBestGroundingLocation` + the tree-node locator + the
  `wall_supporting_radius` exclusion. M-rated, well-scoped, AC-G1 +
  AC-G2 are direct assertions.

## Open Questions

- `[FWD]` Whether Orca's Filler applies a sampling-side transform to mirror —
  settled by the single delegated SUMMARY before Step 2 codes the sampler.
- `[FWD]` Whether the live DEV-081 row already has a `Closed` status in the log's
  convention or whether the packet must change the row to `Closed` (FACT at Step 4
  start; the AC-4 grep tolerates both shapes).
- `[FWD-resolved]` WIT extension shape: per the blast-radius investigation
  in `docs/specs/137-deviations-plan.md`, the canonical D-137 fix
  (extend `run-infill` + bump to 2.3.0) is selected. The four rejected
  alternatives are recorded in `requirements.md` §Acceptance Summary.
