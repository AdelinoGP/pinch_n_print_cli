# Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring

Type: task / measured candidate (100 % output-preserving exact host acceleration + literal standing 8-cell `0.80 %` paired ordinary + accelerated gate; zero geometry exceptions, zero gate waivers, zero WIT/guest changes)
Label: wayfinder:task
Status: resolved (2026-11-23; literal standing all-8-cell `0.80 %` gate PASSED 8/8 on the pre-registered pooled verdict, `448/448` measured runs byte-identical to `t182/reference.json`, zero reference change, zero geometry exceptions, zero gate waivers)
Assignee: agent (2026-11-23 — resolved)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [`ShellClassification` post-t172 residual attribution and speedup screen](180-shell-classification-post-t172-residual-screen.md) ([`evidence/t180-shell-classification-residual/FINDINGS.md`](../evidence/t180-shell-classification-residual/FINDINGS.md)), which attributed the post-t172 `PrePass::ShellClassification` residual across all 8 cells (`15/15` runs byte-identical to `t175/reference.json`, `100 %` call-count reconciliation against `t177`, `4/4` classic/arachne & supports-off/on invariance) and proved via 12-thread exact IR replay (`240/240` `benchy` + `496/496` `base` layers) that the four-mechanism `EXACT_BUNDLE` (`E1 + E2 + E3 + E4a`) cuts stage wall **`−56.06 %`** (`470.77 → 206.83 ms`, `benchy`) and **`−44.58 %`** (`3,307.57 → 1,833.22 ms`, `base`) with **`EXACT_BIT_IDENTICAL`** output.

## Question

**Does landing the 100 % output-preserving `EXACT_BUNDLE` (`E1 + E2 + E3 + E4a`) inside `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, `crates/slicer-core/src/algos/bridge_over_infill.rs`, and `crates/slicer-core/src/algos/prepass_slice.rs` pass the literal standing all-8-cell `0.80 %` paired ordinary + accelerated gate with `16/16` byte-identical output against `t175`'s active currency (and cross `< 1.0x` on both `benchy` supports-off cells on the next Orca board)?**

## What is already known (t180)

- **Replay-verified exact cuts (`12` threads, `EXACT_BIT_IDENTICAL` on 240/240 `benchy` and 496/496 `base` layers):**
  - **`benchy`:** `470.77 → 206.83 ms` (**`−56.06 %`**, **`−263.93 ms`**) — `B5b` `127.9 → 11.9 ms` (`−90.7 %`), `B5d` `118.7 → 29.7 ms` (`−75.0 %`), `B5c` `49.2 → 21.9 ms` (`−55.5 %`), `B2s` `18.8 → 5.4 ms` (`−71.3 %`), `B4` `36.9 → 29.1 ms` (`−21.1 %`).
  - **`base`:** `3,307.57 → 1,833.22 ms` (**`−44.58 %`**, **`−1,474.35 ms`**) — `B2s` `255.9 → 21.2 ms` (`−91.7 %`), `B5d` `558.4 → 77.3 ms` (`−86.2 %`), `B5b` `336.2 → 62.9 ms` (`−81.3 %`), `B5c` `718.5 → 274.5 ms` (`−61.8 %`), `B4` `338.5 → 285.8 ms` (`−15.6 %`).
- **Standing all-8-cell `0.80 %` gate & route projection (`ESTIMATE ONLY`, `t174 × t175 × t180`):**
  - Clears `0.80 %` in **8/8 cells** at **`6.94×–20.60×` floor** (`benchy-classic-off` **`15.63 %` / `19.54×`**, `benchy-arachne-off` **`15.16 %` / `18.94×`**, `benchy-classic-on` **`7.76 %` / `9.70×`**, `benchy-arachne-on` **`7.53 %` / `9.41×`**, `base-classic-off` **`16.48 %` / `20.60×`**, `base-arachne-off` **`15.25 %` / `19.07×`**, `base-classic-on` **`5.55 %` / `6.94×`**, `base-arachne-on` **`5.74 %` / `7.18×`**).
  - **Crosses `< 1.0x` on BOTH `benchy` supports-off cells alone** (`benchy-classic-off` `1.1438x → ~0.9650x` ord / `~0.9432x` acc; `benchy-arachne-off` `1.1548x → ~0.9798x` ord / `~0.9028x` acc), moves `base` supports-off to `~0.6420x / ~0.6713x` ord, and moves binding `benchy-classic-on` `1.5700x → ~1.4481x`.
- **What is intentionally EXCLUDED (keep `t184` 100 % exact):**
  - Do NOT include `L2` (per-`ExPolygon` negative offset in Pass-2 `project_shadow`, `0.174 / 0.825 µm` scanbeam drift; marginal gain clears `0.80 %` in only `2/8` cells), `L1` (sliver prefilter, falsified `13.97 µm`), `L3` (per-`ExPolygon` Pass-1 opening), `E4b` (rect-clipping `B4` anchors, falsified `89.9°`), or `E5` (bbox-filtering Pass-2 neighbors, exact but speed-neutral).

## Scope

1. **`E1` — Parallel Pass-2 shadow merges + parallel/skip `internal_solid_fill` diff (`crates/slicer-runtime/src/slice_postprocess_prepass.rs::compute_region_updates`):**
   - Group top-shadow contributions and bottom-shadow contributions by target `slice_idx` (preserving deterministic per-target slice/order semantics) and execute the per-target `union` merges in parallel across distinct target slices via Rayon.
   - Run the `internal_solid_fill = difference(top_solid_fill, top_solid_seed)` step in parallel across timeline slices, skipping `difference` immediately when `top_solid_seed` is empty (already `top_solid_fill.clone()`) OR when no Pass-2 top shadow contribution targeted that slice (`top_solid_fill == top_solid_seed` => `internal_solid_fill` is empty, eliminating `68.9 %` `benchy` / `75.7 %` `base` redundant diffs).
2. **`E2` — Lazy internal-bridge precompute, monotonic Z-cluster parallel commit, cached region angle invariants, BVH/$O(S)$ `determine_bridging_angle`, and dead final `exp_diff` skip (`crates/slicer-runtime/src/slice_postprocess_prepass.rs::gate_internal_bridge_sites` & `compute_internal_bridge_layer`, `crates/slicer-core/src/algos/bridge_over_infill.rs::determine_bridging_angle` & `internal_bridge_angles`):**
   - **Lazy Phase-B precompute (`gate_internal_bridge_sites` / `compute_internal_bridge_layer`):**
     - Run `qualify_internal_bridge_surface` per `internal_solid_fill` surface *first* (depends only on `(s, unsupported, qualify_spacing_mm, nofilter)`, independent of `deep_infill_area`). If zero surfaces qualify, return `None` without calling `gather_areas_w_depth` or `offset(&deep, +1.5s)`.
     - For layers with qualifying candidates, check `iua` (`offset(windowed_deep, -4.5 * qsp)`) using a **windowed** `gather_areas_w_depth` clipped to `internal_bridge_offset_window` (padded with exact erosion reach) before paying for full-layer `gather_areas_w_depth` + full-layer `offset(&deep, +1.5 * qsp)`. Only compute whole-layer `deep_infill_area` and `dca = offset(&deep, +1.5 * qsp)` when at least one candidate polygon intersects `iua`! (Eliminates `72.1 %` `benchy` / `93.2 %` `base` wasted `None` precomputes, including `ib_comp_pos_offset` — the #1 Clipper site on `base`!)
   - **Monotonic `None` skip + Z-cluster parallel Phase-B commit (`gate_internal_bridge_sites`):**
     - **Soundness proof for `None` skip:** subtracting lower-layer committed bridges (and/or adding them to lower `not_sparse_infill` before `closing_ex`) strictly shrinks `deep_infill_area` (`deep' ⊆ deep`), so `iua' ⊆ iua` and `dca' ⊆ dca`; therefore any layer whose unsubtracted precompute returned `None` is guaranteed `None` after commit subtraction (skips `6/7` serial recomputes on `base`!).
     - **Z-cluster parallelism:** partition surviving `precomputed.is_some()` layers into independent Z-clusters using the **full dependency radius** `dz <= prep.target_flow_height * BRIDGE_FLOW_HEIGHT_FACTOR + SCALED_EPSILON_MM as f32` (`1.6 * tfh`, matching `cluster_candidate_layers` and `depth_window_start` — sound even under variable/adaptive layer heights because `build_bridge_depth_layers` reads `lower.bridge_areas` across `1.6 * tfh`!). Process independent Z-clusters in parallel via Rayon and publish results back to `slices`.
     - **Exactness inside a multi-layer cluster:** when a layer inside a cluster has committed lower bridges within its window, verify exact handling of `build_bridge_depth_layers`'s `lower.bridge_areas` (re-gather with committed lower bridges appended to `not_sparse_infill`, or verify `remove_filled_polygons_on_lower_layers` equivalence with an explicit unit test on touching `<= 2 unit` seams; because `None` layers are already skipped, only genuinely overlapping surviving layers enter this path).
   - **Cached region angle invariants + skip dead final `exp_diff` (`crates/slicer-core/src/algos/bridge_over_infill.rs::internal_bridge_angles`):**
     - Hoist the region-invariant `total_fill` anchor polylines (`boundary_anchor_polylines` on `total_fill_area` / `not_sparse` + `expansion_area` setup) out of per-candidate loops and reuse across commit recomputes.
     - Do NOT compute `remaining_expansion = difference(&remaining_expansion, &expanded)` on the final candidate of `internal_bridge_angles` (its result is immediately dropped at loop exit — saves `102.6 ms` on `base` where `max_cands = 1`).
   - **Exact BVH + $O(S)$ angular vote sweep in `determine_bridging_angle` (`crates/slicer-core/src/algos/bridge_over_infill.rs::determine_bridging_angle`):**
     - Replace the brute-force $O(S \times A)$ nearest-anchor scan in `nearest_anchor_angle` with an exact 2D segment-bbox BVH (exact squared-distance bound pruning, matching canonical `AABBTreeLines::LinesDistancer` semantics) and replace the $O(S^2)$ angular vote accumulation with an exact $O(S \log S)$ sorted-angle sliding window preserving exact floating-point tie-breaking and identical output degrees.
3. **`E3` — Exact rectangular windowing in `unsupported_span_areas` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs::gate_internal_bridge_sites` & `crates/slicer-core/src/algos/bridge_over_infill.rs::unsupported_span_areas`):**
   - In Phase A of `gate_internal_bridge_sites`, compute the bounding box of `candidate.internal_solid_fill` and expand it by a conservative exact reach covering every mitered offset in `unsupported_span_areas` + `qualify_internal_bridge_surface` (specifically `margin_mm >= (1.0 + 3.0 * expansion_multiplier + 1.0) * qualify_spacing_mm` — i.e., `11.0 * qualify_spacing_mm` at `expansion_multiplier = 3.0`, scaled dynamically with `expansion_multiplier` and miter limit `3.0` so non-default `nofilter` or custom multipliers stay equally sound!).
   - Clip `lower_fills` and `lower_solids` via exact `slicer_core::polygon_ops::intersection(_, &[window_rect])` before calling `unsupported_span_areas` (skipping the clip if the input bbox already fits inside `window_rect`). Eliminates `74–88 %` of distant vertices and kills the `benchy` `L1` `128.9 ms` `#3DBenchy` letter-hole straggler (`B5b` `127.9 → 11.9 ms`).
4. **`E4a` — Exact $O((C+E)\log(C+E))$ angular prefix-sum sweep in `detect_bridging_direction_deg` (`crates/slicer-core/src/algos/prepass_slice.rs::detect_bridging_direction_deg`):**
   - Replace the quadratic $O(C \times E)$ angle-vs-edge scoring loop with an exact sorted-angle two-pointer / prefix-sum sweep over the exact same candidate angles and edge orientations, preserving exact tie-breaking and bit-identical `f32` return values.
5. **Required unit & regression tests (falsify every soundness boundary):**
   - `slicer-core`:
     - `unsupported_span_areas` windowed vs unwindowed equivalence across high-miter acute corners, holes, `expansion_multiplier = 1.0` and `3.0`, and polygons straddling the `11 * spacing` window boundary.
     - `determine_bridging_angle` BVH + sorted-window vs brute-force reference equivalence across multi-anchor, equidistant tie-break, and collinear edge cases.
     - `internal_bridge_angles` multi-candidate (`>1` candidates) negative control proving `remaining_expansion` STILL updates between candidate `0` and candidate `1` while skipping only the final iteration.
     - `detect_bridging_direction_deg` fast sweep vs brute-force reference equivalence across wrap-around `0° / 180°` boundaries and exact tie-breaks.
   - `slicer-runtime`:
     - `gate_internal_bridge_sites` variable-layer-height negative control (e.g., `0.15 mm` layers where `L - 2` sits at `0.30 mm <= 1.6 * 0.20 = 0.32 mm`) proving `1.6 * tfh` clustering keeps dependent layers in the same cluster and reproduces serial results bit-for-bit; plus `None`-monotonicity and touching `<= 2 unit` `closing_ex` seam check.
     - `compute_region_updates` multi-step Pass-2 merge ordering and `top_solid_seed == top_solid_fill` skip equivalence.
6. **Standing 8-cell paired ordinary + accelerated + null-band gate (unsandboxed, `RAYON_NUM_THREADS=12`):**
   - Build both `pnp-ordinary` and `pnp-accelerated` (note: **zero guest edits**, so both `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` MUST exit `0` untouched!).
   - **Output parity (fail-closed, zero reference change):** baseline AND candidate MUST reproduce `t175`'s active `reference.json` byte-for-byte on **16/16** `(8 cells × {ordinary, accelerated})` runs (`exit 0`, `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics). Any hash divergence is a bug — do NOT update `reference.json`.
   - **Timing (literal standing 8-cell gate):** 5 interleaved pairs/cell ordinary, 3-pair null control, 3-pair accelerated corroboration; all 8 cells clear the `0.80 %` paired wall floor outside their null band with process CPU corroborating (`lower` in 8/8) and accelerated corroborating.
   - **Pre-commit code gates:** `cargo test -p slicer-core --features host-algos --no-fail-fast`, targeted `slicer-runtime` prepass tests, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, `cargo xtask build-guests --check`, and `cargo xtask build-guests --accelerated --check`.

## Acceptance

- Clean, tested production implementation strictly within `crates/slicer-core` and `crates/slicer-runtime` (zero guest/WIT changes; both guest freshness checks exit `0`).
- Evidence directory `docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/` with reduced paired ordinary + null + accelerated campaign JSONs, `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`verify.py --self-test` exit `0`).
- Updated `## Answer`, `map.md` update, and clean commit after all gates pass (zero reference-set change).

## Answer

**Resolved 2026-11-23 — landed.** The literal standing all-8-cell `0.80 %`
gate passes **8/8** on the pre-registered verdict of record: two pooled ordinary
windows, 10 interleaved pairs and 6 null pairs per cell
([`PREREGISTRATION.md`](../evidence/t184-shell-classification-exact-accel/PREREGISTRATION.md)).

- **Wall:** **`+4.99…+16.35 %`**, 78/80 pairs won.
- **CPU:** lower in 8/8 cells, **`+4.85…+10.68 %`**.
- **Accelerated arm:** `+7.76…+18.65 %` wall, CPU lower in 8/8.
- **Output:** all **`448/448`** measured runs are byte-identical to
  `t182/reference.json`, with zero degraded runs, zero errors and zero code-1200
  diagnostics. The reference set is unchanged.
- **Stage:** `host:shell_classification` falls **`−55…−57 %`** on benchy
  (`~495 → ~215 ms`) and **`−33…−37 %`** on base (`~3.8 → ~2.5 s`).

**Disclosed:**

- Window 1 passed 7/8 (base-classic-on inside a noisy null median) and window 2
  passed 8/8.
- The pooled strict any-pair lens is 7/8; base-classic-on is inside window 1's
  single `11.73 %` null pair.
- Every guest WASM is recompiled because the guests depend on slicer-core.
  Module sets and call counts are identical, but fuel drifts by
  `−0.70…−1.36 %`. A host-only batch (candidate host on the baseline modules)
  attributes the base CPU cut to the host change.

**What landed differs from the ticket's scope in four exactness-driven ways:**

1. **E3:** drops whole lower-layer polygons and holes outside the window instead
   of rect-clipping them. Rect clipping creates rounded crossing vertices, so its
   bit-identity was only empirical.
2. **E2:** replaces the geometric "`None` stays `None`" skip with dependency
   clusters plus *validated speculation*. The speculation recomputes every
   dependent layer in parallel, then confirms in order that each commit equals
   its precomputed one, falling back to the sequential loop on the first
   mismatch. There were 0 misses in all 8 cells. The cluster radius comes from
   the actual `depth_window_start` / `filled_window_start` windows:
   `BRIDGE_FLOW_HEIGHT_FACTOR` is `1.0`, not the `1.6` this ticket assumed. A
   layer whose window reaches past the current cluster's head merges the earlier
   clusters.
3. **E4a:** uses the prefix-sum cost only as a filter, then re-scores every
   candidate within a proven error bound with the exact reference cost. A
   selector that trusts the estimate fails the new quarter-turn tie test.
4. **E1:** skips the internal-solid difference only for slices no top shadow
   reached (`A \ A`). It does not take the `seed empty ⇒ clone` shortcut.

The windowed deep-gather check was dropped: once qualify-first was in, the t180
replay measured no gain from it.

**Route (ESTIMATE ONLY).** Both benchy supports-off cells drop below `1.0x`:
`0.877x` / `0.885x` on the ordinary arm. The binding cells are
benchy-classic-on (`~1.33x`) and benchy-arachne-on (`~1.17x`).

Evidence: [`FINDINGS.md`](../evidence/t184-shell-classification-exact-accel/FINDINGS.md),
`verify.py` PASS, `--self-test` 16/16.
