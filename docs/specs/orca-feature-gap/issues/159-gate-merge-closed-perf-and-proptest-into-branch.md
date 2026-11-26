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

## Progress (2026-11-26)

**Verdict: the PR #11 merge does not satisfy this gate. It remains open on the
`proptest-openrscad` half and on the late-merge delta (checklist item 4).**

1. Upstream workstreams:
   - `perf-vs-orca` — **satisfied.** Closed by human ruling in `6af5186f`
     (`tracker(perf): close perf-vs-orca map by human ruling`) and merged
     through PR #10 (`7b4c5781`), both on `origin/master`.
   - `proptest-openrscad` — **not satisfied.** Packets 01–14 are implemented
     on `origin/master` (their evidence files are under `docs/testing/`, e.g.
     `proptest-support-modules-evidence.md`). Packets 15 and 16 are implemented
     only on `origin/feature/proptest-openrscad`: `4badb776`
     (`pbt-layers-seams`), `b630c7b9`, `4b7062a9`, `ec2de14c` (`pbt-io-gcode`)
     are all *not* ancestors of `origin/master`. Packet 17
     (`pbt-ci-closure`) has no implementation commit anywhere, and
     `docs/testing/proptest-rollout-evidence.md` does not exist. Re-derive
     these four checks at claim time; they move.
2. Branch sync — **satisfied in the opposite direction.** PR #11 merged
   `orca-feature-gaps` into `master` (`4dcc2e32`) rather than merging
   `master` into the branch; the branch is now a strict ancestor of
   `origin/master` and was fast-forwarded to it. No branch merge is needed when
   packets 15–17 land; a fast-forward is enough.
3. Branch health — **verified** at `5b9d6147` (the `4dcc2e32` tree plus the
   ticket-163 docs commit): `build-guests --check` exit `0`, `check-literals` 0
   violations, `check-fmt` 62 packages clean, `clippy --workspace
   --all-targets -- -D warnings` exit `0`, `check-test-quality --report` 0
   findings. See ticket 151's Answer for the log paths.
4. Late-merge delta (`f899cdf2..origin/master`) — **not yet recorded.** This is
   the closing deliverable; it must list the new ADRs, renamed/deleted symbols,
   WIT bumps and config keys from the `perf-vs-orca` tail, `proptest-openrscad`
   packets 15–17, and the PR #11 merge itself.

## Answer
