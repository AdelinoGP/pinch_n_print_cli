# t151 reproduce — skirt-hull brim contour fold

Candidate: `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`)
folds the **brim area's outer contour** into the skirt hull instead of the
emitted brim beads' centreline. Canonical `Print::_make_skirt` appends
`m_objectBrimAreasByInstance[...]` `area.contour.points` to the group's
`occupied_points`, and `make_brim` (`Brim.cpp`) fills that map with the brim
areas after the no-brim subtraction; PnP was folding `path.points` of the
emitted rings, which sit half a flow spacing (0.1786 mm at the matched job)
inside that contour. No host source changes, so all arms share one host binary
and only the `skirt-brim` guest differs.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- A 12-logical-core box (`RAYON_NUM_THREADS=12`); the quoted null bands assume it.
- `pwsh` for the campaign drivers, `python` (stdlib only) for the reducer and
  verifier.
- OrcaSlicer installed at `C:\Program Files\OrcaSlicer` plus network access for
  `evidence/orca-rig-repair/prepare_orca_vendor.ps1` (the oracle's Orca arm).

## 1. Builds and freshness

```bash
cargo test -p skirt-brim
cargo xtask build-guests
cargo build --release -p pnp-cli
cargo xtask build-guests --check                    # exit 0
cargo xtask build-guests --accelerated              # accelerated guest tree
cargo xtask build-guests --accelerated --check      # exit 0
```

`cargo xtask dist --accelerated --edition developer` is not usable on this box
(the controlled driver's host link fails in `libmimalloc-sys`, the known
verbatim-path / MSVC 14.51 failure, t142). The accelerated **output** arm is
therefore staged as the ordinary host plus the accelerated guest tree; no
accelerated timing claim is made.

## 2. Arms

```bash
mkdir -p target/t151-baseline target/t151-candidate
cp target/release/pnp_cli.exe target/t151-baseline/pnp_cli-ordinary.exe

# baseline: the pre-change module tree, copied BEFORE `cargo xtask build-guests`
# rebuilds `modules/core-modules/*/`; the host binary is unchanged by a guest edit
rm -rf target/t151-baseline/modules
cp -r modules/core-modules target/t151-baseline/modules

# candidate: the current ordinary module tree (post-rebuild)
rm -rf target/t151-candidate/modules
cp -r modules/core-modules target/t151-candidate/modules

# candidate accelerated: the accelerated *component* artifacts
# (target/guests-accelerated/artifacts/modules/core-modules/<dir>/<dir>.wasm,
#  NOT target/guests-accelerated/wasm32-unknown-unknown/release/<lib>.wasm,
#  which is the un-componentised intermediate and fails to load)
```

**Do not `cp -r` a module directory onto an existing one** — remove the
destination first, or stale `.wasm` files survive and the campaign measures the
wrong guest.

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/run_t151.ps1 \
  -Mode manifest -Arms baseline-ordinary,candidate-ordinary,candidate-accelerated
```

The manifest must show one `exe_sha256` shared by all three arms, the same
`skirt_brim_wasm_sha256` for the two candidate arms, and a different one for the
baseline arm.

## 3. Campaign

```bash
# 24 rows: 8 cells x {baseline-ordinary, candidate-ordinary, candidate-accelerated}
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/run_t151.ps1 \
  -Mode output -BatchId output

# 48 rows: 3 interleaved pairs x 8 cells x 2 ordinary arms (attribution only)
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/run_t151.ps1 \
  -Mode ab -Pairs 3 -BatchId ab

# null band: the same baseline arm on both sides
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/run_t151.ps1 \
  -Mode ab -Pairs 3 -NullControl -BatchId null \
  -Cells benchy-classic-off,base-classic-off
```

No wall claim is taken from the A/B; it is the attribution record and the null
control measures the instrument's own paired noise.

## 4. Orca reference for the adhesion oracle

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/orca-rig-repair/prepare_orca_vendor.ps1
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 \
  -Fixtures benchy,base -Cells classic-off,arachne-off -Tools orca \
  -Runs 1 -Warmups 0 -KeepGcode All -BatchId t151-oracle
```

The four supports-off cells are the oracle's scope; the supports-on cells are
support-engine-limited and are reported as out of scope (t138/t139).

## 5. Delta audit and oracle

```bash
cd docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour

python audit_delta.py \
  --old-dir ../../../../../target/t151/output/gcode \
  --new-dir ../../../../../target/t151/output/gcode \
  --old-suffix=-baseline-ordinary --new-suffix=-candidate-ordinary \
  --json-out ../../../../../target/t151/audit.json

for cell in benchy-classic-off benchy-arachne-off base-classic-off base-arachne-off; do
  python ../t135-skirt-brim-retake/oracle_skirt_brim.py \
    --pnp ../../../../../target/t151/output/gcode/$cell-baseline-ordinary.gcode \
    --orca ../../../../../target/matched-pair/t151-oracle/gcode/$cell-orca-m1.gcode \
    --cell $cell --json-out ../../../../../target/t151/oracle/$cell-baseline.json
  python ../t135-skirt-brim-retake/oracle_skirt_brim.py \
    --pnp ../../../../../target/t151/output/gcode/$cell-candidate-ordinary.gcode \
    --orca ../../../../../target/matched-pair/t151-oracle/gcode/$cell-orca-m1.gcode \
    --cell $cell --json-out ../../../../../target/t151/oracle/$cell-candidate.json
done
```

## 6. Reduce and verify

```bash
python reduce_t151.py
python verify.py
python verify.py --self-test
```

`reduce_t151.py` reads `target/t151/{manifest.json,output/rows.csv,ab/rows.csv,
null/rows.csv,audit.json,oracle/*.json}` and writes `reduced/summary.json`;
`verify.py` reads only that summary and re-derives every claim in the ticket's
closure.

## 7. Restore

Nothing to restore: the contour fold is the take's production change. Both
freshness gates must still exit 0.
