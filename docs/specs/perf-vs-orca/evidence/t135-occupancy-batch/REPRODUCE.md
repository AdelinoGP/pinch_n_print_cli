# t135 Stage 2 reproduce

Candidate: the cross-layer occupancy batch in
`modules/core-modules/tree-support-planner/src/lib.rs`
(`prefetch_occupancy_memo`, `layer_occupancy_payload`, the chunked emit pass).
Attribution-only probe reverted before the campaign; `probe.patch` is the
artifact.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- `pwsh` for the campaign driver, `python` for the reducer/verifier.

## 1. Baseline arm (HEAD, before the planner change)

At the take's HEAD (`8c874035`), with a clean tree:

```bash
cargo xtask build-guests --check                 # exit 0
cargo build --release -p pnp-cli
mkdir -p target/t135-baseline
cp target/release/pnp_cli.exe target/t135-baseline/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t135-baseline/modules
```

## 2. Candidate arm

Apply the planner change, then:

```bash
cargo test -p tree-support-planner               # 65 unit + 11 binaries
cargo xtask build-guests                         # rebuilds the planner guest only
cargo build --release -p pnp-cli                 # host unchanged
mkdir -p target/t135-candidate
cp -r modules/core-modules target/t135-candidate/modules
```

Record the arms (`baseline-ordinary` vs `candidate-ordinary` share the host
binary; only the planner guest differs):

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode manifest
```

## 3. Probe (temporary, reverted)

```bash
git apply docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/probe.patch
cargo build --release -p pnp-cli
mkdir -p target/t135/probe
for cell in benchy-classic-on benchy-arachne-on base-classic-on base-arachne-on; do
  fixture=3dbenchy; case $cell in base*) fixture=base;; esac
  gen=${cell#*-}; gen=${gen%-on}
  PNP_T135_PROBE=1 RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe slice \
    --model tmp/$fixture.stl \
    --output target/t135/probe/$cell.gcode \
    --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-$gen-supports-on.json \
    --module-dir target/t135-candidate/modules --no-default-module-paths \
    2> target/t135/probe/$cell.stderr.log
done
git apply -R docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/probe.patch
cargo build --release -p pnp-cli                 # production host for the campaign
```

The probe prints one `t135-batch ...` line per `offset_polygons_batch` call.
The reducer keeps only `miter_some=0` / positive-delta lines (the occupancy
batch; the foreign keep-out and seam insets pass `Some(3.0)`).

## 4. Campaign

```bash
# 16-run output gate: baseline vs candidate, one run per cell per arm
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode output -BatchId output

# interleaved paired A/B, all eight cells, 3 pairs each
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 3 -BatchId ab

# extend the supports-off cells (cheap) and the benchy supports-on cells
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 12 -PairFrom 4 -Append -OnlyMissing -BatchId ab -Cells benchy-classic-off,benchy-arachne-off,base-classic-off,base-arachne-off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 7 -PairFrom 4 -Append -OnlyMissing -BatchId ab -Cells benchy-classic-on,benchy-arachne-on
# base arachne-off was extended to 24 pairs after its first block landed in a
# slow machine window
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 24 -PairFrom 13 -Append -OnlyMissing -BatchId ab -Cells base-arachne-off

# null controls: the same arm on both sides, same interleaving
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 3 -BatchId null -NullControl -Cells benchy-classic-off,benchy-classic-on,benchy-arachne-off,benchy-arachne-on,base-classic-off,base-classic-on,base-arachne-off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 6 -PairFrom 4 -Append -OnlyMissing -BatchId null -NullControl -Cells base-arachne-off,base-classic-on
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode ab -Pairs 6 -BatchId null-candidate -NullControl -NullArm candidate-ordinary -Cells base-arachne-off
```

### Accelerated output parity (hybrid, disclosed)

`cargo xtask dist --accelerated --edition developer` builds the accelerated
guests but fails on the host in `libmimalloc-sys`'s verbatim source path (the
same failure t142 recorded). Build the accelerated module tree from the
accelerated artifacts and run it against the ordinary host:

```bash
cargo xtask build-guests --accelerated
mkdir -p target/t135-candidate/accelerated/modules
cp -r modules/core-modules/. target/t135-candidate/accelerated/modules/
for d in modules/core-modules/*/; do
  stem=$(basename "$d")
  art="target/guests-accelerated/artifacts/modules/core-modules/$stem/$stem.wasm"
  [ -f "$art" ] && cp -f "$art" "target/t135-candidate/accelerated/modules/$stem/$stem.wasm"
done
cp target/release/pnp_cli.exe target/t135-candidate/accelerated/pnp_cli.exe
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode manifest -Arms baseline-ordinary,candidate-ordinary,candidate-accelerated
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/run_t135.ps1 -Mode output -Arms candidate-accelerated -BatchId output-accelerated
```

## 5. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/reduce_t135.py
python docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/verify.py
python docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/verify.py --self-test
```

`verify.py` reads `reduced/summary.json` only: no raw G-code, no probe log, no
STL. `--self-test` mutates the summary eight ways and requires each mutation to
be rejected.
