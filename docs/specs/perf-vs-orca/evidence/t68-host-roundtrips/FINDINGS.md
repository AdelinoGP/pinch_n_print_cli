# t68: classic-perimeters host-service round trips — the bucket is 97–99 % clipper work (attribution; no candidate)

**Resolution in one line:** the per-layer 8 `offset_polygons` + 3 `clip_polygons`
round trips of `classic-perimeters` decompose — per call site, guest-observed
plus host-side conversion/ops split — into **97.55 % (benchy-off) / 99.02 %
(base-off) native clipper ops** versus **2.45 % / 0.98 % total round-trip
overhead** (guest marshalling + component-ABI copies + host unit conversions);
every reduction direction t60 named for this lead (site-level, batching,
in-guest migration) has a measured ceiling below any gate this map has ever
applied (serial upper bound **0.80 % / 0.33 %** of run wall; 12-thread
saturated bound **0.07 % / 0.03 %**). **No candidate; no production edit; no
timing campaign.** Both captures are byte-identical to the preserved t58
references; the probe is reverted.

Route: [Classic-perimeters per-layer host-service round trips](../../issues/68-classic-perimeters-host-round-trips.md).

## Method and evidence

- The **T68-PROBE** is two-sided (preserved as `probe.patch`, applied,
  measured, reverted this take):
  - **host side** — a thread-local `DispatchGuard`
    (`crates/slicer-wasm-host/src/probe.rs`) set around the
    classic-perimeters `run_stage` call
    (`crates/slicer-runtime/src/layer_executor.rs`); the `offset_polygons` /
    `clip_polygons` host services (`crates/slicer-wasm-host/src/host.rs`)
    record per call: service, sample arg (delta / clip op), input/output
    polygon and vertex counts, and a three-way wall split —
    `wit_to_ir_expolygons`, the `polygon_ops` call, `ir_to_wit_expolygons`.
    Buckets are keyed `(owner, seq-within-dispatch)`; the guest's per-layer
    call order is fixed, so seq maps 1:1 to the 11 call sites.
  - **guest side** — per-site `now_us` brackets around the 11 call sites in
    `modules/core-modules/classic-perimeters/src/lib.rs` (`emit_walls`),
    summed per invocation and emitted as one `module_log` line per layer.
    `now_us` is dispatch-relative and monotonic within a dispatch, so each
    bracket measures the **full guest-observed round trip** (guest-side
    marshalling + ABI + host service + return).
- Two ordinary captures (`run-t68.ps1`; freshness exit 0 + external-module
  provenance preflight, 12 threads): **benchy classic supports-off**
  (17.575 s instrumented) and **base classic supports-off** (263.530 s
  instrumented, the largest owner cell). Supports-on cells differ by ~4 % in
  t60's owner wall (85.83 vs 89.26 s accumulated, one cell apart from the
  classic-perimeters share of layer content); accelerated mode is invariant at
  the host boundary — neither re-run (t60 disclosure precedent).
- **Every output is byte-identical to its preserved t58 qualification
  reference** (benchy-off `34886ccb…`, base-off `20e7204c…`), status ok,
  degraded=false, zero fatal/non-fatal errors. The probe changes no geometry.
- Reduced set: `captures/*.json` + `summary.json` re-derivable by `reduce.py`
  from the raw tree (`target/t68-host-roundtrips/`, gitignored);
  `verify.py` exits 0 against the in-repo set alone and rejects seven
  mutations (output sha, dropped site, guest-below-host, inflated overhead,
  thin-wall count, dump count, bucket-below-attributed).
- **Units:** every figure below is accumulated thread-time (ADR-0055 /
  ticket 30), never run wall. Instrumented runs never mixed into wall claims;
  no uninstrumented timing ran.

## The per-site decomposition (guest-observed accumulated, both cells)

Call counts: exactly 8 offsets + 3 clips per layer (5,442 calls base;
2,640 benchy) — **site 11 (thin-wall protrusion clip) executes 0 times**: the
static census inference is now dynamically confirmed (its gate resolves false
on the matched job; see the config-resolution note below). Base layer 270
skips the gap-collection trio (one layer whose inner inset collapses).

| site | calls | guest ms | share | ops ms | conv-in ms | conv-out ms | guest−host ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| A0 outer-loop inset | 495 | 18,815.7 | 21.4 % | 18,743.6 | 4.46 | 1.87 | 65.8 |
| A1 inner-loop inset | 495 | 19,567.3 | 22.3 % | 19,517.7 | 1.68 | 1.94 | 45.9 |
| B gap `shrunk_prev` | 495 | 10,605.8 | 12.1 % | 10,547.5 | 3.45 | 2.61 | 52.2 |
| C gap `grown_cur` | 495 | 4,789.6 | 5.4 % | 4,743.7 | 3.86 | 2.25 | 39.8 |
| D gap difference | 495 | 2,160.0 | 2.5 % | 1,891.4 | 4.73 | 33.47 | 230.3 |
| E `infill_area` | 495 | 12,112.7 | 13.8 % | 12,069.3 | 2.17 | 1.23 | 39.9 |
| F `shrunk_inner` | 495 | 6,702.6 | 7.6 % | 6,659.0 | 1.90 | 1.31 | 40.5 |
| G `grown_infill` | 495 | 3,131.8 | 3.6 % | 3,096.0 | 1.91 | 2.49 | 31.4 |
| H final-transition difference | 494 | 1,563.0 | 1.8 % | 1,380.5 | 3.49 | 21.31 | 157.7 |
| J gap width-band filter | 494 | 40.1 | 0.0 % | 27.8 | 0.73 | 0.30 | 11.3 |
| K infill-boundary inset | 494 | 8,411.7 | 9.6 % | 8,363.9 | 2.54 | 1.86 | 43.4 |
| **total (base-off)** | **5,442** | **87,900.3** | 100 % | **87,142.1** | **31.6** | **68.9** | **758.2** |

Benchy-off (5,726.4 ms guest; ops 5,598.9 = 97.55 %): A0 19.6 %, A1 21.3 %,
B 13.3 %, C 5.5 %, D 4.0 %, E 11.9 %, F 7.7 %, G 4.1 %, H 2.7 %, J 0.2 %,
K 9.6 %; total remainder 127.5 ms.

Read plainly:

- **The wall loops and infill boundary (A0, A1, K) are 53.2 % base / 50.5 %
  benchy** and are the output itself (walls, infill region) — irreducible.
- **The gap machinery (B, C, D, E, F, G, H, J) is 46.8 % base / 49.5 %
  benchy.** Its output is real: D returns 4,632 verts/call and H 2,776/call
  (base) of raw gap candidates, which the in-guest width-band morph
  (`opening_ex` / `offset2_ex`, both already in-guest clipper) collapses to
  **105 verts/call before J**, and the filter then emits gap fill on 162/495
  base layers (245 sections) and 56/240 benchy layers (110 sections). Every
  one of the eight sites feeds
  exactly that pipeline (D = f(B,C), H = f(F,G), G = f(E)); no site is
  dead, duplicated, or feeding a discarded result — the pipeline is minimal
  for its algebra.
- **Conversions are negligible** (100.5 ms base = 0.11 %; 13.4 ms benchy =
  0.23 %), and the **guest remainder** (guest marshalling + ABI copies both
  directions) is 0.86 % base / 2.23 % benchy of the bucket.

## Ceilings for every reduction direction t60 named

| direction | what it can remove | measured ceiling (share of bucket) | verdict |
| --- | --- | ---: | --- |
| in-guest migration | remainder + conversions (0.98 % / 2.45 %) | ≤ 0.98 % base / 2.45 % benchy, **minus** the wasm-ops penalty on the 97–99 % that moves from native to wasm | negative or sub-noise |
| batching (fewer crossings) | per-crossing fixed costs only; per-item marshalling/conversion payload unchanged | bounded by the same remainder; base pairs meet the 4,096-primitive fan-out threshold ({A1,B} 8,064 verts, {C,E,F,K} 11,483) but a 12-thread-saturated phase makes fan-out zero-sum; benchy sits at or below it ({A1,B} 3,104; {C,E,F,K} 4,272) | sub-noise |
| site-level reduction | any representation-identical algebraic shortcut | none found: the gap algebra (miter-offset compositions feeding band-morph differences) admits no exact shortcut, and byte-exactness (the qualification contract) forbids representation changes | none |

Wall relevance (t64-style, from the same captures): the bucket occupies
**6.3 % base / 6.6 % benchy** of its per-layer phase's thread-time
(accumulated ÷ 12 ÷ phase wall); the last layer carries only 2.3 ms base /
4.2 ms benchy of it. Even the *entire* removable overhead is ≤ **0.33 % base
/ 0.80 % benchy** of run wall if perfectly serialized, ≤ **0.03 % / 0.07 %**
at full saturation. The smallest wall effect this map has ever kept came with
~1 % wall + CPU corroboration; nothing here can reach that.

## Stability and cross-checks

- Two independent base runs this take (a smoke capture and the driven
  capture): **87.90 s vs 78.46 s** guest-observed — ±11 % run-to-run variance
  in accumulated thread-time (the §3.6 base.stl spread class), while the
  **decomposition shares are stable within ~1 point** (ops 99.02 % vs 99.2 %;
  gap-machinery share 46.8 % vs 46.6 %). Ratios are the signal; absolutes
  drift — the t60 comparison below is disclosure, not a measurement claim.
- Versus t60's committed owner wall (5,907.6 ms benchy / 89,261.9 ms base):
  this take's captures measure 5,726.4 ms (−3.1 %) and 87,900.3 ms (−1.5 %);
  same call counts (2,640 / 5,442) and identical per-site call structure.
  `verify.py` enforces a ±25 % band on this cross-check.
- The rendered `host:native` bucket in each capture contains the attributed
  share (benchy 11.098 s bucket / 5.60 s attributed ops; base 193.37 s /
  87.14 s) — the classic-perimeters share of a bucket whose other owners
  (region derivations, `region_partition`, seam-planner, linker) this take
  does not touch.

## Disclosures

- **Probe overhead:** each site bracket adds two `now_us` host calls (a
  scalar crossing, no marshalling), so the guest-observed figure is inflated
  by roughly one extra crossing per site (~µs each); one `module_log` line
  per layer. The probe changes no geometry — both outputs byte-identical.
- **Host seq-bucket shift:** on base layer 270 (the single skip layer) the
  recorder's seq counter shifts by three for that layer's later calls, so up
  to six host buckets carry one foreign call (±1 call, ≤ 0.3 % of a bucket).
  The guest census is authoritative for per-site counts; `verify.py` bounds
  host buckets to `{layers−1, layers, layers+1}` and checks the exact totals.
- **Config-resolution observation (recorded, not chased):** the thin-wall
  clip resolves **false** on the matched classic job although the classic
  manifest declares `detect_thin_wall` default `true` (arachne declares
  `false`; the registry reconciles a single default per key across module
  declarations). Whether that cross-module default bleed is intended is a
  separate correctness question — for this take it is a measured input fact.
- **Supports-on cells and accelerated mode not re-run** (host-boundary
  invariance; t60's owner split differs by ~4 % between support states).
- The two captures ran on a developer machine, single runs each, under a
  `--profile --profile-verbose --instrument-stderr` configuration that
  inflates wall (PERF-HANDOFF §3.2 class). No wall or speed claim is made.
- The tree state during capture was HEAD + T68-PROBE; the probe-built guest
  hash and exe hash are recorded in `inputs.json`. The probe was reverted
  after capture and production artifacts rebuilt from HEAD.

## Evidence map

- `probe.patch` — the full T68-PROBE diff (5 host files + the guest module;
  applied for the captures, reverted before the resolution commit).
- `run-t68.ps1` — the capture driver (freshness + provenance preflight, two
  jobs, reference-hash and clean-status gates, wall/CPU recorded).
- `captures/*.json` — per-cell reduction (metadata, phases, host site rows,
  guest site census, last-layer row, rendered bucket block).
- `summary.json` — both cells, keyed by run id.
- `reduce.py` — raw-tree reducer and in-repo summary re-derivation.
- `verify.py` — checkout-only verifier + seven rejected negative controls.

## For the map

The take closes this lead as measured: the round-trip bucket is real clipper
work, not overhead. One fact worth keeping in the fog: the gap-candidate
pipeline spends **46.8 % base / 49.5 % benchy** of this bucket producing gap
candidates of which the width-band morph discards ~98.6 % of the raw vertex
volume on every layer — re-enter only if the parity program ever re-derives
the gap-fill algebra in a representation-identical form.
