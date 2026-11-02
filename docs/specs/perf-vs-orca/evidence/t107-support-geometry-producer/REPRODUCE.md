# Reproducing the t107 `PrePass::SupportGeometry` screen

Ticket: [`107-support-geometry-producer-decomposition.md`](../../../issues/107-support-geometry-producer-decomposition.md)

All commands run from this directory. The heavy fixtures `tmp/3dbenchy.stl` and
`tmp/base.stl` are user-supplied and never committed; everything else here is.

The screen needed **no new slice pass and no probe build**. It re-pairs the
raw instrumented streams ticket 104 already produced (gitignored under
`target/t104-instrumented/`) and corroborates the IR's size with one
`support-preview` run. Both source artifacts are older than this take and
independent of it.

```bash
# 0. paths below are relative to the repository root
cd ../../../..

# 1. IR size corroboration (clean HEAD binary; ~12 s on benchy-on).
#    The 77 MB JSON is gitignored; only the counts are committed.
RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe support-preview \
    --input tmp/3dbenchy.stl \
    --output target/t107-preview-benchy-on.json \
    --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
    --module-dir modules/core-modules
python docs/specs/perf-vs-orca/evidence/t107-support-geometry-producer/count-preview.py \
    target/t107-preview-benchy-on.json

# 2. reduce the two brackets out of t104's raw streams -> summary.json
python docs/specs/perf-vs-orca/evidence/t107-support-geometry-producer/reduce.py \
    --raw target/t104-instrumented

# 3. verify (checkout-only once summary.json is committed)
python docs/specs/perf-vs-orca/evidence/t107-support-geometry-producer/verify.py
python docs/specs/perf-vs-orca/evidence/t107-support-geometry-producer/verify.py --self-test
```

`summary.json` is the committed reduction; raw event streams stay in
`target/t104-instrumented/` (gitignored, produced by
`evidence/t104-phase-split/run-instrumented.py`).

## The two brackets under one stage id

`PrePass::SupportGeometry` names **two** stages that run back to back:

1. the host builtin `host:support_geometry`
   (`SUPPORT_GEOMETRY_PRODUCER`,
   `crates/slicer-runtime/src/builtins/support_geometry_producer.rs`), which
   builds `SupportGeometryIR.entries`; and
2. the support-planner module stage, which runs every module claiming
   `support-planner` (`com.core.traditional-support-planner`,
   `com.core.tree-support-planner`).

`stage_start`/`stage_complete` are keyed on `(stage, layer)` only, so both
brackets carry the same stage name; ticket 104's reducer summed them. `reduce.py`
pairs each `stage_complete` with its own `stage_start`, reads the nested
`module_complete` rows, reconciles the sum against ticket 104's committed
`phase-split.json`, and **fails loudly** if the two-bracket shape changes.

## Regimes

- Brackets are **attribution only** — never quoted as wall. Cell fractions use
  the PnP ordinary walls in
  `evidence/t103-orca-rebaseline/summary.json` (base classic-on 160.857 s,
  benchy classic-on 12.4034 s).
- The floor is the map's 0.80 % of the cell. Base classic-on uses ticket 102's
  committed `floor_seconds` (0.80 % of the t91 wall, 1.318424 s), which is
  stricter than 0.80 % of today's wall; the other cells use 0.80 % of their own
  t103 wall.
- No run in this take is a wall claim, and none was needed: the decisive
  quantity is the producer's 9 ms bracket against a 1.32 s floor.
