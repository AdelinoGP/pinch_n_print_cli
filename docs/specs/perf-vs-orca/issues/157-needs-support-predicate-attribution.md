# The surviving `needs_support` predicate: reader census, three-site cost, and exact fast-path probe

Type: task / Phase-1 attribution + probe
Label: wayfinder:task
Status: resolved (2026-11-16; `commit_support_analysis_builtin` is the sole production decision consumer of `derive_needs_support` after Packet 224 retired `region.needs_support()` from `TreeSupport::run_support`; exact fast-path boolean predicate verified at **0 / 3,680** mismatches across all 4 probed cells and unit/contract suites, cutting `derive_needs_support` by **91.0x–120.0x** and pricing at **4.693 s** wall-equivalent on `base classic-off` / **7.414 s** on `base classic-on` / **230.6–237.3 ms** on `benchy`; probe reverted and [158](158-needs-support-fast-predicate.md) graduated)
Assignee: current T3 Code session (wayfinder), 2026-11-16 — the human said "Continue the perf vs orca map" (no ticket named); 1–156 are resolved and the frontier is empty, so the map's own signpost in Not yet specified (`map.md` item `(d) the surviving predicate`) graduates as 157 (AFK Phase-1 attribution + probe, following the t149/t152/t154 precedent)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Prepared-region memo across the fill sync](156-prepared-memo-across-fill-sync.md)
  and [The `Layer::Infill` first-dispatch block and base classic-on's 1.096 s
  residual](155-infill-stage-block-base-on-residual.md) — t155 proved that
  `prepare_regions` (`crates/slicer-wasm-host/src/marshal/prepared.rs`) spends
  **94.3–94.4 %** of its thread-time inside `SliceRegionView::derive_needs_support`
  (`crates/slicer-sdk/src/views.rs`), paid twice per layer in the per-layer phase
  (`Layer::Perimeters` + `Layer::Infill`, **49,021.2 ms** thread on base
  classic-off and **49,465.0 ms** on base classic-on) and a third time per layer
  on supports-on in `commit_support_analysis_builtin`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, **23,346.8
  ms** thread on base classic-on; **72,811.8 ms** total across 1,488 calls).
  t156 tested preserving `LayerArena`'s prepared-region memo across the fill
  sync — removing only the second per-layer derivation while leaving the
  `Layer::Perimeters` derivation and the `PrePass::SupportAnalysis` derivation
  intact — and closed **DROP** on the quiet 5-pair eight-cell wall gate (benchy
  Arachne-on **−2.94 %**, base Arachne-on **+0.17 %**), leaving the surviving
  predicate in the fog.

## Question

Why does `SliceRegionView::derive_needs_support` cost **~49 ms per call** on
`base.stl` (and what does it cost on `3dbenchy.stl`, which t155 never probed),
which production paths actually read its output, and can an exact cheaper
boolean predicate collapse **all three** call sites (`PrePass::SupportAnalysis`,
`Layer::Perimeters`, and `Layer::Infill`) without a schema or arena-lifecycle
change?

Decide and record, in order:

1. **Reader census, by symbol (sharpening t155 §4).** t155 stopped at the guest
   proc-macro adapter `__slicer_adapt_slice_regions`
   (`crates/slicer-macros/src/lib.rs`), which copies WIT
   `slice-region::needs-support` into `SliceRegionView::set_needs_support`. Trace
   the value one step further — to `SliceRegionView::needs_support`
   (`crates/slicer-sdk/src/views.rs`) — across every crate in `crates/` and every
   guest in `modules/`. Establish whether **any** production module in `modules/`
   reads `SliceRegionView::needs_support()`, or whether Packet 224's removal of
   `region.needs_support()` from `TreeSupport::run_support`
   (`modules/core-modules/tree-support/src/lib.rs`) left
   `commit_support_analysis_builtin`
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) as the
   sole production decision consumer in the entire pipeline. Name the contract
   and unit tests that pin `PreparedRegionData::needs_support` and
   `SliceRegionView::derive_needs_support`.
2. **Operand anatomy and three-call-site census on both `base` and `benchy`.**
   Measure `derive_needs_support` at all three call sites —
   `PrePass::SupportAnalysis` (`commit_support_analysis_builtin`),
   `Layer::Perimeters` first dispatch (`prepare_regions`), and `Layer::Infill`
   first dispatch (`prepare_regions`) — on `base classic-off`, `base classic-on`,
   `benchy classic-off`, and `benchy classic-on`. Record the operand geometry of
   `overhang.xy_footprint` and `region.polygons` (`ExPolygon` counts, contour
   vertex counts, hole counts, hole vertex counts, and strict bbox-overlap
   counts) to explain both the ~49 ms/call cost on `base.stl` and the benchy
   scale that made t156's single-derivation removal sub-noise on benchy.
3. **Exact cheaper predicate vs shared evaluation.** Design and dual-compute an
   exact boolean fast path inside `SliceRegionView::derive_needs_support` —
   strict bounding-box overlap prefilter plus a strict-interior
   point-in-`ExPolygon` witness check, falling back to
   `slicer_core::polygon_ops::intersection_ex` whenever no strict-interior
   witness is found — against the full `intersection_ex` across every call on
   all four probed cells and the SDK/runtime test suites:
   - verify **0 answer mismatches** across all calls;
   - measure the witness hit rate and the per-call elapsed time of the fast path
     vs the full Clipper2 sweep;
   - run an env-gated probe pass (`PNP_T157_FAST_PREDICATE=1`) where
     `derive_needs_support` returns the fast-path answer, measuring the
     resulting collapse in `PrePass::SupportAnalysis` stage wall and in both
     `Layer::Perimeters` and `Layer::Infill` `prep_views` brackets, and
     verifying byte-identical G-code against t151's active reference set;
   - compare this shape against the fog's alternative ("one shared evaluation
     per `(layer, region)` across prepass and per-layer"), which still pays one
     full `intersection_ex` per layer (`~23.9 s` thread on `base`) and requires
     a schema/lifecycle change.
4. **Price the ceiling across all three call sites** against `base classic-on`'s
   **1.096 s** per-layer residual (t154) plus its prepass `SupportAnalysis`
   stage wall, `base classic-off`'s 24.51 s wall of record, the `benchy` cells,
   and the standing 0.80 % floor.
5. **Stop rule.** Phase-1 attribution + probe: **no production edit survives
   this ticket**. If the fast-path predicate is exact (0 mismatches) and its
   combined ceiling clears the floor by a wide multiple, revert the probe,
   restore the tree so `cargo xtask build-guests --check` exits 0, and graduate
   the candidate ticket for the standing eight-cell paired A/B gate.

## Acceptance

- Exhaustive workspace reader census recorded by crate-qualified path and symbol
  name, establishing both the production consumer set and the contract-test pin
  set.
- Operand anatomy and three-call-site thread/wall breakdown measured on `base
  classic-off`, `base classic-on`, `benchy classic-off`, and `benchy classic-on`.
- Dual-computed fast-path predicate verified at **0 mismatches** across every
  call in all four probed cells and all unit/contract tests in `slicer-sdk` and
  `slicer-runtime`.
- In-situ fast-predicate probe pass (`PNP_T157_FAST_PREDICATE=1`) measured on
  `base classic-off`, `base classic-on`, `benchy classic-off`, and `benchy
  classic-on`, with every G-code hash matching t151's active set.
- Probe patch preserved in `evidence/t157-needs-support-predicate/probe.patch`,
  tree restored to HEAD, and `cargo xtask build-guests --check` exits 0 after
  restoration.
- `evidence/t157-needs-support-predicate/` holds `FINDINGS.md`, `REPRODUCE.md`,
  the reduced JSON tables, and `verify.py` (`verify.py` and `verify.py
  --self-test` exit 0 checkout-only).
- The map's destination status, selection scope, Decisions so far, and fog are
  updated; the candidate ticket is graduated if the stop rule passes.

## Evidence starts

- [`evidence/t155-infill-stage-block/`](../evidence/t155-infill-stage-block/FINDINGS.md)
  — `prepare_regions` sub-timers and `derive_needs_support` per-layer thread sums.
- [`evidence/t156-prepared-memo/`](../evidence/t156-prepared-memo/FINDINGS.md)
  — the 5-pair eight-cell DROP of the single-derivation memo candidate.
- `crates/slicer-sdk/src/views.rs` — `SliceRegionView::derive_needs_support`,
  `SliceRegionView::needs_support`, `SliceRegionView::set_needs_support`.
- `crates/slicer-wasm-host/src/marshal/prepared.rs` — `prepare_regions`.
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` —
  `commit_support_analysis_builtin`.
- `modules/core-modules/tree-support/src/lib.rs` — `TreeSupport::run_support`
  (`SupportPaintPolicy::DefaultEligible` comment on Packet 224).

## Answer

**Resolved 2026-11-16: `commit_support_analysis_builtin` is the sole production
decision consumer of `derive_needs_support` in the entire pipeline, and an exact
boolean fast path inside `SliceRegionView::derive_needs_support` collapses all
three call sites (`PrePass::SupportAnalysis`, `Layer::Perimeters`, and
`Layer::Infill`) by 91.0x–120.0x with 0/3,680 mismatches. Candidate graduated as
[158](158-needs-support-fast-predicate.md).**

1. **Exhaustive reader census (sharpening t155 §4).** Tracing past
   `__slicer_adapt_slice_regions` (`crates/slicer-macros/src/lib.rs`) to
   `SliceRegionView::needs_support` (`crates/slicer-sdk/src/views.rs`) across
   `crates/` and `modules/` proves **zero production guest modules** read
   `region.needs_support()`:
   - `TreeSupport::run_support` (`modules/core-modules/tree-support/src/lib.rs`)
     explicitly retired `region.needs_support()` from
     `SupportPaintPolicy::DefaultEligible` in Packet 224 because
     `PrePass::SupportAnalysis` already gates auto-support contact detection on
     `needs_support` when producing `SupportAnalysisIR`.
   - `TraditionalSupport::run_support`
     (`modules/core-modules/traditional-support/src/lib.rs`) never called
     `region.needs_support()`.
   - Therefore, `commit_support_analysis_builtin`
     (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) is the
     **sole production decision consumer** of `derive_needs_support` (calling it
     directly on `SliceRegionView::from_ir` only when `enable_support == true`).
   - The two per-layer evaluations per layer in `prepare_regions`
     (`crates/slicer-wasm-host/src/marshal/prepared.rs`) marshal a boolean into
     `PreparedRegionData::needs_support` → WIT `slice-region::needs-support` →
     `SliceRegionView::set_needs_support` where no guest module reads it; only
     contract/unit tests (`region_preparation_sharing_tdd.rs`,
     `region_eligibility_tdd.rs`, `view_seam_identity_tdd.rs`,
     `source_identity_repair_tdd.rs`, `blackboard_layer_arena_tdd.rs`,
     `layer_module_tdd.rs`) assert on it.

2. **Operand anatomy and three-call-site census on `benchy` and `base`.**
   - On `3dbenchy.stl` (`240` layers), each call intersects `a_polys_mean = 2.42`
     (`a_verts_mean = 1,890.1`) against the object-global overhang union
     `b_polys = 201` (`b_verts = 6,107`), costing **3.24–3.39 ms/call**
     (`778.2–814.8 ms` thread per site; `1,571.3 ms` across 2 sites on
     `benchy classic-off` and `2,403.2 ms` across 3 sites on `benchy classic-on`).
     That explains why `t156` (removing only 1 of the 3 sites) had a ceiling of
     only `83–115 ms` on `benchy-on`.
   - On `base.stl` (`496` layers), each call intersects `a_polys_mean = 1.69`
     (`a_verts_mean = 5,155.7`) against `b_polys = 994` (`b_verts = 26,633`),
     costing **49.4–60.9 ms/call** (`51,651.6 ms` across 2 sites on
     `base classic-off` and `84,792.0 ms` across 3 sites on `base classic-on`).

3. **Exact fast-path boolean predicate (`0 / 3,680` mismatches, `91x–120x` speedup).**
   - `derive_needs_support` only asks whether $\text{area}(A \cap B) > 0$. A
     strict axis-aligned bbox prefilter (`a.min_x < b.max_x && b.min_x < a.max_x
     && a.min_y < b.max_y && b.min_y < a.max_y`) plus a strict-interior
     point-in-`ExPolygon` witness check in exact `i128` arithmetic (requiring
     strictly inside outer contour and strictly outside all holes, with boundary
     hits classified as `0` and never accepted as a witness, falling back to
     `slicer_core::polygon_ops::intersection_ex` on overlapping pairs when no
     witness is found) is **exact by construction**.
   - Across all **3,680** dual-mode production calls (`480` on `benchy-off`,
     `720` on `benchy-on`, `992` on `base-off`, `1,488` on `base-on`) and all
     unit/contract tests, mismatches = **`0 / 3,680`**.
   - Every call on `benchy` (`240 / 240` per site) resolves on
     `b_vertex_in_a_sampled` in **19.78** PIP tests (`35.7–37.0 µs/call`,
     **90.95x–92.52x** speedup); on `base`, `492 / 496` resolve on
     `b_vertex_in_a_sampled` and `4 / 496` on `a_vertex_in_b_sampled` in
     **37.65** PIP tests (`410.2–561.6 µs/call`, **113.04x–119.96x** speedup).
   - Under `PNP_T157_FAST_PREDICATE=1`, `prep_views_ms` (`Layer::Perimeters` +
     `Layer::Infill` combined) collapses from `2,102.6 → 480.5 ms` (`-77.1%`) on
     `benchy-off`, `2,140.6 → 513.3 ms` (`-76.0%`) on `benchy-on`,
     `55,331.0 → 3,860.9 ms` (`-93.0%`) on `base-off`, and
     `64,875.1 → 4,011.5 ms` (`-93.8%`) on `base-on`, while
     `PrePass::SupportAnalysis` eligibility wall on `base-on` drops from
     `2,100.5 ms` to `41.5 ms` (**50.6x** wall speedup). All **8/8** probe runs
     match t151's reference G-code SHA-256.

4. **Pricing against the `t154` ledger (`ESTIMATE`, not a ratio of record):**

| cell | thread saved (all sites) | per-layer wall-eq saved | `SupportAnalysis` wall saved | **total priced wall saved** | **% of PnP wall** | **floor multiple (÷0.80%)** | projected ratio (`ESTIMATE`) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy classic-off` | 1,554.0 ms | 230.6 ms | 0.0 ms | **230.6 ms (`0.2306 s`)** | **6.00 %** | **7.50x** | 2.0068x → **1.8864x** |
| `benchy classic-on` | 2,377.2 ms | 166.1 ms | 71.2 ms | **237.3 ms (`0.2373 s`)** | **3.78 %** | **4.73x** | 2.5711x → **2.4739x** |
| `base classic-off` | 51,194.7 ms | 4,692.5 ms | 0.0 ms | **4,692.5 ms (`4.6925 s`)** | **19.15 %** | **23.94x** | 1.8270x → **1.4772x** |
| `base classic-on` | 84,085.2 ms | 5,345.8 ms | 2,068.6 ms | **7,414.4 ms (`7.4144 s`)** | **10.66 %** | **13.33x** | 3.2197x → **2.8765x** |

   On `base classic-on`, the **5.346 s** per-layer wall-equivalent saving alone is
   **4.88x** `t154`'s **1.096 s** non-prepass residual requirement, taking the
   canonical-prepass projection from `1.0508x` (`22.694 s`) to **`0.8033x`
   (`17.349 s vs 21.598 s`)**.

5. **Verdict (stop rule): graduate Phase-2 implementation ticket
   [158](158-needs-support-fast-predicate.md).** Both stop-rule gates pass with
   wide margin (`7.414 s >= 1.5 s` on `base`, `237.3 ms >= 150 ms` on `benchy`).
   The probe is reverted from `crates/`, `probe.patch` is preserved,
   `cargo xtask build-guests --check` exits `0`, and `verify.py --self-test`
   exits `0`.

Evidence: [`evidence/t157-needs-support-predicate/`](../evidence/t157-needs-support-predicate/FINDINGS.md).

