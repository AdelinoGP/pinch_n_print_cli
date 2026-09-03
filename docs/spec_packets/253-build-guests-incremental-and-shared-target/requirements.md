# Requirements: 253-build-guests-incremental-and-shared-target

## Packet Metadata

- Grouped task IDs: `TASK-531` — one new backlog row, id re-derived and registered at Step 10 (see `packet.spec.md` `task_id_note`). No pre-existing docs/07 row covers build-time performance.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`cargo xtask build-guests` rebuilds and componentizes every discovered guest on every invocation. `build_command` in `xtask/src/build_guests.rs` loops the full discovery result through `build_one` with no freshness consultation, even though `check_command` and `build_stale_command` already exist in the same file and are already composed in the freshness-aware order by `handle_guest_freshness_with` in `xtask/src/test.rs`. The default entry point is therefore the only guest-build path in the tree that ignores machinery the tree already trusts. `docs/03_wit_and_manifest.md` already documents the command as building any stale guests, so the code also contradicts its own documentation.

The cost is compounded by three structural issues. First, `build_one_inner` sets a shared `CARGO_TARGET_DIR` only when `spec.tree == GuestTree::TestGuest`; every core guest compiles its own private copy of `slicer-sdk`, `slicer-core`, `slicer-ir`, and `slicer-schema` into its own `target` directory. Second, those private target directories live outside the workspace `target/`, so the `Swatinem/rust-cache@v2` action in `.github/workflows/ci.yml` never caches them. Third, the freshness check itself carries fixed per-guest overhead: `compute_guest_freshness` spawns `rustc -vV` and `wasm-tools --version` once per guest, `canonical_world_model` in `xtask/src/wit_verify.rs` reparses the whole WIT directory per call, and `stale_reason` decodes each artifact twice, once through `embedded_world_model` and again through `verify_embedded_world` inside a block its own comment labels defensive.

Sharing a target directory only pays off when the guests resolve the same dependency versions. They do not: a survey during packet authoring found that only 15 of the 23 core `wit-guest/Cargo.lock` files agree on shared registry crate versions, with divergence on crates including `anyhow`, `autocfg`, and `hashbrown`, and the test-guest locks pinning two different `wit-bindgen` versions. Cargo keys artifacts by package, version, features, and profile, so a divergent lock silently reintroduces a full recompile of the shared dependency stack. Lock convergence is therefore part of the target-sharing work, not an optional tidy-up.

This is one coherent slice because all four phases touch the same orchestration file and the same freshness contract, and because measuring any one of them in isolation gives a misleading number: the shared target changes what a warm rebuild costs, which changes whether the fast local profile is worth shipping at all.

### Measured baseline

Measured on the requester's machine before any change. Re-measure in Step 8; these are the before column of `measurements.md`.

| Scenario | Measured |
| --- | --- |
| Warm `cargo xtask build-guests`, nothing changed | 1m54.6s |
| `cargo xtask build-guests --check` | 4.8s |
| Fresh isolated guest build, cold target | 1m29.7s |
| Second guest reusing the same `CARGO_TARGET_DIR` | 2.3s |

Guest count is a ledger fact; re-derive it with `cargo xtask build-guests --list` rather than quoting a number from this document. At authoring time discovery returned 23 core guests and 24 test-guests.

## In Scope

- Phase A: extract a testable core from `build_command` in `xtask/src/build_guests.rs` that consults `check_command` first and drives `build_stale_command` with only the stale set; add a force mode preserving today's rebuild-everything behaviour.
- Phase A: add a `--force` flag to the `build-guests` arm in `xtask/src/main.rs` and a parsed-flag enum so flag handling is unit-testable.
- Phase A: propagate `EXIT_INFRA_ERROR` (3) out of the build path when the pre-build check reports an infrastructure error; never degrade to a full rebuild.
- Phase A: make `xtask dist` freshness-aware, adding `force_guests` to `DistArgs` and a `--force-guests` flag to `parse_dist_args` in `xtask/src/dist.rs`.
- Phase B: add `guest_target_dir(ws_root)` returning `<ws_root>/target/guests`, and route every guest of both `GuestTree` variants through it, including the intermediate-wasm lookup and the `-component-input.wasm` staging path.
- Phase B: fix `force_rebuild_wit_bindings` to pass the same `CARGO_TARGET_DIR` the build uses, repairing a latent defect that already misdirects the stale-WIT recovery path for test-guests today.
- Phase B: add `cargo xtask build-guests --sync-locks`, which regenerates every core `wit-guest/Cargo.lock` and every test-guest `Cargo.lock` in one pass.
- Phase B: regenerate the whole guest lockfile set once and commit the result.
- Phase B: add a lock-divergence analyser to the freshness check that reports one line per crate on which two guest locks resolve different versions within the same semver-compatibility line, and names `--sync-locks` as the remedy. Intra-lock semver-major coexistence (one lock holding `syn` 1.x, 2.x and 3.x) is ordinary Cargo resolution, not drift, and is not reported.
- Phase C: memoize the `rustc -vV` and `wasm-tools --version` probes so each runs at most once per xtask invocation.
- Phase C: memoize the canonical world model so it is parsed at most once per invocation across both the check path and the build path.
- Phase C: delete the duplicate `verify_embedded_world` decode in `stale_reason`, moving its error mapping onto the single remaining path: `Decode` and `Parse` map to `StaleReason::Undecodable`; `CanonicalEmpty` and `CanonicalUnreadable` map to the synthetic `DriftKind::MissingStagePackage` drift.
- Phase D, conditional: measure guest build wall time and `cargo test -p slicer-runtime` wall time under release-profile guests versus dev-profile guests, and ship `PNP_GUEST_PROFILE=dev|release` only if the edit-test loop is a net win.
- Phase D, conditional: add the resolved profile as a synthetic fingerprint entry and bump `FINGERPRINT_VERSION` from `"v2"` to `"v3"`, including the two existing `starts_with("v2-")` assertions and the `v2_`-prefixed test name that assert on the old value.
- Phase D, conditional: force the release profile in `xtask dist` regardless of the environment variable.
- Record the before and after timing table in `measurements.md` in this packet directory.
- Doc edits listed in `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- sccache. Deferred with a recorded rationale: Phase B already brings guest dependency compilation under the existing CI `./target` cache, so sccache's marginal value is unknown until that is measured, and sccache skips crates built with incremental compilation, so it conflicts with a Phase D dev profile locally. Trigger for revisiting: the post-Phase-B CI timing measurement recorded in `measurements.md`.
- Changing the guest discovery predicate, any `[workspace]` sentinel, or any guest crate's source or manifest. Guests stay separate workspaces; only the target directory is shared. This keeps the packet conformant with ADR-0014, which locks the sentinel-based discovery walk.
- Converging the `arachne-perimeters` dependency on `slicer-core` with `default-features = false` to match the other slicer-core dependants. That is a legitimate feature variant, not lock drift, and Cargo will correctly build it as a second artifact.
- Changing the artifact-verified freshness definition in `CONTEXT.md`, the WIT contract, or anything under `crates/slicer-schema/wit`.
- Adding a numeric performance threshold as a gate. Timings are evidence; CI runner speed varies.
- Any change to `crates/pnp-cli/src/module_new.rs` scaffolding text or to the community-module build instructions in `docs/05_module_sdk.md`, which describe a single external module's own target directory, not the in-tree guest set.

## Authoritative Docs

- `CLAUDE.md` §"Guest WASM Staleness" — direct ranged read; short section, and it contains the sentence this packet must rewrite.
- `CONTEXT.md` §"Artifact-verified freshness" — direct ranged read; the file is long, so read only that term's block.
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — direct read; it is the only ADR governing this area. Its normative constraints are that the `[workspace]` sentinels are retained, that discovery avoids `cargo_metadata` and heavy xtask dependencies, and that shape predicates are added rather than relaxed. This packet conforms: it changes neither discovery nor the sentinels, and adds no dependency.
- `docs/03_wit_and_manifest.md` — over 300 lines; delegate a SUMMARY of `build-guests` mentions.
- `docs/05_module_sdk.md` — over 300 lines; delegate a SUMMARY of `build-guests` workspace-contributor guidance.
- `docs/21_data_defaults_and_fixtures.md` — delegate; only if a new watched struct literal appears in xtask test code.
- `.github/workflows/ci.yml` — short; direct read of the test job only, to confirm the `build-guests` then `--check` ordering still holds after the change.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-16`. Phase A is `AC-1` to `AC-4`; Phase B is `AC-5` to `AC-9`; Phase C is `AC-10` to `AC-12`; Phase D is `AC-13` to `AC-16`.
- Negative: `AC-N1` through `AC-N4`.
- Conditional: `AC-13`, `AC-14`, `AC-15`, and `AC-N3` apply only if Step 8 ships Phase D. If Step 8 rejects it, they are recorded not-applicable with the measured numbers, `FINGERPRINT_VERSION` stays `"v2"`, and the `PNP_GUEST_PROFILE` doc rows are not written.
- **Outcome: Step 8 measured Phase D and REJECTED it.** `AC-13`, `AC-14`, `AC-15`, and `AC-N3` are **not applicable**. No `PNP_GUEST_PROFILE` env var shipped, `FINGERPRINT_VERSION` stays `"v2"`, and no `PNP_GUEST_PROFILE` doc row was written. See `packet.spec.md` §Phase D for the measured numbers.
- Measurable refinements absent from the Given/When/Then text: `AC-8` must be run after `AC-7`'s analyser exists and after the lockfile regeneration step, otherwise it proves nothing. `AC-12` treats the pre-existing unit tests as the behavioural oracle; no assertion in them may be weakened to make the removal pass.
- Cross-packet impact: none. No other packet is active, and no packet directory outside this one is read or modified.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only three gate commands. Every `cargo test` row deliberately omits `--exact`: see the Context Discipline Notes below for the measured false-green this avoids.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p xtask 2>&1 \| tail -15` | Whole xtask unit suite; the behavioural oracle for Phases A and C | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p xtask -- default_build_rebuilds_only_stale_guests 2>&1 \| rg '^test' \| tail -6` | AC-1 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- force_mode_rebuilds_every_discovered_guest 2>&1 \| rg '^test' \| tail -6` | AC-2 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- build_guests_flag_parsing 2>&1 \| rg '^test' \| tail -6` | AC-3, AC-N4 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- dist_args_parse_force_guests 2>&1 \| rg '^test' \| tail -6` | AC-4 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- every_guest_builds_into_the_shared_target_dir 2>&1 \| rg '^test' \| tail -6` | AC-5 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- force_rebuild_wit_bindings_cleans_the_shared_target_dir 2>&1 \| rg '^test' \| tail -6` | AC-6 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- lock_divergence 2>&1 \| rg '^test' \| tail -15` | AC-7, AC-N2 | FACT pass/fail, quoting the result line |
| `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"` | AC-8; also the packet's guest-freshness gate | FACT single line |
| `rg 'test-guests/target' xtask/src CLAUDE.md \|\| echo NO_STALE_PATH_REFS` | AC-9 | FACT single line |
| `cargo test -p xtask -- version_probes_are_invoked_once_per_invocation 2>&1 \| rg '^test' \| tail -6` | AC-10 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- canonical_world_model_is_parsed_once_per_invocation 2>&1 \| rg '^test' \| tail -6` | AC-11 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- stale 2>&1 \| rg '^test' \| tail -9` | AC-12 | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- guest_profile_env 2>&1 \| rg '^test' \| tail -9` | AC-13, Phase D only | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- fingerprint 2>&1 \| rg '^test' \| tail -11` | AC-14, Phase D only | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- dist_forces_release_guest_profile 2>&1 \| rg '^test' \| tail -6` | AC-15, Phase D only | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- switching_guest_profile_marks_guests_stale 2>&1 \| rg '^test' \| tail -6` | AC-N3, Phase D only | FACT pass/fail, quoting the result line |
| `cargo test -p xtask -- infra_error_aborts_build_and_never_falls_back_to_full_rebuild 2>&1 \| rg '^test' \| tail -6` | AC-N1 | FACT pass/fail, quoting the result line |
| `rg -c '^\| ' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md` | AC-16 | FACT count |
| `cargo check --workspace --all-targets 2>&1 \| tail -5` | Workspace still compiles including test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5` | Commit gate | FACT pass/fail |
| `cargo xtask check-literals` | Commit gate | FACT exit code |
| `rg -q 'target/guests' CLAUDE.md CONTEXT.md; echo "docs=$?"` | Doc impact | FACT single line |
| `rg -q 'sync-locks' CLAUDE.md docs/03_wit_and_manifest.md; echo "docs=$?"` | Doc impact | FACT single line |

## Step Completion Expectations

- Ordering is load-bearing between Phase B steps. The lock-divergence analyser (Step 4) must exist before the lockfile regeneration (Step 5), because Step 5's exit condition is the analyser reporting zero divergences. Regenerating locks first would leave the analyser untested against a real divergent tree.
- Step 3 (shared target) and Step 5 (lock regeneration) each invalidate the guest build artifacts on disk. The first `build-guests` run after each is expected to be a full rebuild. Do not read that as a Phase A regression.
- Step 8 is a measurement step whose output decides whether Step 9 runs at all. Step 9 must not begin before Step 8's numbers are written to `measurements.md`.
- If Step 9 runs, its `FINGERPRINT_VERSION` bump invalidates every fingerprint sidecar under `target/guest-fingerprints`. That one-time full rebuild is expected and must be stated in the docs edit.
- Shared scratch state: `measurements.md` in this packet directory is appended by Steps 5, 8, and 10. Each writes its own rows; no step rewrites another's.

## Context Discipline Notes

- **Never add `--exact` to a `cargo test -p xtask` command in this packet.** Measured during authoring: `cargo test -p xtask -- all_fresh_yields_empty_stale_list_and_zero_code --exact` ran zero tests and still printed `test result: ok`, because every xtask test is a unit test whose full path is `build_guests::tests::<name>` and `--exact` matches the full path, not the bare name. That is a silent false green of exactly the kind `CLAUDE.md` warns about. Every AC command here therefore omits `--exact` and pipes through `rg '^test'` so the passed count is visible in the FACT return. A return showing zero passed is a FAIL, never a pass.
- `xtask/src/build_guests.rs` is long and holds both production code and its whole unit suite. Never read it in full. Locate with `rg -n '<symbol>' xtask/src/build_guests.rs` and open a +/-40-line window. The named symbols each step needs are listed in its "Files allowed to read" entry.
- Never read a `Cargo.lock`. The lock-divergence work needs parsed name and version pairs only; obtain them with `rg -A1 '^name = ' <lock>` or via the analyser's own tests against synthetic fixtures. Loading the whole guest lockfile set would exhaust the budget for no benefit.
- Never read anything under `target/`, including `target/guests` and `target/guest-fingerprints`.
- Delegate every cargo run. Timing measurements in Step 8 must be dispatched with a FACT return of the wall-clock line only, never the build log.
- `docs/03_wit_and_manifest.md` and `docs/05_module_sdk.md` are both over 300 lines; delegate a SUMMARY rather than opening them.
- `xtask` has no `tests/` directory. Every test this packet authors is a unit test inside an existing `#[cfg(test)] mod tests` block, so there is no test-binary aggregator to register with and no `mod` declaration to add.
