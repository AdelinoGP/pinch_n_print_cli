# t148 reproduce — supports-on plane carve

This take changes production code: the host publishes the print-plane occupancy
the exact-Z gate validates, and the tree planner carves emitted roles against it.
It then re-slices the four supports-on cells, checks identity on the four
supports-off cells, and re-derives the matched-pair ratios.

## Prerequisites

- `tmp/3dbenchy.stl` (sha256 `6ab57f1c…`) and `tmp/base.stl` (sha256
  `b26c8c79…`). Both user-supplied, never committed.
- The matched-pair configs at `evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- OrcaSlicer installed at `C:\Program Files\OrcaSlicer`, vendor tree materialized
  once (`evidence/orca-rig-repair/prepare_orca_vendor.ps1`).
- Rust toolchain as pinned by the workspace; `pwsh`; `python` (stdlib for
  `verify.py`, `shapely` for `reduce_t148.py`).
- A 12-logical-core box (`RAYON_NUM_THREADS=12`).

## 1. Freshness and build

```bash
cargo xtask build-guests --check     # exit 0
cargo build --release -p pnp-cli     # target/release/pnp_cli.exe
```

The WIT carrier is a package bump (`slicer:prepass-support-geometry@2.1.0`), so a
stale guest tree fails instantiation; the freshness gate is not optional.

## 2. The four supports-on cells

```bash
for fixture in 3dbenchy base; do
  name=${fixture/3dbenchy/benchy}
  for gen in classic arachne; do
    RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
      --model tmp/$fixture.stl \
      --output target/t148/$name-$gen-on.gcode \
      --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-$gen-supports-on.json \
      --module-dir modules/core-modules \
      2> target/t148/$name-$gen-on.stderr.log
  done
done
```

Expected: exit 0, **zero** `exact-Z occupancy` lines in each stderr, and
`degraded: false`.

## 3. The four supports-off identity cells

Same command with `supports-off.json` and `-off.gcode`. Expected hashes (the t144
active set):

```text
benchy classic-off  1ae0cfa84d1b5f59893a2c30ab6ca4afe0bb73ad68249cff2ea8ddc5f09c8c5f
benchy arachne-off  8faf02b1b473405c3b24f261ab4e9daa630701b3d0caa78de70468c93453a7cb
base   classic-off  c0b9014f8eff1daee38b4f9f9bf3e1b66c4ef84e75d8b741e46f468dc37fe0ad
base   arachne-off  17f63e9387bd8d548f7d9df0c4d840742dfe019c22eae4a9e314105bad38e191
```

## 4. The matched-pair campaign (supports-on only)

Two invocations because `-Cells`/`-Tools` bind as arrays only under `-Command`:

```bash
pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy -Cells classic-on,arachne-on -Tools orca,pnp-ordinary \
  -Runs 3 -Warmups 1 -BatchId t148-plane-carve"

pwsh -NoProfile -Command "& 'docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures base -Cells classic-on,arachne-on -Tools orca,pnp-ordinary \
  -Runs 3 -Warmups 1 -BatchId t148-plane-carve"
```

## 5. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t148-print-plane-carve
python reduce_t148.py      # writes census.json from target/t148 + the t148 campaign
python verify.py           # 360/360 checks
python verify.py --self-test  # 9/9 negative controls rejected
```

`reduce_t148.py` also reads t147's in-repo census and the gitignored t147 probe
candidate geometry (`target/t147/probe-*/candidates.json`) for the coverage
reading; `verify.py` is stdlib-only and reads only `census.json`.

## 6. Restore

Nothing to restore: the change is the deliverable. Both freshness gates still
exit 0, and `cargo clippy --workspace --all-targets -- -D warnings` is clean.
