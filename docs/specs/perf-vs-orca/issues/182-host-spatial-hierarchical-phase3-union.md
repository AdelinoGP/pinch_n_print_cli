# Host-side spatial hierarchical Phase-3 union with convex-containment pruning

Type: task / measured candidate (host-side algebraic union rewrite + standing `t137` 4-cell gate + human KEEP ruling for sub-micron Clipper2 scanbeam/associativity rounding)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Planner Phase-3 batched union attribution and mechanism screen](178-planner-phase3-union-attribution.md) ([`evidence/t178-planner-phase3-union/FINDINGS.md`](../evidence/t178-planner-phase3-union/FINDINGS.md)), which attributed clean production Phase-3 union at **4.43 s wall / 45.52 s thread (`15.68 %` of `base-classic-on`)** and **154.5 ms wall / 1.55 s thread (`4.48 %` of `benchy-classic-on`)** and screened 29 mechanisms across 100 % of real captured payloads, finding **`M9n_host_nested_prune_hier_k8`** (`−91.38 %` Phase-3 wall / `−92.43 %` thread on `base`, `−69.26 %` wall / `−74.14 %` thread on `benchy`, `< 1 µm` worst-case boundary Hausdorff).

## Question

**Does implementing host-side fast exact convex-containment pruning + bbox-connected components + Morton Z-order spatial hierarchical union (`slicer_core::polygon_ops::union_spatial_tree`, wired strictly into `clip_polygons_batch` `ClipOperation::Union`) clear the standing `t137` 4-cell planner gate and bank the projected ~4.0 s (`−14 %` cell) `base` supports-on and ~105 ms (`−3 %` cell) `benchy` supports-on savings?**

## What is already known (t178)

- **Why Phase 3 is slow:** `ROLE_BODY` is **99.99 %** of `base` Phase-3 thread-time (`431/480` calls, `136,256` polygons / `13.48 M` vertices collapsing 11.0× into `2,662` polygons / `1.21 M` vertices; **7.64× area overlap**, **28.11 % strictly contained** inside a single larger convex sibling; max single call `1,024` 100-gons / `414.2 ms`). `88.5 %` of vertices sit in giant connected components (`>= 65` polys, max `1,023`), so bbox-connected components alone (`M2`) fails.
- **Architectural placement (`clip_polygons_batch` has ONLY ONE caller in the repo):** `modules/core-modules/tree-support-planner/src/lib.rs::run_clip_batches` is the sole production caller of `clip_polygons_batch`. Dispatching `union_spatial_tree` inside `crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` (plus `crates/slicer-sdk/src/host_batch.rs::clip_polygons_batch` native fallback so native tests match 1:1) when `op == ClipOperation::Union` and `subject.len() + clip.len() > k`:
  - Requires **zero WIT/schema changes** and **zero guest WASM changes** (stays at 37 batches / 480 requests, zero guest fuel penalty, prep parallelizes natively across 12 Rayon threads).
  - Is **100 % structurally inert on all 4 supports-off cells** (`enable_support: false` never calls `clip_polygons_batch`), giving **zero part-geometry risk** across the entire slicer and satisfying `t137`'s four-cell planner gate structurally by construction.
- **Screened performance & precision (`t178` 12-thread offline replay on 100 % real payloads):**
  - `M9n_host_nested_prune_hier_k8` (primary): **`−91.38 %`** Phase-3 wall / **`−92.43 %`** thread on `base` (projects **`−4,052 ms` = `−14.33 %` cell, `17.9×` floor** on `base-classic-on`; **`−4,025 ms` = `−14.06 %`, `17.6×`** on `base-arachne-on`; **`−107 ms` = `−3.10 %`, `3.9×`** on `benchy-classic-on`; **`−107 ms` = `−2.99 %`, `3.7×`** on `benchy-arachne-on`), with worst-case symmetric boundary Hausdorff **`0.949 µm`** on `base` and **`0.400 µm`** on `benchy` (total XOR area `1.64 × 10⁻⁸` on `base`, `4.93 × 10⁻⁹` on `benchy`).
  - `M9_host_worker_prune_hier_k4`: **`−92.12 %`** wall / **`−93.48 %`** thread on `base`, Hausdorff `2.309 µm`.
  - `M9_host_worker_prune_hier_k16`: **`−88.48 %`** wall / **`−90.07 %`** thread on `base`, Hausdorff `0.721 µm`.
- **Clipper2 distant-scanbeam / associativity ruling required (per `t175` / `t137` note):** `t178` proved `147/147` (`base`) and `27/27` (`benchy`) of `M2`'s diffs are `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` distant-scanbeam Y clamping, and hierarchical chunking adds sub-micron integer-grid associativity rounding (`<= 0.949 µm` at `k = 8`). Expect **4/4 supports-off cells byte-identical** to `t175` and **zero part-geometry movement** on supports-on, with minor sub-micron/1 µm `Support` / `Support interface` shifts requiring an explicit human KEEP ruling (matching `t175`'s ceremony) before updating `reference.json` and committing.
- **Route arithmetic (`ESTIMATE ONLY`):** pushes `base-classic-on` `1.221x → ~1.046x` ord (`1.002x` acc) and `base-arachne-on` `1.201x → ~1.032x` ord (`0.991x` acc), closing **79.3 % / 84.0 %** of the remaining ordinary gap; stacking with [`SupportGeometry` post-planner host tail attribution and reduction screen](181-support-geometry-post-planner-host-tail.md) (`7.8 % / 7.0 %` post-planner host tail) crosses `< 1.0x` cleanly on both ordinary `base` cells (`~0.95x`, **4/8**).

## Scope

1. **Implement `slicer_core::polygon_ops::union_spatial_tree`** (`crates/slicer-core/src/polygon_ops.rs` or a new focused submodule `crates/slicer-core/src/polygon_ops_spatial_union.rs`, `host-algos`-gated if using Rayon nested parallelism or clean single-module helper):
   - Fast exact convex-containment pruning (inscribed-disk $O(1)$ fast-accept + $O(\log V)$ exact binary-search point-in-convex-CCW polygon, only pruning polygons strictly inside a single larger hole-free convex CCW polygon; never drops non-canonical or hole-carrying containers).
   - Deterministic X-sweep + union-find bbox-connected components; canonical simple singletons (`region_has_canonical_winding`) bypass Clipper2.
   - 2D Morton Z-order sort of component polygons by bbox center + strictly-terminating chunked hierarchical tree reduction (forcing final union if a round fails to shrink or at max depth `5`), with optional nested Rayon fan-out inside a large component when `tasks.len() >= 4 && total_verts >= 4096`.
   - Wire into `crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` (and `crates/slicer-sdk/src/host_batch.rs` native fallback) **strictly for `ClipOperation::Union`** when `subject.len() + clip.len() > k` (leaving `clip_polygons` serial callers and all `Difference`/`Intersection`/`Xor` ops completely untouched).
   - Add targeted unit + property/exactness tests in `slicer-core` (convex containment exactness, non-convex negative control, hole-bearing preservation, nested islands, non-shrinking disjoint-bbox-chain termination guard, equivalence against monolithic `clip_polygons(..., Union)` within sub-micron XOR tolerance) — watch tests fail or verify falsifiers cleanly.
2. **Parameter sweep on `base-classic-on` (`k ∈ {4, 8, 16}`, worker-local vs nested-Rayon):**
   - Measure 1 warmup + 3 alternating rounds on a quiet box, recording wall, CPU, G-code diff vs `t175` currency (`total_extruding_delta_mm`, moved sections, verifying **zero part-geometry movement**), and select the winner (`k = 8` preferred if within noise of `k = 4` for sub-micron Hausdorff).
3. **Run the standing `t137` 4-cell planner gate (5 interleaved pairs/cell ordinary, 3-pair null control, 3-pair accelerated corroboration):**
   - Verify 4/4 supports-on cells pass (`>= 0.80 %` paired wall outside null band + CPU corroborating, accelerated corroborating).
   - Verify 4/4 supports-off cells are **byte-identical to `t175`'s `reference.json`** (`0` moved bytes) and show no median wall/CPU regression (`INERT-PASS`).
   - Verify **0** non-zero exits, `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics, and ordinary ≡ accelerated output **8/8** (`16/16` incl. baseline).
   - Run full G-code section-by-section delta audit on the 4 supports-on cells confirming **0** part-geometry lines moved (every moved line strictly inside `Support` / `Support interface`).
4. **Human KEEP ruling & pre-commit gates:**
   - Present the measured paired gate + G-code section audit to the human for explicit KEEP ruling on the moved supports-on hashes (matching `t175`'s precedent) before committing and advancing `reference.json`.
   - Run `cargo test -p slicer-core --features host-algos --no-fail-fast`, `cargo test -p slicer-wasm-host`, `cargo test -p tree-support-planner --no-fail-fast`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, `cargo xtask build-guests --check`, and `cargo xtask build-guests --accelerated --check`.

## Acceptance

- Evidence directory `docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/` with sweep, paired A/B + null + accelerated reduced JSONs, G-code delta audit, `reference.json` (if KEEP), `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`verify.py --self-test` exit 0).
- Updated `## Answer`, `map.md` update, and clean commit after explicit human KEEP ruling.

## Answer

_(open)_
