---
status: draft
packet: config-scope-resolution_03_typed-scope-ingestion
task_ids:
  - TASK-564
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: typed-scope-ingestion

## Goal

Introduce registry-directed, typed config ingestion that decodes each supported scope prefix once, preserves authored scope deltas, retains unrecognised keys with deterministic warnings, and supplies typed selector values to claim selection.

## Scope Boundaries

This packet owns ingestion and the compatibility adaptations needed for current consumers to accept typed scope deltas. It turns packet 02's one red authored-value oracle green while preserving its three green controls, and proves the same path through visual-debug; automatic-value expansion, the final unified resolver, drop-unknown enforcement, scope eligibility, aliases, and layer-range loading remain in later packets.

## Prerequisites and Blockers

- Depends on: **FORWARD-DEP** packet 01, `config-scope-resolution_01_config-schema-registry` (`status: active`), for the `slicer_config::ConfigSchemaRegistry`/`RegistryEntry` assembly and lookup contract plus `AssemblyOutcome.registry` and opaque `AssemblyOutcome.warnings: Vec<RegistryWarning>`. Packet 03 consumes no `RegistryWarning` variant or field; the warning enum is non-exhaustive from this packet's perspective, and all variant/field details remain packet-01 diagnostics outside scope.
- Depends on: **FORWARD-DEP** packet 02, `config-scope-resolution_02_authored-value-oracle` (`status: draft`, independent preflight in progress), for its four executor-oracle tests—one red principal, `oracle_authored_values_reach_owning_module_config_views`, and three green controls, `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control`—and `cube_4color.3mf` fixture contract. Packet 02 adds one test module, its aggregator registration, and only the net-new `slicer-config` runtime dev-dependency; it reuses the existing `zip` dev-dependency unchanged.
- Unblocks: queue row 4, `automatic-value-expansion`, by providing typed scope deltas as its input.
- Activation blockers: packet 01 must land with its promised exports reconciled; packet 02 must pass preflight and land without changing its oracle contract.

## Acceptance Criteria

- **AC-1. Given** flat entries `layer_height`, `object_height:obj-a`, `object_config:obj-a:wall_loops`, `paint_config:fuzzy_skin:fuzzy_skin_point_dist`, and `tool_config:1:retract_length`, plus an explicitly scoped modifier delta for object `obj-a`/modifier `mod-a`, **when** `ConfigIngestor` finishes, **then** the result contains exactly the five scopes `ConfigScope::Global`, `Object("obj-a")`, `Modifier { object_id: "obj-a", modifier_id: "mod-a" }`, `PaintSemantic("fuzzy_skin")`, and `Tool(1)`; the `object_config:`, `paint_config:`, and `tool_config:` prefixes occur in no `ScopeDelta.values` key; and `object_height:obj-a` remains a recognised global dynamic key pending packet 05. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd flat_wire_keys_decode_once_into_typed_scopes -- --exact 2>&1 | tee target/test-output.log'`
- **AC-2. Given** packet 02's `cube_4color.3mf` authored strings, **when** registry-directed ingestion types them, **then** `skirt_loops` is `ConfigValue::Int(1)`, `brim_width` and `filter_out_gap_fill` are `ConfigValue::Float(0.0)`, and `tree_support_wall_count` and `support_interface_bottom_layers` are `ConfigValue::Int(0)`—never `Bool`. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd registry_types_the_five_authored_value_oracle_divergences -- --exact 2>&1 | tee target/test-output.log'`
- **AC-3. Given** undeclared key `skrit_loops = "1"` beside declared `skirt_loops`, **when** ingestion runs in this packet's warn-mode, **then** the unknown entry remains in the global delta as `ConfigValue::String("1")` and exactly one `IngestionWarning::UnrecognizedKey { wire_key: "skrit_loops", key: "skrit_loops", suggestion: Some("skirt_loops") }` is returned. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd unknown_key_warns_with_near_miss_and_is_retained -- --exact 2>&1 | tee target/test-output.log'`
- **AC-4. Given** packet 01's assembled registry marks only `wall_generator` as a selector among the current startup claim controls—ordinary rows produced by `host_declaration` have `selector = false`, the `spiral_vase` manifest declarations omit selector metadata and therefore default false, and `support_family` is undeclared—**when** global values `wall_generator = "arachne"`, `spiral_vase = true`, `support_type = "tree(auto)"`, and `support_family = "tree"` are ingested, **then** `IngestionOutcome.selector_values` contains exactly `wall_generator = ConfigValue::String("arachne")`; `spiral_vase` and `support_type` remain ordinary typed values in the global `ScopeDelta`; undeclared `support_family` remains a warn-and-keep string and does not enter `selector_values`; and startup perimeter dedup consumes the typed `wall_generator` selector to select `com.core.arachne-perimeters` without reading the raw source map. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_contract typed_selector_claim_selection_tdd::typed_wall_generator_selector_claim_selection -- --exact --nocapture 2>&1 | tee target/test-output.log'`
- **AC-5. Given** packet 02's registered executor oracle and `resources/cube_4color.3mf`, **when** the executor test target runs after typed ingestion is wired into the production load path, **then** `oracle_authored_values_reach_owning_module_config_views`, `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control` all pass unchanged. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd -- --nocapture 2>&1 | tee target/test-output.log'`
- **AC-6. Given** a model-mode visual-debug request for layer `0`, tap `Layer::Perimeters`, visualization `filled_areas`, and `resources/cube_4color.3mf`, **when** `run_visual_debug` uses the production ingestion path, **then** it succeeds, writes `manifest.json`, records exactly one image for layer `0`/tap `Layer::Perimeters`, and the recorded image path exists. | `bash -o pipefail -c 'mkdir -p target && cargo test -p pnp-cli --all-targets --test config_scope_ingestion_visual_debug_tdd cube_4color_typed_ingestion_renders_manifest -- --exact 2>&1 | tee target/test-output.log'`
- **AC-7. Given** typed ingestion is implemented, **when** the `docs/02_ir_schemas.md` section `Typed config-scope ingestion (TASK-564)` is read, **then** it names `ConfigScope`, `ScopeDelta`, the three decoded wire prefixes, warn-and-keep unknown-key behavior, and the packet-05 deferral for `object_height:<id>`. | `python3 -c "from pathlib import Path; s=Path('docs/02_ir_schemas.md').read_text(); b=s.split('#### Typed config-scope ingestion (TASK-564)',1)[1].split('\n#### ',1)[0]; required=('ConfigScope','ScopeDelta','object_config:','paint_config:','tool_config:','warn','keep','object_height:<id>','packet 05'); missing=[x for x in required if x not in b]; assert not missing, missing"`
- **AC-8. Given** the `docs/04_host_scheduler.md` section “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)” currently requires `wall_generator` to be read directly from the raw config source, **when** packet 03 documents the implemented selector handoff, **then** that bounded section names `IngestionOutcome.selector_values`, `ConfigScope::Global`, `DEFAULT_WALL_GENERATOR`, and `spiral_vase`, and contains neither `read directly from the raw config source` nor `config_source.get("wall_generator")`. | `python3 -c "from pathlib import Path; s=Path('docs/04_host_scheduler.md').read_text(); b=s.split('### Perimeter-generator selection',1)[1].split('\n### Support-generator selection',1)[0]; required=('IngestionOutcome.selector_values','ConfigScope::Global','DEFAULT_WALL_GENERATOR','spiral_vase'); forbidden=('read directly from the raw config source','config_source.get(\"wall_generator\")'); missing=[x for x in required if x not in b]; present=[x for x in forbidden if x in b]; assert not missing and not present, (missing,present)"`

## Negative Test Cases

- **AC-N1. Given** malformed scoped wire key `object_config:obj-a` with no canonical sub-key, **when** `ConfigIngestor::ingest_flat` decodes it, **then** ingestion rejects it as `ConfigIngestionError::MalformedScopeKey { wire_key: "object_config:obj-a", .. }` rather than treating it as global or unknown. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd malformed_scope_key_is_rejected -- --exact 2>&1 | tee target/test-output.log'`
- **AC-N2. Given** declared integer key `skirt_loops = "one"`, **when** registry-directed typing runs, **then** ingestion rejects it as `ConfigIngestionError::TypeMismatch` naming `skirt_loops`, expected type `int`, and authored value `"one"`; no delta is returned. | `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd declared_value_type_mismatch_is_rejected -- --exact 2>&1 | tee target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd -- --nocapture 2>&1 | tee target/test-output.log'`

## Authoritative Docs

- `docs/adr/0067-unified-config-schema-registry.md` — registry-first ordering, selector metadata, and joined declaration authority.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — flat wire compatibility, decode-once boundary, and scope-delta semantics.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — eligibility vocabulary consumed but not enforced until queue row 7.
- `CONTEXT.md` entries `Config scope`, `Scope delta`, `Config schema registry`, and `Authored value` — canonical terminology.
- `docs/22_test_quality.md` — production-oracle, loud-fixture, and negative-control requirements.
- `docs/19_visual_debug.md` sections `Request Shape` and `Reading A Bundle` — delegated/ranged visual-debug acceptance contract.
- `docs/04_host_scheduler.md` section “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)” — current raw-selector documentation and canonical ownership of the replacement typed-selector behavior.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section `Typed config-scope ingestion (TASK-564)` — document the wire-to-typed boundary and packet-05 dynamic-key deferral; verify with the command in `AC-7`.
- `docs/04_host_scheduler.md` section “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)” — replace the current requirement for direct raw-source `wall_generator` access with the implemented global typed-selector handoff while retaining classic/default and spiral-vase fallback semantics; verify with the command in `AC-8`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
