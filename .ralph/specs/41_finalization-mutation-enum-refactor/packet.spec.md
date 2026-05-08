---
status: active
packet: finalization-mutation-enum-refactor
task_ids:
  - TASK-172
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: finalization-mutation-enum-refactor

## Goal

Close `DEV-041` by refactoring `FinalizationOutputBuilder`'s three mutation methods (`modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`) from closure-based APIs to **serializable-enum-based APIs** (`EntityMutation`, `SortKey`, `SyntheticLayerData`) so the WIT boundary can carry them losslessly. Wire the `slicer-macros` `run_finalization` drain-back loop to forward `merge_ops` through the WIT-bound `finalization-output-builder` resource. Add a WASM-side round-trip TDD test (a tiny new test guest under `test-guests/`) that proves a guest module's `modify_entity` call actually mutates the host-side IR. Remove the silent-no-op behavior described in `DEV-041`. Result: a WASM finalization module's mutation calls take effect end-to-end; the WIT surface for `modify-entity` / `sort-layer-by` / `insert-synthetic-layer-after` is no longer a contract lie.

## Scope Boundaries

- In scope:
  - Define three new types in `slicer-sdk` (or thread through from `slicer-ir`) and re-export at the SDK API boundary:
    - `EntityMutation` — serializable enum with variants for every PrintEntity field a near-future PostPass module plausibly mutates. Step 0 audits packet 40 design.md `## Open Questions` and the four future-module references (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) to enumerate concrete variants. Working draft: `SetSpeedFactor(f32)`, `SetExtrusionWidthFactor(f32)`, `SetFlowFactor(f32)`. Final list locked at Step 0 close.
    - `SortKey` — enum with at least: `ByPriorityAndEntityId`, `ByEntityId`, `ByObjectIdThenPriority`. Final list locked at Step 0 close.
    - `SyntheticLayerData` — record with `z: f32` and `paths: Vec<ExtrusionPath3D>`. Host fills all other `LayerCollectionIR` fields with sane defaults at apply time.
  - Replace SDK API:
    - `modify_entity<F: FnOnce(&mut PrintEntity) + 'static>(layer, id, op: F) -> Result<(), String>` → `modify_entity(layer, id, mutation: EntityMutation) -> Result<(), String>`
    - `sort_layer_by<F: Fn(&PrintEntity) -> K + 'static, K: Ord + 'static>(layer, key_fn: F) -> Result<(), String>` → `sort_layer_by(layer, key: SortKey) -> Result<(), String>`
    - `insert_synthetic_layer_after(idx, new_layer: LayerCollectionIR) -> Result<(), String>` → `insert_synthetic_layer_after(idx, data: SyntheticLayerData) -> Result<(), String>`
  - Refactor `MergeOp` enum in `crates/slicer-sdk/src/traits.rs` to plain serializable variants (no `Box<dyn FnOnce>` / `Box<dyn FnMut>`).
  - Update `apply_to` (host-side merge applier) to consume the new `MergeOp` variants — translate `EntityMutation` to a concrete in-place mutation; translate `SortKey` to a stable-sort comparator; translate `SyntheticLayerData` into a fresh `LayerCollectionIR` with default sibling fields. Preserve the existing 5-phase merge order from packet 40.
  - Migrate the 8 existing `crates/slicer-sdk/tests/finalization_builder_tdd.rs` tests from closure-form to enum-form. Behavior assertions stay identical; only the API call shape changes.
  - Reconcile WIT shape in `wit/world-finalization.wit` and the inline WIT in `crates/slicer-macros/src/lib.rs` `build_finalization_world_glue` — confirm SDK and WIT now share the same enum/record names, or align names if drift is found. The `wit_host.rs` `HostFinalizationOutputBuilder` impl SHOULD shrink (translation layer between WIT enum and SDK closure becomes a direct forward).
  - Extend `crates/slicer-macros/src/lib.rs` `run_finalization` drain-back (around lines 1198–1214) to iterate `sdk_output.merge_ops()` and forward each variant via the corresponding WIT method (`modify-entity`, `sort-layer-by`, `insert-synthetic-layer-after`). The catch-all that silently drops these MUST be removed.
  - Add a tiny new finalization test guest under `test-guests/finalization-mutation-roundtrip-guest/` that calls `output.modify_entity(layer, id, EntityMutation::SetSpeedFactor(0.5))` from `run_finalization`; bind it through the host so a host-side test can run it end-to-end.
  - Add at least one new host-side end-to-end test in `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (new file) that runs the new guest module and asserts the post-merge entity has `speed_factor == 0.5` AND that the same path through `modify_entity_unknown_id` round-trips an Err with the offending id.
  - Add `priority_pushes()` and `merge_ops()` accessors to the SDK builder if not already present, so the macro drain-back can iterate them. (Step 0 confirms current accessor surface; `priority_pushes()` was already exposed in Packet 40 Step 3a-fix.)
  - Insert a `TASK-172` row in `docs/07_implementation_status.md`.
  - Close `DEV-041` in `docs/14_deviation_audit_history.md` with a closure note dated at packet acceptance.
- Out of scope:
  - Implementing any of the four future PostPass modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`). They each consume this packet's APIs but each is its own future packet.
  - Adding new `ExtrusionRole` variants.
  - Adding a `Custom(Vec<u8>)` / `Patch(serde_json::Value)` escape-hatch variant to `EntityMutation`. Defer until a real motivating consumer surfaces.
  - Mirroring all `LayerCollectionIR` fields into `SyntheticLayerData`. Minimal `z` + `paths` only; expand when a real consumer needs more.
  - Changing the host's per-layer merge sequence in `crates/slicer-host/src/dispatch.rs`. Packet 40 Step 4 is closed; the dispatch.rs apply_to call site stays put.
  - Changing skirt-brim, wipe-tower, or top-surface-ironing module sources. None consume `merge_ops` and all three were migrated in Packet 40 follow-up.
  - Adding a runtime guard in slicer-macros for a non-empty `merge_ops` Vec. The user explicitly chose to skip this bridge: the gap remains until this packet closes it.
  - Producer-emitted entity reordering, role-priority changes, or any builder method beyond the three mutation methods being refactored.
  - Persistent / config-driven mutation registries.

## Prerequisites and Blockers

- Depends on:
  - Packet `40_finalization-mutation-builder` — `implemented`. Confirmed at packet 40 Step 6 acceptance ceremony 2026-05-07. `DEV-041` was registered at the same time.
- Unblocks:
  - Future modules: `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`. Each will be its own future packet and each requires the WIT round-trip this packet establishes.
- Activation blockers:
  - Step 0 must produce the locked `EntityMutation` and `SortKey` variant lists (audit of the four future modules). Activation requires those lists to be concrete, not "TBD".
  - Step 0 must FACT-confirm that `priority_pushes()` and `merge_ops()` accessors exist on the SDK builder OR that adding them is a one-line change in `slicer-sdk/src/traits.rs`. If the macros need pub access to private fields, the access path is locked at Step 0.

## Acceptance Criteria

- **Given** an SDK `FinalizationOutputBuilder` with a fixture layer holding three entities (ids `1, 2, 3`), entity_id `2` has `path.speed_factor == 1.0`, **when** a caller invokes `builder.modify_entity(layer, 2, EntityMutation::SetSpeedFactor(0.5))` and `builder.apply_to(&mut layers)`, **then** post-merge `layer.ordered_entities` entry with `entity_id == 2` has `path.speed_factor == 0.5` AND the other two entries are byte-unchanged. | `cargo test -p slicer-sdk --test finalization_builder_tdd modify_entity_set_speed_factor_applies -- --exact --nocapture`
- **Given** an SDK `FinalizationOutputBuilder` with a fixture layer holding three entities (ids `1, 2, 3`), each entry's path has a known `extrusion_width_factor`, **when** a caller invokes `builder.modify_entity(layer, 2, EntityMutation::SetExtrusionWidthFactor(0.7))` and `apply_to`, **then** post-merge entry with `entity_id == 2` has `path.extrusion_width_factor == 0.7` AND the others are byte-unchanged. | `cargo test -p slicer-sdk --test finalization_builder_tdd modify_entity_set_extrusion_width_factor_applies -- --exact --nocapture`
- **Given** a layer with five entities of varying roles in producer-emit order, **when** a caller invokes `builder.sort_layer_by(layer, SortKey::ByPriorityAndEntityId)` and `apply_to`, **then** post-merge `ordered_entities` is sorted ascending by `(role.default_priority(), entity_id)` AND every `TravelMove.entity_id` (where present) still resolves to an entity in the layer (Packet 39 anchor invariant survives reorder). | `cargo test -p slicer-sdk --test finalization_builder_tdd sort_layer_by_priority_and_entity_id -- --exact --nocapture`
- **Given** a `Vec<LayerCollectionIR>` with 3 layers, **when** a caller invokes `builder.insert_synthetic_layer_after(0, SyntheticLayerData { z: 0.85, paths: vec![<one path>] })` and `apply_to`, **then** post-merge the Vec has 4 layers in order `[layers[0], <new layer with z==0.85 carrying one entity built from the supplied path>, layers[1], layers[2]]` AND the new layer's `entity_id` namespace is independent (host-stamped from a fresh `LayerEntityIdGen`). | `cargo test -p slicer-sdk --test finalization_builder_tdd insert_synthetic_layer_after_inserts_at_position -- --exact --nocapture`
- **Given** a WASM finalization test guest at `test-guests/finalization-mutation-roundtrip-guest/` whose `run_finalization` calls `output.modify_entity(layer, 1, EntityMutation::SetSpeedFactor(0.5))`, **when** the host runs the full pipeline against a fixture with one layer containing one entity at `entity_id == 1` and `path.speed_factor == 1.0`, **then** post-pipeline the in-memory `LayerCollectionIR` for that layer has the entity's `path.speed_factor == 0.5`. **This is the substantive WIT-round-trip validation — the absence of this assertion is what made `DEV-041` a silent no-op.** | `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd modify_entity_round_trips_through_wit -- --exact --nocapture`
- **Given** the seven existing positive `finalization_builder_tdd` tests from Packet 40 (`push_with_priority_lands_at_sorted_position`, `modify_entity_by_id_applies_closure` → renamed `modify_entity_by_id_applies`, `sort_layer_by_applies_comparator` → renamed/migrated, `insert_synthetic_layer_inserts_at_position` → renamed/migrated, `legacy_push_preserves_prepend`, `ties_preserve_insertion_order`, plus the two `modify_entity_set_*_applies` ACs above), **when** the suite runs after this packet's refactor, **then** every test PASSES against the new enum-form API. | `cargo test -p slicer-sdk --test finalization_builder_tdd -- --nocapture`
- **Given** the `crates/slicer-macros/src/lib.rs` `run_finalization` drain-back loop, **when** the file is grepped for `merge_ops` iteration, **then** at least one matching iteration site exists AND no comment containing the string `silently no-op` or `DEV-041` survives (the gap is closed in code, not just hidden behind a TODO). | `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd drain_back_forwards_merge_ops -- --exact --nocapture`
- **Given** the existing Benchy regression tests from Packet 40 (`benchy_top_surface_precedes_ironing` and `benchy_gcode_contains_ironing_evidence`), **when** they run after this packet, **then** both still PASS — the substantive print-quality fix from Packet 40 is preserved, and the role-priority + dispatch-merge contract is unchanged. | `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`

## Negative Test Cases

- **Given** an SDK `FinalizationOutputBuilder` and a fixture layer with entities of ids `{1, 2}`, **when** a caller invokes `builder.modify_entity(layer, 99, EntityMutation::SetSpeedFactor(0.5))` and `apply_to`, **then** the result is `Err` whose diagnostic message contains the literal substrings `entity_id` AND `99` AND no entity in the layer is mutated. | `cargo test -p slicer-sdk --test finalization_builder_tdd modify_entity_unknown_id_errors -- --exact --nocapture`
- **Given** a `Vec<LayerCollectionIR>` with 3 layers, **when** a caller invokes `builder.insert_synthetic_layer_after(99, SyntheticLayerData { z: 0.0, paths: vec![] })` and `apply_to`, **then** the result is `Err` whose diagnostic message contains the literal substrings `synthetic` AND `99` AND the original Vec length is unchanged. | `cargo test -p slicer-sdk --test finalization_builder_tdd insert_synthetic_layer_out_of_bounds_errors -- --exact --nocapture`
- **Given** a WASM finalization test guest that calls `output.modify_entity(layer, 99, EntityMutation::SetSpeedFactor(0.5))` (unknown id), **when** the host runs the pipeline, **then** the host surfaces a diagnostic containing the literal substrings `entity_id` AND `99` AND the layer's entities are unmodified. | `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd modify_entity_unknown_id_round_trips_error -- --exact --nocapture`
- **Given** the SDK `FinalizationOutputBuilder` source, **when** the file is grepped for closure-bound generic signatures `F: FnOnce` or `F: Fn(&PrintEntity)` on `modify_entity` / `sort_layer_by`, **then** zero matches are found (the closure API is genuinely removed, not deprecated-and-still-callable). | `cargo test -p slicer-sdk --test finalization_builder_tdd closure_api_is_fully_removed -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test -p slicer-sdk --test finalization_builder_tdd -- --nocapture`
- `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo test -p skirt-brim -- --nocapture`
- `cargo test -p wipe-tower -- --nocapture`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only — run once at acceptance ceremony, never during implementation iterations)

## Authoritative Docs

- `docs/01_system_architecture.md` lines 328–363 — `PostPass::LayerFinalization` mutability contract. Direct read; narrow.
- `docs/03_wit_and_manifest.md` — WIT surface conventions and the host-boundary contract for module-facing types. Delegate SUMMARY ≤ 200 words for the `world-finalization` section if needed.
- `docs/05_module_sdk.md` — `FinalizationOutputBuilder` API description. Delegate SUMMARY for any section referencing closure bounds; this packet replaces those.
- `docs/02_ir_schemas.md` — `PrintEntity`, `ExtrusionPath3D`, `LayerCollectionIR`, `TravelMove` shapes. Direct read; narrow line ranges only.
- `docs/04_host_scheduler.md` lines 309–317, 680–717 — composable multi-writer patterns and PostPass scheduler shape. Direct read.
- `.ralph/specs/40_finalization-mutation-builder/design.md` — the predecessor's "Open Questions" section names the four future modules whose mutation needs drive this packet's `EntityMutation` variant choices. Direct read; narrow.
- `docs/14_deviation_audit_history.md` — `DEV-041` entry to be closed by this packet. Direct read; narrow (the entry is ~10 lines).

## OrcaSlicer Reference Obligations

None directly required. OrcaSlicer's per-layer mutation system is C++-template-based and not portable to a serializable enum surface, so it is not a parity reference for this packet. If parity is challenged for the speed/flow factor variants, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` (where MinLayerTime-style speed scaling is applied) for context only — not for byte-identical behavior. All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was authored under the spec-packet-generator's context_discipline preamble. Downstream agents must:

- Treat `design.md`'s code change surface as authoritative; touch nothing outside it.
- Honor `design.md`'s out-of-bounds list (no IR shape changes; no producer code changes; no module source changes; no dispatch.rs changes).
- Delegate every cargo run, every workspace search, and every authoritative-doc fact-check.
- Stop reading at 60% context; hand off at 85%.

This is a **API-shape-refactor + WIT-round-trip-completion** packet. The biggest implementation risks are (a) `EntityMutation` variant coverage being too narrow (Step 0's audit is the critical gate) and (b) the WASM round-trip test guest needing more plumbing than expected (test-guests crate setup, manifest, build script integration). AC-5 is the substantive validation — without it, the packet ships an unverifiable contract.
