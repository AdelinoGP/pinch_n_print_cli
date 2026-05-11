# Design: 50_paint-input-3mf-ingestion

## Implementation Shape

Bounded loader-only change. The shape is:

1. **One bounded loader edit.** `crates/slicer-host/src/model_loader.rs::parse_3mf_model_xml` learns to read the `fuzzy_skin_facets` 3MF attribute and decode the whole-facet (unsubdivided) variant of the TriangleSelector bitstream into a populated `FacetPaintData`. The line-150 `paint_data: None` literal is replaced by the discovered value. One new typed-error variant: `ModelLoadError::PaintMetadata { reason, byte_offset }`.

2. **One binary fixture.** `resources/benchy_painted.3mf` — Benchy geometry + `fuzzy_skin_facets` paint cluster on the smokestack triangles. Committed binary. Reproduction documented in `resources/benchy_painted.README.md`.

3. **Three test additions.** `crates/slicer-host/tests/model_loader_tdd.rs` gains three new tests (one positive, two negative). The pre-existing failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (2 tests, RED 2026-05-10) flip GREEN at packet close.

4. **One docs section.** `docs/02_ir_schemas.md` gains a "3MF paint-metadata extraction" subsection under FacetPaintData provenance.

5. **Three doc closures.** `docs/DEVIATION_LOG.md` DEV-044 → Closed; `docs/07_implementation_status.md` TASK-180 → `[x]`; `docs/14_deviation_audit_history.md` chronology entry.

Total churn estimate: ~ 250 LOC in `model_loader.rs`, one binary fixture, one README, ~ 100 lines across three docs.

## Controlling Code Paths and Surfaces

- **Primary edit surface:** `crates/slicer-host/src/model_loader.rs`. Specifically:
  - `parse_3mf_model_xml` (`:280-352`) — extend to recognize the `fuzzy_skin_facets` attribute on each `<triangle>` element. Decoder produces a `Vec<(triangle_index, paint_state)>` from the hex-nibble bitstream.
  - `load_model` (`:102-150`) — line 150 `paint_data: None` replaced with the parser's output. For STL and OBJ paths the literal `None` remains correct.
  - `ModelLoadError` enum (existing) — new `PaintMetadata { reason: String, byte_offset: usize }` variant.
- **Mapping from paint_state to (PaintSemantic, PaintValue):** for the `fuzzy_skin_facets` channel, state-0 = unpainted → `None` in `facet_values`; state-1 = painted → `Some(PaintValue::Flag(true))` in `facet_values`. Semantic for the whole layer is `PaintSemantic::FuzzySkin` (the IR's first-class variant — NOT `PaintSemantic::Custom("fuzzy_skin")`; see Constraint 9 below). Any other nibble value indicates subdivision — reject with `PaintMetadata { reason: "subdivided facet not supported in v1; only whole-facet paint is decoded", byte_offset }`.
- **Production of `FacetPaintData`:** decoder produces exactly **one** `PaintLayer` per painted object: `PaintLayer { semantic: PaintSemantic::FuzzySkin, facet_values: <Vec<Option<PaintValue>> of length facet_count>, strokes: Vec::new() }`, wrapped as `Some(FacetPaintData { layers: vec![<that layer>] })`. The shape is **pinned by existing consumer code**, not an open Step-1 question:
  - **Length invariant** (`crates/slicer-host/src/paint_segmentation.rs:93-100`): `facet_values.len() != facet_count` → `PaintSegmentationError::MalformedFacetValues`. So the vector MUST be sized exactly `mesh.indices.len() / 3`, padding unpainted slots with `None`.
  - **Strokes ignored** (zero readers in `paint_segmentation.rs`): `strokes: Vec::new()` is correct for whole-facet decode (no sub-facet geometry to populate).
  - **Value arm = `Flag(true)`**: `crates/slicer-host/src/slice_postprocess.rs:366-374 default_fallback_value` maps `PaintSemantic::FuzzySkin → PaintValue::Flag(false)` as the unpainted default; the painted complement is therefore `Flag(true)`. Using any other arm (`ToolIndex`, `Scalar`, `Custom`) breaks downstream `Flag(_)` branching and the WIT-side coercion at `wit_host.rs:2319` silently degrades `Custom` values to `ToolIndex(0)`.

## Neighboring Tests and Fixtures

- **Failing E2E targets (already RED, must turn GREEN):**
  - `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs::painted_3mf_fixture_is_committed`
  - `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs::painted_benchy_3mf_reaches_paint_segmentation`
- **New tests to author (this packet):**
  - `crates/slicer-host/tests/model_loader_tdd.rs::load_3mf_extracts_fuzzy_skin_facets` (positive)
  - `crates/slicer-host/tests/model_loader_tdd.rs::load_3mf_malformed_fuzzy_skin_rejects` (negative — typed error)
  - `crates/slicer-host/tests/model_loader_tdd.rs::load_3mf_without_paint_returns_none` (negative — no-paint default path)
- **Regression-defense targets (must stay green):**
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_e2e_real_pipeline_produces_gcode`
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (full file)
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (full file)
  - `crates/slicer-host/tests/dispatch_tdd.rs` macro_path tier
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` prepass tier
  - `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs` (full file)

## Architecture Constraints (Locked Assumptions)

1. **Loader-only change.** No PaintSegmentation, RegionMap, host-validator, harvest, or WIT change. Their contracts are correct; the bug is upstream.
2. **`FacetPaintData` schema is unchanged.** This packet populates it; does not modify its shape.
3. **Whole-facet only.** Subdivided TriangleSelector bitstreams (recursive nibble pairs) are rejected with a typed error. Subdivision support is a follow-up packet.
4. **One channel only: `fuzzy_skin_facets`.** The other three OrcaSlicer paint channels (`supported_facets`, `seam_facets`, `mmu_segmentation_facets`) are deferred. The decoder's structure should be channel-agnostic so adding a channel in a follow-up packet is local, but only `fuzzy_skin_facets` is wired this packet.
5. **No CLI changes.** Paint enters exclusively via the 3MF model file.
6. **No paint-data warning on empty.** A 3MF with no paint metadata returns `paint_data: None` silently — identical to today's behavior for `resources/benchy.stl`.
7. **Negative-test specificity.** Malformed metadata must fail loudly. Silent partial parsing is forbidden.
8. **The pre-committed failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` must turn GREEN at packet close.** Their assertion text must NOT be weakened to accommodate implementation shortcuts. The line-106 docstring guidance is updated as part of this packet from `Custom("fuzzy_skin")` to `FuzzySkin` (strengthens fidelity; allowed under this constraint because it does not weaken any `assert!`).
9. **Use `PaintSemantic::FuzzySkin`, NOT `PaintSemantic::Custom("fuzzy_skin")`.** The IR exposes a first-class `FuzzySkin` variant (`crates/slicer-ir/src/slice_ir.rs:178`). Emitting `Custom("fuzzy_skin")` instead changes runtime behavior in four call sites: `paint_segmentation.rs:122` (triggers spurious `detect_custom_conflict`), `slice_postprocess.rs:369` (different fallback `Scalar(0.0)` vs `Flag(false)`), `layer_executor.rs:266` (different sort priority bucket), and `wit_host.rs:2319` (silently coerces to `ToolIndex(0)` on WIT view round-trip). The host's WIT-side parser at `dispatch.rs:1975` already maps the string `"fuzzy_skin"` → `PaintSemantic::FuzzySkin`; the loader must be consistent with that data path.

## Selected Approach

**Path A: Loader extension + binary fixture + decoder for whole-facet `fuzzy_skin_facets`.**

- Extend `parse_3mf_model_xml` with a per-triangle attribute scanner. The scanner runs inside the existing `<triangle>` loop; when an `fuzzy_skin_facets` attribute (exact name discovered via Step 1) is present, decode its hex string as a stream of 4-bit nibbles representing per-facet states. Whole-facet = single nibble per triangle (state 0..15 with 0=unpainted, 1=painted, others=error).
- Construct `FacetPaintData` per the pinned shape in "Controlling Code Paths and Surfaces" above: one `PaintLayer { semantic: PaintSemantic::FuzzySkin, facet_values: <Vec<Option<PaintValue>> of length facet_count>, strokes: Vec::new() }`. Painted facets get `Some(PaintValue::Flag(true))`; unpainted facets get `None`. The IR shape is dictated by existing consumers; it is not deliberated here.
- Commit `resources/benchy_painted.3mf` as a real binary. Authoring: open `resources/benchy.stl` in OrcaSlicer (or PrusaSlicer); paint the smokestack with the fuzzy skin tool; export as 3MF; commit. Reproduction documented step-by-step.

### Rejected Alternatives

- **Path B (synthesize a fixture programmatically via a Python or Rust script).** Rejected because the 3MF format's TriangleSelector bitstream encoding is non-trivial and any home-grown encoder risks producing a fixture that the production decoder cannot parse, creating a false-positive AC. Using a real slicer to author the fixture guarantees format conformance and matches real user input.
- **Path C (decode all four paint channels in one packet).** Rejected for scope hygiene. Each channel has distinct semantic mapping rules and distinct downstream consumers; mixing them in one packet inflates risk for a deviation-closure packet. Sequential follow-up packets are cleaner.
- **Path D (support TriangleSelector subdivision in v1).** Rejected. The subdivision tree is recursive 4-bit-nibble traversal; implementing it correctly requires a published reference implementation review. The whole-facet variant is sufficient for the painted-Benchy E2E and unblocks DEV-045 testing.
- **Path E (rely on a `--paint <sidecar.json>` CLI flag instead of 3MF).** Rejected by user direction 2026-05-10 ("STL support would make delivery harder for a use case that doesn't exist"). 3MF only.

## Code Change Surface (authoritative files-in-scope list)

Primary editing surfaces (these are the files an implementer edits):

1. `crates/slicer-host/src/model_loader.rs` (extend `parse_3mf_model_xml`; add `ModelLoadError::PaintMetadata` variant; replace `:150` `paint_data: None`).
2. `resources/benchy_painted.3mf` (new binary).
3. `resources/benchy_painted.README.md` (new doc; reproduction procedure).
4. `crates/slicer-host/tests/model_loader_tdd.rs` (add three new tests).
5. `docs/02_ir_schemas.md` (add "3MF paint-metadata extraction" subsection).
6. `docs/07_implementation_status.md` (add + close TASK-180 — via worker dispatch).
7. `docs/DEVIATION_LOG.md` (flip DEV-044 to Closed — via worker dispatch).
8. `docs/14_deviation_audit_history.md` (chronology entry — via worker dispatch).

No step opens more than 3 of these files at once.

## Read-Only Context the Implementer Needs

- `crates/slicer-host/src/model_loader.rs` — full file expected ≤ 600 lines; read directly with line-range hints (the `parse_3mf_model_xml` function is at `:280-352`).
- `crates/slicer-ir/src/slice_ir.rs` — for the exact `FacetPaintData` and `PaintLayer` struct shapes (read only the FacetPaintData section, ≤ 40 lines).
- `crates/slicer-host/src/paint_segmentation.rs:70-130` — read only the consumer of `paint_data.layers` to confirm the shape the loader must produce.
- `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` — full file (≤ 150 lines); read at Step 1 to anchor the AC contract.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs:130-158` — `run_slicer_host` helper signature only (for any new E2E test scaffolding).

## Out-of-Bounds Files (forbidden direct reads)

- `crates/slicer-macros/src/lib.rs` — out of scope, > 2 300 lines, no edit needed.
- `crates/slicer-host/src/paint_segmentation.rs` outside `:70-130` — the consumer is read-only context, not edit surface.
- `crates/slicer-host/src/region_mapping.rs`, `crates/slicer-host/src/config_resolution.rs`, `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/wit_host.rs` — all out of scope.
- `OrcaSlicerDocumented/` — delegate SUMMARY only; do NOT load directly.
- `target/` — generated artifacts.
- `wit/` and inline-WIT blocks in any crate — no WIT changes in this packet.
- Other `.ralph/specs/` packet directories.
- The OrcaSlicer C++ source (it is documented to not be checked into this repo, but for safety: any path matching `**/Format/3mf.cpp`, `**/Format/bbs_3mf.cpp`, `**/TriangleSelector.cpp`).

## Data and Contract Notes

- **`FacetPaintData` shape is pinned** (no Step-1 deliberation needed):
  - `Some(FacetPaintData { layers: vec![<one PaintLayer>] })` per painted object; `None` per unpainted object (no warning).
  - That single `PaintLayer` has `semantic: PaintSemantic::FuzzySkin`, `facet_values: Vec<Option<PaintValue>>` of length **exactly** `mesh.indices.len() / 3` (consumer at `paint_segmentation.rs:93-100` returns `MalformedFacetValues` on length mismatch), `strokes: Vec::new()` (consumer never reads `.strokes`).
  - Each `facet_values[i]`: `None` for state-0 unpainted; `Some(PaintValue::Flag(true))` for state-1 painted. The `Flag(_)` arm is dictated by `slice_postprocess.rs:366-374 default_fallback_value` mapping `FuzzySkin → Flag(false)` (the unpainted default; painted complement is `Flag(true)`).
- The `fuzzy_skin_facets` 4-bit-nibble bitstream encoding (per OrcaSlicer's `TriangleSelector::serialize`):
  - One nibble per facet for the unsubdivided variant. `0` = unpainted; `1` = painted; any other value = subdivided (NOT supported in v1; raise `PaintMetadata` error).
  - Two hex chars = 1 byte = 2 nibbles = 2 facets.
  - Facet ordering: the same triangle ordering used by the `<triangle>` elements in the model XML.
- The 3MF model XML attribute name: NOT documented in `OrcaSlicerDocumented/`. Step 1 must determine this via dispatch to (a) 3MF Consortium core spec docs, (b) PrusaSlicer `Slic3r_PE_namespace` documentation, or (c) inspect a real 3MF file exported from a known tool. The candidates per PrusaSlicer/Slic3rPE convention are `slic3rpe:fuzzy_skin_facets`, `slic3rpe:fuzzy_skin`, or `paint_fuzzy_skin`.

## Risks and Tradeoffs

- **Risk: 3MF attribute name discovery.** The exact attribute name is not in OrcaSlicerDocumented/. Step 1 grounding is critical; if multiple competing attribute names are found, the implementer must pick the one OrcaSlicer/PrusaSlicer actually emits today. Mitigation: author the binary fixture in a real slicer first (OrcaSlicer GUI), then inspect the emitted 3MF XML to discover the actual attribute name. This grounds the parser against real-world input.
- **Risk: `FacetPaintData::layers` shape mismatch.** If the IR shape expects per-Z-layer paint information (not per-triangle), the loader must produce a "single virtual layer" or "no layer; populate facets-list-only" path. Step 1 grounds this. If the IR shape change is needed, this packet escalates — IR changes are explicitly out of scope.
- **Tradeoff: whole-facet only.** A fixture authored in OrcaSlicer with paint that covers partial triangles will produce subdivided bitstreams. The first attempt to author `benchy_painted.3mf` may produce a subdivided fixture; the authoring procedure must paint at facet granularity (e.g., paint individual triangles or whole-mesh-section selection rather than brush strokes).
- **Tradeoff: docs/02 schema doc.** The new subsection is documentation-only; it does not bump `FacetPaintData::schema_version` because no IR shape change.

## Open Questions

These must be resolved before activation (status: draft → active):

- **Q1 (RESOLVED in spec-review 2026-05-10)**: confirm the `fuzzy_skin_facets` channel choice for v1. Resolved: yes. Alternatives like `mmu_segmentation_facets` rejected because MMU brings tool_index conflation out of scope.
- **Q2**: authoring tool / reproduction procedure for `resources/benchy_painted.3mf`. Recommended: OrcaSlicer GUI with documented step-by-step. Alternative: a deterministic Python script that emits the 3MF XML directly (requires step-1 attribute-name discovery first). Step-1 dispatch must additionally confirm the chosen authoring path produces a **whole-facet** (unsubdivided) bitstream — GUI brush strokes that cover partial triangles produce subdivided bitstreams, which this packet rejects.
- **Q3**: exact 3MF attribute name and namespace URI. Cannot be answered from OrcaSlicerDocumented/; Step 1 dispatch decides.
- **Q4**: `ModelLoadError::PaintMetadata` variant shape — recommended `{ reason: String, byte_offset: usize }` (byte_offset into the bitstream, not a triangle_index — because the bitstream is positional and a triangle index is not always known mid-decode). Confirm before adding the variant.

**Closed during spec-review (2026-05-10), no longer open**:
- ~~`PaintSemantic` choice~~ — pinned to `PaintSemantic::FuzzySkin` per Architecture Constraint 9.
- ~~`PaintValue` arm choice~~ — pinned to `PaintValue::Flag(true)` per Data and Contract Notes (consistent complement of `slice_postprocess.rs:369` unpainted default `Flag(false)`).
- ~~`FacetPaintData` shape~~ — pinned per Data and Contract Notes (one `PaintLayer`, `facet_values.len() == facet_count`, `strokes: Vec::new()`).

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
