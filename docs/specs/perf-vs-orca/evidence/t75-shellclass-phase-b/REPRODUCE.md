# Reproduce t75

All commands run from the workspace root. The model fixtures `tmp/base.stl`
and `tmp/3dbenchy.stl` are user-supplied and gitignored; the raw capture trees
under `target/t75/` and `.local-artifacts/` are gitignored. The committed
reduced set (`split.json`, `qualification.json`, `ab-reduction.json`) is what
`verify.py` and the findings use.

`probe.patch` and `candidate.patch` are both diffs against the pre-take HEAD
`fb83702c`; the candidate is committed as `07b97cad`. Reproduce the attribution
in a detached worktree at `fb83702c` (link or copy the two `tmp/` fixtures into
it), or check out that commit in a scratch clone.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
cargo build --release --bin pnp_cli
```

## Attribution (baseline HEAD)

```bash
git apply docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/probe.patch
cargo build --release --bin pnp_cli
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/run-probe.ps1 -BatchId t75-probe
python docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/reduce.py --raw-root target/t75
python docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/verify.py
```

The probe is inert without `PNP_T75_PROBE`. Revert it before measuring the
candidate.

## Candidate

```bash
git apply docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/candidate.patch
cargo build --release --bin pnp_cli
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/run-probe.ps1 -BatchId t75-candidate
```

The four candidate stage walls in `split.json` come from the same runner
without the probe (the runner sets `PNP_T75_PROBE`, which is inert in the
production candidate).

## Qualification (16 cells, both modes)

Snapshots are built into the gitignored `.local-artifacts` root:

```bash
# with the candidate applied
cargo xtask dist --edition developer
cargo xtask dist --edition developer --accelerated
# copy target/dist/developer and target/dist-accelerated/developer to
# .local-artifacts/perimeter-reference-preparation/t75-shellclassification-phase-b/snapshots/{ordinary,accelerated}-candidate

# with the candidate reverted (baseline)
cargo xtask dist --edition developer
cargo xtask dist --edition developer --accelerated
# copy to .../snapshots/{ordinary,accelerated}-baseline

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/build-qualification.py
```

`run-cell.ps1` is one-shot per cell; delete a cell directory only when
deliberately restarting that cell, and disclose the restart.

## Standing paired gate

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t75-shellclass-phase-b/reduce_ab.py
```

The gate is pre-declared on the four classic supports-on cells, 6 alternating
pairs per cell plus two warmups per arm, `RAYON_NUM_THREADS=12`, external
`GetProcessTimes`, frozen snapshots, and the preserved t58 output references.
