# Implementation Plan: 253-build-guests-incremental-and-shared-target

## Execution Rules

- Work one atomic step at a time; map every step to the packet's grouped task ID.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- The packet's task ID is a ledger fact. Re-derive it in Step 10; do not quote a number from any other file.

## Steps

### Step 1: Extract a testable build core and add force mode

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: make `cargo xtask build-guests` consult the freshness check and rebuild only stale guests, with `--force` preserving today's behaviour and an infrastructure error aborting with exit 3.
- Precondition: `check_command`, `build_stale_command`, and the three exit-code constants exist unchanged in `xtask/src/build_guests.rs`.
- Postcondition: `build_command_with` exists with injected `check`, `rebuild_stale`, `rebuild_all`, and a force flag; `build_command` is a thin wrapper over it; `build_all_command` holds the old unconditional loop; `parse_build_guests_flag` and its enum exist; `xtask/src/main.rs` matches on the enum and still exits 2 on an unknown flag.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — locate with `rg -n 'fn build_command|fn build_stale_command|fn check_command|EXIT_FRESH|EXIT_STALE|EXIT_INFRA_ERROR' xtask/src/build_guests.rs`, then a +/-40-line window around each hit. Never read the file whole.
  - `xtask/src/main.rs` — the `build-guests` match arm only; locate with `rg -n 'build-guests' xtask/src/main.rs`.
  - `xtask/src/test.rs` — `handle_guest_freshness_with` only; it is the shape to mirror. Locate with `rg -n 'fn handle_guest_freshness_with' xtask/src/test.rs`.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/src/main.rs`
- Files explicitly out of bounds:
  - `xtask/src/dist.rs` (Step 2 owns it)
  - `target/` in any form, any `Cargo.lock`, guest sources, `crates/slicer-schema/wit/**`
- Blast-radius discipline: not applicable. No struct field is added and no schema or version constant is bumped in this step.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p xtask 2>&1 | tail -15` and report pass/fail plus any failing assertion; scope: that command; return: `FACT` at most 5 lines
- Context cost: `S`
- Authoritative docs:
  - `CONTEXT.md` — the "Artifact-verified freshness" term block only; it is the justification for rebuilding only stale guests.
- OrcaSlicer refs: none; this packet has no parity surface.
- Verification:
  - `cargo test -p xtask -- default_build_rebuilds_only_stale_guests 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- force_mode_rebuilds_every_discovered_guest 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- infra_error_aborts_build_and_never_falls_back_to_full_rebuild 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- build_guests_flag_parsing 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask 2>&1 | tail -15` — FACT pass/fail; confirms `xtask/src/test.rs` needed no change
- Exit condition: AC-1, AC-2, AC-3, AC-N1, and AC-N4 pass, and the pre-existing xtask suite is still green with no assertion weakened. Falsified if any existing test required editing to pass.

### Step 2: Make dist freshness-aware

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: `xtask dist` rebuilds only stale guests by default and rebuilds all under `--force-guests`.
- Precondition: Step 1 complete; `build_command_with` and the force flag exist.
- Postcondition: `DistArgs` carries `force_guests: bool`; `parse_dist_args` accepts `--force-guests` and defaults it to `false`; the guest-build call in `xtask/src/dist.rs` passes the flag through.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/dist.rs` — `DistArgs`, `parse_dist_args`, the guest-build call site, and the existing arg-parse tests. Locate with `rg -n 'struct DistArgs|fn parse_dist_args|build_guests::|fn .*dist_args' xtask/src/dist.rs`, then +/-30-line windows.
  - `xtask/src/build_guests.rs` — the signature of the wrapper Step 1 created only; locate with `rg -n 'pub fn build_command' xtask/src/build_guests.rs`.
- Files allowed to edit (at most 3):
  - `xtask/src/dist.rs`
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs` (Step 1 finished it; Step 3 reopens it)
  - `target/`, any `Cargo.lock`, guest sources
- Blast-radius discipline: `DistArgs` gains a field, so every struct literal of `DistArgs` must be updated in the same step. Before editing, dispatch a `LOCATIONS` worker for `DistArgs {` across `xtask/src`. A preflight sweep during authoring found exactly one code site, the initializer inside `parse_dist_args` itself; the existing arg-parse tests assert on returned fields and construct no literal, so the blast radius is one line. That count is a ledger fact — re-derive it rather than trusting it. If the dispatch finds a site outside `xtask/src/dist.rs`, stop and re-scope the step. The `check-literals` struct-literal rule applies to test-code literals of watched types (a `pub` struct with at least five named fields); `DistArgs` is `pub(crate)` with four fields after this change, so confirm with `cargo xtask check-literals` rather than assuming either way.
- Expected sub-agent dispatches:
  - Question: list every struct-literal site of `DistArgs` in the repo; scope: `xtask/src/**`; return: `LOCATIONS` at most 10 entries
- Context cost: `S`
- Authoritative docs:
  - none required; `dist` behaviour is described only by its own usage text in `xtask/src/main.rs`, which Step 10 updates.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask -- dist_args_parse_force_guests 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask 2>&1 | tail -15` — FACT pass/fail
  - `cargo xtask check-literals` — FACT exit code
- Exit condition: AC-4 passes and the existing dist arg-parse tests are green without weakening. Falsified if a `DistArgs` literal exists outside this step's allowed file.

### Step 3: Route every guest through one shared target directory

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: all guests of both trees build into `<ws_root>/target/guests`, and the stale-WIT recovery path cleans that same directory.
- Precondition: Steps 1 and 2 complete.
- Postcondition: `guest_target_dir` and `guest_profile_dir` exist; `build_one_inner` sets `CARGO_TARGET_DIR` unconditionally and derives both the intermediate wasm path and the `-component-input.wasm` staging path from it; the `GuestTree::TestGuest` special case and the `crates/slicer-wasm-host/test-guests/target` literal are gone; `force_rebuild_wit_bindings` takes `ws_root` and sets the same `CARGO_TARGET_DIR` on both `cargo clean -p` commands.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — locate with `rg -n 'fn build_one_inner|fn force_rebuild_wit_bindings|CARGO_TARGET_DIR|intermediate_base|component-input' xtask/src/build_guests.rs`, then +/-40-line windows.
  - `.gitignore` — confirm `**/target/` covers the new location. Two-line read.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `target/` in any form — the new directory is created by the build, never inspected by reading
  - any `Cargo.lock` (Steps 4 and 5 own lock work), guest sources, `xtask/src/dist.rs`
- Blast-radius discipline: not applicable. No struct field or version constant changes here. `force_rebuild_wit_bindings` gains a parameter, so update its single call site inside `build_one` in the same file.
- Expected sub-agent dispatches:
  - Question: after the edit, run a forced full guest build and report only the final result line and exit code; scope: `cargo xtask build-guests --force`; return: `FACT` at most 3 lines. Never return the build log.
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` — §"Guest WASM Staleness" only; it states the retired path, which Step 10 rewrites. Read to confirm the wording; do not edit it here.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask -- every_guest_builds_into_the_shared_target_dir 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- force_rebuild_wit_bindings_cleans_the_shared_target_dir 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo xtask build-guests --force >/dev/null 2>&1; echo "exit=$?"` — FACT single line; the first run after this step is expected to be a full rebuild
  - `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"` — FACT single line; must be 0 immediately after the forced build
- Exit condition: AC-5 and AC-6 pass, a forced build succeeds, and the freshness check reports fresh straight afterwards. Falsified if any guest's componentization fails to find its intermediate wasm.

### Step 4: Add the lock-divergence analyser and the sync-locks command

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: detect and report guest lockfiles that resolve different versions of the same crate, and provide the one-pass regeneration command that fixes them.
- Precondition: Step 3 complete. The tree still has divergent locks at this point; that is what makes the analyser testable against reality.
- Postcondition: `parse_lock_packages`, `lock_divergences`, `LockDivergence`, and `sync_locks_command` exist; `check_command` runs the divergence check ahead of the per-guest loop, prints one deterministically ordered line per diverging crate naming `--sync-locks`, and folds the result into `EXIT_STALE`; `xtask/src/main.rs` dispatches `--sync-locks`.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — locate with `rg -n 'pub fn check_command|fn check_command_with|fn discover_guests|EXIT_STALE|EXIT_INFRA_ERROR' xtask/src/build_guests.rs`, then +/-40-line windows.
  - `xtask/src/main.rs` — the `build-guests` match arm and the usage block only.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/src/main.rs`
- Files explicitly out of bounds:
  - Every real `Cargo.lock`. The analyser's tests use synthetic in-memory fixtures. Never open a lockfile to design the parser; the `[[package]]` name and version shape is all it needs.
  - `target/`, guest sources, `xtask/src/dist.rs`
- Blast-radius discipline: not applicable; no struct field or version constant changes. `LockDivergence` is a new type with no existing literals.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check` on the current tree and report only the exit code and the count of divergence lines; scope: that command; return: `FACT` at most 3 lines. Purpose: confirm the analyser fires on the real divergent tree before Step 5 converges it.
- Context cost: `M`
- Authoritative docs:
  - `CONTEXT.md` — the "Artifact-verified freshness" term block only; confirms that lock divergence is a staleness signal, not an infrastructure error.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask -- lock_divergence 2>&1 | rg '^test' | tail -9` — FACT pass/fail
  - `cargo test -p xtask -- lock_divergence_is_stale_not_infra_error 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- build_guests_flag_parsing 2>&1 | rg '^test' | tail -6` — FACT pass/fail; `--sync-locks` is now a parsed variant
  - `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"` — FACT single line; expected 1 at this point, because the real locks have not been converged yet
- Exit condition: AC-7 and AC-N2 pass, and `--check` reports exit 1 with at least one divergence line on the un-converged tree. Falsified if the analyser reports zero divergences on a tree that a manual spot check shows is divergent.

### Step 5: Regenerate every guest lockfile

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: converge the whole guest lockfile set so the shared target directory actually shares compiled dependencies.
- Precondition: Step 4 complete, so the analyser can prove convergence rather than assert it.
- Postcondition: every core `wit-guest/Cargo.lock` and every test-guest `Cargo.lock` is regenerated and committed; the divergence analyser reports zero rows; a forced full build succeeds; the before and after guest-build timings are appended to `measurements.md`.
- Files allowed to read, with ranges when over 300 lines:
  - none. This step runs a command and reads its exit code.
- Files allowed to edit (at most 3):
  - `docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md` — create it in this step with its header row and the Phase B rows.
  - The regenerated `Cargo.lock` files are generated artifacts of `cargo xtask build-guests --sync-locks`, not hand edits. They must never be opened or edited by hand. Their count is a ledger fact; re-derive it with `git status --porcelain -- '**/Cargo.lock' | wc -l` after the run.
- Files explicitly out of bounds:
  - Every `Cargo.lock` as a read or a manual edit. Regenerate only.
  - `target/`, guest sources, all xtask source files (Steps 1 to 4 finished the code this step exercises)
- Blast-radius discipline: not applicable; no struct or constant changes.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --sync-locks` then `cargo xtask build-guests --check` and report only the two exit codes and the number of changed lockfiles; scope: those commands plus `git status --porcelain`; return: `FACT` at most 4 lines
  - Question: time a forced full guest build and then a warm no-change build, reporting only the two wall-clock figures; scope: `cargo xtask build-guests --force` then `cargo xtask build-guests`; return: `FACT` at most 4 lines. Never return the build log.
- Context cost: `S`
- Authoritative docs:
  - none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"` — FACT single line; must be 0
  - `cargo xtask build-guests --force >/dev/null 2>&1; echo "exit=$?"` — FACT single line; must be 0, proving convergence broke no guest build
  - `git status --porcelain -- '**/Cargo.lock' | wc -l` — FACT single line; the count of regenerated locks
- Exit condition: AC-8 passes, a forced full build still succeeds, and `measurements.md` holds the Phase B before and after rows. Falsified if convergence breaks any guest build; in that case record which guest and which crate version, and stop rather than pinning around it silently.

### Step 6: Memoize the version probes and the canonical world model

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: invoke `rustc -vV`, `wasm-tools --version`, and the canonical WIT parse at most once per xtask invocation instead of once per guest.
- Precondition: Step 5 complete; the tree is converged and green.
- Postcondition: a `VersionProbes` value is constructed once and threaded through `CheckContext` and the build path; `compute_guest_freshness` receives it as a parameter rather than spawning the two processes itself; `build_stale_command` loads the canonical model once and passes it into `build_one`, whose two canonical loads both read the memoized value.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — locate with `rg -n 'fn compute_guest_freshness|fn rustc_version_verbose|fn wasm_tools_version|struct CheckContext|fn build_one\b|fn build_stale_command|canonical_world_model' xtask/src/build_guests.rs`, then +/-40-line windows.
  - `xtask/src/wit_verify.rs` — `canonical_world_model` only; locate with `rg -n 'pub fn canonical_world_model' xtask/src/wit_verify.rs` and read +/-45 lines. Note it already ignores its `stage` argument, which is why it is safe to memoize.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/src/wit_verify.rs`
- Files explicitly out of bounds:
  - `target/`, any `Cargo.lock`, guest sources, `xtask/src/dist.rs`, `xtask/src/main.rs`
- Blast-radius discipline: `CheckContext` gains a field, so every struct literal of `CheckContext` must be updated in this step. **This is the largest blast radius in the packet.** A preflight sweep during authoring found roughly a dozen literal sites, one production constructor inside `check_command_with` plus about eleven in the test module, all within `xtask/src/build_guests.rs` and therefore all inside this step's allowed edit list. Treat that count as a ledger fact: dispatch a `LOCATIONS` worker for `CheckContext {` across `xtask/src` and update every hit it returns, rather than trusting the number written here. If the count materially exceeds the sweep, or any site falls outside `xtask/src/build_guests.rs`, split this step into a mechanical literal-update sub-step and a memoization sub-step before proceeding. Prefer adding the new field with a `..` rest pattern at test sites where the surrounding assertions do not depend on it, per `docs/21_data_defaults_and_fixtures.md`; confirm with `cargo xtask check-literals`.
- Expected sub-agent dispatches:
  - Question: list every struct-literal site of `CheckContext` in the repo; scope: `xtask/src/**`; return: `LOCATIONS` at most 10 entries
  - Question: summarize the struct-literal rest-pattern and waiver format rules; scope: `docs/21_data_defaults_and_fixtures.md`; return: `SUMMARY` under 150 words
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` — delegate a SUMMARY; needed only for the `CheckContext` test-literal rule.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask -- version_probes_are_invoked_once_per_invocation 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- canonical_world_model_is_parsed_once_per_invocation 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask 2>&1 | tail -15` — FACT pass/fail; the fingerprint tests must still pass, proving memoization did not change the fingerprint value
  - `cargo xtask check-literals` — FACT exit code
- Exit condition: AC-10 and AC-11 pass and the fingerprint tests are unchanged and green. Falsified if any fingerprint value changes; memoization must return the same strings the per-guest calls returned.

### Step 7: Remove the duplicate artifact decode in stale_reason

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: decode each artifact once per freshness check while preserving every observable `StaleReason`.
- Precondition: Step 6 complete; the canonical model is memoized, so the canonical error variants are surfaced by the loader.
- Postcondition: the `#[cfg(not(test))]` `verify_embedded_world` block is gone from `stale_reason`; `VerifyError::Decode` and `VerifyError::Parse` map to `StaleReason::Undecodable` on the single remaining path; `VerifyError::CanonicalEmpty` and `VerifyError::CanonicalUnreadable` map to the synthetic `Drift` with `DriftKind::MissingStagePackage`; the fingerprint-before-drift priority order is unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — locate with `rg -n 'fn stale_reason|enum StaleReason|compare_worlds|verify_embedded_world|MissingStagePackage' xtask/src/build_guests.rs`, then +/-40-line windows.
  - `xtask/src/wit_verify.rs` — the `VerifyError` and `DriftKind` definitions only; locate with `rg -n 'enum VerifyError|enum DriftKind' xtask/src/wit_verify.rs`.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `xtask/src/wit_verify.rs` (read-only here; Step 6 finished its edit)
  - `target/`, any `Cargo.lock`, guest sources
- Blast-radius discipline: not applicable; no struct field or version constant changes. Note the removed block was `#[cfg(not(test))]`, so the pre-existing tests never exercised it. Do not treat their passing as sufficient proof on its own; add a test asserting the artifact is decoded once, and assert the canonical-error mapping explicitly.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check` on the real tree and report the exit code and any reason lines; scope: that command; return: `FACT` at most 5 lines. Purpose: prove the production path, which the removed block only ran under, still reports fresh correctly.
- Context cost: `S`
- Authoritative docs:
  - `CONTEXT.md` — the "Artifact-verified freshness" term block only; the drift comparison it describes must survive unchanged.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask -- stale 2>&1 | rg '^test' | tail -9` — FACT pass/fail
  - `cargo test -p xtask 2>&1 | tail -15` — FACT pass/fail
  - `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"` — FACT single line; must be 0 on a converged, freshly built tree
- Exit condition: AC-12 passes, the whole xtask suite is green with no assertion weakened, and the real-tree check still reports fresh. Falsified if any existing staleness test needed editing, or if the real-tree check flips to non-zero.

### Step 8: Measure Phase D and decide

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: decide by measurement whether a dev-profile guest build is a net win for the edit-test loop.
- Precondition: Steps 1 to 7 complete and green. Measuring earlier would attribute Phase B's gains to Phase D.
- Postcondition: `measurements.md` holds the Phase C and Phase D rows, and states either `Phase D: ship` or `Phase D: measured, rejected` with the numbers that decided it.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md` — its current rows.
- Files allowed to edit (at most 3):
  - `docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md`
- Files explicitly out of bounds:
  - every source file. This is a read-and-measure step; no production code changes.
  - `target/` in any form.
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: time a forced full guest build under the release profile and under a dev profile passed with `--profile dev`, reporting only the two wall-clock figures; scope: those two commands; return: `FACT` at most 4 lines. Never return the build log.
  - Question: time `cargo test -p slicer-runtime` once with release-profile guest artifacts on disk and once with dev-profile guest artifacts on disk, reporting only the two wall-clock figures and the pass/fail of each run; scope: those two commands; return: `FACT` at most 5 lines.
- Context cost: `S`
- Authoritative docs:
  - none.
- OrcaSlicer refs: none.
- Verification:
  - `rg -c '^\| ' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md` — FACT count
  - `rg -q 'Phase D: (ship|measured, rejected)' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md; echo "decided=$?"` — FACT single line; must be 0
- Exit condition: `measurements.md` records both build-time and test-time figures for each profile, and states the decision. Every cell is a measured value or the literal `unmeasured gap`. Falsified if any figure is written without having been measured in this step.

### Step 9 (conditional on Step 8 shipping Phase D): Guest profile env var and v3 fingerprint

- Task IDs: this packet's single new backlog row (id re-derived in Step 10)
- Objective: let developers select a faster guest profile without letting a dev artifact read as fresh to a release check.
- Precondition: Step 8 recorded `Phase D: ship`. If it recorded `Phase D: measured, rejected`, skip this step entirely, leave `FINGERPRINT_VERSION` at `"v2"`, and go to Step 10.
- Postcondition: `resolve_guest_profile` and the `GuestProfile` enum exist, defaulting to `Release` and erroring on any value other than `dev` or `release`; the resolved profile is a synthetic fingerprint entry at path `synthetic:guest-profile`; `FINGERPRINT_VERSION` is `"v3"`; `xtask dist` resolves to `Release` regardless of the environment; `xtask/src/test.rs` honours the variable when it drives the freshness gate.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — locate with `rg -n 'FINGERPRINT_VERSION|fn compute_guest_freshness|starts_with\("v2|fn v2_' xtask/src/build_guests.rs`, then +/-30-line windows.
  - `xtask/src/dist.rs` — the guest-build call site only.
  - `xtask/src/test.rs` — the freshness-gate call site only.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/src/dist.rs`
  - `xtask/src/test.rs`
- Files explicitly out of bounds:
  - `target/guest-fingerprints` and all of `target/` — the sidecar invalidation is a runtime consequence, never a manual deletion
  - any `Cargo.lock`, guest sources, `xtask/src/main.rs`
- Blast-radius discipline: this step bumps a version constant, so it owns the test-assertion fallout. Before editing, dispatch a `LOCATIONS` worker for the literal `v2-` and for test names carrying `v2` across `xtask/src/build_guests.rs`, and update every hit in this same step. At authoring time the known sites were two `starts_with("v2-")` assertions and one test function whose name begins `v2_fingerprint_covers_`; that list is a ledger fact and must be re-derived at the moment of the bump, not trusted from here. The fingerprint input set also changes, so any test asserting the exact entry set must be updated in this step too.
- Expected sub-agent dispatches:
  - Question: list every site in `xtask/src/build_guests.rs` asserting the literal `v2-` prefix, plus every test function whose name contains `v2`; scope: `xtask/src/build_guests.rs`; return: `LOCATIONS` at most 10 entries
- Context cost: `M`
- Authoritative docs:
  - `CONTEXT.md` — the "Artifact-verified freshness" term block only; the fingerprint covers code inputs, and the build profile is a code input in the sense that it changes the emitted artifact.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p xtask -- guest_profile_env 2>&1 | rg '^test' | tail -9` — FACT pass/fail
  - `cargo test -p xtask -- fingerprint 2>&1 | rg '^test' | tail -11` — FACT pass/fail; this is the binary asserting the old constant value and must run in this step, not at the ceremony
  - `cargo test -p xtask -- dist_forces_release_guest_profile 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask -- switching_guest_profile_marks_guests_stale 2>&1 | rg '^test' | tail -6` — FACT pass/fail
  - `cargo test -p xtask 2>&1 | tail -15` — FACT pass/fail
  - `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"` — FACT single line; expected non-zero on the first run after the bump because every sidecar is invalidated, then 0 after one rebuild
- Exit condition: AC-13, AC-14, AC-15, and AC-N3 pass, no `v2-` assertion remains, and a single rebuild restores a clean check. Falsified if the check reports fresh immediately after the bump, which would mean the version literal is not reaching the sidecar.

### Step 10: Docs, CI confirmation, and backlog registration

- Task IDs: this packet's single new backlog row; re-derive its id here with `rg -o 'TASK-[0-9]+' docs/07_implementation_status.md | sort -uV | tail -1` and take the next free number. Then replace the `TASK-NEW-BUILD-GUESTS-PERF` placeholder in `packet.spec.md` and in `requirements.md`.
- Objective: bring the normative docs in line with the implemented behaviour and register the work in the backlog.
- Precondition: Steps 1 to 8 complete, and Step 9 either complete or explicitly skipped.
- Postcondition: `CLAUDE.md`, `CONTEXT.md`, and `docs/03_wit_and_manifest.md` describe the freshness-aware default, the shared target location, the `--force` and `--sync-locks` flags, and the lock-divergence gate; the `PNP_GUEST_PROFILE` and v3 fingerprint text is present only if Step 9 ran; the CI ordering is confirmed unchanged; the backlog row is registered; `measurements.md` holds the final table and the sccache deferral with its trigger.
- Files allowed to read, with ranges when over 300 lines:
  - `CLAUDE.md` — §"Guest WASM Staleness" only; locate with `rg -n 'Guest WASM Staleness' CLAUDE.md` and read +/-30 lines.
  - `CONTEXT.md` — the "Artifact-verified freshness" term block only; locate with `rg -n 'Artifact-verified freshness' CONTEXT.md` and read +/-15 lines.
  - `.github/workflows/ci.yml` — the `test` job only.
  - `docs/03_wit_and_manifest.md` — over 300 lines; delegate a SUMMARY of its `build-guests` mentions rather than opening it.
- Files allowed to edit (at most 3):
  - `CLAUDE.md`
  - `CONTEXT.md`
  - `docs/03_wit_and_manifest.md`
  - Plus two packet-local files that are this packet's own bookkeeping, not code surface: `measurements.md` and the task-id placeholder in `packet.spec.md` and `requirements.md`. The `docs/07_implementation_status.md` row is added through a worker dispatch, never by reading the backlog in full.
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` as a direct read; dispatch a worker to append the row.
  - every xtask source file; the code is finished by now.
  - `docs/05_module_sdk.md` unless the delegated LOCATIONS check finds it quotes the retired target path; if it does, that edit is in scope and replaces one of the three above.
  - every other packet directory.
- Blast-radius discipline: not applicable; no code changes.
- Expected sub-agent dispatches:
  - Question: what does `docs/03_wit_and_manifest.md` state about `cargo xtask build-guests` and its `--check` variant, and which rows describe the default rebuild behaviour?; scope: that file; return: `SUMMARY` under 200 words
  - Question: does `docs/05_module_sdk.md` quote the retired per-tree guest target path?; scope: that file; return: `LOCATIONS` at most 10 entries
  - Question: append the packet's backlog row to `docs/07_implementation_status.md` using the next free TASK id, and report the id used; scope: that file; return: `FACT` at most 3 lines
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" — direct ranged read and edit.
  - `CONTEXT.md` §"Artifact-verified freshness" — direct ranged read and edit.
  - `docs/03_wit_and_manifest.md` — delegated SUMMARY, then a targeted edit.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'target/guests' CLAUDE.md CONTEXT.md; echo "docs=$?"` — FACT single line; must be 0
  - `rg -q 'sync-locks' CLAUDE.md docs/03_wit_and_manifest.md; echo "docs=$?"` — FACT single line; must be 0
  - `rg 'test-guests/target' xtask/src CLAUDE.md || echo NO_STALE_PATH_REFS` — FACT single line; must print the sentinel
  - `rg -c '^\| ' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md` — FACT count
  - `rg -q 'sccache' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md; echo "deferral=$?"` — FACT single line; must be 0
- Exit condition: AC-9 and AC-16 pass, every doc grep in `packet.spec.md` §Doc Impact Statement returns 0 (or the Phase-D rows are recorded not-applicable with the rejection sentinel), and the backlog row exists with a re-derived id. Falsified if any doc still asserts the retired target path or the old unconditional rebuild behaviour.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two files, closure-injection pattern already exists in the tree |
| Step 2 | S | One file; `DistArgs` blast radius is local to it |
| Step 3 | M | Path derivation touches build and clean paths plus a forced full build |
| Step 4 | M | New analyser plus `--check` integration plus a new flag |
| Step 5 | S | Command-driven; no source reading, generated lockfiles only |
| Step 6 | M | `CheckContext` blast radius plus threading through two call paths |
| Step 7 | S | One localized deletion with explicit error-mapping tests |
| Step 8 | S | Measurement only; all heavy output delegated |
| Step 9 | M | Conditional; owns the version-constant bump and its test fallout |
| Step 10 | S | Docs and registration; two delegated summaries |

Aggregate is M. No step is L. Step 9 is the split candidate if Phase D ships and its budget overruns; it is already isolated behind Step 8's decision.

## Packet Completion Gate

- All steps and exits complete, with Step 9 either done or explicitly recorded as skipped by Step 8's decision.
- Every pipe-suffixed AC command returns PASS, and the Phase D ACs are either PASS or recorded not-applicable with Step 8's numbers.
- Update `docs/07_implementation_status.md` through a worker dispatch with a re-derived TASK id, never a full backlog read.
- No reopened or superseded packet to reconcile; this packet supersedes nothing.
- `packet.spec.md` is ready for `status: implemented`, and its `task_ids` placeholder has been replaced with the registered id.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and every packet-level gate command.
- Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo xtask check-literals`; both are commit gates in this repo.
- Run `cargo xtask build-guests --check` and record the exit code; it must be 0.
- Record remaining packet-local risk: the shared target directory serialises any future parallel guest build, and the fingerprint bump (if Phase D shipped) forces one full rebuild for every developer after merge.
- Confirm context stayed at or below 150k standard, or at or below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command supports it, so test, bench, and example targets compile. `cargo test` compiles test targets by construction; `cargo check` and `cargo clippy` must carry the flag explicitly.
