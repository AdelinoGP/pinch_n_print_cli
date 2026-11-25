# Reproduce ticket 127

Everything runs from the workspace root. The only external inputs are the
user-supplied, licence-encumbered `tmp/3dbenchy.stl` and `tmp/base.stl`. Raw
campaign files and arm snapshots live under gitignored `target/`; the committed
reduced set verifies without them.

## 1. Checkout-only verification

```bash
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/verify.py
```

The positive run re-derives the output identity, provenance, the call-count
drop and the gate verdict. The self-test rejects every negative control.

## 2. Arm snapshots

Build and snapshot the unpatched baseline first:

```bash
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
cargo xtask dist --edition developer
mkdir -p target/t127-baseline
cp -r target/dist/developer target/t127-baseline/ordinary
cargo xtask dist --accelerated --edition developer
cp -r target/dist-accelerated/developer target/t127-baseline/accelerated
```

Apply [`candidate.patch`](candidate.patch), run its tests, and stage the
candidate. The patch changes only
`crates/slicer-wasm-host/src/selection.rs`, so every guest module is identical
between arms and only the host binary differs.

```bash
git apply docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/candidate.patch
cargo test -p slicer-wasm-host --lib selection::tests::raft_band_only
cargo test -p slicer-wasm-host --test contract raft_band_dispatch
cargo xtask dist --edition developer
mkdir -p target/t127-candidate
cp -r target/dist/developer target/t127-candidate/ordinary
cargo xtask dist --accelerated --edition developer
cp -r target/dist-accelerated/developer target/t127-candidate/accelerated
```

## 3. Output and paired campaigns

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/run_t127.ps1 -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/run_t127.ps1 -Mode ab -Pairs 3 -BatchId ab
```

The output mode writes 32 rows; AB writes 48. The driver checks both freshness
gates, diagnoses all 24 modules in every selected arm, validates each run's
matched generator and `slice_complete` evidence, and records wall, process CPU,
bytes and SHA-256. `-Append -OnlyMissing` resumes an existing batch without
replacing its recorded rows.

## 4. Instrumented probe and the transfer proof

```bash
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/run_instrumented.py --arm baseline
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/run_instrumented.py --arm candidate
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/reduce_instrumented.py \
  --rows target/t127/instrumented/rows.json
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/analyze_transfer.py
```

The probes run each arm once per cell with `--instrument-stderr`. The reducer
records the raft-default call count, the remaining `Layer::Infill` call coverage
and the output hash; the analysis records which Infill module is dispatched
first on each layer and that module's elapsed distribution. Baseline call counts
and thread sums in `reduced/instrumented.json` come from ticket 126's committed
instrumented pass.

## 5. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/reduce_t127.py \
  --baseline-binary-sha256 <baseline ordinary pnp_cli sha256> \
  --candidate-binary-sha256 <candidate ordinary pnp_cli sha256> \
  --baseline-module-sha256 <raft-default.wasm sha256, both arms> \
  --candidate-module-sha256 <raft-default.wasm sha256, both arms> \
  --baseline-accelerated-binary-sha256 <baseline accelerated pnp_cli sha256> \
  --candidate-accelerated-binary-sha256 <candidate accelerated pnp_cli sha256> \
  --baseline-accelerated-module-sha256 <raft-default.wasm sha256, both arms> \
  --candidate-accelerated-module-sha256 <raft-default.wasm sha256, both arms>
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/verify.py
```

## 6. Retirement

The measured verdict is DROP, so the source is retired (the patch above stays as
the record). Restore the tree and both host binaries:

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t127-raft-band-dispatch/candidate.patch
cargo xtask dist --edition developer
cargo xtask dist --accelerated --edition developer
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

The rebuilt host binaries are not byte-identical to the campaign snapshots: each
differs by exactly 28 PE-header bytes (the linker's timestamp/debug-GUID stamp).
Do not refresh the Orca scoreboard: nothing landed.
