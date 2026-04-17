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
- `task-map.md` when the packet needs an explicit mapping back to `./docs/07_implementation_status.md`

## Authoring Workflow

1. Pick a small, related set of task IDs from `./docs/07_implementation_status.md`.
2. Copy the files from `./.ralph/specs/_templates/` into a new `./.ralph/specs/<spec-slug>/` folder.
3. Fill in grouped task IDs, scope boundaries, authoritative docs, acceptance criteria, and OrcaSlicer reference obligations.
4. Add atomic steps and verification commands to `implementation-plan.md`.
5. Mark exactly one packet `status: active` in `packet.spec.md`.
6. Run `ralph preflight` and then `ralph run -c ralph.yml`.

If you keep multiple drafted packets around, leave them `status: draft` and never mark more than one packet active.
