# Matched-pair scoreboard — first run (2026-09-22)

Ticket 11 ([Matched-pair rig and first scoreboard](../../issues/11-matched-pair-rig-and-scoreboard.md))
deliverable. Machine: Windows dev box, 12 logical CPUs. PNP ordinary =
`target/release/pnp_cli.exe` + `--module-dir modules/core-modules`; PNP
accelerated = complete `cargo xtask dist --accelerated` snapshot at
`target/dist-accelerated/developer/` (own `pnp_cli.exe` + own `modules/`).
Both guest trees passed their freshness gates (`build-guests --check` and
`--accelerated --check`, exit 0) before every batch.

## Headline scoreboard (median of measured runs; wall seconds / process CPU seconds)

| Cell | OrcaSlicer | PNP ordinary | PNP accelerated | Gap ord/Orca | Gap acc/Orca | acc/ord |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic supports-off | 2.91 / 9.67 (n=3) | 20.83 / 132.07 (n=2) | 21.06 / 114.38 (n=3) | 7.16x | 7.24x | 1.011 |
| benchy arachne supports-off | 3.07 / 10.80 (n=3) | 20.49 / 75.80 (n=3) | 19.27 / 73.94 (n=3) | 6.68x | 6.28x | 0.940 |
| benchy classic supports-on | 3.74 / 14.77 (n=3) | 34.92 / 157.98 (n=3) | 33.71 / 140.20 (n=3) | 9.33x | 9.01x | 0.966 |
| benchy arachne supports-on | 3.83 / 15.47 (n=3) | 31.92 / 102.25 (n=2) | 31.39 / 100.91 (n=3) | 8.34x | 8.21x | 0.983 |
| base classic supports-off | 18.31 / 38.47 (n=3) | 251.34 / 1921.81 (n=3) | 212.87 / 1520.23 (n=3) | 13.73x | 11.62x | 0.847 |
| base arachne supports-off | 17.79 / 40.78 (n=3) | 161.55 / 1017.41 (n=3) | 135.83 / 945.08 (n=2) | 9.08x | 7.63x | 0.841 |
| base classic supports-on | 22.02 / 63.88 (n=3) | 577.96 / 2412.16 (n=3) | 521.57 / 2018.83 (n=3) | 26.24x | 23.68x | 0.902 |
| base arachne supports-on | 21.62 / 64.92 (n=3) | 460.71 / 1479.81 (n=3) | 455.93 / 1435.78 (n=3) | 21.31x | 21.08x | 0.990 |

- **OrcaSlicer wins all 8 cells** on median uninstrumented wall clock, by
  6.3x–26.2x. Process CPU corroborates in every cell (same ordering, never
  contradicting): PNP burns 7–38x Orca's CPU.
- **Accelerated mode buys 0–16% wall** (0–1% on benchy and on both base
  supports-on cells; ~15% on base supports-off), far less than its −35.8% guest
  fuel (see [emit_walls premise falsified](../perf-emit-walls/FINDINGS.md)
  addendum). Acceleration does not change the gap to Orca materially.
- Wall spread (retained samples): benchy groups within ~6%, base groups within
  ~18% (`base/arachne-off/pnp-ordinary` 146.3–184.8 s is the widest).

## The matched job

Defined identically on both sides; every run validated against output evidence
(labels prove nothing):

| Setting | Value | Orca side | PNP side |
| --- | --- | --- | --- |
| nozzle | 0.4 mm | `Bambu Lab X1 Carbon 0.4 nozzle` machine profile | `nozzle_diameter` |
| layer height / first layer | 0.20 / 0.20 mm | `layer_height`, `initial_layer_print_height` | `layer_height`, `first_layer_height` |
| wall count | 2 | `wall_loops` | `wall_count` |
| sparse infill | 20%, gyroid | `sparse_infill_density`, `sparse_infill_pattern` | `sparse_infill_density`, `sparse_fill_holder: gyroid-infill` |
| wall generator | classic / arachne per cell | `wall_generator` | `wall_generator` |
| supports | off / on per cell; tree(auto) when on | `enable_support`, `support_type` | `enable_support`, `support_type` |

Orca side: vendor triple (`0.20mm Standard @BBL X1C` process, X1C 0.4 machine,
Generic PLA) plus a generated string-typed override profile — JSON **number**
values are rejected per key by Orca's profile loader (measured: a numeric
`wall_loops` override did not apply while a string one did; Orca logs
`invalid json type for wall_loops` to its startup `00000.log` and continues
with the inherited value). The effective settings are verified per run from the
`; key = value` config appendix Orca writes into its G-code.

PNP side: `configs/pnp-<generator>-supports-<state>.json`, verified per run by
`resources/perimeter-acceptance/validate_measurement.ps1` (config/gcode/marker/
claim-holder agreement) plus a gyroid dispatch check on the stderr event stream.

Uncontrolled by design (disclosed, not matched): speeds/accelerations, seam
policy, support internals/coordinates, bed size and placement (Orca X1C profile
is 200x200, PNP defaults 250x250), top/bottom surface patterns, line-width
resolution of `0` (Orca auto) vs 0.4 mm, skirt/brim details. Geometry parity is
out of scope for this map; output is disclosed evidence only.

## Protocol

- 1 warmup + 3 measured runs per tool per cell; measured order rotates per
  round (Latin rotation across `orca, pnp-ordinary, pnp-accelerated`) so no
  tool always leads or trails a round.
- Uninstrumented runs only (§3.2 trap). Wall = process creation-to-exit, CPU =
  kernel+user via `GetProcessTimes`, peak WS polled @100 ms — the
  `alloc-bench/run_bench.ps1` lineage (`run_scoreboard.ps1` keeps the
  measurement math identical).
- `RAYON_NUM_THREADS=12` for PNP; Orca defaults.
- Starvation rule (§10.3 trap): cpu/wall ratio quoted per sample; a sample is
  excluded when its ratio falls below 0.75x the group's best ratio. 3 of 72
  measured samples were excluded — each with CPU flat against its peers and
  wall inflated, the deschedule signature:
  - `base-arachne-off-pnp-accelerated-m1`: 450.10 s wall, ratio 2.19 (peers 7.04/6.87, 134–137 s)
  - `benchy-arachne-on-pnp-ordinary-m2`: 53.99 s wall, ratio 1.96 (peers 3.09/3.33, 30–33 s)
  - `benchy-classic-off-pnp-ordinary-m3`: 30.20 s wall, ratio 4.35 (peers 6.37/6.32, 20–21 s)
- Three groups therefore report medians over n=2 (marked in the headline).
  Nothing in the 6.3x–26.2x gap depends on those exclusions.

## Output disclosure (per run in the CSVs)

G-code bytes (measured runs) and per-run `;TYPE:` (PNP) / `; FEATURE:` (Orca)
section counts are recorded per run; rows below show byte ranges:

| Cell | Orca | PNP ordinary | PNP accelerated |
| --- | --- | --- | --- |
| benchy classic supports-off | 3,890,416–3,899,056 | 7,443,822–7,444,752 | 7,444,400–7,444,849 |
| benchy arachne supports-off | 4,680,789–4,681,269 | 4,705,596 | 4,705,596 |
| benchy classic supports-on | 8,923,327–8,929,087 | 15,002,025–15,002,793 | 15,001,914–15,002,608 |
| benchy arachne supports-on | 9,718,965 | 12,263,676 | 12,263,676 |
| base classic supports-off | 20,636,220–20,653,987 | 21,935,777–21,936,132 | 21,935,669–21,936,407 |
| base arachne supports-off | 19,218,998 | 20,994,802 | 20,994,802 |
| base classic supports-on | 37,409,049–37,471,566 | 41,185,789–41,186,111 | 41,185,826–41,186,111 |
| base arachne supports-on | 35,955,804–35,976,855 | 40,244,616 | 40,244,616 |

- Arachne PNP output is byte-identical between ordinary and accelerated in all
  4 arachne cells (mode exactness); classic cells differ only within the known
  same-binary run-to-run jitter (DEV-093 class).
- Benchy/ calicat-class output byte jitter (§3.6) is present in classic cells;
  Arachne cells were byte-stable across runs here.

## DEV-174 disclosure — base supports-on cells are TAINTED, not won or lost

Every base.stl supports-on PNP run (both modes, all 8 runs) completed
`degraded=true` with **172,181 non-fatal errors** (fatal=0) — the DEV-174 class
defect ([Support-correctness repair](../../issues/14-support-correctness-repair.md);
ledger row DEV-174 in `docs/DEVIATION_LOG.md`). Section counts show the work
skipping directly: PNP emits `Support=280` / `Support interface=55` sections
where Orca emits `Support=638–640` / `Support interface=224–225`, i.e. PNP's
supports-on slice does less support work than Orca's. Per the Q5 disqualify rule
the four supports-on cells are **tainted** until ticket 14 lands; their gap
numbers above are disclosed for context only. Benchy supports-on runs are clean
(degraded=false, 0 non-fatals) and are not tainted.

## Caveats and known disclosure gaps

- **PNP config appendix quirk:** `slicer-gcode`'s appendix prints static
  defaults for keys whose config name has no same-name overlay — `wall_loops`
  always prints `2` regardless of `wall_count`. At this matched job (2 walls)
  the display is coincidentally correct; the PNP wall count here is established
  by the committed config artifacts and the per-run dispatch evidence, not by
  the appendix. Filed as
  [Config appendix must reflect resolved settings](../../issues/26-config-appendix-resolved-settings.md).
- **Dispatch-evidence asymmetry:** PNP generator/infill dispatch is proven from
  per-module stderr events; Orca dispatch is proven from its resolved-config
  dump (no per-module event stream exists).
- **n=3** is thin (n=2 after exclusion in three groups). Unambiguous at gaps of
  6x+; any future close cell needs the acceptance protocol (n>=4, strict
  separation) before a win claim.
- Section counts are not comparable *across* tools (different emission
  granularity: Orca emits multiple `; FEATURE:` sections per layer per type);
  they are per-tool disclosure only. Within-tool TYPE instability (§10.8) is
  visible in `Inner wall` counts (e.g. 525–527).

## What this does not decide

No optimization is selected or authorized here — this is the route's evidence
base. Keep/drop of any candidate remains measured-then-human per the map.

## Reproduction

```powershell
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 -Fixtures benchy -Runs 3 -Warmups 1 -BatchId s1-benchy
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 -Fixtures base   -Runs 3 -Warmups 1 -BatchId s2-base
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/summarize_scoreboard.ps1' -BatchIds @('s1-benchy','s2-base')"
```

Raw rows (wall, CPU, cpu/wall ratio, peak WS, bytes, sha256, section counts,
completion counters per run): `results/s1-benchy.csv`, `results/s2-base.csv`.
Batches `s1-benchy` / `s2-base`, 96 runs (24 warmups + 72 measured), 2026-09-22.
