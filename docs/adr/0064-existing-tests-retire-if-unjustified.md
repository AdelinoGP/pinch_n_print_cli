# ADR-0064 Existing tests retire if unjustified

Status: accepted (2026-09-09, test-quality remediation program).

## Context

A five-model test audit (consolidated in `docs/specs/test-quality-remediation-plan.md`)
produced hundreds of keep-vs-retire candidates. Its consolidation pass found that
keep-by-default is the failure mode: audits flag a test as removable, someone
labels it "meaningful" without naming what it protects, and the test survives to be
re-flagged by the next audit. The same pass also found the inverse error —
five models repeatedly recommended deleting tests that DO exercise production
behavior (compile witnesses, edge-case probes, layer-distinct duplicates),
because "small test" was read as "no value".

## Decision

For every test surfaced by the remediation program, the burden of proof is on the
test, not the auditor: a test earns its keep only if a reviewer, after reading the
test body and its references, can name the specific regression input that would
slip through if the test were deleted. If no such input can be named, the test is
retired — regardless of how long it has existed or who wrote it.

Two carve-outs keep the standard honest:

- **Compile witnesses** earn their keep by naming the API surface whose only
  compile check they carry; they are marked with a `// test-quality:` waiver and
  reviewed as witnesses, not as behavior.
- **Protected parity/baseline evidence** (Arachne parity families, golden
  outputs, self-captured baselines) is adjudicated under the parity rules of
  `AGENTS.md` — canonical correctness first — and never loosened to satisfy
  this standard.

Coverage shrinkage is bounded and auditable: Wave 0 of the remediation program
records a per-target test-discovery census before any change, and every retirement
must reconcile against it as an accounted delta.

## Consequences

- Future audits must consult the program's disposition ledger
  (`docs/specs/test-quality-remediation-plan.md` §Ledger) instead of re-litigating
  settled KEEP decisions — unless they can name a regression input the ledger's
  keep rationale missed.
- New tests written by LLM agents are held to the same standard at authoring time
  via `docs/22_test_quality.md` and the `cargo xtask check-test-quality` gate.