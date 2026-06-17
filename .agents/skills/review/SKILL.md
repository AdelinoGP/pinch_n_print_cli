---
name: review
description: Performs multi-dimensional code review for the Pinch 'n Print Rust/WASM codebase. Validates architecture compliance, IR contract integrity, WASM module manifests, backpressure gates, and operational governance rules before code is committed.
---

# Review Command

## Overview

Review code changes against the Pinch 'n Print architecture contract and operational governance rules. This is the final quality gate before code is committed.

Review is rigorous on critical and high priority dimensions — these catch real bugs. Medium and low priorities are noted but do not block.

---

## Input

**Review scope**: `$ARGUMENTS`

This can be:
- Specific file paths to review
- "all-changes" — review all files modified in current working tree
- A feature/module name — find and review all related files

---

## Gather Version Control Context

Before reviewing, gather version control context:

```bash
git status
```

Shows all uncommitted changes — use to identify scope of modified, added, and deleted files.

```bash
git log --oneline -10
```

Recent commit history — use to understand whether changes build on or conflict with recent work.

```bash
git diff
```

Read the actual diffs of all uncommitted changes. This is the primary input for the review.

If `$ARGUMENTS` is "all-changes" or empty, the git diff defines the review scope. If specific files are given, still run `git status` for broader context but focus on requested files.

---

## Review Dimensions

### 1. Architecture Compliance (Critical)

**IR Schema Contracts:**
- [ ] IR field additions are backwards-compatible (additive fields only for minor bumps)
- [ ] No removal or type changes to existing IR fields without major version bump
- [ ] New IR types added to `crates/slicer-ir/` with proper serialization

**WASM Module Contracts:**
- [ ] Manifest `[ir-access].reads` and `[ir-access].writes` match actual usage
- [ ] Manifest `wit-world` references correct world version
- [ ] Manifest `[stage]` uses canonical stage identifiers from `docs/01_system_architecture.md`
- [ ] Claim declarations use canonical claim names (perimeter-generator, infill-generator, etc.)
- [ ] `min-host-version`, `min-ir-schema`, `max-ir-schema` are correctly set

**Host Service Usage:**
- [ ] Host services accessed via SDK wrappers, not raw wasmtime
- [ ] Memory allocated in per-layer arenas, not long-lived WASM memory

### 2. Pipeline Integrity (Critical)

**Stage Ordering:**
- [ ] Modules only read IR produced by earlier stages in `STAGE_ORDER`
- [ ] Data Dependency Matrix in `docs/01_system_architecture.md` is respected
- [ ] Paint propagation contract followed: SlicePostProcess → Perimeters → PerimetersPostProcess

**PrePass/Per-Layer/PostPass Tiering:**
- [ ] PrePass stages run sequentially and produce Blackboard output
- [ ] Per-Layer stages use per-layer arenas and rayon parallelism
- [ ] PostPass stages operate on `Vec<LayerCollectionIR>` with no parallelism

**Error Handling:**
- [ ] `fatal = true` errors abort entire slice
- [ ] `fatal = false` errors continue with last valid IR state and emit degraded warning
- [ ] Error codes follow established conventions (e.g., paint error codes 501-504)

### 3. Claim System Integrity (Critical)

**Claim Uniqueness:**
- [ ] No two modules hold same claim globally without region override
- [ ] Claim holder transitions only allowed for infill-generator and support-generator
- [ ] Non-transitionable claims (perimeter-generator, seam-placer, etc.) remain stable per (object, claim)

**Region Override Resolution:**
- [ ] Object-level overrides applied before region-level overrides
- [ ] No ambiguous overlapping layer-range overrides

### 4. Memory & Resource Safety (Critical)

**Per-Layer Arena:**
- [ ] All geometry allocated in per-layer arenas
- [ ] Memory freed after layer completes — no cross-layer pointer aliasing
- [ ] `Object URL` pattern: if any URL.createObjectURL used, URL.revokeObjectURL called in cleanup

**WASM Instance Management:**
- [ ] Sequential modules use single instance (no pooling)
- [ ] Parallel-safe modules use instance pool sized to rayon threads

**Event/Timer Cleanup:**
- [ ] Event listeners added in `on_mount` → removed in `on_destroy`
- [ ] Timers (setTimeout/setInterval) cleared on component destroy

### 5. Type Safety (High)

**Rust Types:**
- [ ] Types defined in `crates/slicer-ir/` for IR contracts (not inline)
- [ ] Enums in `crates/slicer-ir/` with exhaustive matches
- [ ] No `unsafe` without documented safety invariants
- [ ] No `.unwrap()` on fallible operations in hot paths — use `?` or explicit error handling

**Serialization:**
- [ ] IR types implement proper `serde::Serialize`/`Deserialize`
- [ ] Version fields on IR types match declared schema versions

### 6. Performance (High)

**Parallelism:**
- [ ] Per-layer processing uses rayon, not manual threading
- [ ] No shared mutable state between layer workers
- [ ] `scene.requestRender()` called after entity changes, not on every tick

**Memory Bounds:**
- [ ] Layer count budgets enforced with explicit failure behavior
- [ ] Memory budgets tracked and exceeded gracefully
- [ ] Large collections use pagination or streaming where applicable

### 7. Compatibility Policy (High)

**Version Bumps:**
- [ ] Additive changes: minor version bumps
- [ ] Breaking changes (rename/remove/type change): major version bumps
- [ ] Module rejects incompatible modules at startup with explicit diagnostics

**Startup Diagnostics:**
- [ ] Expected vs actual host version in error messages
- [ ] Expected vs actual WIT world version in error messages
- [ ] Expected IR range vs host IR version in error messages

### 8. Config Robustness (Medium)

**Manifest Config:**
- [ ] Config keys namespaced properly (`com.community.tpms-infill.density`, not `density`)
- [ ] Core keys have no namespace prefix
- [ ] `config.overridable-per-region` and `config.overridable-per-layer` correctly declared

**Validation:**
- [ ] Required config fields validated before use
- [ ] Unknown config keys produce actionable error messages

### 9. Operational Governance (Medium)

**Deviation Handling:**
- [ ] Intentional deviations from architecture docs recorded in `docs/DEVIATION_LOG.md`
- [ ] Critical deviations have explicit waiver from architecture owner
- [ ] Mitigation plans with owner and due date for conditional items

**Release Checklist:**
- [ ] Architecture Acceptance Gate categories documented
- [ ] No unresolved critical deviations before release

### 10. Security (Critical)

- [ ] No hardcoded tokens, passwords, or API keys in source
- [ ] No `eval()` or dynamic code execution
- [ ] File operations validate path boundaries (no path traversal)
- [ ] WASM modules cannot access host resources outside declared IR access

---

## Automated Checks

Run these automated checks and include results in the review. **Do not run `cargo test --workspace` by default** — the suite is >1000 tests and takes ≥11 minutes. Pick targeted tests for the changed surface; only run the full workspace when the user explicitly asks or this is a packet-close acceptance review.

```bash
cargo build --workspace
```

Builds all crates. Any build failure is a blocking issue.

```bash
cargo clippy --workspace -- -D warnings
```

Lint checks. Warnings become errors with `-D warnings`. Any clippy issues are blocking.

```bash
cargo check --workspace --all-features
```

Additional type checking with all feature flags. Fast — runs in seconds, no test execution.

### Test selection (this is the part agents most often get wrong)

From `git diff --stat`, identify the changed crates and modules. Then run **the narrowest test that proves the change**:

- Single test:    `cargo test -p <crate> --test <file> -- <test_name> --nocapture`
- One test file:  `cargo test -p <crate> --test <file>`
- One crate:      `cargo test -p <crate>`

For a multi-crate change, run one targeted command per affected crate — that is still vastly cheaper than `--workspace`. If you cannot identify a narrow test that exercises the change, say so explicitly in the review output rather than falling back to the full suite.

`cargo test --workspace` is reserved for:
1. The user explicitly asks for it, OR
2. A packet acceptance ceremony / completion gate before status flips to `implemented`, AND every narrower test on the changed surface has already passed.

When you do run it, dispatch to a sub-agent with the contract: *"Run `cargo test --workspace`; return FACT pass/fail; on failure return SNIPPETS with failing test name + assertion + ≤20 lines of relevant code."* Never paste the full log into your context.

---

## Output Format

```
## Code Review: [Feature/Module/Files]

### Summary
[2-3 sentences: overall quality assessment]

### Critical Issues (must fix before commit)
1. [CRIT-1] `file:line` — Description — **Fix:** ...

### High Priority (should fix)
1. [HIGH-1] `file:line` — Description — **Fix:** ...

### Medium Priority (consider fixing)
1. [MED-1] `file:line` — Description — **Fix:** ...

### Positive Observations
- [Good] Description of well-implemented pattern

### Automated Check Results
| Check | Status | Details |
|-------|--------|---------|
| Build | PASS/FAIL | ... |
| Tests (targeted) | PASS/FAIL | crates/files exercised, X passed, Y failed |
| Clippy | PASS/FAIL | ... |
| Check (all-features) | PASS/FAIL | ... |

### Verdict
[APPROVED / APPROVED WITH NOTES / CHANGES REQUESTED]
[If changes requested: list critical/high items that must be addressed]
```

---

## Rules

- **Be rigorous on Critical and High dimensions** — these catch real bugs
- **Be pragmatic on Medium and Low dimensions** — note but don't block
- **Never flag issues already caught by clippy** — clippy handles those
- **Always run automated checks** — don't rely on manual inspection alone
- **Run targeted tests, not the full workspace** — the suite is >1000 tests / ≥11 minutes; only run `cargo test --workspace` when the user asks or a packet-close acceptance review demands it
- **Provide specific fix suggestions** — not vague "improve this"
- **Acknowledge good code** — positive reinforcement matters
- **Reference authoritative docs** — cite `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, etc. for architecture claims
- **Check DEVIATION_LOG** — open critical deviations block approval

---

## Handoff

After review:
- If **APPROVED**: ready for commit
- If **APPROVED WITH NOTES**: ready for commit, non-blocking suggestions noted
- If **CHANGES REQUESTED**: list specific fixes needed before re-review
