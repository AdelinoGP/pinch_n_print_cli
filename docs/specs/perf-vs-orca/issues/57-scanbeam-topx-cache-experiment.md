# Scanbeam TopX projection cache: exactness + paired host/guest A/B

Type: task
Label: wayfinder:task
Status: resolved (2026-10-07; **human-accepted DROP** — candidate slower on both metrics in every retained pair; production restored — see ## Answer)
Assignee: agpen (current OpenCode experiment session), 2026-10-07 — human requested execution after scope handoff
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 56

## Question

Does one conservative scanbeam-local unchanged-edge projection cache, integrated
into the shared native and WASM Clipper engine, preserve exact behavior across
the approved surface and pass the paired-mode speed gate on the measured
classic supports-on jobs after all cache overhead?

## Authorization and context

[Scanbeam TopX projection-cache experiment scope](56-scanbeam-topx-cache-experiment-scope.md)
owns the human-approved candidate, dependency/freshness integration, exactness,
qualification, sampling, stop and cleanup contract. Read its resolution before
claiming this execution take; do not reopen its decisions through implementation
shortcuts. Claim before work and return KEEP/DROP to the human.

[Large body-union findings](../evidence/t55-body-union/FINDINGS.md) supply the
selected exact corpus and repeated-projection lead. Its private counter/timer
copy is instrumented, not candidate source or acceptance evidence. Cache impact
is unmeasured; this ticket is not speed approval or permanent fork adoption.

## Work

1. Audit current tree, model/config/toolchain/source identities and prior
   evidence before reuse. Preserve unrelated work and prepare isolated current
   baseline snapshots with both freshness gates and external provenance. Read
   the map's Notes and relevant architecture/coordinate/test-quality/build docs.
2. Prepare the scope's pristine exact-version private source and candidate-only
   explicit dependency seam. Inventory all native/guest consumers and actual
   resolutions. Prove the copied engine participates in guest closure
   fingerprints and deliberate source edits cause staleness before accepting
   candidate freshness. Stop if wiring or coverage cannot satisfy the scope.
3. Implement only the full-key-guarded projection cache. Retain the original
   engine/calculation independently and execute the approved native and WASM
   exactness/fallback/order regressions plus exact full-role replay. Inspect
   feature gates before trusting any run; combined test output must tee to
   `target/test-output.log` and be read before the next invocation. Guest-touching
   broad runs go through `cargo xtask test`; no default full workspace suite.
4. Account for cache state/work separately in diagnostic builds. Prepare clean
   complete ordinary/accelerated host+guest candidate snapshots; pass both
   freshness gates and the entire approved fresh matrix output qualification
   before timing. Preserve proof identities and nonvacuous output checks.
5. Freeze and execute the candidate-specific standing paired protocol on the
   scope's measured jobs only. Preserve raw and reduced rows, starvation
   exclusions and per-cell results without pooling or automatic retry. Charge
   all overhead; never mix diagnostic results into speed acceptance.
6. Run the scope's code gates, preserve reconstructible candidate/tests and
   checkout-verifiable evidence, then restore audited production state and
   verify inventories plus both freshness gates. Report measured outcomes,
   coverage limits and a human KEEP/DROP recommendation. Do not commit, adopt
   a permanent dependency, change modes, repair unrelated parity or retry.

## Comments

### Immediate execution requested — 2026-10-07

The human rejected a scope-only stopping point and requested continuation.
Execute the confirmed experiment now; the scope's safety/acceptance gates and
human KEEP/DROP remain unchanged. This explicitly extends the scope session
into its execution child, rather than treating the prior handoff as completion.

### Qualification interrupted; production restored — 2026-10-07

The isolated shared-engine candidate passed independent native/actual-WASM
exactness, selected full-role replay, negative controls, dependency-resolution
and closure checks, code gates and both candidate freshness checks. Seven
ordinary-mode fresh matrix cells passed exact G-code byte equality. The outer
qualification command reached its aggregate timeout during the eighth cell's
baseline slice (`ordinary-base-arachne-on`); there is no completed full matrix
or paired timing campaign. No cache correctness failure or speed verdict is
established by this interruption.

Production sources/locks and the registry are unchanged; archived active
artifacts and both distribution file sets are restored, and both production
freshness checks exit 0. The shared build cache retained a candidate-root `xtask`
binary; a separately built production-root helper corrected the restoration
check without changing guest freshness semantics. Preserved candidate and
partial evidence: [findings](../evidence/t57-topx-cache/FINDINGS.md).

### Resumed under explicit human authorization — 2026-10-07

The human answered "Continue until completion". Recorded in
[`resume-authorization.json`](../evidence/t57-topx-cache/resume-authorization.json):
the seven completed ordinary cells are re-validated from raw bytes and preserved;
only the interrupted ordinary cell and the eight unrun accelerated cells rerun;
then the frozen paired timing protocol runs, and restoration/handback complete.
The interrupted cell's partial baseline files are archived. The outer aggregate
timeout that stopped the first attempt is removed (the runner's own per-run
timeout stays). No assertion, snapshot or input is changed; still no commit,
permanent dependency, mode adoption or automatic retry beyond this authorization.

### Second stop, completion, and measured DROP — 2026-10-07

That resume re-proved the seven completed cells from raw bytes and reran only
the untouched nine. It completed cells 8–15, then the harness evicted the
session mid cell-16 candidate warmup (OpenCode log: `location services evicted`,
shell stream 404; no reboot, crash or disk pressure). The whole cell-16 was then
rerun under [`resume-authorization-2.json`](../evidence/t57-topx-cache/resume-authorization-2.json)
after archiving its partial files; its completed baseline arm was preserved and
matched the cell-8 cross-mode baseline hash.

The full 16-cell qualification matrix then passed: byte-identical
baseline/candidate outputs in both modes, correct generator claim holders,
clean completions, nonvacuous roles, accelerated equal to ordinary per job and
classic-on anchored to the archived t55 post-repair references.

The frozen paired timing campaign stopped at its first cell exactly as the
protocol requires. On `ordinary-benchy-classic-on` the candidate was slower in
**6 of 6** retained pairs on **both** wall and process CPU:

| metric | baseline median | candidate median | paired median delta | change |
|---|---:|---:|---:|---:|
| wall seconds | 25.817 | 26.370 | +0.505 | +2.14% |
| process CPU seconds | 139.305 | 143.180 | +4.086 | +2.78% |

No pair favored the candidate (wall deltas +0.39…+1.01 s; CPU +2.53…+5.55 s).
Because the gate requires the candidate faster on both metrics in every timed
cell, this cell is decisive and the remaining three cells are recorded
`not_run`; the protocol forbids pooling, resampling or continuing past a
non-passing cell. **Measured recommendation: DROP.** No cache correctness
failure exists — exactness passed throughout — the projection cache's key
checks, stores and teardown simply cost more than the recomputation avoided on
this job.

Production is restored: build inputs/locks/registry unchanged, active artifacts
and both distribution file sets byte-restored, both production freshness checks
exit 0, and `finalize.py` re-sealed the evidence (`validation-stop1.json`
preserves the earlier seal). Nothing was committed and no permanent dependency
or mode adoption occurred. Full disclosure:
[findings](../evidence/t57-topx-cache/FINDINGS.md); verify with
`python docs/specs/perf-vs-orca/evidence/t57-topx-cache/verify.py`.

### Independent methods review — 2026-10-07

The human-requested adversarial review ran after quota recovery. **No BLOCKER or
HIGH findings; the bounded measured DROP remains supported.** Verdicts: reuse
integrity, cell-16 handling, evidence completeness and restoration integrity
**SOUND**; stop/resume authorization provenance **UNVERIFIED** (the second
resume cites the standing "Continue until completion" authorization rather than
a fresh post-stop reply — disclosed, human confirmation invited); measurement
identity-binding, verifier false-pass paths and resume-checkpoint robustness
were **FLAWED** and have been repaired:

- `campaign.summarize` now validates full row cell identity (mode/fixture/
  generator/supports/phase/run-id), with a relabel-rejection regression; the
  verifier exercises it.
- `verify.restored` now cross-checks the seal's outcome, freshness and
  non-vacuous evidence inventory against the campaign and restoration records.
- `manage.write` is atomic; resume adopts any complete unrecorded cell and
  requires no-running-engine quiescence plus explicit `--resume-authorized`
  before archiving a partial cell — never an automatic retry.
- The as-run reducer is preserved as `campaign-as-run.py` (sha256
  e9926960…d333) and pinned for verification; later hardening is supplementary.
- Wording corrected from "0 of 6" to "6 of 6" (candidate slower in every
  retained pair). Full disposition: `review-record.json`.

### DROP accepted — 2026-10-07

The human accepted the measured DROP and confirmed that the standing
authorization covered the externally interrupted-cell rerun after the harness
session eviction (reviewer finding 2). Recorded in
[`drop-acceptance.json`](../evidence/t57-topx-cache/drop-acceptance.json). No
commit, permanent dependency, mode adoption or retry; the restored production
state stands and the candidate patch plus raw archive remain preserved for
reproducibility.
