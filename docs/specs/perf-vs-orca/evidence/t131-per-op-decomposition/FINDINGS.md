# t131 — Common-stage per-op cost decomposition

Route: [Common-stage per-op cost decomposition](../../issues/131-common-stage-per-op-cost-decomposition.md).
Take: 2026-11-09, ordinary arm, a temporary probe build at HEAD `6c6c188b`
(probe reverted; [`probe.patch`](probe.patch) is the only artifact), the
standing matched-pair configs, 24 external core modules, `RAYON_NUM_THREADS=12`,
one probe pass per cell on the four classic cells, plus one `PNP_T131_S5` pass
per cell for the windowed-offset shape. Attribution only: no production edit,
no G-code reference move, both guest-freshness checks untouched.

Every number below is printed by [`arithmetic.py`](arithmetic.py) from
[`summary.json`](summary.json); [`verify.py`](verify.py) re-derives them and
rejects negative controls. Stage walls and ratios of record remain
[t126](../t126-post-t123-phase-split/FINDINGS.md)'s and
[t124](../t124-post-t123-scoreboard/FINDINGS.md)'s.

## Verdict

**The per-op wrapper is not the lever. On every common prepass stage the
Clipper2 sweep holds 98.2–99.9 % of the measured primitive time; path
conversion, WIT marshalling, PolyTree rebuild and allocation together hold
0.1–1.8 %. The 9.8–15.7x is bought by operand size, not by the code around the
kernel — and the one remaining scheduling lever is the planner's 2.19x
effective parallelism against 8.3–11.8x elsewhere.**

Four findings:

1. **No-GO for a per-op (conversion / allocation / rebuild / marshal) rewrite.**
   The largest non-sweep overhead measured on any stage in any cell is
   **473 ms** (base-on `SupportGeometry`, a 56.3 s stage); across the seven
   common stages of base classic-off the whole non-sweep total is **543 ms of
   280,114 ms** of primitive thread-time (**0.19 %**), and on base classic-on
   **1,075 ms of 516,853 ms** (**0.21 %**). The ticket's first gate branch
   ("per-op overhead at least the stage's share of the cell's required cut")
   fails by two orders of magnitude.
2. **The sweep is not slow; the operand is big.** PnP pushes **44.5 M points**
   through `ShellClassification`'s sweep on base classic-off (**89,825 per
   layer**, 9,650 calls, 4,608 points per call) and **293.2 M points** through
   the planner on base classic-on (**592,363 per layer**, 143,766 calls). t70
   measured the Rust Clipper2 at parity with the C++ twin, so the 7.0x / 6.7x
   stage-to-whole-phase ratios are operand volume inside the kernel, not
   kernel throughput and not the wrapper. t129's list of suspects is corrected
   accordingly (§8).
3. **S5 (bbox-clipped internal-bridge offsets) is a real 21.0 s thread-time
   ceiling on base classic-off — but not output-exact as named.** The two
   whole-layer offsets cost **22,964 ms** of thread-time over 130 layers; the
   same offsets on a candidate-bbox-windowed operand cost **1,937 ms**
   (**16.06x** cheaper positive pass, **8.81x** negative) and are
   wall-equivalent to **2,520 ms** at the stage's measured 8.34x parallelism. But the windowed
   result differs from the whole-layer result on **15 of 130** positive layers
   and **6 of 42** negative layers (max **0.71 mm²** / **0.59 mm²**; net
   **−1.27 mm² / +1.18 mm²** = **−0.039 % / +0.076 %** of the clip area), so it
   is a **bounded-delta candidate that needs the 16-cell contract and a delta
   audit**, not an exact shape.
4. **S6 (planner occupancy cache) is exact by construction and worth 14.2 s of
   duplicate thread-time — half of t113's stated ceiling, not all of it.** The
   `+0.35 mm` occupancy family on base classic-on is **864 calls over 433
   distinct payloads** (exactly two calls per payload) costing **28,521 ms**
   host-side; a content-keyed memo removes the **431 duplicate calls =
   14,228 ms** of thread-time, **6,504 ms** wall at the stage's 2.19x
   parallelism. The paired `−0.6 / −0.8 mm` collision family is **1,350 calls
   over 1,350 distinct payloads** — **zero** memo hits, so t113/t129's 32.7 s
   cannot be read as a cache ceiling (§8).

**Go/no-go:** **NO-GO for a per-op rewrite; GO (narrowed) for operand locality
and for the planner's parallel structure.** The ticket's stop rule does **not**
fire: the overhead is indeed mostly the sweep, but the operands are *not*
un-localizable — S6 localizes exactly, and S5 localizes with a bounded, measured
delta. What dies is the belief that conversion, allocation or marshalling is
where the 15.7x lives.

## 1. The measured split

The probe times every `slicer-core::polygon_ops` primitive at four seams:
`wit_in` / `wit_out` (the host↔WIT deep clone in `slicer-wasm-host`'s
`offset_polygons`, `offset_polygons_batch` and `clip_polygons`), `conv_in`
(`ExPolygon` → `Path64`), `sweep` (the Clipper2 `execute`/`execute_tree`), and
`rebuild` (`PolyTree` → `ExPolygon`, including every `Vec`/`Polygon`
allocation it makes). Durations are thread-accumulated (nanoseconds) and
attributed to the enclosing prepass stage by snapshotting the global counters
at the stage bracket; the stage walls come from the runtime's existing
`--instrument-stderr` events. Cell walls are **not** claimed from these runs.

Base classic-off (ms; `calls` are primitive invocations, `overhead` is
`wit_in + conv_in + rebuild + wit_out`):

| stage | wall | calls | sweep | overhead | sweep share | points/call | thread/wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `PrePass::MeshAnalysis` | 4,544 | 3,248 | 4,634.1 | 48.9 | 98.96 % | 616.5 | 1.03x |
| `PrePass::OverhangAnnotation` | 10,935 | 6,314 | 128,826.1 | 241.4 | 99.81 % | 9,145.4 | 11.80x |
| `PrePass::SeamPlanning` | 1,856 | 495 | 12,933.0 | 15.6 | 99.88 % | 5,158.7 | 6.98x |
| `PrePass::ShellClassification` | 14,250 | 9,650 | 118,735.6 | 180.2 | 99.85 % | 4,607.6 | 8.34x |
| `PrePass::Slice` | 2,229 | 3,433 | 14,984.7 | 56.5 | 99.62 % | 4,446.5 | 6.75x |
| `PHASE::PerLayer` | 7,033 | 24,377 | 59,290.3 | 461.7 | 99.23 % | — | 8.50x |

Base classic-on:

| stage | wall | calls | sweep | overhead | sweep share | points/call | thread/wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `PrePass::MeshAnalysis` | 4,137 | 3,248 | 4,570.7 | 43.7 | 99.05 % | 616.5 | 1.12x |
| `PrePass::OverhangAnnotation` | 9,018 | 6,314 | 105,751.0 | 159.4 | 99.85 % | 9,145.4 | 11.74x |
| `PrePass::SeamPlanning` | 1,689 | 495 | 12,701.9 | 14.4 | 99.89 % | 5,158.7 | 7.53x |
| `PrePass::ShellClassification` | 12,963 | 9,650 | 113,701.1 | 156.4 | 99.86 % | 4,607.6 | 8.78x |
| `PrePass::Slice` | 2,065 | 3,433 | 13,454.3 | 53.9 | 99.60 % | 4,446.5 | 6.54x |
| `PrePass::SupportAnalysis` | 12,665 | 4,922 | 144,014.3 | 174.5 | 99.88 % | 7,165.7 | 11.38x |
| `PrePass::SupportGeometry` | 56,289 | 143,766 | 122,657.6 | 473.0 | 99.62 % | 2,039.6 | 2.19x |
| `PHASE::PerLayer` | 11,211 | 29,782 | 84,518.8 | 464.2 | 99.45 % | — | 7.58x |

The benchy controls agree (shell **99.69 % / 99.68 %** at 1,841 points/call;
planner **98.86 %** at 759 points/call; see §9 for the full rows). The
per-layer phase is the same story at **98.31–99.45 %**.

**Non-sweep totals.** base-off **543 ms of 280,657 ms = 0.19 %**;
base-on **1,075 ms of 517,928 ms = 0.21 %**; benchy-off **74 ms of 17,703 ms =
0.42 %**; benchy-on **175 ms of 31,108 ms = 0.56 %**. The largest single
non-sweep bucket is the planner's path conversion (**383.3 ms**), the largest
PolyTree rebuild is `OverhangAnnotation`'s (**97.7 ms**), and the largest WIT
marshal is base-on `SupportGeometry`'s (**31.7 ms** of 123,131 ms). The largest
*relative* non-sweep bucket is `MeshAnalysis`'s path conversion on base-off
(**47.9 ms of 4,683 ms = 1.02 %**).

## 2. Op count versus per-op cost

The ticket asks for the two columns kept separate. Measured on base:

| stage | calls/layer | ms/call | points/call | points/layer | canonical ops/layer (t129) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ShellClassification` | 19.5 | 12.31 | 4,608 | 89,825 | 45–60 (incl. wave expansion) |
| `OverhangAnnotation` | 13.0 | 20.40 | 9,145 | 119,060 | 3 + per-vertex distance |
| `SeamPlanning` | 1.0 | 26.13 | 5,159 | 5,159 | 0 offsets |
| `Slice` | 6.9 | 4.37 | 4,447 | 30,838 | 1 mesh walk + 3 passes |
| `SupportGeometry` (planner) | 290.4 | 0.85 | 2,040 | 592,363 | ~10–14 layer-level booleans + per-node circles |

PnP's shell op count (**19.5/layer**) sits inside canonical's 15–25 range, and
its whole stage still costs **12.96–14.25 s** against canonical's entire
2.10 s prepass. Because the kernel is at parity (t70) and the wrapper is
≤1.8 %, the difference has to be in what each op is fed: PnP hands the kernel
**4,608 points per shell call and 2,040 per planner call, 290 planner calls per
layer**, i.e. **592,363 points per layer** through the planner alone. The
totals are **44,463,545 points** through `ShellClassification` on base
classic-off and **293,219,629 points** through the planner on base classic-on.
Canonical feeds bbox-clipped operands
(`clip_clipper_polygons_with_subject_bbox`) and caches collision/avoidance per
`{radius, layer}`, so its calls see operands that are local to a node or a
wave seed.

This is the measured form of t129's inference: **op count is comparable, and
the per-op cost difference is operand volume, not plumbing.**

## 3. What the overhead is not

- **Path conversion is not a lever.** `ExPolygon` → `Vec<Point64>` costs
  1.7–383 ms of thread-time per stage (**0.03–1.8 %** of that stage's own
  sweep; the 1.8 % is `MeshAnalysis` on benchy-off, a 96 ms sweep). On the two
  carrying stages it is **0.09 %** (shell, 109.6 of 118,735.6) and **0.31 %**
  (planner, 383.3 of 122,657.6).
- **The host↔WIT deep clone is not a lever.** It is measurable only on the
  guest-module path (`SeamPlanning`, 7.2 ms; `SupportGeometry`, 31.7 ms). Host
  built-ins pay zero (same address space).
- **PolyTree rebuild is not a lever.** 0.8–97.7 ms per stage, **≤0.08 %** of
  any stage's sweep.
- **Allocation is distributed, not separable here.** The input `Path64`
  vectors are allocated inside `conv_in`, the output `ExPolygon`/`Polygon`
  vectors inside `rebuild`, and the engine's internal allocation inside the
  `sweep`. A separate allocator shim was not built: with the three measured
  buckets already at ≤1.8 %, the remaining allocation can only be smaller than
  the `sweep` bucket it sits in, and its removal cannot approach the gate.
- **Marshalling/dispatch around the call is not a lever either.** The whole
  host import body (marshal + primitive + marshal back) is what the S6 census
  timed: 57.9 s for all 33,652 singular offset imports on base-on, against a
  stage total of 122.7 s of primitive sweep — the wrapper is inside the noise.

## 4. The scheduling lever: the planner runs at 2.19x

Thread-sum ÷ stage wall is the effective parallelism of each stage:

| stage | base-off | base-on | benchy-off | benchy-on |
| --- | ---: | ---: | ---: | ---: |
| `MeshAnalysis` | 1.03x | 1.12x | 0.65x | 0.65x |
| `OverhangAnnotation` | 11.80x | 11.74x | 9.95x | 11.13x |
| `SeamPlanning` | 6.98x | 7.53x | 4.95x | 4.82x |
| `ShellClassification` | 8.34x | 8.78x | 5.25x | 5.26x |
| `Slice` | 6.75x | 6.54x | 6.79x | 6.88x |
| `SupportAnalysis` | — | 11.38x | — | 5.90x |
| `SupportGeometry` (planner) | — | **2.19x** | — | **1.68x** |

The planner carries **122.7 s of sweep thread-time** on base classic-on and
delivers it in **56.3 s** of wall. At `ShellClassification`'s 8.78x the same
work would be **14.0 s**; at 4x it would be **30.7 s**. Even the conservative
4x case is **25.6 s**, i.e. **28 %** of base classic-on's 92.5 s required
prepass cut — larger than S5 and S6 combined on that cell. `MeshAnalysis` is
effectively serial (1.03–1.12x) but only 4.6 s of thread-time, so its ceiling
is 4.1 s.

This is a *measured* scheduling opportunity, not a claim that the planner can
be parallelised to 8x: its drop loop and per-layer contacts are order-bearing,
and the guest module's parallel structure was not profiled here. It is
graduated as its own research ticket rather than priced as a shape.

## 5. Named shapes

### 5.1 S5 — internal-bridge offsets clipped to the candidate bbox

Measured on the `PNP_T131_S5` passes: for every layer with candidates, both
whole-layer `deep_infill_area` offsets are timed, then re-run on
`deep_infill_area ∩ window` where the window is the candidate bbox expanded by
`6 × qualify_spacing`. The result is compared with the whole-layer result
inside the *unexpanded* candidate bbox, which is the region every consumer
uses.

| cell | layers | positive whole → windowed | negative whole → windowed | wall-equivalent | exact pos | exact neg | max abs Δ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 56 | 603.6 → 68.1 ms (8.86x) | 594.9 → 62.7 ms (9.48x) | 203 ms | 52/56 | 29/30 | 31.19 mm² |
| benchy classic-on | 56 | 591.4 → 63.9 ms (9.26x) | 572.1 → 56.7 ms (10.08x) | 198 ms | 52/56 | 29/30 | 31.19 mm² |
| base classic-off | 130 | 13,052.3 → 812.9 ms (16.06x) | 9,911.4 → 1,124.4 ms (8.81x) | 2,520 ms | 115/130 | 36/42 | 0.71 mm² |
| base classic-on | 130 | 13,162.6 → 805.6 ms (16.34x) | 9,923.8 → 1,185.9 ms (8.37x) | 2,402 ms | 115/130 | 36/42 | 0.71 mm² |

Wall-equivalent uses each cell's own ShellClassification parallelism
(5.25x / 5.26x / 8.34x / 8.78x). Net area: base **−1.27 mm² / +1.18 mm²**
(−0.039 % / +0.076 %); benchy **+31.11 mm² / +0.31 mm²** (+1.03 % / +0.05 %),
dominated by one layer (52) whose windowed positive offset gains 31.19 mm²
inside the candidate bbox.

**Verdict: measured ceiling, not an exact shape.** The time saving is real and
large (21.0 s of base-off thread-time, 7.4x the 340 ms floor), but the
windowed operand is **not output-equivalent** at the layer level: the operands
are NonZero winding sets and clipping them re-normalises the fill, so a
formulation that clips the *operand* must be proven against the 16-cell
reference set, not assumed. Clipping the *result* is trivially exact and saves
nothing. This is a candidate for a build ticket under the standing 16-cell
contract, with the layer-level delta as its risk.

### 5.2 S6 — planner occupancy from a content-keyed memo

The census hashes every singular `offset_polygons` import's payload (every
point and ring boundary) and groups by `(delta_mm, miter, arc)`:

| cell | family | calls | distinct | repeat | total | memo saving | wall-equivalent |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-on | occupancy `+0.35` | 864 | 433 | 431 | 28,521.4 ms | **14,227.7 ms** | **6,504 ms** (2.19x) |
| base classic-on | collision `−0.6/−0.8` | 1,350 | 1,350 | 0 | 19,492.2 ms | 0 | 0 |
| benchy classic-on | occupancy `+0.35` | 365 | 186 | 179 | 1,243.0 ms | 609.6 ms | 363 ms (1.68x) |

The occupancy family is exactly **two calls per payload** on the matched job:
the per-layer emit check and the per-origin role carve inflate the same layer
with the same radius. A memo keyed on the payload content is **output-identical
by construction** (the offset is a pure function of `(polygons, delta, join,
arc, miter)`), which makes S6 the one *exact* shape this take graduates.
t113's 32.7 s / 864 calls was the whole occupancy term; a cache can only remove
the duplicate half — **14.2 s**, not 32.7 s. The collision family is a null
result: 675 `−0.6` and 675 `−0.8` calls are all content-distinct (the two
deltas never repeat a payload), so it is not a memo shape.

Ceiling: **6.5 s of base-on wall = 6.4x the 1,010 ms floor = 7.0 % of the
cell's required prepass cut**, 11.5 % of the stage wall and 12.8 % of the
planner's own 50,660 ms share of that cut.

## 6. The gate

The ticket's first branch — *the measured per-op overhead is at least the
stage's share of the cell's required cut* — **fails everywhere**:

| stage | non-sweep overhead | stage wall | stage's share of the cell's required prepass cut | overhead as % of that share |
| --- | ---: | ---: | ---: | ---: |
| base-off `ShellClassification` | 180.2 ms | 14,250 ms | 11,970 ms (51.4 % of 27,722) | **1.5 %** |
| base-off `OverhangAnnotation` | 241.4 ms | 10,935 ms | 9,185 ms (39.4 %) | **2.6 %** |
| base-on `SupportGeometry` | 473.0 ms | 56,289 ms | 50,660 ms (60.9 % of 92,477) | **0.9 %** |
| base-on `SupportAnalysis` | 174.5 ms | 12,665 ms | 11,399 ms (13.7 %) | **1.5 %** |

The second branch — *a named operand-locality shape's measured ceiling is at
least the stage's share* — **also fails literally**: S5 is 17.7 % of the shell
stage's share, S6 is 11.5 % of the planner's share, and the planner-parallelism
ceiling (§4) is 50 % of it at a conservative 4x but is a scheduling shape, not
an operand one. So the literal per-stage gate fires, exactly as it did in
t129 — but the **route-level stop rule does not**: the operands *can* be
localised (S6 exactly, S5 with a bounded delta), so the "stop and return to the
destination ruling" branch is not triggered.

## 7. Route arithmetic against the destination

With the sweep confirmed as the cost and the wrapper dead, the destination's
58–79 % wall cut must come from *fewer or smaller sweep operands*. The shapes
this take prices contribute, on their own cells:

| shape | wall-equivalent | × 0.80 % floor | % of the cell's required prepass cut |
| --- | ---: | ---: | ---: |
| S5 (base-off) | 2,520 ms | 7.4x | 9.1 % |
| S6 (base-on) | 6,504 ms | 6.4x | 7.0 % |
| planner at 4x parallelism (base-on) | 25,625 ms | 25.4x | 27.7 % |

No combination of S5 + S6 reaches either cell's cut, which is consistent with
t129's route finding and now has a mechanism: the shapes remove *duplicate* or
*out-of-window* work, while the bulk of the sweep is *necessary* work on
whole-layer operands. The destination therefore needs the sweep's **total
operand volume** cut by the same fraction as its wall (58–79 %), which means
the work-set/algorithm changes t129 listed (per-vertex overhang distance,
per-layer overhang polygons, canonical loop projection) **plus** the planner's
parallel structure — not wrapper optimisations.

## 8. Ledger corrections

1. **t129's suspect list is corrected.** It named "operand size and shape,
   path conversion, allocation, and parallel scheduling" as the likely gap.
   Measured: conversion + allocation + rebuild + WIT marshal are **0.1–1.8 %**
   of primitive time, so they are not a lever at any stage. Operand size
   **inside the sweep** is the gap; parallel scheduling matters only for the
   planner (§4).
2. **t113/t129's S6 ceiling is 2.3x overstated.** The S6 ceiling is not the
   occupancy term (32.7 s) but the duplicate share of it (**14.2 s**), because
   a memo cannot remove the first call per payload; the sibling collision
   family has **zero** duplicates.
3. **t126's stage walls reproduce.** This take's base-off ShellClassification
   is 14,250 ms against t126's 14,698 (−3.0 %), OverhangAnnotation 10,935 vs
   9,689 (+12.9 %), MeshAnalysis 4,544 vs 4,260 (+6.7 %), Slice 2,229 vs 2,110
   (+5.6 %), SeamPlanning 1,856 vs 1,718 (+8.0 %), and the whole prepass 34,349
   vs 33,003 (+4.1 %). Shares are taken within this take; destination
   arithmetic still uses t129's census.
4. **New ledger facts.** PnP's shell stage runs **19.5 clipper calls per layer**
   at **4,608 points per call**; the planner runs **290 calls per layer** at
   **2,040 points per call**; `SupportGeometry` on base-on is **143,766**
   primitive calls and **293.2 M** input points. The planner's effective
   parallelism is **2.19x** (base-on) and **1.68x** (benchy-on) against
   8.34–11.80x on the other heavy stages; `MeshAnalysis` is serial (1.03x).

## 9. Benchy controls (full rows)

benchy classic-off: MeshAnalysis 151 / 343 / 96.4 / 1.7 / 98.22 %;
OverhangAnnotation 677 / 3,107 / 6,704.5 / 32.9 / 99.51 %;
SeamPlanning 159 / 240 / 784.8 / 2.9 / 99.63 %;
ShellClassification 1,627 / 4,391 / 8,512.6 / 26.3 / 99.69 %;
Slice 227 / 1,637 / 1,530.6 / 10.1 / 99.34 %; PerLayer 3,101.2 sweep / 53.3
overhead / 98.31 %.

benchy classic-on: MeshAnalysis 151 / 343 / 96.9 / 1.8 / 98.22 %;
OverhangAnnotation 637 / 3,107 / 7,053.6 / 35.3 / 99.50 %;
SeamPlanning 177 / 240 / 849.2 / 3.4 / 99.60 %;
ShellClassification 1,799 / 4,391 / 9,434.6 / 30.3 / 99.68 %;
Slice 244 / 1,637 / 1,668.4 / 11.4 / 99.32 %;
SupportAnalysis 1,029 / 1,574 / 6,044.7 / 25.7 / 99.58 %;
SupportGeometry 3,477 / 32,097 / 5,785.6 / 66.9 / 98.86 %;
PerLayer 7,532.7 / 113.9 / 98.51 %.

(columns: wall ms / calls / sweep ms / overhead ms / sweep share.)

## 10. Disclosures

- **One probe pass per cell, single pass, ordinary arm.** The probe adds two
  clock reads per primitive; stage walls run 3–13 % above t126's on base-off
  and are used only for within-take shares. No wall claim is made from these
  runs; the G-code hash of every run equals ticket 123's candidate-ordinary
  reference, which is the probe build's output-neutrality proof.
- **Thread-time is accumulated across rayon workers,** not wall. The
  thread/wall column is an effective-parallelism measure, not a utilisation
  trace; a stage can be imbalanced within it.
- **Allocation is not isolated.** `conv_in` allocates the input `Path64`
  vectors, `rebuild` allocates the output `ExPolygon`/`Polygon` vectors, and
  the engine's internal allocation sits inside `sweep`. No allocator shim was
  built; the three measured buckets bound the removable wrapper from above at
  1.8 %.
- **Canonical has no per-op measurement** (t110's channels all fail), so the
  op-count column is source-derived at the `v2.5.0` pin (`40ecc66f78`) and the
  comparison is PnP-measured versus canonical-derived, never a measured
  per-stage ratio.
- **The S5 window is a probe construct,** not a proposed patch: it clips the
  operand to the candidate bbox expanded by `6 × qualify_spacing` and compares
  inside the unexpanded bbox. The non-equivalence it measures is a property of
  the operands, and a real candidate must re-run the qualification and the
  16-cell output contract.
- **The S6 census keys on payload content** (FNV-1a over every point and ring
  boundary plus `(delta, join, arc, miter)`), not on `(radius, layer)` as
  canonical does. Content keys can only under-count hits versus a semantic
  key, so 14.2 s is a lower bound for a canonical-style cache on this job.
- **Probe reverted.** The tree at close is `6c6c188b` plus this evidence set
  and the ticket/map edits; `cargo xtask build-guests --check` exits 0, and
  `target/release/pnp_cli.exe` is rebuilt from the reverted tree
  (`f3d50217…`) and reproduces `90638bf3…` on benchy classic-off.

## 11. Evidence map

- [`probe.patch`](probe.patch) — the temporary probe (new
  `slicer-core/src/t131_probe.rs`, seams in `polygon_ops.rs`, `host.rs`,
  `prepass.rs`, `pipeline.rs`, `slice_postprocess_prepass.rs`).
- [`run-probe.py`](run-probe.py) — the four-cell campaign (`probe` and
  `probe+s5`), with the G-code reference assertion.
- [`reduce.py`](reduce.py) — raw stderr → [`summary.json`](summary.json).
- [`arithmetic.py`](arithmetic.py) — every table above, re-derived.
- [`verify.py`](verify.py) — asserts this file's prose against `summary.json`
  and rejects negative controls.
- [`REPRODUCE.md`](REPRODUCE.md) — rebuild and re-run.
- Raw probe streams (≈50 MB, gitignored) live under `target/t131-runs/`; the
  reduced set is sufficient to re-derive every number here.
