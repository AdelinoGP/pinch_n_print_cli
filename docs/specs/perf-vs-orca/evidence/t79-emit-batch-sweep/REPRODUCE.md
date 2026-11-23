# Reproduce t79 — emit-batch constant sweep

All commands run from the workspace root. The model fixtures `tmp/base.stl`
and `tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is
[`candidate.patch`](candidate.patch), a one-line `CLIP_BATCH_MAX_REQUESTS`
256 → 4096 diff against HEAD. The committed reduced observations are
`sweep-summary.json`, `confirm.json`, `snapshots.json` and `summary.json`;
`verify.py` checks them checkout-only. Raw G-code and stderr captures are
large and live under the gitignored `target/t79-sweep/` and
`.local-artifacts/perimeter-reference-preparation/t79-emit-batch-sweep/`.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

## Screening

`run-sweep.ps1` sets the two constants in
`modules/core-modules/tree-support-planner/src/lib.rs`, rebuilds the stale
guest, and runs one instrumented slice per variant on
`ordinary-base-classic-on`. The plans are [`plan-pass1.json`](plan-pass1.json)
(chunk sweep), [`plan-pass2.json`](plan-pass2.json) and
[`plan-pass3.json`](plan-pass3.json) (cap sweep and repeats), and
[`plan-pass4.json`](plan-pass4.json) (interleaved 256/4096).

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/run-sweep.ps1 `
  -Plan docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/plan-pass1.json
python docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/reduce-sweep.py
```

The driver leaves the source at the last variant's constants; restore `4 / 256`
and rebuild before doing anything else.

## Snapshots

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/build-snapshots.ps1 `
  -Arm baseline -E 4 -C 256
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/build-snapshots.ps1 `
  -Arm candidate -E 4 -C 4096
python docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/compare-snapshots.py
```

The baseline arm path-stashes the planner source, builds both developer
snapshots, then pops the stash. Each mode must differ in exactly
`modules/tree-support-planner/tree-support-planner.wasm`, with identical
`pnp_cli.exe` binaries.

## Confirmation

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/run-confirm.ps1 `
  -Mode ordinary -Fixture base
python docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/reduce-confirm.py
```

Two warmups plus two alternating uninstrumented pairs, external
`GetProcessTimes`, 12 rayon threads, frozen snapshots. This is a direction
check; the observed result was **+6.30 % paired-median wall** for the
candidate, so the standing campaign was not started.

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t79-emit-batch-sweep/verify.py
```

Exits 0 only if the screening table, the confirmation regression, the snapshot
provenance and the DROP verdict all hold, and all 13 negative controls are
rejected.

## Standing gate (not run)

The 16-cell proof and the 6-pair campaign harnesses are committed for
completeness ([`run-cell.ps1`](run-cell.ps1), [`run-all-proof.ps1`](run-all-proof.ps1),
[`run-all-measure.ps1`](run-all-measure.ps1), [`build-qualification.py`](build-qualification.py),
[`reduce_ab.py`](reduce_ab.py)) but were not run: the pre-gate confirmation
showed the candidate slower on wall in both pairs, so there was no candidate
to carry into the gate. The constants stay at `4 / 256`.
