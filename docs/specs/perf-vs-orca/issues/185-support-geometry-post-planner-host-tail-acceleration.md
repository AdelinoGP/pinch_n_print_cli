# Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate

Type: task / measured candidate (100 % output-preserving exact host acceleration + standing `t137` 4-cell planner gate; zero geometry exceptions, zero gate waivers, zero WIT/guest changes, zero weakening of exact-Z zero-tolerance validation)
Label: wayfinder:task
Status: resolved (2026-11-23)
Assignee: agent (2026-11-23 — autonomous continuation after t184, per human instruction)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [`SupportGeometry` post-planner host tail attribution and reduction screen](181-support-geometry-post-planner-host-tail.md) ([`evidence/t181-support-geometry-host-tail/FINDINGS.md`](../evidence/t181-support-geometry-host-tail/FINDINGS.md)), which attributed the entire post-planner host tail (`212.91–214.61 ms` `benchy-on`, `1,939.31–1,946.49 ms` `base-on`, `100 %` single-threaded `1.0×`; `22/22` runs byte-identical to `t175/reference.json`, `6/6` `--instrument-stderr` qual clean, `2/2` supports-off inert, `200/200` `benchy` and `432/432` `base` `print_plane_occupancy` planes polygon-for-polygon identical) and proved via 12-thread exact IR replay + a `10,918`-pair adversarial falsification suite (`0` FN / `0` FP) that `M12345_full_exact_bundle` (`M1 + M2 + M3 + M4 + M5`) cuts the host tail **`−99.66 %`** (`210.83 → 0.71 ms`, `benchy`) and **`−99.85 %`** (`1,971.68 → 2.95 ms`, `base`) with zero Clipper2 fallbacks and bit-identical `SupportPlanIR` output.

## Question

**Does landing the 100 % output-preserving, zero-tolerance-preserving `M12345` host bundle (`M1` `print_plane_occupancy` reuse + `M2` exact bbox cull + `M3` zero-clone aggregation cleanup + `M4` exact `i128` topological disjointness certificate + `M5` order-preserving Rayon validation) across `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-wasm-host/src/exact_z_query.rs`, and `crates/slicer-wasm-host/src/support_aggregation.rs` pass the standing `t137` 4-cell planner gate (`>= 0.80 %` paired ordinary + accelerated wall and CPU win on all 4 supports-on cells, structurally inert on all 4 supports-off cells, `16/16` byte-identical output to `t175`'s active currency) while keeping `validate_entry`'s zero-tolerance exact-Z occupancy tripwire completely intact?**

## What is already known (t181)

- **Replay-verified exact cuts (`12` threads, `EXACT_BIT_IDENTICAL` on `200/200` `benchy` and `432/432` `base` entries):**
  - **`benchy`:** `210.83 → 3.38 ms` (`−98.40 %`, `62.5×`, serial `M1234`) and **`210.83 → 0.71 ms` (`−99.66 %`, `295.7×`, parallel `M12345`)**; Clipper2 calls `2,894 → 0` (`2,066` bbox-disjoint + `828` cert-disjoint).
  - **`base`:** `1,971.68 → 15.03 ms` (`−99.24 %`, `131.2×`, process CPU `1,968.75 → 15.63 ms` = `−99.21 %`, serial `M1234`) and **`1,971.68 → 2.95 ms` (`−99.85 %`, `667.9×`, parallel `M12345`)**; Clipper2 calls `5,292 → 0` (`3,199` bbox-disjoint + `2,093` cert-disjoint).
- **Standing `t137` 4-cell planner gate & route projection (`ESTIMATE ONLY`, `t174 × t175 × t177`):**
  - Clears the `0.80 %` floor in **4/4 supports-on cells at `7.57×–9.69×`** (`t177` scale; **`7.59×–8.68×`** same-session probe scale): `benchy-classic-on` **`−6.21 %`** (`7.76×`), `benchy-arachne-on` **`−6.06 %`** (`7.57×`), `base-classic-on` **`−7.75 %`** (`9.69×`), `base-arachne-on` **`−6.97 %`** (`8.72×`); supports-off cells are 100 % structurally inert (`0` active entries, `0 ms`).
  - **Route to 4/8:** stacking `t185` with [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](182-host-spatial-hierarchical-phase3-union.md) pushes **both ordinary `base` supports-on cells cleanly `< 1.0x`** (`base-classic-on` `1.2206x → 0.9511x` ord / `0.9110x` acc; `base-arachne-on` `1.2012x → 0.9485x` ord / `0.9107x` acc).
- **Why `M4` is essential (lesson from `t89`):** parallelizing Clipper2 without `M4` (`M1235`) cuts `base` wall to `95.28 ms` (`−95.17 %`) but burns `1,125 ms` process CPU (`+67 %` higher than serial `M123`'s `671.88 ms`) due to 12-thread Clipper2 cache/allocator contention. `M4` cuts both wall and CPU by `> 99 %`.

## Scope

1. **`M1` — Pre-seeded exact-Z occupancy reuse + lazy `object_geometry` (`crates/slicer-wasm-host/src/exact_z_query.rs`, `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-wasm-host/src/support_aggregation.rs`):**
   - Add an additive constructor / seeder on `ExactZQueryService` (leaving `ExactZQueryService::new` and `ExactZQueryService::query` contracts intact for all existing host/guest/test callers) that seeds a `(object_id, z_units) -> Arc<Vec<ExPolygon>>` map from `SupportAnalysisIR::print_plane_occupancy` joined via `SliceIR` (`(region.object_id, slice.global_layer_index) -> mm_to_units(slice.z)`), **strictly validating** that each seeded `object_id` exists in `MeshIR` with non-empty vertices (preserving missing-object / empty-mesh rejection contracts).
   - Add an occupancy-only lookup `ExactZQueryService::query_occupancy(&self, object_id, physical_z_mm) -> Result<Arc<Vec<ExPolygon>>, String>` used by `validate_entry` so seeded hits borrow `Arc<Vec<ExPolygon>>` directly without constructing unused `world_mesh`, `top` cross-section, `blockers`, or `plate` clones, and without running `slice_mesh_ex`.
   - **Hard soundness invariant:** any `(object_id, z_units)` NOT present in the seeded map (e.g., independent support Z planes, tests calling `ExactZQueryService::new` directly without `SupportAnalysisIR`) MUST automatically fall through to `prefetch_cross_sections` / `slice_mesh_ex` / `cross_section_at_z` identically to today. Never weaken to a Z tolerance.
2. **`M2` + `M4` — Exact full-`ExPolygon` bbox culling + exact `i128` topological disjointness certificate (`crates/slicer-wasm-host/src/support_aggregation.rs`):**
   - Compute full-`ExPolygon` bounds (contour + all holes, guarding against malformed polygons) once per occupancy polygon and body polygon; skip `slicer_core::polygon_ops::intersection` whenever `ax1 <= bx0 || bx1 <= ax0 || ay1 <= by0 || by1 <= ay0`.
   - For bbox-overlapping pairs, build a lazy per-entry `ChunkedPoly` (16-edge ring chunks) and run `exact_disjoint_certificate(a, b)`:
     1. Coordinate magnitude guard (`<= 10^15`, preventing any `i128` overflow); on exceed → `MayOverlapFallback`.
     2. Chunk-pruned exact `i128` segment-segment intersection/touch check across all contour + hole edges; **any** crossing, endpoint touch, or collinear overlap → `MayOverlapFallback`.
     3. Exact `i128` `FillRule::NonZero` winding-number test (matching `slicer_core::polygon_ops::intersection`'s `FillRule::NonZero`) on one representative vertex per ring (contour + all holes) of `a` against `b`, and of `b` against `a`; any nonzero winding or boundary touch → `MayOverlapFallback`.
     4. Return `ProvenDisjoint` only when all three checks pass; on `MayOverlapFallback`, run exact `slicer_core::polygon_ops::intersection(&[a], &[b])` with `SUPPORT_OVERLAP_TOLERANCE == 0`.
3. **`M3` — Zero-clone aggregation plumbing & cleanup (`crates/slicer-runtime/src/prepass.rs`, `crates/slicer-wasm-host/src/support_aggregation.rs`):**
   - Replace `(*plan).clone()` in `crates/slicer-runtime/src/prepass.rs::execute_prepass_with_instrumentation_collecting` with `Arc::unwrap_or_clone(plan)`.
   - Eliminate `plans.clone()` in `aggregate_support_plan_irs_with_policy_attributed` and `entry.clone()` in `try_aggregate_support_plans_with_policy` by moving `plans` / entries directly (recording decline diagnostics and plan-index attribution directly from `(plan_index, entry)` or compact metadata extracted only for diagnostics).
   - Delete the dead `identities` `HashMap` in `try_aggregate_support_plans_with_policy`.
   - Replace $O(N^2)$ linear search in `union_same_family_entries` with an insertion-order-preserving `HashMap` lookup, and gate the cross-family $O(N^2)$ loop on layers that actually have `> 1` distinct family.
4. **`M5` — Order-preserving Rayon parallel validation (`crates/slicer-wasm-host/src/support_aggregation.rs`):**
   - Keep ownership and decline admission serial in deterministic sorted entry order; validate admitted active entries in parallel via Rayon `par_iter()` (falling back to serial if small, or check in paired A/B whether `M1234` vs `M12345` has cleaner CPU on `benchy` — note `M1234` already achieves `−98.40 %` `benchy` / `−99.24 %` `base`, while `M12345` hits `−99.66 %` / `−99.85 %` and shields high-fallback models); fold verdicts into `retained` / `unmet` / `diagnostics` in exact entry order.
5. **Required unit & regression tests (zero-tolerance falsification):**
   - `slicer-wasm-host`:
     - Exact-Z `ChunkedPoly` / `exact_disjoint_certificate` adversarial test suite covering all 16 boundary/topology cases (1-unit = `100 nm` edge overlap, `1 unit²` corner overlap, shared-edge zero-area touch, vertex touch, zero-crossing $A \subset B$ and $B \subset A$, hole interior vs 1-unit hole-edge crossing, CW-reversed contour, concave U-bay disjoint vs 1-unit graze, coordinate-overflow guard `> 10^15` falling back cleanly to Clipper2), asserting `cert_plus_clipper == raw_clipper` with `0` FN and `0` FP.
     - Pre-seeded `ExactZQueryService` tests: seeded hit skips mesh slicing (`geometry_build_count() == 0`), unseeded plane falls back to exact `slice_mesh_ex` and still rejects a 1-unit real overlap, missing/empty object in `MeshIR` still rejects even if stale seeded entry is passed, and independent support Z plane still triggers exact-Z occupancy rejection (code `1200`).
     - Order-preserving parallel fold test (preserving `retained`, `unmet`, `diagnostics`, `ownership_violations` order identically to serial across mixed accepted/rejected/declined/trespassing entries).
6. **Standing `t137` 4-cell planner paired ordinary + accelerated + null-band gate (unsandboxed, `RAYON_NUM_THREADS=12`):**
   - **Zero guest/WIT changes:** both `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` MUST exit `0` untouched.
   - **Output parity (fail-closed, zero reference change):** baseline AND candidate MUST reproduce `t175`'s active `reference.json` byte-for-byte on **16/16** `(8 cells × {ordinary, accelerated})` runs (`exit 0`, `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics). Any hash divergence is a bug — do NOT update `reference.json`.
   - **Timing (standing `t137` 4-cell gate):** 5 interleaved pairs/cell ordinary + 3-pair null control + 3-pair accelerated corroboration across all 4 supports-on cells (clearing `0.80 %` wall with corroborating lower process CPU and accelerated corroboration), plus non-regression check on all 4 structurally inert supports-off cells.
   - **Pre-commit code gates:** `cargo test -p slicer-wasm-host`, targeted `slicer-runtime` support routing/aggregation integration tests (`support_family_routing`, `support_family_closure`, `support_agg_rasterizer_tdd`), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, `cargo xtask build-guests --check`, and `cargo xtask build-guests --accelerated --check`.

## Acceptance

- Clean, tested production implementation strictly within `crates/slicer-wasm-host` and `crates/slicer-runtime` (zero guest/WIT changes; both guest freshness checks exit `0`).
- Evidence directory `docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/` with reduced paired ordinary + null + accelerated campaign JSONs, `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`verify.py --self-test` exit `0`).
- Updated `## Answer`, `map.md` update, and clean commit after all gates pass (zero reference-set change).

## Answer

**Yes: landed.** `M1`–`M4` pass the standing t137 4-cell planner gate on the
pre-registered pooled two windows, with zero output drift
([`evidence/t185-support-geometry-host-tail-accel/FINDINGS.md`](../evidence/t185-support-geometry-host-tail-accel/FINDINGS.md),
`verify.py` PASS, self-test 15/15).

- **Supports-on (gated):** pooled paired wall saving is
  - benchy-classic-on **`+9.28 %`**;
  - benchy-arachne-on **`+7.21 %`**;
  - base-classic-on **`+8.81 %`**;
  - base-arachne-on **`+8.48 %`**.

  That is 36/40 pair wins and `9.0×`–`11.6×` the floor. Every cell is above its
  pooled null median with process CPU lower (`+1.11 %`…`+2.55 %`). Accelerated
  corroborates 4/4 (`+5.92 %`…`+10.92 %` wall, CPU lower, 3/3 pair wins
  everywhere).
- **Supports-off (inert):** pooled `+0.54 %`…`+1.70 %`. These cells run no
  planner tail.
- **Output:** 384/384 runs are byte-identical to t182's active currency (t175's
  set was superseded by t182). Zero degraded runs, errors or code-1200
  diagnostics. Guest module trees are identical between arms, and both
  freshness checks exit `0`.
- **Tail:** the instrumented post-planner host tail drops from
  `254/236 → 4 ms` (benchy) and `2,390/2,180 → 18/17 ms` (base).
- **Zero tolerance kept.**
  - Seeded occupancy covers only the producer's own print planes, for plainly
    affine non-empty objects. Every other plane is sliced as before.
  - The disjointness certificate is stricter than sketched: a 4-unit edge
    margin against Clipper2's rounding and a `2^30` coordinate cap. Anything
    uncertain falls back to the unchanged zero-tolerance Clipper2 test.
- **Not landed:**
  - `M5` (parallel validation): a few ms of the remaining tail.
  - `union_same_family_entries` restructuring: bounded by the `4–18 ms`
    residual.
- **Disclosed, not gated:**
  - Window 1's benchy-arachne-off reads `−8.11 %` against a `7.99 %` null
    median on an inert cell.
  - Under the strict any-pair null lens, base-classic-on sits below one
    `10.32 %` null pair.
  - Three `support_agg_rasterizer_tdd` tests fail identically on baseline
    `5761e533` (pre-existing tracked-baseline drift).
- **Route (ESTIMATE ONLY):** ordinary base supports-on cells reach
  `0.84x`/`0.83x`; benchy supports-on reach `1.20x`/`1.08x`. The estimated
  `< 1.0x` count stays 6/8.
