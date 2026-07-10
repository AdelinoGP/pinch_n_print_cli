---
status: implemented
packet: [spec-slug]
task_ids:
  - TASK-000
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S | M    # aggregate cost; never L for an active packet
copy_note: This file lives in the spec-packet-generator skill. The skill writes a copy into ./.ralph/specs/<spec-slug>/ with status set to draft or active.
---

# Packet Contract: [spec-slug]

Template note: this template is marked `status: implemented` so preflight review ignores it if it is ever read in place. The generator overwrites the status when emitting a real packet.

## Goal

One sentence: the single remediation slice this packet owns. Solution-shaped, not motivation-shaped (motivation belongs in `requirements.md` Problem Statement).

## Scope Boundaries

Prose paragraph, 2–3 sentences. State the bounding box of the change in solution terms. Full in/out-of-scope lists live in `requirements.md`; this section is the preflight-visible summary, not a duplicate.

## Prerequisites and Blockers

- Depends on:
- Unblocks:
- Activation blockers:

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** [initial condition], **when** [action], **then** [observable result]. | `verification-command`
- **AC-2. Given** [second condition], **when** [second action], **then** [second observable result]. | `verification-command`

Each criterion must end with a pipe `|` and a runnable verification command. If multiple criteria share the same verification, repeat the command in each criterion (do not write "see AC-N").

Name exact assertion content in the criterion text. Prefer exact fields, paths, counts, error variants, or output fragments over phrases like "all required fields" or "correct diagnostics".

Each verification command must be **delegation-friendly**: it produces a small, parseable output (exit code, single assertion, JSON path). Commands that dump > 200 lines of log on success should be wrapped or filtered so a sub-agent can return a FACT.

## Negative Test Cases

- **AC-N1. Given** [rejection or failure condition], **when** [action], **then** [observable failure or validation result]. | `verification-command`

Include this section whenever the packet changes validation, enforcement, contract boundaries, or error-handling behavior.

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `[the single targeted integration test that proves the packet's main contract]`

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md`

For each doc, note whether the implementer should load it directly or delegate the read (delegate when the doc is > 300 lines or only one section is needed).

## Doc Impact Statement (Required)

State exactly **one** of the following:

1. **`none`** — and a one-line rationale why no `/docs/` change is needed. Acceptable examples: "test-only acceptance harness", "internal refactor with no public surface change", "bug fix that does not alter contracts". Refactors that change any IR field, WIT type, scheduler rule, claim ID, manifest schema, host service, or module SDK contract are **not** eligible for `none`.

2. **A list of specific doc sections that this packet adds or modifies**, with one verification grep per section so closure can be checked mechanically:

   - `docs/02_ir_schemas.md` §"<section name>" — `rg -q '<unique anchor phrase>' docs/02_ir_schemas.md`
   - `docs/03_wit_and_manifest.md` §"<section name>" — `rg -q '<unique anchor phrase>' docs/03_wit_and_manifest.md`

The doc edits must land in the same packet (not deferred to a follow-up); the verification greps are appended to the Acceptance Criteria above and gate packet close. The `spec-review` skill checks this section is non-empty and that every grep returns a hit before a packet may flip to `status: implemented`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

(Include the verbatim opening paragraph from `references/snippets/orca-delegation.md`, then list the specific `OrcaSlicerDocumented/` files this packet borrows from. Skip this entire section if no OrcaSlicer parity is involved.)

<!-- snippet: context-discipline -->
## Context Discipline Note

(Include the verbatim block from `references/snippets/context-discipline.md`. This snippet is mandatory for every packet.)
