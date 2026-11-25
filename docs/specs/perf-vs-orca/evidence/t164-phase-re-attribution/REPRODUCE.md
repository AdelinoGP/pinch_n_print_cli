# t164 reproduce — phase re-attribution at HEAD

No production change: this take measures, reduces, and re-derives arithmetic.
Ticket: [`164-phase-re-attribution.md`](../../../issues/164-phase-re-attribution.md).

## Prerequisites

- `tmp/3dbenchy.stl` (the 2026-11-12 replacement) and `tmp/base.stl`: user-supplied,
  never committed.
- OrcaSlicer at `C:\Program Files\OrcaSlicer` (**2.5.1**, the t161 boundary), with
  the source-manifest vendor tree materialized at `target/orca-vendor/`.
- `target/release/pnp_cli.exe` built from the ticket's HEAD; its SHA-256 is
  recorded in `reduced/board.json`.
- `pwsh`, `python` (stdlib only), a 12-logical-core box.

## 1. Freshness and build

```bash
cargo xtask build-guests --check      # 0
cargo build --release -p pnp-cli      # record the binary SHA-256
```

## 2. The paired scoreboard (walls of record)

```bash
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy,base \
  -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary \
  -Runs 3 -Warmups 1 -BatchId t164-phase-split"
```

8 cells × 2 tools × (1 warmup + 3 measured) = **64 data rows** in
`target/matched-pair/t164-phase-split/scoreboard-rows.csv`. The rig validates
every run (generator marker, claim holder, `slice_complete` counters, Orca
resolved-config appendix incl. the matched keys) and aborts on failure.

Box load is sampled every 20 s into `target/t164-load.csv` for the duration of
both campaigns.

## 3. The instrumented split (shares, attribution only)

```bash
cd docs/specs/perf-vs-orca/evidence/t164-phase-re-attribution
RAYON_NUM_THREADS=12 python run-instrumented.py     # 8 cells x 3 passes
```

Each pass writes `<cell>.pass<N>.{jsonl,gcode}` into `target/t164-instrumented/`
and is hash-checked against
`evidence/t151-skirt-hull-contour/reference.json`; a mismatch fails the run.

## 4. Reduce and re-derive

```bash
python reduce_scoreboard.py       # raw csv + load csv -> reduced/board.json
python reduce_instrumented.py     # raw event streams -> reduced/instrumented.json
python arithmetic.py              # board + split + t110 constants -> reduced/arithmetic.json
python verify.py                  # 0 = every claim re-derives
python verify.py --self-test      # 16/16 negative controls rejected
```

`reduce_scoreboard.py` and `reduce_instrumented.py` need the gitignored raw
trees under `target/`; `verify.py` is stdlib-only and reads only the committed
`reduced/` set plus t151's reference.

## 5. Restore

Nothing to restore: no production change. Both freshness gates exit 0.

## Regimes, and why they are never mixed

- **Walls** come from the scoreboard (uninstrumented, one window, interleaved).
  **Shares** come from the instrumented passes; `--instrument-stderr` inflates
  wall, so an instrumented wall is never a wall claim and the two are never
  substituted for one another.
- **The implied phase wall** is `share × the wall of record`; it is a
  composition inside one window, never a subtraction of an instrumented bracket
  from an uninstrumented wall.
- **Canonical prepass costs** are t110's tap-derived split, held fixed as the
  substitution's input (`benchy-off` 50.5 ms is a bound, not a point). They are
  not measurements of this window.
- **The substitution result** is an `estimate`: prepass at canonical cost, every
  other phase at this take's measured share.
- **Per-layer and per-module sums are thread-accumulated, never wall.** The
  module-busy union is the union of every `module_start`/`module_complete`
  interval in the phase; `phase wall − union` is the host gap. Intervals are
  paired FIFO per `(phase, module_id)`.
