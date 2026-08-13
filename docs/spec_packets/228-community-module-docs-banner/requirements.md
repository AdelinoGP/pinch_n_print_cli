# Requirements: 228-community-module-docs-banner

## Packet Metadata

- Grouped task IDs: `TASK-339`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The governing design spec's §7 delivers the *social rule* — real community modules are authored in forks as pinned submodules and never added to this repository; the committed Dragon Curve is a labeled example only. That rule has three homes: a `CLAUDE.md` instruction, a `docs/` note, and the backlog rows that track the four queue tasks. This packet lands those documentation deliverables in one coherent, code-free slice, so contributors encounter the rule at every entry point.

## In Scope

- `CLAUDE.md`: add a short "Community Modules" section — real community modules are authored in forks as pinned submodules, never added to this repo; the committed dragon-curve module is a labeled example only; reference `docs/14_submodule_programming_languages.md` and `docs/specs/community-modules-dragon-curve-infill.md`. Keep it brief and consistent with the file's existing short-section style.
- `docs/14_submodule_programming_languages.md` §Community-module context: add the labeled-example social-rule note **only if 225's edits do not already cover it** (coordinate by noting the overlap in this file). Do not duplicate 225's verdict rows.
- `docs/07_implementation_status.md`: create backlog rows `TASK-336` (packet 225), `TASK-337` (packet 226), `TASK-338` (packet 227), `TASK-339` (packet 228) in the existing row format (open checkbox `- [ ]`, task description, spec path), verified absent beforehand.
- `docs/specs/community-modules-dragon-curve-infill.md`: update the status line to note the packet queue plan file path (`docs/specs/community-modules-dragon-curve-plan.md`).
- The four doc-grep ACs in `packet.spec.md`, each with a runnable `rg` verification.

## Out of Scope

- Any code, WIT, manifest, or ADR edit.
- `CONTEXT.md` glossary entries (already present — "Community module" at line 127 and "Authored coloring" at line 139, verified).
- OrcaSlicer parity consultation (this queue carries no parity).
- Authoring the 225/226/227 packet files (this packet only registers their backlog rows).
- Editing the dragon-curve module itself.

## Authoritative Docs

- `docs/14_submodule_programming_languages.md` - 172 lines; direct read of §Community-module context (lines 96-171).
- `docs/07_implementation_status.md` - >300 lines; delegate a SUMMARY of the row format + the TASK-330..335 block (never read wholesale).
- `CLAUDE.md` - 202 lines; direct read of the existing section order to place the new section consistently.
- `docs/specs/community-modules-dragon-curve-infill.md` - 279 lines; direct read of line 3 (status line) only.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (CLAUDE.md section), `AC-2` (docs/14 note), `AC-3` (four backlog rows), `AC-4` (spec status line).
- Negative: `AC-N1` (no duplicate/clobber of task rows).
- Cross-packet impact: overlaps draft 225's edits to `docs/14` — this packet must read the current §Community-module context at edit time and add only the labeled-example sentence 225 does not (225 owns the verdict rows). Overlaps draft 227's referent (the module directory the docs point at); the FORWARD-DEP is recorded.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q '^## Community Modules' CLAUDE.md` | section anchor | FACT pass/fail |
| `rg -q 'labeled example' docs/14_submodule_programming_languages.md` | docs/14 note | FACT pass/fail |
| `rg -c '^- \[ \] TASK-33[6-9] —' docs/07_implementation_status.md` | row count == 4 | FACT pass/fail (exact count) |
| `rg -q 'docs/specs/community-modules-dragon-curve-plan.md' docs/specs/community-modules-dragon-curve-infill.md` | status-line pointer | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 1 (backlog rows) and Step 2 (docs/14 note) both depend on reading the current `docs/14` §Community-module context to avoid duplicating 225's edits; do them in order and record the overlap note in `requirements.md` §Acceptance Summary.
- Step 3 (CLAUDE.md) and Step 4 (spec status line) are independent and may be dispatched in parallel after Step 1.
- The `docs/07_implementation_status.md` edit is a targeted `edit_file` insert into the Workstream 5 block (before or after TASK-335), never a full-file rewrite.

## Context Discipline Notes

- `docs/07_implementation_status.md` is large and mutable (a "ledger fact" per `CLAUDE.md`) — never read it wholesale; delegate the row-format SUMMARY and re-verify the four IDs are absent with a single `rg` before editing.
- The four task IDs are pre-reconciled in the plan file, but the edit must still confirm they are absent at edit time (they are ledger facts, not code facts).
- No cargo command is involved; the only verification tools are `rg` greps.
