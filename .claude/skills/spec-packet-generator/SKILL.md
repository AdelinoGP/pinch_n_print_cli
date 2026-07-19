---
name: spec-packet-generator
description: Generate a Pinch 'n Print spec packet under .ralph/specs/ from a prompt, file, URL, or approved plan; use Batch Protocol for multi-packet plans and to resume a queued plan.
type: anthropic-skill
version: "1.4"
metadata:
  internal: true
---

# Spec Packet Generator

Generate packet artifacts only; never implement them. One packet owns one coherent remediation slice under `./.ralph/specs/<spec-slug>/`.

## Context Contract

- Read budget: 120k absolute tokens. At 100k, state the remaining budget and reconfirm the plan. At 120k, stop reading and finalize, delegate, or hand off. At 150k, stop and emit a handoff with completed work, current state, next action, and files to reopen. Generation never uses an extended band.
- Never read a file over 600 lines in full. Never load generated code, lockfiles, `target/`, vendored dependencies, or full Cargo/test output.
- Before reading, ask whether a subagent can return only the answer. Always delegate `docs/07_implementation_status.md`; `docs/00_project_overview.md` when over 300 lines; any `OrcaSlicerDocumented/` inspection; any input file or URL over 300 lines; cross-crate trait/macro/generic tracing; Cargo check/test/clippy; and exploratory reads without a precise target.
- For direct reads, locate first, then open a default +/-40-line window. Every read tests one stated hypothesis.
- Trust Rust's type system: delegate `cargo check` before reading more code for a suspected type error. Request concrete impls or monomorphized errors, never full macro expansions. Use summarized `cargo metadata --format-version=1 --no-deps` rather than browsing manifests. For failed tests, request the name, assertion, and at most 20 relevant lines.

Every dispatch states one precise question, exact scope, and exactly one return format:

- `FACT: <5 lines or fewer>`
- `LOCATIONS: <at most 20 file:line entries, one context line each>`
- `SNIPPETS: <at most 3 verbatim snippets, 30 lines each, with file:line>`
- `SUMMARY: <at most 200 words, no code unless requested>`

Reject oversized replies and redispatch more narrowly.

Before work, emit:

```text
PLAN
- Goal: <one sentence>
- Files in scope (read+edit): <at most 3>
- Files explicitly out of scope: <list>
- Sub-agent dispatches planned: <question, scope, return-format; or "none">
- Estimated context cost: S / M / L (L means stop and decompose; execute only the first slice and hand off the rest)
- Stop condition: <binary done check>
```

## Inputs And Gates

Parameters:

- `input` (required): rough text, Markdown path, URL, approved plan, or `docs/specs/` plan containing `## Packet Queue`.
- `task_ids` (optional): `TASK-###` IDs from `docs/07`; infer and confirm when absent.
- `spec_slug` (optional): kebab-case; derive from approved scope when absent.
- `output_dir` (optional): defaults to `./.ralph/specs/<spec_slug>/`.
- `status` (optional): defaults to `draft`.

Use `AskUserQuestion` for every unresolved parameter, mapping, scope, status, overwrite, design, or activation decision; batch related questions. Never overwrite an existing packet directory without explicit approval.

Before writing, present the slug, task IDs, goal, in/out scope, files to generate, and downstream context cost (S/M/L). Write only after explicit approval. A batch queue approval is the standing answer unless grounding changes an entry's scope.

Default to `status: draft`. Set `active` only on explicit request, with no other active packet and no unresolved blocker, missing negative case, or missing exit criterion. Ambiguity that cannot be resolved remains `[BLOCK]` in `design.md` and prevents activation.

## Workflow

1. **Detect mode.** A queued `docs/specs/*-plan.md` is batch resume. A plan yielding multiple packets uses the Batch Protocol. For a file, delegate if over 300 lines; for a URL, delegate fetch and summary; otherwise use the prompt directly. Keep working notes to at most 10 bullets.
2. **Resolve backlog scope.** Delegate a `LOCATIONS` survey of `docs/07_implementation_status.md` for the smallest contiguous `TASK-###` slice matching the request. Confirm IDs exist. Prefer a tight slice; offer 1-3 choices when ambiguous. If no canonical task applies, stop and ask rather than inventing one.
3. **Gather authority.** Use `docs/00_project_overview.md` as the normative doc map, then read only relevant ranges of named docs. For parity, delegate a `LOCATIONS` search under `OrcaSlicerDocumented/`; never read OrcaSlicer source directly. Preserve returned paths verbatim in the packet.
4. **Ground claims.** Treat the plan as claims, not evidence. Verify every load-bearing pre-existing symbol and shape, WIT/IR identifier, prerequisite status, schema version, ADR slot, deviation ID, and new-test target against the tree using bounded FACT/LOCATIONS dispatches. Use verified names and `file:line`. Redesign falsified claims; regain approval if scope changes. Put unresolvable claims only in `[BLOCK]` Open Questions, never as facts. Use `.claude/skills/spec-review/references/preflight-gate.md` for the symbol inventory.
5. **Approve metadata.** Resolve slug, IDs, goal, scope, output directory, and status; pass the write gate above.
6. **Author criteria.** Apply the Acceptance Criteria Contract below before writing.
7. **Generate.** Load each applicable template under `references/templates/`, fill it concretely, and copy applicable snippets exactly. No `[spec-slug]`, `TASK-000`, `TBD`, or placeholder prose may remain.
8. **Self-review.** Check implementation detail, ownership, snippet integrity, overlap, atomic steps, context costs, and blockers. Revise failures.
9. **Preflight.** Invoke `spec-review --preflight <packet-dir>` via the Skill tool. Fix `PREFLIGHT BLOCKED` findings and rerun. Only `PREFLIGHT PASS` completes a packet. An unfixable blocker stays verbatim in `design.md`, keeps status `draft`, and is reported.
10. **Report and activate.** Report generated paths, slug, status, task IDs, governing docs, OrcaSlicer refs, preflight verdict, assumptions/questions, self-review result, and aggregate context cost. If activation is requested, recheck its gate, change only the status, and remind the user that the next commands are `/spec-review <packet> --preflight` then `/swarm <packet>`.

This skill ends after generation. Never begin implementation.

## Acceptance Criteria Contract

Each criterion must:

- use Given/When/Then and be falsifiable by one command or test;
- name exact fields, config keys, manifest entries, counts, paths, error codes, enum variants, or output fragments;
- end with `|` plus its own runnable, delegation-friendly command; repeat shared commands rather than writing "see AC-N";
- trace to implementation steps without implied work.

Together, criteria cover the main success path and the likeliest silent regression. Add a negative/rejection criterion for validator, scheduler-rule, contract-boundary, enforcement, or failure-path changes. For IR/config/schema assertions, copy exact field paths and snake_case manifest keys; list every field behind an asserted count.

Every verification command must be delegation-friendly: produce a small, parseable result (exit code, single assertion, JSON path). Wrap or filter any command that would dump more than 200 lines on success so a subagent can return a FACT.

Never use `cargo test --workspace` as a pipe-suffixed AC command. Prefer `cargo test -p <crate> --test <file>` with an optional test name. A packet may list the workspace suite once under packet-level Verification only when closure explicitly requires it; otherwise use targeted tests plus `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`.

Read `references/acceptance-criteria-examples.md` while authoring or reviewing criteria.

## Packet Ownership

Use the templates as the field-level specification:

- `packet.spec.md`: YAML metadata, solution-shaped goal, prose scope summary, authoritative Given/When/Then ACs, 2-3 gate commands, docs impact, and preflight-visible obligations.
- `requirements.md`: motivation, authoritative full scope, docs/Orca references, AC-ID summary, full verification matrix, and only cross-step expectations.
- `design.md`: selected approach, exact code-change surface, read-only/out-of-bounds context, dispatches, contract notes, invariants, risks, context cost, and `[FWD]`/`[BLOCK]` questions.
- `implementation-plan.md`: atomic ordered steps; each includes task IDs, objective, pre/postconditions, allowed reads, at most 3 edits, out-of-bounds files, dispatches, S/M cost, authorities, narrow verification, and a falsifying exit condition. Split any L step.
- `task-map.md`: emit under the conditions stated in its template; it owns the `docs/07` crosswalk.

Do not duplicate owners: ACs occur only in `packet.spec.md`; full scope and verification matrix only in `requirements.md`; code surface only in `design.md`; step contracts only in `implementation-plan.md`.

Snippets under `references/snippets/` are verbatim-or-absent and retain their `<!-- snippet: name -->` marker:

- `context-discipline.md`: mandatory closing block of every `packet.spec.md`.
- `orca-delegation.md`: exactly in `packet.spec.md` and `requirements.md` when parity applies; omit both sections otherwise and never copy it into `design.md`.
- `wasm-staleness.md`: one `design.md` Architecture Constraints bullet when the change surface feeds guest WASM.
- `coord-system.md`: one `design.md` Architecture Constraints bullet for geometry or mm/unit conversion.

## Packet Safety

- Never modify files in another packet directory. To absorb prior work, mark that packet `superseded` in its own `packet.spec.md`, describe the absorption in the new `requirements.md`, and inspect predecessor files through a SUMMARY dispatch.
- A packet must be implementation-grade on first emission: exact assertions, negative cases, decisive code surfaces, and per-step exits; no placeholder prose.
- Above rough reference sizes (`packet.spec.md` 100, `requirements.md` 150, `design.md` 250, `implementation-plan.md` 300 lines), diagnose repetition rather than compressing for a limit. Remove only duplication, generic prose covered elsewhere, or boilerplate replaced by a canonical snippet. Never trim ACs, negative cases, exits, scope lists, dispatch contracts, invariants, or questions. Report the concrete complexity driving excess length.
- Check every applicable snippet is exact or absent, cross-file ownership is respected, bullets/ACs are not near-duplicates, and generic workspace advice is omitted.
- When adding a new struct field or bumping a public schema/version constant, the step that performs the change owns the struct-literal blast radius (every test/non-test site that compiles against the struct) AND the test-assertion fallout (every test that hard-asserts the old constant value). See `references/templates/implementation-plan.md` Blast-radius discipline and `references/templates/design.md` Architecture Constraints. Do not let the implementation worker discover these via a follow-up `cargo check` — pre-bake them into the step's "Files allowed to edit" and verification commands.
- Every AC verification command's `--test` binary must be one that can actually drive the asserted behavior. If the asserted behavior requires an end-to-end driver (`run_slice`, real pipeline, real module dispatch) and the chosen binary is the wrong home, the packet must author the test fixture or pick a binary that has the setup today. See `references/templates/packet.spec.md` AC verification command rule.

## Batch Protocol

For multiple packets, read `references/batch-protocol.md` before proceeding.

- Persist the approved plan verbatim at `docs/specs/<slug>-plan.md` and append `## Packet Queue`; if the plan already has a committed home, reference that path. Obtain one approval for the dependency-ordered queue.
- 2-3 packets: author inline, sequentially, through the complete workflow. 4 or more: orchestrate; authoring subagents write packets and independent reviewer subagents run preflight. The orchestrator authors no packet files, checks each `packet.spec.md` against the plan, and never opens `design.md` or `implementation-plan.md`.
- Resume at the first pending row whose dependencies are generated. Reconstruct dependency exports with bounded SUMMARY dispatches. Blocked dependents stay pending; independent entries may continue.
- Update each queue row immediately. On budget stop, finish the in-flight packet and leave remaining rows pending. The final report includes the queue, blockers, commit-together reminder, and resume instruction.

## References

Load only when triggered:

- `references/templates/*.md`: when generating that packet file.
- `references/snippets/*.md`: when deciding applicability or copying canonical text.
- `references/batch-protocol.md`: for any multi-packet input or queued resume.
- `references/acceptance-criteria-examples.md`: when writing/reviewing ACs.
- `references/troubleshooting.md`: on broad scope, ambiguous/missing task mapping, active conflict, missing Orca reference, existing directory, or budget exhaustion.
- `references/usage-examples.md`: when asked how to invoke the skill.

The packet must let a fresh swarm identify the exact backlog slice, governing docs and Orca references, observable acceptance, implementation order, read/edit/forbidden surfaces, expected dispatches, and per-step context cost without guessing.
