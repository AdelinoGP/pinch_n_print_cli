# t57 — scanbeam TopX projection cache: measured DROP

## Outcome

**Recommendation: DROP (measured). Human decision pending; production restored.**

The full fresh 16-cell output matrix passed byte-for-byte in both modes. The
approved paired timing campaign then stopped at its **first** timed cell, as the
frozen protocol requires: on `ordinary-benchy-classic-on` the candidate was
slower than the baseline on **both** metrics in **6 of 6 retained pairs**
(no starvation exclusions), so no cell can reach the both-modes KEEP gate and
the remaining three cells did not run (stop-on-first-non-pass; no pooling, no
resampling, no automatic retry).

| metric | baseline median | candidate median | paired median delta | change |
|---|---:|---:|---:|---:|
| wall seconds | 25.817 | 26.370 | +0.505 | +2.14% |
| process CPU seconds | 139.305 | 143.180 | +4.086 | +2.78% |

Warmups (excluded) agree: candidate CPU 143.41 s vs baseline 139.83 s. The
cache's per-scanbeam key checks, stores and teardown cost more than the
projection recomputation they avoid on this job. Because the gate requires the
candidate to be faster **and** every timed cell to pass, one failing measured
cell is already decisive; running the other three cells could not change the
verdict, and the protocol forbids continuing past a non-passing cell.

## Exactness proven before timing (all passed)

- Native and actual-WASM cache hit/fallback/lifetime contracts, each with 1,056
  independent ordered engine comparisons and 18 production mesh/boolean/offset
  consumer comparisons; sabotaged copies (stale-y reuse, wrong projection) were
  rejected on both targets.
- Exact recorded full-role replay of the eight selected t55 cases (not
  all-layer replay; coverage limits in `exactness.json`).
- Actual per-graph lock/resolved-edge/feature equality apart from the private
  engine identity; dependency-closure freshness sensitivity verified; both
  modes built, diagnosed external-only, and passed their freshness checks.
- Workspace build, all-targets Clippy, literals, test-quality report, synthetic
  reducer regressions and runner syntax passed. `candidate.patch` is the
  reconstructible candidate change; the pristine original `top_x` implementation
  is unchanged.
- Full matrix: 16/16 cells, byte-identical baseline/candidate outputs, correct
  generator claim holders, clean completions, nonvacuous roles, accelerated
  outputs equal to ordinary per job, classic-on anchored to archived t55
  post-repair outputs.

Diagnostic layout measurements (not peak memory): optional-record stride 80
bytes native, 72 bytes wasm32.

## Stop/resume history (disclosed)

1. First qualification attempt was killed by the assistant's own 40-minute
   shell timeout at cell 8/16. It stopped and returned to the human instead of
   retrying.
2. Under the human's "Continue until completion" authorization, the completed
   7 cells were re-proved from raw bytes and only the remaining 9 reran. That
   attempt completed cells 8-15, then the harness evicted the session mid
   cell-16 candidate warmup (`location services evicted` in the OpenCode log,
   shell stream 404; no reboot, crash, or disk pressure).
3. The second-stop partials were archived, the whole cell-16 was rerun under
   `resume-authorization-2.json` (its completed baseline arm was preserved and
   cross-checked to the cell-8 cross-mode baseline hash), and the 16-cell
   `qualification.json` was sealed.

No gate-failing cell was ever rerun. The timing driver only reruns an
externally interrupted in-flight cell under explicit authorization
(`--resume-authorized`); a measured failure always stops.

Reducer honesty: `campaign.py` now distinguishes KEEP / DROP / INCONCLUSIVE, so
a starvation-inconclusive campaign can never be reported as a measured DROP;
`verify()` re-derives every saved summary from raw rows, rejects a sabotaged
output, enforces the stop discipline, and pins the reducer hash used at
reduction time.

## Independent methods review (2026-10-07)

An adversarial reviewer (human-requested) found **no BLOCKER or HIGH findings**
and confirmed the bounded DROP is supported. It rated reuse integrity, cell-16
handling, evidence completeness and restoration **SOUND**; authorization
provenance for the second resume **UNVERIFIED** (relies on the standing
"Continue until completion" authorization); and it found real verifier/resume
weaknesses, since repaired: cell-identity binding in the reducer, seal
cross-checks, atomic checkpoints, complete-cell adoption with process
quiescence, an immutable as-run reducer pin, and a wording error ("0 of 6" →
"6 of 6"). The frozen pre-campaign six-test regression log is preserved as-is;
the eight-test post-campaign run is the supplementary
`logs/reducer-regressions-current.log`. Full disposition:
`review-record.json`.

## Limits

- Only the first of the four approved timing cells was measured (stop rule);
  the remaining cells are recorded `not_run`. No pooled or cross-cell claim.
- Exactness coverage is the selected recorded corpus, not all-layer replay;
  mesh-consumer checks use synthetic nested-hole solids; layout accounting is
  not peak RSS.
- The measured job is one fixture/generator/supports combination; a different
  job could have different cache economics, but the approved protocol does not
  permit pooling or cherry-picking cells.

## Restoration

Production sources, manifests/locks and the registry are unchanged; archived
active artifacts and both distribution file sets are byte-restored; both
production freshness checks exit 0. `finalize.py` re-verifies all of this after
the campaign and re-seals `validation.json` (the stop-1 seal is preserved as
`validation-stop1.json`). Unrelated dirty work was preserved. Nothing was
committed; no permanent dependency or mode adoption.

## Handback

Measured recommendation **DROP**; **accepted by the human on 2026-10-07**
(`drop-acceptance.json`). No commit, permanent dependency, mode adoption or
retry. Heavy raw captures live in the gitignored archive; checkout-verifiable
evidence: `qualification.json`, `campaign.json`, `campaign-progress.json`,
`exactness.json`, `candidate.patch`, `validation.json`, `review-record.json`
and the reproduction notes in `REPRODUCE.md`.
