---
status: implemented
packet: config-scope-resolution_05_scope-resolution-module
task_ids:
  - TASK-566
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: scope-resolution-module

## Goal

Replace prefix-driven and default-difference config merging with one `slicer-config` resolution module that resolves typed scope deltas for both layer-planning Z-grid input and region scope stacks, then carries typed per-object planning inputs across `prepass-layer-planning@2.0.0`.

## Scope Boundaries

This packet owns the unified resolver, migration of the existing global/object/modifier/paint/tool call sites, the two production entry points, and the one accepted major bump of the layer-planning WIT package. It establishes the insertion seam and normative ordering for the future per-object layer-range scope, but queue row 9 owns range-file ingestion, world-Z interval matching, overlap validation/application, catch-up inheritance, and selector denial.

## Prerequisites and Blockers

- Depends on: packet 03, `config-scope-resolution_03_typed-scope-ingestion` (`status: implemented`), for `ConfigScope`, `ScopeDelta`, `ScopedConfig`, `ConfigIngestor`, and `IngestionOutcome`.
- Depends on: packet 04, `config-scope-resolution_04_automatic-value-expansion` (`status: implemented`), for `ExpansionContext`, `expand_automatic_values`, and `ExpansionError`.
- Unblocks: queue rows 6, 7, 8, 9, and 10.
- Activation blockers: cleared — packets 03 and 04 landed (`status: implemented`); Step 1 reconciled their exports with the names/shapes consumed here (all 8 symbols MATCH).

## Acceptance Criteria

- **AC-1. Given** typed deltas that all state `infill_density` at global `0.10`, object `0.20`, two modifiers in priority order `0.30` then `0.40`, paint semantics in lexical order `material = 0.50` then `support_enforcer = 0.60`, and tool 1 `0.70`, **when** `resolve_scope_stack` resolves the stack for object `obj-a`, both modifiers, both semantics, and tool 1, **then** the literal independently authored expectation is `0.70`; removing each highest scope in turn yields `0.60`, `0.40`, `0.20`, and `0.10`, proving `global < object < [layer range insertion seam] < modifier < paint semantic < tool`, modifier priority-ascending/last-writer-wins, lexical paint order, and tool-last without deriving expectations through the resolver. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd scope_stack_matches_hand_authored_precedence_table -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test scope_stack_matches_hand_authored_precedence_table .* ok" target/test-output.log'`
- **AC-2. Given** a global `wall_count = 3` and an object delta that explicitly states the registry default `wall_count = 2`, plus a module-declared extension key explicitly set to its default at paint scope, **when** `resolve_scope_stack` resolves those scopes, **then** it returns `wall_count = 2` and retains the paint value; no branch compares a delta value with `ResolvedConfig::default()`, and `overlay_resolved` plus the five old scheduler resolver/overlay functions no longer exist. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd explicit_default_is_a_real_override -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test explicit_default_is_a_real_override .* ok" target/test-output.log; ! rg -n "fn (overlay_resolved|apply_overlay|resolve_global_config|resolve_per_object_configs|resolve_per_paint_semantic_configs|resolve_per_tool_configs)" crates/slicer-core/src/algos/region_mapping.rs crates/slicer-scheduler/src/config_resolution.rs'`
- **AC-3. Given** object `obj-a` with world height `2.0`, global `layer_height = 0.20`, object `layer_height = 0.25`, global `first_layer_height = 0.30`, object `support_raft_layers = 2`, and packet 04's expansion context, **when** `query_z_grid` runs, **then** its `ResolvedObjectLayerConfig` is exactly `{ object_id: "obj-a", object_height: 2.0, layer_height: 0.25, first_layer_height: 0.30, support_raft_layers: 2 }`, and the query delegates merge/expansion to the same resolver used by region mapping rather than parsing a prefixed key. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd z_grid_query_returns_hand_computed_object_record -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test z_grid_query_returns_hand_computed_object_record .* ok" target/test-output.log'`
- **AC-4. Given** equivalent typed input supplied to `run_slice_with_collector` and `prepare_prepass_context`, **when** each path reaches layer planning and region mapping, **then** both consume `query_z_grid`/`resolve_scope_stack`, produce the same five fields for each `ResolvedObjectLayerConfig`, and contain no `format!("object_height:` or `format!("object_config:` prefix reconstruction in `crates/slicer-runtime/src/run.rs`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration scope_resolution_module_tdd::both_entry_points_share_resolution_results -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "scope_resolution_module_tdd::both_entry_points_share_resolution_results .* ok" target/test-output.log; ! rg -n "format!\(\"(object_height|object_config):" crates/slicer-runtime/src/run.rs'`
- **AC-5. Given** the canonical layer-planning WIT and the default layer-planner guest, **when** the package is bumped once from `slicer:prepass-layer-planning@1.0.0` to `@2.0.0`, **then** `layer-planning.run` retains `objects`, `output`, and `config`, adds `object-configs: list<object-layer-config>`, and `object-layer-config` has exactly `object-id`, `object-height`, `layer-height`, `first-layer-height`, and `support-raft-layers`; host dispatch and SDK glue pass those records, while `object_layer_height`, `object_height`, and their `format!` prefix sites are absent from `modules/core-modules/layer-planner-default/src/lib.rs`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --all-targets --test contract prepass_layer_planning_v2_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "prepass_layer_planning_v2_tdd.* ok" target/test-output.log; ! rg -n "fn (object_layer_height|object_height)|format!\(\"(layer_height|object_height):" modules/core-modules/layer-planner-default/src/lib.rs'`
- **AC-6. Given** the new resolution architecture and WIT signature, **when** authoritative docs are updated, **then** `docs/02_ir_schemas.md` names `ResolvedObjectLayerConfig` and the complete precedence string, `docs/03_wit_and_manifest.md` names `slicer:prepass-layer-planning@2.0.0` and all five record fields, and `docs/04_host_scheduler.md` names both `query_z_grid` and `resolve_scope_stack`. | `python3 -c "from pathlib import Path; checks={'docs/02_ir_schemas.md':('ResolvedObjectLayerConfig','global < object < layer range < modifier < paint semantic < tool'),'docs/03_wit_and_manifest.md':('slicer:prepass-layer-planning@2.0.0','object-id','object-height','layer-height','first-layer-height','support-raft-layers'),'docs/04_host_scheduler.md':('query_z_grid','resolve_scope_stack')}; missing={p:[x for x in xs if x not in Path(p).read_text(encoding='utf-8')] for p,xs in checks.items()}; missing={p:x for p,x in missing.items() if x}; assert not missing, missing"`

## Negative Test Cases

- **AC-N1. Given** a Z-grid query for an object with no positive finite object height, **when** `query_z_grid` runs, **then** it returns `ResolutionError::InvalidObjectHeight { object_id, value }` and emits no partial planning record. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd invalid_object_height_is_rejected_atomically -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test invalid_object_height_is_rejected_atomically .* ok" target/test-output.log'`
- **AC-N2. Given** overlapping future layer ranges, **when** this packet's public ordering contract is inspected, **then** it states that later-starting `layer_height` wins inside overlap and conflicting values for the same non-`layer_height` key are a load error, while no Rust `LayerRange` scope or overlap implementation is falsely claimed or introduced before queue row 9. Exempt: the pre-existing packet-01 denied-scope vocabulary literal in `crates/slicer-config/src/lib.rs` (`ALLOWED_DENIED_SCOPES`/`PER_REGION_SCOPES`), which is data, not a scope implementation. | `python3 -c "from pathlib import Path; s=Path('docs/02_ir_schemas.md').read_text(encoding='utf-8'); required=('later-starting','layer_height','non-layer_height','load error','queue row 9'); missing=[x for x in required if x not in s]; assert not missing, missing; import subprocess; r=subprocess.run(['rg','-n','LayerRange|layer_range|height_range|z_range','crates/slicer-config/src','--glob','*.rs'],capture_output=True,text=True); q=chr(34); new=[l for l in r.stdout.splitlines() if q+'layer_range'+q+',' not in l]; assert not new, new"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — Resolution, Layer range geometry semantics, Guests and delivery, Testing, and queue row 5.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — decode-once scope deltas and resolution-phase expansion.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — loud rejection contract; enforcement remains row 7/9 work.
- `docs/02_ir_schemas.md` — current modifier/paint/tool precedence and resolved-config delivery.
- `docs/03_wit_and_manifest.md` and `docs/11_operational_governance_and_acceptance_gate.md` — per-stage WIT identity and major-bump policy.
- `docs/04_host_scheduler.md` — LayerPlanning/RegionMapping ownership and deterministic ordering.
- `docs/22_test_quality.md` — independently derived expectations and negative controls.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` — document the unified scope stack, future layer-range insertion/overlap contract, and typed Z-grid record; verified by `AC-6` and `AC-N2`.
- `docs/03_wit_and_manifest.md` — update the layer-planning package/version/signature table to `2.0.0`; verified by `AC-6`.
- `docs/04_host_scheduler.md` — replace scattered resolver/overlay prose with the two unified entry points; verified by `AC-6`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `layer_height_profile_from_ranges` and related `layer_height_profile` behavior; confirm only the later-starting-wins Z-grid rule reserved for queue row 9, citing functions rather than line numbers.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
