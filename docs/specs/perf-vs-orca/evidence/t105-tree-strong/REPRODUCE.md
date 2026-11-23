# t105 reproduction — Orca `tree_strong` vs organic on the four supports-on cells

Measurement-only take. Nothing here changes the PnP side, the matched configs,
or production code.

## Prerequisites

- The installed OrcaSlicer **2.5.0-beta** at
  `C:\Program Files\OrcaSlicer\orca-slicer.exe` (the repaired-rig install).
- The source-manifest vendor tree at `target/orca-vendor/`, materialized by
  `../orca-rig-repair/prepare_orca_vendor.ps1` and version-asserted by the rig's
  preflight. The rig fails closed without it.
- The heavy fixtures `tmp/3dbenchy.stl` and `tmp/base.stl` (gitignored,
  user-supplied).
- PowerShell 7 (`pwsh`) and Python 3.

## Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t105-tree-strong/run_campaign.ps1
```

The runner:

1. records the Orca binary hash, the vendor manifest path and the start time in
   `provenance.json`, and the `cargo xtask build-guests --check` exit code in
   `freshness.txt` (once — the PnP side is not measured here);
2. runs 1 warmup per arm per cell (the t98 cold-vendor-tree trap);
3. runs 4 measured rounds per cell, alternating the arm order by round, one rig
   batch per run (`target/matched-pair/t105-<arm>-r<k>-<fixture>-<cell>`), so the
   two arms interleave inside each cell;
4. records `\Processor(_Total)\% Processor Time` before, between and after the
   rounds in `load.txt`.

Raw batch dirs are gitignored under `target/matched-pair/`.

## Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t105-tree-strong/reduce.py
python docs/specs/perf-vs-orca/evidence/t105-tree-strong/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t105-tree-strong/verify.py
```

`reduce.py` writes the committed `runs.csv` (every measured and warmup row, with
the resolved `support_style`) and `summary.json` (medians, paired per-round
deltas, ratios against ticket 103's PnP ordinary medians, output disclosure).
`verify.py` re-derives every claimed number from those two files plus ticket
103's `summary.json`; it never reads the raw tree. `--self-test` mutates six
values and demands six rejections.

## Rig extension this take carries

`run_scoreboard.ps1` gained `-OrcaSupportStyle` (the pin `tree_strong` is the
standing default; `vendor` restores the vendor default for a disclosed organic
arm), `support_style` in the disclosed appendix excerpt, and a fail-closed
assertion that each row resolved the expected style. A candidate arm therefore
proves which style it ran instead of silently inheriting a version default.
