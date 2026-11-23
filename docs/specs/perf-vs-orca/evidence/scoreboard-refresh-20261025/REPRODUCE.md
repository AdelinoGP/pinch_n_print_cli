# Reproducing the 2026-10-25 scoreboard refresh

The rig is the unchanged ticket-11 one
([`../matched-pair/run_scoreboard.ps1`](../matched-pair/run_scoreboard.ps1),
summarizer beside it). Paths assume the repo root as CWD, PowerShell 7.

## Prerequisites (the rig's own preflight enforces all of these)

- `tmp/3dbenchy.stl`, `tmp/base.stl` (user-supplied, never committed) and the
  OrcaSlicer install + BBL vendor profile triple at
  `C:\Program Files\OrcaSlicer\`.
- Both standing trees rebuilt from the measured HEAD:
  `cargo xtask dist --edition developer` (stages `target/release/pnp_cli.exe`
  and `target/dist/developer/`) and
  `cargo xtask dist --accelerated --edition developer`
  (`target/dist-accelerated/developer/`).
- Both guest freshness gates exit 0:
  `cargo xtask build-guests --check` and
  `cargo xtask build-guests --accelerated --check`.
- `pnp_cli module diagnose` pass=true with all modules provenance `external`
  for both standing trees (the rig preflight asserts 24/24).

## Batches as run

Note: `pwsh -File` cannot bind comma-joined array params; the campaign used
`-Command`, e.g.:

```powershell
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' -Fixtures 'benchy' -Cells 'classic-off','arachne-off','classic-on','arachne-on' -Tools 'orca','pnp-ordinary','pnp-accelerated' -Runs 3 -Warmups 1 -BatchId refresh-20261025-benchy2 -KeepGcode Last"
```

1. `refresh-20261025-benchy2` — benchy, all cells, all tools (complete; ledger).
2. `refresh-20261025-base-classic-off` — base, classic-off, all tools.
3. `refresh-20261025-base-arachne-off` — base, arachne-off, all tools.
4. `refresh-20261025-base-classic-on` — base, classic-on, all tools.
5. `refresh-20261025-base-arachne-on` — base, arachne-on, all tools.

`refresh-20261025-benchy` is **not** ledger input: a shell pipeline failed to
resolve `Tee-Object` and returned 127 after the PowerShell process had already
started, so that batch ID received two complete scheduled rounds. Both rounds
are preserved verbatim as `rows-benchy-duplicate-round.csv` (disclosure); the
clean re-run `refresh-20261025-benchy2` is the ledger.

## Reduce

```powershell
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/summarize_scoreboard.ps1' -BatchIds 'refresh-20261025-benchy2','refresh-20261025-base-classic-off','refresh-20261025-base-arachne-off','refresh-20261025-base-classic-on','refresh-20261025-base-arachne-on'"
```

`SUMMARY.txt` is that command's output as recorded (full per-run dump,
starvation flags, medians, section counts). The ledger CSVs in this directory
are the five batch `scoreboard-rows.csv` files copied verbatim; raw capture
trees remain under the gitignored `target/matched-pair/<BatchId>/`.

## Verify from the committed set only

```bash
python docs/specs/perf-vs-orca/evidence/scoreboard-refresh-20261025/verify.py
python docs/specs/perf-vs-orca/evidence/scoreboard-refresh-20261025/verify.py --self-test
```

`verify.py` re-derives all 24 medians, the 16 wall-gap ratios, the group shapes
(1 warmup + 3 measured), the zero-starvation state, the PnP output flags and
the ordinary==accelerated bytes/SHA identity from the five committed row CSVs.
It never reads the raw capture tree. `--self-test` runs eight negative controls
(perturbed wall median, CPU median, gap ratio, dropped sample, starved sample,
degraded flag, byte count, SHA) and requires all eight to be detected.
