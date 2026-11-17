# t158 reproduce: exact fast-path boolean predicate in `SliceRegionView::derive_needs_support`

The candidate replaces the unconditional
`!slicer_core::polygon_ops::intersection_ex(&self.polygons, &overhang.xy_footprint).is_empty()`
inside `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`)
with `expolygons_intersect_area_positive(a, b)`:

1. strict axis-aligned bounding-box prefilter (`a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3`);
2. strict-interior contour vertex witness in exact `i128` arithmetic (`point_in_ring` returning `1` on the outer contour and `-1` on every hole; boundary hits return `0` and are never accepted as a witness);
3. exact Clipper2 `intersection_ex(&a_sub, &b_sub)` fallback on the bbox-overlapping subset when no strict-interior vertex witness is found.

Because `slicer-sdk` is compiled into the 24 external core guests (`modules/core-modules/`), building the candidate refreshes the guest `.wasm` tree so `cargo xtask build-guests --check` stays green (guests never invoke `derive_needs_support`, which is called only on the host in `commit_support_analysis_builtin` and `prepare_regions`).

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs under
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- `pwsh`, Python 3 (stdlib only), and `RAYON_NUM_THREADS=12`.

## 1. Baseline snapshot (pre-change tree)

Build and stage the ordinary host plus the module tree **before** applying `candidate.patch`:

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli

mkdir -p target/t158-baseline
cp -f target/release/pnp_cli.exe target/t158-baseline/pnp_cli-ordinary.exe
rm -rf target/t158-baseline/modules
cp -r modules/core-modules target/t158-baseline/modules
```

## 2. Candidate build (post-change tree)

Apply [`candidate.patch`](candidate.patch), rebuild the guest modules and the release CLI, and stage `target/t158-candidate/`:

```bash
git apply docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/candidate.patch
cargo xtask build-guests
cargo xtask build-guests --check
cargo build --release -p pnp-cli

mkdir -p target/t158-candidate
cp -f target/release/pnp_cli.exe target/t158-candidate/pnp_cli-ordinary.exe
rm -rf target/t158-candidate/modules
cp -r modules/core-modules target/t158-candidate/modules

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/run_t158.ps1 -Mode manifest
```

## 3. Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/run_t158.ps1 \
  -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/run_t158.ps1 \
  -Mode ab -Pairs 5 -BatchId ab
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/run_t158.ps1 \
  -Mode ab -Pairs 3 -NullControl \
  -Cells base-classic-off,base-classic-on,base-arachne-off -BatchId null
```

`-Mode output` writes 16 rows (8 cells × 2 arms); both arms must emit t151's
active reference hash in every cell. The paired gate is **5 interleaved pairs
per cell** (after the initial 3-pair screen showed sub-floor separate-median noise
on `benchy`, 2 extra pairs were pre-declared for all 8 cells before running pairs
4–5; all 5 pairs enter the median). `-NullControl` runs the same baseline arm on
both sides of three representative base cells × three pairs into
`target/t158/ab/null/rows.csv`.

## 4. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/reduce_t158.py
python docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/verify.py
python docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/verify.py --self-test
```

The reducer reads the gitignored raw tree under `target/t158/`; `verify.py`
reads only `reduced/campaign.json` and re-derives every campaign claim. All six
self-test negative controls must be rejected. Its PASS line intentionally reports
`verdict=DROP`: that is the original mechanical gate result, preserved after the
human's later KEEP ruling in [`RULING.md`](RULING.md).

Re-derive the disclosed cross-window route composition with:

```bash
python docs/specs/perf-vs-orca/evidence/t158-needs-support-fast-predicate/route_arithmetic.py
```

This writes [`route_arithmetic.json`](route_arithmetic.json); it does not run a
fresh Orca scoreboard.

## 5. Unit tests

With [`candidate.patch`](candidate.patch) applied:

```bash
cargo test -p slicer-sdk --test layer_module_tdd 2>&1 | tee target/test-output.log
```

## 6. Candidate patch

[`candidate.patch`](candidate.patch) is the exact production + test diff that was
screened, mechanically dropped and reverted, then **restored and landed** under
the human's 2026-11-17 KEEP ruling. To reproduce the campaign from a later tree,
first restore its pre-change implementation rather than applying the patch on top
of the landed candidate.
