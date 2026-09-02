# Implementation Plan: top-surface-ironing-keys

## Execution Rules

- Work one atomic step at a time; map every step to [21 - Author packet P14 - Quality / Ironing - top-surface-ironing](../specs/orca-feature-gap/issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md); this queue packet has `task_ids: []`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the canonical top manifest and schema guard

- Task IDs: `[]` (wayfinder ticket 21)
- Objective: replace the top manifest's `ironing_enabled` table with exact `ironing_type`, `ironing_angle`, `ironing_angle_fixed`, and `ironing_inset` tables and add a TOML-direct-parse guard.
- Precondition: `top-surface-ironing.toml` has the existing five-key schema, including `ironing_enabled`; no P14 packet directory or test guard claims these keys.
- Postcondition: the four tables have the exact AC-1 types/defaults/bounds/values/display/group; the top manifest has no `ironing_enabled`; the guard binary is auto-discovered and passes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` - full manifest; it is short.
  - `modules/core-modules/top-surface-ironing/Cargo.toml` - relevant package and dev-dependency sections.
  - `modules/core-modules/part-cooling/Cargo.toml` - TOML dev-dependency precedent only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`
  - `modules/core-modules/top-surface-ironing/Cargo.toml`
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_config_schema_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-surface-ironing/**` - P15-owned.
  - `crates/slicer-scheduler/**` - Step 4.
  - `docs/15_config_keys_reference.md` - generated in Step 5.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: none; manifest data and a new test target add no Rust struct field or schema constant.
- Expected sub-agent dispatches:
  - Question: is `toml = "0.8"` absent in the top module and is the module using Cargo test auto-discovery with no explicit aggregator?; scope: the top module `Cargo.toml` and `tests/`; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of exact schema forms.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegated `LOCATIONS` already captured in `requirements.md`; re-dispatch only if a default/bound is disputed.
- Verification:
  - `cargo test -p top-surface-ironing --test top_surface_ironing_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: the schema guard passes and a targeted TOML search confirms no top-manifest `ironing_enabled` table remains.

### Step 2: Wire mode, angle, fixed-angle, and inset behavior with emission invariants

- Task IDs: `[]` (wayfinder ticket 21)
- Objective: make `TopSurfaceIroning::from_config` parse the four keys and make `TopSurfaceIroning::run_infill` apply the four-mode selection and deterministic angle/inset behavior.
- Precondition: Step 1's manifest and guard pass; existing top emission fixtures still compile against the module entry.
- Postcondition: `no ironing`, `top`, `topmost`, and `solid` select exactly the surfaces in AC-2; invalid modes fail; angle/fixed/inset invariants pass; default layer-0 output remains the horizontal baseline aside from the required zero-inset inward boundary.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/top-surface-ironing/src/lib.rs` - config parser, `IRONING_LINE_WIDTH`, generator, and `run_infill` only.
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` - fixture helpers and existing path assertions.
  - `crates/slicer-sdk/src/views.rs` - `SliceRegionView` fill accessors/setters only.
  - `crates/slicer-sdk/src/host.rs` - `offset_polygons` and `OffsetJoinType` signatures only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/top-surface-ironing/src/lib.rs`
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/**` and `crates/slicer-schema/wit/**` - no boundary change.
  - `modules/core-modules/support-surface-ironing/**` - P15-owned.
  - `crates/slicer-sdk/src/host.rs` and `crates/slicer-core/src/polygon_ops.rs` - use existing helpers, do not edit.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: no public struct or schema constant is added; any new mode enum is module-private and tests stay in the existing binary.
- Expected sub-agent dispatches:
  - Question: confirm the exact generator signature, `SliceRegionView` fill accessors, and offset helper signature before editing; scope: the files named above; return: `LOCATIONS`.
  - Question: confirm canonical mode/inset/angle invariants and the unsupported base-direction limitation; scope: delegated Orca files in `requirements.md`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct conversion contract.
  - `docs/03_wit_and_manifest.md` - delegated config-view reachability summary.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` - `Layer::make_ironing` and `Fill::fill_surface`, delegated only.
  - `OrcaSlicerDocumented/tests/fff_print/test_fill.cpp` - angle invariant, delegated only.
- Verification:
  - `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: the existing emission binary passes with named mode, angle, inset, and legacy-key tests, and production `run_infill` invokes the updated generator.

### Step 3: Migrate top-owned configs and integration fixtures

- Task IDs: `[]` (wayfinder ticket 21)
- Objective: replace top-owned `ironing_enabled` inputs with `ironing_type = "topmost"` where the fixture intends current active top ironing, while leaving the support-owned config untouched.
- Precondition: Step 2 parses the canonical key and the top module still emits for `topmost`.
- Postcondition: top contract, executor, e2e, and resource fixtures use the canonical top key; the support contract remains on its separate `ironing_enabled` key; the targeted runtime tests pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs` - top config and test setup.
  - `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs` - top config and assertion comments.
  - `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs` - matched config JSON/comments only; ranged reads.
  - `resources/test_config/benchy_combined_feature_evidence.json` - matched top config entry only.
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` - read-only support boundary check.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`
  - `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`
  - `resources/test_config/benchy_combined_feature_evidence.json`
- Additional atomic substep (same objective, separate edit cap):
  - `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs` only; update its top-owned JSON/comment occurrences and run the e2e binary.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` and all support module files - P15-owned.
  - `docs/15_config_keys_reference.md` - Step 5.
- Blast-radius discipline: test/fixture key strings only; no struct fields or public constants.
- Expected sub-agent dispatches:
  - Question: confirm the matched occurrences are top-owned and the support parity fixture is excluded; scope: the four named config/test paths; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: none beyond the existing test setup and packet scope.
- OrcaSlicer refs: none; this is key migration, not new canonical logic.
- Verification:
  - `cargo test -p slicer-runtime --test contract integrated_parity_top_surface_ironing 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test executor cube_4color_ironing_per_painted_top_color 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test e2e slicing_promotion_e2e_dispatch_regression 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: all top-owned integration tests pass and `rg` shows no top-owned `ironing_enabled` occurrences outside historical/docs records; the support contract still contains its live support gate.

### Step 4: Add real-manifest bounds and CONFIG_BLOCK exact-once coverage

- Task IDs: `[]` (wayfinder ticket 21)
- Objective: prove scheduler enforcement for the four declarations and raw CONFIG_BLOCK serialization without adding padding twins.
- Precondition: Steps 1-3 are green; the top manifest exists in the scheduler fixture path and the runtime driver can run with raw config.
- Postcondition: invalid enum/negative values reject, boundary values resolve, explicit four-key CONFIG_BLOCK lines appear exactly once, and default padding remains `ironing_type = no ironing` only.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - module loading and existing error assertion arms.
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - `config_block_includes_user_passed` and related driver setup.
  - `crates/slicer-gcode/src/serialize.rs` - `serialize_config_block`, `emit_config_kv`, and ironing padding entries only.
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` - read-only; no padding edits.
  - `crates/slicer-scheduler/src/config_resolution.rs` - existing enforcement machinery is not changed.
  - `docs/15_config_keys_reference.md` - Step 5.
- Blast-radius discipline: test-only additions to existing aggregated binaries; no new test file or aggregator is needed.
- Expected sub-agent dispatches:
  - Question: does the bounds binary load the real top manifest and what exact error assertion shape should the four arms use?; scope: scheduler test and config-resolution paths; return: `FACT`.
  - Question: do explicit raw keys reach `serialize_config_block` and deduplicate the existing `ironing_type` padding line?; scope: runtime integration test and named serializer functions; return: `FACT`.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY of CONFIG_BLOCK contract.
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of inclusive bounds.
- OrcaSlicer refs: none; this step tests host plumbing.
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: both binaries pass and their assertions distinguish `TypeMismatch`, `OutOfRange`, exact-once explicit values, and the existing padded default.

### Step 5: Regenerate docs and close packet gates

- Task IDs: `[]` (wayfinder ticket 21)
- Objective: regenerate the generated module-key reference, verify the support/top ownership boundary, rebuild/check guest artifacts, and run the packet's closure gates.
- Precondition: Steps 1-4 pass and all manifest/source/config changes are present.
- Postcondition: `docs/15_config_keys_reference.md` is generated, its `--check` passes, no P14 deviation row is introduced, guest freshness returns exit 0, and workspace check/clippy pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - targeted `rg` probes only; never load in full.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` - generated only through `cargo xtask gen-config-docs`.
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained source snapshot.
  - `modules/core-modules/support-surface-ironing/**` - P15-owned.
  - `target/`, `Cargo.lock`, generated code other than doc output - never load.
- Blast-radius discipline: none; regeneration changes only generated documentation.
- Expected sub-agent dispatches:
  - Question: after regeneration, do the four top rows exist, is the top `ironing_enabled` row gone, is the support row retained, and are there no P14 deviation rows?; scope: generated doc and xtask output; return: `FACT`.
  - Question: run `cargo xtask build-guests --check` and report exit code; if stale, rebuild without `--check` first; scope: xtask and guest artifacts; return: `FACT`.
- Context cost: `S`
- Authoritative docs: none; generated output and gates are the evidence.
- OrcaSlicer refs: none; canonical evidence is captured in earlier steps.
- Verification:
  - `cargo xtask gen-config-docs` - FACT exit code.
  - `cargo xtask gen-config-docs --check` - FACT exit code.
  - `cargo xtask build-guests --check` - FACT exit code; stale means rebuild and repeat.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - Full matrix from `requirements.md` - FACT pass/fail per command.
- Exit condition: every packet AC and gate command passes; generated docs prove the four top keys and preserve the support boundary; guest freshness is exit 0.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Short manifest plus one direct TOML guard |
| Step 2 | M | Existing geometry generator and mode tests |
| Step 3 | M | Top-owned runtime fixtures, with the e2e file split into its own edit substep |
| Step 4 | M | Two existing integration drivers |
| Step 5 | S | Generated output and delegated gates |

Aggregate: `M`; no step is `L`, so no split is required before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is N-A for this queue packet (`task_ids: []`); implementation/authoring is recorded against wayfinder ticket 21 and the crosswalk is re-derived at completion.
- No reopened or superseded packet transition exists.
- `packet.spec.md` remains `draft` until an explicit activation decision; it is otherwise ready for implementation.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: exact canonical relative angle requires future solid-infill orientation metadata; P14's tested fallback is deterministic and explicit.
- Confirm context stayed within the standard band; no extended-band escalation is permitted for this packet.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use the required workspace/all-target or tee conventions.
