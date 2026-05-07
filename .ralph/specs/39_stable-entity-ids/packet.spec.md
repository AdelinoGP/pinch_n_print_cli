---
status: implemented
packet: stable-entity-ids
task_ids:
  - TASK-170
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: stable-entity-ids

## Goal

Decouple `LayerCollectionIR.travel_moves` anchors from positional indices into `ordered_entities`. Add a per-layer-monotonic `entity_id: u64` to every entity in `ordered_entities`; change `TravelMove.entity_idx: u32 → TravelMove.entity_id: u64`; producers issue IDs at construction time; `gcode_emit.rs` resolves travel anchors via a per-layer `entity_id → index` map built once at emit time. **Pure refactor — zero G-code behavioral change.** Foundation packet for future PostPass mutators (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`, finalization-priority sort) which all need stable identity to mutate, reorder, or insert entities without rewriting positional anchors.

## Scope Boundaries

- In scope:
  - Add `entity_id: u64` field to whichever struct sits in `LayerCollectionIR.ordered_entities` (Step 0 FACT confirms exact struct name; predecessor session indicated `PrintEntity` or similar near `crates/slicer-ir/src/slice_ir.rs:1228`).
  - Migrate `TravelMove` from `entity_idx: u32` (or whatever the current field is named) to `entity_id: u64`.
  - Add a `LayerEntityIdGen` helper to `slicer-ir` (or `slicer-sdk`) that issues monotonic IDs per layer; producers must use it.
  - Migrate every existing producer site that constructs entities + travels to issue IDs and reference by ID:
    - `crates/slicer-host/src/layer_executor.rs:605-617` (PerimeterIR walls)
    - `crates/slicer-host/src/layer_executor.rs:619-636` (InfillIR sparse/solid/ironing)
    - `crates/slicer-host/src/layer_executor.rs:638-659` (SupportIR paths)
    - `crates/slicer-host/src/dispatch.rs:2861-2877` (finalization-stage merge)
  - Migrate `crates/slicer-host/src/gcode_emit.rs:182, 285-295`: build a `HashMap<u64, usize>` (`entity_id → index`) once per layer at the top of the emit loop; resolve travel anchors via the map.
  - Add an IR validation helper that detects dangling travel anchors (a `TravelMove.entity_id` not present in the layer's entities).
  - Sweep test fixtures across the workspace that construct `TravelMove` with `entity_idx` and migrate them to `entity_id`. Step 0 LOCATIONS dispatch returns the exhaustive list.
  - Bump IR schema version per `docs/02_ir_schemas.md` versioning rules.
  - Insert `TASK-170` row into `docs/07_implementation_status.md`.
- Out of scope:
  - `FinalizationOutputBuilder` API extensions (priority push, mutate, sort, synthetic layers) — owned by Packet 40.
  - Replacing `dispatch.rs:2877` `splice(0..0, ...)` with `extend` + role-priority sort — owned by Packet 40.
  - `ExtrusionRole::default_priority()` const fn — owned by Packet 40.
  - Any module-level call-site change in `top-surface-ironing` or `skirt-brim` — owned by Packet 40.
  - G-code byte-output change — this packet is regression-free by contract.
  - Mutation API for existing entities — owned by Packet 40.
  - Synthetic-layer insertion API — owned by Packet 40.

## Prerequisites and Blockers

- Depends on:
  - Packet `38-rev1_top-surface-ironing` — `implemented` (provides current FinalizationModule call sites whose travel anchors must continue to resolve).
- Unblocks:
  - Packet `40_finalization-mutation-builder` (priority sort + mutation API + ironing migration).
  - Future PostPass mutators (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`, synthetic layer insertion).
- Activation blockers:
  - Step 0 must confirm exact struct name in `LayerCollectionIR.ordered_entities` and exact current name of `TravelMove`'s anchor field (`entity_idx` is the working hypothesis).
  - Step 0 must enumerate every workspace site referencing the current anchor field name to size the test-fixture sweep.

## Acceptance Criteria

- **Given** a `LayerCollectionIR` constructed by any producer with N entities in `ordered_entities`, **when** the layer is finalized, **then** every entity has an `entity_id: u64` distinct from every other entity in the same layer, AND every `TravelMove.entity_id` references an entity present in the layer's `ordered_entities`. | `cargo test -p slicer-ir --test entity_id_invariants_tdd unique_per_layer_and_resolvable -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end, **when** the slicer produces G-code after the refactor, **then** every existing assertion in `benchy_end_to_end_tdd` continues to PASS (regression-free contract — same `;TYPE:` block presence and ordering as before this packet). | `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- **Given** a fixture layer with two entities at known XY positions and one `TravelMove` whose `entity_id` references the second entity, **when** `gcode_emit` runs, **then** the emitted `G0` travel move's start point matches the last extrusion endpoint of the first entity AND its end point matches the first extrusion start of the second entity (verified by parsing the G0 line's `X`/`Y` arguments against the fixture's known coordinates). | `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd travel_emitted_at_entity_id_endpoints -- --exact --nocapture`
- **Given** a fixture layer with three entities and one `TravelMove` referencing the third entity by `entity_id`, **when** the test permutes `ordered_entities` to `[third, first, second]` (synthetic reorder via `Vec::rotate_left(2)` in test code) and then calls `gcode_emit`, **then** the travel still resolves to the originally-third entity's endpoints — proving the anchor is index-independent. | `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd travel_survives_entity_reorder -- --exact --nocapture`
- **Given** a `LayerCollectionIR` serialized via the IR's bincode/serde derive, **when** the bytes are deserialized into a fresh `LayerCollectionIR`, **then** every entity round-trips its `entity_id: u64` field AND every `TravelMove` round-trips its `entity_id: u64` field — i.e. neither field is dropped, renamed, or aliased to a positional index during serialization. | `cargo test -p slicer-ir --test entity_id_invariants_tdd entity_id_round_trips_through_serde -- --exact --nocapture`
- **Given** a `LayerEntityIdGen` instance fresh-constructed for one layer, **when** `next()` is called K times, **then** K distinct strictly-monotonic `u64` IDs are returned starting at the documented base value (e.g., `1` — exact base set by Step 1 design). | `cargo test -p slicer-ir --test entity_id_invariants_tdd id_gen_is_strictly_monotonic -- --exact --nocapture`

## Negative Test Cases

- **Given** a `LayerCollectionIR` with two entities (IDs `1` and `2`) and a `TravelMove` whose `entity_id` is `99` (not present in the layer), **when** the IR validation helper runs, **then** it returns `Err` whose diagnostic message contains the literal substring `entity_id` AND the literal substring `99`. | `cargo test -p slicer-ir --test ir_validation_tdd dangling_travel_anchor_rejected -- --exact --nocapture`
- **Given** a `LayerEntityIdGen` shared across two threads (or simulated via interleaved `next()` calls), **when** both consumers call `next()`, **then** every returned ID is unique within the layer (the helper either uses interior mutability with atomic counter OR is `!Send` and the test asserts the latter via a `static_assertions::assert_not_impl_any!` check). | `cargo test -p slicer-ir --test entity_id_invariants_tdd id_gen_no_collision_under_contention -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `cargo build -p slicer-ir`
- `cargo build -p slicer-host`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test -p slicer-ir --test entity_id_invariants_tdd -- --nocapture`
- `cargo test -p slicer-ir --test ir_validation_tdd -- --nocapture`
- `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only — run once at acceptance ceremony, never during implementation iterations)

## Authoritative Docs

- `docs/02_ir_schemas.md` — `LayerCollectionIR` struct, `TravelMove` struct, schema versioning rules. Read directly; one section.
- `docs/01_system_architecture.md` lines 328–363 — `PostPass::LayerFinalization` mutability contract (motivates this refactor as the foundation for richer mutation primitives). Read directly; small section.
- `docs/04_host_scheduler.md` § Composable Multi-Writer Patterns (lines 309–317) — describes how multiple producers contribute to `ordered_entities`. Read directly; narrow.
- `docs/05_module_sdk.md` — only the existing `FinalizationOutputBuilder::push_entity_to_layer` signature for the legacy API surface (delegate SUMMARY ≤ 100 words for the relevant section).

## OrcaSlicer Reference Obligations

None. This is an internal IR refactor; OrcaSlicer's IR is structurally different and not a parity reference for entity identity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the spec-packet-generator's context_discipline preamble. Downstream agents implementing or reviewing this packet must:

- Treat `design.md`'s code change surface as authoritative; touch nothing outside it.
- Honor `design.md`'s out-of-bounds list (no FinalizationOutputBuilder API changes; no priority sort; no role-default table; no module call-site changes).
- Delegate every cargo run, every workspace `entity_idx` sweep, and every authoritative-doc fact-check.
- Stop reading at 60% context; hand off at 85%.

This is a foundation refactor packet. The biggest implementation risk is the test-fixture sweep volume — Step 0's LOCATIONS dispatch must return the exhaustive list of `entity_idx` references before Step 1 authoring begins, otherwise the fixture-migration step (Step 5) will overrun budget.
