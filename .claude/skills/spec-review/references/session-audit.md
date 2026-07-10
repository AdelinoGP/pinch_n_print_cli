---
when: Read this when running the session scope of `spec-review` — auditing this session's work against the packet being implemented this session ("what did I miss", "are we ready to ship", "PR-review my work"). The mindset, evidence, trap, and verdict-floor rules in SKILL.md apply throughout and are not repeated here.
keywords: session audit, ship, do not ship, deferred, production readiness, deviations, PR review, self-audit
---

# Session Audit (session scope)

Critical senior-engineer review of **this session's work** against the spec packet being implemented in this session. Read-only on code; the only write permitted is appending to the packet's `Deviations` section under the protocol below.

## Inputs

- **Target packet**: the packet you have been implementing this session. If unclear, list `.ralph/specs/*/packet.spec.md` `status:` lines and cross-check against files in `git status` / `git diff`. If still ambiguous, ask the user to name the packet — do not guess.
- **Session changes**: union of `git status` (untracked + modified), `git diff <main-branch>...HEAD`, and your recollection of files edited this session **cross-checked against the diff**. A file you remember editing but cannot find in the diff is `[unverified]` — possibly reverted or never written.

## Workflow

1. **Identify packet and scope.** Read in full: `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md` (and `task-map.md` only if closure depends on it). Run `git status` and `git diff --stat <main>...HEAD` to enumerate concrete changes.

2. **Build an evidence ledger.** For every AC, requirement, and implementation-plan step, record: spec ref · expected behaviour · `file:line` evidence (or `[unverified]`) · status (Implemented / Partial / Missing). For each AC verification command, either dispatch it to a sub-agent for `FACT pass/fail` (never `cargo test --workspace`), or report `[unverified — command not run]`.

3. **Write the report.** Exactly these three sections, in this order, no additions (template in `references/output-format.md`):

   ### 1. DEFERRED / INCOMPLETE
   Every packet item stubbed, TODO'd, partially implemented, or not implemented. For each:
   - `file:line` — what's missing — why deferred (if known).
   - If none: literally write `None — all packet items implemented.`

   ### 2. PRODUCTION READINESS
   One row per change made this session:
   - `file:line` (or file range) — one-line description — **READY** or **NOT READY** *(reason)*.
   Flag explicitly when present: missing error handling, untested paths, hardcoded values, debug/log artifacts, missing input validation, unhandled edge cases.
   Also apply SKILL.md trap #1 to every helper introduced this session: test-only call sites → mark the row `NOT READY (helper not invoked by production driver)`.

   ### 3. PACKET DEVIATIONS
   Every divergence from the spec — different approach, changed interface, added/removed behaviour, or assumption made. For each, draft a line for the packet's `Deviations` section in this exact format:
   - `[Spec ref] — Specified: X | Implemented: Y | Reason: Z`

4. **End with one line, no trailing prose:**
   - `SHIP` — every change is READY, no deferred items, no undocumented deviations.
   - `DO NOT SHIP — <biggest blocker(s), max 2>` — otherwise.

## Deviation-write protocol (the only permitted write)

**Show all proposed packet edits to the user as a single block before writing.** Only after explicit confirmation (`yes`, `apply`, equivalent) append them under a `## Deviations` heading at the end of `packet.spec.md`. If the section already exists, append; do not rewrite existing entries. No other file may be modified.

## Not this scope

- Packet-wide contract review with the five-value verdict and heavy sub-agent dispatch → **packet scope** (`references/packet-review.md`).
- Reviewing changes with no packet as the contract → **code scope** (`references/code-review.md`).
- Applying fixes → not review at all; even obvious bugs are findings.
- Running the full suite → this scope may dispatch narrow commands, never the workspace ceremony.
