# Accelerated adoption campaign preflight

Captured 2026-09-30. **BLOCKED before building snapshots or running a timing
campaign.** Ordinary remains the production default. No production source,
acceptance threshold, reference output, or build default changed.

## Checks performed

| Check | Observed result |
| --- | --- |
| `rustc -vV` | Exact release/commit/host/LLVM identity matches the controlled policy; output in `toolchain.txt`. |
| `cargo xtask build-guests --check` | Exit 0; no console output (`ordinary-freshness.log`). |
| `cargo xtask build-guests --accelerated --check` | Exit 0; no console output (`accelerated-freshness.log`). |
| `cmp resources/perimeter-acceptance/run_bench.ps1 docs/specs/perf-vs-orca/evidence/alloc-bench/run_bench.ps1` | Exit 0; copies are byte-identical. |
| `pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -DryRun` | Exit 0; six synthetic cells, overall `DROP`, first cell `inconclusive`, second cell exactness `DROP`, automatic commit false. Saved as `dry-run.synthetic.json`; not timing evidence. |
| `python docs/specs/perf-vs-orca/evidence/t38-adoption-preflight/inspect-corpus.py` | Exit 0; all six model/config/reference input sets exist; all models match the map fixtures and config generator names match their cells. Saved as `corpus-inventory.json`; not current-job exactness approval. |
| `pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t38-adoption-preflight/probe-status-roundtrip.ps1` | Exit 1, intentionally red: expected `DROP`, actual `KEEP`. Saved as `status-roundtrip.json`. |

Fresh shared-target guests do not establish freshness/provenance of an existing
dist snapshot. Neither complete host-plus-guest snapshot was built in this take.

## Reproduced acceptance runner defect

`run_bench.ps1` (`resources/perimeter-acceptance/run_bench.ps1`) exports
`non_fatal_error_count`. `Convert-BenchRows`
(`resources/perimeter-acceptance/run-acceptance.ps1`) does not recognize that
column. `Get-PropertyValue` therefore yields null and `Convert-ToCount` converts
the missing value to zero.

The diagnostic imports the runner's actual function definitions using the
PowerShell parser, derives its sample count and the benchmark's actual CSV
header from their assignments, and feeds that CSV through `Convert-BenchRows`
and `New-CellSummary`. It never runs a slicer. Four baseline rows each have one
non-fatal error and four candidate rows each have two; both report the same
degraded status and generator. Synthetic CPU/wall ranges are favorable and
separated solely to isolate the status gate. Both parsed error totals become
zero, and `Get-CellDecision` returns `KEEP` instead of the required `DROP`.

This is a CSV schema round-trip defect, not a guest failure or a performance
result. The stock dry run cannot catch it because its synthetic rows bypass
`Convert-BenchRows`. No fix was applied in this take. Preserve this red-capable
seam as a regression check when repairing the runner; do not only test the Rust
decision-table mirror.

## Reference preparation requires a decision

The existing corpus is the older adoption campaign job, not the map's eight-cell
Orca comparison job: configs explicitly specify a 0.5 mm nozzle, 0.2 mm layer
height, three walls and 25% sparse infill. This difference is not itself a
reason to replace the adoption protocol with the Orca scoreboard.

The saved reference completion events date to 2026-09-11, before the map's
support-correctness and empty-infill-replacement repairs. Both base references
record `degraded=true` and 29,108 non-fatal errors. Benchy reference appendices
also contain the historical `wall_loops = 2` disclosure despite configs asking
for three walls; that is not proof of the actual reference path wall count.

Current outputs have **not** been sliced against these references in this take;
their exactness failure is not claimed. Their suitability for the current
supported job remains unverified. Existing reference files were not modified.
Do not silently regenerate them from accelerated candidate output. The human
must choose independently prepared current-job references or explicitly
authorize a frozen ordinary-reference refresh with provenance before resuming.

## Remaining gates

The canonical obligations are packet-254's AC-1, AC-2, AC-3, AC-3N, AC-5,
AC-N1, AC-N2 and AC-N3 in
`docs/spec_packets/254-exact-perimeter-spatial-queries/packet.spec.md`, its
all-target check/clippy and literal gates, plus AC-4's serial real campaign.
Those tests and build gates were **not run** in this preflight.

After clearing the prerequisites: build complete ordinary and accelerated
developer snapshots, record host/guest/policy/config provenance, re-run both
freshness gates, and provide explicit distinct executable and module paths to
the runner. Preserve strict favorable CPU **and** wall range separation in
every cell, exactness before retained timings, stop-on-failure/inconclusive,
and no automatic retry or production-default switch.

The packet's design records the `powi(2)` lowering premise for the policy
compiler; this take verified compiler identity, not a new lowering artifact.
No campaign measurements, output equivalence, or speed improvement are claimed.
