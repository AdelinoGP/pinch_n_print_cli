---
name: spec-review
description: Adversarial, evidence-based review for this codebase in three scopes — packet (review a spec packet under `.ralph/specs/` against its docs and implementation, including the `--preflight` authoring gate), session (audit this session's work against the packet being implemented; SHIP / DO NOT SHIP), and code (review working-tree changes or the diff since a fixed point — branch, tag, commit — against the architecture contract before commit). Use when reviewing, verifying, or auditing a packet or before packet closure; when the user asks to audit, critique, or PR-review this session's work, asks "what did I miss" or "are we ready to ship", or wants deviations recorded; or when reviewing uncommitted changes, a branch, a feature, or specific files before commit, or asks to "review since X".
type: anthropic-skill
version: "2.1"
metadata:
  internal: true
---

# Review

One review skill, three scopes. Replaces the former `review`, `spec-review`, and `spec-audit-session` skills. The core rules in this file apply to **every** scope; per-scope workflow lives in `references/`.

**Scope target**: `$ARGUMENTS` (packet name or path → packet scope; file paths / "all-changes" / a git ref to review since → code scope; a session-audit request → session scope).

## Scope selection

| Scope | Reviewing what, against what | Verdict | Workflow file |
|---|---|---|---|
| **packet** | A `.ralph/specs/<NN>_<slug>/` packet's implementation against its 5 packet docs | APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED / BLOCKED / DEFERRED | `references/packet-review.md` |
| **session** | This session's work (git diff + working tree) against the packet being implemented this session | SHIP / DO NOT SHIP | `references/session-audit.md` |
| **code** | Uncommitted changes, named files, or the diff since a fixed point — against the architecture contract plus a Fowler smell baseline, no packet as primary contract | APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED | `references/code-review.md` |

Pick by target, not phrasing: "review packet 113" → packet. "what did I miss this session" / "are we ready to ship" → session. "review my changes" / "review foo.rs" / "review since `<ref>`" with no packet involved → code. **Packet scope (full mode) is the only path that may authorize packet closure.** If genuinely ambiguous, ask.

Read the scope's workflow file before starting. `--preflight <packet>` is packet scope — read `references/preflight-gate.md`.

## Mindset (all scopes)

You are a senior engineer reviewing this work cold. You did not write it — and in session scope, where you literally did, that is more reason to distrust your memory, not less. **Session memory is not evidence.** `git status` / `git diff` / `rg` / file reads / dispatched FACTs are.

- Bias toward finding problems. "Looked fine while it was built" is not evidence. Be a critic, not a confirmer.
- **Burden of proof is on the implementation.** A claim with no evidence behind it is `[unverified]`; an AC with no passing dispatched evidence is FAIL — never PASS-by-default.
- A sub-agent reply of "pass" that does not quote its evidence (file:line, assertion, or command result line) is not evidence. Reject it and re-dispatch.
- **Re-verify on disagreement.** When a re-check contradicts an earlier claim (yours or a sub-agent's), the re-check wins, and you say so openly. Ground every load-bearing PASS/FAIL in a real-tree grep; re-run "feels wrong" results with absolute paths from the repo root before concluding. One false sub-claim does not condemn a whole sweep — re-verify each load-bearing claim independently.
- **Never fabricate `file:line`.** If you cannot open the file and confirm the symbol, the row is `[unverified]`.
- **Findings, not fixes.** Review is read-only on code — do not edit, refactor, or "fix while you're here"; even obvious bugs are findings. Sole exception: session scope may append to the packet's `Deviations` section after explicit user confirmation (see `references/session-audit.md`).
- Provide specific fixes, not "improve this". Acknowledge good work — positive observations matter.

## Verdict floors (all scopes)

Leniency is this skill's historical failure mode; these floors are hard.

- Any `[unverified]` load-bearing row (an AC, requirement, or session change) → verdict capped at CHANGES REQUESTED / DO NOT SHIP. No APPROVED, no SHIP.
- Any Critical finding → CHANGES REQUESTED, BLOCKED, or DO NOT SHIP.
- Delta review never authorizes closure. A full review that does not fit budget → DEFERRED with a handoff block. **Budget pressure defers closure; it never waives review.**
- Preflight-gate failure (S4/S5/S6) → the packet cannot be APPROVED.

## Known traps (all scopes; each caught a real shipped defect)

1. **Helper passes, driver never calls it** (P95 W6/W8; survived two closure logs): a helper's unit tests stay green while the production driver never invokes it. For any helper introduced by the work under review, verify a production call site: `rg '<helper>\s*\(' --type rust --glob '!**/tests/**' --glob '!**/*_test*.rs'`. Zero non-test hits = dead code from the pipeline's view = HIGH finding; the AC/change is PARTIAL / NOT READY regardless of test-pass status.
2. **Placeholder tests**: a test whose asserts only check pre-existing fixture data, or that contains `// TBD` / `// not yet implemented` / `// DOCUMENTED EXPECTATION`, is not evidence its AC is met. SNIPPETS-dispatch the test body; confirm its assertions reference the symbols / IR fields the claim names. HIGH finding; AC → PARTIAL.
3. **Unregistered test file — "0 tests run" false pass**: a new file under an aggregated test binary (`tests/contract/`, `tests/unit/`, …) with no `mod` registration silently never compiles into the binary; `cargo test --test <bin> <filter>` reports 0 tests and looks green. Confirm the aggregator registration.
4. **Stale guest WASM**: before attributing any guest / component / module-dispatch test failure to the changes (or to "flakiness"), run `cargo xtask build-guests --check`; if `STALE:`, rebuild and re-run. A stale guest is the reviewer's bug until `--check` proves otherwise (CLAUDE.md).
5. **Fictional symbols** (2026-06 wave: ≥1 defect in every packet 104–112): claims naming functions / fields / enum variants / WIT types / ADR slots that don't exist or have a different shape. Resolving names against the real tree is the only defense — prose review cannot see this. Packet scope runs the full S0–S8 gate (`references/preflight-gate.md`); other scopes apply the same principle to any pre-existing symbol a claim leans on.

## Context discipline (all scopes)

Reviews are the most context-hostile activity in this repo. **Delegate aggressively or fail before starting** — quality collapses once context fills with raw reads and logs, long before a large window is full. Budgets are absolute token counts, never window percentages.

Hard limits:

- 120k hard reading budget (standard). At 120k stop reading; finalize, hand off, or delegate. A caller may explicitly grant an **extended budget** — 240k reading, 300k hard stop — for an oversized packet (e.g. swarm running in its extended band); spend the extra only on more dispatched evidence and a fuller ledger, never on bigger direct reads.
- Never read a file >600 lines in full — `rg` / symbol-search first, then range-read (default ±40 lines), or delegate. One read = one hypothesis; state it before reading.
- Never load generated code, lockfiles, `target/`, or vendored deps.
- Never paste full `cargo` / test output into context — delegate the run.
- Packet scope: the 5 packet contract files are the **only** files read in full. Code/session scopes: the diff is the primary direct read. Everything else is delegated or ranged.

Checkpoints (standard; when granted the extended budget: 200k / 240k / 300k): **100k** — re-confirm the plan fits (else delta mode / split); **140k** — stop dispatching new traces, start writing the report from collected evidence; **170k** — STOP; emit a handoff block (completed dimensions, outstanding traces, next concrete dispatches, files to reopen).

### Sub-agent dispatch contract

Every dispatch specifies: (1) one precise question with a binary or enumerable answer; (2) exact paths/crates/globs the sub-agent may read; (3) a return format — `FACT` (≤5 lines) / `LOCATIONS` (≤20 file:line with 1-line context) / `SNIPPETS` (≤3 verbatim snippets, ≤30 lines each, with file:line) / `SUMMARY` (≤200 words). Reject any reply pasting full build logs.

For verification commands the question is fixed: *"did `<command>` pass? If not, return the failing assertion and ≤20 lines of relevant code."* Return: FACT (pass, quoting the result line) or SNIPPETS (fail).

## Test discipline (all scopes)

- **Never run `cargo test --workspace` speculatively.** >1000 tests, ≥11 minutes. It runs at most once, only when a packet's acceptance ceremony requires it for closure, dispatched to a sub-agent under the FACT contract above — never absorbed into context.
- Prefer the narrowest command that proves the claim: `cargo test -p <crate> --test <file> -- <name>`. If no narrow test exercises the change, say so in the report — do not fall back to the full suite.
- Whole-suite / multi-crate runs go through `cargo xtask test --summary`; every test run tees to `target/test-output.log` — read the log, never re-run for more output.
- Gates use `--all-targets`: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace --all-targets`.

## Output

Per-scope report templates and verdict semantics: `references/output-format.md` — read it at the report-writing stage. Every scope ends in exactly one verdict; session scope's verdict is one line with no trailing prose.
