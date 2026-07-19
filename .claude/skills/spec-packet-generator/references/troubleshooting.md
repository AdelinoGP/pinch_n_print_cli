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
- **Step adds a struct field or schema/version constant but does not list the struct-literal or test-assertion blast radius in "Files allowed to edit":** the implementation worker will report PARTIAL when `cargo check --workspace --all-targets` fails on test files. Re-author the step with the blast-radius list (LOCATIONS-dispatched first) inside "Files allowed to edit", and add the test-assertion fallout to the step's verification commands. See `references/templates/implementation-plan.md` Blast-radius discipline.
- **AC verification command targets a test binary that has never driven the asserted behavior:** AC fails the "test binary can do it" check. Either pick a binary that has the end-to-end setup (LOCATIONS-dispatched), or author a shim or test fixture in the same step and document it in `requirements.md` §In Scope. See `references/templates/packet.spec.md` AC verification command rule.
