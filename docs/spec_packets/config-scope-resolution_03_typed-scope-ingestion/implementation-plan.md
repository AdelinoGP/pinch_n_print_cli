# Implementation Plan: typed-scope-ingestion

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-564`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Packet 01 and packet 02 are FORWARD-DEPs. Reconcile only their `packet.spec.md`, `requirements.md`, and `design.md`; never edit them or read their implementation plans.
- Preserve packet 02's oracle unchanged. A green obtained by weakening its assertions is a packet failure.
- Delegate cargo commands and fixture inspection with the bounded return formats below.

## Steps

### Step 1: Reconcile forward contracts and author the typed-ingestion tests

- Task IDs: `TASK-564`
- Objective: reconcile the landed packet-01 registry/assembly surfaces consumed here—especially `AssemblyOutcome.registry` and opaque `AssemblyOutcome.warnings: Vec<RegistryWarning>`—and the packet-02 oracle/edit-surface contract, then encode AC-1/AC-2/AC-3/AC-N1/AC-N2 as focused red tests. Packet 03 consumes no `RegistryWarning` variant or field; the warning enum is non-exhaustive from this packet's perspective, and all variant/field details are packet-01 diagnostics outside scope.
- Precondition: packet 01 has landed; packet 02 has passed independent preflight and landed.
- Postcondition: `typed_scope_ingestion_tdd.rs` names only available registry exports and fails for missing packet-03 behavior, while packet 02's test file remains byte-unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/config-scope-resolution_01_config-schema-registry/packet.spec.md`
  - `docs/spec_packets/config-scope-resolution_01_config-schema-registry/requirements.md`
  - `docs/spec_packets/config-scope-resolution_01_config-schema-registry/design.md`
  - `docs/spec_packets/config-scope-resolution_02_authored-value-oracle/packet.spec.md`
  - `docs/spec_packets/config-scope-resolution_02_authored-value-oracle/requirements.md`
  - `docs/spec_packets/config-scope-resolution_02_authored-value-oracle/design.md`
  - `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs` — symbol locations only
  - `crates/slicer-config/tests/registry_assembly_tdd.rs` — fixture-builder symbols only
- Files allowed to edit (at most 3):
  - `crates/slicer-config/tests/typed_scope_ingestion_tdd.rs`
- Files explicitly out of bounds:
  - predecessor packet files and implementation plans
  - `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs`
  - production code
- Blast-radius discipline: no production struct field or schema/version constant is added in this test-first step; test literals must use FRU or an exhaustive waiver where required.
- Expected sub-agent dispatches:
  - Question: list packet 01's landed registry/assembly exports consumed here, confirm that `AssemblyOutcome.warnings: Vec<RegistryWarning>` is opaque to packet 03 (do not enumerate or destructure warning variants/fields), and report packet 02's exact four-test roster—one red principal, `oracle_authored_values_reach_owning_module_config_views`, plus the three green controls `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control`—plus fixture contract; scope: predecessor authority files only; return: `SUMMARY` ≤200 words.
  - Question: confirm packet 02 adds one oracle file, one aggregator registration, and only `slicer-config` under runtime `[dev-dependencies]`, while the exact deflate-only `zip` entry already exists and stays unchanged; scope: packet 02 authority files and `crates/slicer-runtime/Cargo.toml`; return: `FACT` ≤5 lines.
  - Question: inspect the fixture for only the five locked key/value strings; scope: `resources/cube_4color.3mf`; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0067-unified-config-schema-registry.md` — registry authority and selectors
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — decode-once and deltas
  - `docs/22_test_quality.md` — independent production oracle and negative control
- OrcaSlicer refs:
  - None; this packet does not port OrcaSlicer source.
- Verification:
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd 2>&1 | tee target/test-output.log'` — FACT expected red solely for missing packet-03 API/behavior.
- Exit condition: all five focused test functions (`flat_wire_keys_decode_once_into_typed_scopes`, `registry_types_the_five_authored_value_oracle_divergences`, `unknown_key_warns_with_near_miss_and_is_retained`, `malformed_scope_key_is_rejected`, and `declared_value_type_mismatch_is_rejected`) compile far enough to fail on absent/wrong typed-ingestion behavior, and a worker confirms the predecessor oracle file is unchanged.

### Step 2: Implement the deep typed-ingestion module

- Task IDs: `TASK-564`
- Objective: implement the exact `ConfigScope`, scope-delta, registry typing, wildcard matching, warning, suggestion, selector, and error contracts from `design.md`.
- Precondition: Step 1's focused tests are red for the intended reasons.
- Postcondition: `slicer-config` exposes one immutable typed-ingestion result and all focused tests pass without defaults or unknown-key drops.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-config/src/lib.rs` — registry public types and methods only
  - `crates/slicer-ir/src/slice_ir.rs` — `ObjectId`, `ModifierId`, `ConfigKey`, `ConfigValue` only
  - `crates/slicer-ir/src/config_schema.rs` — field-type spellings only
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/ingestion.rs`
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/typed_scope_ingestion_tdd.rs`
- Files explicitly out of bounds:
  - scheduler/runtime/model adapters
  - `crates/slicer-ir/src/resolved_config.rs`
  - automatic expansion and scope-eligibility code
- Blast-radius discipline: new packet-local public structs are introduced together with all their test literals in the listed test file; tests use FRU or `// exhaustive:` waivers as required. No existing struct gains a field and no schema/version constant changes.
- Expected sub-agent dispatches:
  - Question: verify the exact `ConfigValue` variants and packet-01 `RegistryEntry.field_type` spellings; scope: named definitions only; return: `SNIPPETS` ≤3 snippets/30 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0067-unified-config-schema-registry.md` — one per-run authority
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed scope and authored delta
  - `CONTEXT.md` — `Config scope`, `Scope delta`, `Authored value`
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd 2>&1 | tee target/test-output.log'` — FACT pass/fail.
- Exit condition: the focused test binary passes and no ingestion code calls `classify_declared_key`, `ResolvedConfig::apply_cli_key`, or inserts registry defaults into a delta.

### Step 3: Preserve authored 3MF values until registry typing

- Task IDs: `TASK-564`
- Objective: make the 3MF adapter syntax-only by preserving string-authored values and removing heuristic declared-type authority.
- Precondition: `ConfigIngestor` can type string values from a registry.
- Postcondition: project/object/modifier sidecar strings remain `ConfigValue::String` at the loader boundary and the five oracle values are typed only after Step 2's ingestion.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/loader.rs` — `read_3mf_project_settings`, `parse_project_settings_json`, `json_to_config_value`, `coerce_string_to_config_value`, and modifier/object metadata assembly only
  - `crates/slicer-model-io/tests/threemf_project_settings_extraction_tdd.rs`
  - `docs/02_ir_schemas.md` — `ObjectConfig.data Population` only
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/src/loader.rs`
  - `crates/slicer-model-io/tests/threemf_project_settings_extraction_tdd.rs`
  - `crates/slicer-model-io/tests/mod_cilindrical_modifier_infill_density_tdd.rs`, `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` — loader-boundary expectations only, widened after measurement: these assert heuristic coercion of string-authored sidecar values and are stale once the loader is syntax-only. Update them to assert exact string preservation; never weaken them or add skips.
- End-to-end assertions are explicitly NOT in scope here: any `crates/slicer-runtime/tests/e2e/**` expectation that a string-authored value reaches a module typed must stay untouched and is expected red until Step 5b/7 wire typed ingestion into the production path.
- Files explicitly out of bounds:
  - `resources/cube_4color.3mf`
  - model geometry/paint loading
  - `crates/slicer-ir/src/resolved_config.rs`
- Blast-radius discipline: no existing public struct field or schema/version constant changes.
- Expected sub-agent dispatches:
  - Question: locate every call to `coerce_string_to_config_value` and every test asserting its heuristic variants; scope: `crates/slicer-model-io`; return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0067-unified-config-schema-registry.md` — module declarations must type module keys
  - `docs/22_test_quality.md` — fixture assertions must fail loudly
- OrcaSlicer refs:
  - None; string-valued sidecar shape is already locked by repository fixtures and ADR-0068.
- Verification:
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-model-io --all-targets --test threemf_project_settings_extraction_tdd 2>&1 | tee target/test-output.log'` — FACT pass/fail.
- Exit condition: model-io tests prove string preservation, and no loader branch uses host-only declared-key classification to type sidecar strings.

### Step 4: Adapt scheduler config consumers to typed deltas and claim controls

- Task IDs: `TASK-564`
- Objective: replace raw prefix and claim-control reads at scheduler seams while retaining current resolution behavior until packet 05.
- Precondition: Steps 2-3 expose typed values/deltas from both input adapters and packet 01 has added `slicer-config` to the workspace.
- Postcondition: scheduler binding/resolution functions consume `ScopedConfig`; perimeter claim helpers receive the typed `wall_generator` selector; non-selector controls remain ordinary typed global values; the three scope prefixes are not parsed downstream.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` — `parse_cli_config_source`, `bind_module_config_view`, claim-selection helpers only
  - `crates/slicer-scheduler/src/config_resolution.rs` — named global/object/paint/tool compatibility resolvers only
  - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` — scope cases only
  - `crates/slicer-scheduler/tests/integration/config_resolution_paint_semantic_tdd.rs`
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/Cargo.toml` — add `slicer-config = { path = "../slicer-config" }` under `[dependencies]`
  - `crates/slicer-scheduler/src/execution_plan.rs`
  - `crates/slicer-scheduler/src/config_resolution.rs`
- Files explicitly out of bounds:
  - packet-05 unified resolver/expansion work
  - support-family candidate population semantics
  - alias tables and denied-scope enforcement
- Blast-radius discipline: do not add fields to existing public request/result structs. Prefer new typed parameters or a new wrapper entry point; if a field proves unavoidable, stop and dispatch a complete struct-literal inventory before editing.
- Expected sub-agent dispatches:
  - Question: inventory raw prefix parsing, raw claim-control reads, and selector-map lookups in the two source files; scope: exact files; return: `LOCATIONS` ≤20.
  - Question: verify the scheduler's direct normal dependency and unchanged reverse dependency direction; scope: `crates/slicer-scheduler/Cargo.toml` and `crates/slicer-config/Cargo.toml`; return: `FACT` ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — no downstream prefix parsing
  - `docs/01_system_architecture.md` — `PrePass::RegionMapping` current inputs only
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_integration config_resolution -- --nocapture 2>&1 | tee target/test-output.log'` — FACT pass/fail.
  - `rg -n 'starts_with\("(object_config|paint_config|tool_config):"\)' crates/slicer-scheduler/src/execution_plan.rs crates/slicer-scheduler/src/config_resolution.rs` — FACT no matches.
- Exit condition: the scheduler declares its direct `slicer-config` dependency, compatibility tests retain prior precedence, and the exact three prefix parsers are absent from scheduler consumers.

### Step 5a: Compose manifest-first ingestion into live module loading

- Task IDs: `TASK-564`
- Objective: make live loading discover manifests, assemble the registry, ingest config, select claims, and only then return execution inputs.
- Precondition: scheduler accepts `ScopedConfig` and the typed `wall_generator` selector.
- Postcondition: one wasm-host orchestration path returns live modules plus `IngestionOutcome` without changing guest/WIT boundaries.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` — `LiveModuleLoadOutput` and `load_live_modules_for_plan*` only
  - `crates/slicer-wasm-host/src/lib.rs` — existing live-loader re-export block only
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/Cargo.toml` — add `slicer-config = { path = "../slicer-config" }` under `[dependencies]`
  - `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - `crates/slicer-wasm-host/src/lib.rs`
- Files explicitly out of bounds:
  - `LiveModuleLoadOutput` field additions unless a prior struct-literal inventory is added to this step
  - guest/module source and WIT
  - packet-02 oracle expectations
- Blast-radius discipline: use a new orchestration return wrapper or tuple rather than adding a field to `LiveModuleLoadOutput`. If that cannot preserve API clarity, dispatch every `LiveModuleLoadOutput` literal/pattern site and amend the allowed edit list before proceeding.
- Expected sub-agent dispatches:
  - Question: locate all `load_live_modules_for_plan*` callers and `LiveModuleLoadOutput` literals/patterns; scope: `crates/**`; return: `LOCATIONS` ≤20.
  - Question: verify the wasm-host's direct normal dependency and unchanged reverse dependency direction; scope: `crates/slicer-wasm-host/Cargo.toml` and `crates/slicer-config/Cargo.toml`; return: `FACT` ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0067-unified-config-schema-registry.md` — module loading precedes ingestion
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed handoff
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo check --all-targets -p slicer-wasm-host` — FACT pass/fail.
- Exit condition: wasm-host declares its direct `slicer-config` dependency, exports one manifest-first orchestration result carrying `IngestionOutcome`, and no production perimeter selector receives the raw source map.

### Step 5b: Retain typed ingestion through runtime entry paths

- Task IDs: `TASK-564`
- Objective: make ordinary slicing and `prepare_prepass_context` use Step 5a's manifest-first output and retain one typed config for later binding/resolution.
- Precondition: Step 5a exposes live modules plus `IngestionOutcome`.
- Postcondition: both runtime entry paths retain the same `ScopedConfig`, surface ingestion warnings through existing startup diagnostics, and do not re-ingest or recover raw prefixed values.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` — `run_slice_with_collector` config/load section and `prepare_prepass_context` counterpart only
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — production loader call sites only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/Cargo.toml` — add `slicer-config = { path = "../slicer-config" }` under `[dependencies]`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`
- Files explicitly out of bounds:
  - guest/module source and WIT
  - packet-02 oracle expectations
  - visual-debug renderer behavior
- Blast-radius discipline: do not add fields to an existing public request/result struct. Prefer a new wrapper or private tuple; if a public field proves unavoidable, stop and inventory every literal/pattern site before changing this step.
- Expected sub-agent dispatches:
  - Question: locate both runtime callers of the live-loader orchestration; scope: named runtime files; return: `LOCATIONS` ≤20.
  - Question: verify the runtime's direct normal dependency and unchanged reverse dependency direction; scope: `crates/slicer-runtime/Cargo.toml` and `crates/slicer-config/Cargo.toml`; return: `FACT` ≤5 lines.
  - Question: run type-check after each signature change and return concrete compiler locations only; scope: workspace all targets; return: `LOCATIONS` ≤20.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0067-unified-config-schema-registry.md` — module loading precedes ingestion
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed handoff
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-runtime --all-targets --test integration live_module_loading -- --nocapture 2>&1 | tee target/test-output.log'` — FACT pass/fail.
  - `cargo check --workspace --all-targets` — FACT pass/fail.
- Exit condition: runtime declares its direct `slicer-config` dependency, both entry paths use the manifest-first orchestration, ingestion warnings reach existing startup diagnostics, and no runtime path reconstructs scoped config from raw prefixed keys.

### Step 6: Lock exact selector extraction and preserve non-selector routing

- Task IDs: `TASK-564`
- Objective: prove that only registry-marked `wall_generator` enters the selector map while retaining spiral-vase and support-family routing as ordinary typed/resolved behavior.
- Precondition: Step 5b supplies typed ingestion output to scheduler claim code.
- Postcondition: one contract test proves exact one-entry selector extraction and typed `wall_generator` perimeter selection; existing scheduler tests separately preserve spiral-vase forcing and support-family candidate/per-region behavior without relabeling those values as selectors.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` — claim helpers only
  - `crates/slicer-scheduler/src/validation.rs` — `resolve_held_claims` only
  - `crates/slicer-scheduler/tests/contract/spiral_vase_arachne_dispatch_tdd.rs` — existing non-selector perimeter invariant only
  - `crates/slicer-scheduler/tests/integration/support_family_selection.rs` — existing invariants only
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/validation.rs`
  - `crates/slicer-scheduler/tests/contract/typed_selector_claim_selection_tdd.rs`
  - `crates/slicer-scheduler/tests/contract/main.rs`
- Files explicitly out of bounds:
  - module manifests
  - support geometry/planning
  - packet-221 support-family behavior
- Blast-radius discipline: no production struct field or schema/version constant changes.
- Expected sub-agent dispatches:
  - Question: verify `scheduler_contract` registration and the existing spiral-vase/support-family assertions; scope: named test files plus `crates/slicer-scheduler/Cargo.toml`; return: `SNIPPETS` ≤3 snippets/30 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0067-unified-config-schema-registry.md` — typed selector consequence
  - `docs/04_host_scheduler.md` — global and per-region claim ownership
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_contract typed_selector_claim_selection_tdd::typed_wall_generator_selector_claim_selection -- --exact --nocapture 2>&1 | tee target/test-output.log'` — FACT pass/fail.
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_contract spiral_vase -- --nocapture 2>&1 | tee target/test-output.log'` — FACT pass/fail for the declared non-selector control.
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_integration support_family_selection -- --exact --nocapture 2>&1 | tee target/test-output.log'` — FACT pass/fail for existing resolved support routing.
- Exit condition: AC-4 passes; `selector_values` has no `spiral_vase`, `support_type`, or `support_family`; and a source-location dispatch finds no raw `config_source.get("wall_generator")` in startup claim code.

### Step 7: Prove predecessor-oracle and visual-debug production reachability

- Task IDs: `TASK-564`
- Objective: turn packet 02's one red executor oracle green unchanged while preserving its three green controls, and add a model-mode visual-debug manifest/image regression over the same fixture.
- Precondition: Steps 2-6 are green and `cargo xtask build-guests --check` exits 0.
- Postcondition: all four executor oracle tests pass unchanged and the visual-debug request emits one indexed image.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs` — read-only assertions and fixture helper only
  - `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` — request/helper pattern only; long file, delegate snippets
  - `docs/19_visual_debug.md` — `Request Shape` and `Reading A Bundle` only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/config_scope_ingestion_visual_debug_tdd.rs`
- Files explicitly out of bounds:
  - packet-02 oracle and registration files
  - `resources/cube_4color.3mf`
  - visual renderer behavior unrelated to ingestion
- Blast-radius discipline: the new `VisualDebugRequest` test literal must use FRU or an `// exhaustive:` waiver naming the request-boundary fixture.
- Expected sub-agent dispatches:
  - Question: return the smallest public helper/request pattern that drives `run_visual_debug` in model mode; scope: existing pnp-cli visual-debug tests; return: `SNIPPETS` ≤3 snippets/30 lines.
  - Question: run guest freshness and then both test binaries; scope: exact commands below; return: `FACT` pass/fail.
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` — production oracle, loud fixture, negative control
  - `docs/19_visual_debug.md` — bundle and manifest contract
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo xtask build-guests --check` — FACT exit 0 required before tests.
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd -- --nocapture 2>&1 | tee target/test-output.log'` — FACT pass/fail.
  - `bash -o pipefail -c 'mkdir -p target && cargo test -p pnp-cli --all-targets --test config_scope_ingestion_visual_debug_tdd 2>&1 | tee target/test-output.log'` — FACT pass/fail.
- Exit condition: the four named oracle tests and visual-debug regression pass, `manifest.json` records one real image path, and the predecessor test file remains unchanged.

### Step 8: Document typed ingestion and perimeter selection, then close

- Task IDs: `TASK-564`
- Objective: document the new typed boundary, replace the scheduler perimeter section's raw `wall_generator` requirement with the typed global-selector handoff, and run packet closure gates.
- Precondition: all behavioral acceptance criteria pass.
- Postcondition: canonical docs distinguish scope wire prefixes from typed internal deltas, preserve packet-05/06/07/09 boundaries, and describe perimeter selection through `IngestionOutcome.selector_values` without requiring raw-source `wall_generator` access.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` — `Modifier Resolution Contract`, `ObjectConfig.data Population`, and `Config Key Namespaces` only
  - `docs/04_host_scheduler.md` — only “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)”
  - `packet.spec.md`
  - `requirements.md`
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md` — only “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)”
- Files explicitly out of bounds:
  - `docs/specs/config-scope-resolution-plan.md`
  - packet 01, packet 02, packet 04
  - generated config-key catalog sections
- Blast-radius discipline: documentation-only step; no struct field or schema/version constant changes.
- Expected sub-agent dispatches:
  - Question: verify the named docs section contains every AC-7 fragment and no claim that later packets already landed; scope: one docs section; return: `FACT` ≤5 lines.
  - Question: verify the bounded scheduler perimeter section names `IngestionOutcome.selector_values`, `ConfigScope::Global`, `DEFAULT_WALL_GENERATOR`, and `spiral_vase`, and no longer contains either forbidden raw-read phrase from AC-8; scope: one docs section; return: `FACT` ≤5 lines.
  - Question: run closure gates; scope: commands below; return: `FACT` pass/fail, bounded failure snippets only.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` — wording authority
  - `CONTEXT.md` — vocabulary authority
- OrcaSlicer refs:
  - None.
- Verification:
  - `python3 -c "from pathlib import Path; s=Path('docs/02_ir_schemas.md').read_text(encoding='utf-8'); b=s.split('#### Typed config-scope ingestion (TASK-564)',1)[1].split('\n#### ',1)[0]; required=('ConfigScope','ScopeDelta','object_config:','paint_config:','tool_config:','warn','keep','object_height:<id>','packet 05'); missing=[x for x in required if x not in b]; assert not missing, missing"` — FACT pass/fail.
  - `python3 -c "from pathlib import Path; s=Path('docs/04_host_scheduler.md').read_text(); b=s.split('### Perimeter-generator selection',1)[1].split('\n### Support-generator selection',1)[0]; required=('IngestionOutcome.selector_values','ConfigScope::Global','DEFAULT_WALL_GENERATOR','spiral_vase'); forbidden=('read directly from the raw config source','config_source.get(\"wall_generator\")'); missing=[x for x in required if x not in b]; present=[x for x in forbidden if x in b]; assert not missing and not present, (missing,present)"` — FACT pass/fail.
  - `cargo xtask check-literals` — FACT pass/fail.
  - `cargo xtask check-test-quality --report` — FACT findings/no findings in touched tests.
- Exit condition: AC-7 and AC-8 pass, all packet gates are green, no out-of-scope file changed, and TASK-564 status is ready for a bounded backlog-update dispatch.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Forward-contract reconciliation and red tests |
| Step 2 | M | Deep ingestion/type policy |
| Step 3 | S | 3MF syntax-only adapter |
| Step 4 | M | Scheduler dependency and transitional consumers |
| Step 5a | M | Manifest-first wasm-host composition |
| Step 5b | M | Runtime entry-path retention |
| Step 6 | S | Exact selector extraction and routing controls |
| Step 7 | S | Existing oracle plus visual-debug smoke |
| Step 8 | S | Typed-ingestion and perimeter-selection docs plus closure gates |

Aggregate remains `M`; no step is `L`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask check-literals` passes and touched-file findings from `cargo xtask check-test-quality --report` are fixed or justified.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile packet 01/02 forward-dependency status without modifying their packet files.
- `packet.spec.md` is ready for `status: implemented` only after the acceptance ceremony.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, especially transitional compatibility code scheduled for packet 05.
- Confirm packet 02's oracle file and all out-of-scope packet/plan files are unchanged.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations use `--all-targets` so test, bench, and example targets compile.
