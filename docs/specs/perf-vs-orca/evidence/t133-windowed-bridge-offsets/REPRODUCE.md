# t133 reproduce

Candidate: the negative `deep_infill_area` erosion inside the internal-bridge
pass is computed on an operand clipped to the union bounding box of the layer's
`internal_solid_fill` candidates, grown by `(2 x 4.5 + 1) x qualify_spacing` — the
erosion distance plus Clipper's miter reach. The positive `deep_infill_clip_area`
stays whole-layer: `internal_bridge_angles` intersects the layer's sparse
expansion area with it and derives bridge directions from the whole-layer
boundary anchors, so clipping it moves bridge directions.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- `pwsh` for the campaign driver.

## 1. Build the baseline arms (before applying the candidate)

At the take's HEAD (`5af3db60`), with a clean tree:

```bash
cargo xtask build-guests --check             # exit 0
cargo build --release -p pnp-cli             # ordinary host binary
cargo xtask dist --accelerated --edition developer

mkdir -p target/t133-baseline
cp target/release/pnp_cli.exe target/t133-baseline/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t133-baseline/modules
cp -r target/dist-accelerated/developer target/t133-baseline/accelerated
```

## 2. Build the candidate arms

Apply the ticket's host change (`internal_bridge_offset_window` next to
`compute_internal_bridge_layer` and the clipped erosion operand), then:

```bash
cargo test -p slicer-runtime --lib internal_bridge_window_preserves_the_erosion_inside_the_candidate_bbox
cargo build --release -p pnp-cli
cargo xtask dist --accelerated --edition developer

mkdir -p target/t133-candidate
cp target/release/pnp_cli.exe target/t133-candidate/pnp_cli-ordinary.exe
cp -r modules/core-modules target/t133-candidate/modules
cp -r target/dist-accelerated/developer target/t133-candidate/accelerated
```

The host binary changes; the module tree does not. Record the arm hashes:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t133-windowed-bridge-offsets/run_t133.ps1 -Mode manifest
```

## 3. Run the campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t133-windowed-bridge-offsets/run_t133.ps1 -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t133-windowed-bridge-offsets/run_t133.ps1 -Mode ab -Pairs 3 -BatchId ab
```

`-Mode output` writes `target/t133/output/rows.csv` (32 rows: 8 cells × 4 arms);
`-Mode ab` writes `target/t133/ab/rows.csv` (48 rows: 3 interleaved pairs × 8
cells × 2 ordinary arms). G-code lands in the matching `gcode/` directory. Both
arms carry their own module tree, so a guest rebuild cannot leak between them.

The candidate is exact by construction (see the erosion-window locality
argument in the ticket's FINDINGS), so the output contract is hash equality:
every arm must reproduce ticket 123's candidate-ordinary set byte for byte. The
delta audit then has nothing to bound — no structural or fill line moves.

## 4. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t133-windowed-bridge-offsets
python reduce_t133.py            # target/t133 -> reduced/summary.json
python verify.py                # every claim re-derived from summary.json
python verify.py --self-test    # negative controls must be rejected
```

`verify.py` needs only `reduced/summary.json`; the raw tree is optional.

## 5. Restore the production tree

Nothing to restore: the window is the take's production change. Both freshness
gates must still exit 0:

```bash
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```
