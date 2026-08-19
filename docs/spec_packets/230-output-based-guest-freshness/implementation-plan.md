# Implementation Plan: 230-output-based-guest-freshness

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- `xtask` tests are inline `#[cfg(test)] mod tests`; every narrow verification is `cargo test -p xtask <module>::tests::<name> -- --exact`, tee'd to `target/test-output.log` per `CLAUDE.md` §"Test output must always tee".
- This packet changes the staleness gate itself. An unexpected `STALE:` report during these steps may be a bug in the new logic — confirm by decoding the named artifact before rebuilding.

### Step 0 (gate, not a work step): confirm the forward dependency

Before Step 1, confirm `docs/spec_packets/229-wit-verify-declaration-model/packet.spec.md` reads `status: implemented`, then dispatch a `FACT` listing the actual signatures of `WorldModel`, `StageExpectation`, `stage_expectation`, `Drift`, `SHARED_PACKAGES`, `ROOT_COMPONENT_PACKAGE`, `canonical_world_model`, `embedded_world_model` and `compare_worlds` in `xtask/src/wit_verify.rs`. Reconcile any rename into `design.md` §Code Change Surface before writing code. If 229 is still `draft`, stop: this packet cannot start.

## Steps

### Step 1: Artifact-based stage resolution (TDD)

- Task IDs: `TASK-341`
- Objective: land `resolve_stage_from_world` and `StageResolutionError` in `xtask/src/wit_verify.rs`, excluding the empty-`wit_package` `STAGES` row.
- Precondition: Step 0 gate passed; `cargo check --workspace --all-targets` passes on the post-229 tree.
- Postcondition: AC-1, AC-2 and AC-3 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/src/lib.rs` — over 600 lines; only the `STAGES` rows' `stage_id` / `wit_package` / `wit_dir` fields and `stage_by_id`
  - `xtask/src/wit_verify.rs` — only the packet-229 model types and `module_stage_wit_dir`
- Files allowed to edit (at most 3):
  - `xtask/src/wit_verify.rs`
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs` (Step 2+), `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs`
- Expected sub-agent dispatches:
  - Question: "List every `STAGES` row's `stage_id` and `wit_package`; confirm exactly one row has an empty `wit_package` and name it"; scope: `crates/slicer-schema/src/lib.rs`; return: `FACT` (<=5 lines)
  - Question: "Decode `modules/core-modules/wipe-tower/wipe-tower.wasm` and one test-guest `.component.wasm`; return only the `package` declaration lines and the export line from each"; scope: those artifacts; return: `FACT` (<=10 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — amended C4 and finding R5-4; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::stage_resolves_from_embedded_package_name -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::empty_wit_package_stage_row_is_excluded_from_resolution -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::zero_multiple_and_unknown_stage_packages_are_unresolvable -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
- Exit condition: a world containing exactly one non-shared package resolves to the matching `StageExpectation`; zero, two, or an unknown package each return the corresponding `StageResolutionError`; the `PrePass::PaintSegmentation` row can never be selected.

### Step 2: `StaleReason` and the per-guest predicate (TDD)

- Task IDs: `TASK-341`
- Objective: land `StaleReason`, `CheckContext` and `stale_reason` in `xtask/src/build_guests.rs`, including the R5-4 manifest cross-check, and re-express `is_stale` on top of it with its third parameter changed from `&FreshnessSnapshot` to `&CheckContext`.
- Precondition: Step 1 exit met.
- Postcondition: AC-4, AC-5, AC-15, AC-18, AC-N1 and AC-N3 pass; the 4 pre-existing `build_guests` tests still pass (two of them call `is_stale` and move with its signature).
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — long; ranged reads only. Only `GuestSpec`, `discover_guests`' `stage_id` population, `is_stale`, `compute_guest_freshness`, `metadata_matches`, `fingerprint_metadata_path`, and the existing `mod tests` `TempDir` helper
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs`; `compute_shared_freshness`, `stage_wit_snapshot`, `shared_input_paths` (packet 231's surface)
- Blast-radius discipline: `is_stale`'s third parameter changes type, so every caller moves in this step. The closed caller set (verified with `rg -n 'is_stale' --glob '*.rs'`) is three sites, all in `xtask/src/build_guests.rs`: the loop inside `check_command`, and the two `is_stale` assertions in the existing test `missing_fingerprint_metadata_is_stale`. `crates/pnp-cli-locator::staleness_reason` names `is_stale` only in a doc comment and is packet 231's surface — do not edit it. No struct field is added to `GuestSpec` and no schema constant is bumped in this step. New tests that construct `GuestSpec` (7 named fields, `pub`) MUST use a `..` rest or an `// exhaustive: <reason>` waiver per `docs/21_data_defaults_and_fixtures.md`; run `cargo xtask check-literals` as part of this step's verification.
- Expected sub-agent dispatches:
  - Question: "Every construction site of `GuestSpec { .. }` in the workspace"; scope: `xtask/src/*.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` — the struct-literal waiver format; direct read
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — amended C4, C10, finding R5-4; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask build_guests::tests::core_guest_artifact_stage_must_equal_manifest_stage_id -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask build_guests::tests::test_guest_stage_comes_from_the_artifact_alone -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask build_guests::tests::undecodable_artifact_is_stale -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask build_guests::tests::is_stale_delegates_to_stale_reason -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `cargo xtask check-literals` — FACT pass/fail
- Exit condition: a core guest whose artifact stage differs from its manifest `[stage] id` yields `StaleReason::StageMismatch`; a test guest never does; an undecodable artifact yields `StaleReason::Undecodable`; `GuestSpec.stage_id` and `parse_stage_id_from_module_manifest` still exist; `is_stale` takes `&CheckContext` and delegates to `stale_reason`.

### Step 3: `CheckOutcome`, exit codes, and both call sites

- Task IDs: `TASK-341`
- Objective: change `check_command`'s signature, add the exit-code constants and the reporting contract, and migrate `xtask/src/main.rs` and `xtask/src/test.rs`.
- Precondition: Step 2 exit met.
- Postcondition: AC-6, AC-7, AC-8, AC-13 and AC-N2 pass; the workspace compiles with no bare-`i32` `check_command` consumer left.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — only `check_command` and `ensure_wasm_tools_available`
  - `xtask/src/test.rs` — long; ranged reads only. Only `test_command`'s freshness block and the existing `mod tests`
  - `xtask/src/main.rs` — full (short CLI dispatch file)
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/src/main.rs`
  - `xtask/src/test.rs`
- Files explicitly out of bounds:
  - `xtask/src/dist.rs` (must keep compiling against the unchanged `build_command`), `xtask/src/wit_verify.rs`
- Expected sub-agent dispatches:
  - Question: "Every call site of `check_command`, `build_command`, `is_stale` and `fingerprint_metadata_path` in the workspace"; scope: `xtask/src/*.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — amended C11 and findings R5-3, R5-7; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask build_guests::tests::stale_report_is_one_marker_line_plus_a_markerless_reason -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask build_guests::tests::missing_wasm_tools_is_infrastructure_error_not_staleness -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask build_guests::tests::unusable_canonical_set_is_infrastructure_error_not_fresh -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `if rg -q 'check_command\(&ws\)\.code' xtask/src/main.rs && rg -q '\.stale' xtask/src/test.rs && rg -q 'build_stale_command' xtask/src/test.rs && ! rg -q 'let check_code' xtask/src/test.rs; then echo PASS; else echo FAIL; fi` — FACT PASS/FAIL (identical to AC-13's command)
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: `check_command` returns `CheckOutcome`; `EXIT_INFRA_ERROR` is distinct from `0` and `1`; the `wasm-tools`-missing path prints no `STALE:` line; `xtask/src/dist.rs` compiles untouched.

### Step 4: Stale-only rebuild in `test_command`

- Task IDs: `TASK-341`
- Objective: add `build_stale_command` and route `test_command` through a testable seam so it rebuilds only stale specs and aborts on the infrastructure code without rebuilding.
- Precondition: Step 3 exit met.
- Postcondition: AC-9, AC-10 and AC-N4 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/test.rs` — only `test_command`, `ensure_pnp_cli_fresh` / `ensure_pnp_cli_fresh_with` (the seam precedent), and `mod tests`
  - `xtask/src/build_guests.rs` — only `build_command` and `build_one`'s signature
- Files allowed to edit (at most 3):
  - `xtask/src/test.rs`
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `xtask/src/main.rs`, `xtask/src/dist.rs`, `xtask/src/wit_verify.rs`
- Expected sub-agent dispatches:
  - Question: "Exact signature and body shape of `ensure_pnp_cli_fresh_with` and how its existing tests inject `run_rebuild`"; scope: `xtask/src/test.rs`; return: `SNIPPETS` (1 snippet, <=30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — locked decisions C6 and C9; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask test::tests::infrastructure_error_aborts_without_rebuilding -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask test::tests::test_command_rebuilds_only_the_stale_specs -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask test::tests::failed_stale_rebuild_aborts_the_suite -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
- Exit condition: the seam receives exactly the `CheckOutcome.stale` list; the infrastructure path invokes no rebuild at all; a failed stale rebuild aborts the suite with a non-zero code.

### Step 5: Fingerprint lifecycle and `v2-` content

- Task IDs: `TASK-341`
- Objective: move the sidecar write to after final verification, delete it at build start and on persistent failure, extend its content per R5-2, and bump the prefix to `v2-`.
- Precondition: Step 4 exit met.
- Postcondition: AC-11, AC-12 and AC-14 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — only `build_one`, the tail of `build_one_inner`, `fingerprint_entries`, `snapshot_from_paths`, `guest_input_paths`, `fingerprint_metadata_path`, and the existing test `fingerprint_is_deterministic_and_content_sensitive`
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/src/wit_verify.rs` (deletion of `module_stage_wit_dir` and its test only)
- Files explicitly out of bounds:
  - `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs`
- Blast-radius discipline (version constant): this step changes the sidecar version prefix from `v1-` to `v2-`. Before editing, run `rg -n '"v1-|v1-\{' xtask/src/` and add every test that asserts on the `v1-` literal to this step's edit list; all 42 guests invalidate exactly once, so this step also owns running `cargo xtask build-guests` to completion. Do not defer either to the acceptance ceremony.
- Expected sub-agent dispatches:
  - Question: "Every occurrence of the literal `v1-` and every test asserting on the fingerprint string in `xtask/src/`"; scope: `xtask/src/*.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — amended C5 and finding R5-2; direct read
  - `CLAUDE.md` — section "Guest WASM Staleness (MUST follow)"; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask build_guests::tests::fingerprint_is_written_only_after_final_verification -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask build_guests::tests::v2_fingerprint_covers_workspace_manifest_lockfile_rustc_and_wasm_tools -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `if rg -q 'module_stage_wit_dir' xtask/src/build_guests.rs xtask/src/wit_verify.rs || rg -qU 'canonical\.is_empty\(\)[\s\S]{0,120}return Ok' xtask/src/build_guests.rs; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL
  - `cargo xtask build-guests` then `cargo xtask build-guests --check; echo "exit=$?"` — FACT: the `exit=` line (expected `exit=0`)
- Exit condition: a failed verification leaves no sidecar; changing any one of the four new fingerprint inputs changes the fingerprint; the emitted string starts with `v2-`; `module_stage_wit_dir` and the `canonical.is_empty()` guard are gone; `--check` returns `exit=0` after a full rebuild.

### Step 6: Measure, record, and close

- Task IDs: `TASK-341`
- Objective: capture the before/after `--check` wall-clock, record it in this packet's `requirements.md`, add the backlog row, and run the closure gates.
- Precondition: Steps 1-5 exits met and all guests rebuilt (`cargo xtask build-guests --check` returns `exit=0`).
- Postcondition: AC-16 and AC-17 pass; all three `packet.spec.md` gate commands pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` — over 300 lines; delegate; read only the returned range around "Workstream 5 — Governance and closure drift"
  - `docs/spec_packets/230-output-based-guest-freshness/requirements.md` — the "Measured Freshness Timing" section
- Files allowed to edit (at most 3):
  - `docs/spec_packets/230-output-based-guest-freshness/requirements.md`
  - `docs/07_implementation_status.md`
  - `docs/spec_packets/230-output-based-guest-freshness/packet.spec.md` (status transition only)
- Files explicitly out of bounds:
  - every other packet directory under `docs/spec_packets/**`; all source files (this step writes no code)
- Expected sub-agent dispatches:
  - Question: "Timed `cargo xtask build-guests --check` with all guests fresh; return only the real/user/sys line"; scope: workspace; return: `FACT` (<=4 lines)
  - Question: "Line number of the `### Workstream 5 — Governance and closure drift` heading and the current highest `TASK-###` in `docs/07_implementation_status.md`"; scope: that file; return: `FACT` (<=3 lines)
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` — sections "No Unverified Metrics" and "Ledger Facts Must Be Re-derived, Not Quoted"; direct read. Re-derive the highest `TASK-###` at write time and renumber on collision.
- Verification:
  - `rg -q '^## Measured Freshness Timing' docs/spec_packets/230-output-based-guest-freshness/requirements.md && rg -q 'measured 20[0-9][0-9]-[0-9][0-9]-[0-9][0-9]' docs/spec_packets/230-output-based-guest-freshness/requirements.md && echo PASS || echo FAIL` — FACT PASS/FAIL
  - `rg -q 'TASK-341 ' docs/07_implementation_status.md && echo PASS || echo FAIL` — FACT PASS/FAIL
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask 2>&1 | tee target/test-output.log | rg '^test result:'` — FACT pass/fail
- Exit condition: both timing figures are recorded with the `measured <YYYY-MM-DD>` tag and the exact command; no unmeasured figure (including the plan's earlier `~38ms`/`~2s`) appears anywhere in the packet; the `TASK-341` row exists; every pipe-suffixed AC returns PASS.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Resolution only; `STAGES` and two artifact decodes by bounded FACT |
| Step 2 | M | `StaleReason` + `CheckContext` + the R5-4 cross-check and its fixtures |
| Step 3 | M | API shape change plus both call sites; three files edited |
| Step 4 | S | One seam mirroring an existing precedent |
| Step 5 | M | Fingerprint lifecycle, content, `v1-`→`v2-` blast radius, one full rebuild |
| Step 6 | S | Measurement, backlog row, closure gates |

Aggregate: `M`. No step is L; no split required before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS.
- `cargo xtask build-guests --check` returns `exit=0` after the one-time `v2-` rebuild.
- The before/after `--check` timings are recorded in `requirements.md` §"Measured Freshness Timing" with the `measured <YYYY-MM-DD>` tag.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Report any rename of a net-new symbol relative to `design.md` §Code Change Surface, because packet 231 consumes `CheckOutcome`, `StaleReason`, `stale_reason`, `build_stale_command`, `FINGERPRINT_VERSION` and `EXIT_INFRA_ERROR` verbatim, and packet 232 documents the exit-code contract.
- Confirm `xtask/src/dist.rs` was not modified and `build_command` kept its signature.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and the three packet-level gate commands.
- Record remaining packet-local risk, specifically: any guest that reported `StageMismatch` or pre-existing drift and had to be rebuilt or investigated.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands use `--all-targets`. `cargo test -p xtask <name> -- --exact` targets the crate's inline test modules and is exempt, since `xtask` declares no separate test targets.
