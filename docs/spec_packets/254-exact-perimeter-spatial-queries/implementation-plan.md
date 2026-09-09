# Implementation Plan: exact-perimeter-spatial-queries

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-561`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Lock query contracts and adversarial fixtures
- Task IDs: `TASK-561`
- Objective: Define independent legacy-vs-indexed query records, exact float-bit assertions, synthetic cases, and feature-gated scoped controls.
- Precondition: Existing legacy symbols and fixture/literal rules are located by bounded dispatch.
- Postcondition: Core tests compile and FAIL-to-compile or assert-fail red against the not-yet-implemented indexed path; every selected edge case and nonvacuous counter is represented without production counters.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-core/src/perimeter_utils.rs` named symbols; `crates/slicer-ir/src/polygon_predicate.rs` named symbol; `docs/21_data_defaults_and_fixtures.md` relevant section.
- Files allowed to edit (at most 3): `crates/slicer-core/tests/perimeter_spatial_tdd.rs`; `crates/slicer-core/Cargo.toml` (adds the `perimeter_spatial_tdd` `[[test]]` entry and the `perimeter-spatial-test-support` feature); `crates/slicer-core/src/lib.rs` (adds `mod perimeter_spatial;` and creates the empty `crates/slicer-core/src/perimeter_spatial.rs` stub in this step, so the red test compiles; these two small edits are pre-approved within this step's budget alongside the new module file).
- Files explicitly out of bounds: generator modules, xtask, WIT/IR contracts, target, lockfiles.
- Expected sub-agent dispatches: Question: locate exact legacy evaluator tests and feature visibility; scope: `crates/slicer-core/**`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` bounded range; `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate parity locations only.
- Verification: `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- exact_queries_match_legacy --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail (expected RED this step; grep the log for the failing test name and `test result: FAILED`).
- Exit condition: The named test exists in the `[[test]]` target list of `crates/slicer-core/Cargo.toml`, compiles, and fails red for the stated contract reason; the plan's Step 2 turns it green. Zero tests or a passing run before implementation is a step failure.

### Step 2: Implement four exact spatial indexes
- Task IDs: `TASK-561`
- Objective: Implement immutable context, computed endpoint envelopes, winding intervals, bridge guard, radius inflation, source-ordinal reduction, and all fallbacks.
- Precondition: Step 1 tests and independent oracle exist.
- Postcondition: Indexed queries are bitwise-equivalent to legacy output and no pruning occurs before bridge overflow safety is established.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-core/src/perimeter_utils.rs` named symbols; `crates/slicer-ir/src/polygon_predicate.rs` named symbol; `docs/08_coordinate_system.md` conversion section.
- Files allowed to edit (at most 3): `crates/slicer-core/src/perimeter_spatial.rs`; `crates/slicer-core/src/lib.rs`; `crates/slicer-core/tests/perimeter_spatial_tdd.rs`.
- Files explicitly out of bounds: module generators, runtime harness, xtask, WIT/IR contracts, target, lockfiles.
- Expected sub-agent dispatches: Question: review numeric proof boundary and rstar API use; scope: new core module and tests; return: `SNIPPETS` (<=30 lines each).
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` bounded range; the query design in this packet's `design.md` Architecture Constraints (settled formulas carried there).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - delegate reusable-query precedent.
- Verification: `cargo test -p slicer-core --features host-algos --test perimeter_spatial_tdd -- exact_queries_match_legacy fallback_and_missing_corpus_are_nonvacuous --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail.
- Exit condition: Exact tests pass, including signed zero and strict bridge behavior, and a fault-injected pruning assertion demonstrates legacy fallback.

### Step 3: Wire Classic and Arachne pass reuse
- Task IDs: `TASK-561`
- Objective: Construct one context outside all relevant passes and route distance/sign/quartile/bridge calls through scoped controls while retaining actual generator behavior.
- Precondition: Step 2 context API is compiling and generator call sites are located.
- Postcondition: Classic and Arachne, including nonplanar and second-pass paths, reuse one region context without public contract fields.
- Files allowed to read, with ranges when over 300 lines: `modules/core-modules/classic-perimeters/src/lib.rs` named functions; `modules/core-modules/arachne-perimeters/src/lib.rs` named functions; relevant module tests.
- Files allowed to edit (at most 3): `modules/core-modules/classic-perimeters/src/lib.rs`; `modules/core-modules/arachne-perimeters/src/lib.rs`; `crates/slicer-core/src/perimeter_spatial.rs`.
- Registration note: the runtime integration test file for capture/nonvacuity is created in Step 4 and registered as `mod perimeter_spatial_capture;` in `crates/slicer-runtime/tests/integration/main.rs` within Step 4's edit list. Step 3's verification therefore runs after Step 4's registration; Step 3 exits only after its own generator wiring compiles with `cargo check -p classic-perimeters -p arachne-perimeters` while its cross-check reuses the Step 4 registered binary.
- Files explicitly out of bounds: WIT/IR/scheduler/public `HostExecutionContext`, xtask, unrelated packets.
- Expected sub-agent dispatches: Question: confirm all pass call sites and no hidden region boundary; scope: two generator files; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/19_visual_debug.md` and `docs/17_agent_debugging.md` delegated bounded summaries.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate traversal comparison.
- Verification: `cargo xtask test --summary -p slicer-runtime --test integration -- perimeter_spatial_capture_and_nonvacuity --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail.
- Exit condition: Real pipeline output is nonempty, pass/region identities are captured, and repeated passes report one context with fewer exact evaluations than the synthetic leaf count.

### Step 4: Add native/WASM capture and independent baselines
- Task IDs: `TASK-561`
- Objective: Capture prepared regions after dispatch filtering and compare each execution mode with its own baseline.
- Precondition: Step 3 real generator wiring exists.
- Postcondition: Native scoped indexed/legacy runs and WASM indexed/legacy runs compare complete outputs, with no native-vs-WASM oracle and no timing support in production artifacts.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/tests/common/perimeter_harness.rs` named symbols; `crates/slicer-wasm-host/src/dispatch.rs` `push_slice_regions` and `sliced_region_to_data_with_prepared`; integration registration.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/perimeter_spatial_capture.rs` (new capture/nonvacuity test file, created here); `crates/slicer-runtime/tests/common/perimeter_harness.rs`; `crates/slicer-wasm-host/src/dispatch.rs` (plus the pre-approved fourth edit: `mod perimeter_spatial_capture;` registration in `crates/slicer-runtime/tests/integration/main.rs`, required so the new file compiles under the `integration` binary).
- Files explicitly out of bounds: public host structs, WIT files, target, generated WASM.
- Expected sub-agent dispatches: Question: verify feature-gated integration dependency visibility and capture ordering; scope: runtime/WASM host tests; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: none beyond requirements delegation.
- Verification: `cargo xtask test --summary -p slicer-runtime --test integration -- perimeter_parity --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail.
- Exit condition: Both modes pass their own exact complete-output baseline and capture filtered prepared regions, including multiple regions and required pass kinds.

### Step 5: Build the controlled rustc policy driver
- Task IDs: `TASK-561`
- Objective: Add the owned shim, exact compiler identity allowlist, structural argv grammar, cfg injection, rejection latch, profile policy, and driver unit tests.
- Precondition: Query cfg name and canonical core source identity are fixed; driver audit is bounded-dispatched.
- Postcondition: Valid host/guest release and optimized-core debug invocations delegate; invalid controls reject and latch; ordinary invocations remain unmodified.
- Files allowed to read, with ranges when over 300 lines: `xtask/src/main.rs` CLI symbols; `xtask/src/test.rs` command symbols; `xtask/src/build_guests.rs` `Invocation`, `VersionProbes`, `guest_build_cargo_command`, `rustc_version_verbose`, `parse_build_guests_flag`; `xtask/src/dist.rs` command symbol.
- Files allowed to edit (at most 3): `xtask/src/rustc_driver.rs`; `xtask/src/main.rs`; `xtask/src/rustc_driver_tests.rs` (a unit test module compiled into the xtask binary; it lives in `src/`, requires a `mod rustc_driver_tests;` declaration in `xtask/src/main.rs` in this same step, and runs via `cargo test -p xtask --bin xtask -- rustc_driver_tests <filter>` because xtask has no `[lib]` target and its only target is the binary rooted at `xtask/src/main.rs`).
- Files explicitly out of bounds: compiler sources, registry dependencies, production core query code, lockfiles.
- Expected sub-agent dispatches: Question: enumerate all private literals and compiler launchers including env rustc probes; scope: `xtask/src/{main,test,dist,build_guests}.rs`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: the driver policy grammar in this packet's `design.md` Code Change Surface; root `AGENTS.md` controlled-build rules; `docs/22_controlled_perimeter_builds.md` created in Step 7 with the full policy grammar.
- OrcaSlicer refs: none.
- Verification: `cargo test -p xtask --bin xtask -- accelerated_policy_rejection_latch --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail (the test module is declared in `xtask/src/main.rs`; xtask has no `[lib]` target, so tests run under the binary target).
- Exit condition: Driver tests cover accepted identity/targets, repeated allowed keys, duplicate keyed rejection, response/LLVM/sysroot rejection, canonical-only cfg, swallowed failure latch, and ordinary mode.

### Step 6: Integrate mode-aware build, freshness, dist, and test entry points
- Task IDs: `TASK-561`
- Objective: Thread private accelerated mode through CLI combinations, guest builds, dist, tests, cache namespaces, metadata, and freshness without changing public `GuestSpec` fields.
- Precondition: Step 5 policy API and existing build/freshness helpers are compiling.
- Postcondition: Explicit accelerated entry points build isolated host/guest artifacts, preserve exit 0/1/3 freshness semantics, reject opposite-mode artifacts, and keep test-support out of production artifacts.
- Files allowed to read, with ranges when over 300 lines: `xtask/src/main.rs` build-guests dispatch; `xtask/src/test.rs` `test_command`; `xtask/src/dist.rs` `dist_command`; `xtask/src/build_guests.rs` `parse_build_guests_flag`, named helpers and inline tests; root `AGENTS.md` guest rules.
- Files allowed to edit (at most 3): `xtask/src/main.rs`; `xtask/src/test.rs`; `xtask/src/build_guests.rs` (dist mode wiring lives in `xtask/src/dist.rs`; it is a pre-approved fourth bounded edit for this step, verified against `dist_command` before editing).
- Files explicitly out of bounds: public `GuestSpec` shape unless all literals are in the same step; Cargo lockfiles; generated artifacts.
- Expected sub-agent dispatches: Question: enumerate `GuestSpec` and `Invocation` literal blast radius before any field addition; scope: `xtask/src/build_guests.rs` and tests; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/07_implementation_status.md` delegated row only; root `AGENTS.md` guest staleness rules.
- OrcaSlicer refs: none.
- Verification: `cargo test -p xtask --bin xtask -- accelerated_mode_freshness --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail (build_guests tests are inline unit tests in `xtask/src/build_guests.rs`, compiled into the binary target; xtask has no `[lib]` target).
- Exit condition: Combined flags parse correctly, mode namespaces and metadata are distinct, unchanged ordinary freshness remains valid, accelerated `--check` is explicit, and zero-test/missing-artifact paths fail visibly.

### Step 7: Commit reproducible corpus validator and serialized acceptance runner
- Task IDs: `TASK-561`
- Objective: Implement the fixed corpus provenance format and ABBA/BAAB runner/validator with exactness-first KEEP/DROP/inconclusive rules. The runner is a NEW committed PowerShell script (proposed path `tmp/perimeter-acceptance/run-acceptance.ps1`) that loops `tmp/alloc-bench/run_bench.ps1` per sample using its actual parameters (`-ExePath -InputModel -Config -ModuleDir -OutputPath -Threads -Label -ResultsPath [-Warmup]` plus the repaired `-ExpectedGenerator` validation); `run_bench.ps1` itself is NOT edited.
- Precondition: Steps 4-6 provide both modes, provenance, and real capture; local corpus paths are available but not committed.
- Postcondition: A fresh coordinator can execute the three workloads, both generators, isolated host+guest snapshots, warmup/sample schedule, and produce visible CPU/wall ratios and terminal status.
- Files allowed to read, with ranges when over 300 lines: `tmp/alloc-bench/run_bench.ps1` parameter block (lines 1-100 only); `tmp/perf-next/EXPERIMENT.md` and `MEASUREMENTS.md` delegated summaries; committed fixture guidance.
- Files allowed to edit (at most 3): `tmp/perimeter-acceptance/run-acceptance.ps1` (new); `crates/slicer-runtime/tests/integration/perimeter_acceptance.rs` (new, created here); `crates/slicer-runtime/tests/integration/main.rs` (`mod perimeter_acceptance;` registration; pre-approved as this step's fourth bounded edit alongside the new doc file below). `docs/22_controlled_perimeter_builds.md` (new; confirmed absent today) is additionally pre-approved in this step's edit budget.
- Files explicitly out of bounds: `tmp/alloc-bench/run_bench.ps1` (reused, not edited), local supplied models/corpora, production timing counters, automatic commit tooling.
- Expected sub-agent dispatches: Question: validate generator marker/config/stderr ownership and corpus provenance fields; scope: acceptance harness and existing bench script; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: the acceptance protocol in this packet's `packet.spec.md` AC-4 and `design.md` Locked Assumptions; `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: none.
- Verification: after the runner exists, `pwsh -NoProfile -File tmp/perimeter-acceptance/run-acceptance.ps1 -DryRun` returns exit 0 with a JSON summary whose `status` is one of `KEEP|DROP|inconclusive` and `automatic_commit` is `false` (dry-run validates the schedule and validator without timing models; a real campaign is user-gated in Step 8). Command: `pwsh -NoProfile -File tmp/perimeter-acceptance/run-acceptance.ps1 -DryRun 2>&1 | tee target/test-output.log`. Also run the registered validator test with a single filter: `cargo test -p slicer-runtime --test integration -- overlap_is_inconclusive_and_never_keep --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail.
- Exit condition: Exactness or validation failure is DROP, overlap is inconclusive with no rerun, only strict CPU+wall separation for every cell is KEEP, and no automatic commit occurs. The runner fails closed on a missing corpus/model/config with an explicit missing-artifact error, and the registered `perimeter_acceptance` integration test proves the validator's KEEP/DROP/inconclusive decision logic on synthetic sample rows without touching models.

### Step 8: Run serialized packet gates and document outcomes
- Task IDs: `TASK-561`
- Objective: Run focused ordinary/controlled core, module, runtime, and guest checks, then the required all-target gates and acceptance campaign without concurrent heavy work. This step also lands the two remaining doc edits from the packet's Doc Impact list: amend the guest-target rule wording in root `AGENTS.md` for the mode-aware namespace and update the guest build/freshness section in `docs/03_wit_and_manifest.md` (`build-guests --check` is already documented there; the edit adds the accelerated-mode rule and preserves the existing text).
- Precondition: Steps 1-7 compile and packet-local tests are registered.
- Postcondition: Evidence is captured in bounded logs, guest freshness mode is explicit, doc-impact greps all return matches, and the user receives measured KEEP/DROP/inconclusive outcome without status fabrication.
- Files allowed to read, with ranges when over 300 lines: `tmp/perimeter-acceptance/run-acceptance.ps1` (only to correct defects its own dry-run or synthetic validator exposes); `crates/slicer-runtime/tests/integration/perimeter_acceptance.rs` (same); packet-local fixture files for corrections exposed by their own focused failures; root `AGENTS.md` guest-target section; `docs/03_wit_and_manifest.md` guest build/freshness section (bounded range). No packet contract edits during execution.
- Files allowed to edit (at most 3): root `AGENTS.md` guest-target section (amend the mode-aware namespace wording); `docs/03_wit_and_manifest.md` guest build/freshness section (add the accelerated-mode rule; preserve existing `build-guests --check` text); `tmp/perimeter-acceptance/run-acceptance.ps1` + packet-local fixture/validator corrections exposed by their own focused failures (pre-approved additional bounded edits). No packet contract edits during execution.
- Files explicitly out of bounds: other packets, backlog status, target payloads, generated guests unless the freshness command directs a rebuild.
- Expected sub-agent dispatches: Question: summarize each command's exit and failures; scope: `target/test-output.log`; return: `FACT`.
- Context cost: `S`
- Authoritative docs: root `AGENTS.md` required gates and packet contract; `docs/22_controlled_perimeter_builds.md` (created in Step 7) for the accelerated gate sequence; `docs/03_wit_and_manifest.md` guest build/freshness section (bounded range) for the doc edit.
- OrcaSlicer refs: none.
- Verification: `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask test --summary -p slicer-runtime --test integration -- perimeter_spatial_capture_and_nonvacuity` - FACT pass/fail, each serialized. Doc greps: `rg -q 'target/guests' AGENTS.md`; `rg -q 'build-guests --check' docs/03_wit_and_manifest.md`; `rg -q 'controlled perimeter build' docs/22_controlled_perimeter_builds.md` - each returns exit 0 after its edit.
- Exit condition: Every required focused check passes, no zero-test false positive occurred, freshness exit is interpreted correctly (exit 0 fresh / 1 stale → rebuild then re-run / 3 infrastructure error → stop, never read as clean), and the real acceptance campaign is executed once per the fixed schedule with its measured KEEP/DROP/inconclusive outcome reported to the user; overlapping evidence stops the campaign without extra runs. The user decides KEEP/DROP and any commit; nothing is committed automatically.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Numeric fixtures and feature seam |
| Step 2 | M | Four indexes and proof boundary |
| Step 3 | M | Two generators and all passes |
| Step 4 | M | Native/WASM capture |
| Step 5 | M | Actual argv policy and latch |
| Step 6 | M | Mode-aware xtask plumbing |
| Step 7 | M | Acceptance runner and validator |
| Step 8 | S | Serialized evidence gates |

The aggregate is `M` as a dependency-sequenced candidate; this is a complexity classification, not an effort estimate. The worker must split any step that becomes `L` before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS or the acceptance command returns an explicitly measured terminal status.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile no prior packet transition; TASK-561 remains pending until user-approved completion.
- `packet.spec.md` is ready for `status: implemented` only after exactness and acceptance evidence support that transition.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command in one serialized validation lane.
- Record remaining packet-local risk and every CPU/wall ratio.
- Confirm context stayed at or below the standard band; no extended-band escalation is authorized by this packet.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
