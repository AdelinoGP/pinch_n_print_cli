# t150 reproduce — MeshAnalysis bridge retirement

This take changes production code: `classify_object` stops producing
`bridge_regions` and the clustering machinery behind it is deleted. It then
re-slices all eight cells on both arms for identity, and runs the paired A/B
that measures the saving.

## Prerequisites

- `tmp/3dbenchy.stl` (sha256 `6ab57f1c…`) and `tmp/base.stl` (sha256
  `b26c8c79…`). Both user-supplied, never committed.
- The matched-pair configs at `evidence/matched-pair/configs/`.
- The core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; `pwsh`; `python` (stdlib only —
  `verify.py` and `reduce_t150.py` need no third-party package).
- A 12-logical-core box (`RAYON_NUM_THREADS=12`).

## 1. Freshness and the two arms

```bash
cargo xtask build-guests --check     # exit 0
```

The baseline is the pre-change tree. Stash the two candidate files, build,
save the binary, restore, build the candidate:

```bash
git stash push -- crates/slicer-core/src/algos/mesh_analysis.rs \
                   crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs
cargo build --release -p pnp-cli
cp target/release/pnp_cli.exe target/t150/bin/pnp_cli-baseline.exe
git stash pop
cargo build --release -p pnp-cli
cp target/release/pnp_cli.exe target/t150/bin/pnp_cli-candidate.exe
```

## 2. Identity — 8 cells x 2 arms

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement/run_t150.ps1 \
  -Mode output -BatchId output
```

Expected: 16 rows, every `sha256` equal to
`evidence/t148-print-plane-carve/reference.json`'s `active_set`, every
`degraded` false, `exact_z_rejections` 0.

## 3. Stage attribution (optional, instrumented)

```bash
for arm in baseline candidate; do
  RAYON_NUM_THREADS=12 target/t150/bin/pnp_cli-$arm.exe slice \
    --model tmp/base.stl \
    --output target/t150/stage/base-classic-off-$arm.gcode \
    --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
    --module-dir modules/core-modules --instrument-stderr \
    2> target/t150/stage/base-classic-off-$arm.stderr.log > /dev/null
done
grep '"stage":"PrePass::MeshAnalysis","module_id"' \
  target/t150/stage/base-classic-off-candidate.stderr.log
```

Expected `elapsed_ms` ≈ 4,445 on the baseline and ≈ 732 on the candidate. Stage
readings are not wall claims.

## 4. Paired A/B

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement/run_t150.ps1 \
  -Mode ab -Pairs 3 -NullControl -BatchId t150
# base classic-on and the null control were extended to 6 pairs:
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement/run_t150.ps1 \
  -Mode ab -Cells base-classic-on -PairFrom 4 -PairTo 6 -Append -BatchId t150
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement/run_t150.ps1 \
  -Mode null -PairFrom 4 -PairTo 6 -Append -BatchId t150
```

Each cell gets one discarded baseline warmup, then interleaved pairs whose
order alternates by pair parity. The CSV is appended per run.

## 5. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t150-meshanalysis-retirement
python reduce_t150.py        # writes census.json from target/t150/
python verify.py             # 71/71 checks
python verify.py --self-test # 5/5 negative controls rejected
```

## 6. Restore

Nothing to restore: the change is the deliverable. Both freshness gates exit 0,
`cargo clippy --workspace --all-targets -- -D warnings` is clean,
`cargo xtask check-literals` is clean, and `cargo xtask check-test-quality
--report` reports 0 findings.
