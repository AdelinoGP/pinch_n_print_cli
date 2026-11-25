# Reproducing the t155 `Layer::Infill` block attribution

Ticket: [`155-infill-stage-block-base-on-residual.md`](../../issues/155-infill-stage-block-base-on-residual.md)

All commands run from the repository root. The heavy fixtures `tmp/base.stl`
and `tmp/3dbenchy.stl` are user-supplied and never committed; everything else
here is. Raw probe streams land in `target/t155-probe/` (gitignored);
`probe.json` and `summary.json` are the committed reductions.

```bash
# 0. gates (both must exit 0 before any measurement is attributed)
cargo xtask build-guests --check
cargo build --release -p pnp-cli          # ordinary arm; record its SHA-256

# 1. baseline attribution at HEAD, no probe (the t154 method)
RAYON_NUM_THREADS=12 ./target/release/pnp_cli.exe slice \
  --model tmp/base.stl --output target/t155-instrumented/base-classic-off.gcode \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules --instrument-stderr \
  2> target/t155-instrumented/base-classic-off.jsonl

# 2. apply the temporary probe and rebuild
git apply docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/probe.patch
cargo build --release -p pnp-cli          # ~8.5 min from a warm tree; record SHA-256

# 3. probe passes (each exits 0 and must reproduce t151's active set)
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/run_probe.py \
  --cell base-classic-off --stem base-classic-off-ns
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/run_probe.py \
  --cell base-classic-on  --stem base-classic-on-ns
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/run_probe.py \
  --cell base-classic-off --stem base-classic-off-decline --decline-raft

# 4. reduce and price
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/reduce.py
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/price.py

# 5. restore the tree and re-verify freshness
git stash push -m "t155 probe" -- \
  crates/slicer-runtime/src/layer_executor.rs \
  crates/slicer-runtime/src/region_partition.rs \
  crates/slicer-sdk/src/views.rs \
  crates/slicer-wasm-host/src/dispatch.rs \
  crates/slicer-wasm-host/src/marshal/prepared.rs
cargo build --release -p pnp-cli
cargo xtask build-guests --check          # must exit 0

# 6. verify (checkout-only; inputs are the committed files)
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/verify.py
python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/verify.py --self-test
```

## Probe sites and what each one answers

The probe is five files (`probe.patch`), env-gated on `PNP_T155_PROBE`, and
writes `{"event":"probe",...}` lines into the same stderr stream:

| site | question it answers |
| --- | --- |
| `bracket_start` → `prep_views` | is the first-dispatch block `prepare_dispatch_region_views`? |
| `prep_views` → `input_ctor` | does `LayerStageInput` construction (Arc clones) matter? |
| `input_ctor` → `run_stage` | is it the prepared-plan wrapper or the runner? |
| `rs_selection` / `rs_transport` / `rs_envelope` | target maps, transport assembly, output envelope |
| `dl_acquire` / `dl_linker` / `dl_instantiate` / `dl_push_regions` / `dl_call` / `dl_return` | pool lease, linker, component instantiation, region marshal, typed export, teardown |
| `pr_needs_support` / `pr_quartile` / `pr_overhang_areas` / `pr_prev_boundary` … | the derivation's internal composition |
| `ns_call` | `derive_needs_support` census: footprints, bbox hits, answer, elapsed |
| `sync_audit` | do the fill sync's writes touch any field `prepare_regions` reads? |
| `PNP_T155_DECLINE_RAFT` | t127's decline-and-watch transfer test (probe-only gate) |

## Regimes, and why they are never mixed

- **Shares and site sums** come from probe passes. `--instrument-stderr`
  inflates wall and the probe adds stderr writes, so an instrumented or probed
  wall is never a wall claim.
- **Denominators** are t154's measured per-layer effective parallelism
  (10.91x / 11.18x) and its destination-arithmetic walls of record; they are
  encoded in `price.py` with their provenance.
- **Ceilings** are `thread_sum / effective_parallelism`, charging 100 % of the
  carrier's parallel work to the critical path; never a price.
- **The derivation's exactness** is the sync audit (0/1,488 changed read-field
  digests) plus the code fact that the sync writes only the four fill polygons.
- **The predicate census** is `derive_needs_support`'s own instrumentation:
  footprint count, bbox overlap count, answer and elapsed per call.
