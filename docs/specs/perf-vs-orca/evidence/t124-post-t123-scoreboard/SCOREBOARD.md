# t124 post-t123 Orca scoreboard

The fresh matched-cell scoreboard after ticket 123's bridge-classification
landing, 2026-11-07. The standing matched-pair rig ran
**8 cells × 3 tools × (1 warmup + 3 measured)** — OrcaSlicer 2.5 (version-gated
vendor tree, `tree_strong` on supports-on cells per the t105 ruling) against
PnP ordinary and PnP accelerated — with Orca and PnP interleaved inside each
cell.

Batches: `t124-post-t123` (six cells), `t124-resume-base-classic-on`,
`t124-resume-base-arachne-on`. The first batch was killed by the host at
13:14 local after 79 runs; its partial `base classic-on` cell
(`orca` 1+1, `pnp-ordinary` 1+2, `pnp-accelerated` 1+1) is **discarded whole**
and named in [`scoreboard-summary.json`](scoreboard-summary.json)'s
`discarded_partial_sources`; the cell was re-taken in its own interleaved
batch. Raw rows: [`scoreboard.csv`](scoreboard.csv) (96 rows). Reducer output:
[`scoreboard.txt`](scoreboard.txt). Verifier: [`verify.py`](verify.py)
(`--self-test` rejects 10/10 negative controls, exit 0 checkout-only).

## Ratios (median wall, standing starvation rule)

| cell | Orca | PnP ordinary | PnP accelerated | ordinary / Orca | accelerated / Orca |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.14 s | 5.77 s | 5.26 s | **2.70x** | **2.46x** |
| benchy classic-on | 2.77 s | 10.01 s | 9.74 s | **3.61x** | **3.52x** |
| benchy arachne-off | 2.14 s | 5.34 s | 5.37 s | **2.50x** | **2.52x** |
| benchy arachne-on | 2.77 s | 9.96 s | 9.61 s | **3.59x** | **3.47x** |
| base classic-off | 14.44 s | 42.53 s | 41.06 s | **2.95x** | **2.84x** |
| base classic-on | 27.28 s | 126.27 s | 121.31 s | **4.63x** | **4.45x** |
| base arachne-off | 17.45 s | 41.18 s | 41.90 s | **2.36x** | **2.40x** |
| base arachne-on | 26.24 s | 122.10 s | 122.50 s | **4.65x** | **4.67x** |

- **PnP loses all eight cells.** Ordinary/Orca range **2.36x–4.65x**;
  accelerated/Orca **2.40x–4.67x**. Best cell: base arachne-off (2.36x);
  worst: base arachne-on (4.65x).
- **No starvation exclusions anywhere**: all 24 `(cell, tool)` groups retain
  n=3 (`excluded=0`).
- **Required cut to beat Orca: 58% (base arachne-off) to 79% (base arachne-on)**
  of today's PnP wall. A 1.5x margin needs **72%–86%**.
- Process CPU (median, retained samples): PnP ordinary is **4.3x–6.5x** Orca's
  (e.g. base classic-on 643.0 s vs 99.3 s), which is the 12-Rayon-thread PnP
  engine against Orca's narrower process; it corroborates the wall ordering
  (PnP is slower on both) and never contradicts it.

## Proof gates

- **Roster:** 96 rows = 8 cells × 3 tools × (1 warmup + 3 measured).
- **PnP output identity:** ordinary ≡ accelerated G-code bytes in **8/8**
  cells; every measured sample within an arm is byte-identical.
- **PnP provenance:** every cell's PnP SHA-256 equals the t123 candidate set
  (`../t123-facet-retirement/reduced/summary.json`), i.e. the PnP side measured
  here is the post-t123 tree. The arms' binaries are the t123 candidate
  artifacts (`7737a74f…` ordinary, `a6ae2adc…` accelerated).
- **PnP health:** 64/64 PnP runs (warmups included) `degraded=false`,
  `non_fatal=0`, `completion_status=ok`; PnP resolves `brim_width = 8` in every
  run.
- **Orca resolved-config pins**, asserted by the rig and re-checked from the
  committed CSV on 32/32 runs: `enable_arc_fitting = disabled`,
  `elefant_foot_compensation = 0`, `brim_width = 0`, and the repaired filament
  leg `filament_density = 1.24`, `filament_max_volumetric_speed = 12`,
  `first_layer_temperature = 220` — never the t98 defect signature
  (`0 / 2 / 200`).
- **Guest freshness:** `cargo xtask build-guests --check` and
  `--accelerated --check` both exit 0.

## Reading

**This take is not a before/after claim against t121.** The arms are
today-vs-today and interleaved inside each cell, which is what makes each
ratio valid; the t123 causal measurement is its own paired A/B (median wall
−2.73% … −22.23%). Against t121's scoreboard the PnP ordinary medians moved
**−25.7% … +0.7%** (base classic-off −24.2% and base arachne-off −25.7%,
consistent with t123's paired −22.2% / −20.9%; the two benchy-on cells and the
two base-on cells are noisier), but the **Orca side also moved**
(−12.3% … +11.5%: benchy +11.5/+4.5/−0.9/+0.7%, base
−12.3/+5.4/+6.7/+5.2%, in cell order), so the ratio change mixes both arms.
The ratio of record is this table's.

- **The destination is not met:** 0 of 8 cells won, against the map's 8/8
  requirement. The required cut is **still 58%–79%** of the current PnP wall.
- **Adhesion asymmetry (standing disclosure):** PnP prints its 8 mm
  skirt-brim default in every cell while pinned-back Orca prints none
  (`brim_width = 0`). The job is not identical and closing it is a separate
  human decision; its slice-time price was measured at ~235 ms (t103).
- **Batch split:** the host killed the first batch mid-cell; the two remaining
  cells were taken in separate windows. Each cell is internally interleaved,
  so no ratio spans windows. The discarded partial source is recorded, not
  averaged.
- **Load:** the campaign ran on the shared dev box with the owner's browser
  processes active (pre-campaign load sample 13%); the interleaved schedule and
  the CPU/wall starvation rule are the control. This is not an idle-box claim.
- **The t105 support-style pin** is asserted per Orca run by the rig
  (`support_style = tree_strong` on supports-on, `default` on supports-off);
  the rig aborts on a mismatch. The CSV does not carry a `support_style`
  column, so the committed rows cannot re-check it — the run-time assertion is
  the gate.

## Evidence index

- `scoreboard.csv` — merged 96 rows from the three batches.
- `scoreboard-summary.json` — reduced medians, retained counts, per-cell ratios
  and the discarded partial source.
- `scoreboard.txt` — per-sample wall/CPU/ratio with starvation flags and gap
  table.
- `reduce_t124.py` — CSV(s) → summary/scoreboard.txt; a cell must be complete
  in exactly one batch.
- `verify.py` — re-derives every gate above from the committed CSV plus the
  t123 reference set; `--self-test` rejects 10/10 negative controls.
