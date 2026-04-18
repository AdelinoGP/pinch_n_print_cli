---
name: spec-review
description: Review and verify a spec packet implementation against its packet.spec.md, requirements.md, design.md, and implementation-plan.md. Performs thorough critique comparing spec against actual code implementation, identifying gaps, missing pieces, and deviations.
type: anthropic-skill
version: "1.1"
metadata:
  internal: true
---

# Spec Review Command

## Overview

Review and verify a spec packet implementation against its full documentation suite. This is the quality gate that validates spec-driven development was executed correctly.

**Review scope**: `$ARGUMENTS` — either a spec packet name (e.g., `manifest-ir-access-and-config-schema`) or a path to a packet directory.

Default review mode is full-packet review. Swarm may also invoke a delta review for intermediate iterations by supplying changed steps or changed files. Delta review is for iteration speed only; a final packet-close decision still requires a full review.

## Review Modes

- **Full review**: Read and evaluate the full packet and all applicable implementation evidence. This is the default and the only mode that may authorize packet closure.
- **Delta review**: Focus on `changed_steps`, `changed_files`, and directly affected acceptance criteria, requirements, design constraints, and verification commands. Use this only for intermediate Swarm loops.
- **Delegated review**: If Swarm offloads review, use exactly one sequential review subagent after all code workers finish. Do not shard review by packet step or by review dimension.

---

## Gathering Context

### Step 0: Preflight — Acceptance Criteria Completeness Check

Before reading packet files or running any verification, verify every acceptance criterion in the packet has a pipe-suffixed runnable verification command.

This preflight always runs, including delta review. Packet-authoring defects are global and cannot be scoped away.

For each criterion in `packet.spec.md`:
- [ ] Criterion ends with `|` followed by a concrete command
- [ ] The command uses correct paths, flags, and module names
- [ ] The command is runnable in the current environment without additional context
- [ ] The criterion names exact assertion content rather than generic phrases like "all required fields" or "correct diagnostics"

Packet-quality preflight:
- [ ] The packet includes at least one negative or rejection criterion when the slice changes validation, enforcement, or failure behavior
- [ ] `design.md` selects one implementation approach when more than one is plausible
- [ ] `implementation-plan.md` gives each step an explicit exit condition, precondition, or postcondition
- [ ] Open questions that would change implementation scope are resolved, or the packet is still `draft` and the blocker is explicit

**If any criterion lacks a per-criterion verification command:**
- Mark it as a **HIGH** finding (not MED).
- In the report table, mark it `PARTIAL/INCOMPLETE`.
- Do not proceed to full review until the finding is logged.

**If the packet fails the packet-quality preflight:**
- Treat the packet as under-specified before evaluating implementation completeness.
- Log a **HIGH** finding for each missing packet-quality element.
- Continue the review only after clearly separating packet-authoring defects from implementation defects.

### Step 2: Identify the Spec Packet

If given a packet name, locate it:
```bash
ls .ralph/specs/
```

If given a path, validate it contains packet files:
- `packet.spec.md` — Packet contract (goal, scope, acceptance criteria)
- `requirements.md` — Problem statement and requirements
- `design.md` — Architecture constraints and design decisions
- `implementation-plan.md` — Step-by-step execution plan
- `task-map.md` — Task ID to packet step mapping

```bash
# Validate packet structure
ls [packet-dir]/
```

### Step 3: Read All Spec Documents

Read in this order:
1. `packet.spec.md` — Primary contract (goal, scope, acceptance criteria, verification commands)
2. `requirements.md` — Problem statement and acceptance summary
3. `design.md` — Controlling code paths and architecture constraints
4. `implementation-plan.md` — Step-by-step plan (each step maps to task IDs)
5. `task-map.md` — Bridge to backlog task IDs

### Step 4: Determine Implementation Artifacts

From `design.md`, identify:
- **Primary code paths** — What files were expected to change?
- **Test/fixture paths** — What tests serve as acceptance gates?
- **Authoritative docs** — What docs validate correctness?

From `implementation-plan.md`, identify:
- **Steps 1-N** — Each step's objective and verification command
- **Step exit criteria** — What proves each step is actually complete?
- **Files expected to change per step**

### Step 5: Build the Review Scope

If running in full review mode:
- Review the entire packet contract, code surface, verification set, and task-map impact.

If running in delta review mode:
- Identify `changed_steps` and `changed_files` from the caller.
- Expand the scope to direct dependencies and directly affected acceptance criteria.
- Include any requirement, task-map, or design-fidelity checks that those changes can invalidate.
- Do not pretend unchanged sections are approved for packet closure; delta review is a focused checkpoint, not a final signoff.

---

## Mode-Specific Guidance

- In full review, complete all core review dimensions across the full packet.
- In delta review, keep the review focused on the affected slice, but still check for cross-step regressions caused by that slice.
- A delta review may conclude that the changed slice is clear or still blocked, but it does not replace the final full review required before moving a packet to `implemented`.

## Core Review Dimensions

### 1. Scope Coverage (Critical)

**Goal Verification:**
- [ ] The implemented code actually fulfills the stated goal in `packet.spec.md`
- [ ] No goal creep — implementation doesn't exceed scope
- [ ] No scope gaps — everything in-scope is addressed

**In/Out of Scope Check:**
- [ ] Items listed as "in scope" are actually addressed
- [ ] Items listed as "out of scope" are genuinely not touched
- [ ] Any boundary items are explicitly noted with justification

### 2. Acceptance Criteria Fulfillment (Critical)

For each acceptance criterion in `packet.spec.md`:
- [ ] **Given/When/Then** structure is met by implementation
- [ ] Verification command passes (or explanation of why not yet)
- [ ] Test coverage exists for the criterion
- [ ] No partial fulfillment — "mostly done" counts as incomplete
- [ ] The criterion's promised assertion content is actually proven by the test or command output
- [ ] Negative or rejection criteria are implemented and verified when the packet requires them

### 3. Requirements Traceability (Critical)

For each requirement in `requirements.md`:
- [ ] Trace requirement to specific code files/functions
- [ ] Verify no orphaned requirements (stated but not implemented)
- [ ] Verify no extra implementations (done but not required)

**Acceptance Summary Check:**
- [ ] All bullets in acceptance summary are addressed
- [ ] Each bullet has corresponding verification
- [ ] Measurable outcomes in `requirements.md` are actually measurable in the evidence provided

### 4. Design Fidelity (High)

**Architecture Constraints:**
- [ ] Implementation respects architecture constraints in `design.md`
- [ ] Module stage assignments match documented stage IDs
- [ ] No ad-hoc workarounds that violate documented constraints
- [ ] Locked assumptions and invariants called out by the packet are preserved

**Controlling Code Paths:**
- [ ] Changes made to expected files (no surprise changes)
- [ ] No changes to unexpected files without justification
- [ ] Test/fixture files properly updated
- [ ] The implementation follows the selected approach from `design.md`, not an unreviewed alternative

**Data & Contract Notes:**
- [ ] IR field paths match exact names in `crates/slicer-ir/src/`
- [ ] Type constraints followed (e.g., config schema types)
- [ ] Stage ordering and tiering respected

### 5. Implementation Completeness (Critical)

**Step Execution:**
- [ ] Each step in `implementation-plan.md` was executed
- [ ] Steps executed in logical order
- [ ] Each step achieved its stated objective
- [ ] Verification commands documented and passing
- [ ] Each step satisfied its explicit precondition, postcondition, and exit condition
- [ ] Read-only discovery steps produced the exact inventory or decision the packet said they would produce

**Task Map Traceability:**
- [ ] Each task ID from `task-map.md` corresponds to completed work
- [ ] No unmapped task completions or gaps
- [ ] Backlog source (e.g., `docs/07_implementation_status.md`) updated appropriately
- [ ] Reopened or superseded packet work is reconciled explicitly rather than implied

### 6. Verification Quality (High)

**Verification Commands:**
- [ ] All documented verification commands run successfully
- [ ] Commands produce expected outputs
- [ ] No hard-coded assumptions in verification

**Test Coverage:**
- [ ] Acceptance gate tests exist and pass
- [ ] Tests cover the full acceptance criteria
- [ ] No skipped tests for completed work
- [ ] Tests are properly integrated into CI

### 7. Deviation Documentation (Medium)

**Open Questions Resolution:**
- [ ] All open questions in `design.md` are answered or tracked
- [ ] Answers are documented (code comments, doc comments, or deviation log)
- [ ] No packet marked `active` or `implemented` still depends on unanswered questions that would change implementation scope

**Known Risks:**
- [ ] Identified risks are mitigated or documented
- [ ] Tradeoffs documented with rationale

**Undocumented Deviations:**
- [ ] Implementation deviations from spec are documented
- [ ] Deviations have explicit rationale
- [ ] Critical deviations have waivers where required

### 8. Documentation Quality (Medium)

**Authoritative Docs:**
- [ ] All referenced docs exist and are accurate
- [ ] No stale references to removed docs
- [ ] Cross-references between docs are consistent

**OrcaSlicer Reference Obligations:**
- [ ] Any stated OrcaSlicer parity obligations are met
- [ ] Geometry or behavior comparisons are accurate

---

## Running Verification

### Build and Test
```bash
# From implementation-plan.md verification commands
[insert verification commands from packet.spec.md and implementation-plan.md]
```

Verification guidance by mode:

- **Full review**: Run all packet acceptance commands and the packet-level verification commands needed for closure.
- **Delta review**: Run only the commands affected by `changed_steps`, `changed_files`, or the directly impacted acceptance criteria, unless a broader rerun is needed to disambiguate a regression.

### Additional Checks
```bash
# Verify no untracked changes in expected files
git status [expected-files]

# Verify expected files match implementation-plan.md claims
ls -la [expected-directories]
```

---

## Output Format

```
## Spec Review: [Packet Name]

**Packet Path**: `path/to/packet`
**Status**: `draft | in-review | implemented | blocked`
**Review Mode**: `full | delta`
**Delta Scope**: `changed_steps=[...] changed_files=[...]`
**Reviewed**: YYYY-MM-DD

---

### Summary

[2-3 sentences: overall assessment of spec fulfillment quality]

---

### Spec Contract Review

#### Goal Verification
| Criterion | Status | Evidence |
|----------|--------|----------|
| Goal fulfilled | PASS/PARTIAL/FAIL | Brief evidence |
| No goal creep | PASS/FAIL | Brief evidence |
| Scope boundaries respected | PASS/FAIL | Brief evidence |

#### Acceptance Criteria Check
| Criterion | Status | Trace | Assertion Quality |
|-----------|--------|-------|-------------------|
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
| ... | ... | ... |

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
| Verification cmd 1 | PASS/FAIL | [output summary] |
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
| **Overall Verdict** | **APPROVED** / **APPROVED WITH NOTES** / **CHANGES REQUESTED** / **BLOCKED** |

---

### Delegated Review Return Contract

When `spec-review` is run as a Swarm review subagent, keep the response compact and structured:

- review mode and delta scope
- blocking findings by severity
- impacted steps, files, and acceptance criteria
- verification commands run and pass/fail status
- a single verdict with one short rationale paragraph

Do not repeat large packet excerpts or full build logs unless the caller asks for them.

---

## Rules

- **Be rigorous on Critical dimensions** — scope coverage and acceptance criteria fulfillment are non-negotiable
- **Verify against authoritative docs** — don't assume; check `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, etc.
- **Trace every requirement** — if you can't trace it to code, it's not done
- **Run verification commands** — documented commands must pass
- **Document all deviations** — undocumented deviations from spec are issues
- **Acknowledge good work** — positive observations matter
- **Provide specific fixes** — vague "improve this" is not actionable
- **Do not shard review** — if review is delegated, use one holistic reviewer, not a fan-out of review workers
- **Delta review is not final signoff** — always require a final full review before packet closure
- **Keep delegated review compact** — return findings, traceability, and verification status without repeating the full packet

---

## Handoff

After review:
- **APPROVED**: Spec implementation is complete and correct; this only authorizes packet closure in full review mode
- **APPROVED WITH NOTES**: Implementation complete, non-blocking improvements noted
- **CHANGES REQUESTED**: Specific changes needed before re-review
- **BLOCKED**: Critical issues that require significant rework or design decisions
