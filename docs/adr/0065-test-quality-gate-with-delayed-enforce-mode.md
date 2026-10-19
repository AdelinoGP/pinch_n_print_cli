# ADR-0065 Test-quality gate ships in report mode, enforce mode after remediation

Status: accepted (2026-09-09, test-quality remediation program).

## Context

`docs/21_data_defaults_and_fixtures.md` exists because prose guidance alone did
not stop struct-literal churn — it had to become the `check-literals` gate. The
test-quality audit found the same dynamic for false-green test patterns. But the
new gate's rules (`cargo xtask check-test-quality`, R1–R8 in
`docs/22_test_quality.md`) match existing tests that are legitimate: compile
witnesses, documented count pins, deliberate opt-in probes. Enforcing on day one
would force writing a large waiver backlog before any false green is actually
repaired — and bulk-written waivers are rubber stamps, not reviews.

## Decision

The gate ships in `--report` mode: it prints findings and exits 0, and
`cargo xtask test` runs it in its preflight non-blockingly. Each crate wave of
the remediation program closes with zero unwaived findings for that crate
(waivers written per-test during the earn-their-keep review, per ADR-0064).
Enforce mode — exit 1 on unwaived findings, preflight-blocking — is switched on
only when the remediation program's final wave closes. The single promotion
point is the `TEST_QUALITY_ENFORCED` constant in `xtask/src/test.rs`.

## Consequences

Until promotion, the gate cannot block a bad commit; the promotion step is a
required deliverable of the program's final wave, not an optional follow-up.
The finding list is the shared work queue: a crate wave that closes with
findings still open has not met its exit criterion.