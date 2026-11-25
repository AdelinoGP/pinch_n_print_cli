---
when: Read when reporting a spec-review in packet, session, or code scope.
keywords: report, evidence, verdict, findings
---

# Review Output Formats

Report findings with severity, file and symbol, the evidence, the impact, and a concrete fix. Include evidence for every passing contract item; mark checks not run as `[unverified]` with the reason. Use these structures, omitting empty optional sections.

## Packet scope

```markdown
## Spec Review: <packet>
Packet: <path> · Status: <status> · Mode: full / delta · Delta scope: <if applicable>

### Summary
<assessment of the implementation against the packet>

### Contract and design
| Goal, scope item, AC, legacy requirement, or design constraint | Status | Code and test evidence |
|---|---|---|
| <item> | PASS / PARTIAL / FAIL / UNVERIFIED | <file, symbol, assertion or result> |

### Plan and verification
| Step or command | Result | Evidence |
|---|---|---|
| <step or command> | PASS / PARTIAL / FAIL / UNVERIFIED | <done-when, result, or blocker> |

### Findings
<critical, high, then medium findings; note documentation, deviations, and parity where relevant>

### Positive observations
<optional>

### Verdict
APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED / BLOCKED / DEFERRED
<reason and remaining work, if any>
```

Preflight mode uses the report in `../../spec-packet-generator/references/preflight-checklist.md`.

## Session scope

Use exactly these sections, then the one-line verdict:

```markdown
## Session Audit: <packet>

### 1. DEFERRED / INCOMPLETE
<location, missing work and reason; or `None — all packet items implemented.`>

### 2. PRODUCTION READINESS
<location, change, READY or NOT READY and reason, for each session change>

### 3. PACKET DEVIATIONS
<spec reference — Specified: X | Implemented: Y | Reason: Z>

SHIP
```

If the work is not ready, replace `SHIP` with `DO NOT SHIP — <biggest blocker(s), max 2>`.

## Code scope

```markdown
## Code Review: <scope>
### Summary
<assessment>
### Findings
<critical, high, and medium findings>
### Smell Baseline
<non-blocking judgement calls, or `None spotted.`>
### Positive Observations
<optional>
### Automated Check Results
| Check | Status | Evidence |
|---|---|---|
| <check> | PASS / FAIL / UNVERIFIED | <result or blocker> |
### Verdict
APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED
```
