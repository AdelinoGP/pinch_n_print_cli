# t134 reproduce

Attribution-only take: the probe build is temporary and reverted; nothing in
the production tree changes.

## Prerequisites

- `tmp/3dbenchy.stl` and `tmp/base.stl` (user-supplied, never committed).
- The matched-pair configs at
  `docs/specs/perf-vs-orca/evidence/matched-pair/configs/`.
- The 24 external core modules under `modules/core-modules/`.
- Rust toolchain as pinned by the workspace; run from the repository root.
- Python 3 (stdlib only) for the reducer and verifier.

## Rebuild the probe

The take's HEAD is `aee7b0db` (t133's resolution). The patch is against that
tree; applying it on a later docs-only HEAD is fine as long as the code paths
are unchanged.

```bash
git switch --detach aee7b0db
git apply docs/specs/perf-vs-orca/evidence/t134-planner-parallelism/probe.patch
cargo xtask build-guests           # the planner guest changed; rebuilds all guests on a source change
cargo build --release -p pnp-cli
```

The patch adds `crates/slicer-core/src/t134_probe.rs`, sweep seams in
`polygon_ops.rs`, stage/module deltas in `prepass.rs`, the batch census in
`batch.rs`, the `T134SNAP:` hook in `wasm-host`'s `log`, and the
`[PERF-T134-PROBE]` guest spans in the tree-support planner. Without
`PNP_T134_PROBE=1` every seam is one relaxed atomic load and a branch.

## Run

```bash
python docs/specs/perf-vs-orca/evidence/t134-planner-parallelism/run_t134_probe.py
```

The driver runs the eight matched cells once each on the ordinary arm with
`PNP_T134_PROBE=1` and `RAYON_NUM_THREADS=12`, writes G-code and stderr under
`target/t134/runs/` (gitignored), and **fails on any G-code hash that is not
ticket 123's candidate-ordinary reference**. The probe adds no production
behaviour; the hash check is the output-neutrality control.

The two extra repeatability passes are optional; if present under
`target/t134-runs/` the reducer records them. They were taken during the take
on the same probe build before the built-in stage brackets were added (the
module and phase counters are identical).

## Reduce and verify

```bash
cd docs/specs/perf-vs-orca/evidence/t134-planner-parallelism
python reduce_t134.py          # target/t134/runs -> reduced/summary.json
python verify.py               # 94 checks against summary.json
python verify.py --self-test   # 12/12 negative controls must be rejected
```

`verify.py` needs only `reduced/summary.json`; the raw stderr tree is optional
and gitignored.

## Restore the production tree

```bash
git checkout -- crates modules
git reset -- crates/slicer-core/src/t134_probe.rs
rm -f crates/slicer-core/src/t134_probe.rs
cargo xtask build-guests --check   # must exit 0 (rebuild if the guest is stale)
cargo xtask build-guests --accelerated --check
```

The probe patch is the only artifact; `git status` after the restore must show
only the ticket, map and evidence files.
