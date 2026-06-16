# Design: 107_overhang-pipeline-consumers-and-refactor

## Controlling Code Paths

- Primary code path: `SliceRegionView` gains an `overhang_quartile_polygons()` accessor (host pre-filters per-region by intersecting `SurfaceClassificationIR.overhang_quartile_polygons[layer_index]` with the region's polygon). `overhang-classifier-default::run_finalization` is rewritten to read `Point3WithWidth.overhang_quartile` per-vertex from `LayerCollectionView` entities and emit `EntityMutation::SetSpeedFactor` based on the read quartile + `overhang_X_4_speed` config. `classify.rs` and `lines_distancer.rs` are deleted (their wall-distance algorithm is superseded by P106's classifier). End-to-end TDD validates the full path; regression TDD captures pre-vs-post behavioural delta.
- Neighboring tests / fixtures: 3 new TDD files. Existing P106 tests (mesh_analysis_overhang_xy_footprint, overhang_annotation_ramp, prepass_overhang_annotation_stage_order) stay green.
- OrcaSlicer comparison surface: none new (workspace-internal consumer side).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- ADR-0008 invariant preserved: speed-factor application stays at `PostPass::LayerFinalization`. This packet's refactor narrows what the module does (consumer only) but keeps it in the finalization tier per the original ADR.
- ADR-0012 invariant: classification reads come from `SurfaceClassificationIR.overhang_quartile_polygons` (P106's output); the module never recomputes from wall geometry.
- View pre-filtering pattern: the host pre-filters per-region quartile bands at view-construction (cheap point-in-polygon prefilter); the guest receives only data relevant to the current region. Mirrors `bridge_areas()` pre-filter pattern.
- Schema-version contract: no IR bump in this packet (the IR was bumped in P106). WIT mirror is additive.
- Module-shrink invariant: the post-refactor `overhang-classifier-default/src/lib.rs` MUST be a single file ≤ 80 LOC with no other source files in the directory (per AC-3).

## Code Change Surface

- Selected approach: the view accessor lands first; the module refactor consumes it. The refactor's reading path is per-vertex `overhang_quartile` from `LayerCollectionView` entities, not from the view accessor — because the classifier still needs per-vertex info, and P104's perimeter modules write to `Point3WithWidth.overhang_quartile`. The view accessor exists for OTHER consumers (T-077 in P108, future fuzzy-skin variants). End-to-end TDD validates both data paths; regression TDD validates faithfulness.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-sdk/src/views.rs` — `overhang_quartile_polygons()` accessor + struct field.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — WIT mirror.
  - `crates/slicer-wasm-host/src/host.rs` — `SliceRegionData` field + populator (intersect quartile polygons with region polygon).
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` — refactor to consumer-only.
  - `modules/core-modules/overhang-classifier-default/src/classify.rs` — DELETE.
  - `modules/core-modules/overhang-classifier-default/src/lines_distancer.rs` — DELETE.
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` — manifest narrowing.
  - 3 new TDD files.
  - `docs/05_module_sdk.md`, `docs/01_system_architecture.md`, `docs/DEVIATION_LOG.md`, `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- Rejected alternatives that were considered and why they were not chosen:
  - Keep `classify.rs` as deprecated dead code: rejected — leaves dead machinery; AC-3 demands deletion.
  - Compute per-vertex `overhang_quartile` in the view accessor (point-in-quartile-polygon per vertex): rejected — that work belongs upstream (P104's perimeter modules write the per-vertex field directly from view polygons). The view accessor returns polygons; the module reads pre-written per-vertex values.
  - Skip the regression check (AC-6): rejected — speed-factor changes are observable in G-code; without a regression bed the refactor could silently degrade benchy quality.

## Files in Scope (read + edit)

- `crates/slicer-sdk/src/views.rs` — new accessor.
- `modules/core-modules/overhang-classifier-default/src/lib.rs` — refactor.
- `crates/slicer-wasm-host/src/host.rs` — populator.
- `crates/slicer-schema/wit/deps/ir-types.wit` — WIT.
- `modules/core-modules/overhang-classifier-default/{classify,lines_distancer}.rs` — DELETE.
- `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` — manifest.
- 3 new TDD files.
- 4 docs per Doc Impact Statement.

## Read-Only Context

- `docs/adr/0008-overhang-as-finalization-module.md` — read full — purpose: confirm speed-factor stays at finalization.
- `docs/adr/0012-overhang-classification-at-prepass.md` — read full — purpose: confirm classification reads from IR.
- `docs/specs/overhang-pipeline-restructuring.md` — range-read Phase 3/4/5.
- `docs/05_module_sdk.md` — delegate SUMMARY for `SliceRegionView` accessor convention.
- `modules/core-modules/overhang-classifier-default/src/lib.rs` — read full (≤ 100 LOC pre-refactor).
- `modules/core-modules/overhang-classifier-default/src/classify.rs` — read once to confirm what's deleted; do not re-read.
- `crates/slicer-sdk/src/views.rs` — range-read existing accessor patterns.
- `crates/slicer-wasm-host/src/host.rs` — range-read populator pattern (e.g., bridge_areas populator).
- `CLAUDE.md` — §"Guest WASM Staleness" + §"WIT/Type Changes Checklist".

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate (not expected to be needed for this packet).
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Vendored deps — never load.
- `crates/slicer-core/src/algos/overhang_annotation.rs` (from P106) — out of scope; consume only.
- `crates/slicer-core/src/algos/mesh_cross_section.rs` (from P106) — out of scope.
- All perimeter module `lib.rs` files — out of scope (P104 + future follow-up).
- `crates/slicer-ir/src/slice_ir.rs` — out of scope (no IR change here; consume P106's additions).
- All other modules + crates not in §Files in Scope.

## Expected Sub-Agent Dispatches

- "Find the `bridge_areas` populator pattern in `crates/slicer-wasm-host/src/host.rs` (`sliced_region_to_data` or analogous); return SNIPPETS ≤ 30 lines showing the pre-filter logic." — Step 1.
- "FACT: confirm the `QuartileBand` type from P106 is `pub struct QuartileBand { quartile: u8, polygons: Vec<ExPolygon> }`; return field list." — Step 1.
- "Find the `LayerCollectionView::ordered_entities` accessor signature; return FACT (signature + 1-line doc)." — Step 2.
- "Run `cargo check --workspace --all-targets` after each step; return FACT + SNIPPETS ≤ 20 lines on fail."
- "Run `cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd`; FACT pass/fail per case."
- "Run `cargo test -p slicer-runtime --test integration overhang_classifier_refactor_regression_tdd`; FACT pass/fail with tolerance-deviation summary on fail."
- "Run `cargo xtask build-guests --check`; FACT (clean / STALE list)." — Step 1 closure gate.

## Data and Contract Notes

- IR or manifest contracts touched: no IR change (P106 did it). Manifest narrowing: `overhang-classifier-default.toml` drops broad `LayerCollectionIR` reads; declares narrow `overhang_quartile` read on per-vertex `Point3WithWidth`.
- WIT boundary considerations: new `slice-region-view::overhang-quartile-polygons` accessor. Additive — backward-compatible.
- Determinism or scheduler constraints: the refactored `overhang-classifier-default` is deterministic over its inputs (per-vertex quartile + config). No scheduler change.
- View pre-filtering: per-region quartile polygons = intersection of `SurfaceClassificationIR.overhang_quartile_polygons[layer_index]` with the region's polygon, computed at view-construction (Tier 2 view-builder). The full HashMap stays on the Blackboard; only the per-region projection crosses the guest boundary.

## Locked Assumptions and Invariants

- `Point3WithWidth.overhang_quartile` is the per-vertex source of truth for downstream consumers (overhang-classifier-default's refactor reads it). P104's perimeter modules write this field; this packet does not change that contract.
- `SliceRegionView::overhang_quartile_polygons()` returns polygons (not per-vertex values); it's a different consumer surface used by T-077 (P108) and future overhang-aware modules.
- `overhang-classifier-default/src/lib.rs` is the only source file in the module directory post-refactor. Auxiliary files are forbidden.
- D-10, D-12, D-OVERHANG-QUARTILE-NONE all close in this packet. If a closure cannot land (e.g., P104's `None` shipping path isn't rewired and AC-5 documents the gap), the deviation transitions to "partially closed — perimeter-side wiring tracked as T-024-WIRE-VIEW-CONSUMER follow-up" rather than fully closed.
- Tolerance for AC-6 regression check: speed factors may differ in the 3rd–6th decimal due to the algorithm change (per-XY-distance vs per-entity-worst-case). Gross behavioural shifts (wall fully losing overhang treatment) are failures.

## Risks and Tradeoffs

- AC-5 depends on P104's `None` shipping path being rewired. If it's not, AC-5 documents the gap and registers a follow-up rather than failing the packet. This is a deliberate scope decision — wiring P104 here would creep this packet into the perimeter modules.
- Regression check (AC-6) requires recording pre-refactor reference G-code SHAs. If those aren't already recorded, Step 4 records them BEFORE the refactor lands (using the pre-refactor module). This adds a sub-step but is mandatory.
- `LayerCollectionView::ordered_entities` iteration: the refactored module walks every wall entity to read `overhang_quartile`. If the entity count is high (large prints), this is O(N) per finalization run — acceptable per existing module's behaviour, which was already O(N) × distance-computation.
- Deletion of `classify.rs` + `lines_distancer.rs`: irreversible without consulting git history. Confirm via Step 2's pre-deletion grep that no other module imports from these files (overhang-classifier-default is the sole owner).

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (Step 2 — module refactor + deletion + new TDDs).
- Highest-risk dispatch: AC-6 regression comparison. Implementer should delegate the diff to a sub-agent that returns FACT.

## Open Questions

- `[FWD]` `PaintRegionLayerView` mirror (O-T032): default no mirror unless a consumer is named. If the implementer finds during Step 2 that the refactored module would benefit from the mirror, add it; otherwise skip.
- `[FWD]` Pre-refactor regression baseline: if no recorded benchy / standard fixture SHAs exist for `overhang-classifier-default`, record them in Step 4 BEFORE the refactor lands. Document in closure log.
- `[FWD]` AC-5 P104 wiring branch: if encountered, the implementer registers `T-024-WIRE-VIEW-CONSUMER` in the perimeter roadmap and notes in closure log. Do not attempt to land it here.
