# Spec Packets

This directory holds per-run execution packets for Ralph.

`./docs/07_implementation_status.md` remains the canonical backlog. A packet narrows that backlog to one coherent remediation slice so Ralph does not re-plan the whole project on each run.

## Runtime Rules

- Packet folders live at `./.ralph/specs/<spec-slug>/`.
- Exactly one packet may be active at a time.
- The active packet is the folder whose `packet.spec.md` front matter sets `status: active`.
- `packet.spec.md` is the preflight-visible contract and must carry real Given/When/Then acceptance criteria.
- Completed packets should be marked `status: implemented` so Ralph preflight can skip them.

## Packet Shape

Each packet should contain:

- `packet.spec.md`
- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` when the packet spans more than one task ID, reopens or supersedes earlier packet work, or needs an explicit mapping back to `./docs/07_implementation_status.md`

## Authoring Workflow

1. Pick a small, related set of task IDs from `./docs/07_implementation_status.md`.
2. Copy the files from `./.ralph/specs/_templates/` into a new `./.ralph/specs/<spec-slug>/` folder.
3. Fill in grouped task IDs, scope boundaries, authoritative docs, acceptance criteria, and OrcaSlicer reference obligations.
4. Make the packet implementation-grade before activation: acceptance criteria must name exact assertion content, include a negative or rejection case when the slice changes validation or failure behavior, and avoid unresolved open questions.
5. Add atomic steps, step exit criteria, and targeted verification commands to `implementation-plan.md`.
6. Mark exactly one packet `status: active` in `packet.spec.md` only after blockers and open questions are resolved.
7. Run `ralph preflight` and then `ralph run -c ralph.yml`.

If you keep multiple drafted packets around, leave them `status: draft` and never mark more than one packet active.

## Activation Gate

Do not activate a packet if any of the following remain true:

- A Given/When/Then criterion lacks a pipe-suffixed verification command.
- The packet uses vague acceptance language such as "all required fields" without naming the exact fields or outputs.
- The packet changes validation, enforcement, or error handling but includes no negative or rejection criterion.
- `design.md` still contains open questions that would change implementation scope.
- `implementation-plan.md` steps lack explicit exit conditions, preconditions, or postconditions.
