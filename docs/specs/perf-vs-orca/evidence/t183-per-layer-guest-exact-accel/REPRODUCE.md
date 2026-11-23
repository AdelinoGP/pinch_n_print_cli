# Reproduce t183 (exact per-layer guest acceleration: `tree-support` renderer fast paths + `infill-linker` open-subject tagging)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/verify.py
```

Both commands exit `0` using only committed JSONs under `reduced/` and committed sibling artifacts (`t182/reference.json`, `t182/reduced/route.json`).

## Full campaign replay from baseline + candidate

1. **Stage baseline arms** (at pre-t183 HEAD `3e2aae38`, production code `1d9d6a94`):
   - Build `cargo xtask dist --edition developer` and copy `target/dist/developer/pnp_cli.exe` + `modules/` into `target/t183-baseline/pnp_cli-ordinary.exe` and `target/t183-baseline/modules/`.
   - Build `cargo xtask dist --edition developer --accelerated` and copy `target/dist-accelerated/developer/pnp_cli.exe` + `modules/` into `target/t183-baseline-accelerated/pnp_cli-accelerated.exe` and `target/t183-baseline-accelerated/modules/`.
2. **Stage candidate arms** (at `t183` tree):
   - Build `cargo xtask dist --edition developer` and stage into `target/t183-candidate/pnp_cli-ordinary.exe` + `modules/`.
   - Build `cargo xtask dist --edition developer --accelerated` and stage into `target/t183-candidate-accelerated/pnp_cli-accelerated.exe` + `modules/`.
3. **Verify 16/16 byte-identical output and deterministic `--profile` guest fuel**, then run the **standing 8-cell paired ordinary + null + accelerated gate** (unsandboxed, `RAYON_NUM_THREADS=12`, quiet box):

```powershell
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/run_t183.ps1 -Mode output -BatchId ref
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/run_t183.ps1 -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/run_t183.ps1 -Mode fuel -BatchId fuel
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/run_t183.ps1 -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/run_t183.ps1 -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/run_t183.ps1 -Mode ab -Accelerated -BatchId accel -Pairs 3
```

4. **Reduce and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/reduce_t183.py
python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t183-per-layer-guest-exact-accel/verify.py
```

5. **Pre-commit code, test, and freshness gates:**

```bash
cargo test -p clipper2-rust
cargo test -p slicer-core --features host-algos --no-fail-fast
cargo test -p slicer-sdk --features test
cargo test --manifest-path modules/core-modules/tree-support/Cargo.toml
cargo test --manifest-path modules/core-modules/infill-linker/Cargo.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --manifest-path modules/core-modules/tree-support/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path modules/core-modules/infill-linker/Cargo.toml --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
