# t142 reproduce: overhang band-partition deletion

This take deletes the four-band polygon partition from
`PrePass::OverhangAnnotation` and derives the per-vertex quartile from the
signed distance to the previous-layer boundary. No model fixture is committed.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- `pwsh` and `python` (stdlib only; the reducer and verifier use no third-party
  package).

## 1. Baseline snapshot (pre-change tree)

Build the ordinary host binary and stage the pre-change guest modules **before**
editing:

```bash
cargo xtask build-guests            # exit 0 freshness precondition
cargo build --release -p pnp-cli
mkdir -p target/t142-baseline
cp target/release/pnp_cli.exe target/t142-baseline/pnp_cli-ordinary.exe
rm -rf target/t142-baseline/modules
cp -r modules/core-modules target/t142-baseline/modules
```

## 2. Candidate build (post-change tree)

```bash
cargo xtask build-guests
cargo xtask build-guests --check    # exit 0
cargo build --release -p pnp-cli
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/run_t142.ps1 -Mode manifest
```

## 3. Campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/run_t142.ps1 \
  -Mode output -BatchId output1
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/run_t142.ps1 \
  -Mode ab -Pairs 3 -BatchId ab
```

`-Mode output` writes 16 rows (8 cells × 2 arms: baseline-ordinary,
candidate-ordinary). Every cell's two sha256 values must be equal; the candidate
hash must also equal t139's reference hash for that cell.

`-Mode ab` writes 48 rows (3 interleaved pairs × 8 cells × 2 ordinary arms),
with the pair order alternating by parity.

## 4. Reduce and verify

```bash
python docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/reduce_t142.py
python docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/verify.py
python docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/verify.py --self-test
```

The reducer reads the gitignored raw tree under `target/t142/`; `verify.py`
reads only `reduced/campaign.json` and re-derives every number in it.

## 5. Accelerated arm (parity, disclosed build)

The controlled `cargo xtask dist --accelerated --edition developer` could not
produce the accelerated **host** binary in this environment: the driver's
`RUSTC` wrapper makes `libmimalloc-sys`'s C++ build script receive a
verbatim (`\\?\`) source path, and MSVC 14.51 (`c19xx`) mangles it into
`\mimalloc-static.cc` (`C1083`). The same build **without** the wrapper
succeeds, so this is a driver/toolchain interaction, not a source change.

The accelerated **output** was therefore checked with a disclosed manual host
build plus the driver-built accelerated guests:

```bash
cargo xtask build-guests --accelerated
RUSTFLAGS="--cfg pnp_perimeter_spatial_accelerated" \
  CARGO_TARGET_DIR=target/dist-host-accelerated-manual \
  cargo build -p pnp-cli --release

mkdir -p target/t142-candidate/accelerated
cp target/dist-host-accelerated-manual/release/pnp_cli.exe \
   target/t142-candidate/accelerated/pnp_cli_accelerated.exe
rm -rf target/t142-candidate/accelerated/modules
cp -r modules/core-modules target/t142-candidate/accelerated/modules
for d in target/guests-accelerated/artifacts/modules/core-modules/*/; do
  name=$(basename "$d")
  [ -f "$d/$name.wasm" ] && cp -f "$d/$name.wasm" \
    "target/t142-candidate/accelerated/modules/$name/$name.wasm"
done

pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/run_t142.ps1 \
  -Mode output -Arms candidate-accelerated -BatchId output-accel2
```

All eight cells emit the same hashes as the ordinary candidate. This arm is a
parity check only — it is never used for timing, and it is not the
controlled-driver artifact.
