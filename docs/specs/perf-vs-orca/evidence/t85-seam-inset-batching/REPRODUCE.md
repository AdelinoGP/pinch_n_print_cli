# Reproduce t85 — seam planner inset-offset batching

All commands run from the workspace root. The model fixtures `tmp/base.stl` and
`tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is the
working-tree change to
`modules/core-modules/seam-planner-default/src/lib.rs`; the committed reduced
observations are `proof-rows.json`, `measure-rows.json`, `attribution.json`
and the `*.json` reductions `verify.py` checks checkout-only.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green **before** the snapshots are built. The accelerated
gate also verifies the controlled-perimeter toolchain identity
(`docs/23_controlled_perimeter_builds.md`).

## Snapshots (two arms, two modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/build-snapshots.ps1 -Arm candidate -Modes both
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/build-snapshots.ps1 -Arm baseline  -Modes both
```

The baseline arm is the same tree with the planner source `git stash`ed
(path-limited; never a checkout). The script pops the stash in a `finally`
block. `--edition developer` stages `pnp_cli.exe` plus 24 external modules per
mode under
`.local-artifacts/perimeter-reference-preparation/t85-seam-inset-batching/snapshots/<mode>-<arm>/`.

The guest is the only *executing* difference between arms. The `pnp_cli.exe`
binaries are **not** byte-identical: `pnp-cli` depends on
`slicer-integrated-modules`, which links `seam-planner-default` into the host
binary even for the developer edition. Both arms run with
`--no-integrated-modules`, so that linked copy is inert and dispatch is
external in both.

## Exactness before timing

Native parity test (batched path vs the pre-batching serial loop, per entry):

```bash
cargo test -p seam-planner-default
```

`batched_inset_offsets_match_the_serial_reference_per_region`
(`modules/core-modules/seam-planner-default/src/lib.rs`) drives a multi-region
fixture with two different `scoring_width` values and a paint-blocked fallback
region, and asserts entry-for-entry equality plus fixture-derived projected
positions. The end-to-end gate is byte identity against the derived post-t81
reference set.

## Proof matrix (16 cells x 2 arms)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/extract.py
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/reduce.py --raw
```

`run-all-proof.ps1` is resumable (one-shot per cell dir). Ordinary cells are
checked against `references.json`, which `derive_references.py` re-derives from
the committed t81 campaign rows; accelerated cells are gated on arm-to-arm
identity and their agreement with the ordinary cell is reported.

## Paired A/B (4 cells, 6 alternating pairs each)

Requires `qualification.json` from the step above; every measure cell is
guarded on it.

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/extract.py
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/reduce.py --raw
```

The machine must be otherwise idle: the harness pins `RAYON_NUM_THREADS=12`,
and a starved sample (cpu/wall below 0.5x its arm's median) is excluded as a
whole pair, never resampled.

## Attribution (stage walls, never a wall claim)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/run-instrumented.ps1 `
  -Arm baseline  -Mode ordinary -Fixture base -Generator classic -Supports off -Tag attribution
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/run-instrumented.ps1 `
  -Arm candidate -Mode ordinary -Fixture base -Generator classic -Supports off -Tag attribution
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/extract.py
```

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/derive_references.py
python docs/specs/perf-vs-orca/evidence/t85-seam-inset-batching/verify.py --self-test
```

`verify.py` exits 0 against the in-repo set alone (no `.local-artifacts`),
re-derives the reference table from the t81 campaign rows, and rejects its
negative controls.
