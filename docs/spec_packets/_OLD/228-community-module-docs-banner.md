---
status: implemented
packet: 228-community-module-docs-banner
task_ids:
  - TASK-339
---

# 228-community-module-docs-banner

## Goal

Land the social-rule documentation deliverables for community modules: the `CLAUDE.md` instruction, the `docs/14_submodule_programming_languages.md` labeled-example note, the four backlog rows in `docs/07_implementation_status.md`, and the queue-plan pointer in the governing spec's status line.

## Problem Statement

The governing design spec's §7 delivers the *social rule* — real community modules are authored in forks as pinned submodules and never added to this repository; the committed Dragon Curve is a labeled example only. That rule has three homes: a `CLAUDE.md` instruction, a `docs/` note, and the backlog rows that track the four queue tasks. This packet lands those documentation deliverables in one coherent, code-free slice, so contributors encounter the rule at every entry point.

## Architecture Constraints

- The social rule is enforced **socially**, not mechanically (spec §1): banner README + docs note + `CLAUDE.md` instruction. No loader, no allowlist, no CI gate is added here.
- The four backlog rows must match the existing row format exactly: `- [ ] TASK-<n> — <description>. Spec: docs/spec_packets/<slug>/.` (open checkbox, task description, spec path), matching the TASK-330..335 rows at lines 320-325.
- `CONTEXT.md` already carries the "Community module" and "Authored coloring" glossary entries — do not re-add them.
- The docs/14 edit must be **additive only**: read the current §Community-module context at edit time and add only the labeled-example sentence 225's edits do not already contain (225 owns the Go/MoonBit verdict rows).

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
