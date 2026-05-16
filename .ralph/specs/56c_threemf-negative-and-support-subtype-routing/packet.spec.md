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

1. **`negative_part`** — Introduce a new host stage `apply_negative_part_subtract` (file: `crates/slicer-host/src/negative_part_subtract.rs`) with signature `pub fn apply_negative_part_subtract(slice_irs: &mut [SliceIR], modifier_volumes: &[ModifierVolume])`. Insert its invocation as a **phase-0 built-in inside `crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins_configured`**, before `commit_region_mapping_builtin` and before any phase-1 user prepass stage (which includes `PrePass::PaintSegmentation`). `pipeline.rs` itself is NOT modified — the prepass orchestration already runs paint segmentation and region mapping internally. For each `ModifierVolume` whose `config_delta.fields[&ConfigKey::from("subtype")] == ConfigValue::String("negative_part")`, project the mesh once via `slicer_core::slice_mesh_ex(&mv.mesh, &layer_zs)` to obtain `Vec<Vec<ExPolygon>>`, then per layer `li` replace each `slice_irs[li].regions[ri].polygons` with `slicer_core::polygon_ops::difference(&slice_irs[li].regions[ri].polygons, &projection[li])`. This is the Activation Q3 = Option 1 lock from the original packet, with the insertion point updated to reflect the real prepass topology.

2. **`support_enforcer` / `support_blocker`** — Augment `crates/slicer-host/src/paint_segmentation.rs` to emit synthetic `PaintRegionIR` entries for each `support_enforcer` / `support_blocker` modifier volume. `paint_segmentation.rs` reads modifier volumes directly from `mesh_ir.objects[].modifier_volumes` (no new parameter on `execute_paint_segmentation`). Project each volume per layer via `slice_mesh_ex`; map to `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. Synthetic entries are union-merged with any existing `Vec<SemanticRegion>` at the same `(layer, semantic)` via `slicer_core::polygon_ops::union`. These synthetic entries flow through Packet 51's `paint_overrides` overlay; no new region-mapping code beyond Packet 56b's `modifier_part` overlap stamp.

This packet adds one new synthetic-fixture E2E suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds 3MF archives in-memory via the existing `zip::write::ZipWriter` pattern from `threemf_transform_tdd.rs`. The synthetic fixtures exercise: a `negative_part` reducing layer polygon area; a `support_enforcer` emitting `PaintRegionIR` at every overlapping layer; a `support_blocker` emitting `PaintRegionIR` at every overlapping layer.

No new deviations are registered by this packet. DEV-047, DEV-048, and DEV-049 are already closed by Packets 56 / 56b. The behavior added here is contract-conformant: it consumes existing `ObjectMesh.modifier_volumes` plumbing and Packet 51's paint-semantic overlay; no fallback paths.

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW file. Defines `apply_negative_part_subtract(slice_irs: &mut [SliceIR], modifier_volumes: &[ModifierVolume])`. For each `negative_part` volume, projects via `slice_mesh_ex` once across all layer Zs and calls `slicer_core::polygon_ops::difference` per layer per region. Mutates `slice_irs[li].regions[ri].polygons` in place.
  - `crates/slicer-host/src/prepass.rs` — insert `apply_negative_part_subtract(...)` call as a phase-0 built-in inside `execute_prepass_with_builtins_configured`, before `commit_region_mapping_builtin` and before any phase-1 user prepass stage. Modifier volumes are pulled from the current object's `ObjectMesh.modifier_volumes` already accessible to the prepass orchestrator.
  - `crates/slicer-host/src/paint_segmentation.rs` — augment `execute_paint_segmentation` to read `mesh_ir.objects[].modifier_volumes` internally and emit synthetic `PaintRegionIR` per layer for each `support_enforcer` / `support_blocker` volume. No new parameter on the function signature.
  - `crates/slicer-host/src/lib.rs` (or the module-root file confirmed at Step 2 via FACT dispatch) — declare `pub mod negative_part_subtract`.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. Builds in-memory 3MF archives with `negative_part`, `support_enforcer`, `support_blocker` sidecars. Asserts: post-subtract polygon area, `PaintRegionIR` entries at correct layers, polygon match within ±0.005 mm² of the projected modifier volume.
  - `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows naming this packet.

- Out of scope:
  - Sidecar parser, `resolve_object` branching, schema bump, `modifier_part` region overlap, fuzzy-skin manifest gate — all closed by Packets 56 / 56b.
  - Any change to `crates/slicer-ir/`. `SliceIR`, `PaintRegionIR`, `PaintSemantic` already exist; this packet consumes them unchanged.
  - Any change to `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`. WIT scope confirmed clean by Packets 56 / 56b.
  - Any change to `crates/slicer-macros/`, `crates/slicer-sdk/`.
  - Any change to `crates/slicer-host/src/region_mapping.rs` or `model_loader.rs`. Owned by Packets 56b / 56 respectively; touching them here violates Cross-Packet Mutation Rule for in-flight active packets.
  - Any change to `crates/slicer-host/src/pipeline.rs`. The phase-0 insertion lands inside `prepass.rs`, which is already orchestrated by `pipeline.rs`'s existing `execute_prepass_with_builtins_configured` call.
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
- **Given** a synthetic 3MF archive where part 2 is classified `support_enforcer`, **when** the model is sliced, **then** for every global layer index `n` intersecting the enforcer volume `paint_region_ir.per_layer.get(&n).and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer))` is `Some(regions)` with at least one `SemanticRegion` AND the aggregate polygon area across all returned `SemanticRegion`s' `polygons` matches the modifier's per-layer projection area within ±0.005 mm² total area difference. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region -- --exact --nocapture`
- **Given** a synthetic 3MF archive where part 2 is classified `support_blocker`, **when** the model is sliced, **then** the same invariant holds with `PaintSemantic::SupportBlocker` in place of `PaintSemantic::SupportEnforcer` (aggregate area across all returned `SemanticRegion`s within ±0.005 mm² of the per-layer projection). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_blocker_emits_paint_region -- --exact --nocapture`
- **Given** the new `apply_negative_part_subtract` stage runs in the correct pipeline order (between prepass and region-mapping per Activation Q3 = Option 1), **when** a synthetic fixture combining `negative_part` AND a painted `paint_color` triangle covering the same area is sliced, **then** the painted region polygons reflect the post-subtract area (i.e., paint segmentation sees the reduced polygons). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_subtract_runs_before_paint_segmentation -- --exact --nocapture`
- **Given** the `support_enforcer` synthetic fixture runs through Packet 51's `paint_overrides` overlay, **when** `ResolvedConfig` is resolved for the `PaintSemantic::SupportEnforcer` semantic at any intersecting layer, **then** the resolved config's `fields` map contains an entry under `ConfigKey::from("support_threshold_angle")` (a Packet 51 paint-supports override key) AND the value at that key differs from the same key's value in the default `ResolvedConfig` for the same layer. (Key chosen from Packet 51's paint-supports override set; the test asserts at least one such key differs to prove the overlay is wired, not necessarily this specific key — substitute any key from `paint_overrides[PaintSemantic::SupportEnforcer]` that is non-default.) | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_flows_through_paint_overrides -- --exact --nocapture`
- **Given** clippy is the lint gate, **when** Step 5 runs, **then** `cargo clippy -p slicer-host --tests -- -D warnings` is green AND `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy -p slicer-host --tests -- -D warnings && cargo clippy --workspace -- -D warnings`
- **Given** the existing regression-defense surfaces must stay GREEN, **when** Step 5 runs, **then** `cargo test -p slicer-host --test threemf_transform_tdd` AND `cargo test -p slicer-host --test gcode_emit_tdd` AND `cargo test -p slicer-host --test benchy_painted_e2e_tdd` AND `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` AND `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` AND `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` all report all-pass. | `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd`
- **Given** TASK-192b, TASK-192c, and TASK-193 are registered by this packet, **when** Step 6 runs, **then** `docs/07_implementation_status.md` contains rows `[x] TASK-192b`, `[x] TASK-192c`, AND `[x] TASK-193` each naming this packet (`56c_threemf-negative-and-support-subtype-routing`). | `rg -q '\[x\] TASK-192b.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-192c.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-193.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md`
- **Given** no new deviations are introduced by this packet, **when** Step 6 runs, **then** `docs/DEVIATION_LOG.md` contains no DEV row whose status column reads `Closed — Packet 56c` (zero rows; the negative ensures no accidental DEV registration). | `! rg -q '^\| DEV-.*Closed.*Packet 56c' docs/DEVIATION_LOG.md`

## Negative Test Cases

- Negative volume entirely above the parent's Z-extent → no subtract occurs at any layer; parent polygons unchanged. Verified by:
  - **Given** a synthetic fixture where the `negative_part` cube's Z-min is greater than the parent's Z-max, **when** the loader runs and `apply_negative_part_subtract` executes, **then** the parent's per-layer polygons are bit-identical to a baseline run without the negative sidecar entry. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_above_parent_no_subtract -- --exact --nocapture`
- `negative_part` volume with zero triangles (degenerate) → `slice_mesh_ex` returns empty per-layer projection; `polygon_ops::difference` is short-circuited and parent polygons unchanged; no warning. Verified by:
  - **Given** a synthetic fixture where the `negative_part` mesh has zero triangles, **when** the loader runs and `apply_negative_part_subtract` executes, **then** every `slice_irs[li].regions[ri].polygons` is bit-identical to a baseline run without the negative sidecar entry AND no `log::warn!` is emitted. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_negative_part_no_subtract -- --exact --nocapture`
- `support_enforcer` volume with zero triangles (degenerate) → no `PaintRegionIR` entries emitted; no warning. Verified by:
  - **Given** a synthetic fixture where the `support_enforcer` mesh has zero triangles, **when** sliced, **then** for every global layer index `n` the lookup `paint_region_ir.per_layer.get(&n).and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer))` is `None` (or `Some(regions)` with `regions.is_empty()`) AND no `log::warn!` is emitted (degenerate is not an error). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_support_enforcer_emits_nothing -- --exact --nocapture`
- `support_blocker` volume with zero triangles (degenerate) → no `PaintRegionIR` entries emitted; no warning. Verified by:
  - **Given** a synthetic fixture where the `support_blocker` mesh has zero triangles, **when** sliced, **then** for every global layer index `n` the lookup `paint_region_ir.per_layer.get(&n).and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportBlocker))` is `None` (or `Some(regions)` with `regions.is_empty()`) AND no `log::warn!` is emitted. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_support_blocker_emits_nothing -- --exact --nocapture`

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
