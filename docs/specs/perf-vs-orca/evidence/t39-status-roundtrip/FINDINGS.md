# Acceptance runner CSV status preservation

Captured 2026-09-30. Runner-only repair; no slicer or timing campaign is run by
these checks. All test timings are synthetic, not performance evidence.

## Root cause and repair

The original [preflight probe](../t38-adoption-preflight/probe-status-roundtrip.ps1)
was reproduced in this session before editing the runner: exit 1, input
non-fatal counts 1/2 per row became 0/0, totals became 0/0, and the actual
decision was `KEEP` rather than `DROP`.

`Convert-BenchRows` (`resources/perimeter-acceptance/run-acceptance.ps1`)
omitted the producer's `non_fatal_error_count` spelling from its aliases.
`Get-PropertyValue` returned null and `Convert-ToCount` substituted zero.
The repair recognizes that exact column while retaining the older aliases.
The neighboring-column audit also found the producer's `validated_generator`
was omitted; it is now recognized before the existing marker aliases and
G-code fallback, so independently validated CSV disagreement is not hidden.

`completion_status`, `fatal_error_count`, and `degraded` already map correctly.
Their retained values and saved summaries are covered by the regression.
The producer (`resources/perimeter-acceptance/run_bench.ps1`), decision table,
sample schedule, exactness tolerances, and CPU/wall thresholds are unchanged.
Permissive legacy count conversion and missing-marker fallback are unchanged;
this is a repair of the actual validated producer schema, not a redesign of
legacy CSV validation.

## Regression seam

`Read-TestRows` (`resources/perimeter-acceptance/test-status-roundtrip.ps1`)
derives the real producer header, writes synthetic CSVs, and passes them through
the actual `Convert-BenchRows`. The script loads production functions via the
PowerShell parser without executing the runner's campaign entry point.
It invokes `New-CellSummary`, `Get-OverallDecision`, and `Write-Summary`, then
reads the saved JSON. Assertions check retained counts and status, aggregate
counts, validated markers, and decisions. No Rust mirror substitutes for this
path. Scratch outputs are isolated under `target/perimeter-acceptance-status-test/`.

Before the repair the new regression exited 1 with
`nonfatal-count-changed/baseline non-fatal count: expected '1', got '0'`.
After the repair all 13 cases passed: differing non-fatal, completion status,
degraded flag, either-side fatal, or validated generator reject; identical
clean/degraded status keeps only with favorable separated CPU and wall ranges;
exactness failure drops; touching CPU or wall ranges are inconclusive; adverse
disjoint CPU or wall ranges drop. Synthetic `KEEP` cases are regression controls,
not approval of production adoption.

The original probe now reports counts 1/2 per row, totals 4/8, and `DROP`.

## Validation

| Command | Result |
| --- | --- |
| `pwsh -NoProfile -File resources/perimeter-acceptance/test-status-roundtrip.ps1` | Exit 0; all 13 cases pass, captured in `regression.log`. |
| `pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t38-adoption-preflight/probe-status-roundtrip.ps1` | Exit 0; original mismatch now returns `DROP`, captured in `status-roundtrip.json`. |
| `pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -DryRun` | Exit 0; six cells, overall `DROP`, first cell `inconclusive`, second exactness `DROP`, automatic commit false; parsed and asserted, captured in `dry-run.synthetic.json`. |
| `cmp resources/perimeter-acceptance/run_bench.ps1 docs/specs/perf-vs-orca/evidence/alloc-bench/run_bench.ps1` | Exit 0; producer copies remain byte-identical. |
| `cargo test -p slicer-runtime --test integration -- overlap_is_inconclusive_and_never_keep --exact --nocapture` | Exit 0; one test ran and passed (unchanged Rust decision-table mirror), captured in `rust-decision-table.log`. |
| `git --no-pager diff --check` | Exit 0. |

The Cargo invocation combined stdout/stderr through `tee target/test-output.log`;
that log was read and copied into the evidence directory. It reported
future-incompatibility warnings for `nom` and `quick-xml`, not test failures.
The required sample-count assignment and the schema/function imports fail
loudly if they cannot be loaded; the regression never silently skips.

## Scope and remaining gates

No production Rust/guest code, build mode/default, reference output, or benchmark
producer changed. No automatic commit is authorized or performed. Real
exactness, campaign timings, complete snapshot provenance, guest freshness,
and packet-wide Rust/all-target gates are not established by this runner test.
The campaign still waits on
[Current-job adoption reference preparation policy](../../issues/40-acceptance-reference-refresh-policy.md).
