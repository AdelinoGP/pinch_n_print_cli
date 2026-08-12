---
status: draft
packet: 219-anchored-entity-execution
task_ids:
  - TASK-330
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Generated from the approved support-families and anchored-entities plan.
---

# Packet Contract: anchored-entity-execution

## Goal

Add a generic anchored-work IR and capability-derived execution path so each global-layer worker returns ordered event collections for planar and atomic Z-spanning entities without replacing global layers as the scheduler barrier.

## Scope Boundaries

This packet owns the anchored entity data model, per-anchor stage closure, ordered event collection commit, contract-aware Z validation, and per-event optimization/cooling/time hooks. It preserves signed raft prefix layers and does not implement support-family analysis or a cross-layer scheduler. Support-family packets consume the exported anchored shapes.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: `support-analysis-family-contracts` (TASK-331).
- Activation blockers: exact additive/breaking schema and WIT version migration must be selected against the live generated guest set.

## Acceptance Criteria

- **AC-1. Given** an anchored entity with `anchor_global_layer_index`, stable local ID, planar `z`, requested/input capabilities, and provenance, **when** the scheduler builds its execution plan, **then** the entity is represented in an ordered event collection associated with that anchor and the collection preserves the entity ID and provenance fields. | `rg -q 'anchor_global_layer_index' crates/slicer-ir/src crates/slicer-scheduler/src && rg -q 'provenance' crates/slicer-ir/src crates/slicer-scheduler/src`
- **AC-2. Given** an anchor whose requested capabilities require `Layer::PathOptimization`, **when** capability closure is computed, **then** the closure includes `Layer::PathOptimization` and does not rely on a hardcoded event-kind table or feature-owned stage list. | `cargo test -p slicer-scheduler --test scheduler_integration capability_derived_anchor_closure -- --exact`
- **AC-3. Given** a planar anchored event and an ordinary same-Z model event, **when** the global-layer worker commits output, **then** planar events are ordered by physical Z before the upper anchor's model event and same-Z support remains in the ordinary model event ordering. | `cargo test -p slicer-runtime --test integration anchored_event_ordering -- --exact`
- **AC-4. Given** a Z-spanning entity with declared `min_z` and `max_z`, **when** path commit validation runs, **then** every point is within the declared range and the entity remains one atomic ordered event even when points lie outside the anchor model-layer envelope. | `cargo test -p slicer-runtime --test integration anchored_z_span_validation -- --exact`
- **AC-5. Given** a planar anchored entity whose point has Z outside its declared plane by more than coordinate tolerance, **when** commit validation runs, **then** the commit is rejected with the exact error fragment `anchored entity planar z mismatch` and no partial event is retained. | `cargo test -p slicer-runtime --test integration anchored_z_validation -- anchored_entity_planar_z_mismatch --exact`
- **AC-6. Given** an anchored event that is path-optimized, **when** optimization completes, **then** the event has an independently optimized ordered entity collection and its cooling/time accounting is recorded without reordering across physical event boundaries. | `cargo test -p slicer-runtime --test integration anchored_event_accounting -- --exact`
- **AC-7. Given** forced-serial and forced-parallel generation over identical immutable prepass state, **when** both executions complete, **then** ordered event collections and anchored geometry compare equal and the `layer-parallel-safe` hint governs concurrent anchored invocations. | `cargo test -p slicer-runtime --test integration anchored_parallel_determinism -- --exact`

## Negative Test Cases

- **AC-N1. Given** a Z-spanning entity with a path point outside `[min_z, max_z]`, **when** the host commits the event, **then** it rejects the entity with the exact error fragment `anchored entity z-span violation`, drops the complete entity, and does not silently clip the path. | `cargo test -p slicer-runtime --test integration anchored_z_span_validation -- rejects_out_of_range_point --exact`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration anchored_event_ordering -- --exact`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - direct full read; approved queue and architecture decisions.
- `docs/adr/0059-support-families-and-anchored-entities.md` - delegated bounded summary required before implementation; anchored ordering and global-layer barrier are governing constraints.
- `docs/adr/0009-raft-as-layer-infill-role.md` - delegated bounded summary required before implementation; signed raft-prefix behavior is retained.
- `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` - delegated bounded summary required before implementation; staged layer commit remains the seam.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/02_ir_schemas.md` anchored entities and ordered event collections section - `rg -q 'anchored entity' docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md` capability-derived anchored closure section - `rg -q 'capability-derived' docs/04_host_scheduler.md`
- `docs/03_wit_and_manifest.md` anchored event contract section - `rg -q 'anchor-global-layer-index' docs/03_wit_and_manifest.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
