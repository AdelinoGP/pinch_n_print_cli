# Tiled collision operand for the planner's phase-1 carve

Type: task / measured candidate (output-identical work-set change + standing
gate)
Label: wayfinder:task
Status: resolved
Assignee: 2026-11-21 wayfinder session — resolved (KEEP `tile-2mm` by explicit human ruling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t173 support-path boolean volume](173-support-geometry-boolean-volume.md)
([`evidence/t173-support-boolean-volume/FINDINGS.md`](../evidence/t173-support-boolean-volume/FINDINGS.md)),
which censused the support path's clipper work and priced the phase-1 bbox/tile
window as **60.8 % / 61.6 %** of the two base supports-on cells' required cuts.

## Question

**Does an exact tiled/windowed collision operand on
`build_roles_batched`'s phase-1 carve clear the standing gate — and what wall
does it actually save against t173's 60.8 % / 61.6 % ceiling?**

t173 measured the shape, not the implementation: phase 1 issues **68,381
calls**, each subtracting the whole layer collision set (**426.9 M clip
points**, mean **6,241/call**, 4.34 paths/call) from a **98.7-point** subject;
only **285,368** clip points (**0.0669 %**) lie inside the subject's bbox. A
polygon-level bbox filter is exact but removes only **28.5 %** of the operand.
The question is whether cutting the collision set into tiles once per layer,
then clipping each region against only the tiles its bbox touches, realizes
enough of that ceiling to matter.

## What is already known (t173)

- **Exactness is by construction.** For a difference, clip geometry whose bbox
  is disjoint from the subject bbox cannot change the result; a tile set whose
  union covers the subject bbox is therefore output-identical, with no parity
  exception. The tiling cost (partition per layer, boundary edges) is the
  unmeasured term.
- **The site is `planner::batched_phase1_carve`** in
  `modules/core-modules/tree-support-planner/src/lib.rs`
  (`build_roles_batched` phase 1 → `run_clip_batches`). 68,381 requests on both
  base supports-on cells; phase 3 (`batched_phase3_union`, 480 calls) is a
  separate shape and is **not** part of this ticket.
- **The ceiling leaves 1.150x / 1.120x.** The phase-1 window alone cannot win
  either base cell; it is a component. The standing gate is the ticket's bar,
  not the 8/8 destination.
- **The benchy supports-on pair is not this program.** Its whole planner clip
  wall is 0.73 / 0.72 s against cuts of 2.08 / 2.15 s; the tile shape covers
  only 23.2 % / 21.8 % of those cuts. Do not expect a benchy win here.

## Scope

1. Implement a per-layer tile index for the collision set used by
   `build_roles_batched` phase 1. Sweep a small number of tile sizes (e.g.
   1 / 2 / 5 / 10 mm) on `base classic-on`, choose by measured wall, and record
   the sweep. The per-region operand is the union of tiles overlapping the
   region bbox; the request order and every other phase stay unchanged.
2. Prove output identity on all eight cells against t172's active reference set
   (the tile union must cover the subject bbox exactly; no tolerance).
3. Run the standing gate: four supports-on cells positive 0.80 % paired
   wall/CPU improvement, eight cells byte-identical, four supports-off cells
   with no median wall/CPU regression (t137's generalized form).
4. Record the measured tile-partition cost and the realized operand reduction
   per call, so the 60.8 % / 61.6 % ceiling is replaced by a measured number.

## Acceptance

- `evidence/t175-tiled-collision-carve/` with the tile-size sweep, the gate
  campaign, byte-identity proof, `FINDINGS.md`, and a checkout-only
  `verify.py --self-test`.
- Map: destination-status block, Decisions-so-far line, and an updated
  **Not yet specified** (the measured phase-1 win, and the phase-3 companion
  still unnamed).
- No production edit until the output-identity check and the gate pass; the
  change is a work-set reduction, not a geometry change, so it does not need a
  new t81 ruling. The t111/t128 prepass exception is unchanged.

## Answer

**Resolved 2026-11-21 — KEEP (`tile-2mm`), standing `t137` 4-cell planner gate PASSED (4/4 supports-on PASS, 4/4 supports-off INERT-PASS), landed by explicit human KEEP ruling advancing only the 2 `base` supports-on hashes for proven Clipper2 distant-scanbeam unclamping.** Evidence: [`evidence/t175-tiled-collision-carve/FINDINGS.md`](../evidence/t175-tiled-collision-carve/FINDINGS.md), active reference set [`evidence/t175-tiled-collision-carve/reference.json`](../evidence/t175-tiled-collision-carve/reference.json).

- **Implementation (guest-only, zero host/WIT/schema edits):** Added exact non-perturbing per-layer collision tile index `CollisionTileIndex` in `modules/core-modules/tree-support-planner/src/collision_tile_index.rs` (flat CSR edge grid + row-crossing table, `O(dx + dy)` row-slab supercover edge binning, exact `i128` ray parity for `Empty`/`Covered` short-circuits, universal-cover exterior-frame closure retaining every touching edge verbatim, per-tile-union memo, `DEFAULT_COLLISION_TILE_UNITS = 20_000` = 2 mm, `MIN_INDEXED_COLLISION_VERTICES = 64`), wired exclusively into `modules/core-modules/tree-support-planner/src/lib.rs::build_roles_batched` Phase 1. Request order and Phases 2–8 are untouched.
- **Tile-size sweep on `base-classic-on` (`1 / 2 / 5 / 10 mm`, 1 warmup + 3 alternating rounds, load `1.43 % → 5.2 %`):** `baseline` **41.024 s** (`292.28 s` CPU, `0e13ff7b…`); `tile-1mm` **28.336 s** (`+30.93 %` wall / `+40.87 %` CPU, `−99.65 %` clip pts); **`tile-2mm` (chosen, lowest wall) 28.311 s** (`+30.99 %` wall / `+40.27 %` CPU, `−99.15 %` clip pts); `tile-5mm` **28.587 s** (`+30.32 %` / `+40.05 %`, `−96.91 %`); `tile-10mm` **29.124 s** (`+29.01 %` / `+38.88 %`, `−92.18 %`). All four tiled sizes produce the exact same G-code SHA-256 `7b90e749…`.
- **Measured partition cost & realized operand reduction (`--profile`, `probe.patch` cleanly reverted):** On `base-classic-on`, `tile_index_build` costs **45.9 ms** (`1.44 B` fuel, 430 layers) and `tile_index_query` costs **84.6 ms** (`1.55 B` fuel, 68,381 queries) = **130.5 ms** total, collapsing `phase1_clip_batches` from **12,436.6 → 174.4 ms** (`−98.6 %`), sent requests from **68,381 → 30,639** (`37,742` empty-gated, `23,468` cache hits), Phase-1 clip points from **426,868,700 → 3,629,342** (**−99.15 %**, `0.85 %` remaining vs `t173`'s `1.62 %` estimate), mean clip points per query from **6,242.5 → 53.1** (`118.5` per sent request), net planner guest fuel from **62.27 → 48.58 B** (**−22.0 %**), and `host:native` clip thread from **173.90 → 56.02 s**. On `benchy-classic-on`, build `8.7 ms` + query `9.2 ms` = **17.9 ms**, `phase1_clip_batches` **626.9 → 28.4 ms**, sent requests **10,600 → 5,165** (`5,435` empty-gated), clip points **28,230,127 → 251,534** (**−99.11 %**).
- **Standing `t137` gate (5 interleaved pairs/cell ordinary, 3-pair null control, 3-pair accelerated; load `0 %–6 %`; 208/208 runs clean `exit=0`, `degraded=false`, `0` code-1200):**
  - `benchy-classic-on`: `4.057 → 3.407 s`, **+16.06 %** wall (`5/5`, `20.08×` floor, null `2.02 %`), **+26.75 %** CPU (`5/5`); acc **+16.42 % / +25.20 %** (`3/3`) — **PASS**.
  - `benchy-arachne-on`: `4.222 → 3.520 s`, **+16.62 %** wall (`5/5`, `20.77×` floor, null `2.29 %`), **+25.36 %** CPU (`5/5`); acc **+15.84 % / +27.06 %** (`3/3`) — **PASS**.
  - `base-classic-on`: `41.169 → 28.235 s`, **+31.32 %** wall (`5/5`, `39.15×` floor, null `1.10 %`), **+40.86 %** CPU (`5/5`); acc **+31.39 % / +40.97 %** (`3/3`) — **PASS**.
  - `base-arachne-on`: `41.556 → 28.663 s`, **+30.90 %** wall (`5/5`, `38.62×` floor, null `1.08 %`), **+40.10 %** CPU (`5/5`); acc **+31.53 % / +40.39 %** (`3/3`) — **PASS**.
  - Four supports-off cells: `−0.58 %` to `+0.74 %` ordinary (`−0.73 %` to `+0.55 %` accelerated), structurally inert — **4/4 INERT-PASS**.
- **Output identity & Clipper2 distant-scanbeam proof:** Baseline reproduces `t172`'s active set **16/16**; candidate ordinary ≡ candidate accelerated **16/16**. **6/8 cells are byte-identical to `t172`** (all four supports-off cells plus both `benchy` supports-on cells). Only `base-classic-on` (`0e13ff7b… → 7b90e749…`, `−1 B`, **7 / 976,781 lines**, `−0.00071 mm` out of `780,433.17 mm`) and `base-arachne-on` (`adf87012… → f77633af…`, `−1 B`, **5 / 972,259 lines**, `−0.00071 mm` out of `784,107.76 mm`) move — strictly **4 XY vertices by 1 µm (`0.001 mm`)** in `Support interface` (layer 35) and `Support` (layer 45) plus 3/1 cumulative-`E` rounding ticks (`dx = dy = 0`), with **zero** part-geometry movement. Exhaustive 68,381-call audit proved **0** `Empty`/`Covered` mismatches and **9/9 (100 %)** single-vertex `Polys` differences caused by `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamping shallow intersections to global `scanline_list` Y coordinates injected by unrelated distant collision vertices 14–36 mm away (`8/9` bulk + 9th isolated `full_eq_win_with_dummy_far_y = true`). No Clipper2 clip-operand reduction can guarantee literal 8/8 byte-identity on `base.stl`. Human explicitly ruled **KEEP `tile-2mm`** and updated the two `base` supports-on hashes on 2026-11-21.
- **Route arithmetic (cross-window ESTIMATE vs `t174` quiet board, not a ratio of record):** `base-classic-on` `1.777× → ~1.221×` ord (`1.169×` acc, closes **71.6 %** of `t174`'s cut, beating `t173`'s `60.8 %` ceiling); `base-arachne-on` `1.738× → ~1.201×` (`1.153×` acc, **72.8 %** vs `61.6 %`); `benchy-classic-on` (binding) `1.870× → ~1.570×` (`1.602×` acc, **34.5 %** vs `23.2 %`); `benchy-arachne-on` `1.665× → ~1.389×` (`1.409×` acc, **41.6 %** vs `21.8 %`). Three more cells cross below ~1.5x; Phase-3 union (`51.6 s` thread) is now **~92.1 %** of remaining `host:native` clip thread on `base-classic-on` and stays unnamed in **Not yet specified**.
- **Verification:** `cargo test -p tree-support-planner --no-fail-fast` (`71/71` lib + `84/84` integration; 1 pre-existing unrelated `slicer_module_binding_tdd::binding_surface_matches_manifest` `2.1.0` vs `2.0.0` string mismatch disclosed), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, `cargo xtask build-guests --check`, `cargo xtask build-guests --accelerated --check`, `audit_t175.py --self-test` (6/6), `verify.py --self-test` (13/13), `verify.py` PASS.
