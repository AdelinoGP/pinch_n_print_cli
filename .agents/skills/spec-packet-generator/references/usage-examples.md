---
when: Read when asked how to invoke this skill.
keywords: invocation, input, task_ids, packet_prefix, packet_number, packet_name, status
---

# Usage Examples

```text
/spec-packet-generator input:"Rework TASK-121 and TASK-122 into one manifest contract packet" task_ids:TASK-121,TASK-122
```

Standalone packets (no plan) still require a prefix; the number defaults to the
next free `NN` for that prefix (here: `manifest-contract_01_task-121-contract`):

```text
/spec-packet-generator input:notes/task-121-prompt.md packet_prefix:manifest-contract packet_name:task-121-contract status:draft
```

Plan-driven packets derive the prefix from the plan file (and `NN` from the
queue row); no prefix argument is needed:

```text
/spec-packet-generator input:docs/specs/test-quality-remediation-plan.md status:draft
```
