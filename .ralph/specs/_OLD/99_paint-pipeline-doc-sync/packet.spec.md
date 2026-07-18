---
status: implemented
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

## Deviations

### DEV-1 — AC-N1 / AC-N2 / AC-N3 strict greps cannot pass without destroying historical records (runtime-discovered packet-authoring defect)

**Discovered during:** Step 8 (per-AC grep verification) and Step 8 follow-up (live-stale purge) of the swarm run that executed this packet.

**What:** The negative AC commands

```
! rg -q 'boundary_paint' docs/
! rg -q 'commit_paint_regions|point_in_paint_region|fn paint_regions\(' docs/
! rg -q 'core-modules/mesh-segmentation|modules/core-modules/mesh-segmentation' docs/
```

are too broad. They trigger on every match in `docs/`, including intentional historical records of what was renamed or deleted. The packet's intent is "no live references to deleted types/functions in current docs"; the grep pattern enforces "no mention of the term anywhere in `docs/`" — a stronger claim that conflicts with the packet's own design.md which preserves the 1021-line parity spec as a historical record.

**Evidence after live-stale purge (Step 8 follow-up):** the 5 live doc files (docs/01_system_architecture.md, docs/03_wit_and_manifest.md, docs/04_host_scheduler.md, docs/05_module_sdk.md, docs/10_scenario_traces.md) contain **zero** live references to the deleted types/functions/fields. All 6 originally-flagged live-stale references plus ~10 additional related references the worker reasonably extended to cover (PaintRegionIR read-only inputs, boundary_paint propagation, SlicedRegion.boundary_paint field descriptions, etc.) were rephrased to use the post-P91 vocabulary (segment_annotations, PaintRegionLayerView, SliceIR/RegionMapIR).

The residual matches that still trigger the strict greps are all in **intentional historical records**:

- `docs/02_ir_schemas.md` lines 785, 811, 954 — schema version notes that document "renamed from `boundary_paint` in packet 91".
- `docs/05_module_sdk.md:486` — WIT SDK accessor `region.boundary_paint()` (a WIT resource method name on `slice-region-view`, not the deleted IR field; comment added to clarify it maps to `segment_annotations` on the blackboard side). Renaming the WIT method is out of scope for this packet.
- `docs/07_implementation_status.md:198, 224, 227` — historical log entries recording what was renamed/deleted (TASK-200e Chunk 5, P97 deletion log, etc.).
- `docs/specs/default-builder-migration.md` (lines 562, 782, 1074, 1207, 1362, 1367) — migration spec.
- `docs/specs/orca-paint-segmentation-parity.md` (lines 9, 34, 38, 70, 902, 908) — 1021-line algorithmic blueprint preserved per design.md ("don't delete; the 1021-line spec stays as the algorithmic blueprint reference").
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` (lines 83, 271, 306, 485, 619, 663, 745, 759, 838, 873) — the roadmap itself, intentionally preserved.
- `docs/specs/support-modules-orca-port.md:66` — historical support-module port context.
- `docs/adr/0012-spatial-indexing-as-reconstruction-only-companions.md` (lines 9, 28) — historical ADR context.

**Resolution:** The packet's strict negative ACs are accepted as **LIVE-DOC-PASS / STRICT-FAIL** — they pass on the live doc set (docs/01..08) and fail on `docs/` as a whole purely due to intentional historical records. Future packet authors writing similar retro-sync packets should narrow the AC scope to live docs only (e.g. `rg -q '...' docs/0[1-8]_*.md docs/CONTEXT.md`) or use a different gate (e.g. "no live references in sections other than explicit Historical / Migration / Closure-log sections").

**Status impact:** Packet status remains `draft` (user chose `implement` mode without explicit finalization). Implementation is functionally complete; the deviation is a packet-authoring observation, not a regression.

**Confirmed by user:** Yes, via the swarm close-out question on 2026-06-15.

### DEV-4 — Prepass-order count interpretation (9 vs 6 stages) + heading-level sub-finding

**Discovered during:** Spec-audit re-verification of AC-1 "Given" clause.

**What:** Packet's AC-1 "Given" specified a 9-stage sequence including `user-early`, `user-late`, `slice`, and `shell_classification`. The post-audit implementation resolves this as 6 `PrePass::*` enum variants (MeshSegmentation retired, MeshAnalysis, LayerPlanning, RegionMapping, PaintSegmentation, SupportGeometry) plus 2 host-callable Layer stages (`host:slice`, `host:shell_classification`) that bracket `PaintSegmentation` in the broader pipeline.

**Resolution:** docs/01 line 75 now reads "The six prepass stages execute in this order:" and a new follow-up paragraph (immediately after the code block) explains that `host:slice` and `host:shell_classification` are Layer-stage host calls (not `PrePass::*` enum variants) that run between `MeshAnalysis` (stage 2) and `PaintSegmentation` (stage 5). The `PrePass::*` enum is the type-system boundary in the production scheduler; the packet's "Given" wording collapsed PrePass stages and host-callable Layer stages into a single 9-stage list, which is a category error. The packet's authoritative resolution of the "Given" clause is the 6-PrePass + 2-host-callable-Layer split, recorded here.

**Heading-level sub-finding:** The "Variant-Chain Region Splitting" sub-section in docs/01 uses `####` (4 hashes) at line 191 instead of the packet's design.md-specified `###` (3 hashes). Trivial; the section is still locatable and the content is correct. Recorded for completeness only.

### META-1 — AC-10.1's grep gate was too narrow to catch cross-doc inconsistency (packet-text defect)

**Discovered during:** Spec-audit diff review of docs/04_host_scheduler.md.

**What:** AC-10.1's "Then" clause specifies "`PrePass::MeshSegmentation` ... produces MeshIR via `replace_mesh`" for a stage that was retired by P94r (host kernel removed) + P97 (WASM guest removed). The grep gate `rg -q 'PrePass::MeshSegmentation.*replace_mesh' docs/04_host_scheduler.md` only checks for the literal string pattern on the same line — it does not validate the output name nor check that the stage is consistent with the rest of the doc set.

The initial implementation satisfied the gate by writing a row that contradicted docs/01 (the stage is retired) and misassigned `SurfaceClassificationIR` (which is the output of `MeshAnalysis`, not `MeshSegmentation`). The post-audit fix deleted the row entirely (since the stage is retired), so AC-10.1's grep gate now FAILs by design — the gate is **INTENT-MET, not satisfied**.

**Lesson for future retro-sync packets:**

1. Phrase retired-stage rows with no output claim (the `PrePass::MeshSegmentation` row should have been written as "(retired — see docs/01)" or omitted entirely, rather than fabricating a `replace_mesh` output).
2. AC grep gates for retro-sync packets should include cross-doc consistency checks. Example gate that would have caught this defect: `rg 'PrePass::MeshSegmentation' docs/ should return only matches consistent with 'retired' framing; any 'produces ...' sentence on a retired-stage row should fail the gate`.
3. AC-10.1's "Then" clause for the MeshSegmentation row is itself a packet-text defect (it claims the stage produces MeshIR, but the stage is retired). The grep gate inherited the defect.

**Resolution:** The packet is implemented; AC-10.1's grep gate is INTENT-MET (the row no longer exists because the stage is retired). The packet-text defect is recorded here for the next packet-authoring pass.

**Confirmed by user:** Yes, via the audit close-out on 2026-06-15.
