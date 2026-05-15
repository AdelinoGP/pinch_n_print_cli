---
status: draft
packet: 56c_threemf-negative-and-support-subtype-routing
task_ids:
  - TASK-192b
  - TASK-192c
  - TASK-193
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - 56_threemf-sidecar-parser (must be status: implemented)
  - 56b_threemf-modifier-part-ir-routing (must be status: implemented)
unblocks: []
---

# Packet Contract: 56c_threemf-negative-and-support-subtype-routing

> This packet is the third of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet. Packet 56 owns the sidecar parser (TASK-190). Packet 56b owns `resolve_object` branching + schema bump + `modifier_part` region overlap (TASK-191, TASK-192a). This packet (56c) owns the remaining two downstream consumers — `negative_part` and `support_enforcer`/`support_blocker` — plus the synthetic-fixture E2E coverage that ties them together (TASK-192b, TASK-192c, TASK-193). Aggregate cost is **M**.

## Goal

Wire the remaining two downstream consumers for non-`NormalPart` subtypes already routed into `ObjectMesh.modifier_volumes` by Packet 56b:

1. **`negative_part`** — Introduce a new host stage `apply_negative_part_subtract` (file: `crates/slicer-host/src/negative_part_subtract.rs`). Insert its invocation in `crates/slicer-host/src/pipeline.rs` between `execute_prepass_*` and `execute_region_mapping`. For each `ModifierVolume` whose `config_delta.fields[ConfigKey::from("subtype")] == ConfigValue::String("negative_part")`, iterate the parent object's per-layer slice polygons and replace each with `slicer_core::polygon_ops::difference(slice_polygons, projected_negative_per_layer)`. This is the Activation Q3 = Option 1 lock from the original packet.

2. **`support_enforcer` / `support_blocker`** — Augment `crates/slicer-host/src/paint_segmentation.rs` (or a sibling helper) to emit synthetic `PaintRegionIR` entries for each `support_enforcer` / `support_blocker` modifier volume. Project each volume per layer; map to `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. These synthetic entries flow through Packet 51's `paint_overrides` overlay; no new region-mapping code beyond Packet 56b's `modifier_part` overlap stamp.

This packet adds one new synthetic-fixture E2E suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds 3MF archives in-memory via the existing `zip::write::ZipWriter` pattern from `threemf_transform_tdd.rs`. The synthetic fixtures exercise: a `negative_part` reducing layer polygon area; a `support_enforcer` emitting `PaintRegionIR` at every overlapping layer; a `support_blocker` emitting `PaintRegionIR` at every overlapping layer.

No new deviations are registered by this packet. DEV-047, DEV-048, and DEV-049 are already closed by Packets 56 / 56b. The behavior added here is contract-conformant: it consumes existing `ObjectMesh.modifier_volumes` plumbing and Packet 51's paint-semantic overlay; no fallback paths.

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW file. Defines `apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` (signature locked at Step 2 via FACT dispatch on `SliceIR` shape). For each `negative_part` volume, projects per layer and calls `slicer_core::polygon_ops::difference` against each parent layer's polygons. Mutates `SliceIR` in place.
  - `crates/slicer-host/src/pipeline.rs` — insert `apply_negative_part_subtract(...)` call between `execute_prepass_*` and `execute_region_mapping` (per Activation Q3 = Option 1). Thread `modifier_volumes` into the new subtract call (by pulling them from the blackboard's mesh objects).
  - `crates/slicer-host/src/paint_segmentation.rs` — augment to accept `&[ModifierVolume]` (or pull from `ExecutionPlan`); emit synthetic `PaintRegionIR` per layer for each `support_enforcer` / `support_blocker` volume.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. Builds in-memory 3MF archives with `negative_part`, `support_enforcer`, `support_blocker` sidecars. Asserts: post-subtract polygon area, `PaintRegionIR` entries at correct layers, polygon match within ±0.005 mm² of the projected modifier volume.
  - `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows naming this packet.

- Out of scope:
  - Sidecar parser, `resolve_object` branching, schema bump, `modifier_part` region overlap, fuzzy-skin manifest gate — all closed by Packets 56 / 56b.
  - Any change to `crates/slicer-ir/`. `SliceIR`, `PaintRegionIR`, `PaintSemantic` already exist; this packet consumes them unchanged.
  - Any change to `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`. WIT scope confirmed clean by Packets 56 / 56b.
  - Any change to `crates/slicer-macros/`, `crates/slicer-sdk/`.
  - Any change to `crates/slicer-host/src/region_mapping.rs` or `model_loader.rs`. Owned by Packets 56b / 56 respectively; touching them here violates Cross-Packet Mutation Rule for in-flight active packets.
  - Any change to `modules/core-modules/fuzzy-skin/` (manifest already gated by Packet 56b).
  - Any new fuzzy-skin behavior (this packet does not stamp `fuzzy_skin.apply-to-all` for `negative_part` or `support_*` volumes — those subtypes have their own consumers).
  - Consuming `<part>/<metadata key="extruder">` per-modifier override (not consumed by this packet either; future work).
  - Sidecar `<assemble>` / `<plate>` sections.

## Prerequisites and Blockers

- Depends on:
  - **Packet 56 (`56_threemf-sidecar-parser`) status: implemented.** Provides `parse_3mf_sidecar` and `PartSubtype` enum.
  - **Packet 56b (`56b_threemf-modifier-part-ir-routing`) status: implemented.** Provides `resolve_object` branching, `MeshIR.schema_version == 1.1.0`, populated `ObjectMesh.modifier_volumes` for ALL non-`NormalPart` subtypes (including `negative_part` and `support_*` which were routed but had no consumer until this packet).
  - `slicer_core::polygon_ops::difference` (Clipper2-backed). Public export.
  - Packet 50b's paint-supports semantic and Packet 51's `paint_overrides` overlay remain intact (regression).
- Unblocks: nothing further. This is the terminal packet in the three-way split.
- Activation blockers (must be resolved before flipping `status: draft` → `active`):
  - **Q1 (Packets 56 and 56b status).** Confirm both are `status: implemented`. Verify by grep on each packet's `packet.spec.md`.

## Acceptance Criteria

- **Given** a synthetic 3MF archive (built in test code with an in-memory `zip::write::ZipWriter`) where the sidecar classifies part 2 as `subtype="negative_part"` whose mesh is a 5 × 5 × 5 mm cube positioned at the centroid of a 20 × 20 × 20 mm parent cube, **when** the model is loaded and the new `apply_negative_part_subtract` host stage runs, **then** for every layer Z in the negative volume's extent the post-subtract sum of layer polygon areas is strictly less than the pre-subtract sum AND for every layer Z outside the negative volume's extent the polygon areas are unchanged (verify by comparing two slice runs differing only in the presence of the negative sidecar entry). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area -- --exact --nocapture`
- **Given** the same synthetic fixture, **when** the polygon area reduction is measured at the centroid Z of the negative volume, **then** the reduction matches `25.0 mm²` (the negative cube's cross-section) within ±0.005 mm² (Clipper2 rounding tolerance). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_area_reduction_matches_cube_cross_section -- --exact --nocapture`
- **Given** a synthetic 3MF archive where part 2 is classified `support_enforcer`, **when** the model is sliced, **then** for every layer N intersecting the enforcer volume `PaintRegionIR.per_layer[N].semantic_regions.get(&PaintSemantic::SupportEnforcer) == Some(_)` AND the polygon set in that semantic matches the modifier's per-layer projection within ±0.005 mm² total area difference. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region -- --exact --nocapture`
- **Given** a synthetic 3MF archive where part 2 is classified `support_blocker`, **when** the model is sliced, **then** the same invariant holds with `PaintSemantic::SupportBlocker` in place of `PaintSemantic::SupportEnforcer`. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_blocker_emits_paint_region -- --exact --nocapture`
- **Given** the new `apply_negative_part_subtract` stage runs in the correct pipeline order (between prepass and region-mapping per Activation Q3 = Option 1), **when** a synthetic fixture combining `negative_part` AND a painted `paint_color` triangle covering the same area is sliced, **then** the painted region polygons reflect the post-subtract area (i.e., paint segmentation sees the reduced polygons). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_subtract_runs_before_paint_segmentation -- --exact --nocapture`
- **Given** the `support_enforcer` synthetic fixture runs through Packet 51's `paint_overrides` overlay, **when** `ResolvedConfig` is resolved for the support-enforcer semantic at any intersecting layer, **then** the resolved config carries the support-enforcer-specific override keys (verify by asserting at least one config key whose value differs between the support-enforcer semantic and the default — see Packet 51 acceptance criteria for the exact key set; reuse the same key list). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_flows_through_paint_overrides -- --exact --nocapture`
- **Given** clippy is the lint gate, **when** Step 5 runs, **then** `cargo clippy -p slicer-host --tests -- -D warnings` is green AND `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy -p slicer-host --tests -- -D warnings && cargo clippy --workspace -- -D warnings`
- **Given** the existing regression-defense surfaces must stay GREEN, **when** Step 5 runs, **then** `cargo test -p slicer-host --test threemf_transform_tdd` AND `cargo test -p slicer-host --test gcode_emit_tdd` AND `cargo test -p slicer-host --test benchy_painted_e2e_tdd` AND `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` AND `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` AND `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` all report all-pass. | `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd`
- **Given** TASK-192b, TASK-192c, and TASK-193 are registered by this packet, **when** Step 6 runs, **then** `docs/07_implementation_status.md` contains rows `[x] TASK-192b`, `[x] TASK-192c`, AND `[x] TASK-193` each naming this packet (`56c_threemf-negative-and-support-subtype-routing`). | `rg -q '\[x\] TASK-192b.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-192c.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-193.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md`
- **Given** no new deviations are introduced by this packet, **when** Step 6 runs, **then** `docs/DEVIATION_LOG.md` contains no DEV row whose status column reads `Closed — Packet 56c` (zero rows; the negative ensures no accidental DEV registration). | `! rg -q '^\| DEV-.*Closed.*Packet 56c' docs/DEVIATION_LOG.md`

## Negative Test Cases

- Negative volume entirely above the parent's Z-extent → no subtract occurs at any layer; parent polygons unchanged. Verified by:
  - **Given** a synthetic fixture where the `negative_part` cube's Z-min is greater than the parent's Z-max, **when** the loader runs and `apply_negative_part_subtract` executes, **then** the parent's per-layer polygons are bit-identical to a baseline run without the negative sidecar entry. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_above_parent_no_subtract -- --exact --nocapture`
- `support_enforcer` volume with zero triangles (degenerate) → no `PaintRegionIR` entries emitted; no warning. Verified by:
  - **Given** a synthetic fixture where the `support_enforcer` mesh has zero triangles, **when** sliced, **then** no `PaintRegionIR.per_layer[N].semantic_regions.get(&PaintSemantic::SupportEnforcer)` entries exist for any N AND no `log::warn!` is emitted (degenerate is not an error). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_support_enforcer_emits_nothing -- --exact --nocapture`

## Verification

- `cargo check --workspace` — compile health.
- `cargo clippy -p slicer-host --tests -- -D warnings` — lint gate (per-crate).
- `cargo clippy --workspace -- -D warnings` — lint gate (workspace).
- `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` — synthetic-fixture E2E.
- `cargo test -p slicer-host --test threemf_transform_tdd` — transform regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — G-code emission regression.
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — no-sidecar E2E regression.
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — paint-semantic regression.
- `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — Packet 56b's modifier-part E2E (must stay green).
- `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — Packet 56's parser suite (must stay green).
- `cargo test --workspace` — acceptance ceremony only (Step 7); dispatched via worker as `FACT pass/fail`. This packet closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice; a workspace-wide gate is justified at terminal closure of the three-way-split.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintRegionIR`, `PaintSemantic` (lines covering paint IR; delegate to the `PaintRegionIR` block search). Informational; no IR edit.
- `docs/04_host_scheduler.md` — prepass / region-mapping ordering. Delegate SUMMARY at Step 2 for the exact insertion point name.
- `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility. Informational.
- `docs/08_coordinate_system.md` — scaled integer units. Read directly.
- `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` (or sibling) — negative-part per-layer subtract entry. Delegate ONE Explore agent dispatch at Step 2 with the LOCATIONS contract.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) — support enforcer/blocker geometry paths. Delegate ONE Explore agent dispatch at Step 3 with the LOCATIONS contract.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

Aggregate cost is **M** (3M + 5S step distribution). Downstream agents:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor the out-of-bounds list — `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, OrcaSlicer source, `crates/slicer-sdk/`, `crates/slicer-ir/`, `crates/slicer-host/src/region_mapping.rs`, `crates/slicer-host/src/model_loader.rs` — they must not be loaded directly (the last two were closed by Packets 56 / 56b and are immutable here);
- delegate every `cargo` run via a sub-agent FACT contract;
- stop reading at 60% context and hand off at 85%.

This packet closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice. Step 7 runs `cargo test --workspace` exactly once at acceptance ceremony via worker FACT dispatch — the only packet in the three-way split that does so.
