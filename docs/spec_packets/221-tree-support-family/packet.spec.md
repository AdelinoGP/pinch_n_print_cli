---
status: draft
packet: 221-tree-support-family
task_ids:
  - TASK-332
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on implemented support-analysis-family-contracts (packet 220); activation blockers resolved.
---

# Packet Contract: tree-support-family

## Goal
Split the existing tree planner into `tree-support-planner` and make its paired `tree-support` renderer emit distributed, collision-safe structural support bodies and printable anchored events.

## Scope Boundaries
This packet owns the tree family algorithm, planner and renderer manifests, tree-specific config, and tree family tests. It consumes the universal analysis, exact-Z query, structural `SupportPlanIR`, structured `SupportIR`, and anchored execution contracts from TASK-331/TASK-330. Mixed-family conflict routing and final closure evidence remain downstream.

## Prerequisites and Blockers
- Depends on: implemented `support-analysis-family-contracts` (TASK-331, packet 220) and implemented `anchored-entity-execution` (TASK-330, packet 219), consumed as forward dependencies.
- Unblocks: `mixed-support-family-routing` (TASK-334) and `support-family-orca-closure` (TASK-335).
- Activation blockers: RESOLVED (both inherited from TASK-331). (1) Exact-Z seam ownership now lives in `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs`, injected into `HostExecutionContext`, normalized to repo units, immutable per-(object,region,Z) caching, returning occupancy, blockers, eligible termination geometry, and the baseline envelope. (2) The WIT migration is a breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0` (`CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` stays 1.0.0). Activation now waits only on this packet's own preflight.

## Acceptance Criteria
- **AC-1. Given** distributed overhang candidates assigned to `tree`, **when** `tree-support-planner` runs once per object, **then** `SupportPlanIR` contains stable `family_id=tree`, demand/body IDs, semantic `support_body` or interface regions, and distributed corner/contour/interior contacts rather than a triangle-centroid-only contact. | `cargo test -p tree-support-planner --test tree_family_tdd distributed_contacts -- --exact`
- **AC-2. Given** a tree body at each planned physical Z, **when** exact-Z occupancy and baseline envelope are queried, **then** the emitted complete body polygons include the local branch radius and have no positive-area model overlap, including tapered lower branches. | `cargo test -p tree-support-planner --test tree_family_tdd radius_aware_collision -- --exact`
- **AC-3. Given** a reachable tree demand, **when** planning completes, **then** body/interface polygons and optional skeleton metadata connect the demand to an eligible plate/model termination while preserving merged source demand IDs and anchored support height/Z. | `cargo test -p tree-support-planner --test tree_family_tdd anchored_heights_and_termination -- --exact`
- **AC-4. Given** validated tree plan entries, **when** `tree-support` renders `Layer::Support`, **then** `SupportIR` entries retain `family_id`, body ID, demand IDs, role, attribution, and extrusion paths, and trunk diameter is represented by walls/fill rather than one extrusion width. | `cargo test -p tree-support --test tree_family_tdd polygon_renderer_identity -- --exact`
- **AC-5. Given** `support_type` is `tree*` or `hybrid*`, **when** family selection runs, **then** the matching `support-family:tree` planner and renderer are selected atomically and the manifests retain `support-planner`/`support-generator` role claims. | `rg -q 'support-family:tree' modules/core-modules/tree-support/tree-support.toml && rg -q 'support-family:tree' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'support-generator' modules/core-modules/tree-support/tree-support.toml && rg -q 'support-planner' modules/core-modules/tree-support-planner/tree-support-planner.toml`
- **AC-6. Given** support is disabled or a candidate is declined, **when** the tree family executes, **then** it emits no body, anchored event, or fallback filler and records a structured decline reason. | `cargo test -p tree-support-planner --test tree_family_tdd disabled_and_declined -- --exact`

## Negative Test Cases
- **AC-N1. Given** a tree body whose full polygon intersects exact-Z model occupancy or leaves its routing cell, **when** validation runs, **then** the complete body is dropped, its demands are unmet with a structured diagnostic, and no clipped or fallback geometry is rendered. | `cargo test -p tree-support-planner --test tree_family_tdd invalid_body_rejected -- --exact`
- **AC-N2. Given** a tree renderer receives a plan entry with a non-tree family ID, **when** render dispatch validates attribution, **then** it returns a family-attribution error and emits no `SupportIR` path. | `cargo test -p tree-support --test tree_family_tdd mismatched_family_rejected -- --exact`

## Verification
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tree-support-planner --test tree_family_tdd -- --exact`

## Authoritative Docs
- `docs/specs/support-families-anchored-entities-plan.md` - direct read, §§5-10 and invariants 1-7, 13-14.
- `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` - delegated bounded summaries.
- `docs/08_coordinate_system.md` - delegated summary for unit conversion.

## Doc Impact Statement
- `docs/15_config_keys_reference.md` tree support configuration section - implementation-time verification: `rg -q 'support_family' docs/15_config_keys_reference.md`.
- `docs/02_ir_schemas.md` structural `SupportPlanIR` section - implementation-time verification: `rg -q 'SupportPlanIR' docs/02_ir_schemas.md`.
- `docs/03_wit_and_manifest.md` family claims and manifest section - implementation-time verification: `rg -q 'support-family:tree' docs/03_wit_and_manifest.md`.
- `docs/19_visual_debug.md` support-family visual inspection section - implementation-time verification: `rg -q 'tree-support-planner' docs/19_visual_debug.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388` `TreeSupport::generate_contact_points()` - grid sampling over the bounding box, rather than triangle-centroid-only contacts.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportSpotsGenerator.cpp:1130` `full_search(...)` - distributed support-point generation over overhang areas; stability check at `:845`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1839` `TreeSupport::get_collision(radius, layer_nr)`, `:1823` `get_avoidance(radius, obj_layer_nr)`, and `:1855` `get_collision_polys(radius, layer_nr)` - radius-aware exact-Z collision and avoidance.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:2652` `TreeSupport::drop_nodes()` - builds the tree/MST from contacts downward; `MinimumSpanningTree` is at `:2823-2834`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1969` `TreeSupport::draw_circles()` - per-layer collision-radius body emission.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:2050` area/interface (`roof_areas`) emission inside `draw_circles`; `:1772`/`:1792` `calc_branch_radius(...)` taper; `:2143` top-interface/termination on model/plate.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
