---
status: draft
packet: 50_paint-input-3mf-ingestion
task_ids:
  - TASK-180
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 50_paint-input-3mf-ingestion

## Goal

Close DEV-044 by giving `PrePass::PaintSegmentation` a user-reachable input surface on the live binary path. Today `crates/slicer-host/src/model_loader.rs:150` unconditionally sets `ObjectMesh::paint_data: None` and `parse_3mf_model_xml` discards every Bambu/Orca paint metadata attribute, so a stage whose WIT/IR contract is green (DEV-025 closed 2026-05-08) operates on `None` for every production run. This packet extends `parse_3mf_model_xml` to honor the `fuzzy_skin_facets` paint channel from the 3MF model XML, produces a `FacetPaintData` on the host loader output, and commits a painted-Benchy 3MF fixture so the end-to-end claim is falsifiable via the failing TDD-RED tests already committed at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (2026-05-10).

Scope is intentionally narrow: only the `fuzzy_skin_facets` channel, and only the unsubdivided/whole-facet variant of the TriangleSelector bitstream (state-0-or-state-1 per facet — no recursive subdivision). The other three OrcaSlicer paint channels (`supported_facets`, `seam_facets`, `mmu_segmentation_facets`) and full TriangleSelector subdivision decoding are deferred to follow-up packets. The fuzzy_skin channel was chosen because (a) it maps 1:1 to the IR's first-class `PaintSemantic::FuzzySkin` variant with no MMU/material conflation, (b) it is the most useful semantic for a visual-difference test against an unpainted Benchy, and (c) it composes cleanly with the existing failing test fixture at `benchy_painted_e2e_tdd.rs::painted_benchy_3mf_reaches_paint_segmentation`.

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/model_loader.rs` — extend `parse_3mf_model_xml` to recognize the `fuzzy_skin_facets` attribute on `<triangle>` (or equivalent element discovered during Step 1 grounding); decode the unsubdivided 4-bit-nibble bitstream variant; populate `ObjectMesh::paint_data: Some(FacetPaintData)` with exactly one `PaintLayer { semantic: PaintSemantic::FuzzySkin, facet_values: <Vec of length facet_count, None for unpainted, Some(PaintValue::Flag(true)) for painted>, strokes: Vec::new() }`. `model_loader.rs:150` `paint_data: None` literal is replaced with the loader's discovered value.
  - `resources/benchy_painted.3mf` — commit a reproducible painted-Benchy 3MF fixture. Paint cluster: smokestack triangles (Z roughly [50mm, 72mm]). Paint channel: `fuzzy_skin_facets` only. Whole-facet paint only (no subdivision).
  - `resources/benchy_painted.README.md` — a one-page authoring procedure (which tool / script, exact steps, expected attribute names) so a future packet can regenerate the fixture deterministically.
  - `docs/02_ir_schemas.md` — add a "3MF paint-metadata extraction" subsection under FacetPaintData provenance documenting the recognized 3MF attribute and the whole-facet decode contract; explicitly list the deferred channels and the deferred subdivision support.
  - `docs/07_implementation_status.md` — add TASK-180 row; flip to `[x]` at packet close.
  - `docs/DEVIATION_LOG.md` — flip DEV-044 row from `Open` to `Closed — Packet 50, 2026-MM-DD`.
  - `docs/14_deviation_audit_history.md` — append a 2026-MM-DD chronology entry recording closure.

- Out of scope:
  - The other three OrcaSlicer paint channels: `supported_facets`, `seam_facets`, `mmu_segmentation_facets`. Each gets its own follow-up packet.
  - Full TriangleSelector subdivision decoding (recursive 4-bit-nibble tree). Whole-facet only in this packet.
  - Any CLI flag additions on `slicer-host` or `slicer-cli`. Paint data enters exclusively via the `.3mf` model file.
  - Any change to `crates/slicer-host/src/paint_segmentation.rs`, `region_mapping.rs`, `dispatch.rs`, `wit_host.rs`, or any host validator/harvest code.
  - Any change to WIT files under `wit/`.
  - Any change to `crates/slicer-macros/src/lib.rs`.
  - Any change to `crates/slicer-ir/src/slice_ir.rs::FacetPaintData` shape. This packet only populates it.
  - Any STL paint-sidecar JSON ingestion. Rejected as YAGNI per user direction 2026-05-10.
  - 3MF write/export support.
  - Multi-extruder paint→tool_index resolution. The `fuzzy_skin_facets` channel does not interact with tool_index.

## Prerequisites and Blockers

- Depends on:
  - DEV-025 closure (already complete; Packet 43-rev1, 2026-05-08).
  - Failing TDD-RED tests already committed at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (2026-05-10).
- Unblocks:
  - DEV-045 closure (Packet 51 `paint-semantic-region-overrides`) — end-to-end testability requires this packet's `benchy_painted.3mf` fixture.
- Activation blockers (must be resolved before flipping `status: draft` → `active`):
  - Q1: confirm `fuzzy_skin_facets` is the chosen channel for the fixture; reject alternative-channel proposals.
  - Q2: confirm the authoring tool / regeneration procedure for `benchy_painted.3mf` (recommend: an OrcaSlicer or PrusaSlicer GUI session with documented steps, exported as 3MF, committed binary; OR a deterministic Python script that emits the 3MF XML directly).
  - Q3: confirm the exact 3MF attribute name and namespace URI to be parsed (e.g., `slic3rpe:fuzzy_skin_facets="..."` or `paint_color="..."` — OrcaSlicerDocumented/ does NOT disclose this; the implementer must dispatch a documentation search against the 3MF Consortium core spec or PrusaSlicer `Slic3r_PE_namespace` documentation in Step 1).
  - Q4: confirm the error variant shape for malformed paint metadata: `ModelLoadError::PaintMetadata { reason: String, byte_offset: usize }` (the byte_offset, not a triangle_index, because the bitstream is positional and a triangle index is not always available mid-decode).

## Acceptance Criteria

- **Given** a 3MF whose model XML contains a `fuzzy_skin_facets` attribute on at least one `<triangle>` (or equivalent host element), **when** `load_model` is called on that 3MF, **then** the returned `ObjectMesh::paint_data` is `Some(FacetPaintData { layers, .. })` with `layers.len() == 1`, the layer's `semantic == PaintSemantic::FuzzySkin`, `layer.facet_values.len() == mesh.indices.len() / 3`, and at least one entry equals `Some(PaintValue::Flag(true))`. | `cargo test -p slicer-host --test model_loader_tdd load_3mf_extracts_fuzzy_skin_facets -- --exact --nocapture`
- **Given** the fixture must exist for E2E tests, **when** Step 2 commits the fixture, **then** `resources/benchy_painted.3mf` exists and is parseable. | `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_3mf_fixture_is_committed -- --exact --nocapture`
- **Given** paint data reaches the live pipeline, **when** the painted Benchy is sliced via the production `slicer-host` binary against the same module set as the unpainted Benchy, **then** the painted GCode after normalization differs byte-wise from the unpainted GCode (paint must have an observable effect). | `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation -- --exact --nocapture`
- **Given** the existing unpainted-Benchy capstone test must stay green, **when** Step 6 runs, **then** `benchy_e2e_real_pipeline_produces_gcode` passes. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode -- --exact --nocapture`
- **Given** the Packet-43-rev1 macro-arm proof must remain green, **when** Step 6 runs, **then** the five regression-defense commands all pass. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd && cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd && cargo test -p slicer-host --test dispatch_tdd macro_path && cargo test -p slicer-host --test macro_all_worlds_roundtrip_tdd prepass && cargo test -p slicer-host --test guest_fixture_freshness_tdd`
- **Given** the IR-schema doc records FacetPaintData provenance, **when** Step 5 edits docs/02, **then** docs/02_ir_schemas.md contains a "3MF paint-metadata extraction" subsection naming the supported channel (`fuzzy_skin_facets`) and explicitly listing deferred channels. | `rg -q '3MF paint-metadata extraction' docs/02_ir_schemas.md && rg -q 'fuzzy_skin_facets' docs/02_ir_schemas.md && rg -q 'deferred:.*supported_facets|seam_facets|mmu_segmentation_facets' docs/02_ir_schemas.md`
- **Given** clippy is the lint gate, **when** Step 7 runs, **then** `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy --workspace -- -D warnings`
- **Given** DEV-044 is closed, **when** Step 7 edits the deviation registry, **then** docs/DEVIATION_LOG.md DEV-044 row shows `Closed` and docs/07_implementation_status.md shows `[x] TASK-180`. | `rg -q '^\| DEV-044.*Closed' docs/DEVIATION_LOG.md && rg -q '\[x\] TASK-180' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** a 3MF whose `fuzzy_skin_facets` attribute is malformed (e.g., non-hex characters, odd nibble count, bitstream length does not match the geometry section's `<triangle>` count, or a nibble in `{2..15}` indicating recursive subdivision), **when** `load_model` is called, **then** the loader returns `Err(ModelLoadError::PaintMetadata { reason, byte_offset })` rather than silently producing partial paint data; the diagnostic names the byte_offset into the bitstream where the decode failed. | `cargo test -p slicer-host --test model_loader_tdd load_3mf_malformed_fuzzy_skin_rejects -- --exact --nocapture`
- **Given** a 3MF whose model XML has NO paint metadata at all, **when** `load_model` is called, **then** the returned `ObjectMesh::paint_data` is `None` and no warning is emitted (the no-paint path remains the default, identical to today's behavior for `resources/benchy.stl` after this packet lands). | `cargo test -p slicer-host --test model_loader_tdd load_3mf_without_paint_returns_none -- --exact --nocapture`

## Verification

- `cargo build --workspace` — must pass after every edit step.
- `cargo clippy --workspace -- -D warnings` — must pass at the packet completion gate.
- `cargo test -p slicer-host --test model_loader_tdd` — full file (gains three new tests for the paint-extraction paths above).
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — full file (2 tests; both must turn GREEN at packet close).
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode` — backward-compat regression.
- The five Packet-43-rev1 regression-defense commands listed above.
- **No `cargo test --workspace` is required for this packet** — no contract, validator, scheduler, or IR-schema change. The targeted suite covers the surface.

## Authoritative Docs

- `docs/01_system_architecture.md:78` — single-line citation marking PrePass `MeshIR` input source (loaded STL/3MF/OBJ); the only load-related mention in docs/01.
- `docs/02_ir_schemas.md:82-99` — `ObjectMesh::paint_data` field + `FacetPaintData`/`PaintLayer` struct definitions; load directly (≤ 20 lines). The packet adds a "3MF paint-metadata extraction" subsection in this neighborhood. Note: `:135` (`PaintStroke.triangles`) is not part of FacetPaintData's shape; do not cite it for that purpose.
- `docs/07_implementation_status.md` — delegate ALL reads (file is large); only edits at Step 7 via worker dispatch (close TASK-180).
- `docs/DEVIATION_LOG.md` — delegate SNIPPET fetch for DEV-044 row before editing.
- `docs/14_deviation_audit_history.md` — delegate SNIPPET fetch for the 2026-05-10 chronology entry.

## OrcaSlicer Reference Obligations

- The TriangleSelector hex-nibble bitstream format and the four paint-channel surfaces (`supported_facets`, `seam_facets`, `mmu_segmentation_facets`, `fuzzy_skin_facets`) are documented at:
  - `OrcaSlicerDocumented/generated_documentation/04_refactoring_hazards.md` — H524, H1105 (bitstream format, no version tag, re-indexing hazards).
  - `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — state-index semantics (0=unpainted, 1..N=enforcer/blocker enum values).
  - `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:516` — `TriangleSelector` and `ModelVolume` painted-state ownership.
- **Authoritative gap:** the exact 3MF XML attribute name (`slic3rpe:fuzzy_skin_facets` or equivalent) and the `xmlns:` URI for the Slic3rPE/BBS extension are NOT documented in `OrcaSlicerDocumented/`. Step 1 grounding MUST dispatch a documentation search against the 3MF Consortium core spec or the PrusaSlicer `Slic3r_PE_namespace` documentation to determine the literal attribute name. Do NOT read OrcaSlicer source directly.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

- `crates/slicer-macros/src/lib.rs` is OUT OF BOUNDS for direct reads in this packet (not touched).
- `crates/slicer-host/src/model_loader.rs` is the primary edit surface; direct reads permitted with line-range hints (≤ 600 lines expected).
- Authoritative docs > 300 lines must be delegated for SNIPPET/FACT reads (specifically `docs/07_implementation_status.md` and `docs/DEVIATION_LOG.md`).
- Aggregate context cost: **M**. Step 3 (decoder implementation) is the only M-leaning step; if it actually measures L during execution, split before proceeding.
