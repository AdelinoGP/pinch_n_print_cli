---
name: swarm
description: Implement a spec packet under docs/spec_packets/ with a planner and parallel workers, then close it with an independent review. Also refines a draft packet or audits one without code changes.
type: anthropic-skill
version: "2.0"
metadata:
  internal: true
---

# Swarm: Planner-Worker Packet Implementation

You are the planner. You own the packet, the step ledger, validation, review, and every commit. Workers own step edits. You may read code and run cheap cargo commands yourself to plan, unblock, or verify; hand a step's edits to a worker.

Delegate large reads (`OrcaSlicerDocumented/`, long files, broad surveys) and noisy cargo output to subagents, and keep only their conclusions. The packet layout and the implementation gotchas live in [`docs/spec_packets/README.md`](../../../docs/spec_packets/README.md). Read it first, and pass the gotchas relevant to a step into its worker prompt.

## Parameters

- `packet` (required): packet directory name or path. Ask when absent.
- `mode`: `implement` (default for `draft`), `refine-draft` (fix the packet docs only), or `review-only` (audit, no code changes). An `implemented` packet is closed unless the user asks for an audit or reopen; a `superseded` one redirects to its successor.
- `workers` (default 2, ceiling 4): maximum concurrent workers.
- `max_iterations` (default 4): review/fix loops after the first implementation pass.
- `scope`: restrict the run to named steps.

## Workflow

### 1. Load the packet

Read `packet.spec.md` and `implementation-plan.md` (legacy packets: also `requirements.md` and `design.md`; for `supersedes:`, the predecessor's `packet.spec.md`). Build a step ledger: step, files touched, verify command, done-when, dependencies, status.

Sanity-check the packet against the tree before dispatching work. A cited symbol that does not exist, a criterion no test binary can drive, or a missing negative case for an enforcement change is a packet defect. In `implement` mode, fix the packet and record each fix under `## Deviations` in `packet.spec.md`; when the fix changes scope, stop and ask. Never bend code to satisfy a broken criterion.

Resolve `[BLOCK]` questions and unmet implementation prerequisites before dispatching implementation work. In `refine-draft` mode, update only the packet, run the authoring checklist, report the result, and stop. In `review-only` mode, run the requested `spec-review` and report its verdict without changing code, packet status, or commits.

### 2. Plan the dispatch

Steps run in parallel only when their files are disjoint, neither consumes the other's output, and their verification commands do not contend for the same build artifacts, temp dirs, or fixtures. Otherwise serialize. Two steps touching one crate's tests usually serialize. When unsure, serialize the edits and parallelize read-only analysis.

### 3. Dispatch workers

Each worker prompt carries:

- the packet goal and the acceptance criteria its steps serve;
- its step blocks, verbatim from `implementation-plan.md`;
- the gotchas that apply to those steps;
- the verify commands, run after the step's edits and before reporting;
- what it must not do: edit files outside its steps' surface without saying why, commit, or create branches.

A worker reports in short plain text, per step: status (DONE / PARTIAL / BLOCKED), files changed, each command with PASS or FAIL and one result line, and for a failure the failing assertion in at most 20 lines. A claim that a file is missing or unreadable carries the literal command and its output; without that evidence, check it yourself before dropping any work.

### 4. Validate and review

- After each worker returns, confirm each step's done-when against evidence and rerun its verify command when the return is thin. A "done" without step-level evidence counts as incomplete.
- When all steps are in, complete the Docs impact edits and run the acceptance checks in Close below, then dispatch **one** full review worker bound by `spec-review` packet scope. Give it the check results and the tree they apply to, so it can reuse valid evidence. Its prompt includes this charter verbatim:

  > *You did not write this code; review it cold and bias toward finding problems. Burden of proof is on the implementation: an AC without passing evidence is FAIL, a claim you cannot trace to file:line is [unverified], and any [unverified] load-bearing row caps the verdict at CHANGES REQUESTED. Return the evidence line behind every PASS.*

- Reject a review whose PASS rows carry no evidence. The reviewer reports once, after all its checks land; a mid-wave status update is not a verdict.
- Sort findings into packet defects (fix the packet first) and implementation defects. Send each high or critical finding to one worker with only the affected step and criteria, rerun the affected commands, and re-review (delta review is fine between iterations). Stop after `max_iterations` and report what remains. If changes follow a review, obtain a final full review before closure.

### 5. Close

1. Run `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and every acceptance-criterion and Verification command in `packet.spec.md`; reuse valid results from the unchanged tree. Workspace tests follow AGENTS.md §Test Discipline.
2. Get a final **full** `spec-review` packet-scope verdict of APPROVED or APPROVED WITH NOTES. Delta review never closes a packet; a full review that examined the final tree and these results need not be repeated.
3. Set `status: implemented`. On a superseding packet, also set the predecessor's `status: superseded` and `superseded_by:`. A step-scoped run that has not fulfilled the whole packet remains PARTIAL; it does not close the packet.

If the run cannot finish, stop with a PARTIAL report: the step ledger, what is verified, the next concrete actions, and checkpoint hashes.

## Checkpoint commits

Commit only when asked, except on checkpointed runs: plans with 4 or more steps, or when the user asks. On those, commit after every verified step (done-when met and verify command green), before the next dispatch, with a message starting `wip(<packet-name>):` that names the verified steps and lists open items. Include only this run's changes, preserving pre-existing work and staging. At close, squash the checkpoints into one packet commit, but only when every commit since the first checkpoint's parent is a checkpoint from this run; otherwise report and ask. A PARTIAL stop keeps the checkpoints. Drop a stash entry only when `git stash show -p <entry>` shows nothing missing from HEAD, or on an explicit ask.

## Report

```text
## Swarm Report: <packet>
Status: COMPLETED | PARTIAL | BLOCKED    Mode: <mode>    Iterations: N    Workers: N
Steps: <step: DONE/PARTIAL/BLOCKED, evidence line>
Review verdict: <verdict>
Packet changes: <status change, deviations recorded, docs edited>
Commits: <squash hash, or checkpoint hashes>
Remaining issues / next actions: <list, required unless COMPLETED>
```
