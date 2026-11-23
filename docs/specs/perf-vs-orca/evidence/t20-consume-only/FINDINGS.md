# Consume-only-when-read annotations (ticket 20)

Route ticket: [Consume-only-when-read annotations](../../issues/20-consume-only-when-read-annotations.md).
This take was authorized by the human lifting the below-fold park and selecting
the ticket explicitly.

**Status: human-approved KEEP; candidate committed `053904aa`.** The 16-cell
proof passes and the standing paired gate passes in all four pre-declared
classic supports-off cells (wall −25.93 % / −34.47 % / −14.38 % / −18.10 %,
CPU corroborating at −37.67 % / −39.61 % / −24.47 % / −21.58 %, 6/6 pairs
everywhere).

## Change

The classifier's early return was already the only semantic consumer of the
two per-vertex overhang stamps on the matched jobs. The candidate makes that
liveness predicate a shared, single definition and lets the producers skip the
queries when it is false:

- `crates/slicer-sdk/src/config_predicates.rs` adds
  `overhang_annotations_consumed` (the exact predicate:
  `enable_overhang_speed` && any `overhang_{1..4}_4_speed` != 0) and a
  producer-side variant that reports inactive for a missing key. The
  classifier calls the required-read variant, so producer and consumer share
  one definition.
- `classic-perimeters` and `arachne-perimeters` declare the five predicate keys
  in their manifests (same types/defaults/denied scopes as
  `overhang-classifier-default.toml`) and read the predicate once per
  `run_perimeters`.
- `PathAnnotationMode::Unconsumed` (`crates/slicer-core/src/perimeter_spatial.rs`)
  emits neither annotation and runs neither query. Classic's planar wall path
  uses `Planar` when live and `Unconsumed` when dead; its non-planar shell path
  uses `NonPlanarNoQuartile` when live and `Unconsumed` when dead. Arachne
  skips both stamp loops when dead and keeps the pre-take stamps when live.
- `is_bridge` is deliberately untouched: the t18 split localizes the hot cost
  in the two exact scan families, and `is_bridge` has no consumer liveness
  predicate. Classic's missing `bridge_areas.is_empty()` parity guard remains a
  separate observation.

When the predicate is live, the exact pre-take queries and modes run, so the
change is value-identical wherever the stamps are read.

## Dynamic premise check

One ordinary base classic-supports-off slice at `RAYON_NUM_THREADS=12` with a
temporary `T20-PROBE` log in the classifier printed:

```text
T20-PROBE classifier consumed=false layers=495
```

The probe was removed (grep-guarded by `build-snapshots.ps1`) before any
snapshot build. That cell's output hash `20e7204c...` equals the preserved t58
base-classic-off reference.

## Protocol

- Snapshots: `build-snapshots.ps1` builds baseline (pre-take HEAD) and
  candidate arms from the same working tree by stashing the seven build
  inputs for the baseline build; each arm has its own ordinary and accelerated
  `pnp_cli.exe` + `modules/` tree under
  `.local-artifacts/perimeter-reference-preparation/t20-consume-only/snapshots/`.
- Proof: all 16 cells ({benchy, base} x {classic, arachne} x supports off/on x
  ordinary/accelerated), one run per arm, byte-identical and equal to the
  preserved t58 references (`build-qualification.py`).
- Standing gate: the four pre-declared classic supports-off cells,
  6 alternating pairs per cell plus 2 warmups per arm,
  `RAYON_NUM_THREADS=12`, external `GetProcessTimes`
  (`run-all-measure.ps1`, `reduce_ab.py`).
- Verification: `verify.py` reads only the committed reduced set and rejects
  four negative controls.

## Results

### Output identity (proof gate)

**PASS 16/16.** One baseline and one candidate run per cell, all 16 cells
({benchy, base} x {classic, arachne} x supports off/on x ordinary/accelerated),
every pair byte-identical and equal to its preserved t58 reference
(`qualification.json`, `pass: true`). Every completion is `ok`,
`degraded=false`, zero fatal/non-fatal errors, with non-vacuous role counts.

Proof warmup walls (single runs per arm, **not** the paired gate; context only):

| cell | baseline wall s | candidate wall s |
| --- | ---: | ---: |
| ordinary benchy classic-off | 15.521 | 10.550 |
| ordinary benchy classic-on | 22.879 | 18.908 |
| ordinary benchy arachne-off | 9.857 | 10.113 |
| ordinary benchy arachne-on | 18.965 | 18.241 |
| ordinary base classic-off | 224.884 | 160.586 |
| ordinary base classic-on | 382.638 | 327.319 |
| ordinary base arachne-off | 117.587 | 104.003 |
| ordinary base arachne-on | 312.602 | 297.130 |
| accelerated benchy classic-off | 13.806 | 13.632 |
| accelerated benchy classic-on | 19.070 | 17.339 |
| accelerated benchy arachne-off | 8.920 | 9.834 |
| accelerated benchy arachne-on | 17.308 | 16.580 |
| accelerated base classic-off | 149.388 | 125.669 |
| accelerated base classic-on | 352.297 | 306.909 |
| accelerated base arachne-off | 118.725 | 101.949 |
| accelerated base arachne-on | 290.343 | 236.868 |

### Standing paired gate

**PASS in all four pre-declared cells** (6/6 retained pairs each, zero
starvation exclusions, every pair's output byte-identical):

| cell | baseline median wall | candidate median wall | paired wall Δ | paired CPU Δ |
| --- | ---: | ---: | ---: | ---: |
| ordinary benchy classic-off | 12.367 s | 9.282 s | **−25.93 %** | −37.67 % |
| ordinary base classic-off | 157.775 s | 103.396 s | **−34.47 %** | −39.61 % |
| accelerated benchy classic-off | 10.192 s | 8.733 s | **−14.38 %** | −24.47 % |
| accelerated base classic-off | 120.004 s | 98.399 s | **−18.10 %** | −21.58 % |

The wall win is corroborated by a larger CPU reduction in every cell: this is
work removal, not a scheduling change. The mechanism matches the ticket's
premise — the two exact scan families (`inside_scan`, `quartile_scan`) were
57.9 % of the accelerated base classic module fuel in t18 and 69.8 % of
classic guest fuel in t08, and the matched configs keep the classifier's
liveness predicate false on every layer.

**Recommendation: KEEP — human-approved 2026-10-22; candidate committed
`053904aa`.**

### Caveats

- The proof warmup walls above are single runs, not the paired gate. The gate
  table is the verdict evidence.
- The gate cells are classic supports-off. Arachne and supports-on output
  identity is proven in all 16 cells, but their wall effect was not timed in
  this take; the proof warmups show the same direction on base classic-on and
  base arachne cells.
- The candidate does not touch `is_bridge`; classic's missing
  `bridge_areas.is_empty()` parity guard remains a separate observation.
- Measured snapshots were built probe-free (the `T20-PROBE` line was removed
  before `build-snapshots.ps1`, which grep-guards it). The standing accelerated
  dist was archived to `target/t20/archive-dist-accelerated-dev` and restored
  after both arms; the candidate ordinary dist is parked at
  `target/t20/candidate-dist-ordinary/`.
- `verify.py` exits 0 from the committed reduced set alone and rejects four
  negative controls.


