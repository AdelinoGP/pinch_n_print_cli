# t106 reproduction — post-pin supports-on Orca scoreboard

Measurement-only take. Nothing here changes the PnP side, the matched configs,
or production code. The PnP denominators are ticket 103's, unchanged: a style
pin on Orca cannot move a PnP median.

## Prerequisites

- The installed OrcaSlicer **2.5.0-beta** at
  `C:\Program Files\OrcaSlicer\orca-slicer.exe` (the repaired-rig install).
- The source-manifest vendor tree at `target/orca-vendor/`, materialized by
  `../orca-rig-repair/prepare_orca_vendor.ps1` and version-asserted by the rig's
  preflight. The rig fails closed without it.
- The heavy fixtures `tmp/3dbenchy.stl` and `tmp/base.stl` (gitignored,
  user-supplied).
- PowerShell 7 (`pwsh`) and Python 3.
- A **quiet machine**: this take becomes the scoreboard of record, so a cell
  measured under a game or a compile is not a usable row.

## Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t106-post-pin-scoreboard/run_campaign.ps1
```

The runner:

1. records the Orca binary hash, the pinned `support_style`, the vendor manifest
   path and the start time in `provenance.json`, and the
   `cargo xtask build-guests --check` exit code in `freshness.txt` (once — the
   PnP side is not measured here);
2. runs 1 warmup per cell (the t98 cold-vendor-tree trap);
3. runs 3 measured rounds per cell (ticket 103's schedule, so the pre-pin and
   post-pin medians rest on the same number of samples), one rig batch per run
   (`target/matched-pair/t106-r<k>-<fixture>-<cell>`);
4. records `\Processor(_Total)\% Processor Time` before and after each round in
   `load.txt`.

The rig's standing default (ticket 105 ruling) pins `support_style =
tree_strong` on supports-on runs and asserts the resolved appendix per row, so a
row that resolved anything else aborts its batch. The campaign passes the pin
explicitly as well, so the record names the style it measured.

Raw batch dirs are gitignored under `target/matched-pair/`.

## Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t106-post-pin-scoreboard/reduce.py
python docs/specs/perf-vs-orca/evidence/t106-post-pin-scoreboard/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t106-post-pin-scoreboard/verify.py
```

`reduce.py` writes the committed `runs.csv` (every measured and warmup row, with
the resolved `support_style` and `enable_support`) and `summary.json` (medians,
spread, ticket-103 PnP ratios, cut-to-park, the pre-pin ticket-103 Orca
comparison, and the ticket-105 transfer check). `verify.py` re-derives every
claimed number from those two files plus ticket 103's `summary.json` and its
committed `rows-*.csv`; `--self-test` runs nine negative controls.

## Pre-registered transfer-agreement rule

The question this take answers is whether the **measured** post-pin table agrees
with ticket 105's **derived** transfer (`t103 Orca wall × (1 + t105 style delta
%)`). The tolerance is fixed before measuring:

> A cell's transfer **agrees** when
> `|measured median − predicted median| ≤ max(t106 measured range, t103 measured
> range)`, where a range is `max − min` over that take's own measured samples
> (starvation rule applied to t106; ticket 103's retained samples as committed).

The rule uses only in-repo data, and a disagreement is **disclosed**, never
resolved by substituting one number for the other or by re-measuring until the
two line up.
