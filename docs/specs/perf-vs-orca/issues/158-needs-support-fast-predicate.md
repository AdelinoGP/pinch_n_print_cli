# Exact fast-path boolean predicate in `SliceRegionView::derive_needs_support`

Type: task / measured candidate + standing gate
Label: wayfinder:task
Status: resolved
Resolved: 2026-11-16; human-amended 2026-11-17 — **KEEP**; exact fast-path boolean intersection predicate (`expolygons_intersect_area_positive`) inside `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) is byte-identical in 8/8 cells against t151's active reference set, lowers process CPU in 8/8 cells by 5.88 %–21.38 % (5.3x–19.1x the 1.12 % any-pair CPU null band), and passes all four `base` cells at 6.78x–18.58x the 0.80 % floor (+5.42 % to +14.87 % wall, −4.23 to −6.37 s, 20/20 pairs candidate-faster). It failed the original literal eight-cell separate-arm median wall gate on two `benchy` cells (`benchy-classic-on` +0.766 %, 0.96x floor; `benchy-arachne-off` −0.076 %, −0.10x floor, despite +2.69 % and +3.16 % pairwise medians and 4/5 and 3/5 pair wins); the human reviewed that result and explicitly ruled **“I accept it as a keep.”** Production code and unit tests are landed; the original mechanical DROP, measurements, and missing `benchy` null control remain disclosed in `evidence/t158-needs-support-fast-predicate/`.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [The surviving `needs_support` predicate: reader census, three-site cost, and exact fast-path probe](157-needs-support-predicate-attribution.md) — t157 proved that `commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) is the sole production decision consumer of `derive_needs_support` (after Packet 224 retired `region.needs_support()` from `TreeSupport::run_support`), and that replacing the unconditional Clipper2 `intersection_ex` inside `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) with an exact boolean fast path (strict bbox overlap prefilter + strict-interior point-in-`ExPolygon` witness in `i128`, with `intersection_ex` fallback on overlapping pairs when no witness is found) produces **0 / 3,680** answer mismatches across `benchy` and `base`, accelerates all three call sites (`PrePass::SupportAnalysis`, `Layer::Perimeters`, `Layer::Infill`) by **91.0x–120.0x**, and prices at **4.693 s** wall-equivalent on `base classic-off` (**19.15 %** of PnP wall, **23.94x** floor), **7.414 s** on `base classic-on` (**10.66 %** of PnP wall, **13.33x** floor), and **230.6–237.3 ms** on `benchy` (**3.78–6.00 %** of PnP wall, **4.73x–7.50x** floor).

## Question

Does landing the exact fast-path boolean intersection predicate inside
`SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) pass
the standing eight-cell paired wall/CPU gate with byte-identical G-code across
all eight cells, and how much of the **4.69 s / 7.41 s** (`base`) and
**231–237 ms** (`benchy`) priced ceiling is realised on the quiet paired gate?

## Scope

1. **Shape.** Replace the unconditional `!slicer_core::polygon_ops::intersection_ex(&self.polygons, &overhang.xy_footprint).is_empty()` inside `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`) with an exact boolean intersection helper (`expolygons_intersect_area_positive(a, b)`):
   - strict axis-aligned bounding-box prefilter (`a.min_x < b.max_x && b.min_x < a.max_x && a.min_y < b.max_y && b.min_y < a.max_y`) rejecting disjoint and boundary-touching-only `ExPolygon` pairs in $O(|A| + |B|)$;
   - strict-interior point-in-`ExPolygon` witness check in exact `i128` cross-product arithmetic (requiring the candidate contour vertex of $B_j$ to lie strictly inside $A_i$'s outer contour and strictly outside all holes of $A_i$, or vice versa, with boundary/vertex hits classified as `0` and never accepted as a witness);
   - exact Clipper2 fallback (`!slicer_core::polygon_ops::intersection_ex(&a_sub, &b_sub).is_empty()`) restricted to the `ExPolygon`s in `overlapping_pairs` whenever no strict-interior vertex witness is found (e.g., two polygons crossing edge-over-edge with all vertices outside each other).
2. **Unit & boundary-case tests.** Add targeted unit tests in `crates/slicer-sdk` covering:
   - disjoint bboxes (`false`);
   - edge-touching / vertex-touching polygons with zero 2D overlap area (`false` — verifying that boundary contact is not misclassified as `true`);
   - an overhang polygon sitting strictly inside a hole of `region.polygons` (`false`);
   - a region polygon sitting strictly inside a hole of `overhang.xy_footprint` (`false`);
   - two long thin rectangles crossing in a `+` shape with all 8 vertices strictly outside the other polygon (`true` via the `intersection_ex` fallback);
   - standard interior-vertex overlap (`true` via the fast witness).
3. **Output identity.** The candidate must be byte-identical in **all eight** cells against the active reference set ([`evidence/t151-skirt-hull-contour/reference.json`](../evidence/t151-skirt-hull-contour/reference.json)), with every run exiting `0`, `degraded=false`, `fatal_error_count=0`, `non_fatal_error_count=0`.
4. **Standing eight-cell gate.** Paired wall and CPU medians, interleaved baseline/candidate, against the same-run `0.80 %` floor, in **all eight** cells (`benchy` and `base` × `classic` and `arachne` × supports `off` and `on`). Record the measured null band, pair count, box load sample, and floor multiples per cell.
5. **Price the realised saving** against `t157`'s priced ceiling (`230.6 ms` / `237.3 ms` on `benchy classic-off` / `on`; `4.693 s` / `7.414 s` on `base classic-off` / `on`) and re-derive the ratios of record if banked.
6. **Stop rule.** If the standing gate passes, bank the tranche and update the scoreboard of record and `t154`'s canonical-prepass ledger. If the gate fails, record DROP with the measured per-cell table and revert. **Human amendment (2026-11-17):** after the recorded gate failure and mechanical revert, the human explicitly accepted the candidate as KEEP. The original gate result remains recorded; the candidate is restored and the route ledger is re-derived as a disclosed cross-window composition, not a fresh Orca scoreboard.

## Acceptance

- Unit tests in `crates/slicer-sdk` pin all six geometric cases (disjoint, edge/vertex-touching zero-area, inside-hole both ways, `+`-crossing fallback, interior witness) against `slicer_core::polygon_ops::intersection_ex`.
- 8/8 G-code hashes equal t151's active reference set; every run exits `0`, `degraded=false`, zero diagnostics.
- Paired eight-cell wall/CPU table with measured null band, retained pairs, and floor multiples per cell.
- `cargo xtask build-guests --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask check-literals` exit `0`.
- `evidence/t158-needs-support-fast-predicate/` holds `FINDINGS.md`, `REPRODUCE.md`, reduced JSON tables, and `verify.py` (`--self-test` exits `0` checkout-only).
- `map.md` updated with the banked tranche (or DROP) and refreshed ratios of record.

## Evidence starts

- [`evidence/t157-needs-support-predicate/`](../evidence/t157-needs-support-predicate/FINDINGS.md) — reader census, `0 / 3,680` dual-mode equivalence verification, `probe.patch`, and `t154`-denominated pricing.
- [`evidence/t156-prepared-memo/`](../evidence/t156-prepared-memo/FINDINGS.md) — why single-derivation memo preservation dropped on the 8-cell gate.
- `crates/slicer-sdk/src/views.rs` — `SliceRegionView::derive_needs_support`.

## Resolution

- **Final verdict:** **KEEP by explicit human ruling on 2026-11-17.** The human said **“I accept it as a keep”** after reviewing the original mechanical DROP. The candidate diff (including the 4 new geometric boundary/hole/fallback tests in `crates/slicer-sdk/tests/layer_module_tdd.rs`) is restored from [`evidence/t158-needs-support-fast-predicate/candidate.patch`](../evidence/t158-needs-support-fast-predicate/candidate.patch). The ruling and its disclosures are recorded in [`RULING.md`](../evidence/t158-needs-support-fast-predicate/RULING.md).
- **Original mechanical result, preserved:** **DROP** under the standing eight-cell separate-arm median wall gate (`wall_saving_pct >= 0.80 %` in all 8 cells). `reduced/campaign.json` and `verify.py` intentionally continue to describe that measured gate result; the human amendment changes the disposition, not the observations.
- **Candidate shape (`crates/slicer-sdk/src/views.rs` — `SliceRegionView::derive_needs_support`):**
  Replaced the unconditional `!slicer_core::polygon_ops::intersection_ex(&self.polygons, &overhang.xy_footprint).is_empty()` with `expolygons_intersect_area_positive(&self.polygons, &overhang.xy_footprint)`:
  1. Strict axis-aligned bounding-box prefilter (`expoly_bbox` + `bboxes_strictly_overlap`) rejecting disjoint and boundary-touching-only `ExPolygon` pairs in $O(|A| + |B|)$.
  2. Strict-interior point-in-`ExPolygon` witness (`point_strictly_in_expolygon` / `point_in_ring_strict_or_boundary` in exact `i128` cross-product arithmetic), accepting a contour vertex of $B_j$ only when it lies strictly inside $A_i$'s outer contour and strictly outside all holes of $A_i$ (or vice versa), with boundary/vertex hits classified as `0` and never accepted as a witness.
  3. Exact `slicer_core::polygon_ops::intersection_ex` fallback restricted to the `ExPolygon`s participating in `overlapping_pairs` whenever no strict-interior vertex witness is found.
  All 72 tests in `crates/slicer-sdk/tests/layer_module_tdd.rs` passed on the candidate tree (`cargo test -p slicer-sdk --test layer_module_tdd`).
- **Output identity (16 runs = 8 cells × 2 arms):** **8/8 byte-identical** to `t151`'s active reference set (`3d28d1b3…`, `eb202973…`, `39ad6289…`, `77c3c578…`, `9bb18f0e…`, `eccbd523…`, `2f249cdb…`, `1811425f…`). All 16 output runs, all 80 A/B runs, and all 18 null-control runs exited `0` with `degraded=false`, `fatal_error_count=0`, `non_fatal_error_count=0`, `diagnostic_lines=0`, and matching G-code SHA-256 hashes.
- **Paired eight-cell A/B table (5 interleaved pairs/cell = 80 runs; box load `11.15 %` pre → `7.54 %` post):**

  | Cell | Base wall med (s) | Cand wall med (s) | Wall saving (%) | Floor multiple (`0.80 %`) | Pairwise wall med (%) | Faster wall pairs | Base CPU med (s) | Cand CPU med (s) | CPU delta (%) | Lower CPU pairs | Cell gate |
  |---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
  | `benchy-classic-off` | `3.659` | `3.486` | **`+4.715 %`** | `5.89x` | `-0.066 %` | `2/5` | `19.09` | `17.14` | **`-10.23 %`** | `5/5` | PASS |
  | `benchy-classic-on` | `7.628` | `7.570` | **`+0.766 %`** | **`0.96x`** | **`+2.690 %`** | `4/5` | `40.63` | `38.17` | **`-6.04 %`** | `4/5` | **DROP (`< 0.80 %`)** |
  | `benchy-arachne-off` | `3.592` | `3.595` | **`-0.076 %`** | **`-0.10x`** | **`+3.158 %`** | `3/5` | `19.63` | `18.19` | **`-7.32 %`** | `5/5` | **DROP (`< 0.80 %`)** |
  | `benchy-arachne-on` | `7.926` | `7.799` | **`+1.599 %`** | `2.00x` | `+1.599 %` | `3/5` | `42.02` | `39.55` | **`-5.88 %`** | `5/5` | PASS |
  | `base-classic-off` | `28.583` | `24.334` | **`+14.865 %`** | `18.58x` | `+15.246 %` | `5/5` | `208.72` | `164.09` | **`-21.38 %`** | `5/5` | PASS |
  | `base-classic-on` | `86.278` | `81.602` | **`+5.420 %`** | `6.78x` | `+8.667 %` | `5/5` | `601.67` | `530.59` | **`-11.81 %`** | `5/5` | PASS |
  | `base-arachne-off` | `28.920` | `24.687` | **`+14.637 %`** | `18.30x` | `+13.655 %` | `5/5` | `211.92` | `168.28` | **`-20.59 %`** | `5/5` | PASS |
  | `base-arachne-on` | `84.720` | `78.351` | **`+7.518 %`** | `9.40x` | `+6.911 %` | `5/5` | `603.45` | `535.61` | **`-11.24 %`** | `5/5` | PASS |

- **Same-arm null control (3 base cells × 3 pairs = 18 runs; baseline vs baseline):**
  - Any-pair absolute max band: **`11.762 %` wall / `1.117 %` CPU**.
  - Cell median max band: **`4.410 %` wall / `0.934 %` CPU** (`base-classic-off` median `-4.410 %` wall / `-0.831 %` CPU; `base-classic-on` `-3.136 %` / `-0.934 %`; `base-arachne-off` `+0.504 %` / `-0.300 %`).
- **Structural diagnosis (why the `base` win is decisive while two `benchy` cells miss the separate-arm wall floor):**
  1. **Process CPU is lower in 8/8 cells by `5.88 %–21.38 %`** (`5.3x–19.1x` the worst-case single-pair CPU null band of `1.117 %`), and the candidate wins `39/40` pairs on CPU (`19/20` on `benchy`, `20/20` on `base`).
  2. **All four `base` cells pass decisively (`6.78x–18.58x` floor, `20/20` pairs candidate-faster):** `base-classic-off` saves **`4.249 s` (`+14.87 %`)** wall and **`44.63 CPU s` (`-21.38 %`)** against `t157`'s `4.693 s` / `51.23 CPU s` ceiling (`90.5 %` wall / `87.1 %` CPU realisation); `base-arachne-off` saves **`4.233 s` (`+14.64 %`)** wall and **`43.64 CPU s` (`-20.59 %`)**; `base-classic-on` saves **`4.676 s` (`+5.42 %` separate-arm, `+8.67 %` pairwise)** wall and **`71.08 CPU s` (`-11.81 %`)** against `t157`'s `7.414 s` / `84.10 CPU s` ceiling (`63.1 %` separate-arm / `100.9 %` pairwise realisation); `base-arachne-on` saves **`6.369 s` (`+7.52 %`)** wall and **`67.84 CPU s` (`-11.24 %`)**.
  3. **On `benchy`, the entire three-site `derive_needs_support` cost is only `~231–237 ms` wall-equivalent (`~3.0 %` of `7.6–7.9 s` supports-on wall):** even though the candidate wins `4/5` pairs on `benchy-classic-on` (pairwise median **`+2.690 %`**, `-2.45 CPU s`) and `3/5` pairs on `benchy-arachne-off` (pairwise median **`+3.158 %`**, `-1.44 CPU s`), single-run wall jitter on a `3.5–7.9 s` process shifts the separate-arm 5-run medians to `+0.766 %` (about `3 ms` short of the `0.80 %` bar) and `-0.076 %`. Thus, no `derive_needs_support`-only candidate can reliably clear an unconditional `benchy` separate-arm wall floor on its own unless paired with a `benchy`-active lever or evaluated under a `base` / CPU-corroborated gate.
- **Indicative route composition after KEEP:** applying the measured wall deltas to `t154`'s cross-window ledger moves the eight ratios to `1.9122x / 2.5514x / 1.7641x / 2.6482x` on `benchy` and `1.5554x / 3.0452x / 1.5349x / 2.8302x` on `base` (classic-off / classic-on / arachne-off / arachne-on). Subtracting the measured savings from `t154`'s canonical-prepass projections makes **8/8** projected cells win; `base-classic-on` moves **1.0508x → 0.8343x**. These are compositions, not a fresh Orca scoreboard, and the negative `benchy-arachne-off` delta is retained. Re-derive with [`route_arithmetic.py`](../evidence/t158-needs-support-fast-predicate/route_arithmetic.py); results are in [`route_arithmetic.json`](../evidence/t158-needs-support-fast-predicate/route_arithmetic.json).
- **Landing verification (2026-11-17):** the restored diff passes all 72 `layer_module_tdd` tests, workspace Clippy with all targets, guest freshness, struct-literal and test-quality checks. Fresh release baseline/candidate slices of `benchy-arachne-off` and `base-classic-off` all exit `0`, report `degraded=false` with zero fatal/non-fatal errors, and reproduce their active reference hashes (`39ad6289…` / `9bb18f0e…`).
- **Evidence:** [`evidence/t158-needs-support-fast-predicate/FINDINGS.md`](../evidence/t158-needs-support-fast-predicate/FINDINGS.md), [`REPRODUCE.md`](../evidence/t158-needs-support-fast-predicate/REPRODUCE.md), [`reduced/campaign.json`](../evidence/t158-needs-support-fast-predicate/reduced/campaign.json), [`candidate.patch`](../evidence/t158-needs-support-fast-predicate/candidate.patch), [`run_t158.ps1`](../evidence/t158-needs-support-fast-predicate/run_t158.ps1), [`reduce_t158.py`](../evidence/t158-needs-support-fast-predicate/reduce_t158.py), and [`verify.py`](../evidence/t158-needs-support-fast-predicate/verify.py) (`--self-test` exits `0`, 6/6 negative controls rejected).

