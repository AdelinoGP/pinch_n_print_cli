# Requirements: 50_paint-input-3mf-ingestion

## Packet Metadata

- Slug: `50_paint-input-3mf-ingestion`
- Status: `draft`
- Task IDs: `TASK-180`
- Backlog source: `docs/07_implementation_status.md`

## Problem Statement

DEV-044 (registered 2026-05-10, see `docs/DEVIATION_LOG.md`) records that `PrePass::PaintSegmentation` is contract-green at the WIT/IR layer (DEV-025 closed 2026-05-08) but has NO user-reachable input surface on the live binary path. Two coupled failures combine to make the implementation unfalsifiable end-to-end:

1. **Loader discards paint metadata.** `crates/slicer-host/src/model_loader.rs:280-352` (`parse_3mf_model_xml`) parses only `<vertex>` and `<triangle>` XML elements; every Bambu/Orca paint metadata attribute (the OrcaSlicer `FacetsAnnotation` channels: `supported_facets`, `seam_facets`, `mmu_segmentation_facets`, `fuzzy_skin_facets`) is silently dropped at parse time. Line 150 unconditionally sets `ObjectMesh::paint_data: None` after parsing.

2. **No CLI surface accepts paint input.** Neither the production `slicer-host` binary (`crates/slicer-host/src/cli.rs:18-43`) nor the developer `slicer-cli` (`cli/slicer-cli/src/main.rs:18-60`) exposes a paint flag. The only documented paint-input vector per `docs/01_system_architecture.md:78` and `docs/02_ir_schemas.md:64` is the host loader producing `FacetPaintData` — which it does not do.

Downstream of `paint_data`, the pipeline is correctly wired (`paint_segmentation.rs:70-130`, `wit_host.rs:2498/2653`, layer-world `paint-region-layer-view` at `wit/deps/ir-types.wit:194-218`), but every code path along it operates on always-`None` input on the live binary path.

This packet closes the loader-side gap for one channel (`fuzzy_skin_facets`), commits the fixture needed to verify it end-to-end, and documents the decode contract in `docs/02_ir_schemas.md`. The other three paint channels and full TriangleSelector subdivision decoding are explicitly deferred to follow-up packets.

## Task Mapping

- **TASK-180** (new — to be added to `docs/07_implementation_status.md` at Step 7):
  *"Wire 3MF `fuzzy_skin_facets` paint metadata through the host loader so PaintSegmentation has a user-reachable input on the live binary path. Covers DEV-044."*
  → Closes when AC-1 through AC-8 (plus both negative tests) are all green.

## In Scope

- `crates/slicer-host/src/model_loader.rs`:
  - Extend `parse_3mf_model_xml` to detect and decode the `fuzzy_skin_facets` (or equivalent — see Q3 below) attribute on `<triangle>` elements (or whatever 3MF host element Step 1 grounding determines is correct).
  - Decode whole-facet TriangleSelector bitstreams (state-0 = unpainted, state-1 = painted; reject any nibble pair indicating recursive subdivision with a typed error).
  - Replace the line-150 `paint_data: None` literal with the loader's discovered value.
  - Add `ModelLoadError::PaintMetadata { reason: String, byte_offset: usize }` variant.
- `resources/benchy_painted.3mf` — committed binary fixture. Smokestack triangles painted with the `fuzzy_skin_facets` channel. Authoring procedure documented in companion README.
- `resources/benchy_painted.README.md` — authoring tool, steps, expected attribute names. Enables future regeneration.
- `docs/02_ir_schemas.md` — new "3MF paint-metadata extraction" subsection under FacetPaintData provenance.
- `docs/07_implementation_status.md` — add TASK-180; close it at packet close.
- `docs/DEVIATION_LOG.md` — flip DEV-044 to `Closed — Packet 50, 2026-MM-DD`.
- `docs/14_deviation_audit_history.md` — chronology entry recording DEV-044 closure.

## Out of Scope

- The other three OrcaSlicer paint channels: `supported_facets`, `seam_facets`, `mmu_segmentation_facets`. Each becomes its own packet.
- Full TriangleSelector recursive subdivision decoding. This packet rejects any subdivided facet with a typed error.
- Any CLI flag additions. Paint enters exclusively via the 3MF.
- Any change to PaintSegmentation, RegionMap, host validators, harvest code, WIT files, or the macros crate.
- Any change to `crates/slicer-ir/src/slice_ir.rs::FacetPaintData` shape (this packet only populates it).
- STL paint-sidecar ingestion (YAGNI per user direction 2026-05-10).
- 3MF write/export support.
- Multi-extruder paint→tool_index resolution.

## Authoritative Docs

- `docs/01_system_architecture.md:78` — single-line citation marking PrePass `MeshIR` input source (loaded STL/3MF/OBJ); the only load-related mention in docs/01.
- `docs/02_ir_schemas.md:82-99` — `ObjectMesh::paint_data` field + `FacetPaintData`/`PaintLayer` struct definitions; load directly (≤ 20 lines). Note: `:135` (`PaintStroke.triangles`) is not part of FacetPaintData's shape; do not cite it for that purpose.
- `docs/07_implementation_status.md` — delegate ALL reads/edits (large file).
- `docs/DEVIATION_LOG.md` — delegate SNIPPET fetch for DEV-044 row.
- `docs/14_deviation_audit_history.md` — delegate SNIPPET fetch for the chronology section.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/generated_documentation/04_refactoring_hazards.md` — H524, H1105: bitstream format and re-indexing hazards.
- `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — state-index semantics.
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:516` — TriangleSelector + ModelVolume painted-state ownership.
- **External documentation gap:** the literal 3MF XML attribute name and `xmlns:` URI are NOT in `OrcaSlicerDocumented/`. Implementer must dispatch a Step-1 documentation search against the 3MF Consortium core spec (https://github.com/3MFConsortium/spec_core) and/or the PrusaSlicer `Slic3r_PE_namespace` extension documentation. Do NOT read OrcaSlicer C++ source directly.

## Acceptance Summary

The packet is complete when:

1. `parse_3mf_model_xml` decodes `fuzzy_skin_facets` whole-facet paint into a populated `FacetPaintData`; `model_loader.rs:150` no longer hardcodes `paint_data: None`.
2. `resources/benchy_painted.3mf` is committed and parseable; `resources/benchy_painted.README.md` documents reproduction.
3. `painted_benchy_3mf_reaches_paint_segmentation` (RED today) goes GREEN: painted-Benchy GCode is observably different from unpainted-Benchy GCode after normalization.
4. `painted_3mf_fixture_is_committed` (RED today) goes GREEN.
5. `benchy_e2e_real_pipeline_produces_gcode` stays GREEN (backward compat).
6. The five Packet-43-rev1 regression-defense commands all stay GREEN.
7. Two negative tests pass: malformed metadata rejected with typed `PaintMetadata` error; no-paint 3MF returns `paint_data: None` with no warning.
8. `docs/02_ir_schemas.md` documents the decode contract and explicitly lists deferred channels.
9. `cargo clippy --workspace -- -D warnings` is green.
10. DEV-044 flipped to `Closed`; TASK-180 closed in `docs/07_implementation_status.md`; chronology entry added in `docs/14`.

## Cross-Packet Dependencies

- **Depends on:** DEV-025 closure (Packet 43-rev1, 2026-05-08). The macro/host paint contract must be intact for paint to reach `PaintSegmentationOutput`.
- **Unblocks:** Packet 51 (`paint-semantic-region-overrides`). Packet 51's end-to-end test depends on this packet's `benchy_painted.3mf` fixture.

## Verification Commands

Targeted verification (use these for per-step adjudication):

- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test model_loader_tdd`
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode -- --exact`
- Packet-43-rev1 regression battery (five commands; see `packet.spec.md` AC-5).

`cargo test --workspace` is **not** required at packet close.

## Step Completion Expectations

Each implementation step in `implementation-plan.md` declares files-allowed-to-read, files-allowed-to-edit (≤ 3), expected sub-agent dispatches, context cost (S/M; never L), and a falsifying check or exit condition. Step boundaries are non-negotiable; no step may load OrcaSlicer source, generated WIT bindings, or `target/` artifacts.

## Context Discipline Notes

- Read budget: 60% (≈ 120 k). Stop reading at 60%, hand off at 85%.
- `crates/slicer-macros/src/lib.rs` is out of bounds for direct reading in this packet.
- `docs/07_implementation_status.md` and `docs/DEVIATION_LOG.md` are large; delegate all reads.
- The painted-Benchy fixture is binary; do not attempt to read it inline. Authoring is one bounded step (Step 2) with documented reproduction.
