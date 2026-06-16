---
status: implemented
packet: 50a
task_ids: [TASK-180b-prereq]
backlog_source: docs/07_implementation_status.md
phases: [phase-1-dominant-state, phase-2-stroke-geometry]
blocks: [50b_paint-input-3mf-mmu-supports]
---

# Packet 50a — TriangleSelector Subdivision Parser + Paint Stroke Geometry

## Goal

Implement a TriangleSelector hex-tree decoder in the 3MF paint parser so that
real-world painted 3MF files (including `benchy_4color.3mf`) load without error.

**Phase 1** extracts a per-facet dominant state from the tree and unblocks packet 50b.  
**Phase 2** reconstructs sub-triangle 3D geometry and populates `PaintLayer.strokes`.

Resolves the blocker documented in `.ralph/specs/50b_paint-input-3mf-mmu-supports/packet.spec.md`:
`benchy_4color.3mf` could not be loaded because long paint hex strings were rejected.

## Scope

**In scope**
- `crates/slicer-host/src/model_loader.rs` — hex tree walker, dominant-state extractor, stroke geometry decoder
- `crates/slicer-host/tests/model_loader_tdd.rs` — update one existing test, add four new tests

**Out of scope**
- IR type definitions (`crates/slicer-ir`) — `PaintLayer.strokes` already exists; no struct changes needed
- `PaintStroke.triangles` coordinate-system conversion (coordinates emitted in 3MF mm units; any unit normalization is a follow-up)
- Any change to WIT, scheduler, or host-module boundary
- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — read only via sub-agent delegation

## Prerequisites / Blockers

- Packet 50 (`50_paint-input-3mf-ingestion`) must be `status: implemented`. ✓ (closed 2026-05-11)
- All 8 paint tests in `model_loader_tdd.rs` must pass before starting Phase 1.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintLayer`, `PaintStroke`, `FacetPaintData` field paths
- `docs/08_coordinate_system.md` — coordinate unit rules (1 unit = 100 nm; 3MF verts are in mm → use `mm_to_units()` when populating `PaintStroke.triangles`)
- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — split geometry formulas (delegate; never read directly)
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:490–580` — TriangleSelector schema

---

## Acceptance Criteria

### Phase 1 — Dominant-State Tree Decoder

**AC-1** (positive — file loads)  
Given `resources/benchy_4color.3mf`, when `load_model` is called, then it returns `Ok(_)` with no `ModelLoadError::PaintMetadata`.  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_benchy_4color_loads --nocapture`

**AC-2** (positive — Material channel present)  
Given `resources/benchy_4color.3mf` loaded successfully, then the resulting IR for the first object contains a `PaintLayer` with `semantic == PaintSemantic::Material` and at least one `facet_values` entry that is `Some(PaintValue::ToolIndex(_))`.  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_benchy_4color_loads --nocapture`

**AC-3** (positive — SupportEnforcer channel present)  
Given `resources/benchy_4color.3mf` loaded successfully, then the IR contains a `PaintLayer` with `semantic == PaintSemantic::SupportEnforcer` and at least one `facet_values` entry that is `Some(PaintValue::Flag(true))`.  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_benchy_4color_loads --nocapture`

**AC-4** (positive — dominant state from synthetic subdivided tree)  
Given a synthetic 3MF with `paint_color="51C"` on triangle 0 (a 2-child tree: child 0 is leaf state 1/T0, child 1 is extended-state leaf state 4/T1; dominant = T1 because it appears more, or first-nonzero if tied), when loaded, then `facet_values[0]` in the Material layer is `Some(PaintValue::ToolIndex(_))` (any non-zero tool).  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_subdivision_dominant_state --nocapture`

**AC-5** (negative — malformed hex chars rejected)  
Given a synthetic 3MF with `paint_fuzzy_skin="GG"` (non-hex characters), when `load_model` is called, then it returns `Err(ModelLoadError::PaintMetadata { .. })` containing "invalid hex digit".  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_invalid_paint_hex_rejects --nocapture`

**AC-6** (negative — truncated tree rejected)  
Given a synthetic 3MF with `paint_fuzzy_skin="5"` (nibble 0101: split_type=1 declares 2 children but string ends), when `load_model` is called, then it returns `Err(ModelLoadError::PaintMetadata { .. })` containing "unexpected end".  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_truncated_paint_tree_rejects --nocapture`

### Phase 2 — Stroke Geometry Population

**AC-7** (positive — strokes populated)  
Given `resources/benchy_4color.3mf` loaded after Phase 2, then the Material `PaintLayer.strokes` contains at least one `PaintStroke` whose `triangles` field is non-empty.  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_benchy_4color_strokes_populated --nocapture`

**AC-8** (positive — stroke triangles are non-degenerate)  
For each `PaintStroke` in any layer from `benchy_4color.3mf`, each `[Point3; 3]` in `.triangles` has at least two distinct vertices (no zero-area triangles).  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_benchy_4color_strokes_populated --nocapture`

**AC-9** (negative — stroke geometry skipped for whole-facet triangles)  
Given a synthetic 3MF with `paint_color="4"` (whole-facet, no subdivision), when loaded, then the Material `PaintLayer.strokes` is empty (only `facet_values` is populated).  
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_wholefacet_has_no_strokes --nocapture`

---

## Supplemental Verification

```bash
# Type-check after each step (seconds)
cargo check --workspace

# Clippy required before closing
cargo clippy --workspace -- -D warnings

# Full suite only at packet closure
cargo test --workspace
```

## Cross-Packet Notes

- Packet 50b (`50b_paint-input-3mf-mmu-supports`) is unblocked after AC-1 through AC-4 pass (Phase 1 complete).
- Packet 50 (`50_paint-input-3mf-ingestion`) is unaffected; its tests must continue to pass throughout.
- The test `load_3mf_subdivision_paint_rejects` (packet 50 artifact) is updated in Step 3 — it is renamed and repurposed, not deleted. The original rejection behavior (split bits ≠ 0 in short strings) is now superseded.
