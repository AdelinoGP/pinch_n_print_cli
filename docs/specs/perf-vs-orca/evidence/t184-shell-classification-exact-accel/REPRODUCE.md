# Reproduce t184 (exact `PrePass::ShellClassification` acceleration)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/verify.py
```

Both exit `0` using only the committed `reduced/*.json` plus the committed
sibling artifacts `../t182-spatial-hierarchical-phase3-union/reference.json`
and `../t183-per-layer-guest-exact-accel/reduced/route.json`.

## Full campaign replay

1. **Baseline arms** (pre-t184 tree; production code unchanged since `785d67a1`):
   stash the t184 changes, then
   - `cargo xtask dist --edition developer`, copy `target/dist/developer/pnp_cli.exe`
     and `modules/` to `target/t184-baseline/pnp_cli-ordinary.exe` and
     `target/t184-baseline/modules/`;
   - `cargo xtask dist --edition developer --accelerated`, copy
     `target/dist-accelerated/developer/pnp_cli.exe` and `modules/` to
     `target/t184-baseline-accelerated/pnp_cli-accelerated.exe` and
     `target/t184-baseline-accelerated/modules/`.
2. **Candidate arms** (t184 tree): the same two builds, staged into
   `target/t184-candidate/` and `target/t184-candidate-accelerated/`.
3. **Campaign** (unsandboxed, `RAYON_NUM_THREADS=12`, quiet box — close games,
   browsers and other builds first; the script records a CPU-load sample per
   batch):

```powershell
$run = "docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/run_t184.ps1"
pwsh -NoProfile -File $run -Mode output -BatchId ref
pwsh -NoProfile -File $run -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File $run -Mode fuel -BatchId fuel
pwsh -NoProfile -File $run -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Instrument -BatchId stage -Pairs 3
pwsh -NoProfile -File $run -Mode ab -HostOnly -BatchId hostonly -Pairs 3
# window 2 (PREREGISTRATION.md): the verdict of record pools ab+ab2 and null+null2
pwsh -NoProfile -File $run -Mode ab -BatchId ab2 -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null2 -Pairs 3
```

   Every run fails closed on a hash that differs from t182's active set.
   - The `stage` batch passes `--instrument-stderr` and is attribution only; it
     is never part of the timing gate.
   - The `hostonly` batch runs the candidate host against the **baseline**
     modules, separating the host change from guest recompilation drift.

4. **Reduce and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/reduce_t184.py
python docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t184-shell-classification-exact-accel/verify.py
```

## Sub-phase probe (optional, attribution only)

`probe.raw.diff` is the env-gated timer patch (`PNP_T184_PROBE=1`) used to split
the stage into Pass 1 / Pass 2 / external-bridge orientation / internal-bridge
phases A, precompute and B. It was taken against an intermediate t184 tree and
is not committed production code; apply it by hand to a scratch copy only.

## Code, test and freshness gates

```bash
cargo test -p slicer-core --features host-algos --test shell_classification_exact_accel_tdd
cargo test -p slicer-core --features host-algos --no-fail-fast
cargo test -p slicer-runtime --lib -- slice_postprocess_prepass
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
