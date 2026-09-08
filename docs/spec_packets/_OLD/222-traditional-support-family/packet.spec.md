---
status: implemented
packet: 222-traditional-support-family
task_ids:
  - TASK-333
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on implemented support-analysis-family-contracts (packet 220); activation blockers resolved.
---

# Packet Contract: traditional-support-family

## Goal
Add a `traditional-support-planner` that plans cross-layer contact, base, interface, obstacle, and termination geometry, and make `traditional-support` render only its paired structural polygons.

## Scope Boundaries
This packet owns the traditional family planner, renderer, manifests, traditional support configuration, and family tests. It consumes TASK-331 strategy-neutral analysis, exact-Z queries, universal structural `SupportPlanIR` and structured `SupportIR`, plus TASK-330 anchored execution. Tree planning, mixed-family routing, and final closure evidence are separate packets.

## Prerequisites and Blockers
- Depends on: implemented `support-analysis-family-contracts` (TASK-331, packet 220) and implemented `anchored-entity-execution` (TASK-330, packet 219), consumed as forward dependencies.
- Unblocks: `mixed-support-family-routing` (TASK-334) and `support-family-orca-closure` (TASK-335).
- Activation blockers: RESOLVED (packet 220, 2026-08-13). (1) Exact-Z seam ownership: the host exact-Z support query service is `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs`, injected into `HostExecutionContext`, normalized to repo units, immutable per-(object,region,Z) caching, returning occupancy, blockers, eligible termination geometry, and baseline envelope. (2) WIT migration: breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0. These inherited blockers are resolved; this packet is no longer held draft by them.

## Acceptance Criteria
- **AC-1. Given** overhang candidates assigned to `traditional`, **when** `traditional-support-planner` runs, **then** `SupportPlanIR` contains stable `family_id=traditional`, demand/body IDs, and contact-area body/interface roles derived across layers rather than a per-layer filler. | `cargo test -p traditional-support-planner --test traditional_family_tdd contact_area_planning -- --exact`
- **AC-2. Given** accepted traditional contacts, **when** downward planning runs, **then** base polygons propagate through eligible layers, interface polygons honor `support_interface_top_layers`/`support_interface_bottom_layers` and the `support_base_pattern` key added by this packet, and obstacles are excluded using exact-Z occupancy. | `cargo test -p traditional-support-planner --test traditional_family_tdd base_interface_obstacle -- --exact`
- **AC-3. Given** a reachable traditional demand, **when** planning completes, **then** structural body/interface entries preserve all demand IDs and connect to an eligible plate/model termination with anchored support Z/heights. | `cargo test -p traditional-support-planner --test traditional_family_tdd anchored_termination -- --exact`
- **AC-4. Given** validated traditional plan entries, **when** `traditional-support` renders `Layer::Support`, **then** it scan-fills only planned body/interface polygons into attributed `SupportIR` and never reads `region.polygons()` or independently derives eligibility. | `cargo test -p traditional-support --test traditional_family_tdd planned_polygon_renderer -- --exact`
- **AC-5. Given** `support_type` is `normal*` or `classic*`, **when** family selection runs, **then** the matching `support-family:traditional` planner and renderer are selected atomically and manifests retain `support-planner`/`support-generator` role claims. | `rg -q 'support-family:traditional' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'support-family:traditional' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'support-generator' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'support-planner' modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
- **AC-6. Given** support is disabled or a candidate is declined, **when** the traditional family executes, **then** it emits no body, anchored event, or fallback filler and records a structured decline reason. | `cargo test -p traditional-support-planner --test traditional_family_tdd disabled_and_declined -- --exact`

## Negative Test Cases
- **AC-N1. Given** a traditional body polygon intersects exact-Z model occupancy or an obstacle, **when** validation runs, **then** the complete body is dropped, attached demands become unmet with structured diagnostics, and no clipped/fallback geometry is emitted. | `cargo test -p traditional-support-planner --test traditional_family_tdd invalid_body_rejected -- --exact`
- **AC-N2. Given** `traditional-support` receives a plan entry with a non-traditional family ID or no planned polygon, **when** rendering runs, **then** it returns a family-attribution/plan-required error and emits no filler path. | `cargo test -p traditional-support --test traditional_family_tdd mismatched_or_missing_plan -- --exact`

## Verification
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p traditional-support-planner --test traditional_family_tdd -- --exact`

## Authoritative Docs
- `docs/specs/support-families-anchored-entities-plan.md` - direct read, §§5-10 and invariants 1-7, 13-14.
- `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` - delegated bounded summaries.
- `docs/15_config_keys_reference.md` - delegated config-key summary.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374` `PrintObjectSupportMaterial::generate(PrintObject&)` - main orchestration.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2095` `top_contact_layers(...)` - contact-area/overhang detection (top contacts).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2592` `bottom_contact_layers_and_layer_support_areas(...)` - contact areas plus obstacle map.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:1451` - overhang grow into contact.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2760` `raft_and_intermediate_support_layers(...)` - placeholder intermediate/base layers.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2953` `generate_base_layers(...)` - downward propagation of support to plate.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3068`/`:3070` - projected-down geometry merge and trim by obstacle collision polygons.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3074` - base layer marking and termination of down-grow.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3106` `trim_support_layers_by_object(...)` - collision handling with gap_support_object/gap_xy offsets.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3208` `clip_by_pillars(...)` - collision/obstacle avoidance around pillars.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:480`/`OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` `generate_interface_layers(...)` - roof/floor interface.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2735` `trim_top_contacts_by_bottom_contacts(...)` - terminates top contacts by bottom contacts.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:523` `generate_support_layers(...)`; `:555` `generate_support_toolpaths(...)`; `:1980` `merge_contact_layers(...)`; `:487` `generate_raft_base(...)`.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` traditional family role section - `rg -q 'traditional-support-planner' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` family claims section - `rg -q 'support-family:traditional' docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md` traditional dispatch section - `rg -q 'traditional-support-planner' docs/04_host_scheduler.md`
- `docs/15_config_keys_reference.md` traditional support key ownership - `rg -q 'support_base_pattern' docs/15_config_keys_reference.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
