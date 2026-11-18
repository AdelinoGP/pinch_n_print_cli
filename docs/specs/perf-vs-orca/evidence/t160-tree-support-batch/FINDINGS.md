# t160 — Tree-support render placement: whole-object batch vs per-layer dispatch

Status: **probe resolved 2026-11-17** — the fold is a pure placement move
(input read set identical on every layer of all eight cells), the batch arm's
output is byte-identical to t151's active set in **8/8**, and the placement
verdict is per cell below.

Ticket: [`issues/160-tree-support-prepass-batch-probe.md`](../../issues/160-tree-support-prepass-batch-probe.md)
Ruling that spawned it: [`issues/159-tree-support-render-prepass-fold.md`](../../issues/159-tree-support-render-prepass-fold.md)
Probe only: production code and tests are reverted; [`probe.patch`](probe.patch)
preserves the exact diff. No contract change, no guest change.

## 1. Question

Does dispatching the **unchanged** `com.core.tree-support` renderer
(`TreeSupport::run_support`, `modules/core-modules/tree-support/src/lib.rs`)
for all layers in one parallel whole-object pass after
`PrePass::SupportGeometry` — instead of per layer in `Layer::Support` — lower
wall on a supports-on cell by at least the 0.80 % floor and outside the
measured null band, with byte-identical G-code?

## 2. Arms and protocol

One release binary serves every arm; the probe is off unless its env var is
`1`, so the baseline is the production path with zero code difference other
than the gate.

| arm | env | what it does |
|---|---|---|
| baseline | — | production per-layer dispatch |
| batch | `PNP_T160_SUPPORT_BATCH=1` | one parallel whole-object pass after the prepass; `Layer::Support` replays the cached commit |
| nullrender | `PNP_T160_SUPPORT_NULL=1` | `Layer::Support` skips the render (ceiling arm; output not matched) |
| hash | `PNP_T160_INPUT_HASH=1` | read-only digest pass at both dispatch points |

Ordinary arm, `RAYON_NUM_THREADS=12`, matched-pair configs, one warmup pair,
5 interleaved pairs per cell for the paired campaign, a same-arm null control
(baseline on both sides) on the four supports-on cells, and 5 null-renderer
runs per supports-on cell. The box is a shared dev machine; load samples are
recorded per batch.

## 3. Input identity — the fold is a pure placement move

`PNP_T160_INPUT_HASH=1` prints the `Layer::Support` read-set digest twice per
layer, split so a mismatch names its class:

- `plan` — the committed `SupportPlanIR` rows whose `anchor_layer_index`
  matches the layer (`support_plan_entries_for` plus the renderer's own filter);
- `view` — `SliceRegionView` identity, z and effective height;
- `paint` — the slice polygons and `segment_annotations` `paint_policy_for`
  walks.

| cell | layers | batch digests | per-layer digests | mismatched layers |
|---|---:|---:|---:|---:|
| benchy classic off | 240 | 240 | 240 | 0 |
| benchy classic on | 240 | 240 | 240 | 0 |
| benchy arachne off | 240 | 240 | 240 | 0 |
| benchy arachne on | 240 | 240 | 240 | 0 |
| base classic off | 496 | 496 | 496 | 0 |
| base classic on | 496 | 496 | 496 | 0 |
| base arachne off | 496 | 496 | 496 | 0 |
| base arachne on | 496 | 496 | 496 | 0 |

**Result: identical in 8/8 cells on every layer** (`plan`, `view` and `paint`
all zero mismatches). The renderer reads only prepass-committed state: the
post-perimeter `infill_areas` sync and any prior per-layer stage do not reach
it. The ticket's item-3 stop condition does not fire; the work can be moved
with its inputs held constant.

## 4. Output identity

8 cells × {baseline, batch}; every run exit `0`, `degraded=false`, zero
diagnostics. Both arms reproduce t151's active reference set in **8/8**.

| cell | g-code SHA-256 | identical | active set | bytes |
|---|---|---:|---:|---:|
| benchy classic off | `3d28d1b3…` | yes | yes | 4,085,138 |
| benchy classic on | `eb202973…` | yes | yes | 8,673,267 |
| benchy arachne off | `39ad6289…` | yes | yes | 4,227,129 |
| benchy arachne on | `77c3c578…` | yes | yes | 8,814,878 |
| base classic off | `9bb18f0e…` | yes | yes | 21,187,100 |
| base classic on | `eccbd523…` | yes | yes | 39,732,305 |
| base arachne off | `2f249cdb…` | yes | yes | 21,046,764 |
| base arachne on | `1811425f…` | yes | yes | 39,594,635 |

## 5. Paired campaign

5 interleaved pairs per cell, ordinary arm, 80 runs. Wall/CPU are separate-arm
medians; the pairwise median is the median of each pair's relative delta.

| cell | baseline wall | batch wall | wall Δ | pairwise | CPU Δ | pair wins | batch pass |
|---|---:|---:|---:|---:|---:|---:|---:|
| benchy classic off | 3.26 s | 3.34 s | **−2.40 %** | −2.40 % | −1.38 % | 1/5 | 39 ms |
| benchy classic on | 6.94 s | 7.20 s | **−3.77 %** | −2.84 % | −0.20 % | 1/5 | 493 ms |
| benchy arachne off | 3.34 s | 3.41 s | **−2.26 %** | −1.13 % | +0.17 % | 2/5 | 38 ms |
| benchy arachne on | 7.06 s | 7.28 s | **−3.13 %** | −3.19 % | −1.32 % | 0/5 | 488 ms |
| base classic off | 21.28 s | 21.44 s | **−0.77 %** | −0.77 % | −1.32 % | 2/5 | 223 ms |
| base classic on | 69.57 s | 70.77 s | **−1.71 %** | −1.41 % | −0.60 % | 2/5 | 4,164 ms |
| base arachne off | 21.67 s | 22.02 s | **−1.59 %** | +0.56 % | −1.03 % | 3/5 | 225 ms |
| base arachne on | 69.85 s | 71.03 s | **−1.69 %** | −1.49 % | −0.39 % | 1/5 | 3,987 ms |

**No cell clears the floor in the batch arm's favour.** The batch arm is slower
in all eight separate-arm medians (−0.77 % to −3.77 %), and the supports-on
verdict cells are −1.69 % to −3.77 % — floor multiples **−2.1x, −2.1x, −3.9x,
−4.7x** (negative = below the 0.80 % floor in the candidate's favour; the four
supports-off cells read −2.7x, +1.5x, −3.1x, −5.4x on a skipped pass). The
batch pass's own wall is 3.9–4.2 s on the two base supports-on cells and
0.49 s on the two benchy supports-on cells; it is a rayon `par_iter` over all
layers on the same global pool (`RAYON_NUM_THREADS=12`), so its parallelism is
the per-layer phase's, without the pipeline's overlap;
it lands **inside the per-layer phase bracket**, before the layer loop, and is
therefore charged against the same cell wall. The per-layer phase then shrinks
by the render it no longer does, but the move loses the overlap the per-layer
pipeline had with perimeter/infill work and pays one extra per-layer `SliceIR`
clone — the net is the measured regression.

The four supports-off cells are measured on the **skipped** shape (the batch
pass returns an empty cache when the resolved `enable_support` is false, so the
production per-layer path runs unchanged): every candidate run prints
`t160 support_batch skipped enable_support=false` and no batch-pass wall is
recorded. Their separate-arm medians still move — benchy classic-off
**−2.14 %**, benchy arachne-off **+1.20 %**, base classic-off **−2.45 %**,
base arachne-off **−4.31 %** — with the same code path on both sides. That is
the box's drift at 5 pairs, not a candidate effect; it is disclosed rather than
gated, and it confirms the width of the null instrument on this machine.

## 6. Same-arm null band

Baseline on both sides, 3 interleaved pairs per supports-on cell.

| cell | wall deltas | wall median | wall max abs | CPU max abs |
|---|---|---:|---:|---:|
| benchy classic on | −7.00, +0.95, +0.98 % | +0.95 % | 7.00 % | 2.04 % |
| benchy arachne on | +0.36, +0.07, +0.21 % | +0.21 % | 0.36 % | 5.89 % |
| base classic on | −9.50, +2.54, −1.91 % | −1.91 % | 9.50 % | 0.49 % |
| base arachne on | +0.98, −1.94, −0.64 % | −0.64 % | 1.94 % | 0.56 % |

**Band: 9.50 % wall any-pair absolute; 1.91 % median.** One pair on each base
and benchy supports-on cell produced a 7–9.5 % swing, reproducing t156's 9.52 %
any-pair band on the same box; the medians are ≤ 1.94 %. The batch arm's
−1.69 % to −3.77 % sits inside the any-pair band, and no cell is faster, so the
null control cannot rescue a "hurts" verdict for the batch arm.

## 7. Null-renderer ceiling

5 null-renderer runs per supports-on cell; output is not matched (the render is
removed), so this arm only bounds what the render's placement could cost at
most. Medians are quoted against the same cell's baseline median from §5.

| cell | null-render wall | baseline wall | render ceiling | all runs clean |
|---|---:|---:|---:|---:|
| benchy classic on | 6.29 s | 6.94 s | **9.46 %** | yes |
| benchy arachne on | 6.17 s | 7.06 s | **12.61 %** | yes |
| base classic on | 61.82 s | 69.57 s | **11.14 %** | yes |
| base arachne on | 63.45 s | 69.85 s | **9.16 %** | yes |

**The renderer costs 9.2–12.6 % of a supports-on cell wall, and the batch arm
does not remove any of it.** The ceiling is what a *deletion* could buy; the
fold only moves the same work, and measures −1.7 % to −3.8 % instead. So the
gap is the render work itself, not per-layer placement overhead — the
per-op/algorithm question, not a scheduling question.

## 8. Verdict

**The per-layer placement is EXONERATED in all four supports-on cells
(2026-11-17).** The verdict rule is "hurts where the batch arm is faster by
≥ 0.80 % of wall and outside the measured null band"; no supports-on cell is
faster at all (−1.69 %, −1.71 %, −3.13 %, −3.77 %), so the structure the human
asked about is not costing wall. The fold is a pure placement move (input
identity 8/8) but a **measured regression** as an implementation: it adds a
serialized whole-object pass and a per-layer slice clone and gives up the
per-layer pipeline's overlap with the other stage work. Beside the 9.2–12.6 %
null-renderer ceiling, the finding is sharper than "placement is fine": **the
support renderer's cost is the work itself, so a scheduling change cannot
recover it; the route is a per-op/algorithm reduction or a deletion.**

Disposition is probe only: no adoption, no contract change, no production
edit. The measured regression is the answer to the ticket's question, not a
candidate to adopt.

## 9. Disclosures

- **One binary, one module tree.** Arms differ only by their env gate; the
  manifest records one exe SHA and one module-tree SHA for all four arms.
- **The batch pass clones each layer's `SliceIR` into a throwaway arena**
  before dispatching. The baseline pays that clone once per layer in the
  ordinary path; the batch arm therefore carries **one extra per-layer clone**
  and holds every layer's `SupportIR` until replay. Both effects are
  conservative against the candidate. **Peak working set is not claimed**: the
  harness's `PeakWorkingSet64` read returned 0 in every row (a .NET process
  quirk on this box), so memory is disclosed as unmeasured rather than
  fabricated.
- **Supports-off cells are structurally inert for placement** (`enable_support:
  false` declines before the render), so they are output-identity and
  no-regression checks only, not verdict cells.
- **No accelerated arm.** The accelerated host remains unbuildable in the
  controlled driver (`libmimalloc-sys`, t142), and this probe is host-only.
- **No fresh Orca scoreboard.** This is a phase-placement experiment, not a
  ratio of record.
- **Canonical per-op control: disclosed gap.** The Orca rig
  (`evidence/matched-pair/run_scoreboard.ps1`) can run the installed slicer
  with `--debug 5` and writes `orca-cli.log`, but it captures no
  machine-readable `BOOST_LOG` `STAGE_GENERATE_TOOLPATHS` report; no canonical
  per-op number is claimed from this take.

## 10. Evidence layout

- [`run_t160.ps1`](run_t160.ps1) — arms, manifest, hash/output/ab/ceiling modes.
- [`reduce_t160.py`](reduce_t160.py) — raw tree → [`reduced/campaign.json`](reduced/campaign.json).
- [`verify.py`](verify.py) — re-derives every claim from the reduced JSON;
  `--self-test` runs 8 negative controls.
- [`REPRODUCE.md`](REPRODUCE.md) — build, gate order, restore.
- [`probe.patch`](probe.patch) — the exact probe diff (host only).

Restored tree: `cargo xtask build-guests --check` exit 0; the restored release
binary (`target/release/pnp_cli.exe`, no probe env) reproduces
`eccbd523…` on base classic-on, exit 0, clean, zero diagnostics.
