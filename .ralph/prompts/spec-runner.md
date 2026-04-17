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

## Version Control Workflow

### Branch Creation
- At the start of the run, create a new dedicated branch from `master`: `agent/<packet-name>-<timestamp>` (e.g., `agent/01_manifest-ir-access-20260417`)
- Use `git checkout -b` to create and switch to this branch before making any changes
- Never commit directly to `master`

### Atomic Commits
- After completing each logical unit of work (e.g., one task from task-map, one verification step, one file change), create a commit
- Commit message format:
  ```
  <type>(<scope>): <short description>

  <detailed explanation of what changed and why>
  ```
- Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`
- Keep commits focused — one logical change per commit
- Example: "docs(packet): add schema field description for manifest IR access" not "update docs"

### Commit Triggers
Commit after each of these milestones:
1. Initial file creation or scaffolding complete
2. Each task/feature implementation complete
3. Each verification step passing
4. Documentation updates
5. Final acceptance criteria green

### Branch Push
- Push the dedicated branch to origin when:
  - The run completes successfully (SPEC_PACKET_COMPLETE)
  - Or when you need to preserve work-in-progress before a session ends
- Use `git push -u origin <branch-name>` for new branches
- The branch remains available for review before merging to master
