# t149 reproduce — MeshAnalysis work-set reader census and sub-phase price

This take changes **no production code**. It applies a non-landing probe to
`crates/slicer-core/src/algos/mesh_analysis.rs`, runs three slices, captures the
sub-phase lines, and reverts.

## Prerequisites

- `tmp/base.stl` (sha256 `b26c8c79…`) and `tmp/3dbenchy.stl` (sha256
  `6ab57f1c…`). Both user-supplied, never committed.
- The matched-pair configs at `evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- A 12-logical-core box (`RAYON_NUM_THREADS=12`); the probe's sub-phase times
  are single-run `Instant` deltas, so this is only the setting the census was
  captured under.

## 1. Build

```bash
cargo build --release -p pnp-cli      # target/release/pnp_cli.exe
```

## 2. Apply the probe

```bash
git apply docs/specs/perf-vs-orca/evidence/t149-meshanalysis-readers/probe.patch
```

The probe wraps four blocks of `classify_object` in `Instant` deltas and prints
one line per object:

```text
PROBE mesh_analysis object=<id> tris=<n> overhang=<n> classify_ms=<f> footprint_ms=<f> needs_support_ms=<f>
PROBE mesh_analysis object=<id> bridge_ms=<f> bridge_regions=<n> total_ms=<f>
```

## 3. The three cells

```bash
mkdir -p target/t149
RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t149/base-classic-off.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules 2> target/t149/base-classic-off.stderr.log

RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t149/base-classic-on.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules 2> target/t149/base-classic-on.stderr.log

RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
  --model tmp/3dbenchy.stl --output target/t149/benchy-classic-off.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules 2> target/t149/benchy-classic-off.stderr.log
```

Expected (the census's values; single-run `Instant` deltas, not wall claims):

```text
base    classic-off  cell 35244 ms  classify 187.0  footprint 529.1  needs 0.2  bridge 4563.2  total 5325.8
base    classic-on   cell 85525 ms  classify 135.9  footprint 510.9  needs 0.1  bridge 4150.2  total 4832.9
benchy  classic-off  cell  3702 ms  classify   6.4  footprint  91.1  needs 0.0  bridge   95.1  total  195.1
```

`cell` is the `slice_complete` event's `elapsed_ms`; `degraded` must be `false`
and `layer_count` 496 (base) / 240 (benchy).

## 4. The profiled reconciliation run

```bash
RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t149/base-classic-off-prof.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules --profile 2> target/t149/base-classic-off-prof.stderr.log
```

The `profile_summary` event carries `host:mesh_analysis`'s `total_wall_ns`
(**5,639.5 ms**) against the probe total of **4,914.1 ms**. Profile wall columns
are accumulated span time, so this row reconciles the probe's accounting; it is
not a wall claim.

## 5. Restore production and verify

```bash
git checkout -- crates/slicer-core/src/algos/mesh_analysis.rs
cd docs/specs/perf-vs-orca/evidence/t149-meshanalysis-readers
python verify.py --self-test        # 43 claims, 4/4 negative controls rejected
```

`verify.py` reads `census.json` and re-derives every published number, then
mutates four copies of the census (halved bridge half, a sub-phase larger than
the stage, swapped base cell walls, profiled stage below the probe) and requires
each to be rejected.
