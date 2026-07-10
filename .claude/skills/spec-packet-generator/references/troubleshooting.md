# Troubleshooting

**When to read:** you hit one of the failure modes below during packet generation.

**Topics:** broad prompts, ambiguous task mapping, missing tasks, active-packet conflicts, missing OrcaSlicer refs, existing directories, context budget exhaustion.

- **Prompt too broad.** Narrow it to one remediation slice and explain the cut before generating files.
- **Task mapping unclear.** Delegate a LOCATIONS dispatch over `docs/07` and present the candidates from the return; ask the user to confirm one via `AskUserQuestion`.
- **No relevant task ids in `docs/07`.** Stop and tell the user the prompt is outside the canonical backlog.
- **Another packet is already active.** Keep the new packet as `draft` and call out the conflict.
- **OrcaSlicer reference missing.** Note that the packet has no OrcaSlicer dependency rather than inventing one.
- **Existing packet directory already present.** Ask whether to overwrite, revise in place, or choose a new slug — never overwrite without explicit approval.
- **Your own context is approaching the 120k reading budget.** Stop populating files. Emit the partial packet, hand off remaining files as a numbered TODO list, and tell the user to resume in a fresh session with the partial packet as input.
