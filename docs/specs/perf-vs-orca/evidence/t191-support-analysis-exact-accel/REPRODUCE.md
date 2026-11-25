# Reproduce t191 (exact host-only `PrePass::SupportAnalysis` acceleration)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/verify.py
```

Both exit `0` using only the committed `reduced/*.json` plus the committed
sibling artifacts `../t182-spatial-hierarchical-phase3-union/reference.json`
and `../t190-tree-support-planner-exact-accel/reduced/route.json`.

## Full campaign replay

1. **Baseline arms** (pre-t191 tree, commit `63e07300`):
   - `cargo xtask dist --edition developer`
     Stage `target/dist/developer/pnp_cli.exe` to
     `target/t191-baseline/pnp_cli-ordinary.exe` and `target/dist/developer/modules/`
     to `target/t191-baseline/modules/`.
   - `cargo xtask dist --edition developer --accelerated`
     Stage `target/dist-accelerated/developer/pnp_cli.exe` to
     `target/t191-baseline-accelerated/pnp_cli-accelerated.exe` and
     `target/dist-accelerated/developer/modules/` to
     `target/t191-baseline-accelerated/modules/`.
2. **Candidate arms** (t191 tree, host-only edits in `crates/slicer-core` and
   `crates/slicer-runtime`; zero guest source changes):
   - `cargo xtask dist --edition developer`
     Stage `target/dist/developer/pnp_cli.exe` to
     `target/t191-candidate/pnp_cli-ordinary.exe` and `target/dist/developer/modules/`
     to `target/t191-candidate/modules/`.
   - `cargo xtask dist --edition developer --accelerated`
     Stage `target/dist-accelerated/developer/pnp_cli.exe` to
     `target/t191-candidate-accelerated/pnp_cli-accelerated.exe` and
     `target/dist-accelerated/developer/modules/` to
     `target/t191-candidate-accelerated/modules/`.
   - Verify both `cargo xtask build-guests --check` and
     `cargo xtask build-guests --accelerated --check` exit `0`.
3. **Campaign** (quiet box, `RAYON_NUM_THREADS=12` set inside `run_t191.ps1`;
   records instantaneous `Win32_Processor` load before/after each batch):

```powershell
$run = "docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/run_t191.ps1"
pwsh -NoProfile -File $run -Mode output -BatchId ref
pwsh -NoProfile -File $run -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File $run -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Instrument -BatchId stage -Pairs 3
pwsh -NoProfile -File $run -Mode profile -BatchId profile
pwsh -NoProfile -File $run -Mode ab -BatchId ab2 -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null2 -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel2 -Pairs 3
```

   Every run fails closed on a hash that differs from t182's active set. The
   `stage` batch passes `--instrument-stderr` and the `profile` batch passes
   `--profile` (both attribution only).

4. **Reduce and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/reduce_t191.py
python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/verify.py
```

## Code, test, and freshness gates

```bash
cargo test -p slicer-core --features host-algos --lib triangle_mesh_slicer::tests::t191_ 2>&1 | tee target/test-output.log
cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --lib support_analysis_producer 2>&1 | tee target/test-output.log
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
