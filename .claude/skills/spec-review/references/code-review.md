---
when: Read this when running the code scope of `spec-review` — reviewing uncommitted changes, named files, or the diff since a fixed point (branch, tag, commit) against the architecture contract before commit, with no packet as the primary contract. The mindset, evidence, trap, and verdict-floor rules in SKILL.md apply throughout and are not repeated here.
keywords: code review, working tree, all-changes, fixed point, branch diff, architecture compliance, IR contract, manifests, claims, code smells, commit gate
---

# Code Review (code scope)

Review code changes against the Pinch 'n Print architecture contract and operational governance rules — the final quality gate before code is committed. Rigorous on Critical and High dimensions (these catch real bugs); Medium findings are noted but do not block.

## Input & version-control context

The caller's scope argument is one of: specific file paths; `all-changes` / empty (the working-tree diff defines scope); a feature/module name (find and review related files); or a **fixed point** — a commit SHA, branch, tag, or `HEAD~N` ("review since X", "review this branch").

**Pin the diff before anything else.** With a fixed point: confirm it resolves (`git rev-parse <ref>`), then the review input is `git diff <ref>...HEAD` (three-dot — compares against the merge-base) plus the commit list from `git log <ref>..HEAD --oneline`. Without one: `git diff` on the working tree. A bad ref or an empty diff fails here — report it and stop; never spend dimensions or dispatches on an unpinned or empty diff.

Then gather context:

- `git status` — scope of modified, added, and deleted files.
- `git log --oneline -10` — does the change build on or conflict with recent work.
- The pinned diff — the primary review input.

If specific files are given, still run `git status` for broader context but focus on the requested files.

## Review dimensions

### 1. Architecture compliance (Critical)

**IR schema contracts:**
- IR field additions are backwards-compatible (additive fields only for minor bumps).
- No removal or type changes to existing IR fields without major version bump.
- New IR types added to `crates/slicer-ir/` with proper serialization.

**WASM module contracts:**
- Manifest `[ir-access].reads` / `[ir-access].writes` match actual usage.
- Manifest `wit-world` references the correct world version.
- Manifest `[stage]` uses canonical stage identifiers from `docs/01_system_architecture.md`.
- Claim declarations use canonical claim names (perimeter-generator, infill-generator, …).
- `min-host-version`, `min-ir-schema`, `max-ir-schema` correctly set.

**Host service usage:**
- Host services accessed via SDK wrappers, not raw wasmtime.
- Memory allocated in per-layer arenas, not long-lived WASM memory.

### 2. Pipeline integrity (Critical)

**Stage ordering:**
- Modules only read IR produced by earlier stages in `STAGE_ORDER`.
- Data Dependency Matrix in `docs/01_system_architecture.md` respected.
- Paint propagation contract followed: SlicePostProcess → Perimeters → PerimetersPostProcess.

**PrePass / Per-Layer / PostPass tiering:**
- PrePass stages run sequentially and produce Blackboard output.
- Per-Layer stages use per-layer arenas and rayon parallelism.
- PostPass stages operate on `Vec<LayerCollectionIR>` with no parallelism.

**Error handling:**
- `fatal = true` errors abort the entire slice.
- `fatal = false` errors continue with last valid IR state and emit a degraded warning.
- Error codes follow established conventions (e.g., paint error codes 501–504).

### 3. Claim system integrity (Critical)

- No two modules hold the same claim globally without region override.
- Claim holder transitions only allowed for infill-generator and support-generator.
- Non-transitionable claims (perimeter-generator, seam-placer, …) remain stable per (object, claim).
- Object-level overrides applied before region-level overrides; no ambiguous overlapping layer-range overrides.

### 4. Memory & resource safety (Critical)

- All geometry allocated in per-layer arenas.
- Memory freed after layer completes — no cross-layer pointer aliasing.
- Sequential modules use a single WASM instance (no pooling); parallel-safe modules use an instance pool sized to rayon threads.

### 5. Type safety (High)

- Types for IR contracts defined in `crates/slicer-ir/` (not inline); enums with exhaustive matches.
- No `unsafe` without documented safety invariants.
- No `.unwrap()` on fallible operations in hot paths — use `?` or explicit error handling.
- IR types implement proper `serde::Serialize` / `Deserialize`; version fields match declared schema versions.

### 6. Performance (High)

- Per-layer processing uses rayon, not manual threading; no shared mutable state between layer workers.
- Layer count / memory budgets enforced with explicit failure behavior; exceeded gracefully.
- Large collections use pagination or streaming where applicable.

### 7. Compatibility policy (High)

- Additive changes: minor version bumps. Breaking changes (rename/remove/type change): major version bumps.
- Host rejects incompatible modules at startup with explicit diagnostics: expected vs actual host version, WIT world version, and IR range in the error messages.

### 8. Config robustness (Medium)

- Config keys use snake_case (never kebab-case) and are namespaced properly (`com.community.tpms-infill.density`, not `density`); core keys have no namespace prefix.
- `config.overridable-per-region` / `config.overridable-per-layer` correctly declared.
- Required config fields validated before use; unknown keys produce actionable error messages.

### 9. Operational governance (Medium)

- Intentional deviations from architecture docs recorded in `docs/DEVIATION_LOG.md`; critical deviations have explicit waivers.
- Mitigation plans with owner and due date for conditional items.
- No unresolved critical deviations before release.

### 10. Security (Critical)

- No hardcoded tokens, passwords, or API keys in source.
- No `eval()`-style dynamic code execution.
- File operations validate path boundaries (no path traversal).
- WASM modules cannot access host resources outside declared IR access.

### 11. Smell baseline (judgement calls — never blocking)

A fixed set of Fowler code smells (*Refactoring*, ch. 3) matched against the pinned diff — the **quality axis**, running beside the architecture axis of dimensions 1–10. Two rules bind it:

- **The repo overrides.** A documented standard (CLAUDE.md, `docs/`) wins; where it endorses something the baseline would flag, suppress the smell.
- **Always a judgement call.** Each hit is a labelled heuristic ("possible Feature Envy"), never a hard violation. Report hits under the report's own `Smell Baseline` heading — never rerank them into the severity lists, so neither axis masks the other — and smells never move the verdict.

**Dispatch the sweep as one sub-agent**: give it the pinned diff command and this smell list pasted in full (the sub-agent cannot see this file). Return format: SUMMARY — per hit, smell name + `file:line` + the offending hunk quoted.

Each smell reads *what it is* → *how to fix*:

- **Mysterious Name** — a function, variable, or type whose name doesn't reveal what it does or holds. → rename it; if no honest name comes, the design's murky.
- **Duplicated Code** — the same logic shape appears in more than one hunk or file in the change. → extract the shared shape, call it from both.
- **Feature Envy** — a method that reaches into another object's data more than its own. → move the method onto the data it envies.
- **Data Clumps** — the same few fields or params keep travelling together (a type wanting to be born). → bundle them into one type, pass that.
- **Primitive Obsession** — a primitive or string standing in for a domain concept that deserves its own type. → give the concept its own small type.
- **Repeated Switches** — the same `match`/`if`-cascade on the same type recurs across the change. → replace with polymorphism, or one map both sites share.
- **Shotgun Surgery** — one logical change forces scattered edits across many files in the diff. → gather what changes together into one module.
- **Divergent Change** — one file or module is edited for several unrelated reasons. → split so each module changes for one reason.
- **Speculative Generality** — abstraction, parameters, or hooks added for needs the task doesn't have. → delete it; inline back until a real need shows.
- **Message Chains** — long `a.b().c().d()` navigation the caller shouldn't depend on. → hide the walk behind one method on the first object.
- **Middle Man** — a class or function that mostly just delegates onward. → cut it, call the real target direct.
- **Refused Bequest** — a trait implementer that ignores or stubs most of what it implements. → drop the impl, use composition or a narrower trait.

## Automated checks

Run these and include the results in the report. Test selection follows SKILL.md Test discipline — narrowest command that proves the change, never `cargo test --workspace` by default.

- `cargo build --workspace` — any build failure is blocking.
- `cargo clippy --workspace --all-targets -- -D warnings` — blocking. (`--all-targets` is required: plain `--workspace` skips test/bench targets and has shipped broken test targets before.)
- `cargo check --workspace --all-targets` — fast type-check of everything, seconds not minutes.
- Targeted tests: from the pinned diff's `--stat`, identify changed crates/modules; run one narrow proving command per affected crate. If no narrow test exercises the change, say so explicitly in the report rather than falling back to the full suite.
- If the diff touches any guest-WASM input path (see CLAUDE.md "Guest WASM Staleness"), run `cargo xtask build-guests --check` before interpreting any test result.

## Rules

- Never flag issues clippy already catches — clippy handles those.
- Always run the automated checks — do not rely on manual inspection alone.
- Reference authoritative docs for architecture claims (`docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, …).
- Check `docs/DEVIATION_LOG.md` — open critical deviations block approval.

## Handoff

- **APPROVED** — ready for commit.
- **APPROVED WITH NOTES** — ready for commit; non-blocking suggestions noted.
- **CHANGES REQUESTED** — list the specific fixes needed before re-review.

Report template: `references/output-format.md`.
