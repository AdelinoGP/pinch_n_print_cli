# Requirements: typed-scope-ingestion

## Packet Metadata

- Grouped task IDs: `TASK-564`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Config ingestion currently guesses types before loaded-module schemas are available and carries scope inside strings that multiple downstream functions re-parse. Module-owned numeric values such as `"0"` and `"1"` therefore become booleans, while startup claim selection reads raw values through a separate exception path. This packet establishes one registry-directed ingestion boundary and retains only authored settings in typed scope deltas.

## In Scope

- Consume packet 01's promised per-run `ConfigSchemaRegistry`, including exact declarations, wildcard declarations such as `object_height:*`, field types, and `selector` metadata; use `AssemblyOutcome.registry` and carry `AssemblyOutcome.warnings: Vec<RegistryWarning>` opaquely. Packet 03 consumes no `RegistryWarning` variant or field; the warning enum is non-exhaustive from this packet's perspective, and all variant/field details are packet-01 diagnostics outside scope.
- Add `slicer_config::ingestion` with `ConfigScope`, `ScopeDelta`, `ScopedConfig`, `ConfigIngestor`, `IngestionOutcome`, `IngestionWarning`, and `ConfigIngestionError` as specified in `design.md`.
- Decode `object_config:<object_id>:<key>`, `paint_config:<semantic>:<key>`, and `tool_config:<tool_index>:<key>` once; store stripped canonical keys under typed scopes.
- Admit modifier config through `ConfigIngestor::ingest_delta(ConfigScope::Modifier { .. }, ..)` because modifier values already arrive inside typed model structure rather than a flat scope prefix.
- Keep `object_height:<id>` and `layer_height:<id>` as recognised global dynamic/wildcard keys until packet 05 replaces their guest formatting sites with typed per-object query input.
- Type every declared authored value from its reconciled `RegistryEntry.field_type`; accept already-correct variants, parse string-encoded sidecar values, permit integer-to-float normalisation, and reject lossy or incompatible coercions.
- Preserve undeclared values unchanged while emitting one deterministic warning per wire key. Suggest the lexicographically first nearest declared canonical key only when Levenshtein distance is at most two; otherwise return `suggestion: None`.
- Match exact and `<prefix>:*` registry declarations before classifying a key as undeclared.
- Build selector values only from registry entries with `selector == true` in `ConfigScope::Global`. Under packet 01's exact declarations, `wall_generator` is the only current startup claim control in that map: `host_declaration` sets ordinary host rows to false, `spiral_vase` declarations omit selector metadata and default false, and `support_family` is undeclared. `spiral_vase` and `support_type` remain ordinary typed global values; undeclared `support_family` follows this packet's warn-and-keep policy. Startup perimeter selection must consume typed `wall_generator`, not raw source strings.
- Reorder the production live-load path internally as manifest discovery → registry assembly → typed ingestion → claim selection → DAG/instance preparation, without requiring config to discover manifests.
- Adapt current config-binding/resolution compatibility functions to read `ScopedConfig`/`ScopeDelta` and not parse the three scope prefixes again. Their consolidation into one resolver remains packet 05.
- Preserve 3MF authored strings until registry typing; remove `coerce_string_to_config_value` as an ingestion authority rather than extending its host-only declaration probe.
- Keep packet 02's four oracle tests unchanged—`oracle_authored_values_reach_owning_module_config_views` plus the green controls `oracle_selector_matrix_exposes_each_perimeter_owner`, `oracle_population_derivation_controls`, and `oracle_comparator_is_type_aware_negative_control`—and make the formerly red principal pass. Packet 02's edit surface is one new test module, one aggregator registration, and one net-new `slicer-config` runtime dev-dependency; its already-present `zip` dev-dependency is reused unchanged.
- Add a visual-debug model-mode regression that checks `manifest.json` and its emitted image path after slicing `resources/cube_4color.3mf`.
- Add the canonical typed-ingestion documentation section to `docs/02_ir_schemas.md`.
- Update only the “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)” section of `docs/04_host_scheduler.md` so it no longer requires a raw-config `wall_generator` read and instead records the global `IngestionOutcome.selector_values` handoff, classic/default behavior, and spiral-vase fallback.
- Add normal `slicer-config` dependencies to `slicer-scheduler`, `slicer-wasm-host`, and `slicer-runtime`, the three crates that directly name packet-03 registry/ingestion types; `slicer-model-io` remains a syntax-only adapter and does not gain that dependency.

## Out of Scope

- Automatic-value expansion and `ExpansionContext` (queue row 4).
- Replacing all compatibility resolution functions with the final scope-stack/Z-grid resolver or replacing dynamic `object_height:<id>`/`layer_height:<id>` guest queries (queue row 5).
- Flipping unrecognised keys from warn-and-keep to drop, always-resolved `ConfigView`, or the no-drop end-to-end census (queue row 6).
- Enforcing `denied_scopes` or deleting legacy manifest override tables (queue row 7).
- Config aliases or Orca feature-gap key renames.
- Typed modifier-kind migration (queue row 8) and `Metadata/layer_config_ranges.xml` ingestion (queue row 9).
- WIT changes, IR schema-version changes, geometry algorithms, or any OrcaSlicer source port.
- Editing packet 01, packet 02, packet 04, or the approved queue plan.

## Authoritative Docs

- `docs/adr/0067-unified-config-schema-registry.md` — direct read.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — direct read.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — direct read.
- `CONTEXT.md` — only entries `Config scope` through `Automatic value`; the file is long and must not be loaded whole.
- `docs/02_ir_schemas.md` — only `Modifier Resolution Contract`, `ObjectConfig.data Population`, and `Config Key Namespaces`.
- `docs/22_test_quality.md` — direct read; production oracle and negative-control rules govern packet 02 reuse.
- `docs/19_visual_debug.md` — delegated or ranged reads only: `Request Shape` and `Reading A Bundle`.
- `docs/04_host_scheduler.md` — only “Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)”; it currently specifies the raw read that this packet removes.

## Acceptance Summary

- Positive: `AC-1` through `AC-8` in `packet.spec.md` prove decode-once scope storage, exact registry typing, warn-and-keep diagnostics, exact selector extraction plus typed non-selector claim controls, unchanged predecessor oracles, visual-debug reachability, and both owned documentation updates.
- Negative: `AC-N1` and `AC-N2` reject malformed scope encodings and incompatible declared values.
- Cross-packet impact: packet 03 consumes packet 01's registry/assembly contract and opaque warning collection (no `RegistryWarning` variant or field), plus packet 02's test-only oracle, then exports typed deltas for packet 04; neither predecessor is represented as implemented.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-config --all-targets --test typed_scope_ingestion_tdd 2>&1 | tee target/test-output.log'` | Complete typed-ingestion success and rejection contract | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-model-io --all-targets --test threemf_project_settings_extraction_tdd 2>&1 | tee target/test-output.log'` | 3MF adapter preserves authored strings for later registry typing | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_contract typed_selector_claim_selection_tdd::typed_wall_generator_selector_claim_selection -- --exact --nocapture 2>&1 | tee target/test-output.log'` | Selector extraction contains only registry-marked `wall_generator`, and perimeter selection consumes it | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_integration config_resolution -- --nocapture 2>&1 | tee target/test-output.log'` | Ordinary typed values retain scope-resolution compatibility behavior | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-scheduler --all-targets --test scheduler_integration support_family_selection -- --exact --nocapture 2>&1 | tee target/test-output.log'` | Existing resolved support-family selection remains separate from registry selector extraction | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd -- --nocapture 2>&1 | tee target/test-output.log'` | Packet 02 oracle passes unchanged | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | Establish guest-artifact freshness before interpreting executor/visual-debug failures | FACT exit 0/1/3 |
| `bash -o pipefail -c 'mkdir -p target && cargo test -p pnp-cli --all-targets --test config_scope_ingestion_visual_debug_tdd 2>&1 | tee target/test-output.log'` | Real visual-debug bundle survives typed ingestion | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `python3 -c "from pathlib import Path; s=Path('docs/02_ir_schemas.md').read_text(); b=s.split('#### Typed config-scope ingestion (TASK-564)',1)[1].split('\n#### ',1)[0]; required=('ConfigScope','ScopeDelta','object_config:','paint_config:','tool_config:','warn','keep','object_height:<id>','packet 05'); missing=[x for x in required if x not in b]; assert not missing, missing"` | Canonical docs record the new boundary and explicit deferral | FACT pass/fail |
| `python3 -c "from pathlib import Path; s=Path('docs/04_host_scheduler.md').read_text(); b=s.split('### Perimeter-generator selection',1)[1].split('\n### Support-generator selection',1)[0]; required=('IngestionOutcome.selector_values','ConfigScope::Global','DEFAULT_WALL_GENERATOR','spiral_vase'); forbidden=('read directly from the raw config source','config_source.get(\"wall_generator\")'); missing=[x for x in required if x not in b]; present=[x for x in forbidden if x in b]; assert not missing and not present, (missing,present)"` | Scheduler docs replace the raw `wall_generator` requirement with the typed global-selector handoff while retaining fallback rules | FACT pass/fail |
| `cargo check --workspace --all-targets` | Compile all production and test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Review touched tests for detectable false-green patterns | FACT findings/no findings in touched files |

## Step Completion Expectations

- Packet 02's four oracle tests are read-only evidence: do not weaken assertions, narrow the fixture population, or edit expected types to obtain green; packet 03 turns the one red principal green while preserving the three green controls.
- The typed-ingestion API lands before adapters and claim-selection call sites switch, so compiler failures identify every remaining raw-map consumer.
- Unknown-key retention remains observable until packet 06; no compatibility adapter may silently discard an unknown entry.
- The scheduler perimeter-selection documentation changes in the same step that documents typed ingestion and must not continue to prescribe either direct raw-source access or `config_source.get("wall_generator")`.

## Context Discipline Notes

Treat `crates/slicer-model-io/src/loader.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-scheduler/src/execution_plan.rs`, and `docs/19_visual_debug.md` as ranged/delegated files. Cargo commands and the fixture/archive census must be delegated with bounded return formats.
