---
status: implemented
packet: 64_paint-native-migration
task_ids:
  - TASK-204
  - TASK-136
---

# 64_paint-native-migration

## Goal

Eliminate the `paint-segmentation` and `paint-region-annotator` WASM modules, consolidate both into the already-existing host-native implementations, add a dedicated `Layer::PaintRegionAnnotation` pipeline stage before `SlicePostProcess`, apply per-point parallelism to the annotation loop, and provide a config toggle to re-evaluate the union-at-harvest tradeoff.

## Problem Statement

The Pinch 'n Print pipeline runs two WASM modules — `paint-segmentation` (PrePass) and `paint-region-annotator` (per-layer `SlicePostProcess`) — that duplicate host-native implementations already present in `paint_segmentation.rs` and `slice_postprocess.rs`. The guest `paint-region-annotator` consumes 1,370,992 CPU-ms across threads on a benchy_4color run, performing point-in-region containment checks that the host's `execute_slice_postprocess_paint_annotation` already computes natively. The guest `paint-segmentation` projects 3D facets to 2D via WIT serialization, while `execute_paint_segmentation()` contains a complete independent implementation never wired into the dispatch path.

Beyond duplication, the current architecture has a design defect: `Layer::SlicePostProcess` conflates general post-processing with paint-specific annotation. A WASM module claiming `SlicePostProcess` for a different purpose (e.g., polygon smoothing) has no interaction with paint annotation, but the stage name and fallback guard (`paint_annotation_ran`) make the relationship implicit and fragile.

The WASM boundary imposes serialization cost even after migration: `paint_region_ir_to_layer_data()` re-serializes `PaintRegionIR` (~60 KB per layer) for `tree-support` and `traditional-support` modules that query `PaintRegionLayerView`. Eliminating the two guest modules removes the dominant CPU cost (1.37M CPU-ms) while the support-module serialization path survives independently.

Packet 62 optimized the host annotation path (union, AABB, cache, early-break, `par_iter`) but these optimizations only apply when the host fallback runs — not when the WASM module is loaded. Packet 63 will add R-tree spatial indexing to the query path. Making the host path always-on ensures both packets' optimizations are always active.

This packet completes the consolidation: delete both WASM modules, wire the host implementations as guard-based fallbacks, add a dedicated `Layer::PaintRegionAnnotation` stage, apply per-point parallelism, and provide a config toggle to re-evaluate the union-at-harvest tradeoff.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `Layer` stage enum lives in `crates/slicer-ir/src/slice_ir.rs` (or equivalent executor enum). Adding `Layer::PaintRegionAnnotation` requires updating all match arms across the workspace. The implementer must delegate a `cargo check` after adding the variant to discover all affected match arms.

- `execute_paint_segmentation()` currently uses `push_polygon_region()` which appends per-facet polygons without unioning and sets `aabb: None`. The shared `group_and_union_paint_regions()` replaces `push_polygon_region()` — it groups by `(layer_index, object_id, semantic, value)`, unions polygons via `slicer_core::union()`, computes AABB from unioned contour points, and sorts descending by `paint_order`. This must produce byte-identical output to the current `harvest_paint_segmentation_ir()` post-processing.

- `execute_paint_segmentation()` validates `MissingSurfaceObject` and `MissingLayerParticipation` (errors not present in the WASM guest). These validations are **kept** — they are correctness improvements that fail fast rather than producing silently wrong output. The WASM guest would fail later with a worse error. These errors must be mapped to the dispatch error type in the guard-based fallback.

- `execute_paint_segmentation()` detects `DeterministicConflict` at segmentation time via `detect_custom_conflict()` with polygon-overlap checks. The WASM guest does not — conflicts surface at query time in `point_in_paint_region`. The conflict detection is **kept** — it is a correctness improvement (fail-fast at segmentation rather than failing per-layer at query time). This is a behavioral change: overlapping custom regions with equal `paint_order` now fail during the prepass rather than during per-layer annotation. The `point_in_paint_region` conflict path is still preserved as a defense-in-depth check.

- `execute_slice_postprocess_paint_annotation()` has richer behavior than the WASM `paint-region-annotator` guest: edge-ambiguity detection (code 504 warnings), default value filling for out-of-region points, and FuzzySkin modifier support. The richer behavior is **kept** — it is strictly better (fewer `None` values, better diagnostics). The WASM guest's minimal behavior (leave `None`, no warnings) was a gap, not a designed contract.

- The `union_paint_regions_at_harvest` config key is a `bool` defaulting to `true`. When `false`, `group_and_union_paint_regions()` skips `slicer_core::union()` but still computes AABB. The key is scoped to the paint-segmentation config schema, not a global config. It is documented as a temporary benchmarking toggle; the user may remove it after data confirms the right default.

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR` output shape unchanged. `SemanticRegion` retains its fields (`object_id`, `polygons`, `value`, `paint_order`, `aabb`). No schema version bump.
- WIT boundary considerations: `slicer:world-prepass@1.0.0` still defines `run-paint-segmentation`. The host no longer dispatches it, but the WIT contract stays for future extension. `PaintRegionLayerView` serialization (`paint_region_ir_to_layer_data()`) survives for support modules.
- Determinism: `group_and_union_paint_regions()` sorts groups by `(paint_order, object_id, value_key)` within each semantic Vec — identical to the current harvest sort order. Byte-deterministic output across runs is preserved (tested by AC-N3 in packet 62).
- Scheduler: new `Layer::PaintRegionAnnotation` stage inserted between `Layer::Slice` and `Layer::SlicePostProcess`. No DAG edge changes — it's a per-layer sequential stage. `PrePass::PaintSegmentation` order unchanged.
- Manifest: each deleted module's `.toml` manifest is deleted with the directory. Discovery via `discover_manifest_paths()` naturally skips them. No hardcoded module paths exist.
- Config: new `union_paint_regions_at_harvest` key added to paint segmentation config schema. Default `true`. No other config changes.

## Locked Assumptions and Invariants

- `slicer_core::union` discards holes. All guest-produced paint region entries carry `holes: vec![]` (triangles only). The shared function documents this at the call site. If a future module emits hole-bearing paint regions through the WASM override path, the host fallback's union path must switch to a hole-preserving variant.
- `paint_order` values from the shared function are `min(paint_order)` per group — identical to the current harvest behavior. Precedence (higher `paint_order` wins) is preserved.
- The AABB pre-filter in `semantic_region_contains_point` is an optional optimization — the shared function always computes it (even when `union_paint_regions_at_harvest: false`). Setting `aabb = None` at construction time would change query-path performance but not correctness.
- `rayon` is already a `slicer-host` dependency — no new `Cargo.toml` entry. `par_chunks(32)` uses the existing `use rayon::prelude::*` import.
- `PaintRegionIR` is `Arc`-wrapped and read-only — thread-safe for `par_chunks` parallel point queries.
- Group key `(layer_index, object_id, semantic, value)` is correct: same-value regions are query-equivalent and safe to merge; `object_id` preserves per-object boundaries; `paint_order` conflict logic only triggers between regions of different values.
- WIT `PaintRegionLayerView` serialization stays because `tree-support` and `traditional-support` query it per layer. Removing this path is a separate work item (these modules could be refactored to use `PaintRegionIR` directly).
- Test-guests `test-guests/prepass-guest/` and `test-guests/sdk-prepass-paintseg-guest/` stay unchanged — they validate the WIT contract, not the production module.

## Risks and Tradeoffs

- **Test churn**: 20 test files touched (2 migrated, 5 rewritten, 13 read-only verification). This is the largest single source of work in the packet. Each rewrite must preserve the original test's assertion strength.
- **WASM extension surface**: The guard-based fallback preserves the ability for future WASM modules to override both stages. If no module ever does, the guard is dead code — but it costs one `if wasm_ran { skip }` check per stage execution.
- **`execute_paint_segmentation()` validation errors**: `MissingSurfaceObject` and `MissingLayerParticipation` are new failure modes that didn't exist in the WASM guest path. If the upstream stages (`MeshAnalysis`, `LayerPlanning`) have bugs that the WASM guest silently tolerated, these errors could surface as pipeline failures after migration. Mitigated by the fact that the host path already validates these in tests (`paint_segmentation_executor_tdd.rs`).
- **Conflict detection at segmentation time vs query time**: `DetectCustomConflict` now fires during the prepass rather than during per-layer annotation. This is a behavioral change: the error surfaces earlier and is a prepass-level fatal, not a per-layer error. Downstream error-handling code that expected conflicts at query time may need updating. The `point_in_paint_region` conflict check is preserved as defense-in-depth.
- **Per-point parallelism determinism**: `par_chunks(32)` processes points non-deterministically. The `boundary_paint` output must be order-independent — each point's result depends only on its coordinates, not on other points' results. Verified by the existing `slice_postprocess_paint_annotation_tdd` tests.
- **Per-point parallelism overhead**: Rayon's per-task overhead for 32 containment checks (~32 × AABB check + 0-1 polygon containment) is higher than for 64. But 1,000-2,000 points / 32 = 32-64 tasks per layer, providing 2-4 tasks per thread on 16 cores — enough for good utilization. If profiling shows per-task overhead dominating, increase to `par_chunks(64)`.
- **Union toggle**: `union_paint_regions_at_harvest: false` produces un-unioned regions (many small polygons per SemanticRegion). This regresses query-path performance (more polygons to iterate, even with AABB pre-filter). The toggle is for benchmarking only — not recommended for production. Document as such.
