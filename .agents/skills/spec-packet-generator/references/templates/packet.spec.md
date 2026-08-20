---
status: implemented
packet: [spec-slug]
task_ids:
  - TASK-000
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S | M
copy_note: Template only; emitted copies use draft or explicitly approved active status.
---

# Packet Contract: [spec-slug]

This template is `implemented` so preflight ignores it in place. Replace all placeholders in emitted copies.

## Goal

[One solution-shaped sentence. Motivation belongs in `requirements.md`.]

## Scope Boundaries

[A 2-3 sentence prose bounding box. Full lists belong in `requirements.md`.]

## Prerequisites and Blockers

- Depends on:
- Unblocks:
- Activation blockers:

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** [condition], **when** [action], **then** [exact observable result]. | `[delegation-friendly verification command]`
- **AC-2. Given** [condition], **when** [action], **then** [exact observable result]. | `[verification command]`

Every AC names exact fields, paths, counts, errors, variants, or output fragments and ends with its own runnable command. Repeat shared commands; never write "see AC-N". Commands that dump more than 200 successful output lines must be wrapped or filtered so a subagent can return a FACT.

AC verification command rule (mandatory): each pipe-suffixed command's `--test` binary must be one that can actually drive the asserted behavior. Before authoring an AC that requires an end-to-end driver (e.g. `run_slice`, full pipeline, real module dispatch), verify the target test binary has a working setup for that driver today — dispatch a `LOCATIONS` check for existing call sites. If the target binary is the wrong home (e.g. `--test unit` for a `run_slice` test when no unit test today calls `run_slice`), either pick a binary that has the setup, or author a shim/test fixture in the same step and document the shim in `requirements.md` §In Scope.

## Negative Test Cases

- **AC-N1. Given** [rejection/failure condition], **when** [action], **then** [observable rejection]. | `[verification command]`

Include this section for validation, enforcement, contract-boundary, or error-path changes.

## Verification

List only 2-3 closure-gate commands; the full matrix belongs in `requirements.md`. Use `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` as defaults.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `[targeted integration test proving the primary contract]`

## Authoritative Docs

- `[docs/path.md]` - [state direct range read or delegated summary; delegate when the doc is over 300 lines or only one section applies]

## Doc Impact Statement (Required)

Choose exactly one:

- **`none`** - [one-line rationale; only for work that changes no IR, WIT, scheduler, claim, manifest, host-service, or SDK contract].
- Specific same-packet doc edits; each entry must list a verification grep, for example: `docs/02_ir_schemas.md` section "<name>" - `rg -q '<anchor>' docs/02_ir_schemas.md`. The full list must contain one grep per edited section.

Append doc greps to the ACs. `spec-review` requires this section and verifies each grep before `status: implemented`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

[When parity applies, replace this placeholder with the exact snippet and packet-specific paths. Otherwise omit the entire section.]

<!-- snippet: context-discipline -->
## Context Discipline Note

[Replace with the exact mandatory context-discipline snippet.]
