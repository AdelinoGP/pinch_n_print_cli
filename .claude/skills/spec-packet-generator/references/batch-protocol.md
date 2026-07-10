---
when: Read this when the spec-packet-generator input decomposes into more than one packet, or the input is a `docs/specs/` plan file with a `## Packet Queue` section (resume). SKILL.md's Batch Protocol laws — plan file as anchor, mode by size, author ≠ reviewer — apply throughout and are not repeated here.
keywords: batch, plan file, packet queue, inline, orchestrated, authoring subagent, reviewer subagent, exports ledger, resume
---

# Batch Protocol — plan file, modes, and dispatch contracts

## Plan file

Location: `docs/specs/<slug>-plan.md` (the existing home for plan documents). Contents: the approved plan **verbatim** — never condensed; if the plan already lives in a committed file, reference its path — then:

```markdown
## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | <slug>      | <goal>              | TASK-…   | —          | pending | — |
| 2 | <slug>      | <goal>              | TASK-…   | #1         | pending | — |
```

`status`: `pending` · `generated` (5 files written, PREFLIGHT PASS) · `blocked` (gate failed after 2 fix rounds, or an unanswerable `[BLOCK]` question) · `superseded` (absorbed/dropped — note where in the goal column). Update the row the moment its packet's outcome is known.

Present the queue (slugs, goals, task ids, dependency order — `depends on` always points backward) via `AskUserQuestion` and get **one approval**: it is the Step-5 metadata gate's standing answer for every entry; re-ask only when grounding falsifies an entry's scope. The skill never commits — the report reminds the user to commit the plan file and packet dirs together.

## Mode by size

- **2–3 packets — inline.** This session authors each packet sequentially in dependency order through the normal workflow (Steps 2–16 per packet), updating the queue row after each Step-14 gate.
- **4+ packets — orchestrated.** This session dispatches authoring and reviewer subagents and authors nothing itself.

## Orchestrated mode

Sequential, in dependency order.

**Authoring subagent** — assigned up to 2 adjacent packets, up to 3 only when coupled (each consumes the previous one's net-new symbols). Its prompt carries: the plan file path, its queue rows, the accumulated exports ledger from prior packets, and the packet workflow obligations (grounding per Step 4, files per Steps 7–13, self-review per Step 12). It reads the tree directly and greedily — its context is disposable; the delegation discipline protects the orchestrator, not subagents. It returns:

- per packet: the packet dir + an **exports ledger** entry for every net-new symbol (name, crate, shape) later packets may consume;
- any `[FWD]` questions it recorded in `design.md`;
- OR an early `BLOCKED: <precise question>` return **before writing files** on a `[BLOCK]`-class ambiguity (scope-changing, plan premise falsified). The orchestrator relays via `AskUserQuestion` and re-dispatches with the answer.

**Reviewer subagent** — independent per packet, never the author: runs the S0–S8 preflight gate (`spec-review --preflight <packet dir>`), returns the gate table + verdict only.

**Orchestrator loop** per packet: dispatch author → dispatch reviewer → read `packet.spec.md` in full and check plan conformance (the plan item is covered, no scope creep, deps match the queue) → on `PREFLIGHT PASS` + conformance, mark the row `generated`, append its exports to the ledger, continue. The orchestrator never opens `design.md` or `implementation-plan.md`.

**Failure:** `PREFLIGHT BLOCKED` → return the gate findings to the authoring subagent for a fix pass, re-review; at most 2 rounds, then mark the row `blocked`. A blocked packet's dependents stay `pending` — their premises consume uncertified exports; independent packets continue.

**Budget:** SKILL.md checkpoints govern. If the 100k checkpoint fires mid-batch: finish the in-flight packet, update the queue, stop — remaining rows stay `pending`.

## Resume (invocation with a plan file)

1. Read the plan file in full — it is the anchor; work from it, not from any recollection of the plan.
2. Select the first `pending` row whose `depends on` rows are all `generated`. Rebuild the exports ledger with one SUMMARY dispatch per `generated` dependency: "list the net-new symbols `<packet dir>` creates — name, crate, shape."
3. Continue in the mode the remaining `pending` count dictates (2–3 → inline; 4+ → orchestrated).
4. Queue exhausted → report the final table; the batch is complete.

## Edge cases

- **Grounding falsifies an entry's premise** (the symbol/behavior it targets doesn't exist as the plan claimed): revise the entry's goal or mark it `superseded` — user approval either way — recording what the tree actually showed.
- **The user revises the plan mid-batch:** replace the plan text wholesale, reconcile `pending` rows against the new text (re-approve changed rows). `generated` rows are never edited retroactively — a changed plan that invalidates a generated packet is a new packet (Cross-Packet Mutation Rule).
