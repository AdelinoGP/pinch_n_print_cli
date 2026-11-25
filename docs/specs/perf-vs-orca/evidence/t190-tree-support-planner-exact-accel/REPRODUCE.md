# Reproduce t190 (exact `com.core.tree-support-planner` pure-guest acceleration)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/verify.py
```

Both exit `0` using only the committed `reduced/*.json` plus the committed
sibling artifacts `../t182-spatial-hierarchical-phase3-union/reference.json`
and `../t185-support-geometry-host-tail-accel/reduced/route.json`.

## Full campaign replay

1. **Baseline arms** (pre-t190 tree, commit `36c096aa`): stash the t190 changes,
   then build both ordinary and accelerated developer distributions:
   - `cargo xtask dist --edition developer`
     Copy `target/dist/developer/pnp_cli.exe` to
     `target/t190-baseline/pnp_cli-ordinary.exe` and `target/dist/developer/modules/`
     to `target/t190-baseline/modules/`.
   - `cargo xtask dist --edition developer --accelerated`
     Copy `target/dist-accelerated/developer/pnp_cli.exe` to
     `target/t190-baseline-accelerated/pnp_cli-accelerated.exe` and
     `target/dist-accelerated/developer/modules/` to
     `target/t190-baseline-accelerated/modules/`.
2. **Candidate arms** (t190 tree, editing `modules/core-modules/tree-support-planner/src/lib.rs`
   only): rebuild only the guest WASMs (`cargo xtask build-guests` and
   `cargo xtask build-guests --accelerated`) and stage into `target/t190-candidate/`
   and `target/t190-candidate-accelerated/` with the exact same host `pnp_cli`
   executables and non-planner module trees as the baseline arms (only
   `modules/core-modules/tree-support-planner/tree-support-planner.wasm` differs).
   The reducer verifies `exe_identical`, `other_modules_identical`, and
   `planner_wasm_changed` in both modes via SHA-256.
3. **Campaign** (quiet box, `RAYON_NUM_THREADS=12` set inside `run_t190.ps1`;
   records instantaneous `Win32_Processor` load before/after each batch):

```powershell
$run = "docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/run_t190.ps1"
pwsh -NoProfile -File $run -Mode output -BatchId ref
pwsh -NoProfile -File $run -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File $run -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Instrument -BatchId stage -Pairs 3
pwsh -NoProfile -File $run -Mode fuel -BatchId profile
```

   Every run fails closed on a hash that differs from t182's active set. The
   `stage` batch passes `--instrument-stderr` and the `fuel` batch passes
   `--profile` (both attribution only).

4. **Reduce and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/reduce_t190.py
python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t190-tree-support-planner-exact-accel/verify.py
```

## Code, test, and freshness gates

```bash
cargo test -p tree-support-planner --lib -- t190_
cargo test -p tree-support-planner --no-fail-fast 2>&1 | tee target/test-output.log
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

Note: `tree-support-planner --test slicer_module_binding_tdd::binding_surface_matches_manifest`
is a pre-existing failure on baseline commit `36c096aa` (`2.1.0#run` vs `2.0.0#run` in
`modules/core-modules/tree-support-planner/tree-support-planner.toml`, untouched by t190).
All 9 other test binaries (`160/160` tests, including all 5 new `t190_*` tests) pass.
