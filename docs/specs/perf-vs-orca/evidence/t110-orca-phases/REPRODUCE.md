# Reproduce — t110 Orca-side phase attribution

Everything below runs from a plain checkout with the OrcaSlicer install the map
already uses (`C:\Program Files\OrcaSlicer\orca-slicer.exe`, 2.5.0-beta) and the
vendor tree the repaired rig stages. No elevation, no guest rebuild, no cargo.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (the map's licence-encumbered fixtures;
  `tmp/base.stl` is 123,061,784 bytes, `tmp/3dbenchy.stl` 11,289,384 bytes).
- `target/orca-vendor/` — stage it with
  `docs/specs/perf-vs-orca/evidence/orca-rig-repair/prepare_orca_vendor.ps1`
  (the rig fails closed without it).
- Python 3 (measured with 3.14.4). Standard library only.

## 1. Capability census (Phase 0)

```bash
python docs/specs/perf-vs-orca/evidence/t110-orca-phases/probe_capabilities.py
```

Writes `capabilities.json`: the log-flush measurement (one benchy classic-off slice
at `--debug 4`, log tapped every 2 ms, markers grouped into 5 ms bursts) and the
host's verbatim ETW refusal text. The `--pipe` result is in `FINDINGS.md`; it needs
a named-pipe server and is not automated here.

## 2. Campaign (Phase 2)

```bash
python docs/specs/perf-vs-orca/evidence/t110-orca-phases/run_campaign.py \
  --repeats 3 \
  --out docs/specs/perf-vs-orca/evidence/t110-orca-phases/runs.csv \
  --rawdir target/t110-runs
```

18 rows: 6 `--info` controls (2 per model), 4 discarded warmups (one per cell), and
3 measured repeats x 4 classic cells, round-robin. Raw run dirs (G-code, CLI logs)
land in `target/t110-runs/` and are gitignored; only `runs.csv` is committed.

## 3. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t110-orca-phases/reduce.py
python docs/specs/perf-vs-orca/evidence/t110-orca-phases/verify.py
python docs/specs/perf-vs-orca/evidence/t110-orca-phases/verify.py --self-test
```

`reduce.py` writes `phase-split.json` (Orca medians joined to ticket 104's PnP phase
table). `verify.py` re-derives every committed number from `runs.csv` and
`../t104-phase-split/phase-split.json` and exits 0; `--self-test` mutates the inputs
in memory and requires all seven negative controls to be rejected.

## Phase definitions

| phase | how it is measured |
| --- | --- |
| `startup_import_ms` | median wall of `--info` on the same model with the same `--load-settings` / `--load-filaments` (parses and reports the mesh, no slicing). An **upper bound** on the slice path's startup + vendor + import. |
| `prephase_ms` | launch -> `plate_1.gcode.tmp` first appears (Orca creates it empty at export start, i.e. at the end of its prepass). |
| `prepass_ms` | `prephase_ms - startup_import_ms`. |
| `emit_ms` | `plate_1.gcode.tmp` -> `plate_1.gcode` rename (per-layer paths + G-code emission). |
| `tail_ms` | rename -> process exit. |

Cumulative process CPU is read at each boundary, so each phase has CPU seconds as
well as wall.
