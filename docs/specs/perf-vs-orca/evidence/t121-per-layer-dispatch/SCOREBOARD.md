# t121 post-merge Orca scoreboard (complete)

Post-merge, post-fix scoreboard on the current tree, 2026-11-05/06. The
matched-pair rig ran **8 cells × 3 tools × (1 warmup + 3 measured runs)**:
OrcaSlicer 2.5 (vendor tree, `tree_strong` on supports-on cells per the t105
ruling) against PnP ordinary and PnP accelerated. Orca and PnP were interleaved
within each cell; every run was validated by the rig.

Batches: `t121-post-merge2` (six cells), `t121-cell-base-classic-on2`,
`t121-cell-base-arachne-on3`. Raw rows:
[`scoreboard.csv`](scoreboard.csv) (96 rows). Reducer output:
[`scoreboard.txt`](scoreboard.txt). Verifier:
[`verify_scoreboard.py`](verify_scoreboard.py) (`--self-test` rejects 6/6).

## Ratios (median wall, standing starvation rule)

| cell | Orca | PnP ordinary | PnP accelerated | ordinary / Orca | accelerated / Orca |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 1.92 s | 5.73 s | 5.54 s | **2.99x** | **2.89x** |
| benchy classic-on | 2.65 s | 10.64 s | 10.26 s | **4.01x** | **3.87x** |
| benchy arachne-off | 2.16 s | 6.01 s | 5.96 s | **2.78x** | **2.76x** |
| benchy arachne-on | 2.75 s | 11.26 s | 11.49 s | **4.10x** | **4.18x** |
| base classic-off | 16.47 s | 56.08 s | 54.48 s | **3.40x** | **3.31x** |
| base classic-on | 25.88 s | 132.24 s | 131.88 s | **5.11x** | **5.10x** |
| base arachne-off | 16.36 s | 55.43 s | 54.18 s | **3.39x** | **3.31x** |
| base arachne-on | 24.95 s | 124.36 s | 124.25 s | **4.98x** | **4.98x** |

- **PnP ordinary and accelerated G-code bytes are identical in all 8 cells**
  (`sha256_of_gcode` equal per cell); `degraded=false`, `non_fatal=0` on every
  PnP run. Orca's G-code SHA is not an identity: its appendix embeds timing.
- One accelerated sample on benchy arachne-on (wall 20.10 s, cpu/wall 2.30) is
  below 0.75x the group's best ratio and is excluded by the standing rule; the
  retained median is n=2. The sample remains in the CSV.
- Load: the campaign ran on the shared dev box with browser processes active;
  the rig's interleaved schedule and CPU/wall starvation rule are the control.
  This is not an idle-box claim.

## Reading

**PnP loses all eight cells by 2.78x–5.11x.** This is the first post-merge
scoreboard of record (the t103/t106 ratios predate the merge and t116); the
t121 fix moved PnP's side substantially, but the destination's 8/8 is not met.

The required cut to beat Orca is **64% (benchy arachne-off) to 80% (base
classic-on)** of today's PnP wall; a 1.5x margin needs **76%–87%**. The
per-layer/emit program remains the next map stage; the t111 prepass campaign
left base at roughly 2.86x–4.00x, and the measured scores here are consistent
with that floor being the remaining gap.
