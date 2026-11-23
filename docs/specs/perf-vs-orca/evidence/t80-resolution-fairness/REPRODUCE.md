# Reproducing the t80 resolution-fairness take

Measurement-only. Two OrcaSlicer arms, identical in every job key, differing
only in `resolution`:

| arm | effective `resolution` | meaning |
| --- | --- | --- |
| `default` | 0.01 mm (inherited from the vendor process profile) | what the matched job has always run |
| `floor` | 0.001 mm (explicit) | OrcaSlicer's canonical minimum — `DynamicPrintConfig::normalize_fdm` clamps anything below 0.001 up to 0.001, so "disabled" is not expressible |

A positive delta (`floor` slower than `default`) is what Orca gains from its
slicing-time contour simplification. PnP implements no equivalent pass.

## Prerequisites

- `C:\Program Files\OrcaSlicer\` with the BBL vendor profile triple
  (`0.20mm Standard @BBL X1C` process, `X1 Carbon 0.4 nozzle` machine,
  `Generic PLA` filament).
- `tmp/3dbenchy.stl`, `tmp/base.stl` (user-supplied, never committed).
- PowerShell 7. No PnP build, guest artifact or freshness gate is involved: this
  take runs OrcaSlicer only.

## Run

One invocation per fixture (a single invocation takes one fixture), sequencing
the two arms alternately so no arm always leads:

```powershell
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/t80-resolution-fairness/run_resolution_pair.ps1' -Fixture benchy -Cells 'classic-off','classic-on','arachne-off','arachne-on' -Pairs 3 -Warmups 1 -BatchId classic-20261022-benchy"
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/t80-resolution-fairness/run_resolution_pair.ps1' -Fixture base -Cells 'classic-off','classic-on','arachne-off','arachne-on' -Pairs 3 -Warmups 1 -BatchId classic-20261022-base"
```

The harness derives both profiles from the vendor process JSON (so the job keys
cannot drift from `matched-pair/configs/`), runs
`orca-slicer.exe --slice 0 --load-settings <machine>;<derived> --load-filaments <filament>`,
and measures wall as process creation-to-exit and CPU as `GetProcessTimes`
kernel+user — the same math as `matched-pair/run_scoreboard.ps1`.

**Fail-closed validity gate:** every run's effective `resolution` is read back
from the `; resolution = ` line of Orca's own G-code config appendix and must
equal the arm's expected value, alongside the matched job keys
(`wall_generator`, `wall_loops`, `sparse_infill_density`, `sparse_infill_pattern`,
`enable_support`, `support_type`, `layer_height`). A profile override that
silently failed to apply is rejected, not recorded as "no effect". This gate is
what caught the 0.001 clamp.

## Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t80-resolution-fairness/reduce.py \
  target/t80-resolution/classic-20261022-benchy/rows.csv \
  target/t80-resolution/classic-20261022-base/rows.csv
python docs/specs/perf-vs-orca/evidence/t80-resolution-fairness/verify.py
python docs/specs/perf-vs-orca/evidence/t80-resolution-fairness/verify.py --self-test
```

`reduce.py` writes the committed `rows.csv` and `summary.json`; `verify.py`
re-derives every median and paired delta from `rows.csv` alone and rejects five
negative controls.

Raw G-code, Orca logs and per-run trees stay under `target/t80-resolution/`
(gitignored) and are regenerable with the harness.
