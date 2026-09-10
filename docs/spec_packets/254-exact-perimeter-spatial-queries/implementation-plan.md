# Implementation Plan: exact-perimeter-spatial-queries

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-561`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every step edits at most 3 files. Fixture data files recorded by a `record_*` test function are data outputs, listed separately, and do not count against the cap. Aggregator `mod` registrations count.
- Doc slot: `docs/23_controlled_perimeter_builds.md` assumes 23 is free (true 2026-09-10); re-derive with `ls docs/2*.md` before creating it and propagate any change to all five packet files.

## Steps

### Step 1: Lock the core seam and red tests
- Task IDs: `TASK-561`
- Objective: Add the `perimeter-spatial-test-support` feature, the `perimeter_spatial_tdd` `[[test]]` target, the reserved-cfg declaration, and the red exactness/fallback/both-modes tests against the not-yet-existing indexed API.
- Precondition: Legacy symbols (`signed_distance_to_boundary`, `expolygon_to_path3d`, `point_in_any_polygon`, `point_in_polygon_winding`) and fixture/literal rules are located by bounded dispatch; `crates/slicer-core` has no `build.rs` (verified 2026-09-10).
- Postcondition: The test target exists and fails RED because `slicer_core::perimeter_spatial` is unresolved; every selected edge case and nonvacuous counter assertion is represented without production counters.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-core/src/perimeter_utils.rs` named symbols; `crates/slicer-ir/src/polygon_predicate.rs` named symbol; `crates/slicer-core/Cargo.toml` `[features]` and two `[[test]]` entries; `docs/21_data_defaults_and_fixtures.md` relevant section.
- Files allowed to edit (at most 3): `crates/slicer-core/tests/perimeter_spatial_tdd.rs` (new); `crates/slicer-core/Cargo.toml` (adds `perimeter-spatial-test-support = []` and `[[test]] name = "perimeter_spatial_tdd" required-features = ["host-algos", "perimeter-spatial-test-support"]`); `crates/slicer-core/build.rs` (new; emits only `cargo::rustc-check-cfg=cfg(pnp_perimeter_spatial_accelerated)`).
- Files explicitly out of bounds: `crates/slicer-core/src/**`, generator modules, xtask, WIT/IR contracts, target, lockfiles.
- Expected sub-agent dispatches: Question: locate exact legacy evaluator tests and feature visibility; scope: `crates/slicer-core/**`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` bounded range; `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate parity locations only.
- Verification: `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd -- exact_queries_match_legacy --nocapture 2>&1 | tee target/test-output.log` - FACT (expected RED: the log shows `error[E0433]`/unresolved `perimeter_spatial`, not `0 tests`). Also `cargo check -p slicer-core --features host-algos` - FACT pass (lib still compiles; build.rs runs).
- Exit condition: The named test target exists in `crates/slicer-core/Cargo.toml`, the build script compiles, and the test run fails for the unresolved-module reason. Zero tests or a passing run before implementation is a step failure.

### Step 2: Implement four exact spatial indexes
- Task IDs: `TASK-561`
- Objective: Implement immutable context, computed endpoint envelopes, winding intervals, bridge guard, radius inflation, source-ordinal reduction, thread-scoped test-support diagnostics (including the always-compiled `observe_prepared_region` no-op), and all fallbacks.
- Precondition: Step 1 red tests and the independent oracle exist.
- Postcondition: Indexed queries are bitwise-equivalent to legacy output, no pruning occurs before bridge overflow safety is established, and both modes plus every fallback are counter-observable.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-core/src/perimeter_utils.rs` named symbols; `crates/slicer-ir/src/polygon_predicate.rs` named symbol; `docs/08_coordinate_system.md` conversion section.
- Files allowed to edit (at most 3): `crates/slicer-core/src/perimeter_spatial.rs` (new); `crates/slicer-core/src/lib.rs` (adds `pub mod perimeter_spatial;`); `crates/slicer-core/tests/perimeter_spatial_tdd.rs`.
- Files explicitly out of bounds: module generators, runtime harness, xtask, WIT/IR contracts, target, lockfiles.
- Expected sub-agent dispatches: Question: review numeric proof boundary and rstar API use; scope: new core module and tests; return: `SNIPPETS` (<=30 lines each).
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` bounded range; the query design in this packet's `design.md` Architecture Constraints (settled formulas carried there).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - delegate reusable-query precedent.
- Verification: `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail (covers `exact_queries_match_legacy`, `accelerated_mode_exercised_not_vacuous`, `fallback_paths_are_nonvacuous`). Then `cargo xtask build-guests --check` - FACT exit code (expected 1: the new `build.rs` staled both perimeter guests; rebuild with `cargo xtask build-guests`, then re-check for exit 0).
- Exit condition: Exact tests pass, including signed zero and strict bridge behavior, a fault-injected pruning assertion demonstrates legacy fallback, and the guest freshness check is exit 0 after the rebuild.

### Step 3: Wire Classic and Arachne pass reuse
- Task IDs: `TASK-561`
- Objective: Construct one context outside all relevant passes and route distance/sign/quartile/bridge calls through it in both generators while retaining actual generator behavior.
- Precondition: Step 2 context API is compiling and generator call sites are located (Classic: `expolygon_to_path3d` in `emit_walls` and `emit_nonplanar_shells`, `point_in_any_polygon` in `emit_walls`; Arachne: `signed_distance_to_boundary`, `point_in_polygon_winding`, `point_in_any_polygon` in `build_walls`).
- Postcondition: Classic and Arachne, including nonplanar and second-pass paths, reuse one region context without public contract fields, and existing perimeter parity tests are unchanged.
- Files allowed to read, with ranges when over 300 lines: `modules/core-modules/classic-perimeters/src/lib.rs` named functions (1462 lines; ranged reads only); `modules/core-modules/arachne-perimeters/src/lib.rs` named functions (1263 lines; ranged reads only); relevant module tests.
- Files allowed to edit (at most 3): `modules/core-modules/classic-perimeters/src/lib.rs`; `modules/core-modules/arachne-perimeters/src/lib.rs`; `crates/slicer-core/src/perimeter_spatial.rs`.
- Files explicitly out of bounds: WIT/IR/scheduler/public `HostExecutionContext`, runtime tests, xtask, unrelated packets.
- Expected sub-agent dispatches: Question: confirm all pass call sites and no hidden region boundary; scope: two generator files; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/19_visual_debug.md` and `docs/17_agent_debugging.md` delegated bounded summaries.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate traversal comparison.
- Verification: `cargo check -p classic-perimeters -p arachne-perimeters --all-targets` - FACT pass; `cargo xtask build-guests` then `cargo xtask build-guests --check` - FACT exit 0; `cargo xtask test --summary -p slicer-runtime --test integration -- perimeter_parity perimeter_edge_cases --nocapture 2>&1 | tee target/test-output.log` - FACT pass (pre-existing tests; proves the rewired guests produce unchanged output; the log must show a nonzero test count).
- Exit condition: Both generators compile with the context wired at every listed call site, guests are fresh, and every pre-existing perimeter parity/edge-case test still passes with a nonzero run count.

### Step 4: Native in-process capture test (AC-2)
- Task IDs: `TASK-561`
- Objective: Add the forwarding feature on `slicer-runtime` and the native in-process test proving one context per region across all passes, with the feature-less guard test.
- Precondition: Step 3 generator wiring exists; the native-entry call shape is located in `crates/slicer-runtime/tests/contract/integrated_parity_classic_perimeters_tdd.rs`.
- Postcondition: `perimeter_spatial_capture_and_nonvacuity` passes under the feature; without the feature the module's single guard test panics with `perimeter-spatial-test-support feature required`.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/Cargo.toml` `[features]` and `[dependencies]` blocks; `crates/slicer-runtime/tests/common/integrated_parity_harness.rs` `run_integrated_parity`; `crates/slicer-runtime/tests/contract/integrated_parity_classic_perimeters_tdd.rs` call shape; `crates/slicer-runtime/tests/integration/main.rs` `mod` list.
- Files allowed to edit (at most 3): `crates/slicer-runtime/Cargo.toml` (adds `perimeter-spatial-test-support = ["slicer-core/perimeter-spatial-test-support"]`); `crates/slicer-runtime/tests/integration/perimeter_spatial_capture.rs` (new); `crates/slicer-runtime/tests/integration/main.rs` (adds `mod perimeter_spatial_capture;`).
- Files explicitly out of bounds: `crates/slicer-wasm-host/**`, public host structs, WIT files, target, generated WASM.
- Expected sub-agent dispatches: Question: verify feature unification reaches the `integration` binary and both native perimeter dev-dependencies; scope: `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/tests/integration/main.rs`; return: `FACT`.
- Context cost: `M`
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: none beyond requirements delegation.
- Verification: `cargo test -p slicer-runtime --features perimeter-spatial-test-support --test integration -- perimeter_spatial_capture_and_nonvacuity --nocapture 2>&1 | tee target/test-output.log` - FACT pass; then `cargo test -p slicer-runtime --test integration -- perimeter_spatial_capture --nocapture 2>&1 | tee target/test-output.log` - FACT expected FAIL with the guard panic text in the log (a `0 tests` result is a step failure).
- Exit condition: Real native pipeline output is nonempty, pass/region identities are captured, repeated passes report one context with fewer exact evaluations than the synthetic leaf count, and the feature-less run fails loudly.

### Step 5: WASM prepared-region capture hook
- Task IDs: `TASK-561`
- Objective: Call the always-compiled `observe_prepared_region` from `push_slice_regions` with an owned projection, and extend the WASM harness to expose captured records alongside output.
- Precondition: Step 2's diagnostics entry exists; `push_slice_regions` (`dispatch.rs`) and `sliced_region_to_data_with_prepared` (`marshal/in_.rs`) are located by bounded dispatch.
- Postcondition: The WASM path records filtered prepared regions when the feature is on and is a no-op otherwise; `slicer-wasm-host` gains no `[features]` table.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/dispatch.rs` `push_slice_regions` only (4131 lines; ranged reads only); `crates/slicer-wasm-host/src/marshal/in_.rs` `sliced_region_to_data_with_prepared` only; `crates/slicer-wasm-host/src/host.rs` `SliceRegionData` only; `crates/slicer-runtime/tests/common/perimeter_harness.rs` named symbols.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/dispatch.rs` (one call site); `crates/slicer-core/src/perimeter_spatial.rs` (projection record type if not already final); `crates/slicer-runtime/tests/common/perimeter_harness.rs`.
- Files explicitly out of bounds: `crates/slicer-wasm-host/Cargo.toml`, `marshal/in_.rs` (read-only), public host structs, WIT files, target, generated WASM.
- Expected sub-agent dispatches: Question: confirm the projection call sits after filtering and before/after the resource push per design; scope: `crates/slicer-wasm-host/src/dispatch.rs` `push_slice_regions`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` delegated bounded summary of the prepared-region contract.
- OrcaSlicer refs: none.
- Verification: `cargo check -p slicer-wasm-host -p slicer-runtime --all-targets` - FACT pass; `cargo check -p slicer-runtime --features perimeter-spatial-test-support --all-targets` - FACT pass.
- Exit condition: Both feature states compile across `slicer-core`, `slicer-wasm-host`, and `slicer-runtime`; the hook is one call with no host types crossing into `slicer-core`.

### Step 6: Native and WASM self-baselines (AC-5)
- Task IDs: `TASK-561`
- Objective: Record deterministic fixture inputs and per-mode baselines, and add `perimeter_spatial_self_baseline_native_indexed`, `perimeter_spatial_self_baseline_native_legacy`, and `perimeter_spatial_self_baseline_wasm` tests.
- Precondition: Steps 4-5 provide native capture and the WASM hook; guests are fresh (`cargo xtask build-guests --check` exit 0).
- Postcondition: Each mode compares its complete `PerimeterIR` postcard bytes to its own baseline; the WASM test asserts ≥2 captured regions with identities matching the native run; no native-vs-WASM output comparison exists.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/tests/common/perimeter_harness.rs` named symbols; `crates/slicer-runtime/tests/fixtures/perimeter_parity/` listing only; `docs/21_data_defaults_and_fixtures.md` fixture-recording section.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/perimeter_spatial_capture.rs` (adds the three tests and their `record_*` functions). Data outputs (not counted): new files under `crates/slicer-runtime/tests/fixtures/perimeter_spatial/`, each with versioned postcard provenance (explicit integer coordinates, float bits).
- Files explicitly out of bounds: `crates/slicer-wasm-host/**`, generator modules, xtask, target, generated WASM.
- Expected sub-agent dispatches: Question: confirm fixture sizes stay under the 1 MB direct-load rule and provenance fields are explicit; scope: `crates/slicer-runtime/tests/fixtures/perimeter_spatial/`; return: `FACT`.
- Context cost: `M`
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: none.
- Verification: `cargo xtask test --summary -p slicer-runtime --features perimeter-spatial-test-support --test integration -- perimeter_spatial_self_baseline --nocapture 2>&1 | tee target/test-output.log` - FACT pass with three tests counted.
- Exit condition: All three self-baseline tests pass, fixture provenance is explicit, and the recorded baselines are byte-stable across two consecutive runs.

### Step 7: Build the controlled rustc policy driver
- Task IDs: `TASK-561`
- Objective: Add the owned shim, exact compiler identity allowlist, structural argv grammar, cfg injection, rejection latch, profile policy, and driver unit tests.
- Precondition: Query cfg name and canonical core source identity are fixed; driver audit is bounded-dispatched; xtask has no `[lib]` and no `[[bin]]` table (verified 2026-09-10).
- Postcondition: Valid host/guest release and optimized-core debug invocations delegate; invalid controls reject and latch; ordinary invocations remain unmodified.
- Files allowed to read, with ranges when over 300 lines: `xtask/src/main.rs` subcommand match; `xtask/src/build_guests.rs` `Invocation`, `VersionProbes`, `guest_build_cargo_command`, `rustc_version_verbose`, `parse_build_guests_flag` (4253 lines; ranged reads only).
- Files allowed to edit (at most 3): `xtask/src/rustc_driver.rs` (new); `xtask/src/rustc_driver_tests.rs` (new unit-test module compiled into the binary); `xtask/src/main.rs` (adds `mod rustc_driver;` and `#[cfg(test)] mod rustc_driver_tests;`).
- Files explicitly out of bounds: compiler sources, registry dependencies, production core query code, `xtask/src/{test,dist,build_guests}.rs`, lockfiles.
- Expected sub-agent dispatches: Question: enumerate all private literals and compiler launchers including env rustc probes; scope: `xtask/src/{main,test,dist,build_guests}.rs`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: the driver policy grammar in this packet's `design.md` Code Change Surface; root `AGENTS.md` controlled-build rules.
- OrcaSlicer refs: none.
- Verification: `cargo test -p xtask --bin xtask -- accelerated_policy_rejection_latch --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail (tests run under the implicit `xtask` binary target).
- Exit condition: Driver tests cover accepted identity/targets, repeated allowed keys, duplicate keyed rejection, response/LLVM/sysroot rejection, canonical-only cfg, swallowed failure latch, and ordinary mode.

### Step 8: Mode-aware build-guests and freshness
- Task IDs: `TASK-561`
- Objective: Thread the private accelerated mode through `build-guests` parsing, cache namespaces, artifact metadata, and freshness (`--accelerated --check`) without changing public `GuestSpec` fields.
- Precondition: Step 7 policy API and existing build/freshness helpers are compiling.
- Postcondition: `build-guests --accelerated` builds isolated guest artifacts, preserves exit 0/1/3 semantics, and `--accelerated --check` rejects opposite-mode artifacts.
- Files allowed to read, with ranges when over 300 lines: `xtask/src/main.rs` build-guests dispatch; `xtask/src/build_guests.rs` `parse_build_guests_flag`, `compute_guest_freshness`, `guest_target_dir`, named helpers and inline tests (ranged reads only); root `AGENTS.md` guest rules.
- Files allowed to edit (at most 3): `xtask/src/main.rs`; `xtask/src/build_guests.rs`.
- Files explicitly out of bounds: public `GuestSpec` shape; `xtask/src/{test,dist}.rs`; Cargo lockfiles; generated artifacts.
- Expected sub-agent dispatches: Question: enumerate `GuestSpec` and `Invocation` literal blast radius before any field addition; scope: `xtask/src/build_guests.rs` and tests; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/07_implementation_status.md` delegated row only; root `AGENTS.md` guest staleness rules.
- OrcaSlicer refs: none.
- Verification: `cargo test -p xtask --bin xtask -- accelerated_mode_freshness --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail (inline unit tests in `xtask/src/build_guests.rs`). Then `cargo xtask build-guests --check` - FACT exit 0 (ordinary mode unaffected).
- Exit condition: Combined flags parse correctly, mode namespaces and metadata are distinct, unchanged ordinary freshness remains exit 0, accelerated `--check` is explicit, and zero-test/missing-artifact paths fail visibly.

### Step 9: Accelerated test and dist entry points
- Task IDs: `TASK-561`
- Objective: Add `--accelerated` to `cargo xtask test` and `cargo xtask dist` (via `DistArgs`), applying the controlled debug profile and keeping test-support features out of dist artifacts.
- Precondition: Steps 7-8 driver and guest mode exist; `test_command(ws_root, passthrough)` and `pub(crate) dist_command(ws_root, &DistArgs)` shapes verified.
- Postcondition: `xtask test --accelerated` sets `RUSTC` to the shim and runs the gated pipeline; `xtask dist --accelerated` stages accelerated host + guest artifacts under a mode-distinct path with recorded metadata.
- Files allowed to read, with ranges when over 300 lines: `xtask/src/test.rs` `test_command`; `xtask/src/dist.rs` `DistArgs` and `dist_command`; `xtask/src/main.rs` `test`/`dist` dispatch.
- Files allowed to edit (at most 3): `xtask/src/test.rs`; `xtask/src/dist.rs`; `xtask/src/main.rs`.
- Files explicitly out of bounds: `xtask/src/build_guests.rs`, `xtask/src/rustc_driver.rs`, lockfiles, generated artifacts.
- Expected sub-agent dispatches: Question: confirm no dist edition path stages a test-support-enabled artifact; scope: `xtask/src/dist.rs`, `xtask/src/editions.rs`; return: `FACT`.
- Context cost: `S`
- Authoritative docs: root `AGENTS.md` build & test commands; this packet's `design.md` xtask shapes.
- OrcaSlicer refs: none.
- Verification: `cargo test -p xtask --bin xtask -- accelerated_entry_points --nocapture 2>&1 | tee target/test-output.log` - FACT pass (inline unit tests in `test.rs`/`dist.rs` named `accelerated_entry_points_*`).
- Exit condition: Both entry points parse, reject `--accelerated` combined with doctests, and never enable `perimeter-spatial-test-support` in dist output.

### Step 10: Tracked acceptance runner, bench copy, and controlled-build doc
- Task IDs: `TASK-561`
- Objective: Create the serialized runner with `-DryRun`, single-cell (`-Workload -ExpectedGenerator`), and `-Campaign` modes, the tracked verbatim copy of the bench script, and `docs/23_controlled_perimeter_builds.md`.
- Precondition: Steps 4-9 provide both modes, provenance, and capture; `tmp/alloc-bench/run_bench.ps1` exists locally (untracked) with parameters `-ExePath -InputModel -Config -ModuleDir -OutputPath -Threads -Warmup -Label -Runs -ResultsPath -Instrumented -PeakSampleMs -ExpectedGenerator -Profile` (verified 2026-09-10).
- Postcondition: A fresh coordinator with the corpus can execute the six cells; `-DryRun` validates schedule and validator with synthetic rows and no corpus; cell/campaign modes fail closed with `missing-artifact: <path>`; `-CorpusRoot` defaults to `tmp/rtree_query_corpus/`.
- Files allowed to read, with ranges when over 300 lines: `tmp/alloc-bench/run_bench.ps1` parameter block (lines 1-30 only); `tmp/perf-next/EXPERIMENT.md` and `MEASUREMENTS.md` delegated summaries; `docs/22_test_quality.md` heading style only (for doc numbering/format).
- Files allowed to edit (at most 3): `resources/perimeter-acceptance/run-acceptance.ps1` (new); `resources/perimeter-acceptance/run_bench.ps1` (new; byte-identical copy of `tmp/alloc-bench/run_bench.ps1`, verified with `cmp`; header comment in the runner records the source path); `docs/23_controlled_perimeter_builds.md` (new; must contain the phrase `controlled perimeter build`, the policy grammar, mode rules, and a §Corpus section describing `tmp/rtree_query_corpus/` layout).
- Files explicitly out of bounds: `tmp/alloc-bench/run_bench.ps1` (copied, never edited), local supplied models/corpora, production timing counters, automatic commit tooling, `crates/**`, `xtask/**`.
- Expected sub-agent dispatches: Question: validate generator marker/config/stderr ownership and corpus provenance fields; scope: the tracked bench copy; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: the acceptance protocol in this packet's `packet.spec.md` AC-4 and `design.md` Locked Assumptions; `docs/21_data_defaults_and_fixtures.md` delegated summary.
- OrcaSlicer refs: none.
- Verification: `cmp resources/perimeter-acceptance/run_bench.ps1 tmp/alloc-bench/run_bench.ps1` - FACT exit 0; `pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -DryRun 2>&1 | tee target/test-output.log` - FACT exit 0 with a JSON summary whose `status` is one of `KEEP|DROP|inconclusive`, `automatic_commit` is `false`, and `cells` has 6 entries; `pwsh -NoProfile -File resources/perimeter-acceptance/run-acceptance.ps1 -Workload supports-off-benchy -ExpectedGenerator classic -CorpusRoot target/perimeter-acceptance/absent-corpus 2>&1 | tee target/test-output.log; test "${PIPESTATUS[0]}" -ne 0 && rg -q '^missing-artifact: ' target/test-output.log` - FACT pass; `rg -q 'controlled perimeter build' docs/23_controlled_perimeter_builds.md` - FACT exit 0.
- Exit condition: Dry-run and missing-artifact behaviors are proven, the bench copy is byte-identical, and the doc exists with the required sections.

### Step 11: Acceptance validator integration test (AC-N3)
- Task IDs: `TASK-561`
- Objective: Prove the validator's KEEP/DROP/inconclusive decision logic on synthetic sample rows without touching models.
- Precondition: Step 10's runner defines the `summary.json` schema (`status`, `automatic_commit`, `threads`, `samples_per_cell`, `warmup_per_cell`, `cells[]` with `cpu_separated`, `wall_separated`, ratios, generator marker, degraded/non-fatal counts).
- Postcondition: `overlap_is_inconclusive_and_never_keep` passes; exactness failure is DROP; identical pre-existing degraded/non-fatal counts do not reject; a changed count is a generator disagreement.
- Files allowed to read, with ranges when over 300 lines: `resources/perimeter-acceptance/run-acceptance.ps1` validator section only; `crates/slicer-runtime/tests/integration/main.rs` `mod` list.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/perimeter_acceptance.rs` (new); `crates/slicer-runtime/tests/integration/main.rs` (adds `mod perimeter_acceptance;`).
- Files explicitly out of bounds: models/corpora, `resources/perimeter-acceptance/run_bench.ps1`, `crates/slicer-core/**`, `xtask/**`.
- Expected sub-agent dispatches: Question: confirm the Rust validator mirrors the runner's decision table field-for-field; scope: the new test and the runner validator section; return: `FACT`.
- Context cost: `S`
- Authoritative docs: `packet.spec.md` AC-4/AC-N3.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test integration -- overlap_is_inconclusive_and_never_keep --nocapture 2>&1 | tee target/test-output.log` - FACT pass with a nonzero test count.
- Exit condition: Overlap is inconclusive with no rerun, exactness failure is DROP, only strict CPU+wall separation for all six cells is KEEP, and no automatic commit path exists.

### Step 12: Serialized packet gates, doc edits, and the campaign
- Task IDs: `TASK-561`
- Objective: Land the two remaining Doc Impact edits, run the focused ordinary/controlled checks and the required all-target gates serially, then run the real acceptance campaign once (user-gated on corpus presence) and report its measured outcome.
- Precondition: Steps 1-11 complete; `cargo xtask build-guests --check` exit 0; the user has confirmed `tmp/rtree_query_corpus/` is prepared per `docs/23` §Corpus (absent as of 2026-09-10; if still absent, AC-4 is reported as blocked-on-corpus, never as inconclusive or KEEP).
- Postcondition: Evidence is captured in bounded logs, all three Doc Impact greps return exit 0, `CLAUDE.md` is regenerated, and the user receives the measured KEEP/DROP/inconclusive (or blocked-on-corpus) outcome without status fabrication.
- Files allowed to read, with ranges when over 300 lines: root `AGENTS.md` §"Guest WASM Staleness"; `docs/03_wit_and_manifest.md` §"Build & Freshness Contract (Normative)" (bounded range around the `build-guests --check` bullets); `target/test-output.log` via dispatched summaries.
- Files allowed to edit (at most 3): root `AGENTS.md` (add the `build-guests --accelerated` namespace rule); `docs/03_wit_and_manifest.md` (add the `build-guests --accelerated --check` rule; preserve existing text). Corrective edits are limited to files this packet created (Steps 1-11) when their own focused command fails, each logged as "Step 12 fix-up: <file> — <focused command re-run>". `CLAUDE.md` is regenerated by `cargo xtask sync-agents`, not edited.
- Files explicitly out of bounds: other packets, `docs/07_implementation_status.md` (updated only by the completion-gate worker dispatch), target payloads, generated guests unless the freshness command directs a rebuild, packet contract files.
- Expected sub-agent dispatches: Question: summarize each command's exit and failures; scope: `target/test-output.log`; return: `FACT`.
- Context cost: `S`
- Authoritative docs: root `AGENTS.md` required gates; `docs/23_controlled_perimeter_builds.md` for the accelerated gate sequence; `docs/03_wit_and_manifest.md` guest build/freshness section.
- OrcaSlicer refs: none.
- Verification (serialized, in this order): `rg -q 'build-guests --accelerated' AGENTS.md && cargo xtask sync-agents` - FACT exit 0; `rg -q 'build-guests --accelerated --check' docs/03_wit_and_manifest.md` - FACT exit 0; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` - FACT exit codes; every AC command from `packet.spec.md` (AC-1, AC-2, AC-3, AC-3N, AC-5, AC-N1, AC-N2, AC-N3) - FACT pass each; then, only if the corpus is present, the AC-4 campaign command - JSON status.
- Exit condition: Every required focused check passes with nonzero test counts, freshness exits are interpreted correctly (0 fresh / 1 stale → rebuild then re-run / 3 infrastructure error → stop), both doc greps flipped from exit 1 to exit 0 across their edits, and the campaign is executed once per the fixed schedule with its measured outcome reported to the user; overlapping evidence stops the campaign without extra runs. The user decides KEEP/DROP and any commit; nothing is committed automatically.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Feature, test target, check-cfg build script, red tests |
| Step 2 | M | Four indexes, diagnostics, proof boundary |
| Step 3 | M | Two generators and all passes; guest rebuild |
| Step 4 | M | Forwarding feature + native capture test |
| Step 5 | M | WASM dispatch hook + harness |
| Step 6 | M | Self-baseline tests and fixtures |
| Step 7 | M | Driver argv policy and latch |
| Step 8 | M | Mode-aware build-guests/freshness |
| Step 9 | S | test/dist entry points |
| Step 10 | M | Runner, bench copy, docs/23 |
| Step 11 | S | Validator test |
| Step 12 | S | Doc edits, gates, campaign |

The aggregate is `M` as a dependency-sequenced candidate; this is a complexity classification, not an effort estimate. The worker must split any step that becomes `L` before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS or the acceptance command returns an explicitly measured terminal status (or an explicit blocked-on-corpus report for AC-4).
- All three Doc Impact greps return exit 0 and `cargo xtask sync-agents` has been run.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile no prior packet transition; TASK-561 remains pending until user-approved completion.
- `packet.spec.md` is ready for `status: implemented` only after exactness and acceptance evidence support that transition.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command in one serialized validation lane.
- Record remaining packet-local risk and every CPU/wall ratio for all six cells.
- Confirm context stayed at or below the standard band; no extended-band escalation is authorized by this packet.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` (check/clippy) or name their `--features` explicitly (test) so the test, bench, and example targets compile and feature-gated targets are not silently skipped.
