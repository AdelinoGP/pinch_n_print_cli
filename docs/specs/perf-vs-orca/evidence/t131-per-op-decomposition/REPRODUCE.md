# t131 reproduce

Attribution-only take: the probe build is temporary and reverted; nothing in
the production tree changes.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.

## Rebuild the probe

```bash
git switch --detach 6c6c188b       # the take's HEAD
git apply docs/specs/perf-vs-orca/evidence/t131-per-op-decomposition/probe.patch
cargo build --release -p pnp-cli
```

The patch adds `crates/slicer-core/src/t131_probe.rs` and env-gated seams in
`polygon_ops.rs`, `slicer-wasm-host/src/host.rs`, `slicer-runtime/src/prepass.rs`,
`pipeline.rs` and `slice_postprocess_prepass.rs`. It compiles to a no-op without
`PNP_T131_PROBE=1`; `PNP_T131_S5=1` additionally enables the windowed-offset
probe.

## Run

```bash
# per-op split, four classic cells
python docs/specs/perf-vs-orca/evidence/t131-per-op-decomposition/run-probe.py \
  --flags probe

# windowed-offset (S5) pass
python docs/specs/perf-vs-orca/evidence/t131-per-op-decomposition/run-probe.py \
  --flags probe+s5
```

Raw streams and G-code land in `target/t131-runs/` (gitignored). Every run
asserts its G-code SHA-256 against ticket 123's candidate-ordinary reference
set; a mismatch fails the run. The probe build's timings are attribution only —
no wall claim is taken from them.

## Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t131-per-op-decomposition
python reduce.py            # target/t131-runs -> summary.json
python arithmetic.py        # every table in FINDINGS.md
python verify.py            # 150 checks against summary.json + FINDINGS prose
python verify.py --self-test # 11/11 negative controls rejected
```

`verify.py` needs only `summary.json` and `FINDINGS.md`; the raw tree is
optional.

## Restore the production tree

```bash
git checkout -- crates/
rm -f crates/slicer-core/src/t131_probe.rs
cargo build --release -p pnp-cli
cargo xtask build-guests --check   # must exit 0
```

The reverted binary is `f3d50217…` and reproduces `90638bf3…` on benchy
classic-off.
