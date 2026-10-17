# t71 — xtask compile-time workspace root: poisoning mechanism reproduced, fixed loud

Take: 2026-10-17, perf-vs-orca map
([ticket 71](../../issues/71-xtask-compile-time-workspace-root.md)), route
picked by the human from the post-t70 frontier. Machine: Windows dev box.
Baseline HEAD: `bdd6461d` (pre-fix; t70's docs commit). No slicer crates
touched; all changes live in `xtask`, the two gate docs, and the tracker.

## 1. Baseline (unpoisoned, pre-fix)

`cargo xtask check-literals` from the main tree:

```
check-literals: 0 violation(s) in 0 file(s) (watchlist: 159 types)
```

The real watchlist is 159 types (matches ticket 70's recorded purge-recovery
figure).

## 2. Before-repro — the poisoning mechanism, measured stepwise

Reproduction of ticket 70's closeout incident, contained in a detached temp
worktree (`wt71`, a clean checkout of `bdd6461d` outside the repo) sharing
the main tree's target via `CARGO_TARGET_DIR`. Steps in order:

1. **Foreign build into the shared target.** From `wt71`:
   `CARGO_TARGET_DIR=<main>/target cargo xtask check-literals` →
   `0 violation(s) ... (watchlist: 159 types)` (green — it scanned the
   WORKTREE through the baked root). Bytes measured after:
   `target/debug/xtask.exe` contains the `wt71` path string **1×** and the
   main repo path **0×** — the shared binary is foreign-baked.
2. **Foreign path alive, main-tree invocation.** From the main tree:
   `cargo xtask check-literals` → `0 violation(s) ... (watchlist: 159 types)`,
   exit 0. This was the foreign binary scanning the foreign tree
   successfully — **indistinguishable from a correct pass**. The gate never
   told you it was not this checkout's files.
3. **Foreign path dead.** `mv wt71 wt71-gone` (same trick ticket 70's
   incident hit: the foreign worktree was gone), then from the main tree:

   ```
   xtask: cannot read C:\Users\agpen\AppData\Local\Temp\opencode\wt71\crates: check-literals: 0 violation(s) in 0 file(s) (watchlist: 0 types)
   The system cannot find the path specified. (os error 3)
   EXIT=0
   ```

   (stderr/stdout interleaving in the capture; the two lines are the
   `cannot read <dead-root>\crates` diagnostic and the summary.) **exit 0,
   watchlist 0, zero files scanned — ticket 70's false green reproduced
   exactly**: the pre-commit gate passed while scanning nothing.

Mechanism refinement over ticket 70's wording (measured): the top-level
`target/debug/xtask.exe` (plus its un-hashed `deps/xtask.exe` link) is
overwritten by whichever checkout last ran `cargo xtask`. While the foreign
root is alive the poisoned binary still scans it — same-content checkouts
make the output indistinguishable from a true pass — so the hazard is
**latent the moment a foreign invocation happens**, and turns into the loud
`cannot read` + zero-scan green only once that root dies.

## 3. Recovery before the fix (ticket 70's recorded procedure, re-measured)

```
rm -f target/debug/xtask.exe target/debug/deps/xtask* target/debug/.fingerprint/xtask-*
```

plus worktree removal; afterwards the main-tree gate returned to the real
159-type watchlist.

## 4. The fix

- `workspace_root()` (`xtask/src/build_guests.rs`): runtime resolution —
  walk up from the current directory to the nearest ancestor holding both
  `Cargo.toml` and `crates/` (`locate_root_from`, tested) — and a loud panic
  with an actionable message when no ancestor matches. The compile-time
  macro is gone from the production resolution path entirely (a fixed binary
  contains no baked root at all; grep ×0 in the foreign-built artifact).
- `check_literals::run` / `check_test_quality::run`: zero-scan guard — when
  the enforced scan collects zero files (with or without filters; report mode
  included), print a distinct error naming the workspace root and exit 1.
  Report mode downgrades findings, never a missing scan.
- The two `#[cfg(test)]` helpers (`editions.rs`, `wit_verify.rs`) resolve
  runtime-first with the compile-time parent as fallback, so a cached
  cross-checkout test binary resolves the tree it runs in.
- Behavior kept: `workspace_root()` still canonicalizes as before; all
  subcommands funnel through `main.rs`'s single `workspace_root()` call per
  subcommand, so one fix point covers every subcommand and every derived
  artifact path (guest staging, dist targets).

Docs: one paragraph each added to `docs/21_data_defaults_and_fixtures.md`
(check-literals exit contract) and `docs/22_test_quality.md` (the report
mode cannot downgrade a missing scan).

## 5. After-evidence

1. **Cross-checkout invocation, shared target.** Worktree `wt71b` at
   `bdd6461d` with the five fixed files copied in byte-identical; from it:
   `CARGO_TARGET_DIR=<main>/target cargo xtask check-literals` →
   `0 violation(s) ... (watchlist: 159 types)` (resolves the worktree — the
   invocation lives there; correct).
2. **Main tree right after the foreign build** →
   `0 violation(s) ... (watchlist: 159 types)`, exit 0;
   `grep -c wt71b target/debug/xtask.exe` → 0 (no foreign-root bytes).
3. **Foreign-checkout-built binary, behavioral contract** (deterministic,
   independent of cargo's fingerprint interplay — run the worktree's own
   freshly built `xtask.exe` directly):
   - CWD = main tree → `0 violation(s) in 0 file(s) (watchlist: 159 types)`,
     exit 0. A binary whose build-root is another checkout scans THE MAIN
     TREE — the t70 scenario, now correct by construction.
   - CWD = foreign void (`C:\Users\agpen\AppData\Local\Temp\opencode`, no
     repo) → `cannot locate the workspace root by walking up from
     C:\Users\agpen\AppData\Local\Temp\opencode`, exit **101** — loud
     failure, never green.
4. **In-repo unit tests** (the re-derivable guard): `cargo test -p xtask` →
   **185 passed; 0 failed** (9 new: `locate_root_from_finds_marker_ancestor`,
   `locate_root_from_stops_at_the_nearest_marker`,
   `locate_root_from_unmatched_start_is_none`,
   `refuses_to_pass_when_enforced_scan_is_empty` +
   `refuses_to_pass_when_path_filters_match_nothing` +
   `minimal_clean_workspace_still_passes` in both gate modules, the
   test-quality variant additionally asserting report mode refuses).
5. **Standing gates at the fixed HEAD:**
   `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
   `cargo xtask check-literals` → `0 violation(s) ... (watchlist: 159 types)`;
   `cargo xtask check-test-quality --report` → `0 finding(s) in 0 file(s)
   [report mode]`; `cargo xtask build-guests --check` exit 0. No guest was
   touched; both freshness namespaces unaffected.

## 6. Residual, named

Cargo's own cross-checkout fingerprint interplay (reuse vs rebuild) is
nondeterministic from outside and left unmodeled on purpose: the resolver's
contract no longer depends on it, because resolution follows the current
directory and a foreign-built fixed binary resolves any tree it runs in
(transcript 5.3). The zero-scan guard is the second layer: even a binary
reaching an unintended-but-real tree reports which root it scanned, and a
scan of nothing fails loud.

## 7. Re-derivation

From a plain checkout: `cargo test -p xtask` runs the nine behavioral guards.
The §2/§5 transcripts re-derive with `git worktree add <tmp> HEAD`, the
`CARGO_TARGET_DIR` invocations above, and the direct-binary runs. No capture
tree or heavy fixture is referenced.