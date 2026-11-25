# Reproduce t182 (host-side spatial hierarchical Phase-3 union with convex-containment pruning)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/audit_t182.py --self-test
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/verify.py
```

Every script exits `0` using only committed JSONs under `reduced/`, optional `reference.json`, and committed sibling reduced artifacts (`t175/reference.json`, `t175/reduced/route.json`, `t178/reduced/arithmetic.json`).

## Full campaign replay from baseline + candidate

1. **Stage baseline arms** (at pre-t182 HEAD `7b82f188`, identical code to `t175` commit `43d167d6`):
   - Build `cargo xtask dist --edition developer` and copy `target/dist/developer/pnp_cli.exe` + `modules/` into `target/t182-baseline/pnp_cli-ordinary.exe` and `target/t182-baseline/modules/`.
   - Build `cargo xtask dist --edition developer --accelerated` and copy `target/dist-accelerated/developer/pnp_cli.exe` + `modules/` into `target/t182-baseline-accelerated/pnp_cli-accelerated.exe` and `target/t182-baseline-accelerated/modules/`.
2. **Stage parameter-sweep binaries** (at `t182` tree):
   - Build `cargo build --release --bin pnp_cli` for each combination of `DEFAULT_SPATIAL_UNION_CHUNK_SIZE ∈ {4, 8, 16}` × `DEFAULT_SPATIAL_UNION_NESTED_PAR ∈ {false, true}` into `target/t182-sweep/{worker,nested}-k{4,8,16}/pnp_cli.exe`.
3. **Stage final candidate arms** (selected winner `worker-k8`: `DEFAULT_SPATIAL_UNION_CHUNK_SIZE = 8`, `DEFAULT_SPATIAL_UNION_NESTED_PAR = false`):
   - Build `cargo xtask dist --edition developer` and stage into `target/t182-candidate/pnp_cli-ordinary.exe` + `modules/`.
   - Build `cargo xtask dist --edition developer --accelerated` and stage into `target/t182-candidate-accelerated/pnp_cli-accelerated.exe` + `modules/`.
4. **Run sweep, 16-cell output check, paired A/B, null control, and accelerated corroboration** (unsandboxed, `RAYON_NUM_THREADS=12`, quiet box):

```powershell
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/run_t182.ps1 -Mode sweep -BatchId sweep1 -SweepRuns 3
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/run_t182.ps1 -Mode output -BatchId ref
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/run_t182.ps1 -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/run_t182.ps1 -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/run_t182.ps1 -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/run_t182.ps1 -Mode ab -Accelerated -BatchId accel -Pairs 3
```

5. **Audit, reduce, and verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/audit_t182.py --sweep
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/audit_t182.py
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/reduce_t182.py
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/audit_t182.py --self-test
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/verify.py
```

6. **Pre-commit code, test, and freshness gates:**

```bash
cargo test -p slicer-core --features host-algos --no-fail-fast
cargo test -p slicer-wasm-host
cargo test -p slicer-sdk --features test --lib host_batch
cargo test --manifest-path modules/core-modules/tree-support-planner/Cargo.toml --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
