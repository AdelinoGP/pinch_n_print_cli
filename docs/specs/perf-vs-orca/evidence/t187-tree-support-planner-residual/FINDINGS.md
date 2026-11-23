# Ticket 187 Findings — `PrePass::SupportGeometry` Remainder (`com.core.tree-support-planner`) Attribution & Exact Speedup Screen

- **Ticket:** [187 — `PrePass::SupportGeometry` remainder (`com.core.tree-support-planner`) attribution and speedup screen](../../issues/187-tree-support-planner-post-t182-residual-screen.md)
- **Date:** 2026-11-23
- **HEAD stamped:** `aba444ca21cf8a2cb50585c47ad1044068d32efc` (zero production code edits landed; `git diff -- crates/ modules/ vendor/ xtask/` empty; `cargo xtask build-guests --check` exit `0`; `cargo xtask build-guests --accelerated --check` exit `0`)
- **Active reference currency:** [`t182/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json) (`100 %` byte-identical across all `16/16` probe runs and all `42/42` measured + `2/2` warmup screen runs)
- **Verifier:** `python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/verify.py --self-test` (checkout-only, exit `0`, 13/13 negative controls rejected)

---

## Executive Verdict

1. **Where the post-t185 `PrePass::SupportGeometry` remainder goes (`benchy-classic-on` / `base-classic-on`, 3-pass probe median):**
   - **`com.core.traditional-support-planner` (wasted host dispatch, `H1`):** **`33.33 ms`** (`benchy`) / **`183.10 ms`** (`base`) spent projecting and WIT-lowering `MeshIR` / `RegionSegmentationIR` / `SupportAnalysisIR` into `traditional-support-planner` only for it to decline with `0` entries on tree-support jobs (`24.94 / 153.54 ms` inside `call_run` WIT lowering alone). Separate host/scheduler finding; **not** bundled into the pure-guest graduated candidate.
   - **`com.core.tree-support-planner` (`753.61 ms` `benchy` / `5,973.65 ms` `base` dispatch total; `713.38 / 5,712.82 ms` inside `run_support_geometry_with_analysis`):** splits nearly `50 / 50` between **host Clipper wall** (`364.61 ms` = `51.1 %` `benchy`; `3,028.08 ms` = `53.0 %` `base`, of which **`167.21 ms` / `1,172.66 ms` is 100 % serial `1.0×`**) and **guest CPU + WIT marshaling** (`348.76 ms` = `48.9 %` `benchy`; `2,684.74 ms` = `47.0 %` `base`, burning **`5.852 B` / `48.565 B`** clean deterministic Wasm instructions).
   - **Top five exact bottlenecks inside `modules/core-modules/tree-support-planner/src/lib.rs`:**
     1. **`TreeVolumes::ensure_avoidance` (`139.13 ms` / `980.21 ms`, `100 %` serial):** materializes all `240` / `496` layers across `20` / `38` radius buckets (`4,800` / `18,848` layers), even though **`54.40 %` (`2,611 / 4,800`)** on `benchy` and **`56.56 %` (`10,661 / 18,848`)** on `base` lie strictly above the highest layer ever queried for their bucket — burning `4,780` / `18,810` serial `offset_single` + `4,780` / `18,810` serial `clip_single(Union)` calls.
     2. **Pure guest hot loops (`5.852 B` / `48.565 B` clean fuel):** `point_in_any_expoly` (`12.09 %` / `13.78 %` probed fuel — fresh `Vec<[f32; 2]>` heap allocation per ring, zero AABB), `is_line_cut_by_contour` (`12.62 %` / `13.71 %` — zero edge/ring AABB), `prim_mst` (`4.48 %` / `10.47 %` — `.sqrt()` inside $O(V^2)$ inner loop), `skeleton_wall_counts` (`3.64 %` / `9.31 %` — $O(S \times N)$ `7.21M` / `148.52M` inner iterations), `projection_onto` (`6.96 %` / `8.08 %` — per-ring `Vec` allocation + `.sqrt()` per edge), `swallowed_by_collision` (`9.02 %` / `6.73 %` — called `2.2×` per active node and evaluated *after* building a 100-vertex `node_ellipse`), `node_roles_ancestor_walk` (`1.43M` / `12.65M` steps even when `base_n == 0`), and `move_out_expolys` (`78.13 %` / `74.34 %` duplicate serial host `offset_single` calls = `3.48 / 16.68 ms`).
     3. **`emit_prefetch_occupancy` (`87.65 ms` / `1,016.58 ms`):** `19` / `37` tiny 24-request chunks + $O(L^2)$ linear scans of `support_analysis.model_occupancy` and `print_plane_occupancy`.
     4. **`TreeVolumes::ensure_collision` (`73.34 ms` / `299.01 ms`):** `31` / `49` radius buckets (`7,440` / `24,304` layers materialized vs `3,442` / `12,360` distinct layers queried) + `1,973` / `1,421` serial `clip_single(Union)` calls.
     5. **`contact_generation` (`31.15 ms` / `218.08 ms`):** `188` / `1,395` **100 % serial** `contact_sample_erode::offset_single` calls (`26.25 / 191.46 ms`), plus `br_phase3_union` (`78.03 / 599.99 ms`, including `~23 / 143 ms` guest polygon-cloning/WIT marshaling overhead).
2. **Exact zero-drift screen result (`P1 + P2 + P3 + P4 + P5_zeroclone`, pure guest inside `modules/core-modules/tree-support-planner/src/lib.rs`, zero host/WIT edits):**
   - **Output identity:** **`42/42` measured runs (+ `2/2` warmups = `44/44`) `100 %` byte-identical to `t182/reference.json`**, including both supports-off inertness checks (`benchy-classic-off`, `base-classic-off`).
   - **3-round interleaved ordinary A/B (`RAYON_NUM_THREADS=12`, unsandboxed, clean HEAD `pnp_cli.exe`):** **`12/12` individual pair wall wins**, clearing `t137`'s `0.80 %` standing planner floor in **4/4 supports-on cells at `4.95×–13.72×` floor** with process CPU corroborating in **4/4 cells**:
     - `benchy-classic-on`: **`+8.64 %`** wall (`2.7051 → 2.4714 s`, `10.80×` floor, `3/3`), **`+4.22 %`** CPU, planner **`722 → 476 ms` (`−34.07 %`)**, fuel **`−38.85 %`**.
     - `benchy-arachne-on`: **`+9.12 %`** wall (`2.7697 → 2.5172 s`, `11.40×` floor, `3/3`), **`+4.47 %`** CPU, planner **`723 → 471 ms` (`−34.85 %`)**, fuel **`−38.85 %`**.
     - `base-classic-on`: **`+3.96 %`** wall (`20.8204 → 19.9969 s`, `4.95×` floor, `3/3`; stage-implied **`+10.11 %`**), **`+2.44 %`** CPU, planner **`5,770 → 3,665 ms` (`−36.48 %`)**, fuel **`−41.03 %`**.
     - `base-arachne-on`: **`+10.98 %`** wall (`20.8262 → 18.5399 s`, `13.72×` floor, `3/3`; stage-implied **`+9.39 %`**), **`+2.24 %`** CPU, planner **`5,753 → 3,752 ms` (`−34.78 %`)**, fuel **`−41.03 %`**.
   - **Cross-window route projection (`ESTIMATE ONLY`, `t185/reduced/route.json × screen`):** pushes **`benchy-arachne-on` across `< 1.00x`** (`1.0846x → ~0.9857x` A/B / `~0.9835x` stage-delta, reaching **`7/8` estimated `< 1.0x`**!) and cuts binding `benchy-classic-on` roughly in half of its remaining gap (`1.2036x → ~1.0996x` A/B / `~1.0932x` stage-delta), while deepening `base` supports-on to `~0.74x–0.81x`.
3. **Falsified mechanism (`P5_tile`):** Reusing `CollisionTileIndex` to window `build_roles_batched` Phase-5 set-difference clip operands was byte-identical on `benchy-classic-on` (`a8c93535…`) but **FALSIFIED as zero-drift on `base-classic-on`** (`ea650ef8064a…` vs `7e115231d4b0…`) due to Clipper2 distant-scanbeam clamping (`vendor/clipper2-rust/src/engine.rs::add_new_intersect_node`, same mechanism proven in `t175`). **Rejected** from the exact bundle.
4. **Graduated ticket:** [Ticket 190 — Exact `com.core.tree-support-planner` acceleration: height-capped bucket-parallel `TreeVolumes`, zero-alloc guest geometry, and whole-object batched occupancy/contact offsets](../../issues/190-tree-support-planner-exact-acceleration.md) (pure guest, zero WIT/host changes, zero geometry exceptions, standing `t137` 4-cell planner gate on ordinary + accelerated arms).

---

## 1. Classic ≡ Arachne Invariance

Per `reduced/arithmetic.json::classic_arachne_invariance`:

| Check | `benchy` (`classic-on` vs `arachne-on`) | `base` (`classic-on` vs `arachne-on`) |
| --- | --- | --- |
| Probe structural counters (33 counters) | **`0` diffs** | **`0` diffs** |
| Probe `phase5_audit` | **Identical** | **Identical** |
| Probe `run_support_geometry_total_fuel` | **`7,003,380,506 == 7,003,380,506`** (`0` diff) | **`56,124,660,340 == 56,124,660,340`** (`0` diff) |
| Clean `--profile` `tree-support-planner` fuel (baseline) | `5,851,954,889` vs `5,851,955,424` (`+535` = `0.000009 %`, WIT config-map parse) | `48,564,812,421` vs `48,564,812,457` (`+36` = `0.00000007 %`, WIT config-map parse) |
| Clean `--profile` `host:native` call reduction (baseline − candidate) | **`−10,333` == `−10,333`** (`0` diff) | **`−34,817` == `−34,817`** (`0` diff) |

Inside `modules/core-modules/tree-support-planner/src/lib.rs::run_support_geometry_with_analysis`, classic and arachne execute bit-for-bit identical planner instructions and host geometry requests.

---

## 2. Post-t185 `PrePass::SupportGeometry` Attribution (Observation-Only Probe)

Captured with `probe.patch` (`Cell<u64>` zero-overhead counters; `--profile`-gated fine `fuel::*` scopes), `16/16` runs exit `0` and byte-identical to `t182/reference.json`, `4/4` `--instrument-stderr` qual passes clean (`degraded = false`, `fatal = 0`, `non_fatal = 0`, `code_1200 = 0`).

### 2.1 Stage Dispatch Split (`reduced/probe_wall.json`, 3-pass median)

| Component | `benchy-classic-on` (ms) | `base-classic-on` (ms) | Notes |
| --- | ---: | ---: | --- |
| `com.core.traditional-support-planner` dispatch (`H1`) | **33.33** | **183.10** | Declines with `0` entries on tree supports (`24.94` / `153.54 ms` in WIT lowering `call_run` + `8.40` / `29.56 ms` view projection). Separate host/scheduler opportunity; not in `t190`. |
| `com.core.tree-support-planner` dispatch total | **753.61** | **5,973.65** | `run_support_geometry_total`: `713.38` / `5,712.82 ms`; WIT input lowering est.: `33.45` / `234.46 ms`; view projection + instantiate: `6.06` / `23.89 ms`; harvest: `0.72` / `2.48 ms`. |
| — Host Clipper ops wall (`batch_wall_ns` sum) | **364.61** | **3,028.08** | Serial (`*_single`): **`167.21`** / **`1,172.66 ms`** (`1.0×`); Batch (`*_batch`): `197.40` / `1,855.42 ms`. Accumulated worker thread-time (NOT wall): `1,624.63` / `18,039.62 ms`. |
| — Guest CPU + WIT marshal residual | **348.76** | **2,684.74** | Driven by `5.852 B` / `48.565 B` clean Wasm instructions. |

### 2.2 Substage & Host-Op Breakdown Inside `run_support_geometry_with_analysis`

| Substage / Host Site | `benchy` wall (ms) | `base` wall (ms) | Calls (`benchy` / `base`) | Key Structural Finding |
| --- | ---: | ---: | --- | --- |
| `tv_ensure_avoidance` | **139.13** | **980.21** | `20` / `38` buckets | **100 % serial (`1.0×`)**; `4,780` / `18,810` `offset_single` (`62.95` / `428.45 ms`) + `4,780` / `18,810` `clip_single(Union)` (`50.54` / `418.29 ms`). **`54.40 %` / `56.56 %` of materialized layers lie strictly above `max_layer` ever queried.** |
| `emit_prefetch_occupancy` | **87.65** | **1,016.58** | `19` / `37` batches | Tiny 24-request chunks (`450` / `868` reqs; host batch wall `73.36` / `951.80 ms`, accumulated worker thread-time `418.05` / `8,055.39 ms`) + $O(L^2)$ occupancy scans. |
| `drop_move_pass` (exc. `tv_ensure_*`) | **76.32** | **634.39** | `225` / `434` layers | Pure guest CPU (`point_in_any_expoly`, `projection_onto`, `is_line_cut_by_contour`) + `move_out_expolys` (`78.13 %` / `74.34 %` duplicate host `offset_single` calls = `3.48` / `16.68 ms`). |
| `br_phase3_union` | **78.03** | **599.99** | `18` / `37` batches | Host batch wall `55.05` / `457.46 ms` (accumulated worker `508.37` / `4,540.90 ms`); remaining `22.98` / `142.53 ms` is guest `head.to_vec()` + `tail.to_vec()` cloning and WIT marshaling. |
| `tv_ensure_collision` | **73.34** | **299.01** | `31` / `49` buckets | `7,440` / `24,304` layers materialized vs `3,442` / `12,360` distinct layers queried; `1,973` / `1,421` serial `clip_single(Union)` calls (`12.98` / `16.47 ms`). |
| `emit_node_roles` | **26.88** | **314.71** | `225` / `434` layers | `1,432,292` / `12,650,919` ancestor-walk steps (even when `base_n == 0`) + `point_in_any_expoly` without AABB. |
| `drop_group_and_mst` | **16.78** | **295.16** | `225` / `434` layers | `prim_mst` `.sqrt()` in $O(V^2)$ loop + `assign_node_group` `point_in_any_expoly` allocations. |
| `br_phase5_set_diff` | **35.07** | **217.07** | `18` / `37` batches | Host batch wall `25.83` / `176.45 ms`. Phase-5 audit proves non-empty sub-micron deltas (`area_changed = 47/206` `benchy`, `312/517` `base`, max `4.72e-5` / `3.27e-5 mm²`, `0` emptied); cannot be skipped or tiled (`P5_tile` falsified). |
| `contact_generation` | **31.15** | **218.08** | `1` / `1` | `188` / `1,395` **100 % serial** `contact_sample_erode::offset_single` (`26.25` / `191.46 ms`) + linear scan of `seg_origins`. |
| `br_phase1_batch` + `br_phase1_guest` | **49.36** | **351.23** | `18` / `37` batches | `CollisionTileIndex` queries (`10,600` / `68,381`), host carve batch (`12.23` / `86.45 ms`), guest region bbox/winding. |
| `emit_draw_ellipses` (exc.) + `emit_origin_and_skeleton` + `emit_mst_and_tips` + `emit_layer_setup` | **52.92** | **402.22** | `225` / `434` layers | `swallowed_by_collision` called `2.2×` per active node (`63.4k` / `357.7k`) after building 100-gon `node_ellipse`; `skeleton_wall_counts` $O(S \times N)$ `7.21M` / `148.52M` inner iters. |
| `tree_volumes_new` (`tv_simplify_union` + `tv_outlines_below`) | **19.53** | **167.80** | `1` / `1` | `240` / `495` serial `tv_simplify_union` + `240` / `495` serial `tv_outlines_below` unions (`9.16` / `86.75 ms` host). |
| `smooth_nodes` | **10.43** | **72.27** | `1` / `1` | Repeated `Vec` reallocation + `Vec<String>` `demand_ids` cloning per smoothing pass. |

### 2.3 Top Fine Guest-Fuel Scopes (`reduced/probe_fuel.json`, `--profile`)

*(Probed totals: `7,003,380,506` `benchy`, `56,124,660,340` `base`; clean unprobed totals in §3.)*

| Fine Scope (`fuel::*`) | `benchy` calls | `benchy` % probed | `base` calls | `base` % probed | Targeted by |
| --- | ---: | ---: | ---: | ---: | --- |
| `fuel::point_in_any_expoly` | 187,257 | 12.09 % | 1,083,864 | 13.78 % | `P2` (`IndexedPolySet` AABB + zero-alloc ring test) |
| `fuel::is_line_cut_by_contour` | 72,801 | 12.62 % | 426,174 | 13.71 % | `P2` (`LayerOutlineEdgeIndex` `f64` ring/edge AABBs) |
| `fuel::prim_mst` | 312 | 4.48 % | 868 | 10.47 % | `P2` (squared-distance $O(V^2)$ loop, single `.sqrt()` at commit) |
| `fuel::skeleton_wall_counts` | 200 | 3.64 % | 432 | 9.31 % | `P2` ($O(1)$ `HashSet` active-node lookup) |
| `fuel::projection_onto` | 56,291 | 6.96 % | 327,166 | 8.08 % | `P2` (zero-alloc `closest_point_on_ring_sq` + AABB lower bound) |
| `fuel::swallowed_by_collision` | 63,357 | 9.02 % | 357,736 | 6.73 % | `P2` (per-node `swallowed_cache` checked *before* `node_ellipse`) |
| `fuel::node_roles_is_floor` | 27,799 | 4.96 % | 162,507 | 5.44 % | `P2` (`IndexedPolySet` floor lookup) |
| `fuel::expolygons_simplify` | 7,680 | 4.97 % | 24,800 | 4.43 % | `P1` (eliminates unused collision layers above queried cells) |
| `fuel::node_ellipse` | 27,342 | 5.59 % | 159,575 | 4.06 % | `P2` (skipped when `swallowed_cache` is true) |
| `fuel::node_roles_ancestor_walk` | 27,799 | 0.98 % | 162,507 | 0.93 % | `P2` (short-circuit when `base_n == 0`) |

---

## 3. Exact Zero-Drift Prototype Screen (`P1 + P2 + P3 + P4 + P5_zeroclone`)

All five exact mechanisms live strictly inside `modules/core-modules/tree-support-planner/src/lib.rs` (zero host, WIT, or Clipper2 edits):

- **`P1` (Height-capped + bucket-parallel `TreeVolumes`):** Exact upper bound `max_layer` per radius bucket derived from `contacts_by_layer` + `prefetch_drop_ladders` (single-batch collision offsets across all needed `(r, layer)` cells + bucket-parallel layer-by-layer avoidance via `offset_polygons_batch` and `clip_polygons_batch`, strictly guarding union batching to `<= 8` polygons so `t182`'s `> 8` spatial-union threshold never triggers!) + `prefetch_collision_cells` before `emit_pass` (only exact queried `(r, layer)` cells) + guarded batched `tv_simplify_union` + lazy fallback.
- **`P2` (Exact zero-alloc guest geometry & $O(1)$ lookups):** `IndexedPolySet` (precomputed per-ring `f32` AABBs, zero-alloc `point_in_ring_pts` & `closest_point_on_ring_sq`), `LayerOutlineEdgeIndex` (`f64` ring/edge AABBs in `LineCutCache`), $O(1)$ `HashSet` `skeleton.wall_counts`, `base_n`-gated `node_roles` ancestor walk, per-node `swallowed_cache` evaluated before `node_ellipse`, squared-distance `prim_mst` inner loop, `TreeVolumes::move_out_cache` dilation memo, and zero-alloc `assign_node_group_indexed`.
- **`P3` (Whole-object batched occupancy prefetch + $O(1)$ layer maps):** Build `model_occ_by_layer`, `print_occ_by_layer`, and `seg_origins_by_layer` once per object; batch all active layers' occupancy offsets in 256-request chunks instead of 24-request mini-batches.
- **`P4` (Batched contact erosion + $O(1)$ segmentation set):** `seg_origin_set` + single `offset_polygons_batch` across all eligible candidates' overhang erodes in `contact_generation`.
- **`P5_zeroclone` (Move instead of clone in `build_roles_batched`):** `std::mem::take` across Phases 3/5/6/7 and moving the final origin's vectors into `RoleBuildInput` instead of cloning.

### 3.1 Measured Screen Results (`reduced/screen.json` & `reduced/arithmetic.json`)

Clean HEAD `pnp_cli.exe` (`16136737…`), ordinary arm, `RAYON_NUM_THREADS=12`, unsandboxed, `--no-default-module-paths`, 3 interleaved A/B pairs + 1 `--instrument-stderr` + 1 `--profile` per arm per cell (`42` measured + `2` warmup = **`44/44` byte-identical to `t182/reference.json`**):

| Cell | A/B Wall Med (s) | Wall Cut | Floor × (`0.80%`) | Pair Wins | A/B CPU Cut | `tree-support-planner` (ms) | `SupportGeometry` (ms) | Clean Planner Fuel | `host:native` Calls |
| --- | ---: | ---: | ---: | :---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-on` | `2.7051 → 2.4714` | **`+8.64 %`** | **`10.80×`** | `3/3` | **`+4.22 %`** | `722 → 476` (**`−34.07 %`**) | `762 → 514` (`−32.55 %`) | `5.852 → 3.579 B` (**`−38.85 %`**) | `41,609 → 31,276` (`−24.83 %`) |
| `benchy-arachne-on` | `2.7697 → 2.5172` | **`+9.12 %`** | **`11.40×`** | `3/3` | **`+4.47 %`** | `723 → 471` (**`−34.85 %`**) | `767 → 509` (`−33.64 %`) | `5.852 → 3.579 B` (**`−38.85 %`**) | `40,396 → 30,063` (`−25.58 %`) |
| `base-classic-on` | `20.8204 → 19.9969` | **`+3.96 %`** | **`4.95×`** | `3/3` | **`+2.44 %`** | `5,770 → 3,665` (**`−36.48 %`**) | `5,983 → 3,879` (`−35.17 %`) | `48.565 → 28.636 B` (**`−41.03 %`**) | `145,921 → 111,104` (`−23.86 %`) |
| `base-arachne-on` | `20.8262 → 18.5399` | **`+10.98 %`** | **`13.72×`** | `3/3` | **`+2.24 %`** | `5,753 → 3,752` (**`−34.78 %`**) | `5,958 → 4,003` (`−32.81 %`) | `48.565 → 28.636 B` (**`−41.03 %`**) | `143,484 → 108,667` (`−24.27 %`) |

- **Supports-off inertness:** `benchy-classic-off` (`4c67ef0b…`) and `base-classic-off` (`02ffa2d6…`) both matched `t182/reference.json` byte-for-byte; `enable_support: false` returns immediately at the top of `run_support_geometry_with_analysis`, qualifying cleanly for `t137`'s standing 4-cell planner gate.
- **`host:native` accumulated thread-time note (`--profile`, NOT wall):** `benchy-classic-on` `3.127 → 2.984 s` (`−4.57 %`), `benchy-arachne-on` `3.098 → 2.949 s` (`−4.79 %`), `base-classic-on` `30.760 → 30.344 s` (`−1.35 %`), `base-arachne-on` `29.041 → 29.358 s` (`+1.09 %` — minor parallel-cache-contention jitter while wall fell `−2.286 s` and process CPU fell `−2.24 %`). `host:support_analysis` call counts were 100 % unchanged in all 4 cells.
- **Note on `base-classic-on` A/B variance:** rounds 1–3 saw `20.98 → 20.17 s` (`+3.87 %`), `20.82 → 20.00 s` (`+3.96 %`), and `19.94 → 17.83 s` (`+10.56 %`), while `PrePass::SupportGeometry` dropped cleanly by `−2,104 ms` (`stage_implied_wall_saved_pct = 10.11 %`), matching `base-arachne-on`'s `+10.98 %` wall / `−1,955 ms` stage delta. The full 5-pair + null + accelerated campaign in graduated Ticket `190` will settle the exact pooled median cleanly.

### 3.2 Route Projection (`ESTIMATE ONLY`, Cross-Window)

Composing `docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/reduced/route.json` (`est_post_t185_ratio_ordinary`) with the t187 screen savings (**`ESTIMATE ONLY` — not a scoreboard ratio of record**):

| Cell | Est. Post-t185 (ord) | Est. Post-t190 from A/B Med | Est. Post-t190 from Stage Delta | `< 1.00x`? |
| --- | ---: | ---: | ---: | :---: |
| `benchy-classic-on` | `1.2036x` | **`~1.0996x`** | `~1.0932x` | No (`~9.1 %` cut left) |
| `benchy-arachne-on` | `1.0846x` | **`~0.9857x`** | `~0.9835x` | **YES (crosses `< 1.00x`!)** |
| `base-classic-on` | `0.8434x` | **`~0.8100x`** | `~0.7581x` | YES |
| `base-arachne-on` | `0.8334x` | **`~0.7419x`** | `~0.7551x` | YES |
| 4 supports-off cells (inert) | `0.6263x–0.8802x` | unchanged | unchanged | **4/4 YES** |
| **Estimated cells `< 1.00x`** | `6/8` | **`7/8`** | **`7/8`** | — |

---

## 4. Falsified & Deferred Mechanisms

1. **`P5_tile` (REJECTED — NOT ZERO-DRIFT):** Reusing `CollisionTileIndex` to window `build_roles_batched` Phase-5 set-difference clip polygons by each role polygon's bbox reproduced `benchy-classic-on` (`a8c93535629eff18…`) byte-for-byte, but **diverged on `base-classic-on`** (`ea650ef8064a…`, exact SHA recorded in `reduced/screen.json::falsified_candidates`) vs `t182/reference.json` (`7e115231d4b0…`) due to Clipper2 distant-scanbeam clamping (`vendor/clipper2-rust/src/engine.rs::add_new_intersect_node`). Rejected from the exact bundle.
2. **`H1` (`com.core.traditional-support-planner` wasted dispatch — SEPARATE HOST/SCHEDULER CANDIDATE):** `33.33 ms` (`benchy`) / `183.10 ms` (`base`) spent projecting and WIT-lowering IR into `traditional-support-planner` before it declines with `0` entries on tree-support jobs (`~1.2 %` `benchy-on` / `~0.9 %` `base-on`). Kept strictly out of pure-guest `t190` so `t190` touches zero host/WIT/scheduler code; can be evaluated alongside `t188`/`t189` or as a tiny host dispatch pre-filter if needed to close the final `~9 %` on `benchy-classic-on`.
3. **`P2k` (Optional zero-drift polish for `t190`):** Eliminating per-pass `Vec` reallocations (`std::mem::swap`) and `Vec<String>` `demand_ids` cloning (`Rc<Vec<String>>`) in `smooth_nodes` (`10.43 / 72.27 ms`, `437M / 2.9B` fuel) is trivially exact and recommended as an incremental zero-drift addition inside `t190`.

---

## 5. Graduation

Graduates [Ticket 190 — Exact `com.core.tree-support-planner` acceleration: height-capped bucket-parallel `TreeVolumes`, zero-alloc guest geometry, and whole-object batched occupancy/contact offsets](../../issues/190-tree-support-planner-exact-acceleration.md) (AFK, unblocked, standing `t137` 4-cell planner gate on ordinary + accelerated arms, `100 %` byte-identical to `t182/reference.json`, zero host/WIT changes, zero geometry exceptions).
