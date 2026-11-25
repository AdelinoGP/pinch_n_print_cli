# Ticket 184 — Exact `PrePass::ShellClassification` acceleration

- **Date:** 2026-11-23 (UTC)
- **Base commit:** `934cef78` (production code unchanged since `785d67a1`, post-t183)
- **Active reference currency (unchanged, zero reference change):** t182's set,
  [`../t182-spatial-hierarchical-phase3-union/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json)
- **Guest freshness after the candidate build:** `cargo xtask build-guests --check`
  exit `0`; `cargo xtask build-guests --accelerated --check` exit `0`
- **Arms (`reduced/output.json` manifest):**
  - `baseline-ordinary` exe `b8e7a710…`, modules tree `f1dc4080…` (= t183's candidate tree)
  - `baseline-accelerated` exe `3af8e37d…`, modules tree `d758a6c9…` (= t183's candidate tree)
  - `candidate-ordinary` exe `79a61cec…`, modules tree `6823f914…`
  - `candidate-accelerated` exe `e23a254f…`, modules tree `718d7892…`
  - `candidate-host-baseline-modules` (attribution only): candidate exe + baseline modules
- **Verifier:** `verify.py` → `VERIFY PASS`; `verify.py --self-test` → 16/16 mutations rejected

## 1. Verdict

**LANDED.** The literal standing all-8-cell `0.80 %` gate passes 8/8 on the
pre-registered verdict of record (two pooled ordinary windows,
[`PREREGISTRATION.md`](PREREGISTRATION.md)), with zero output drift and zero
gate waivers.

1. **Output parity (fail-closed).** All 448 measured runs exited `0` and
   reproduced t182's active hashes byte for byte: 16 ordinary output, 16
   accelerated output, 16 `--profile` fuel, 160 ordinary A/B, 96 null, 48
   accelerated A/B, 48 instrumented stage, 48 host-only. Every run had
   `degraded = false`, `0` fatal or non-fatal errors and `0` code-1200
   diagnostics. `reference.json` is unchanged.
2. **Gate (pooled, 10 interleaved pairs/cell, 6 null pairs/cell, unsandboxed,
   `RAYON_NUM_THREADS=12`):** wall **`+4.99 %` … `+16.35 %`**, 78/80 pair wins
   (`6.24×`–`20.44×` the floor). Process CPU is lower in 8/8 cells (**`+4.85 %` …
   `+10.68 %`**, 76/80 pair wins). Every cell is strictly above its pooled null
   median.
3. **Per-window disclosure.**
   - Window 1 passes the literal gate 7/8. base-classic-on's `+5.48 %` sat inside
     its `5.79 %` null median; that window's same-binary base-classic-on walls
     spanned 22.3–27.8 s while CPU stayed within 112.5–118.3 s.
   - Window 2 passes 8/8.
   - The pooling rule was written down before window 2 was measured.
   - On the stricter any-pair null lens the pooled result is 7/8.
     base-classic-on (`+4.99 %`) is inside window 1's single worst null pair
     (`11.73 %`). Its CPU cut is strictly separated (`+6.61 %`, 10/10).
4. **Accelerated corroboration (3 pairs):** wall `+7.76 %` … `+18.65 %` (23/24
   wins), CPU lower in 8/8 (`+3.70 %` … `+11.32 %`).
5. **Stage (instrumented, 3 pairs; attribution only):**
   `host:shell_classification` median **`493–503 → 213–218 ms` (`−55.4…−56.9 %`)**
   on benchy and **`3,687–3,936 → 2,401–2,536 ms` (`−32.8…−37.2 %`)** on base.
   The t180 replay predicted `−56.06 %` and `−44.58 %`. The base shortfall is the
   price of the two exactness substitutions described in "Production
   implementation" below (exact polygon/hole filtering instead of rect
   clipping, validated speculation instead of the geometric `None` skip).
6. **Gate counters (deterministic, all 8 cells):** benchy has 18 prep layers in 6
   clusters with 10 recomputes; base has 44 prep layers in 26 clusters with 7
   recomputes. There were **0 speculation misses**: every recompute reproduced
   its precomputed commit, so the whole commit phase ran in parallel.
   ([`reduced/gate_counters.txt`](reduced/gate_counters.txt))

## 2. Guest recompilation drift and host-only attribution

t184 edits no guest source, and no guest calls the edited slicer-core functions.
Their only references are comments. Every guest is still rebuilt, because the
freshness fingerprint covers slicer-core, and the rebuilt WASM differs in all 24
modules. The baseline rebuild reproduced t183's WASM bytes exactly, so the
difference comes from the slicer-core edit, not build noise.

`--profile` shows identical module sets and identical call counts in 8/8 cells.
Fuel drifts by **`−0.70 %` … `−1.36 %`** in total; the largest single module is
the perimeter generator at `−1.44 %` … `−2.00 %`. Output is byte-identical, so
this is codegen, not behaviour ([`reduced/fuel.json`](reduced/fuel.json)).

To keep that drift out of the claim, a host-only batch ran the candidate host
against the **baseline** modules, 3 pairs ([`reduced/host_only.json`](reduced/host_only.json)).

| Cell | Host-only wall | Host-only CPU | Full-arm pooled wall | Full-arm pooled CPU |
|---|---:|---:|---:|---:|
| benchy-classic-off | +20.41 % (3/3) | +5.51 % | +15.15 % | +7.67 % |
| benchy-classic-on | +6.59 % (3/3) | +1.49 % | +8.75 % | +4.85 % |
| benchy-arachne-off | +17.65 % (3/3) | +4.14 % | +16.35 % | +7.11 % |
| benchy-arachne-on | +9.73 % (2/3) | +5.52 % | +8.49 % | +4.93 % |
| base-classic-off | +13.67 % (3/3) | +10.36 % | +12.31 % | +10.68 % |
| base-classic-on | +1.50 % (2/3) | +6.00 % | +4.99 % | +6.61 % |
| base-arachne-off | +12.43 % (3/3) | +9.79 % | +10.72 % | +10.06 % |
| base-arachne-on | +2.12 % (2/3) | +6.19 % | +5.76 % | +5.53 % |

The host change alone carries the CPU cut in every base cell and most of it on
benchy. Benchy CPU attribution is not clean at 3 pairs, and benchy-classic-on's
host-only `+1.49 %` against the full arm's `+4.85 %` suggests part of the
benchy-classic-on CPU cut there is guest codegen. The base supports-on walls are
too noisy at 3 pairs to split. **No host-only claim is made beyond this table.**

## 3. Route (ESTIMATE ONLY: t174 × t175 × t182 × t183 × t184; not a ratio of record)

| Cell | ord post-t183 → post-t184 | acc post-t183 → post-t184 |
|---|---:|---:|
| benchy-classic-off | 1.0341x → **0.8774x** | 1.0370x → 0.8788x |
| benchy-classic-on | 1.4539x → 1.3266x | 1.4130x → 1.2990x |
| benchy-arachne-off | 1.0583x → **0.8852x** | 0.9885x → 0.8128x |
| benchy-arachne-on | 1.2773x → 1.1689x | 1.2637x → 1.1455x |
| base-classic-off | 0.7181x → 0.6297x | 0.7473x → 0.6079x |
| base-classic-on | 0.9734x → 0.9249x | 0.9255x → 0.8537x |
| base-arachne-off | 0.7747x → 0.6916x | 0.7596x → 0.6799x |
| base-arachne-on | 0.9662x → 0.9106x | 0.9173x → 0.8211x |

Both benchy supports-off cells now sit below `1.0x` on the composed estimate.
That claims nothing until the next single-HEAD Orca scoreboard. The binding
cells remain benchy-classic-on (`~1.33x`) and benchy-arachne-on (`~1.17x`).

## 4. Disclosures

- This session's box ran slower than t183's in absolute terms. For example, the
  base-classic-off baseline median was `~12.3 s` here, against t183's `11.02 s`.
  The paired design absorbs that, and no absolute wall is compared across
  windows.
- A first stage-split probe (`probe.raw.diff`, `PNP_T184_PROBE`) was taken on an
  intermediate tree before validated speculation existed. The box was at
  `50–65 %` load from another application at the time, so its absolute numbers
  are not used. It showed the serial recompute chain was a large share of the
  commit phase, which motivated the speculation.
- After the campaign, clippy required three behaviour-neutral edits. In
  production, `settle_speculation`'s `is_none`/`expect` pair became a `match`
  that consumes each speculated entry exactly once, as before. In tests, two
  lint fixes were made. The committed tree is therefore not byte-for-byte the
  measured candidate source. A rebuild of the committed tree (`pnp_cli.exe`
  `be7fe920…`) reproduces t182's ordinary hashes 8/8, and both guest freshness
  checks exit `0` on it.
- The t180 replay harness (`.local-artifacts/perf-vs-orca/t180-shell-replay`)
  was rebuilt against the t184 slicer-core. Its `M0_baseline` stayed
  `EXACT_BIT_IDENTICAL` on all 240 benchy and 496 base layers, which is an
  independent exactness check of the slicer-core changes. Its `E3x` row is the
  exact-filter measurement quoted in "Production implementation".


## 5. Production implementation

All changes are host-side, in `crates/slicer-core` and `crates/slicer-runtime`.
No WIT, guest or module source changed.

1. **E1 — parallel Pass-2 merge and skipped empty differences**
   (`crates/slicer-runtime/src/slice_postprocess_prepass.rs::merge_shadow_contributions`,
   `compute_region_updates`). Pass-2 shadow contributions are grouped by target
   slice, keeping each target's own subsequence in the sequential seed order, and
   the per-target `union` folds run in parallel. The `internal_solid_fill =
   top_solid_fill \ top_solid_seed` step runs in parallel over the timeline and
   is skipped only for slices that received no top Pass-2 shadow. Those slices
   still hold the Pass-1 fill, which is a clone of their seed, so `A \ A` is
   empty. The t180 prototype's second shortcut (`seed empty ⇒ fill.clone()`) was
   **not** taken, because `difference(fill, [])` re-sweeps the fill through
   Clipper and is not guaranteed to return the same ring representation.
2. **E2 — internal-bridge gate.**
   - *Qualify first* (`gate_internal_bridge_sites_clustered`, phase A).
     `qualify_internal_bridge_surface` reads only the surface, the unsupported
     set and the spacing, never the deep-infill area. It now runs once per
     surface in phase A, and a layer none of whose surfaces qualify leaves before
     the depth gather and its offsets. `compute_internal_bridge_layer` applies
     the deep-sparse clip test to the pre-qualified surfaces in the same surface
     order, which retains the identical polygons.
   - *Dependency clusters* (`dependent_layer_clusters`). A commit-phase layer
     reads earlier commits only over its `filled_window_start` and
     `depth_window_start` windows. Layers are partitioned so that every earlier
     prep layer inside a layer's window shares its cluster. A layer whose window
     reaches past the current cluster's first layer (variable layer heights)
     merges the earlier clusters. Clusters run in parallel.
   - *Validated speculation* (`settle_speculation`). Within the windows, every
     layer is evaluated at once under the assumption that each earlier layer
     commits exactly its precomputed polygons. Each cluster is then walked in
     order. While every real commit equals its precomputed polygons the
     speculative outcome stands, because the only thing a later layer reads from
     an earlier one is its committed polygons. From the first mismatch on, the
     cluster's remaining layers are evaluated one at a time against the real
     commits — the plain sequential loop. This replaces the t180 prototype's
     geometric "`None` stays `None`" skip, whose soundness held only up to Clipper
     integer rounding. The speculation is exact by construction and costs the
     same CPU as the old serial recomputes, but runs them in parallel.
   - *`internal_bridge_angles`* (`crates/slicer-core/src/algos/bridge_over_infill.rs`).
     The candidate-invariant `total_fill_area` anchor rings are computed once,
     and each candidate's anchors are that prefix plus its own limiting rings —
     the sequence `boundary_anchor_polylines` produced. The unread final
     `difference(expansion_area, area)` and the first candidate's unread
     collision offset are skipped.
   - *`determine_bridging_angle`* (`AnchorSegmentIndex`, `vote_bridging_angle`).
     A bounding-box hierarchy replaces the `O(S × A)` nearest-anchor scan. It
     evaluates the reference's own distance expression (`/ len2`, not a
     reciprocal multiply), keeps the first segment among equal distances, and
     prunes a node only when its box bound exceeds the best distance by a
     relative `1e-9` plus one unit², so no segment that could win or tie is ever
     skipped. The `O(S²)` vote visits only the three index ranges
     `candidate + kπ ± window` (widened by `1e-9` rad) and applies the
     reference's own test to each sample in ascending index order, so the score,
     the summation order and the replacement rule are the reference's.
3. **E3 — windowed unsupported spans**
   (`bridge_over_infill.rs::unsupported_span_areas_near`, `polygons_touching_box`).
   The unsupported set is read only through `intersection(surface, unsupported)`.
   Lower-layer polygons and holes whose bounding box misses the surfaces'
   bounding box, grown by the chain's offset reach, are dropped. The reach is
   `max(2 m s, 2 s + 2 (1 + m) s) + s + 8 ε`: miter limit 2, fill erosion `m s`,
   solid shrink-grow `s` then `(1 + m) s`, plus one spacing of safety. Nothing is
   clipped, and every kept ring passes through unchanged. The t180 prototype
   instead rect-clipped the operands. That creates new rounded crossing vertices
   that can tilt a long edge by up to half a unit inside the window, so its
   bit-identity was empirical, not structural. In the t180 replay the exact
   filter keeps B5b at 18.7 ms on benchy (rect clip 12.0 ms) and 153 ms on base
   (rect clip 66 ms). That cost is accepted for provable exactness.
4. **E4a — external-bridge direction selector**
   (`crates/slicer-core/src/algos/prepass_slice.rs::min_cost_bridging_candidate`).
   Candidate dedup uses a hash set with the same first-seen order. The cost
   `Σ|e·n|` is estimated for every candidate from sorted folded-angle prefix sums.
   Every candidate within `2 tol` of the best estimate is re-scored with the
   exact reference cost and picked by the reference `(cost, key)` rule, where
   `tol = 16 (E + 8) ε L` bounds the estimate's error. The reference winner is
   always among the re-scored candidates. The t180 prototype ranked candidates
   by the prefix-sum value itself. Restricting the re-scoring to the
   best-estimate candidate alone (mutation `cutoff = min_estimate`) is
   equivalent to that rule, and it fails
   `pruned_bridging_selector_matches_brute_force_including_exact_ties`. The
   failure is on quarter-turn-symmetric edge sets, whose candidates tie in exact
   arithmetic and differ only in summation rounding.
