---
name: swarm
description: Planner-Worker pattern skill that orchestrates subagents to implement or refine spec packets using the repo's real packet lifecycle, packet docs, and backlog rules.
type: anthropic-skill
version: "1.1"
metadata:
  internal: true
---

# Swarm — Planner-Worker Packet Implementation Skill

## Overview

Swarm implements or refines one packet under `.ralph/specs/<packet>/` by using a planner plus a small number of worker subagents.

This repo's contract is:

- `.ralph/specs/README.md` defines packet lifecycle and activation rules.
- `packet.spec.md` is the preflight-visible packet contract.
- `requirements.md`, `design.md`, `implementation-plan.md`, and `task-map.md` refine that contract.
- `docs/07_implementation_status.md` is the canonical backlog and task ledger, not the technical source of truth for code behavior.

This skill is an alternative execution strategy for packet work. It must follow the packet docs exactly rather than substituting generic workflow steps.

## When to Use

- Exactly one packet is `status: active` and ready for implementation.
- The user names a specific `draft` packet and asks for packet refinement, gap analysis, or a dry-run implementation plan.
- The packet has enough independent steps or disjoint file surfaces to justify subagent parallelism.
- You want a planner to coordinate packet steps, validation, and review against the packet contract.

Do not use Swarm when the packet touches one shared hotspot and offers no safe parallel slice. Execute sequentially instead.

## Parameters

- `packet` (optional): Packet slug or path. If omitted, resolve the single active packet from `.ralph/specs/**/packet.spec.md`.
- `workers` (optional, default: `3`): Maximum concurrent workers. Use fewer when file overlap is high.
- `max_iterations` (optional, default: `2`): Max review/fix loops after the first implementation pass.
- `scope` (optional): Restrict work to specific `implementation-plan.md` steps.
- `mode` (optional): `implement`, `refine-draft`, or `review-only`. If omitted, infer from packet status and user request.

## Repo Contract This Skill Must Obey

- Exactly one packet may be `active` at a time.
- `draft`, `active`, `implemented`, and `superseded` are meaningful status states and must not be collapsed into a single readiness check.
- Every acceptance criterion in `packet.spec.md` must end with a pipe-suffixed runnable verification command.
- Validation or enforcement packets need at least one negative or rejection criterion before activation.
- `implementation-plan.md` is the authoritative source for step ordering, expected files, and narrow verification commands.
- `task-map.md` is the bridge from packet work back to `docs/07_implementation_status.md`, especially for reopened or superseded work.
- `docs/07_implementation_status.md` should only be edited when the packet completion gate or the actual task state requires it. Do not update backlog rows unconditionally.
- If a packet has `supersedes: ...`, read the superseded packet's `packet.spec.md` first and any additional predecessor files needed to understand what is being corrected.

## Status Handling

- `active`: Full implementation mode is allowed. Lock the packet before dispatching workers.
- `draft`: Default to `refine-draft` or `review-only`. Only implement from a draft when the user explicitly asks for it, and do not flip the status automatically at the end.
- `implemented`: Treat as closed unless the user explicitly asks for an audit, regression review, or reopen packet work.
- `superseded`: Do not implement this packet. Resolve and use the successor packet instead.

If no packet is supplied and there is not exactly one active packet, stop and ask the user which packet to use.

## Workflow

### Phase 1: Resolve the Packet and Run Packet-Quality Preflight

#### Step 1.1: Resolve the packet

Read:
1. `.ralph/specs/README.md`
2. `packet.spec.md`
3. `requirements.md`
4. `design.md`
5. `implementation-plan.md`
6. `task-map.md` when present
7. `docs/07_implementation_status.md`

If the packet has `supersedes`, also read the predecessor `packet.spec.md`, then any predecessor design or task-map files needed to understand what is being corrected.

#### Step 1.2: Infer the execution mode

- Use `implement` for an `active` packet, or for a `draft` packet only when the user explicitly wants code changes against that draft.
- Use `refine-draft` when the packet itself needs authoring fixes, tighter acceptance criteria, or updated step or task mapping.
- Use `review-only` when the request is to audit or analyze without writing code.

#### Step 1.3: Packet-quality preflight

Before decomposition, apply the same gate used by `spec-review`:

- Every acceptance criterion ends with a runnable `| command`
- Validation or enforcement packets include at least one negative criterion
- `design.md` resolves open questions that would change scope, or the packet remains `draft`
- Every implementation step has an objective, precondition, postcondition, verification, and exit condition
- `task-map.md` exists when the packet spans multiple task IDs, reopens work, or supersedes a prior packet

If preflight fails:

- In `refine-draft` mode, fix the packet docs first.
- In `implement` mode, stop and report the packet-authoring defects instead of guessing.

### Phase 2: Build the Execution Graph from `implementation-plan.md`

#### Step 2.1: Parse packet steps

For each step, extract:

- step number and title
- task IDs
- objective
- precondition and postcondition
- files expected to change
- authoritative docs and Orca refs
- verification commands
- exit condition

Do not invent generic step groups when the packet already gives exact step boundaries.

#### Step 2.2: Determine safe parallelism

A step is safe to parallelize only when all of the following are true:

- its expected files do not overlap with another worker's write surface
- its postcondition is not a precondition for another active step
- its acceptance evidence does not rely on another step finishing first
- it does not share a reopened task slice with another worker in a way that would force conflicting packet or doc updates

Parallelization rules:

- Read-only inventory steps can run in parallel with other read-only analysis.
- Test-writing and code-writing steps that touch the same crate or test file should usually be serialized.
- If two steps both list the same source file, keep them sequential.
- If the packet funnels multiple steps through one hotspot, Swarm should fall back to sequential execution instead of forcing parallel workers.

#### Step 2.3: Capture docs and backlog impact

From `task-map.md` and `docs/07_implementation_status.md`, decide whether the run should:

- leave backlog rows untouched
- update task notes only
- close or reopen task rows
- reconcile a superseded predecessor packet

Do not assume a `docs/07_implementation_status.md` edit is always required. For retrofit packets correcting already-closed work, the correct outcome may be `no backlog delta`.

### Phase 3: Dispatch Workers with the Real Tooling

#### Step 3.1: Use the available subagent tools

This environment uses:

- `runSubagent` for worker execution
- `multi_tool_use.parallel` to launch independent workers simultaneously
- `agentName: "Explore"` for read-only research
- `agentName: "coder"` for code or packet edits

Do not refer to an `Agent` tool or background workers. Workers here are synchronous and stateless: each invocation must receive all required packet context up front and will return one final report.

#### Step 3.2: Worker prompt requirements

Each worker prompt must include:

1. packet slug and current status
2. execution mode
3. packet goal and in-scope or out-of-scope items
4. exact `implementation-plan.md` steps owned by the worker
5. exact files allowed to change
6. authoritative docs and Orca refs for those steps
7. narrow verification commands for each step
8. exit conditions for each step
9. whether the worker is read-only or may edit files
10. explicit instruction not to touch unrelated files or commit changes

Worker prompt template:
```text
Packet: <packet-slug>
Status: <draft|active|implemented|superseded>
Mode: <implement|refine-draft|review-only>

Goal:
<from packet.spec.md>

Scope:
- In scope: ...
- Out of scope: ...

Assigned steps:
- Step N: ...
  - Task IDs: ...
  - Objective: ...
  - Precondition: ...
  - Postcondition: ...
  - Files allowed to change: ...
  - Authoritative docs: ...
  - Orca refs: ...
  - Verification: ...
  - Exit condition: ...

Execution rules:
1. Read the listed docs before changing code.
2. Follow the packet's TDD and step ordering rules.
3. Validate immediately after the first substantive edit using the step's narrow command.
4. Do not modify files outside the allowed list.
5. Do not commit or create branches.
6. Return a concise report with files changed, validation run, and any blockers.
```

#### Step 3.3: Collect and merge worker results

After the worker batch finishes:

- aggregate the returned evidence
- verify whether each step actually met its exit condition
- run the narrowest validation command for any touched step before dispatching adjacent follow-up work
- serialize any remaining edits that now touch shared files

If a worker says `complete` but did not show step-level evidence, treat the step as incomplete.

### Phase 4: Validate and Review

#### Step 4.1: Validate by packet step, not by habit

After the first substantive edit in a step, immediately run the step's narrow verification command from `implementation-plan.md`.

Validation order:

1. the step's own targeted falsifying command
2. the acceptance-criterion command from `packet.spec.md` if it is different
3. packet-level verification commands
4. broader workspace checks only if the packet asks for them

Do not replace packet-specific commands with hardcoded generic workspace tests unless the packet explicitly lists them.

#### Step 4.2: Review against the packet

After the implementation pass, load `.claude/skills/spec-review/SKILL.md` and run the packet through that checklist.

Treat findings in two buckets:

- packet-authoring defects: missing commands, weak acceptance language, unresolved scope, stale task mapping
- implementation defects: missing code, wrong control path, incomplete verification, regression

Fix packet-authoring defects first when they block trustworthy implementation review.

#### Step 4.3: Iterate only on targeted gaps

For each high or critical finding:

1. map it back to a packet step or acceptance criterion
2. assign it to one worker if the file surface is isolated
3. rerun only the affected validation commands
4. rerun review

Stop after `max_iterations` and report any remaining gaps honestly.

### Phase 5: Completion, Status Transitions, and Docs

#### Step 5.1: Run the packet acceptance ceremony

Re-run:

- every pipe-suffixed acceptance command from `packet.spec.md`
- every non-duplicate verification command from `implementation-plan.md` that still matters at packet scope
- every packet-level command from the `Verification` section

#### Step 5.2: Apply status transitions carefully

- `draft` packet used only for refinement: keep it `draft`
- `draft` packet implemented by explicit user request: only move it to `implemented` if the user asked for finalization and the acceptance ceremony is green
- `active` packet: may move to `implemented` after the packet completion gate is satisfied
- predecessor packets marked by `supersedes`: update only when the current packet docs explicitly require the transition

Do not automatically rewrite packet status just because code compiled.

#### Step 5.3: Update `docs/07_implementation_status.md` only when justified

Edit `docs/07_implementation_status.md` only if one of the following is true:

- the packet completion gate explicitly requires it
- task states or notes actually changed
- a reopened or superseded task needs reconciliation in the backlog

If the tasks are already closed and the packet is a retrofit correction, record `no docs/07 delta` in the report instead of forcing a ledger edit.


## Output Format

```text
## Swarm Execution Report: <packet-slug>

Status: COMPLETED | PARTIAL | BLOCKED
Mode: implement | refine-draft | review-only
Starting packet status: draft | active | implemented | superseded
Iterations: N
Workers used: N

Packet-quality preflight:
- PASS / FAIL
- Notes: ...

Step results:
- Step N: DONE / PARTIAL / SKIPPED
  - Evidence: ...
  - Validation: ...

Review verdict:
- APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED / BLOCKED

Docs and status impact:
- packet status change: ...
- docs/07 change: yes / no
- superseded packet reconciliation: ...

Remaining issues:
1. ...

Verification commands run:
- ...
```

## Rules

- Packet docs are authoritative. Do not substitute generic commands for packet commands.
- `docs/07_implementation_status.md` is the backlog and progress ledger, not the technical specification for code behavior.
- Prefer no more than 2-3 concurrent workers unless the file surfaces are obviously disjoint.
- If safe write parallelism is unclear, use workers for read-only analysis and keep edits sequential.
- Do not commit, create branches, or require per-worker commits unless the user explicitly asks.
- Do not mark a step complete without evidence that its exit condition was met.
- If the packet status or backlog state conflicts with reality, report the mismatch instead of silently normalizing it.
- When a packet reopens previously closed work, reconcile the packet, predecessor packet, and `docs/07_implementation_status.md` explicitly.

## Error Handling

### Draft packet asked to implement

If a packet is `draft` and the user wants code changes:

- proceed only in explicit `implement` mode
- keep the packet `draft` unless the user also wants activation or finalization
- report whether the packet is implementation-grade or still needs authoring fixes

### Multiple active packets

If more than one packet is `active`:

- do not guess
- report the conflict and ask which packet should own the run

### Worker overlap

If worker scopes collide after decomposition:

- cancel the parallel plan
- regroup as sequential steps or read-only workers plus planner-owned edits

### Missing or stale verification commands

If `packet.spec.md` or `implementation-plan.md` lacks runnable commands:

- treat that as a packet defect
- in `refine-draft` mode, fix the packet docs
- in `implement` mode, stop and report the missing contract

## Dependencies

- `.ralph/specs/README.md`
- `docs/07_implementation_status.md`
- the packet's `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, and `task-map.md`
- `.claude/skills/spec-review/SKILL.md`
- `runSubagent`
- `multi_tool_use.parallel`
- packet-specified build and test commands

## Usage Examples

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:refine-draft
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:implement workers:2
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement scope:"Step 1,Step 2,Step 3"
```

## Troubleshooting

**"Packet is draft"**: Refine or review it by default. Only run implementation from a draft when the user explicitly asks for code changes.

**"Packet tasks already show [x] in docs/07"**: Treat the packet as a retrofit or reopen slice. Do not blindly toggle backlog rows; reconcile the mismatch in the report.

**"Worker produced no changes"**: Verify whether the step was read-only or already satisfied. If not, treat the step as incomplete.

**"Review keeps finding packet-authoring defects"**: Fix the packet docs first; do not keep rerunning code workers against an under-specified packet.

**"Build fails after a worker run"**: Map the failure back to the specific packet step, repair that slice, and rerun the same narrow validation before widening scope.
