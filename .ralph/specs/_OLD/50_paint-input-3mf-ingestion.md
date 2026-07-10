---
status: implemented
packet: 50_paint-input-3mf-ingestion
task_ids:
  - TASK-180
---

# 50_paint-input-3mf-ingestion

## Goal

Close DEV-044 by giving `PrePass::PaintSegmentation` a user-reachable input surface on the live binary path. Today `crates/slicer-host/src/model_loader.rs:150` unconditionally sets `ObjectMesh::paint_data: None` and `parse_3mf_model_xml` discards every Bambu/Orca paint metadata attribute, so a stage whose WIT/IR contract is green (DEV-025 closed 2026-05-08) operates on `None` for every production run. This packet extends `parse_3mf_model_xml` to honor all four OrcaSlicer paint attributes (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`) from the 3MF model XML, produces a `FacetPaintData` on the host loader output, and commits a painted-Benchy 3MF fixture so the end-to-end claim is falsifiable via the failing TDD-RED tests already committed at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (2026-05-10).

Scope covers all four OrcaSlicer/BambuStudio per-triangle paint channel attributes (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`) with whole-facet decoding only (no TriangleSelector subdivision). The `paint_fuzzy_skin` channel maps to `PaintSemantic::FuzzySkin`; `paint_supports` maps to `SupportEnforcer`/`SupportBlocker`; `paint_seam` maps to `Custom("seam_enforcer")`/`Custom("seam_blocker")`; `paint_color` maps to `Material` with `ToolIndex(N)` values. TriangleSelector subdivision decoding (hex strings > 2 chars or split bits ≠ 0) is deferred to a follow-up packet.

## Problem Statement

DEV-044 (registered 2026-05-10, see `docs/DEVIATION_LOG.md`) records that `PrePass::PaintSegmentation` is contract-green at the WIT/IR layer (DEV-025 closed 2026-05-08) but has NO user-reachable input surface on the live binary path. Two coupled failures combine to make the implementation unfalsifiable end-to-end:

1. **Loader discards paint metadata.** `crates/slicer-host/src/model_loader.rs:280-352` (`parse_3mf_model_xml`) parses only `<vertex>` and `<triangle>` XML elements; every Bambu/Orca paint metadata attribute (the OrcaSlicer `FacetsAnnotation` channels: `supported_facets`, `seam_facets`, `mmu_segmentation_facets`, `fuzzy_skin_facets`) is silently dropped at parse time. Line 150 unconditionally sets `ObjectMesh::paint_data: None` after parsing.

2. **No CLI surface accepts paint input.** Neither the production `slicer-host` binary (`crates/slicer-host/src/cli.rs:18-43`) nor the developer `slicer-cli` (`cli/slicer-cli/src/main.rs:18-60`) exposes a paint flag. The only documented paint-input vector per `docs/01_system_architecture.md:78` and `docs/02_ir_schemas.md:64` is the host loader producing `FacetPaintData` — which it does not do.

Downstream of `paint_data`, the pipeline is correctly wired (`paint_segmentation.rs:70-130`, `wit_host.rs:2498/2653`, layer-world `paint-region-layer-view` at `wit/deps/ir-types.wit:194-218`), but every code path along it operates on always-`None` input on the live binary path.

This packet closes the loader-side gap for **all four OrcaSlicer per-triangle paint channels** (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`), commits a fuzzy-skin-paint binary fixture needed to verify the end-to-end painted slice, and documents the decode contract in `docs/02_ir_schemas.md`. The supports/seam/color positive tests are exercised via synthetic in-test XML buffers; a four-color binary fixture (`benchy_4color.3mf`) is reserved for follow-up Packet 50b. Full TriangleSelector subdivision decoding (hex strings > 2 chars or split bits ≠ 0) is explicitly deferred to a later follow-up packet.

**Scope expansion note (2026-05-12):** This packet was originally scoped to `paint_fuzzy_skin` only; mid-implementation the scope was intentionally widened to all four channels because the per-triangle attribute decoder is channel-agnostic and absorbing the other three at once costs little extra LOC. `packet.spec.md` and this file (along with `design.md`, `implementation-plan.md`, `task-map.md`) were resynced to that scope after the implementation landed.

## Architecture Constraints (Locked Assumptions)

1. **Loader-only change.** No PaintSegmentation, RegionMap, host-validator, harvest, or WIT change. Their contracts are correct; the bug is upstream.
2. **`FacetPaintData` schema is unchanged.** This packet populates it; does not modify its shape.
3. **Whole-facet only.** The per-triangle attribute format guarantees whole-facet granularity implicitly (one attribute per triangle). Subdivision support is a follow-up packet.
4. **All four OrcaSlicer per-triangle paint channels** are wired in this packet: `paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`. The decoder uses a single shared `decode_paint_hex_state` for all four; per-attribute branches build separate `PaintLayer`s with channel-appropriate (`PaintSemantic`, `PaintValue`) mappings (see table above). Channel coverage was widened mid-packet from the original `paint_fuzzy_skin`-only scope; the binary fixture `benchy_painted.3mf` still carries fuzzy_skin only, with synthetic in-test XML covering the other three channels. A multi-channel binary fixture is reserved for Packet 50b.
5. **No CLI changes.** Paint enters exclusively via the 3MF model file.
6. **No paint-data warning on empty.** A 3MF with no paint metadata returns `paint_data: None` silently — identical to today's behavior for `resources/benchy.stl`.
7. **Negative-test specificity.** Malformed metadata must fail loudly. Silent partial parsing is forbidden.
8. **The pre-committed failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` must turn GREEN at packet close.** Their assertion text must NOT be weakened to accommodate implementation shortcuts. The line-106 docstring guidance is updated as part of this packet from `Custom("fuzzy_skin")` to `FuzzySkin` (strengthens fidelity; allowed under this constraint because it does not weaken any `assert!`).
9. **Use `PaintSemantic::FuzzySkin`, NOT `PaintSemantic::Custom("fuzzy_skin")`.** The IR exposes a first-class `FuzzySkin` variant (`crates/slicer-ir/src/slice_ir.rs:178`). Emitting `Custom("fuzzy_skin")` instead changes runtime behavior in four call sites: `paint_segmentation.rs:122` (triggers spurious `detect_custom_conflict`), `slice_postprocess.rs:369` (different fallback `Scalar(0.0)` vs `Flag(false)`), `layer_executor.rs:266` (different sort priority bucket), and `wit_host.rs:2319` (silently coerces to `ToolIndex(0)` on WIT view round-trip). The host's WIT-side parser at `dispatch.rs:1975` already maps the string `"fuzzy_skin"` → `PaintSemantic::FuzzySkin`; the loader must be consistent with that data path.

## Data and Contract Notes

- **`FacetPaintData` shape is pinned**:
  - `Some(FacetPaintData { layers })` per painted object (one `PaintLayer` per active channel — between 1 and 5 layers possible: fuzzy_skin, support_enforcer, support_blocker, seam_enforcer, seam_blocker, material); `None` per unpainted object (no warning).
  - Each `PaintLayer` has `semantic` per the channel mapping table above, `facet_values: Vec<Option<PaintValue>>` of length **exactly** `mesh.indices.len() / 3` (consumer at `paint_segmentation.rs:93-100` returns `MalformedFacetValues` on length mismatch), `strokes: Vec::new()` (consumer never reads `.strokes`).
  - `facet_values[i]`: `None` for unpainted state in that channel; `Some(PaintValue::Flag(true))` for fuzzy_skin/supports/seam painted states; `Some(PaintValue::ToolIndex(N))` for `paint_color` painted states. The arm choices are dictated by `slice_postprocess.rs:366-374 default_fallback_value` unpainted defaults (`FuzzySkin/Support* → Flag(false)`, `Material → ToolIndex(0)`); painted complements match those arms.
- The 3MF encoding uses **per-triangle hex attributes** (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`), not the PrusaSlicer-style positional hex bitstream. The fixture was produced with **OrcaSlicer** (a BambuSlicer fork) and emits the BambuStudio XML convention:
  - Painted facets: `<triangle ... paint_fuzzy_skin="4" />`, `<triangle ... paint_supports="4" />`, etc. (1- or 2-nibble hex).
  - Unpainted facets: attribute omitted entirely.
  - **Hex state decoder (shared across channels):** A 1-nibble hex string `N` encodes state `N >> 2`; the low 2 bits are the TriangleSelector split bits and MUST be zero (otherwise = subdivision = rejected). A 2-nibble hex string `AB` encodes state `A + 3`, with the second nibble required to be either `0xC` (continuation marker) or to have zero split bits. Hex strings > 2 chars indicate full subdivision and are rejected.
  - Per-channel state validation:
    - `paint_fuzzy_skin`: state ∈ {0, 1}; state > 1 → `PaintMetadata` error.
    - `paint_supports`: state ∈ {0, 1, 2} (0=none, 1=enforcer, 2=blocker); state > 2 → `PaintMetadata` error.
    - `paint_seam`: state ∈ {0, 1, 2} (0=none, 1=enforcer, 2=blocker); state > 2 → `PaintMetadata` error.
    - `paint_color`: any state ≥ 0; nonzero state N → `ToolIndex(N)`.
  - Any malformed value (non-hex digit, invalid state for the channel, or subdivision hex) raises `ModelLoadError::PaintMetadata { reason, byte_offset }` where `byte_offset` is the XML stream offset for diagnostic purposes.
  - The "whole-facet only" guarantee comes from rejecting subdivision in `decode_paint_hex_state`; the per-triangle attribute itself is always whole-facet.

## Risks and Tradeoffs

- **Risk: 3MF attribute name discovery.** The exact attribute name was not in OrcaSlicerDocumented/ and required inspecting the emitted fixture XML. Resolved: `paint_fuzzy_skin` (unprefixed).
- **Risk: `FacetPaintData::layers` shape mismatch.** If the IR shape expects per-Z-layer paint information (not per-triangle), the loader must produce a "single virtual layer" or "no layer; populate facets-list-only" path. Step 1 grounds this. If the IR shape change is needed, this packet escalates — IR changes are explicitly out of scope.
- **Tradeoff: whole-facet only.** The per-triangle attribute format (`paint_fuzzy_skin`) inherently represents whole-facet paint; partial-triangle paint would require a different encoding (subdivision bitstream) which this packet does not support.
- **Tradeoff: docs/02 schema doc.** The new subsection is documentation-only; it does not bump `FacetPaintData::schema_version` because no IR shape change.

## Locked Assumptions and Invariants

The implementation must preserve these invariants. If any one is violated, the change is rejected.

1. `crates/slicer-host/src/paint_segmentation.rs`, `region_mapping.rs`, `dispatch.rs`, `wit_host.rs` are unchanged after this packet.
2. `crates/slicer-ir/src/slice_ir.rs::FacetPaintData` shape is unchanged (loader populates the existing shape).
3. No WIT files change.
4. No CLI flag changes on either `slicer-host` or `slicer-cli`.
5. The pre-committed failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (RED 2026-05-10) turn GREEN at packet close WITHOUT their assertions being weakened.
6. No existing passing test is weakened (no assertion removed; no `#[ignore]` added; no `assert!` → `eprintln!` regression).
7. Test discipline: targeted `cargo test -p slicer-host --test <file>` only; never `cargo test --workspace`.
8. STL paint-sidecar JSON ingestion remains explicitly out-of-scope (YAGNI per user direction 2026-05-10).
