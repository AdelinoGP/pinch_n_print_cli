---
name: spec-review
description: Adversarial, evidence-based review of a spec packet, this session's work, or a code diff against this repo's architecture contract. Three scopes — packet (`--preflight` authoring gate or full/delta closure review), session (SHIP / DO NOT SHIP audit), code (pre-commit review of changes since a ref). Use when reviewing, auditing, or verifying a packet before closure; "what did I miss", "are we ready to ship", "review my changes", or "review since X".
type: anthropic-skill
version: "3.2"
metadata:
  internal: true
---

# Review

One skill, three scopes. Core rules below apply to **every** scope; per-scope workflow lives in `references/`.

**Scope target**: `$ARGUMENTS` — packet name/path → packet; file paths / `all-changes` / git ref → code; session-audit request → session.

## Scope selection

| Scope | Reviewing what vs. what | Verdict | Workflow file |
|---|---|---|---|
| **packet** | `docs/spec_packets/<prefix>_<NN>_<packet-name>/` impl vs. its 5 packet docs | APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED / BLOCKED / DEFERRED | `references/packet-review.md` |
| **session** | This session's git diff + working tree vs. the packet being implemented | SHIP / DO NOT SHIP | `references/session-audit.md` |
| **code** | Uncommitted changes, named files, or diff since a fixed point — vs. architecture contract (no packet) | APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED | `references/code-review.md` |

Pick by target, not phrasing. **Packet scope (full mode) is the only path that may authorize closure.** If genuinely ambiguous, ask. Read the scope's workflow file before starting. `--preflight <packet>` is packet scope — read `references/preflight-gate.md`.

## Mindset (all scopes)

You are a senior engineer reviewing cold — you did not write it. In session scope, you literally did, which is more reason to distrust memory. **Session memory is not evidence.** `git status` / `git diff` / `rg` / file reads / dispatched FACTs are.

- Bias toward finding problems. "Looked fine while it was built" is not evidence.
- **Burden of proof is on the implementation.** A claim with no evidence is `[unverified]`; an AC with no passing dispatched evidence is FAIL — never PASS-by-default.
- A sub-agent "pass" that doesn't quote its evidence (file:line, assertion, or result line) is not evidence. Reject and re-dispatch.
- **Re-verify on disagreement.** A re-check wins over an earlier claim — say so openly. Ground every load-bearing PASS/FAIL in a real-tree grep; re-run "feels wrong" results with absolute repo-root paths before concluding. One false sub-claim doesn't condemn a sweep — re-verify each load-bearing claim independently.
- **Never fabricate `file:line`.** If you can't open the file and confirm the symbol, the row is `[unverified]`.
- **Findings, not fixes.** Review is read-only — don't edit, refactor, or "fix while you're here"; obvious bugs are findings. Sole exception: session scope may append to the packet's `Deviations` section after explicit user confirmation (`references/session-audit.md`).
- Specific fixes, not "improve this". Acknowledge good work.

## Verdict floors (all scopes)

Leniency is this skill's historical failure mode; these floors are hard.

- Any `[unverified]` load-bearing row (AC, requirement, or session change) → verdict capped at CHANGES REQUESTED / DO NOT SHIP. No APPROVED, no SHIP.
- Any Critical finding → CHANGES REQUESTED, BLOCKED, or DO NOT SHIP.
- Delta review never authorizes closure. A full review that doesn't fit budget → DEFERRED with a handoff block. **Budget pressure defers closure; it never waives review.**
- Preflight-gate failure (S4/S5/S6) → packet cannot be APPROVED.

## Known traps (all scopes; each caught a real shipped defect)

1. **Helper passes, driver never calls it** (P95 W6/W8): a helper's unit tests stay green while the production driver never invokes it. For any helper introduced, verify a production call site: `rg '<helper>\s*\(' --type rust --glob '!**/tests/**' --glob '!**/*_test*.rs'`. Zero non-test hits = dead code = HIGH finding; AC/change is PARTIAL regardless of test-pass status.
2. **Placeholder tests**: asserts only check pre-existing fixture data, or contain `// TBD` / `// not yet implemented` / `// DOCUMENTED EXPECTATION`. SNIPPETS-dispatch the test body; confirm assertions reference the symbols/IR fields the claim names. HIGH finding; AC → PARTIAL.
3. **Unregistered test file — "0 tests run" false pass**: a new file under an aggregated test binary (`tests/contract/`, `tests/unit/`, …) with no `mod` registration silently never compiles; `cargo test --test <bin> <filter>` reports 0 tests and looks green. Confirm aggregator registration.
4. **Stale guest WASM**: before attributing any guest/component/module-dispatch test failure to the changes or "flakiness", run `cargo xtask build-guests --check` and judge by its exit code (exit 0 = fresh; non-zero = stale or infrastructure error). Never grep for `STALE:` — a `wasm-tools`-missing error prints no `STALE:` and would read as fresh. If stale, rebuild and re-run. A stale guest is the reviewer's bug until `--check` proves otherwise (CLAUDE.md).
5. **Fictional symbols** (2026-06 wave: ≥1 defect in every packet 104–112): claims naming functions/fields/enum variants/WIT types/ADR slots that don't exist or have a different shape. Resolving names against the real tree is the only defense — prose review cannot see this. Packet scope runs the full S0–S8 gate (`references/preflight-gate.md`); other scopes apply the same principle to any pre-existing symbol a claim leans on.

## Context discipline (all scopes)

Reviews are the most context-hostile activity in this repo. **Delegate aggressively or fail before starting** — quality collapses once context fills with raw reads and logs. Aggressive means **batched**: several independent facts, symbols, or commands go to one sub-agent (see Sub-agent dispatch contract below); trickling them out one dispatch at a time pays the sub-agent's startup cost per item and wastes the budget this discipline exists to protect. Budgets are absolute token counts, never window percentages.

Hard limits:
- **120k hard reading budget** (standard). At 120k stop reading; finalize, hand off, or delegate. A caller may grant an **extended budget** — 240k reading, 300k hard stop — for an oversized packet; spend the extra only on more dispatched evidence and a fuller ledger, never on bigger direct reads.
- Never read a file >600 lines in full — `rg` / symbol-search first, then range-read (default ±40 lines), or delegate. One read = one hypothesis; state it before reading.
- Never load generated code, lockfiles, `target/`, or vendored deps.
- Never paste full `cargo` / test output into context — delegate the run.
- Packet scope: the 5 packet contract files are the **only** files read in full. Code/session scopes: the diff is the primary direct read. Everything else is delegated or ranged.

Checkpoints (standard; extended: 200k / 240k / 300k): **100k** — re-confirm the plan fits (else delta mode / split); **140k** — stop dispatching new traces, start writing the report; **170k** — STOP; emit a handoff block (completed dimensions, outstanding traces, next concrete dispatches, files to reopen).

### Sub-agent dispatch contract

Every dispatch specifies: (1) one precise question with a binary or enumerable answer; (2) exact paths/crates/globs the sub-agent may read; (3) a return format — `FACT` (≤5 lines) / `LOCATIONS` (≤20 file:line with 1-line context) / `SNIPPETS` (≤3 verbatim snippets, ≤30 lines each, with file:line) / `SUMMARY` (≤200 words). Reject any reply pasting full build logs.

**Batch independent work into one dispatch.** One numbered batch may carry several facts, symbols, greps, verification commands, or files, provided they share a reader (same crate, directory, or doc) and share one return format. Each sub-agent pays a fixed startup cost (context assembly, navigation, tool warmup), so N items in one dispatch cost far less than N dispatches. Split only when the items need different return formats, different readers, or one sub-agent's context would fill before the batch completes.

- **One dispatch per review dimension, not per item.** Symbol existence/shape checks (S5), WIT/IR identifiers (S6), design-fidelity constraints, and doc-existence greps each batch into one dispatch per dimension even when they cover dozens of symbols.
- **Verification commands batch too.** Hand one sub-agent the packet's full command list (all AC commands, or all affected commands in delta mode) and ask for a per-command FACT line. A command that fails gets a focused SNIPPETS follow-up — never a re-dispatch of the whole list. A batch that returns missing items is a **partial return, not a pass**: re-dispatch the omitted tail only (never the passing prefix), or mark each omitted item `[unverified]` — which caps the verdict.
- **Name the capability the batch needs** — read-only search vs. shell access (`git`, `cargo`, `rg`). A shell-needing batch routed to a read-only worker comes back "no shell" and burns the dispatch: route it to a command-capable worker, or close the items with bounded direct runs.
- **Per-item dispatches are the exception, and only for:** a single expensive command whose output is unbounded (e.g. `cargo test --workspace`), a re-verification triggered by a disagreement, or an item whose failure would change what the rest of the batch asks. State the reason when you split.

For verification-command batches the question is fixed: *"run every command below — all of them; do not stop after the first few. Return one FACT line per command — `<command>`: PASS (`<result line>`) or FAIL, and for each FAIL the failing assertion plus ≤20 lines of relevant code. If the environment blocks a command (missing tool, no shell), name the command and the blocker instead of omitting it."* Keep the batch under the point where its return outgrows the limit; split a list of 30 commands into two or three, not thirty.

Never batch into a dispatch: items requiring contradictory instructions, an independent re-verification of another sub-agent's claim (SKILL.md re-verify-on-disagreement needs a fresh reader), or anything whose failure mode would invalidate the batch's other answers.

**One wave, one report.** Launch the whole planned dispatch wave before adjudicating anything (parallel where the work is independent). Emit the review report exactly once — after every dispatch has returned or failed. A mid-wave message may note which batches have landed and which are still in flight; it is never a partial report or verdict, and it never re-dispatches work already running. When completions resume you, collect them and continue to the final report. If a dispatch returns empty or partial, resolve it (`[unverified]` rows, tail-only re-dispatch, or bounded direct runs) and keep going. The only ways to end a review with work outstanding are the checkpoint handoff (SKILL.md Context discipline) and a genuinely blocked dispatch you cannot resolve — and both state what is outstanding and emit the partial report in the same message.

## Test discipline (all scopes)

- **Never run `cargo test --workspace` speculatively.** >1000 tests, ≥11 minutes. Runs at most once, only when a packet's acceptance ceremony requires it for closure, dispatched to a sub-agent under the FACT contract — never absorbed into context.
- Prefer the narrowest command that proves the claim: `cargo test -p <crate> --test <file> -- <name>`. If no narrow test exercises the change, say so — don't fall back to the full suite.
- Whole-suite / multi-crate runs go through `cargo xtask test --summary`; every test run tees to `target/test-output.log` — read the log, never re-run for more output.
- Gates use `--all-targets`: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace --all-targets`.

## Output

Per-scope report templates and verdict semantics: `references/output-format.md` — read it at the report-writing stage. Every scope ends in exactly one verdict; session scope's verdict is one line with no trailing prose.