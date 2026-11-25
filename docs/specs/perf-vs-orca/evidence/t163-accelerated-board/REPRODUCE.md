# t163 reproduce — three-tool eight-cell board (accelerated arm)

No production change: this take measures and records. The accelerated arm it
uses was restored by [t162](../t162-accelerated-host-verbatim-path/FINDINGS.md).

## Prerequisites

- `tmp/3dbenchy.stl` (the 2026-11-12 replacement) and `tmp/base.stl`: user-supplied,
  never committed.
- OrcaSlicer at `C:\Program Files\OrcaSlicer` (**2.5.1**, the t161 boundary) with
  the vendor tree materialized at `target/orca-vendor/`.
- The accelerated snapshot: `cargo xtask dist --accelerated --edition developer`
  → `target/dist-accelerated/developer/{pnp_cli.exe,modules}`.
- `pwsh`, `python` (stdlib only), a 12-logical-core box.

## 1. Freshness

```bash
cargo xtask build-guests --check                 # 0
cargo xtask build-guests --accelerated --check   # 0
```

The scoreboard driver re-runs both because the tool set names `pnp-accelerated`,
and fails closed.

## 2. The campaign

```bash
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy,base \
  -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary,pnp-accelerated \
  -Runs 3 -Warmups 1 -BatchId t163-accelerated-board"
```

8 cells × 3 tools × (1 warmup + 3 measured) = **96 data rows** in
`target/matched-pair/t163-accelerated-board/scoreboard-rows.csv`
(header + 96). Box load sampled at 20 s into `target/t163-load.csv`; the reduced
copy is `reduced/load-samples.csv`.

## 3. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t163-accelerated-board
cp ../../../../../target/t163-load.csv reduced/load-samples.csv   # once
python reduce_t163.py        # raw csv + reference.json -> reduced/summary.json
python verify.py             # re-derives every quoted number, exit 0
python verify.py --self-test # 14/14 mutations rejected, exit 0
```

`reduce_t163.py` needs the gitignored raw campaign tree; `verify.py` is
stdlib-only and reads only `reduced/summary.json` plus
`evidence/t151-skirt-hull-contour/reference.json`.

## 4. Restore

Nothing to restore: no production change. Both freshness gates exit 0.

## 5. The profile pair (section 7 of FINDINGS)

Attribution only — `--profile` meters wasmtime fuel and costs throughput, so no
wall number from it is a claim.

```bash
RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice --model tmp/base.stl   --output target/t163-profile/ord.gcode   --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json   --module-dir modules/core-modules --profile 2> target/t163-profile/ord.txt

RAYON_NUM_THREADS=12 target/dist-accelerated/developer/pnp_cli.exe slice --model tmp/base.stl   --output target/t163-profile/acc.gcode   --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json   --module-dir target/dist-accelerated/developer/modules --profile 2> target/t163-profile/acc.txt
```

Both write `9bb18f0e…`; the `profile_summary` JSON line in each stderr carries
the per-module fuel and the scope table used above.
