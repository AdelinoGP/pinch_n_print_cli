# t169 findings: canonical `append_offset_path` edge decimation

**Verdict: measured candidate, gate PASSED in 8/8 cells, landed.** Porting
canonical `v2.5.0`'s pre-offset edge decimation into `polygon_ops` takes
**+21.9 % to +25.1 %** of the four `base` cells' wall and **+5.7 % to +15.2 %**
of the four `benchy` cells', five of five pairs in every cell, against a
same-window null band of **3.06 %** any-pair and **1.99 %** median on wall.
Process CPU agrees in 8/8 on both build arms.

Resolved 2026-11-20. Ticket:
[169](../issues/169-canonical-offset-edge-decimation-and-flat-opening.md).

The change is **not** output-neutral, and the human granted the **fifth recorded
t81-style exception** (per-layer geometry, in-session 2026-11-19/20) so that it
could land: decimation deletes vertices, so the slice moves in 8/8 cells. The
active reference set is now t169's
([`reference.json`](reference.json)); t168's is superseded.

## 1. What was ported

Canonical `v2.5.0` (`40ecc66f78`) `ClipperUtils.cpp::append_offset_path` runs
before **every** `ClipperOffset` call in `ClipperUtils`, with

```text
shortest = |delta| * ClipperOffsetShortestEdgeFactor   // 0.005, ClipperUtils.hpp
```

and does three things a plain conversion does not:

1. **Wrap-around trim.** For closed rings, trailing vertices within `shortest` of
   the first are removed before anything else, so the closing edge is not a
   near-duplicate.
2. **Collapse against the last kept vertex.** A vertex is dropped when it is
   within `shortest` of the **last kept** vertex — not the previous one — so a
   run of near-duplicates collapses to one point instead of every other point.
3. **`< 3` drop.** A polygon ring left with fewer than three vertices is dropped
   entirely rather than handed on degenerate.

The caller (`c2_offset_expolygons`) adds two more rules that the port mirrors:
a zero-area contour is dropped when shrinking and **kept** when growing, a
zero-area hole is always dropped, and windings are normalised (contour CCW, hole
CW) after decimation.

`polygon_ops` now harvests its rings that way in
[`offset_paths`](../../../../crates/slicer-core/src/polygon_ops.rs) — used by
`offset`, `offset_with_miter_limit`, `opening`, `closing_ex` and pass 1 of
`offset2_ex` — and by `offset_paths_flat` for **pass 2** of `offset2_ex`, whose
threshold is `|delta2| * 0.005`, a different number from pass 1's. That second
decimation matters: canonical's `c2_offset2_closed` runs the harvest on both
passes, and pass 2's input is pass 1's offset result.

The factor is dimensionless, so `0.005 * |delta|` in PnP's 100 nm grid is
`0.005 * |delta_units|`, exactly canonical's threshold expressed in its 1 nm ×
100 grid. No rescaling is needed and none is applied.

## 2. Arm 1 was already in the tree — the whole effect is Arm 2

The ticket split the work into Arm 1 (flat-path pass-1 `offset2_ex`, no
intermediate `PolyTree64`) and Arm 2 (decimation). **Arm 1 landed on 2026-07-11
(`a5433f93`),** before this map was charted; `b56f429a` (2026-10-08) later
extracted it into `offset2_ex_first_pass` so the emptiness predicate and the
two-pass entry point share one first pass. There is therefore **no Arm-1 delta
to measure** and none is claimed: every number below is Arm 2 plus the pass-2
decimation that canonical applies. That also means t167's `f2` factor
(`1.72x–1.76x` on `0.04 mm Miter 3` openings) was **partly already banked** in
PnP before this take — the remaining canonical gap there is `f1` (Rust 1.1.0 →
C++ 1.5.2) plus the decimation measured here.

## 3. Falsifiers, proven red before the fix

`slicer-core` unit tests (`polygon_ops::tests`) pin the helper's three
distinctive rules, each invisible in a square's offset output:

| test | what it falsifies |
| --- | --- |
| `append_offset_path_trims_a_wrap_around_near_duplicate` | the wrap trim (red when the trim loop is removed: 5 vertices instead of 4) |
| `append_offset_path_collapses_a_run_against_the_last_kept_vertex` | comparison against the last kept vertex (a previous-vertex-only port returns 6) |
| `append_offset_path_drops_a_ring_left_with_two_vertices` | the `< 3` drop |
| `append_offset_path_keeps_a_ring_that_decimates_to_exactly_three` | the boundary is `< 3`, not `<= 3` |
| `append_offset_path_compares_exactly_at_a_zero_threshold` | `shortest == 0` falls back to exact equality |
| `offset_paths_drops_a_degenerate_contour_only_when_eroding` | canonical's asymmetric degenerate-contour branch |

Integration tests (`tests/offset_edge_decimation_tdd.rs`) pin the behaviour
visible through the public API:

- `sub_threshold_pinhole_is_deleted_instead_of_inflated_by_erosion` — a 4 µm
  hole under a 1 mm erosion. **Watched failing before the fix**: without the
  arm the hole survives and Clipper2 inflates it into a ~2 mm crater (the
  `IB_DEEP_NEG` class t167 measured). Green after.
- `hole_above_the_threshold_is_kept_and_grows_with_the_erosion` — a 20 µm hole is
  above the 50-unit threshold, survives, and grows to 2.02 mm.
- `offset2_ex_preserves_hole_nesting_across_both_decimated_passes` — a 20 mm
  plate with a 6 mm hole round-trips with its hole intact after pass 2's
  decimation.

## 4. Campaign

One window. Manifest (`target/t169/manifest.json`, reproduced in
[`reduced/output.json`](reduced/output.json)):

| arm | exe sha256 | module tree |
| --- | --- | --- |
| baseline ordinary | `e7b67146…` | `6899f824…` |
| candidate ordinary | `b2e8261e…` | `9c1da3bf…` |
| baseline accelerated | `0784a754…` | `82d74609…` |
| candidate accelerated | `f56c9271…` | `9504e039…` |

The module trees move because `slicer-core::polygon_ops` is linked into **seven**
guest modules, two of which call the decimated helpers in-process:
`classic-perimeters` (`offset2_ex`, `opening_ex`, `opening_is_empty`) and
`arachne-perimeters` (`offset2_ex`, `offset_with_miter_limit`). So this is not a
host-only change: the same decimation runs inside those guests in addition to the
host paths (`prepass_slice`, `slice_postprocess_prepass`, the
`offset_polygons_batch` host dispatch). The other five (`infill-linker`,
`lightning-infill`, `tree-support-planner`, `tree-support`, `wave-overhangs`) use
`polygon_ops`' clip/boolean primitives only, which this port does not touch.

**Output (8 cells × 2 ordinary arms, plus 8 × 2 accelerated arms).** The
baseline reproduces **t168's** active set in **8/8** — the control that makes the
candidate's movement attributable to this change. The candidate moves **8/8**.
Both accelerated arms are byte-identical to their ordinary counterparts
(**16/16**).

| cell | t168 (baseline) | t169 (candidate) | bytes |
| --- | --- | --- | --- |
| benchy classic off | `d6cb4acd…` | `f505910c…` | −11,842 |
| benchy classic on | `21c6969f…` | `9b40bf51…` | −88,359 |
| benchy arachne off | `5c0f846e…` | `224fb6f0…` | −2,808 |
| benchy arachne on | `6fe4b09b…` | `f32047c3…` | −81,012 |
| base classic off | `9e360450…` | `33468ea9…` | +14,554 |
| base classic on | `f77cf265…` | `661b9a51…` | −113,054 |
| base arachne off | `cd896bab…` | `e861766a…` | +9,388 |
| base arachne on | `3044c4d6…` | `2124b5f5…` | −119,441 |

All 16 output runs exited 0.

## 5. Paired wall and CPU (ordinary arm, 5 interleaved pairs per cell)

Positive = candidate faster. The null band is the same instrument measured in
the same window (`-Mode ab -NullControl`, 3 pairs/cell), shown any-pair /
median.

| cell | wall s (base → cand) | wall median (spread) | wins | null wall | CPU median | wins | null CPU |
| --- | --- | --- | --- | --- | --- | --- | --- |
| benchy classic off | 2.438 → 2.245 | **+7.98 %** (+7.04…+9.16) | 5/5 | 1.83 / 0.37 | +16.36 % | 5/5 | 2.29 / 2.24 |
| benchy classic on | 5.929 → 5.024 | **+15.21 %** (+14.30…+15.32) | 5/5 | 0.79 / 0.32 | +18.06 % | 5/5 | 4.22 / 3.14 |
| benchy arachne off | 2.463 → 2.309 | **+5.74 %** (+2.00…+7.97) | 5/5 | 2.23 / 1.99 | +15.30 % | 5/5 | 3.89 / 1.70 |
| benchy arachne on | 6.018 → 5.137 | **+14.76 %** (+13.90…+15.55) | 5/5 | 1.38 / 0.64 | +19.71 % | 5/5 | 2.88 / 1.29 |
| base classic off | 18.363 → 13.730 | **+25.08 %** (+24.67…+25.45) | 5/5 | 2.28 / 1.23 | +30.28 % | 5/5 | 0.79 / 0.48 |
| base classic on | 63.183 → 49.336 | **+21.91 %** (+21.89…+23.02) | 5/5 | 3.06 / 1.65 | +27.66 % | 5/5 | 0.50 / 0.28 |
| base arachne off | 18.843 → 14.396 | **+23.43 %** (+18.50…+24.36) | 5/5 | 2.41 / 0.85 | +29.18 % | 5/5 | 1.09 / 0.21 |
| base arachne on | 63.621 → 49.832 | **+22.41 %** (+21.36…+24.03) | 5/5 | 0.95 / 0.47 | +26.96 % | 5/5 | 0.40 / 0.20 |

**Gate: PASSED on all eight cells.** Every base cell clears the 0.80 % floor by
**27.4x–31.4x** and sits far outside its own null band's any-pair bound; every
benchy cell is positive, outside its band, and 7.2x–19.0x the floor. Process CPU
is the corroborating channel and is lower in 8/8, 5/5 pairs in every cell.

**Accelerated corroboration (3 pairs per cell).** Wall medians
**+7.11 / +15.37 / +6.22 / +13.93 / +25.18 / +22.71 / +24.25 / +21.91 %**,
3/3 pairs in every cell; CPU **+17.46 / +23.17 / +15.77 / +19.36 / +30.62 /
+27.06 / +29.94 / +27.49 %**, 3/3 in every cell. The arm reproduces the ordinary
result, which is what a real work deletion should do.

## 6. Delta audit

`audit_t169.py` reads the campaign's G-code and writes
[`reduced/audit.json`](reduced/audit.json). **Every section moves somewhere in
every cell; no feature is lost.** Total extruding path changes by **−0.029 %**
to **+0.616 %**.

| cell | total path | moved sections | largest deltas |
| --- | --- | --- | --- |
| benchy classic off | −29.94 mm (−0.029 %) | 11/11 | Sparse infill −34.98 (−0.144 %), Bottom surface +21.19 (+0.738 %), Internal Bridge −20.01 (−1.814 %) |
| benchy classic on | +301.03 mm (+0.163 %) | 13/13 | Support +345.61 (+0.418 %), Sparse infill −34.98, Support interface −31.30 (−10.656 %) |
| benchy arachne off | +31.81 mm (+0.031 %) | 10/10 | Sparse infill +24.62 (+0.098 %), Internal Bridge −20.01, Bottom surface +20.01 |
| benchy arachne on | +362.22 mm (+0.196 %) | 12/12 | Support +345.61 (+0.418 %), Support interface −31.30 (−10.656 %), Sparse infill +24.62 |
| base classic off | +333.69 mm (+0.086 %) | 10/10 | Sparse infill +327.60 (+0.188 %), Top surface +11.05 (+5.146 %), Internal Bridge −9.25 (−72.005 %) |
| base classic on | +4,795.31 mm (+0.616 %) | 11/11 | Support +4,383.20 (+1.127 %), Sparse infill +327.60, Support interface +49.42 (+1.947 %) |
| base arachne off | +2.44 mm (+0.001 %) | 9/9 | Top surface +13.20 (+4.905 %), Internal Bridge −9.84 (−73.224 %), Bottom surface +4.98 (+6.670 %) |
| base arachne on | +4,464.01 mm (+0.570 %) | 10/10 | Support +4,383.20 (+1.127 %), Support interface +49.42, Skirt +28.92 (+1.701 %) |

Two readings matter for parity:

- **`Internal Bridge` collapses by 72 %–73 %** (−9.25 / −9.84 mm) on the two
  base supports-off cells. That is the expected direction and the mechanism
  t167 priced: the internal-bridge offsets run at large negative deltas
  (`-1.6069 mm` → 8.03 µm threshold), so sub-threshold pinholes in the operand
  are dropped instead of being inflated into craters. The section is small, so
  the absolute move is small.
- **`Support` grows +0.42 % / +1.13 %** on the supports-on cells (+345.61 /
  +4,383.20 mm) and `Support interface` moves −10.7 % on benchy and +1.9 % on
  base. Support geometry is the one section whose *direction* this port does not
  predict from canonical's rules — it is a downstream consequence of differently
  decimated occupancy offsets, and it is reported, not rationalised.

Total path deltas are **≤ 0.62 %** in every cell, and the per-layer census shows
both directions on nearly every section (e.g. base classic-off `Sparse infill`
moves on 228 layers up and 245 down), which is the signature of a vertex-level
relabelling that re-flows through the partition rather than of a lost feature
set.

## 7. Where the time came from (`--profile`, attribution only)

One `--profile` pass per cell per arm (`run_profile_t169.py`). `--profile`
inflates wall and sums concurrent spans, so **nothing here is a wall claim**;
the ratios are the point.

| cell | `polygon_ops::offset` self wall | `polygon_ops::offset2_ex` self wall | guest scope fuel |
| --- | --- | --- | --- |
| benchy classic off | 5,393 → 3,182 ms (−41.0 %) | 1,459 → 1,207 ms (−17.2 %) | −16.3 % |
| benchy classic on | 13,861 → 7,605 ms (−45.1 %) | 1,552 → 1,162 ms (−25.1 %) | −13.5 % |
| benchy arachne off | 5,222 → 2,904 ms (−44.4 %) | 1,493 → 1,211 ms (−18.9 %) | **+3.1 %** |
| benchy arachne on | 13,693 → 7,512 ms (−45.1 %) | 1,493 → 1,208 ms (−19.0 %) | −6.0 % |
| base classic off | 82,061 → 40,251 ms (−50.9 %) | 15,640 → 13,003 ms (−16.9 %) | −21.5 % |
| base classic on | 247,894 → 110,348 ms (−55.5 %) | 15,948 → 12,739 ms (−20.1 %) | −15.2 % |
| base arachne off | 80,454 → 39,337 ms (−51.1 %) | 15,920 → 13,303 ms (−16.4 %) | **+2.1 %** |
| base arachne on | 245,847 → 115,228 ms (−53.1 %) | 15,416 → 13,472 ms (−12.6 %) | **+1.4 %** |

The single-pass `offset` scope is the lever (−41 % to −55 % of its own
self-time); `offset2_ex` gains less (−13 % to −25 %) because only its two
passes are decimated, not its wrapper. Guest-side fuel is **deterministic** and
falls 13.5 %–21.5 % on the classic cells but **rises 1.4 %–3.1 %** on the three
Arachne cells: the extra scan over already-sparse Arachne geometry costs more
instructions than the sweep saves there. Those three cells still win on wall
(5.7 %–23.4 %) because their prepass offsets are host-side, but the cost is real
and is disclosed rather than netted out.

## 8. What was not ported (disclosed, pre-existing)

Canonical `ClipperUtils` has three more behaviours this ticket did **not** touch.
All three predate it and none is introduced by it:

- **`c2_offset_as_group`'s per-path fallback** (`ClipperUtils.cpp`): canonical
  offsets path-by-path and unions when some CW hole is not enclosed by a CCW
  contour; PnP always uses Clipper2's grouped `AddPaths`. Input nesting is
  unchanged by decimation, so the predicate would evaluate the same before and
  after this port.
- **`c2_offset_setup`'s clamping** (`ClipperUtils.cpp`): canonical clamps
  `MiterLimit` to `>= 2` and `ArcTolerance` to `<= |delta| / 4`; PnP passes its
  knobs through unchanged.
- **Zero-delta conventions:** `offset(…, 0)` still returns the input verbatim
  (PnP's `inflate_paths_64` short-circuit), where canonical deduplicates exact
  repeats and runs a cleanup pass. At `delta1 == 0` the first pass of
  `offset2_ex` now deduplicates exactly as canonical does, but its second pass
  already ran Clipper2's cleanup, so the difference is not observable in output.

Guest modules that import `slicer-core::polygon_ops` (the reason the module trees
move): `classic-perimeters` and `arachne-perimeters` call the decimated offset
helpers; `infill-linker`, `lightning-infill`, `tree-support-planner`,
`tree-support` and `wave-overhangs` use the clip/boolean primitives only.

## 9. Route arithmetic (estimate, not a scoreboard)

Applying each cell's paired saving to the last full board
([t163's accelerated board](../t163-accelerated-board/reduced/summary.json), ordinary
arm) gives an **estimate** — the saving is paired and measured, the board is a
different window, and the product is not a ratio of record:

| cell | t163 ratio | × (1 − t169 saving) | estimated ratio | remaining cut |
| --- | --- | --- | --- | --- |
| benchy classic off | 1.782x | 0.9202 | **1.64x** | 39.0 % |
| benchy classic on | 2.802x | 0.8479 | **2.38x** | 57.9 % |
| benchy arachne off | 1.940x | 0.9427 | **1.83x** | 45.3 % |
| benchy arachne on | 2.829x | 0.8524 | **2.41x** | 58.5 % |
| base classic off | 1.389x | 0.7492 | **1.04x** | 3.9 % |
| base classic on | 2.710x | 0.7809 | **2.12x** | 52.7 % |
| base arachne off | 1.457x | 0.7657 | **1.12x** | 10.3 % |
| base arachne on | 2.618x | 0.7759 | **2.03x** | 50.8 % |

Two things follow. `base classic-off` moves from "1.5x short" to **3.9 % short**
— the first cell in this map to come within a single-digit cut of Orca. And the
binding cell moves to **`benchy arachne-on` at ~2.41x / 58.5 %**, which is a
benchy cell, so t128's "benchy classic-off cannot be won by prepass deletion
alone" survives in a stronger form: neither can benchy arachne-on.

## 10. Verification

- `cargo test -p slicer-core --features host-algos --no-fail-fast` — **92 test
  binaries, zero failures** (`target/test-output.log`).
- `cargo test -p classic-perimeters -p arachne-perimeters --no-fail-fast` —
  green (the two guest modules that call `offset2_ex` / `opening_ex` directly).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask build-guests --check` — exit 0; `--accelerated --check` — exit 0.
- `python verify.py` — PASS; `--self-test` rejects **6/6** negative controls.
- `python audit_t169.py --self-test` — PASS; rejects **4/4** negative controls.

## 11. Disclosures

- **The human granted the per-layer exception in-session** (2026-11-19/20), on
  the campaign's numbers, because the port compiles into per-layer guest modules
  and therefore changes computed geometry there. Without it the change would
  have been preserved as a proposal, per t152's precedent.
- **The reference set moves 8/8 and every section moves somewhere.** That is the
  port's point, not a side effect, but it means no earlier G-code hash in this
  map may be compared with a post-t169 one without a re-record.
- **Box load was light but not idle**: 1 %–7 % sampled at batch boundaries, one
  window. The null band is the instrument's own spread in that window
  (3.06 % any-pair / 1.99 % median wall); the smallest candidate median
  (`benchy arachne-off`, +5.74 %) clears it by 1.9x.
- **`--profile` walls are inflated and thread-summed**; they attribute the
  mechanism and are never quoted as speed.
- **Guest fuel rises 1.4 %–3.1 % on three Arachne cells** — a real cost of the
  extra decimation scan, disclosed in §7.
- **The route table in §9 is an estimate** built by multiplying a paired saving
  by a ratio from another window; t161 showed a 25 % box shift can move a
  single-arm ratio, so no cell's new ratio is a measurement of record until a
  fresh two-tool board is run.

## 12. Reproduce

```bash
python ../../../../../docs/specs/perf-vs-orca/evidence/t169-offset-decimation-flat-opening/run_t169.ps1 -Mode output -BatchId ref
python run_t169.ps1 -Mode ab -BatchId ab -Pairs 5
python run_t169.ps1 -Mode ab -NullControl -BatchId null -Pairs 3
python run_t169.ps1 -Mode output -Accelerated -BatchId accel
python run_t169.ps1 -Mode ab -Accelerated -BatchId accel -Pairs 3
python run_profile_t169.py
python audit_t169.py
python reduce_t169.py
python verify.py --self-test && python verify.py
```

`run_t169.ps1` stages nothing: the four arms must already exist under
`target/t169-{baseline,candidate}[-accelerated]/` (see the manifest in
`reduced/output.json`).
