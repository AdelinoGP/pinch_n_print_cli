---
when: Read when input decomposes into multiple packets or resumes a `docs/specs/` plan containing `## Packet Queue`.
keywords: batch, queue, inline, orchestrated, exports, resume
---

# Batch Protocol

The core laws in `SKILL.md` remain authoritative: plan-file anchor, 2-3 inline versus 4+ orchestrated, and author-reviewer independence.

## Plan File

Use `docs/specs/<slug>-plan.md`. Store the approved plan verbatim; if it already has a committed home, reference that path. Append:

```markdown
## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | <slug>      | <goal>              | TASK-... | -          | pending | - |
| 2 | <slug>      | <goal>              | TASK-... | #1         | pending | - |
```

Statuses:

- `pending`: not generated.
- `generated`: required packet files written and `PREFLIGHT PASS`.
- `blocked`: unanswerable `[BLOCK]` question or gate failure after two fix rounds.
- `superseded`: absorbed/dropped; identify where in the goal column.

Update each row immediately. Dependencies always point backward. Present slugs, goals, task IDs, and dependency order via `AskUserQuestion`; one approval covers every entry unless grounding changes scope. This skill never commits; report that the plan and packet directories should be committed together.

## Modes

- **2-3 packets, inline:** author sequentially in dependency order through the complete per-packet workflow, updating the row after preflight.
- **4+ packets, orchestrated:** the orchestrator authors nothing. Process dependency order sequentially.

An authoring subagent receives at most two adjacent packets, or three only when tightly coupled. Its prompt includes the plan path, assigned queue rows, prior exports ledger, Step-4 grounding obligations, the Steps 7-13 generation workflow, the file-purpose ownership model, snippet verbatim rule, AC contract, self-review checklist, and the preflight completion rule. It may read the tree greedily because its context is disposable. It returns per packet:

- packet directory;
- every net-new symbol later packets may consume as name, crate, and shape;
- `[FWD]` questions;
- or `BLOCKED: <precise question>` before writing when a scope-changing premise is unresolved.

Relay `BLOCKED` through `AskUserQuestion`, then redispatch with the answer.

An independent reviewer subagent, never the author, runs `spec-review --preflight <packet-dir>` and returns only its S0-S8 table and verdict.

For each packet: dispatch author, dispatch reviewer, then read only `packet.spec.md` and check plan coverage, scope, and dependencies. On pass and conformance, mark `generated` and append exports. Never open `design.md` or `implementation-plan.md` as orchestrator.

On `PREFLIGHT BLOCKED`, return findings to the author and re-review, for at most two rounds. Then mark `blocked`. Dependents remain `pending`; independent packets continue. If the 100k checkpoint fires, finish the in-flight packet, update the queue, and stop.

## Resume

1. Read the plan file in full and work from it, not memory.
2. Select the first `pending` row whose dependencies are all `generated`.
3. Rebuild exports with one SUMMARY per generated dependency: list net-new symbols by name, crate, and shape.
4. Choose mode by remaining count: 2-3 inline, 4+ orchestrated.
5. When exhausted, report the final queue.

If grounding falsifies an entry, revise its goal or mark it `superseded`, with user approval and evidence. If the user revises the plan, replace its text wholesale and reapprove changed pending rows. Never retroactively edit generated packets; create a new packet when the revision invalidates one.
