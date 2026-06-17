---
name: spec-packet-generator
description: Generates a Pinch 'n Print Ralph spec packet under .ralph/specs/ (packet.spec.md, requirements.md, design.md, implementation-plan.md, task-map.md) from a rough prompt, file, or URL.
type: anthropic-skill
version: "1.2"
metadata:
  internal: true
---

# Spec Packet Generator

## Context Discipline (primacy — overrides any later instruction)

Treat context as a budget, not a buffer. Quality collapses past ~140k of 200k. This block is the rule, not an aspiration: packet generation is read-heavy (backlog file, authoritative docs, OrcaSlicer refs, sometimes a predecessor packet), and you are the only agent that can keep this skill from quietly burning the implementer's budget.

**Hard limits**
- Read budget: 60% (≈120k). At 60% stop reading and finalize, hand off, or delegate.
- NEVER read a file > 600 lines in full. Use symbol search, line ranges, or delegate.
- NEVER load generated code, lockfiles, `target/`, vendored deps, or full `cargo`/test output. Delegate or skip.

**Delegate before reading.** Before opening any file ask: "Can a sub-agent return just the answer?" If yes, delegate. You MUST delegate:
- `docs/07_implementation_status.md` survey
- `docs/00_project_overview.md` map (when > 300 lines)
- Any `OrcaSlicerDocumented/` inspection
- Any prompt file or URL > 300 lines
- Trait/macro/generic tracing across crates
- `cargo check` / `test` / `clippy` runs
- Any exploratory read where you do not yet know what you are looking for

**Sub-agent contract.** Each dispatch specifies:
1. Question — one precise question, binary or enumerable answer
2. Scope — exact paths/crates/globs
3. Return format — exactly one of:
   - `FACT: <≤ 5 lines>`
   - `LOCATIONS: <file:line list, ≤ 20 entries, 1-line context each>`
   - `SNIPPETS: <≤ 3 verbatim snippets, ≤ 30 lines each, with file:line>`
   - `SUMMARY: <≤ 200 words, no code unless asked>`

Reject any reply that exceeds the contracted format. Re-dispatch with tighter scope; do not paste oversize replies into context.

**Reading discipline (when reading directly).** Locate first (`rg`/symbol search), open second. Default window: ±40 lines. One read = one hypothesis; if a read does not test a stated hypothesis, you are exploring — delegate instead.

**Rust-specific.** Packets target this Rust workspace, so populating `design.md` and `implementation-plan.md` often requires peeking at code:
- Trust the type system. Run `cargo check` (via sub-agent) before reading more code to chase a bug.
- For trait/generic confusion: ask a sub-agent for the monomorphized error or the concrete impl; do not read trait hierarchies yourself.
- Never paste full macro expansions into your context. Delegate a summary.
- Workspace navigation: `cargo metadata --format-version=1 --no-deps` (via sub-agent, summarized) beats reading every `Cargo.toml`.
- Test failures: sub-agent returns failing test name, assertion, and ≤ 20 lines of relevant code — not the full test file.

**PLAN block (emit before starting any task).**
```
PLAN
- Goal: <one sentence>
- Files in scope (read+edit): <list, ≤ 3>
- Files explicitly out of scope: <list>
- Sub-agent dispatches planned: <{question, scope, return-format}, or "none">
- Estimated context cost: S / M / L  (if L → STOP and decompose; execute only the first slice and surface the rest as a numbered handoff)
- Stop condition: <binary "done" check>
```

**Checkpoints**
- 60%: state remaining budget; re-confirm plan fits.
- 70%: stop reading. Finalize, hand off, or compress.
- 85%: STOP. Output a handoff block — completed steps, current state, next concrete action, files to reopen.

## When to Use

- The user has a rough prompt and wants a packet.
- A `docs/07_implementation_status.md` slice needs runnable spec artifacts.
- A task group needs scope, authoritative docs, OrcaSlicer refs, and acceptance criteria before implementation.

## Rules

- Packets live under `./.ralph/specs/[spec-slug]/` and contain `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, and `task-map.md` when explicit `docs/07` mapping is needed.
- Each packet is one coherent remediation slice — no spanning unrelated workstreams.
- `packet.spec.md` is preflight-visible; it MUST contain real Given/When/Then acceptance criteria.
- Default `status: draft`. Mark `active` only if the user explicitly asks AND no other packet is active.
- A packet must be implementation-grade on first emission: exact assertions, negative cases, step exit criteria, decisive code surfaces. No placeholder prose.
- A packet must be context-budget-aware: `design.md` and `implementation-plan.md` declare files-in-scope per step, expected sub-agent dispatches, and per-step context cost (S/M/L). The implementer reads these verbatim.
- Unresolved ambiguity blocks activation. Open question → keep `draft` and record the blocker.
- Use `docs/00_project_overview.md` as the normative document map.
- Cite specific paths under `OrcaSlicerDocumented/` when parity matters; never read OrcaSlicer source directly.
- This skill ends after the packet is generated. Do not begin implementation.
- Use `AskUserQuestion` for ANY unresolved input — missing parameters, ambiguous mapping, scope, status, overwrite confirmation, design questions, activation. Batch related questions into one call. Do not pass a gate (scope approval, file generation, activation) without an explicit answer.

## Cross-Packet Mutation Rule

A packet MUST NOT modify files in another packet's directory. If correcting/completing prior work, mark the prior packet `status: superseded` in its own `packet.spec.md` and note the absorption in this packet's `requirements.md` Problem Statement. Inspect predecessors via SUMMARY dispatch — do not read all 5 of their files yourself.

## Parameters

- **input** (required) — rough text, markdown file path, or URL.
- **task_ids** (optional) — `TASK-###` ids from `docs/07`. If omitted, infer and confirm.
- **spec_slug** (optional) — kebab-case folder name; derive from prompt + scope when omitted.
- **output_dir** (optional, default `./.ralph/specs/[spec_slug]/`).
- **status** (optional, default `draft`).

Constraints: ask for missing required params via `AskUserQuestion`; support text/file/URL input (delegate summarization for > 300 lines); never overwrite an existing packet directory without explicit approval; present scope and get explicit approval before generating files; keep packets small (a handful of related `docs/07` tasks, not a phase).

## Workflow

### 0. PLAN

Emit the PLAN block. Default for this skill:
- Files in scope: the 5 packet files under `./.ralph/specs/<slug>/`.
- Out of scope: the workspace; `target/`; lockfiles; `OrcaSlicerDocumented/` (delegated).
- Dispatches: at minimum one for `docs/07`, one for `docs/00` if needed, one for OrcaSlicer if parity matters, optionally one per predecessor packet.
- Cost: M is typical. L → split (most often, predecessor reconciliation becomes its own preliminary packet).
- Stop condition: 5 packet files written, self-review checklist green, scope approved.

### 1. Detect Input Mode

- File path: read; if > 300 lines, delegate a SUMMARY.
- URL: delegate fetch + summarization. Do not paste page body into context.
- Otherwise: treat as direct prompt text.

Extract: core remediation slice, likely subsystems, likely authoritative docs, stated verification requirements. Keep working notes ≤ 10 bullets — longer = over-read.

### 2. Resolve Backlog Scope

Delegate the `docs/07` survey:

```
Question: which TASK-### ids in docs/07_implementation_status.md form the smallest contiguous slice that satisfies "<prompt summary>"?
Scope: ./docs/07_implementation_status.md
Return: LOCATIONS (one entry per candidate, with status column and one-line topic)
```

Confirm each `TASK-###` exists (FACT follow-up if needed). Prefer one contiguous/tight slice. If too broad, narrow and explain. If ambiguous, present 1–3 options via `AskUserQuestion`.

### 3. Gather Authoritative References

Use `docs/00_project_overview.md` as the normative map (delegate SUMMARY if > 300 lines). Load only the named docs, only the relevant sections, with line ranges.

Likely candidates: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`, `docs/08_coordinate_system.md`, `docs/09_progress_events.md`, `docs/11_operational_governance_and_acceptance_gate.md`, `docs/12_architecture_gate_metrics.md`.

For OrcaSlicer parity, delegate:

```
Question: which OrcaSlicer files implement <behavior>? Return file paths and a one-line role each. No code snippets.
Scope: ./OrcaSlicerDocumented/
Return: LOCATIONS
```

Record returned paths verbatim in `requirements.md` and `packet.spec.md`.

### 4. Resolve Packet Metadata

Determine: slug, task ids, goal, in-scope, out-of-scope, output dir, status.

**Gate.** Present a short plan — slug, grouped task ids, one-paragraph goal, in/out-of-scope, files to generate, expected downstream context cost (S/M/L based on crates touched). MUST NOT write files until the user approves.

### 5. Acceptance Criteria Completeness Checklist

Before writing any packet file, verify each AC meets the rules below. If any item fails, resolve before proceeding.

**Per criterion**
- [ ] Falsifiable by a single command or test.
- [ ] Names exact field names (IR paths, config keys, manifest entries) — not generic categories.
- [ ] States observable assertion content (exact fields, counts, paths, error codes, enum variants, output fragments) — not phrases like "all required fields" or "correct diagnostics".
- [ ] Ends with `|` followed by a runnable verification command.
- [ ] Command is delegation-friendly: small parseable output (exit code, single assertion, JSON path). Wrap/filter commands that would dump > 200 lines on success so a sub-agent can return a FACT.
- [ ] If multiple criteria share a verification, each carries its own pipe-suffixed command (repeat — do not write "see AC-N").

**Per packet**
- [ ] At least one negative or rejection case when the packet changes a validator, scheduler rule, contract boundary, or failure path.
- [ ] Criteria together cover the main success path AND the failure mode most likely to regress silently.
- [ ] Each criterion traces to one or more implementation steps without implied work.

**For IR/config/schema criteria**
- [ ] IR field paths spelled exactly as in `docs/02_ir_schemas.md`.
- [ ] Config field keys spelled exactly as in the module's `.toml` manifest.
- [ ] If a count is asserted (e.g., "all six fields"), list the field names inline.

**Workspace test discipline**
- [ ] No AC's pipe-suffixed verification command is `cargo test --workspace`. The full suite is >1000 tests / ≥11 minutes — ACs must verify with a targeted `cargo test -p <crate> --test <file>` (optionally `-- <test_name>`) so the implementer can falsify them in seconds, not minutes.
- [ ] At most one packet-level entry under `Verification` in `packet.spec.md` lists `cargo test --workspace`, and only when the packet explicitly requires a workspace-wide gate at closure. Otherwise omit it entirely. Use targeted per-crate commands plus `cargo check --workspace` and `cargo clippy --workspace -- -D warnings` instead.

See `references/acceptance-criteria-examples.md` for a compliant and a non-compliant AC.

### 6. Create Packet Structure

Create `./.ralph/specs/[spec_slug]/` and generate the packet files. Use the templates at `references/templates/` (inside this skill) as starting structure but replace placeholders with packet-specific content. Templates already include context-discipline fields (files-in-scope, sub-agent dispatches, context cost) — fill in concretely; no placeholders, no "TBD", no "see above".

**File-purpose hierarchy (each concept has one owner; everything else references back).** Enforce this when filling in templates:

| Concept | Owner | Other files |
|---|---|---|
| Goal (one sentence, solution-shaped) | `packet.spec.md` | — |
| Problem motivation (why; motivation-shaped) | `requirements.md` | — |
| Scope (full in/out lists) | `requirements.md` | `packet.spec.md` carries a 2–3 sentence prose summary only |
| Acceptance Criteria (Given/When/Then + verification command) | `packet.spec.md` | `requirements.md` references by ID, never copies |
| Verification commands (full matrix with delegation hints) | `requirements.md` | `packet.spec.md` lists only 2–3 gate commands |
| Code-change surface | `design.md` | — |
| Files-in-scope / out-of-bounds | `design.md` (packet-level) + `implementation-plan.md` (per-step) | — |
| Step sequence with cost contract | `implementation-plan.md` | — |
| `docs/07` crosswalk | `task-map.md` (conditional) | — |

**Snippet library (`references/snippets/`).** Workspace-invariant boilerplate is centralized as snippets that are included **verbatim** when applicable. Generated packets are still self-contained — there is no runtime macro engine; the skill copies snippet text into the packet during generation and marks it with an inline `<!-- snippet: <name> -->` HTML comment so self-review can grep for it.

- `context-discipline.md` — closing block of `packet.spec.md`. **Mandatory for every packet.**
- `orca-delegation.md` — opening paragraph of `OrcaSlicer Reference Obligations` in `packet.spec.md` AND `requirements.md`. **Skip both sections entirely** if the packet involves no OrcaSlicer parity. Never include a third copy in `design.md`.
- `wasm-staleness.md` — bullet in `design.md` §Architecture Constraints. **Skip** if no path in the change surface feeds guest WASM (host-only refactors, doc edits, etc.).
- `coord-system.md` — bullet in `design.md` §Architecture Constraints. **Skip** for packets that touch only G-code text, config parsing, scheduler wiring, or other non-geometric concerns.

Decide per snippet whether the packet needs it. If included, copy verbatim. **Paraphrasing snippet content is forbidden** — paraphrases drift and rot; verbatim or absent.

### 7. `packet.spec.md`

YAML frontmatter (`status`, `packet`, `task_ids`, `backlog_source: docs/07_implementation_status.md`); one-sentence solution-shaped Goal; **prose** Scope Boundaries (2–3 sentences, NOT a bullet list duplicating `requirements.md` §In Scope); Given/When/Then ACs as the single authoritative source (each ending with `|` + runnable command, each labelled `AC-1`, `AC-2`, …); at least one negative/rejection criterion (`AC-N1`, `AC-N2`, …) when the packet touches validation, enforcement, or error handling; prerequisites and blockers when sequencing matters or a prior packet is being corrected; Verification section listing only the **2–3 gate commands** the closure check runs (full matrix lives in `requirements.md`); authoritative docs; Doc Impact Statement (Required); OrcaSlicer Reference Obligations (include `orca-delegation` snippet if applicable, omit section otherwise); Context Discipline Note (include `context-discipline` snippet — mandatory).

**Do not** include a static "Packet Files" section — the 5-file structure is documented once in `.ralph/specs/README.md`.

### 8. `requirements.md`

Problem Statement (motivation-shaped — why the gap matters); grouped task ids; **full** In Scope / Out of Scope bullet lists; authoritative docs with size/delegation notes; OrcaSlicer Reference Obligations (include `orca-delegation` snippet if applicable); Acceptance Summary that **references ACs by ID** and adds measurable refinements that didn't fit Given/When/Then (never copies the criterion text); Verification Commands as a **table with delegation hints** (the full matrix — `packet.spec.md` carries only the gate subset); Step Completion Expectations that document only cross-step invariants the per-step blocks in `implementation-plan.md` cannot express (write `None.` if not applicable, never restate per-step preconditions); packet-specific Context Discipline Notes only (do not restate the workspace-wide discipline — that lives in the `context-discipline` snippet in `packet.spec.md`).

### 9. `design.md`

Implementation shape (no implementation):
- Controlling code paths / likely surfaces (point at `requirements.md` for OrcaSlicer parity surface; do not restate the delegation rules).
- Neighboring tests/fixtures.
- Architecture Constraints — packet-specific bullets, plus `wasm-staleness` snippet bullet **if** the change surface feeds guest WASM, plus `coord-system` snippet bullet **if** the packet touches geometry/mm-unit conversion. Decide per snippet; skip the bullet entirely if not applicable.
- One selected approach (rejected alternatives noted briefly — must choose one).
- Explicit Code Change Surface: exact functions, traits, manifests, tests, fixtures expected to move (target ≤ 3 primary files).
- Read-Only Context the implementer needs (with line-range hints when files > 300 lines).
- Out-of-Bounds Files (large generated, OrcaSlicer source, vendored, unrelated crates).
- Expected Sub-Agent Dispatches with explicit return-format hints per dispatch.
- Data and Contract Notes.
- Locked Assumptions and Invariants — never omitted silently. If the change is reversible via config defaults with no behavior locks, write `None — change is reversible via existing config defaults; no behavior locks introduced.`
- Risks and Tradeoffs.
- Context Cost Estimate (aggregate, largest single step, highest-risk dispatch).
- Open Questions — never omitted silently. Tag with `[FWD]` (forward-looking; implementer can resolve mid-flight) or `[BLOCK]` (activation-blocking). Write `None.` if none.

### 10. `implementation-plan.md`

Atomic ordered steps. Each step:
- Title; linked task ids; objective; precondition; postcondition.
- Files allowed to read (with line-range hints when relevant).
- Files allowed to edit (≤ 3).
- Expected sub-agent dispatches (e.g., "delegate `cargo test --package slicer-ir mesh::bbox` and return FACT pass/fail + failing assertion").
- Context cost: S / M / L. **No step may be L; if it would, split.**
- Authoritative docs; OrcaSlicer refs.
- Narrow verification commands.
- Cheapest falsifying check or explicit exit condition.

Steps stay inside the packet boundary, reflect TDD/narrow validation, and are actionable without guesswork. Read-only discovery steps must state expected output. Include a packet completion gate at the end.

### 11. Self-Review (mandatory before reporting)

**Implementation-grade checks**

- [ ] Every AC is implementation-grade and names exact assertion content.
- [ ] At least one negative case when the slice changes validation, enforcement, or contract behavior.
- [ ] `requirements.md` states measurable outcomes, not topical summaries.
- [ ] `design.md` selects one approach, lists exact code surfaces, lists out-of-bounds files.
- [ ] Each step has precondition, postcondition, falsifying check / exit condition, files-to-read, files-to-edit, expected dispatches, and context cost.
- [ ] No step has cost L.
- [ ] Verification commands are delegation-friendly.
- [ ] Reopened/superseding packet explains what the prior packet missed and how this one narrows the gap.
- [ ] Open questions are answered, OR the packet stays `draft` with the blocker called out.

**Repetition checks (target redundancy, not length)**

- [ ] **Snippet integrity.** Every passage that conveys content owned by a `references/snippets/*.md` snippet is either (a) the verbatim snippet text marked with `<!-- snippet: <name> -->`, or (b) absent because the snippet does not apply to this packet. Grep the packet for each snippet's distinctive opening phrase to confirm — no paraphrased near-duplicates allowed.
- [ ] **Cross-file overlap.** No section in `packet.spec.md` restates content from `requirements.md` or `design.md` beyond a single-sentence summary + a back-reference. Acceptance Criteria are stated once in `packet.spec.md`; `requirements.md` Acceptance Summary references them by ID, never copies them. `packet.spec.md` §Verification lists only the 2–3 gate commands; the full matrix lives only in `requirements.md`.
- [ ] **In-file repetition.** No two bullets in the same list say the same thing in different wording. No two ACs differ only in their pipe-suffixed verification command.
- [ ] **Boilerplate-vs-content ratio.** If a section is dominated by general-knowledge prose ("config keys are snake_case", "treat trait bounds carefully"), strip the prose. The rule lives in `CLAUDE.md`, the snippets, or `docs/`; the packet only carries the packet-specific facts.

**Length as a *signal*, never a gate (anti-gaming guard)**

Length-target gaming has been observed in prior packets: when instructed to "keep `design.md` ≤ N lines", the skill compresses legitimately complex content (ACs collapsed, edge cases dropped, sub-agent dispatches truncated) to hit the number. **This is forbidden.** The rule:

> When a file exceeds the rough reference size below, **do not compress**. Run the four repetition checks above and ask: is this length driven by duplication (fix it) or by genuine packet complexity (leave it)?

Reference sizes (re-examination triggers, not limits):
- `packet.spec.md` ~100 lines
- `requirements.md` ~150 lines
- `design.md` ~250 lines
- `implementation-plan.md` ~300 lines

Legitimate length reductions are only:
1. Removing duplication that the repetition checks flagged.
2. Cutting general-knowledge prose covered by snippets / `CLAUDE.md` / `docs/`.
3. Replacing copy-pasted boilerplate with the canonical snippet (marked with `<!-- snippet: <name> -->`).

**Forbidden** length reductions:
- Trimming ACs, negative cases, or step exit conditions.
- Truncating files-in-scope or out-of-bounds lists.
- Shortening sub-agent dispatch contracts (each must keep question + scope + return-format).
- Eliding locked invariants or open questions to hit a number.

**Surface the reasoning when triggered.** When a file exceeds its reference size, the self-review output names which complexity drove it. Examples:
- ✅ `design.md` 340 lines: 12 distinct code surfaces, 8 sub-agent dispatches with return-format contracts, 6 locked invariants. None reducible without losing implementation-grade detail.
- ❌ `design.md` 340 lines: large. (← unacceptable — hand-waving)

The user reviews this reasoning when signing off on `status: active`. If the reasoning is hand-waving, the packet stays `draft`.

If any item fails, revise before presenting as complete. If you cannot resolve from available sources, stop and tell the user exactly what is ambiguous.

### 12. `task-map.md`

Add when it clarifies how packet steps map back to `docs/07`. Especially when: the packet spans > 1 task id; multiple docs are authoritative for different steps; OrcaSlicer refs differ by step; the packet reopens or supersedes prior work.

### 13. Report

List generated files with paths. Summarize: slug, status, task ids covered, authoritative docs chosen, OrcaSlicer refs chosen, open questions or assumptions, whether self-review passed cleanly or remained `draft` with blockers, aggregate context cost (sum of step S/M/L) so the user can decide on scheduling against a fresh agent.

### 14. Activation

If `draft`, ask whether to mark `active`. If yes:
- Confirm no other active packet.
- Confirm no unresolved questions, missing negative cases, or missing exit criteria.
- Update `packet.spec.md` to `status: active`.
- Remind user: next steps are `ralph preflight` then `ralph run -c ralph.yml`.

## Output Contract

The packet must be sufficient for a Ralph run to know:
- Exact backlog slice in scope.
- Which docs govern the behavior.
- Which OrcaSlicer references to check.
- What acceptance looks like.
- Implementation step order.
- Files allowed to read, allowed to edit, forbidden to load.
- Sub-agent dispatches the implementer should plan for.
- Per-step context cost.

## References

Load on demand:

**Templates** (`references/templates/`) — starting structure for every packet file. Replace placeholders with packet-specific content; do not emit a packet file that still contains `[spec-slug]`, `TASK-000`, or "TBD".
- `packet.spec.md` — preflight-visible contract. YAML frontmatter, Goal, prose Scope Boundaries, ACs (single source of truth), Negative Test Cases, gate-only Verification, Authoritative Docs, Doc Impact Statement, OrcaSlicer Reference Obligations (snippet), Context Discipline Note (snippet).
- `requirements.md` — Problem Statement, full Scope lists, Authoritative Docs, OrcaSlicer obligations (snippet), Acceptance Summary (refs ACs by ID), full Verification Commands matrix, cross-step Step Completion Expectations, packet-specific Context Discipline Notes.
- `design.md` — Controlling Code Paths, Architecture Constraints (snippet bullets when applicable), Code Change Surface, Files in Scope, Read-Only Context, Out-of-Bounds Files, Expected Sub-Agent Dispatches, Data and Contract Notes, Locked Assumptions, Risks, Context Cost Estimate, Open Questions.
- `implementation-plan.md` — Execution Rules, atomic Steps (each with full field set), Per-Step Budget Roll-Up, Packet Completion Gate, Acceptance Ceremony.
- `task-map.md` — conditional: only when packet spans > 1 task ID, reopens prior work, or supersedes earlier packet. Skip otherwise.

**Snippets** (`references/snippets/`) — workspace-invariant boilerplate. Include verbatim (with `<!-- snippet: <name> -->` marker) when applicable; skip entirely when not. **Paraphrasing is forbidden.**
- `context-discipline.md` — closing block of every `packet.spec.md`. Mandatory.
- `orca-delegation.md` — opening paragraph of `OrcaSlicer Reference Obligations` in `packet.spec.md` and `requirements.md`. Skip both sections if no OrcaSlicer parity.
- `wasm-staleness.md` — bullet in `design.md` §Architecture Constraints when the change surface feeds guest WASM. Skip for host-only changes.
- `coord-system.md` — bullet in `design.md` §Architecture Constraints when the packet touches geometry / mm↔unit conversion. Skip for non-geometric packets.

**Examples and operations**
- `references/acceptance-criteria-examples.md` — when writing or reviewing ACs in Step 5/7. Shows a compliant AC and a non-compliant one side by side.
- `references/usage-examples.md` — when the user asks how to invoke the skill or wants an example invocation string.
- `references/troubleshooting.md` — when you hit a failure mode: prompt too broad, ambiguous task mapping, no relevant tasks in `docs/07`, an active packet conflict, missing OrcaSlicer ref, existing packet directory, or your own context approaching 60%.
