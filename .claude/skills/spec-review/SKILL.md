---
name: spec-review
description: Review a spec packet under `.ralph/specs/` against its packet docs and implementation; identify gaps, deviations, and unmet acceptance criteria. Use when reviewing, verifying, or auditing a packet, or before packet closure.
type: anthropic-skill
version: "1.3"
metadata:
  internal: true
---

# Spec Review

Review a spec packet under `.ralph/specs/<NN>_<slug>/` against its 5 packet docs and the implementation. **Scope target**: `$ARGUMENTS` (packet name or path).

## Context discipline (overrides everything below)

Reviews are the most context-hostile activity in this repo: 5 packet docs, every code file the design surface touches, every authoritative doc, every OrcaSlicer ref, and every verification command's output. You cannot read all of that and finish coherently. **Delegate aggressively or fail before starting.** Quality collapses past ~140k tokens of actual usage.

### Hard limits

- 60% (~120k) hard reading budget. At 60% stop reading; finalize, hand off, or delegate.
- Never read a file >600 lines in full. Symbol-search, range-read, or delegate.
- Never load generated code, lockfiles, `target/`, or vendored deps.
- Never paste full `cargo` / test output into context. Delegate the run.
- Never read an authoritative `docs/` file >300 lines to verify a single fact. Delegate a FACT.

### Delegate-first rule

Before opening any file ask: *can a sub-agent return just the answer?* If yes, delegate. You **must** delegate for:

- Tracing each requirement / AC to `file:line` evidence.
- Confirming whether a function, trait impl, manifest entry, or fixture exists.
- Running every verification command in `packet.spec.md` / `implementation-plan.md`.
- Reading any single file >300 lines for a specific constraint.
- Comparing OrcaSlicer reference behavior to the implementation.
- Confirming a field name, schema entry, or constant in `docs/`.
- Diffing an architectural constraint against actual code.

### Sub-agent dispatch contract

Every dispatch specifies:

1. **Question** — one precise question with a binary or enumerable answer.
2. **Scope** — exact paths, crates, or globs the sub-agent may read.
3. **Return format** — exactly one of:
   - `FACT`: ≤5 lines.
   - `LOCATIONS`: ≤20 `file:line` entries with 1-line context each.
   - `SNIPPETS`: ≤3 verbatim snippets, ≤30 lines each, with `file:line`.
   - `SUMMARY`: ≤200 words, no code unless asked.

For verification commands the question is fixed: *"did `<command>` pass? If not, return the failing assertion and ≤20 lines of relevant code."* Return is `FACT` (pass) or `SNIPPETS` (fail). Reject any reply pasting full build logs.

### Direct-read discipline

When you must read directly:

- `rg` / symbol search first; file-open second. Locate, then read.
- Use line ranges. Default window: ±40 lines around the symbol.
- One read = one hypothesis. State the hypothesis before reading; if a read does not test it, you are exploring — delegate.
- The 5 packet files (`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`) are the **only** files you load in full. Everything else is delegated or ranged.
- For Rust: prefer `cargo doc` and module trees over source. Read trait defs before impls; read `mod.rs` / `lib.rs` for shape before drilling. For `Cargo.toml`, only `[dependencies]`, `[features]`, `[workspace]` sections relevant to the task.

### Rust specifics

- Trust the type system. `cargo check` (via sub-agent) before chasing a bug through code.
- For trait/generic confusion: ask the sub-agent for the monomorphized error or the concrete impl. Do not read trait hierarchies yourself.
- Never paste full macro expansions. Delegate to summarize.
- Workspace nav: `cargo metadata --format-version=1 --no-deps` (via sub-agent, summarized) beats reading every `Cargo.toml`.
- Test failures: sub-agent returns failing test name, assertion, and ≤20 lines of relevant code — not the full test file.

### Checkpoints

- **60%**: state remaining budget; re-confirm the plan still fits. If not, switch to delta scope.
- **70%**: stop dispatching new traces. Begin writing the report from evidence collected.
- **85%**: STOP. Emit a handoff block: completed dimensions, outstanding traces, next concrete dispatches, files to reopen. Do not continue.

## Review modes

- **Full** — entire packet contract, code surface, verification set, task-map impact. Default. Only mode that may authorize closure.
- **Delta** — focused on `changed_steps` / `changed_files` and directly affected ACs, requirements, constraints, commands. For Swarm intermediate loops, or when full review will not fit budget. Never substitutes for final closure review.
- **Delegated** — when Swarm offloads review, use exactly **one** sequential review subagent after all code workers finish. Do **not** shard review by step or dimension.

A full review that does not fit budget must split across sessions, **not** silently approve.

## Workflow

### Step 0 — Emit a PLAN block first

```
PLAN
- Goal: review packet <slug> in <full | delta> mode
- Files in scope (read directly): the 5 packet files only
- Files explicitly out of scope: code under design.md surface (delegate per AC), authoritative docs (delegate per fact-check), OrcaSlicer refs (delegate), all cargo command output (delegate)
- Sub-agent dispatches planned: one per AC trace, one per requirement trace, one per verification command, one per architecture constraint, one per OrcaSlicer parity check
- Estimated context cost: <S / M / L>; if L → delta mode or split the packet
- Stop condition: review report emitted with verdict, all dispatched evidence collected
```

If the estimate is L, prefer **delta** scoped to `changed_steps` / `changed_files`, or split across sessions. Never run a full review when remaining budget cannot fit it.

### Step 1 — Preflight (always runs, including delta)

Packet-authoring defects are global; you cannot scope them away. For every AC in `packet.spec.md`:

- Ends with `|` followed by a concrete runnable command.
- Command uses correct paths, flags, module names; runnable as-is.
- Names exact assertion content, not generic phrases ("all required fields", "correct diagnostics").
- Command is delegation-friendly — small, parseable output on success (not >200-line logs).
- **AC test exercises the production code path it describes.** A test that ends in placeholder asserts (e.g. only checks pre-existing fixture data) or contains comments like `// TBD`, `// not yet implemented`, `// DOCUMENTED EXPECTATION` is **not** evidence the AC is met. Dispatch a `SNIPPETS` read of the test body and confirm its assertions reference the symbols / IR fields named in the Given/When/Then. If the test is a placeholder, log a **HIGH** finding (packet-authoring defect — same severity as a missing runnable command) and treat the AC as `PARTIAL/INCOMPLETE` regardless of test-pass status.

Packet-quality preflight:

- ≥1 negative / rejection criterion if the slice changes validation, enforcement, or failure behavior.
- `design.md` selects one approach when several are plausible.
- `design.md` declares files-in-scope, read-only context, and out-of-bounds files.
- Each `implementation-plan.md` step has explicit precondition, postcondition, exit condition, files-to-read, files-to-edit, expected sub-agent dispatches, context-cost estimate.
- No step is rated context cost L (otherwise the packet should have been split).
- Open questions that would change scope are resolved, or the packet is still `draft` with explicit blocker.

If any AC lacks a runnable command → **HIGH** finding (not MED), mark `PARTIAL/INCOMPLETE` in the report, and do not proceed until logged. If packet-quality fails, log a **HIGH** per missing element and clearly separate packet-authoring defects from implementation defects.

#### Doc Impact Statement gate (Required)

Every packet's `packet.spec.md` must contain a **Doc Impact Statement** section
(see `.claude/skills/spec-packet-generator/references/templates/packet.spec.md`). Verify:

1. The section exists and is non-empty.
2. Its content is **either** the literal string `none` with a one-line
   rationale, **or** a list of specific `docs/<NN>_*.md` sections plus one
   verification grep per section.
3. If `none`: confirm the rationale is genuinely scope-bound (test-only, pure
   refactor, doc-only change). Refactors that touch IR fields, WIT types,
   scheduler rules, claim IDs, manifest schema, host services, or module SDK
   contracts are **not** eligible — flag **HIGH** if `none` is claimed for
   any such packet.
4. If a section list: dispatch each verification grep as a `FACT pass/fail`.
   Every grep must return a hit before the packet may close. Missing greps
   = `CHANGES REQUESTED`; failing greps = `CHANGES REQUESTED` with the
   missing doc edits enumerated.

A missing or non-conformant Doc Impact Statement is a packet-authoring defect
of equal severity to a missing AC verification command — block closure until
fixed.

### Step 2 — Identify the packet

Given a name, list `.ralph/specs/` (small, OK to read directly). Given a path, validate it contains all 5 packet files.

### Step 3 — Read the packet, and ONLY the packet

Read in this order (these 5 are the **only** direct reads):

1. `packet.spec.md` — goal, scope, ACs, verification commands.
2. `requirements.md` — problem statement, acceptance summary.
3. `design.md` — architecture constraints, controlling code paths, selected approach.
4. `implementation-plan.md` — steps with task IDs, exit criteria.
5. `task-map.md` — bridge to backlog task IDs.

Do not "just peek" at code surface — that is how reviews silently bloat past 140k.

### Step 4 — Build a dispatch list, not a read list

From `design.md`: primary code paths, test/fixture paths, authoritative docs, out-of-bounds files. From `implementation-plan.md`: each step's objective, exit criteria, files expected to change, expected sub-agent dispatches.

Compose a **dispatch list**: one trace per AC, one per requirement, one run per verification command, one parity check per OrcaSlicer ref. This list — not the packet itself — is your working ledger.

### Step 5 — Confirm scope fits

If full mode: confirm the dispatch list fits remaining budget; otherwise downgrade to delta or split. If delta mode: identify `changed_steps` / `changed_files` from the caller, expand to direct dependencies and directly affected ACs, requirements, task-map, and design-fidelity checks. Keep delta scope focused on the affected slice, but **still check for cross-step regressions caused by that slice**. Delta is a focused checkpoint, **not** final signoff.

## Review dimensions

Every check is verified by **dispatching a sub-agent** for the underlying evidence. Compose precise dispatches; adjudicate the returned FACTs. Do not read code yourself to fill in a check.

### 1. Scope coverage (Critical)

- Implementation actually fulfills the stated goal in `packet.spec.md` *(dispatch: "does `<crate>::<fn>` implement `<behavior>`? FACT")*.
- No goal creep; no scope gaps.
- "In scope" items addressed; "out of scope" items genuinely untouched *(dispatch: "are there commits/edits in `<out-of-scope path>`? FACT")*.
- Boundary items have explicit justification.

### 2. Acceptance criteria fulfillment (Critical)

For each AC in `packet.spec.md`:

- Given/When/Then is met by implementation.
- Verification command passes (or explicit reason it does not yet) *(dispatch the command; FACT pass/fail)*.
- Test exists and asserts the criterion's promised content *(dispatch: "does test `<name>` exist and assert `<content>`? FACT")*.
- No partial fulfillment — "mostly done" = incomplete.
- Negative / rejection criteria are implemented and verified when the packet requires them.

### 3. Requirements traceability (Critical)

For each requirement in `requirements.md`:

- Trace to specific code via dispatch *(dispatch: "find function implementing `<requirement>`; LOCATIONS")*.
- No orphaned requirements (stated, not implemented).
- No unrequested implementations (done, not required).
- Acceptance summary bullets each have a verification.
- Measurable outcomes are actually measured by the evidence.

### 4. Design fidelity (High)

- Architecture constraints in `design.md` respected *(one dispatched FACT per constraint)*.
- Module stage assignments match documented stage IDs.
- No ad-hoc workarounds violating constraints.
- Locked assumptions / invariants preserved.
- Changes hit expected files; no surprises *(dispatch a `git diff --stat` summary)*.
- Test/fixture files updated.
- Implementation follows the **selected approach** from `design.md`, not an unreviewed alternative.
- IR field paths match exact names in `crates/slicer-ir/src/` *(one dispatched FACT per field)*.
- Type constraints, stage ordering, tiering respected.

### 5. Implementation completeness (Critical)

- Each step in `implementation-plan.md` executed in logical order; each achieved its objective.
- Verification commands documented and passing *(dispatch each)*.
- Each step satisfied explicit precondition, postcondition, exit condition.
- Read-only discovery steps produced the exact inventory / decision the packet promised.
- Each `task-map.md` task ID corresponds to completed work; no unmapped completions or gaps.
- Backlog source (e.g., `docs/07_implementation_status.md`) updated *(dispatched FACT)*.
- Reopened or superseded packet work reconciled explicitly.

### 6. Verification quality (High)

- All documented verification commands run successfully *(dispatch each as FACT pass/fail)*.
- Commands produce expected outputs; no hard-coded assumptions.
- Acceptance gate tests exist, pass, cover full ACs.
- No skipped tests for completed work.
- Tests integrated into CI.

### 7. Deviation documentation (Medium)

- All open questions in `design.md` answered or tracked.
- Answers documented (code/doc comments or deviation log).
- No `active`/`implemented` packet still depends on unanswered scope-changing questions.
- Identified risks mitigated or documented; tradeoffs have rationale.
- Deviations from spec are documented with explicit rationale; critical deviations have waivers when required.

### 8. Documentation quality (Medium)

- Referenced docs exist and are accurate *(one dispatched FACT per doc)*.
- No stale references to removed docs; cross-refs consistent.
- OrcaSlicer parity obligations met *(dispatch parity check; never read OrcaSlicer source yourself)*.
- Geometry / behavior comparisons accurate.

## Running verification

Every verification command in `packet.spec.md` and `implementation-plan.md` is dispatched. The dispatch contract is fixed:

```
Question: did `<command>` pass? If not, return the failing assertion and ≤20 lines of relevant code.
Scope: the workspace; the command itself implies its scope.
Return format: FACT (pass) or SNIPPETS (fail).
```

- **Full review**: dispatch all packet ACs and packet-level commands needed for closure.
- **Delta review**: dispatch only commands affected by `changed_steps` / `changed_files` / impacted ACs, unless a broader rerun is needed to disambiguate a regression.

**`cargo test --workspace` is run at most once, only when the packet's acceptance ceremony or completion gate requires it for closure.** The suite is >1000 tests and takes ≥11 minutes — do not dispatch it speculatively, and never use it as a substitute for the packet's targeted verification commands. If a delta review touches no closure gate, do not run it at all. Do not re-dispatch it across review iterations: one pass at the end is the contract.

Other dispatched (not direct) checks: `git status` summary on expected files (FACT); expected directory structure (FACT or LOCATIONS).

## Output

Emit the report using the exact template in `references/output-format.md` — read that file when you reach the report-writing stage. The template covers: header (packet path, status, review mode, delta scope, date, context cost & remaining budget), Summary, Spec Contract Review (Goal, ACs, Requirements), Design Fidelity Review (Architecture Constraints, Controlling Code Paths), Implementation Completeness (Step Execution, Task Map), Critical / High / Medium Issues, Positive Observations, Verification Results, Recommendations, Verdict.

Status enum: `draft | in-review | implemented | blocked`.
Verdict enum: **APPROVED** / **APPROVED WITH NOTES** / **CHANGES REQUESTED** / **BLOCKED** / **DEFERRED**.

### Delegated return contract (Swarm-invoked)

When run as a Swarm review subagent, return compact and structured:

- review mode and delta scope;
- blocking findings by severity;
- impacted steps, files, ACs;
- verification commands run with pass/fail;
- a single verdict with one short rationale paragraph.

Do not repeat large packet excerpts or full build logs unless the caller asks.

## Verdict semantics

- **APPROVED** — implementation complete and correct; only authorizes closure in full review mode.
- **APPROVED WITH NOTES** — complete; non-blocking improvements noted.
- **CHANGES REQUESTED** — specific changes needed before re-review.
- **BLOCKED** — critical issues requiring significant rework or design decisions.
- **DEFERRED** — review could not complete within context budget; emit partial findings with explicit list of remaining dispatches and recommended next-session entry point.

## Rules (invariants)

- **Be rigorous on Critical dimensions** — scope coverage and AC fulfillment are non-negotiable.
- **Dispatch, don't read** — every code/cargo/doc check goes to a sub-agent. The 5 packet files are the only direct reads.
- **Trace every requirement** — if you cannot trace it via returned LOCATIONS, it is not done.
- **Run verification commands via dispatch** — never absorb their full log.
- **Do not run `cargo test --workspace` speculatively** — it is >1000 tests / ≥11 minutes; reserve it for the single acceptance-ceremony gate when the packet requires it.
- **Document all deviations** — undocumented deviations are issues.
- **Acknowledge good work** — positive observations matter.
- **Provide specific fixes** — vague "improve this" is not actionable.
- **Do not shard review** — if delegated, one holistic reviewer; no fan-out.
- **Delta review is not final signoff** — final closure always requires a full review *and only when budget supports it*.
- **Keep delegated review compact** — findings, traceability, verification status; no full-packet repetition.
- **Stop at 60%, hand off at 85%** — emit a partial review with a clear handoff list rather than continuing into degraded performance.
