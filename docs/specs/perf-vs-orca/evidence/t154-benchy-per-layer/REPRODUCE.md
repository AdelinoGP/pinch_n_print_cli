# Reproducing the t154 benchy per-layer attribution

Ticket: [`154-benchy-per-layer-carrier-attribution.md`](../../../issues/154-benchy-per-layer-carrier-attribution.md)

All commands run from this directory. The heavy fixtures `tmp/3dbenchy.stl` and
`tmp/base.stl` are user-supplied and never committed; everything else here is.

```bash
# 0. gates (both must exit 0 before any measurement is attributed)
cargo xtask build-guests --check
cargo build --release -p pnp-cli          # ordinary arm; record its SHA-256

# 1. eight instrumented passes, one per matched cell (attribution only)
python run-instrumented.py

# 2. reduce the raw streams -> summary.json (the committed split)
python reduce.py

# 3. fuel-profile arm -> one reduced scope table per cell
python run-profile.py

# 4. destination arithmetic and the carrier table
python arithmetic.py
python carriers.py

# 5. verify (checkout-only; inputs are the committed files)
python verify.py              # 0 = every claim holds
python verify.py --self-test  # negative controls must all be detected
```

Raw event streams, profile stderr and G-code land in
`target/t154-instrumented/` (gitignored); `summary.json`, `arithmetic.json`,
`carriers.json`, `runs.json` and the `profile-*.json` tables are the committed
reductions.

## Regimes, and why they are never mixed

- **Shares and brackets** come from the instrumented passes. `--instrument-stderr`
  inflates wall, so an instrumented wall is never a wall claim; a phase's implied
  wall is `share x the wall of record`, never a subtraction of an instrumented
  bracket from an uninstrumented wall.
- **Denominators** are t145's paired benchy/base supports-off scoreboard and
  t148's measured supports-on arms, carried forward by t150's measured per-cell
  deltas. Each cell's provenance string is in `arithmetic.json`. No ratio here
  is a ratio of record: the chain spans two windows and is labelled `estimate`.
- **Canonical prepass costs** are t110's tap-derived split (benchy-off 50.5 ms is
  a bound, not a point); the substitution holds every other phase at this take's
  measured share.
- **Per-layer and per-module sums are thread-accumulated, never wall.** A
  carrier's wall-equivalent ceiling is `thread_sum / measured effective
  parallelism` and charges 100 % of that carrier's parallel work to the critical
  path; it is an upper bound, never a price.
- **The module-busy union** is the union of every `module_start`/`module_complete`
  interval in the phase; `phase wall - union` is the host gap. Intervals are
  paired FIFO per `(phase, module_id)`.
- **Fuel is not wall.** The `--profile` arm meters wasm fuel and costs
  throughput; its `total_wall_ns` is the in-module wall only, which is why the
  difference between an instrumented bracket and its module's profiled wall is
  reported as host-side time of unmeasured composition, not as a split.

## Provenance and invariants

- Every instrumented pass must exit 0, be `degraded=false` with
  `fatal_error_count=0` and `non_fatal_error_count=0`, and produce G-code whose
  SHA-256 equals **t151's active set** (the currency after the skirt-hull fold).
- `RAYON_NUM_THREADS=12`; the standing matched-pair configs under
  `evidence/matched-pair/configs/`; 24 external core modules; ordinary arm only.
- Record the machine load sample beside the take; the passes are attribution,
  but a loaded box still moves shares.
- The profile arm covers all eight cells; the reduced tables are committed
  beside `summary.json` so the carrier citations resolve without the raw stderr.
