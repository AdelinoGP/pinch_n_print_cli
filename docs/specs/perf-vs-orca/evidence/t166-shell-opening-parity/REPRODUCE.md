# t166 reproduce: canonical top/bottom opening in `PrePass::ShellClassification`

The candidate replaces Pass 1's opening in
`commit_shell_classification_builtin`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`):

| | before | after |
| --- | --- | --- |
| radius | `line_width / 2` (0.2 mm) | `external_perimeter_width / 10` (0.04 mm) |
| join | `Round`, arc tolerance 0 | `Miter`, miter limit 3 |
| calls | `offset(-r, Round)` + `offset(+r, Round)` | one `opening_ex(diff, r, Miter, 3.0)` |

Pass 2's shrinking-shadow width is unchanged (one `line_width`); the two widths
are resolved separately in `ShellClassificationParams` so the parity fix cannot
move the cascade.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs under
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- `pwsh`, Python 3 (stdlib only), and `RAYON_NUM_THREADS=12` (the runner sets it).
- The OrcaSlicer source pin for the canonical citation: `v2.5.0` = `40ecc66f78`.

## 1. Baseline snapshot (pre-change tree)

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli

mkdir -p target/t166-baseline
cp -f target/release/pnp_cli.exe target/t166-baseline/pnp_cli-ordinary.exe
rm -rf target/t166-baseline/modules
cp -r modules/core-modules target/t166-baseline/modules
```

## 2. Candidate build (post-change tree)

```bash
cargo xtask build-guests --check     # host-only change: guests stay fresh
cargo build --release -p pnp-cli

mkdir -p target/t166-candidate
cp -f target/release/pnp_cli.exe target/t166-candidate/pnp_cli-ordinary.exe
rm -rf target/t166-candidate/modules
cp -r modules/core-modules target/t166-candidate/modules

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/run_t166.ps1 -Mode manifest
```

`-Mode manifest` records both executables' SHA-256 and the module-tree hash; the
two arms must share one module-tree hash (this change is host-only).

## 3. Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/run_t166.ps1 \
  -Mode output -BatchId ref
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/run_t166.ps1 \
  -Mode ab -Pairs 5 -BatchId ab
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/run_t166.ps1 \
  -Mode ab -Pairs 3 -NullControl \
  -Cells base-classic-off,base-arachne-off,benchy-classic-off -BatchId null
```

- `-Mode output` writes 16 rows (8 cells × 2 arms). The baseline arm must
  reproduce **all eight** of t151's active reference hashes; the candidate arm
  records the new set.
- `-Mode ab` is the paired gate: 5 interleaved pairs per cell, alternating
  which arm goes first. `-Append -OnlyMissing` resumes an interrupted batch
  without re-running completed rows.
- `-NullControl` runs the baseline arm on both sides of three cells × three
  pairs and measures the instrument's own band.
- `-Mode ab` deletes each run's G-code after recording the hash; `-Mode output`
  keeps it, which is what `parity_delta.py` reads.

## 4. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/reduce_t166.py
python docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/verify.py
python docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/verify.py --self-test

python docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/parity_delta.py
python docs/specs/perf-vs-orca/evidence/t166-shell-opening-parity/parity_delta.py --self-test
```

`reduce_t166.py` reads the gitignored raw tree under `target/t166/`;
`verify.py` and `parity_delta.py` read only `reduced/` (parity additionally
reads the campaign's G-code, which `-Mode output` leaves under
`target/t166/output/`).

## 5. Unit tests

```bash
cargo test -p slicer-runtime --lib slice_postprocess_prepass 2>&1 | tee target/test-output.log
```
