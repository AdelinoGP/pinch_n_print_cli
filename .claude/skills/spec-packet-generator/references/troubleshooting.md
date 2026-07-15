---
when: Read when packet generation hits one of these failure modes.
keywords: scope, task mapping, active packet, OrcaSlicer, overwrite, budget
---

# Troubleshooting

- **Prompt too broad:** narrow to one remediation slice and explain the cut before generating.
- **Task mapping unclear:** delegate a `LOCATIONS` search over `docs/07`; present candidates and confirm one via `AskUserQuestion`.
- **No relevant `docs/07` task:** stop; the request is outside the canonical backlog.
- **Another packet active:** keep the new packet `draft` and report the conflict.
- **No OrcaSlicer reference:** state that no OrcaSlicer dependency was found; never invent one.
- **Packet directory exists:** ask whether to overwrite, revise in place, or choose another slug.
- **Approaching the 120k read budget:** stop populating files; emit partial work and a numbered fresh-session handoff.
