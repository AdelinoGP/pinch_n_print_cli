# Ticket 157 — `needs_support` Predicate Attribution & Fast-Path Pricing (`FINDINGS.md`)

- **Ticket:** [`docs/specs/perf-vs-orca/issues/157-needs-support-predicate-attribution.md`](../../issues/157-needs-support-predicate-attribution.md)
- **Kind:** Attribution & pricing only (`--instrument-stderr` + temporary `PNP_T157_PROBE=1` probe, reverted before ticket close; zero wall claims denominated from probe runs).
- **HEAD commit:** `c74e64ec` (`feature/perf-vs-orca`)

---

## 1. Exhaustive Workspace Reader Census of `needs_support` / `needs-support`

Ticket `155` (§4) stopped at the SDK macro expansion boundary [`__slicer_adapt_slice_regions`](../../../../crates/slicer-macros/src/lib.rs) (`crates/slicer-macros/src/lib.rs`), which copies WIT `slice-region::needs-support` into [`SliceRegionView::set_needs_support`](../../../../crates/slicer-sdk/src/views.rs). Tracing one step further to [`SliceRegionView::needs_support`](../../../../crates/slicer-sdk/src/views.rs) across the entire workspace (`crates/` and `modules/`) establishes:

| Layer / Crate | Symbol / Site | Role | Production Decision Consumer? |
|---|---|---|---|
| `slicer-runtime` | [`commit_support_analysis_builtin`](../../../../crates/slicer-runtime/src/builtins/support_analysis_producer.rs) (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) | Calls `SliceRegionView::from_ir(region, 0.0, Vec::new()).derive_needs_support(surface_classification.as_deref())` directly in a parallel `par_iter()` when `enable_support == true`, then gates angle-thresholded contact detection on `support_type.is_auto() && *needs_support`. | **YES — sole production consumer** |
| `slicer-wasm-host` | [`prepare_regions`](../../../../crates/slicer-wasm-host/src/marshal/prepared.rs) (`crates/slicer-wasm-host/src/marshal/prepared.rs`) | Calls `view.derive_needs_support(surface_classification)` when building `PreparedRegionData` for [`prepare_perimeter_source_region_views`](../../../../crates/slicer-runtime/src/layer_executor.rs) (`Layer::Perimeters`) and [`prepare_ordinary_region_views`](../../../../crates/slicer-runtime/src/layer_executor.rs) (`Layer::Infill`, after `commit_perimeter` invalidates the cached views). | **NO** — populates `PreparedRegionData::needs_support` only |
| `slicer-wasm-host` | [`slice_region_to_wit_from_prepared`](../../../../crates/slicer-wasm-host/src/marshal/in_.rs) & [`prepared_to_region_view`](../../../../crates/slicer-wasm-host/src/marshal/native.rs) | Copies `prepared.needs_support` into WIT `slice-region::needs-support` and native `SliceRegionView::set_needs_support`. | **NO** — transport only |
| `slicer-macros` | [`__slicer_adapt_slice_regions`](../../../../crates/slicer-macros/src/lib.rs) (`crates/slicer-macros/src/lib.rs`) | Copies WIT `r.needs_support` into `SliceRegionView::set_needs_support`. | **NO** — transport only |
| `tree-support` | [`TreeSupport::run_support`](../../../../modules/core-modules/tree-support/src/lib.rs) (`modules/core-modules/tree-support/src/lib.rs`) | **Retired in Packet 224:** comment in `TreeSupport::run_support` explicitly states `SupportPaintPolicy::DefaultEligible` no longer gates on `region.needs_support()` because `PrePass::SupportAnalysis` already gates auto-support contact detection on `needs_support` when producing `SupportAnalysisIR`. | **NO (retired)** |
| `traditional-support` | [`TraditionalSupport::run_support`](../../../../modules/core-modules/traditional-support/src/lib.rs) (`modules/core-modules/traditional-support/src/lib.rs`) | Never calls `region.needs_support()`. | **NO** |
| All other guests (`modules/core-modules/*`) | — | Zero references to `needs_support`. | **NO** |
| Tests (`crates/slicer-runtime/tests/*`, `crates/slicer-sdk/tests/*`) | `region_preparation_sharing_tdd.rs`, `region_eligibility_tdd.rs`, `view_seam_identity_tdd.rs`, `source_identity_repair_tdd.rs`, `blackboard_layer_arena_tdd.rs`, `layer_module_tdd.rs` | Contract and unit assertions on `PreparedRegionData::needs_support` and `SliceRegionView::derive_needs_support`. | Test-only |

### Why Ticket 156 Dropped and Why Option A Is Strictly Superior

Ticket `156` proposed carrying `needs_support` across `commit_perimeter` so `Layer::Infill` would not recompute `derive_needs_support`. That shape had two structural limitations:
1. It only removed **1 of the 3** evaluations per region on supports-on (`Layer::Infill`), leaving `Layer::Perimeters` (`25.88–30.20 s` thread on `base`) and `PrePass::SupportAnalysis` (`24.52 s` thread on `base` supports-on) paying the full ~50–61 ms/call Clipper2 `intersection_ex`.
2. On `3dbenchy.stl`, the single `Layer::Infill` preparation ceiling is only `131.4–206.2 ms` (`t154` [`carriers.json`](../t154-benchy-per-layer/carriers.json)), so cutting 1 of 3 evaluations could not clear the `benchy` gate on its own.

By contrast, **Option A** — an exact fast-path boolean intersection predicate inside [`SliceRegionView::derive_needs_support`](../../../../crates/slicer-sdk/src/views.rs) — accelerates **all three** call sites (`PrePass::SupportAnalysis`, `Layer::Perimeters`, and `Layer::Infill`) by **91x–134x**, preserves every WIT/SDK/test contract without schema or lifecycle changes, and leaves the Clipper2 boolean kernel (`slicer_core::polygon_ops::intersection_ex`) as the exact fallback when no fast witness is found.

---

## 2. Exact Fast-Path Predicate Design & Equivalence Proof

[`SliceRegionView::derive_needs_support`](../../../../crates/slicer-sdk/src/views.rs) asks a **boolean** question: for a given region's `self.polygons: &[ExPolygon]` ($A$) and each non-empty `overhang.xy_footprint: &[ExPolygon]` ($B$), is $\text{area}(A \cap B) > 0$?

Because `intersection_ex(A, B)` (`Clipper2` boolean intersection on closed `ExPolygon`s) is non-empty if and only if $A$ and $B$ share positive 2D area:
1. **Strict bbox prefilter (`false` fast path):** Compute axis-aligned bounding boxes `(min_x, min_y, max_x, max_y)` for each contour in $A$ (plus $A$'s union bbox) and filter contours of $B$ whose bbox strictly overlaps $A$'s union bbox (`a.min_x < b.max_x && b.min_x < a.max_x && a.min_y < b.max_y && b.min_y < a.max_y`). Pairwise strict bbox overlap builds `overlapping_pairs`. If `overlapping_pairs` is empty, $\text{area}(A \cap B) = 0 \implies \text{false}$ (exact: touching only along a 1D bbox boundary has zero 2D area).
2. **Strict-interior point-in-`ExPolygon` witness (`true` fast path):** Any point $p \in B$ that lies **strictly inside** an `ExPolygon` $A_i \in A$ (strictly inside $A_i$'s outer contour and strictly outside all holes of $A_i$, with boundary/vertex hits classified as `0` via exact `i128` cross-product and never accepted as a witness) has an open neighborhood $U \ni p$ inside $A_i$. Since $p$ is a vertex of a simple non-degenerate polygon $B_j \in B$, $U \cap \text{int}(B_j)$ has positive 2D area, so $\text{area}(A_i \cap B_j) > 0 \implies \text{true}$ (exact). Symmetrically, any vertex of $A_i$ strictly inside $B_j$ proves $\text{area}(A_i \cap B_j) > 0 \implies \text{true}$.
   - Pass 1 samples up to 8 evenly spaced contour vertices of $B_j$ in $A_i$ across `overlapping_pairs`.
   - Pass 2 samples up to 8 evenly spaced contour vertices of $A_i$ in $B_j$.
   - Pass 3 scans remaining contour vertices of $B_j$ in $A_i$ and $A_i$ in $B_j$.
3. **Exact Clipper2 fallback on overlapping subset:** If neither bbox disjointness nor a strict-interior vertex witness resolves the pair (e.g., two polygons cross edge-over-edge with all vertices outside each other, or share only boundary contact), fall back to `!slicer_core::polygon_ops::intersection_ex(&a_sub, &b_sub).is_empty()` restricted to the `ExPolygon`s in `overlapping_pairs`.

### Dual-Mode Equivalence Results Across All 4 Benchmark Cells

Across all **3,680** production calls in `dual` mode (`240 × 2 = 480` on `benchy-classic-off`, `240 × 3 = 720` on `benchy-classic-on`, `496 × 2 = 992` on `base-classic-off`, `496 × 3 = 1,488` on `base-classic-on`, plus all unit/contract tests):
- **Predicate mismatches (`fast_val != full_val`):** **`0 / 3,680`** (`100.0%` exact agreement).
- **G-code SHA-256 parity:** All **8** probe runs (`4` cells × `dual` and `fast` modes) matched [`docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/reference.json`](../t151-skirt-hull-contour/reference.json) byte-for-byte (`degraded=false`, `fatal_error_count=0`, `non_fatal_error_count=0`).
- **Branch distribution:**
  - `benchy` (`240` layers, `a_verts_mean = 1,890.1`, `b_polys = 201`, `b_verts = 6,107`): `240 / 240` calls per site resolved by `b_vertex_in_a_sampled` in an average of **19.78** point-in-polygon tests (`0` Clipper2 fallbacks).
  - `base` (`496` layers, `a_verts_mean = 5,155.7`, `b_polys = 994`, `b_verts = 26,633`): `492 / 496` calls per site resolved by `b_vertex_in_a_sampled` and `4 / 496` by `a_vertex_in_b_sampled` in an average of **37.65** point-in-polygon tests (`0` Clipper2 fallbacks).

---

## 3. Per-Site Thread-Time Attribution (`dual` vs `fast` Probe)

All numbers below are reduced from `target/t157-probe/<cell>.<mode>.jsonl` into [`probe.json`](probe.json) and [`summary.json`](summary.json).

### 3.1 `derive_needs_support` Call-Site Breakdown (`dual` mode)

| Cell | Caller Site | Calls | Full `intersection_ex` Thread Sum (ms) | Mean Full (µs/call) | Fast Predicate Thread Sum (ms) | Mean Fast (µs/call) | Thread Time Saved (ms) | Site Speedup | Mismatches |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | `Layer::Perimeters` | 240 | 793.102 | 3,304.6 | 8.706 | 36.3 | 784.396 | **91.1x** | 0 |
| `benchy-classic-off` | `Layer::Infill` | 240 | 778.191 | 3,242.5 | 8.570 | 35.7 | 769.621 | **90.8x** | 0 |
| `benchy-classic-off` | **Total (`2` sites)** | **480** | **1,571.293** | **3,273.5** | **17.276** | **36.0** | **1,554.017** | **90.95x** | **0** |
| `benchy-classic-on` | `PrePass::SupportAnalysis` | 240 | 782.187 | 3,259.1 | 8.250 | 34.4 | 773.937 | **94.8x** | 0 |
| `benchy-classic-on` | `Layer::Perimeters` | 240 | 814.775 | 3,394.9 | 8.856 | 36.9 | 805.919 | **92.0x** | 0 |
| `benchy-classic-on` | `Layer::Infill` | 240 | 806.262 | 3,359.4 | 8.869 | 37.0 | 797.393 | **90.9x** | 0 |
| `benchy-classic-on` | **Total (`3` sites)** | **720** | **2,403.224** | **3,337.8** | **25.975** | **36.1** | **2,377.249** | **92.52x** | **0** |
| `base-classic-off` | `Layer::Perimeters` | 496 | 25,875.565 | 52,168.5 | 228.424 | 460.5 | 25,647.141 | **113.3x** | 0 |
| `base-classic-off` | `Layer::Infill` | 496 | 25,776.062 | 51,967.9 | 228.496 | 460.7 | 25,547.566 | **112.8x** | 0 |
| `base-classic-off` | **Total (`2` sites)** | **992** | **51,651.627** | **52,068.2** | **456.920** | **460.6** | **51,194.707** | **113.04x** | **0** |
| `base-classic-on` | `PrePass::SupportAnalysis` | 496 | 24,522.552 | 49,440.6 | 203.469 | 410.2 | 24,319.083 | **120.5x** | 0 |
| `base-classic-on` | `Layer::Perimeters` | 496 | 30,203.772 | 60,894.7 | 278.568 | 561.6 | 29,925.204 | **108.4x** | 0 |
| `base-classic-on` | `Layer::Infill` | 496 | 30,065.682 | 60,616.3 | 224.787 | 453.2 | 29,840.895 | **133.8x** | 0 |
| `base-classic-on` | **Total (`3` sites)** | **1,488** | **84,792.006** | **56,983.9** | **706.824** | **475.0** | **84,085.182** | **119.96x** | **0** |

### 3.2 Impact on `prepare_dispatch_region_views` (`prep_views`) and Stage Thread Sums (`dual` vs `fast` Mode)

Comparing `prep_views_ms` (`prepare_dispatch_region_views` in [`crates/slicer-runtime/src/layer_executor.rs`](../../../../crates/slicer-runtime/src/layer_executor.rs)) between the `dual` pass and the `fast`-only pass (`PNP_T157_FAST_PREDICATE=1`) directly isolates `derive_needs_support` from the remaining per-layer view preparation work (`hydrate_shell_Metadata` + `hydrate_bridge_metadata`):

| Cell | Stage | `prep_views_ms` (`dual`) | `prep_views_ms` (`fast`) | `prep_views_ms` Reduction | `module_complete` Thread Sum (`dual` → `fast`, ms) |
|---|---|---:|---:|---:|---:|
| `benchy-classic-off` | `Layer::Perimeters` | 1,060.972 | 240.997 | **-819.975 (-77.3%)** | 2,534 → 1,669 (**-865 ms**) |
| `benchy-classic-off` | `Layer::Infill` | 1,041.621 | 239.538 | **-802.083 (-77.0%)** | 1,553 → 809 (**-744 ms**) |
| `benchy-classic-on` | `Layer::Perimeters` | 1,077.530 | 260.066 | **-817.464 (-75.9%)** | 2,556 → 1,782 (**-774 ms**) |
| `benchy-classic-on` | `Layer::Infill` | 1,063.062 | 253.214 | **-809.848 (-76.2%)** | 1,578 → 873 (**-705 ms**) |
| `base-classic-off` | `Layer::Perimeters` | 27,727.088 | 1,946.998 | **-25,780.090 (-93.0%)** | 39,805 → 14,683 (**-25,122 ms**) |
| `base-classic-off` | `Layer::Infill` | 27,603.944 | 1,913.939 | **-25,690.005 (-93.1%)** | 30,151 → 4,596 (**-25,555 ms**) |
| `base-classic-on` | `Layer::Perimeters` | 32,524.999 | 2,005.586 | **-30,519.413 (-93.8%)** | 47,155 → 15,216 (**-31,939 ms**) |
| `base-classic-on` | `Layer::Infill` | 32,350.084 | 2,005.914 | **-30,344.170 (-93.8%)** | 36,504 → 5,384 (**-31,120 ms**) |

And in `PrePass::SupportAnalysis` (`sa_eligibility` parallel pre-pass wall inside [`commit_support_analysis_builtin`](../../../../crates/slicer-runtime/src/builtins/support_analysis_producer.rs)):
- **`benchy-classic-on`:** `sa_eligibility` wall drops from **`72.415 ms`** (`dual`) to **`16.128 ms`** (`fast`), and the `PrePass::SupportAnalysis` stage bracket drops from `760 ms` to `697 ms` (`-63 ms`).
- **`base-classic-on`:** `sa_eligibility` wall drops from **`2,100.514 ms`** (`dual`) to **`41.509 ms`** (`fast`, a **50.6x** wall speedup on the eligibility pre-pass), and the `PrePass::SupportAnalysis` stage bracket drops from `15,558 ms` to `13,472 ms` (`-2,086 ms` wall).

---

## 4. Priced Wall-Equivalent Savings Against the `t154` Ledger

Using `t154`'s carry-forward effective parallelism and cell wall baselines ([`docs/specs/perf-vs-orca/evidence/t154-benchy-per-layer/arithmetic.json`](../t154-benchy-per-layer/arithmetic.json) and [`carriers.json`](../t154-benchy-per-layer/carriers.json)), [`price.py`](price.py) computes the ledger-priced wall-equivalent savings (`ESTIMATE`, not a ratio of record until `t158` runs a clean 3-iteration `hyperfine` scoreboard):

| Cell | `t154` Baseline PnP Wall (s) | Orca Wall (s) | `t154` Ratio | `t154` Per-Layer $P_{\text{eff}}$ | Per-Layer Thread Saved (ms) | Priced Per-Layer Wall Saved (ms) | Priced `SupportAnalysis` Wall Saved (ms) | **Total Priced Wall Saved (ms / s)** | **% of Baseline PnP Wall** | **Projected PnP Wall (s)** | **Projected Ratio (`ESTIMATE`)** |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | 3.8431 | 1.915 | 2.0068x | 6.74 | 1,554.0 | 230.6 | 0.0 | **230.6 ms (`0.2306 s`)** | **6.00%** | 3.6125 | **1.8864x** |
| `benchy-classic-on` | 6.2786 | 2.442 | 2.5711x | 9.65 | 1,603.3 | 166.1 | 71.2 | **237.3 ms (`0.2373 s`)** | **3.78%** | 6.0413 | **2.4739x** |
| `base-classic-off` | 24.5071 | 13.414 | 1.8270x | 10.91 | 51,194.7 | 4,692.5 | 0.0 | **4,692.5 ms (`4.6925 s`)** | **19.15%** | 19.8146 | **1.4772x** |
| `base-classic-on` | 69.5401 | 21.598 | 3.2197x | 11.18 | 59,766.1 | 5,345.8 | 2,068.6 | **7,414.4 ms (`7.4144 s`)** | **10.66%** | 62.1257 | **2.8765x** |

### Impact on `t154`'s Per-Layer Residual Requirement on `base-classic-on`

In `t154` ([`map.md` §Where the 1.0x gap lives](../../map.md)), `base-classic-on` at canonical prepass (`10.445 s`) still landed at **`1.0508x`** (`22.694 s vs 21.598 s`), requiring **`1.0964 s`** (`8.44%` of `per_layer`) from outside `prepass` to reach `1.0x`.
- The `5.3458 s` per-layer wall-equivalent saving priced above on `base-classic-on` (and `5,693 ms` observed per-layer phase delta in the attribution probe: `12,585 ms → 6,892 ms`) is **4.88x larger** than the entire `1.0964 s` non-prepass residual needed on `base-classic-on`.
- With `5.3458 s` removed from `base-classic-on` `per_layer`, `base-classic-on` at canonical prepass drops from `1.0508x` (`22.694 s`) to **`0.8033x` (`17.349 s vs 21.598 s`)**, flipping `base-classic-on` from a `1.0964 s` deficit into a **`4.249 s` margin below `1.0x`**.

---

## 5. Stop-Rule Evaluation & Graduation of Ticket 158

Ticket `157`'s stop rule specifies:
> Graduate a Phase-2 implementation ticket (`158`) if and only if the priced wall-equivalent saving is **$\ge 1.5\text{ s}$ on `base`** (`classic-off` or `classic-on`) **or $\ge 150\text{ ms}$ on `benchy`** (`classic-off` or `classic-on`) at `t154`'s effective parallelism.

Both gates pass with wide margin:
- **`base` gate ($\ge 1.5\text{ s}$):** Passes at **`4.6925 s`** (`base-classic-off`, **3.1x** threshold) and **`7.4144 s`** (`base-classic-on`, **4.9x** threshold).
- **`benchy` gate ($\ge 150\text{ ms}$):** Passes at **`230.6 ms`** (`benchy-classic-off`, **1.54x** threshold) and **`237.3 ms`** (`benchy-classic-on`, **1.58x** threshold).

Accordingly, **Ticket 158** ([`docs/specs/perf-vs-orca/issues/158-needs-support-fast-predicate.md`](../../issues/158-needs-support-fast-predicate.md)) is graduated as the next frontier ticket to land Option A in production code (`SliceRegionView::derive_needs_support` in `crates/slicer-sdk/src/views.rs`) and price it with a clean 3-iteration `hyperfine` pass.
