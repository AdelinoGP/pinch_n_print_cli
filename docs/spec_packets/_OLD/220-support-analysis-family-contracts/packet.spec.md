---
status: implemented
packet: 220-support-analysis-family-contracts
task_ids:
  - TASK-331
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on the implemented anchored-entity-execution packet (TASK-330).
---

# Packet Contract: support-analysis-family-contracts

## Goal

Split host support analysis from family strategy planning by adding exact-Z host queries, universal structural support plan/output contracts, and atomic per-region planner-renderer family selection with fatal pairing validation and no fallback filler.

## Scope Boundaries

This packet owns `PrePass::SupportAnalysis`, exact-Z occupancy/envelope service, universal `SupportPlanIR` and structured `SupportIR`, family claims/selection, host aggregation/dispatch, and the removal of missing-plan fallback semantics. Tree and traditional algorithms are downstream packets; anchored execution is consumed from TASK-330.

## Prerequisites and Blockers

- Depends on: implemented `anchored-entity-execution` (TASK-330), with exported `AnchoredEntity`, `AnchoredGeometryContract`, `CapabilityDerivedEventClosure`, `OrderedEventCollection`, and `AnchoredEventRuntimeHooks`.
- Unblocks: `tree-support-family` (TASK-332) and `traditional-support-family` (TASK-333).
- Activation blockers: exact universal role/schema migration and region-level loader representation must be reconciled with generated WIT consumers.

## Acceptance Criteria

- **AC-1. Given** support is enabled and the host has sliced objects, **when** `PrePass::SupportAnalysis` runs, **then** it commits one `SupportAnalysisIR` containing candidates, enforcer/blocker annotations, occupancy/termination surfaces, shared settings, baseline feasible envelope, and deterministic family assignments. | `rg -q 'SupportAnalysisIR' crates/slicer-ir/src crates/slicer-runtime/src && rg -q 'SupportAnalysis' crates/slicer-scheduler/src`
- **AC-2. Given** a family requests occupancy at a non-model physical Z, **when** the exact-Z host query service is called, **then** it returns normalized cached occupancy, blockers, eligible termination geometry, and baseline envelope for the requested object/region/Z. | `cargo test -p slicer-wasm-host --test contract exact_z_support_query -- --exact`
- **AC-3. Given** one family planner invocation emits structural entries, **when** the host aggregates the result, **then** `SupportPlanIR` entries contain `family_id`, demand IDs, body IDs, anchor index/Z, semantic `ExPolygon` roles, optional skeleton metadata, capabilities/provenance, and declined-candidate reasons, and contain no nozzle-width extrusion paths. | `cargo test -p slicer-wasm-host --test contract support_plan_structural_contract -- --exact`
- **AC-4. Given** family renderers emit paths for an anchored event, **when** host commit completes, **then** structured `SupportIR` preserves family ID, body ID, source demand IDs, object/region attribution, role, and extrusion paths through `Layer::Support`, path optimization, diagnostics, and G-code handoff. | `cargo test -p slicer-runtime --test integration structured_support_identity -- --exact` (the `integration` target is registered by TASK-330 packet 219 Step 0 and consumed here)
- **AC-5. Given** a region has canonical `support_family`, or compatibility aliases `support_type=normal*`/`classic*` or `tree*`/`hybrid*`, **when** module selection runs, **then** `normal*` and `classic*` map to the traditional family, `tree*` and `hybrid*` map to the tree family, planner and renderer are selected atomically from the same family, and per-region candidates are retained rather than globally deduplicated. | `cargo test -p slicer-scheduler --test scheduler_integration support_family_selection -- --exact`
- **AC-6. Given** all selected family plans are aggregated, **when** validation runs, **then** complete invalid bodies are dropped against exact-Z occupancy/routing cells and their demands receive structured unmet diagnostics; slicing continues degraded for unroutable support. | `cargo test -p slicer-wasm-host --test contract support_plan_validation -- --exact`
- **AC-7. Given** the host invokes a family planner, **when** the planner returns a declined candidate, **then** the decline reason is one of `declined-policy`, `no-route`, `blocked`, or `unsupported-mode` and no fallback support filler is generated. | `cargo test -p slicer-wasm-host --test contract support_decline_contract -- --exact`

## Negative Test Cases

- **AC-N1. Given** a manifest set contains a planner without a matching renderer or a renderer with a mismatched `support-family:<id>` claim, **when** startup validation runs, **then** slicing fails before execution with a structured family-pairing startup error and does not select another family as fallback. | `cargo test -p slicer-scheduler --test scheduler_integration support_family_pairing_rejected -- --exact`
- **AC-N2. Given** support is disabled, **when** prepass and layer execution run, **then** no support candidates, plans, anchored support events, or support paths are produced. | `cargo test -p slicer-runtime --test integration support_disabled_no_output -- --exact` (the `integration` target is registered by TASK-330 packet 219 Step 0 and consumed here)

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-scheduler --test scheduler_integration support_family_selection -- --exact`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - direct full read; §§3-9 and invariants 1-7, 13.
- `docs/02_ir_schemas.md` - delegated bounded summary for current SupportPlanIR/SupportIR schema documentation.
- `docs/03_wit_and_manifest.md` - delegated bounded summary for claims and WIT boundary.
- `docs/04_host_scheduler.md` - delegated bounded summary for claim dedup and stage order.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/02_ir_schemas.md` universal support plan/output section - `rg -q 'SupportAnalysisIR' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` support-family claims section - `rg -q 'support-family:' docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md` planner-renderer pairing section - `rg -q 'planner-renderer' docs/04_host_scheduler.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - documented traditional support contact/propagation/termination behavior.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - documented support-family geometry and exact-Z collision behavior.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
