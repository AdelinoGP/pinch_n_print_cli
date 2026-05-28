---
name: spec-audit-session
description: Senior-engineer PR-style audit of this session's work against the spec packet being implemented this session. Produces a strict three-section report (Deferred/Incomplete, Production Readiness, Packet Deviations) ending in a one-line SHIP / DO NOT SHIP verdict. Read-only on code; the only writes permitted are appending entries to the packet's Deviations section, and only after showing the diff and getting explicit user confirmation. Use when the user asks to audit, critique, or PR-review this session's work, asks "what did I miss" or "are we ready to ship", wants a senior-engineer review of the current diff, or wants deviations recorded before packet closure.
---

# Spec Audit (Session)

Critical senior-engineer review of **this session's work** against the spec packet being implemented in this session. Read-only on code. The only writes permitted are to the packet's `Deviations` section, and only after showing the proposed edits.

## Mindset

You are a senior engineer reviewing this PR cold. You did not write the code. **Trust nothing from session memory.** Verify every claim against the working tree, `git diff`, and the packet docs. When evidence is not on disk, mark the row `[unverified]` and continue — do not hand-wave.

Bias toward finding problems. "Looked fine while I was building it" is not evidence. Be a critic.

## Inputs

- **Target packet**: the packet you have been implementing this session. If unclear, list `.ralph/specs/*/packet.spec.md` `status:` lines and cross-check against files in `git status` / `git diff`. If still ambiguous, ask the user to name the packet — do not guess.
- **Session changes**: union of `git status` (untracked + modified), `git diff <main-branch>...HEAD`, and your recollection of files edited this session **cross-checked against the diff**. A file you remember editing but cannot find in the diff is `[unverified]` — possibly reverted or never written.

## Workflow

1. **Identify packet and scope.** Read in full: `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md` (and `task-map.md` only if closure depends on it). Run `git status` and `git diff --stat <main>...HEAD` to enumerate concrete changes.

2. **Build an evidence ledger.** For every AC, requirement, and implementation-plan step, record: spec ref · expected behaviour · `file:line` evidence (or `[unverified]`) · status (Implemented / Partial / Missing). For each AC verification command, either dispatch it to a sub-agent for `FACT pass/fail` (never `cargo test --workspace`), or report `[unverified — command not run]`.

3. **Write the report.** Exactly these three sections, in this order, no additions:

   ### 1. DEFERRED / INCOMPLETE
   Every packet item stubbed, TODO'd, partially implemented, or not implemented. For each:
   - `file:line` — what's missing — why deferred (if known).
   - If none: literally write `None — all packet items implemented.`

   ### 2. PRODUCTION READINESS
   One row per change made this session:
   - `file:line` (or file range) — one-line description — **READY** or **NOT READY** *(reason)*.
   Flag explicitly when present: missing error handling, untested paths, hardcoded values, debug/log artifacts, missing input validation, unhandled edge cases.

   ### 3. PACKET DEVIATIONS
   Every divergence from the spec — different approach, changed interface, added/removed behaviour, or assumption made. For each, draft a line for the packet's `Deviations` section in this exact format:
   - `[Spec ref] — Specified: X | Implemented: Y | Reason: Z`

   **Show all proposed packet edits to the user as a single block before writing.** Only after explicit confirmation (`yes`, `apply`, equivalent) append them under a `## Deviations` heading at the end of `packet.spec.md`. If the section already exists, append; do not rewrite existing entries. No other file may be modified.

4. **End with one line, no trailing prose:**
   - `SHIP` — every change is READY, no deferred items, no undocumented deviations.
   - `DO NOT SHIP — <biggest blocker(s), max 2>` — otherwise.

## Rules (invariants)

- **Read-only on code.** Do not edit, refactor, format, or "fix while you're here". Even obvious bugs are findings, not fixes.
- **Never fabricate `file:line`.** If you cannot open the file and confirm the symbol, the row is `[unverified]`.
- **Never run `cargo test --workspace`** speculatively. Dispatch the packet's narrow verification commands instead, or report `[unverified — command not run]`.
- **Never write to the packet without showing the diff first.** No silent edits.
- **Session memory is not evidence.** `git status` / `git diff` / `rg` / file reads are.
- **One verdict line.** No paragraphs after `SHIP` / `DO NOT SHIP`.

## What this skill is NOT

- Not `spec-review` — that's a full packet review with heavy sub-agent dispatch and a five-value verdict enum. This is a session-scoped self-audit ending in SHIP / DO NOT SHIP.
- Not `code-review` / `simplify` — no fixes applied.
- Not a verification runner — it may dispatch a command, but it does not run the full suite.
