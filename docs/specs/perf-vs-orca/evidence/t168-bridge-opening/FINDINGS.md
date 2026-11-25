# t168 findings: the bridge opening's auto-sentinel defect

**Verdict: canonical-correctness fix, landed. The bridge opening now resolves
canonical's auto sentinel instead of running at radius zero; the bridge path
falls in 8/8 cells, process CPU is lower in 8/8 cells on both build arms, and
no cell's wall effect clears the measured null band. The standing 0.80 % gate
fails on wall (0 of 4 `base` cells clear their null band), so nothing is banked
as speed.**

Resolved 2026-11-19. Ticket:
[168](../issues/168-bridge-opening-auto-sentinel.md).

## 1. The defect

Canonical `PrintObject::detect_surfaces_type` labels the unsupported bottom set
with

```text
bottom = opening_ex(diff_ex(slices, lower_slices, ApplySafetyOffset::Yes),
                    external_perimeter_width / 10)
```

`classify_unsupported_bridge_areas`
(`crates/slicer-core/src/algos/prepass_slice.rs`) implemented all of it except
the width: it called `resolve_external_perimeter_width_mm`, which returned
`ConfigValue::Float(0.0)` verbatim. The matched job authors
`outer_wall_line_width = 0`, canonical's **auto** sentinel, so the radius was
`0.1 x 0 = 0` and the opening deleted nothing. Every sub-printable sliver in the
unsupported diff therefore survived as a bridge.

The same sentinel had already been found and fixed on the shell side (t166,
`resolve_shell_external_width_mm`,
`crates/slicer-runtime/src/slice_postprocess_prepass.rs`). The two resolvers had
drifted: the shell copy required a positive value, the bridge copy did not.

## 2. The fix

One policy, one function: `resolve_external_perimeter_width_mm`
(`crates/slicer-core/src/algos/prepass_slice.rs`) now requires a **positive**
absolute `outer_wall_line_width` (both the `Float` and the absolute
`FloatOrPercent` form), else the typed `line_width`, else 0.4 mm. It is `pub`,
and `resolve_shell_external_width_mm`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) is now a one-line
delegate to it, so the shell and bridge openings cannot drift again.

Nothing else changed: the safety offset stays at `SCALED_EPSILON_MM`
(`offset(previous, 0.0001, Square)`), the miter limit stays 3.0, and the
classifier's clip to `region.infill_areas` is untouched.

### Falsifier, proven red before the fix

`external_perimeter_width_treats_zero_extension_as_auto` asserts that
`Float(0.0)` resolves to the typed width (0.4 here) and that the opening radius
`0.1 x width` is positive. Restoring the old arm
(`Some(ConfigValue::Float(value)) => *value as f32`) makes it fail with
`left: 0.0, right: 0.4`; the guard was then restored. A second case,
`external_perimeter_width_treats_zero_percent_form_as_auto`, pins the same
fallback for the zero absolute `FloatOrPercent` form.

## 3. Campaign

One window. Manifest (`target/t168/manifest.json`, reproduced in
`reduced/output.json`):

| arm | exe sha256 | module tree |
| --- | --- | --- |
| baseline ordinary | `28d16ac7…` | `6899f824…` |
| candidate ordinary | `3e87a6e8…` | `6899f824…` |
| baseline accelerated | `1391c550…` | `82d74609…` |
| candidate accelerated | `c5c7280f…` | `82d74609…` |

Each build mode shares one module tree: the change is host-only, and the 49
guests rebuilt (the fingerprint covers `slicer-core` sources) are byte-equal in
behaviour — the baseline arm reproduces the pre-change reference set in 8/8,
which is the control that makes that claim rather than assumes it.

**Output (8 cells x 2 ordinary arms, plus 8 x 2 accelerated arms).** The
baseline reproduces t166's active set in **8/8**; the candidate moves **8/8**;
the two accelerated arms are byte-identical to their ordinary counterparts in
**16/16**.

| cell | baseline | candidate | bytes |
| --- | --- | --- | --- |
| benchy classic off | `276782a7…` | `d6cb4acd…` | −2,373 |
| benchy classic on | `44564d29…` | `21c6969f…` | −2,327 |
| benchy arachne off | `46111bb0…` | `5c0f846e…` | −2,112 |
| benchy arachne on | `12c5e45c…` | `6fe4b09b…` | −2,218 |
| base classic off | `0a1ee3df…` | `9e360450…` | +24,435 |
| base classic on | `bc5cb5e1…` | `f77cf265…` | +24,313 |
| base arachne off | `cadebffa…` | `cd896bab…` | −5,288 |
| base arachne on | `77f2b655…` | `3044c4d6…` | −5,328 |

All 286 campaign runs report `degraded: false` and zero code-1200 exact-Z
diagnostics (108,350 event records scanned).

## 4. Delta audit

`audit_t168.py` reads the campaign's G-code and writes `reduced/audit.json`.

**Sections that move (path, mm; candidate − baseline).** Only three of the four
audited sections move at all; `Gap infill` is 0.0 in 8/8.

| cell | Bridge | Internal Bridge | Sparse infill |
| --- | --- | --- | --- |
| benchy classic off / on | −10.5 | +3.6 | −5.0 |
| benchy arachne off / on | −5.0 | +3.6 | −5.0 |
| base classic off / on | −11.4 | +2.0 | +490.2 |
| base arachne off / on | −8.1 | +2.0 | −18.0 |

**Direction.** Total bridge path falls in **8/8** (the expected direction) and
internal bridge rises in 8/8. Sparse infill rises only on `base classic`
(+490.2 mm, +0.28 % of that section); it **falls** by 5.0 mm on the benchy cells
(0.02 % of 24.2 m) and by 18.0 mm on `base arachne` (0.01 % of 179.1 m). That is
a partial reversal of the ticket's expected direction and is reported, not
widened. The per-layer census shows why it is not a simple split: bridge path
**grows** on 19–26 layers and **falls** on 27–33 of the affected base layers
(benchy: +2 / −4 of 7), so the opening relabels the bottom set and the
downstream partition re-flows rather than shrinking monotonically. First
differing layer is 3 in every cell.

**Bridge-anchor direction.** `bridge_orientation_deg` is not printed, so the
proxy measured here is the length-weighted dominant angle of the `Bridge`
section's extruding segments (doubled-angle circular mean). It moves by
>= 45 degrees on **12 of 48** base-classic bridge layers, **13 of 61**
base-arachne and **2 of 7** benchy — a genuine re-derivation of the direction on
those layers, which is what canonical does with its post-opening area (the
orientation is derived from the surviving bridge area, not from the raw
overhang). The proxy is labelled: it cannot distinguish a true orientation flip
from a change in which sub-area dominates the segment mix.

## 5. Paired wall and CPU (ordinary arm, 10 interleaved pairs per cell)

Positive = candidate faster. The null band is the same instrument measured in
the same window (`-Mode ab -NullControl`), shown any-pair / median.

| cell | wall median (spread) | wall wins | CPU median | CPU wins | null wall | null CPU |
| --- | --- | --- | --- | --- | --- | --- |
| benchy classic off | +1.99 (−24.51…+6.07) | 7/10 | +4.49 | 10/10 | 5.44 / 4.12 | 2.85 / 2.67 |
| benchy classic on | +1.29 (−11.51…+4.23) | 6/10 | +1.89 | 8/10 | not measured | not measured |
| benchy arachne off | +1.17 (−1.20…+8.07) | 8/10 | +5.38 | 10/10 | not measured | not measured |
| benchy arachne on | +1.71 (−5.01…+7.27) | 8/10 | +2.27 | 9/10 | not measured | not measured |
| base classic off | +4.09 (−5.96…+12.32) | 7/10 | +3.46 | 10/10 | 4.89 / 3.05 | 1.23 / 0.20 |
| base classic on | −0.14 (−2.63…+5.36) | 5/10 | +0.99 | 10/10 | 9.86 / 1.19 | 1.54 / 0.34 |
| base arachne off | +4.15 (−8.29…+8.13) | 8/10 | +3.38 | 10/10 | 9.46 / 4.75 | 0.73 / 0.34 |
| base arachne on | +0.73 (−8.16…+15.55) | 6/10 | +0.74 | 8/10 | 3.85 / 1.75 | 0.61 / 0.24 |

**Gate: FAILED on wall, 0 of 4 `base` cells.** The two supports-off base cells
clear the 0.80 % floor (5.11x / 5.19x) but their medians sit **inside** the
null band's any-pair bound (4.09 < 4.89 and 4.15 < 9.46), and the two
supports-on base cells are at −0.14 % and +0.73 %. The benchy clause passes:
no median regression, all four positive.

**Process CPU is the one consistent channel: lower in 8/8 cells**, 10/10 pairs
in five of them, on both arms. Against the per-cell null it is outside on
`base classic off` (+3.46 vs 1.23) and `base arachne off` (+3.38 vs 0.73),
inside on `base classic on` (+0.99 vs 1.54) and marginal on `base arachne on`
(+0.74 vs 0.61). The direction is what a fix that deletes work should show; the
size is not separable from the instrument on the supports-on cells.

## 6. Accelerated corroboration (3 pairs per cell)

| cell | wall median | wall wins | CPU median | CPU wins |
| --- | --- | --- | --- | --- |
| benchy classic off | +0.12 | 2/3 | +4.32 | 3/3 |
| benchy classic on | +0.71 | 3/3 | +0.35 | 2/3 |
| benchy arachne off | +3.57 | 3/3 | +5.43 | 3/3 |
| benchy arachne on | +6.09 | 3/3 | +2.96 | 3/3 |
| base classic off | +0.53 | 2/3 | +2.49 | 3/3 |
| base classic on | +2.07 | 2/3 | +1.41 | 3/3 |
| base arachne off | −2.96 | 1/3 | +3.48 | 3/3 |
| base arachne on | +2.75 | 2/3 | +0.89 | 3/3 |

Wall is mixed and unresolvable at three pairs (t163 bounds the arm at
−13.2…+4.4 %); CPU is lower in 8/8, agreeing with the ordinary arm. Output is
identical 16/16.

## 7. Disclosures

- **No speed is banked.** The wall gate fails; no ratio of record is quoted
  from this take and the map's route arithmetic is unchanged. The change is a
  canonical-parity fix and lands on that ground.
- **The wall instrument is weak in this window.** The null control's median
  band reaches 4.75 % on one cell and one benchy pair recorded −24.51 % on a
  2.7 s job. Those are the instrument's numbers, not the candidate's.
- **Null coverage is partial:** three pairs each on three cells (first window)
  and five pairs each on the two base supports-on cells (second window). The
  three remaining benchy cells have no null band of their own; they carry no
  speed claim.
- **Ten pairs per cell, not the standing five.** The first five pairs produced
  a −3.07 % median on `benchy classic off` and a −4.76 % median on
  `base arachne on`; the campaign was extended to ten before any verdict, and
  both moved to positive medians. The extension is disclosed as a
  sequential decision; the first-half and second-half medians are in the raw
  rows.
- **The anchor-direction measurement is a proxy**, not `bridge_orientation_deg`
  (not printed): the dominant segment angle of the printed bridge lines.
- **Box load** was sampled, not controlled: 43 % at the first A/B start, 62 % at
  the extension start, 3 % / 0 % / 2 % / 4 % at the respective ends. Nothing was
  measured on an idle box.
- **Accelerated arm has no null band** (3 pairs per cell, corroboration only).
- The `bridge_areas` consumers named in the ticket (`sparse_infill_area` in
  `crates/slicer-runtime/src/region_partition.rs`, per-vertex `is_bridge` in
  `crates/slicer-core/src/perimeter_spatial.rs`, and
  `detect_bridging_direction_deg`) all see the moved set; this take measures the
  printed consequences, not each consumer's internal value.

## 8. Files

- `crates/slicer-core/src/algos/prepass_slice.rs` — resolver policy + two tests.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — one delegate.
- `run_t168.ps1`, `reduce_t168.py`, `audit_t168.py`, `verify.py`, `REPRODUCE.md`
  and `reduced/*.json` in this directory.
- `verify.py` PASS (8 cells x 10 pairs, reference reproduction 8/8, accelerated
  identity 8/8, CPU lower 8/8 on both arms, bridge falls 8/8), `--self-test` 8/8
  negative controls rejected; `audit_t168.py --self-test` 5/5.
