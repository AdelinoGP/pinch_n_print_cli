# Requirements: 56c_threemf-negative-and-support-subtype-routing

## Problem Statement

Packet 56b (`56b_threemf-modifier-part-ir-routing`) routes ALL four non-`NormalPart` subtypes (`modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`) into `ObjectMesh.modifier_volumes` and wires the `modifier_part` consumer (region-mapping fuzzy overlap stamp). After Packet 56b lands, fixtures with `negative_part` or `support_*` parts have populated `modifier_volumes` entries — but no downstream consumer reads them. A `negative_part` cube does not subtract from the parent's slice polygons. A `support_enforcer` volume does not emit `PaintRegionIR` entries.

This packet (56c) closes that gap. It introduces:

1. A new host stage `apply_negative_part_subtract` in `crates/slicer-host/src/negative_part_subtract.rs`. The stage runs between prepass and region-mapping (Activation Q3 = Option 1 locked at original-packet-author time). For each `negative_part` modifier volume, it projects per layer and calls `slicer_core::polygon_ops::difference` against the parent's slice polygons.

2. Synthetic `PaintRegionIR` emission for `support_enforcer` and `support_blocker` volumes in `crates/slicer-host/src/paint_segmentation.rs`. Each volume is projected per layer; the projections are emitted as `PaintRegionIR` entries with `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. These flow through Packet 51's `paint_overrides` overlay path with no new region-mapping code.

3. A new synthetic-fixture E2E test suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds 3MF archives in-memory via the existing `zip::write::ZipWriter` pattern. The synthetic fixtures cover the three subtypes' consumer behavior plus pipeline-ordering correctness (negative subtract must run before paint segmentation).

No new IR types are introduced. `SliceIR`, `PaintRegionIR`, `PaintSemantic::SupportEnforcer`, `PaintSemantic::SupportBlocker` already exist (Packets 50b / 51). This packet is consumer-side wiring on already-populated IR.

No new deviations are registered. DEV-047, DEV-048, and DEV-049 were closed by Packets 56 and 56b. The behavior here is contract-conformant; the synthetic fixtures exercise positive paths only (plus two degenerate-case negative tests for completeness).

This packet is the third and terminal packet in the three-way split. It runs `cargo test --workspace` exactly once at acceptance ceremony — the only packet in the split that does so. This workspace gate confirms that the full original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice (sidecar parser → IR routing → all four consumer wirings) is operational without regressions.

WIT scope is **clean** — confirmed by Packets 56 / 56b. This packet introduces no IR types and is not re-checked.

This packet does not modify Packet 56's or Packet 56b's directories. Cross-Packet Mutation Rule satisfied.

## Task IDs (registered by this packet)

- **TASK-192b** — New host stage `apply_negative_part_subtract`. Inserts between prepass and region-mapping. Per-layer 2D subtract via `slicer_core::polygon_ops::difference` for each `negative_part` modifier volume.
- **TASK-192c** — Synthetic `PaintRegionIR` emission for `support_enforcer` and `support_blocker` modifier volumes via paint-segmentation piggyback. Flows through Packet 51's overlay.
- **TASK-193** — TDD coverage: synthetic-fixture E2E (`threemf_subtypes_synthetic_e2e_tdd.rs`); no-regression sweep; pipeline-ordering correctness assertion.

(TASK-190 = Packet 56. TASK-191, TASK-192a = Packet 56b. The three TASK ids in this packet complete the original packet's TASK-190..193 set.)

## In Scope

- Files-in-scope (write):
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW; `apply_negative_part_subtract` host stage.
  - `crates/slicer-host/src/pipeline.rs` — insert stage call between `execute_prepass_*` and `execute_region_mapping`.
  - `crates/slicer-host/src/paint_segmentation.rs` — augment to emit synthetic `PaintRegionIR` for `support_*` volumes.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW; synthetic-fixture E2E suite.
  - `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows naming this packet.

## Out of Scope

- Sidecar parser, `resolve_object` branching, schema bump, `modifier_part` region overlap, fuzzy-skin manifest gate — all closed by Packets 56 / 56b.
- Any change to `crates/slicer-ir/`. Existing IR types are sufficient.
- Any change to `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`. WIT clean.
- Any change to `crates/slicer-macros/`, `crates/slicer-sdk/`.
- Any change to `crates/slicer-host/src/region_mapping.rs` or `model_loader.rs`. Owned by Packets 56b / 56 respectively; immutable in this packet per Cross-Packet Mutation Rule.
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

- `apply_negative_part_subtract` mutates `SliceIR` in place: for each layer Z in the negative volume's extent, per-layer polygon area decreases by the negative volume's cross-section at Z within ±0.005 mm². Outside the extent, polygons are unchanged.
- The pipeline order places `apply_negative_part_subtract` BEFORE paint segmentation and region mapping (per Activation Q3 = Option 1). Paint segmentation sees the post-subtract polygons.
- `support_enforcer` modifier volumes emit `PaintRegionIR` entries at every overlapping layer with `PaintSemantic::SupportEnforcer`; polygons match the modifier's per-layer projection within ±0.005 mm² total area.
- `support_blocker` modifier volumes emit `PaintRegionIR` entries at every overlapping layer with `PaintSemantic::SupportBlocker`; same polygon match tolerance.
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
