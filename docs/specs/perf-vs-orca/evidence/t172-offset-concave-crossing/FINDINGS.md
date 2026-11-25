# t172 findings: canonical `OffsetConcaveCrossing` in `vendor/clipper2-rust`

**Verdict: measured candidate, strict gate PASSED in 8/8 cells, landed under the sixth recorded t81-style exception.** Porting canonical OrcaSlicer `v2.5.0` (`40ecc66f78`) `clipper.offset.cpp::OffsetConcaveCrossing` (≈30 lines, `.round()` conversion, AGPLv3 attribution header) into `vendor/clipper2-rust/src/offset.rs` saves **+14.89 % to +23.01 %** wall across all eight cells — **5/5 pairs in every cell**, **18.6x–28.8x** the 0.80 % floor, strictly outside every cell's same-window any-pair null band — with process CPU corroborating at **+19.20 % to +30.67 %** (**5/5** in every cell) and the accelerated arm reproducing the saving at **+14.19 % to +22.63 %** wall / **+19.33 % to +30.93 %** CPU (**3/3** in every cell).

Both base supports-off cells beat t169's estimated required cuts by wide margins — **`base classic-off` +21.41 % vs 3.9 % (5.5x)** and **`base arachne-off` +19.88 % vs 10.3 % (1.9x)** — putting them at an estimated **~0.82x** and **~0.89x** of OrcaSlicer (pending a fresh two-tool board in [t174](../../issues/174-post-t172-scoreboard-refresh.md) to record 2/8 wins officially).

Resolved 2026-11-20. Ticket: [172](../../issues/172-port-offset-concave-crossing.md).

The human granted the **sixth recorded t81-style exception** (2026-11-20, Q1 ruling) before any production edit, scoped strictly to `OffsetConcaveCrossing` in `vendor/clipper2-rust/src/offset.rs` and its downstream host/guest `polygon_ops` offset consumers. The active reference set is now t172's ([`reference.json`](reference.json)); t169's is superseded.

## 1. What was ported

Canonical OrcaSlicer `v2.5.0` (`40ecc66f78`) carries a fork patch in `clipper.offset.cpp::OffsetConcaveCrossing` (not present in upstream Clipper2 2.0.1 or `clipper2-rust` 1.1.0/1.2.0): for a closed polygon vertex `j` (with prior `k`, next `next`) where `delta_callback` is unset and `path[j] != path[k]`, if the corner is concave (`cos_a > -0.999` and `sin_a * group_delta < 0.0`) and both adjacent edges are long enough (`4 * x² <= dist²(path[k], path[j])` and `4 * x² <= dist²(path[j], path[next])`, where `x = |group_delta * sin_a| / (1 + cos_a)`), it emits the single rounded crossing point `round(path[j] + (norms[k] + norms[j]) * (group_delta / (1 + cos_a)))` instead of Clipper2's 3-point concave loop that the finishing union later has to clip away.

- **File changed:** `vendor/clipper2-rust/src/offset.rs` (`ClipperOffset::offset_concave_crossing`, called from `ClipperOffset::offset_polygon`), with the standard AGPLv3 OrcaSlicer porting header prepended per `docs/ORCASLICER_ATTRIBUTION.md`. No `polygon_ops` wrapper code changed.
- **Explicitly excluded (per Q1 ruling):** `DoRound` (`std::round` vs `ceil`), `GetLineIntersectPt`, `c2_offset_as_group`, and any other `clipper2-rust` or `polygon_ops` changes.
- **Residual disclosed (from t170):** **1.04–1.30x** per-call Rust-vs-C++ overhead remains on hot offset sites (rounding/arena differences), and `DoRound` is not included.

## 2. Falsifiers, proven red before the fix

Two unit tests in `vendor/clipper2-rust/src/offset_tests.rs` pin the mechanism and its guards:

- `test_offset_concave_crossing_eliminates_three_point_loops_and_rounds` — 32-gon (radius 100,000, inward offset `-2,000`). **Watched failing before the patch**: `do_group_offset` emitted **96** raw points (3-point concave loop at every vertex) instead of **32**. Green after the patch, matching canonical's `.round()` crossing formula at every vertex and returning a 32-vertex eroded polygon through `execute()`.
- `test_offset_concave_crossing_respects_short_edge_and_callback_guards` — confirms both the `4x² > dist²` short-edge fallback and the `delta_callback` bypass revert to 3-point joins (12 raw points on a 4-vertex square).

**Self-captured fixture update (`slicer-core`):** `polygon_ops::tests::prepared_polyline_clip_universe_matches_pre_hoist_reference` compares `PreparedPolylineClip::new(&clip).clip_paths` verbatim against a pre-patch JSON recording (`crates/slicer-core/tests/fixtures/clip_polylines_prepared_reference.json`). All clipped polyline outputs in `tests/polygon_ops_tdd.rs` passed unchanged, while two inflated input rings moved as expected: `"hole_and_island"`'s CW hole square (`[(40001, 40001)...]` → `[(40001, 59999)...]`, identical 4 vertices cyclically rotated by 1) and `"dense_ring"` (96 of 256 concave sawtooth vertices shifted by **≤ 1 unit = 100 nm** from direct `.round()` vs 3-point union clipping). The fixture was updated to match canonical-correct output per `AGENTS.md`.

## 3. Campaign and output currency

Four arms staged from HEAD (`9c3125d5` baseline = verified t169 candidate binaries; candidate built via `cargo xtask dist --edition developer` and `--accelerated`):

| arm | exe sha256 | module tree sha256 |
| --- | --- | --- |
| baseline ordinary | `b2e8261e…` | `9c1da3bf…` |
| candidate ordinary | `9f3fcba8…` | `bd132f43…` |
| baseline accelerated | `f56c9271…` | `9504e039…` |
| candidate accelerated | `490c6dc2…` | `91821d61…` |

**Output (16 ordinary + 16 accelerated runs, all `exit 0`).** The baseline reproduces **t169's** active reference set in **8/8** on both arms (control). The candidate moves **8/8**, and the accelerated candidate is **byte-identical to the ordinary candidate in 8/8** (**16/16** across baseline and candidate).

| cell | t169 (baseline) | t172 (candidate) | bytes delta |
| --- | --- | --- | ---: |
| benchy classic off | `f505910c…` | `4c67ef0b…` | −1,708 |
| benchy classic on | `9b40bf51…` | `fdca4e2d…` | −47,093 |
| benchy arachne off | `224fb6f0…` | `fd631384…` | +5,157 |
| benchy arachne on | `f32047c3…` | `2869186c…` | −41,224 |
| base classic off | `33468ea9…` | `02ffa2d6…` | −32,488 |
| base classic on | `661b9a51…` | `0e13ff7b…` | −75,187 |
| base arachne off | `e861766a…` | `e806296f…` | −22,521 |
| base arachne on | `2124b5f5…` | `adf87012…` | −65,049 |

## 4. Paired wall and CPU (ordinary arm, 5 interleaved pairs per cell)

Positive = candidate faster. Null band from 3 pairs/cell of baseline-vs-baseline (`-NullControl`), shown as any-pair / median.

| cell | wall s (base → cand) | wall median (spread) | wins | floor x | null wall | CPU median | wins | null CPU | verdict |
| --- | --- | --- | ---: | ---: | --- | --- | ---: | --- | --- |
| benchy classic off | 2.297 → 1.768 | **+23.01 %** (+21.13…+25.10) | 5/5 | 28.76x | 2.63 / 0.70 | **+25.70 %** | 5/5 | 4.22 / 2.05 | **PASS** |
| benchy classic on | 5.104 → 4.301 | **+18.41 %** (+14.32…+26.58) | 5/5 | 23.02x | 5.39 / 2.20 | **+24.83 %** | 5/5 | 0.69 / 0.53 | **PASS** |
| benchy arachne off | 2.388 → 1.877 | **+21.49 %** (+12.66…+26.64) | 5/5 | 26.86x | 13.69 / 3.82 | **+22.40 %** | 5/5 | 1.97 / 1.90 | **PASS** |
| benchy arachne on | 5.277 → 4.319 | **+17.37 %** (+13.29…+19.39) | 5/5 | 21.72x | 3.46 / 2.95 | **+23.56 %** | 5/5 | 5.92 / 1.32 | **PASS** |
| **base classic off** | 14.187 → 11.233 | **+21.41 %** (+15.75…+26.25) | 5/5 | 26.77x | 2.29 / 1.71 | **+30.67 %** | 5/5 | 1.28 / 1.10 | **PASS (beats 3.9 %)** |
| base classic on | 53.182 → 45.116 | **+14.89 %** (+11.49…+18.51) | 5/5 | 18.62x | 5.19 / 3.44 | **+19.20 %** | 5/5 | 1.31 / 0.99 | **PASS** |
| **base arachne off** | 14.814 → 12.255 | **+19.88 %** (+16.34…+22.57) | 5/5 | 24.85x | 8.95 / 3.20 | **+28.77 %** | 5/5 | 1.05 / 0.67 | **PASS (beats 10.3 %)** |
| base arachne on | 54.900 → 44.315 | **+15.00 %** (+13.12…+21.06) | 5/5 | 18.75x | 5.51 / 2.63 | **+19.98 %** | 5/5 | 0.99 / 0.66 | **PASS** |

**Gate: PASSED 8/8 on every clause of Q2.** Every cell clears 0.80 % by **18.6x–28.8x**, sits outside its own any-pair wall null band (even `benchy arachne-off`'s noisy 13.69 % single-pair spike is cleared by +21.49 %), has **5/5** pair wins on both wall and CPU, and both base supports-off cells beat their t169 estimated required cuts (`+21.41 % >= 3.9 %`, `+19.88 % >= 10.3 %`). Notably, every cell exceeded t170's conservative host-only estimate (t170 predicted +5.2–16.4 % host-only; actuals are +14.9–23.0 % because both host prepass offsets and in-process guest perimeter offsets speed up).

**Accelerated corroboration (3 interleaved pairs per cell).** Wall medians **+22.54 / +18.74 / +21.36 / +18.89 / +22.63 / +14.19 / +22.54 / +15.16 %**, **3/3** wins in every cell; CPU **+24.87 / +26.46 / +19.73 / +23.77 / +30.93 / +19.98 / +29.17 / +19.33 %**, **3/3** in every cell.

## 5. Delta audit

`audit_t172.py` compares the ordinary baseline and candidate G-code (`reduced/audit.json`). **No feature is lost**; total extruding path changes by **−0.745 % to +0.007 %** across all eight cells.

| cell | total path delta | largest section deltas |
| --- | --- | --- |
| benchy classic off | −48.34 mm (−0.048 %) | Sparse infill −49.17 (−0.203 %), Bottom surface +9.56 (+0.331 %), Gap infill +7.55 (+1.232 %) |
| benchy classic on | −1,375.22 mm (−0.745 %) | Support −1,303.47 (−1.571 %), Sparse infill −49.17, Brim −24.17 (−1.211 %) |
| benchy arachne off | +7.41 mm (+0.007 %) | Inner wall +12.83 (+0.042 %), Sparse infill −12.67 (−0.050 %), Top surface +12.41 (+0.368 %) |
| benchy arachne on | −1,319.41 mm (−0.713 %) | Support −1,303.47 (−1.571 %), Brim −24.10 (−1.208 %), Inner wall +12.83 |
| base classic off | −575.33 mm (−0.149 %) | Sparse infill −574.98 (−0.329 %), Gap infill −4.07 (−1.122 %), Bridge +3.76 (+2.670 %) |
| base classic on | −3,270.49 mm (−0.417 %) | Support −2,722.63 (−0.692 %), Sparse infill −574.98, Support interface +25.63 (+0.990 %) |
| base arachne off | −305.90 mm (−0.078 %) | Sparse infill −311.46 (−0.174 %), Bridge +3.24 (+1.734 %), Brim +1.97 (+0.278 %) |
| base arachne on | −3,003.68 mm (−0.382 %) | Support −2,722.63 (−0.692 %), Sparse infill −311.46, Support interface +25.63 |

- **Outer/Inner walls** move by **≤ 0.042 %** (`Outer wall` ≤ 0.0012 %) with symmetric per-layer up/down counts (e.g. `base classic-off` Outer wall 271 up / 225 down over 496 layers), the signature of sub-micron concave-crossing rounding rather than shape loss.
- **`Internal Bridge`** moves −0.49 % on benchy (−5.35 mm) and +17.17 % on base (+0.62 mm over a 3.60 mm base).
- **`Support`** falls −1.57 % on benchy (−1,303.47 mm) and −0.69 % on base (−2,722.63 mm), reversing part of t169's +0.42 / +1.13 % drift; reported, not rationalised.

## 6. Route arithmetic (estimate, not a scoreboard) & kernel-route closure

Composing t172's paired medians with t169's post-decimation estimate (`t163` ordinary board × `(1 − t169)` × `(1 − t172)` — a two-step cross-window **estimate**, not a ratio of record):

| cell | t163 board | post-t169 est | × (1 − t172) | **post-t172 est** | remaining cut |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic off | 1.782x | 1.640x | 0.7700 | **1.26x** | 20.8 % |
| benchy classic on | 2.802x | 2.376x | 0.8159 | **1.94x** | 48.4 % |
| benchy arachne off | 1.940x | 1.829x | 0.7851 | **1.44x** | 30.3 % |
| benchy arachne on | 2.829x | 2.412x | 0.8263 | **1.99x** | **49.8 %** |
| **base classic off** | 1.389x | 1.041x | 0.7859 | **0.82x** | **−18.2 % (est win)** |
| base classic on | 2.710x | 2.116x | 0.8511 | **1.80x** | 44.5 % |
| **base arachne off** | 1.457x | 1.116x | 0.8012 | **0.89x** | **−10.6 % (est win)** |
| base arachne on | 2.618x | 2.031x | 0.8500 | **1.73x** | 42.1 % |

**Kernel-route verdict: EXHAUSTED for the six non-winning cells.** Both base supports-off cells are now estimated well below 1.0x (0.82x and 0.89x), leaving **six cells** still above 1.0x (benchy supports-off ~1.26x / ~1.44x; the four supports-on cells ~1.73x–1.99x, binding cell `benchy arachne-on` at ~1.99x / 49.8 % cut). With t169's decimation and t172's `OffsetConcaveCrossing` both landed, `vendor/clipper2-rust` sits at 1.04–1.30x of canonical C++ on hot offsets and 0.61–0.94x (faster) on booleans (t170). No further Clipper2 kernel work can close the remaining 21–50 % cuts on those six cells; the live routes forward are **t173** (support-path boolean volume) and **t174** (two-tool OrcaSlicer 2.5.1 scoreboard refresh to officially verify the two estimated base supports-off wins and re-baseline the six remaining cuts).

## 7. Disclosures

- **Campaign pause / resume disclosure:** the first campaign invocation hit a 30-minute command timeout at row 75 of 80 in the ordinary A/B batch (after completing both 16-run output batches). The remaining 5 rows finished via `-Append -OnlyMissing` (CPU load 1 % → 3 %), so `pair 5 base-classic-on` spans the pause (its +14.89 % saving sits on the exact median of all five pairs); all other 39 ordinary pairs, 24 null pairs, and 24 accelerated pairs ran strictly back-to-back.
- **Box load:** null batch ran at 20 % → 16 % sampled CPU load (producing a single 13.69 % wall outlier on `benchy arachne-off`), accel batch at 7 % → 3 %. Every cell's candidate median clears its own any-pair wall null bound regardless.
- **No `--profile` attribution pass** was run in this take; t170 already attributed the exact per-site kernel speedup across three corpora, and the accelerated arm + CPU channel corroborate the wall savings across 8/8.
- **Cross-window route table (§6) is an estimate**, not a ratio of record (t161 forbids quoting cross-window compositions as official ratios); [t174](../../issues/174-post-t172-scoreboard-refresh.md) is charted to measure the fresh two-tool board directly.

## 8. Verification

- `cargo test -p clipper2-rust --no-fail-fast` — **405 passed, 0 failed** + 3 doc-tests (including both new `OffsetConcaveCrossing` tests; negative control verified red before the patch).
- `cargo test -p slicer-core --features host-algos --no-fail-fast` — **92/92 test binaries green** (`199/199` lib tests after updating the pre-patch `clip_polylines_prepared_reference.json` fixture).
- `cargo test -p classic-perimeters -p arachne-perimeters --no-fail-fast` — green.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask build-guests --check` and `--accelerated --check` — both exit 0.
- `python verify.py --self-test` — PASS (8/8 negative controls rejected); `python verify.py` — PASS.
- `python audit_t172.py --self-test` — PASS (4/4 negative controls rejected).

## 9. Reproduce

```bash
pwsh -NoProfile -File run_t172.ps1 -Mode output -BatchId ref
pwsh -NoProfile -File run_t172.ps1 -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File run_t172.ps1 -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File run_t172.ps1 -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File run_t172.ps1 -Mode ab -Accelerated -BatchId accel -Pairs 3
python audit_t172.py
python reduce_t172.py
python verify.py --self-test && python verify.py
```
