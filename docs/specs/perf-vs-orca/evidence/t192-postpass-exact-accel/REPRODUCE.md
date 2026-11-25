# Reproduce t192 (exact Zero-WIT `PostPass` acceleration)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/verify.py
```

Both exit `0` using only the committed `reduced/*.json` plus the committed
sibling artifacts `../t182-spatial-hierarchical-phase3-union/reference.json`
and `../t191-support-analysis-exact-accel/reduced/route.json`.

## Full campaign replay

1. **Baseline arms** (pre-t192 tree, commit `3eec27b0`):
   - `cargo xtask dist --edition developer`
     Stage `target/dist/developer/pnp_cli.exe` to
     `target/t192-baseline/pnp_cli-ordinary.exe` and `target/dist/developer/modules/`
     to `target/t192-baseline/modules/`.
   - `cargo xtask dist --edition developer --accelerated`
     Stage `target/dist-accelerated/developer/pnp_cli.exe` to
     `target/t192-baseline-accelerated/pnp_cli-accelerated.exe` and
     `target/dist-accelerated/developer/modules/` to
     `target/t192-baseline-accelerated/modules/`.
2. **Candidate arms** (t192 tree, zero WIT schema edits):
   - `cargo xtask build-guests && cargo xtask build-guests --accelerated`
   - `cargo xtask dist --edition developer`
     Stage `target/dist/developer/pnp_cli.exe` to
     `target/t192-candidate/pnp_cli-ordinary.exe` and `target/dist/developer/modules/`
     to `target/t192-candidate/modules/`.
   - `cargo xtask dist --edition developer --accelerated`
     Stage `target/dist-accelerated/developer/pnp_cli.exe` to
     `target/t192-candidate-accelerated/pnp_cli-accelerated.exe` and
     `target/dist-accelerated/developer/modules/` to
     `target/t192-candidate-accelerated/modules/`.
   - Verify both `cargo xtask build-guests --check` and
     `cargo xtask build-guests --accelerated --check` exit `0`.
3. **Campaign** (quiet box, `RAYON_NUM_THREADS=12` set inside `run_t192.ps1`):

```powershell
$run = "docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/run_t192.ps1"
pwsh -NoProfile -File $run -Mode output -BatchId ref
pwsh -NoProfile -File $run -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File $run -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel2 -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Instrument -BatchId stage -Pairs 3
pwsh -NoProfile -File $run -Mode profile -BatchId profile -Cells benchy-classic-on,benchy-arachne-on,base-classic-on,base-arachne-on
```

4. **Reduce and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/reduce_t192.py
python docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/verify.py
```

## Code, test, and freshness gates

```bash
cargo test -p slicer-gcode --lib 2>&1 | tee target/test-output.log
cargo test -p slicer-sdk --lib layer_collection_view_new_lazy 2>&1 | tee target/test-output.log
cargo test -p slicer-wasm-host --lib layer_collection_view_data_from_ir_shared 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --lib kinematic_fingerprint_tests 2>&1 | tee target/test-output.log
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
