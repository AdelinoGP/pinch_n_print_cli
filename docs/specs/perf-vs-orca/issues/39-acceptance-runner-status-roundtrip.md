# Acceptance runner CSV status preservation

Type: task
Status: resolved
Assignee: agpen (OpenCode session)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Repair and regress the acceptance runner's CSV status round-trip so a change in
real harness non-fatal counts cannot earn `KEEP`.

`run_bench.ps1` (`resources/perimeter-acceptance/run_bench.ps1`) emits
`non_fatal_error_count`, but `Convert-BenchRows`
(`resources/perimeter-acceptance/run-acceptance.ps1`) does not recognize it and
silently substitutes zero. The actual reader plus decision functions return
`KEEP` for baseline count 1 versus candidate count 2 with otherwise compatible
status and favorable synthetic timing ranges. The dry run bypasses that reader.

Reproduction and captured result:
[Accelerated adoption campaign preflight](../evidence/t38-adoption-preflight/PREFLIGHT.md)
and [red-capable round-trip probe](../evidence/t38-adoption-preflight/probe-status-roundtrip.ps1).

Use the real benchmark CSV schema and actual PowerShell reader/decision path
as the regression seam, not only the Rust decision-table mirror. Preserve
reported counts in retained rows and summary, verify unchanged status is
accepted only when the other gates pass and differing status is rejected,
and do not weaken timing or exactness gates. Audit the neighboring status
columns while repairing the schema translation. Follow repo test discipline;
no production-default switch or automatic commit is authorized.

This prerequisite unblocks
[Accelerated production adoption acceptance campaign](38-accelerated-adoption-acceptance-campaign.md).

## Resolution comment — 2026-09-30

**Repaired and regressed.** `Convert-BenchRows`
(`resources/perimeter-acceptance/run-acceptance.ps1`) now recognizes the real
producer's `non_fatal_error_count`, preserving counts in retained rows and
the serialized summary. The original preflight probe was red before editing
(1/2 became 0/0 and earned `KEEP`) and is green afterward (1/2 preserved,
totals 4/8, `DROP`).

Neighboring-column audit found `validated_generator` was also missing. Its
explicit CSV value now takes precedence over older marker aliases and the
G-code fallback, so a mismatched validated marker cannot earn `KEEP`.
`completion_status`, `degraded`, and `fatal_error_count` already mapped
correctly and are covered by regression checks.

The new `Read-TestRows` regression seam
(`resources/perimeter-acceptance/test-status-roundtrip.ps1`) derives the actual
benchmark header and exercises the real PowerShell CSV reader, decision
functions, and JSON writer. It went red before the repair, then passed all 13
cases after it: count/status/marker changes and fatal errors reject; identical
status only keeps with exactness and strict favorable CPU **and** wall range
separation; exactness failure and adverse ranges drop, touching ranges remain
inconclusive. Added its command to the preparation guide so the stock dry run
(which bypasses CSV reading) is not the sole runner check.

Dry-run assertions, benchmark-copy comparison, the narrow Rust decision-table
test, and whitespace validation passed. Evidence and exact commands:
[Runner status round-trip findings](../evidence/t39-status-roundtrip/FINDINGS.md).
No decision-table rule, timing/exactness threshold, schedule, producer, reference,
production Rust/guest code, or default changed. No campaign timing, guest
freshness check, snapshot build, all-target gate, or automatic commit ran in
this take. Adoption remains blocked on
[Current-job adoption reference preparation policy](40-acceptance-reference-refresh-policy.md).
