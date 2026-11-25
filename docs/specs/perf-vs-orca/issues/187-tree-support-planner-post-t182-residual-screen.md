# `PrePass::SupportGeometry` remainder (`com.core.tree-support-planner`) attribution and speedup screen

Type: task / attribution + mechanism screen (no production code edit)
Label: wayfinder:task
Status: resolved (2026-11-23; observation-only probe 16/16 + pure-guest exact screen 44/44 byte-identical to t182/reference.json; zero production edits; verify.py --self-test PASS 13/13; graduates 190)
Assignee: 2026-11-23 Antigravity wayfinder session — resolved (zero production edits).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t185 full-matrix attribution and 8/8 closure headroom map](186-post-t185-full-matrix-attribution.md)

## Question

**Inside the surviving `PrePass::SupportGeometry` stage (`825 / 831 ms` = `29.5 / 27.5 %` of `benchy` supports-on, `6,512 / 6,111 ms` = `33.0 / 30.4 %` of `base` supports-on — the ONLY single stage able to close binding `benchy-classic-on` alone at `→ 0.849x`), where do the remaining `785 / 791 ms` (`benchy`) and `6,296 / 5,884 ms` (`base`) of `com.core.tree-support-planner` go across guest CPU (`5.85 / 48.56 B` fuel, now #2 in supports-on fuel), `host:native` `offset` (`1.43 / 17.49 s`), and remaining `host:native` `clip_polygons` (`1.72 / 14.82 s`), and which exact (zero-drift) mechanism(s) clear the standing `t137` 4-cell planner gate?**

## Scope

- Substage attribution inside `modules/core-modules/tree-support-planner/src/lib.rs` (drop/influence loop, occupancy offset batching, phase-1 tiled carve, phase-2, phase-3 residual after `t182`, phase-4 simplify/emit, WIT marshal/decode, and `5.85 / 48.56 B` guest fuel hotspots).
- Offline/probe replay of exact (0-drift vs `t182/reference.json`) candidate mechanisms across `benchy` and `base` supports-on (re-checking classic ≡ arachne invariance).
- Zero production code edits landed in this ticket; graduate an implementation & standing-gate ticket for the winning exact bundle.

## Pre-declared gate

- Attribution & screen only (no production code edits); any temporary probe patch reverted before resolution, `cargo xtask build-guests --check` exit 0, and `100 %` byte-identical to `t182/reference.json`.
- Any graduated implementation candidate must be priced against `t137`'s standing 4-cell planner gate (`>= 0.80 %` paired wall saving with CPU corroborating in all 4 supports-on cells, structurally inert on supports-off).

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/` with `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit 0).
- Updated `## Answer`, `map.md`, and graduated implementation ticket (if a priced exact mechanism clears `t137`).

## Answer

**Attributed in full and screened to 100 % byte-identity (`16/16` probe + `44/44` screen runs reproduce `t182/reference.json`; `verify.py --self-test` PASS with `13/13` negative controls rejected; zero production edits landed; both `cargo xtask build-guests --check` and `--accelerated --check` exit `0`), graduating pure-guest exact acceleration [Ticket 190](190-tree-support-planner-exact-acceleration.md).**

### 1. Post-t185 `PrePass::SupportGeometry` attribution (`benchy-classic-on` / `base-classic-on`, 3-pass probe median)

- **Classic ≡ Arachne invariance:** `0` diffs across all 33 structural counters, identical `phase5_audit`, and bit-identical `run_support_geometry_total` fuel (`7,003,380,506` `benchy`, `56,124,660,340` `base`).
- **Wasted host dispatch (`H1`, separate host/scheduler finding — NOT bundled into pure-guest `t190`):** `com.core.traditional-support-planner` burns **`33.33 ms`** (`benchy`) / **`183.10 ms`** (`base`) projecting and WIT-lowering IR (`24.94 / 153.54 ms` inside `call_run`) before declining with `0` entries on tree-support jobs.
- **Inside `com.core.tree-support-planner` (`753.61 ms` / `5,973.65 ms` dispatch total; `713.38 / 5,712.82 ms` inside `run_support_geometry_with_analysis`):** splits ~50/50 between **host Clipper wall** (`364.61 ms` / `3,028.08 ms`, of which **`167.21 ms` / `1,172.66 ms` is 100 % serial `1.0×`**) and **guest CPU + WIT marshaling** (`348.76 ms` / `2,684.74 ms`, `5.852 B` / `48.565 B` clean deterministic fuel).
  1. **`tv_ensure_avoidance` (`139.13 / 980.21 ms`, 100 % serial):** `54.40 %` (`2,611 / 4,800`, `benchy`) and `56.56 %` (`10,661 / 18,848`, `base`) of materialized avoidance layers sit strictly above the highest queried layer for their bucket, burning `4,780 / 18,810` serial `offset_single` + `4,780 / 18,810` serial `clip_single(Union)` calls.
  2. **Pure guest hot loops (`5.852 / 48.565 B` clean fuel):** `point_in_any_expoly` (`12.09 / 13.78 %` probed fuel — per-ring `Vec<[f32; 2]>` heap alloc, zero AABB), `is_line_cut_by_contour` (`12.62 / 13.71 %` — zero AABB), `prim_mst` (`4.48 / 10.47 %` — `.sqrt()` in $O(V^2)$ loop), `skeleton_wall_counts` (`3.64 / 9.31 %` — $O(S \times N)$ `7.21M / 148.52M` inner iters), `projection_onto` (`6.96 / 8.08 %`), `swallowed_by_collision` (`9.02 / 6.73 %` — `2.2×` per active node, evaluated *after* building 100-gon `node_ellipse`), `node_roles_ancestor_walk` (`1.43M / 12.65M` steps even when `base_n == 0`), and `move_out_expolys` (`78.13 / 74.34 %` duplicate serial host dilations = `3.48 / 16.68 ms`).
  3. **`emit_prefetch_occupancy` (`87.65 / 1,016.58 ms`):** 19/37 tiny 24-request batches + $O(L^2)$ scans of `support_analysis`.
  4. **`tv_ensure_collision` (`73.34 / 299.01 ms`):** `7,440 / 24,304` layers materialized vs `3,442 / 12,360` distinct layers queried + `1,973 / 1,421` serial unions.
  5. **`contact_generation` (`31.15 / 218.08 ms`):** `188 / 1,395` **100 % serial** `contact_sample_erode::offset_single` calls (`26.25 / 191.46 ms`), plus `br_phase3_union` (`78.03 / 599.99 ms`, with `~23 / 143 ms` guest polygon-cloning/WIT marshal overhead).

### 2. Exact zero-drift prototype screen (`P1 + P2 + P3 + P4 + P5_zeroclone`, pure guest)

3-round interleaved ordinary A/B (`RAYON_NUM_THREADS=12`, unsandboxed, clean HEAD `pnp_cli.exe`, **`42` measured + `2` warmup = `44/44` byte-identical to `t182/reference.json`**, `12/12` pair wall wins, supports-off inert `2/2`):

- **`benchy-classic-on`:** **`+8.64 %`** wall (`2.7051 → 2.4714 s`, **`10.80×`** floor, `3/3`), **`+4.22 %`** CPU, `tree-support-planner` **`722 → 476 ms` (`−34.07 %`)**, fuel **`−38.85 %`**, `host:native` calls **`−24.83 %`**. Est. route (`ESTIMATE ONLY`): `1.2036x → ~1.0996x` ord (`~1.0932x` stage-delta).
- **`benchy-arachne-on`:** **`+9.12 %`** wall (`2.7697 → 2.5172 s`, **`11.40×`** floor, `3/3`), **`+4.47 %`** CPU, `tree-support-planner` **`723 → 471 ms` (`−34.85 %`)**, fuel **`−38.85 %`**, `host:native` calls **`−25.58 %`**. Est. route (`ESTIMATE ONLY`): **`1.0846x → ~0.9857x` ord (`~0.9835x` stage-delta — crosses `< 1.00x` → `7/8` estimated `< 1.0x`!)**.
- **`base-classic-on`:** **`+3.96 %`** wall (`20.8204 → 19.9969 s`, **`4.95×`** floor, `3/3`; stage-implied `+10.11 %`), **`+2.44 %`** CPU, `tree-support-planner` **`5,770 → 3,665 ms` (`−36.48 %`)**, fuel **`−41.03 %`**, `host:native` calls **`−23.86 %`**. Est. route (`ESTIMATE ONLY`): `0.8434x → ~0.8100x` ord (`~0.7581x` stage-delta).
- **`base-arachne-on`:** **`+10.98 %`** wall (`20.8262 → 18.5399 s`, **`13.72×`** floor, `3/3`; stage-implied `+9.39 %`), **`+2.24 %`** CPU, `tree-support-planner` **`5,753 → 3,752 ms` (`−34.78 %`)**, fuel **`−41.03 %`**, `host:native` calls **`−24.27 %`**. Est. route (`ESTIMATE ONLY`): `0.8334x → ~0.7419x` ord (`~0.7551x` stage-delta).

### 3. Falsified mechanism & graduation

- **`P5_tile` (REJECTED):** reusing `CollisionTileIndex` to window `build_roles_batched` Phase-5 set-difference clip operands matched `benchy-classic-on` (`a8c93535…`) but **diverged on `base-classic-on`** (`ea650ef8064a…` vs `7e115231d4b0…`) due to Clipper2 distant-scanbeam clamping (`vendor/clipper2-rust/src/engine.rs::add_new_intersect_node`). Excluded from the exact bundle.
- **Evidence:** [`evidence/t187-tree-support-planner-residual/FINDINGS.md`](../evidence/t187-tree-support-planner-residual/FINDINGS.md) (`verify.py --self-test` PASS, `13/13` negative controls rejected).
- **Graduated:** [Exact `com.core.tree-support-planner` acceleration: height-capped bucket-parallel `TreeVolumes`, zero-alloc guest geometry, and whole-object batched occupancy/contact offsets](190-tree-support-planner-exact-acceleration.md) (AFK, unblocked, standing `t137` 4-cell planner gate).
