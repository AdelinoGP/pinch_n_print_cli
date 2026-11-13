# t143 reproduce — union-phase request grouping

Candidate shape: **raise the planner's emit buffering depth**
(`EMIT_BATCH_LAYERS` in `modules/core-modules/tree-support-planner/src/lib.rs`)
from 4 to 16 layers, so the planner's union requests are issued to
`clip_polygons_batch` roughly seventeen per call instead of 4.4, and the
occupancy batch is issued sixteen layers at a time instead of four.

Everything else is unchanged: each request is the same clipper op with the same
arguments, the host returns the i-th result for the i-th request, and the flush
order stays layer-ascending, so exactness is by construction.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- A 12-logical-core box (`RAYON_NUM_THREADS=12`); the quoted floors assume it.
- `pwsh` for the campaign driver, `python` (stdlib only) for the depth probe,
  reducer and verifier.

## 1. Baseline arm (HEAD before the constant change)

```bash
cargo xtask build-guests --check                 # exit 0
cargo build --release -p pnp-cli
mkdir -p target/t143-baseline
cp target/release/pnp_cli.exe target/t143-baseline/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t143-baseline/modules
```

## 2. Probe (temporary, reverted): buffering-depth table

The depth table needs the t136 host/guest instrumentation, which already marks
the union phase and folds every host batch call into it:

```bash
git apply docs/specs/perf-vs-orca/evidence/t136-clip-fanout/probe.patch
cargo xtask build-guests
cargo build --release -p pnp-cli
```

For each depth (4 / 8 / 12 / 16), set `EMIT_BATCH_LAYERS` in
`modules/core-modules/tree-support-planner/src/lib.rs`, rebuild the guest
(`cargo xtask build-guests`), and run the probe on the two supports-on cells:

```bash
python docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_depth.py \
  --cell base-classic-on --depth 4
python docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_depth.py \
  --cell benchy-classic-on --depth 4
```

Depth 12 is the candidate; 4, 8 and 16 are the pre-declared table. The table
shows 16 failing its own merge-4 bracket (its thread-time inflates 24 %), so the
production constant carries 12.

`run_depth.py` records wall, CPU, peak working set
(`PROCESS_MEMORY_COUNTERS.PeakWorkingSetSize`), the `t136-segment` union wall,
the clip-core guest span, and the time from process launch to the first
per-layer stage event. It fails on any G-code hash that is not the active
post-t139 reference. Restore the probe tree afterwards:

```bash
git checkout -- crates modules
git reset -- crates/slicer-wasm-host/src/t136_probe.rs
rm -f crates/slicer-wasm-host/src/t136_probe.rs
```

## 3. Candidate arm

```bash
# EMIT_BATCH_LAYERS = 12 (the depth chosen by the measured table; set it after
# the probe restore, which reverts the const to 4)
cargo test -p tree-support-planner
cargo xtask build-guests                         # planner guest only
cargo build --release -p pnp-cli                 # host unchanged
mkdir -p target/t143-candidate
cp -r modules/core-modules target/t143-candidate/modules
# both arms share one byte-identical host binary
cp target/release/pnp_cli.exe target/t143-baseline/pnp_cli-ordinary.exe
```

Record the arms:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode manifest
```

## 4. Campaign

```bash
# 16-run output gate: baseline vs candidate, one run per cell per arm
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode output -BatchId output

# interleaved paired A/B, all eight cells, 3 pairs each
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode ab -Pairs 3 -BatchId ab
```

A null control (the same arm on both sides, same interleaving) bounds the
instrument's own paired noise. The take ran it on the four supports-off cells
(where the form's "no regression" clause is read) and on the two supports-on
cells (where the CPU clause is read):

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode ab -Pairs 3 -NullControl -BatchId null -Cells benchy-classic-off,benchy-arachne-off,base-classic-off,base-arachne-off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t143-union-grouping/run_t143.ps1 -Mode ab -Pairs 3 -NullControl -BatchId null -Cells base-classic-on,benchy-classic-on -Append -OnlyMissing
```

## 5. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t143-union-grouping/reduce_t143.py
python docs/specs/perf-vs-orca/evidence/t143-union-grouping/verify.py
python docs/specs/perf-vs-orca/evidence/t143-union-grouping/verify.py --self-test
```

`reduce_t143.py` reads `target/t143/depth/rows.jsonl`, `target/t143/ab/rows.csv`
and `target/t143/output/rows.csv` and writes `reduced/summary.json`;
`verify.py` reads only that summary.
