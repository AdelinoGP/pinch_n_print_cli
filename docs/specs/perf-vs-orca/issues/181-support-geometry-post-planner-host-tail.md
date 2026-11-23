# `SupportGeometry` post-planner host tail attribution and reduction screen

Type: task / attribution + candidate screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: resolved 2026-11-22 — attribution + candidate screen complete (`22/22` runs byte-identical to `t175/reference.json`, `6/6` `--instrument-stderr` qual clean, `2/2` supports-off inert, `632/632` `print_plane_occupancy` planes polygon-for-polygon identical, `10,918/10,918` zero-tolerance adversarial pairs `0` FN / `0` FP, `probe.patch` cleanly reverted, both guest freshness checks exit `0`, zero production edits); graduated 100 % exact `M12345` host bundle (`−99.66 %` `benchy` / `−99.85 %` `base` tail wall, clears standing `t137` 4-cell gate at `7.57×–9.69×` floor with zero output drift and zero tolerance weakening) as [Exact `SupportGeometry` post-planner host-tail acceleration](185-support-geometry-post-planner-host-tail-acceleration.md)
Assignee: agent (2026-11-22)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t175 full-matrix attribution and 0.50x headroom map](177-post-t175-full-matrix-attribution.md)
([`evidence/t177-post-t175-attribution/FINDINGS.md`](../evidence/t177-post-t175-attribution/FINDINGS.md)),
which discovered that **16.1–20.0 %** of `PrePass::SupportGeometry` — **`2,293 ms`
(7.8 % of cell wall)** on `base-classic-on`, **`2,017 ms` (7.0 %)** on
`base-arachne-on`, **`215 ms` (6.2 %)** on `benchy-classic-on`, and **`216 ms`
(6.1 %)** on `benchy-arachne-on` (median of 3 passes; `0 ms` on all four
supports-off cells) — elapses *after*
`com.core.tree-support-planner::module_complete` and *before*
`PrePass::SupportGeometry::stage_complete`. On `benchy` supports-on this host
tail is larger than Phase-3 union itself, and on `base` supports-on it closes
nearly half of the remaining `16.7–18.1 %` cut to `< 1.0x`.

## Question

**What executes on the host during the `2,017–2,293 ms` (`base` supports-on,
7.0–7.8 % of cell) and `215–216 ms` (`benchy` supports-on, 6.1–6.2 % of cell)
between `com.core.tree-support-planner::module_complete` and
`PrePass::SupportGeometry::stage_complete`, and what exact, output-preserving
candidate reduces it cleanly under the standing `t137` 4-cell planner gate
without weakening the zero-tolerance exact-Z occupancy tripwire?**

Concretely:

1. **Sub-bracket the post-planner tail** in `crates/slicer-runtime/src/prepass.rs`
   (after `com.core.tree-support-planner` returns) through
   `crates/slicer-runtime/src/blackboard.rs::Blackboard::commit_support_geometry`
   and `crates/slicer-runtime/src/builtins/support_analysis_producer.rs::validate_entry`
   on `base-classic-on` and `benchy-classic-on`:
   - WIT result decode / conversion into `SupportGeometryIR`,
   - `SupportGeometry` merge / validation / exact-Z occupancy gate
     (`validate_entry`, including whether it runs serially across bodies/layers
     or rebuilds spatial/polygon indexes per entry),
   - `blackboard.commit_support_geometry` storage/cloning/indexing,
   - temporary buffer / WASM store drop before `stage_complete`.
2. **Verify concurrency and operand structure:** measure whether the tail is
   single-threaded (`1.0x`) on 12 cores, how many support bodies / polygons /
   occupancy queries it processes, and whether any duplicate work (e.g., cloning
   large polygon sets, rebuilding per-layer occupancy indexes, or serial
   per-body validation) dominates.
3. **Screen and price concrete exact candidates:** for each candidate (e.g.,
   parallelizing independent per-layer/per-body occupancy validation with Rayon,
   spatial indexing / bbox culling inside exact validation without relaxing its
   zero-tolerance check, eliminating redundant clones or WIT-to-IR conversions,
   or deferring/avoiding unnecessary drops), state its ceiling in ms and % of
   cell wall across all four supports-on cells (`base` and `benchy`), confirm
   strict zero-tolerance preservation of `validate_entry` (0 code-1200,
   byte-identical 8/8 output), and check whether it clears the standing `t137`
   4-cell gate.
4. **Decide graduation:** graduate an implementation + standing-gate ticket if
   a verified exact candidate captures a material fraction of the `6.1–7.8 %`
   tail across all four supports-on cells (and note its contribution alongside
   [Planner Phase-3 batched union attribution and mechanism screen](178-planner-phase3-union-attribution.md)
   toward closing `base` supports-on to `4/8` and cutting binding
   `benchy-classic-on`).

## Scope

- Attribution and candidate pricing only; no production commit in this take.
  Revert any diagnostic probe cleanly and preserve it as `probe.patch` in the
  evidence directory.
- **Hard invariant (`map.md` occupancy-gate rule):** never weaken
  `validate_entry` to a tolerance; exact-Z occupancy validation must remain
  zero-tolerance and fire on any real plane mismatch.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Unmodified baseline runs reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- The sub-brackets sum to and reconcile with `t177`'s measured `2,293 ms`
  (`base-classic-on`) and `215 ms` (`benchy-classic-on`) post-planner tail.

## Acceptance

- Evidence directory
  `docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/` with
  `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used), and
  checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and graduated
  implementation ticket if a candidate clears the `t137` bar.

## Answer

**Resolved (2026-11-22) — attribution + candidate screen complete; zero production edits (`probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt, both guest freshness checks exit `0`). Graduated [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](185-support-geometry-post-planner-host-tail-acceleration.md).**

Evidence: [`evidence/t181-support-geometry-host-tail/FINDINGS.md`](../evidence/t181-support-geometry-host-tail/FINDINGS.md) (`verify.py` PASS `7/7` check groups; `verify.py --self-test` PASS `14/14` negative controls; checkout-only Python 3 stdlib).

### 1. Qualification, Premise Clarification & `t177` Reconciliation

- **22/22** live probe/control/qual/dump runs exit `0` and reproduce [`evidence/t175-tiled-collision-carve/reference.json`](../evidence/t175-tiled-collision-carve/reference.json) SHA-256 and byte count identically; **6/6** `--instrument-stderr` qualification runs report `degraded = false`, `fatal = 0`, `non_fatal = 0`, `code_1200 = 0`; **2/2** supports-off controls (`benchy-classic-off`, `base-classic-off`) are structurally inert (`0` active entries, `0` overlap pairs, `0 ms` instrumented tail, `0.043 ms` probe tail); classic ≡ arachne counters are **100 % identical** on both `benchy` and `base`.
- **Exact code location (premise clarification):** WIT result decode (`crates/slicer-wasm-host/src/dispatch.rs::harvest_support_plan_ir`) and WASM store drop occur *inside* `runner.run_stage`, strictly before `com.core.tree-support-planner::module_complete`. The entire post-planner host tail (`212.91 / 214.61 ms` `benchy-on`, `1,939.31 / 1,946.49 ms` `base-on`; reconciling within `0.065–0.54 ms` of `--instrument-stderr` and `<= 0.018 ms` additive bracket residual) executes **100 % single-threaded (`1.0×` on 12 cores)** in `crates/slicer-runtime/src/prepass.rs::execute_prepass_with_instrumentation_collecting` → `crates/slicer-wasm-host/src/support_aggregation.rs::aggregate_support_plan_irs_degrading_with_attributed_diagnostics` → `crates/slicer-wasm-host/src/exact_z_query.rs::ExactZQueryService` → `crates/slicer-runtime/src/blackboard.rs::Blackboard::commit_support_plan`.

### 2. Root-Cause Breakdown (`99.0–99.5 %` = Two Serial Bottlenecks + 3× Deep Clones)

1. **Duplicate single-threaded `slice_mesh_ex` re-slicing (`39.11–40.24 %` of tail: `85.67–85.70 ms` `benchy`, `761.21–764.83 ms` `base`):** `ExactZQueryService::prefetch_cross_sections` re-transforms the mesh, slices an unused `top` cross-section, and runs single-threaded `slicer_core::slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) over `200` (`benchy`) / `432` (`base`) print planes. Live polygon-for-polygon verification proves **`200/200` (`benchy`) and `432/432` (`base`) — `100.0 %`, `0` missing planes — are already computed identically in `PrePass::SupportAnalysis`** (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::collect_print_plane_occupancy`) and stored in `SupportAnalysisIR::print_plane_occupancy`.
2. **Unfiltered serial Clipper2 intersections in `overlaps_any` (`57.44–59.06 %` of tail: `122.56–123.27 ms` `benchy`, `1,138.45–1,149.51 ms` `base`):** `2,894` (`benchy`) / `5,292` (`base`) serial `slicer_core::polygon_ops::intersection` calls (`0` positive overlaps) with zero bbox filtering:
   - **`71.39 %` (`2,066`, `benchy`) / `60.45 %` (`3,199`, `base`)** have **strictly disjoint bounding boxes**, wasting `61.49–61.90 ms` (`benchy`) and `462.57–464.83 ms` (`base`).
   - **`28.61 %` (`828`, `benchy`) / `39.55 %` (`2,093`, `base`)** have overlapping bounding boxes (`60.17–60.45 ms` `benchy`, `670.62–679.18 ms` `base`) — support bodies sitting in concave bays or holes of model cross-sections where no edges intersect and no ring contains the other.
3. **Three redundant deep clones of `SupportPlanIR` + drops (`1.21–1.68 %` of tail: `3.42–3.58 ms` `benchy`, `23.37–24.41 ms` `base`):** `(*plan).clone()` in `prepass.rs`, `plans.clone()` in `aggregate_support_plan_irs_with_policy_attributed`, and `entry.clone()` in `try_aggregate_support_plans_with_policy`, plus an unread `identities` `HashMap`. Everything else combined (`commit_support_plan`, `union_same_family_entries`, `TerritoryClipper`, cross-family overlap, `drop(exact_z)`) is `< 1.0 %`.

### 3. Candidate Screen, Zero-Tolerance Proof & Route Impact (`ESTIMATE ONLY`)

Standalone 12-thread exact IR replay (`5` interleaved rounds, `M0_baseline` reproducing live `classic-on` probe at `0.9902×` `benchy` / `1.0167×` `base`, `7/7` modes bit-identical to live `SupportPlanIR`):

- **`M1234` (serial: `M1` PPO reuse with full unseeded fallback + `M2` bbox cull + `M3` zero-clone + `M4` exact `i128` topological disjointness certificate):** `210.83 → 3.38 ms` (**`−98.40 %`**, `62.5×`) on `benchy`; `1,971.68 → 15.03 ms` (**`−99.24 %`**, `131.2×`, process CPU `1,968.75 → 15.63 ms` = `−99.21 %`) on `base`; `0` Clipper2 calls (`828/828` and `2,093/2,093` proven disjoint).
- **`M12345` (`M1234` + `M5` order-preserving Rayon validation):** **`210.83 → 0.71 ms` (`−99.66 %`, `295.7×`)** on `benchy` and **`1,971.68 → 2.95 ms` (`−99.85 %`, `667.9×`)** on `base` (whereas `M1235` without `M4` burned `1,125 ms` CPU on `base`, reproducing `t89`'s parallel-Clipper CPU trap).
- **Zero-tolerance invariant preserved:** `M4` is a sound mathematical fast-reject (homotopy invariance of `FillRule::NonZero` winding number using exact `i128` arithmetic) that falls back to exact `slicer_core::polygon_ops::intersection` on any crossing, endpoint touch, collinear contact, containment, or coordinate-overflow guard. Exhaustively verified across **`10,918/10,918` pairs** (`8,186` real + `2,732` synthetic adversarial cases down to `1 unit = 100 nm` edge grazes and hole crossings) with **`0` false negatives and `0` false positives**.
- **Standing `t137` 4-cell gate & route (`ESTIMATE ONLY`):** clears `0.80 %` across **4/4 supports-on cells at `7.57×–9.69×`** `t177` scale (`7.59×–8.68×` probe scale): `benchy-classic-on` **`−6.21 %` (`1.5700x → 1.4726x`)**, `benchy-arachne-on` **`−6.06 %` (`1.3885x → 1.3044x`)**, `base-classic-on` **`−7.75 %` (`1.2206x → 1.1260x`)**, `base-arachne-on` **`−6.97 %` (`1.2012x → 1.1174x`)**. Stacking `t185 + t182` closes **both ordinary `base` supports-on cells `< 1.0x`** (`0.9511x / 0.9485x` ord, `0.9110x / 0.9107x` acc); the 4-track disjoint stack `t182 + t183 + t184 + t185` puts **6/8 cells `< 1.0x`** and brings `benchy-arachne-on` to `1.0651x` and binding `benchy-classic-on` to `1.2294x`.

Graduated: [Exact `SupportGeometry` post-planner host-tail acceleration: `print_plane_occupancy` reuse, zero-clone aggregation, and exact `i128` disjointness certificate](185-support-geometry-post-planner-host-tail-acceleration.md).
