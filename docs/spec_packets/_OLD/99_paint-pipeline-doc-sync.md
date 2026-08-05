---
status: implemented
packet: 99
task_ids: [TASK-249]
---

# 99_paint-pipeline-doc-sync

## Goal

Bring `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/07_implementation_status.md`, and `docs/08_coordinate_system.md` into line with the new paint pipeline shape that landed across packets 89-98; explicitly: rewrite the prepass-order section of `docs/01` to describe the new sequence (mesh_segmentation → mesh_analysis → user-early → region_mapping → slice → shell_classification → paint_segmentation → support_geometry → user-late), add the variant-chain region-splitting model, remove the obsolete `PrePass::MeshSegmentation [new — runs first]` block that described an unwired stage; bump `docs/02` SliceIR and RegionMapIR to 2.0.0 (per P91), document `variant_chain` on `RegionKey` and `SlicedRegion`, document `segment_annotations` (renamed from `boundary_paint`), document `ConfigId(u32)` + `configs: Vec<ResolvedConfig>` interner, REMOVE `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `PaintRegionRTreeIndex`, `MeshSegmentationIR`, `FacetPaintMark`, note the `PaintValue::Vector` deferred follow-up; add the `[[region_split]]` manifest schema section + priority registry + value-type validation rules + cross-manifest aggregation behavior to `docs/03`, REMOVE the `mesh-segmentation-output` WIT resource documentation; update `docs/04`'s stage-prerequisites table (`PrePass::MeshSegmentation` → no prerequisites + replace_mesh; `PrePass::PaintSegmentation` → SliceIR + RegionMapIR prerequisites + replace_slice_ir), document host-filtered dispatch contract, REMOVE the "guard-based fallback contract" sentence for paint-segmentation; mark paint-segmentation parity, mesh-segmentation wiring, region-splitting IR, and Phase 5 as implemented in `docs/07`, flag three follow-ups (community paint ingestion, PaintValue::Vector, host:raw_slice); add the constant-conversion table from spec §5 to `docs/08`; flip `docs/specs/orca-paint-segmentation-parity.md`'s `Status:` from `awaiting Slice Rework` to `implemented` (keep file as historical record); CONTEXT.md was already updated during planning (variant chain / painted variant / region-split semantic / segment annotation glossary; "region" ambiguity expanded) — verify the entries are present.

## Problem Statement

Packets 89-98 reshape the paint pipeline end-to-end: new IR shapes (P91), manifest schema + dispatch (P92), region-mapping cross-product (P93), mesh-segmentation host wiring (P94), paint-segmentation port (P95), Phase 5 width-limiting (P96), WASM mesh-segmentation deletion (P97), loader symmetry (P98). The implementation is now the source of truth, but `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/07_implementation_status.md`, and `docs/08_coordinate_system.md` are stale. They reference deleted types (`PaintRegionIR`, `MeshSegmentationIR`), wrong prepass orders, missing manifest schemas, and obsolete WIT resources.

The roadmap intentionally deferred doc sync to a single closing packet to avoid mid-roadmap doc churn (every intermediate state would have required a doc update; consolidating saves effort and avoids transient incorrect documentation). This packet finishes that sync.

The work is entirely doc edits — no production code changes, no fixture changes, no test changes. Verification is grep-based: each AC names a phrase to find or NOT find.

## Architecture Constraints

- No-production-code invariant: this packet does NOT edit any file under `crates/`, `modules/`, `wit/`, or `resources/`. AC-17's byte-identical g-code is the regression guard.
- Sync-not-redefine invariant: every doc edit reflects the state landed in packets 89-98. The packet does NOT introduce new design decisions; if a question arises about content shape, the source-of-truth is the implementation or the planning docs, not the doc text being edited.
- Deletion-content invariant: when removing references to deleted types (`PaintRegionIR` et al.), don't replace with placeholder prose. Delete the section / paragraph entirely.

## Data and Contract Notes

- IR contracts: documented (not changed).
- WIT boundary: documented (not changed).
- Determinism: this packet has no effect on runtime determinism.

## Locked Assumptions and Invariants

- **Behavior preservation**: AC-17 confirms.
- **Source-of-truth is the implementation**: if a doc-edit content question is ambiguous, the answer is in the code, not invented.
- **Deletions are full deletions, not redactions**: removed-type sections are erased, not replaced with placeholder prose.

## Risks and Tradeoffs

- **Risk: a doc edit inadvertently includes outdated information** about a deleted type. Mitigation: per-doc grep checks (AC-N1, AC-N2, AC-N3) catch this.
- **Risk: `docs/07_implementation_status.md` becomes corrupted** by direct loading + editing. Mitigation: delegate the edit to a sub-agent that runs a small `Edit` operation; never load the full file.
- **Tradeoff: doc-sync packet vs. doc-edits-per-packet**: deferred sync (this approach) trades transient incorrect documentation in the middle of the roadmap for a clean closing packet. The user explicitly chose this trade.
