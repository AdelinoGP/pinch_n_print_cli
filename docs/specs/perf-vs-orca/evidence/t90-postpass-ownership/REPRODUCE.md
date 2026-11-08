# Reproduce t90 — single-ownership collection through finalization and emission

All commands run from the workspace root. The model fixtures `tmp/base.stl` and
`tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is the
working-tree change to five host files:

- `crates/slicer-gcode/src/emit.rs`
- `crates/slicer-runtime/src/layer_finalization.rs`
- `crates/slicer-runtime/src/lib.rs`
- `crates/slicer-runtime/src/pipeline.rs`
- `crates/slicer-runtime/src/postpass.rs`

The committed reduced observations are `proof-rows.json`, `measure-rows.json`,
`attribution.json`, `provenance.json` and the `*.json` reductions `verify.py`
checks checkout-only.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green **before** the snapshots are built. The candidate is
host-only, so both arms ship byte-identical module trees; `provenance.json`
records the per-file module SHAs and both arms' `pnp_cli.exe` SHAs.

## Snapshots (two arms, two modes)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/build-snapshots.ps1 -Arm candidate -Modes both
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/build-snapshots.ps1 -Arm baseline  -Modes both
```

The baseline arm is the same tree with the five candidate host files `git
stash`ed (path-limited; never a checkout). The script pops the stash in a
`finally` block and writes `provenance/<mode>-<arm>.json`.

## Exactness before timing

```bash
cargo test -p slicer-gcode --test gcode_emit_owned_parity_tdd
cargo test -p slicer-runtime --test executor layer_finalization
cargo test -p slicer-runtime --test visual_debug_postpass_tap_tdd
```

`gcode_emit_owned_parity_tdd` pins owned-vs-borrowed IR and serialized-text
equality, the rotation fixture's exact entity order and `;TYPE:` sequence, the
single-tool negative control (no `T<n>`, authored order preserved), the error
path, and the metadata layer count. The finalization executor tests pin the
owning entry point's commit and the abort contract (the working collection is
consumed).

## Proof matrix (16 cells x 2 arms)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/extract.py
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/reduce.py --raw
```

`run-all-proof.ps1` is resumable (one-shot per cell dir). Ordinary cells are
checked against `references.json`, which `derive_references.py` re-derives from
the committed t81 campaign rows; accelerated cells are gated on arm-to-arm
identity and their agreement with the ordinary cell is reported.

## Paired A/B (4 cells, 6 alternating pairs each)

Requires `qualification.json` from the step above; every measure cell is
guarded on it. The pre-declared timing set is the four base supports-on cells,
`{ordinary, accelerated} x {classic, arachne}`.

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/extract.py
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/reduce.py --raw
```

The machine must be otherwise idle: the harness pins `RAYON_NUM_THREADS=12`,
and a starved sample (cpu/wall below 0.5x its arm's median) is excluded as a
whole pair, never resampled.

## Attribution (stage walls, never a wall claim)

One instrumented capture per arm per measured cell:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/run-all-attribution.ps1
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/extract.py
```

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/derive_references.py
python docs/specs/perf-vs-orca/evidence/t90-postpass-ownership/verify.py --self-test
```

`verify.py` exits 0 against the in-repo set alone (no `.local-artifacts`),
re-derives the reference table from the t81 campaign rows, and rejects its
negative controls.

## Pre-existing reds that are not this candidate's

At the take's HEAD `eb059f60`, the `slicer-runtime` contract / executor /
integration suites carry pre-existing failures of the post-ticket-81
`resolution` fixture class (a hand-built config without the now-mandatory
`resolution` key, or a CONFIG_BLOCK expectation predating it):
23 in `--test contract`, 3 in `--test executor`, 50 in `--test integration`.

The contract suite's failure set was **measured** identical with and without
this candidate (the five host files stashed, suite re-run: 23/23 same tests,
291 passed either way). The executor and integration sets were **diagnosed by
error signature**, not re-run stashed: 3/3 and 50/50 of their failures carry the
`resolution is missing or not of type float` message or the `resolution` row in
a CONFIG_BLOCK/oracle expectation, and this candidate touches no config
resolution path. They are disclosed, not counted as regressions.
