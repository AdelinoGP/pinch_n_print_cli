# t156 reproduce: prepared-region memo across the fill sync

The candidate keeps `LayerArena`'s prepared-region memo alive across
`sync_perimeter_infill_areas_into_slice`, whose fill-polygon writes do not touch
any input `prepare_regions` reads. A modifier-footprint split still clears the
memo because it changes the region list and base polygons. The change is
host-only; no guest is rebuilt.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs under
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- `pwsh`, Python 3 (stdlib only), and `RAYON_NUM_THREADS=12`.

## 1. Baseline snapshot (pre-change tree)

Build and stage the ordinary host plus the module tree **before** editing:

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli

mkdir -p target/t156-baseline
cp -f target/release/pnp_cli.exe target/t156-baseline/pnp_cli-ordinary.exe
rm -rf target/t156-baseline/modules
cp -r modules/core-modules target/t156-baseline/modules
```

## 2. Candidate build (post-change tree)

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli

mkdir -p target/t156-candidate
cp -f target/release/pnp_cli.exe target/t156-candidate/pnp_cli-ordinary.exe
rm -rf target/t156-candidate/modules
cp -r modules/core-modules target/t156-candidate/modules

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t156-prepared-memo/run_t156.ps1 -Mode manifest
```

## 3. Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t156-prepared-memo/run_t156.ps1 \
  -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t156-prepared-memo/run_t156.ps1 \
  -Mode ab -Pairs 5 -BatchId ab
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t156-prepared-memo/run_t156.ps1 \
  -Mode ab -Pairs 3 -NullControl \
  -Cells base-classic-off,base-classic-on,base-arachne-off -BatchId null
```

`-Mode output` writes 16 rows (8 cells × 2 arms); both arms must emit t151's
active reference hash in every cell. The paired gate is **5 interleaved pairs
per cell** on a quiet box. The exploratory first 3-pair screen and its invalid
extension (started at **85 % CPU load**) are preserved as
`target/t156/ab-loaded/` plus [`reduced/loaded-window.json`](reduced/loaded-window.json),
but they are **not** timing evidence and are not reproduced as gate commands.

`-NullControl` runs the same baseline arm on both sides of three cells × three
pairs into `target/t156/null/rows.csv`; the reducer reports its measured band,
never smooths it.

The script records a CPU-load sample before and after each batch under
`target/t156/<batch>/load.json`.

## 4. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t156-prepared-memo/reduce_t156.py
python docs/specs/perf-vs-orca/evidence/t156-prepared-memo/verify.py
python docs/specs/perf-vs-orca/evidence/t156-prepared-memo/verify.py --self-test
```

The reducer reads the gitignored raw tree under `target/t156/`; `verify.py`
reads only `reduced/campaign.json` and re-derives every claim. The six
self-test controls must all be rejected.

## 5. Unit / contract falsifier

Apply the candidate patch first; on the current DROP tree the two tests are
absent. With the patch applied:

```bash
cargo test -p slicer-runtime --test contract region_preparation_sharing_tdd:: -- --nocapture
```

On the pre-change tree, `fill_only_sync_preserves_both_prepared_region_projections`
is red (`fill-only sync must preserve the ordinary derivation`). With the
candidate applied it passes, and
`modifier_footprint_sync_clears_both_prepared_region_projections` guards the
invalidation path. The pre-change red is preserved in
[`red-test.log`](red-test.log).

## 6. Candidate patch

[`candidate.patch`](candidate.patch) is the exact production + test diff that was
screened and then **reverted** on the DROP. The working tree is back at the
pre-change implementation.
