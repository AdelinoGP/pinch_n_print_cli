---
when: Read this when you reach the report-writing stage of any `spec-review` scope and need the exact report template — packet review, session audit, or code review.
keywords: report template, output format, verdict, findings tables, review sections, ship, do not ship
---

# Review Output Formats

One template per scope. Use the exact structure for the scope you are running.

## Packet scope

```
## Spec Review: [Packet Name]

**Packet Path**: `path/to/packet`
**Status**: `draft | in-review | implemented | blocked`
**Review Mode**: `full | delta`
**Delta Scope**: `changed_steps=[...] changed_files=[...]`
**Reviewed**: YYYY-MM-DD
**Reviewer Context Cost**: <S / M / L>; remaining budget at report time: <%>

---

### Summary

[2-3 sentences: overall assessment of spec fulfillment quality]

---

### Spec Contract Review

#### Goal Verification
| Criterion | Status | Evidence (dispatch return) |
|----------|--------|----------------------------|
| Goal fulfilled | PASS/PARTIAL/FAIL | Brief evidence |
| No goal creep | PASS/FAIL | Brief evidence |
| Scope boundaries respected | PASS/FAIL | Brief evidence |

#### Acceptance Criteria Check
| Criterion | Status | Trace (file:line from dispatch) | Assertion Quality |
|-----------|--------|----------------------------------|-------------------|
| AC-1: [criterion text] | PASS/PARTIAL/FAIL | [file:function or test] | Exact / Vague / Missing |
| AC-2: [criterion text] | PASS/PARTIAL/FAIL | [file:function or test] | Exact / Vague / Missing |

#### Requirements Traceability
| Requirement | Status | Implementation Trace |
|-------------|--------|---------------------|
| REQ-1: [requirement text] | COMPLETE/PARTIAL/MISSING | [file:function] |
| REQ-2: [requirement text] | COMPLETE/PARTIAL/MISSING | [file:function] |

---

### Design Fidelity Review

#### Architecture Constraints
| Constraint | Status | Notes |
|------------|--------|-------|
| [constraint from design.md] | COMPLIANT/VIOLATED | [evidence] |

#### Controlling Code Paths
| Expected Path | Status | Notes |
|---------------|--------|-------|
| [path from design.md] | CHANGED/UNCHANGED/SURPRISE | [what changed] |
| [test fixture path] | UPDATED/MISSING/OK | [notes] |

---

### Implementation Completeness

#### Step Execution
| Step | Status | Verification | Exit Condition |
|------|--------|--------------|----------------|
| Step 1: [name] | DONE/PARTIAL/SKIP | [verification evidence] | Met / Weak / Missing |
| Step 2: [name] | DONE/PARTIAL/SKIP | [verification evidence] | Met / Weak / Missing |

#### Task Map Resolution
| Task ID | Status | Evidence |
|---------|--------|----------|
| TASK-XXX | COMPLETE/INCOMPLETE/MISSING | [evidence] |

---

### Critical Issues

1. **[CRIT-1]** `file:line` — **[Issue]** — **Fix:** ...
2. **[CRIT-2]** ... — **[Issue]** — **Fix:** ...

---

### High Priority Items

1. **[HIGH-1]** ... — **Fix:** ...
2. **[HIGH-2]** ... — **Fix:** ...

---

### Medium Priority Items

1. **[MED-1]** ... — **Consider:** ...
2. **[MED-2]** ... — **Consider:** ...

---

### Positive Observations

- **[GOOD]** Description of well-implemented aspect
- **[GOOD]** Strong test coverage for [feature]

---

### Verification Results

| Check | Result | Details |
|-------|--------|---------|
| Verification cmd 1 | PASS/FAIL | [output summary, ≤ 1 line; dispatch returned FACT/SNIPPETS] |
| Verification cmd 2 | PASS/FAIL | [output summary] |
| Build | PASS/FAIL | [if applicable] |
| Tests | PASS/FAIL | [test count] |

---

### Recommendations

1. **[RECOMMENDATION]** [Actionable recommendation]
2. **[RECOMMENDATION]** [Actionable recommendation]

---

### Verdict

| Level | Decision |
|-------|----------|
| **Critical Issues** | [N outstanding] |
| **High Priority Items** | [N outstanding] |
| **Overall Verdict** | **APPROVED** / **APPROVED WITH NOTES** / **CHANGES REQUESTED** / **BLOCKED** / **DEFERRED** |
```

Preflight mode (`--preflight`) uses the gate report format in `references/preflight-gate.md` instead.

## Session scope

Exactly these three sections, in this order, no additions, then the one-line verdict with no trailing prose.

```
## Session Audit: [Packet Name]

### 1. DEFERRED / INCOMPLETE

- `file:line` — [what's missing] — [why deferred, if known]
(or, literally: None — all packet items implemented.)

### 2. PRODUCTION READINESS

- `file:line` — [one-line description] — **READY** / **NOT READY** *(reason)*

### 3. PACKET DEVIATIONS

- [Spec ref] — Specified: X | Implemented: Y | Reason: Z

SHIP
(or)
DO NOT SHIP — <biggest blocker(s), max 2>
```

## Code scope

```
## Code Review: [Feature/Module/Files]

### Summary
[2-3 sentences: overall quality assessment]

### Critical Issues (must fix before commit)
1. [CRIT-1] `file:line` — Description — **Fix:** ...

### High Priority (should fix)
1. [HIGH-1] `file:line` — Description — **Fix:** ...

### Medium Priority (consider fixing)
1. [MED-1] `file:line` — Description — **Fix:** ...

### Smell Baseline (judgement calls — non-blocking)
1. [SMELL-1] `file:line` — Possible [smell name] — [quoted hunk or one-line rationale] — **Refactor:** ...
(or, literally: None spotted.)

### Positive Observations
- [Good] Description of well-implemented pattern

### Automated Check Results
| Check | Status | Details |
|-------|--------|---------|
| Build | PASS/FAIL | ... |
| Tests (targeted) | PASS/FAIL | crates/files exercised, X passed, Y failed |
| Clippy (--all-targets) | PASS/FAIL | ... |
| Check (--all-targets) | PASS/FAIL | ... |

### Verdict
[APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED]
[If changes requested: list critical/high items that must be addressed]
```