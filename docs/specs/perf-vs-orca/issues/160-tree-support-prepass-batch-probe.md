# Tree-support render placement probe: whole-object batch vs per-layer dispatch

Type: task / measured probe
Label: wayfinder:task
Status: resolved (2026-11-17; measured probe — the fold is a pure placement move
  in 8/8 cells, output byte-identical 8/8, and the batch arm is **slower in
  every supports-on cell** (−1.69 % / −1.71 % / −3.13 % / −3.77 %), so the
  per-layer structure is **exonerated**; the 9.16–12.61 % null-renderer
  ceiling says the renderer's cost is the work, not its placement)
Assignee: 2026-11-17 OpenCode wayfinder session — the human said "Continue the perf vs orca map" (no ticket named); 160 is the only unblocked frontier ticket.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none ([159](159-tree-support-render-prepass-fold.md) resolved 2026-11-17)
Graduated from: [Tree-support render: fold the per-layer `Layer::Support` render into the prepass?](159-tree-support-render-prepass-fold.md)
  — the human ruled the fold an **experiment, not adoption**: measure whether
  the planner/renderer structure (renderer dispatched per layer inside the
  per-layer pipeline) costs wall against a whole-object pass, with the work held
  constant.

## Question

Does dispatching the **unchanged** `com.core.tree-support` renderer
(`TreeSupport::run_support`, `modules/core-modules/tree-support/src/lib.rs`)
for all layers in one parallel whole-object pass after
`PrePass::SupportGeometry` — instead of per layer in `Layer::Support` — lower
wall on any supports-on cell by at least the 0.80 % floor and outside the null
band, with byte-identical G-code? And how does PnP's render cost compare with
canonical `generate_toolpaths` (`TreeSupport.cpp`)?

## Scope

1. **Batch arm (`PNP_T160_SUPPORT_BATCH=1`, probe-only).** After
   `PrePass::SupportGeometry` commits the support plan, the host dispatches the
   resolved `Layer::Support` module(s) for every layer in one parallel pass using
   the existing dispatch machinery, caches each layer's support output and
   anchored collection, and `Layer::Support` replays the cache instead of
   dispatching. No WIT change, no guest change. Disclose the batch pass's
   parallelism and where its wall lands (prepass vs per-layer).
2. **Null-renderer ceiling arm (`PNP_T160_SUPPORT_NULL=1`).** `Layer::Support`
   skips the render. Output is **not** expected to match; this arm only bounds
   what the renderer's per-layer placement can cost at most.
3. **Input-identity check first.** Before timing, prove the batch arm's
   per-layer inputs equal the per-layer arm's (hash the `SliceRegionView` fields
   and support-plan entries `run_support` reads, prepass vs at dispatch). If they
   differ (e.g. post-perimeter fill-area syncs reach the support view), stop and
   record the dependency — the fold is then not a pure placement move.
4. **Output identity.** Batch arm byte-identical to t151's active set
   ([`evidence/t151-skirt-hull-contour/reference.json`](../evidence/t151-skirt-hull-contour/reference.json))
   in all eight cells; every run exit `0`, `degraded=false`, zero diagnostics.
   Supports-off cells must be inert (no support dispatch).
5. **Paired campaign.** Ordinary arm. 5 interleaved baseline/batch pairs per
   supports-on cell, plus null-arm runs; **same-arm null control on all four
   supports-on cells including benchy** (closes t158's missing benchy null
   band). Record box load samples.
6. **Per-op control.** If the Orca rig captures `TreeSupport::generate`'s
   profiler report (`STAGE_GENERATE_TOOLPATHS`, `BOOST_LOG` info), record
   canonical toolpath time per supports-on cell beside PnP's `tree-support`
   thread and wall time; otherwise disclose the gap.
7. **Verdict, per supports-on cell:** **hurts** where the batch arm is faster
   by ≥ 0.80 % of wall *and* outside the measured null band; otherwise
   **exonerated**. Report the null arm's ceiling beside it.
8. **Disposition:** probe only. Revert, preserve `probe.patch`. Adoption, any
   contract change, and any change to the Support family / Support layer
   renderer terms in `CONTEXT.md` go to a separate human ruling.

## Acceptance

- Input-identity result recorded (equal, or the dependency named).
- 8/8 batch-arm hashes equal t151's active set, or the moved cells disclosed
  with the input-identity finding that explains them.
- Per-cell table: baseline / batch / null medians (wall + CPU), null band, pair
  wins, floor multiple, verdict.
- Canonical `generate_toolpaths` timing or a disclosed gap.
- `evidence/t160-tree-support-batch/` with `FINDINGS.md`, `REPRODUCE.md`,
  reduced JSON, `probe.patch`, and `verify.py` (`--self-test` exit 0).
- Restored tree: `cargo xtask build-guests --check` exit 0 and a restored-binary
  run reproducing the active set.

## Evidence starts

- [`evidence/t154-benchy-per-layer/`](../evidence/t154-benchy-per-layer/FINDINGS.md)
  — `tree-support` 47.8 % / 44.0 % / 36.7 % of per-layer thread on benchy
  classic-on / benchy arachne-on / base classic-on; 96.6–96.8 % self fuel;
  per-layer effective parallelism 9.65x–11.30x.
- [`evidence/t155-infill-stage-block/`](../evidence/t155-infill-stage-block/FINDINGS.md)
  — `tree-support`'s 85.9 ms p50 bracket on base-on is in-module render; the
  t127 lesson (decline/move and watch the bracket) applies.
- `crates/slicer-runtime/src/layer_executor.rs` — per-layer dispatch.

## Answer

**Resolved 2026-11-17.** Probe only: the production tree is restored; the exact
diff is [`probe.patch`](../evidence/t160-tree-support-batch/probe.patch), and
`cargo xtask build-guests --check` exits 0.

### 1. Input identity — the fold is a pure placement move

`PNP_T160_INPUT_HASH=1` printed the `Layer::Support` read-set digest twice per
layer, split into `plan` (support-plan rows), `view` (region identity, z,
effective height) and `paint` (slice polygons + segment annotations). **All
three components match on every layer of all eight cells** (240/240 benchy,
496/496 base per side; zero mismatched layers). The post-perimeter
`infill_areas` sync and prior per-layer stages do not reach `run_support`; the
ticket's item-3 stop condition does not fire.

### 2. Output identity

8 cells × {baseline, batch}, all runs exit 0, `degraded=false`, zero
diagnostics. Both arms reproduce t151's active reference set in **8/8**; the
batch pass moves no bytes.

### 3. Paired campaign (5 interleaved pairs per cell, ordinary, 12 threads)

Batch arm separate-arm median wall Δ (negative = batch slower):

| cell | Δ wall | pairwise | Δ CPU | pair wins | batch pass |
|---|---:|---:|---:|---:|---:|
| benchy classic on | **−3.77 %** | −2.84 % | −0.20 % | 1/5 | 493 ms |
| benchy arachne on | **−3.13 %** | −3.19 % | −1.32 % | 0/5 | 488 ms |
| base classic on | **−1.71 %** | −1.41 % | −0.60 % | 2/5 | 4,164 ms |
| base arachne on | **−1.69 %** | −1.49 % | −0.39 % | 1/5 | 3,987 ms |

No supports-on cell clears the 0.80 % floor in the batch arm's favour; every
one is slower. The batch pass's wall (3.9–4.2 s base, 0.49 s benchy) lands
inside the per-layer phase bracket and is charged against the same cell wall.

Supports-off cells are **skipped** (`enable_support=false` returns before the
pass); every candidate run prints the skip line, no batch dispatch occurs, and
output is identical. Their separate-arm medians still move ±1.2–4.3 % with the
same code on both sides — disclosed box drift, not a candidate effect.

### 4. Null band and ceiling

Same-arm null control (baseline both sides, 3 pairs × four supports-on cells):
**9.50 % wall any-pair maximum; 1.91 % median**; CPU maximum 5.89 %. The batch
arm's −1.69 % to −3.77 % sits inside that band, and it is never faster, so the
null control cannot turn any cell into "hurts".

Null-renderer ceiling (5 runs per supports-on cell, output not matched):
**9.46 % / 12.61 % / 11.14 % / 9.16 %** of cell wall. The renderer's work is
9–13 % of a supports-on cell; the fold moves it without removing any of it.

### 5. Verdict

**Exonerated in all four supports-on cells.** The per-layer placement is not
costing wall; the batch implementation is a measured regression (serialized
pass + one extra per-layer `SliceIR` clone + loss of overlap with the other
per-layer stages). The support renderer's cost is the work itself, so the open
route is a per-op/algorithm reduction or a deletion, not a scheduling change.
Adoption, any contract change, and any Support-family glossary/ADR change stay
reserved for a separate human ruling; none is proposed here.

### 6. Disclosures

- One binary serves every arm; arms differ only by the env gate.
- Peak working set is **unmeasured** (the harness's `PeakWorkingSet64` read
  returned 0 in every row — a .NET process quirk on this box).
- No accelerated arm (the accelerated host remains unbuildable in the
  controlled driver on `libmimalloc-sys`, t142).
- No canonical `STAGE_GENERATE_TOOLPATHS` capture: the Orca rig writes
  `orca-cli.log` and can run `--debug 5`, but captures no machine-readable
  `BOOST_LOG` stage report; no canonical per-op number is claimed.
- No fresh Orca scoreboard; this is a placement experiment, not a ratio of
  record.

### 7. Evidence

[`evidence/t160-tree-support-batch/`](../evidence/t160-tree-support-batch/FINDINGS.md)
— `verify.py` PASS and `--self-test` 9/9 negative controls rejected;
`reduced/campaign.json` holds every claim.

Restored-tree check: `cargo xtask build-guests --check` exit 0; the restored
release binary (no probe env) reproduces `eccbd523…` on base classic-on —
exit 0, clean, zero diagnostics.
