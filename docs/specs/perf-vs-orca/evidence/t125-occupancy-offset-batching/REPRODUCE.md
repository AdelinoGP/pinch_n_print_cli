# Reproduce ticket 125

Everything runs from the workspace root. The only external inputs are the
user-supplied, licence-encumbered `tmp/3dbenchy.stl` and `tmp/base.stl`. Raw
campaign files and arm snapshots live under gitignored `target/`; the committed
reduced set verifies without them.

## 1. Checkout-only verification

```bash
python docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/verify.py
```

The positive run re-derives 442 checks. The self-test rejects all 10 negative
controls.

## 2. Arm snapshots

Build and snapshot the unpatched baseline first:

```bash
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
cargo xtask dist --edition developer
mkdir -p target/t125-baseline
cp -r target/dist/developer target/t125-baseline/ordinary
cargo xtask dist --accelerated --edition developer
cp -r target/dist-accelerated/developer target/t125-baseline/accelerated
```

Apply [`candidate.patch`](candidate.patch), run its tests, and stage the
candidate. The patch changes only
`modules/core-modules/tree-support-planner/src/lib.rs`; host binaries are
therefore identical between arms.

```bash
git apply docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/candidate.patch
cargo test --manifest-path modules/core-modules/tree-support-planner/Cargo.toml
cargo xtask build-guests
cargo xtask build-guests --accelerated
cargo xtask dist --edition developer
mkdir -p target/t125-candidate
cp -r target/dist/developer target/t125-candidate/ordinary
cargo xtask dist --accelerated --edition developer
cp -r target/dist-accelerated/developer target/t125-candidate/accelerated
```

## 3. Output and paired campaigns

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/run_t125.ps1 -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/run_t125.ps1 -Mode ab -Pairs 3 -BatchId ab
```

The output mode writes 32 rows; AB writes 48. The driver checks both freshness
gates, diagnoses all 24 modules in every selected arm, validates each run's
matched generator and `slice_complete` evidence, and records wall, process CPU,
bytes and SHA-256. `-Append -OnlyMissing` resumes an existing batch without
replacing its recorded rows.

## 4. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/reduce_t125.py \
  --baseline-binary-sha256 7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0 \
  --candidate-binary-sha256 7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0 \
  --baseline-module-sha256 a53e2df38778b7f29e1e27332a020861709b6c978d3ea18367fad9c95b68bcad \
  --candidate-module-sha256 9ce369d3b20e8abd3e7d58670f4f38e46d2c5e1eb7044c99133853a3e5942e9a \
  --baseline-accelerated-binary-sha256 a6ae2adc998572217664a124ac0ac62c58977cfd46804379dcf5153fbf53e41b \
  --candidate-accelerated-binary-sha256 a6ae2adc998572217664a124ac0ac62c58977cfd46804379dcf5153fbf53e41b \
  --baseline-accelerated-module-sha256 817ed9a4583d9e5502c8127d341fbced77100416025a33315f16de3c55cdcad3 \
  --candidate-accelerated-module-sha256 33554f970cd167d77728bafb745e34cf8fc8c02b9a7ea2b63222b2ea28d2e38d
python docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/verify.py --self-test
```

## 5. Retire the candidate

The measured verdict is DROP, so restore the source and both planner artifacts:

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/candidate.patch
cargo xtask build-guests
cargo xtask build-guests --accelerated
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

Do not refresh the Orca scoreboard: no candidate landed.
