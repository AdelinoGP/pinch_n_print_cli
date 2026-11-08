# Reproducing the 2026-10-17/18 scoreboard refresh

The rig is the unchanged ticket-11 one
([`../matched-pair/run_scoreboard.ps1`](../matched-pair/run_scoreboard.ps1),
summarizer beside it). Paths assume the repo root as CWD, PowerShell 7.

## Prerequisites (the rig's own preflight enforces all of these)

- `tmp/3dbenchy.stl`, `tmp/base.stl` (user-supplied, never committed) and the
  OrcaSlicer install + BBL vendor profile triple at
  `C:\Program Files\OrcaSlicer\`.
- `cargo build --release -p pnp-cli` (writes `target/release/pnp_cli.exe`) and
  a complete accelerated snapshot at `target/dist-accelerated/developer/`
  (`cargo xtask dist --accelerated`).
- Both guest freshness gates must exit 0:
  `cargo xtask build-guests --check` and
  `cargo xtask build-guests --accelerated --check` (the rig runs both in every
  batch preflight and aborts on non-zero).
- `pnp_cli module diagnose` pass=true with all modules provenance `external`
  for both standing trees (rig preflight).

## Batches as run

Note: `pwsh -File` cannot bind comma-joined array params; the campaign used
`-Command`, e.g.:

```powershell
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' -Fixtures 'benchy' -Cells 'classic-off','arachne-off','classic-on','arachne-on' -Tools 'orca','pnp-ordinary','pnp-accelerated' -Runs 3 -Warmups 1 -BatchId refresh-20261017-benchy -KeepGcode Last"
```

1. `refresh-20261017-benchy` — benchy, all cells, all tools (complete).
2. `refresh-20261017-base` — base, all cells, all tools (evicted mid
   `arachne-off`; only its complete `classic-off` rows are ledger input,
   curated into `rows-base-classic-off.csv`).
3. `refresh-20261017-base-rest` — base `arachne-off`, `classic-on`,
   `arachne-on` (evicted mid `classic-on` m2; only its complete `arachne-off`
   rows are ledger input, curated into `rows-base-arachne-off.csv`).
4. Recovery chunks, base `classic-on` and `arachne-on`, one tool per batch:
   `-Cells '<cell>' -Tools '<tool>'` with BatchIds
   `refresh-20261018-base-<cell>-{orca,ord,acc}`.

The two evicted partial batches are preserved verbatim as
`rows-refresh-20261017-base.interrupted.csv` and
`rows-refresh-20261017-base-rest.interrupted.csv` (and the raw capture trees
remain under the gitignored `target/matched-pair/<BatchId>/`).

## Reduce

```powershell
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/summarize_scoreboard.ps1' -BatchIds 'refresh-20261017-benchy','refresh-20261017-base','refresh-20261017-base-rest','refresh-20261018-base-classic-on-orca','refresh-20261018-base-arachne-on-orca','refresh-20261018-base-classic-on-ord','refresh-20261018-base-classic-on-acc','refresh-20261018-base-arachne-on-ord','refresh-20261018-base-arachne-on-acc'"
```

`SUMMARY.txt` is that command's output as recorded (full per-run dump,
starvation flags, medians, sections).

## Verify from the committed set only

```bash
python docs/specs/perf-vs-orca/evidence/scoreboard-refresh-20261017/verify.py
python docs/specs/perf-vs-orca/evidence/scoreboard-refresh-20261017/verify.py --self-test
```

`verify.py` re-derives all 24 medians and the 8 wall-gap ratios from the nine
committed row CSVs and asserts the starvation state, sample counts and
output-flags; it never reads the raw capture tree.
