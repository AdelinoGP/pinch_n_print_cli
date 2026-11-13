# t145 reproduce — benchy fixture re-baseline

This take edits no production code. It records the benchy reference currency,
classifies the fixture swap, and re-derives the 8-cell scoreboard for the
ordinary arm.

## Prerequisites

- `tmp/3dbenchy.stl` (the replacement export, sha256 `6ab57f1c…`, 11,285,384
  bytes) and `tmp/base.stl` (sha256 `b26c8c79…`). Both user-supplied, never
  committed.
- The matched-pair configs at `evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- OrcaSlicer installed at `C:\Program Files\OrcaSlicer`, and the vendor tree
  materialized once:
  `pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/orca-rig-repair/prepare_orca_vendor.ps1`
  (produces `target/orca-vendor/`).
- Rust toolchain as pinned by the workspace; `pwsh`; `python` (stdlib only for
  the scoreboard half; `trimesh` + `shapely` only to re-cut the STL
  cross-sections).
- A 12-logical-core box (`RAYON_NUM_THREADS=12`); the quoted spreads assume it.

## 1. Freshness and build

```bash
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated --check      # exit 0
cargo build --release -p pnp-cli                    # target/release/pnp_cli.exe
```

The scoreboard driver's own preflight re-runs both freshness checks and
`pnp_cli module diagnose` on the ordinary arm, and fails closed.

## 2. The campaign

One batch, two invocations because `-Cells`/`-Tools` bind as arrays only under
`-Command` (from `pwsh -File` a comma string arrives as one value and fails the
`ValidateSet`):

```bash
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary -Runs 3 -Warmups 1 -BatchId t145-benchy"

pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures base -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary -Runs 3 -Warmups 1 -BatchId t145-benchy"
```

The second invocation appends to the same crash-safe CSV (64 rows total). The
driver asserts the matched keys from each arm's own resolved-config appendix and
aborts the batch on any mismatch; it also validates every PnP run through
`resources/perimeter-acceptance/validate_measurement.ps1` and every Orca run
against its config appendix.

## 3. The accelerated attempt (expected to fail)

```bash
cargo xtask dist --accelerated --edition developer
```

Exit 1 at the host link: `c1xx: fatal error C1083: Cannot open source file:
'\\mimalloc-static.cc'`. This is the documented t142 failure; no manual-cfg host
is substituted because `docs/23_controlled_perimeter_builds.md` forbids granting
the acceleration cfg from an ordinary invocation.

## 4. The swap audit (raw sources)

The reducer reads the gitignored raw tree:

```
target/t142/output1/gcode/*-baseline-ordinary.gcode   old export, t142 tree
target/t143/output/gcode/*-baseline-ordinary.gcode    new export, t143 baseline
tmp/3dbenchy.stl, tmp/base.stl
docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/reduced/summary.json
```

The two takes are output-equivalent on the unchanged fixture: all four base
cells are byte-identical across them (`ca14d055…`, `585c6a8e…`, `d8fdc1d9…`,
`c9fee59a…`). That is the control the classification rests on.

## 5. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t145-benchy-rebaseline
python reduce_t145.py        # writes reduced/summary.json + reference.json
python verify.py             # 98/98 checks, exit 0
python verify.py --self-test # 25/25 negative controls rejected, exit 0
```

`reduce_t145.py` needs `trimesh` + `shapely` for the cross-section re-cut;
`verify.py` is stdlib-only and reads only the two in-repo artifacts, so the
claims are checkable from a plain checkout without the raw tree.

## 6. Restore

Nothing to restore: no production change. Both freshness gates still exit 0.
