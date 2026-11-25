# t181 Findings — `SupportGeometry` Post-Planner Host Tail Attribution & Exact Acceleration Screen

Ticket: [`SupportGeometry` post-planner host tail attribution and reduction screen](../../issues/181-support-geometry-post-planner-host-tail.md).
Take: 2026-11-22, ordinary arm at HEAD `7b82f188` (last code commit `5b138f5c`, t175), `RAYON_NUM_THREADS=12`, `--no-default-module-paths`.
**Zero production code edits landed (`probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt, both guest freshness checks exit `0`), zero reference moves, no OrcaSlicer scoreboard re-run.**

> **Disclosure:** All probe timings are observation-only attribution (`probe.patch` reverted); all candidate speedups are 12-thread standalone exact IR replay on real dumped `benchy` (`200` entries) and `base` (`432` entries) snapshots; every cross-window post-t175 ratio (`t174 × t175 × t177`) and stacked disjoint-stage projection is explicitly **`ESTIMATE ONLY`**, never a three-tool scoreboard ratio of record.

---

## Verdict

1. **Premise clarification & exact code location:** WIT result decode (`crates/slicer-wasm-host/src/dispatch.rs::harvest_support_plan_ir`) and WASM store drop execute **inside** `runner.run_stage` *before* `com.core.tree-support-planner::module_complete`. The entire **`212.91–214.61 ms` (`benchy` supports-on)** and **`1,939.31–1,946.49 ms` (`base` supports-on)** host tail between `com.core.tree-support-planner::module_complete` and `PrePass::SupportGeometry::stage_complete` executes **100 % single-threaded (`1.0×` on 12 cores)** in pure host Rust inside `crates/slicer-runtime/src/prepass.rs::execute_prepass_with_instrumentation_collecting` → `crates/slicer-wasm-host/src/support_aggregation.rs::aggregate_support_plan_irs_degrading_with_attributed_diagnostics` → `crates/slicer-wasm-host/src/exact_z_query.rs::ExactZQueryService` (note: the ticket question hypothesized `Blackboard::commit_support_geometry` and `support_analysis_producer.rs::validate_entry`; the actual host symbols are `Blackboard::commit_support_plan`, `support_aggregation.rs::validate_entry`, and `exact_z_query.rs::ExactZQueryService`).
2. **97.4–98.2 % of the entire host tail is just two single-threaded (`1.0×`) bottlenecks** (plus `1.2–1.7 %` in three redundant deep clones of `SupportPlanIR`, bringing the top three to **`99.0–99.5 %`**; `commit_support_plan`, `union_same_family_entries`, `TerritoryClipper`, cross-family overlap, and drops together are **`< 1.0 %`**):
   - **Bottleneck 1 — Duplicate single-threaded mesh re-slicing (`39.11–40.24 %` of tail: `85.67–85.70 ms` `benchy`, `761.21–764.83 ms` `base`):** `ExactZQueryService::prefetch_cross_sections` calls single-threaded `slicer_core::slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) over `200` (`benchy`) / `432` (`base`) print-plane Zs (`84.62–84.76 ms` / `745.44–747.06 ms`), plus builds an unused `world_mesh` + `top` cross-section (`0.97–1.02 ms` / `14.76–15.48 ms`). Live polygon-for-polygon verification proves **`200/200` (`benchy`) and `432/432` (`base`) — 100.0 %, `0` missing planes — are already computed identically in `PrePass::SupportAnalysis`** (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::collect_print_plane_occupancy`) and sitting on the blackboard in `SupportAnalysisIR::print_plane_occupancy`.
   - **Bottleneck 2 — Unfiltered serial Clipper2 intersections in `overlaps_any` (`57.44–59.06 %` of tail: `122.56–123.27 ms` `benchy`, `1,138.45–1,149.51 ms` `base`):** `support_aggregation.rs::validate_entry` calls `overlaps_any` serially across all `(body, occupancy)` pairs (`2,894` `benchy`, `5,292` `base`; `0` positive overlaps) with **zero bounding-box culling**:
     - **`71.39 %` (`2,066/2,894`, `benchy`) and `60.45 %` (`3,199/5,292`, `base`)** of all Clipper2 `intersection` calls have **strictly disjoint bounding boxes**, burning **`61.49–61.90 ms` (`28.8–28.9 %`)** on `benchy` and **`462.57–464.83 ms` (`23.9 %`)** on `base`!
     - The remaining **`28.61 %` (`828`, `benchy`) and `39.55 %` (`2,093`, `base`)** bbox-overlapping pairs (`60.17–60.45 ms` `benchy`, `670.62–679.18 ms` `base`) sit in concave bays or holes of model cross-sections where boundaries do not intersect and neither polygon contains the other — yet Clipper2 sweeps all `1,700–10,000` vertices of the model layer serially on 1 core only to return `[]`.
   - **Bottleneck 3 — Three redundant deep clones of `SupportPlanIR` (`1.21–1.68 %` of tail: `3.42–3.58 ms` `benchy`, `23.37–24.41 ms` `base`):** `prepass.rs` clones `(*plan).clone()` out of a refcount-1 `Arc<SupportPlanIR>`, `aggregate_support_plan_irs_with_policy_attributed` clones `plans.clone()`, and `try_aggregate_support_plans_with_policy` clones every `entry.clone()` a third time — plus builds a dead `identities` `HashMap` that is never read.
3. **100 % exact, zero-tolerance acceleration bundle (`M12345_full_exact_bundle` = `M1 + M2 + M3 + M4 + M5`) eliminates 99.66–99.85 % of the entire host tail (`296×` `benchy`, `668×` `base`) while preserving `validate_entry`'s zero-tolerance tripwire intact:**
   - Standalone 12-thread exact IR replay (`5` interleaved rounds, `M0_baseline` reproducing live `classic-on` probe at `0.9902×` `benchy` and `1.0167×` `base`, `100 %` bit-identical `SupportPlanIR` output across all `7/7` modes):
     - **`M1234` (serial with `M1` PPO reuse + `M2` bbox cull + `M3` zero-clone + `M4` exact `i128` topological disjointness certificate):** `210.83 → 3.38 ms` (**`−98.40 %`**, `62.5×`) on `benchy` and `1,971.68 → 15.03 ms` (**`−99.24 %`**, `131.2×`, process CPU `1,968.75 → 15.63 ms` = `−99.21 %`) on `base`.
     - **`M12345` (`M1234` + `M5` Rayon order-preserving parallel validation):** `210.83 → 0.71 ms` (**`−99.66 %`**, `295.7×`) on `benchy` and `1,971.68 → 2.95 ms` (**`−99.85 %`**, `667.9×`) on `base`.
     - **Why `M4` beats `M1235` (parallel Clipper2 without `M4`):** `M1235` cuts `base` wall to `95.28 ms` (`−95.17 %`), **but burns `1,125 ms` process CPU** (`+67 %` higher CPU than serial `M123`'s `671.88 ms` due to 12-thread Clipper2 cache/allocator contention — the exact failure mode that dropped `t89`). Adding `M4` proves **`828/828` (`benchy`) and `2,093/2,093` (`base`)** bbox-overlapping pairs mathematically disjoint (`0` Clipper2 fallbacks) and eliminates **`> 99 %` of BOTH wall and CPU**!
   - **Zero-tolerance adversarial proof (`10,918 / 10,918` verdicts exact, `0` false negatives, `0` false positives):** `M4` is a sound mathematical fast-reject based on homotopy invariance of `FillRule::NonZero` winding number (`i128` exact segment-intersection + `i128` winding number on one vertex per ring; **any** crossing, endpoint touch, collinear contact, containment, or coordinate-guard trigger falls back to exact `slicer_core::polygon_ops::intersection`). Exhaustively verified against raw Clipper2 across all **`8,186` real pairs** (`2,894` `benchy` + `5,292` `base`) plus **`2,732` synthetic adversarial cases** (`2,718` true overlaps down to `100 nm` / `1 unit` edge grazes, hole-edge crossings, and zero-crossing strict interior containments + `14` touching-only / hole-interior disjoints) with **`0` FN and `0` FP**.
4. **Gate clearance & route impact (`ESTIMATE ONLY`):** `M12345` clears the standing `t137` 4-cell planner gate (`>= 0.80 %` on all 4 supports-on cells, structurally inert `0 ms` on all 4 supports-off cells, `16/16` byte-identical output to `t175` with zero geometry exceptions and zero WIT/guest changes) at **`7.57×–9.69×` floor** on `t177` scale (`7.59×–8.68×` same-session probe scale): **`−6.21 %`** (`benchy-classic-on`), **`−6.06 %`** (`benchy-arachne-on`), **`−7.75 %`** (`base-classic-on`), and **`−6.97 %`** (`base-arachne-on`).
   - **Route to 4/8:** stacking `t185` (graduated from this screen) + `t182` (Phase-3 union) crosses `< 1.0x` cleanly on **both ordinary `base` supports-on cells** (`base-classic-on` `1.2206x → 0.9511x` ord / `0.9110x` acc; `base-arachne-on` `1.2012x → 0.9485x` ord / `0.9107x` acc).
   - **4-track disjoint stack (`t182 + t183 + t184 + t185`):** puts **6/8 cells `< 1.0x`** (`benchy-off` `0.8477x / 0.9184x`, `base-off` `0.6228x / 0.6546x`, `base-on` `0.8077x / 0.8005x`) and brings `benchy-arachne-on` to **`1.0651x`** and binding `benchy-classic-on` to **`1.2294x`**.
5. **Decision:** Graduate [`Exact SupportGeometry post-planner host-tail acceleration: print_plane_occupancy reuse, zero-clone aggregation, and exact i128 disjointness certificate`](../../issues/185-support-geometry-post-planner-host-tail-acceleration.md) as an AFK implementation + standing `t137` 4-cell gate ticket.

---

## 1. Qualification, Invariance, `t177` Reconciliation & PPO Equivalence

All **22/22** probe, control, qualification, and dump runs exit `0` and match [`evidence/t175-tiled-collision-carve/reference.json`](../t175-tiled-collision-carve/reference.json) SHA-256 and byte count identically; **6/6** `--instrument-stderr` qualification runs report `degraded = false`, `fatal = 0`, `non_fatal = 0`, `code_1200 = 0`; **2/2** supports-off controls (`benchy-classic-off`, `base-classic-off`) are structurally inert (`0` active entries, `0` overlap pairs, `0 ms` instrumented tail, `0.043 ms` probe tail); classic vs arachne counters are **100 % identical** on both `benchy` and `base`.

| Cell | `t177` tail (`%` stage / `%` cell) | Probe med tail (`%` stage / `%` wall) | Qual probe vs `--instrument-stderr` | Top-level / `T3` residual | `PPO` exact match |
| --- | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-on` | 215 ms (19.62 % / 6.23 %) | **212.91 ms** (20.10 % / 6.20 %) | 214.46 vs 215 ms (`Δ 0.54 ms`) | 0.012 / 0.010 ms | **`200/200`** (`0` missing) |
| `benchy-arachne-on` | 216 ms (20.00 % / 6.08 %) | **214.61 ms** (20.12 % / 6.09 %) | 213.07 vs 213 ms (`Δ 0.065 ms`) | 0.012 / 0.010 ms | *(identical counters)* |
| `base-classic-on` | 2,293 ms (17.47 % / 7.76 %) | **1,939.31 ms** (16.68 % / 6.95 %) | 1,936.10 vs 1,936 ms (`Δ 0.096 ms`) | 0.017 / 0.013 ms | **`432/432`** (`0` missing) |
| `base-arachne-on` | 2,017 ms (16.08 % / 6.98 %) | **1,946.49 ms** (16.72 % / 6.90 %) | 1,935.22 vs 1,935 ms (`Δ 0.221 ms`) | 0.018 / 0.013 ms | *(identical counters)* |

Module execution order in `PrePass::SupportGeometry` (confirmed on all 4 cells):
1. `com.core.traditional-support-planner`: `0` plan entries (`33.1–35 ms` `benchy`, `187–194 ms` `base`).
2. `com.core.tree-support-planner`: `200` (`benchy`) / `432` (`base`) plan entries — **last module in stage**.
3. Everything between `tree-support-planner::module_complete` and `PrePass::SupportGeometry::stage_complete` is the single-threaded (`1.0×`) host aggregation/validation tail below.

---

## 2. Sub-Bracket Anatomy of the Post-Planner Host Tail (Median of 3 Passes)

| Sub-bracket (`reduced/arithmetic.json::probe_attribution`) | `benchy-classic-on` | `benchy-arachne-on` | `base-classic-on` | `base-arachne-on` |
| --- | ---: | ---: | ---: | ---: |
| **Total post-planner host tail (`T1..T5`)** | **212.91 ms** (`100 %`) | **214.61 ms** (`100 %`) | **1,939.31 ms** (`100 %`) | **1,946.49 ms** (`100 %`) |
| **1. `t3e` `ExactZQueryService::prefetch_cross_sections`** | **85.67 ms (`40.24 %`)** | **85.70 ms (`39.93 %`)** | **764.83 ms (`39.44 %`)** | **761.21 ms (`39.11 %`)** |
| └─ `t3e3` `slice_mesh_ex` (serial `1.0×`, `200`/`432` planes) | 84.62 ms (39.74 %) | 84.76 ms (39.49 %) | 745.44 ms (38.44 %) | 747.06 ms (38.38 %) |
| └─ `t3e1` `object_geometry` (`world_mesh` + `bounds` + unused `top`) | 1.02 ms (0.48 %) | 0.97 ms (0.45 %) | 15.48 ms (0.80 %) | 14.76 ms (0.76 %) |
| **2. `t3f4` `validate_entry` → `overlaps_any` (serial Clipper2)** | **122.56 ms (`57.56 %`)** | **123.27 ms (`57.44 %`)** | **1,138.45 ms (`58.70 %`)** | **1,149.51 ms (`59.06 %`)** |
| └─ `t3f4a` **Strictly bbox-disjoint** pairs (`2,066` / `3,199`) | **61.49 ms (`28.88 %`)** | **61.90 ms (`28.84 %`)** | **462.57 ms (`23.85 %`)** | **464.83 ms (`23.88 %`)** |
| └─ `t3f4b` BBox touch-only pairs (`0` / `0`) | 0.00 ms (0.00 %) | 0.00 ms (0.00 %) | 0.00 ms (0.00 %) | 0.00 ms (0.00 %) |
| └─ `t3f4c` **BBox-overlapping** pairs (`828` / `2,093`, serial!) | **60.17 ms (`28.26 %`)** | **60.45 ms (`28.17 %`)** | **670.62 ms (`34.58 %`)** | **679.18 ms (`34.89 %`)** |
| **3. 3× redundant `SupportPlanIR` deep clones + drops** | **3.58 ms (`1.68 %`)** | **3.42 ms (`1.60 %`)** | **23.37 ms (`1.21 %`)** | **24.41 ms (`1.25 %`)** |
| **4. All remaining host steps combined** (`query`, `in_routing_cell`, `union_same_family`, `TerritoryClipper`, cross-family, `commit_support_plan`, `drop(exact_z)`) | **1.11 ms (`0.52 %`)** | **2.22 ms (`1.03 %`)** | **12.66 ms (`0.65 %`)** | **11.36 ms (`0.58 %`)** |

**Deterministic operand census (classic ≡ arachne):**
- **`benchy` supports-on:** `200` active entries (`206` roles, `930` body polys, `351,626` body verts), `200` distinct `anchor_z` planes (`538` occupancy polys, `233,791` occupancy verts; mesh `112,662` verts / `225,706` tris), **`2,894`** total `(body, occupancy)` intersection calls (`1,036,788` body verts + `1,263,734` occupancy verts fed into Clipper2; `2,066` strictly bbox-disjoint = `71.39 %`, `828` bbox-overlapping = `28.61 %`, `0` positive overlaps), `0` territory layers, `0` cross-family pairs, `0` diagnostics.
- **`base` supports-on:** `432` active entries (`517` roles, `2,778` body polys, `1,227,970` body verts), `432` distinct `anchor_z` planes (`781` occupancy polys, `1,337,951` occupancy verts; mesh `1,230,602` verts / `2,461,234` tris), **`5,292`** total `(body, occupancy)` intersection calls (`2,440,987` body verts + `8,995,592` occupancy verts fed into Clipper2; `3,199` strictly bbox-disjoint = `60.45 %`, `2,093` bbox-overlapping = `39.55 %`, `0` positive overlaps), `0` territory layers, `0` cross-family pairs, `0` diagnostics.

---

## 3. Standalone 12-Thread Exact IR Replay & Candidate Mechanism Screen

Standalone Rust replay harness (`.local-artifacts/perf-vs-orca/t181-host-tail-replay/`, `mimalloc`, `RAYON_NUM_THREADS=12`, linking clean unmodified `slicer-ir` + `slicer-core`, 5 interleaved rounds per fixture). `M0_baseline` reproduces live `classic-on` probe tail at **`0.9902×` (`210.83 vs 212.91 ms`, `benchy`)** and **`1.0167×` (`1,971.68 vs 1,939.31 ms`, `base`)** and matches the live `SupportPlanIR` + diagnostic count **bit-for-bit** across all `7/7` modes.

### Mechanism definitions
- **`M1_ppo_reuse`:** Pre-seed exact-Z occupancy map `(object_id, z_units) -> &[ExPolygon]` from `SupportAnalysisIR::print_plane_occupancy` joined via `SliceIR`'s `mm_to_units(slice.z)` (verifying object existence & non-empty mesh in `MeshIR`), using an occupancy-only lookup in `validate_entry` so `world_mesh`, `top`, `blockers`, `plate`, and `slice_mesh_ex` are skipped on seeded hits while preserving automatic lazy fallback to `slice_mesh_ex` / `cross_section_at_z` for any unseeded plane (e.g., independent support Z layers or tests without `SupportAnalysisIR`).
- **`M2_bbox_cull`:** Exact full-`ExPolygon` (contour + holes) strict bounding-box reject (`ax1 <= bx0 || bx1 <= ax0 || ay1 <= by0 || by1 <= ay0`) before calling Clipper2.
- **`M3_zero_clone_cleanup`:** `Arc::unwrap_or_clone` in `crates/slicer-runtime/src/prepass.rs`, move `plans`/`entries` cleanly through `support_aggregation.rs` without `plans.clone()` or `entry.clone()`, delete dead `identities` `HashMap`, construct `family_to_plan`/`body_to_plan` lazily only when diagnostics are non-empty, use `HashMap` in `union_same_family_entries`, and skip $O(N^2)$ cross-family scan when all entries on an `(object_id, layer)` share one family.
- **`M4_exact_cert`:** Exact `i128` topological disjointness certificate (`ChunkedPoly`, 16-edge ring chunks + exact `i128` segment-intersection/touch test + exact `i128` `FillRule::NonZero` winding number on one vertex per ring of $A$ and $B$). **Sound fast-reject only:** if any edge intersects or touches, any winding number is nonzero, or any coordinate exceeds `10^15`, falls back to exact `slicer_core::polygon_ops::intersection(&[a], &[b])`.
- **`M5_parallel_validate`:** Order-preserving Rayon `par_iter()` across admitted active entries in `try_aggregate_support_plans_with_policy` (preserving serial ownership/decline admission and deterministic output/diagnostic fold order).

### Replay timing table (`reduced/replay_benchy.json`, `reduced/replay_base.json`)

| Mode | Exact | `benchy` Wall (Cut / Speedup) | `benchy` CPU* | `benchy` Clipper calls | `base` Wall (Cut / Speedup) | `base` CPU* | `base` Clipper calls |
| --- | :---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `M0_baseline` | YES | **210.83 ms** (0.0 % / 1.00×) | 218.75 ms | 2,894 | **1,971.68 ms** (0.0 % / 1.00×) | 1,968.75 ms | 5,292 |
| `M1_ppo_reuse` | YES | 123.44 ms (−41.45 % / 1.71×) | 125.00 ms | 2,894 | 1,144.79 ms (−41.94 % / 1.72×) | 1,140.63 ms | 5,292 |
| `M2_bbox_cull` | YES | 148.77 ms (−29.44 % / 1.42×) | 156.25 ms | 828 | 1,498.06 ms (−24.02 % / 1.32×) | 1,500.00 ms | 2,093 |
| `M123_ppo_bbox_zeroclone` | YES | 60.21 ms (−71.44 % / 3.50×) | 62.50 ms | 828 | 675.22 ms (−65.75 % / 2.92×) | 671.88 ms | 2,093 |
| `M1235_ppo_bbox_zeroclone_par` | YES | 9.06 ms (−95.70 % / 23.28×) | 15.63 ms | 828 | 95.28 ms (−95.17 % / 20.69×) | **1,125.00 ms** (+67 % vs M123!) | 2,093 |
| **`M1234_ppo_bbox_zeroclone_cert`** | **YES** | **3.38 ms (−98.40 % / 62.45×)** | `< 15.6 ms` (~3.4 ms) | **0** (`828` cert) | **15.03 ms (−99.24 % / 131.23×)** | **15.63 ms (−99.21 %)** | **0** (`2,093` cert) |
| **`M12345_full_exact_bundle`** | **YES** | **0.71 ms (−99.66 % / 295.7×)** | `< 15.6 ms` | **0** (`828` cert) | **2.95 ms (−99.85 % / 667.9×)** | `< 15.6 ms` | **0** (`2,093` cert) |

*\*CPU clock note: Windows `GetProcessTimes` quantizes at a `15.625 ms` (64 Hz) timer tick, so single-digit-millisecond medians read `0.0` or `15.625 ms`; for single-threaded `M1234`, CPU equals wall (`~3.4 ms` `benchy`, `~15.0 ms` `base`). Notice `M12345` round 5 on `base` logged one `187.5 ms` tick artifact when Rayon's 12 workers briefly spun before parking; on a live `pnp_cli` run the Rayon pool is already hot across prepass/per-layer, and the implementation ticket (`t185`) can compare `M1234` vs `M12345` directly if any Rayon spin noise appears on `benchy` — the wall gap between them is only `2.7 ms` (`benchy`) / `12.1 ms` (`base`), while `M12345` also shields models with real Clipper2 fallbacks.*

### Zero-Tolerance Adversarial Falsification Suite (`10,918 / 10,918` Exact)
Every pair was checked by comparing `cert_plus_clipper_overlaps(a, b)` directly against exact `raw_clipper_overlaps(a, b)` (`slicer_core::polygon_ops::intersection` + `area > 0.0`):
- **Real production pairs:** `2,894/2,894` (`benchy`) + `5,292/5,292` (`base`) = **`8,186/8,186` exact** (`5,265` bbox-disjoint, `2,921` cert-disjoint, `0` fallbacks, `0` mismatches).
- **Synthetic adversarial pairs (`1,366` per fixture = `2,732` total):** `16` handcrafted boundary/topology edge cases (`1 unit` = `100 nm` edge overlap, `1 unit²` corner overlap, shared-edge zero-area touch, single-vertex touch, zero-crossing $A \subset \mathrm{int}(B)$ and $B \subset \mathrm{int}(A)$, $A$ strictly inside hole of $B$, $A$ touching hole edge, $A$ crossing hole edge by `1 unit`, $A$ surrounding hole of $B$ inside $B$'s solid ring, $B$ inside hole of $A$ vs crossing by `1 unit`, CW-reversed contour under NonZero winding, concave U-bay disjoint vs `1 unit` graze, sub-unit grazing triangle) PLUS `1,350` real-polygon shift perturbations (`15` real bodies × `15` real holed occupancies × `6` shifts `{-10, -1, 0, +1, +10, +500}` units) → **`2,718` true overlaps + `14` true disjoints, `0` false negatives, `0` false positives!**

---

## 4. Standing `t137` 4-Cell Gate Clearance & `ESTIMATE ONLY` Route Context

All ratios below are **`ESTIMATE ONLY`** (`t174 × t175 × t177` for single-track and disjoint-stage stacks; never a ratio of record). Supports-off cells are 100 % structurally inert (`0 ms`, `0` entries).

### Single-track `t137` 4-cell planner gate (`M12345_full_exact_bundle`, `floor = 0.80 %`)

| Cell | Post-t175 est (`ord` / `acc`) | Tail cut (`t177` scale) | Floor mult (`t177` / probe) | Est. ratio after `t185` (`ord` / `acc`) | Share of `< 1.0x` cut |
| --- | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-on` | `1.5700x` / `1.6016x` | **−6.207 %** (−6.179 % probe) | **7.76×** / 7.72× | `1.4726x` / `1.5022x` | 17.10 % |
| `benchy-arachne-on` | `1.3885x` / `1.4091x` | **−6.058 %** (−6.069 % probe) | **7.57×** / 7.59× | `1.3044x` / `1.3237x` | 21.65 % |
| `base-classic-on` | `1.2206x` / `1.1692x` | **−7.752 %** (−6.940 % probe) | **9.69×** / 8.68× | `1.1260x` / `1.0786x` | **42.90 %** |
| `base-arachne-on` | `1.2012x` / `1.1533x` | **−6.974 %** (−6.890 % probe) | **8.72×** / 8.61× | `1.1174x` / `1.0729x` | **41.64 %** |

*(Even serial `M1234` alone clears all 4 cells at **`7.48×–9.63×`** `t177` scale / **`7.49×–8.62×`** probe scale!)*

### Stacked disjoint-stage 8-cell route (`ESTIMATE ONLY`: `t185` + `t182` + `t183` + `t184`)

| Cell | Post-t175 (`ord`) | `+t185` (host tail) | `+t185 +t182` (SupportGeom) | `+t185 +t182 +t183` | **All 4 (`+t184`) `ord` / `acc`** |
| --- | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | `1.1438x` | `1.1438x` | `1.1438x` | `1.0264x` | **`0.8477x` / `0.8285x`** (**WIN**) |
| `benchy-arachne-off` | `1.1548x` | `1.1548x` | `1.1548x` | `1.0934x` | **`0.9184x` / `0.8461x`** (**WIN**) |
| `base-classic-off` | `0.7687x` | `0.7687x` | `0.7687x` | `0.7495x` | **`0.6228x` / `0.6203x`** (**WIN**) |
| `base-arachne-off` | `0.7921x` | `0.7921x` | `0.7921x` | `0.7754x` | **`0.6546x` / `0.6395x`** (**WIN**) |
| `base-classic-on` | `1.2206x` | `1.1260x` | **`0.9511x`** (**WIN**) | **`0.8754x`** | **`0.8077x` / `0.7736x`** (**WIN**) |
| `base-arachne-on` | `1.2012x` | `1.1174x` | **`0.9485x`** (**WIN**) | **`0.8695x`** | **`0.8005x` / `0.7686x`** (**WIN**) |
| `benchy-arachne-on` | `1.3885x` | `1.3044x` | `1.2628x` | `1.1696x` | `1.0651x` / `1.0809x` (`6.1 %` left) |
| **`benchy-classic-on`** | **`1.5700x`** | `1.4726x` | `1.4238x` | `1.3513x` | **`1.2294x` / `1.2542x`** (`18.7 %` left) |

---

## 5. Methodological Disclosures

1. **Zero production code changes:** `probe.patch` was reverted cleanly (`git diff crates/` empty; `git apply --check probe.patch` exit `0`), clean release `pnp_cli.exe` was rebuilt from unmodified `HEAD`, and both guest freshness checks (`cargo xtask build-guests --check` and `--accelerated --check`) exited `0`.
2. **Same-window probe vs `t177` window:** Live probe `base-classic-on` ran slightly faster (`27.89 s` total, `1,939 ms` tail = `6.95 %` of wall) than `t177`'s window (`29.53 s` instrumented, `2,293 ms` tail = `7.76 %` of slice); both scales are reported side by side in `reduced/arithmetic.json` (`7.57×–9.69×` `t177` scale vs `7.59×–8.68×` probe scale).
3. **Standalone replay zero-clone accounting:** In production `crates/slicer-runtime/src/prepass.rs`, `PrepassStageOutput::SupportPlan(plan)` delivers a refcount-1 `Arc<SupportPlanIR>`, so `Arc::unwrap_or_clone` + moving entries performs zero polygon clones. In the replay benchmark loop (which re-runs the same immutable fixture 5 times), zero-clone modes borrow `&SupportPlanEntry` during sort/validation/merge-key checks (while a separate `verify_mode_exact` pass materializes the full `SupportPlanIR` and asserts bit-for-bit equality against the dumped live output).
4. **Postcard `ObjectMesh.world_z_extent` wire note:** `slicer_ir::ObjectMesh::world_z_extent` carries `#[serde(skip_deserializing, default)]` without `skip_serializing`, so postcard serializes the `Option<(f32, f32)>` bytes; the standalone replay harness decodes the exact wire bytes via a local `ObjectMeshWire` struct without touching `slicer-ir`.
5. **No accelerated timing arm & no Orca run:** Attribution + screen only; accelerated parity + paired ordinary/accelerated A/B belong to the graduated implementation ticket [`185`](../../issues/185-support-geometry-post-planner-host-tail-acceleration.md).

---

## 6. Verification

- `python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/verify.py` — **PASS** (`7/7` check groups green).
- `python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/verify.py --self-test` — **PASS** (`14/14` negative controls rejected with exact prefixes).
- `cargo xtask build-guests --check` — exit `0`; `cargo xtask build-guests --accelerated --check` — exit `0`.

See [`REPRODUCE.md`](REPRODUCE.md) for the exact command sequence.
