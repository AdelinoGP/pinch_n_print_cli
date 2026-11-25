# Ticket 190 — Exact `com.core.tree-support-planner` pure-guest acceleration

- **Date:** 2026-11-24 (UTC)
- **Base commit:** `36c096aa` (t185 landed; t186–t189 attribution & screen tickets resolved)
- **Active reference currency (unchanged, zero reference change):** t182's set,
  [`../t182-spatial-hierarchical-phase3-union/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json).
  Every t190 run fails closed against it (`0/264` hash mismatches).
- **Strict scope isolation:** All production and test edits stay strictly inside
  `modules/core-modules/tree-support-planner/src/lib.rs` (`git diff -- crates/ vendor/ xtask/`
  is empty; zero host, WIT, schema, or Clipper2 changes).
- **Guest freshness after the candidate build:** `cargo xtask build-guests --check`
  exit `0`; `cargo xtask build-guests --accelerated --check` exit `0`.
- **Arms (`reduced/output.json` manifest):**
  - `baseline-ordinary`: `pnp_cli` `e661baa5…`, non-planner modules tree `29d34082…`, `tree-support-planner.wasm` `cf917686…`
  - `candidate-ordinary`: `pnp_cli` `e661baa5…` (**identical**), non-planner modules tree `29d34082…` (**identical**), `tree-support-planner.wasm` `10a30e5e…` (**changed**)
  - `baseline-accelerated`: `pnp_cli` `a7260880…`, non-planner modules tree `8456dd0b…`, `tree-support-planner.wasm` `d977a7ea…`
  - `candidate-accelerated`: `pnp_cli` `a7260880…` (**identical**), non-planner modules tree `8456dd0b…` (**identical**), `tree-support-planner.wasm` `68f7669f…` (**changed**)
  - Both host executables (`exe_identical`) and all non-planner guest modules
    (`other_modules_identical`) are byte-identical between arms in both modes,
    so the A/B isolates `modules/core-modules/tree-support-planner/src/lib.rs` alone.
- **Verifier:** `verify.py` → `VERIFY PASS`; `verify.py --self-test` → `18/18` mutations rejected.

## 1. Verdict

**LANDED.** The standing `t137` 4-cell planner gate passes **4/4 supports-on cells**
on a single quiet window (5 interleaved pairs/cell ordinary, 3 null pairs/cell,
3 accelerated pairs/cell), clearing the `0.80 %` wall floor by **`9.67×`–`19.83×`**
and clearing even the strict any-pair null lens on **4/4 supports-on cells**, with
process CPU lower in 4/4 and accelerated corroborating 4/4. All 4 supports-off cells
are structurally inert (`INERT-PASS`, zero regression beyond `−0.80 %`). Zero output
drift, zero geometry exceptions, and zero gate waivers.

1. **Output parity (fail-closed).** All **264/264** measured runs exited `0` and
   reproduced `t182`'s active hashes byte for byte (`hash_mismatches = 0`):
   - `16` ordinary and `16` accelerated output runs;
   - `80` ordinary A/B, `48` null, `48` accelerated A/B, `48` instrumented stage,
     and `8` `--profile` runs.

   Every run had `0` non-zero exits, `0` degraded runs, `0` fatal or non-fatal
   errors, and `0` code-1200 diagnostics. `t182/reference.json` is unchanged.

2. **Standing `t137` 4-cell planner gate — supports-on (`5` interleaved pairs/cell ordinary, `3` null pairs/cell, `3` accelerated pairs/cell, `RAYON_NUM_THREADS=12`):**

   | cell | baseline / candidate median wall | wall saving | floor mult | wins | CPU saving | null med / max | accel wall / CPU | gate |
   |---|---:|---:|---:|---:|---:|---:|---:|---|
   | `benchy-classic-on` | 2.783 / 2.418 s | **+11.07 %** | 13.83× | 5/5 | +5.71 % (4/5) | 2.68 / 4.43 % | +16.32 % / +2.44 % | **PASS** |
   | `benchy-arachne-on` | 2.828 / 2.535 s | **+7.74 %** | 9.67× | 5/5 | +2.68 % (4/5) | 1.37 / 2.47 % | +8.56 % / +5.21 % | **PASS** |
   | `base-classic-on` | 21.921 / 19.369 s | **+8.84 %** | 11.05× | 5/5 | +2.24 % (4/5) | 1.91 / 4.73 % | +1.22 % / +0.93 % | **PASS** |
   | `base-arachne-on` | 22.908 / 19.298 s | **+15.86 %** | 19.83× | 5/5 | +3.17 % (5/5) | 1.24 / 3.24 % | +9.57 % / +1.51 % | **PASS** |

   - **Strict any-pair null lens (`_gate_strict_any_pair_null`):** **4/4 PASS** —
     every supports-on cell's median wall saving exceeds its largest single
     same-binary null pair (`+11.07 > 4.43 %`, `+7.74 > 2.47 %`, `+8.84 > 4.73 %`,
     `+15.86 > 3.24 %`).
   - **Pairwise wins:** **20/20** ordinary supports-on wall pairs won (`base-arachne-on`
     has strict non-overlapping separation on both wall and CPU); **11/12** accelerated
     supports-on wall pairs won (`benchy-classic-on`, `benchy-arachne-on`, and
     `base-arachne-on` all have `3/3` strict wall separation; `base-classic-on`
     won `2/3`, median `+1.22 %` wall / `+0.93 %` CPU).

3. **Supports-off (structurally inert, floor `−0.80 %`):**

   | cell | ordinary wall | wins | CPU | null med / max | accel wall / CPU | gate |
   |---|---:|---:|---:|---:|---:|---|
   | `benchy-classic-off` | +1.22 % | 4/5 | −1.66 % | 1.02 / 1.15 % | +8.60 % / −2.99 % | **INERT-PASS** |
   | `benchy-arachne-off` | +2.45 % | 5/5 | +0.34 % | 1.48 / 4.33 % | +0.91 % / +1.58 % | **INERT-PASS** |
   | `base-classic-off` | −0.48 % | 2/5 | +0.03 % | 6.64 / 15.06 % | −0.41 % / +0.61 % | **INERT-PASS** |
   | `base-arachne-off` | −0.80 % (−0.796 %) | 2/5 | −0.14 % | 4.68 / 5.51 % | −0.78 % / −1.54 % | **INERT-PASS** |

   Inside `com.core.tree-support-planner`, `enable_support: false` returns
   immediately before any t190 code runs (pinned by unit test
   `t190_enable_support_false_is_structurally_inert`). On `stage.json`, baseline
   vs candidate planner elapsed medians on supports-off cells are `18 → 17 ms`
   (`benchy-classic-off`), `17 → 17 ms` (`benchy-arachne-off`), `106 → 104 ms`
   (`base-classic-off`), and `106 → 103 ms` (`base-arachne-off`) — purely the
   shared host WIT IR marshalling floor.

4. **Instrumented planner module wall & deterministic `--profile` fuel + `host:native` calls (attribution only):**

   | cell | planner wall (3-pair med) | `SupportGeometry` stage | deterministic planner fuel | `host:native` calls (`Δ`) |
   |---|---:|---:|---:|---:|
   | `benchy-classic-on` | 723.0 → 490.0 ms (**−32.23 %**) | 763.0 → 530.0 ms (−30.54 %) | 5.852 → 3.575 B (**−38.91 %**) | 41,609 → 31,276 (**−10,333 / −24.83 %**) |
   | `benchy-arachne-on` | 733.0 → 481.0 ms (**−34.38 %**) | 775.0 → 520.0 ms (−32.90 %) | 5.852 → 3.575 B (**−38.91 %**) | 40,396 → 30,063 (**−10,333 / −25.58 %**) |
   | `base-classic-on` | 5,710.0 → 4,023.0 ms (**−29.54 %**) | 5,927.0 → 4,249.0 ms (−28.31 %) | 48.565 → 28.953 B (**−40.38 %**) | 145,921 → 111,104 (**−34,817 / −23.86 %**) |
   | `base-arachne-on` | 6,306.0 → 3,942.0 ms (**−37.49 %**) | 6,527.0 → 4,168.0 ms (−36.14 %) | 48.565 → 28.953 B (**−40.38 %**) | 143,484 → 108,667 (**−34,817 / −24.27 %**) |

   - **Classic ≡ Arachne invariance:** `host:native` call reduction is exact to the
     single call between classic and arachne (`−10,333` on `benchy`: `−3,280`
     `clip_polygons` + `−7,053` `offset`; `−34,817` on `base`: `−10,778`
     `clip_polygons` + `−24,039` `offset`), and planner fuel savings are identical
     to four significant figures (`−38.91 %` on `benchy`, `−40.38 %` on `base`).
   - Downstream `com.core.tree-support` fuel is flat (`1.500 B` `benchy`,
     `11.437–11.440 B` `base`), confirming the planner plan emitted to downstream
     stages is unchanged.

## 2. Route (ESTIMATE ONLY: `t174 × t175 × t182 × t183 × t184 × t185 × t190`; not a ratio of record)

`reduced/route.json` scales `t185`'s estimate by each supports-on cell's paired
t190 wall saving (and carries forward exact `t185` ratios on the four structurally
inert supports-off cells while disclosing their measured noise deltas separately):

| cell | ordinary post-t185 → post-t190 | accelerated post-t185 → post-t190 |
|---|---:|---:|
| `benchy-classic-on` | 1.2036x → **1.0704x** | 1.2221x → **1.0227x** |
| `benchy-arachne-on` | 1.0846x → **1.0007x** | 1.0759x → **0.9838x** |
| `base-classic-on` | 0.8434x → **0.7688x** | 0.7684x → **0.7590x** |
| `base-arachne-on` | 0.8334x → **0.7012x** | 0.7315x → **0.6615x** |

- `benchy-arachne-on` reaches virtual parity on ordinary (**`1.0007x`**) and crosses
  `< 1.00x` on accelerated (**`0.9838x`**, `7/8` estimated `< 1.0x` accelerated).
- Binding `benchy-classic-on` drops from `1.2036x` to **`1.0704x`** ordinary
  (`1.0227x` accelerated), leaving `~6.6 %` for the remaining two exact tracks
  (`t191` `PrePass::SupportAnalysis` and `t192` Zero-WIT `PostPass`).
- Official running tally remains **2/8 provisionally won** pending a fresh Orca
  scoreboard.

## 3. Disclosures

- **Box load and execution environment (`_meta`):** All `pnp_cli` campaign batches
  ran sequentially on a quiet host inside the standard command environment with
  `RAYON_NUM_THREADS=12` pinned inside `run_t190.ps1`. Instantaneous `Win32_Processor`
  load samples before/after each batch: `ab` `15.62 → 8.09 %`, `null` `7.59 → 5.09 %`,
  `accel` `5.29 → 13.35 %`, `stage` `27.53 → 4.56 %`. Because both arms ran
  interleaved in identical pairs within the same environment, paired wall/CPU
  ratios and deterministic fuel/call counts are directly comparable and reproduce
  `t187`'s unsandboxed screen numbers (`planner` `723/733 → 490/481 ms` vs `t187`'s
  `722/723 → 476/471 ms`; `fuel` `−38.91 / −40.38 %` vs `t187`'s `−38.85 / −41.03 %`;
  `host:native` calls `−10,333 / −34,817` bit-identical to `t187`).
- **Non-idempotent `f32` `ceil_radius` bug caught and fixed by output parity:**
  `TreeVolumes::ceil_radius(r) = (r / 0.2_f32).ceil() * 0.2_f32` is non-idempotent
  in IEEE-754 `f32` at values such as `r = 2.2 mm` (`k = 11`: `(11.0_f32 * 0.2_f32) / 0.2_f32 > 11.0`,
  so a second `ceil_radius` rounds up to `2.4 mm`, key `24000` instead of `22000`).
  Initial output verification caught a hash mismatch on `benchy-classic-on` caused
  by re-calling `ceil_radius` inside prefetch helpers. Introducing
  `prefetch_collision_cells_keyed` and `prefetch_drop_ladders_keyed` so `ceil_radius`
  runs strictly once at the entry boundary resolved the mismatch (`32/32` output +
  `264/264` campaign runs byte-identical) and is permanently guarded by
  `t190_tree_volumes_height_capped_prefetch_matches_serial_and_union_guard` using
  `r = 2.2` and `2.6 mm`.
- **Explicit non-goals honored:** `P5_tile` (falsified in `t187` as non-zero-drift
  due to Clipper2 distant-scanbeam clamping) and `H1` (`traditional-support-planner`
  host dispatch) were excluded.
- **Pre-existing test failure, unchanged by t190:**
  `cargo test -p tree-support-planner --test slicer_module_binding_tdd`
  (`binding_surface_matches_manifest`) fails identically on baseline commit
  `36c096aa` (`2.1.0#run` vs `2.0.0#run` in
  `modules/core-modules/tree-support-planner/tree-support-planner.toml`, untouched
  by t190). All 9 other test binaries in `tree-support-planner` (`160/160` tests)
  pass.

## 4. Production implementation

All edits live strictly in `modules/core-modules/tree-support-planner/src/lib.rs`:

- **`P1` — Height-capped, bucket-parallel `TreeVolumes`:**
  - `TreeVolumes::new` batches initial `tv_simplify_union` across all layers via
    `run_exact_union_batch`, which enforces `BATCH_UNION_EXACT_POLY_LIMIT = 8`
    so host `clip_polygons_batch` never triggers `t182`'s `> 8` `union_spatial_tree`
    path (inputs with `> 8` polygons stay on direct single-op `host::clip_polygons`).
  - `run_offset_batches` chunks offset work in 256-request batches.
  - Exact per-bucket `max_layer` upper bounds (`drop_bucket_max`) derived from
    `contacts_by_layer` cap avoidance ladders strictly to queried heights.
  - `TreeVolumes::prefetch_drop_ladders_keyed` batches all drop-pass collision
    offsets and advances the avoidance recurrence layer-by-layer in parallel
    across active radius buckets (calling `ceil_radius` strictly once per entry).
  - `TreeVolumes::prefetch_collision_cells_keyed` pre-materializes the exact
    `(key, r_ceil, cache_idx)` collision cells queried by active nodes before
    `emit_pass`, retaining cell-exact lazy fallback in `get_collision` /
    `get_avoidance`.
- **`P2` + `P2k` — Zero-alloc guest geometry, $O(1)$ lookups, and `smooth_nodes` ping-pong:**
  - `RingBboxF32` + `IndexedPolySet` (`contains`, `contains_poly_scaled`,
    `projection_onto`) with zero-alloc `point_in_ring_pts` and
    `closest_point_on_ring_sq` (exact squared-distance AABB screening followed
    by `.sqrt()` confirmation preserving bit-exact IEEE-754 tie-breaking).
  - `LayerOutlineEdgeIndex` (`f64` ring and edge AABBs lazily built inside
    `LineCutCache`) for `is_line_cut_by_contour`.
  - $O(1)$ `HashSet` (`extra_wall_xy` with `-0.0` normalized via `f32_eq_key`)
    replacing linear scans in `skeleton.wall_counts`.
  - `node_roles` short-circuit (`is_floor` / `is_roof` / `base_n == 0` /
    `below_roof > base_n`) using `outlines_indexed_at`.
  - Per-node `swallowed_cache` evaluated *before* constructing the 100-vertex
    `node_ellipse`.
  - Squared-distance inner loop in `prim_mst` (preserving exact tie-breaking).
  - `TreeVolumes::move_out_cache` (`move_out_cached`) eliminating redundant
    serial `move_out_expolys` host dilations.
  - Zero-alloc `assign_node_group_indexed`.
  - `P2k`: `PlannedSupportNode.demand_ids: Rc<Vec<String>>`; `smooth_nodes` flat
    `Vec<bool>` alive bitmap, hoisted scratch buffers, and `std::mem::swap`
    ping-pong across iterations.
- **`P3` — Whole-object batched occupancy prefetch & $O(1)$ layer maps:**
  - Pre-indexed `layer_idx_by_global`, `seg_origins_by_layer`,
    `model_occ_by_layer`, and `print_occ_by_layer` built once per object.
  - `prefetch_occupancy_memo_all` issues active-layer occupancy offsets in
    256-request chunks.
- **`P4` — Batched overhang contact erosion & $O(1)$ segmentation set:**
  - `HashSet` `seg_origin_set` lookup, `sample_contact_points_with_erodes`
    executing all eligible overhang erodes in one batched offset pass, and
    `collision(0.0)` contact-target prefetch.
- **`P5_zeroclone` — Move instead of clone in `build_roles_batched_owned`:**
  - `role_regions_owned` + `std::mem::take` across Phases 2/3/5/6/7 and moving
    the final eligible origin's polygon vectors in `plan_for_object`.

### Tests added (`modules/core-modules/tree-support-planner/src/lib.rs`)

- `t190_indexed_polyset_matches_reference_contains_projection_and_grouping`
- `t190_layer_outline_edge_index_matches_unindexed_contour_scan`
- `t190_tree_volumes_height_capped_prefetch_matches_serial_and_union_guard` (including non-idempotent `f32` radii `2.2` and `2.6 mm` + `<= 8` and `> 8` `run_exact_union_batch` coverage)
- `t190_prim_mst_squared_distance_matches_sqrt_reference_on_ties`
- `t190_enable_support_false_is_structurally_inert`

### Gates run

- `cargo test -p tree-support-planner --no-fail-fast`: `160/160` passing across
  9 test binaries (all 5 `t190_*` tests pass; only pre-existing
  `slicer_module_binding_tdd::binding_surface_matches_manifest` baseline failure).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (exit `0`).
- `cargo xtask check-literals`: `0` violations across 188 watched types (exit `0`).
- `cargo xtask check-test-quality --report`: `0` findings (exit `0`).
- `cargo xtask build-guests --check`: exit `0`.
- `cargo xtask build-guests --accelerated --check`: exit `0`.
- `python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/verify.py --self-test`: `18/18` rejected (exit `0`).
- `python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/verify.py`: `VERIFY PASS` (exit `0`).
