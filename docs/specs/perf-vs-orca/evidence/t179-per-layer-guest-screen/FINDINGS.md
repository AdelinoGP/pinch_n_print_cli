# Ticket 179 — Per-layer `tree-support` renderer and `infill-linker` guest-fuel attribution and candidate screen

- **Date:** 2026-11-21
- **Base commit:** `11a356b5` (last production code commit `5b138f5c`, post-t175; zero production edits landed in this take; observation-only diagnostic probe reverted cleanly and preserved as [`probe.patch`](probe.patch); all prototype code reverted cleanly)
- **Active reference currency:** `t175`'s active reference set ([`../t175-tiled-collision-carve/reference.json`](../t175-tiled-collision-carve/reference.json))
- **Guest freshness:** `cargo xtask build-guests --check` exit `0` and `cargo xtask build-guests --accelerated --check` exit `0` after probe & prototype revert
- **Stamped binary:** `target/release/pnp_cli.exe` (`f41f0d0efcc0b6fda095724bf4bd1fddd3c1fce7ab2470452d39be6c45f6a067`), `RAYON_NUM_THREADS=12`, unsandboxed
- **Verifier:** `python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/verify.py --self-test` → `VERIFY PASS` (`6/6` checks, `130/130` runs byte-identical to `t175`, `16/16` exact integer fuel folds, `14/14` targeted negative controls rejected)

---

## 1. Executive Summary & Premise Correction

1. **Major premise correction on `com.core.tree-support` (`t66` was DROPPED, never landed):** Ticket 179 opened assuming `[t64](../../issues/64-tree-support-renderer-attribution.md)` and `[t66](../../issues/66-paint-policy-dead-walk-reduction.md)` had already landed the `paint_policy_for` dead-walk fix. Re-checking `docs/specs/perf-vs-orca/issues/66-paint-policy-dead-walk-reduction.md` and `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for` proves `t66` was **DROPPED on 2026-10-14** (when `benchy` took ~25 s and `tree-support` was ~2 % of wall). Post-`t175`, `PaintRegionLayerView::paint_policy_for` still walks **723.5 M** ring edges on `benchy` supports-on and **6.76 B** ring edges on `base` supports-on across `0` painted annotations (`paint_has_ann = 0`, `paint_cover_hits = 0`), consuming **80.09 %** (`benchy`) and **80.27 %** (`base`) of all `com.core.tree-support` guest fuel.
2. **`com.core.tree-support` is 98.1–98.5 % reducible in exact guest fuel (byte-identical 8/8):** Three exact kernels account for **98.5 %** (`benchy`) and **98.1 %** (`base`) of renderer fuel:
   - `PaintRegionLayerView::paint_policy_for` (`crates/slicer-sdk/src/traits.rs`): **80.09 %** (`benchy`) / **80.27 %** (`base`) fuel (`71.6–76.7 %` of `--profile` scope wall, attribution only).
   - `skeleton_wall_count` + `point_in_expolygon` (`modules/core-modules/tree-support/src/lib.rs`): **15.56 %** (`benchy`) / **16.30 %** (`base`) fuel — rescans the contour bounding box `358.8 M` (`benchy`) / `3.43 B` (`base`) times even though **99.21 %** (`211,789 / 213,486`) and **99.06 %** (`1,811,210 / 1,828,426`) of skeleton points have `count <= best` and can never raise `best`.
   - Redundant in-guest `slicer_core::polygon_ops::union_ex` (`modules/core-modules/tree-support/src/lib.rs::TreeSupport::run_support`): **2.85 %** (`benchy`) / **1.51 %** (`base`) fuel (`1.19 B` / `5.87 B`).
   - Exact ablation **`TS-1`** (empty-paint guard + precomputed bbox / `count <= best` skip) cuts `tree-support` fuel by **95.61 %** (`22.8×`) on `benchy` and **96.55 %** (`29.0×`) on `base`. Adding safe redundant `role_union` bypass (**`TS-2`**) cuts fuel by **98.45 %** (`64.7×`, `41.94 B → 0.65 B`) on `benchy-classic-on` and **98.06 %** (`51.4×`, `388.79 B → 7.56 B`) on `base-classic-on`, **100 % byte-identical** to `t175`. The surviving host wall inside `tree-support` is real `host:native` `offset_polygons` (`render_walls` + `render_fill_offset`: `2,001` calls on `benchy`, `6,345` on `base`).
3. **`com.core.infill-linker` is 73.0–78.6 % reducible in exact guest fuel (byte-identical 8/8), and `L2` is 71 % `clip_batch_repair`:**
   - **49.6–52.9 %** of ALL `infill-linker` fuel sits in `PreparedPolylineClip::clip_batched` (`crates/slicer-core/src/polygon_ops.rs`) geometric re-attribution (`attributed_runs`, `16.2–32.0 %`) plus per-subject ambiguity repairs (`17.6–36.8 %`; `1,878–1,895` repairs on `benchy`, `3,820–4,002` on `base`). On `benchy` Layer `2` alone — which holds **34.08 %** (`classic`) / **32.51 %** (`arachne`) of the entire 240-layer slice's `infill-linker` fuel — **35** ambiguity repairs consume **70.83–71.25 %** of `L2`'s fuel (`2.49–2.58 B`).
   - **Exact mechanism (`IL-1`):** In `vendor/clipper2-rust/src/engine.rs`, open subjects never intersect each other (`intersect_edges` returns immediately when both edges are open), `check_join_left`/`check_join_right`/`add_trial_horz_join` skip open paths, and open `OutRec`s are born only in `start_open_path` and `add_local_min_poly`. Carrying a `source_id: u32` tag from `LocalMinima` onto open `OutRec`s (`add_open_subject_tagged` + `execute_open_tagged`) eliminates geometric attribution and ambiguity repairs completely — cutting `infill-linker` fuel by **49.73–53.16 %** (`2.0×–2.1×`) across all 8 cells, **byte-identical 8/8**.
   - **Exact companions (`IL-2` + `IL-3`):** Skipping dead `source_segments` (`union_groups == 0` in 8/8) + `O(1)` uniform-metadata fast path in `point_with_metadata` (`modules/core-modules/infill-linker/src/orchestrate.rs`, `8.5–21.7 %` fuel, `3.2–30.3 M` linear scans) and bbox/precomputed-edge `project_on_ring` + `O(E)` sorted-ring `nearest_pair_candidates` (`modules/core-modules/infill-linker/src/graph.rs` & `connect.rs`, `10.8–16.2 %` fuel, `4.5–11.5 M` edge projections, `1.94–1.97 M` on `L2`) bring combined **`IL-ALL`** fuel cuts to **72.96–73.47 %** (`3.70×–3.77×`) on `benchy` and **78.28–78.60 %** (`4.60×–4.67×`) on `base`, **byte-identical 8/8**.
4. **Critical discovery from the unsandboxed screen — why `TS-2` and `IL-ALL` MUST graduate together as one combined ticket (`t183`):**
   - **Standalone `TS-2` hits an `L2` straggler unmasking trap on binding `benchy-classic-on`:** cutting `tree-support` thread time by **81.2 %** (`2,927 → 550 ms`) accelerates the other 239 layers so much that `L2`'s unoptimized `infill-linker` (`227 ms`) snaps onto the critical path, **more than doubling** the 11-idle-core straggler tail (`50 ms → 110 ms` on `benchy-classic-on`, `1 ms → 69 ms` on `benchy-arachne-on`), dropping `cpu/wall` (`4.57× → 3.97×`) and capping standalone `TS-2`'s `benchy-classic-on` screen wall read (`−1.39 %` in 3 rounds despite `−11.94 %` CPU). Standalone `TS-2` is also structurally inert on supports-off.
   - **Standalone `IL-ALL` hits a structural sub-floor trap on `base-classic-on`:** `IL-ALL` destroys the `0.80 %` floor on all 4 supports-off cells (**`+10.26 %` / `+5.32 %` / `+2.50 %` / `+2.11 %`**, `3/3` wins each), collapsing `L2` `infill-linker` `186 → 42 ms` (`benchy-classic-off`) and `237 → 84 ms` (`benchy-arachne-off`). But on `27.9 s` `base-classic-on`, `infill-linker`'s entire `per_layer` wall reduction is only `146 ms` (**`0.52 % < 0.80 %`**, screen `−0.07 %` wall despite `−1.48 %` CPU), and on `benchy-classic-on` it is masked behind `tree-support`'s `2,745 ms` thread mass (screen `−0.40 %`).
   - **Combined `TS-2 + IL-ALL` (`arm_both` on supports-on, `arm_il` on supports-off) eliminates BOTH traps and clears the literal standing ALL-8-CELL `0.80 %` gate with ZERO waivers and ZERO geometry exceptions:**
     - `per_layer` wall drops **−44.56 %** (`772 → 428 ms`, `L2` tail `50 → 6 ms`) on `benchy-classic-on`, **−34.57 %** (`781 → 511 ms`) on `benchy-arachne-on`, **−44.34 %** (`4,028 → 2,242 ms`) on `base-classic-on`, **−41.14 %** (`4,560 → 2,684 ms`) on `base-arachne-on`, and **−29.09 % / −23.05 % / −6.98 % / −12.25 %** on supports-off.
     - Uninstrumented 3-round screen (attribution only, not a formal landing gate): **8/8 cells clear `0.80 %`** at **`2.64×–12.82×`** the floor (`+2.11 %` to `+10.26 %` wall), **`24/24` individual round wall wins** (`7/8` strict separation across rounds), and **`24/24` CPU wins (`8/8` strict separation, `+3.33 %` to `+13.75 %` CPU)**. Every one of **130/130** runs across probe, fuel ablation, and screen matched `t175/reference.json` byte-for-byte.
   - **Graduated ticket:** [Exact per-layer guest acceleration: `tree-support` renderer fast paths and `infill-linker` open-subject tagging](../../issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md) (AFK, unblocked, literal standing 8-cell `0.80 %` paired ordinary + accelerated gate, 8/8 byte-identical to `t175`).

---

## 2. Reconciliation Against `t177` (Post-`t175` Baseline)

All figures re-derived directly from [`../t177-post-t175-attribution/reduced/arithmetic.json`](../t177-post-t175-attribution/reduced/arithmetic.json) and [`../t177-post-t175-attribution/reduced/profile-detail-clean.json`](../t177-post-t175-attribution/reduced/profile-detail-clean.json) in [`reduced/arithmetic.json`](reduced/arithmetic.json).

| Cell | `t177` `per_layer` (% cell) | `t177` `L2`/last tail (% cell) | `tree-support` `t177` impl. wall % / fuel % | `infill-linker` `t177` impl. wall % / fuel % | Combined `TS+IL` impl. wall % / fuel % | `ESTIMATE ONLY` post-`t175` ord / acc |
|---|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | 550 ms (`32.35 %`) | 134 ms `L2` (`7.88 %`) | `0.28 %` / `0.43 %` | **`11.91 %` / `50.66 %`** | `12.19 %` / `51.09 %` | `1.144x` / `1.118x` |
| `benchy-classic-on` | 660 ms (`19.22 %`) | 1 ms `L2` (`0.03 %`) | **`9.89 %` / `61.01 %`** | **`3.59 %` / `14.94 %`** | **`13.48 %` / `75.95 %`** | `1.570x` / `1.602x` |
| `benchy-arachne-off` | 612 ms (`34.32 %`) | 104 ms `L2` (`5.83 %`) | `0.20 %` / `0.42 %` | **`9.78 %` / `54.24 %`** | `9.98 %` / `54.67 %` | `1.155x` / `1.064x` |
| `benchy-arachne-on` | 784 ms (`22.07 %`) | 4 ms `L57` (`0.11 %`) | **`9.50 %` / `60.82 %`** | **`3.64 %` / `16.11 %`** | **`13.14 %` / `76.94 %`** | `1.389x` / `1.409x` |
| `base-classic-off` | 2,085 ms (`21.28 %`) | 1 ms (`0.01 %`) | `0.48 %` / `0.29 %` | **`4.66 %` / `30.39 %`** | `5.14 %` / `30.68 %` | `0.769x` / `0.766x` |
| `base-classic-on` | 4,212 ms (`14.26 %`) | 14 ms (`0.05 %`) | **`8.34 %` / `68.95 %`** | `1.31 %` / `6.64 %` | **`9.66 %` / `75.59 %`** | `1.221x` / `1.169x` |
| `base-arachne-off` | 2,661 ms (`25.24 %`) | 0 ms (`0.00 %`) | `0.44 %` / `0.31 %` | **`4.31 %` / `35.86 %`** | `4.75 %` / `36.17 %` | `0.792x` / `0.774x` |
| `base-arachne-on` | 4,693 ms (`16.07 %`) | 4 ms (`0.01 %`) | **`8.21 %` / `70.28 %`** | `1.41 %` / `7.30 %` | **`9.63 %` / `77.58 %`** | `1.201x` / `1.153x` |

Together, `com.core.tree-support` + `com.core.infill-linker` own **75.59–77.58 %** of all guest fuel on the four supports-on cells (**9.63–13.48 %** implied cell wall) and **30.68–54.67 %** on the four supports-off cells (**4.75–12.19 %** implied cell wall).

---

## 3. `com.core.tree-support` Inner-Kernel Attribution

Observation-only probe ([`probe.patch`](probe.patch)): fuel overhead vs `t177` clean baseline is `+0.040 %` (`benchy`) / `−0.011 %` (`base`); `classic` and `arachne` counters are **100 % identical** on both `benchy` and `base` (`tree_support_invariance = {benchy: true, base: true}`); exact integer fuel conservation `unmarked_self_fuel + Σ scope.self_fuel == total_fuel` holds on all `8/8` cells. Scope wall percentages are parallel thread-wall under `--profile` (**attribution only, never a wall claim**).

| Kernel (crate-qualified symbol) | `benchy-classic-on` Fuel (%) / Scope Wall % | `base-classic-on` Fuel (%) / Scope Wall % | Structural Counter Evidence |
|---|---:|---:|---|
| **`PaintRegionLayerView::paint_policy_for`** (`crates/slicer-sdk/src/traits.rs`) | **33.60 B (`80.09 %`)** / `2,246 ms` (`71.63 %`) | **312.06 B (`80.27 %`)** / `21,297 ms` (`76.47 %`) | `paint_has_ann = 0`, `paint_cover_hits = 0`; walks **723,509,123** (`benchy`) / **6,756,284,530** (`base`) ring edges across `929` / `2,775` queries |
| **`skeleton_wall_count`** + `point_in_expolygon` (`modules/core-modules/tree-support/src/lib.rs`) | **6.53 B (`15.56 %`)** / `278.5 ms` (`8.88 %`) | **63.37 B (`16.30 %`)** / `2,936.5 ms` (`10.54 %`) | `358.8 M` / `3.43 B` contour bbox rescans; **`99.21 %`** (`211,789/213,486`) & **`99.06 %`** (`1,811,210/1,828,426`) of points have `count <= best` |
| **`role_union`** inclusive (`slicer_core::polygon_ops::union_ex` in `TreeSupport::run_support`) | **1.19 B (`2.85 %`)** / `129.1 ms` (`4.12 %`) | **5.87 B (`1.51 %`)** / `552.4 ms` (`1.98 %`) | `206` / `517` in-guest Clipper2 unions over `929` / `2,775` polygons already unioned by planner or `regularize_entry_roles` |
| **`regularize_entry_roles`** inclusive (`slicer_core::support_regularize::regularize_entry_roles`) | `0.17 B` (`0.41 %`) / `16.3 ms` (`0.52 %`) | `5.44 B` (`1.40 %`) / `464.9 ms` (`1.67 %`) | `regize_none = 194/200` (`97.0 %`, `benchy`), `350/432` (`81.0 %`, `base`) |
| **`render_walls` + `render_fill_offset`** (`TreeSupport::render_polygon_with_wall_count` host WIT offsets) | `0.08 B` (`0.19 %`) / **`450.5 ms` (`14.36 %`)** | `0.32 B` (`0.08 %`) / **`2,533.5 ms` (`9.10 %`)** | `1,072 + 929 = 2,001` (`benchy`) / `3,570 + 2,775 = 6,345` (`base`) `host:native` `offset_polygons` calls — invisible to WASM fuel, irreducible without offset changes |
| **`TreeSupport::scan_fill_region`** + `output_emit` + unmarked | `0.38 B` (`0.91 %`) / `14.9 ms` (`0.47 %`) | `1.70 B` (`0.44 %`) / `66.7 ms` (`0.24 %`) | Confirms `t64` on `base` too; hatching itself is negligible (`0.18–0.38 %` fuel) |

### Deterministic Fuel Ablation (`tree-support`, byte-identical to `t175`)

- **`TS-1`** (empty support-paint fast path in `PaintRegionLayerView::paint_policy_for` checking `SupportBlocker`/`SupportEnforcer` presence before `region_covers_expoly` + precomputed contour/hole bboxes & `if count <= best { continue; }` in `skeleton_wall_count`):
  - `benchy-classic-on`: `41.937 B → 1.840 B` (**−95.61 %**, `22.79×`); `benchy-arachne-on`: `41.937 B → 1.841 B` (**−95.61 %**, `22.78×`).
  - `base-classic-on`: `388.791 B → 13.420 B` (**−96.55 %**, `28.97×`); `base-arachne-on`: `388.790 B → 13.422 B` (**−96.55 %**, `28.97×`).
- **`TS-2`** (`TS-1` + skipping redundant `slicer_core::polygon_ops::union_ex` in `TreeSupport::run_support`):
  - `benchy-classic-on`: `41.937 B → 0.649 B` (**−98.45 %**, `64.66×`), `ref_match = True`.
  - `base-classic-on`: `388.791 B → 7.562 B` (**−98.06 %**, `51.41×`), `ref_match = True`.
  - **Production soundness invariant for `TS-2`:** `modules/core-modules/tree-support-planner/src/lib.rs` can merge multiple plan entries via `role.regions.extend(incoming_role.regions)`. Therefore `TreeSupport::run_support` must bypass `union_ex(&regions)` **only** when `regularized.is_some()` (already disjoint output of `regularize_entry_roles`) OR `regions.len() <= 1` OR the polygons in `regions` are **pairwise bounding-box disjoint** (fast exact check; mean `k ≈ 4.5–5.4` polygons per role). That preserves exact semantics on arbitrary overlapping or merged inputs while eliminating all redundant unions on canonical planner output.

---

## 4. `com.core.infill-linker` Inner-Kernel Attribution & `benchy` `L2` Breakdown

Observation-only probe ([`probe.patch`](probe.patch)): fuel overhead vs `t177` clean baseline is `+0.132 %` to `+0.219 %` across all 8 cells; exact integer fuel conservation holds on `8/8` cells. (`infill-linker` fuel is virtually identical between supports-off and supports-on for each `(model, generator)` pair.)

| Kernel (crate-qualified symbol) | `benchy-classic-off` Fuel (%) / Scope Wall | `benchy-arachne-off` Fuel (%) / Scope Wall | `base-classic-off` Fuel (%) / Scope Wall | `base-arachne-off` Fuel (%) / Scope Wall | **`benchy` `L2` (`classic-off`, `34.08 %` of slice!)** |
|---|---:|---:|---:|---:|---:|
| **`clip_batch_repair`** (`PreparedPolylineClip::clip_batched`, `crates/slicer-core/src/polygon_ops.rs`) | **3.64 B (`35.39 %`)** / `231.2 ms` | **4.09 B (`36.78 %`)** / `296.2 ms` | **6.61 B (`17.62 %`)** / `585.9 ms` | **7.68 B (`19.00 %`)** / `665.5 ms` | **2.49 B (`70.83 %`!)** / `117.0 ms` (`35` repairs!) |
| **`clip_batch_attr`** (`attributed_runs`, `crates/slicer-core/src/polygon_ops.rs`) | **1.70 B (`16.55 %`)** / `181.6 ms` | **1.80 B (`16.15 %`)** / `195.5 ms` | **11.99 B (`31.97 %`)** / `1,439.0 ms` | **12.49 B (`30.91 %`)** / `1,479.3 ms` | `26.0 M` (`0.74 %`) / `1.6 ms` |
| **`post_clip_meta`** (`link_paths_without_offset` + `point_with_metadata`, `modules/core-modules/infill-linker/src/orchestrate.rs`) | **0.91 B (`8.83 %`)** / `103.5 ms` | **0.95 B (`8.53 %`)** / `107.1 ms` | **8.15 B (`21.72 %`)** / `944.4 ms` | **8.42 B (`20.83 %`)** / `982.4 ms` | `1.8 M` (`0.05 %`) / `0.1 ms` |
| **`clip_batch_exec`** (single batched `Clipper64::execute`, `crates/slicer-core/src/polygon_ops.rs`) | **1.63 B (`15.79 %`)** / `157.9 ms` | **1.71 B (`15.33 %`)** / `180.8 ms` | **4.68 B (`12.48 %`)** / `516.9 ms` | **5.09 B (`12.59 %`)** / `529.0 ms` | `266.1 M` (`7.58 %`) / `12.5 ms` |
| **`connect_bpos_proj`** (`project_on_ring` / `BoundaryInfillGraph::project`, `connect.rs`/`graph.rs`) | **1.09 B (`10.62 %`)** / `78.4 ms` | **1.18 B (`10.63 %`)** / `90.1 ms` | **2.49 B (`6.65 %`)** / `243.3 ms` | **2.83 B (`7.01 %`)** / `247.2 ms` | **475.1 M (`13.54 %`!)** / `23.0 ms` (`1.94 M` edges!) |
| **`connect_pair_scan`** (`nearest_pair_candidates`, `connect.rs`) | `0.32 B` (`3.07 %`) / `30.7 ms` | `0.33 B` (`2.94 %`) / `32.0 ms` | `0.76 B` (`2.03 %`) / `85.3 ms` | `0.78 B` (`1.92 %`) / `92.9 ms` | `47.7 M` (`1.36 %`) / `2.3 ms` |
| **`prep_clip_new`** (`PreparedPolylineClip::new`, `polygon_ops.rs`) | `0.31 B` (`3.00 %`) / `32.4 ms` | `0.35 B` (`3.11 %`) / `47.7 ms` | `0.79 B` (`2.10 %`) / `95.1 ms` | `0.94 B` (`2.33 %`) / `134.6 ms` | `111.9 M` (`3.19 %`) / `5.8 ms` |
| **`connect_merge` + `bpos_hole` + `graph_new`** (`connect.rs`/`graph.rs`) | `0.26 B` (`2.54 %`) / `28.1 ms` | `0.29 B` (`2.59 %`) / `32.1 ms` | `0.80 B` (`2.12 %`) / `111.1 ms` | `0.91 B` (`2.26 %`) / `112.7 ms` | `78.2 M` (`2.23 %`) / `5.1 ms` |
| **`setup` + `offset_boundary` + rest** | `0.43 B` (`4.20 %`) / `97.3 ms` | `0.44 B` (`3.95 %`) / `103.9 ms` | `1.24 B` (`3.31 %`) / `262.1 ms` | `1.27 B` (`3.14 %`) / `318.5 ms` | `9.7 M` (`0.28 %`) / **`21.2 ms`** (host offset) |

### Root Causes & Exact Candidates (`IL-1`, `IL-2`, `IL-3`)

1. **`IL-1` — Native open-subject `source_id: u32` tagging in `vendor/clipper2-rust` (`49.7–53.2 %` fuel):**
   - `t65` batched open polyline clips into one `Clipper64` pass, then geometrically reverse-engineered which output segment came from which input polyline (`attributed_runs`) and re-ran individual `PreparedPolylineClip::clip` on every ambiguous subject (`1,895` / `1,878` on `benchy`, `3,820` / `4,002` on `base`). On `benchy` `L2` (12,583 boundary vertices across 26 rings), just **35** single-subject fallback clips burn `2.49 B` fuel (**70.83 %** of `L2`!).
   - **Exactness proof (`vendor/clipper2-rust/src/engine.rs` & `engine_fns.rs`):** open subjects never intersect each other (`intersect_edges` returns immediately when both edges are open), `check_join_left`/`check_join_right` and `add_trial_horz_join` skip open paths, `get_maxima_pair` matches disjoint per-polyline `vertex_top` arena indices, and open `OutRec`s are created strictly in `start_open_path` and `add_local_min_poly`. Carrying `source_id: u32` from `add_open_subject_tagged` → `LocalMinima` → `OutRec` → `execute_open_tagged` returns `Vec<(u32, Vec<Point64>)>` with exact source attribution directly out of the single batched sweep, eliminating `attributed_runs` and ambiguity repairs completely.
   - **Measured fuel cut (`IL-1` alone):** `−52.24 %` (`benchy-classic`), `−53.16 %` (`benchy-arachne`), `−49.73 %` (`base-classic`), `−50.04 %` (`base-arachne`), **8/8 byte-identical**.
2. **`IL-2` — Dead `source_segments` + `O(1)` uniform-metadata fast path (`8.5–21.7 %` fuel):**
   - `union_groups == 0` in all 8 cells, yet `link_paths_without_offset` (`modules/core-modules/infill-linker/src/orchestrate.rs`) builds `source_segments` (with `hypot`) that `link_region_group` discards (`_`).
   - `point_with_metadata` performs `3.22–3.36 M` (`benchy`) / `29.29–30.26 M` (`base`) linear scans over `path.points` to interpolate `(z, width, flow_factor, overhang_quartile, dist_to_top_mm, overhang_distance_mm)` even when every point on the source path has identical metadata. Checking uniformity once per path makes it `O(1)` exact.
3. **`IL-3` — Precomputed ring/edge geometry + bbox-pruned `project_on_ring` + `O(E)` sorted-ring `nearest_pair_candidates` (`10.8–16.2 %` fuel):**
   - `project_on_ring` (`connect.rs`) recomputes `dx.hypot(dy)` for every edge of every ring on every endpoint (`4.51–4.88 M` `benchy`, `10.13–11.51 M` `base`, `1.94–1.97 M` on `L2` alone) with no ring or edge bounding-box pruning; `nearest_pair_candidates` runs `O(E²)` (`1.14–2.26 M` pairs) over `endpoints` already sorted by `arc_position`. Precomputing cumulative arc lengths, edge squared lengths, and per-ring/edge bboxes + scanning adjacent sorted endpoints within `max_walk_distance` preserves exact output.
4. **Combined `IL-ALL` (`IL-1 + IL-2 + IL-3`) deterministic fuel cut:**
   - `benchy-classic`: `10.271 B → 2.778 B` (**−72.96 %**, `3.70×`); `benchy-arachne`: `11.108 B → 2.947 B` (**−73.47 %**, `3.77×`).
   - `base-classic`: `37.462 B → 8.019 B` (**−78.60 %**, `4.67×`); `base-arachne`: `40.355 B → 8.764 B` (**−78.28 %**, `4.60×`).
   - **8/8 byte-identical** to `t175/reference.json`.

### Residual `benchy` `L2` Analysis (Why No Standalone `SCHED-1` Ticket Graduates)

Under baseline `benchy-classic-off`, `L0 + L1` serialize ahead of `L2` on Rayon worker 0's left spine (`crates/slicer-runtime/src/layer_executor.rs::execute_per_layer_with_instrumentation_and_support_tools`, `global_layers.par_iter()` without `.with_max_len(1)`), pushing `L2` (`400 ms`) to finish at `+581 ms` with a `148 ms` idle tail (`111 ms` on `benchy-arachne-off`). Under **`IL-ALL`**, `L2` `infill-linker` collapses **`186 → 42 ms`** (`benchy-classic-off`) and **`237 → 84 ms`** (`benchy-arachne-off`):
- On `benchy-arachne-off`, the `L2` straggler tail **vanishes completely** (`111 ms (L2) → 5 ms (L115)`).
- On `benchy-classic-on` and `benchy-arachne-on` (`arm_both`), the `L2` tail is **`6 ms` and `5 ms`** (`L102`).
- On `benchy-classic-off`, the tail drops **`148 ms → 31 ms`** (`1.9 %` of cell). Inside `L2`'s surviving `256 ms`, `~159 ms` is host `region_partition` running 3× per layer (`Perimeters` + `fuzzy-skin` + `seam-placer`, already noted in `map.md` `Not yet specified` from `t60`). Because a standalone `with_max_len(1)` scheduler change has `<= 0.3 %` headroom on 7/8 cells once `IL-ALL` lands, **no standalone `SCHED-1` ticket is graduated**; `region_partition` 3× dedup remains the larger remaining `benchy` `per_layer` lever if needed for `<= 0.50x`.

---

## 5. Unsandboxed Attribution & Wall/CPU Screen (NOT a Formal Landing Gate)

Measured unsandboxed on the untouched production `pnp_cli.exe` (`f41f0d0e…`) against isolated scratch module dirs (`target/t179-arm-{ts1,ts2,il,both}`), **100 % `reference_match = True`** (`26/26` instrumented + `78/78` uninstrumented wall = `104/104` clean runs; `0` failures, `0` code-1200, `degraded = false`).

### 5.1 Part 1 — Instrumented Phase Wall, Thread Sums, and `L2` Straggler Unmasking

| Cell | Arm | `per_layer` ms (Δ %) | `ts_th` ms (Δ %) | `il_th` ms (Δ %) | `L2` `il` ms | Straggler tail (layer) |
|---|---|---:|---:|---:|---:|---:|
| `benchy-classic-off` | `baseline` → **`arm_il`** | `581 → 412` (**−29.1 %**) | `33 → 15` | `924 → 334` (**−63.9 %**) | `186 → 42` | `148 ms (L2) → 31 ms (L2)` |
| `benchy-arachne-off` | `baseline` → **`arm_il`** | `642 → 494` (**−23.1 %**) | `25 → 23` | `1,004 → 422` (**−58.0 %**) | `237 → 84` | `111 ms (L2) → 5 ms (L115)` |
| `benchy-classic-on` | `baseline` | `772` | `2,927` | `1,098` | `337` | `50 ms (L2)` |
| `benchy-classic-on` | `arm_ts1` | `593` (−23.2 %) | `655` (−77.6 %) | `910` | `219` | **`115 ms (L2)` (unmasked!)** |
| `benchy-classic-on` | `arm_ts2` | `595` (−22.9 %) | `550` (−81.2 %) | `926` | `227` | **`110 ms (L2)` (unmasked!)** |
| `benchy-classic-on` | `arm_il` | `620` (−19.7 %) | `2,745` | `346` (−68.5 %) | `62` | `1 ms (L85)` |
| `benchy-classic-on` | **`arm_both`** | **`428` (−44.6 %!)** | **`520` (−82.2 %)** | **`350` (−68.1 %)** | **`64`** | **`6 ms (L102)` (eliminated!)** |
| `benchy-arachne-on` | `baseline` | `781` | `2,849` | `1,146` | `344` | `1 ms (L235)` |
| `benchy-arachne-on` | `arm_ts2` | `634` (−18.8 %) | `533` (−81.3 %) | `1,016` | `252` | **`69 ms (L2)` (unmasked!)** |
| `benchy-arachne-on` | `arm_il` | `737` (−5.6 %) | `2,732` | `388` (−66.1 %) | `65` | `1 ms (L117)` |
| `benchy-arachne-on` | **`arm_both`** | **`511` (−34.6 %!)** | **`507` (−82.2 %)** | **`365` (−68.2 %)** | **`61`** | **`5 ms (L102)` (eliminated!)** |
| `base-classic-off` | `baseline` → **`arm_il`** | `1,948 → 1,812` (**−7.0 %**) | `369 → 359` | `3,669 → 1,169` (**−68.1 %**) | `1 → 1` | `2 → 4 ms` |
| `base-arachne-off` | `baseline` → **`arm_il`** | `2,539 → 2,228` (**−12.2 %**) | `433 → 354` | `4,073 → 1,199` (**−70.6 %**) | `1 → 1` | `1 → 2 ms` |
| `base-classic-on` | `baseline` | `4,028` | `23,937` | `3,774` | `1` | `3 ms` |
| `base-classic-on` | `arm_ts1` / `arm_ts2` | `2,524` / `2,499` (−38.0 %) | `4,361` / `3,852` (−83.9 %) | `3,925` / `3,934` | `1` | `2` / `17 ms` |
| `base-classic-on` | `arm_il` | `3,882` (−3.6 %, **`−146 ms`**) | `24,213` | `1,158` (−69.3 %) | `1` | `9 ms` |
| `base-classic-on` | **`arm_both`** | **`2,242` (−44.3 %!)** | **`3,764` (−84.3 %)** | **`1,244` (−67.0 %)** | `1` | `2 ms` |
| `base-arachne-on` | `baseline` → **`arm_both`** | `4,560 → 2,684` (**−41.1 %**) | `23,810 → 3,660` (**−84.6 %**) | `4,012 → 1,240` (**−69.1 %**) | `1` | `24 → 24 ms` |

### 5.2 Part 2 — Uninstrumented Interleaved Wall & CPU Screen (`1 warmup + 3 rounds`, screen only)

#### Combined `TS-2 + IL-ALL` (the graduated `t183` candidate: `arm_il` on supports-off where `TS-2` is structurally inert, `arm_both` on supports-on)

| Cell | Effective Arm | Base Wall (s) | Cand Wall (s) | Wall Saved (ms / %) | `0.80 %` Multiple | Wall Wins (Strict Sep) | CPU Saved (%) | CPU Wins (Strict Sep) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | `arm_il` | `1.8150` | `1.6289` | **`+186.2 ms` (`+10.26 %`)** | **12.82×** | `3/3` (`true`) | **`+4.58 %`** | `3/3` (`true`) |
| `benchy-classic-on` | `arm_both` | `3.4108` | `3.2533` | **`+157.5 ms` (`+4.62 %`)** | **5.77×** | `3/3` (`true`) | **`+11.63 %`** | `3/3` (`true`) |
| `benchy-arachne-off` | `arm_il` | `1.8667` | `1.7673` | **`+99.4 ms` (`+5.32 %`)** | **6.65×** | `3/3` (`true`) | **`+4.37 %`** | `3/3` (`true`) |
| `benchy-arachne-on` | `arm_both` | `3.5390` | `3.3016` | **`+237.4 ms` (`+6.71 %`)** | **8.39×** | `3/3` (`true`) | **`+13.75 %`** | `3/3` (`true`) |
| `base-classic-off` | `arm_il` | `10.6520` | `10.3857` | **`+266.3 ms` (`+2.50 %`)** | **3.12×** | `3/3` (`true`) | **`+3.36 %`** | `3/3` (`true`) |
| `base-classic-on` | `arm_both` | `27.9202` | `26.1890` | **`+1,731.3 ms` (`+6.20 %`)** | **7.75×** | `3/3` (`true`) | **`+11.45 %`** | `3/3` (`true`) |
| `base-arachne-off` | `arm_il` | `11.1680` | `10.9319` | **`+236.1 ms` (`+2.11 %`)** | **2.64×** | `3/3` (`false`* ) | **`+3.33 %`** | `3/3` (`true`) |
| `base-arachne-on` | `arm_both` | `28.5638` | `26.6830` | **`+1,880.8 ms` (`+6.58 %`)** | **8.22×** | `3/3` (`true`) | **`+10.88 %`** | `3/3` (`true`) |

*`base-arachne-off` won all 3 paired rounds (`10.932 vs 11.000`, `11.052 vs 11.346`, `10.910 vs 11.168`); Round 2 box drift made `max(cand)=11.052 > min(base)=11.000`.

#### Standalone Arm Disclosure (Why Neither Arm Graduates Alone)

- **Standalone `arm_ts2` on supports-on:** `benchy-arachne-on` `+4.88 %` wall / `+10.32 %` CPU (`3/3`), `base-classic-on` `+4.79 %` / `+10.85 %` (`3/3`), `base-arachne-on` `+5.69 %` / `+10.11 %` (`3/3`), **but `benchy-classic-on` `−1.39 %` wall (`2/3` wins) despite `+11.94 %` CPU (`3/3`)** because `arm_ts2` unmasks `L2`'s `110 ms` straggler tail (`c/w` drops `4.57× → 3.97×`), AND supports-off is 100 % inert. (`arm_ts1` on `base-classic-on`: `+4.90 %` wall / `+9.81 %` CPU; `arm_ts2` saves an additional `1.78 s` CPU on `base` and `0.44 s` CPU on `benchy`.)
- **Standalone `arm_il` on supports-on:** `benchy-arachne-on` `+2.32 %` wall / `+2.23 %` CPU (`3/3`), `base-arachne-on` `+1.50 %` / `+1.01 %` (`3/3`), **but `base-classic-on` `−0.07 %` wall (`2/3`, CPU `+1.48 %` `3/3`)** — structurally bounded by `146 ms / 27,920 ms = 0.52 % < 0.80 %` — and **`benchy-classic-on` `−0.40 %` wall (`1/3`, CPU `+0.30 %`)** while masked behind `tree-support`.

---

## 6. Route Impact (`ESTIMATE ONLY` — Cross-Window Composition, Never a Ratio of Record)

Composing `t177`'s post-`t175` baseline (`t174` quiet board × `t175` paired savings) with `TS-2 + IL-ALL` under BOTH the conservative `t177`-implied proportional thread ceiling and the 3-round screen median, plus stacking with [`t182`](../../issues/182-host-spatial-hierarchical-phase3-union.md) (`M9n_k8`, from `t178/reduced/arithmetic.json`) and [`t181`](../../issues/181-support-geometry-post-planner-host-tail.md) (`SupportGeometry` post-planner host tail ceiling, from `t177/reduced/arithmetic.json`):

| Cell | Post-`t175` est. ord / acc | `t177`-model cut → est. ord | Screen cut → est. ord / acc | Share of `<1.0x` / `<=0.50x` ord cut (screen) | Stacked `+ t182` est. ord / acc | Stacked `+ t182 + t181` (full) est. ord |
|---|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | `1.144x` / `1.118x` | `7.60 %` → `1.057x` | `10.26 %` → **`1.026x` / `1.003x`** | **`81.6 %`** / `18.2 %` | — | — |
| `benchy-classic-on` | `1.570x` / `1.602x` | `10.58 %` → `1.404x` | `4.62 %` → **`1.498x` / `1.528x`** | `12.7 %` / `6.8 %` | `1.449x` / `1.478x` | `1.351x` |
| `benchy-arachne-off` | `1.155x` / `1.064x` | `5.67 %` → `1.089x` | `5.32 %` → **`1.093x` / `1.007x`** | **`39.7 %`** / `9.4 %` | — | — |
| `benchy-arachne-on` | `1.389x` / `1.409x` | `10.29 %` → `1.246x` | `6.71 %` → **`1.295x` / `1.315x`** | `24.0 %` / `10.5 %` | `1.254x` / `1.272x` | `1.169x` |
| `base-classic-off` | `0.769x` / `0.766x` | `3.17 %` → `0.744x` | `2.50 %` → **`0.750x` / `0.747x`** | already `<1x` / `7.2 %` | — | — |
| `base-classic-on` | `1.221x` / `1.169x` | `7.91 %` → `1.124x` | `6.20 %` → **`1.145x` / `1.097x`** | **`34.3 %`** / `10.5 %` | **`0.970x` / `0.929x`** | **`0.875x`** |
| `base-arachne-off` | `0.792x` / `0.774x` | `3.04 %` → `0.768x` | `2.11 %` → **`0.775x` / `0.758x`** | already `<1x` / `5.7 %` | — | — |
| `base-arachne-on` | `1.201x` / `1.153x` | `7.93 %` → `1.106x` | `6.58 %` → **`1.122x` / `1.077x`** | **`39.3 %`** / `11.3 %` | **`0.953x` / `0.915x`** | **`0.869x`** |

**Route takeaways (`ESTIMATE ONLY`):**
- **4/8 cells (`base` all four):** `t182` (`M9n_k8`) + `t183` (`TS-2 + IL-ALL`) **cross `< 1.0x` on both ordinary `base` supports-on cells (`~0.970x` / `~0.953x`) even before `t181`**, and with `t181` reach `~0.875x` / `~0.869x` (approaching the `<= 0.85x` provisional band).
- **6/8 cells (`benchy` supports-off):** `t183` alone closes **81.6 %** (`benchy-classic-off`, `~1.026x` ord / `~1.003x` acc) and **39.7 %** (`benchy-arachne-off`, `~1.093x` ord / `~1.007x` acc) of the gap to Orca; any small `ShellClassification` cut from [`t180`](../../issues/180-shell-classification-post-t172-residual-screen.md) pushes both well under `< 0.90x`.
- **`<= 0.50x` follow-on effort:** `t183` removes `41–45 %` of `per_layer` wall on supports-on and `23–29 %` on `benchy` supports-off — delivering a substantial down payment (`6.8–18.2 %` of the `0.50x` cut across all 8 cells) with zero output change.

---

## 7. Disclosures & Methodological Limits

1. **No formal landing gate in this take:** Part 2 is a 3-round unsandboxed screen on the ordinary arm (`pnp-ordinary`) only, with no null-band control and no accelerated timing arm. The graduated ticket (`t183`) must pass the full standing 5-pair ordinary + 3-pair accelerated + null-band paired gate and `16/16` byte-identity against `t175/reference.json`.
2. **`--profile` scope wall & `--instrument-stderr` times are attribution only:** Never cited as uninstrumented wall claims.
3. **Cross-window route ratios are `ESTIMATE ONLY`:** Composed from `t174` × `t175` × `t179` (+ `t178`/`t177`), never a fresh three-tool scoreboard ratio of record.
4. **Painted-workload parity coverage required at landing:** Because the matched job has `paint_has_ann = 0`, `t183` must include explicit unit/integration tests verifying `PaintRegionLayerView::paint_policy_for` still honors `SupportBlocker`, `SupportEnforcer`, and blocker-over-enforcer precedence on painted regions.
