---
status: implemented
packet: 107_overhang-pipeline-consumers-and-refactor
task_ids:
  - O-T030
  - O-T031
  - O-T032
  - O-T040
  - O-T041
  - O-T042
  - O-T050
  - O-T051
  - O-T052
  - O-T053
---

# 107_overhang-pipeline-consumers-and-refactor

## Goal

Land the consumer-side half of the overhang pipeline restructuring: add `SliceRegionView::overhang_quartile_polygons()` (and confirm `overhang_areas()` from P104 now returns non-empty data), refactor `overhang-classifier-default` from a wall-distance computer to a pure-consumer that reads per-vertex `overhang_quartile` from `LayerCollectionView` entities and applies speed factors only, register an end-to-end overhang-quartile propagation TDD, and close perimeter-roadmap deviations D-10 / D-12 / D-OVERHANG-QUARTILE-NONE while unblocking T-024 and T-077.

## Problem Statement

P106 (draft) will land the PrePass-side foundation of the overhang pipeline restructuring — `OverhangRegion.xy_footprint` field already exists in `slice_ir.rs:581` (P106 populates it at runtime), `SurfaceClassificationIR.overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` and `QuartileBand` are P106 FORWARD-DEPs not yet in the tree. Without this consumer-side packet, that data is stranded on the Blackboard: no view accessor exposes the quartile polygons to Tier 2 modules; `overhang-classifier-default` still runs its old per-entity wall-distance computation (the algorithm ADR-0022 — authored by P106 at the next free ADR slot, 0022 — supersedes for classification purposes); and the perimeter-roadmap decisions D-10 / D-12 (in the roadmap) plus the to-be-registered deviation `D-104-OVERHANG-QUARTILE-NONE` stay open. T-077 (`extra_perimeters_on_overhangs`) in P108 cannot transition from its previously-planned no-op pattern to a real consumer until this packet ships the view accessor on top of P106's data.

This packet closes all four concerns. The view accessor + WIT mirror + host populator are mechanical extensions of the patterns established by `bridge_areas()`. The `overhang-classifier-default` refactor shrinks the module from ~100 LOC + 2 helper files to ~50 LOC of pure consumer logic that reads `Point3WithWidth.overhang_quartile` and emits `EntityMutation::SetSpeedFactor`. The end-to-end TDD validates the full data path from mesh through gcode. The regression check confirms the refactor preserves observable behaviour within calibrated tolerances. The closure pass turns three open deviations into resolved/superseded entries and unblocks two perimeter-roadmap tasks.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- ADR-0008 invariant preserved: speed-factor application stays at `PostPass::LayerFinalization`. This packet's refactor narrows what the module does (consumer only) but keeps it in the finalization tier per the original ADR.
- ADR-0022 invariant (FORWARD-DEP on draft P106 — ADR-0022 is the correct slot; 0012 is taken by `0012-spatial-indexing-as-reconstruction-only-companions.md`): classification reads come from `SurfaceClassificationIR.overhang_quartile_polygons` (P106's output); the module never recomputes from wall geometry.
- View pre-filtering pattern: the host pre-filters per-region quartile bands at view-construction (cheap point-in-polygon prefilter); the guest receives only data relevant to the current region. Mirrors `bridge_areas()` pre-filter pattern.
- Schema-version contract: no IR bump in this packet (the IR was bumped in P106). WIT mirror is additive.
- Module-shrink invariant: the post-refactor `overhang-classifier-default/src/lib.rs` MUST be a single file ≤ 80 LOC with no other source files in the directory (per AC-3).

## Data and Contract Notes

- IR or manifest contracts touched: no IR change (P106 did it). Manifest narrowing: `overhang-classifier-default.toml` pre-refactor has `reads = ["LayerCollectionIR"]` (confirmed in tree). Post-refactor drops this broad entry and declares a narrower `overhang_quartile`-annotated read on per-vertex `Point3WithWidth`. The `writes = ["LayerCollectionIR"]` entry stays (needed for `SetSpeedFactor` mutations).
- FORWARD-DEP symbols consumed from upstream drafts:
  - `SurfaceClassificationIR.overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` ← produced by draft P106 (`status= draft`; not yet in tree)
  - `QuartileBand { quartile: u8, polygons: Vec<ExPolygon> }` ← produced by draft P106.
  - `SliceRegionView::overhang_areas(&self) -> &[ExPolygon]` ← produced by draft P104 (`status= draft`; not yet in `crates/slicer-sdk/src/views.rs`).
  - `SliceRegionView::surface_group(&self) -> Option<&SurfaceGroup>` ← produced by draft P104.
  - `docs/adr/0022-overhang-classification-at-prepass.md` ← authored by draft P106 at ADR slot 0022.
- Already-in-tree symbols (no forward-dep needed):
  - `OverhangRegion.xy_footprint: Vec<ExPolygon>` — present at `crates/slicer-ir/src/slice_ir.rs:581` (P106 populates it at runtime, but the field definition is already there).
  - `Point3WithWidth.overhang_quartile: Option<u8>` — present at `crates/slicer-ir/src/slice_ir.rs:1516`.
  - `LayerCollectionIR.ordered_entities: Vec<PrintEntity>` — present at `crates/slicer-ir/src/slice_ir.rs:1946`.
  - `SurfaceClassificationIR` struct itself — present at `crates/slicer-ir/src/slice_ir.rs:612` (without the quartile-polygons field yet).
- WIT boundary considerations: new `slice-region-view::overhang-quartile-polygons` accessor. Additive — backward-compatible.
- Determinism or scheduler constraints: the refactored `overhang-classifier-default` is deterministic over its inputs (per-vertex quartile + config). No scheduler change.
- View pre-filtering: per-region quartile polygons = intersection of `SurfaceClassificationIR.overhang_quartile_polygons[layer_index]` with the region's polygon, computed at view-construction (Tier 2 view-builder). The full HashMap stays on the Blackboard; only the per-region projection crosses the guest boundary.

## Locked Assumptions and Invariants

- `Point3WithWidth.overhang_quartile: Option<u8>` is the per-vertex source of truth for downstream consumers (confirmed in tree at `crates/slicer-ir/src/slice_ir.rs:1516`). P104's perimeter modules will write this field once P104 ships; this packet does not change that contract.
- `SliceRegionView::overhang_areas()` and `SliceRegionView::surface_group()` are FORWARD-DEPs on draft P104 — neither exists in `crates/slicer-sdk/src/views.rs` yet. This packet adds `overhang_areas()` as part of its own scope (O-T030 confirms the stub; if P104 has not yet added it, this packet adds it).
- `SliceRegionView::overhang_quartile_polygons()` returns polygons (not per-vertex values); it's a different consumer surface used by T-077 (P108) and future overhang-aware modules.
- `overhang-classifier-default/src/lib.rs` is the only source file in the module directory post-refactor. Auxiliary files are forbidden.
- D-10, D-12, D-OVERHANG-QUARTILE-NONE all close in this packet. If a closure cannot land (e.g., P104's `None` shipping path isn't rewired and AC-5 documents the gap), the deviation transitions to "partially closed — perimeter-side wiring tracked as T-024-WIRE-VIEW-CONSUMER follow-up" rather than fully closed.
- Tolerance for AC-6 regression check: speed factors may differ in the 3rd–6th decimal due to the algorithm change (per-XY-distance vs per-entity-worst-case). Gross behavioural shifts (wall fully losing overhang treatment) are failures.

## Risks and Tradeoffs

- AC-5 depends on P104's `None` shipping path being rewired. If it's not, AC-5 documents the gap and registers a follow-up rather than failing the packet. This is a deliberate scope decision — wiring P104 here would creep this packet into the perimeter modules.
- Regression check (AC-6) requires recording pre-refactor reference G-code SHAs. If those aren't already recorded, Step 4 records them BEFORE the refactor lands (using the pre-refactor module). This adds a sub-step but is mandatory.
- `LayerCollectionView::ordered_entities` iteration: the refactored module walks every wall entity to read `overhang_quartile`. If the entity count is high (large prints), this is O(N) per finalization run — acceptable per existing module's behaviour, which was already O(N) × distance-computation.
- Deletion of `classify.rs` + `lines_distancer.rs`: irreversible without consulting git history. Confirm via Step 2's pre-deletion grep that no other module imports from these files (overhang-classifier-default is the sole owner).
