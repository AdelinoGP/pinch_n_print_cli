---
status: draft
packet: 228-community-module-docs-banner
task_ids:
  - TASK-339
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 228-community-module-docs-banner

## Goal

Land the social-rule documentation deliverables for community modules: the `CLAUDE.md` instruction, the `docs/14_submodule_programming_languages.md` labeled-example note, the four backlog rows in `docs/07_implementation_status.md`, and the queue-plan pointer in the governing spec's status line.

## Scope Boundaries

This packet edits four documentation files only — `CLAUDE.md`, `docs/14_submodule_programming_languages.md`, `docs/07_implementation_status.md`, and `docs/specs/community-modules-dragon-curve-infill.md` — to state the fork/submodule social rule and register the four task rows. It authors no code, no WIT, no manifest, and no ADR.

## Prerequisites and Blockers

- Depends on: draft `227-dragon-curve-community-module` (the module exists so the banner has a real referent).
- Unblocks: nothing downstream — this is the terminal packet of the queue.
- Activation blockers: **FORWARD-DEP on draft 227-dragon-curve-community-module** — the docs note references `modules/community-modules/dragon-curve/`; the note is written but its referent does not exist until 227 lands. The doc edits themselves are non-code and can be drafted in parallel, but activation (status flip) is gated on 227.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** `CLAUDE.md`, **when** the new "Community Modules" section is inspected, **then** it states that real community modules are authored in forks as pinned submodules and never added to this repo, and that the committed dragon-curve module is a labeled example only, referencing `docs/14_submodule_programming_languages.md` and the spec. | `rg -q '^## Community Modules' CLAUDE.md && rg -q 'never added to this repository' CLAUDE.md && rg -q 'labeled example only' CLAUDE.md && rg -q 'docs/14_submodule_programming_languages.md' CLAUDE.md`
- **AC-2. Given** `docs/14_submodule_programming_languages.md` §Community-module context, **when** the labeled-example social rule is inspected, **then** the section states real community modules are authored in forks as pinned submodules and never added here, with the committed Dragon Curve as a labeled example only, without duplicating 225's verdict edits. | `rg -q 'labeled example' docs/14_submodule_programming_languages.md && rg -q 'never added' docs/14_submodule_programming_languages.md`
- **AC-3. Given** `docs/07_implementation_status.md`, **when** the four new backlog rows are inspected, **then** `TASK-336`, `TASK-337`, `TASK-338`, and `TASK-339` each appear exactly once in the existing row format (open checkbox, task description, spec path), and none was present before this packet. | `rg -q '^- \[ \] TASK-336 —' docs/07_implementation_status.md && rg -q '^- \[ \] TASK-337 —' docs/07_implementation_status.md && rg -q '^- \[ \] TASK-338 —' docs/07_implementation_status.md && rg -q '^- \[ \] TASK-339 —' docs/07_implementation_status.md`
- **AC-4. Given** `docs/specs/community-modules-dragon-curve-infill.md`, **when** its status line is inspected, **then** it retains the grilling-complete note and adds the packet queue plan path `docs/specs/community-modules-dragon-curve-plan.md`. | `rg -q 'Status: grilling complete; ready to be broken into spec packets by a downstream session' docs/specs/community-modules-dragon-curve-infill.md && rg -q 'docs/specs/community-modules-dragon-curve-plan.md' docs/specs/community-modules-dragon-curve-infill.md`

## Negative Test Cases

- **AC-N1. Given** the pre-packet `docs/07_implementation_status.md`, **when** the packet's edit is applied, **then** the four task IDs were absent before the edit and present exactly once after (no duplicate rows, no clobber of the existing `TASK-330`/`TASK-331` rows at lines 57-58 and 320-321). | `rg -c '^- \[ \] TASK-33[6-9] —' docs/07_implementation_status.md` returns `4`, and each of `TASK-336`/`TASK-337`/`TASK-338`/`TASK-339` has a single `^- \[ \]` occurrence.

## Verification

- `rg -n '^## Community Modules' CLAUDE.md` (AC-1 anchor present).
- `rg -n '^- \[ \] TASK-33[6-9] —' docs/07_implementation_status.md` (AC-3 rows, one each).
- `rg -n 'Status: grilling complete' docs/specs/community-modules-dragon-curve-infill.md` (AC-4 status line intact).

## Authoritative Docs

- `docs/14_submodule_programming_languages.md` - 172 lines; direct read of §Community-module context only (lines 96-171).
- `docs/07_implementation_status.md` - >300 lines; delegated SUMMARY of the tail row format + the TASK-330..335 block; never read wholesale.
- `CLAUDE.md` - 202 lines; direct read of the section ordering to place the new section consistently.
- `docs/specs/community-modules-dragon-curve-infill.md` - 279 lines; direct read of the status line (line 3) only.

## Doc Impact Statement (Required)

- `CLAUDE.md` section "Community Modules" - `rg -q '^## Community Modules' CLAUDE.md`.
- `docs/14_submodule_programming_languages.md` section "Community-module context" - `rg -q 'labeled example' docs/14_submodule_programming_languages.md`.
- `docs/07_implementation_status.md` backlog rows - `rg -q '^- \[ \] TASK-336 —' docs/07_implementation_status.md`.
- `docs/specs/community-modules-dragon-curve-infill.md` status line - `rg -q 'docs/specs/community-modules-dragon-curve-plan.md' docs/specs/community-modules-dragon-curve-infill.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
