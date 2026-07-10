---
name: swarm
description: Planner-Worker orchestration to implement or refine an active spec packet under .ralph/specs/. Planner stays small via strict context budget; workers do reads, edits, validation, and OrcaSlicer parity checks.
type: anthropic-skill
version: "1.5"
metadata:
  internal: true
---

# Swarm — Planner-Worker Packet Implementation

You are the planner on a Rust workspace. **Workers are the delegation primitive.** Every code read, code edit, cargo run, doc fact-check, and OrcaSlicer parity check is a worker dispatch. If you find yourself reading code or running cargo directly, you have stopped being the planner.

This block overrides anything below it that pushes you to read broadly, accumulate full logs, or paste large files.

Ancillary content lives in `references/`:
- `references/worker-prompt-template.md` — read before composing each worker prompt in Phase 3.2.
- `references/output-format.md` — read at completion to emit the Swarm Execution Report.
- `references/usage-examples.md` — read only if the user asks for invocation examples or you hit a known troubleshooting case.

## Context budget

The window is large (1M-class); the binding constraint is planning quality and per-turn cost, not fitting the window. Reasoning degrades once context fills with raw logs, full files, and stale transcripts — long before a large window is full — and every retained token is re-paid in cache cost and latency on each subsequent turn. **All budgets are absolute token counts, never window percentages.**

Two bands:

| Band | Reading budget | Checkpoints | Entry |
|------|----------------|-------------|-------|
| **standard** (default) | 120k | 100k / 120k / 150k | always |
| **extended** | 240k | 200k / 240k / **300k hard stop** | escalation protocol only |

Nearly every packet should finish in standard band. Extended exists for the genuinely large, unsplittable run — not for sloppier reading.

### Invariants (identical in both bands — extension never relaxes these)

- NEVER read a file > **600 lines** in full. Use line ranges, symbol search, or delegate.
- NEVER load generated code, lockfiles, `target/`, or vendored deps.
- NEVER paste full cargo/test output. The worker schema forbids it; reject any reply that violates it.
- NEVER absorb full worker transcripts. Only the structured return is kept.

The planner reads directly only:
- the 5 packet files (`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`)
- `.ralph/specs/README.md`
- relevant rows of `docs/07_implementation_status.md` (delegate the survey if > 300 lines)
- the predecessor packet's `packet.spec.md` if `supersedes:` is set

Everything else is a worker dispatch — in both bands.

### What extended headroom buys (and what it never buys)

Extra budget is spent only on **structured state** — the categories that degrade planning least per token:

- more review/fix iterations (their bounded structured returns)
- the full AC × evidence matrix retained across iterations instead of compressed
- per-step command summaries retained across all iterations (no mid-run ledger compression)
- the final full `spec-review` dispatch and acceptance ceremony in-session instead of deferring closure

Never on: bigger direct reads, absorbed logs, worker transcripts, or packet re-reads. A planner that escalates and then reads a 2000-line file has broken the contract twice.

### Escalation protocol (standard → extended)

Extension is declared, never drifted into. Escalate only when at least one holds:

1. Phase 0 honestly rates planner cost **L** for a packet that genuinely cannot be split (cross-cutting WIT/IR surface, atomic schema migration) — declare extended in the PLAN block.
2. At the 120k decision point the step ledger shows most steps DONE and verified, and closing (remaining fixes + ceremony + final review) verifiably exceeds remaining standard headroom — a handoff would discard a large verified ledger.
3. The user explicitly asks for single-session closure of a large packet.

To escalate, append an ESCALATION block to the step ledger and state it in the transcript:

```
ESCALATION
- Trigger: <criterion 1 | 2 | 3, plus one line of evidence>
- Spend plan: <which permitted categories, rough split>
- Stop line: 300k hard
```

One escalation per run. There is no band above extended: at 300k the only moves are finalize or `Status: PARTIAL` handoff.

## Subagent contract

Each worker dispatch must specify:
1. **Question/Assignment** — one precise step or one precise question.
2. **Scope** — exact files allowed to read; exact files allowed to edit.
3. **Return format** — the worker return schema below (for code/edit workers) or one of FACT / LOCATIONS / SNIPPETS / SUMMARY (for read-only research).

Reject any reply that exceeds the contracted return format. Do not paste it; re-dispatch with tighter scope.

### Worker return schema (code/edit workers)

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
          "summary": "one line",
          "failing_assertion": "<≤ 20 lines, only when result=FAIL>"
        }
      ],
      "exit_condition_met": true,
      "blocker": null
    }
  ],
  "follow_up": ["optional concise next action"]
}
```

The worker MUST NOT return full diffs, full logs, or repeated packet excerpts.

## Reading discipline

- One read = one hypothesis. State the hypothesis before reading; if the read does not test it, delegate.
- Load each packet file once; compile into the execution manifest (Step 1.4) and work from the manifest thereafter.
- For `Cargo.toml`: never read directly — dispatch SUMMARY of relevant `[dependencies]`/`[features]`/`[workspace]` sections.
- For trait/generic confusion: ask a worker for the monomorphized error or the concrete impl. Don't read trait hierarchies in the planner.
- Never paste full macro expansions; delegate to summarize.
- Workspace navigation: `cargo metadata --format-version=1 --no-deps` (via worker, summarized) beats reading every `Cargo.toml`.
- Test failures: worker returns failing test name, assertion, and ≤ 20 lines of relevant code — not the full test file.
- A `cargo check` worker dispatch beats reading more code to chase a bug.

## Checkpoints

Standard band:

- **100k**: state remaining budget; drop non-essential transcripts; re-confirm the manifest is the working state. Compress iteration history to a step ledger plus a one-line outcome per iteration.
- **120k — decision point**: stop reading. Either finalize/hand off within remaining headroom, or escalate to extended band via the protocol above. Not deciding = finalize.
- **150k**: STOP (if not escalated). Emit a Swarm Execution Report with `Status: PARTIAL`, the current step ledger, the next concrete dispatch list, and the files to reopen.

Extended band:

- **200k**: compress to the ledger; delta-only review; only targeted-fix workers.
- **240k**: stop dispatching everything except the acceptance ceremony and the final full review.
- **300k**: HARD STOP. `Status: PARTIAL` report + handoff. No exceptions — a `PARTIAL` report with a clean handoff is always better than a degraded final report.

## When to use

- Exactly one packet is `status: active` and ready for implementation.
- The user names a specific `draft` packet and asks for refinement, gap analysis, or a dry-run plan.
- The packet has independent steps or disjoint file surfaces to justify subagent parallelism.
- The packet's per-step context cost in `implementation-plan.md` does not exceed M; if any step is L, split the packet first.

Do not use Swarm when the packet touches one shared hotspot and offers no safe parallel slice — execute sequentially instead.

## Parameters

- `packet` (optional): packet slug or path. If omitted, resolve the single active packet from `.ralph/specs/**/packet.spec.md`.
- `workers` (optional, default `2`): max concurrent workers. Ceiling `4`; raise only after file-surface and command-independence checks pass.
- `max_iterations` (optional, default `4`): max review/fix loops after the first implementation pass.
- `scope` (optional): restrict to specific `implementation-plan.md` steps.
- `mode` (optional): `implement`, `refine-draft`, or `review-only`. Inferred from packet status and user request if omitted.
- `state_backend` (optional): `session-memory` or `packet-checkpoint`. Use the checkpoint only when the run must survive session loss or handoff.

## Status handling

- `active`: full implementation allowed. Lock the packet before dispatching workers.
- `draft`: default to `refine-draft` or `review-only`. Implement only on explicit user request; do not auto-flip status at the end.
- `implemented`: closed unless the user explicitly asks for an audit, regression review, or reopen.
- `superseded`: do not implement; resolve and use the successor packet.

If no packet is supplied and there is not exactly one `active` packet, stop and ask the user.

## Repo contract

- Exactly one packet may be `active` at a time.
- `draft`, `active`, `implemented`, `superseded` are meaningful states — do not collapse into a single readiness check.
- Every acceptance criterion in `packet.spec.md` ends with a pipe-suffixed runnable verification command.
- Validation/enforcement packets need at least one negative or rejection criterion before activation.
- `implementation-plan.md` is authoritative for step ordering, expected files, narrow verification commands, **and per-step context cost estimates**.
- `task-map.md` bridges packet work back to `docs/07_implementation_status.md`, especially for reopened or superseded work.
- `docs/07_implementation_status.md` is the backlog/progress ledger, not the technical source of truth — edit only when the packet completion gate or the actual task state requires it.
- If `supersedes: ...` is set, read the predecessor's `packet.spec.md` first.
- The planner MUST compile packet docs once into a compact execution manifest and use that manifest plus rolling deltas thereafter.
- Worker outputs MUST be structured and bounded.
- Intermediate review may be delta-scoped; packet-close decisions always need a final full review — if the run cannot fit one even after a justified escalation, closure defers to a fresh session (see Phase 5.1).
- Do not commit, create branches, or require per-worker commits unless the user explicitly asks.

## Workflow

### Phase 0 — Emit the PLAN block

Before reading any packet file:

```
PLAN
- Goal: <implement | refine-draft | review-only> packet <slug>
- Files in scope (planner reads): the 5 packet files + .ralph/specs/README.md
- Files explicitly out of scope (planner): all code, all docs > 300 lines, all OrcaSlicer source, all cargo output
- Worker dispatches planned: <Step N → worker M with files X,Y; ...>
- Estimated planner context cost: <S/M/L/XL>
- Band: <standard | extended — cost L + genuinely unsplittable; include the ESCALATION block here>
- Stop condition: packet completion gate green OR max_iterations reached with explicit handoff
```

If honest cost is L: first try to split the packet, reduce `max_iterations`, or run `review-only` to scope the work; only if the packet genuinely cannot be split, declare extended band from the start (escalation criterion 1). If honest cost is XL — it will not fit even the extended band — do not start; split the packet.

### Phase 1 — Resolve the packet & preflight

**1.1 Direct planner reads:** `.ralph/specs/README.md`, `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md` (when present). Delegate the `docs/07_implementation_status.md` survey (return: LOCATIONS of this packet's task IDs plus their status column). For `supersedes`, read the predecessor `packet.spec.md` directly; SUMMARY-dispatch any other predecessor files.

**1.2 Infer mode:** `implement` for `active` (or `draft` + explicit user ask for code changes). `refine-draft` when the packet itself needs authoring fixes, tighter acceptance criteria, or updated step/task mapping. `review-only` for audit without code changes.

**1.3 Packet-quality preflight (same gate as `spec-review`):**
- Every acceptance criterion ends with a runnable `| command`.
- Validation/enforcement packets include at least one negative criterion.
- `design.md` resolves open questions that would change scope, or the packet remains `draft`.
- `design.md` declares files-in-scope, read-only context, and explicit out-of-bounds files.
- Every step has objective, precondition, postcondition, verification, exit condition, files-to-read, files-to-edit, expected sub-agent dispatches, and context cost estimate.
- No step has context cost L. (Extended band tolerates a single L step when `design.md` justifies why it cannot be split; it runs on a dedicated worker and serializes with everything else. XL steps are always a split.)
- `task-map.md` exists when the packet spans multiple task IDs, reopens work, or supersedes a prior packet.

If preflight fails: in `refine-draft` fix the packet docs first; in `implement` stop and report the packet-authoring defects instead of guessing.

**1.4 Compile the execution manifest** (the planner's working context for the rest of the run; the original packet text should not need to be re-read):
- packet slug, status, scope summary, predecessor/successor relationships
- acceptance criteria registry with stable IDs, one-line assertions, command IDs
- step table with step IDs, task IDs, objectives, pre/postconditions, **files allowed to read**, **files allowed to edit**, **expected sub-agent dispatches**, **context cost (S/M/L)**, verification commands, exit conditions
- dependency graph derived from preconditions/postconditions
- file-ownership matrix for parallelism checks
- command registry marking each command as gating, targeted, packet-level, or informational
- docs and backlog impact summary

Store in session memory by default; use a packet-local checkpoint file only when the run must survive session loss or planner handoff. Do not re-read full packet docs unless the files changed or the manifest is incomplete.

### Phase 2 — Build the execution graph from `implementation-plan.md`

**2.1 Parse steps.** Copy each step's required fields verbatim into the manifest. Derive step dependencies, allowed worker ownership boundaries (from per-step `files-to-edit` — never widen them), and direct links from acceptance criteria to the steps and commands that satisfy them. Do not invent generic step groups when the packet already gives exact step boundaries.

**2.2 Safe parallelism — all must be true:**
- expected files do not overlap with another worker's write surface
- postcondition is not another active step's precondition
- acceptance evidence does not depend on another step finishing first
- does not share a reopened task slice with another worker in a way that would force conflicting packet/doc updates
- combined context cost does not push the planner past its band's decision point (120k standard / 200k extended) when structured returns come back

Rules: read-only inventory steps may run in parallel with other read-only analysis. Test-writing and code-writing steps that touch the same crate or test file should usually serialize. Two steps listing the same source file → sequential. Multiple steps funneling through one hotspot → sequential. Default `2` workers. Allow `3` only when write surfaces are disjoint and no precondition chain is split. Allow `4` only when source files, test files, and verification commands are all independent and no shared build/temp-resource lock is expected. If verification commands contend on the same build artifacts, temp dirs, databases, or fixture outputs, serialize even when edits are disjoint. If safe write parallelism is unclear, use workers for read-only analysis and keep edits sequential. The worker ceiling is set by write-surface safety and planner merge attention, not window size — it does not scale with the budget band.

**2.3 Capture docs/backlog impact.** Decide: leave backlog rows untouched / update task notes only / close-or-reopen task rows / reconcile a superseded predecessor. Do not assume an edit is always required — for retrofit packets correcting closed work, the correct outcome may be `no backlog delta`. Track the decision in the manifest so it is not re-derived during review.

### Phase 3 — Dispatch workers

**3.1 Tooling.** This environment uses `runSubagent` for worker execution; `multi_tool_use.parallel` to launch independent workers simultaneously; `agentName: "Explore"` for read-only research; `agentName: "coder"` for code or packet edits. Do not refer to an `Agent` tool or background workers. Workers are synchronous and stateless: each invocation must receive all required packet context up front and returns one final report.

**3.2 Worker prompts.** Each prompt must be step-sliced. **Do not paste full packet docs into every worker.** Each prompt must include:
1. packet slug and current status
2. execution mode
3. compact packet digest (goal summary, scope summary, only the relevant acceptance-criteria one-liners)
4. exact `implementation-plan.md` step blocks owned by the worker (verbatim from the manifest, including files-to-read, files-to-edit, expected sub-agent dispatches, exit condition)
5. authoritative docs and Orca refs for those steps
6. narrow verification commands per step
7. exit conditions per step
8. read-only vs may-edit
9. explicit instruction not to touch unrelated files or commit changes
10. the strict return schema
11. **context-discipline reminders**: workers are bound by the same hard limits as the planner — files-to-read is hard; no full logs

Prefer references to the manifest over repeated packet text. If a worker only owns one or two steps, include only those in detail. The full prompt template lives in `references/worker-prompt-template.md`.

**3.3 Collect and merge.**
- Parse the worker return into a rolling step ledger.
- Record `changed_steps`, `changed_files`, command outcomes.
- Verify each step actually met its exit condition. If a worker says `complete` without step-level evidence, treat the step as incomplete.
- Request detailed logs only for failed/ambiguous commands, only as a follow-up FACT/SNIPPETS dispatch.
- Run the narrowest validation command for any touched step before dispatching adjacent follow-up work.
- Serialize remaining edits that now touch shared files.

The planner carries forward only manifest, step ledger, `changed_steps`/`changed_files`, and concise command summaries. **Discard worker raw transcripts** as soon as the structured return is parsed.

### Phase 4 — Validate & review

**4.1 Validate by packet step.** After the first substantive edit in a step, immediately dispatch the step's narrow verification command from `implementation-plan.md`.

Validation order:
1. the step's own targeted falsifying command
2. the acceptance-criterion command from `packet.spec.md` if different
3. packet-level verification commands
4. broader workspace checks only if the packet asks for them

Do not replace packet-specific commands with generic workspace tests. Use the manifest's command registry so later iterations rerun only the commands affected by `changed_steps`/`changed_files`.

**`cargo test --workspace` is forbidden during implementation iterations.** The suite is >1000 tests and takes ≥11 minutes per run — running it inside a fix loop burns budget without adding signal beyond the targeted command. It runs at most once, in Phase 5.1's acceptance ceremony, and only if the packet itself lists it as a closure gate. Targeted commands (`cargo test -p <crate> --test <file>` / `-- <test_name>`) and `cargo check --workspace` are the workhorses; reach for `--workspace` test runs deliberately, never reflexively.

**4.2 Review.** After the implementation pass, dispatch a single review worker bound by `.claude/skills/spec-review/SKILL.md` (packet scope) with the same context-discipline reminders. The dispatch prompt must include this adversarial charter verbatim — a review worker without it drifts into confirming the implementation instead of attacking it:

> *You did not write this code; review it cold and bias toward finding problems. Burden of proof is on the implementation: an AC without passing dispatched evidence is FAIL, a claim you cannot trace to file:line is [unverified], and any [unverified] load-bearing row caps the verdict at CHANGES REQUESTED. Return the evidence line behind every PASS.*

The planner must reject a review return whose PASS rows carry no evidence — a verdict without evidence is not a review, and re-dispatching is cheaper than closing on a rubber stamp.

Treat findings in two buckets:
- **packet-authoring defects**: missing commands, weak acceptance language, unresolved scope, stale task mapping
- **implementation defects**: missing code, wrong control path, incomplete verification, regression

Fix packet-authoring defects first when they block trustworthy implementation review.

Intermediate loops may use a delta review keyed to `changed_steps`/`changed_files`. A final packet-close review must be full-scope provided the planner's remaining budget supports it; otherwise surface the constraint and propose a fresh-session full review. **Do not fan out `spec-review`** — at most one dedicated review worker after all code workers finish.

**4.3 Iterate only on targeted gaps.** For each high/critical finding: map to a packet step or acceptance criterion; assign to one worker if the file surface is isolated; rerun only the affected validation commands; rerun review. Stop after `max_iterations` and report remaining gaps honestly. Resend only the changed step slices and their directly affected acceptance criteria, not the full packet.

### Phase 5 — Completion, status, docs

**5.1 Acceptance ceremony.** Re-dispatch every pipe-suffixed acceptance command from `packet.spec.md`; every non-duplicate verification command from `implementation-plan.md` that still matters at packet scope; every packet-level command from the `Verification` section; a final **full** `spec-review` (packet scope) pass before status change. If budget does not allow a full review, the packet does not close this session — report DEFERRED and propose a fresh-session full review. Budget pressure defers closure; it never waives review.

**5.2 Status transitions.**
- `draft` used only for refinement → keep `draft`.
- `draft` implemented by explicit user ask → move to `implemented` only if the user asked for finalization AND the acceptance ceremony is green.
- `active` → `implemented` after the packet completion gate is satisfied.
- Predecessors marked by `supersedes` → update only when the current packet docs explicitly require the transition.

Do not auto-rewrite packet status just because code compiled. If packet status or backlog state conflicts with reality, report the mismatch instead of silently normalizing it.

**5.3 Edit `docs/07_implementation_status.md`** only if: the completion gate explicitly requires it; task states/notes actually changed; or a reopened/superseded task needs reconciliation. For retrofit packets correcting already-closed work, record `no docs/07 delta` instead of forcing a ledger edit. Dispatch the edit to a worker — never load the full backlog into the planner. When a packet reopens previously closed work, reconcile packet, predecessor packet, and `docs/07_implementation_status.md` explicitly.

## Error handling

- **Draft packet asked to implement** → only in explicit `implement` mode; keep status `draft` unless the user wants finalization; report whether the packet is implementation-grade.
- **Multiple active packets** → don't guess; report the conflict and ask which packet should own the run.
- **Worker scope overlap after decomposition** → cancel the parallel plan; regroup as sequential steps or read-only workers + planner-owned edits.
- **Planner context pressure** → rebuild the compact manifest from the packet docs once; discard stale transcripts; retain only step ledger, `changed_steps`/`changed_files`, and command summaries; full logs only via SNIPPETS dispatch on demand. Past the band's decision point (120k standard / 200k extended), switch to delta-only review and stop dispatching exploratory workers.
- **Worker output overflow** → treat as non-compliant; do not paste; ask for a compact rerun citing the context-discipline rules; never paste verbose worker output into subsequent worker prompts.
- **Missing/stale verification commands** → packet defect. In `refine-draft` fix the docs; in `implement` stop and report the missing contract.
- **Step rated context cost L** → recommend a split before activation. In extended band a single justified-unsplittable L step may run on a dedicated worker (serialized); never dispatch an L step silently in standard band. XL steps are always un-runnable — the worker faces the same quality budget the planner does, regardless of window size.

## Dependencies

- `.ralph/specs/README.md`
- `docs/07_implementation_status.md` (read via worker dispatch)
- the packet's `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`
- `.claude/skills/spec-review/SKILL.md`
- `runSubagent`, `multi_tool_use.parallel`
- session memory or a packet-local checkpoint file for the manifest and step ledger
- packet-specified build/test commands
