---
status: implemented
packet: config-scope-resolution_06_resolved-config-view
task_ids:
  - TASK-567
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: resolved-config-view

## Goal

Make every guest `ConfigView` (load-time binding and per-region layer dispatch) a registry-complete projection of resolved config, make `CONFIG_BLOCK` emission registry-driven over the full registry, remove all 87 load-bearing guest config-literal fallbacks, and enforce warn-then-drop handling for genuinely undeclared keys only after every host-consumed key is registered and the no-drop oracle passes.

## Scope Boundaries

This packet owns:

- registry-default seeding of resolved config;
- registry typing and bounds for `ResolvedConfig.extensions`;
- per-key `config_block` metadata and full-registry emission, with Orca-parity escaping of `CONFIG_BLOCK` string values;
- registration of the host-consumed keys that no declaration covers today;
- required `ConfigView` accessors;
- the exact 87-site guest cleanup, including the manifest declarations it needs;
- the no-drop end-to-end oracle;
- the packet-03 warn-to-drop transition;
- the Step-6 pre-existing blockers, fixed in-packet (Step 6a and 6a-bis):
  - `seed_expansion_context` (`crates/slicer-runtime/src/run.rs`) falls back to the registry default `0.4` when `nozzle_diameter` is unauthored; an authored value still wins;
  - the seam-planner aligned-mode coordinate fix in the `seam-planner-default` guest (`modules/core-modules/seam-planner-default/src/`): it emits planner (inset-boundary) coordinates, not mesh-corner coordinates;
  - the two pre-existing stock reds — the Packet-68 stamping red and the NegativeSpacing red — with their exact files named by the Step 6a-bis diagnosis.

It does not add eligibility policy, layer ranges, modifier typing, aliases, or Phase-C automatic-value expansion; it does not modify host seam escalation or expansion semantics beyond the `nozzle_diameter` registry-default fallback.

## Prerequisites and Blockers

- **Depends on packet 05** (`config-scope-resolution_05_scope-resolution-module`, `status: draft`, implementation in progress). This is a FORWARD-DEP whose names and shapes were reconciled against both packet 05's `design.md` and the working tree:
  - `resolve_scope_stack` and `query_z_grid`;
  - `ResolvedObjectLayerConfig { object_id: String, object_height: f64, layer_height: f64, first_layer_height: f64, support_raft_layers: u32 }`;
  - `ResolutionError::InvalidObjectHeight { object_id: String, value: Option<f64> }`.

  All four are in `crates/slicer-config/src/resolution.rs` and re-exported from the `slicer-config` crate root. It also supplies WIT `slicer:prepass-layer-planning@2.0.0`.
- **Consumes landed packet 01** (`status: implemented`) registry exports: `ConfigSchemaRegistry`, `RegistryEntry`, `assemble_registry`, `HostChannels::from_live` (`crates/slicer-config/src/lib.rs`).
- **Consumes landed packet 03** (`status: implemented`): `ConfigIngestor`, `IngestionOutcome`, `IngestionWarning::UnrecognizedKey { wire_key, key, suggestion }` (`crates/slicer-config/src/ingestion.rs`), which is currently warn-and-keep.
- **Consumes landed packet 04** (`status: implemented`): `expand_automatic_values` (`crates/slicer-config/src/lib.rs`).
- **Unblocks:** reliable resolved config delivery for queue rows 8–10.
- **Activation blockers:**
  - Packet 05's `packet.spec.md` reads `status: implemented`, and Step 1 re-reconciles the exports above against the landed tree.
  - Packet 05 also edits `crates/slicer-config/src/lib.rs`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `modules/core-modules/layer-planner-default/src/lib.rs` (a census guest), and several `crates/slicer-runtime/tests/**` files this packet migrates. None of these may be edited under this packet until packet 05 lands.

## Compatibility Checklist

- **IR schema versions:** unchanged. No versioned IR changes shape.
  - `ResolvedConfig.extensions` keeps its `BTreeMap<String, ConfigValue>` storage and hash/interner semantics; its content now includes seeded registry defaults.
  - `SliceOutcome` gains `ingestion_warnings`. It is a runtime API struct, not an IR.
- **WIT packages:** unchanged.
  - The new `ConfigView` accessors are Rust methods on `slicer_ir::ConfigView`, not WIT functions.
  - Packet 05's `prepass-layer-planning@2.0.0` is consumed, not bumped.
- **CLI schema wire:** `CONFIG_SCHEMA_WIRE_VERSION` (`crates/slicer-scheduler/src/manifest.rs`) is unchanged.
  - `build_config_schema_json` does not emit the new flag, and `ConfigFieldEntry`'s new field is `#[serde(skip)]`.
  - Its `host` array, built by `build_host_key_entries`, gains the newly registered runtime rows. These are data, not shape.
  - Runtime rows may now render `"default": null`. Host rows already permit that, because `HostConfigKey.default` is `Option<String>`.
  - No field is added. Under `docs/11_operational_governance_and_acceptance_gate.md` only additive fields bump the minor version, so there is no bump.
- **Registry:**
  - `HostRuntimeKey.default` becomes `Option<&'static str>`, so a key can be registered without a default.
  - `HOST_RUNTIME_KEYS` gains the Step-1-inventoried host-consumed keys. The keys verified at preflight are `gcode_flavor`, `printer_model`, `filament_colour`, `extruder_colour`, `filament_cost`, `printable_area`, `support_type`, `support_family`, `thumbnails`, `machine_max_acceleration_retracting`, and `extruder`. `extruder` is read from `extensions` by `crates/slicer-wasm-host/src/dispatch.rs` and gets no default, so seeding never injects it.
- **Manifest schema:**
  - optional snake_case `config_block` (bool) per `[config.schema.<key>]`, defaulting to `true`;
  - new guest declarations for keys the guests already read (see `design.md`).
- **G-code wire:** `CONFIG_BLOCK` bytes change deliberately:
  1. Every registry key with a value in the global resolved config and a reconciled `config_block = true` is emitted. This grows the block; the size is unmeasured and is derived from the registry at test time.
  2. String values are C-style escaped per canonical `ConfigOptionString::serialize` (`escape_string_cstyle`, OrcaSlicer `Config.cpp`).
  3. `ORCA_CONFIG_PADDING` is largely suppressed by the existing 96-key cap in `serialize_config_block`.
  4. Exactly four keys carry explicit `config_block = false`: the three `mmu_segmented_region_*` keys and `thumbnail_path`.
  5. `filament_diameter` keeps its per-filament rendering, now from the effective value instead of a hard-coded `1.75`.

## Acceptance Criteria

- **AC-1. Given** a loaded module whose schema declares keys absent from the authored source, **when** `build_live_execution_plan` binds it via `bind_module_config_view(module, &ResolvedConfig)`, **then** its `ConfigView` contains every exact key the module declares that has a registry default or an authored value, with its effective value; contains no undeclared key; and still carries `support_type` / `support_family` for `support-family:` claimants when authored. Neither `bind_module_config_view` nor `build_live_execution_plan` accepts a `HashMap<ConfigKey, ConfigValue>` source. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract resolved_config_view_tdd::production_binding_is_registry_complete_and_resolved -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_tdd::production_binding_is_registry_complete_and_resolved \.\.\. ok" target/test-output.log'`
- **AC-2. Given** an extension key declared as `float` with bounds and a default, **when** global/object/paint/tool deltas state it and `resolve_scope_stack` produces the effective config, **then** `ResolvedConfig.extensions` contains only the registry-typed, bounds-checked effective `ConfigValue::Float`. An explicitly authored value equal to the default remains a real override over a non-default lower scope. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd extensions_are_typed_bounded_and_presence_preserving -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test extensions_are_typed_bounded_and_presence_preserving \.\.\. ok" target/test-output.log'`
- **AC-3. Given** the live guest-source census, **when** production Rust under the 13 guests' `modules/core-modules/<guest>/src/**/*.rs` is scanned, **then** all three of the following hold. **Zero sites remain.** A site is a config-read chain that ends in `.unwrap_or(<number|bool|string literal>)`. A chain starts at a `ConfigView` read (`get`, `get_bool`, `get_int`, `get_float`, `get_string`, `get_abs_value`, or a guest wrapper over them such as infill-linker's `config_float`) and may pass through `.map` / `.filter` / `.or_else` / `.and_then`. The baseline is exactly 87 sites: arachne-perimeters 29, classic-perimeters 27, rectilinear-infill 10, overhang-classifier-default 4, tree-support 3, traditional-support 3, infill-linker 3, tree-support-planner 2, layer-planner-default 0, gyroid-infill 2, lightning-infill 2, wave-overhangs 1, traditional-support-planner 1. **The detector is calibrated.** It flags one verbatim baseline snippet per chain shape: direct, `.map`, `.or_else`, `.and_then(match…)`, the `config_float(..).filter(..)` helper, and `config.and_then(|c| c.get_abs_value(..))`. It does not flag `unwrap_or_else`, sort comparators, `unwrap_or(<const|expr>)`, or `#[cfg(test)]` code. **Every string-literal key read is declared.** This covers keys read through a `ConfigView` getter or a guest wrapper taking `(config, "<key>", ..)` (e.g. `cfg_bool`, `cfg_float`). Each must be declared in that guest's own manifest `[config.schema]` or be a support-family key injected by `bind_module_config_view`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract resolved_config_view_tdd::guest_ -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for t in guest_config_literal_fallback_census_is_zero guest_fallback_detector_is_calibrated guest_config_reads_are_declared; do rg -q "resolved_config_view_tdd::$t \.\.\. ok" target/test-output.log; done'`
- **AC-4. Given** the real registry (host channels plus every `modules/core-modules/*/*.toml` manifest) and a non-default global `ResolvedConfig` whose `extensions` also retains one undeclared authored key, **when** `ConfigSchemaRegistry::config_block_map` projects it, **then** its key set equals the keys of `to_config_map()` that are either a registry entry without `omit_from_config_block` or a `ResolvedConfig::typed_field_keys()` key with no registry entry, and each value equals the effective value. Exactly `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_interlocking_beam`, and `thumbnail_path` are absent through `config_block = false`, and the undeclared key is absent. The expected set is computed by registry iteration, never a literal key list. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd config_block_map_is_registry_driven -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test config_block_map_is_registry_driven \.\.\. ok" target/test-output.log'`
- **AC-5. Given** `resources/cube_4color.3mf` plus a synthesized flat config holding one value per exact registry key, **when** the real `run_slice` path ingests and resolves both inputs, **then** `SliceOutcome.ingestion_warnings` holds zero `IngestionWarning::UnrecognizedKey` for that population, and every owning module's view receives its declared key. The synthesized population is derived by registry iteration. Each value is the first of these that exists: the declared default rendered to wire form; the first declared enum value; the declared `min`; the declared `max`; or a type-neutral value (`false`, `0`, or `""`). Fixture absence is a hard failure, not a skip. A negative control withholds one declaration from the assembled registry and asserts the same oracle reports exactly that key. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e resolved_config_view_no_drop_tdd:: -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for t in cube_and_full_registry_config_have_zero_unrecognized_keys no_drop_oracle_detects_a_withheld_declaration; do rg -q "resolved_config_view_no_drop_tdd::$t \.\.\. ok" target/test-output.log; done'`
- **AC-6. Given** AC-5 is green, **when** ingestion receives undeclared `skrit_loops = "1"` against a registry that declares `skirt_loops` (declared by `modules/core-modules/skirt-brim/skirt-brim.toml`), **then** it emits exactly one `IngestionWarning::UnrecognizedKey { wire_key: "skrit_loops", key: "skrit_loops", suggestion: Some("skirt_loops") }`, and `skrit_loops` is absent from every `ScopeDelta` and from the `ResolvedConfig` returned by `resolve_scope_stack`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd unknown_key_warns_once_and_is_dropped_from_deltas_and_resolution -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test unknown_key_warns_once_and_is_dropped_from_deltas_and_resolution \.\.\. ok" target/test-output.log'`
- **AC-7. Given** the completed delivery contract, **when** the architecture docs are inspected, **then** all three docs carry the contract. `docs/02_ir_schemas.md` states that `ConfigView` is always resolved and registry-complete, and that the CONFIG_BLOCK viewer keys come from `config_block_map` over registered `HOST_RUNTIME_KEYS`. `docs/03_wit_and_manifest.md` documents the optional `config_block` flag (default `true`, any-false-wins across declarations), lists the four explicit false keys on one line, and documents C-style escaping. `docs/04_host_scheduler.md` states in its Packet 51 and Packet 73 sections that every declared key with a registry default or an authored value is present and the view is registry-complete. | `python3 -c "from pathlib import Path; a=Path('docs/02_ir_schemas.md').read_text(encoding='utf-8'); b=Path('docs/03_wit_and_manifest.md').read_text(encoding='utf-8'); c=Path('docs/04_host_scheduler.md').read_text(encoding='utf-8'); four=('config_block = false','thumbnail_path','mmu_segmented_region_max_width','mmu_segmented_region_interlocking_depth','mmu_segmented_region_interlocking_beam'); miss=[x for x in ('always resolved','registry-complete','config_block_map','HOST_RUNTIME_KEYS') if x not in a]+[x for x in ('any declaration','escape_string_cstyle') if x not in b]+([] if any(all(k in l for k in four) for l in b.splitlines()) else ['four-key line'])+[x for x in ('registry-complete','registry default or an authored value') if x not in c]; assert not miss, miss"`
- **AC-8. Given** a registry assembled from host channels plus at least one module declaring non-host keys with defaults (including a `float_or_percent` key with a `base_key`), **when** `resolve_scope_stack` resolves any target with no authored value for those keys, **then** the returned config's `to_config_map()` contains every exact registry key that has a registry default. Seeded defaults pass through Phase-B expansion exactly like authored values, so no covered percent default reaches the map unexpanded. Wildcard (`prefix:*`) entries and entries without a default seed nothing. Apart from the documented `support_line_width` expansion shadow written by `expand_automatic_values`, `extensions` holds no key that is a `declare_resolved_config!` field or a `selector` entry. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd every_resolved_config_carries_every_exact_registry_key -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test every_resolved_config_carries_every_exact_registry_key \.\.\. ok" target/test-output.log'`
- **AC-9. Given** a `CONFIG_BLOCK` map holding a multi-line `machine_start_gcode` value containing `M190`, a backslash, and a double quote, **when** it is serialized through `run_pipeline_with_raw_config`, **then** the value is emitted on one line, escaped per canonical `escape_string_cstyle` (`\n`, `\r`, `\\`, `\"`), and no line inside `CONFIG_BLOCK_START`..`CONFIG_BLOCK_END` begins with `M190`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd::config_block_escapes_multiline_strings_cstyle -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "gcode_header_thumbnail_config_blocks_tdd::config_block_escapes_multiline_strings_cstyle \.\.\. ok" target/test-output.log'`
- **AC-10. Given** AC-6's ingestion outcome for `skrit_loops = "1"`, **when** the runtime resolves it, binds a module with `bind_module_config_view`, and projects `ConfigSchemaRegistry::config_block_map`, **then** `skrit_loops` is absent from the bound `ConfigView` and from the `CONFIG_BLOCK` map. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract resolved_config_view_tdd::unknown_key_absent_from_binding_and_config_block_map -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_tdd::unknown_key_absent_from_binding_and_config_block_map \.\.\. ok" target/test-output.log'`
- **AC-11. Given** a real `run_slice` with `gcode_flavor = "klipper"` and one undeclared authored key, **when** the G-code is produced, **then** `CONFIG_BLOCK` contains, exactly once, a Step-1-chosen registry key declared only by a loaded module's manifest whose default differs from its `ORCA_CONFIG_PADDING` value, printed with its registry default. It also contains `; gcode_flavor = klipper` exactly once, and lacks the four `config_block = false` keys and the undeclared key. `run_slice` must return `Ok`; a registry assembly error fails the test. This proves production wiring through `run_slice_with_collector` → `run_postpass_with_thumbnail`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e resolved_config_view_no_drop_tdd::run_slice_config_block_is_registry_projection -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_no_drop_tdd::run_slice_config_block_is_registry_projection \.\.\. ok" target/test-output.log'`
- **AC-12. Given** the warn-to-drop flip, **when** each registered host-consumed key (`gcode_flavor`, `printer_model`, `filament_colour`, `extruder_colour`, `filament_cost`, `printable_area`, `support_type`, `support_family`, `thumbnails`, `machine_max_acceleration_retracting`, `extruder`, plus any Step 1 adds) is authored, **then** ingestion emits no `UnrecognizedKey` for it, and its value is present in the global `ScopeDelta` and in the resolved `to_config_map()`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd host_consumed_keys_survive_the_drop -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test host_consumed_keys_survive_the_drop \.\.\. ok" target/test-output.log'`
- **AC-13. Given** `ResolvedConfig::default()` and one non-default config (non-empty `filament_density`, a tree `support_type`, one `cli_opt` row set to `Some`), **when** the macro-generated `to_config_map` runs, **then** it matches the old hand-written map on every key that map emitted, with an identical `ConfigValue`. This includes `support_type` rendered via `as_canonical_str()`, `infill_type` in its current rendering, and `filament_density` omitted when empty. The generated map additionally emits every other `cli` / `plain` row and every `Some` `cli_opt` row. The oracle is the old hand map, moved verbatim into a `#[cfg(test)] fn legacy_to_config_map` in `crates/slicer-ir/src/resolved_config.rs` before the production copy is deleted. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-ir --lib generated_to_config_map_matches_legacy_rendering -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "generated_to_config_map_matches_legacy_rendering \.\.\. ok" target/test-output.log'`

## Negative Test Cases

- **AC-N1. Given** a registry-declared extension value outside its declared `min`/`max`, **when** `resolve_scope_stack` runs, **then** it returns `ResolutionError::Application(ConfigResolutionError::OutOfRange { key, value, .. })` naming the key and authored value, and yields no `ResolvedConfig`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd out_of_bounds_extension_is_rejected_atomically -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test out_of_bounds_extension_is_rejected_atomically \.\.\. ok" target/test-output.log'`
- **AC-N2. Given** a guest attempts to read a key it did not declare, **when** the resolved binding is materialized, **then** the key is absent even if another module or the host registry declares it, and `ConfigView::require_float` on it returns `Err` naming the key. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract resolved_config_view_tdd::resolved_binding_preserves_module_encapsulation -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_tdd::resolved_binding_preserves_module_encapsulation \.\.\. ok" target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests --check` (judge by exit code: `0` fresh, `1` stale, `3` infrastructure error)
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --test resolved_config_view_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract resolved_config_view_tdd:: 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e resolved_config_view_no_drop_tdd:: 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. [1-9][0-9]* passed" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — RC-4/RC-5, the "Drop-unknown ships in two steps" ingestion paragraph, "Guests and delivery", the no-drop gate under Testing, and queue row 6.
- `docs/adr/0067-unified-config-schema-registry.md` — one registry, per-field reconciliation rules, and declaration provenance.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — the typed scope boundary. The drop-unknown staging itself lives only in the plan, not in this ADR.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, and `docs/22_test_quality.md` (§2.1 self-referential oracle, §2.4 hand-maintained roster, §2.6 silent fixture skip, §4 derivation rule).
- Canonical OrcaSlicer:
  - `GCode::append_full_config` (`GCode.cpp`) — footer line format;
  - `ConfigOptionString::serialize` (`Config.hpp`) and `escape_string_cstyle` (`Config.cpp`) — string escaping;
  - `ConfigBase::load_from_gcode_file` (`Config.cpp`) — the 80-key loader gate the padding cap serves.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md`:
  - `ConfigView` mentions — document registry-complete, always-resolved `ConfigView`;
  - §"CONFIG_BLOCK viewer-key contract" — viewer keys are registered `HOST_RUNTIME_KEYS` projected by `config_block_map`, not raw passthrough.

  Verified by `AC-7`.
- `docs/03_wit_and_manifest.md` §"Config Field Types Reference" / "Common per-field keys" — document `config_block`, its any-false-wins reconciliation, the four exclusions, and string escaping. Verified by `AC-7`.
- `docs/04_host_scheduler.md` §"Layer Stage Dispatch ConfigView Sourcing (Normative — Packet 51)" and §"PrePass Config-View Plumbing (Normative — Packet 73)" — replace "absent keys return `None`" with "undeclared keys are absent; every declared key with a registry default or an authored value is present". Verified by `AC-7`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
