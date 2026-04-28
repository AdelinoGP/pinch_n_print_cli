---
when: Read this when you reach the report-writing stage of `spec-review` and need the exact section template for the review report.
keywords: report template, output format, verdict, findings tables, review sections
---

# Spec Review Output Format

Use this exact structure when emitting the review report.

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
- **[GOOD]** Clear documentation of [aspect]

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
