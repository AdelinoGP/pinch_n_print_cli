# Implementation Plan: typed-modifier-kind

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-569`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Re-run the literal and production-subtype inventories before editing; the grounded lists are authoring evidence, not mutable ledger facts.
- Every cargo invocation is delegated and tees combined output to `target/test-output.log`.

## Steps

### Step 1: Reconcile dependencies and freeze inventories

- Task IDs: `TASK-569`
- Objective: confirm packet-03/05/07 landed API shapes, exactly ten production classification sites, every `ModifierVolume` literal, every old-version assertion, and the shared runtime composition seam.
- Precondition: packets 05 and 07 are implemented; packet 03 exports are landed.
- Postcondition: bounded inventories identify all edit files and no scope/API mismatch remains.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/config-scope-resolution_03_typed-scope-ingestion/{packet.spec.md,requirements.md,design.md}`
  - `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{packet.spec.md,requirements.md,design.md}`
  - `docs/spec_packets/config-scope-resolution_07_scope-eligibility/{packet.spec.md,requirements.md,design.md}`
- Files allowed to edit (at most 3):
  - None; discovery only.
- Files explicitly out of bounds:
  - Production code, tests, docs, fixtures, and predecessor packets.
- Blast-radius discipline:
  - Dispatch `LOCATIONS` for every `ModifierVolume {`, `ModifierScope`, `applies_to`, `CURRENT_MESH_IR_SCHEMA_VERSION`, hard-coded MeshIR 1.1.0 assertion, and production subtype read. Record all results before Step 2; do not rely on compilation discovery.
- Expected sub-agent dispatches:
  - Question: return landed config API signatures and the common entry-point seam; scope: named packet/code files; return: `FACT` ≤5 lines.
  - Question: return all literal/old-surface/version locations in ≤20-entry batches; scope: `crates/**/*.rs`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — row 8, Modifiers, and owner decision 2.
  - `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — accepted shape.
- OrcaSlicer refs:
  - None in this discovery step.
- Verification:
  - `python3 -c "from pathlib import Path; assert all(Path(p).exists() for p in ['crates/slicer-ir/src/slice_ir.rs','crates/slicer-model-io/src/loader.rs','crates/slicer-runtime/src/run.rs'])"` — FACT pass/fail.
- Exit condition: the dependency signatures reconcile without scope change and inventories cover ten production reads plus all literal/version fallout.

### Step 2: Add the typed enum and a construction compatibility seam

- Task IDs: `TASK-569`
- Objective: add `ModifierKind` plus a temporary `ModifierVolume` constructor and `kind()` accessor so all existing literals/consumers can migrate before the struct field changes; record the live MeshIR version without bumping it yet.
- Precondition: Step 1 inventories are complete; every literal site is assigned to Step 2 or Steps 3–5 in a compilation-safe edit sequence.
- Postcondition: the enum is public and serde-capable; the constructor accepts an explicit kind; `kind()` returns it through a temporary legacy-compatible representation; the struct shape/version remain unchanged until Step 8.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — schema constants and modifier types only.
  - `crates/slicer-ir/src/lib.rs` — modifier exports only.
  - `crates/slicer-ir/tests/ir_tests.rs` — modifier and schema tests only.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/lib.rs`
  - `crates/slicer-ir/tests/ir_tests.rs`
- Files explicitly out of bounds:
  - Loader, core/runtime consumers, docs, WIT, and other IR constants.
- Blast-radius discipline:
  - No struct field or version changes here. The temporary constructor/accessor exists solely to reduce the later field blast radius; it must not be published as a completed packet state or documented as a lasting subtype-string API.
- Expected sub-agent dispatches:
  - Question: verify old-version assertion locations and representative legacy serde payload shape; scope: slicer-ir tests only; return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — IR Versioning Contract and MeshIR current shape.
  - `docs/specs/config-scope-resolution-plan.md` — owner decision 2.
- OrcaSlicer refs:
  - None; this is a PnP IR contract.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-ir --all-targets --test ir_tests typed_modifier_kind_bridge_contract -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "test typed_modifier_kind_bridge_contract .* ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: all four enum variants round-trip and the explicit-kind constructor/accessor supports migration without changing `ModifierVolume` fields or the MeshIR version.

### Step 3: Map the loader once and migrate model/core construction sites

- Task IDs: `TASK-569`
- Objective: map four non-normal `PartSubtype` values once through the compatibility constructor, reserve `subtype`, and migrate model/core literals to explicit kinds without changing geometry behavior.
- Precondition: `ModifierKind` exists; Step 1 literal inventory is available.
- Postcondition: loaded modifiers and slicer-core construction sites supply explicit kinds through the constructor; the temporary bridge owns any legacy subtype representation until Step 8, so callers never insert/read it directly.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/loader.rs` — `resolve_object` only.
  - `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` — sidecar fixture helper and subtype tests.
  - Assigned slicer-core literal files from Step 1 — only literal helpers.
- Files allowed to edit (at most 3 per atomic substep):
  - Substep 3a: `crates/slicer-model-io/src/loader.rs`, `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`
  - Substep 3b: `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs`, `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs`
  - Substep 3c: `crates/slicer-core/tests/algo_region_mapping_tdd.rs`, `crates/slicer-core/tests/paint_segmentation_base_fallback_tdd.rs`, `crates/slicer-core/tests/paint_segmentation_multi_object_isolation_tdd.rs`
- Files explicitly out of bounds:
  - Runtime consumers/tests, config resolver, docs, WIT, and binary fixtures.
- Blast-radius discipline:
  - Each assigned `ModifierVolume` literal gains an explicit semantically correct kind. Test helpers must not infer kind by duplicating production string mapping.
- Expected sub-agent dispatches:
  - Question: verify every model/core literal from Step 1 is assigned exactly once and no `ModifierScope` import remains there; scope: listed files; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — host-local/IR seam.
  - `docs/02_ir_schemas.md` — current sidecar routing behavior.
- OrcaSlicer refs:
  - None; mapping preserves existing PnP parser behavior.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd modifier_parts_cross_mesh_ir_with_typed_kinds -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "test modifier_parts_cross_mesh_ir_with_typed_kinds .* ok" target/test-output.log'` — FACT pass/fail.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --all-targets --test algo_region_mapping_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: four-kind mapping is exact, `NormalPart` creates no modifier, callers do not author subtype routing entries, and every assigned model/core literal is migrated.

### Step 4: Replace all five slicer-core classifications exhaustively

- Task IDs: `TASK-569`
- Objective: replace the five grounded slicer-core subtype reads with exhaustive matches on `mv.kind()` while preserving the existing support-kind-only exclusion and the parameter/negative-part routes; Step 8 will make the accessor field-backed.
- Precondition: loader/core literals carry typed kind.
- Postcondition: the three region-mapping and two paint-segmentation sites classify only through the enum.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/region_mapping.rs` — the three named symbols only.
  - `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs` — `slice_modifier_volumes` only.
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — `mesh_has_any_paint` only.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/region_mapping.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- Files explicitly out of bounds:
  - Runtime, loader, resolver API, docs, and WIT.
- Blast-radius discipline:
  - No struct field is added here; any in-file test literal already migrated in Step 3.
- Expected sub-agent dispatches:
  - Question: confirm exactly five production matches in these three files and no wildcard enum arm; scope: listed symbols; return: `LOCATIONS` ≤10.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — parameter geometry.
  - `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — exhaustive routing.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — delegated function-name summary of preserved support exclusion.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --all-targets --test algo_region_mapping_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
  - `bash -lc '! rg -n "config_delta\.fields\.get\(\"subtype\"\)" crates/slicer-core/src --glob "*.rs"'` — FACT pass/fail.
- Exit condition: all five sites explicitly handle all four variants and focused geometry/config tests pass.

### Step 5: Migrate runtime literals and retire fictional scope tests

- Task IDs: `TASK-569`
- Objective: assign explicit kinds to every runtime literal and remove tests whose oracle depends on unread `ModifierScope` behavior.
- Precondition: Step 1's runtime literal/old-scope inventory is complete.
- Postcondition: all runtime source/test constructors use `kind`; no `ModifierScope` or `applies_to` remains; truthful existing behavior tests are preserved.
- Files allowed to read, with ranges when over 300 lines:
  - Only Step 1-listed literal helpers and the three obsolete `ModifierScope` tests `modifier_resolution_picks_highest_priority_within_scope`, `modifier_resolution_breaks_priority_ties_deterministically_by_id`, `all_features_modifier_participates_in_every_scope_resolution` (helper `resolve_winner_for_scope`) in `acceptance_gate_gaps_tdd.rs`.
- Files allowed to edit (at most 3 per atomic substep):
  - Substep 5a: `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, `crates/slicer-runtime/tests/executor/cube_fuzzy_painted_tdd.rs`, `crates/slicer-runtime/tests/executor/modifier_region_split_tdd.rs`
  - Substep 5b: `crates/slicer-runtime/tests/contract/modifier_split_subregion_density_tdd.rs`, `crates/slicer-runtime/tests/e2e/acceptance_gate_gaps_tdd.rs`, `crates/slicer-runtime/tests/e2e/cube_4color_modifier_part_e2e_tdd.rs`
  - Substep 5c: `crates/slicer-runtime/tests/e2e/mixed_density_internal_bridge_rejection_e2e_tdd.rs`, `crates/slicer-runtime/tests/e2e/modifier_support_type_family_e2e_tdd.rs`, `crates/slicer-runtime/tests/e2e/threemf_subtypes_synthetic_e2e_tdd.rs`
  - Substep 5d: `crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs`
- Files explicitly out of bounds:
  - Production classification logic, resolver wiring, docs, WIT, and unrelated acceptance-gap tests.
- Blast-radius discipline:
  - Every Step 1 runtime literal appears in exactly one substep. Retire only tests whose helper filters `.applies_to`; do not weaken unrelated assertions.
- Expected sub-agent dispatches:
  - Question: compare Step 1 runtime inventory with assigned substeps and report omissions/duplicates; scope: listed files; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` — retire-if-unjustified and false-oracle rules.
  - `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — old field is inert.
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -lc '! rg -n "ModifierScope|\.applies_to|applies_to:" crates --glob "*.rs"'` — FACT pass/fail.
  - `cargo xtask check-literals` — FACT pass/fail.
- Exit condition: exhaustive search finds no old surface and no unrelated test was deleted or weakened.

### Step 6: Route modifier deltas through registry admission in both entry points

- Task IDs: `TASK-569`
- Objective: author runtime e2e tests, ingest each modifier delta through packet-03 scope typing, validate packet-07 eligibility, and resolve it through packet-05 instead of direct extension copying.
- Precondition: forward dependency APIs are reconciled and modifiers have typed kind/IDs.
- Postcondition: both production entry points share one registry-backed modifier ingestion path; typed fields/extensions resolve and invalid deltas fail atomically.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` — common setup plus both named entry points only.
  - Final landed `crates/slicer-config/src/{ingestion.rs,resolution.rs}` — named APIs only.
  - `crates/slicer-runtime/tests/e2e/main.rs` — module registration only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/tests/e2e/main.rs`
  - `crates/slicer-runtime/tests/e2e/typed_modifier_kind_tdd.rs`
- Files explicitly out of bounds:
  - Scheduler compatibility resolvers, guest/WIT code, manifests, packet-09 layer ranges, and test fixtures unrelated to modifiers.
- Blast-radius discipline:
  - New watched-type literals use FRU or an exhaustive waiver; no public struct field is introduced.
- Expected sub-agent dispatches:
  - Question: verify both entry points reach the same helper and errors preserve exact key/scope; scope: run.rs and new e2e module; return: `LOCATIONS` ≤10.
- Context cost: `M`
- Authoritative docs:
  - Packet 03/05/07 final contracts — exact APIs after reconciliation.
  - `docs/02_ir_schemas.md` — modifier precedence.
- OrcaSlicer refs:
  - None; registry policy is governed by ADR-0069/0070.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: AC-3 and AC-N1 pass and direct untyped extension stamping is absent from the modifier path.

### Step 7: Replace all five runtime classifications exhaustively

- Task IDs: `TASK-569`
- Objective: convert the five grounded runtime production sites to explicit four-variant matches on `kind()` and preserve all route behavior; Step 8 will make the accessor field-backed.
- Precondition: runtime literals are migrated and e2e route tests exist.
- Postcondition: runtime production has no subtype read/string comparison and all four routes pass.
- Files allowed to read, with ranges when over 300 lines:
  - Only the five named symbols in the listed runtime files.
- Files allowed to edit (at most 3 per atomic substep):
  - Substep 7a: `crates/slicer-runtime/src/region_partition.rs`, `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-runtime/src/negative_part_subtract.rs`
  - Substep 7b: `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
- Files explicitly out of bounds:
  - Other runtime stage logic, config APIs, docs, WIT, and guest code.
- Blast-radius discipline:
  - No field addition; in-source support-analysis test literals were migrated in Step 5.
- Expected sub-agent dispatches:
  - Question: confirm exactly five runtime matches, all explicit, with no wildcard and no subtype-string read; scope: named symbols; return: `LOCATIONS` ≤10.
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — current negative/support/partition route ownership.
  - `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — exhaustive-match requirement.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — delegated preserved-route summary.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd::all_modifier_kinds_keep_their_geometry_routes -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "typed_modifier_kind_tdd::all_modifier_kinds_keep_their_geometry_routes .* ok" target/test-output.log'` — FACT pass/fail.
  - `bash -lc '! rg -n "config_delta\.fields\.get\(\"subtype\"\)" crates/slicer-runtime/src --glob "*.rs"'` — FACT pass/fail.
- Exit condition: all five sites handle every `ModifierKind` explicitly and route behavior is unchanged.

### Step 8: Finalize the ModifierVolume field, legacy serde, and MeshIR minor bump

- Task IDs: `TASK-569`
- Objective: after all literals use the constructor, replace the temporary subtype-backed bridge with `ModifierVolume.kind`, remove `applies_to`/`ModifierScope`, implement one-way legacy deserialization, and increment the activation-recorded MeshIR version by exactly one minor.
- Precondition: Steps 3 and 5 migrated every Step-1 literal; Steps 4 and 7 classify only through `kind()`; no caller authors subtype.
- Postcondition: `ModifierVolume` has exactly `id`, `mesh`, `config_delta`, `priority`, and `kind`; the accessor is field-backed; serialization emits no legacy surface; representative legacy payloads recover every kind; old scope exports are gone.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` — modifier types, serde, and MeshIR constant only.
  - `crates/slicer-ir/src/lib.rs` — modifier exports only.
  - `crates/slicer-ir/tests/ir_tests.rs` — modifier/version/serde tests only.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/lib.rs`
  - `crates/slicer-ir/tests/ir_tests.rs`
- Files explicitly out of bounds:
  - All migrated constructor call sites, production consumers, other IR constants, docs, WIT, and fixtures.
- Blast-radius discipline:
  - Step 1's literal inventory must now return zero direct `ModifierVolume {` sites outside `slice_ir.rs` and the intentional serde test. This step owns the centralized constructor field initialization, public export deletion, live constant bump, and every test that asserts the old MeshIR version/shape.
- Expected sub-agent dispatches:
  - Question: prove no direct literal remains outside this step and list every old-version assertion; scope: `crates/**/*.rs`; return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — owner decision 2.
  - `docs/02_ir_schemas.md` — IR Versioning Contract.
- OrcaSlicer refs:
  - None; this is a PnP compatibility boundary.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-ir --all-targets --test ir_tests typed_modifier_kind_mesh_ir_minor_contract -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "test typed_modifier_kind_mesh_ir_minor_contract .* ok" target/test-output.log; ! rg -n "ModifierScope|applies_to" crates/slicer-ir/src --glob "*.rs"'` — FACT pass/fail.
- Exit condition: AC-4 passes, the version is one activation-derived minor higher, and no unassigned struct-literal/version fallout exists.

### Step 9: Add visual-debug and documentation closure

- Task IDs: `TASK-569`
- Objective: prove modifier geometry via a deterministic visual-debug bundle and update the three authoritative contract documents.
- Precondition: typed routing and registry delivery pass focused tests.
- Postcondition: visual bundle exists with the required manifest/image and docs describe only the new contract.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` — Request Shape and RegionMapping silhouette sections only.
  - `docs/02_ir_schemas.md` — MeshIR/modifier sections only.
  - `docs/04_host_scheduler.md` — RegionMapping/modifier routing sections only.
- Files allowed to edit (at most 3 per atomic substep):
  - Substep 9a: `crates/pnp-cli/tests/typed_modifier_kind_visual_debug_tdd.rs`
  - Substep 9b: `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md`
- Files explicitly out of bounds:
  - Visual-debug production code, other docs/ADRs, binary fixtures, and layer-range material.
- Blast-radius discipline:
  - Test fixtures construct watched types with FRU or exhaustive waivers; no production struct changes.
- Expected sub-agent dispatches:
  - Question: run the visual-debug test and return manifest/image assertion verdict only; scope: exact test; return: `FACT` ≤5 lines.
  - Question: verify required/forbidden doc strings from AC-6; scope: three docs; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/19_visual_debug.md` — bundle contract.
  - `docs/22_test_quality.md` — independent observable assertions.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — cite canonical behavior by function name only in updated prose.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --all-targets --test typed_modifier_kind_visual_debug_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
  - `python3 -c "from pathlib import Path; a=Path('docs/02_ir_schemas.md').read_text(); assert 'ModifierKind' in a and 'ModifierScope' not in a and 'applies_to' not in a"` — FACT pass/fail.
- Exit condition: AC-5/AC-6 pass and docs contain no stale old-scope contract.

### Step 10: Run packet closure gates

- Task IDs: `TASK-569`
- Objective: run freshness, focused suites, all-target compilation/lint, literal, and touched-test quality gates without implementation changes.
- Precondition: Steps 2–9 pass their narrow validations.
- Postcondition: every packet gate passes or a bounded failure is reported without weakening tests.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` — only failing-test windows or summary lines.
- Files allowed to edit (at most 3):
  - None unless a failure sends work back to its owning step.
- Files explicitly out of bounds:
  - Unrelated code/tests, Cargo.lock, generated code, and fixture baselines.
- Blast-radius discipline:
  - Step 1 inventory is rechecked before claiming completion; no field/version fallout may remain.
- Expected sub-agent dispatches:
  - Question: run each exact gate and return pass/fail with ≤20 failure lines; scope: named command; return: `FACT` or `SNIPPETS`.
- Context cost: `S`
- Authoritative docs:
  - `AGENTS.md` — test entry, WASM freshness, literal, and quality gates.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo xtask build-guests --check` — FACT exit code.
  - `cargo check --workspace --all-targets` — FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail.
  - `cargo xtask check-literals` — FACT pass/fail.
  - `cargo xtask check-test-quality --report` — FACT touched-file findings.
- Exit condition: all required gates pass and every pipe-suffixed AC can be independently re-dispatched.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Dependency and complete blast-radius inventories |
| Step 2 | S | IR/serde/version contract |
| Step 3 | S | Loader and construction migration |
| Step 4 | S | Five slicer-core matches |
| Step 5 | S | Runtime literal/test migration |
| Step 6 | M | Shared registry/admission wiring |
| Step 7 | S | Five runtime matches |
| Step 8 | S | Centralized field/serde/version finalization |
| Step 9 | S | Visual and docs closure |
| Step 10 | S | Delegated gates |

Aggregate remains `M`; no step is `L`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile packet-03/05/07 forward dependencies and record final exports for downstream packets.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, including any legacy serde assumptions.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations use `--all-targets`; every test invocation tees combined output to `target/test-output.log` when executed.
