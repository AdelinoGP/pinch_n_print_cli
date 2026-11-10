---
packet: proptest-openrscad_01_pbt-foundation
status: draft
---

# Opt-in property harness and an executed IR vertical slice

## Goal

Establish the default-off SDK property harness and registry-backed `cargo xtask test` selection, and prove them with an independently-oracled travel-anchor property, failure probes, and a required Windows routine lane.

## Why

The approved source is `docs/specs/proptest-openrscad-adoption.md`, especially §§2, 4–5, 7, 12 and G-BASE/G-HARNESS. The tree has no Proptest dependency or property feature. `test_command` (`xtask/src/test.rs`) already owns literal/test-quality preflights, guest freshness/rebuild, CLI freshness, logging and summaries. Its `build_test_arguments` deliberately omits automatic `host-algos` injection when callers specify features; a new selector cannot rely on that fallback.

`validate_travel_anchors` (`crates/slicer-ir/src/validation.rs`) is a useful first correctness seam: its documented result is membership in `LayerCollectionIR.ordered_entities`, and errors identify a missing `TravelMove.entity_id`. It allows an independent set oracle without geometry, canonical ports or a guest campaign. This packet establishes infrastructure, not completion of the H-IR family.

## Scope

- In: workspace-managed Proptest on the inspected `1.11` API line after MSRV/package recheck; SDK `property_testing`; per-case RAII; measured profiles, fresh sampling/replay, target-local diagnostics; typed property registry; initial selectors/discovery/execution checks; one IR property; controlled failure subprocesses; incremental Windows PR execution; contributor documentation/baseline evidence; focused canonical-correct fixes with named concrete regressions if the first property exposes a confirmed SUT defect.
- Out: unrelated production algorithm changes, new IR fields/schema/WIT versions, geometry strategies, OpenRSCAD or SCAD features/engine builds, the remaining H-IR obligations, a new testing framework, general guest/E2E randomization, scheduled jobs, and final rollout/workspace acceptance. A larger confirmed defect becomes explicit blocking fix work for this slice, never a weakened property or an ignored acceptance failure.

## Depends on

None. The approved plan is an input, not implemented prerequisite code. Authoring checks are not G-BASE or G-HARNESS execution evidence.

## Design notes

### Grounded seams and dependency isolation

- `slicer-sdk` currently exposes `test = []`, a self dev-dependency enabling `test`, and `test_support` gated by `any(test, feature = "test")` (`crates/slicer-sdk/Cargo.toml`, `crates/slicer-sdk/src/lib.rs`). Add **new** `property_testing = ["test", "dep:proptest", "dep:serde_json"]`, with optional Proptest/serde_json dependencies and existing serde support for context/artifacts. Do not add `property_testing` to `test`, a default feature, the SDK self dev-dependency, or any guest manifest. Retain Proptest `std`, `fork` and `timeout` capabilities; not every profile uses them.
- Add a **new** empty `slicer-ir/property_testing` test-selection feature and **new** `property_ir` integration target with `required-features = ["property_testing"]`. Its `proptest` and `slicer-sdk` with `features = ["property_testing"]` edges are dev-dependencies only. This deliberately opts the owning IR test graph into shared support; it must not create any normal-dependency cycle or production Proptest edge. Measure the dev-graph cost rather than claiming it is free. `slicer-ir`'s existing features are `default = []` and `std = []`.
- ADR-0004 (`docs/adr/0004-test-support-lives-in-slicer-sdk.md`), ADR-0016 (`docs/adr/0016-test-prelude-whole-module-gate.md`) and the fixture-home amendment to ADR-0054 (`docs/adr/0054-host-side-test-support-crate.md`) retain one SDK fixture home. Do not reuse or expand the binary-locator-only `pnp-cli-locator` surface. No new ADR or deviation identifier is reserved.
- Verify normal/build dependency edges separately from test/dev edges: default `pnp-cli`, default SDK, SDK `test` alone, and actual discovered guest workspaces remain Proptest/OpenRSCAD-free. At this point the SCAD-enabled graph is a documented future expectation, not an executable build configuration.

### New SDK exports: stable handoff contract

All names below are **new**, in `slicer_sdk::test_support::property_testing`, crate `slicer-sdk`, implemented under the **new** `crates/slicer-sdk/src/test_support/property_testing/` directory. Gate the whole module with `all(feature = "property_testing", not(target_arch = "wasm32"))`, following ADR-0016 rather than scattering per-item gates. Property support is not a guest export. Do not change production visibility to expose private kernels.

```rust
pub struct PropertyRun { /* private fields */ }
pub struct PropertyRunError { /* private fields */ }
// PropertyRunError: Debug + Display + std::error::Error.
impl PropertyRun {
    pub fn new(property_id: &'static str, test_name: &'static str)
        -> Result<Self, PropertyRunError>;
    pub fn bound(&self, name: &str) -> Result<u64, PropertyRunError>;
    pub fn check<S, F>(&self, strategy: &S, test: F)
        -> Result<(), PropertyRunError>
    where
        S: proptest::strategy::Strategy,
        F: Fn(S::Value, &mut CaseGuard)
            -> proptest::test_runner::TestCaseResult;
}
pub struct CaseGuard { /* private fields; !Send + !Sync */ }
impl CaseGuard {
    pub fn new() -> Self;
    pub fn install_mock(&mut self, host: crate::test_support::mock_host::MockHost);
    pub fn entity_ids(&self) -> &slicer_ir::LayerEntityIdGen;
    pub fn infill(&mut self) -> &mut crate::test_support::capture::InfillOutputCapture;
    pub fn perimeters(&mut self) -> &mut crate::test_support::capture::PerimeterOutputCapture;
    pub fn support(&mut self) -> &mut crate::test_support::capture::SupportOutputCapture;
}
// CaseGuard implements Drop. PropertyRun::check creates/drops it for EVERY
// generated invocation and shrink invocation, not once per Rust test.
```

`PropertyRun::new` validates the selected context and identity. Missing context fails with an actionable instruction to use the registry-backed xtask command; it does not silently choose an unmeasured profile. `bound` rejects absent names/invalid values instead of defaulting to an empty domain. The actual libtest name and the fully qualified Proptest `Config.test_name` are distinct: construct/verify the latter from the caller's fully qualified name, with a fork probe before claiming process isolation works.

`CaseGuard::new` clears the current thread's log capture and mesh source, installs an empty log sink, and creates fresh captures and `LayerEntityIdGen`. It uses `reset_global_state`, `mock_host_setup`, `mock_host_teardown` (`crates/slicer-sdk/src/test_support/mod.rs`) and the existing `MockHost::install` seam (`crates/slicer-sdk/src/test_support/mock_host.rs`). Destruction first retains available case logs for diagnostics, then clears state on return, `TestCaseError`, and unwind, without panicking; teardown must not discard the only failure log. Same-thread nested guards are unsupported and rejected before mutating the active guard's state. No per-case `install_panic_handler`, `take_hook`, or `set_hook`; `generate_module_test_impl` (`crates/slicer-macros/src/lib.rs`) remains a once-per-test harness, not a substitute for case isolation. Test log isolation through real `host::log_warn`/`take_log_messages` and mesh isolation through real `host::raycast_z_down`, not mock-only counters.

### New registry and profile/configuration shape

The **new** root `test-properties.toml` is the central executable registry. **New** `xtask/src/properties.rs` parses it into typed records and validates it before spawning Cargo. Register it in `xtask/src/main.rs`; xtask is bin-only, so runner tests live in registered internal modules, not an imagined xtask library target. Later packets append records to this schema, not a second roster.

| Scope | Required keys and exact value kinds |
|---|---|
| Root | `schema_version = 1`; `property` is an array of tables. |
| Property | `id: string` matching `[A-Z][A-Z0-9-]*`; `package: string`; `target_kind: "test" | "lib"`; `target_name: string` equal to Cargo metadata target name; `test_name: string` equal to full libtest-discovered name, with final component starting `proptest_`; `production_symbol: string`; `production_path: string`; `features: array<string>` of Cargo feature specifications; `requires_host_algos: bool`; `requires_scad: bool`; `generator_version: positive integer`; `charter: workspace-relative Markdown path plus heading anchor`; `routine` and/or `extended` profile tables. |
| Each profile | `cases: positive u32`; `max_local_rejects: u32`; `max_global_rejects: u32`; `max_flat_map_regens: u32`; `max_default_size_range: positive usize`; `max_shrink_iters: positive u32`; `max_shrink_time_ms: u32`; `rng_algorithm: "chacha"` (maps to Proptest `RngAlgorithm::ChaCha`); `isolation: "in_process" | "fork"`; `timeout_ms: u32`; `bounds: table<string, u64>`. A nonzero timeout requires process isolation. Profiles are accepted only after pilots; authoring assigns no runtime budget. |

The first record is `id = "H-IR-ANCHORS"`, `package = "slicer-ir"`, `target_kind = "test"`, `target_name = "property_ir"`, `test_name = "proptest_ir_travel_anchors_membership"`, `production_symbol = "slicer_ir::validate_travel_anchors"`, `production_path = "crates/slicer-ir/src/validation.rs"`, `features = ["slicer-ir/property_testing"]`, `requires_host_algos = false`, `requires_scad = false`, `generator_version = 1`, `charter = "docs/testing/properties.md#h-ir-anchors"`. Its bounds keys are exactly `max_entities` and `max_travels`; measured values must permit empty, singleton, repeated-anchor, missing-anchor and reordered cases. Both routine and extended profiles are required.

The typed parser rejects duplicate IDs/test identities, unknown keys, nonexistent package/target/feature claims, missing profiles, unsafe path components and contradictory isolation. Do not infer activation from a directory existing. Explicitly merge features per selected child command. For a selected `slicer-core` target requiring host algorithms, retain `slicer-core/host-algos` even with explicit SDK/SCAD features; do not append that qualified feature to an unrelated package such as xtask. Native SDK dependencies already enable `host-algos`, which metadata can verify. Synthetic registry fixtures prove the future SCAD-feature composition without claiming a real SCAD property exists yet.

### New command, context and evidence contract

```text
cargo xtask test --summary --properties --tier routine
cargo xtask test --summary --properties --tier extended
cargo xtask test --summary --properties --property H-IR-ANCHORS --seed 42
```

- `--properties` selects registry execution; `--tier routine|extended` defaults to routine; repeatable `--property ID` restricts selection; `--seed U64` replays one recorded concrete seed (decimal u64). All properties in a selected run receive that recorded seed; their identity/configuration is recorded independently. Fresh OS-entropy-backed seed resolution is the default, before spawning cases. Do not claim different random draws must always produce different seeds.
- `--with-scad` is parsed/reserved here: without it select only `requires_scad = false`; with it include eligible core and SCAD records and require at least one eligible SCAD record. Until the scripted packet registers one, return usage exit `2` with `no_eligible_scad_properties`. This is an honest unavailable selector, not SCAD acceptance.
- Property-only flags require `--properties`. Unknown IDs/tiers, malformed seeds, empty selections, selectors with `--summary-from`, and property mode combined with ordinary package/test/filter/`--no-run`/`--list`/libtest selectors are usage errors (`2`). Reject `--properties --accelerated` initially with a named conflict rather than silently running a different profile. Preserve all ordinary non-property passthrough and accelerated behavior, including existing `caller_feature_choice_is_not_overridden` (`xtask/src/test.rs`).
- Discovery uses each registry record's feature-enabled package/target selection, verifies the exact test exists and is not ignored, then executes that exact test with `--exact --nocapture`. An unrelated passing test, an ignored test, a compile-only run, zero generated cases or missing completion evidence is not success. Compare discovered `proptest_` names against registry entries in both directions within selected targets; no fixed binary count.
- Reuse the real `test_command` preflight/guest/CLI gate implementations once for the selected run. Route the real command through the new executor; test-only helper coverage is insufficient. Preserve `--summary`, ordinary summaries and `--summary-from`. Append all discovery/execution child stdout/stderr and delimiters into one `target/test-output.log`, retaining earlier child failures. Do not recursively invoke xtask or truncate the log for each target. A log write failure is a failed run.
- Set **new** `PNP_PROPERTY_CONTEXT` to one JSON object per selected child. Schema: `schema_version: 1`, `run_token: string`, `property_id: string`, `tier: "routine" | "extended"`, `seed: u64`, `package: string`, `target_kind: string`, `target_name: string`, `test_name: string`, `features: array<string>`, `profile: object` with the profile keys above, `artifact_dir: string`, `proptest_version: string`, `generator_version: positive integer`, `source_revision: string`, `dirty_paths: array<string>`, `source_state_digest: string`, `cargo_target_dir: string | null`, `completion_eligible: bool`. Paths are serialized, never shell-interpolated. Context values must agree with the selected record. Runtime bounds are available through `PropertyRun::bound`. Save a source-state record/digest covering the revision, tracked diff and relevant untracked source/manifests/config bytes, not merely a list of dirty names; changing dirty contents must change the digest.
- Resolve `Config` intentionally from that context, including seed/algorithm and measured rejection/shrink/isolation settings. Disable source-adjacent seed persistence (`failure_persistence = None` is sufficient); use target-local diagnostics. Reject conflicting upstream sampling/size/rejection/shrink/seed/RNG/fork/timeout controls (including `PROPTEST_CASES`, `PROPTEST_RNG_ALGORITHM` and `PROPTEST_MAX_DEFAULT_SIZE_RANGE`) with `profile_override_conflict`, rather than silently ignoring them or accepting below-profile completion evidence. Logging-only overrides may be allowed and recorded. Direct Proptest consumers outside this harness retain upstream behavior.
- SDK events are newline-delimited `PNP_PROPERTY_EVENT ` followed by JSON. Required envelope keys: `schema_version = 1`, `run_token`, `property_id`, `phase = "begin" | "end"`, `process_role = "controller" | "worker"`. End includes `outcome = "passed" | "failed" | "exhausted"`, `accepted_cases: u64 | null`, `accepted_test_calls: u64`, `rejected_test_calls: u64`, `runner_statistics: string`, `shrink_completed: bool | null`, `completion_eligible: bool`. Success requires a matching controller begin/end, `outcome = "passed"`, accepted cases at least profile `cases`, exact libtest success, and successful child status. Fork-worker re-entry/events must not impersonate or duplicate controller completion. Retain process-tagged case observations when counting forked callbacks; in-memory counters in the parent alone cannot observe child execution. Do not fabricate private Proptest counters: call counters include shrink invocations; `accepted_cases` on unsuccessful runs may be null; retain the actual runner statistics/limits and label unavailable counts. Distinguish exhausted/rejected runs from assertion failures. Do not claim minimization completed when it hit a limit or when completion cannot be established.
- Save `context.json` before execution and a per-attempt `case.txt` before the SUT callback, then `failure.json` when possible, under `target/property-testing/<run_token>/<property_id>/`. Failure schema includes classification (`sut_or_contract`, `generator_or_oracle`, `exhausted`, `panic`, `process_failure`, `harness`), context, concrete/debug input, reproduction command, captured SDK logs, and shrink status. Classification may initially be `sut_or_contract` pending investigation; do not label an unexplained assertion a confirmed production defect. The later domains packet supplies reusable concrete Rust fixture formatting. Seeds are not committed regression history.
- Child failure wins over missing evidence. Preserve ordinary exit codes in `1..=255`; record platform-native status in full, and map an otherwise unrepresentable nonzero Windows status/abnormal termination to exit `1`, never through a wrapping cast to success. Missing/zero execution with otherwise successful child returns `1` and `missing_property_execution`; a controlled child exit `17` remains `17`. Usage errors return `2` before running gates.

### Vertical-slice charter

The **new** `proptest_ir_travel_anchors_membership` lives in the owning crate at **new** `crates/slicer-ir/tests/property_ir.rs`. It calls the actual `validate_travel_anchors` every case through `PropertyRun::check`. This is a library-validator property, not a claim that the runtime currently calls the validator. Generate bounded lists of entity IDs and anchors, not just already-valid cases. Build entities with FRU over `print_entity_base` (`crates/slicer-sdk/src/test_support/fixtures.rs`), and layers/travels with `Default` plus the fields under test. Empty paths/other layer fields are immaterial to this validator and are not certified as printable geometry.

The independent oracle sorts/deduplicates a vector or uses an ordered set of the generated `ordered_entities` IDs; it does not call the production validator, reuse its `HashSet` helper or look up entities outside `ordered_entities`. Expected result: `Ok` iff every anchor belongs; otherwise the error contains `entity_id`, the first missing ID in travel order, and `layer.ordered_entities`. Integer membership is exact. Include deterministic empty/valid/dangling witnesses, repeated anchors, reordered entities and IDs `0`/`u64::MAX`; this validator is not an allocator/sentinel validator. Shrinking preserves the selected valid/dangling mode where a mode-specific assertion requires it; do not filter invalid membership out of the property. A synthetic faulty accept-all validator and an oracle-independent missing-ID witness must be detected by a controlled probe, not registered as a passing production property.

### G-BASE / G-HARNESS evidence and Windows execution

Implementation creates **new** `docs/testing/proptest-foundation-evidence.md` with the actual revision, dirty paths, toolchain/OS, features/targets, Proptest resolution/MSRV, `CARGO_TARGET_DIR`, commands, full-log/artifact locations and measured compilation/execution context. Run the existing IR validation/entity-ID tests and SDK isolation/log tests before edits; record pass/fail from their logs, not their stale comments. Measure pilots before committing routine/extended counts and bounds to the registry. Record dependency graphs before/after, including discovered guest normal/build graphs. A default workspace build must not acquire an engine simply because a future adapter becomes a member.

Add **new** `jobs.properties-windows` to `.github/workflows/ci.yml` on `windows-latest`, using the checked-in toolchain policy, wasm target/wasm-tools prerequisites for existing gates, and `cargo xtask test --summary --properties --tier routine`. Upload `target/test-output.log` and `target/property-testing/` on failure. Keep current Ubuntu jobs intact. Routine acceptance requires an actual Windows run plus required PR job wiring; local source inspection or upstream build claims do not satisfy it. Extended is manually reproducible and must pass for this packet's closure; final manual-dispatch UX belongs to the closure packet.

<!-- snippet: wasm-staleness -->
- Feeds guest WASM: run `cargo xtask build-guests --check` after edits and judge by exit code (0 fresh, 1 stale, 3 infrastructure error) before interpreting any guest test failure (AGENTS.md §Guest WASM Staleness).

## Acceptance criteria

All test names/targets introduced below are **new planned tests**, except existing baseline targets explicitly identified in Verification. Narrow command aliases are fully defined there; they preserve status and log output.

- **AC-1** `property_testing` implies SDK `test` and optional Proptest with fork/timeout capability, while normal/build graphs for default `pnp-cli`, SDK default/`test`, and discovered guests contain neither `proptest` nor OpenRSCAD packages. `property_dependency_isolation` under `X`; actual `cargo tree -e normal,build,features` output and MSRV/lock resolution are retained in G-BASE evidence. An executable graph checker must read Cargo metadata, not assert source text.
- **AC-2** `case_guard_cleans_success_failure_and_unwind`, `case_guard_isolates_concurrent_threads` and `case_guard_rejects_nested_use_before_mutation` under `S` observe real logs/mesh wrappers, captures and the first fresh entity ID across consecutive cases, returned failures, deliberate panic and concurrent threads. The old once-per-test macro is not the tested reset boundary.
- **AC-3** `property_registry_validates_and_discovers_exact_tests` under `X` validates the stated registry schema, derives feature-enabled targets from Cargo metadata, and detects unregistered discovered `proptest_` tests in selected targets. The initial record is exactly `H-IR-ANCHORS`/`slicer-ir`/`property_ir`/`proptest_ir_travel_anchors_membership`.
- **AC-4** `cargo xtask test --summary --properties --property H-IR-ANCHORS --tier routine` executes the real validator through the independent charter and emits matching begin/end evidence, exact test success and `accepted_cases >= cases`. Deterministic empty/valid/dangling/extreme-ID witnesses execute; a synthetic accept-all mutation is rejected by `property_anchor_oracle_catches_accept_all` under `X`.
- **AC-5** `property_seed_replay_and_artifacts` under `X` launches a real controlled child twice with the same seed/config/source and compares the concrete pre-shrink case sequence and minimized/retained failure, not only the seed string. The replay probe uses iteration-bounded shrinking without an elapsed-time cutoff so it does not assert wall-clock-deterministic minimization. `context.json`, `case.txt`, `failure.json`, feature/profile/version/source provenance and a working reproduction command exist under the specified target directory; no source-adjacent seed file is created.
- **AC-6** `property_feature_union_preserves_host_algos` under `X` demonstrates owner-scoped merging with explicit SDK features and a synthetic SCAD record while retaining required `slicer-core/host-algos`. The existing ordinary-passthrough feature-choice tests still pass. No real SCAD-enabled run is claimed.
- **AC-7** `property_executor_appends_logs_and_preserves_child_status` under `X` drives the same executor reached by `test_command`, using successful and deliberately failing subprocesses: both children remain in the combined log, exit `17` is retained, abnormal native status cannot wrap to zero, and log-write failure fails. Existing gate injection tests still exercise actual shared gate implementations.
- **AC-8** G-BASE/G-HARNESS evidence contains measured routine/extended pilot settings, an actual passing Windows routine and extended run of `H-IR-ANCHORS`, before/after graph evidence, actual existing-test results, and the required `properties-windows` job/artifact wiring. No unmeasured performance or SCAD-feasibility claims appear.
- **AC-N1** `property_selector_rejects_conflicts_and_unknown_ids` under `X` checks unknown ID/tier/seed, property flags without `--properties`, `--summary-from` conflicts, selection/compile-only/filter conflicts, accelerated conflict, empty selection and `--with-scad` without a SCAD entry. They return `2`, with `no_eligible_scad_properties` for the last case, before gates/children run.
- **AC-N2** `property_executor_rejects_missing_zero_ignored_and_mismatched_evidence` under `X` launches fixtures for missing feature-gated test, zero selected tests, ignored test, a passing unrelated test, begin-only, wrong property/token, zero accepted cases and successful process without the callback. Every case fails with `missing_property_execution` (exit `1`) rather than a green summary.
- **AC-N3** `property_profile_rejects_override_and_exhaustion` under `X`/`S` rejects context/identity/bound mismatches and below-profile environment overrides; a bounded all-rejected strategy reports `exhausted`, nonzero status and retained input/config, never `passed`. Controlled panic/fork failure probes leave readable artifacts and no leaked SDK thread state.
- **AC-N4** If a confirmed production defect is found, G-HARNESS evidence names its root cause, concrete previously failing input, focused fix, named regression destination and narrow passing command; the property and existing affected tests pass without relaxed membership checks. If none is observed, record that fact instead of manufacturing a regression. Larger blocking fix work leaves this slice incomplete.

## Verification

Authoring performed source/doc inspection and locked/offline Cargo metadata discovery only. All execution below is an **implementation acceptance obligation**, not a claim of a current pass.

For every plain Cargo test command below, run in Bash with `set -o pipefail`, ensure `target/` exists, and append `2>&1 | tee target/test-output.log`. Read/archive the file before the next run overwrites it. The xtask runner owns combined-log capture for its own children. PowerShell equivalents must preserve `$LASTEXITCODE` across `Tee-Object`.

- `X` means `cargo test -p xtask --bin xtask -- property_ --nocapture` with the capture wrapper above; tests added under `xtask/src/properties.rs` must be reached by the `mod properties` registration. Also run the existing `test::tests` module with `cargo test -p xtask --bin xtask -- test::tests --nocapture` using the same wrapper.
- `S` means `cargo test -p slicer-sdk --features property_testing --test property_harness -- --nocapture` with that wrapper; **new** `property_harness` is an explicitly `required-features = ["property_testing"]` target in `crates/slicer-sdk/Cargo.toml`.
- Before changes: `cargo test -p slicer-ir --test ir_validation_tdd` and `cargo test -p slicer-ir --test entity_id_invariants_tdd`, each captured separately; `cargo test -p slicer-sdk --features test --test test_support_mock_host_isolation_tdd` and `cargo test -p slicer-sdk --features test --test test_support_log_capture_round_trip_tdd`, each captured separately. These existing standalone targets have been confirmed in Cargo metadata; SDK targets require `test`, IR targets have no feature gate.
- After narrow harness tests: `cargo xtask test --summary --properties --tier routine`; `cargo xtask test --summary --properties --tier extended`; `cargo xtask test --summary --properties --property H-IR-ANCHORS --seed 42`. Read `target/test-output.log` and archive each selected-run artifact before the next run.
- `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo check -p slicer-sdk --all-targets --features property_testing`; `cargo clippy -p slicer-sdk --all-targets --features property_testing -- -D warnings`; `cargo check -p slicer-ir --all-targets --features property_testing`; `cargo clippy -p slicer-ir --all-targets --features property_testing -- -D warnings`.
- `cargo xtask check-literals`; `cargo xtask check-test-quality --report`; `cargo xtask build-guests --check` (rebuild on exit `1`, stop on `3`). No workspace test run is required by this packet.

## Docs impact

- `docs/05_module_sdk.md`, `## Test Support (slicer-sdk feature)`: document the native-only `property_testing` feature, per-case guard, private-kernel ownership and no per-case panic hooks. Check: `rg -n 'property_testing|CaseGuard' docs/05_module_sdk.md`.
- `docs/21_data_defaults_and_fixtures.md`, `## 5. Fixture policy`: add property support/FRU and concrete-regression-versus-seed guidance. Check: `rg -n 'property_testing|concrete regression' docs/21_data_defaults_and_fixtures.md`.
- **New** `docs/testing/properties.md`: registry schema, measured profiles, event/context schema, execution/replay/failure workflow and the complete `H-IR-ANCHORS` charter (all §5 source-plan fields). Check: `rg -n 'H-IR-ANCHORS|PNP_PROPERTY_CONTEXT|missing_property_execution' docs/testing/properties.md`.
- **New** `docs/testing/proptest-foundation-evidence.md`: G-BASE/G-HARNESS measured evidence and explicit deferred real-SCAD checks. Check: `rg -n 'G-BASE|G-HARNESS|source_revision|Windows' docs/testing/proptest-foundation-evidence.md`.
- `xtask/src/main.rs::USAGE`: add selectors/conflicts with a short link to the workflow. No AGENTS.md edit is needed in this packet; any later instruction edit must first load `writing-for-agents`.

## Open questions

- [FWD] Select the compatible Proptest `1.11` resolution and entropy/serialization dependencies after checking current metadata/MSRV; record the resulting lock and keep only property-feature dependencies optional. This is dependency verification, not permission to disable fork/timeout.
- [FWD] Measure first-property compile/execution/rejection/shrink pilots on Windows, then choose positive routine/extended counts and `max_entities`/`max_travels`; no numeric budget has been approved or measured during authoring.
- [FWD] Prove fork re-entry/test-name handling and truthful shrink-completion diagnostics using the pinned Proptest API; if the API cannot identify completion, report unknown/false rather than inventing counters. Hard native render containment remains a later scripted-property obligation.

## Authoring preflight

| # | Check | Result | Evidence / applicability |
|---|---|---|---|
| 1 | Symbols | PASS | Existing SDK host seams, fixture base, validator, macro and runner symbols read at the crate-qualified paths above; every addition is marked new. |
| 2 | WIT/IR identifiers | PASS | `ordered_entities`, `travel_moves`, `TravelMove.entity_id`, `PrintEntity.entity_id` checked against `crates/slicer-ir/src/slice_ir.rs`; no WIT change. |
| 3 | Schema versions | PASS | No IR/WIT version bump or pinned future version. Registry/context version is a new private test protocol. |
| 4 | ADRs | PASS | ADR-0004/0016 and ADR-0054 amendment inspected; no new slot or amendment required. |
| 5 | Deviation IDs | PASS | No deviation ID consumed, reserved, closed or created. |
| 6 | Prerequisites | PASS | No implementation prerequisite claimed. |
| 7 | Test targets | PASS | Existing baseline targets/features verified with locked/offline Cargo metadata; new standalone feature-gated targets and internal xtask module registration are explicit. |
| 8 | Criteria | PASS | Exact IDs/keys/errors/statuses, actual-execution evidence and negative process probes; no workspace criterion command. |
| 9 | Fallout | PASS | No production struct field/schema bump. New opaque helper internals have no existing literal sites; all test fixtures use existing bases/FRU. |
| 10 | Docs impact | PASS | SDK/fixture/workflow/evidence sections and check commands listed. |
| 11 | Completeness | PASS | No unresolved scope-changing premise, template placeholder or blocking question; measurements and dependency verification are implementation work. |

**Verdict: PREFLIGHT PASS (authoring only; G-BASE/G-HARNESS not executed).**
