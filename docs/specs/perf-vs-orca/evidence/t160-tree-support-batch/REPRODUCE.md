# t160 reproduce: tree-support render placement — whole-object batch vs per-layer dispatch

Probe-only, env-gated. No guest change, no contract change. The single release
binary serves every arm; the probe is off unless its env var is `1`.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- Matched-pair configs under `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- `pwsh` (PowerShell 7), Python 3 (stdlib only), `RAYON_NUM_THREADS=12`.
- Fresh guests: `cargo xtask build-guests --check` exit 0 (the probe is host-only).

## 1. Build and stage the probe tree

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli
mkdir -p target/t160/probe
cp -f target/release/pnp_cli.exe target/t160/probe/pnp_cli-ordinary.exe
rm -rf target/t160/probe/modules
cp -r modules/core-modules target/t160/probe/modules
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/run_t160.ps1 -Mode manifest
```

Arms (same exe, same module tree):

| arm | env |
|---|---|
| `baseline` | none (production path) |
| `batch` | `PNP_T160_SUPPORT_BATCH=1` |
| `nullrender` | `PNP_T160_SUPPORT_NULL=1` |
| `hash` | `PNP_T160_INPUT_HASH=1` |

## 2. Input-identity gate (run first)

`-Mode hash` runs every cell once with `PNP_T160_INPUT_HASH=1`: the pass prints
the `Layer::Support` read-set digest for each layer twice — once at
prepass-commit state (`batch`) and once at per-layer dispatch state
(`per_layer`), split into `plan` (support-plan rows), `view` (region identity,
z, effective height) and `paint` (slice polygons + segment annotations).

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/run_t160.ps1 \
  -Mode hash -BatchId hash
```

If any component differs on any layer, stop: the fold is not a pure placement
move and the dependency is recorded instead of a campaign.

## 3. Output identity

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/run_t160.ps1 \
  -Mode output -BatchId output
```

16 rows (8 cells × {baseline, batch}); every batch-arm hash must equal t151's
active reference set (`evidence/t151-skirt-hull-contour/reference.json`).

## 4. Paired campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/run_t160.ps1 \
  -Mode ab -Pairs 5 -BatchId ab
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/run_t160.ps1 \
  -Mode ab -Pairs 3 -NullControl -BatchId null
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/run_t160.ps1 \
  -Mode ceiling -Pairs 5 -BatchId ceiling
```

Interleaved order alternates per pair; each pair is one baseline run and one
batch run of the same cell. The null control runs the baseline arm on both
sides of the four supports-on cells. The ceiling arm runs the null renderer
(output not matched) to bound what the renderer's placement can cost at most.

## 5. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/reduce_t160.py
python docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/verify.py
python docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/verify.py --self-test
```

`reduce_t160.py` reads the gitignored raw tree under `target/t160/`;
`verify.py` reads only `reduced/campaign.json` and re-derives every claim.

## 6. Restore

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t160-tree-support-batch/probe.patch
cargo xtask build-guests --check
cargo build --release -p pnp-cli
target/release/pnp_cli.exe slice --model tmp/base.stl --output target/t160/restored.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json
```

The restored binary must reproduce t151's active hash for the checked cell.
