# Requirements: 56c_threemf-negative-and-support-subtype-routing

## Problem Statement

Packet 56b (`56b_threemf-modifier-part-ir-routing`) routes ALL four non-`NormalPart` subtypes (`modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`) into `ObjectMesh.modifier_volumes` and wires the `modifier_part` consumer (region-mapping fuzzy overlap stamp). After Packet 56b lands, fixtures with `negative_part` or `support_*` parts have populated `modifier_volumes` entries — but no downstream consumer reads them. A `negative_part` cube does not subtract from the parent's slice polygons. A `support_enforcer` volume does not emit `PaintRegionIR` entries.

This packet (56c) closes that gap. It introduces:

1. A new host stage `apply_negative_part_subtract` in `crates/slicer-host/src/negative_part_subtract.rs` with signature `pub fn apply_negative_part_subtract(slice_irs: &mut [SliceIR], modifier_volumes: &[ModifierVolume])`. The stage runs as a phase-0 built-in inside `crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins_configured`, before `commit_region_mapping_builtin` and before any phase-1 user prepass stage including `PrePass::PaintSegmentation` (Activation Q3 = Option 1 locked at original-packet-author time; insertion point updated to reflect actual prepass topology). For each `negative_part` modifier volume, it projects via `slice_mesh_ex(&mv.mesh, &layer_zs)` and calls `slicer_core::polygon_ops::difference` against each `slice_irs[li].regions[ri].polygons`.

2. Synthetic `PaintRegionIR` emission for `support_enforcer` and `support_blocker` volumes in `crates/slicer-host/src/paint_segmentation.rs`. Modifier volumes are read directly from `mesh_ir.objects[].modifier_volumes` (already populated by Packet 56b) — no new parameter on `execute_paint_segmentation`. Each volume is projected per layer; the projections are emitted as `SemanticRegion` entries inserted into `LayerPaintMap.semantic_regions` under `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`, union-merged with any existing entries via `slicer_core::polygon_ops::union`. These flow through Packet 51's `paint_overrides` overlay path with no new region-mapping code.

3. A new synthetic-fixture E2E test suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds 3MF archives in-memory via the existing `zip::write::ZipWriter` pattern. The synthetic fixtures cover the three subtypes' consumer behavior plus pipeline-ordering correctness (negative subtract must run before paint segmentation) plus four degenerate-case negative tests (negative above parent, empty negative, empty support_enforcer, empty support_blocker).

No new IR types are introduced. `SliceIR`, `PaintRegionIR`, `PaintSemantic::SupportEnforcer`, `PaintSemantic::SupportBlocker` already exist (Packets 50b / 51). This packet is consumer-side wiring on already-populated IR.

No new deviations are registered. DEV-047, DEV-048, and DEV-049 were closed by Packets 56 and 56b. The behavior here is contract-conformant; the synthetic fixtures exercise positive paths only (plus two degenerate-case negative tests for completeness).

This packet is the third and terminal packet in the three-way split. It runs `cargo test --workspace` exactly once at acceptance ceremony — the only packet in the split that does so. This workspace gate confirms that the full original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice (sidecar parser → IR routing → all four consumer wirings) is operational without regressions.

WIT scope is **clean** — confirmed by Packets 56 / 56b. This packet introduces no IR types and is not re-checked.

This packet does not modify Packet 56's or Packet 56b's directories. Cross-Packet Mutation Rule satisfied.

## Task IDs (registered by this packet)

- **TASK-192b** — New host stage `apply_negative_part_subtract` with signature `(&mut [SliceIR], &[ModifierVolume])`. Inserted as a phase-0 built-in inside `prepass.rs::execute_prepass_with_builtins_configured` (before region-mapping commit and before user prepass stages). Per-layer 2D subtract via `slicer_core::polygon_ops::difference` for each `negative_part` modifier volume.
- **TASK-192c** — Synthetic `PaintRegionIR` emission for `support_enforcer` and `support_blocker` modifier volumes via paint-segmentation piggyback. `paint_segmentation.rs` reads `mesh_ir.objects[].modifier_volumes` directly (no new parameter). Flows through Packet 51's overlay.
- **TASK-193** — TDD coverage: synthetic-fixture E2E (`threemf_subtypes_synthetic_e2e_tdd.rs`) with 10 test functions (6 positive + 4 degenerate negative); no-regression sweep; pipeline-ordering correctness assertion.

(TASK-190 = Packet 56. TASK-191, TASK-192a = Packet 56b. The three TASK ids in this packet complete the original packet's TASK-190..193 set.)

## In Scope

- Files-in-scope (write):
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW; `apply_negative_part_subtract` host stage with signature `(&mut [SliceIR], &[ModifierVolume])`.
  - `crates/slicer-host/src/prepass.rs` — insert stage call as a phase-0 built-in inside `execute_prepass_with_builtins_configured`, before `commit_region_mapping_builtin` and before phase-1 user prepass stages.
  - `crates/slicer-host/src/paint_segmentation.rs` — augment `execute_paint_segmentation` to read `mesh_ir.objects[].modifier_volumes` internally and emit synthetic `PaintRegionIR` for `support_*` volumes.
  - `crates/slicer-host/src/lib.rs` (or module-root file confirmed at Step 2 via FACT dispatch) — declare `pub mod negative_part_subtract`.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW; synthetic-fixture E2E suite.
  - `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows naming this packet.

## Out of Scope

- Sidecar parser, `resolve_object` branching, schema bump, `modifier_part` region overlap, fuzzy-skin manifest gate — all closed by Packets 56 / 56b.
- Any change to `crates/slicer-ir/`. Existing IR types are sufficient.
- Any change to `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`. WIT clean.
- Any change to `crates/slicer-macros/`, `crates/slicer-sdk/`.
- Any change to `crates/slicer-host/src/region_mapping.rs` or `model_loader.rs`. Owned by Packets 56b / 56 respectively; immutable in this packet per Cross-Packet Mutation Rule.
- Any change to `crates/slicer-host/src/pipeline.rs`. The phase-0 insertion lands inside `prepass.rs`, which is already invoked by `pipeline.rs`'s existing `execute_prepass_with_builtins_configured` call.
- Any change to `modules/core-modules/fuzzy-skin/`. Manifest gated by Packet 56b.
- Any new fuzzy-skin semantics for `negative_part` or `support_*` volumes. Each subtype has its own consumer.
- Sidecar `<assemble>` / `<plate>` sections; `extruder` per-modifier consumer; sidecar matrix as geometry source.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintRegionIR`, `PaintSemantic`, `SliceIR` shapes (informational; delegate to the relevant section search).
- `docs/04_host_scheduler.md` — prepass / region-mapping ordering. Delegate SUMMARY at Step 2 for the exact insertion-point function name.
- `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility (informational).
- `docs/08_coordinate_system.md` — scaled integer units. Read directly.
- `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193.

## OrcaSlicer Reference Obligations

Host implementation MUST be project-internal Rust.

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` (or sibling) — negative-part per-layer subtract entry. Delegate ONE LOCATIONS dispatch at Step 2.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) — support enforcer/blocker geometry paths. Delegate ONE LOCATIONS dispatch at Step 3.

## Acceptance Summary (measurable outcomes)

- `apply_negative_part_subtract` mutates each layer's `SliceIR` in place via `slice_irs[li].regions[ri].polygons`: for each layer Z in the negative volume's extent, the aggregate polygon area across all `SlicedRegion`s decreases by the negative volume's cross-section at Z within ±0.005 mm². Outside the extent, polygons are bit-identical.
- The prepass order places `apply_negative_part_subtract` (phase-0 built-in inside `execute_prepass_with_builtins_configured`) BEFORE `commit_region_mapping_builtin` and BEFORE any phase-1 user prepass stage including `PrePass::PaintSegmentation` (per Activation Q3 = Option 1). Paint segmentation sees the post-subtract polygons.
- `support_enforcer` modifier volumes emit `SemanticRegion` entries into `LayerPaintMap.semantic_regions` under `PaintSemantic::SupportEnforcer` at every overlapping global layer index; the aggregate `polygons` area across all returned `SemanticRegion`s matches the modifier's per-layer projection within ±0.005 mm² total area.
- `support_blocker` modifier volumes emit `SemanticRegion` entries into `LayerPaintMap.semantic_regions` under `PaintSemantic::SupportBlocker` at every overlapping global layer index; same aggregate-area tolerance.
- The emitted `PaintRegionIR` for `support_*` flows through Packet 51's `paint_overrides` overlay, producing the support-enforcer / support-blocker per-semantic `ResolvedConfig` at every intersecting layer.
- All existing regression suites (`threemf_transform_tdd`, `gcode_emit_tdd`, `benchy_painted_e2e_tdd`, `benchy_painted_overrides_e2e_tdd`, `benchy_4color_modifier_part_e2e_tdd`, `threemf_sidecar_classification_tdd`) stay GREEN.
- `cargo clippy --workspace -- -D warnings` clean.
- `cargo test --workspace` clean at acceptance ceremony (Step 7).

## Negative Cases (explicit)

- Negative volume entirely above the parent's Z-extent → no subtract at any layer; parent polygons unchanged.
- Negative volume with zero triangles → `slicer_core::polygon_ops::difference` returns unchanged; no warning.
- `support_enforcer` volume with zero triangles → no `PaintRegionIR` entries emitted; no warning.
- `support_blocker` volume with zero triangles → no `PaintRegionIR` entries emitted; no warning.

## Cross-Packet Dependencies / Unblockers

- Depends on **Packet 56** (`56_threemf-sidecar-parser`) and **Packet 56b** (`56b_threemf-modifier-part-ir-routing`) both being `status: implemented`. Both `parse_3mf_sidecar` AND `resolve_object` branching MUST be operational before this packet's consumers can read `ObjectMesh.modifier_volumes`.
- Depends on `slicer_core::polygon_ops::difference` (Clipper2-backed).
- Depends on Packet 50b's `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker` enum variants.
- Depends on Packet 51's `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` overlay.
- Unblocks nothing further. Terminal packet in the three-way split.

## Verification Commands

```powershell
cargo check --workspace
cargo clippy -p slicer-host --tests -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd
cargo test -p slicer-host --test threemf_transform_tdd
cargo test -p slicer-host --test gcode_emit_tdd
cargo test -p slicer-host --test benchy_painted_e2e_tdd
cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd
cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd
cargo test -p slicer-host --test threemf_sidecar_classification_tdd
```

Per CLAUDE.md Test Discipline: `cargo test --workspace` runs exactly once at this packet's acceptance ceremony (Step 7) — dispatched via worker as `FACT pass/fail`. This is the terminal closure of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` three-way slice; a workspace-wide gate is justified here. Iterative steps use the targeted commands above.
