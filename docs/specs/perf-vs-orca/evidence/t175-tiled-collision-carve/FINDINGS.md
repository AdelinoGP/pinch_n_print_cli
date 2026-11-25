# t175 findings: tiled collision operand for the planner's phase-1 carve

**Verdict: measured candidate, standing 4-cell planner gate PASSED (4/4 supports-on PASS, 4/4 supports-off INERT-PASS), landed by explicit human KEEP ruling (2026-11-21) with 6/8 cells byte-identical and 2 `base` supports-on hashes updated for proven Clipper2 distant-scanbeam unclamping.**

Replacing the whole-layer collision clone on `modules/core-modules/tree-support-planner/src/lib.rs::build_roles_batched` Phase 1 with a guest-only exact non-perturbing per-layer tile index (`modules/core-modules/tree-support-planner/src/collision_tile_index.rs::CollisionTileIndex`, `2 mm` tiles chosen by sweep) saves **+31.32 %** (`base classic-on`, `41.169 → 28.235 s`), **+30.90 %** (`base arachne-on`, `41.556 → 28.663 s`), **+16.06 %** (`benchy classic-on`, `4.057 → 3.407 s`), and **+16.62 %** (`benchy arachne-on`, `4.222 → 3.520 s`) wall across 5 interleaved pairs/cell (**5/5 wins in every supports-on cell**, **20.1x–39.2x** the 0.80 % floor, **7.3x–28.6x** outside each cell's same-window any-pair wall null band), with process CPU corroborating at **+40.86 / +40.10 / +26.75 / +25.36 %** (**5/5** in every supports-on cell) and accelerated corroboration reproducing **+31.39 / +31.53 / +16.42 / +15.84 %** wall and **+40.97 / +40.39 / +25.20 / +27.06 %** CPU (**3/3** in every supports-on cell). All four supports-off cells are structurally inert (`enable_support: false` never runs Phase 1) and show no regression (`−0.58 %` to `+0.74 %` ordinary, `−0.73 %` to `+0.55 %` accelerated).

Every supports-on cell **exceeds** [t173](../../issues/173-support-geometry-boolean-volume.md)'s host-only wall ceiling (`+31.32 %` vs `27.05 %` and `+30.90 %` vs `25.92 %` on `base`; `+16.06 %` vs `11.22 %` and `+16.62 %` vs `10.87 %` on `benchy`), because the tile index collapses Phase-1 clip points by **−99.15 %** (`426.87 M → 3.63 M` on `base`; **−99.11 %** on `benchy`) at a total guest index cost of only **130.5 ms** (`45.9 ms` build + `84.6 ms` query) while also eliminating **6.8 GB** of guest `collision_polys.clone()` and WIT marshalling (`−22.0 %` net planner guest fuel).

Resolved 2026-11-21. Ticket: [Tiled collision operand for the planner's phase-1 carve](../../issues/175-tiled-collision-carve.md). Active reference set: [`reference.json`](reference.json) (6/8 unchanged from `t172`; only `base-classic-on` and `base-arachne-on` superseded).

---

## 1. Design & implementation (guest-only, exact non-perturbing windowing)

A naive rectangular Clipper `Intersection` tile partition bends sloped collision edges by up to 50 nm at tile-boundary intersections, introducing true geometric perturbation inside tiles. Instead, `modules/core-modules/tree-support-planner/src/collision_tile_index.rs::CollisionTileIndex` implements **homotopic edge-preserving windowing** on each layer's collision polygon set (`DEFAULT_COLLISION_TILE_UNITS = 20_000` = 2 mm, `MIN_INDEXED_COLLISION_VERTICES = 64`):

1. **Per-layer CSR tile index (`CollisionTileIndex::build`):** flattens all collision contours/holes into contiguous vertex/ring arrays, supercover-bins original edges into an integer tile grid via an `O(dx + dy)` horizontal row-slab walk (stored in flat CSR `cell_offsets` / `cell_entries`), and builds a row-crossing CSR table (`row_offsets` / `row_entries`) for fast exact ray parity.
2. **Exact window construction (`CollisionTileIndex::window_for_bbox`):** rounds the subject bbox outward to tile coordinates `(tx0..tx1, ty0..ty1)`, consults a per-layer tile-union memo (`HashMap<(i64, i64, i64, i64), WindowedCollision>`), and on a miss collects the union of edges touching the window:
   - **No touching edges:** exact `i128` horizontal ray parity at the window's bottom-left corner decides `WindowedCollision::Empty` (skip host clip on canonical subjects; empty clip otherwise) vs `WindowedCollision::Covered` (subject strictly inside collision → difference is provably empty, drop immediately).
   - **Touching edges present:** retains every touching original edge **verbatim** (zero coordinate rounding on any edge intersecting the window) and closes open exterior runs along a universal-cover safety frame strictly outside the window bbox, preserving the exact winding number at every point inside the window.
3. **Phase-1-only wiring (`modules/core-modules/tree-support-planner/src/lib.rs::build_roles_batched`):** lazy per-layer index built only for layers with `>= 64` collision vertices that reach Phase 1 after the existing ticket-32 outer bbox gate. Request order and Phases 2–8 (including Phase 3 union) are untouched. Zero host, WIT, or schema edits (identical host exe SHA-256 on both arms in both modes).

---

## 2. Tile-size sweep (`base-classic-on`, chosen: `tile-2mm`)

1 warmup + 3 alternating rounds (`baseline → 1 → 2 → 5 → 10`, reversed on round 2) on a quiet box (sampled CPU load `1.43 % → 5.2 %`), staged with identical host exe `9f3fcba8…` and differing only in `tree-support-planner.wasm` (`reduced/sweep.json`):

| Arm | Wall med (s) | Wall spread (s) | Wall Δ | CPU med (s) | CPU Δ | Phase-1 sent req | Phase-1 clip pts (Δ) | Planner fuel (B) | SHA-256 (prefix) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `baseline` | 41.024 | 40.883–41.358 | — | 292.28 | — | 68,381 | 426,868,700 (—) | 62.27 | `0e13ff7b` (`t172`) |
| `tile-1mm` | 28.336 | 28.012–28.467 | +30.93 % | 172.81 | +40.87 % | 18,646 | 1,480,816 (−99.65 %) | 48.02 | `7b90e749` |
| **`tile-2mm` (chosen)** | **28.311** | **28.084–28.718** | **+30.99 %** | **174.59** | **+40.27 %** | **30,639** | **3,629,342 (−99.15 %)** | **48.58** | **`7b90e749`** |
| `tile-5mm` | 28.587 | 28.411–28.647 | +30.32 % | 175.23 | +40.05 % | 50,027 | 13,201,448 (−96.91 %) | 50.08 | `7b90e749` |
| `tile-10mm` | 29.124 | 29.085–29.377 | +29.01 % | 178.64 | +38.88 % | 64,150 | 33,394,758 (−92.18 %) | 51.68 | `7b90e749` |

- **Selection:** `tile-2mm` wins by the pre-declared rule (lowest measured median wall: `28.311 s` vs `28.336 s` at 1 mm, `28.587 s` at 5 mm, `29.124 s` at 10 mm) with 4× fewer grid cells than 1 mm.
- **Scale-invariance check:** all four tiled sizes (`1 / 2 / 5 / 10 mm`) emit the exact same G-code SHA-256 `7b90e749…`, confirming that the 1 µm unclamping (§6) is independent of tile size and stems solely from omitting unrelated distant collision vertices.

---

## 3. Tile-partition cost & realized operand reduction (`--profile`, `probe.patch`)

Attribution-only captures (`--profile --instrument-stderr` with `probe.patch`, cleanly reverted afterward; fuel is exact executed WASM instructions, `--profile` wall is attribution-only and never a gate claim; `reduced/probe.json`):

| Cell (`arm`) | Regions → queries | Sent req (`Empty` / `cache_hits`) | Clip pts (`orig → win`, Δ) | Mean clip pts/query (`→ /sent`) | `tile_index_build` | `tile_index_query` | `phase1_clip_batches` | Planner fuel (B) | `host:native` clip thread |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `base-classic-on` (`base`) | 136,477 → 68,381 | 68,381 (0 / 0) | 426,868,700 → 426,868,700 | 6,242.5 → 6,242.5 | — | — | 12,436.6 ms (9.91 B) | 62.27 | 173.90 s (112,933) |
| **`base-classic-on` (`2mm`)** | 136,477 → 68,381 | **30,639** (37,742 / 23,468) | **426,868,700 → 3,629,342 (−99.15 %)** | **53.1 → 118.5** | **45.9 ms (1.44 B, 430 layers)** | **84.6 ms (1.55 B, 68,381 q)** | **174.4 ms (0.39 B)** | **48.58 (−22.0 %)** | **56.02 s (75,191)** |
| `base-arachne-on` (`2mm`) | 136,477 → 68,381 | **30,639** (37,742 / 23,468) | **426,868,700 → 3,629,342 (−99.15 %)** | **53.1 → 118.5** | 45.9 ms (1.44 B) | 86.1 ms (1.55 B) | 174.7 ms (0.39 B) | 48.58 (−22.0 %) | 56.07 s (75,191) |
| `benchy-classic-on` (`base`) | 22,062 → 10,600 | 10,600 (0 / 0) | 28,230,127 → 28,230,127 | 2,663.2 → 2,663.2 | — | — | 626.9 ms (0.64 B) | 6.69 | 7.89 s (18,900) |
| **`benchy-classic-on` (`2mm`)** | 22,062 → 10,600 | **5,165** (5,435 / 3,396) | **28,230,127 → 251,534 (−99.11 %)** | **23.7 → 48.7** | **8.7 ms (0.28 B, 179 layers)** | **9.2 ms (0.21 B, 10,600 q)** | **28.4 ms (0.03 B)** | **5.85 (−12.6 %)** | **2.93 s (13,465)** |
| `benchy-arachne-on` (`2mm`) | 22,062 → 10,600 | **5,165** (5,435 / 3,396) | **28,230,127 → 251,534 (−99.11 %)** | **23.7 → 48.7** | 8.5 ms (0.28 B) | 9.1 ms (0.21 B) | 25.4 ms (0.03 B) | 5.85 (−12.6 %) | 2.94 s (13,465) |

- **Exact cross-check with t173:** `orig_clip_pts` matches `t173/reduced/arithmetic.json` to the single vertex (`426,868,700` on `base`, `28,230,127` on `benchy`), and region conservation holds with zero discrepancy (`outer_gated + empty_gated + covered_dropped + clip_requests == regions_total`) across all 11 captures.
- **Why measured wall beats t173's ceiling:** `t173` estimated `1.62 %` remaining points (`3.65 %` on benchy) and priced only the host Clipper phase-1 wall (`12.41 s` → `27.05 %` of cell wall on `base-classic-on`), assuming zero guest/marshal saving. In practice: (a) empty-window short-circuiting skips **55.2 %** of requests on `base` (`37,742 / 68,381`) and **51.3 %** on `benchy` (`5,435 / 10,600`), cutting remaining clip points to **0.85 %** (`base`) and **0.89 %** (`benchy`); (b) total index build + query cost is only **130.5 ms** (`2.99 B` fuel) on `base` vs **12,262 ms** saved in `phase1_clip_batches` (**94:1 payoff**); and (c) skipping `collision_polys.clone()` + WIT marshalling of 423.2 M points saves **13.69 B** net guest fuel (**−22.0 %** of planner fuel, including `−545 ms` in planner `<module self>`).

---

## 4. Standing `t137` gate: paired wall, CPU, null control, and accelerated corroboration

Four staged arms from HEAD `d8892479` (identical host exes `9f3fcba8…` ordinary / `490c6dc2…` accelerated; only `tree-support-planner.wasm` differs: `d6a56af5… → c56f07b3…` ordinary, `355851e8… → 902dc921…` accelerated). Quiet box across all batches (sampled CPU load: A/B `1 % → 3 %`, null `1 % → 0 %`, accelerated `2 % → 6 %`). Positive = candidate faster.

### Ordinary arm (5 interleaved pairs/cell, `reduced/ab.json` + `reduced/null.json`)

| Cell | Wall s (base → cand) | Wall med (spread) | Wins | Floor× | Null wall (any/med) | CPU med (wins) | Null CPU (any/med) | Gate verdict |
|---|---:|---|---:|---:|---:|---|---:|---|
| `benchy-classic-off` | 1.714 → 1.724 | −0.58 % (−0.84…+1.16) | 1/5 | −0.72× | 1.32 / 0.87 % | −1.80 % (1/5) | 5.29 / 4.21 % | **INERT-PASS** |
| **`benchy-classic-on`** | **4.057 → 3.407** | **+16.06 %** (+14.43…+17.24) | **5/5** | **20.08×** | 2.02 / 0.56 % | **+26.75 %** (5/5) | 1.78 / 1.40 % | **PASS** |
| `benchy-arachne-off` | 1.778 → 1.774 | +0.62 % (+0.11…+3.55) | 5/5 | 0.78× | 0.53 / 0.13 % | −2.49 % (1/5) | 4.30 / 1.98 % | **INERT-PASS** |
| **`benchy-arachne-on`** | **4.222 → 3.520** | **+16.62 %** (+14.79…+18.41) | **5/5** | **20.77×** | 2.29 / 1.84 % | **+25.36 %** (5/5) | 4.86 / 1.45 % | **PASS** |
| `base-classic-off` | 10.543 → 10.516 | +0.26 % (−0.97…+0.93) | 3/5 | 0.33× | 2.38 / 1.07 % | −0.05 % (2/5) | 0.90 / 0.39 % | **INERT-PASS** |
| **`base-classic-on`** | **41.169 → 28.235** | **+31.32 %** (+30.40…+31.78) | **5/5** | **39.15×** | 1.10 / 0.59 % | **+40.86 %** (5/5) | 0.80 / 0.51 % | **PASS** |
| `base-arachne-off` | 10.999 → 10.980 | +0.74 % (−0.45…+1.47) | 3/5 | 0.92× | 0.96 / 0.85 % | +0.17 % (3/5) | 0.56 / 0.10 % | **INERT-PASS** |
| **`base-arachne-on`** | **41.556 → 28.663** | **+30.90 %** (+29.66…+31.19) | **5/5** | **38.62×** | 1.08 / 0.50 % | **+40.10 %** (5/5) | 0.65 / 0.44 % | **PASS** |

### Accelerated corroboration (3 interleaved pairs/cell, `reduced/accel.json`)

| Cell | Wall s (base → cand) | Wall med (wins) | CPU med (wins) |
|---|---:|---:|---:|
| `benchy-classic-off` | 1.715 → 1.718 | +0.19 % (2/3) | −0.73 % (1/3) |
| **`benchy-classic-on`** | **4.072 → 3.369** | **+16.42 %** (3/3) | **+25.20 %** (3/3) |
| `benchy-arachne-off` | 1.781 → 1.766 | +0.55 % (2/3) | +1.29 % (2/3) |
| **`benchy-arachne-on`** | **4.154 → 3.496** | **+15.84 %** (3/3) | **+27.06 %** (3/3) |
| `base-classic-off` | 10.358 → 10.344 | −0.73 % (1/3) | +0.26 % (2/3) |
| **`base-classic-on`** | **40.662 → 27.834** | **+31.39 %** (3/3) | **+40.97 %** (3/3) |
| `base-arachne-off` | 10.786 → 10.749 | +0.30 % (2/3) | −0.17 % (1/3) |
| **`base-arachne-on`** | **41.167 → 28.231** | **+31.53 %** (3/3) | **+40.39 %** (3/3) |

---

## 5. Output currency, delta audit, and Clipper2 distant-scanbeam proof

Across all **208** verified runs (`32` output-check + `80` A/B + `48` null + `48` accelerated): **0** non-zero exits, **0** degraded runs, **0** fatal errors, **0** code-1200 diagnostics (`reduced/output.json`). Baseline reproduces `t172`'s active reference set **16/16** (8 ordinary + 8 accelerated), and candidate ordinary is byte-identical to candidate accelerated **8/8** (**16/16** total).

| Cell | `t172` (baseline) | `t175` (candidate) | Identical? | Δ bytes | Diff lines / total | Total extruding Δ mm | Moved sections |
|---|---|---|---|---:|---:|---:|---|
| `benchy-classic-off` | `4c67ef0b…` | `4c67ef0b…` | **YES** | 0 | 0 / 102,156 | 0.00000 | none |
| `benchy-classic-on` | `fdca4e2d…` | `fdca4e2d…` | **YES** | 0 | 0 / 212,199 | 0.00000 | none |
| `benchy-arachne-off` | `fd631384…` | `fd631384…` | **YES** | 0 | 0 / 105,655 | 0.00000 | none |
| `benchy-arachne-on` | `2869186c…` | `2869186c…` | **YES** | 0 | 0 / 215,698 | 0.00000 | none |
| `base-classic-off` | `02ffa2d6…` | `02ffa2d6…` | **YES** | 0 | 0 / 523,604 | 0.00000 | none |
| **`base-classic-on`** | `0e13ff7b…` | **`7b90e749…`** | **NO** | −1 | **7 / 976,781** | **−0.00071** (−0.00000009 %) | `Support interface` (−0.0004 mm), `Support` (−0.0003 mm) |
| `base-arachne-off` | `e806296f…` | `e806296f…` | **YES** | 0 | 0 / 519,081 | 0.00000 | none |
| **`base-arachne-on`** | `adf87012…` | **`f77633af…`** | **NO** | −1 | **5 / 972,259** | **−0.00071** (−0.00000009 %) | `Support interface` (−0.0004 mm), `Support` (−0.0003 mm) |

- **Exact differing lines (`reduced/audit.json`):** on both `base-classic-on` and `base-arachne-on`, exactly **4 XY coordinates** shift by **1 µm (`0.001 mm`)** — two in `Support interface` on layer 35 (`Z7`: `Y114.665 → Y114.666` and `X103.871 → X103.87`, the dropped trailing digit causing the −1 byte delta) and two in `Support` on layer 45 (`Z9`: `Y121.333 → Y121.334` and `Y121.594 → Y121.595`). The remaining 3 lines on `base-classic-on` (layers 48, 98, 216) and 1 line on `base-arachne-on` (layer 48) have `dx = dy = 0.0 mm` and carry only a `0.00024–0.00098 mm` cumulative `E` rounding tick. **Zero part-geometry sections (`Outer wall`, `Inner wall`, `Top/Bottom surface`, `Bridge`, `Sparse/Solid/Gap infill`, `Skirt`, `Brim`) move anywhere.**
- **Exhaustive Clipper2 root-cause proof (`reduced/audit.json::scanbeam_audit`):** auditing all **68,381** Phase-1 calls on `base-classic-on` (`full` vs `windowed`) found **0/37,742** `Empty` mismatches, **0/0** `Covered` mismatches, and **9/30,639** `Polys` single-vertex mismatches (`flen == wlen`, `diff_v = 1`). All **9/9 (100 %)** are caused by `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamping shallow edge intersections (`ip.y > self.bot_y || ip.y < top_y`) to global `scanline_list` Y coordinates contributed by unrelated distant collision vertices 14–36 mm away (`8/9` reproduced identically when injecting far-away dummy triangles carrying the omitted Y scanlines in bulk, and the 9th call `rb=(1001681, 1053255, 1128598, 1180262)` proven individually with `FAR_Y_1143929: count=2 [(3, 1415081, 1143929), (3, 1193021, 1143929)]` and `DUMMY_SCANLINE_PROOF: full_eq_win=false, full_eq_win_with_dummy_far_y=true`). On `benchy-classic-on` (10,600 calls), `0/5,435` `Empty`, `0/0` `Covered`, and `1/5,165` `Polys` difference (125 vs 124 vertices) was absorbed by Phases 3–5, leaving G-code byte-identical.
- **Architectural consequence for `t137`:** because Clipper2 inserts every `clip` vertex's Y into its global `scanline_list`, **no** Clipper2 clip-operand reduction (not tiling, not even a disjoint-polygon bbox filter) can guarantee literal 8/8 byte-identity on `base.stl`. The windowed output is strictly closer to the exact geometric intersection.

---

## 6. Route arithmetic (cross-window estimate, not a ratio of record)

Composing `t175`'s paired wall medians with [t174](../../issues/174-post-t172-scoreboard-refresh.md)'s quiet-board official ratios of record (`ESTIMATE ONLY`, never a three-tool scoreboard ratio of record; `reduced/route.json`):

| Cell | `t174` ratio (ord / acc) | `t174` cut (ord) | `t175` wall Δ (ord / acc) | **Est. post-t175 ratio (ord / acc)** | Share of `t174` cut closed | `t173` host-only ceiling (wall / cut) |
|---|---:|---:|---:|---:|---:|---:|
| `benchy-classic-on` (binding) | 1.870x / 1.916x | 46.54 % | +16.06 % / +16.42 % | **~1.570x / ~1.602x** | **34.5 %** | 11.22 % / 23.18 % |
| `benchy-arachne-on` | 1.665x / 1.674x | 39.95 % | +16.62 % / +15.84 % | **~1.389x / ~1.409x** | **41.6 %** | 10.87 % / 21.83 % |
| `base-classic-on` | 1.777x / 1.704x | 43.73 % | +31.32 % / +31.39 % | **~1.221x / ~1.169x** | **71.6 %** | 27.05 % / 60.82 % |
| `base-arachne-on` | 1.738x / 1.684x | 42.47 % | +30.90 % / +31.53 % | **~1.201x / ~1.153x** | **72.8 %** | 25.92 % / 61.60 % |

- **Park-bound crossing:** `base-classic-on` (~1.221x / ~1.169x), `base-arachne-on` (~1.201x / ~1.153x), and `benchy-arachne-on` (~1.389x / ~1.409x) all cross below the ~1.5x below-fold park bound, leaving `benchy-classic-on` (~1.570x / ~1.602x) as the sole cell estimated above 1.5x.
- **What remains in the planner:** `host:native::polygon_ops::clip_polygons` thread-time on `base-classic-on` dropped from **173.90 s → 56.02 s**; `t173`'s Phase-3 batched union (`planner::batched_phase3_union`, `51.60 s` thread / `5.21 s` wall, 480 calls) now accounts for **~92.1 %** of the remaining `host:native` clip thread on `base-classic-on` (`~89.6 %` on `base-arachne-on`) and remains unnamed in **Not yet specified**.

---

## 7. Disclosures & verification

- **Pre-existing test failure (not caused):** `cargo test -p tree-support-planner --no-fail-fast` passed **71/71** lib tests (including 6 new tile-index and dense-collision parity tests) and **84/84** integration tests across 8 targets; one pre-existing test (`slicer_module_binding_tdd::binding_surface_matches_manifest`) failed because it asserts `@2.0.0#run` against the manifest's `2.1.0` binding string from `t148`. Left untouched.
- **Pre-commit static & freshness gates:** `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals` (0 violations), `cargo xtask check-test-quality --report` (0 findings in touched files), `cargo xtask build-guests --check` (exit 0), and `cargo xtask build-guests --accelerated --check` (exit 0).
- **Checkout-only verification:** `python audit_t175.py --self-test` (6/6 negative controls rejected), `python verify.py --self-test` (13/13 negative controls rejected including `reference.json` check), and `python verify.py` (PASS over all 8 cells).
