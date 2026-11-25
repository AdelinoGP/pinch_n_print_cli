---
when: Read when input decomposes into multiple packets or resumes a `docs/specs/` plan containing `## Packet Queue`.
keywords: batch, queue, inline, orchestrated, exports, resume
---

# Batch Protocol

## Plan File

Use `docs/specs/<slug>-plan.md`. Store the approved plan verbatim; if it already has a committed home, reference that path. Append:

```markdown
## Packet Queue

| # | packet slug | goal (one sentence) | depends on | status | packet dir |
|---|-------------|---------------------|------------|--------|------------|
| 1 | <slug>      | <goal>              | -          | pending | - |
| 2 | <slug>      | <goal>              | #1         | pending | - |
```

Older plans carry a `task ids` column; leave it in place.

**Packet directory naming:** every generated packet lands at `docs/spec_packets/<plan-prefix>_<NN>_<slug>/`. `<plan-prefix>` is the plan file name minus `-plan`, `<NN>` is the queue row number zero-padded to two digits, and `<slug>` is the row's packet slug. Fill the `packet dir` column when the row reaches `generated`. A superseded row that never gets a directory keeps `-` and leaves a number gap, so later rows do not renumber.

Statuses:

- `pending`: not generated.
- `generated`: both packet files written and the preflight checklist passed. This records authoring only; implementation still waits for any prerequisite packet that must be implemented first.
- `blocked`: an unanswerable `[BLOCK]` question, or preflight failures after two fix rounds.
- `superseded`: absorbed or dropped; say where in the goal column.

Update each row immediately. Dependencies always point backward. Present slugs, goals, and dependency order to the user; one approval covers every entry unless grounding changes scope. This skill never commits; report that the plan and packet directories should be committed together.

## Modes

- **2-3 packets, inline:** author sequentially in dependency order through the complete per-packet workflow, updating the row after the preflight checklist passes.
- **4+ packets, orchestrated:** the orchestrator authors nothing and processes rows in dependency order.

An authoring subagent receives at most two adjacent packets, or three only when tightly coupled. Its prompt includes the plan path, assigned queue rows, the prior exports ledger, and the instruction to follow `spec-packet-generator` steps 1-6 except the confirmation (the queue approval stands in). It returns per packet:

- packet directory;
- every net-new symbol later packets may consume, as name, crate, and shape;
- `[FWD]` questions;
- or `BLOCKED: <precise question>` before writing, when a scope-changing premise is unresolved.

Relay `BLOCKED` to the user, then redispatch with the answer.

An independent reviewer subagent, never the author, runs `spec-review --preflight <packet-dir>` and returns only its checklist table and verdict.

For each packet: dispatch the author, dispatch the reviewer, then read only `packet.spec.md` and check plan coverage, scope, and dependencies. On pass and conformance, mark `generated` and append exports. The orchestrator never opens `implementation-plan.md`.

On `PREFLIGHT BLOCKED`, return the findings to the author and re-review, for at most two rounds. Then mark `blocked`. Dependents remain `pending`; independent packets continue. If the session must stop, finish the in-flight packet, update the queue, and stop.

## Resume

1. Read the plan file in full and work from it, not memory.
2. Select the first `pending` row whose dependencies are all `generated`.
3. Rebuild exports with a short summary per generated dependency: net-new symbols by name, crate, and shape. For an archived dependency, use its `_OLD/<packet-name>.md` digest and re-derive the exports from the current tree; the digest omits implementation plans.
4. Choose mode by remaining count: 2-3 inline, 4+ orchestrated.
5. When exhausted, report the final queue, blockers, the commit-together reminder, and how to resume.

If grounding falsifies an entry, revise its goal or mark it `superseded`, with user approval and evidence. If the user revises the plan, replace its text wholesale and reapprove changed pending rows. Never retroactively edit generated packets; create a new packet when a revision invalidates one.
