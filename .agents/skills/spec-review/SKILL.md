---
name: spec-review
description: Adversarial, evidence-based review against this repo's architecture contract, in three scopes. Packet scope covers a spec packet's `--preflight` authoring check or a full/delta closure review. Session scope is a SHIP / DO NOT SHIP audit of this session's work. Code scope reviews the changes since a ref. Use when reviewing, auditing, or verifying a packet; for "what did I miss", "are we ready to ship", "review my changes", or "review since X".
type: anthropic-skill
version: "4.0"
metadata:
  internal: true
---

# Review

One skill, three scopes. The rules below apply to every scope; each scope's workflow lives in its reference file. Read that file before starting.

**Scope target**: `$ARGUMENTS`. A packet name or path means packet scope; file paths, `all-changes`, or a git ref mean code scope; a session-audit request means session scope.

| Scope | Reviewing what vs. what | Verdict | Workflow file |
|---|---|---|---|
| **packet** | a packet's implementation vs. its `packet.spec.md` + `implementation-plan.md` | APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED / BLOCKED / DEFERRED | `references/packet-review.md` |
| **packet `--preflight`** | a draft packet vs. the tree | PREFLIGHT PASS / PREFLIGHT BLOCKED | `../spec-packet-generator/references/preflight-checklist.md` |
| **session** | this session's diff and working tree vs. the packet being implemented | SHIP / DO NOT SHIP | `references/session-audit.md` |
| **code** | uncommitted changes, named files, or the diff since a ref vs. the architecture contract | APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED | `references/code-review.md` |

Pick by target, not phrasing; when it is genuinely ambiguous, ask. **Only a full packet-scope review can authorize closure.**

Delegate large reads and noisy cargo output to subagents, and keep only their conclusions.

## Mindset

You are a senior engineer reviewing cold. In session scope you wrote the code, which is more reason to distrust memory: **session memory is not evidence**. `git diff`, `rg`, file reads, and command results are.

- Bias toward finding problems. **The burden of proof is on the implementation.** A claim without evidence is `[unverified]`; a criterion without passing evidence is FAIL, never PASS by default.
- A subagent's "pass" that quotes no evidence (file:line, assertion, or result line) is not evidence. Re-check it.
- **Re-verify on disagreement.** A re-check wins over an earlier claim; say so openly. Re-run a result that "feels wrong" with absolute repo-root paths before concluding. One false sub-claim does not condemn a sweep: re-verify each load-bearing claim on its own.
- **Never fabricate `file:line`.** If you cannot open the file and see the symbol, the row is `[unverified]`.
- **Findings, not fixes.** Review is read-only; an obvious bug is a finding. The one exception: session scope may append to the packet's `## Deviations` after explicit user confirmation.
- Give specific fixes, not "improve this". Acknowledge good work.
- Launch all your checks before adjudicating, and emit the report once, after every check has returned or been marked `[unverified]`.

## Verdict floors

Leniency is this skill's historical failure mode; these floors are hard.

- Any `[unverified]` load-bearing row (criterion, scope item, or session change) caps the verdict at CHANGES REQUESTED / DO NOT SHIP.
- Any Critical finding means CHANGES REQUESTED, BLOCKED, or DO NOT SHIP.
- Delta review never authorizes closure. A full review that cannot finish is DEFERRED with a handoff; it never waives review.
- A failed preflight item 1, 2, or 4 (fictional or wrong-shape names, ADR collisions) means the packet cannot be APPROVED.

## Known traps

Apply every gotcha in [`docs/spec_packets/README.md`](../../../docs/spec_packets/README.md) in every scope. Four of them decide verdicts:

- **Helper passes, driver never calls it**: zero non-test call sites is a HIGH finding, and the criterion is PARTIAL regardless of green tests.
- **Placeholder tests**: read the test body. Assertions that never touch the claimed symbols are a HIGH finding, and the criterion is PARTIAL.
- **Unregistered test file**: confirm the `mod` registration before believing a green "0 tests" run.
- **Stale guest WASM**: run `cargo xtask build-guests --check` and judge by exit code before blaming a guest or dispatch failure on the change or on flakiness.

## Tests

Use the narrowest command that proves a claim. If no narrow test exercises a change, say so. Follow AGENTS.md §Test Discipline for feature gates, output capture, `--all-targets`, and when workspace tests are permitted.

## Output

Report templates and verdict semantics are in `references/output-format.md`; read it when writing the report. Every scope ends in exactly one verdict. Session scope's verdict is one line with no trailing prose.
