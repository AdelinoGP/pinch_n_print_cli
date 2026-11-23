# Reproduce t96 — windowed clip area in internal-bridge qualification

All commands run from the workspace root. The model fixtures `tmp/base.stl` and
`tmp/3dbenchy.stl` are user-supplied and gitignored. The probe is host-only, so
the snapshot is the standing `target/dist/developer` module tree plus a
probe-built `pnp_cli.exe`; no guest is rebuilt.

## Preconditions

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
```

Both gates must be green before the probe is built and again after the take
closes. The probe file is not in any guest's dependency closure.

## Phase 1 — the screening probe

```bash
git apply docs/specs/perf-vs-orca/evidence/t96-windowed-clip-area/probe.patch
cargo build -p pnp-cli --release
ROOT=.local-artifacts/perimeter-reference-preparation/t96-windowed-clip-area
mkdir -p "$ROOT/snapshots/ordinary" "$ROOT/snapshots/accelerated"
cp -f target/release/pnp_cli.exe "$ROOT/snapshots/ordinary/pnp_cli.exe"
cp -rf target/dist/developer/modules "$ROOT/snapshots/ordinary/modules"
# accelerated is only needed if the exactness screen is being re-run there
cp -f target/release/pnp_cli.exe "$ROOT/snapshots/accelerated/pnp_cli.exe"
cp -rf target/dist-accelerated/developer/modules "$ROOT/snapshots/accelerated/modules"

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t96-windowed-clip-area/run-one.ps1 \
    -Mode ordinary -Arm probe -Run 1
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t96-windowed-clip-area/run-one.ps1 \
    -Mode ordinary -Arm probe-window -Run 2
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t96-windowed-clip-area/run-one.ps1 \
    -Mode ordinary -Arm window -Run 1     # expected to fail the SHA check: that is the finding

python docs/specs/perf-vs-orca/evidence/t96-windowed-clip-area/extract.py
# revert the probe
git checkout -- crates/slicer-runtime/src/slice_postprocess_prepass.rs
```

`run-one.ps1` is one slice of ordinary base classic supports-off with
`RAYON_NUM_THREADS=12` and an external `GetProcessTimes` wall + process CPU. The
`probe` arm runs the production whole-layer dilation and times the windowed
counterfactual after it; the `probe-window` arm is the order check (windowed
production, whole-layer counterfactual after it). The `window` arm is
uninstrumented and **throws on output drift** — expected: the windowed form emits
`5fc8fa72…` (21,023,889 bytes) against the reference `292df501…` (21,023,739).

## Verification

```bash
python docs/specs/perf-vs-orca/evidence/t96-windowed-clip-area/verify.py --self-test
```

Exits 0 against the in-repo set alone: it re-derives both runs' accumulated
saving and share from `probe-rows.json` (130 rows each, above the 0.80 % floor),
checks that the windowed arm is deterministic and drifted, checks the first
differing block is an `Internal Bridge` at `;Z:21.4` with 8 → 10 moves, checks the
inside-the-window set check is clean at that layer (the isolation argument), and
rejects six negative controls.

## Phase 2

Not run. The shape was rejected on exactness before the wall screen, so no
candidate, no 16-cell proof and no paired campaign exist for t96.
