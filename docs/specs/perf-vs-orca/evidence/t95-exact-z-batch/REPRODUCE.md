# Reproducing the t95 evidence

Two layers, as the map's evidence policy requires: the committed reduced set
verifies from a plain checkout, and the raw capture tree is only the fuller
optional source.

## Checkout-only (no model fixtures, no build)

```bash
cd docs/specs/perf-vs-orca/evidence/t95-exact-z-batch
python verify.py --self-test
```

Reads only the committed files: `proof-rows.json`, `measure-rows.json`,
`captures/*.probe.log`, `provenance/*.json`, and the post-t81 reference set in
`../t91-exact-z-memo/references.json`. Re-derives `qualification.json`,
`ab-reduction.json`, `phase1.json` and `summary.json`, checks every headline
claim, and rejects seven negative controls.

`python reduce.py` reproduces the four JSONs from the committed rows.

## Raw captures (needs `tmp/base.stl`, both standing trees, ~3 h)

The probe patch applies to the **pre-candidate tree** (`8f19a6a9`), because the
candidate rewrote the same regions of the two files; check it out from the
candidate commit's parent to re-run Phase 1.

```powershell
# 1. probe (Phase 1) — probe.patch must be applied to the pre-candidate tree
pwsh -File build-probe.ps1
pwsh -File run-probe.ps1 -Generator classic
pwsh -File run-probe.ps1 -Generator classic -Round 2
pwsh -File run-probe.ps1 -Generator arachne

# 2. the two arms' snapshots (baseline stashes the candidate sources and pops)
pwsh -File build-snapshots.ps1 -Arm baseline -Modes both
pwsh -File build-snapshots.ps1 -Arm candidate -Modes both

# 3. the 16-cell proof matrix, then the four-cell paired campaign
pwsh -File run-all-proof.ps1
python reduce.py --raw          # writes qualification.json
pwsh -File run-all-measure.ps1
python extract.py --raw
python reduce.py --raw
```

Captures land under
`.local-artifacts/perimeter-reference-preparation/t95-exact-z-batch/`
(gitignored), one `rows.csv` per cell. `run-cell.ps1` enforces the pre-declared
timing set (base supports-on only), the one-shot cell guard, clean-output and
isolation checks, the reference SHA, and the vacuous-role guard.
