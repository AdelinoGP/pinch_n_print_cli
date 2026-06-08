---
status: draft
packet: 99
task_ids: [TASK-249]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 99 — Paint Pipeline Doc Sync

## Goal

Bring `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/07_implementation_status.md`, and `docs/08_coordinate_system.md` into line with the new paint pipeline shape that landed across packets 89-98; explicitly: rewrite the prepass-order section of `docs/01` to describe the new sequence (mesh_segmentation → mesh_analysis → user-early → region_mapping → slice → shell_classification → paint_segmentation → support_geometry → user-late), add the variant-chain region-splitting model, remove the obsolete `PrePass::MeshSegmentation [new — runs first]` block that described an unwired stage; bump `docs/02` SliceIR and RegionMapIR to 2.0.0 (per P91), document `variant_chain` on `RegionKey` and `SlicedRegion`, document `segment_annotations` (renamed from `boundary_paint`), document `ConfigId(u32)` + `configs: Vec<ResolvedConfig>` interner, REMOVE `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `PaintRegionRTreeIndex`, `MeshSegmentationIR`, `FacetPaintMark`, note the `PaintValue::Vector` deferred follow-up; add the `[[region_split]]` manifest schema section + priority registry + value-type validation rules + cross-manifest aggregation behavior to `docs/03`, REMOVE the `mesh-segmentation-output` WIT resource documentation; update `docs/04`'s stage-prerequisites table (`PrePass::MeshSegmentation` → no prerequisites + replace_mesh; `PrePass::PaintSegmentation` → SliceIR + RegionMapIR prerequisites + replace_slice_ir), document host-filtered dispatch contract, REMOVE the "guard-based fallback contract" sentence for paint-segmentation; mark paint-segmentation parity, mesh-segmentation wiring, region-splitting IR, and Phase 5 as implemented in `docs/07`, flag three follow-ups (community paint ingestion, PaintValue::Vector, host:raw_slice); add the constant-conversion table from spec §5 to `docs/08`; flip `docs/specs/orca-paint-segmentation-parity.md`'s `Status:` from `awaiting Slice Rework` to `implemented` (keep file as historical record); CONTEXT.md was already updated during planning (variant chain / painted variant / region-split semantic / segment annotation glossary; "region" ambiguity expanded) — verify the entries are present.

## Scope Boundaries

This packet is pure doc maintenance. No production code touched. The doc edits sync written knowledge with the implementation that already shipped in packets 89-98. Every doc edit either describes the post-packet state (additions) or removes content that no longer reflects reality (deletions). Verification is grep-based: each AC names a phrase that must appear (or must NOT appear) post-packet. Full file list in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packets 89, 90, 91, 92, 93, 94, 95, 96, 97, 98 all `implemented`. Doc sync only works if the implementation is final.
- Unblocks: nothing in this roadmap. Future paint-related packets may now reference `docs/02`'s updated SliceIR shape as authoritative.
- Activation blockers: all prior packets closed.

## Acceptance Criteria

### AC-1 — `docs/01_system_architecture.md` prepass-order section rewritten to the new 9-stage sequence

**Given** the new prepass order (mesh_segmentation → mesh_analysis → user-early → region_mapping → slice → shell_classification → paint_segmentation → support_geometry → user-late),
**When** `docs/01_system_architecture.md` is grepped,
**Then** the prepass-order section names each stage in this order; no stale "PrePass::MeshSegmentation [new — runs first]" wired-flag warning remains.

| `rg -B2 -A20 'PrePass::MeshSegmentation' docs/01_system_architecture.md | rg -q 'PrePass::MeshAnalysis' && ! rg -q 'new — runs first|unwired|placeholder' docs/01_system_architecture.md`

### AC-2 — `docs/01` adds variant-chain region-splitting model description

**Given** the new region-splitting model,
**When** `docs/01` is grepped,
**Then** it documents `variant_chain` as the discriminator that splits regions into painted variants; references `docs/02` for the IR shape and `docs/03` for the manifest declaration.

| `rg -q 'variant_chain|variant chain|painted variant' docs/01_system_architecture.md`

### AC-3 — `docs/02_ir_schemas.md` SliceIR + RegionMapIR bumped to 2.0.0

| `rg -q 'SliceIR.*2\.0\.0|version: 2\.0\.0' docs/02_ir_schemas.md && rg -q 'RegionMapIR.*2\.0\.0' docs/02_ir_schemas.md`

### AC-4 — `docs/02` documents `variant_chain` on `RegionKey` and on `SlicedRegion`; documents `segment_annotations`

| `rg -q 'variant_chain' docs/02_ir_schemas.md && rg -q 'segment_annotations' docs/02_ir_schemas.md`

### AC-5 — `docs/02` documents `ConfigId(u32)` + `configs: Vec<ResolvedConfig>` interner

| `rg -q 'ConfigId' docs/02_ir_schemas.md && rg -q 'configs:.*ResolvedConfig|configs interner|intern' docs/02_ir_schemas.md`

### AC-6 — `docs/02` REMOVES `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `PaintRegionRTreeIndex`, `MeshSegmentationIR`, `FacetPaintMark`

| `! rg -q 'PaintRegionIR|LayerPaintMap|SemanticRegion|PaintRegionRTreeIndex|MeshSegmentationIR|FacetPaintMark' docs/02_ir_schemas.md`

### AC-7 — `docs/02` notes `PaintValue::Vector` deferred follow-up

| `rg -q 'PaintValue::Vector|Vector\(Vec<f32>\)' docs/02_ir_schemas.md`

### AC-8 — `docs/03_wit_and_manifest.md` adds `[[region_split]]` manifest schema section + priority registry + value-type validation + cross-manifest aggregation

| `rg -q '\[\[region_split\]\]' docs/03_wit_and_manifest.md && rg -q 'CORE_REGION_SPLIT_PRIORITIES|priority registry' docs/03_wit_and_manifest.md && rg -q 'value_type|value-type' docs/03_wit_and_manifest.md && rg -q 'aggregat|cross-manifest' docs/03_wit_and_manifest.md`

### AC-9 — `docs/03` REMOVES `mesh-segmentation-output` WIT resource documentation

| `! rg -q 'mesh-segmentation-output' docs/03_wit_and_manifest.md`

### AC-10 — `docs/04_host_scheduler.md` updates PrePass stage-prerequisites table

**Given** the new stage shapes,
**When** the table is grepped,
**Then** it lists `PrePass::MeshSegmentation` with no prerequisites + produces MeshIR via replace_mesh; `PrePass::PaintSegmentation` with SliceIR + RegionMapIR prerequisites + produces split SliceIR via replace_slice_ir; the "guard-based fallback contract" sentence for paint-segmentation is REMOVED (guest path deleted in P97).

| `rg -q 'PrePass::MeshSegmentation.*replace_mesh' docs/04_host_scheduler.md && rg -q 'PrePass::PaintSegmentation.*replace_slice_ir' docs/04_host_scheduler.md && ! rg -q 'guard-based fallback contract' docs/04_host_scheduler.md`

### AC-11 — `docs/04` documents host-filtered dispatch contract

| `rg -q 'host-filtered dispatch|module_invocation_allowed|paint-transparent' docs/04_host_scheduler.md`

### AC-12 — `docs/07_implementation_status.md` marks paint-segmentation parity, mesh-segmentation wiring, region-splitting IR, Phase 5 as implemented

**Given** the packets,
**When** `docs/07` is inspected,
**Then** task entries `TASK-241` through `TASK-249` are marked implemented (or equivalent — match the existing notation); the 3 deferred follow-ups (community paint ingestion, PaintValue::Vector, host:raw_slice) are recorded as future items.

| `for t in TASK-239 TASK-240 TASK-241 TASK-242 TASK-243 TASK-244 TASK-245 TASK-246 TASK-247 TASK-248 TASK-249; do rg -q "$t" docs/07_implementation_status.md || { echo "MISSING: $t"; exit 1; }; done`

### AC-13 — `docs/08_coordinate_system.md` adds the constant-conversion table from spec §5

| `rg -q '100 nm|10\^-4 mm|/100' docs/08_coordinate_system.md && rg -q 'OrcaSlicer constant.*divide|conversion table' docs/08_coordinate_system.md`

### AC-14 — `docs/specs/orca-paint-segmentation-parity.md` flipped to `Status: implemented`

| `rg -q '^Status: implemented' docs/specs/orca-paint-segmentation-parity.md && ! rg -q 'awaiting Slice Rework' docs/specs/orca-paint-segmentation-parity.md`

### AC-15 — `CONTEXT.md` glossary entries `Variant chain`, `Painted variant`, `Region-split semantic`, `Segment annotation` present (already added in planning — verify)

| `rg -q 'Variant chain' CONTEXT.md && rg -q 'Painted variant' CONTEXT.md && rg -q 'Region-split semantic' CONTEXT.md && rg -q 'Segment annotation' CONTEXT.md`

### AC-16 — Workspace test / clippy / build-guests --check still green

| `cargo clippy --workspace --all-targets -- -D warnings && cargo xtask build-guests --check`

### AC-17 — Behavior preservation: g-code on wedge + cube_4color byte-identical vs post-P98 baseline (doc-only packet, zero production impact)

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p99-wedge.gcode && sha256sum /tmp/p99-wedge.gcode && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p99-cube.gcode && sha256sum /tmp/p99-cube.gcode`

## Negative Test Cases

### AC-N1 — No reference to `boundary_paint` survives in `docs/` after the rename in P91

| `! rg -q 'boundary_paint' docs/`

### AC-N2 — No reference to `commit_paint_regions` / `point_in_paint_region` / `paint_regions()` accessor survives in `docs/`

| `! rg -q 'commit_paint_regions|point_in_paint_region|fn paint_regions\(' docs/`

### AC-N3 — No reference to the WASM `mesh-segmentation` core-module survives in `docs/`

| `! rg -q 'core-modules/mesh-segmentation|modules/core-modules/mesh-segmentation' docs/`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests --check`
4. Each AC's grep command (run individually; doc edits are easily verified by grep).

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

This packet EDITS docs; the source docs are the targets. Read sections per edit.

- `docs/01_system_architecture.md` — read prepass-order section only.
- `docs/02_ir_schemas.md` — read SliceIR / RegionMapIR / RegionKey / RegionPlan / PaintValue sections.
- `docs/03_wit_and_manifest.md` — read manifest-schema section.
- `docs/04_host_scheduler.md` — read PrePass stage table + dispatch section.
- `docs/07_implementation_status.md` — read tail (where new tasks land); delegate any other-section read.
- `docs/08_coordinate_system.md` — read constants table.
- `docs/specs/orca-paint-segmentation-parity.md` — read frontmatter / Status section only.
- `CONTEXT.md` — read glossary entries.

## Doc Impact Statement

This packet's entire purpose IS doc updates. The list above IS the doc impact.

- `docs/01_system_architecture.md` §"PrePass" — rewritten — `rg -q 'mesh_segmentation → mesh_analysis' docs/01_system_architecture.md`.
- `docs/02_ir_schemas.md` §"SliceIR", §"RegionMapIR", §"RegionKey", §"SlicedRegion", §"ConfigId" — added / updated — `rg -q 'ConfigId' docs/02_ir_schemas.md`.
- `docs/03_wit_and_manifest.md` §"[[region_split]]" — added — `rg -q '\[\[region_split\]\]' docs/03_wit_and_manifest.md`.
- `docs/04_host_scheduler.md` §"PrePass Stage Prerequisites" — updated; §"Module Dispatch" gains host-filtered subsection — `rg -q 'host-filtered' docs/04_host_scheduler.md`.
- `docs/07_implementation_status.md` — TASK-239 through TASK-249 implemented entries — `rg -q 'TASK-249' docs/07_implementation_status.md`.
- `docs/08_coordinate_system.md` — constants conversion table — `rg -q '100 nm' docs/08_coordinate_system.md`.
- `docs/specs/orca-paint-segmentation-parity.md` — Status flipped — `rg -q '^Status: implemented' docs/specs/orca-paint-segmentation-parity.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent.

Files to inspect for this packet:

- None directly. OrcaSlicer parity content already documented in the doc edits' source material (`docs/specs/orca-paint-segmentation-parity.md`).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
