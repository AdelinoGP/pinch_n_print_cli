# Reproducing the t126 post-t123 phase split

Ticket: [`126-post-t123-phase-split-refresh.md`](../../../issues/126-post-t123-phase-split-refresh.md)

All commands run from this directory. The heavy fixtures `tmp/3dbenchy.stl` and
`tmp/base.stl` are user-supplied and never committed; everything else here is.

```bash
# 0. gates (both must exit 0 before any measurement is attributed)
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
cargo build --release -p pnp-cli          # ordinary arm; record its SHA-256

# 1. one instrumented pass per cell, all eight cells (attribution only)
python run-instrumented.py

# 2. reduce raw event streams -> summary.json (the committed reduction)
python reduce.py

# 3. counterfactual arithmetic against ticket 124's medians of record
python arithmetic.py

# 4. verify (checkout-only; inputs are the committed files)
python verify.py              # 0 = every claim holds
python verify.py --self-test  # negative controls must all be detected
```

Raw event streams, `runs.json` and G-code land in
`target/t126-instrumented/` (gitignored); `summary.json` and
`counterfactuals.json` are the committed reductions.

## Regimes, and why they are never mixed

- **Shares and brackets** come from the instrumented passes. `--instrument-stderr`
  inflates wall, so an instrumented wall is never a wall claim; a phase's
  implied wall is `share x ticket 124's median of record`, never a subtraction
  of an instrumented bracket from an uninstrumented wall.
- **Denominators** are ticket 124's paired, interleaved medians
  (`evidence/t124-post-t123-scoreboard/scoreboard-summary.json`) — the current
  ratios of record. No ratio is taken from before t123.
- **Per-layer and per-module sums are thread-accumulated, never wall.** The
  per-layer phase runs on the rayon pool, so its `sum(module elapsed)` exceeds
  the phase wall by the measured parallelism. A module's wall-equivalent
  ceiling is `thread_sum / parallelism`, which charges 100 % of that module's
  parallel work to the critical path; it is an **upper bound**, not a price.
- The **module-busy union** is the union of every `module_start`/`module_complete`
  interval in the phase; `phase wall - union` is the host gap (idle wall inside
  the phase). Intervals are paired FIFO per `(phase, module_id)`.

## Provenance and invariants

- The ordinary arm must be ticket 123's candidate artifact
  (`7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0`) on a
  tree whose only changes since t123 are docs.
- Every instrumented pass must exit 0, be `degraded=false` with
  `fatal_error_count=0` and `non_fatal_error_count=0`, and produce G-code whose
  SHA-256 equals the ticket-123 candidate-ordinary reference for its cell.
- `RAYON_NUM_THREADS=12`; the standing matched-pair configs under
  `evidence/matched-pair/configs/`; 24 external core modules.
- Record the machine load sample beside the take; the instrumented pass is
  attribution, but a loaded box still moves shares.
