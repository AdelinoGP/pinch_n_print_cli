# Design: 228-community-module-docs-banner

## Controlling Code Paths

- Primary code path: documentation prose across `CLAUDE.md`, `docs/14_submodule_programming_languages.md`, `docs/07_implementation_status.md`, and `docs/specs/community-modules-dragon-curve-infill.md`.
- Neighboring tests/fixtures: none — the verification surface is four `rg` greps in `packet.spec.md`.
- OrcaSlicer comparison: none — this packet carries **no** OrcaSlicer parity and no OrcaSlicer Reference Obligations section.

## Architecture Constraints

- The social rule is enforced **socially**, not mechanically (spec §1): banner README + docs note + `CLAUDE.md` instruction. No loader, no allowlist, no CI gate is added here.
- The four backlog rows must match the existing row format exactly: `- [ ] TASK-<n> — <description>. Spec: docs/spec_packets/<slug>/.` (open checkbox, task description, spec path), matching the TASK-330..335 rows at lines 320-325.
- `CONTEXT.md` already carries the "Community module" and "Authored coloring" glossary entries — do not re-add them.
- The docs/14 edit must be **additive only**: read the current §Community-module context at edit time and add only the labeled-example sentence 225's edits do not already contain (225 owns the Go/MoonBit verdict rows).

## Code Change Surface

- Selected approach: four targeted documentation edits, one per file, each with a runnable `rg` AC.
- Exact edits:
  - `CLAUDE.md` - insert a short `## Community Modules` section (matching the file's existing `## <topic>` short-section style) after an existing top-level section, stating: real community modules are authored in forks as pinned submodules, never added to this repo; the committed dragon-curve module is a labeled example only; see `docs/14_submodule_programming_languages.md` and `docs/specs/community-modules-dragon-curve-infill.md`.
  - `docs/14_submodule_programming_languages.md` §Community-module context - append one sentence: the committed `modules/community-modules/dragon-curve/` is a labeled example only; real community modules are authored in forks as pinned submodules and never added to this repository (coordinate with 225's edits to avoid duplication).
  - `docs/07_implementation_status.md` - insert four rows: `TASK-336` (packet 225), `TASK-337` (packet 226), `TASK-338` (packet 227), `TASK-339` (packet 228), in the Workstream 5 block after the TASK-330..335 family rows, each `- [ ]`.
  - `docs/specs/community-modules-dragon-curve-infill.md` - update line 3's status line to append: "Packet queue plan: `docs/specs/community-modules-dragon-curve-plan.md`."
- Rejected alternatives and reasons:
  - A dedicated new `docs/` file for the social rule (rejected: the spec's §7 names three existing homes; a new file adds a fourth discovery surface without need).
  - A loader/CI mechanical enforcement (rejected: spec §1 says "socially, not mechanically").
  - Rewriting the whole `docs/07_implementation_status.md` (rejected: it is a large append-only ledger; targeted insert only).

## Files in Scope (read + edit)

- `CLAUDE.md` - role: contributor-facing instruction; expected change: new "Community Modules" section.
- `docs/14_submodule_programming_languages.md` - role: living language verdict + social-rule note; expected change: one additive sentence in §Community-module context.
- `docs/07_implementation_status.md` - role: backlog; expected change: four new task rows.
- `docs/specs/community-modules-dragon-curve-infill.md` - role: governing spec status line; expected change: append the queue-plan pointer.

## Read-Only Context

- `docs/14_submodule_programming_languages.md` - lines `96-171` (§Community-module context) - purpose: current prose to avoid 225 overlap.
- `docs/07_implementation_status.md` - lines `320-325` (TASK-330..335 rows) - purpose: exact row format to mirror.
- `CLAUDE.md` - lines `1-27` (section order) - purpose: place the new section consistently.
- `docs/specs/community-modules-dragon-curve-infill.md` - line `3` - purpose: the status line to amend.

## Out-of-Bounds Files

- `docs/spec_packets/225-*/**`, `226-*/**`, `227-*/**` - do not load; the plan file's Central Symbol Contract and this packet's coordination note are the only cross-packet surface.
- `CONTEXT.md` - read-only (glossary already present; do not edit).
- `docs/DEVIATION_LOG.md` - out of scope (no deviation authored here).
- `OrcaSlicerDocumented/`, `target/`, `Cargo.lock` - never load.

## Expected Sub-Agent Dispatches

- Question: what is the exact current prose of `docs/14_submodule_programming_languages.md` §Community-module context (does it already contain the labeled-example sentence)? scope: that section only; return: `SNIPPETS` (≤20 lines); purpose: avoid 225 duplication.
- Question: what is the exact row format and surrounding block for the TASK-330..335 rows in `docs/07_implementation_status.md`? scope: lines 320-325; return: `SNIPPETS` (≤10 lines); purpose: match the format.

## Data and Contract Notes

- IR/manifest contracts: none.
- WIT boundary: none.
- Determinism/scheduler constraints: none (docs-only; the four rows and section anchors are idempotent).

## Locked Assumptions and Invariants

- **L1** — the four task IDs `TASK-336`..`TASK-339` are absent from `docs/07_implementation_status.md` today (the plan file says so, but the edit must re-verify with `rg` at edit time — a ledger fact).
- **L2** — `CONTEXT.md` already carries the two glossary entries; no edit is made there.
- **L3** — 225's edits to `docs/14` own the verdict rows; this packet only adds the labeled-example sentence if 225 does not.

## Risks and Tradeoffs

- The docs/14 coordination depends on reading 225's eventual edit; since 225 is draft and its directory does not exist, the implementer reads the current `docs/14` at edit time and records the overlap note — if 225 lands first, the labeled-example sentence may already be present and AC-2's `rg` still passes (the sentence text is idempotent).
- The backlog row description text for the four tasks is authored here (matching the plan file's one-sentence goals); if 225/226/227 later rename their slugs, the row's spec path must be updated in that packet, not here.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (the backlog-row edit is the only multi-row insertion)
- Highest-risk dispatch and required return format: the docs/14 §Community-module context read — `SNIPPETS` (≤20 lines) to detect 225 duplication.

## Open Questions

- None `[FWD]` — the only coordination point (does 225's docs/14 edit already carry the labeled-example sentence) is resolved at edit time by the dispatch above, not left open.
- None `[BLOCK]` — the single FORWARD-DEP on draft 227 is recorded in `packet.spec.md`.
