# Reproduce t93 — windowed deep-infill erosion

All commands run from the workspace root. The model fixtures `tmp/base.stl` and
`tmp/3dbenchy.stl` are user-supplied and gitignored. The candidate is the
working tree's change to one host file:

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  (`eroded_area_in_qualified_neighbourhood` and its unit test)

It is host-only, so both arms ship byte-identical 24-module trees; only
`pnp_cli.exe` differs.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green **before** the snapshots are built and again after the
take closes. The candidate file is not in any guest's dependency closure, so
the probe and the candidate never invalidate a guest artifact.

## Phase 1 — screening probe (reverted before close)

```bash
git apply docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/probe.patch
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/build-probe.ps1
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/run-probe.ps1
# preserve run 1 (the extractor expects run1-* and the run-2 file names)
cd .local-artifacts/perimeter-reference-preparation/t93-deep-infill-offsets/probe/captures
for f in ordinary-base-classic-off-probe.*; do cp -f "$f" "run1-${f#ordinary-base-classic-off-probe.}"; done
cd -
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/run-probe.ps1
python docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/extract_probe.py
# revert the probe before building the Phase-2 arms
git checkout -- crates/slicer-runtime/src/slice_postprocess_prepass.rs
```

`run-probe.ps1` is one instrumented slice of ordinary base classic supports-off
with `PNP_T93_PROBE=1 --instrument-stderr --profile --profile-verbose`,
`RAYON_NUM_THREADS=12`, and an external `GetProcessTimes` wall + process CPU.
It aborts on any output drift from the post-t81 reference
(`292df501…`, 21,023,739 bytes).

## Phase 2 — snapshots

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/build-snapshots.ps1 -Arm candidate -Modes both
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/build-snapshots.ps1 -Arm baseline  -Modes both
```

The baseline arm is the same tree with
`crates/slicer-runtime/src/slice_postprocess_prepass.rs` `git stash`ed
(path-limited; never a checkout). The script pops the stash in a `finally`
block and writes `provenance/<mode>-<arm>.json`.

## Phase 2 — exactness

```bash
cargo test -p slicer-runtime --lib windowed_erosion
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/run-all-proof.ps1
python docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/extract.py
python docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/reduce.py --raw
```

`run-all-proof.ps1` is resumable (one-shot per cell dir) and covers the 16-cell
matrix (2 modes × 2 fixtures × 2 generators × supports off/on), baseline and
candidate per cell. Ordinary cells are checked against
`t91-exact-z-memo/references.json` (the post-t81 set); accelerated cells are
gated on arm-to-arm identity and their agreement with the ordinary cell is
reported. The unit test pins the observable contract on a fixture whose
qualified set contains a polygon disjoint from the sparse area and whose window
covers a small part of it.

## Phase 2 — paired A/B

Requires `qualification.json` from the step above; every measure cell is guarded
on it. The pre-declared timing set is the four base classic cells,
`{ordinary, accelerated} × {supports off, on}`.

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/run-all-measure.ps1
python docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/extract.py
python docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/reduce.py --raw
```

The machine must be otherwise idle: the harness pins `RAYON_NUM_THREADS=12`,
and a starved sample (cpu/wall below 0.5× its arm's median) is excluded as a
whole pair, never resampled. The CPU clause is the map's work-preserving
ceiling (**≤ baseline + 3.0 %**).

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t93-deep-infill-offsets/verify.py --self-test
```

`verify.py` exits 0 against the in-repo set alone (no `.local-artifacts`):
it re-derives the Phase-1 ceilings from `probe-rows.json`, re-derives
`qualification.json` / `ab-reduction.json` / `summary.json` from the committed
reduced rows, checks `provenance.json` (24 modules per snapshot, byte-identical
module trees per mode, distinct binaries per arm), and rejects every negative
control.

## Revert the probe-built binary (optional)

`target/release/pnp_cli.exe` is probe-built until the next
`cargo build -p pnp-cli --release`. Nothing under `target/dist/` or
`target/dist-accelerated/` was written by the probe; the Phase-2 snapshot
builds rewrite both standing trees from their own arms.
