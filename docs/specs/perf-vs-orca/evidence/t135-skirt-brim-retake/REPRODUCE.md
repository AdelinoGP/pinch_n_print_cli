# t135 Stage 1 reproduce: canonical skirt/brim re-take

Candidate: the `com.core.skirt-brim` module replaces its bounding-box rectangles
with canonical geometry ported from the pinned OrcaSlicer `v2.5.0`
(`40ecc66f78`): a contour-anchored concentric brim (`make_brim` /
`tryExPolygonOffset`, `Brim.cpp`) and a convex-hull-anchored skirt that folds the
brim footprint in (`Print::_make_skirt`, `Print.cpp`). The matched job authors
`brim_width = 8`, `skirt_loops = 6`, `skirt_distance = 3`, `skirt_height = 1` on
both arms.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/` (the four adhesion
  keys are authored there).
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- `pwsh` for the campaign drivers, `python` for the oracle/audit/verify.

## 1. Build the candidate arms

```bash
cargo test -p skirt-brim                                  # module suite (20 tests)
cargo xtask build-guests                                  # ordinary guest
cargo build --release -p pnp-cli                          # ordinary host binary
cargo xtask dist --accelerated --edition developer        # accelerated snapshot

mkdir -p target/t135-candidate
cp target/release/pnp_cli.exe target/t135-candidate/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t135-candidate/modules
cp -r target/dist-accelerated/developer target/t135-candidate/accelerated
```

## 2. Build the baseline arms (pre-port module, same host binaries)

The port is a guest-only change, so the baseline arms keep the *same* host
binaries and swap only the `skirt-brim` module directory for the pre-port one
(old `.wasm` + old manifest — no `brim_object_gap` / `min_skirt_length` keys).

On a clean checkout at the pre-port commit, build the module tree as usual and
keep `target/t135-baseline/modules/skirt-brim` from it. In this take the
pre-port module tree is the t133 candidate snapshot (`target/t133-candidate`),
whose host source is the same commit this ticket branched from:

```bash
mkdir -p target/t135-baseline
cp target/release/pnp_cli.exe target/t135-baseline/pnp_cli-ordinary.exe
cp -r target/t135-candidate/modules target/t135-baseline/modules
rm -rf target/t135-baseline/modules/skirt-brim
cp -r target/t133-candidate/modules/skirt-brim target/t135-baseline/modules/skirt-brim
cp -r target/dist-accelerated/developer target/t135-baseline/accelerated
rm -rf target/t135-baseline/accelerated/modules/skirt-brim
cp -r target/t133-candidate/accelerated/modules/skirt-brim \
      target/t135-baseline/accelerated/modules/skirt-brim
```

Both ordinary arms must share one host binary; both accelerated arms must share
one accelerated binary (a rebuild changes the bytes even when the source is
identical). `-Mode manifest` checks the hashes.

**Do not `cp -r` a module directory onto an existing one** — `cp -r a/b c/d`
copies `b` *into* `d` when `d` exists, leaving the old `.wasm` in place. Remove
the destination first; the campaign otherwise measures the stale guest and the
hashes silently repeat.

## 3. Record the arm manifest and run the campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake/run_t135.ps1 -Mode manifest
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake/run_t135.ps1 \
  -Mode output -Arms baseline-ordinary,baseline-accelerated -BatchId output-baseline-relink
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake/run_t135.ps1 \
  -Mode output -Arms candidate-ordinary -BatchId output-closing
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake/run_t135.ps1 \
  -Mode output -Arms candidate-accelerated -BatchId output-closing-acc
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake/run_t135.ps1 \
  -Mode ab -Pairs 3 -BatchId ab3
```

Each output batch writes `target/t135/<batch>/rows.csv` (8 rows) and the A/B
writes 48 rows (3 interleaved pairs × 8 cells × 2 ordinary arms). G-code lands
in the matching `gcode/` directory. Both arms carry their own module tree, so a
guest rebuild cannot leak between them.

## 4. Scoreboard (the closing gate)

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 \
  -Fixtures benchy,base -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary,pnp-accelerated -Runs 3 -Warmups 1 -BatchId t135-scoreboard2
```

The rig pins and asserts `brim_width = 8`, `brim_type = outer_only`,
`skirt_loops = 6`, `skirt_distance = 3`, `skirt_height = 1` from each arm's
resolved-config appendix (Orca) and from the PnP appendix (`Test-PnpAdhesionKeys`).
`brim_type` is pinned because Orca 2.5's `auto_brim` recomputes the width from
its adhesion heuristic and silently discards an explicit `brim_width` (measured
on benchy classic-off: a ~10 × 5 mm, 3-loop brim).

`-KeepGcode` defaults to `Last`, so only the last measured run of each
cell/tool keeps its `.gcode` — the oracle below uses `-m3`.

## 5. Oracle, delta audit, reduce, verify

```bash
cd docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake

# Parity oracle: PnP candidate vs Orca, one pair per cell.
for cell in benchy-classic-off benchy-classic-on benchy-arachne-off benchy-arachne-on \
            base-classic-off base-classic-on base-arachne-off base-arachne-on; do
  python oracle_skirt_brim.py \
    --pnp  ../../../../target/t135/output-closing/gcode/$cell-candidate-ordinary.gcode \
    --orca ../../../../target/matched-pair/t135-scoreboard2/gcode/$cell-orca-m3.gcode \
    --cell $cell --json-out ../../../../target/t135/oracle-closing/$cell.json
done

# Delta audit: only skirt/brim/config lines may move vs the pre-port arm.
python audit_delta.py \
  --dir ../../../../target/t135/output-closing/gcode \
  --old-suffix -baseline-ordinary --new-suffix -candidate-ordinary \
  --json-out ../../../../target/t135/audit.json

python reduce_t135.py            # target/t135 -> reduced/summary.json
python verify.py                # every claim re-derived from summary.json
python verify.py --self-test    # negative controls must be rejected
```

## 6. Restore

Nothing to restore: the port is the take's production change. Both freshness
gates must still exit 0:

```bash
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
