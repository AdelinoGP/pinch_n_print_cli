# Reproduce — t114 e2e bucket latency

All commands run from the workspace root on Windows (Git Bash), quiet box where
noted. The slice numbers are debug-profile; that is the profile the dev loop
pays.

## 0. Freshness preconditions

```bash
cargo xtask build-guests --check          # exit 0
cargo build --bin pnp_cli                 # the locator's stale gate
```

`cargo xtask test` does both (guest check + `ensure_pnp_cli_fresh`); a plain
`cargo test -p slicer-runtime --test e2e` does neither. Nine e2e tests fail in
0.1 s on the stale-binary gate if the binary is older than `crates/*/src/**`.

## 1. One slice, decomposed

```bash
# plain wall (3 runs, median)
for i in 1 2 3; do
  TIMEFORMAT='wall=%3R'
  time target/debug/pnp_cli.exe slice \
    --model resources/calicat.stl \
    --module-dir modules/core-modules \
    --no-default-module-paths > /dev/null
done

# pipeline phases vs total
target/debug/pnp_cli.exe slice \
  --model resources/calicat.stl \
  --module-dir modules/core-modules \
  --no-default-module-paths --instrument-stderr \
  > /dev/null 2> target/probe-calicat.stderr

grep '"event":"phase_complete"' target/probe-calicat.stderr \
  | sed -E 's/.*"phase":"([a-z_]*)".*"elapsed_ms":([0-9]+).*/\1 \2/'
grep '"event":"slice_complete"' target/probe-calicat.stderr | tail -1
```

`fixed_ms = slice_complete.elapsed_ms − Σ phase_complete.elapsed_ms`.
`slice_complete` reports 11,844 ms against 1,833 ms of phases (HEAD) — a
10,011 ms fixed cost. The output hash is
`d12cdfd10dab66a0010a0b5d5fca53d9387f4e96227ad5e74d0ec18197c0e260`.

The 24-thread probe lived only inside
`load_live_modules_for_plan_manifest_first` (`crates/slicer-wasm-host/src/execution_plan_live.rs`):
compile every non-integrated module's `.wasm` into a slot on its own thread
before the binding loop, then consume the slots in order. It was reverted with
`git checkout -- crates/slicer-wasm-host/src/execution_plan_live.rs`; nothing in
this commit carries it.

## 2. Module inventory (serial)

```bash
# one module per process, one test thread, end-to-end wall
cargo test -p slicer-runtime --test e2e -- --test-threads 1 \
  lightning_migration_tdd:: 2>&1 | tee target/test-output.log
```

The sweep scripts are throwaway (`target/e2e-module-sweep.sh`,
`target/e2e-module-sweep2.sh`); the module wall is the `date +%s` bracket around
each `cargo test` invocation and the test count is the harness's
`test result:` line. `lightning_migration_tdd` was stopped after 1,411 s with 5
tests complete and a 6th in flight; `preparation_runtime_lifecycle_tdd` is the
ticket's recorded 14-of-16-in-30-minutes observation.

## 3. Concurrency

```bash
# alone
cargo test -p slicer-runtime --test e2e -- --exact \
  mm_real_fixture_gcode_tdd::mm_painted_fixture_t0_t1 --test-threads 1

# 12 processes at once (nextest), same test reported per-test
cargo nextest run -p slicer-runtime --test e2e -j 12 --no-fail-fast \
  --status-level pass --final-status-level slow
```

Alone: 23.25 s test time / 24.95 s process. Under `-j 12`: 200.622 s, with 12
e2e processes observed concurrently and a game process holding ~6.6 of 12 cores
(disclosed confound; the 8.6x is an upper bound, not a clean factor).

## 4. Verifier

```bash
python docs/specs/perf-vs-orca/evidence/t114-e2e-latency/verify.py
python docs/specs/perf-vs-orca/evidence/t114-e2e-latency/verify.py --self-test
```

Both exit 0 from a plain checkout with no raw measurement tree.
