# t121 post-merge scoreboard — PAUSED at 6/8 cells

**This is a partial campaign, not a scoreboard of record.** The matched-pair
rig started at 2026-11-05 20:46 and was paused at ~21:16 when the session
restarted. Batch `t121-post-merge2` holds **76 data rows**: six complete cells
plus the warmups and one Orca measured run of `base classic-on`.

Raw rows: [`scoreboard-partial.csv`](scoreboard-partial.csv). Reducer output:
[`scoreboard-partial.txt`](scoreboard-partial.txt). The standing reducer is
`docs/specs/perf-vs-orca/evidence/matched-pair/summarize_scoreboard.ps1`.

## Ratios (median wall, standing starvation rule) — partial

| cell | Orca | PnP ordinary | PnP accelerated | ordinary / Orca | accelerated / Orca |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 1.92 s | 5.73 s | 5.54 s | **2.99x** | **2.89x** |
| benchy classic-on | 2.65 s | 10.64 s | 10.26 s | **4.01x** | **3.87x** |
| benchy arachne-off | 2.16 s | 6.01 s | 5.96 s | **2.78x** | **2.76x** |
| benchy arachne-on | 2.75 s | 11.26 s | 11.49 s | **4.10x** | **4.18x** |
| base classic-off | 16.47 s | 56.08 s | 54.48 s | **3.40x** | **3.31x** |
| base arachne-off | 16.36 s | 55.43 s | 54.18 s | **3.39x** | **3.31x** |

- One accelerated sample on benchy arachne-on (wall 20.10 s, cpu/wall 2.30) is
  below 0.75x the group's best ratio and is excluded by the standing rule;
  the retained median is n=2. The sample is still in the CSV.
- PnP ordinary and accelerated G-code bytes are identical within every
  complete cell; `degraded=false`, `non_fatal=0` on every PnP run.
- Orca is pinned to `tree_strong` on the supports-on cells (t105 ruling); the
  rig asserts the resolved appendix per row.

## Missing cells

- `base classic-on` — only Orca warmup/m1 and PnP warmups were recorded.
- `base arachne-on` — not started.

## Resume

The resumed per-cell batches were stopped before any measured row:
`target/matched-pair/t121-cell-base-classic-on/scoreboard-rows.csv` holds only
its header; `t121-cell-base-arachne-on` was never created. Run each cell as its
own batch, then reduce with the two new batch ids and drop the partial
`base classic-on` rows from the combined view:

```powershell
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' -Fixtures @('base') -Cells @('classic-on') -Tools @('orca','pnp-ordinary','pnp-accelerated') -Runs 3 -Warmups 1 -BatchId 't121-cell-base-classic-on' -KeepGcode 'None'"
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' -Fixtures @('base') -Cells @('arachne-on') -Tools @('orca','pnp-ordinary','pnp-accelerated') -Runs 3 -Warmups 1 -BatchId 't121-cell-base-arachne-on' -KeepGcode 'None'"
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/summarize_scoreboard.ps1 -BatchIds t121-post-merge2,t121-cell-base-classic-on,t121-cell-base-arachne-on
```

The destination (8/8 cells won) is not reached: PnP loses every complete cell
by **2.78x–4.10x**. This partial table is a current-tree screen, not a ratio of
record; no map decision should quote it as the post-merge scoreboard.

## Conditions

The campaign ran on the same box as the t121 A/B, with browser processes active;
the rig records process wall/CPU and the starvation rule, but the host was not
independently sampled as idle. Orca and PnP ran interleaved within each cell.
