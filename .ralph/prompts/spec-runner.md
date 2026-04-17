# ModularSlicer Spec Runner

Execute one prepared ModularSlicer packet and nothing else.

## Backlog Source

- `./docs/07_implementation_status.md` is the canonical backlog and prioritization source.
- It is not permission to broaden scope beyond the active packet.

## Active Packet Selection

1. Search `./.ralph/specs/*/packet.spec.md`, excluding `./.ralph/specs/_templates/`.
2. Exactly one packet must declare `status: active` in front matter.
3. If zero or multiple packets are active, stop and report the exact blocking condition.
4. After selection, treat only that packet folder as the run-scoped requirement set.

## Required Packet Inputs

- `packet.spec.md` — thin contract and Given/When/Then acceptance criteria
- `requirements.md` — grouped task IDs, scope boundaries, docs, and OrcaSlicer obligations
- `design.md` — intended technical shape and decisive code paths
- `implementation-plan.md` — atomic steps and verification commands
- `task-map.md` — optional mapping back to `./docs/07_implementation_status.md`

## Operating Rules

- Only the packet's grouped task IDs are in scope.
- Use the normative doc map in `./docs/00_project_overview.md` to resolve authoritative sources.
- Use runtime tasks for active step tracking and `./.ralph/agent/memories.md` for learned context.
- Apply TDD and run the narrowest falsifying checks before broad workspace gates.
- Inspect `./OrcaSlicerDocumented/` when the packet asks for reference behavior.
- Finish the run only after the packet acceptance criteria and verification commands are green, `./docs/07_implementation_status.md` is updated for the packet task IDs, and you emit `SPEC_PACKET_COMPLETE`.
