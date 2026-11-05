---
when: Read this when running the packet scope of `spec-review`, either reviewing a `docs/spec_packets/` packet's implementation or before closing a packet. The mindset, trap, and verdict-floor rules in SKILL.md apply throughout.
keywords: packet review, acceptance criteria, scope traceability, design fidelity, closure, full, delta, delegated
---

# Packet Review (packet scope)

Review a packet under `docs/spec_packets/<prefix>_<NN>_<packet-name>/` against its `packet.spec.md`, its `implementation-plan.md`, and the implementation. Legacy packets also carry `requirements.md` and `design.md`; treat their scope lists and design constraints as part of the contract.

## Modes

- **Full** (default): the entire contract, code surface, and verification set. The only mode that may authorize closure.
- **Delta**: the changed steps and files, plus the criteria, constraints, and commands they affect, including cross-step regressions they cause. Used between swarm iterations. Never a final signoff.
- **Delegated**: when swarm offloads review, exactly one reviewer runs after all code workers finish. Do not shard review by step or dimension.
- **Preflight** (`--preflight`): run `../../spec-packet-generator/references/preflight-checklist.md` and emit its report, and nothing else.

## Workflow

1. **Read the packet** in full, then pin the change: `git diff --stat` plus the commit list for the packet's work.
2. **Preflight pass.** Run checklist items 1, 2, 4, 7, and 8 against the tree, plus the two criterion checks below, in delta mode too: authoring defects are global and cannot be scoped away. Report fictional or wrong-shape names and ADR collisions as Critical and the rest as High. Stop tracing criteria whose symbols are fictional until the failure is acknowledged.
   - Each criterion's test exercises the production path it describes (trap: placeholder tests).
   - A criterion proven by a helper's unit test has a production call site for that helper (trap: helper passes, driver never calls it).
3. **Docs impact.** New packets have a section that is either `none` with an eligible reason or a list of doc sections with checks. Review legacy packets against their own docs-impact format. `none` claimed for an IR, WIT, scheduler, claim, manifest, host-service, or SDK change is HIGH. Verify the required doc edits before closure.
4. **Trace each dimension below**, then run verification, then write the report.

## Review dimensions

### 1. Scope coverage (Critical)
- The implementation reaches the Goal. Every "In" item is addressed, every "Out" item is untouched (check the diff against out-of-scope paths), and boundary edits are justified.

### 2. Acceptance criteria (Critical)
For each criterion:
- The behaviour exists, and its command passes, or there is an explicit reason it does not.
- The test exists and asserts the content the criterion names.
- Negative criteria are implemented and verified where the packet requires them.
- "Mostly done" is incomplete.

### 3. Scope traceability (Critical)
- Each scope item maps to specific code (file + symbol). Flag orphans (stated, not implemented) and unrequested implementations (done, not stated).

### 4. Design fidelity (High)
- Design notes constraints, invariants, and the selected approach are respected; there are no ad-hoc workarounds.
- Module stage assignments match documented stage IDs, and IR field paths match `crates/slicer-ir/src/` exactly.
- The diff hits the files the plan expects: surprises are explained, and tests and fixtures are updated.

### 5. Implementation completeness (Critical)
- Each plan step reached its done-when, and its verify command passes.
- A superseded predecessor is reconciled (`status: superseded`, `superseded_by:`).
- A plan file's `## Packet Queue` row is generation-scoped (`pending` / `generated` / `blocked` / `superseded`). A row still `generated` for an `implemented` packet is expected, not a gap.

### 6. Verification quality (High)
- Every documented command runs green with the expected output. The acceptance tests exist, pass, and cover the full criteria. No test is skipped for completed work.

### 7. Deviations (Medium)
- Open questions are answered or tracked. Each divergence from the packet is recorded under `## Deviations` with its rationale, and critical deviations carry waivers in `docs/DEVIATION_LOG.md` where required.

### 8. Documentation and parity (Medium)
- Referenced docs exist and are accurate. Check any OrcaSlicer parity obligations against available canonical sources.

## Running verification

Check every criterion command and every Verification command, using results from the same unchanged tree or running the missing checks. Each PASS row quotes its result line. A command that was not run, was blocked by the environment, or was missing from a batch return is `[unverified]`, with the blocker quoted, and the verdict floor applies. Delta mode reruns only the commands the changed steps affect. `cargo test --workspace` follows SKILL.md's Tests rule.

## Delegated return (swarm-invoked)

Return compactly:
- mode and delta scope;
- blocking findings by severity;
- impacted steps, files, and criteria;
- every command with pass/fail and **the evidence line behind each PASS**;
- one verdict with one short rationale paragraph.

## Verdict semantics

- **APPROVED**: complete and correct; authorizes closure only in full mode. Every criterion PASS with evidence, zero `[unverified]` load-bearing rows, preflight clean.
- **APPROVED WITH NOTES**: complete, with non-blocking improvements noted.
- **CHANGES REQUESTED**: specific changes are needed before re-review.
- **BLOCKED**: critical issues need significant rework or a design decision.
- **DEFERRED**: the review could not complete. Emit the partial findings, the remaining checks, and the next-session entry point.
