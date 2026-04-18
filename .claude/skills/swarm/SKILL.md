---
name: swarm
description: Planner-Worker pattern skill that orchestrates subagents to implement or refine spec packets using the repo's real packet lifecycle, packet docs, and backlog rules.
type: anthropic-skill
version: "1.2"
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
- `workers` (optional, default: `3`): Maximum concurrent workers. Start at `2`; only raise the count after file-surface and command independence checks pass.
- `max_iterations` (optional, default: `4`): Max review/fix loops after the first implementation pass.
- `scope` (optional): Restrict work to specific `implementation-plan.md` steps.
- `mode` (optional): `implement`, `refine-draft`, or `review-only`. If omitted, infer from packet status and user request.
- `state_backend` (optional): `session-memory` or `packet-checkpoint`. Use session memory for ephemeral runs; use a packet-local checkpoint only when the run must survive session loss or handoff.

## Repo Contract This Skill Must Obey

- Exactly one packet may be `active` at a time.
- `draft`, `active`, `implemented`, and `superseded` are meaningful status states and must not be collapsed into a single readiness check.
- Every acceptance criterion in `packet.spec.md` must end with a pipe-suffixed runnable verification command.
- Validation or enforcement packets need at least one negative or rejection criterion before activation.
- `implementation-plan.md` is the authoritative source for step ordering, expected files, and narrow verification commands.
- `task-map.md` is the bridge from packet work back to `docs/07_implementation_status.md`, especially for reopened or superseded work.
- `docs/07_implementation_status.md` should only be edited when the packet completion gate or the actual task state requires it. Do not update backlog rows unconditionally.
- If a packet has `supersedes: ...`, read the superseded packet's `packet.spec.md` first and any additional predecessor files needed to understand what is being corrected.
- The planner should compile packet docs once into a compact execution manifest and use that manifest plus rolling deltas as the default context source on later iterations.
- Worker outputs must be structured and bounded. Do not let the planner accumulate full diffs, full logs, or repeated copies of the packet docs unless diagnosing a failure.
- Intermediate review may be delta-scoped, but any packet-close decision still requires a final full review.

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

#### Step 1.4: Compile the packet once into an execution manifest

After the packet docs are loaded, compile them into a compact execution manifest. The manifest is the planner's working context for the rest of the run.

Recommended manifest contents:

- packet slug, status, scope summary, and predecessor or successor relationships
- acceptance criteria registry with stable IDs, one-line assertions, and command IDs
- step table with step IDs, task IDs, objectives, preconditions, postconditions, expected files, verification commands, and exit conditions
- dependency graph derived from step preconditions and postconditions
- file-ownership matrix for parallelism checks
- command registry marking which commands are gating, targeted, packet-level, or informational
- docs and backlog impact summary

Store this manifest in session memory by default. Use a packet-local checkpoint file only when the run must survive session loss or planner handoff.

Do not re-read the full packet docs on every iteration unless the packet files changed or the manifest is incomplete.

### Phase 2: Build the Execution Graph from `implementation-plan.md`

#### Step 2.1: Parse packet steps

From the implementation plan, compile or refresh the manifest entries for each step:

- step number and title
- task IDs
- objective
- precondition and postcondition
- files expected to change
- authoritative docs and Orca refs
- verification commands
- exit condition

Also derive:

- step dependencies
- allowed worker ownership boundaries
- direct links from acceptance criteria to the steps and commands that satisfy them

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
- Default to `2` workers.
- Allow `3` workers only when write surfaces are disjoint and no step precondition chain is being split across workers.
- Allow `4` workers only when source files, test files, and verification commands are all independent and no shared build or temp-resource lock is expected.
- If verification commands contend on the same build artifacts, temp directories, databases, or fixture outputs, serialize them even when file edits are disjoint.

#### Step 2.3: Capture docs and backlog impact

From `task-map.md` and `docs/07_implementation_status.md`, decide whether the run should:

- leave backlog rows untouched
- update task notes only
- close or reopen task rows
- reconcile a superseded predecessor packet

Do not assume a `docs/07_implementation_status.md` edit is always required. For retrofit packets correcting already-closed work, the correct outcome may be `no backlog delta`.

Track this decision in the execution manifest so the planner does not have to re-derive it during review and acceptance.

### Phase 3: Dispatch Workers with the Real Tooling

#### Step 3.1: Use the available subagent tools

This environment uses:

- `runSubagent` for worker execution
- `multi_tool_use.parallel` to launch independent workers simultaneously
- `agentName: "Explore"` for read-only research
- `agentName: "coder"` for code or packet edits

Do not refer to an `Agent` tool or background workers. Workers here are synchronous and stateless: each invocation must receive all required packet context up front and will return one final report.

#### Step 3.2: Worker prompt requirements

Each worker prompt must be step-sliced. Do not paste the full packet docs into every worker.

Each worker prompt must include:

1. packet slug and current status
2. execution mode
3. a compact packet digest: goal summary, scope summary, and only the acceptance criteria IDs or one-line assertions relevant to that worker
4. exact `implementation-plan.md` steps owned by the worker
5. exact files allowed to change
6. authoritative docs and Orca refs for those steps
7. narrow verification commands for each step
8. exit conditions for each step
9. whether the worker is read-only or may edit files
10. explicit instruction not to touch unrelated files or commit changes
11. a strict return schema

Prefer references to the execution manifest over repeated copies of the packet text. If a worker only owns one or two steps, include only those steps in full detail.

Worker return schema:

```json
{
  "worker_id": "w1",
  "overall_status": "DONE|PARTIAL|BLOCKED",
  "steps": [
    {
      "step_id": "Step 2",
      "status": "DONE|PARTIAL|BLOCKED",
      "files_changed": ["path/to/file"],
      "commands_run": [
        {
          "command": "cargo test ...",
          "result": "PASS|FAIL|NOT_RUN",
          "summary": "one line"
        }
      ],
      "exit_condition_met": true,
      "blocker": null
    }
  ],
  "follow_up": ["optional concise next action"]
}
```

Do not return full diffs, full logs, or repeated packet excerpts unless the planner explicitly asks for them.

Worker prompt template:
```text
Packet: <packet-slug>
Status: <draft|active|implemented|superseded>
Mode: <implement|refine-draft|review-only>

Packet digest:
- Goal: <1-3 lines>
- Scope: <1-3 lines>
- Relevant acceptance criteria: <AC-1 one-line summary>, <AC-3 one-line summary>

Execution manifest references:
- Step ledger entries: <Step 2, Step 3>
- Allowed files: ...
- Relevant docs: ...

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
6. Return JSON matching the worker return schema.
7. Keep command summaries to one line each. Do not paste full logs unless asked.
```

#### Step 3.3: Collect and merge worker results

After the worker batch finishes:

- parse the worker return schema into a rolling step ledger
- record `changed_steps`, `changed_files`, and command outcomes
- verify whether each step actually met its exit condition
- request detailed logs only for failed or ambiguous commands
- run the narrowest validation command for any touched step before dispatching adjacent follow-up work
- serialize any remaining edits that now touch shared files

If a worker says `complete` but did not show step-level evidence, treat the step as incomplete.

The planner should carry forward only the manifest, the step ledger, the `changed_steps` or `changed_files` set, and concise command summaries. Do not keep every raw worker transcript in the planner context.

### Phase 4: Validate and Review

#### Step 4.1: Validate by packet step, not by habit

After the first substantive edit in a step, immediately run the step's narrow verification command from `implementation-plan.md`.

Validation order:

1. the step's own targeted falsifying command
2. the acceptance-criterion command from `packet.spec.md` if it is different
3. packet-level verification commands
4. broader workspace checks only if the packet asks for them

Do not replace packet-specific commands with hardcoded generic workspace tests unless the packet explicitly lists them.

Use the command registry from the execution manifest so later iterations rerun only the commands affected by `changed_steps` or `changed_files`.

#### Step 4.2: Review against the packet

After the implementation pass, load `.claude/skills/spec-review/SKILL.md` and run the packet through that checklist.

Treat findings in two buckets:

- packet-authoring defects: missing commands, weak acceptance language, unresolved scope, stale task mapping
- implementation defects: missing code, wrong control path, incomplete verification, regression

Fix packet-authoring defects first when they block trustworthy implementation review.

Review guidance:

- Intermediate loops may use a delta review keyed to `changed_steps` or `changed_files`.
- A final packet-close review must still be full-scope.
- If review is delegated, use at most one dedicated review worker after all code workers finish. Do not fan out `spec-review` across multiple workers or shard it by review dimension.

#### Step 4.3: Iterate only on targeted gaps

For each high or critical finding:

1. map it back to a packet step or acceptance criterion
2. assign it to one worker if the file surface is isolated
3. rerun only the affected validation commands
4. rerun review

Stop after `max_iterations` and report any remaining gaps honestly.

Use the step ledger to drive iteration. The planner should resend only the changed step slices and their directly affected acceptance criteria instead of the full packet.

### Phase 5: Completion, Status Transitions, and Docs

#### Step 5.1: Run the packet acceptance ceremony

Re-run:

- every pipe-suffixed acceptance command from `packet.spec.md`
- every non-duplicate verification command from `implementation-plan.md` that still matters at packet scope
- every packet-level command from the `Verification` section
- a final full `spec-review` pass before changing packet status or claiming packet closure

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
Planner state backend: session-memory | packet-checkpoint

Packet-quality preflight:
- PASS / FAIL
- Notes: ...

Execution manifest:
- built: yes / no
- delta scope for this iteration: changed_steps=<...>, changed_files=<...>

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
- Prefer no more than 2 concurrent code workers by default. Raise to 3 or 4 only after file-surface and verification-resource checks pass.
- If safe write parallelism is unclear, use workers for read-only analysis and keep edits sequential.
- Do not commit, create branches, or require per-worker commits unless the user explicitly asks.
- Do not mark a step complete without evidence that its exit condition was met.
- If the packet status or backlog state conflicts with reality, report the mismatch instead of silently normalizing it.
- When a packet reopens previously closed work, reconcile the packet, predecessor packet, and `docs/07_implementation_status.md` explicitly.
- Keep the planner context compact: manifest, step ledger, deltas, and concise command summaries. Pull raw logs only on demand.
- Do not fan out `spec-review`; use a single holistic review pass or a single review worker.

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

### Planner context pressure

If the planner context starts to bloat:

- rebuild the compact execution manifest from the packet docs once
- discard stale raw worker transcripts from active context
- retain only the step ledger, `changed_steps` or `changed_files`, and command summaries
- request full logs only for the specific failed command or ambiguous step

### Worker output overflow

If a worker returns verbose prose or full logs instead of the schema:

- treat the response as non-compliant
- ask for a compact rerun using the schema
- do not paste the entire verbose worker output into subsequent worker prompts

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
- session memory or a packet-local checkpoint file for the execution manifest and step ledger
- packet-specified build and test commands

## Usage Examples

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:refine-draft
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:implement workers:2
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement mode:implement workers:3 state_backend:session-memory
```

```text
/swarm packet:02-rev2_runtime-access-audit-and-declaration-enforcement scope:"Step 1,Step 2,Step 3"
```

## Troubleshooting

**"Packet is draft"**: Refine or review it by default. Only run implementation from a draft when the user explicitly asks for code changes.

**"Packet tasks already show [x] in docs/07"**: Treat the packet as a retrofit or reopen slice. Do not blindly toggle backlog rows; reconcile the mismatch in the report.

**"Worker produced no changes"**: Verify whether the step was read-only or already satisfied. If not, treat the step as incomplete.

**"The planner is running out of context"**: Rebuild the compact execution manifest and continue from the step ledger plus deltas instead of reloading the full packet and prior worker transcripts.

**"Review keeps finding packet-authoring defects"**: Fix the packet docs first; do not keep rerunning code workers against an under-specified packet.

**"Build fails after a worker run"**: Map the failure back to the specific packet step, repair that slice, and rerun the same narrow validation before widening scope.
