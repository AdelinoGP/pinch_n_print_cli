---
status: draft
packet: 253-build-guests-incremental-and-shared-target
task_ids:
  - TASK-NEW-BUILD-GUESTS-PERF
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
task_id_note: >
  No existing docs/07 row covers developer or CI build-time performance; every
  xtask row (TASK-214, TASK-341, TASK-342, TASK-343) is closed correctness work.
  This packet registers ONE new backlog row. Its numeric id is a ledger fact and
  is deliberately NOT frozen here. Step 10 re-derives it at registration time
  with `rg -o 'TASK-[0-9]+' docs/07_implementation_status.md | sort -uV | tail -1`
  and takes the next free number, then replaces the placeholder above.
---

# Packet Contract: 253-build-guests-incremental-and-shared-target

## Goal

Make `cargo xtask build-guests` rebuild only stale guests by default, compile every guest into one shared workspace-local target directory backed by converged guest lockfiles, and remove the per-guest fixed overhead from the freshness check, so the warm no-change path costs the freshness check rather than a full rebuild of every discovered guest.

## Scope Boundaries

This packet changes `xtask` build orchestration, the guest `Cargo.lock` set, and the docs that describe guest build behaviour. It does not change any guest's source, any WIT contract, the artifact-verified freshness definition, or the guest discovery predicate. sccache is explicitly deferred with a recorded trigger; no CI action is added beyond the existing `--check` step picking up the new lock-divergence gate.

## Prerequisites and Blockers

- Depends on: the artifact-verified freshness machinery already on disk — `check_command`, `build_stale_command`, `CheckOutcome`, `stale_reason`, and the `EXIT_FRESH` / `EXIT_STALE` / `EXIT_INFRA_ERROR` contract, all in `xtask/src/build_guests.rs` (closed by TASK-341 / TASK-342 / TASK-343).
- Unblocks: an sccache evaluation, whose trigger is the post-Phase-B CI timing measurement recorded by this packet.
- Activation blockers: none. All design decisions were settled in an approved grilling session; see `design.md` §Locked Assumptions and Invariants.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

### Phase A — freshness-aware default

- **AC-1. Given** a testable core `build_command_with(check, rebuild_stale, rebuild_all)` extracted from `build_command` in `xtask/src/build_guests.rs`, **when** the injected `check` returns a `CheckOutcome` with `code == EXIT_FRESH` and an empty `stale` vec, **then** `rebuild_stale` is invoked with an empty slice, `rebuild_all` is never invoked, and the returned code is `0`. | `cargo test -p xtask -- default_build_rebuilds_only_stale_guests 2>&1 | rg '^test' | tail -6`
- **AC-2. Given** the same testable core, **when** it is called in force mode, **then** `rebuild_all` is invoked exactly once, `check` is never invoked, and the returned code is the code `rebuild_all` produced. | `cargo test -p xtask -- force_mode_rebuilds_every_discovered_guest 2>&1 | rg '^test' | tail -6`
- **AC-3. Given** the `build-guests` flag parser in `xtask/src/build_guests.rs`, **when** it is given `--force`, `--sync-locks`, `--check`, `--list`, or no flag, **then** each maps to its own distinct parsed variant, and an unrecognised flag maps to the rejection variant that `xtask/src/main.rs` renders as exit code `2`. | `cargo test -p xtask -- build_guests_flag_parsing 2>&1 | rg '^test' | tail -6`
- **AC-4. Given** `parse_dist_args` in `xtask/src/dist.rs`, **when** it parses `--force-guests`, **then** `DistArgs.force_guests` is `true`, it defaults to `false` for empty args, and the guest-build call in `xtask/src/dist.rs` receives that flag so a default `dist` invocation rebuilds only stale guests. | `cargo test -p xtask -- dist_args_parse_force_guests 2>&1 | rg '^test' | tail -6`

### Phase B — shared guest target and lock convergence

- **AC-5. Given** the helper `guest_target_dir(ws_root)` in `xtask/src/build_guests.rs`, **when** `build_one_inner` builds any guest of either `GuestTree` variant, **then** it sets `CARGO_TARGET_DIR` to `<ws_root>/target/guests` for every guest, and both the intermediate `<lib_name>.wasm` lookup and the `<lib_name>-component-input.wasm` staging path derive from `<ws_root>/target/guests/wasm32-unknown-unknown/<profile-dir>`. | `cargo test -p xtask -- every_guest_builds_into_the_shared_target_dir 2>&1 | rg '^test' | tail -6`
- **AC-6. Given** `force_rebuild_wit_bindings` in `xtask/src/build_guests.rs`, **when** it issues its `cargo clean -p slicer-macros` and `cargo clean -p slicer-schema` commands, **then** each command carries `CARGO_TARGET_DIR` set to the same `guest_target_dir(ws_root)` value the build uses, so the stale-WIT recovery path cleans the directory the build actually reads. | `cargo test -p xtask -- force_rebuild_wit_bindings_cleans_the_shared_target_dir 2>&1 | rg '^test' | tail -6`
- **AC-7. Given** a lock-divergence analyser over the parsed `[[package]]` name and version pairs of a set of guest `Cargo.lock` files, **when** two guest locks resolve different versions of the same registry crate, **then** it reports exactly one line per diverging crate naming the crate and every distinct version, and reports nothing when all locks agree. | `cargo test -p xtask -- lock_divergence 2>&1 | rg '^test' | tail -9`
- **AC-8. Given** the real tree after `cargo xtask build-guests --sync-locks` has regenerated every guest lockfile, **when** `cargo xtask build-guests --check` runs, **then** it exits `0` and prints no divergence line. | `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"`
- **AC-9. Given** the shared target move is complete, **when** the tree is searched for the retired per-tree target path, **then** no occurrence of `test-guests/target` remains in `xtask/src` or in `CLAUDE.md`, and the search prints the sentinel instead. | `rg 'test-guests/target' xtask/src CLAUDE.md || echo NO_STALE_PATH_REFS`

### Phase C — warm-run overhead

- **AC-10. Given** the `rustc -vV` and `wasm-tools --version` probes injected as counting closures into the freshness path, **when** a freshness check evaluates more than one guest, **then** each probe is invoked exactly once for the whole invocation regardless of guest count. | `cargo test -p xtask -- version_probes_are_invoked_once_per_invocation 2>&1 | rg '^test' | tail -6`
- **AC-11. Given** a counting canonical-WIT loader injected into the build path, **when** more than one guest is rebuilt in a single invocation, **then** `canonical_world_model` is evaluated exactly once for that invocation rather than once per guest. | `cargo test -p xtask -- canonical_world_model_is_parsed_once_per_invocation 2>&1 | rg '^test' | tail -6`
- **AC-12. Given** the duplicate `verify_embedded_world` call removed from `stale_reason` in `xtask/src/build_guests.rs`, **when** the pre-existing freshness unit tests run, **then** every one still passes, proving no observable `StaleReason` changed. | `cargo test -p xtask -- stale 2>&1 | rg '^test' | tail -9`

### Phase D — conditional fast local profile

Phase D ships only if Step 8's measurement shows a net win for the edit-test loop. If Step 8 rejects it, AC-13 through AC-15 and AC-N3 are recorded as not-applicable with the measured numbers, and `FINGERPRINT_VERSION` stays `"v2"`.

- **AC-13. Given** Phase D shipped, **when** `PNP_GUEST_PROFILE` is unset, set to `release`, or set to `dev`, **then** the resolved cargo profile is `release`, `release`, and `dev` respectively, and any other value is a hard error naming the two accepted values. | `cargo test -p xtask -- guest_profile_env 2>&1 | rg '^test' | tail -9`
- **AC-14. Given** Phase D shipped, **when** the guest fingerprint is computed, **then** the resolved profile appears as its own synthetic fingerprint entry and `FINGERPRINT_VERSION` in `xtask/src/build_guests.rs` equals `"v3"`. | `cargo test -p xtask -- fingerprint 2>&1 | rg '^test' | tail -11`
- **AC-15. Given** Phase D shipped, **when** `xtask dist` resolves the guest build profile with `PNP_GUEST_PROFILE=dev` in the environment, **then** it resolves to `release` anyway. | `cargo test -p xtask -- dist_forces_release_guest_profile 2>&1 | rg '^test' | tail -6`
- **AC-16. Given** Step 8's measurement ran, **when** `measurements.md` in this packet directory is read, **then** it contains a table row for each of: warm no-change `build-guests`, `build-guests` after touching `crates/slicer-core/src`, `build-guests --check`, forced full build, and the `cargo test -p slicer-runtime` wall time under both guest profiles; each cell holds a measured value or the literal string `unmeasured gap`. | `rg -c '^\| ' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md`

## Negative Test Cases

- **AC-N1. Given** the testable core from AC-1, **when** the injected `check` returns `code == EXIT_INFRA_ERROR` (3), **then** the core returns `3` and neither `rebuild_stale` nor `rebuild_all` is invoked; it must never fall back to a full rebuild. | `cargo test -p xtask -- infra_error_aborts_build_and_never_falls_back_to_full_rebuild 2>&1 | rg '^test' | tail -6`
- **AC-N2. Given** guest lockfiles that disagree on a shared crate version, **when** the freshness check runs, **then** it yields `EXIT_STALE` (1) and not `EXIT_INFRA_ERROR` (3), and its divergence output names `--sync-locks` as the remedy. | `cargo test -p xtask -- lock_divergence_is_stale_not_infra_error 2>&1 | rg '^test' | tail -6`
- **AC-N3. Given** Phase D shipped and a guest fingerprinted under the `dev` profile, **when** a freshness check runs with the profile resolved to `release`, **then** that guest is reported stale with `StaleReason::FingerprintMismatch` rather than fresh. | `cargo test -p xtask -- switching_guest_profile_marks_guests_stale 2>&1 | rg '^test' | tail -6`
- **AC-N4. Given** the `build-guests` argument parser, **when** it receives an unknown flag such as `--fast`, **then** it yields the rejection variant that `xtask/src/main.rs` renders as exit code `2` with the usage block, unchanged from today's behaviour. | `cargo test -p xtask -- build_guests_flag_parsing 2>&1 | rg '^test' | tail -6`

## Verification

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p xtask 2>&1 | tail -15`
- `cargo xtask build-guests --check >/dev/null 2>&1; echo "exit=$?"`

## Authoritative Docs

- `CLAUDE.md` §"Guest WASM Staleness" — direct ranged read; normative statement of guest freshness rules, and it asserts the retired test-guest target path this packet moves.
- `CONTEXT.md` §"Artifact-verified freshness" — direct ranged read; defines the property that justifies rebuilding only stale guests.
- `docs/03_wit_and_manifest.md` — over 300 lines; delegate a SUMMARY of its `build-guests` mentions. It already documents `cargo xtask build-guests` as building any stale guests, which the code does not yet do; this packet closes that documented-versus-actual gap.
- `docs/05_module_sdk.md` — over 300 lines; delegate a SUMMARY of its `build-guests` workspace-contributor guidance to confirm whether the retired target path is quoted there.
- `docs/21_data_defaults_and_fixtures.md` — delegate; consult only if a new watched struct literal appears in xtask test code and needs a rest pattern or waiver.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `CLAUDE.md` §"Guest WASM Staleness" — replace the sentence asserting test-guests build into the per-tree target dir; document the freshness-aware default and the `--force` flag. Verify: `rg -q 'target/guests' CLAUDE.md`
- `CLAUDE.md` §"Guest WASM Staleness" — document the lock-convergence gate as a second failure mode of `--check`. Verify: `rg -q 'sync-locks' CLAUDE.md`
- `CONTEXT.md` — add the shared-guest-target term adjacent to "Artifact-verified freshness". Verify: `rg -q 'target/guests' CONTEXT.md`
- `docs/03_wit_and_manifest.md` — update the `build-guests` command rows so the documented default matches the implemented default and the lock gate is listed. Verify: `rg -q 'sync-locks' docs/03_wit_and_manifest.md`
- Phase-D-conditional: `CLAUDE.md` and `CONTEXT.md` gain `PNP_GUEST_PROFILE` and the v3 fingerprint only if Step 8 ships Phase D. Verify when shipped: `rg -q 'PNP_GUEST_PROFILE' CLAUDE.md CONTEXT.md`. If Step 8 rejects Phase D, this row is not applicable; verify the rejection instead with `rg -q 'Phase D: measured, rejected' docs/spec_packets/253-build-guests-incremental-and-shared-target/measurements.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
