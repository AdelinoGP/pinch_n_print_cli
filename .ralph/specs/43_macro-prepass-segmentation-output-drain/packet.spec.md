---
status: draft
packet: macro-prepass-segmentation-output-drain
task_ids:
  - TASK-130
  - TASK-130a
  - TASK-130b
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: macro-prepass-segmentation-output-drain

## Goal

Close `TASK-130`, `TASK-130a`, and `TASK-130b` (DEV-025 mismatch 3) by draining the SDK `PaintSegmentationOutput` builder back through the WIT `paint-segmentation-output::push-paint-region` resource from the `#[slicer_module]` macro's `PrePass::PaintSegmentation` arm — mirroring the `MeshSegmentation` arm's existing drain pattern at `crates/slicer-macros/src/lib.rs:1733-1746`. Add two end-to-end macro-path round-trip TDDs that load a macro-authored guest WASM and assert the harvested `PaintRegionIR` / `MeshSegmentationIR` faithfully reflects what the macro-authored module emitted, including hole-bearing polygons and Custom-semantic structured values (made possible by Packet 42's transport widening). Update `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, and `docs/14_deviation_audit_history.md` to reflect TASK-130/130a/130b completion and DEV-025 final closure (assuming Packet 42's mismatches 4 + 5 closure has landed).

## Scope Boundaries

- In scope:
  - **Macro arm drain** in `crates/slicer-macros/src/lib.rs::build_prepass_world_glue`, the `"PrePass::PaintSegmentation"` arm at lines 1760-1788. After the trait call returns `Ok`, before returning, drain `sdk_output.regions()` and push each entry via the WIT `paint-segmentation-output::push-paint-region` resource. Mirror the MeshSegmentation drain's error handling: `ModuleError { code: 10, fatal: true }` on push failure. Replace the existing comment block at lines 1760-1769 (which currently rationalizes why no drain exists) with a brief comment describing the drain pattern (or no comment if the code is self-evident).
  - **Round-trip TDD 1** — `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (NEW): load a macro-authored prepass guest WASM whose `run_paint_segmentation` calls `sdk_output.push_paint_region(...)` with at least one hole-bearing polygon and at least one `PaintValueView::Custom`-style payload (post Packet 42, the value channel is typed). Drive `dispatch_prepass_call` for `PrePass::PaintSegmentation`. Assert the harvested `PaintRegionIR` has matching `per_layer[N].semantic_regions` including hole geometry and the typed value variant.
  - **Round-trip TDD 2** — `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (NEW): symmetric end-to-end test for `MeshSegmentation`. A macro-authored guest calls `sdk_output.mark_triangle_paint`. Assert returned `MeshSegmentationIR` carries the marks. This is the test the original Packet 06 should have produced; it is added now as part of TASK-130b's "macro-path regression coverage" requirement.
  - **Test-guest authorship** — author or extend a macro-authored prepass guest under `test-guests/sdk-prepass-guest/` (per the existing pattern of `sdk-prepass-guest.component.wasm` and `sdk-layer-plan-guest.component.wasm`). The guest's `run_paint_segmentation` and `run_mesh_segmentation` must exercise `sdk_output.push_paint_region` and `sdk_output.mark_triangle_paint` respectively, with fixtures that include the hole-bearing + Custom-value cases. If a suitable macro guest already exists under `test-guests/`, extend it; if not, author a new one.
  - **Pre-built guest rebuild** — run `test-guests/build-test-guests.sh` to ensure the new/updated guest .wasm is available for the round-trip tests.
  - **Backlog updates** — flip TASK-130 (currently `[~]`), TASK-130a (currently `[ ]`), and TASK-130b (currently `[ ]`) to `[x]` in `docs/07_implementation_status.md`. Remove TASK-130a and TASK-130b from the Architecture Acceptance Gate blocker list at line ~180 (TASK-130c was added by Packet 42 and closes there too — confirm it is removed before this packet activates the closure note).
  - **DEV-025 closure** in `docs/DEVIATION_LOG.md`: mark mismatch 3 closed-by-Packet-43 with the closure date. Confirm mismatches 1 and 2 are already closed-by-Packet-06 (TASK-128a/128b per the handoff context) and mismatches 4 + 5 are closed-by-Packet-42. Set DEV-025 status to `closed` if all five are resolved.
  - **Audit-history update** in `docs/14_deviation_audit_history.md`: update DEV-025 row to reference TASK-130/130a/130b closure of mismatch 3, alongside the prior TASK-128a/128b and TASK-130c references.
- Out of scope:
  - SDK API widening, WIT shape changes, host harvest changes. All transport work is Packet 42's responsibility; this packet relies on Packet 42 being `implemented`.
  - Macro `PrePass::MeshSegmentation` arm drain — already implemented at lib.rs:1733-1746 in a prior change. This packet only ADDS a round-trip test for it; the production drain code is not modified.
  - Other prepass arms (`Slicing`, `LayerPlanning`, `MeshAnalysis`, `SeamPlanning`, `SupportPlanning`).
  - Module manifest edits.
  - WIT version bumps.
  - Any change to the IR schemas, host scheduler, or progress events.
  - Any new test guest beyond the one needed for the two round-trip TDDs. If `test-guests/sdk-prepass-guest.component.wasm` already covers the surface, extend rather than fork.

## Prerequisites and Blockers

- Depends on:
  - Packet `42_paint-region-transport-widening` — must be `implemented`. Without Packet 42, the macro arm drain cannot losslessly forward `sdk_output.regions()` (the SDK builder carries `ExPolygon`-bearing polygons + typed `PaintValueView`; the WIT push side must accept them in matching shape).
  - Packet `06_macro-prepass-segmentation-bridge` — `implemented` (closed DEV-025 mismatches 1 + 2 and laid the macro-arm scaffolding this packet completes). Confirmed at packet authoring.
- Unblocks:
  - DEV-025 final closure (assuming Packet 42 has closed mismatches 4 + 5).
  - The Architecture Acceptance Gate: removing TASK-130a and TASK-130b from the blocker list (alongside TASK-130c removed in Packet 42) brings the gate measurably closer to closure.
- Activation blockers:
  - Packet 42 must be `implemented` before this packet activates. Step 0 of the implementation plan FACT-confirms the predecessor's status.
  - Step 0 must FACT-confirm: (a) which existing test-guest (if any) under `test-guests/` is macro-authored and exercises the prepass world; (b) the exact line range of the PaintSegmentation arm body in `crates/slicer-macros/src/lib.rs` (the line numbers in this packet are valid at packet authoring; if Packet 42's inline-WIT edits shift them, Step 0 records the new range).

## Acceptance Criteria

- **Given** the macro `PrePass::PaintSegmentation` arm in `crates/slicer-macros/src/lib.rs::build_prepass_world_glue`, **when** the file is grepped within the arm body for `sdk_output.regions()` AND for `_output.push_paint_region`, **then** at least one matching iteration is found AND the iteration follows the trait call's `Ok(())` and precedes the arm's `Ok(())` return AND a `ModuleError { code: 10, fatal: true }` is constructed on push failure (mirrors the MeshSegmentation drain). | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd macro_arm_drains_regions_to_wit -- --exact --nocapture`
- **Given** a macro-authored prepass test guest whose `run_paint_segmentation` pushes one entry with `polygons = vec![ExPolygonView { contour: <square>, holes: vec![<inner square>] }]` and `value = PaintValueView::ToolIndex(7)` and `semantic = "material"` and `layer_index = 3` and `object_id = "obj-a"`, **when** the host runs `dispatch_prepass_call` for `PrePass::PaintSegmentation`, **then** the harvested `PaintRegionIR.per_layer[3].semantic_regions[PaintSemantic::Material][0]` has `polygons[0].holes.len() == 1` AND `value == PaintValue::ToolIndex(7)` AND `object_id == "obj-a"`. **This is the substantive macro-path round-trip validation that mismatch 3 of DEV-025 demanded.** | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd hole_bearing_typed_value_round_trips -- --exact --nocapture`
- **Given** a macro-authored prepass test guest whose `run_paint_segmentation` pushes one entry with `semantic = "custom:my_profile"` and `value = PaintValueView::Custom("profile_high".into())` (or the chosen Custom representation locked by Packet 42's Step 0), **when** the host runs `dispatch_prepass_call`, **then** the harvested `SemanticRegion.value` is **not** `PaintValue::ToolIndex(0)` (the silent-fallback failure mode the old transport silently produced) AND the Custom payload `"profile_high"` is preserved verbatim AND the resulting semantic key is `PaintSemantic::Custom("my_profile".into())`. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd custom_semantic_and_custom_value_round_trip -- --exact --nocapture`
- **Given** a macro-authored prepass test guest whose `run_mesh_segmentation` calls `sdk_output.mark_triangle_paint(object_id="obj-a", facet_index=12, semantic="material", value="3")`, **when** the host runs `dispatch_prepass_call` for `PrePass::MeshSegmentation`, **then** the harvested `MeshSegmentationIR` for `object_id="obj-a"` carries an entry with `facet_index == 12`, `semantic`-derived `PaintSemantic::Material`, and the value parsed appropriately for the MeshSegmentation transport (which still uses string `value` as out of scope per the packet boundary). | `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd mesh_segmentation_marks_round_trip -- --exact --nocapture`
- **Given** the macro arm push call returns an `Err` from the host validator (e.g., empty `polygons`), **when** the macro path drains, **then** the macro arm returns `ModuleError { code: 10, message: <the host error message verbatim>, fatal: true }` AND the surrounding dispatch surfaces this as a fatal failure (no silent drop). | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd push_failure_surfaces_as_fatal_module_error -- --exact --nocapture`
- **Given** `crates/slicer-macros/src/lib.rs` after this packet's edit, **when** the file is grepped within the PaintSegmentation arm for the original comment string `Same disconnect as MeshSegmentation` or `the SDK PaintSegmentationOutput builder operates on an in-Rust tree`, **then** zero matches are found (the comment that rationalized the missing drain is removed since the drain now exists). | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd legacy_comment_block_removed -- --exact --nocapture`
- **Given** `docs/07_implementation_status.md` after this packet, **when** the TASK-130 cluster (lines ~68-72) is read, **then** TASK-130 is `[x]`, TASK-130a is `[x]`, and TASK-130b is `[x]` AND the Architecture Acceptance Gate blocker list (line ~180) no longer contains TASK-130a or TASK-130b. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd docs_07_marks_130_cluster_done -- --exact --nocapture`
- **Given** `docs/DEVIATION_LOG.md` after this packet, **when** DEV-025 is read, **then** mismatch 3 is marked closed-by-Packet-43 AND mismatches 1, 2, 4, 5 are already closed (per packets 06 and 42) AND DEV-025's overall status is `closed`. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd dev_025_fully_closed -- --exact --nocapture`
- **Given** `docs/14_deviation_audit_history.md` after this packet, **when** the DEV-025 row is read, **then** the row references TASK-128a, TASK-128b, TASK-130, TASK-130a, TASK-130b, TASK-130c as the closing tasks. | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd dev_025_audit_history_complete -- --exact --nocapture`

## Negative Test Cases

- **Given** a macro-authored guest that calls `sdk_output.push_paint_region` with an empty `polygons: vec![]`, **when** the host runs `dispatch_prepass_call`, **then** the dispatch returns a fatal `ModuleError { code: 10, fatal: true }` (the host validator rejects empty polygons; the macro drain forwards the rejection without silent drop). | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd empty_polygons_rejected_at_host_validator -- --exact --nocapture`
- **Given** the macro arm body in `crates/slicer-macros/src/lib.rs`, **when** the file is grepped within the PaintSegmentation arm for an early `return Ok(())` that bypasses the drain, **then** zero such matches are found (no escape hatch that skips the drain). | `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd no_early_return_bypasses_drain -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./test-guests/build-test-guests.sh` (rebuild any updated guest)
- `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd -- --nocapture`
- `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd -- --nocapture`
- `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd -- --nocapture`
- `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd -- --nocapture`
- `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd -- --nocapture`
- `cargo test -p slicer-host --test macro_mesh_raycast_z_down_tdd -- --nocapture`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only — run once at acceptance ceremony, never during implementation iterations)

## Authoritative Docs

- `docs/05_module_sdk.md` — `#[slicer_module]` macro and the prepass-stage builder lifecycles. Delegate SUMMARY ≤ 200 words for the section that names `run_paint_segmentation` / `PaintSegmentationOutput`.
- `docs/02_ir_schemas.md` — `PaintRegionIR`, `MeshSegmentationIR` shapes. Direct read; narrow line ranges only.
- `docs/03_wit_and_manifest.md` — WIT prepass world surface. Delegate SUMMARY ≤ 150 words.
- `docs/14_deviation_audit_history.md` — DEV-025 audit row to be updated. Direct read; narrow.
- `docs/DEVIATION_LOG.md` — DEV-025 entry to close mismatch 3. Direct read; narrow.
- `.ralph/specs/06_macro-prepass-segmentation-bridge/` — predecessor packet; read only `packet.spec.md` and the relevant `design.md` section that DEFERRED 130a/130b. Direct read; narrow. Do not load all five files of the prior packet.
- `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` — co-prerequisite packet; confirm `status: implemented` before activation.

## OrcaSlicer Reference Obligations

- None directly required for this packet's drain mechanics. The OrcaSlicer parity argument was made and recorded in Packet 42 (paint-region transport shape mirrors `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md::ExPolygon` and `pseudocode_multimaterial_segmentation.md`). This packet's job is to wire the macro path through that already-corrected transport. If parity is challenged for the round-trip fixtures' polygon shapes, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` for context only — not for byte-identical behavior.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was authored under the spec-packet-generator's context_discipline preamble. Downstream agents must:

- Treat `design.md`'s code change surface as authoritative; touch nothing outside it.
- Honor `design.md`'s out-of-bounds list (no transport-shape changes; no IR changes; no other prepass arm changes; no module manifest changes; no version bumps).
- Delegate every cargo run, every workspace search, and every authoritative-doc fact-check.
- Stop reading at 60% context; hand off at 85%.

This is a **macro-arm-drain + end-to-end-round-trip-test** packet. The biggest implementation risks are (a) the macro arm's lifetime / borrow patterns differing subtly from the MeshSegmentation arm's and (b) the macro test-guest fixture authoring needing more plumbing than expected (component-model bindgen quirks for the typed `paint-value-input` variant, especially the `Custom(String)` case). AC-2 (`hole_bearing_typed_value_round_trips`) is the substantive macro-path proof — without it, mismatch 3 of DEV-025 cannot be closed. AC-3 (`custom_semantic_and_custom_value_round_trip`) is the proof that the original feature purpose ("Custom(id) → passed through for the registering module to consume") finally holds end-to-end through the macro path.
