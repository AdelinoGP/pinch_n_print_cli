# 159 — Gate: merge closed `perf-vs-orca` and `proptest-openrscad` into `orca-feature-gaps`

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**Manual Phase 2 synchronization gate (HITL / user-triggered):** wait until both `perf-vs-orca` and `proptest-openrscad` have fully closed and merged into `origin/master`, then bring `orca-feature-gaps` up to date with that settled `origin/master` so Phase 2 rewrites all 52 draft packets **once** against the final codebase.

User ruling (2026-11-23): `orca-feature-gaps` returns to main development only after `perf-vs-orca` and `proptest-openrscad` are fully closed and merged. Running Phase 1 (`149`, `123`, `151`–`158`) immediately while gating Phase 2 packet rewrites (`160` and its graduated batch tickets) on this ticket prevents paying twice for SOTA 2-file rewrites against a moving target.

### Checklist to close this gate

1. **Confirm both upstream workstreams are closed and merged to `origin/master`:**
   - `perf-vs-orca` closed & merged to `origin/master`
   - `proptest-openrscad` (remaining packets through `17`) closed & merged to `origin/master`
2. **Merge `origin/master` into `orca-feature-gaps`** and resolve any conflicts cleanly (re-verifying no dual/stale seams are re-introduced).
3. **Verify branch health after the final merge:**
   - `cargo check --workspace --all-targets` exits `0`
   - `cargo xtask build-guests --check` exits `0` (rebuild if stale)
   - `cargo xtask check-literals` and `cargo xtask check-fmt` exit `0`
   - Re-run affected test suites (or `cargo xtask test --summary --workspace`) if the final merge touched code overlapping `orca-feature-gaps`.
4. **Record the late-merge delta:** diff `f899cdf2..origin/master` (or the commit `154`/`155`/`158` audited against) and list any **new** architectural changes, ADRs, renamed/deleted symbols, WIT bumps, or config keys that landed during the final tail of `perf-vs-orca` and `proptest-openrscad`, so [160 — Graduate Phase 2 SOTA packet re-authoring batches](./160-graduate-phase2-sota-packet-reauthoring-batches.md) can fold that delta straight into the Phase 2 batch briefs without re-running `154`/`155`/`158` from scratch.

### Resolution

Closed when the final merge commit SHA on `orca-feature-gaps`, the settled `origin/master` SHA, gate results, and the late-merge delta summary are recorded in `## Answer` (and `map.md` `## Decisions so far` is updated).

## Answer
