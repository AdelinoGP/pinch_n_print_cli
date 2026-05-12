# Design: 50_paint-input-3mf-ingestion

## Implementation Shape

Bounded loader-only change. The shape is:

1. **One bounded loader edit.** `crates/slicer-host/src/model_loader.rs::parse_3mf_model_xml` learns to read all four OrcaSlicer per-triangle paint attributes (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`) and populate `FacetPaintData` from the per-triangle paint state. The line-150 `paint_data: None` literal is replaced by the discovered value. One new typed-error variant: `ModelLoadError::PaintMetadata { reason, byte_offset }`. One shared hex-state decoder (`decode_paint_hex_state`) handles all channels and rejects subdivision.

2. **One binary fixture.** `resources/benchy_painted.3mf` — Benchy geometry + `paint_fuzzy_skin` paint cluster on the smokestack triangles (fuzzy-skin only — supports/seam/mmu_color positive tests use synthetic in-test XML). Committed binary. Reproduction documented in `resources/benchy_painted.README.md`. A multi-channel binary fixture (`benchy_4color.3mf`) is reserved for Packet 50b.

3. **Eight test additions.** `crates/slicer-host/tests/model_loader_tdd.rs` gains eight new tests (four channel-positive: fuzzy_skin/supports/seam/mmu_color; four negative: malformed fuzzy_skin value, malformed support value, subdivision rejection, no-paint default). The pre-existing failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (2 tests, RED 2026-05-10) flip GREEN at packet close.

4. **One docs section.** `docs/02_ir_schemas.md` gains a "3MF paint-metadata extraction" subsection under FacetPaintData provenance, naming all four supported attributes.

5. **Three doc closures.** `docs/DEVIATION_LOG.md` DEV-044 → Closed; `docs/07_implementation_status.md` TASK-180 → `[x]`; `docs/14_deviation_audit_history.md` chronology entry.

Total churn estimate: ~ 374 LOC in `model_loader.rs` (4 channels + shared hex decoder), one binary fixture, one README, ~ 100 lines across three docs.

**Scope expansion note (2026-05-12):** Originally scoped to `paint_fuzzy_skin` only; widened mid-packet to all four channels because the per-triangle attribute decoder is channel-agnostic and adding three more arms costs little extra LOC. Architecture Constraint 4 below is updated accordingly.

## Controlling Code Paths and Surfaces

- **Primary edit surface:** `crates/slicer-host/src/model_loader.rs`. Specifically:
  - `parse_3mf_model_xml` — extend to recognize all four per-triangle paint attributes on each `<triangle>` element: `paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`. Per-attribute state is decoded via shared `decode_paint_hex_state` (1- or 2-nibble hex; 0xC continuation marker accepted; subdivision rejected).
  - `load_model` — `paint_data: None` literal replaced with the parser's `Option<FacetPaintData>` output. For STL and OBJ paths the literal `None` remains correct.
  - `ModelLoadError` enum — new `PaintMetadata { reason: String, byte_offset: usize }` variant.
- **Per-channel mapping (state → (PaintSemantic, PaintValue)):**
  | Attribute | State semantics | `PaintSemantic` | `PaintValue` |
  | --- | --- | --- | --- |
  | `paint_fuzzy_skin` | 0 = unpainted, 1 = painted (state > 1 → error) | `FuzzySkin` | `Flag(true)` |
  | `paint_supports` | 0 = unpainted, 1 = enforcer, 2 = blocker (state > 2 → error) | `SupportEnforcer` / `SupportBlocker` | `Flag(true)` |
  | `paint_seam` | 0 = unpainted, 1 = enforcer, 2 = blocker (state > 2 → error) | `Custom("seam_enforcer")` / `Custom("seam_blocker")` (no first-class seam variants in IR) | `Flag(true)` |
  | `paint_color` | 0 = unpainted, N>0 = tool index N | `Material` | `ToolIndex(N)` |
- Absent attribute = unpainted → `None` in that channel's `facet_values`. Any unexpected attribute value raises `PaintMetadata { reason: "<descriptive>", byte_offset }`. Use of `PaintSemantic::FuzzySkin` (first-class variant) — NOT `Custom("fuzzy_skin")` — is required (see Constraint 9 below).
- **Production of `FacetPaintData`:** decoder produces **one `PaintLayer` per active channel**: each `PaintLayer { semantic: <per table above>, facet_values: <Vec<Option<PaintValue>> of length facet_count>, strokes: Vec::new() }`, wrapped as `Some(FacetPaintData { layers })` if any channel is active; `None` otherwise. The per-layer shape is **pinned by existing consumer code**:
  - **Length invariant** (`crates/slicer-host/src/paint_segmentation.rs:93-100`): `facet_values.len() != facet_count` → `PaintSegmentationError::MalformedFacetValues`. So each layer's vector MUST be sized exactly `mesh.indices.len() / 3`, padding unpainted slots with `None`.
  - **Strokes ignored** (zero readers in `paint_segmentation.rs`): `strokes: Vec::new()` is correct for whole-facet decode (no sub-facet geometry to populate).
  - **Value arms**: per the mapping table — `Flag(true)` for fuzzy/supports/seam (booleans), `ToolIndex(N)` for `paint_color` (multi-tool). `crates/slicer-host/src/slice_postprocess.rs:366-374 default_fallback_value` aligns: `FuzzySkin → Flag(false)`, `SupportEnforcer/Blocker → Flag(false)`, `Material → ToolIndex(0)`. The painted complement values match these unpainted defaults' arms.

## Neighboring Tests and Fixtures

- **Failing E2E targets (already RED, must turn GREEN):**
  - `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs::painted_3mf_fixture_is_committed`
  - `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs::painted_benchy_3mf_reaches_paint_segmentation`
- **New tests to author (this packet):** Eight tests in `crates/slicer-host/tests/model_loader_tdd.rs`:
  - `load_3mf_extracts_fuzzy_skin_facets` (positive — uses `benchy_painted.3mf` fixture)
  - `load_3mf_extracts_support_facets` (positive — synthetic in-test XML)
  - `load_3mf_extracts_seam_facets` (positive — synthetic in-test XML)
  - `load_3mf_extracts_mmu_color` (positive — synthetic in-test XML)
  - `load_3mf_malformed_fuzzy_skin_rejects` (negative — typed error)
  - `load_3mf_malformed_support_value_rejects` (negative — typed error, state > 2)
  - `load_3mf_subdivision_paint_rejects` (negative — typed error, split bits ≠ 0 or hex string > 2 chars)
  - `load_3mf_without_paint_returns_none` (negative — no-paint default path)
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
3. **Whole-facet only.** The per-triangle attribute format guarantees whole-facet granularity implicitly (one attribute per triangle). Subdivision support is a follow-up packet.
4. **All four OrcaSlicer per-triangle paint channels** are wired in this packet: `paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`. The decoder uses a single shared `decode_paint_hex_state` for all four; per-attribute branches build separate `PaintLayer`s with channel-appropriate (`PaintSemantic`, `PaintValue`) mappings (see table above). Channel coverage was widened mid-packet from the original `paint_fuzzy_skin`-only scope; the binary fixture `benchy_painted.3mf` still carries fuzzy_skin only, with synthetic in-test XML covering the other three channels. A multi-channel binary fixture is reserved for Packet 50b.
5. **No CLI changes.** Paint enters exclusively via the 3MF model file.
6. **No paint-data warning on empty.** A 3MF with no paint metadata returns `paint_data: None` silently — identical to today's behavior for `resources/benchy.stl`.
7. **Negative-test specificity.** Malformed metadata must fail loudly. Silent partial parsing is forbidden.
8. **The pre-committed failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` must turn GREEN at packet close.** Their assertion text must NOT be weakened to accommodate implementation shortcuts. The line-106 docstring guidance is updated as part of this packet from `Custom("fuzzy_skin")` to `FuzzySkin` (strengthens fidelity; allowed under this constraint because it does not weaken any `assert!`).
9. **Use `PaintSemantic::FuzzySkin`, NOT `PaintSemantic::Custom("fuzzy_skin")`.** The IR exposes a first-class `FuzzySkin` variant (`crates/slicer-ir/src/slice_ir.rs:178`). Emitting `Custom("fuzzy_skin")` instead changes runtime behavior in four call sites: `paint_segmentation.rs:122` (triggers spurious `detect_custom_conflict`), `slice_postprocess.rs:369` (different fallback `Scalar(0.0)` vs `Flag(false)`), `layer_executor.rs:266` (different sort priority bucket), and `wit_host.rs:2319` (silently coerces to `ToolIndex(0)` on WIT view round-trip). The host's WIT-side parser at `dispatch.rs:1975` already maps the string `"fuzzy_skin"` → `PaintSemantic::FuzzySkin`; the loader must be consistent with that data path.

## Selected Approach

**Path A (selected, mid-packet expanded to Path C): Loader extension + binary fixture + decoder for all four whole-facet paint channels.**

- Extend `parse_3mf_model_xml` with a per-triangle attribute scanner. The scanner runs inside the existing `<triangle>` loop; for each of the four attributes (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`), if present, read the hex value via the shared `decode_paint_hex_state` helper. The helper accepts 1- or 2-nibble hex (with `0xC` continuation marker) and rejects subdivision (split bits ≠ 0 or hex strings > 2 chars). Per-channel state-value validation (e.g., `fuzzy > 1 → error`, `supports > 2 → error`) layered on top. Any unexpected value raises `PaintMetadata`.
- Construct `FacetPaintData` per the pinned shape in "Controlling Code Paths and Surfaces" above: one `PaintLayer` per active channel; per-layer `facet_values: Vec<Option<PaintValue>>` of length exactly `mesh.indices.len() / 3`. Painted facets get the channel-appropriate `Some(PaintValue::*)`; unpainted facets get `None` in that channel's layer. The IR shape is dictated by existing consumers; it is not deliberated here.
- Commit `resources/benchy_painted.3mf` as a real binary (fuzzy-skin paint only). Authoring: open `resources/benchy.stl` in OrcaSlicer; paint the smokestack with the fuzzy skin tool; export as 3MF; commit. Reproduction documented step-by-step. Multi-channel binary fixture (e.g., MMU + supports) is reserved for Packet 50b; this packet covers the supports/seam/mmu_color positive ACs via synthetic in-test XML buffers (the decoder is the same code path either way).

### Rejected Alternatives

- **Path B (synthesize a fixture programmatically via a Python or Rust script).** Rejected for the *binary* fixture because any home-grown encoder risks producing a fixture that the production decoder cannot parse, creating a false-positive AC. Using a real slicer (OrcaSlicer) to author the binary guarantees format conformance and matches real user input. (Synthetic in-test XML buffers for the supports/seam/mmu_color ACs are acceptable because they exercise the same parser with assertions tied to specific known triangles, and they do not stand in as the user-facing reproducible artifact.)
- **Path D (support TriangleSelector subdivision in v1).** Rejected. The per-triangle attribute format used by the fixture does not represent subdivision; subdivision support would require a different encoding (recursive 4-bit-nibble tree, hex strings > 2 chars) and is deferred to a follow-up packet. The current decoder rejects subdivision with a typed `PaintMetadata` error.
- **Path E (rely on a `--paint <sidecar.json>` CLI flag instead of 3MF).** Rejected by user direction 2026-05-10 ("STL support would make delivery harder for a use case that doesn't exist"). 3MF only.

(Path C — "decode all four paint channels in one packet" — was originally rejected for scope hygiene but was re-selected mid-implementation when the decoder turned out to be channel-agnostic by construction. See Implementation Shape scope-expansion note.)

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

## Open Questions

These must be resolved before activation (status: draft → active):

- **Q1 (RESOLVED in spec-review 2026-05-10; SUPERSEDED by scope expansion 2026-05-12)**: original v1 scope was `paint_fuzzy_skin` only. Resolved: yes for the binary fixture. **Scope expansion (2026-05-12):** the decoder also covers `paint_supports`, `paint_seam`, and `paint_color` because the channel-agnostic decoder absorbs them at little extra cost; the binary fixture remains fuzzy-skin only, with the other three channels' positive ACs exercised via synthetic in-test XML. A multi-channel binary fixture is reserved for Packet 50b.
- **Q2 (RESOLVED 2026-05-11)**: authoring tool = **OrcaSlicer** (BambuSlicer fork). Reproduction: open `resources/benchy.stl`; use the fuzzy-skin paint tool on the smokestack triangles; export as 3MF. The emitted format is a per-triangle attribute, not a positional bitstream, so whole-facet granularity is implicit.
- **Q3 (RESOLVED 2026-05-11)**: exact attribute name = **`paint_fuzzy_skin`** (unprefixed, no namespace URI). Discovered by inspecting the OrcaSlicer-exported fixture XML. Unpainted triangles omit the attribute; painted triangles carry `paint_fuzzy_skin="4"`.
- **Q4 (RESOLVED 2026-05-11)**: `ModelLoadError::PaintMetadata { reason: String, byte_offset: usize }` is acceptable. `byte_offset` refers to the XML stream offset for diagnostic purposes (the per-triangle attribute format does not use a positional bitstream, so the original rationale no longer applies, but the shape is still useful for pointing into the XML).

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
