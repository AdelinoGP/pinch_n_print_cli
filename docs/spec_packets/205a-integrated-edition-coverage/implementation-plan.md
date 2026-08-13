# Implementation Plan: 205a-integrated-edition-coverage

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- `slicer-integrated-modules` is a library crate; its tests are in-file `#[cfg(test)]`. The parity tests are integration tests under `crates/slicer-runtime/tests/contract/` and `tests/integration/`, registered by `mod` lines in the respective `main.rs`.
- No step may write a module name read from `dist/editions.toml` into this plan, into an AC, into CI YAML, or into a doc. Re-derive it at the point of use.
- **The two transport-blocked modules (`path-optimization-default`, `machine-gcode-emit`) are OUT OF SCOPE.** No step may add a registry feature, native entry, parity test, or passthrough feature for them. They are packet 205b's surface.

## Steps

### Step 1: Reconcile the sixteen-module surface and the native-transport scope

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: establish, against the tree, the exact `#[slicer_module]` type name and SDK trait for each of the sixteen modules, and re-verify that each module's stage is committed in `crates/slicer-wasm-host/src/marshal/native.rs` (and that `path-optimization-default` / `machine-gcode-emit` are the only two that are not).
- Precondition: packets 201, 202, 204, 205 are `implemented`; `crates/slicer-integrated-modules/`, `crates/slicer-runtime/tests/common/parity_invariants.rs`, `dist/editions.toml`, and `xtask/src/dist.rs` exist.
- Postcondition: a written note (in the swarm working log, not a new file) listing the sixteen modules, each with its `#[slicer_module]` type name, SDK trait, stage id, and native-transport status (committed), plus confirmation that exactly `path-optimization-default` and `machine-gcode-emit` are transport-blocked.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/<name>/src/lib.rs` for each of the sixteen — the `#[slicer_module]` type and trait, by `rg -n 'slicer_module|impl .*Module|pub struct'`
  - `crates/slicer-wasm-host/src/marshal/native.rs` — the stage dispatch (lines ~390-870)
  - `crates/slicer-integrated-modules/src/lib.rs` — whole file
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — whole file
- Files allowed to edit (at most 3): none — read-only discovery step.
- Files explicitly out of bounds: `modules/core-modules/path-optimization-default/**`, `modules/core-modules/machine-gcode-emit/**`, `docs/spec_packets/203-*/design.md`, `docs/spec_packets/204-*/implementation-plan.md`, `target/`, `Cargo.lock`.
- Blast-radius discipline: not applicable — no struct field or constant is added or changed in this step.
- Expected sub-agent dispatches:
  - Question: `#[slicer_module]` type name + SDK trait for each of the sixteen? scope: `modules/core-modules/<name>/src/lib.rs`; return: `LOCATIONS` (≤32 entries)
  - Question: which stages does the native marshal commit vs return a fatal `Err`? scope: `crates/slicer-wasm-host/src/marshal/native.rs`; return: `FACT` (≤10 lines)
- Context cost: `S`
- Authoritative docs: `docs/adr/0056-integrated-modules-native-dispatch.md`, `docs/spec_packets/204-hybrid-pilot-parity/design.md` §Gaps Inherited from Packet 202.
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'for m in fuzzy-skin gyroid-infill infill-linker layer-planner-default lightning-infill overhang-classifier-default part-cooling rectilinear-infill seam-placer seam-planner-default skirt-brim support-surface-ironing top-surface-ironing traditional-support tree-support wipe-tower; do test -d modules/core-modules/$m || { echo "MISSING: $m"; exit 1; }; done; echo PASS'` — FACT pass/fail
- Exit condition: the sixteen facts are recorded, each module's stage is confirmed committed, and exactly `path-optimization-default` + `machine-gcode-emit` are confirmed transport-blocked. If any of the sixteen is NOT committed, STOP and re-scope (that module moves to 205b).

### Step 2: Registry features and native entries for the sixteen

- Task IDs: `ADR-0056`
- Objective: add the sixteen modules to `crates/slicer-integrated-modules/` behind per-module cargo features, extending `integrated_registrations()` and `native_entries()`.
- Precondition: Step 1's facts are recorded.
- Postcondition: `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set` and `full_coverage_native_entry_families_match_stage_ids` pass (every feature in the explicit named set in the command must be named — the crate's `default` is empty and the three pilots are optional).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-integrated-modules/Cargo.toml` — whole file
  - `crates/slicer-integrated-modules/src/lib.rs` — whole file
  - `modules/core-modules/<name>/<name>.toml` — the `[module] id` and `[stage] id` for each of the sixteen
- Files allowed to edit (at most 3):
  - `crates/slicer-integrated-modules/Cargo.toml`
  - `crates/slicer-integrated-modules/src/lib.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/pnp-cli/**`, `xtask/**`, `dist/editions.toml`.
- Blast-radius discipline: not applicable — additive optional deps and cfg-gated push blocks; no struct field, no constant.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Authoritative docs: `docs/adr/0056-integrated-modules-native-dispatch.md`, `docs/spec_packets/204-hybrid-pilot-parity/design.md` §Code Change Surface.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set` — FACT pass/fail
  - `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_native_entry_families_match_stage_ids` — FACT pass/fail
- Exit condition: both tests pass, and neither contains a hardcoded module count that would rot when a module is added or removed — the expected registration set is derived from the union of the pilot set and this packet's set (the registered set), not a literal `19`.

### Step 3: New family comparators (finalization, seam-plan, layer-planning) with self-tests

- Task IDs: `ADR-0056`
- Objective: author the parity comparators for the stage families 204 did not demonstrate, each with its own self-tests proving non-vacuity, before any subject test.
- Precondition: Step 2 landed.
- Postcondition: the new comparators' self-tests pass, including the negative cases (dropped geometry, dropped entry, dropped point).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — whole file
  - `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs` — whole file
  - `crates/slicer-ir/src/stage_io.rs`, `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/stage_io.rs` — the commit/IR type shapes
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/parity_invariants.rs`
  - `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`
- Files explicitly out of bounds: `crates/slicer-wasm-host/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`.
- Blast-radius discipline: not applicable — new free functions.
- Expected sub-agent dispatches:
  - Question: exact shapes of `FinalizationOutput`, `SeamPlanIR`, `LayerCollectionIR`, and the finalization runner's return type? scope: `crates/slicer-ir/src/stage_io.rs`, `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/stage_io.rs`, `crates/slicer-wasm-host/src/traits.rs`; return: `SNIPPETS` (≤4, ≤30 lines)
- Context cost: `M`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- parity_comparator_` — FACT pass/fail
- Exit condition: the new comparators' self-tests pass, and each negative test fails when its `Err` branch is deleted (verify by temporarily returning `Ok(())` and re-running — the tests must go red). A negative test that passes against a stubbed-out check is vacuous.

### Step 4a1: Infill-family parity tests (gyroid-infill, lightning-infill)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `Layer::Infill` module, each mounting the layer-family comparator (`assert_parity_structural`) on a byte-identical `LayerStageInput`.
- Precondition: Steps 2-3 landed; `cargo xtask build-guests --check` reports clean (re-run after any manifest edit).
- Postcondition: `integrated_parity_gyroid_infill` and `integrated_parity_lightning_infill` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-runtime/tests/common/` — fixtures
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_gyroid_infill_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_lightning_infill_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_gyroid_infill integrated_parity_lightning_infill` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_gyroid_infill` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_lightning_infill` — FACT pass/fail
- Exit condition: both pass. If a module's parity test fails on a widened-tolerance need, follow the §Open Questions rule (measure, widen to next order of magnitude, never past `1e-2` mm) and trigger Step 7's conditional DEVIATION_LOG row. If a module's native path returns a fatal `Err`, STOP — that module is transport-blocked and belongs in 205b, not here.

### Step 4a2: Infill-family parity tests (rectilinear-infill, top-surface-ironing)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `Layer::Infill` module, mounting `assert_parity_structural` on a byte-identical `LayerStageInput`.
- Precondition: Step 4a1 landed (the infill-family pattern is proven); `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_rectilinear_infill` and `integrated_parity_top_surface_ironing` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-runtime/tests/common/` — fixtures
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_rectilinear_infill_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_rectilinear_infill integrated_parity_top_surface_ironing` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_rectilinear_infill` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_top_surface_ironing` — FACT pass/fail
- Exit condition: both pass. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4a3: InfillPostProcess-family parity test (infill-linker)

- Task IDs: `ADR-0056`
- Objective: author the parity contract test for `infill-linker` (`Layer::InfillPostProcess`), mounting `assert_parity_structural` on a byte-identical `LayerStageInput`.
- Precondition: Step 4a2 landed; `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_infill_linker` passes and is independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-runtime/tests/common/` — fixtures
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_infill_linker_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_infill_linker` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_infill_linker` — FACT pass/fail
- Exit condition: it passes. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4b1: Support-family parity tests (traditional-support, tree-support)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `Layer::Support` module, mounting `assert_parity_structural` on a byte-identical `LayerStageInput`.
- Precondition: Steps 3 and 4a landed (the layer-family pattern is proven); `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_traditional_support` and `integrated_parity_tree_support` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-runtime/tests/common/` — fixtures (including `support_wedge.rs`)
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_traditional_support_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_tree_support_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_traditional_support integrated_parity_tree_support` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_traditional_support` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_tree_support` — FACT pass/fail
- Exit condition: both pass. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4b2: SupportPostProcess/PerimetersPostProcess-family parity tests (support-surface-ironing, fuzzy-skin)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `Layer::SupportPostProcess` / `Layer::PerimetersPostProcess` module, mounting `assert_parity_structural` on a byte-identical `LayerStageInput`.
- Precondition: Step 4b1 landed; `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_support_surface_ironing` and `integrated_parity_fuzzy_skin` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-runtime/tests/common/` — fixtures
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_fuzzy_skin_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_support_surface_ironing integrated_parity_fuzzy_skin` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_support_surface_ironing` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_fuzzy_skin` — FACT pass/fail
- Exit condition: both pass. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4b3: PerimetersPostProcess-family parity test (seam-placer)

- Task IDs: `ADR-0056`
- Objective: author the parity contract test for `seam-placer` (`Layer::PerimetersPostProcess`), mounting `assert_parity_structural` on a byte-identical `LayerStageInput`.
- Precondition: Step 4b2 landed; `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_seam_placer` passes and is independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction
  - `crates/slicer-runtime/tests/common/` — fixtures
  - `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, stage input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_seam_placer_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_seam_placer` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_seam_placer` — FACT pass/fail
- Exit condition: it passes. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4c: PrePass-family parity contract tests (layer-planner-default, seam-planner-default)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `PrePass::LayerPlanning` / `PrePass::SeamPlanning` module, mounting `assert_prepass_parity_structural` and the new seam-plan comparator from Step 3 on byte-identical prepass inputs.
- Precondition: Steps 3 and 4a landed (the seam-plan comparator and its self-tests exist); `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_layer_planner` and `integrated_parity_seam_planner` each pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — the prepass comparator and the Step-3 seam-plan comparator
  - `crates/slicer-runtime/tests/common/` — prepass fixtures
  - `crates/slicer-wasm-host/src/binding.rs` — `PrepassStageInput` construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_layer_planner_tdd.rs`
  - `crates/slicer-runtime/tests/contract/integrated_parity_seam_planner_tdd.rs`
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_` pass for the prepass family? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `M`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'for t in integrated_parity_layer_planner integrated_parity_seam_planner; do cargo test -p slicer-runtime --test contract -- $t >/dev/null 2>&1 || { echo "FAIL: $t"; exit 1; }; done; echo PASS'` — FACT `PASS` / the failing test name
- Exit condition: both pass. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4d1: Finalization-family parity tests (overhang-classifier-default, part-cooling)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `PostPass::LayerFinalization` module, mounting the Step-3 finalization comparator over the merged `LayerCollectionIR` on a byte-identical finalization input.
- Precondition: Steps 3 and 4a landed (the finalization comparator and its self-tests exist); `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_overhang_classifier` and `integrated_parity_part_cooling` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — the Step-3 finalization comparator
  - `crates/slicer-wasm-host/src/traits.rs` — the finalization runner declaration only
  - `crates/slicer-wasm-host/src/binding.rs` — finalization input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_overhang_classifier_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_part_cooling_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_overhang_classifier integrated_parity_part_cooling` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_overhang_classifier` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_part_cooling` — FACT pass/fail
- Exit condition: both pass. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4d2: Finalization-family parity tests (skirt-brim, wipe-tower)

- Task IDs: `ADR-0056`
- Objective: author one parity contract test per `PostPass::LayerFinalization` module, mounting the Step-3 finalization comparator over the merged `LayerCollectionIR` on a byte-identical finalization input.
- Precondition: Step 4d1 landed; `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_skirt_brim` and `integrated_parity_wipe_tower` pass and are independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — the Step-3 finalization comparator
  - `crates/slicer-wasm-host/src/traits.rs` — the finalization runner declaration only
  - `crates/slicer-wasm-host/src/binding.rs` — finalization input construction
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_skirt_brim_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/integrated_parity_wipe_tower_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-runtime/tests/integration/**`.
- Blast-radius discipline: not applicable — new test files.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test contract -- integrated_parity_skirt_brim integrated_parity_wipe_tower` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_skirt_brim` — FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_wipe_tower` — FACT pass/fail
- Exit condition: both pass. Tolerance-widening and fatal-`Err` handling exactly as Step 4a1.

### Step 4e: External-override integration test (AC-N2)

- Task IDs: `ADR-0056`
- Objective: author `full_coverage_external_override_tdd.rs` — the integration test proving that for each of the sixteen newly-integrated modules, an external module of the same `module.id` on a disk search root forces the wasm path (`LiveModuleBinding.native_entry: None`, `wasm_component: Some(..)`) — and register its `mod` line in `tests/integration/main.rs`. This is the test 205b later extends to its two modules.
- Precondition: Steps 2 and 4a1-4d2 landed (the sixteen modules are registered and natively entered); `cargo xtask build-guests --check` reports clean.
- Postcondition: `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` passes and is independently red or green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/hybrid_pilot_external_override_tdd.rs` — packet 204's external-override test (the pattern to follow)
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` — `load_live_modules_for_plan_with_integrated`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds: `modules/core-modules/**/src/**`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/**`, `crates/slicer-wasm-host/src/marshal/**`.
- Blast-radius discipline: not applicable — new test file.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` pass? scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs: `docs/adr/0056-integrated-modules-native-dispatch.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` — FACT pass/fail (AC-N2)
- Exit condition: it passes; each covered module's external override forces `native_entry: None`, `wasm_component: Some(..)`.

### Step 5: pnp-cli passthrough features and the coverage-gate proof

- Task IDs: `ADR-0057`
- Objective: declare `integrated-<name> = ["slicer-integrated-modules/<name>"]` in `crates/pnp-cli/Cargo.toml` for the sixteen, and prove the coverage gate now reports only the two transport-blocked modules.
- Precondition: Steps 2-3 and 4a-4d landed.
- Postcondition: `cargo check -p pnp-cli --all-targets` passes; `cargo xtask dist --edition integrated --plan` exits `1` naming exactly `path-optimization-default` and `machine-gcode-emit`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/Cargo.toml` — whole file
  - `dist/editions.toml` — whole file
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/Cargo.toml`
- Files explicitly out of bounds: `crates/slicer-integrated-modules/**`, `modules/core-modules/**`, `xtask/**`, `dist/editions.toml`.
- Blast-radius discipline: not applicable — additive, off-by-default cargo features; `default = ["report"]` is not modified.
- Expected sub-agent dispatches:
  - Question: does AC-5's command print `PASS`? scope: repo root; return: `FACT` `PASS` / the single `FAIL` line
- Context cost: `S`
- Authoritative docs: `docs/spec_packets/205-editions-xtask-dist-ci/packet.spec.md` AC-7.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p pnp-cli --all-targets` — FACT pass/fail
  - AC-5's `sh -c` command from `packet.spec.md` — FACT `PASS` / `FAIL`
- Exit condition: every one of the sixteen has a passthrough feature whose body names the `slicer-integrated-modules` feature of the identical name, `cargo check -p pnp-cli --all-targets` passes, and the coverage gate names exactly the two transport-blocked modules.

### Step 6: Doc surfaces and the plan's follow-on note

- Task IDs: `ADR-0057`
- Objective: update the two doc surfaces per `packet.spec.md` §Doc Impact Statement.
- Precondition: Step 5 landed.
- Postcondition: both doc greps pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/01_system_architecture.md` — §"Producing the tier-4 layout: `cargo xtask dist`" only
  - `docs/specs/multi-edition-distribution-plan.md` — §"Also unscheduled" only
- Files allowed to edit (at most 3):
  - `docs/01_system_architecture.md`
  - `docs/specs/multi-edition-distribution-plan.md`
- Files explicitly out of bounds: `docs/adr/**`, `CONTEXT.md`, `docs/07_implementation_status.md`, any other `docs/*.md`.
- Blast-radius discipline: not applicable — prose only.
- Expected sub-agent dispatches:
  - Question: which files still name the pre-205a coverage state? scope: `docs/`; return: `LOCATIONS` (≤10 entries)
- Context cost: `S`
- Authoritative docs: `docs/adr/0057-three-editions-and-integrated-tier.md`.
- OrcaSlicer refs: none.
- Verification:
   - `sh -c 'rg -q "path-optimization-default" docs/01_system_architecture.md && rg -q "205a" docs/specs/multi-edition-distribution-plan.md && echo PASS'` — FACT `PASS` / `FAIL`
- Exit condition: both greps pass.

### Step 7: DEVIATION_LOG row for a widened parity tolerance (conditional — only if triggered by Steps 4a1-4d2)

- Task IDs: `ADR-0056`
- Objective: record any measured residual divergence that forced a coordinate-tolerance widening during Steps 4a1-4d2, so the deviation is owned and auditable rather than silently absorbed into a comparator constant.
- Precondition: at least one of Steps 4a1-4d2 widened a `ParityTolerance` past the `1e-3` mm default (with a measured maximum per-point delta, per `design.md` §Open Questions). **If no widening occurred, this step is skipped in its entirety and `docs/DEVIATION_LOG.md` is not touched.**
- Postcondition: `docs/DEVIATION_LOG.md` carries one new row naming the module(s), the measured maximum per-point delta, the widened tolerance, and the rationale; the row uses the next DEV-### after the highest DEV-### present in the file, re-derived at write time.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the header/format and the highest DEV-### row only (the file is over 300 lines; do not read it whole)
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds: `docs/adr/**`, `CONTEXT.md`, `docs/07_implementation_status.md`, every comparator source file (the widening itself landed in Steps 4a1-4d2; this step is prose only).
- Blast-radius discipline: not applicable — prose only.
- Expected sub-agent dispatches:
  - Question: what is the highest DEV-### currently in `docs/DEVIATION_LOG.md`? scope: `docs/DEVIATION_LOG.md`; return: `FACT` (the id only)
- Context cost: `S`
- Authoritative docs: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision.
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'rg -q "DEV-1[0-9][0-9]" docs/DEVIATION_LOG.md && echo PASS'` — FACT `PASS` / `FAIL` (the new row id must exceed the pre-step highest DEV-###, re-derived at write time)
- Exit condition: the row exists with the measured number and rationale, or the step was verifiably skipped because no widening occurred.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only reconciliation of sixteen modules + transport scope |
| Step 2 | M | Registry features + native entries |
| Step 3 | M | New family comparators + self-tests |
| Step 4a1 | S | Infill parity (gyroid-infill, lightning-infill) |
| Step 4a2 | S | Infill parity (rectilinear-infill, top-surface-ironing) |
| Step 4a3 | S | InfillPostProcess parity (infill-linker) |
| Step 4b1 | S | Support parity (traditional-support, tree-support) |
| Step 4b2 | S | SupportPostProcess/PerimetersPostProcess parity (support-surface-ironing, fuzzy-skin) |
| Step 4b3 | S | PerimetersPostProcess parity (seam-placer) |
| Step 4c | M | PrePass parity (layer-planner-default, seam-planner-default) |
| Step 4d1 | S | Finalization parity (overhang-classifier-default, part-cooling) |
| Step 4d2 | S | Finalization parity (skirt-brim, wipe-tower) |
| Step 4e | S | External-override integration test, AC-N2 |
| Step 5 | S | pnp-cli passthrough features + coverage proof |
| Step 6 | S | Two doc surfaces |
| Step 7 | S | Conditional DEVIATION_LOG row (skipped if no tolerance widening) |

Aggregate: `L` (sixteen modules). No single step is L; the packet is split into per-family steps. Per the swarm escalation protocol, this packet runs in the extended band or is split further at activation.

## Packet Completion Gate

- All steps (1, 2, 3, 4a-4d, 5, 6, and 7-if-triggered) and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` carries no TASK row for this program; do **not** invent one and do **not** edit that file while the parallel 194-199 session is active.
- Reconcile reopened/superseded status transitions: none — this packet supersedes nothing.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (AC-1 … AC-7, AC-N1 … AC-N3) and the three packet-level gate commands.
- Run `cargo xtask build-guests --check` immediately before the AC-3 re-run; a `STALE:` report invalidates the parity proof.
- This packet does **not** close the plan — `--edition integrated` still fails on the two transport-blocked modules. Record that 205b (transport completion for `Layer::PathOptimization` and gcode-command application, then integration of `path-optimization-default` + `machine-gcode-emit`) is the required follow-on that makes the Integrated edition build.
- Confirm context stayed at or below the band's limit, or record a logged swarm ESCALATION.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
