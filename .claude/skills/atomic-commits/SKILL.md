---
name: atomic-commits
description: Organize uncommitted working-tree changes into small, logical, atomic commits by reading architecture docs for context and grouping changes by concern.
type: anthropic-skill
version: "1.0"
metadata:
  internal: true
---

# Atomic Commits

## Overview

Organize every uncommitted change in the working tree into a series of small, self-contained commits with descriptive messages. Each commit must be buildable and testable on its own: no commit may break `cargo build --workspace`.

This skill reads the architecture docs first to understand the domain, then studies the diffs in that context to find the correct grouping boundaries.

---

## Step 1 — Read Architecture Context

Read these docs **before** looking at any code. They provide the vocabulary for commit message scoping and reveal which changes are coupled at the architecture level.

```
docs/01_system_architecture.md   — pipeline tiers, stage names, claim system, data ownership
docs/02_ir_schemas.md            — IR struct contracts and versioning rules
docs/03_wit_and_manifest.md      — WIT worlds, host-boundary, module manifests
docs/04_host_scheduler.md        — DAG validation, four-phase execution, error handling
docs/05_module_sdk.md            — SDK helpers, #[slicer_module] macro, builder lifecycles
docs/07_implementation_status.md — current phase, open tasks, deviation log
docs/10_glossary_and_scenario_traces.md — terminology and normative scenario traces
```

Record the active packet slug (from `docs/07_implementation_status.md`) and the relevant domain terms. You will use both when naming commits.

---

## Step 2 — Gather Working-Tree State

Run these commands, in order, and study the full output:

```bash
git status
```

Identify every modified, added, untracked, and deleted path.

```bash
git diff
```

Read the full diff of tracked changes. Take note of which files change together and why.

```bash
git diff --cached
```

Note any changes already staged — these must be included in the commit plan.

```bash
git log --oneline -10
```

Observe the existing commit message style and scope tokens (e.g. `feat(host)`, `test(slicer-ir)`, `docs(packets)`).

---

## Step 3 — Group Changes into Logical Units

Partition every changed file (tracked and untracked) into named groups. A group becomes exactly one commit.

### Grouping rules

**Couple together:**
- A new type in `slicer-ir` and the matching dispatch/host wiring that uses it (they are meaningless apart).
- A spec or design file and the matching `docs/07_implementation_status.md` status row update.
- A `.toml` manifest change and the matching `src/lib.rs` in the same module (they are a single contract).
- A `#[test]` or `tests/` file and the exact production code it tests, when they were written together as a TDD pair.
- Related changes across `blackboard.rs`, `dispatch.rs`, and `lib.rs` when they implement a single feature (e.g. a new PrePass variant).

**Keep separate:**
- Spec / doc-only changes from code changes — docs are never mixed with implementation.
- Test-only changes from production code when the tests are standalone additions to an existing feature.
- Config (`.toml`) changes from Rust source when the config change is the entire commit (e.g. adding a new parameter key).
- Changes in different pipeline tiers (slicer-ir vs. slicer-host vs. a WASM module) when they are independent — unless they share an IR type boundary that was introduced in this batch.
- Untracked test files that exercise a specific feature should travel with that feature's implementation commit.

**One group = one concern.** If a group description requires "and", split it unless the "and" describes a single indivisible contract (e.g. "add `LiveRetraction` to IR and wire it into dispatch").

### Group template

For each group, record:

```
Group: <short label>
Files: <list of files>
Rationale: <one sentence explaining why these belong together>
Commit type: feat | fix | test | docs | refactor | chore
Scope: <crate or module name, e.g. slicer-ir, slicer-host, path-optimization-default>
Message: <type(scope): imperative summary under 72 chars>
Body: <optional: what changed and why, if the subject line is insufficient>
```

---

## Step 4 — Order the Commits

Sequence groups so that each commit compiles cleanly:

1. IR schema additions first (other code depends on them).
2. Host wiring that uses the new IR types second.
3. WASM module changes that consume the new host surface third.
4. Test additions after the code they test, unless the tests were written TDD-first (in that case, tests come before or alongside the implementation commit).
5. Spec / doc updates last (they reference the implemented code).

If a doc commit updates `docs/07_implementation_status.md` to close a task, it must come after the implementation commits that satisfy that task.

---

## Step 5 — Stage and Commit Each Group

For each group in order:

1. Stage only the files in that group:
   ```bash
   git add <file1> <file2> ...
   ```
   For untracked files, use `git add <path>`. Never use `git add -A` or `git add .`.

2. Verify the staging area contains exactly the intended files:
   ```bash
   git diff --cached --name-only
   ```

3. Create the commit with the drafted message. Use a HEREDOC to preserve formatting:
   ```bash
   git commit -m "$(cat <<'EOF'
   <type(scope): subject line>

   <optional body paragraph>

   EOF
   )"
   ```

4. After each commit, run a quick build check if the group touched Rust source:
   ```bash
   cargo check --workspace 2>&1 | tail -5
   ```
   If `cargo check` fails, stop and fix the issue before continuing to the next group.

---

## Step 6 — Final Verification

After all groups are committed:

```bash
git log --oneline -15
```

Confirm each commit appears with the correct message.

```bash
git status
```

Confirm the working tree is clean (no untracked or modified files remain, unless the user excluded them intentionally).

```bash
cargo build --workspace
```

Confirm the full workspace still builds after all commits.

---

## Commit Message Style

Follow the project's conventional-commit style observed in `git log`:

- **Format:** `type(scope): imperative verb phrase`
- **Types:** `feat`, `fix`, `test`, `docs`, `refactor`, `chore`
- **Scope examples:** `slicer-ir`, `slicer-host`, `path-optimization-default`, `packets`, `blackboard`, `dispatch`
- **Subject line:** ≤72 characters, present tense, no trailing period
- **Body:** optional; explain *what* and *why*, not *how*
- **Never mention:** issue numbers, PR numbers, or the words "various" / "misc" / "cleanup" without specifics

---

## Guardrails

- **Do not amend any existing commit.** Create new commits only.
- **Do not `git add -A` or `git add .`.** Stage files explicitly by path.
- **Do not commit files that contain secrets** (`.env`, credential files). Warn the user and skip them.
- **Do not skip `cargo check`** after Rust source commits. A broken intermediate state is worse than a large commit.
- **Do not push** unless the user explicitly asks.
- **Do not create branches.** Work on the current branch.
- **Pause and ask** if a group boundary is genuinely ambiguous and splitting would leave an uncompilable intermediate state.
