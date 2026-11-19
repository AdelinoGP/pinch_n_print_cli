# t168 reproduce: the bridge opening's auto sentinel

The candidate changes one resolution policy. `resolve_external_perimeter_width_mm`
(`crates/slicer-core/src/algos/prepass_slice.rs`) now requires **positive** for
both `ConfigValue::Float` and absolute `FloatOrPercent` `outer_wall_line_width`,
and is the single policy the shell stage calls too. The opening radius in
`classify_unsupported_bridge_areas` therefore becomes `0.1 x line_width`
instead of `0.1 x 0`.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs under
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The core modules under `modules/core-modules/`.
- `pwsh`, Python 3 (stdlib only), and `RAYON_NUM_THREADS=12` (the runner sets it).
- The canonical citation: OrcaSlicer `v2.5.0` = `40ecc66f78`,
  `PrintObject::detect_surfaces_type` (`PrintObject.cpp`).

## 1. Baseline snapshot (pre-change tree)

```bash
git stash push -m "t168 wip"
cargo xtask build-guests --check
cargo build --release -p pnp-cli

mkdir -p target/t168-baseline
cp -f target/release/pnp_cli.exe target/t168-baseline/pnp_cli-ordinary.exe
rm -rf target/t168-baseline/modules
cp -r modules/core-modules target/t168-baseline/modules

cargo xtask dist --accelerated
mkdir -p target/t168-baseline-accelerated
cp -f target/dist-accelerated/developer/pnp_cli.exe \
      target/t168-baseline-accelerated/pnp_cli-accelerated.exe
rm -rf target/t168-baseline-accelerated/modules
cp -r target/dist-accelerated/developer/modules \
      target/t168-baseline-accelerated/modules

git stash pop
```

## 2. Candidate build (post-change tree)

```bash
cargo xtask build-guests --check     # host-only change; rebuild if the
                                     # slicer-core fingerprint reads stale
cargo build --release -p pnp-cli
mkdir -p target/t168-candidate
cp -f target/release/pnp_cli.exe target/t168-candidate/pnp_cli-ordinary.exe
rm -rf target/t168-candidate/modules
cp -r modules/core-modules target/t168-candidate/modules

cargo xtask dist --accelerated
mkdir -p target/t168-candidate-accelerated
cp -f target/dist-accelerated/developer/pnp_cli.exe \
      target/t168-candidate-accelerated/pnp_cli-accelerated.exe
rm -rf target/t168-candidate-accelerated/modules
cp -r target/dist-accelerated/developer/modules \
      target/t168-candidate-accelerated/modules

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t168-bridge-opening/run_t168.ps1 -Mode manifest
```

`-Mode manifest` records all four executables and both module trees; the two
ordinary arms must share one tree hash and the two accelerated arms another
(this change is host-only).

## 3. Campaign

```bash
pwsh -NoProfile -File .../run_t168.ps1 -Mode output -BatchId ref
pwsh -NoProfile -File .../run_t168.ps1 -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File .../run_t168.ps1 -Mode ab -Pairs 5 -BatchId ab
pwsh -NoProfile -File .../run_t168.ps1 -Mode ab -Pairs 10 -PairFrom 6 -PairTo 10 \
  -Append -OnlyMissing -Cells base-classic-off,base-classic-on,base-arachne-off,base-arachne-on,benchy-classic-off -BatchId ab
pwsh -NoProfile -File .../run_t168.ps1 -Mode ab -Pairs 3 -NullControl \
  -Cells base-classic-off,base-arachne-off,benchy-classic-off -BatchId null
pwsh -NoProfile -File .../run_t168.ps1 -Mode ab -Pairs 5 -NullControl \
  -Cells base-classic-on,base-arachne-on -BatchId null2
pwsh -NoProfile -File .../run_t168.ps1 -Mode ab -Accelerated -Pairs 3 -BatchId accel
```

- `-Mode output` keeps the G-code (the audit reads it); `-Mode ab` deletes each
  run's G-code after recording the hash.
- `-Append -OnlyMissing` resumes an interrupted batch without re-running rows.
- The campaign was extended from five to ten pairs per cell after the first
  five showed mixed-sign medians; both extension boundaries are disclosed in
  `FINDINGS.md`.

## 4. Reduce, audit, verify

```bash
python .../reduce_t168.py
python .../audit_t168.py
python .../audit_t168.py --self-test
python .../verify.py
python .../verify.py --self-test
```

`reduce_t168.py` and `audit_t168.py` read the gitignored raw tree under
`target/t168/`; `verify.py` reads only `reduced/`, so it exits 0 from a plain
checkout.

## 5. Unit tests and gates

```bash
cargo test -p slicer-core --features host-algos --lib prepass_slice
cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd
cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd
cargo test -p slicer-runtime --lib
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-literals
cargo xtask check-test-quality --report
cargo xtask build-guests --check
```

The falsifier was proven red by restoring the old resolver arm
(`Some(ConfigValue::Float(value)) => *value as f32`) and re-running
`external_perimeter_width_treats_zero_extension_as_auto`; the guard was then
restored.
