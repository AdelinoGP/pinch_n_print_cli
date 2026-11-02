# Reproducing the t104 phase split

Ticket: [`104-prepass-vs-per-layer-attribution.md`](../../../issues/104-prepass-vs-per-layer-attribution.md)

All commands run from this directory. The heavy fixtures `tmp/3dbenchy.stl` and
`tmp/base.stl` are user-supplied and never committed; everything else here is.

```bash
# 1. uninstrumented corroboration walls (1 warmup + N measured per cell)
python run-walls.py --cells benchy-classic-off,benchy-classic-on,base-classic-off,base-classic-on \
    --runs 2 --out walls.csv

# 2. one instrumented pass per cell (attribution only)
python run-instrumented.py --cells benchy-classic-off,benchy-classic-on,base-classic-off,base-classic-on
#    optional share-stability repeat:
python run-instrumented.py --cells benchy-classic-off --suffix .rep2

# 3. reduce raw event streams -> phase-split.json
python reduce.py

# 4. arithmetic -> counterfactuals.json
python arithmetic.py

# 5. verify (checkout-only; inputs are the committed files)
python verify.py            # 0 = every claim holds
python verify.py --self-test  # negative controls must all be detected
```

Raw event streams and G-code land in `target/t104-instrumented/` (gitignored);
`phase-split.json` and `counterfactuals.json` are the committed reductions.

## Regimes, and why they are never mixed

- **Walls** come from `run-walls.py` (uninstrumented, process creation-to-exit).
  In this take they were measured under **disclosed external load** (a sibling
  checkout's integration-test binary, ~10 of 12 cores), so they are
  **corroboration only** and are reported as a load-inflation factor beside the
  shares. The **ratio of record** and every counterfactual denominator come from
  ticket 103's paired, quiet, today-vs-today medians
  (`evidence/t103-orca-rebaseline/summary.json`).
- **Shares** come from the instrumented passes. `--instrument-stderr` inflates
  wall, so a phase's implied wall is `share x ratio-of-record median`, never a
  subtraction of an instrumented bracket from an uninstrumented wall.
- Instrumented passes are validated: exit 0, `degraded=false`,
  `non_fatal_error_count=0`, and G-code SHA-256 equal to the t87 post-t81
  reference set (`evidence/t91-exact-z-memo/references.json`), which
  `verify.py` re-asserts.
