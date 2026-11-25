# t139 reproduce: canonical mid-layer sampling

This take changes the production slice plane (host + guest) and re-records the
matched job's reference set, the adhesion oracle, the old-vs-new A/B, and the
8×3 scoreboard. No model fixture is committed.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- `pwsh`, `python` with `numpy`, `trimesh`, `shapely`, and the OrcaSlicer rig
  (`prepare_orca_vendor.ps1` → `target/orca-vendor/`) for the scoreboard.

## 1. Builds

```bash
cargo xtask build-guests
cargo xtask build-guests --accelerated
cargo build --release -p pnp-cli
cargo xtask dist --accelerated --edition developer

cargo xtask build-guests --check              # exit 0
cargo xtask build-guests --accelerated --check # exit 0
```

## 2. Arms

The change spans the host slice producer and the guest planner, so the ordinary
arms carry their own binary and module tree. The baseline is the t135 Stage-1
candidate snapshot (pre-t139 source):

```bash
mkdir -p target/t139-baseline target/t139-candidate
cp -r target/t135-candidate/modules target/t139-baseline/modules
cp target/t135-candidate/pnp_cli-ordinary.exe target/t139-baseline/pnp_cli-ordinary.exe

cp target/release/pnp_cli.exe target/t139-candidate/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t139-candidate/modules
cp -r target/dist-accelerated/developer target/t139-candidate/accelerated

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/run_t139.ps1 -Mode manifest
```

**Do not `cp -r` a module directory onto an existing one** — the destination is
removed first above; otherwise stale `.wasm` files survive and the campaign
measures the wrong guest.

## 3. Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/run_t139.ps1 \
  -Mode output -BatchId output-final2
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/run_t139.ps1 \
  -Mode ab -Pairs 3 -BatchId ab2
```

`output-final2` writes 24 rows (8 cells × 3 arms) and is the reference set;
`ab2` writes 48 rows (3 interleaved pairs × 8 cells × 2 ordinary arms). The
baseline column must reproduce the t135 candidate set exactly — `delta_audit.py`
checks it.

## 4. Adhesion oracle

```bash
mkdir -p target/t139/oracle-final
for cell in benchy-classic-off benchy-classic-on benchy-arachne-off benchy-arachne-on \
            base-classic-off base-classic-on base-arachne-off base-arachne-on; do
  python docs/specs/perf-vs-orca/evidence/t135-skirt-brim-retake/oracle_skirt_brim.py \
    --pnp  target/t139/output-final2/gcode/$cell-candidate-ordinary.gcode \
    --orca target/matched-pair/t135-scoreboard3/gcode/$cell-orca-m3.gcode \
    --cell $cell --json-out target/t139/oracle-final/$cell.json
done
```

## 5. Scoreboard (closing gate)

```bash
pwsh -NoProfile -Command "& './docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1' \
  -Fixtures benchy,base -Cells classic-off,classic-on,arachne-off,arachne-on \
  -Tools orca,pnp-ordinary,pnp-accelerated -Runs 3 -Warmups 1 -BatchId t139-scoreboard"
```

Array parameters must go through `-Command`; `pwsh -File` binds `benchy,base` as
one string.

## 6. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/delta_audit.py
python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/reduce_t139.py
python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/reduce_scoreboard.py
python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/verify.py
python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/verify.py --self-test
```

The three reducers read the gitignored raw tree; `verify.py` reads only
`reduced/delta.json`, `reduced/campaign.json`, and `reduced/scoreboard.json`.

## 7. Restore

Nothing to restore: the plane change is the take's production change. Both
freshness gates must exit 0 after the final edit.
