# Exact `com.core.tree-support-planner` acceleration: height-capped bucket-parallel `TreeVolumes`, zero-alloc guest geometry, and whole-object batched occupancy/contact offsets

Type: task / measured candidate (100 % output-preserving pure-guest acceleration + standing `t137` 4-cell planner gate; zero host/WIT changes, zero geometry exceptions, zero gate waivers)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [`PrePass::SupportGeometry` remainder (`com.core.tree-support-planner`) attribution and speedup screen](187-tree-support-planner-post-t182-residual-screen.md) ([`evidence/t187-tree-support-planner-residual/FINDINGS.md`](../evidence/t187-tree-support-planner-residual/FINDINGS.md)), which attributed the entire post-t185 `PrePass::SupportGeometry` stage (`16/16` probe runs byte-identical to `t182/reference.json`, `4/4` qual clean, classic ≡ arachne bit-exact inside `run_support_geometry_with_analysis`) and screened the pure-guest exact bundle `P1 + P2 + P3 + P4 + P5_zeroclone` across `42` measured + `2` warmup runs (`44/44` byte-identical to `t182/reference.json`, `12/12` A/B wall wins, `+3.96…+10.98 %` wall = `4.95×–13.72×` floor, CPU corroborating `4/4`, `tree-support-planner` `−34.1…−36.5 %`, deterministic planner fuel `−38.85 / −41.03 %`, `host:native` calls `−23.9…−25.6 %`, supports-off structurally inert `2/2`).

## Question

**Does landing the 100 % output-preserving, pure-guest bundle `P1 + P2 (+ P2k) + P3 + P4 + P5_zeroclone` strictly inside `modules/core-modules/tree-support-planner/src/lib.rs` (zero host, WIT, or Clipper2 changes) pass the standing `t137` 4-cell planner gate (`>= 0.80 %` paired ordinary + accelerated wall and CPU win on all 4 supports-on cells, structurally inert with non-regression on all 4 supports-off cells, `16/16` byte-identical to `t182/reference.json`)?**

## What is already known (t187)

- **Screen results (`modules/core-modules/tree-support-planner/src/lib.rs` only, clean HEAD `pnp_cli.exe`, `RAYON_NUM_THREADS=12`, unsandboxed, `44/44` byte-identical to `t182/reference.json`):**
  - `benchy-classic-on`: **`+8.64 %`** wall (`2.7051 → 2.4714 s`, `10.80×` floor, `3/3`), **`+4.22 %`** CPU, `tree-support-planner` `722 → 476 ms` (`−34.07 %`), deterministic planner fuel `5.852 → 3.579 B` (`−38.85 %`), `host:native` calls `41,609 → 31,276` (`−24.83 %`).
  - `benchy-arachne-on`: **`+9.12 %`** wall (`2.7697 → 2.5172 s`, `11.40×` floor, `3/3`), **`+4.47 %`** CPU, `tree-support-planner` `723 → 471 ms` (`−34.85 %`), deterministic planner fuel `5.852 → 3.579 B` (`−38.85 %`), `host:native` calls `40,396 → 30,063` (`−25.58 %`).
  - `base-classic-on`: **`+3.96 %`** wall (`20.8204 → 19.9969 s`, `4.95×` floor, `3/3`; stage-implied `+10.11 %`), **`+2.44 %`** CPU, `tree-support-planner` `5,770 → 3,665 ms` (`−36.48 %`), deterministic planner fuel `48.565 → 28.636 B` (`−41.03 %`), `host:native` calls `145,921 → 111,104` (`−23.86 %`).
  - `base-arachne-on`: **`+10.98 %`** wall (`20.8262 → 18.5399 s`, `13.72×` floor, `3/3`; stage-implied `+9.39 %`), **`+2.24 %`** CPU, `tree-support-planner` `5,753 → 3,752 ms` (`−34.78 %`), deterministic planner fuel `48.565 → 28.636 B` (`−41.03 %`), `host:native` calls `143,484 → 108,667` (`−24.27 %`).
- **Route projection (`ESTIMATE ONLY`, cross-window composition of `t185/reduced/route.json × t187` screen):**
  - Pushes **`benchy-arachne-on` across `< 1.00x`** (`1.0846x → ~0.9857x` ord A/B / `~0.9835x` stage-delta → **`7/8` estimated `< 1.0x`**).
  - Cuts binding `benchy-classic-on` roughly halfway to parity (`1.2036x → ~1.0996x` ord A/B / `~1.0932x` stage-delta), leaving `~9 %` for `t188` (`SupportAnalysis`) and `t189` (`PostPass`).
  - Deepens `base` supports-on to `~0.8100x / ~0.7419x` ord (`~0.7581x / ~0.7551x` stage-delta).
- **Explicit non-goals (proven in t187):**
  - **Do NOT include `P5_tile`** (reusing `CollisionTileIndex` in `build_roles_batched` Phase 5): falsified as non-zero-drift on `base-classic-on` (`ea650ef8064a…` vs `7e115231d4b0…`) due to Clipper2 distant-scanbeam clamping (`vendor/clipper2-rust/src/engine.rs::add_new_intersect_node`).
  - **Do NOT include `H1`** (`com.core.traditional-support-planner` wasted host dispatch, `33.33 / 183.10 ms`): belongs to host/scheduler dispatch, not pure-guest `tree-support-planner`.

## Scope

All production edits stay strictly inside `modules/core-modules/tree-support-planner/src/lib.rs` (zero host, WIT, schema, or Clipper2 changes):

1. **`P1` — Height-capped + bucket-parallel `TreeVolumes`:**
   - Compute exact `max_layer` upper bound per radius bucket from `contacts_by_layer` (avoiding `54.4 %` `benchy` / `56.6 %` `base` avoidance layers strictly above the highest queried layer) and prefetch the drop-pass collision + avoidance ladders via `TreeVolumes::prefetch_drop_ladders`:
     - Batch all needed `(r, layer)` collision offsets in a single `offset_polygons_batch` pass (with exact `<= 8` polygon guard on any batched `clip_polygons_batch(Union)` so `t182`'s `> 8` `union_spatial_tree` threshold never fires and output remains 100 % byte-identical!).
     - Advance the avoidance recurrence layer-by-layer in parallel across all active radius buckets (`offset_polygons_batch` + `<= 8`-guarded `clip_polygons_batch(Union)` per layer step), preserving exact single-op Clipper2 semantics when `polys.len() > 8`.
   - Before `emit_pass`, collect the exact set of `(r, layer)` collision cells queried by active nodes and materialize only those cells via `TreeVolumes::prefetch_collision_cells` (keeping lazy fallback for any unprefetched query).
   - Batch `tv_simplify_union` (with the same `<= 8`-polygon guard on batched `Union`).
2. **`P2` (+ optional `P2k`) — Exact zero-alloc guest geometry & $O(1)$ lookups:**
   - Add `IndexedPolySet` storing precomputed per-ring `f32` AABBs alongside `ExPolygon`s; replace allocating `point_in_any_expoly` and `projection_onto` hot loops with zero-alloc `point_in_ring_pts` and `closest_point_on_ring_sq` (using exact AABB containment / lower-bound pruning that preserves bit-for-bit tie-breaking and floating-point arithmetic).
   - Add `LayerOutlineEdgeIndex` (precomputed `f64` ring and edge AABBs inside `LineCutCache`) for `is_line_cut_by_contour`.
   - Replace $O(S \times N)$ linear scans in `skeleton.wall_counts` with an $O(1)$ `HashSet` of active node positions per layer.
   - Short-circuit `node_roles` ancestor walks when `base_n == 0`.
   - Memoize `swallowed_by_collision` per active node per layer (`swallowed_cache`) and evaluate it *before* constructing the 100-vertex `node_ellipse`.
   - Replace `.sqrt()` inside `prim_mst`'s $O(V^2)$ inner loop with squared-distance comparisons (preserving exact tie-breaking).
   - Add `TreeVolumes::move_out_cache` to eliminate `78.1 %` (`benchy`) / `74.3 %` (`base`) duplicate serial `move_out_expolys` host dilations.
   - Use zero-alloc `assign_node_group_indexed`.
   - **Optional zero-drift polish (`P2k`):** reuse ping-pong `Vec` buffers (`std::mem::swap`) and `Rc<Vec<String>>` for `demand_ids` inside `smooth_nodes` (`10.43 / 72.27 ms`, `437M / 2.9B` fuel) if it keeps `16/16` byte-identity cleanly.
3. **`P3` — Whole-object batched occupancy prefetch + $O(1)$ layer maps:**
   - Build `model_occ_by_layer`, `print_occ_by_layer`, and `seg_origins_by_layer` once per object instead of $O(L^2)$ linear scans of `support_analysis`.
   - Prefetch all active layers' occupancy offsets in 256-request chunks instead of 24-request mini-batches.
4. **`P4` — Batched contact erosion + $O(1)$ segmentation set:**
   - Replace linear `seg_origins` lookup with `HashSet`; collect all eligible overhang candidates in `contact_generation` and execute their erodes in a single `offset_polygons_batch` call (preserving exact candidate order).
5. **`P5_zeroclone` — Move instead of clone in `build_roles_batched`:**
   - Use `std::mem::take` across Phases 3/5/6/7 and move the final origin's polygon vectors into `RoleBuildInput`.
6. **Required unit & regression tests:**
   - Unit tests in `modules/core-modules/tree-support-planner` proving:
     - `IndexedPolySet` `point_in_any` and `projection_onto` match the reference implementations across convex/concave polygons, holes, boundary points, and degenerate rings.
     - `LayerOutlineEdgeIndex` `is_line_cut_by_contour` matches the unindexed contour edge scan across crossing, grazing, collinear, and disjoint segments.
     - `TreeVolumes` height-capped prefetch + lazy fallback produces identical collision & avoidance polygons to full-height serial materialization, and obeys the `<= 8` union batching guard.
     - `prim_mst` squared-distance loop produces identical edge set and total length on equal-distance ties.
     - `enable_support: false` returns immediately (supports-off inertness contract).
7. **Standing `t137` 4-cell planner paired ordinary + accelerated + null-band gate (`RAYON_NUM_THREADS=12`, unsandboxed):**
   - Rebuild BOTH ordinary and accelerated guests (`cargo xtask build-guests` and `cargo xtask build-guests --accelerated`; verify both `--check` commands exit `0`).
   - **Output parity (fail-closed, zero reference change):** baseline AND candidate MUST reproduce `t182`'s active `reference.json` byte-for-byte on **16/16** `(8 cells × {ordinary, accelerated})` runs (`exit 0`, `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics). Any hash divergence is a bug — do NOT update `reference.json`.
   - **Timing (standing `t137` 4-cell gate):** 5 interleaved pairs/cell ordinary + 3-pair null control + 3-pair accelerated corroboration across all 4 supports-on cells (clearing `>= 0.80 %` wall with corroborating lower process CPU and accelerated corroboration), plus non-regression check on all 4 structurally inert supports-off cells.
   - **Pre-commit code gates:** `cargo test -p tree-support-planner` (excluding any pre-existing unrelated manifest-version check if documented, or fixing cleanly if in scope — check baseline first), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, `cargo xtask build-guests --check`, and `cargo xtask build-guests --accelerated --check`.

## Acceptance

- Clean, tested production implementation strictly within `modules/core-modules/tree-support-planner/` (zero host/WIT/Clipper2 edits; both `cargo xtask build-guests --check` and `--accelerated --check` exit `0`).
- Evidence directory `docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/` with reduced paired ordinary + null + accelerated + `--profile` JSONs, `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`verify.py --self-test` exit `0`).
- Updated `## Answer`, `map.md` update, and clean commit after all gates pass (zero reference-set change).
