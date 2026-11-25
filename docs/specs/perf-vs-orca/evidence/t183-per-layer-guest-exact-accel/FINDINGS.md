# Ticket 183 — Exact per-layer guest acceleration (`tree-support` renderer fast paths + `infill-linker` open-subject tagging)

- **Date:** 2026-11-23 (UTC) / 2026-11-22 (local)
- **Base commit:** `3e2aae38` (production code `1d9d6a94`, post-`t182`)
- **Active reference currency (unchanged — zero reference change):** `t182`'s active reference set ([`../t182-spatial-hierarchical-phase3-union/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json))
- **Guest freshness:** `cargo xtask build-guests --check` exit `0` and `cargo xtask build-guests --accelerated --check` exit `0`
- **Stamped arms (`reduced/output.json`):**
  - `baseline-ordinary`: `pnp_cli-ordinary.exe` `dd668170…` (48,543,232 B), modules tree `557c356a…` (`tree-support.wasm` `a6adbef8…`, `infill-linker.wasm` `daa991e4…`)
  - `candidate-ordinary`: `pnp_cli-ordinary.exe` `ce5bf89c…` (48,536,064 B), modules tree `f1dc4080…` (`tree-support.wasm` `ae528d5d…`, `infill-linker.wasm` `2a5a17f2…`)
  - `baseline-accelerated`: `pnp_cli-accelerated.exe` `3c2bc193…` (48,542,208 B), modules tree `b5ccab3a…` (`tree-support.wasm` `105d6ba9…`, `infill-linker.wasm` `e83b1dcc…`)
  - `candidate-accelerated`: `pnp_cli-accelerated.exe` `c16f2752…` (48,562,176 B), modules tree `d758a6c9…` (`tree-support.wasm` `bd13830a…`, `infill-linker.wasm` `606d6ced…`)
- **Verifier:** `python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/verify.py --self-test` → `PASS (16/16)`; `python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/verify.py` → `PASS`

---

## 1. Verdict & Executive Summary

**LANDED — LITERAL STANDING ALL-8-CELL `0.80 %` GATE PASSED (`8/8 PASS`), ZERO OUTPUT DRIFT (`224/224` MEASURED RUNS BYTE-IDENTICAL TO `t182/reference.json`), ZERO GEOMETRY EXCEPTIONS, ZERO GATE WAIVERS.**

1. **Output parity (fail-closed, zero reference change):** Every one of **224/224** measured runs across ordinary output (`16`), accelerated output (`16`), `--profile` fuel (`16`), 5-pair ordinary A/B (`80`), 3-pair null control (`48`), and 3-pair accelerated corroboration (`48`) exited `0`, reported `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics, and reproduced `t182`'s active reference hashes **byte-for-byte (`0` hash mismatches)**. `t182/reference.json` stays the active currency without modification.
2. **Literal standing 8-cell ordinary paired gate (`5 interleaved pairs/cell`, unsandboxed, `RAYON_NUM_THREADS=12`, load `2 % → 3 %`; 3-pair null load `1 % → 9 %`):**
   - **8/8 PASS** on the literal standing gate (`>= 0.80 %` median wall saving, strictly above null median wall, process CPU lower): **`+2.20 %` to `+9.59 %`** wall (**2.75×–11.99×** the `0.80 %` floor, **`39/40` individual pair wall wins**, `6/8` strict wall separation) and **`+3.10 %` to `+16.54 %`** process CPU (**`40/40` individual pair CPU wins, `8/8` strict CPU separation**).
   - **Stricter any-pair null lens (`_gate_strict_any_pair_null`):** **7/8 PASS**; `base-arachne-off` (`+2.20 %` wall, `5/5` wins, **4.47×** its `0.49 %` null median; `+3.34 %` CPU = **4.25×** its `0.79 %` null any-pair CPU) is disclosed `INSIDE-NULL-ANY-PAIR` solely against one `3.56 %` null pair-1 wall spike (pairs 2–3 null were `0.47 %` and `0.49 %`).
3. **Accelerated corroboration (`3 interleaved pairs/cell`, unsandboxed, load `0 % → 1 %`):**
   - **8/8 cells** clear the `0.80 %` wall floor (**`+1.85 %` to `+7.97 %`**, `23/24` wall wins, `6/8` strict sep) with process CPU corroborating in **8/8 cells (`+3.53 %` to `+15.92 %`, `23/24` wins + 1 tie, `7/8` strict sep)**.
4. **Deterministic WASM guest fuel (`--profile`, 16/16 byte-identical):**
   - **`com.core.tree-support`:** `41.94 B → 1.50 B` (**−96.42 %**, **27.95×**) on both `benchy` supports-on cells and `388.78 B → 11.44 B` (**−97.06 %**, **33.99×**) on both `base` supports-on cells. *(Note on `TS-2` soundness: strictly enforcing Ticket 183's pairwise AABB-disjointness predicate on unregularized multi-polygon roles preserves unconditional soundness on merged/overlapping inputs; concave-disjoint tree branches whose AABBs overlap still run `union_ex`, retaining `0.85 B` (`benchy`) / `3.88 B` (`base`) vs `t179`'s unconditional-skip ablation — `~5 / ~30 ms` wall — while eliminating `96.4–97.1 %` of renderer fuel.)*
   - **`com.core.infill-linker`:** `−74.03 %` (`3.85×`, `10.27 B → 2.67 B`) on `benchy-classic`, `−74.56 %` (`3.93×`, `11.11 B → 2.83 B`) on `benchy-arachne`, `−79.34 %` (`4.84×`, `37.46 B → 7.74 B`) on `base-classic`, and `−79.05 %` (`4.77×`, `40.35 B → 8.45 B`) on `base-arachne` — exceeding `t179`'s screen reductions (`−73.0…−78.6 %`) via `BoundaryRing` edge/bbox reuse across `vertices_between`, `contour_stub`, `point_strictly_in_ring`, and cross-ring `boundary_position` pruning.
   - **Total slice guest fuel:** `−23.92 %` to `−40.17 %` on supports-off and **`−69.73 %` to `−73.83 %`** (`3.30×–3.82×`) on supports-on.
5. **Cross-window route impact (`ESTIMATE ONLY` — `t174 × t175 × t182 × t183`, never a scoreboard ratio of record):**
   - **Both ordinary `base` supports-on cells cross `< 1.0x`:** `base-classic-on` `1.0515x → ~0.9734x` ord (`0.9255x` acc) and `base-arachne-on` `1.0329x → ~0.9662x` ord (`0.9173x` acc) — putting **all 4 `base` cells `< 1.0x`** on both ordinary and accelerated arms!
   - `benchy-arachne-off` crosses `< 1.0x` on accelerated (`1.0640x → ~0.9885x`, ordinary `1.0583x`); `benchy-classic-off` closes **76.3 %** of its remaining gap (`1.1438x → ~1.0341x` ord / `~1.0370x` acc); binding `benchy-classic-on` moves `1.5402x → ~1.4539x` ord (`1.4130x` acc) and `benchy-arachne-on` moves `1.3550x → ~1.2773x` ord (`1.2637x` acc).
   - Official running tally remains **2/8 provisionally won** until a fresh single-HEAD Orca scoreboard refresh is taken.

---

## 2. Production Implementation Summary

1. **`TS-1a` (`crates/slicer-core/src/paint_policy.rs::has_support_paint`, `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for`):**
   - Added `has_support_paint` using the exact `entries_has_some_paint` predicate as `support_eligibility` for `PaintSemantic::SupportBlocker | PaintSemantic::SupportEnforcer`.
   - `paint_policy_for` now skips `region_covers_expoly` for every region lacking support paint and short-circuits on `SupportPaintPolicy::Blocked` (highest precedence).
   - Verified with 4 targeted `slicer-sdk` unit tests covering `SupportBlocker`, `SupportEnforcer`, blocker-over-enforcer precedence on mixed painted/unpainted regions, non-convex L-shaped `expoly` probe (centroid in notch), and unpainted/non-support/all-`None` fast-path equivalence.
2. **`TS-1b` & `TS-2` (`modules/core-modules/tree-support/src/lib.rs`):**
   - `skeleton_wall_count`: precomputes contour and hole AABBs once per `expoly` (`ring_bbox_units`, `point_in_expolygon_with_bboxes`) and skips `count <= best` before ray-casting.
   - `unify_role_regions` + `expolygons_pairwise_bbox_disjoint`: bypasses redundant in-guest `slicer_core::polygon_ops::union_ex(&regions)` strictly when `regularized.is_some()` OR `regions.len() <= 1` OR every contour has `>= 3` vertices and all contour AABBs are strictly pairwise disjoint (`ax1 < bx0 || bx1 < ax0 || ay1 < by0 || by1 < ay0`).
   - Verified with unit tests including a negative control proving overlapping or edge-touching unregularized polygons still invoke `union_ex` and merge identically.
3. **`IL-1` (`vendor/clipper2-rust/src/engine.rs`, `engine_public.rs`, `crates/slicer-core/src/polygon_ops.rs::PreparedPolylineClip::clip_batched`):**
   - Added `source_id: u32` (default `0`) to `LocalMinima` and `OutRec`, `ClipperBase::add_loc_min_tagged` / `add_path_tagged`, propagation in `start_open_path`, `add_local_min_poly`, and `join_outrec_paths` (with `debug_assert!` verifying matching `source_id` on open joins), plus `Clipper64::add_open_subject_tagged` and `Clipper64::execute_open_tagged -> Result<Vec<(u32, Path64)>, Clipper2Exception>`.
   - Rewired `PreparedPolylineClip::clip_batched` to tag each valid open subject with its subject index and bucket `execute_open_tagged` directly into `results[position]`, deleting 249 lines of geometric re-attribution (`attributed_runs`, `candidate_subjects`, `segment_rides`, `path_to_point2`) and eliminating all per-subject ambiguity fallback clips while retaining the `Err` / `u32` overflow defensive fallback.
   - Verified with `clipper2-rust` (`406/406` + 3 doc-tests, including `test_execute_open_tagged_crossing_collinear_holes_and_empty`) and `slicer-core` (`polygon_ops_clip_batched_tdd` `11/11`, including coincident/duplicate/crossing/multi-hole stress).
4. **`IL-2` (`modules/core-modules/infill-linker/src/orchestrate.rs`):**
   - Added `collect_source_segments: bool` to `link_paths_against` and `link_paths_without_offset` (`false` in `link_region_group`, `true` in `link_union_group`), eliminated redundant `tagged.clone()` and per-subject polyline cloning, and added bitwise-exact `uniform_point_metadata` (destructuring every field of `Point3WithWidth`) for `O(1)` metadata attachment on uniform paths with exact fallback to `point_with_metadata` when any metadata field varies.
   - Verified with uniform + non-uniform negative-control unit tests across `width`, `flow_factor`, and `overhang_quartile`.
5. **`IL-3` (`modules/core-modules/infill-linker/src/graph.rs` & `connect.rs`):**
   - Added precomputed `BoundaryEdge` (`start`, `dx`, `dy`, `len`, `len_sq = len * len`, `start_arc`, edge AABB) and ring AABB on `BoundaryRing`; added bit-exact `BoundaryRing::project_bounded` used by both `BoundaryInfillGraph::project` and `connect::boundary_position` (cross-ring `best_distance` pruning + hole AABB early exit in `point_strictly_in_ring`); reused precomputed edges in `vertices_between` and `contour_stub`.
   - Replaced `O(E²)` `nearest_pair_candidates` with per-ring `local_position`-sorted forward/backward circular walks stopping as soon as directed arc distance exceeds `best_dist`, preserving exact `directed_distance` + `endpoint_order` tie-breaking and output ordering.
   - Verified with bit-exact brute-force equivalence tests over multi-hole/multi-island graphs, wrap-around seams, coincident endpoints, and incompatible tool/width paths (`infill-linker` `43/43` tests green).

---

## 3. Standing 8-Cell Gate Results (`reduced/ab.json`, `reduced/null.json`, `reduced/accel.json`)

Unsandboxed, `RAYON_NUM_THREADS=12`, quiet box (`ab` `2 → 3 %`, `null` `1 → 9 %`, `accel` `0 → 1 %`), **224/224** measured runs exit `0`, `degraded = false`, `0` code-1200, **100 % byte-identical** to `t182/reference.json`.

| Cell | Ord Base → Cand (s) | Ord Wall Saved (wins, floor×, sep) | Ord CPU Saved (wins, sep) | Null Wall med / any (%) | Acc Wall / CPU Saved (3p) | Literal 8-Cell Gate | Strict Any-Pair Lens |
|---|---:|---:|---:|---:|---:|---|---|
| `benchy-classic-off` | `1.739 → 1.571` | **`+9.59 %`** (`5/5`, `11.99×`, `true`) | **`+6.24 %`** (`5/5`, `true`) | `0.90 / 1.86` | **`+7.23 %` / `+3.87 %`** | **PASS** | **PASS** |
| `benchy-classic-on` | `3.288 → 3.104` | **`+5.60 %`** (`4/5`*, `7.00×`, `false`) | **`+16.54 %`** (`5/5`, `true`) | `0.68 / 1.39` | **`+7.54 %` / `+15.92 %`** | **PASS** | **PASS** |
| `benchy-arachne-off` | `1.800 → 1.664` | **`+8.36 %`** (`5/5`, `10.45×`, `true`) | **`+8.45 %`** (`5/5`, `true`) | `1.22 / 1.75` | **`+7.09 %` / `+7.06 %`** | **PASS** | **PASS** |
| `benchy-arachne-on` | `3.395 → 3.238` | **`+5.73 %`** (`5/5`, `7.16×`, `true`) | **`+14.60 %`** (`5/5`, `true`) | `3.29 / 3.33` | **`+6.77 %` / `+15.47 %`** | **PASS** | **PASS** |
| `base-classic-off` | `11.016 → 10.366` | **`+6.58 %`** (`5/5`, `8.23×`, `true`) | **`+3.10 %`** (`5/5`, `true`) | `0.90 / 1.53` | **`+2.39 %` / `+3.59 %`** | **PASS** | **PASS** |
| `base-classic-on` | `24.315 → 22.359` | **`+7.43 %`** (`5/5`, `9.29×`, `true`) | **`+15.40 %`** (`5/5`, `true`) | `0.85 / 1.00` | **`+7.27 %` / `+15.06 %`** | **PASS** | **PASS** |
| `base-arachne-off` | `11.191 → 10.918` | **`+2.20 %`** (`5/5`, `2.75×`, `false`) | **`+3.34 %`** (`5/5`, `true`) | `0.49 / 3.56` | **`+1.85 %` / `+3.53 %`** | **PASS** | `INSIDE-NULL-ANY-PAIR`† |
| `base-arachne-on` | `24.613 → 22.938` | **`+6.46 %`** (`5/5`, `8.07×`, `true`) | **`+14.24 %`** (`5/5`, `true`) | `0.59 / 4.29` | **`+7.97 %` / `+14.66 %`** | **PASS** | **PASS** |

*`benchy-classic-on` won 4/5 ordinary pairs (`3.091 / 3.111 / 3.104 / 3.078 s` vs baseline `3.261–3.344 s`); pair 3 candidate hit a single ~220 ms OS scheduling blip (`3.324 s` wall despite `12.297 vs 14.734 s` CPU = `−16.54 %` CPU on that same pair). Accelerated won `3/3` (`+7.54 %` wall, strict separation).
†`base-arachne-off` won `5/5` ordinary pairs (`+2.20 %` median wall = **4.47×** its `0.49 %` null median wall; `+3.34 %` CPU = **4.25×** its `0.79 %` null any-pair CPU) and `3/3` accelerated pairs (`+1.85 %` wall / `+3.53 %` CPU); its null batch pair 1 had one `3.56 %` transient while null pairs 2–3 were `0.47 %` and `0.49 %`.

---

## 4. Deterministic Guest Fuel Before/After (`reduced/fuel.json`)

Captured via `--profile` on `baseline-ordinary` vs `candidate-ordinary` (`16/16` byte-identical to `t182/reference.json`; attribution only).

| Cell | `com.core.tree-support` Fuel (Cut / Speedup) | `com.core.infill-linker` Fuel (Cut / Speedup) | Total Slice Guest Fuel (Cut / Speedup) |
|---|---:|---:|---:|
| `benchy-classic-off` | `86.39 M → 86.39 M` (`0.00 %`, inert) | `10.27 B → 2.67 B` (**−74.03 %**, `3.85×`) | `20.44 B → 12.83 B` (**−37.25 %**, `1.59×`) |
| `benchy-classic-on` | `41.94 B → 1.50 B` (**−96.42 %**, `27.95×`) | `10.27 B → 2.67 B` (**−74.03 %**, `3.85×`) | `68.91 B → 20.86 B` (**−69.73 %**, `3.30×`) |
| `benchy-arachne-off` | `86.72 M → 86.70 M` (`+0.02 %`, inert) | `11.11 B → 2.83 B` (**−74.56 %**, `3.93×`) | `20.65 B → 12.35 B` (**−40.17 %**, `1.67×`) |
| `benchy-arachne-on` | `41.94 B → 1.50 B` (**−96.42 %**, `27.95×`) | `11.11 B → 2.83 B` (**−74.56 %**, `3.93×`) | `69.11 B → 20.39 B` (**−70.50 %**, `3.39×`) |
| `base-classic-off` | `351.91 M → 351.89 M` (`+0.01 %`, inert) | `37.46 B → 7.74 B` (**−79.34 %**, `4.84×`) | `124.51 B → 94.73 B` (**−23.92 %**, `1.31×`) |
| `base-classic-on` | `388.78 B → 11.44 B` (**−97.06 %**, `33.99×`) | `37.46 B → 7.74 B` (**−79.34 %**, `4.84×`) | `565.14 B → 158.02 B` (**−72.04 %**, `3.58×`) |
| `base-arachne-off` | `353.17 M → 353.18 M` (`−0.00 %`, inert) | `40.35 B → 8.45 B` (**−79.05 %**, `4.77×`) | `113.78 B → 81.82 B` (**−28.09 %**, `1.39×`) |
| `base-arachne-on` | `388.78 B → 11.44 B` (**−97.06 %**, `33.98×`) | `40.35 B → 8.45 B` (**−79.05 %**, `4.77×`) | `554.41 B → 145.11 B` (**−73.83 %**, `3.82×`) |

---

## 5. Cross-Window Route Impact (`reduced/route.json` — `ESTIMATE ONLY`)

Composed from `t174` quiet-board official ratios × `t175` × `t182` × `t183` paired savings (**never a scoreboard ratio of record**).

| Cell | Est. Post-`t182` ord / acc | `t183` Saved ord / acc | Est. Post-`t183` ord / acc | Remaining `< 1.0x` Cut ord / acc | Share of Post-`t182` Cut Closed (ord) |
|---|---:|---:|---:|---:|---:|
| `benchy-classic-off` | `1.1438x` / `1.1179x` | `+9.59 %` / `+7.23 %` | **`1.0341x` / `1.0370x`** | `3.30 %` / `3.57 %` | **`76.3 %`** |
| `benchy-classic-on` | `1.5402x` / `1.5283x` | `+5.60 %` / `+7.54 %` | **`1.4539x` / `1.4130x`** | `31.22 %` / `29.23 %` | `16.0 %` |
| `benchy-arachne-off` | `1.1548x` / `1.0640x` | `+8.36 %` / `+7.09 %` | **`1.0583x` / `0.9885x`** | `5.51 %` / **`0.00 %`** | **`62.4 %`** |
| `benchy-arachne-on` | `1.3550x` / `1.3554x` | `+5.73 %` / `+6.77 %` | **`1.2773x` / `1.2637x`** | `21.71 %` / `20.87 %` | `21.9 %` |
| `base-classic-off` | `0.7687x` / `0.7656x` | `+6.58 %` / `+2.39 %` | **`0.7181x` / `0.7473x`** | `0.00 %` / `0.00 %` | already `< 1.0x` |
| `base-classic-on` | `1.0515x` / `0.9980x` | `+7.43 %` / `+7.27 %` | **`0.9734x` / `0.9255x`** | **`0.00 %`** / **`0.00 %`** | **`151.6 %` (crosses `< 1.0x`!)** |
| `base-arachne-off` | `0.7921x` / `0.7739x` | `+2.20 %` / `+1.85 %` | **`0.7747x` / `0.7596x`** | `0.00 %` / `0.00 %` | already `< 1.0x` |
| `base-arachne-on` | `1.0329x` / `0.9968x` | `+6.46 %` / `+7.97 %` | **`0.9662x` / `0.9173x`** | **`0.00 %`** / **`0.00 %`** | **`202.5 %` (crosses `< 1.0x`!)** |

**What remains on the frontier:**
- [Exact `ShellClassification` acceleration](../../issues/184-shell-classification-exact-acceleration.md) (`+15.2–16.5 %` supports-off, `+5.6–7.8 %` supports-on projected) pushes `benchy-classic-off` and `benchy-arachne-off` well below `< 0.90x` (toward **6/8 `< 1.0x`**) and pushes `base` supports-off toward `~0.60x`.
- [Exact `SupportGeometry` post-planner host-tail acceleration](../../issues/185-support-geometry-post-planner-host-tail-acceleration.md) (`+6.1–7.8 %` supports-on projected) pushes `base` supports-on well below `0.90x` and combines with `t184` to bring `benchy-arachne-on` near `~1.10x` and binding `benchy-classic-on` near `~1.26x`.

---

## 6. Pre-Commit Code & Freshness Gates (All Green)

- `cargo test -p clipper2-rust`: **406/406 + 3 doc-tests** passed
- `cargo test -p slicer-core --features host-algos --no-fail-fast`: **91/91 binaries** passed, 0 failed (including `polygon_ops_clip_batched_tdd` `11/11`)
- `cargo test -p slicer-sdk --features test`: **30/30 binaries** passed, 0 failed (including `paint_policy_tests` `4/4`)
- `cargo test --manifest-path modules/core-modules/tree-support/Cargo.toml`: all unit + integration suites passed (including `skeleton_wall_count` & `unify_role_regions` negative control)
- `cargo test --manifest-path modules/core-modules/infill-linker/Cargo.toml`: **43/43 tests** passed (including `metadata_tests` & `il3_equivalence_tests`)
- `cargo clippy --workspace --all-targets -- -D warnings` + guest clippy on `tree-support` & `infill-linker`: **0 warnings, 0 errors**
- `cargo xtask check-literals`: **0 violations** (187 watched types)
- `cargo xtask check-test-quality --report`: **0 findings**
- `cargo xtask build-guests --check`: exit `0`; `cargo xtask build-guests --accelerated --check`: exit `0`
