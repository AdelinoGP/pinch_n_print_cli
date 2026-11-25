# Reproduce t185 (exact SupportGeometry post-planner host tail)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/verify.py
```

Both exit `0` using only the committed `reduced/*.json` plus the committed
sibling artifacts `../t182-spatial-hierarchical-phase3-union/reference.json`
and `../t184-shell-classification-exact-accel/reduced/route.json`.

## Full campaign replay

1. **Baseline arms** (pre-t185 tree, t184 landed): stash the t185 changes, then
   - `cargo xtask dist --edition developer`, copy `target/dist/developer/pnp_cli.exe`
     and `modules/` to `target/t185-baseline/pnp_cli-ordinary.exe` and
     `target/t185-baseline/modules/`;
   - `cargo xtask dist --edition developer --accelerated`, copy
     `target/dist-accelerated/developer/pnp_cli.exe` and `modules/` to
     `target/t185-baseline-accelerated/pnp_cli-accelerated.exe` and
     `target/t185-baseline-accelerated/modules/`.
2. **Candidate arms** (t185 tree): the same two builds, staged into
   `target/t185-candidate/` and `target/t185-candidate-accelerated/`.
   t185 edits host crates only, so `diff -rq` of each arm pair's `modules/`
   must be empty (the reducer records this as `modules_identical`).
3. **Campaign** (unsandboxed, quiet box — close games, browsers and other
   builds first; the script records a CPU-load sample per batch):

```powershell
$run = "docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/run_t185.ps1"
pwsh -NoProfile -File $run -Mode output -BatchId ref
pwsh -NoProfile -File $run -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File $run -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Accelerated -BatchId accel -Pairs 3
pwsh -NoProfile -File $run -Mode ab -Instrument -BatchId stage -Pairs 3
# window 2 (PREREGISTRATION.md): the verdict of record pools ab+ab2 and null+null2
pwsh -NoProfile -File $run -Mode ab -BatchId ab2 -Pairs 5
pwsh -NoProfile -File $run -Mode ab -NullControl -BatchId null2 -Pairs 3
```

   Every run fails closed on a hash that differs from t182's active set. The
   `stage` batch passes `--instrument-stderr` and is attribution only.

4. **Reduce and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/reduce_t185.py
python docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t185-support-geometry-host-tail-accel/verify.py
```

## Code, test and freshness gates

```bash
cargo test -p slicer-wasm-host --lib -- support_overlap
cargo test -p slicer-wasm-host --test contract -- support_exact_z_seeding
cargo test -p slicer-wasm-host
cargo test -p slicer-runtime --test integration -- support_family_routing support_family_closure support_agg_rasterizer
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
