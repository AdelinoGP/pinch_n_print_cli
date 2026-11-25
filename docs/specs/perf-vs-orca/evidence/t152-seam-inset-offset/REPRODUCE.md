# t152 reproduce — seam inset deletion screen

The screened candidate removes the prepass inset offset from
`run_region_planning_entries`
(`modules/core-modules/seam-planner-default/src/lib.rs`) and emits the raw
chosen position. The per-layer `seam-placer` is untouched. This directory's
[`candidate.patch`](candidate.patch) is the exact screened diff; it is **not**
landed.

## Prerequisites

- `tmp/base.stl` and `tmp/3dbenchy.stl` (user-supplied, never committed).
- The matched configs at `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- `target/release/pnp_cli.exe` built from HEAD.
- The guest tree built and fresh: `cargo xtask build-guests --check` exit 0.
- `pwsh` and Python 3 (stdlib only).
- 12 logical cores: `RAYON_NUM_THREADS=12`.

## 1. Baseline arm

```bash
cargo xtask build-guests                 # baseline wasm in modules/core-modules
rm -rf target/t151-baseline/modules
cp -r modules/core-modules target/t151-baseline/modules
cp -f target/release/pnp_cli.exe target/t151-baseline/pnp_cli-ordinary.exe
```

## 2. Candidate arm

```bash
git apply docs/specs/perf-vs-orca/evidence/t152-seam-inset-offset/candidate.patch
cargo xtask build-guests                 # rebuilds seam-planner-default only
rm -rf target/t151-candidate/modules
cp -r modules/core-modules target/t151-candidate/modules
```

The two arms share one byte-identical host binary; only the
`seam-planner-default` guest differs.

## 3. Output screen (one cell)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/run_t151.ps1 \
  -Mode output -Cells base-classic-off \
  -Arms baseline-ordinary,candidate-ordinary -BatchId t152-screen
```

Baseline must reproduce the t151 active reference for base classic-off
(`9bb18f0e8173…`); the measured candidate is `4265d619dbf1…`.

## 4. Stage timing

```bash
RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t152/base-classic-off-baseline.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir target/t151-baseline/modules --no-default-module-paths \
  --instrument-stderr 2> target/t152/base-classic-off-baseline.stderr.jsonl

RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t152/base-classic-off-candidate.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir target/t151-candidate/modules --no-default-module-paths \
  --instrument-stderr 2> target/t152/base-classic-off-candidate.stderr.jsonl
```

## 5. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t152-seam-inset-offset
python census.py \
  ../../../../../target/t151/t152-screen/gcode/base-classic-off-baseline-ordinary.gcode \
  ../../../../../target/t151/t152-screen/gcode/base-classic-off-candidate-ordinary.gcode \
  --json-out census.json
python stage.py \
  ../../../../../target/t152/base-classic-off-baseline.stderr.jsonl \
  ../../../../../target/t152/base-classic-off-candidate.stderr.jsonl \
  --json-out stage.json
python verify.py
python verify.py --self-test
```

## 6. Unit-test falsifier

Run in each arm; the candidate arm is the one with the patch applied:

```bash
cargo test --manifest-path modules/core-modules/seam-planner-default/Cargo.toml --lib
```

Baseline: 31 passed / 0 failed. Candidate: 29 passed / 2 failed
(`batched_inset_offsets_match_the_serial_reference_per_region`,
`candidate_less_region_still_emits_one_keyed_plan_entry`).

## 7. Restore

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t152-seam-inset-offset/candidate.patch
cargo xtask build-guests
cargo xtask build-guests --check          # exit 0
```

Nothing else is restored: the screen made no production change.
