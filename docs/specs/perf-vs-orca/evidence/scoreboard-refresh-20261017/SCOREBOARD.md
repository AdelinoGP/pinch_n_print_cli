# Matched-pair scoreboard refresh — 2026-10-17/18

Ticket: [72 Matched-pair scoreboard refresh](../../issues/72-scoreboard-refresh.md).
Supersedes the 2026-09-22 numbers in
[`../matched-pair/SCOREBOARD.md`](../matched-pair/SCOREBOARD.md), which is kept
as history (do not edit). The rig is unchanged:
[`../matched-pair/run_scoreboard.ps1`](../matched-pair/run_scoreboard.ps1) and
its summarizer, so the two scoreboards are protocol-comparable.

## Protocol and environment

- Matrix: {benchy, base} × {classic, arachne} × {supports off, on} = 8 cells;
  three tools per cell — OrcaSlicer (vendor profile triple + generated
  string-typed overrides), PNP ordinary (`target/release/pnp_cli.exe` +
  `modules/core-modules`), PNP accelerated (complete
  `target/dist-accelerated/developer/` snapshot).
- 1 warmup + 3 measured runs per tool per cell (n=3 medians), rotating tool
  order in the interleaved batches; `RAYON_NUM_THREADS=12`.
- Per-run evidence validation ran on every sample (PNP: config appendix,
  gyroid dispatch, completion counters; Orca: resolved-config gcode appendix);
  a validation failure aborts the batch — no sample below is unvalidated.
- Starvation rule (PERF-HANDOFF §10.3): a sample is excluded when its cpu/wall
  ratio is < 0.75× its group's best ratio. **Zero samples were excluded in any
  group** (all 72 measured samples retained).
- HEAD: `f067efa2` (xtask runtime workspace root; no slicer-source change
  since the t70 keep `7127b4d6`); release host binary fingerprint-fresh
  (`cargo build --release -p pnp-cli` no-op); both guest freshness gates exit
  0 in every batch preflight; `module diagnose` pass=true, 24/24 modules,
  provenance `external` on both standing trees.
- Machine: same Windows dev box, 12 logical CPUs as t11. Ambient load was
  disclosed and non-trivial (CPU ≈ 22 % at launch, browser churn throughout);
  see the Orca-side drift note below. This is a busier machine than the t11
  capture, and ratios rather than absolute walls are the deliverable.

## Headline scoreboard (median wall seconds / median process CPU seconds)

| Cell | OrcaSlicer | PNP ordinary | PNP accelerated | Gap ord/Orca | Gap acc/Orca | acc/ord |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic supports-off | 4.27 / 12.06 | 21.96 / 118.48 | 18.52 / 97.66 | 5.14x | 4.34x | 0.844 |
| benchy arachne supports-off | 4.45 / 11.62 | 14.65 / 66.97 | 14.71 / 61.05 | 3.30x | 3.31x | 1.004 |
| benchy classic supports-on | 4.73 / 15.84 | 27.73 / 121.38 | 24.06 / 102.27 | 5.87x | 5.09x | 0.868 |
| benchy arachne supports-on | 4.38 / 16.17 | 20.87 / 74.94 | 20.30 / 73.09 | 4.76x | 4.63x | 0.973 |
| base classic supports-off | 18.84 / 38.75 | 216.28 / 1606.39 | 173.41 / 1182.64 | 11.48x | 9.20x | 0.802 |
| base arachne supports-off | 18.06 / 40.66 | 133.42 / 796.67 | 130.63 / 762.27 | 7.39x | 7.24x | 0.979 |
| base classic supports-on | 24.38 / 64.45 | 439.66 / 1986.59 | 408.35 / 1580.12 | 18.03x | 16.75x | 0.929 |
| base arachne supports-on | 23.78 / 67.94 | 372.16 / 1208.95 | 368.58 / 1174.75 | 15.65x | 15.50x | 0.990 |

**Verdict against the destination:** OrcaSlicer still wins all 8 cells on
median uninstrumented wall (3.30x–18.03x), and process CPU corroborates the
same ordering in every cell (Orca lowest, PNP 4.5x–41.5x Orca's CPU; the
ordinary/accelerated CPU ordering never contradicts the wall ordering). The
best cell is benchy arachne supports-off at 3.30x; **no cell is within ~1.5x
of Orca**, so the [Gap budget per cell](../../issues/12-gap-budget-per-cell.md)
below-fold park condition is re-derived and still binding.

## Change since the 2026-09-22 scoreboard (ordinary wall medians)

| Cell | 2026-09-22 | 2026-10-17 | PNP Δ | Orca Δ |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 20.83 | 21.96 | +5.4 % | +46.7 % |
| benchy arachne-off | 20.49 | 14.65 | −28.5 % | +45.0 % |
| benchy classic-on | 34.92 | 27.73 | −20.6 % | +26.5 % |
| benchy arachne-on | 31.92 | 20.87 | −34.6 % | +14.4 % |
| base classic-off | 251.34 | 216.28 | −13.9 % | +2.9 % |
| base arachne-off | 161.55 | 133.42 | −17.4 % | +1.5 % |
| base classic-on | 577.96 | 439.66 | −23.9 % | +10.7 % |
| base arachne-on | 460.71 | 372.16 | −19.2 % | +10.0 % |

Reading: PNP ordinary wall improved in 7 of 8 cells (−13.9 % to −34.6 %,
consistent with the t27/t58/t65/t67/t70 keeps); benchy classic-off is +5.4 %,
the one cell no wall-moving keep touched (inside load drift). OrcaSlicer also
measured slower than on 2026-09-22 in every cell — from +1.5 % (base
arachne-off) to +45–47 % on the benchy cells, where a fixed GUI-subsystem
startup dominates a 3–5 s run. **Part of the raw gap improvement is this
Orca-side drift, not PNP work.** The per-cell Orca sample spread is tight
within this take (benchy 4.03–5.33 s; base 17.76–24.78 s; no sample near the
one 36.0 s outlier seen in an evicted batch, excluded — see recovery), so the
today-vs-today ratios are internally sound; the t11-vs-today deltas should be
read with the machine-state caveat.

## Output disclosure and the DEV-174 taint

All 16 PNP tool groups report `degraded=false`, `non_fatal=0` in every
measured run; the four supports-on cells are **untainted for the first time**
in a scoreboard revision (the t14 repair is verified at the output level, not
assumed). PNP ordinary and accelerated G-code bytes are identical in all 8
cells; the per-cell bytes (ord = acc) are:

| Cell | PNP bytes | Orca bytes |
| --- | ---: | ---: |
| benchy classic-off | 4,332,737 | 3,890,392 |
| benchy arachne-off | 4,126,744 | 4,675,521 / 4,684,161 |
| benchy classic-on | 8,658,663 | 8,923,328 |
| benchy arachne-on | 8,452,736 | 9,724,749 / 9,728,109 |
| base classic-off | 22,428,863 | 20,636,232 / 20,648,047 / 20,648,136 |
| base arachne-off | 21,180,367 | 19,219,011 |
| base classic-on | 43,912,792 | 37,409,049 / 37,471,477 / 37,483,470 |
| base arachne-on | 42,664,440 | 35,976,856 |

Per-cell TYPE/FEATURE section counts for the median run of every group are in
[`SUMMARY.txt`](SUMMARY.txt); the full per-run stats JSONs stay in the
gitignored `target/matched-pair/` capture tree. Output is disclosed, not
compared as a parity target (the fairness contract).

## Interruptions and recovery (disclosed)

The campaign ran as nine batches because the host environment evicted two
long background runs (the map's known infrastructure hazard, as in t58/t66):

1. `refresh-20261017-benchy` — complete (48 rows: 4 cells × 3 tools ×
   1 warmup + 3 measured).
2. `refresh-20261017-base` — **evicted at 17:20** mid
   `base-arachne-off-pnp-accelerated-m3`. Complete `classic-off` rows kept;
   the partial `arachne-off` rows preserved verbatim as
   [`rows-refresh-20261017-base.interrupted.csv`](rows-refresh-20261017-base.interrupted.csv)
   and **excluded** from the ledger.
3. `refresh-20261017-base-rest` — **evicted at ≈ 00:13** mid
   `base-classic-on-pnp-accelerated-m2`. Complete `arachne-off` rows kept;
   the partial `classic-on` rows preserved as
   [`rows-refresh-20261017-base-rest.interrupted.csv`](rows-refresh-20261017-base-rest.interrupted.csv)
   and excluded.
4. Remaining cells run as per-tool chunks (orca/ordinary/accelerated separately,
   `refresh-20261018-*`): base classic-on and arachne-on. **Disclosed protocol
   deviation:** these two cells' tool arms were measured in separate windows
   rather than interleaved; each arm is internally stable (Orca spread ≤ 1.2 s
   absolute, PNP arm spread ≤ 2.6 % wall), so the per-tool medians are
   unaffected beyond the
   drift the sample spreads already show. No partial round was ever stitched
   into a final arm: every ledger arm is a complete 1+3 group from one batch.

No gate-failing cell was ever re-run (there are no keep/drop gates in a
measurement take; all re-runs were infrastructure recovery of interrupted
cell groups).

## Reproduction

[`REPRODUCE.md`](REPRODUCE.md) carries the exact commands. `verify.py` in this
directory re-derives every median and gap above from the committed row CSVs
alone (no raw tree) and exits 0; `python verify.py --self-test` demonstrates
the verifier rejects a perturbed expectation.
