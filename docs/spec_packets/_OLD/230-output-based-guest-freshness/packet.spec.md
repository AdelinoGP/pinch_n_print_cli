---
status: implemented
packet: 230-output-based-guest-freshness
task_ids:
  - TASK-341
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 230-output-based-guest-freshness

## Goal

Wire packet 229's artifact verifier into `cargo xtask build-guests --check` so guest WIT staleness is answered by decoding each artifact — with the stage resolved from the artifact and cross-checked against the core guest's manifest `[stage] id` — while `check_command` returns the stale list, `test_command` rebuilds only stale guests, `wasm-tools`-missing is a distinct infrastructure exit code, and the `v2-` fingerprint is written only after final verification succeeds.

## Scope Boundaries

This packet owns artifact-based stage resolution, the `check_command` / `build_stale_command` / `test_command` API and its two call sites, the exit-code-based reporting contract, and the fingerprint lifecycle and content (`v2-` prefix, plus workspace-root `Cargo.toml`, the guest's own `Cargo.lock`, `rustc -vV` and the `wasm-tools --version` string). It does **not** perform the dependency-closure walk, does **not** delete `compute_shared_freshness` or `stage_wit_snapshot`, and does **not** touch `xtask/src/dist.rs` — those belong to packet 231.

## Prerequisites and Blockers

- Depends on: `docs/spec_packets/229-wit-verify-declaration-model` (`status: draft` at authoring time — this is a **FORWARD-DEP**, not a satisfied dependency). This packet consumes exactly 16 items from `xtask/src/wit_verify.rs`: `WorldModel`, `PackageModel`, `InterfaceModel`, `StageExpectation`, `stage_expectation`, `Drift`, `DriftKind`, `SHARED_PACKAGES`, `ROOT_COMPONENT_PACKAGE`, `canonical_world_model`, `embedded_world_model`, `compare_worlds`, `verify_embedded_world`, `embedded_wit_text`, and the `VerifyError` variants `Decode` / `Parse` / `CanonicalEmpty` / `CanonicalUnreadable`. Packet 229's `packet.spec.md` §Prerequisites names the same 16 and its `design.md` §Code Change Surface carries their signatures. This packet must not start until 229 is `status: implemented`.
- Unblocks: `docs/spec_packets/231-guest-closure-fingerprint` (consumes `CheckOutcome`, `StaleReason`, `stale_reason`, `build_stale_command`, `FINGERPRINT_VERSION`, `EXIT_INFRA_ERROR`).
- Activation blockers: packet 229 not yet `implemented`.

## Acceptance Criteria

- **AC-1. Given** a decoded artifact's `WorldModel`, **when** `resolve_stage_from_world` runs, **then** it matches the single non-shared, non-`root:component` package against `slicer_schema::STAGES` on the version-stripped `wit_package` and returns the corresponding `StageExpectation`. | `mkdir -p target && cargo test -p xtask wit_verify::tests::stage_resolves_from_embedded_package_name -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-2. Given** `STAGES` contains the `PrePass::PaintSegmentation` row whose `wit_package` is the empty string, **when** stage resolution scans `STAGES`, **then** that row is excluded, so an embedded package name of `""` can never resolve to a stage. | `mkdir -p target && cargo test -p xtask wit_verify::tests::empty_wit_package_stage_row_is_excluded_from_resolution -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-3. Given** an embedded world with zero candidate stage packages, with two, or with one that matches no `STAGES` row, **when** `resolve_stage_from_world` runs, **then** it returns `Err` with `StageResolutionError::NoStagePackage`, `::Ambiguous` and `::UnknownPackage` respectively, and the guest is reported stale in each case. | `mkdir -p target && cargo test -p xtask wit_verify::tests::zero_multiple_and_unknown_stage_packages_are_unresolvable -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-4. Given** a core guest whose `GuestSpec.stage_id` (parsed from the sibling module manifest's `[stage] id` by `parse_stage_id_from_module_manifest`) is `Layer::Infill` but whose artifact resolves to `Layer::Support`, **when** `stale_reason` runs, **then** it returns `Some(StaleReason::StageMismatch { expected: "Layer::Infill", resolved: "Layer::Support" })` — the manifest stage remains an independent expectation, so the check is never self-referential. | `mkdir -p target && cargo test -p xtask build_guests::tests::core_guest_artifact_stage_must_equal_manifest_stage_id -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-5. Given** a test guest, which has no module manifest and therefore `GuestSpec.stage_id == None`, **when** `stale_reason` runs, **then** artifact-derived resolution is the sole resolver and no `StageMismatch` is produced. | `mkdir -p target && cargo test -p xtask build_guests::tests::test_guest_stage_comes_from_the_artifact_alone -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-6. Given** a guest whose artifact drifts from canonical, **when** `check_command` runs, **then** stdout carries exactly one line `STALE: <crate_name>` for that guest and the drift reason on a following line that does **not** contain the substring `STALE:`, and the returned `CheckOutcome.code` is `1`. | `mkdir -p target && cargo test -p xtask build_guests::tests::stale_report_is_one_marker_line_plus_a_markerless_reason -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-7. Given** every guest is fresh, **when** `check_command` runs, **then** `CheckOutcome.stale` is empty and `CheckOutcome.code` is `0`. | `mkdir -p target && cargo test -p xtask build_guests::tests::all_fresh_yields_empty_stale_list_and_zero_code -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-8. Given** `wasm-tools` cannot be resolved on `PATH`, **when** `check_command` runs, **then** it returns `CheckOutcome.code == EXIT_INFRA_ERROR` (a non-zero value distinct from both `0` and `1`), `CheckOutcome.stale` is empty, and no `STALE:` line is printed — the condition is an infrastructure error, never staleness and never freshness. | `mkdir -p target && cargo test -p xtask build_guests::tests::missing_wasm_tools_is_infrastructure_error_not_staleness -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-9. Given** `check_command` returned `EXIT_INFRA_ERROR`, **when** `test_command` handles it, **then** it aborts and returns that same code **without** invoking any rebuild. | `mkdir -p target && cargo test -p xtask test::tests::infrastructure_error_aborts_without_rebuilding -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-10. Given** `check_command` returned a non-empty stale list, **when** `test_command` rebuilds, **then** it calls `build_stale_command` with exactly that list and rebuilds no other guest. | `mkdir -p target && cargo test -p xtask test::tests::test_command_rebuilds_only_the_stale_specs -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-11. Given** a guest build whose final verification fails, **when** `build_one` returns, **then** no fingerprint sidecar exists at `target/guest-fingerprints/<crate_name>.fingerprint` — the sidecar is removed at build start and on every persistent failure (`EmbeddedWorldUndecodable`, `StaleEmbeddedWorld`), and written only after the final verification succeeds. | `mkdir -p target && cargo test -p xtask build_guests::tests::fingerprint_is_written_only_after_final_verification -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-12. Given** the fingerprint content, **when** any one of the workspace-root `Cargo.toml`, the guest's own `Cargo.lock`, the `rustc -vV` output string, or the `wasm-tools --version` string changes, **then** the computed fingerprint changes, and the emitted string starts with the prefix `v2-`. | `mkdir -p target && cargo test -p xtask build_guests::tests::v2_fingerprint_covers_workspace_manifest_lockfile_rustc_and_wasm_tools -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-13. Given** the migrated API, **when** the two call sites are inspected, **then** `xtask/src/main.rs`'s `Some("--check")` arm exits with `check_command(&ws).code`, and `xtask/src/test.rs` binds the returned `CheckOutcome` — reading `.stale` and passing it to `build_stale_command` — with the pre-migration bare-`i32` binding `let check_code = build_guests::check_command(ws_root);` gone. | `if rg -q 'check_command\(&ws\)\.code' xtask/src/main.rs && rg -q '\.stale' xtask/src/test.rs && rg -q 'build_stale_command' xtask/src/test.rs && ! rg -q 'let check_code' xtask/src/test.rs; then echo PASS; else echo FAIL; fi`
- **AC-14. Given** `xtask/src/build_guests.rs` and `xtask/src/wit_verify.rs` after this packet, **when** the retired symbols are searched for, **then** `module_stage_wit_dir` is absent from both files and `build_one` contains no `canonical.is_empty()` early-return. | `if rg -q 'module_stage_wit_dir' xtask/src/build_guests.rs xtask/src/wit_verify.rs || rg -qU 'canonical\.is_empty\(\)[\s\S]{0,120}return Ok' xtask/src/build_guests.rs; then echo FAIL; else echo PASS; fi`
- **AC-15. Given** `GuestSpec` after this packet, **when** its fields are inspected, **then** `stage_id: Option<String>` still exists and `parse_stage_id_from_module_manifest` is still called by `discover_guests` — the manifest-derived expectation survives (R5-4). | `rg -q 'pub stage_id: Option<String>' xtask/src/build_guests.rs && rg -q 'parse_stage_id_from_module_manifest' xtask/src/build_guests.rs && echo PASS || echo FAIL`
- **AC-16. Given** the migration is complete, **when** the implementer times `cargo xtask build-guests --check` on this machine after the change, **then** the after wall-clock figure and the exact timing command are recorded in this packet's `requirements.md` under the heading "Measured Freshness Timing", the before figure is either recorded or its non-capture is explicitly documented with rationale, and no unmeasured figure is quoted anywhere in the packet. | `rg -q '^## Measured Freshness Timing' docs/spec_packets/230-output-based-guest-freshness/requirements.md && rg -q 'measured 20[0-9][0-9]-[0-9][0-9]-[0-9][0-9]' docs/spec_packets/230-output-based-guest-freshness/requirements.md && rg -q '\*\*Before\*\*' docs/spec_packets/230-output-based-guest-freshness/requirements.md && rg -q '\*\*After\*\*' docs/spec_packets/230-output-based-guest-freshness/requirements.md && echo PASS || echo FAIL`
- **AC-17. Given** `docs/07_implementation_status.md`, **when** the packet closes, **then** a `TASK-341` row exists under "Workstream 5 — Governance and closure drift". | `rg -q 'TASK-341 ' docs/07_implementation_status.md && echo PASS || echo FAIL`
- **AC-18. Given** the retained convenience predicate, **when** `is_stale` is inspected, **then** its signature is `pub fn is_stale(spec: &GuestSpec, ws_root: &Path, ctx: &CheckContext) -> bool` — it no longer takes `&FreshnessSnapshot` — and its body is exactly `stale_reason(spec, ws_root, ctx).is_some()`, so the two predicates can never disagree. | `mkdir -p target && cargo test -p xtask build_guests::tests::is_stale_delegates_to_stale_reason -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`

## Negative Test Cases

- **AC-N1. Given** an artifact that `wasm-tools` cannot decode, **when** `stale_reason` runs, **then** it returns `Some(StaleReason::Undecodable(_))` — the guest is stale, and the undecodable artifact is never reported fresh. | `mkdir -p target && cargo test -p xtask build_guests::tests::undecodable_artifact_is_stale -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-N2. Given** a canonical WIT tree that `canonical_world_model` reports as empty or unreadable, **when** `check_command` runs, **then** it returns `EXIT_INFRA_ERROR` and prints no `STALE:` line — the retired `if canonical.is_empty() { return Ok(()) }` fail-open must not be reachable in any form. | `mkdir -p target && cargo test -p xtask build_guests::tests::unusable_canonical_set_is_infrastructure_error_not_fresh -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-N3. Given** a guest whose artifact file does not exist at all, **when** `stale_reason` runs, **then** it returns `Some(StaleReason::ArtifactMissing)` without attempting a decode, and for a core guest the manifest `[stage] id` is still the resolver used for the subsequent rebuild. | `mkdir -p target && cargo test -p xtask build_guests::tests::never_built_guest_is_stale_via_manifest_stage -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-N4. Given** a stale guest whose rebuild fails, **when** `build_stale_command` runs, **then** it returns non-zero and `test_command` aborts the suite rather than proceeding to run tests against a known-stale artifact. | `mkdir -p target && cargo test -p xtask test::tests::failed_stale_rebuild_aborts_the_suite -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p xtask 2>&1 | tee target/test-output.log | rg '^test result:'`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — section "Build & Freshness Contract (Normative)" and its staleness-guard table row; direct ranged read. This packet changes the behavior the section describes; the section's **text** is rewritten by packet 232, not here.
- `docs/07_implementation_status.md` — over 300 lines; delegate. Only the "Workstream 5 — Governance and closure drift" section is edited, to add the `TASK-341` row.
- `CLAUDE.md` — sections "Guest WASM Staleness", "No Unverified Metrics", "Test Discipline"; direct read.
- `docs/specs/guest-freshness-artifact-verification-plan.md` — locked decisions 1, 4, 5, 6, 9, 10 and 11 (the plan's §"Locked decisions" numbered list; only items 1-3 and 13 carry a literal `C#` label in its prose, so cite them by ordinal) as amended by Round 5 findings R5-2, R5-3, R5-4, R5-7. **This plan file is untracked in git at authoring time — it must be committed together with these packet directories, per its own commit rule.**

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` section "Workstream 5 — Governance and closure drift" — add the `TASK-341` row: `rg -q 'TASK-341 ' docs/07_implementation_status.md`

The normative freshness-contract prose in `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `CLAUDE.md` and the `wasm-staleness` snippet is deliberately **not** rewritten here; packet 232 owns that surface in one coherent pass, and splitting it across two packets would leave the docs internally inconsistent between them. A non-normative pointer note was added under `docs/03_wit_and_manifest.md`'s "Build & Freshness Contract (Normative)" heading (post-review, 2026-08-20) flagging that the prose below it describes the pre-packet contract; packet 232 replaces the section and the note together.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
