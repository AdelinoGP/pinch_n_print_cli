# t136 reproduce

Attribution-only take: the probe build is temporary and reverted; nothing in the
production tree changes.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- A 12-logical-core box (`RAYON_NUM_THREADS=12` is what the floors are quoted
  against; on another width re-derive them from `thread_ns`).
- Python 3 (stdlib only) for the driver, reducer and verifier.

## Rebuild the probe

The take's HEAD is `8fbbfe49` (t135 Stage 2's resolution). The patch is against
that tree; applying it on a later docs-only HEAD is fine as long as the code
paths are unchanged.

```bash
git apply docs/specs/perf-vs-orca/evidence/t136-clip-fanout/probe.patch
cargo xtask build-guests           # the planner guest changed
cargo build --release -p pnp-cli
```

The patch adds `crates/slicer-wasm-host/src/t136_probe.rs`, the per-call
counters and marker echo in `crates/slicer-wasm-host/src/host.rs` (plus the
`mod` line in `lib.rs`), and the `[PERF-T136-PROBE]` segment brackets in
`modules/core-modules/tree-support-planner/src/lib.rs`. Without
`PNP_T136_PROBE=1` every seam is one relaxed atomic load and a branch.

## Run

```bash
python docs/specs/perf-vs-orca/evidence/t136-clip-fanout/run_t136.py --passes 3
```

The driver runs `benchy-classic-on` and `base-classic-on` three times each on
the ordinary arm with `PNP_T136_PROBE=1`, `RAYON_NUM_THREADS=12` and
`--instrument-stderr`, writes G-code and stderr under `target/t136/campaign/`
(gitignored), and **fails on any G-code hash that is not the active post-t139
reference**. The hash check is the output-neutrality control: the probe adds no
production behaviour.

The manifest is overwritten on every run. Two concurrent campaigns writing one
`rows.csv` corrupt it — observed once during this take, which is why the driver
no longer appends.

## Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t136-clip-fanout
python reduce_t136.py          # target/t136/campaign -> reduced/summary.json
python verify.py               # 155 checks against summary.json
python verify.py --self-test   # 27/27 negative controls must be rejected
```

`reduce_t136.py` resolves `--run-dir` (default `target/t136/campaign`) against
the repository root, so it runs from any directory. `verify.py` needs only
`reduced/summary.json`; the raw stderr tree is optional and gitignored.
`summary.json` carries each phase's per-chunk `thread_ns:max_request_ns` list,
so the merge floors are re-derivable from the in-repo set alone.

## Restore the production tree

```bash
git checkout -- crates modules
git reset -- crates/slicer-wasm-host/src/t136_probe.rs
rm -f crates/slicer-wasm-host/src/t136_probe.rs
cargo xtask build-guests --check   # must exit 0 (rebuild if the guest is stale)
cargo xtask build-guests --accelerated --check
```

The probe patch is the only artifact; `git status` after the restore must show
only the ticket, map and evidence files.
