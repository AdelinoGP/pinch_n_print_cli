# Implementation Plan: 50_paint-input-3mf-ingestion

## Execution Rules

- One atomic step at a time. Each step has its own precondition + postcondition + falsifying check.
- Each step honors the context-discipline preamble. Files-allowed-to-read, files-allowed-to-edit, expected dispatches, and context cost are budget contracts, not metadata.
- Stop reading at 60% context. Hand off at 85%.
- `crates/slicer-macros/src/lib.rs` is out of bounds; not touched.
- Do not run `cargo test --workspace` at any step. Use targeted `cargo test -p slicer-host --test <file>` only.

## Steps

### Step 1: Activation gate + grounding (3MF attribute name + FacetPaintData shape)

- Task IDs: `TASK-180`
- Objective: Resolve the remaining open questions Q2-Q4 (Q1 already resolved in spec-review 2026-05-10; FacetPaintData/PaintSemantic/PaintValue choices already pinned in design.md per spec-review). Specifically: (a) determine the literal 3MF XML attribute name and `xmlns:` URI for the `fuzzy_skin_facets` channel; (b) confirm authoring path for `benchy_painted.3mf` and that it emits a **whole-facet (unsubdivided)** bitstream; (c) confirm `ModelLoadError::PaintMetadata { reason, byte_offset }` is acceptable; (d) re-verify the IR-side invariants the design pins on (sanity check against drift since spec-review).
- Precondition: master is clean; failing tests at `benchy_painted_e2e_tdd.rs` already RED.
- Postcondition: Q2-Q4 resolutions recorded in design.md "Open Questions" section. Packet flips from `draft` to `active` only after the resolutions are recorded.
- Files allowed to read:
  - none directly. All discovery is via dispatch.
- Files allowed to edit:
  - `.ralph/specs/50_paint-input-3mf-ingestion/design.md` (record Q2-Q4 resolutions only).
  - `.ralph/specs/50_paint-input-3mf-ingestion/packet.spec.md` (flip `status: draft` → `active` after recording resolutions).
- Files explicitly out-of-bounds:
  - `crates/slicer-host/src/model_loader.rs` (read at Step 3, not here).
- Expected sub-agent dispatches:
  - `Question: What is the exact 3MF XML attribute name and xmlns: URI that OrcaSlicer/PrusaSlicer/Bambu Studio emit for facet-level "fuzzy skin painting"? Look at: (a) PrusaSlicer documentation on the Slic3rPE namespace; (b) 3MF Consortium core spec; (c) optionally inspect a 3MF fixture file produced by OrcaSlicer using the fuzzy-skin paint tool. Return: FACT (attribute name string + xmlns URI string)`.
  - `Question: For the chosen authoring path (OrcaSlicer GUI fuzzy-skin paint tool OR direct XML emission), confirm the produced 3MF will contain a WHOLE-FACET (unsubdivided) bitstream — GUI brush strokes that cross triangle boundaries produce subdivided bitstreams, which this packet rejects. Return: FACT (yes/no + recommended authoring procedure to guarantee whole-facet output, e.g. "select entire facet group via shift-click rather than brush stroke")`.
  - `Question: Confirm crates/slicer-host/src/model_loader.rs::ModelLoadError enum's current variants; return the enum body verbatim. Return: SNIPPET ≤ 20 lines`.
  - `Question: Re-verify the three pinned IR invariants from design.md "Data and Contract Notes": (1) crates/slicer-ir/src/slice_ir.rs PaintLayer fields are still {semantic, facet_values: Vec<Option<PaintValue>>, strokes}; (2) PaintSemantic still has a first-class FuzzySkin variant; (3) crates/slicer-host/src/slice_postprocess.rs default_fallback_value still maps PaintSemantic::FuzzySkin → PaintValue::Flag(false). Return: FACT yes/yes/yes or report drift`.
- Context cost: S (four FACT/SNIPPET dispatches).
- Authoritative docs: none directly.
- OrcaSlicer refs: dispatched only.
- Verification: 4 FACT/SNIPPET dispatches succeed; resolutions recorded.
- Exit condition: design.md Open Questions section shows Q2-Q4 resolved with the dispatch answers; pinned IR invariants confirmed un-drifted; packet.spec.md status flipped to `active`.

### Step 2: Author binary fixture `resources/benchy_painted.3mf`

- Task IDs: `TASK-180`
- Objective: Produce a real painted-Benchy 3MF whose `fuzzy_skin_facets` paint cluster covers the smokestack triangles. Commit the binary. Document the reproduction procedure.
- Precondition: Step 1 resolved Q1 (channel = `fuzzy_skin_facets`) and Q3 (attribute name known).
- Postcondition: `resources/benchy_painted.3mf` committed; `resources/benchy_painted.README.md` documents the authoring tool and exact steps to regenerate; `painted_3mf_fixture_is_committed` test (AC-2) goes GREEN.
- Files allowed to read:
  - `resources/benchy.stl` (existing — as the source geometry for the painted fixture).
- Files allowed to edit:
  - `resources/benchy_painted.3mf` (new binary).
  - `resources/benchy_painted.README.md` (new markdown).
- Files explicitly out-of-bounds:
  - any source code.
- Expected sub-agent dispatches:
  - `Question: Inspect resources/benchy_painted.3mf — confirm it contains at least one triangle with a non-zero fuzzy_skin_facets attribute; report the exact attribute name as it appears in the model XML so Step 3's parser can match. Return: FACT (yes/no + attribute name string)`.
- Context cost: S (one FACT dispatch; no code reads). NOTE: authoring is a manual GUI session OR a small scripted emit; this packet does not prescribe the tool. Document whichever path the implementer chooses.
- Authoritative docs: none.
- OrcaSlicer refs: none directly; the OrcaSlicer GUI is one option for authoring but its source is out of bounds.
- Verification:
  - `test -f resources/benchy_painted.3mf` succeeds.
  - `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_3mf_fixture_is_committed -- --exact --nocapture` PASS.
- Exit condition: AC-2 GREEN; fixture committed; README documents reproduction.

### Step 3: Extend `parse_3mf_model_xml` to decode all four per-triangle paint channels

- Task IDs: `TASK-180`
- Objective: Add the per-triangle attribute scanner covering all four channels (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`); add the shared `decode_paint_hex_state` helper (1- and 2-nibble hex, 0xC continuation marker, subdivision rejection); populate `FacetPaintData::layers` with one `PaintLayer` per active channel per the design.md mapping table; add `ModelLoadError::PaintMetadata { reason, byte_offset }`; replace `model_loader.rs:150` `paint_data: None` with the decoder's output.
- Precondition: Step 2 fixture exists; Step 1 grounded the FacetPaintData shape and the attribute names.
- Postcondition: `parse_3mf_model_xml` returns populated paint_data for the painted fixture; AC-1..AC-4 positive tests pass against the new model_loader_tdd tests.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` (entire file; ≤ 600 lines expected).
  - `crates/slicer-ir/src/slice_ir.rs` (only the FacetPaintData section; line range from Step 1 grounding).
  - `crates/slicer-host/src/paint_segmentation.rs:70-130` (only the consumer-side block).
- Files allowed to edit (≤ 1):
  - `crates/slicer-host/src/model_loader.rs`.
- Files explicitly out-of-bounds:
  - any other source file.
- Expected sub-agent dispatches:
  - `Question: cargo build --workspace after the load_3mf extension — does it compile? If not, return the first error line. Return: FACT pass/fail`.
- Context cost: M (one source file edited; multiple read-only sections).
- Authoritative docs: `docs/02_ir_schemas.md:64-83, :135` (read directly).
- OrcaSlicer refs: none additional beyond Step 1.
- Verification:
  - `cargo build --workspace` PASS.
- Exit condition: model_loader.rs compiles; `load_model` returns `Some(FacetPaintData)` when fed the painted fixture.

### Step 4: Add eight model_loader_tdd tests (4 channel-positive + 4 negative)

- Task IDs: `TASK-180`
- Objective: Add eight tests to `crates/slicer-host/tests/model_loader_tdd.rs`:
  - `load_3mf_extracts_fuzzy_skin_facets` (positive — calls `load_model` on the `benchy_painted.3mf` fixture; asserts `paint_data` is `Some(FacetPaintData)` with `layers.len() == 1`, layer `semantic == PaintSemantic::FuzzySkin`, `facet_values.len() == mesh.indices.len() / 3`, and at least one `Some(PaintValue::Flag(true))`).
  - `load_3mf_extracts_support_facets` (positive — synthetic in-test XML; asserts `SupportEnforcer` and/or `SupportBlocker` layers with `Flag(true)` on the painted triangles).
  - `load_3mf_extracts_seam_facets` (positive — synthetic in-test XML; asserts `Custom("seam_enforcer")` and/or `Custom("seam_blocker")` layers with `Flag(true)`).
  - `load_3mf_extracts_mmu_color` (positive — synthetic in-test XML; asserts `Material` layer with `ToolIndex(N)` matching the encoded state).
  - `load_3mf_malformed_fuzzy_skin_rejects` (negative — synthetic malformed `paint_fuzzy_skin` value; expects `Err(ModelLoadError::PaintMetadata { .. })`).
  - `load_3mf_malformed_support_value_rejects` (negative — `paint_supports` value > 2; expects `Err(ModelLoadError::PaintMetadata { .. })`).
  - `load_3mf_subdivision_paint_rejects` (negative — hex string indicating subdivision (split bits ≠ 0 or > 2 chars); expects `Err(ModelLoadError::PaintMetadata { .. })`).
  - `load_3mf_without_paint_returns_none` (negative — load a 3MF with no paint attribute; expects `paint_data == None` and no warning).
- Precondition: Step 3 complete; decoder works on the real fixture and on synthetic XML for all four channels.
- Postcondition: All eight new tests GREEN; AC-1..AC-4 and NEG-1..NEG-4 all GREEN.
- Files allowed to read:
  - `crates/slicer-host/tests/model_loader_tdd.rs` (full file; expected ≤ 400 lines).
- Files allowed to edit (≤ 1):
  - `crates/slicer-host/tests/model_loader_tdd.rs`.
- Files explicitly out-of-bounds:
  - any non-test source file.
- Expected sub-agent dispatches:
  - `Question: Run cargo test -p slicer-host --test model_loader_tdd; return pass/fail counts and the names of any failures. Return: FACT`.
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test model_loader_tdd` all PASS (existing tests + 3 new).
- Exit condition: all 3 new tests GREEN.

### Step 5: Flip the failing E2E benchy tests GREEN

- Task IDs: `TASK-180`
- Objective: Run the existing failing tests at `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` and confirm they go GREEN with no changes to the test file. The test assertions must not be weakened. (Note: these tests use the `benchy_painted.3mf` fixture, which carries fuzzy-skin paint only; that is sufficient to prove paint reaches PaintSegmentation end-to-end.)
- Precondition: Steps 3 and 4 complete; decoder works; fixture exists.
- Postcondition: Both E2E tests GREEN; backward-compat regression check confirms `benchy_e2e_real_pipeline_produces_gcode` stays GREEN.
- Files allowed to read:
  - none directly. Tests are already authored.
- Files allowed to edit:
  - none. The test file MUST NOT be modified in this packet — the assertions are the AC contract.
- Files explicitly out-of-bounds:
  - `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (no edits).
- Expected sub-agent dispatches:
  - `Question: Run cargo test -p slicer-host --test benchy_painted_e2e_tdd; report pass/fail per test. Return: FACT`.
  - `Question: Run cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode -- --exact; report pass/fail. Return: FACT`.
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - Both E2E tests GREEN.
  - Backward-compat test GREEN.
- Exit condition: AC-2, AC-3, AC-4 all GREEN.

### Step 6: Regression-defense battery

- Task IDs: `TASK-180`
- Objective: Dispatch the five Packet-43-rev1 regression-defense commands; confirm all stay GREEN.
- Precondition: Step 5 complete; all packet-local tests GREEN.
- Postcondition: Five regression commands all GREEN; AC-5 GREEN.
- Files allowed to read: none.
- Files allowed to edit: none.
- Expected sub-agent dispatches:
  - `Question: Run the five commands listed in packet.spec.md AC-5 (macro_paint_segmentation_output_roundtrip_tdd, macro_mesh_segmentation_output_roundtrip_tdd, dispatch_tdd macro_path, macro_all_worlds_roundtrip_tdd prepass, guest_fixture_freshness_tdd). Report pass/fail counts per command. Return: FACT (one line per command)`.
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all five GREEN.
- Exit condition: AC-5 GREEN.

### Step 7: docs/02 paint-extraction subsection + DEV-044 closure + TASK-180 closure

- Task IDs: `TASK-180`
- Objective: Add a "3MF paint-metadata extraction" subsection to `docs/02_ir_schemas.md`; flip DEV-044 to Closed in `docs/DEVIATION_LOG.md`; add `[x] TASK-180` to `docs/07_implementation_status.md`; add a chronology entry to `docs/14_deviation_audit_history.md`. All large-file reads are delegated.
- Precondition: Steps 1-6 complete.
- Postcondition: AC-6, AC-7, AC-8 all GREEN; clippy GREEN.
- Files allowed to read:
  - `docs/02_ir_schemas.md` (read the FacetPaintData section directly; ≤ 40 lines from line 64).
- Files allowed to edit (≤ 4 — one above the usual cap because four docs need closure-edits; each edit is small):
  - `docs/02_ir_schemas.md`
  - `docs/07_implementation_status.md` (via worker dispatch)
  - `docs/DEVIATION_LOG.md` (via worker dispatch)
  - `docs/14_deviation_audit_history.md` (via worker dispatch)
- Files explicitly out-of-bounds:
  - everything else.
- Expected sub-agent dispatches:
  - `Question: In docs/07_implementation_status.md, locate the appropriate row group (likely after TASK-167) where the new TASK-180 row should be inserted; report file:line for insertion. Return: FACT`.
  - `Question: In docs/DEVIATION_LOG.md, locate the DEV-044 row; report its file:line and current Status column value. Return: FACT`.
  - `Question: In docs/14_deviation_audit_history.md, locate the chronology section's tail (before ## Legacy Backlog Crosswalk); report file:line for appending. Return: FACT`.
  - After each edit: `Question: Verify <grep pattern> matches in <file>. Return: FACT`.
- Context cost: M (three docs, all delegated reads).
- Authoritative docs: the three docs being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q '3MF paint-metadata extraction' docs/02_ir_schemas.md` and `rg -q 'paint_fuzzy_skin' docs/02_ir_schemas.md` and `rg -q 'paint_supports' docs/02_ir_schemas.md` and `rg -q 'paint_seam' docs/02_ir_schemas.md` and `rg -q 'paint_color' docs/02_ir_schemas.md`.
  - `rg -q '^\| DEV-044.*Closed' docs/DEVIATION_LOG.md`.
  - `rg -q '\[x\] TASK-180' docs/07_implementation_status.md`.
  - `rg -q '2026-...DEV-044' docs/14_deviation_audit_history.md` (or equivalent date string).
  - `cargo clippy --workspace -- -D warnings` PASS.
- Exit condition: AC-6, AC-7, AC-8 GREEN; clippy GREEN.

### Step 8: Acceptance ceremony

- Task IDs: `TASK-180`
- Objective: Re-dispatch every pipe-suffixed AC verification command and confirm green. Run `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` for the final sweep.
- Precondition: Steps 1-7 complete.
- Postcondition: Packet ready to flip to `status: implemented`.
- Files allowed to read: none.
- Files allowed to edit:
  - `.ralph/specs/50_paint-input-3mf-ingestion/packet.spec.md` (status flip from `active` to `implemented` after all ACs green).
- Files explicitly out-of-bounds: everything else.
- Expected sub-agent dispatches:
  - One dispatch per AC (re-run each pipe-suffixed command). Each returns FACT pass/fail.
  - Final dispatch: `cargo clippy --workspace -- -D warnings`. Return FACT pass/fail.
- Context cost: M (11 positive ACs + 4 negs + clippy = 16 FACT dispatches).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all AC commands return PASS; clippy GREEN.
- Exit condition: all 15 ACs (11 positive + 4 negative) GREEN; clippy GREEN; packet ready for implementer to set `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | Four FACT/SNIPPET dispatches; activation gate. |
| 2 | S | Binary fixture authoring + one inspection dispatch. |
| 3 | M | Decoder implementation in model_loader.rs. |
| 4 | S | Eight new tests added (4 channel-positive + 4 negative); one cargo test dispatch. |
| 5 | S | Re-run pre-existing E2E tests; no edits. |
| 6 | S | Five regression dispatches. |
| 7 | M | Three docs (all delegated reads); clippy. |
| 8 | M | 11 FACT dispatches. |

Aggregate: M. No step is L. If any step measures L during execution, split before proceeding.

## Packet Completion Gate

- All 8 steps complete.
- Every step's exit condition met.
- All 15 ACs (11 positive + 4 negative) green.
- `cargo clippy --workspace -- -D warnings` green.
- DEV-044 flipped to Closed in `docs/DEVIATION_LOG.md`.
- TASK-180 closed `[x]` in `docs/07_implementation_status.md`.
- `docs/14_deviation_audit_history.md` chronology entry committed.
- `resources/benchy_painted.3mf` committed; `resources/benchy_painted.README.md` documents reproduction.
- `packet.spec.md` ready to move from `status: active` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`. Each returns FACT pass/fail.
- Confirm packet-level verification commands are green (cargo build, cargo clippy, targeted tests).
- Record the implementer's peak context usage. If it exceeded 70%, log it as a packet-authoring lesson — this packet was estimated M and should not have approached the budget.
