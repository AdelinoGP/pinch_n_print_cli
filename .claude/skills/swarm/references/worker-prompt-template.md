# Worker Prompt Template

**When to read this:** every time you compose a worker prompt in Phase 3.2 of the swarm workflow.

**Topics:** worker prompt structure, execution rules per worker, step-block layout, context-discipline reminders for workers.

The return schema itself stays in core (`SKILL.md` → "Worker return schema") because the planner uses it to parse every reply. This file holds the prose template for what the planner sends *to* the worker.

## Template

```text
Packet: <packet-slug>
Status: <draft|active|implemented|superseded>
Mode: <implement|refine-draft|review-only>

Context discipline: you are bound by the same hard limits as the planner.
- Files allowed to read: <exact list from manifest>
- Files allowed to edit: <exact list from manifest>
- You MUST NOT paste full cargo/test logs in your return. Use the `failing_assertion` field with ≤ 20 lines on failure only.
- You MUST NOT load files outside the allowed-read list, even to "double-check" something. Re-dispatch is the planner's job, not yours.

Packet digest:
- Goal: <1-3 lines>
- Scope: <1-3 lines>
- Relevant acceptance criteria: <AC-1 one-line summary>, <AC-3 one-line summary>

Execution manifest references:
- Step ledger entries: <Step 2, Step 3>
- Allowed files (read): ...
- Allowed files (edit): ...
- Relevant docs: ...

Assigned steps:
- Step N: ...
  - Task IDs: ...
  - Objective: ...
  - Precondition: ...
  - Postcondition: ...
  - Files allowed to read: ...
  - Files allowed to edit: ...
  - Expected sub-agent dispatches: ...
  - Authoritative docs: ...
  - Orca refs: ...
  - Verification: ...
  - Exit condition: ...

Execution rules:
1. Read the listed docs before changing code.
2. Follow the packet's TDD and step ordering rules.
3. Validate immediately after the first substantive edit using the step's narrow command.
4. Do not modify files outside the allowed-edit list.
5. Do not commit or create branches.
6. Return JSON matching the worker return schema.
7. Keep command summaries to one line each. Do not paste full logs unless the failure cannot be summarized in 20 lines.
```

## Notes for the planner

- Prefer references to the execution manifest over repeated copies of the packet text. If a worker only owns one or two steps, include only those steps in full detail.
- The "Assigned steps" block should be a verbatim copy of the step entries from the manifest, not a paraphrase.
- For read-only research workers, replace the return-schema instruction with one of FACT / LOCATIONS / SNIPPETS / SUMMARY and remove the files-to-edit line.