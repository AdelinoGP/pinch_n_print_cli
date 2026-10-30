# Reproduce t99 — split-and-union the whole-layer clip-area dilation

All commands run from the workspace root. The model fixture `tmp/base.stl` is
user-supplied and gitignored. The probe is host-only: the standing
`target/dist/developer` module tree plus a probe-built `pnp_cli.exe`; no guest is
rebuilt.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green before the probe is built and again after the take
closes. The probe file is not in any guest's dependency closure.

## Phase 1 — the screening probe

```bash
git apply docs/specs/perf-vs-orca/evidence/t99-clip-split-union/probe.patch
cargo build -p pnp-cli --release
ROOT=.local-artifacts/perimeter-reference-preparation/t99-clip-split-union
mkdir -p "$ROOT/snapshots/ordinary"
cp -f target/release/pnp_cli.exe "$ROOT/snapshots/ordinary/pnp_cli.exe"
cp -rf target/dist/developer/modules "$ROOT/snapshots/ordinary/modules"

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t99-clip-split-union/run-one.ps1 \
    -Mode ordinary -Arm probe-prod-first -Run 1
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t99-clip-split-union/run-one.ps1 \
    -Mode ordinary -Arm probe-split-first -Run 1

python docs/specs/perf-vs-orca/evidence/t99-clip-split-union/extract.py
# revert the probe
git checkout -- crates/slicer-runtime/src/slice_postprocess_prepass.rs
```

`run-one.ps1` is one slice of ordinary base classic supports-off with
`RAYON_NUM_THREADS=12` and an external `GetProcessTimes` wall + process CPU, and
it **throws on output drift**. Both arms are expected to reproduce the reference
`292df501…` (21,023,739 bytes) because the counterfactual never feeds production
geometry. Each run writes `rows-<order>.csv`-shaped per-call rows under
`.local-artifacts/.../phase1/ordinary/`; the copies committed here are
`rows-prod-first.csv` / `rows-split-first.csv` with their `run-*.json`.

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t99-clip-split-union/verify.py --self-test
```

Exits 0 against the in-repo set alone: it re-derives both runs' accumulated
production and split spans, the 0.8230x / 0.8252x ratio and its 0.27 % order
spread, the 0.43 % / 0.41 % share against the 585.4 ms floor, the union-free
bound, the exactness counts, and rejects six negative controls.

## Phase 2

Not run. The shape is below the 0.80 % floor in both orders before any
representation question is asked, and it is not representation-safe on top, so
no candidate, no 16-cell proof and no paired campaign exist for t99.
