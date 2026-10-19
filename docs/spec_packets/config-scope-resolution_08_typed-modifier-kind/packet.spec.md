---
status: implemented
packet: config-scope-resolution_08_typed-modifier-kind
task_ids:
  - TASK-569
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: typed-modifier-kind

## Goal

Replace stringly modifier subtype routing with an exhaustive `ModifierKind` carried by `ModifierVolume`, validate modifier settings through the registry-backed modifier scope, and retire the inert `ModifierScope`/`applies_to` contract with the owner-approved one-minor MeshIR bump derived from the live constant at activation.

## Scope Boundaries

This packet owns the four non-normal modifier kinds, all ten grounded production classification sites, registry ingestion/admission of modifier deltas, the complete `ModifierVolume` construction/serde blast radius, and removal of the old subtype routing key and inert scope field. It preserves existing negative-part, support-enforcer/blocker, parameter-modifier, priority, and wall-less geometry behavior; layer-range scope remains queue row 9.

## Prerequisites and Blockers

- Depends on: **FORWARD-DEP** packet 05, `config-scope-resolution_05_scope-resolution-module`, for `resolve_scope_stack`, `ResolutionTarget`, and modifier precedence.
- Depends on: **FORWARD-DEP** packet 07, `config-scope-resolution_07_scope-eligibility`, for `ConfigSchemaRegistry::admission_set` and `ResolutionError::ScopeDenied { key, scope }`.
- Also consumes packet 03's draft `ConfigScope::Modifier { object_id, modifier_id }`, `ConfigIngestor::ingest_delta`, `ScopeDelta`, and `ScopedConfig` contracts.
- Unblocks: no queue dependency; it removes the modifier-kind ambiguity before later config work.
- Activation blockers: packets 05 and 07 must land, and all forward-export names/shapes must be reconciled before Step 1.

## Compatibility Checklist

- IR schema versions: `CURRENT_MESH_IR_SCHEMA_VERSION` receives exactly one minor increment from its live activation-time value, with major unchanged and patch zero, unconditionally under the approved owner decision in the plan. The authoring-time value was re-grounded as 1.1.0 but is not a future-version lock.
- WIT packages: unchanged; `MeshIR` remains host-owned and is not passed directly to guests.
- CLI schema wire: unchanged.
- Manifest schema: unchanged; modifier deltas consume packet 07's per-key `denied_scopes` authority.
- Serialized fixtures: pre-change `ModifierVolume` payloads must deserialize with the correct kind derived from their legacy `config_delta.fields["subtype"]`; if this cannot be preserved, implementation stops for compatibility escalation rather than shipping a broken minor bump.

## Acceptance Criteria

- **AC-1. Given** a 3MF sidecar containing `modifier_part`, `negative_part`, `support_enforcer`, and `support_blocker` parts, **when** `resolve_object` builds `ModifierVolume` values, **then** their `kind` fields are exactly `ModifierKind::ParameterModifier`, `ModifierKind::NegativePart`, `ModifierKind::SupportEnforcer`, and `ModifierKind::SupportBlocker`, respectively; no resulting `config_delta.fields` contains the routing key `subtype`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd modifier_parts_cross_mesh_ir_with_typed_kinds -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test modifier_parts_cross_mesh_ir_with_typed_kinds .* ok" target/test-output.log'`
- **AC-2. Given** the grounded ten production classification sites—`stamp_modifier_sub_region_configs`, `modifier_footprint_groups`, and `execute_region_mapping_inner` (`crates/slicer-core/src/algos/region_mapping.rs`); `slice_modifier_volumes` (`crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs`); `mesh_has_any_paint` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`); `split_modifier_sub_regions_for_prepass`, nested `build_paint_semantic_configs` inside `execute_prepass_with_builtins_configured_instr_collecting`, `apply_negative_part_subtract`, `stage_modifier_footprints`, and `collect_support_territory` in their grounded runtime files—**when** modifier routing compiles and its focused behavior tests run, **then** every site matches `ModifierKind` exhaustively, parameter modifiers still create/stamp wall-less sub-regions, negative parts still subtract, support kinds still route to support paint semantics, and no production `slicer-core`/`slicer-runtime` source reads `config_delta.fields.get("subtype")`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd::all_modifier_kinds_keep_their_geometry_routes -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "typed_modifier_kind_tdd::all_modifier_kinds_keep_their_geometry_routes .* ok" target/test-output.log; ! rg -n "config_delta\.fields\.get\(\"subtype\"\)" crates/slicer-core/src crates/slicer-runtime/src --glob "*.rs"'`
- **AC-3. Given** a parameter modifier on object `obj-a` with id `mod-a` stating declared `wall_loops = 4` and a declared module extension, **when** either `run_slice_with_collector` or `prepare_prepass_context` constructs scoped config and region mapping resolves the modifier sub-region, **then** both paths call registry-backed modifier ingestion under `ConfigScope::Modifier { object_id: "obj-a", modifier_id: "mod-a" }`, the typed host field is `wall_loops = 4`, the extension retains its registry-typed value, and the base region remains unchanged. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd::both_entry_points_registry_type_modifier_deltas -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "typed_modifier_kind_tdd::both_entry_points_registry_type_modifier_deltas .* ok" target/test-output.log'`
- **AC-4. Given** the activation step records the then-live `CURRENT_MESH_IR_SCHEMA_VERSION` before editing and the owner-approved compatibility decision requires a minor bump, **when** the IR contract tests run, **then** the final constant preserves that recorded major, increments that recorded minor by exactly one, sets patch to zero, `ModifierVolume` serializes exactly `id`, `mesh`, `config_delta`, `priority`, and `kind` but not `applies_to`, `ModifierScope` is absent from public exports, and a representative pre-change modifier payload derives the matching typed kind from its legacy `subtype` without retaining that key. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-ir --all-targets --test ir_tests typed_modifier_kind_mesh_ir_minor_contract -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test typed_modifier_kind_mesh_ir_minor_contract .* ok" target/test-output.log; ! rg -n "ModifierScope|applies_to" crates/slicer-ir/src --glob "*.rs"'`
- **AC-5. Given** a model-mode visual-debug request using a synthetic modifier-bearing 3MF, layer 0, tap `PrePass::RegionMapping`, and `silhouette`, **when** `run_visual_debug` renders after typed-kind migration, **then** it writes `manifest.json`, records a `PrePass::RegionMapping` image for layer 0, and that image path exists. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --all-targets --test typed_modifier_kind_visual_debug_tdd typed_modifier_kind_renders_region_mapping_bundle -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test typed_modifier_kind_renders_region_mapping_bundle .* ok" target/test-output.log'`
- **AC-6. Given** the typed modifier contract is implemented, **when** canonical docs are inspected, **then** `docs/02_ir_schemas.md` names the live `CURRENT_MESH_IR_SCHEMA_VERSION`, `ModifierKind`, all four variants, registry-typed modifier deltas, and contains neither `ModifierScope` nor `applies_to`; `docs/04_host_scheduler.md` names typed modifier-kind routing and registry admission; and ADR-0070 records implementation by `TASK-569`. | `python3 -c "from pathlib import Path; a=Path('docs/02_ir_schemas.md').read_text(); b=Path('docs/04_host_scheduler.md').read_text(); c=Path('docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md').read_text(); req=('CURRENT_MESH_IR_SCHEMA_VERSION','ModifierKind','ParameterModifier','NegativePart','SupportEnforcer','SupportBlocker','registry'); missing=[x for x in req if x not in a]; forbidden=[x for x in ('ModifierScope','applies_to') if x in a]; assert not missing and not forbidden and all(x in b for x in ('ModifierKind','registry')) and all(x in c for x in ('Implemented','TASK-569')),(missing,forbidden)"`

## Negative Test Cases

- **AC-N1. Given** a modifier delta states `bed_shape` or an out-of-bounds declared value, **when** either production entry point ingests/resolves it, **then** the operation fails atomically with packet 07's `ResolutionError::ScopeDenied { key, scope: ConfigScope::Modifier { .. } }` for `bed_shape` or the registry's typed/bounds error for the invalid value; no partial modifier config is interned. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd::invalid_modifier_delta_is_rejected_atomically -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "typed_modifier_kind_tdd::invalid_modifier_delta_is_rejected_atomically .* ok" target/test-output.log'`
- **AC-N2. Given** production source and test fixtures after migration, **when** obsolete surfaces are searched, **then** no Rust code contains `ModifierScope`, `.applies_to`, an `applies_to:` field, or a production comparison against modifier subtype strings; the old `acceptance_gate_gaps_tdd` tests `modifier_resolution_picks_highest_priority_within_scope`, `modifier_resolution_breaks_priority_ties_deterministically_by_id`, and `all_features_modifier_participates_in_every_scope_resolution` (with their `resolve_winner_for_scope` helper) that asserted fictional per-feature `ModifierScope` behavior are retired rather than weakened. | `bash -lc 'set -euo pipefail; ! rg -n "ModifierScope|\.applies_to|applies_to:" crates --glob "*.rs"; ! rg -n "config_delta\.fields.*subtype|subtype.*(negative_part|support_enforcer|support_blocker|modifier_part)" crates/slicer-core/src crates/slicer-runtime/src --glob "*.rs"; ! rg -n "resolve_winner_for_scope|modifier_resolution_picks_highest_priority_within_scope|modifier_resolution_breaks_priority_ties_deterministically_by_id|all_features_modifier_participates_in_every_scope_resolution" crates/slicer-runtime/tests/e2e/acceptance_gate_gaps_tdd.rs'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — RC-6/RC-9, Modifiers, owner decision 2, cross-cutting gates, and queue row 8.
- `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — accepted typed-kind, registry-routing, and old-scope retirement decision.
- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — wall-less parameter-modifier geometry that must not regress.
- `docs/02_ir_schemas.md` — MeshIR, modifier resolution, sidecar routing, and current 1.1.0 contract.
- `docs/04_host_scheduler.md` — RegionMapping and negative/support routing ownership.
- `docs/19_visual_debug.md` and `docs/22_test_quality.md` — geometry gate and falsifiable-test requirements.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` sections “IR 0 — MeshIR”, “Modifier Resolution Contract”, and “ModifierVolume.config_delta Sources” — document the activation-derived MeshIR minor bump, `ModifierKind`, and registry-typed deltas; verified by `AC-6`.
- `docs/04_host_scheduler.md` sections “RegionMapIR Compilation” and “Modifier-Part and Negative-Volume Routing” — replace subtype-string prose with typed routing and registry admission; verified by `AC-6`.
- `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` Status — record implementation by `TASK-569`; verified by `AC-6`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — confirm by function name that support modifier kinds remain excluded from ordinary per-volume config application and parameter modifiers retain wall-less region behavior; no C++ line-number citation may be emitted.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
