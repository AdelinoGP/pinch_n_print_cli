# Implementation Plan: resolved-config-view

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-567`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Delegate every cargo command and tee its output to `target/test-output.log`.
- `cargo test` commands never carry `--all-targets`, because it overrides `--test`. `check` and `clippy` always carry it.
- Each substep edits at most three files. Every file a substep names must appear in `design.md` §Files in Scope or in the Step 1 inventory.
- Substeps inherit their parent step's Task IDs, authoritative docs, OrcaSlicer refs, and files-to-read unless they override them.
- Every substep states its precondition, postcondition, and exit condition.

## Steps

### Step 1: Reconcile prerequisites and freeze blast radii

- Task IDs: `TASK-567`
- Objective: confirm packet 05's landed exports and record every inventory later steps depend on.
- Precondition: `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/packet.spec.md` reads `status: implemented`.
- Postcondition: an implementation note records each item below, using crate-qualified symbol names.
  1. The resolver signatures, and the `ScopedConfig` layer where registry defaults enter.
  2. Every call site, split into production and test, of:
     - `resolve_scope_stack`;
     - `bind_module_config_view`;
     - `build_live_execution_plan`;
     - `run_pipeline_with_raw_config` and `run_pipeline_with_instrumentation`;
     - `SliceOutcome {`.
  3. Every test that asserts whole-value `ResolvedConfig` equality or empty `extensions`.
  4. Every `HostKeyMeta`, `HostRuntimeKey`, `ConfigFieldEntry`, or `RegistryEntry` literal that lacks a `..` rest.
  5. Every read of `raw_config_source` in `crates/slicer-runtime/src/pipeline.rs`, and every prepass consumer of that map (e.g. `resolve_line_width_mm`), each with the key it reads.
  6. Host production reads of keys that have no registry entry.
     - Sources to cover: `ScopeDelta` values, `extensions`, raw or expanded source maps, the CONFIG_BLOCK map, and the key list in `docs/02_ir_schemas.md` §"CONFIG_BLOCK viewer-key contract".
     - For each key, record its absent-value behavior and the runtime-row type and default (`None` or `Some`) that preserve it.
     - Verified at preflight: `gcode_flavor`, `printer_model`, `filament_colour`, `extruder_colour`, `filament_cost`, `printable_area`, `support_type`, `support_family`, `thumbnails`, `machine_max_acceleration_retracting`, `extruder`.
  7. The 89 fallback sites, each with its key, getter, registry `field_type`, and whether the reading guest declares the key.
  8. Every other undeclared string-literal getter or wrapper read. Verified at preflight: the eleven listed in `design.md`.
  9. The AC-11 discriminating key: a key declared only by a loaded module's manifest, whose default differs from its `ORCA_CONFIG_PADDING` value.
  10. A guest batch table, at most three source files per batch.
  11. Guest tests, both `modules/core-modules/*/tests/*.rs` and `#[cfg(test)]`, that pass an empty or partial `ConfigView` into a code path containing a classified site. Each is assigned to the batch that edits that site.
  12. Tests anywhere in the workspace that assert unknown-key retention. Verified at preflight: `experimental_xyz` in `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, `unrelated_extension` in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, and packet 03's `crates/slicer-config/tests/typed_scope_ingestion_tdd.rs`.
- Files allowed to read, with ranges when over 300 lines: predecessor packet docs; the named symbols in the Files-in-Scope list; `docs/02_ir_schemas.md` §"CONFIG_BLOCK viewer-key contract"; the 13 guest production trees, through bounded searches.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds: code edits, fixture contents, `OrcaSlicerDocumented/**`, generated code.
- Blast-radius discipline: this step is the inventory. Later steps may not discover call sites by compile error.
- Expected sub-agent dispatches: the six listed in `design.md` §Expected Sub-Agent Dispatches. Prerequisite exports return `FACT` ≤5 lines; every other dispatch returns `LOCATIONS` ≤20 per query.
- Context cost: `S`
- Authoritative docs: plan RC-4/RC-5, "Guests and delivery", and queue row 6; the packet 03/05 export contracts.
- OrcaSlicer refs: none.
- Verification:
  - the fallback total is `29+27+10+4+3+3+3+2+2+2+2+1+1 = 89`, with no excluded category counted;
  - item 6 contains at least the eleven verified keys;
  - items 11 and 12 contain at least their verified files;
  - item 8 contains at least the eleven verified reads.
- Exit condition: do not start Step 2 if any of these holds:
  - a count mismatch;
  - an unresolved signature;
  - a surviving `overlay_object_layer_planning` or guest wildcard-instance read;
  - a host-consumed key whose absent-value behavior no runtime row can preserve.

### Step 2: Registry metadata, host-key registration, generated map, seeding, validation

- Task IDs: `TASK-567`
- Objective:
  - land `omit_from_config_block` on all channels;
  - make `HostRuntimeKey.default` optional and register the host-consumed keys;
  - generate `to_config_map` and `typed_field_keys`;
  - seed registry defaults before Phase-B expansion;
  - validate `extensions`;
  - add `config_block_map`.
- Precondition: Step 1 inventories are complete.
- Postcondition: AC-2, AC-4, AC-8, AC-13, and AC-N1 pass, and existing slicer-config and scheduler tests pass with updated expectations.
- Files allowed to read: named symbols in `crates/slicer-ir/src/{resolved_config.rs,config_schema.rs}`, `crates/slicer-scheduler/src/manifest.rs`, and `crates/slicer-config/src/{lib.rs,resolution.rs}`.
- Context cost: `M` (substep budgets are slices, not additive)
- Authoritative docs: ADR-0067 (reconciliation); plan RC-4 and the Extensions/Guests sections; `docs/11_operational_governance_and_acceptance_gate.md` (wire-version rule).
- OrcaSlicer refs: none.
- Exit condition: stop if any of these holds:
  - a key with a registry default is not seeded;
  - a typed-field or selector key is seeded;
  - a seeded percent default escapes expansion;
  - an invalid extension is accepted;
  - output depends on hash iteration;
  - a `Default`-built entry is silently omitted from emission;
  - a synthesized key (e.g. `printer_model`) gains a default.

#### Step 2a: Host table, runtime registration, generated map

- Precondition: Step 1 complete.
- Postcondition:
  - `HostKeyMeta.omit_from_config_block` exists (`false` in `HostKeyMeta::NONE`) and is `true` on the three `mmu_segmented_region_*` `cli` rows and on the `thumbnail_path` runtime row.
  - `HostRuntimeKey.default` is `Option<&'static str>`, and every item-6 key is registered with its Step-1 type and default.
  - `to_config_map` and `typed_field_keys()` are macro-generated.
  - The old hand map survives only as `#[cfg(test)] fn legacy_to_config_map`, which is the AC-13 oracle.
  - `runtime_declaration` passes `default` through as-is.
  - `build_host_key_entries` renders `None` as `null`.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/resolved_config.rs`; `crates/slicer-config/src/lib.rs` (`runtime_declaration` only); `crates/slicer-scheduler/src/manifest.rs` (`build_host_key_entries` only).
- Files explicitly out of bounds: `config_schema.rs`, runtime, gcode, guests, WIT, `CONFIG_SCHEMA_WIRE_VERSION`.
- Blast-radius discipline: `HostRuntimeKey` literals are the `HOST_RUNTIME_KEYS` rows in this same file. Test fallout goes to 2c.
- Expected sub-agent dispatches:
  - `cargo check -p slicer-ir -p slicer-config -p slicer-scheduler` `FACT`;
  - AC-13 `FACT`;
  - `cargo xtask build-guests` rebuild, because guests depend on slicer-ir.
- Context cost: `M`
- Verification: the check above; the AC-13 exact command; `cargo xtask build-guests --check` exits `0`.
- Exit condition: any of these:
  - `to_config_map` retains a production literal key list;
  - AC-13 fails;
  - `build_config_schema_json` output changes other than by the added runtime rows and `null` defaults;
  - guests remain stale.

#### Step 2b: Manifest `config_block` flag

- Precondition: Step 2a green.
- Postcondition: `ConfigFieldEntry.omit_from_config_block` exists (`#[serde(skip)]`) and is parsed from `config_block = false` by `parse_config_field_entry`.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/config_schema.rs`; `crates/slicer-scheduler/src/manifest.rs` (`parse_config_field_entry` only).
- Files explicitly out of bounds: `build_config_schema_json`, slicer-config, runtime.
- Blast-radius discipline: `ConfigFieldEntry` has two production literals, both in `manifest.rs`. Step-1 test literals lacking `..` go to 2c′.
- Expected sub-agent dispatches: `cargo check -p slicer-ir -p slicer-scheduler --lib` `FACT`; `cargo xtask build-guests` rebuild.
- Context cost: `S`
- Verification: the check above; `cargo xtask build-guests --check` exits `0`.
- Exit condition: the new field appears in `module config-schema` output.

#### Step 2c: Registration test fallout

- Precondition: Step 2b green.
- Postcondition:
  - `registry_census_tdd`'s runtime-row pins (`host_runtime_rows_are_exact` and the runtime count) list the registered set;
  - `registry_assembly_tdd`'s `runtime_key` helper uses `Option` defaults;
  - `typed_selector_claim_selection_tdd` asserts that `support_family` is now recognized, with no `UnrecognizedKey`.
- Files allowed to edit (at most 3): `crates/slicer-config/tests/registry_census_tdd.rs`; `crates/slicer-config/tests/registry_assembly_tdd.rs`; `crates/slicer-scheduler/tests/contract/typed_selector_claim_selection_tdd.rs`.
- Files explicitly out of bounds: production sources.
- Expected sub-agent dispatches: per-binary test `FACT`.
- Context cost: `S`
- Verification: `cargo test -p slicer-config --test registry_census_tdd`, `cargo test -p slicer-config --test registry_assembly_tdd`, and `cargo test -p slicer-scheduler --test contract typed_selector_claim_selection_tdd::` each pass with a non-zero count.
- Exit condition: a pin is deleted instead of updated.

#### Step 2c′: Remaining literal fallout

- Precondition: Step 2c green.
- Postcondition: every remaining Step-1 item-4 literal compiles.
- Files allowed to edit (at most 3 per substep): the Step-1 item-4 files, split as needed.
- Expected sub-agent dispatches: `cargo check --workspace --all-targets` `FACT`.
- Context cost: `S`
- Verification: `cargo check --workspace --all-targets`.
- Exit condition: a literal is waived rather than given a `..` rest without an `// exhaustive:` justification.

#### Step 2d: slicer-config reconciliation, seeding, validation, projection

- Precondition: Step 2c′ green.
- Postcondition:
  - `RegistryEntry.omit_from_config_block` is reconciled any-true-wins in `assemble_registry`;
  - `ConfigSchemaRegistry::config_block_map` implements the design's population rule;
  - `resolve_scope_stack` seeds the seed set at the lowest precedence, before expansion;
  - `apply_delta` and seeding reject violations through `ResolutionError::Application(ConfigResolutionError::{TypeMismatch, OutOfRange})`.
- Files allowed to edit (at most 3): `crates/slicer-config/src/lib.rs`; `crates/slicer-config/src/resolution.rs`; `crates/slicer-config/tests/resolved_config_view_tdd.rs` (AC-2, AC-4, AC-8, AC-N1).
- Files explicitly out of bounds: runtime, scheduler binding, gcode, guests.
- Blast-radius discipline: `RegistryEntry` has one production literal, in `assemble_registry`. Edit it here.
- Expected sub-agent dispatches: cargo test `FACT`, or failure `SNIPPETS` ≤20 lines.
- Context cost: `M`
- Verification: the AC-2, AC-4, AC-8, and AC-N1 exact commands.
- Exit condition: any of the four fails.

#### Step 2e: Seeding fallout in existing tests

- Precondition: Step 2d green.
- Postcondition: every Step-1 item-3 assertion expects the seeded defaults. The expected values are derived from the registry declarations, never captured from the resolver.
- Files allowed to edit (at most 3 per substep): the Step-1 item-3 files, split into adjacent substeps 2e, 2e′, and so on.
- Files explicitly out of bounds: production sources.
- Expected sub-agent dispatches: per-file `cargo test -p <crate> --test <bin>` `FACT`.
- Context cost: `S`
- Verification: `cargo test -p slicer-config` (every flat test binary) and each touched scheduler or runtime test binary.
- Exit condition: an expectation is re-derived from resolver output. That is the self-referential oracle forbidden by `docs/22_test_quality.md` §2.1.

### Step 3: Registry-driven, escaped CONFIG_BLOCK

- Task IDs: `TASK-567`
- Objective: build the production CONFIG_BLOCK from `config_block_map`, with no raw overlay and with canonical string escaping.
- Precondition: Step 2 green.
- Postcondition: AC-9 and AC-11 pass, and the pre-existing CONFIG_BLOCK tests pass with re-derived expectations.
- Files allowed to read: `run_pipeline_fork`, `run_pipeline_core`, `run_postpass_with_thumbnail`, `serialize_config_block`, `ThumbnailAwareSerializer::new`, and the four CONFIG_BLOCK test files.
- Context cost: `M`
- Authoritative docs: plan RC-4 and the "Guests and delivery" emission bullet; `docs/02_ir_schemas.md` §"CONFIG_BLOCK viewer-key contract".
- OrcaSlicer refs: `GCode::append_full_config` (`GCode.cpp`), `ConfigOptionString::serialize` (`Config.hpp`), `escape_string_cstyle` (`Config.cpp`).
- Exit condition: stop if any of these holds:
  - a hand-maintained emission roster remains in production;
  - any key other than the four `config_block = false` keys is implicitly omitted;
  - a raw authored or unknown key reaches the production block;
  - flavor selection regresses.

#### Step 3a: Production emission

- Precondition: Step 2 green.
- Postcondition:
  - The crate-private pipeline path takes `config_block: Option<&BTreeMap<String, ConfigValue>>`.
  - `run_slice_with_collector` passes `Some(&registry.config_block_map(&global_resolved))`, and `run_postpass_with_thumbnail` uses that map verbatim.
  - The public `run_pipeline_with_*` keep their signatures and pass `None`, which selects the legacy composition.
  - `raw_config_source` keeps its thumbnail and prepass roles.
  - `serialize_config_block` escapes strings and renders `filament_diameter` per filament from the effective value.
- Files allowed to edit (at most 3): `crates/slicer-runtime/src/run.rs`; `crates/slicer-runtime/src/pipeline.rs`; `crates/slicer-gcode/src/serialize.rs`.
- Files explicitly out of bounds: the `ORCA_CONFIG_PADDING` contents and cap, binding code, guests.
- Blast-radius discipline: the public pipeline signatures are unchanged, so their test callers compile untouched.
- Expected sub-agent dispatches: `cargo check -p slicer-runtime -p slicer-gcode --all-targets` `FACT`.
- Context cost: `M`
- Verification: the check above.
- Exit condition: the check fails, or the production path still removes `thumbnail_path` in code rather than through metadata.

#### Step 3b: Existing CONFIG_BLOCK tests

- Precondition: Step 3a compiles.
- Postcondition:
  - `config_block_escapes_multiline_strings_cstyle` (AC-9) exists and passes.
  - `wedge_per_region_config_delivery_structural_canary` (production `run_slice`) derives its expected key count and required keys from the registry, instead of `95` / `wall_loops`.
  - AC-Neg-3 and the exactly-once pins in `machine_start_end_gcode_emission_tdd.rs` still pass. That file changes only if escaping alters a pinned line.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`; `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`; `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`.
- Files explicitly out of bounds: production sources.
- Expected sub-agent dispatches: per-binary test `FACT`.
- Context cost: `S`
- Verification: the AC-9 exact command; `cargo test -p slicer-runtime --test integration gcode_` and `cargo test -p slicer-runtime --test e2e slice_end_to_end_tdd::` each pass with a non-zero passed count.
- Exit condition: a canary expectation is a pasted literal, or AC-Neg-3 fails.

#### Step 3b′: Flavor test and production-wiring e2e

- Precondition: Step 3b green.
- Postcondition:
  - `gcode_flavor_config_block_tdd.rs` passes; it is edited only if its exactly-once `gcode_flavor` assertions changed.
  - `crates/slicer-runtime/tests/e2e/resolved_config_view_no_drop_tdd.rs` exists with `run_slice_config_block_is_registry_projection` (AC-11).
  - `crates/slicer-runtime/tests/e2e/main.rs` contains `mod resolved_config_view_no_drop_tdd;`.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs`; the new e2e file; `crates/slicer-runtime/tests/e2e/main.rs` (one `mod` line).
- Files explicitly out of bounds: production sources.
- Blast-radius discipline: an unregistered module compiles to zero tests and reports green.
- Expected sub-agent dispatches: `cargo xtask build-guests --check` exit code; AC-11 `FACT`.
- Context cost: `S`
- Verification: the AC-11 exact command; `cargo test -p slicer-runtime --test integration gcode_flavor_config_block_tdd::`.
- Exit condition: AC-11 fails, or the discriminating key is emitted at its padding value.

### Step 4: Resolved binding

- Task IDs: `TASK-567`
- Objective: binding accepts only the effective `ResolvedConfig`, and guests gain required accessors.
- Precondition: Step 3 green.
- Postcondition: AC-1, AC-N2, and every migrated binding caller pass. The census and declared-reads tests exist and fail only on residual counts.
- Files allowed to read: the named binding and live-plan functions, the `ConfigView` impl, `ModuleError`, and the 12 caller files.
- Context cost: `M`
- Authoritative docs: ADR-0068; the Packet 51 and Packet 73 sections of `docs/04_host_scheduler.md`.
- OrcaSlicer refs: none.
- Exit condition: stop if any production path binds a source map, omits a declared default, exposes another module's key, or loses an authored `support_type` / `support_family` for support-family claimants.
- Ordering note: `crates/slicer-runtime/tests/common/mod.rs` declares `pub mod perimeter_harness;`, which is compiled into every slicer-runtime test binary. No slicer-runtime test binary can build between 4a and the end of 4c‴, so all test runs wait until 4d.

#### Step 4a: Production binding

- Precondition: Step 3 green.
- Postcondition: the following compile:
  - `bind_module_config_view(module, &ResolvedConfig)`, with support-family injection read from the resolved map;
  - `build_live_execution_plan(.., resolved: &ResolvedConfig, ..)`;
  - both `run.rs` callers;
  - the migrated `#[cfg(test)]` caller in `execution_plan.rs`.
- Files allowed to edit (at most 3): `crates/slicer-scheduler/src/execution_plan.rs`; `crates/slicer-wasm-host/src/execution_plan_live.rs`; `crates/slicer-runtime/src/run.rs`.
- Files explicitly out of bounds: `crates/slicer-wasm-host/src/dispatch.rs`, guests, gcode.
- Blast-radius discipline: this is a public fn signature change. External test callers migrate in 4c–4c‴, and nothing else may break.
- Expected sub-agent dispatches: `cargo check -p slicer-scheduler --lib --profile test` and `cargo check -p slicer-wasm-host -p slicer-runtime --lib` `FACT`.
- Context cost: `M`
- Verification: the two checks above.
- Exit condition: a production path still requires a source map to compile, or the in-file test module fails to compile.

#### Step 4b: Required accessors

- Precondition: Step 4a compiles.
- Postcondition: `ConfigView::require_{bool,int,float,string,abs_value}` and `ConfigReadError` exist in slicer-ir, and `From<ConfigReadError> for ModuleError` (fatal) exists in slicer-sdk.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/slice_ir.rs`; `crates/slicer-sdk/src/error.rs`.
- Files explicitly out of bounds: `crates/slicer-sdk/src/traits.rs`, WIT, guests.
- Expected sub-agent dispatches: `cargo check -p slicer-ir -p slicer-sdk --lib` `FACT`; `cargo xtask build-guests` rebuild.
- Context cost: `S`
- Verification: the check above; `cargo xtask build-guests --check` exits `0`.
- Exit condition: an accessor returns `Option`, or the error omits the key.

#### Steps 4c, 4c′, 4c″, 4c‴: Migrate existing binding callers

- Precondition: Step 4b green.
- Postcondition: every caller passes a `ResolvedConfig`. Assertions that relied on raw passthrough now assert resolved or default values.
- Files allowed to edit (at most 3 per substep):
  - **4c:** `crates/slicer-runtime/tests/common/perimeter_harness.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs`.
  - **4c′:** `crates/slicer-runtime/tests/contract/config_view_encapsulation_source_tdd.rs`, `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`.
  - **4c″:** `crates/slicer-runtime/tests/integration/tree_support_family.rs`, `crates/slicer-runtime/tests/integration/traditional_support_family.rs`, `crates/slicer-runtime/tests/integration/perimeter_spatial_capture.rs`.
  - **4c‴:** `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`, `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`, `crates/slicer-scheduler/tests/unit/execution_plan_tdd.rs`.
- Files explicitly out of bounds: production sources.
- Expected sub-agent dispatches: after 4c‴ only, one `cargo check --workspace --all-targets` `FACT`.
- Context cost: `S` each
- Verification: after 4c‴, `cargo check --workspace --all-targets` passes.
- Exit condition: an assertion is deleted rather than re-derived, or the all-targets check fails.

#### Step 4d: New contract tests and full binding verification

- Precondition: Step 4c‴ green.
- Postcondition:
  - `crates/slicer-runtime/tests/contract/resolved_config_view_tdd.rs` holds AC-1, AC-N2, and the AC-3 trio (census, calibration, declared-reads). AC-10 is added in Step 6b.
  - `crates/slicer-runtime/tests/contract/main.rs` contains `mod resolved_config_view_tdd;`.
  - AC-1, AC-N2, and calibration pass. Census and declared-reads fail only on residual counts.
- Files allowed to edit (at most 3): the new contract file; `crates/slicer-runtime/tests/contract/main.rs` (one `mod` line).
- Files explicitly out of bounds: production sources, other test modules.
- Blast-radius discipline: an unregistered or misnamed module compiles to zero tests and reports green.
- Expected sub-agent dispatches: AC-1 and AC-N2 `FACT`; per-binary runs `FACT`.
- Context cost: `S`
- Verification:
  - the AC-1 and AC-N2 exact commands;
  - `resolved_config_view_tdd::guest_fallback_detector_is_calibrated` passes;
  - `cargo test -p slicer-runtime --test contract`, `--test integration`, `--test executor`, and `--test e2e` pass with non-zero counts, apart from the known red census and declared-reads tests;
  - `cargo test -p slicer-scheduler --test unit` passes with a non-zero count.
- Exit condition: registration is missing, AC-1 or AC-N2 fails, calibration fails, or any migrated binary regresses.

### Step 5: Guest manifest declarations and fallback removal

- Task IDs: `TASK-567`
- Objective: declare the undeclared reads, then replace all 89 classified fallbacks with typed `require_*` reads. Excluded `unwrap_or` uses stay untouched.
- Precondition: Step 4 green, so views are complete and the accessors are available.
- Postcondition: AC-3 passes (census zero, calibration and declared-reads green), and `cargo xtask build-guests --check` exits `0`.
- Files allowed to read: the Step-1-recorded locations and their adjacent function bodies; the `[config.schema]` tables of the six manifests.
- Context cost: `M`
- Authoritative docs: plan "Guests and delivery"; the locked classification in AC-3; ADR-0067 (declared types must agree across declarers).
- OrcaSlicer refs: none.
- Exit condition: stop if any of these holds:
  - the census is non-zero;
  - more than the 89 baseline sites changed;
  - an excluded site was edited;
  - a `float_or_percent` key is read with `require_float` without a guaranteed-`Float` justification.
- Type-agreement check: `registry_census_tdd`'s `projected_module_declarations` overrides manifest types, so it cannot catch a `TypeDisagreement`. Steps 5a and 5b use AC-11 instead. On the real `run_slice` path, `load_live_modules_for_plan_manifest_first` runs `assemble_registry` over every discovered manifest. A disagreement returns `LiveModuleLoadError::Registry` as a `SliceRunError`, and AC-11 requires `run_slice` to return `Ok`.

#### Step 5a: Manifest declarations, batch 1

- Precondition: Step 4 green.
- Postcondition: each key below is declared, typed to match the registry's existing type.
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`: `outer_wall_line_width`, `line_width`
  - `modules/core-modules/tree-support/tree-support.toml`: `nozzle_diameter`, `layer_height`, `support_line_width`
  - `modules/core-modules/traditional-support/traditional-support.toml`: `nozzle_diameter`, `layer_height`, `support_line_width`
- Files explicitly out of bounds: guest sources, host code.
- Expected sub-agent dispatches: AC-11 `FACT`.
- Context cost: `S`
- Verification: the AC-11 exact command.
- Exit condition: a type disagreement or registry assembly error.

#### Step 5b: Manifest declarations, batch 2

- Precondition: Step 5a green.
- Postcondition: each key below is declared.
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/infill-linker.toml`: `infill_density`, declared as-is with no rename
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`: `infill_shift_step`, a new registry entry whose type and default equal the guest's current effective fallback, per Step 1
  - `modules/core-modules/wave-overhangs/wave-overhangs.toml`: `thick_bridges`, typed as its existing declarers declare it
- Expected sub-agent dispatches: AC-11 `FACT`; declared-reads `FACT`.
- Context cost: `S`
- Verification: the AC-11 exact command; the declared-reads test's residual count reaches zero.
- Exit condition: a type disagreement, or any undeclared read remains.

#### Steps 5c…: Guest source batches

- Precondition: Step 5b green.
- Postcondition: the batch's classified sites use `require_*` reads that match each key's registry type. Helpers that returned plain values now return `Result<_, ModuleError>`. The batch's Step-1 item-11 guest tests build views holding the keys their path reads, at manifest-default values.
- Files allowed to edit (at most 3 per substep): the next rows of the Step-1 batch table, starting with arachne-perimeters and classic-perimeters. The item-11 guest tests for each batch go in an adjacent substep 5c′ of at most three files.
- Files explicitly out of bounds: guest tests not in item 11, non-classified `unwrap_or` sites, host APIs, WIT.
- Expected sub-agent dispatches: one bounded location/result report per batch, then a guest rebuild, a freshness check, and a `cargo test -p <guest crate>` `FACT` per touched guest.
- Context cost: `S` each
- Verification: after each batch and its 5c′, `cargo test -p <guest crate>` passes with a non-zero count for every touched guest, and `cargo xtask build-guests --check` exits `0` after the rebuild. After the last batch, the AC-3 exact command passes.
- Exit condition: a batch edits an excluded site, a guest fails to build, or a guest test is deleted instead of migrated.

### Step 6: Prove no-drop, then flip unknown retention

- Task IDs: `TASK-567`
- Objective: add the independent full-registry e2e with a negative control, then change unknown-key handling to warn-and-drop.
- Precondition: Steps 2–5 pass, and guest freshness exits `0`.
- Postcondition: AC-5, AC-6, AC-10, AC-11, and AC-12 pass after the flip.
- Files allowed to read: the packet-03 ingestion module, `SliceOutcome`, the registry census helper, and the existing `run_slice` e2e fixtures.
- Context cost: `M`
- Authoritative docs: the plan's "Drop-unknown ships in two steps" paragraph and no-drop gate; `docs/22_test_quality.md` §2.4, §2.6, §4.
- OrcaSlicer refs: none.
- Exit condition: stop if any of these holds:
  - the fixture is skipped;
  - the synthesized population is hand-listed instead of registry-derived;
  - the negative control passes vacuously;
  - any registry key warns as unknown.

#### Step 6a: No-drop e2e in retained mode

- Precondition: Step 5 green.
- Postcondition: `run_slice_with_collector` populates `SliceOutcome.ingestion_warnings` (its only literal is in `run.rs`), and both AC-5 tests pass while unknown keys are still retained.
- Files allowed to edit (at most 3): `crates/slicer-runtime/src/run.rs`; `crates/slicer-runtime/tests/e2e/resolved_config_view_no_drop_tdd.rs`.
- Files explicitly out of bounds: ingestion, guests, `resources/cube_4color.3mf` contents.
- Expected sub-agent dispatches: fixture member check `FACT` ≤5 lines; e2e run `FACT`.
- Context cost: `M`
- Verification: the AC-5 exact command.
- Exit condition: AC-5 fails, or the population is not registry-derived.

#### Step 6b: Warn-to-drop flip

- Precondition: Step 6a green, and every Step-1 item-6 key is registered (done in Step 2a).
- Postcondition: undeclared keys warn once and appear in no delta, resolved config, view, or `CONFIG_BLOCK`. Registered host-consumed keys survive. AC-5 and AC-11 still pass.
- Files allowed to edit (at most 3): `crates/slicer-config/src/ingestion.rs`; `crates/slicer-config/tests/resolved_config_view_tdd.rs` (AC-6, AC-12); `crates/slicer-runtime/tests/contract/resolved_config_view_tdd.rs` (AC-10).
- Files explicitly out of bounds: guests, emission code.
- Expected sub-agent dispatches: AC-5, AC-6, AC-10, AC-11, and AC-12 `FACT`.
- Context cost: `S`
- Verification: the AC-5, AC-6, AC-10, AC-11, and AC-12 exact commands. The Step-1 item-12 retention tests are expected red here and are inverted in 6b′.
- Exit condition: a registered host-consumed key is dropped, or flavor or support selection regresses.

#### Step 6b′: Invert unknown-retention assertions

- Precondition: Step 6b green, except the Step-1 item-12 tests.
- Postcondition: every item-12 assertion now asserts drop semantics. For example, `experimental_xyz` and `unrelated_extension` are absent from `extensions`, and `unknown_key_skips_bounds_check` still reports no bounds error.
- Files allowed to edit (at most 3): `crates/slicer-config/tests/typed_scope_ingestion_tdd.rs`; `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`; `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`. Any further item-12 files go in an adjacent 6b″.
- Expected sub-agent dispatches: per-binary test `FACT`.
- Context cost: `S`
- Verification: `cargo test -p slicer-config --test typed_scope_ingestion_tdd` and `cargo test -p slicer-scheduler --test integration` each pass with a non-zero count.
- Exit condition: an assertion is deleted rather than inverted.

### Step 7: Update contracts and run closure gates

- Task IDs: `TASK-567`
- Objective: document resolved delivery, host-key registration, manifest metadata, emission escaping, and view sourcing; then run all packet gates.
- Precondition: Steps 1–6 pass.
- Postcondition: AC-7 and all closure gates pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md`: the `ConfigView` mentions and §"CONFIG_BLOCK viewer-key contract";
  - `docs/03_wit_and_manifest.md`: §"Config Field Types Reference";
  - `docs/04_host_scheduler.md`: the Packet 51 and Packet 73 sections.
- Files allowed to edit (at most 3): `docs/02_ir_schemas.md`; `docs/03_wit_and_manifest.md`; `docs/04_host_scheduler.md`.
- Files explicitly out of bounds: other docs, the plan, packets, and the backlog (except the delegated completion update).
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches: each cargo or doc gate returns `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs: the approved plan and ADR-0067/0068.
- OrcaSlicer refs: none.
- Verification:
  - AC-7;
  - `cargo check --workspace --all-targets`;
  - clippy;
  - `cargo xtask check-literals`;
  - `cargo xtask check-test-quality --report`;
  - `cargo xtask build-guests --check`.
- Exit condition: any AC or gate fails, or the docs claim a WIT or IR version change.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | inventories only |
| 2 | M | 2a host table/registration/generated map, 2b manifest flag, 2c/2c′ test and literal fallout, 2d slicer-config, 2e seeded-expectation fallout |
| 3 | M | 3a production emission, 3b CONFIG_BLOCK tests, 3b′ flavor test and wiring e2e |
| 4 | M | 4a binding, 4b accessors, 4c–4c‴ caller migration, 4d new contract tests and verification |
| 5 | M | 5a/5b manifests, 5c… ≤3-file guest batches with 5c′ guest-test migrations |
| 6 | M | 6a no-drop e2e and `ingestion_warnings`, 6b flip, 6b′ retention-assertion inversions |
| 7 | S | docs/gates |

## Packet Completion Gate

- All steps and their exit conditions are complete, and every AC command passes.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `cargo xtask build-guests --check` exits `0`, and the all-target check, clippy, and quality gates pass.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record the intentional CONFIG_BLOCK change:
  - the measured key count before and after;
  - string escaping;
  - suppressed padding;
  - the four exclusions;
  - `filament_diameter` rendering.
- Record the registered host-consumed keys, their defaults, and the `module config-schema` `host` array growth.
- Record the value deltas of the eleven newly declared guest reads against their old literal or helper defaults.
- Record residual non-`unwrap_or` guest literal defaults (match arms, helper default arguments, `resolve_float`) as follow-up scope.
- Record the measured distinct-`ResolvedConfig` count and size impact of seeding, or state "unmeasured".
- Confirm context stayed within the standard band, or record the required swarm escalation and lesson.
