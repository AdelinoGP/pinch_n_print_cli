# t161 reproduce — post-t158 eight-cell scoreboard

This take edits no production code. It re-derives the matched-settings ratios on
the current tree in one interleaved window, and checks that the output is the
currency the map already quotes.

## Prerequisites

- `tmp/3dbenchy.stl` (the 2026-11-12 replacement, sha256 `6ab57f1c…`): user-supplied,
  never committed.
- `tmp/base.stl` (sha256 `b26c8c79…`): user-supplied, never committed.
- The matched-pair configs at `evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- OrcaSlicer at `C:\Program Files\OrcaSlicer`, with the vendor tree materialized:
  `pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/orca-rig-repair/prepare_orca_vendor.ps1`
  (produces `target/orca-vendor/`).
- `pwsh`, `python` (stdlib only).
- A 12-logical-core box. **The numbers are box-state dependent** — see §5.

## 1. Freshness and build

```bash
cargo xtask build-guests --check            # exit 0
cargo xtask build-guests --accelerated --check   # exit 0
cargo build --release -p pnp-cli            # target/release/pnp_cli.exe at HEAD
```

The scoreboard driver re-runs both freshness checks and `pnp_cli module diagnose`
in its own preflight and fails closed.

## 2. The campaign

One window, one CSV. `-Cells` / `-Tools` / `-Fixtures` bind as arrays only under
`-Command` (from `pwsh -File` a comma string arrives as one value and fails the
`ValidateSet`).

```bash
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy,base \
  -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary -Runs 3 -Warmups 1 -BatchId t161-scoreboard"
```

Rows land in `target/matched-pair/t161-scoreboard/scoreboard-rows.csv` (header +
64 rows: 8 cells × 2 tools × (1 warmup + 3 measured)). The driver asserts the
matched keys from each arm's own resolved-config appendix, validates every PnP
run through `resources/perimeter-acceptance/validate_measurement.ps1`, and aborts
the batch on any mismatch.

Box load is sampled beside the campaign (20 s cadence) into
`target/t161-load.csv`; the reduced copy is `reduced/load-samples.csv`.

## 3. The accelerated attempt (expected to fail)

```bash
cargo xtask dist --accelerated --edition developer
```

Recorded for the disclosure only. The known failure is the controlled driver's
`libmimalloc-sys` verbatim-source-path bug (t142, still open at t145/t148/t151).
No manual-cfg host is substituted: `docs/23_controlled_perimeter_builds.md`
forbids granting the acceleration cfg from an ordinary invocation.

**Timing hygiene:** this build is run *after* the campaign, never during it.

## 4. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t161-scoreboard-refresh
python reduce_t161.py         # raw campaign csv + reference.json -> reduced/summary.json
python verify.py              # re-derives every quoted number, exit 0
python verify.py --self-test  # 15/15 mutations rejected, exit 0
```

`reduce_t161.py` reads the gitignored raw campaign tree; `verify.py` is
stdlib-only and reads only `reduced/summary.json` plus
`evidence/t151-skirt-hull-contour/reference.json`, so every claim is checkable
from a plain checkout without the raw tree.

## 5. Restore

Nothing to restore: no production change. Both freshness gates still exit 0.
