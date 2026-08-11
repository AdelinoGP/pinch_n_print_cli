# Implementation Plan: 205b-native-transport-completion

## Execution Rules

- Work one atomic step at a time; use TDD, implementation, then the narrowest falsifying validation.
- `slicer-integrated-modules` tests remain in-file `#[cfg(test)]`; parity tests belong under `crates/slicer-runtime/tests/contract/` and `tests/integration/`, mounted by `mod` lines in their `main.rs`.
- Delegate every cargo run and authoritative-doc fact-check. Do not hardcode module counts.
- Do not edit `docs/07`, geometry call sites, edition membership, or generated guests.

## Steps

### Step 1: Reconcile the two transport and module surfaces

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: confirm both manifests, annotated types, stage ids, native output shapes, and the exact fatal arms in `native.rs`.
- Precondition: 201, 202, 204, and 205 are `implemented` (the registry, parity comparator, and coverage gate exist in the tree); `205a-integrated-edition-coverage` is currently `draft` and must be `implemented` before this packet activates — its sixteen modules are the prerequisite coverage this packet's two complete.
- Postcondition: a written note (in the swarm working log, not a new file) recording both modules' `#[slicer_module]` type names, SDK traits, stage ids, native output shapes, and the exact line ranges of the two fatal arms to complete.
- Files allowed to read, with ranges when over 300 lines: the two module manifests and bounded module sources (by `rg` only); `crates/slicer-wasm-host/src/marshal/native.rs` — 869 lines, only ~560-680 and ~800-869; existing registry and parity tests.
- Files allowed to edit (at most 3): none — read-only discovery step.
- Files explicitly out of bounds: `docs/spec_packets/205a-*/implementation-plan.md`, `target/`, `Cargo.lock`, generated guests.
- Expected dispatches: symbol lookup and transport-shape facts, each returned as `LOCATIONS` or `SNIPPETS`.
- Context cost: `S`
- Verification: `rg` confirms the two module stage ids and the two fatal transport messages.
- Exit condition: both module stages and output builders are identified; no other module is pulled into scope.

### Step 2: Complete native layer and postpass commits

- Task IDs: `ADR-0056`
- Objective: replace the two fatal commit arms with complete IR conversion and ordered gcode-command application.
- Precondition: Step 1's facts are recorded — both modules' stage ids, annotated types, native output shapes, and the exact fatal arms are identified.
- Postcondition: the `Layer::PathOptimization` arm commits its native output to `LayerStageCommit` (no fatal `Err`), and the postpass gcode arm applies the collected commands in order through the existing accumulator and returns the normal `PostpassOutput`; unsupported commands fail with an explicit error naming the command kind.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/native.rs` — 869 lines; only the two fatal arms and their nearest committed neighbors (~lines 560-680 and ~800-869)
  - the two module manifests and bounded module sources (by `rg` only) — the native output shapes
  - the existing committed layer/postpass converters — the conversion pattern to mirror
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/marshal/native.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-sdk/src/native.rs`, `crates/slicer-macros/**`, dispatch routing, `xtask/**`, `crates/slicer-integrated-modules/**`.
- Blast-radius discipline: not applicable — both arms currently return fatal errors, so no working call site changes behavior class; no struct field or constant is added.
- Expected sub-agent dispatches:
  - Question: which existing converter and accumulator apply path should each transport call? scope: bounded `native.rs` and stage IR definitions; return: `SNIPPETS` (≤4, ≤30 lines)
  - Question: does the targeted wasm-host test set compile and pass after the edit? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Verification:
  - `cargo check -p slicer-wasm-host --all-targets` — FACT pass/fail (the edited marshal compiles)
  - `sh -c 'if rg -Uq "PathOptimization[^}]{0,200}does not yet support|does not yet support[^}]{0,200}PathOptimization" crates/slicer-wasm-host/src/marshal/native.rs; then echo "pathopt-still-fatal"; else echo "pathopt-committed"; fi'` — FACT: expected `pathopt-committed`. The multiline (`rg -U`) check verifies that no `PathOptimization` occurrence sits in a fatal `Err("...does not yet support...")` arm — it matches only when `PathOptimization` and the fatal-message text appear in the same arm block (across the `match`-arm line and its `Err(format!(...))` body), regardless of whether they share a line. `Layer::SlicePostProcess` may legitimately remain fatal; the check is specific to `PathOptimization`.
  - Functional proof deferred to Step 4b's AC-3/AC-4 parity tests (they drive the committed transport against the wasm path); unsupported gcode command kinds must fail with an explicit error naming the kind.
- Exit condition: both transports return the same success shape as wasm for their contract fixtures and no emitted path/command is silently dropped.

### Step 3: Register both modules behind features

- Task IDs: `ADR-0056`
- Objective: add optional dependencies, per-module features, registrations, native entries, and in-file registry tests.
- Precondition: Step 2 landed (the transports the two modules depend on are committed); Step 1's annotated type names and SDK traits are recorded.
- Postcondition: `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set` and `full_coverage_native_entry_families_match_stage_ids` pass (every feature in the explicit named set in the command must be named — the crate's `default` is empty and the pilots and 205a features are optional).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-integrated-modules/Cargo.toml` — whole file
  - `crates/slicer-integrated-modules/src/lib.rs` — whole file (under 300 lines)
  - the two module `.toml` manifests — the `[module] id` and `[stage] id`
- Files allowed to edit (at most 3):
  - `crates/slicer-integrated-modules/Cargo.toml`
  - `crates/slicer-integrated-modules/src/lib.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-wasm-host/**`, `crates/pnp-cli/**`, `xtask/**`, `dist/editions.toml`.
- Blast-radius discipline: not applicable — additive optional deps and cfg-gated push blocks; no struct field, no constant.
- Expected sub-agent dispatches:
  - Question: do the AC-1 and AC-2 commands pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Verification: the AC-1 and AC-2 commands from `packet.spec.md`; the expected set is derived from the registered set (union of pilot set, 205a set, and this packet's two), never a literal count.
- Exit condition: both manifest ids, origin labels, and native entry families (`Layer(..)` for `path-optimization-default`, `Postpass(..)` for `machine-gcode-emit`) are present only when their features are enabled, and neither test hardcodes a module count.

### Step 4a: Path-optimization and gcode comparators + negative self-tests

- Task IDs: `ADR-0056`
- Objective: extend `crates/slicer-runtime/tests/common/parity_invariants.rs` with the path-optimization structural comparator (over `LayerStageCommit::PathOptimization(PathOptimizationCommit)`) and the gcode-sequence comparator, each with a negative self-test proving non-vacuity (AC-N1: `parity_comparator_rejects_dropped_path`).
- Precondition: Steps 2-3 landed; the two transports are committed.
- Postcondition: the new comparators' self-tests pass, including the negative dropped-path case; each negative test goes red if its `Err` branch is stubbed out.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — whole file (under 300 lines)
  - `crates/slicer-ir/src/stage_io.rs` — `PathOptimizationCommit` shape
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/parity_invariants.rs`
  - `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-wasm-host/src/marshal/**`.
- Blast-radius discipline: not applicable — additive free functions.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_path` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_path` — FACT pass/fail (AC-N1)
- Exit condition: the negative comparator test passes and is non-vacuous.

### Step 4b: Dual-dispatch parity contract tests

- Task IDs: `ADR-0056`
- Objective: author the two parity contract tests (`integrated_parity_path_optimization`, `integrated_parity_machine_gcode_emit`), each mounting the Step-4a comparator on a byte-identical stage input.
- Precondition: Step 4a landed (the comparators and their self-tests exist); `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_path_optimization` and `integrated_parity_machine_gcode_emit` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_path_optimization_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_machine_gcode_emit_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-wasm-host/src/marshal/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: do `cargo test -p slicer-runtime --test contract -- integrated_parity_path_optimization` and `integrated_parity_machine_gcode_emit` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_path_optimization 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail (AC-3)
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_machine_gcode_emit 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail (AC-4)
- Exit condition: both pass and are independently red or green.

### Step 4c: Extend the external-override integration test

- Task IDs: `ADR-0056`
- Objective: extend the existing `full_coverage_external_override_tdd.rs` integration test (created and registered by packet 205a for its sixteen modules) to additionally cover `path-optimization-default` and `machine-gcode-emit` — proving the two newly-integrated modules never bypass a user's external override. **Amend the 205a test; do not create a second file or re-register it** (its `mod` line in `integration/main.rs` was added by 205a).
- Precondition: Step 4b landed; packet 205a's `full_coverage_external_override_tdd.rs` exists in the tree; `cargo xtask build-guests --check` reports clean.
- Postcondition: the extended test passes and is independently red or green; the two new modules' override checks (external override forces `native_entry: None`, `wasm_component: Some(..)`) are covered.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` — packet 205a's existing test (the pattern to extend)
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` — `load_live_modules_for_plan_with_integrated`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` (amended — created by 205a)
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-wasm-host/src/marshal/**`.
- Blast-radius discipline: not applicable — amending an existing test file.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` pass after the extension? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0056-integrated-modules-native-dispatch.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` — FACT pass/fail (AC-N2)
- Exit condition: it passes; an external override forces the wasm path (`native_entry: None`, `wasm_component: Some(..)`) for all covered modules including the two new ones.

### Step 5: Add CLI passthrough features and close the integrated plan

- Task IDs: `ADR-0057`
- Objective: delegate both CLI features to the registry and prove the integrated edition plan has no external staging.
- Precondition: Steps 3-4c landed (both modules are registered, natively entered, and parity-gated).
- Postcondition: `integrated-path-optimization-default` and `integrated-machine-gcode-emit` exist in `crates/pnp-cli/Cargo.toml` with bodies naming `slicer-integrated-modules/<name>`; AC-6's command prints `PASS`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/Cargo.toml` — whole file
  - `dist/editions.toml` — whole file (read-only; membership is unchanged)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/Cargo.toml`
- Files explicitly out of bounds: `crates/slicer-integrated-modules/**`, `modules/core-modules/**`, `xtask/**`, `dist/editions.toml`, `crates/slicer-wasm-host/**`.
- Blast-radius discipline: not applicable — additive, off-by-default cargo features.
- Expected sub-agent dispatches:
   - Question: do the AC-5, AC-6, AC-7, and AC-N3 commands pass, including the `discover_guests`-derived coverage test? scope: repo root; return: `FACT` `PASS` / the single `FAIL` line
  - Question: do workspace check, clippy, and guest freshness gates pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Verification: AC-5, AC-6, AC-7, AC-N3, workspace check, clippy, and guest freshness commands. AC-6 must run `dist_plan_integrated_stages_nothing_externally`, whose expected core set is derived from `discover_guests`, before checking the emitted plan.
- Exit condition: `cargo xtask dist --edition integrated --plan` exits successfully, the discover-guests-derived coverage test passes, its `integrated` lines cover every registered core module stem, and it has no external lines.

### Step 6: Doc surfaces and the plan's closure note

- Task IDs: `ADR-0057`
- Objective: update the two doc surfaces per `packet.spec.md` §Doc Impact Statement.
- Precondition: Step 5 landed (the Integrated edition verifiably builds, so the docs state a proven fact).
- Postcondition: both doc greps from `packet.spec.md` §Doc Impact Statement pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/01_system_architecture.md` — §"Producing the tier-4 layout: `cargo xtask dist`" only
  - `docs/specs/multi-edition-distribution-plan.md` — §"Also unscheduled" only
- Files allowed to edit (at most 3):
  - `docs/01_system_architecture.md`
  - `docs/specs/multi-edition-distribution-plan.md`
- Files explicitly out of bounds: `docs/adr/**`, `CONTEXT.md`, `docs/07_implementation_status.md`, any other `docs/*.md`.
- Blast-radius discipline: not applicable — prose only.
- Expected sub-agent dispatches:
  - Question: which doc files still name the pre-205b coverage state? scope: `docs/`; return: `LOCATIONS` (≤10 entries)
- Context cost: `S`
- Verification:
  - `sh -c 'rg -q "integrated" docs/01_system_architecture.md && rg -q "205b" docs/specs/multi-edition-distribution-plan.md && echo PASS'` — FACT `PASS` / `FAIL`
- Exit condition: both greps pass, and the plan note records that the Integrated edition now builds, closing the plan's Integrated-edition row.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only reconciliation |
| Step 2 | M | Two native transport commits |
| Step 3 | M | Registry features and entries |
| Step 4a | M | Path-optimization + gcode comparators, AC-N1 |
| Step 4b | M | Two parity gates, AC-3/AC-4 |
| Step 4c | S | External-override proof, AC-N2 |
| Step 5 | S | CLI features and closure |
| Step 6 | S | Two doc surfaces |

Aggregate: `M`; no step is `L`.

## Packet Completion Gate

- All six steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` is clean before parity reruns.
- `docs/07_implementation_status.md` is not edited.
- `packet.spec.md` is ready for `status: implemented`.
