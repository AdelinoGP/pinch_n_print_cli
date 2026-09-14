---
status: draft
packet: config-scope-resolution_06_resolved-config-view
task_ids:
  - TASK-567
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: resolved-config-view

## Goal

Make every guest `ConfigView` a registry-complete projection of resolved config, make `CONFIG_BLOCK` emission registry-driven, remove all 89 load-bearing guest config-literal fallbacks, and enforce warn-then-drop handling for genuinely undeclared keys only after the no-drop oracle passes.

## Scope Boundaries

This packet owns resolved-view materialization, registry typing for `ResolvedConfig.extensions`, per-key `config_block` metadata and emission, the exact 89-site guest cleanup, the no-drop end-to-end oracle, and the packet-03 warn-to-drop transition. It does not add eligibility policy, layer ranges, modifier typing, aliases, or Phase-C automatic-value expansion.

## Prerequisites and Blockers

- Depends on: packet 05, `config-scope-resolution_05_scope-resolution-module`, for `resolve_scope_stack`, `query_z_grid`, `ResolvedObjectLayerConfig`, and post-merge expansion.
- Also consumes landed packet 01 registry exports and packet 03's forward `ConfigIngestor`, `IngestionOutcome`, and `IngestionWarning::UnrecognizedKey` contract.
- Unblocks: reliable resolved config delivery for queue rows 8–10.
- Activation blockers: packet 05 and packet 03 must land and their final exported shapes must be reconciled.

## Compatibility Checklist

- IR schema versions: unchanged; `ResolvedConfig.extensions` retains its existing representation and hash/interner semantics.
- WIT packages: unchanged; packet 05's `prepass-layer-planning@2.0.0` is consumed, not bumped.
- CLI schema wire: no version bump unless `config_block` is deliberately exposed in `module config-schema`; the selected design keeps it registry/manifest metadata only.
- Manifest schema: optional snake_case `config_block` is added per key; absent means `true`.
- G-code wire: `CONFIG_BLOCK` bytes may change because every registry-declared, non-excluded effective key is emitted; the three `mmu_segmented_region_*` exclusions remain explicit through `config_block = false`.

## Acceptance Criteria

- **AC-1. Given** a loaded module whose schema declares keys absent from the authored source, **when** the production binding path receives packet 05's resolved config, **then** its `ConfigView` contains every key declared by that module with the registry-resolved effective value or registry default, contains no undeclared key, and `bind_module_config_view` no longer reads raw `config_source`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd::production_binding_is_registry_complete_and_resolved -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_tdd::production_binding_is_registry_complete_and_resolved .* ok" target/test-output.log'`
- **AC-2. Given** an extension key declared as `float` with bounds and a default, **when** global/object/paint/tool deltas state it and `resolve_scope_stack` produces the effective config, **then** `ResolvedConfig.extensions` contains only the registry-typed, bounds-checked effective `ConfigValue::Float`; an explicitly authored value equal to the default remains a real override. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test resolved_config_view_tdd extensions_are_typed_bounded_and_presence_preserving -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test extensions_are_typed_bounded_and_presence_preserving .* ok" target/test-output.log'`
- **AC-3. Given** the live guest-source census, **when** production Rust under `modules/core-modules/**/src/**/*.rs` is classified as a config-read chain (`get_bool`, `get_int`, `get_float`, `get_string`, `get_abs_value`, or equivalent `ConfigView` read) ending in `unwrap_or(<literal>)`, **then** zero such sites remain after deleting exactly 89 baseline sites across 13 guests: arachne-perimeters 29, classic-perimeters 27, rectilinear-infill 10, overhang-classifier-default 4, tree-support 3, traditional-support 3, infill-linker 3, tree-support-planner 2, layer-planner-default 2, gyroid-infill 2, lightning-infill 2, wave-overhangs 1, traditional-support-planner 1; the census excludes `unwrap_or_else`, sort comparators, non-config options, test code, and non-literal fallbacks. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd::guest_config_literal_fallback_census_is_zero -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_tdd::guest_config_literal_fallback_census_is_zero .* ok" target/test-output.log'`
- **AC-4. Given** the assembled registry and a non-default resolved config, **when** the G-code serializer builds `CONFIG_BLOCK`, **then** its key set equals registry entries with `config_block = true`, its values equal the effective resolved values, and only `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, and `mmu_segmented_region_interlocking_beam` are excluded by explicit `config_block = false`; the result is independent of a hand-maintained key roster. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration gcode_header_thumbnail_config_blocks_tdd::config_block_is_registry_driven -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "gcode_header_thumbnail_config_blocks_tdd::config_block_is_registry_driven .* ok" target/test-output.log'`
- **AC-5. Given** `resources/cube_4color.3mf` plus a synthesized flat config that independently authors one type-valid value for every exact, non-wildcard registry census key, **when** the real `run_slice` path ingests and resolves both inputs, **then** it reports zero `IngestionWarning::UnrecognizedKey` values for the census population and every owning module view receives its declared key; fixture absence is a hard failure, not a skip. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e resolved_config_view_no_drop_tdd::cube_and_full_registry_config_have_zero_unrecognized_keys -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_no_drop_tdd::cube_and_full_registry_config_have_zero_unrecognized_keys .* ok" target/test-output.log'`
- **AC-6. Given** AC-5's no-drop oracle is green, **when** ingestion receives undeclared `skrit_loops = "1"`, **then** it emits exactly one `IngestionWarning::UnrecognizedKey { wire_key: "skrit_loops", key: "skrit_loops", suggestion: Some("skirt_loops") }` and omits `skrit_loops` from every `ScopeDelta`, resolved config, guest `ConfigView`, and `CONFIG_BLOCK`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test resolved_config_view_tdd unknown_key_warns_once_and_is_dropped_everywhere -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test unknown_key_warns_once_and_is_dropped_everywhere .* ok" target/test-output.log'`
- **AC-7. Given** the completed delivery contract, **when** architecture docs are inspected, **then** `docs/02_ir_schemas.md` states that `ConfigView` is always resolved and registry-complete and `docs/03_wit_and_manifest.md` documents optional `config_block`, default `true`, and the three explicit false keys. | `python3 -c "from pathlib import Path; a=Path('docs/02_ir_schemas.md').read_text(); b=Path('docs/03_wit_and_manifest.md').read_text(); req_a=('ConfigView','always resolved','registry-complete'); req_b=('config_block','default','true','mmu_segmented_region_max_width','mmu_segmented_region_interlocking_depth','mmu_segmented_region_interlocking_beam'); missing=[x for x in req_a if x not in a]+[x for x in req_b if x not in b]; assert not missing, missing"`

## Negative Test Cases

- **AC-N1. Given** a registry-declared extension value outside its bounds, **when** ingestion/resolution runs, **then** it returns the existing registry typing/bounds error naming the key and authored value and produces no partial resolved config or `ConfigView`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test resolved_config_view_tdd out_of_bounds_extension_is_rejected_atomically -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test out_of_bounds_extension_is_rejected_atomically .* ok" target/test-output.log'`
- **AC-N2. Given** a guest attempts to read a key it did not declare, **when** the resolved binding is materialized, **then** the key is absent even if another module or the host registry declares it. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd::resolved_binding_preserves_module_encapsulation -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "resolved_config_view_tdd::resolved_binding_preserves_module_encapsulation .* ok" target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e resolved_config_view_no_drop_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — RC-4/RC-5, Extensions, Guests and delivery, no-drop gate, and queue row 6.
- `docs/adr/0067-unified-config-schema-registry.md` — one registry and declaration provenance.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed scope boundary and drop-unknown staging.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, and `docs/22_test_quality.md` — resolved delivery, manifest metadata, and independent-oracle rules.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` — document registry-complete, always-resolved `ConfigView`; verified by `AC-7`.
- `docs/03_wit_and_manifest.md` — document `config_block` and its three exclusions; verified by `AC-7`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
