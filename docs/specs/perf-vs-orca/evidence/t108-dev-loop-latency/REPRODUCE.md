# Reproducing the t108 dev-loop latency take

Ticket: [`108-dev-loop-latency-preflight-concurrency.md`](../../issues/108-dev-loop-latency-preflight-concurrency.md)

All commands run from the repository root. Nothing here needs the heavy model
fixtures, a guest build, or the Orca rig.

## 1. Verify the committed set (checkout-only)

```bash
python docs/specs/perf-vs-orca/evidence/t108-dev-loop-latency/verify.py
python docs/specs/perf-vs-orca/evidence/t108-dev-loop-latency/verify.py --self-test
```

`verify.py` reads only `summary.json` plus `xtask/src/test.rs` (presence check
for the candidate's seam). It re-derives the medians, the delta, the percentage,
the non-overlap, the gate arithmetic and the inventory claims, and exits non-zero
if any of them stops holding.

## 2. Re-run the lever A/B

The claim is about the harness binary, so the two arms must be frozen binaries
built from the same HEAD; invoking `cargo xtask` directly would rebuild the
harness inside the timed window.

```bash
# before: stash the change and build the harness
git stash push -- xtask/src/test.rs
cargo build -p xtask
cp target/debug/xtask.exe target/t108-xtask-before.exe
git stash pop

# after: rebuild with the change applied
cargo build -p xtask
cp target/debug/xtask.exe target/t108-xtask-after.exe

# 4 interleaved rounds; each round reports milliseconds
for round in 1 2 3 4; do
  for arm in before after; do
    S=$(date +%s%N)
    ./target/t108-xtask-$arm.exe test \
        -p slicer-runtime --test contract authored_tool_index_tdd --summary \
        >/dev/null 2>&1
    E=$(date +%s%N)
    echo "$arm $round $(( (E-S)/1000000 ))"
  done
done
```

The payload (`authored_tool_index_tdd`, 4 tests) executes in 0.00 s and does not
rebuild, so the delta is the gate.

## 3. Re-measure the gate's components

```bash
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
```

Run them back to back; the serial sum is what `xtask test` used to pay, and the
slowest of the three is what it pays now.

## 4. Behavioural check of the candidate

```bash
cargo test -p xtask preflights_overlap -- --nocapture
```

The test records each closure's run interval and asserts the three intervals
overlap while each closure's own exit code survives the join. A sequential
implementation fails the overlap assertion rather than passing slowly.
