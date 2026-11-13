# t147 reproduce — supports-on body-rejection attribution

This take edits no production code. It applies a temporary host probe (saved as
`probe.patch`), runs one benchy and one base supports-on slice, and reduces the
probe output into `census.json`.

## Prerequisites

- `tmp/3dbenchy.stl` (sha256 `6ab57f1c…`) and `tmp/base.stl` (sha256
  `b26c8c79…`). Both user-supplied, never committed.
- The matched-pair config at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json`.
- The 24 external core modules under `modules/core-modules/`.
- The t145 raw campaign at `target/matched-pair/t145-benchy/` (its Orca
  `gcode/*-classic-on-orca-m3.gcode` files are the comparison arm; the PnP
  hashes are the currency check).
- Rust toolchain as pinned by the workspace; `python` and `shapely` for the
  reducer (`verify.py` is stdlib-only).
- `cargo xtask build-guests --check` must exit 0 (the probe changes the host
  only).

## 1. Apply the probe and build

```bash
git apply docs/specs/perf-vs-orca/evidence/t147-supports-rejection/probe.patch
cargo build --release -p pnp-cli
```

The probe is inert unless `PNP_T147_PROBE_DIR` is set. It adds
`serde_json` to `crates/slicer-wasm-host/Cargo.toml` only for the dump; both
files are restored before landing (the patch is evidence, not a change).

## 2. Run the two cells

```bash
# benchy classic-on, geometry window around the rejected layer
PNP_T147_PROBE_DIR=target/t147/probe-benchy3 \
PNP_T147_WINDOW=150:200 \
RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
  --model tmp/3dbenchy.stl \
  --output target/t147/benchy-classic-on.probe3.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules

# base classic-on, geometry window around the first cluster
PNP_T147_PROBE_DIR=target/t147/probe-base3 \
PNP_T147_WINDOW=14:42 \
RAYON_NUM_THREADS=12 target/release/pnp_cli.exe slice \
  --model tmp/base.stl \
  --output target/t147/base-classic-on.probe3.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules
```

Each run writes `entries.jsonl` (every validated entry + every rejection's
print-plane and midpoint overlap), `candidates.json` (the committed analysis
candidates), and `stderr.log` (the code-1200 diagnostics).

**Output neutrality check:** the probe G-code must equal the t145 currency:

```text
benchy  d9fc80baf6ae1e71e2f1a7df4e907120b6c42c046ec1d8a77b9b53751cab3f78
base    585c6a8edc2fd1de72dd2a86933fdad25c7ed144662dd80bbc68fb7856e4f629
```

If either differs, the probe moved output and the take must stop.

## 3. Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t147-supports-rejection
python reduce_t147.py     # writes census.json from target/t147
python verify.py          # every claim in FINDINGS.md
python verify.py --self-test  # 6/6 negative controls rejected
```

`reduce_t147.py` reads the t145 Orca g-code at
`target/matched-pair/t145-benchy/gcode/{benchy,base}-classic-on-orca-m3.gcode`;
`verify.py` reads only `census.json`.

## 4. Restore

```bash
git apply -R docs/specs/perf-vs-orca/evidence/t147-supports-rejection/probe.patch
cargo check -p slicer-wasm-host
```

Nothing else to restore: the probe is the only code change and it is removed
before landing. The raw `target/t147/` tree stays gitignored.
