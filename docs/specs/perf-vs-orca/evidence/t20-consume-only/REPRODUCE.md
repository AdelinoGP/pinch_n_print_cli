# Reproduce t20 (consume-only-when-read annotations)

All commands run from the workspace root. The model fixtures `tmp/base.stl`
and `tmp/3dbenchy.stl` are user-supplied and gitignored; the snapshot tree under
`.local-artifacts/perimeter-reference-preparation/t20-consume-only/` is
gitignored. The committed reduced set (`qualification.json`,
`ab-reduction.json`) is what `verify.py` reads.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

## Candidate

`candidate.patch` is the diff against the pre-take HEAD `4d62a885`. It touches
seven build inputs (the new `config_predicates` module, `slicer-core`'s
`PathAnnotationMode::Unconsumed`, the two perimeter modules and their
manifests, and the classifier's shared-predicate call). Tests and docs are not
part of the snapshot build.

## Snapshots (both arms, both modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t20-consume-only/build-snapshots.ps1
```

The script stashes the seven candidate files for the baseline arm, builds
`target/dist/developer` and `target/dist-accelerated/developer`, copies them to
`snapshots/{ordinary,accelerated}-baseline`, restores the archived standing
accelerated tree, pops the stash, then repeats for the candidate arm. It parks
the candidate ordinary dist at `target/t20/candidate-dist-ordinary/` and
restores the standing accelerated tree.

## Proof (16 cells, both modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t20-consume-only/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t20-consume-only/build-qualification.py
```

## Standing paired gate

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t20-consume-only/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t20-consume-only/reduce_ab.py
python docs/specs/perf-vs-orca/evidence/t20-consume-only/verify.py
```

Pre-declared: gate cells are the four classic supports-off cells (6 alternating
pairs each, 2 warmups per arm, `RAYON_NUM_THREADS=12`, external
`GetProcessTimes`, frozen snapshots). Wall must fall and paired-median CPU must
not exceed +1.0 % in all four. Supports-off isolates the per-layer perimeter
phase; the full 16-cell proof covers arachne and supports-on output identity.
