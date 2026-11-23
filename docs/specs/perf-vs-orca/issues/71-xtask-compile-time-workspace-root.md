# xtask compile-time workspace root poisons gates from alternate checkouts

Type: task
Status: resolved (2026-10-17; fix verified and committed as `f067efa2`)
Assignee: claimed and resolved by the 2026-10-17 OpenCode wayfinder session
continuing the map (human's route pick from the post-t70 frontier)
Blocked by: none

## Question

Route and fix the closeout follow-up surfaced by
[Clipper in-tree vendoring and first value-exact engine candidate](70-clipper-in-tree-vendoring-first-candidate.md):
`cargo xtask` resolves the workspace root from the compile-time
`env!("CARGO_MANIFEST_DIR")`, so invoking it from a worktree or alternate
checkout bakes THAT directory into the shared target's cached xtask binary.
Afterwards, the main tree's xtask resolves `crates/` — and every derived
artifact path, including guest wasm staging and dist targets — at the dead
path. The dangerous failure mode is not the dead path itself but the gates:
`cargo xtask check-literals` / `check-test-quality` then silently scan
**zero files** and still print green ("watchlist: 0 types" for a real
watchlist of 159). Any commit made while a gate run in that state is
unverified.

Measured on 2026-10-17 (t70 closeout; the operational recovery — purging
`target/debug/xtask.exe`, `deps/xtask-*`, `.fingerprint/xtask-*` and
re-running — is recorded on ticket 70).

Code facts (re-verified 2026-10-17 at HEAD): the compile-time bake lives in
`workspace_root()` in `xtask/src/build_guests.rs`, and the same
`env!("CARGO_MANIFEST_DIR")` pattern has two further copies in `xtask`:
`editions.rs` (its own `fn workspace_root`) and `wit_verify.rs`. A fix must
cover all three or name why the remainder is safe.

Decision to make: the fix shape — runtime root resolution (e.g. walk upward
from the current directory to a repo marker, as the t70 campaign drivers
already do, or `git rev-parse`-based) instead of the compile-time macro, plus
whether the zero-scan gates get a hard guard (error when the scan visits zero
files or the workspace root is unreadable, so a future recurrence fails loud
instead of green). Scope stays inside `xtask`; no slicer crates touched.
Acceptance: the main tree's gates stay green, and a worktree invocation no
longer changes what the main tree's xtask resolves (reproduce the t70
poisoning scenario before and after).
## Resolution

**2026-10-17 — resolved as a verified infra repair; commit pending the
human's approval** (the map's no-auto-commit discipline; this is the
t39/t30 infra-repair class, not a perf candidate).

**Before (measured, reproducing ticket 70's mechanism + refining it).** A
worktree invocation overwrites the shared target's top-level
`target/debug/xtask.exe` with a binary carrying the foreign root (bytes
measured: foreign path ×1, main path ×0 after the foreign run). While the
foreign root is ALIVE, a main-tree invocation of that binary silently scans
the foreign tree — same-content checkouts make the output indistinguishable
from a true pass (green, real watchlist, so the hazard is latent from the
first foreign invocation). When the foreign path dies it becomes ticket 70's
exact false green: `cannot read <dead>\crates` (os error 3) +
`0 violation(s) in 0 file(s) (watchlist: 0 types)` + **exit 0**, reproduced.

**Fix shipped (uncommitted).**
- `workspace_root()` (`xtask/src/build_guests.rs`) resolves at runtime —
  walk up from the current directory to the nearest ancestor holding both
  `Cargo.toml` and `crates/` — with a loud, actionable panic when nothing
  matches. The compile-time root is gone from the production path entirely;
  every subcommand funnels through it, covering guest staging and dist
  targets with the one fix point.
- Zero-scan loud guard in both gates (`check_literals::run`,
  `check_test_quality::run`): zero enforced files → distinct error naming
  the workspace root, exit 1, in every mode including `--report`.
- The two `#[cfg(test)]` helpers (`editions.rs`, `wit_verify.rs`) resolve
  runtime-first with the compile-time parent as fallback. Reason the
  production path is the one that mattered while the helpers are still
  aligned: the t70-class hazard was the cached cross-checkout *binary*; the
  helpers' compile-time fallback only ever fires for runners that don't set
  the package dir as CWD.

**After (measured).**
- A foreign-checkout-built fixed binary run with CWD in the main tree
  scans the main tree: `0 violation(s) ... (watchlist: 159 types)`, exit 0
  — the t70 scenario, now correct by construction (resolution carries no
  build-root dependence; the foreign-built binary contains no baked root,
  grep ×0). CWD in a repo-less void → `cannot locate the workspace root by
  walking up from ...`, exit 101 — loud, never green.
- Cross-checkout + shared-target invocations in both build orders: main tree
  green with the real watchlist, right after a foreign build.
- `cargo test -p xtask`: **185 passed; 0 failed** — 9 new behavioral tests
  (nearest-marker walk ×3, zero-scan refusal ×3 per gate incl. the
  report-mode variant, clean-fixture pass ×2).
- `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
  `cargo xtask check-literals` green (real watchlist 159);
  `cargo xtask check-test-quality --report` → 0 finding(s);
  `cargo xtask build-guests --check` exit 0. No guest touched.
- Docs updated (`docs/21` exit contract + `docs/22` report-mode clause);
  full transcripts + re-derivation commands:
  [`evidence/t71-xtask-worktree-root/FINDINGS.md`](../evidence/t71-xtask-worktree-root/FINDINGS.md).
