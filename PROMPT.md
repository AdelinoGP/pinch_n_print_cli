# ModularSlicer Ralph Entry Note

Root `PROMPT.md` is no longer the execution contract for this repository.

Ralph now runs from `./.ralph/prompts/spec-runner.md` via `./ralph.yml`, and each implementation run must point at one prepared packet under `./.ralph/specs/<spec-slug>/`.

`./docs/07_implementation_status.md` remains the canonical backlog and prioritization source.

## Packet Workflow

1. Choose a small, coherent task group from `./docs/07_implementation_status.md`.
2. Copy the templates from `./.ralph/specs/_templates/` into `./.ralph/specs/<spec-slug>/`.
3. Fill in `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, and `task-map.md` if needed.
4. Mark exactly one packet `status: active` in `packet.spec.md`.
5. Run `ralph preflight` and then `ralph run -c ralph.yml`.

This file is intentionally human-facing and thin so stale planner instructions are never picked up implicitly.
