---
status: implemented
packet: paint-region-transport-widening
task_ids:
  - TASK-130c
---

# 42_paint-region-transport-widening

## Goal

Widen the paint-region transport between the SDK builder, the WIT `paint-segmentation-output::push-paint-region` resource, and the host harvest into `PaintRegionIR` so paint regions carry **(a) full `ExPolygon` polygons with holes** (matching OrcaSlicer's `ExPolygons[layer][extruder]` shape and the IR's existing `SemanticRegion::polygons: Vec<ExPolygon>` field) and **(b) typed `PaintValue` variants** (replacing the lossy `value: string` channel that the host currently re-parses with a four-grammar guesser falling back to `ToolIndex(0)`). Migrate the canonical `paint-segmentation` core module's `wit-guest/` emit code to the new shape, and migrate the one direct-wiring test (`dispatch_tdd.rs::paint_segmentation_output_rejects_invalid_entries`). Result: the paint-region transport stops silently corrupting hole-bearing regions and stops silently coercing `Custom`-semantic / non-numeric `PaintValue`s to `ToolIndex(0)`. This packet is the architectural prerequisite for Packet 43, which will then drain `PaintSegmentationOutput` from the macro-authored prepass arm.

## Problem Statement

`DEV-025` ("Prepass segmentation SDK↔WIT shapes are still misaligned") originally enumerated three mismatches. While planning the close of mismatch 3 (the macro-arm drain, deferred from Packet 06), an architectural review surfaced **two additional mismatches** that the original DEV-025 audit did not catch:

- **Mismatch 4 — paint value channel string-coerced.** The WIT `paint-region-entry.value: string` is parsed by the host (`crates/slicer-host/src/dispatch.rs:1975-1985 parse_value`) using a four-grammar guesser: `"true"`/`"false"` → `PaintValue::Flag`, parsable `u32` → `ToolIndex`, parsable `f32` → `Scalar`, otherwise → `ToolIndex(0)`. The fallback is silent data loss. The feature purpose explicitly lists `Custom(id) → passed through for the registering module to consume` — a `Custom` semantic with a structured value (`"profile_high"`, `"color:#ff0000"`) currently degrades to `ToolIndex(0)`. Meanwhile the SDK already exposes a structured `PaintValueView { kind, flag, scalar, tool_index }` and the WIT *read* side already has a typed `paint-value` view (`wit_host.rs:2303-2308 ir_to_wit_paint_value`, `2413-2418 ir_to_wit_paint_value_view`, `5589-5594 convert_paint_value`). The string-coerced *write* side is asymmetric debt.

- **Mismatch 5 — SDK paint-region polygons hole-blind.** The SDK `PaintRegionEntry::contour_points: Vec<[f64; 2]>` cannot represent a region with interior holes. The IR target (`SemanticRegion::polygons: Vec<ExPolygon>`) has held holes since inception, and OrcaSlicer's facet-painted layer regions natively produce `ExPolygons[layer][extruder]` (Clipper convention: CCW outer contour, CW inner holes; see `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` and `pseudocode_multimaterial_segmentation.md`). Real-world examples that hole-blindness silently corrupts: a SupportEnforcer ring around an unpainted pillar (models as a filled disc → spurious supports inside the pillar); a Material ring around an unpainted core (wrong tool deposited in the core); a FuzzySkin window-frame paint (fuzz applied to the window opening too).

This packet closes mismatches 4 and 5. It does not close mismatch 3 — that is the deferred macro-arm drain and is the subject of the follow-on Packet 43.

This packet is the architectural prerequisite for Packet 43: the macro `PrePass::PaintSegmentation` arm cannot losslessly drain `PaintSegmentationOutput::regions()` through `push-paint-region` while the WIT carries `value: string` and the SDK carries `contour_points` instead of polygons. Plan A (split into Packet 42 + 43) was chosen over Plan B (single packet) because Plan B would push aggregate cost into L-territory and would mix transport-shape-refactor concerns with macro-arm drain concerns, complicating both review and acceptance.

This packet does **not** reopen or supersede a prior packet. It extends DEV-025 with mismatches that a fresh audit identified.

## Architecture Constraints

- **The IR is the contract anchor.** `crates/slicer-ir/src/slice_ir.rs` defines `SemanticRegion::polygons: Vec<ExPolygon>` (with `ExPolygon { contour, holes }`) and `PaintValue` (enum `Flag(bool) | Scalar(f32) | ToolIndex(u32) | ...`). The IR has been correctly shaped since Phase A; this packet aligns the SDK and WIT to match. **No IR shape change is in scope** except possibly the additive `PaintValue::Custom(String)` variant — Step 0 locks whether that addition is needed.
- **WIT view side already typed.** `wit_host.rs:2303-2308 ir_to_wit_paint_value`, `2413-2418 ir_to_wit_paint_value_view`, `5589-5594 convert_paint_value` already use a typed `paint-value` (or `paint-value-view`) variant for IR→guest reads. Widening the *push* side to a parallel typed variant is consistent, not novel. Step 0 confirms the exact name (`paint-value` vs `paint-value-input`) the new push variant should use; if a suitable type already exists in `wit/deps/ir-types.wit`, reuse it.
- **WIT version gate.** `crates/slicer-host/src/manifest.rs:705-729 WIT_WORLD_ALLOWLIST` fatally rejects unrecognized world versions. Per packet decision, this packet **does not bump the world-prepass version**. The architecture-rule deviation is recorded as a Locked Assumption below, with rationale and audit trail.
- **Determinism.** `harvest_paint_segmentation_ir` derives `paint_order` from `enumerate()` index (`idx as u64`, dispatch.rs:2024). This is order-deterministic per dispatch and does not need to change. SDK `PaintRegionEntry::paint_order` is therefore redundant and can be dropped (Step 0 confirms).
- **Cross-packet boundary.** The macro `PrePass::PaintSegmentation` arm at `crates/slicer-macros/src/lib.rs:1760-1788` is **Packet 43's territory**. This packet edits the **inline-WIT block** in the same file (lines 1283-1314) but does **not** edit the arm itself. Implementers must respect the line boundary.

## Data and Contract Notes

- IR or manifest contracts touched: `PaintRegionIR.per_layer[layer_index].semantic_regions[PaintSemantic][SemanticRegion]` shape is unchanged. The transport-side widening produces the same IR; this packet does not alter how IR consumers (paint_segmentation_executor, slice_postprocess) read it.
- WIT boundary considerations:
  - `paint-region-entry.value` becomes `paint-value-input` (variant). Cases: `flag(bool) | scalar(f32) | tool-index(u32) | custom(string)`.
  - `paint-region-entry.layer-index` reconciled to one type (Step 0 picks; canonical `layer-idx` (s32) is the more authoritative form per `wit/deps/ir-types.wit`).
  - Inline-WIT in `slicer-macros/src/lib.rs` MUST byte-match canonical (modulo whitespace) post-packet.
  - The `paint-segmentation-output::push-paint-region` method signature in WIT is unchanged in shape (`func(entry: paint-region-entry) -> result<_, string>`); only the entry's field types change.
- Determinism or scheduler constraints: `paint_order` is host-derived from enumeration index. Ordering is deterministic across the WIT push call sequence within a single dispatch. Removing the SDK `PaintRegionEntry::paint_order` field (Step 0 decision) does not affect determinism.

## Locked Assumptions and Invariants

- **WIT world-prepass stays at `@1.0.0` despite a non-additive change.** Rationale: DEV-025 is openly registered as "Prepass segmentation SDK↔WIT shapes are still misaligned"; no prepass module ships externally; the entire prepass contract is under active remediation per `docs/07_implementation_status.md`. Strict-version-rule deviation is recorded here as the audit trail. Mitigation: when the Architecture Acceptance Gate closes (TASK-130c + 130/130a/130b + other blockers), DEV-025 will close; the next non-additive WIT change after gate closure MUST follow the documented version-bumping rule.
- **`paint_order` is host-derived from enumeration index.** The SDK's `paint_order` field is redundant with the host's `idx as u64` derivation. Step 0 confirms no other reader exists; if true, the field is dropped. If a reader is found, Step 0 records it and the field stays.
- **`PaintSemantic::Custom(String)` already round-trips.** The semantic side is correctly typed via `parse_semantic` (dispatch.rs:1961) and `paint_semantic_to_string` (wit_host.rs:2402). This packet does not alter the semantic channel.
- **SDK `ExPolygonView` mirrors IR `ExPolygon` exactly.** Either `pub use slicer_ir::ExPolygon as ExPolygonView` or a 1:1 wrapper struct. Step 0 picks; the choice is recorded in dispatch.rs as a doc comment.
- **The macro `PrePass::PaintSegmentation` arm is Packet 43's territory.** This packet edits inline-WIT in the same file but does not modify the arm body. Implementers must respect the line boundary 1760-1788.

## Risks and Tradeoffs

- **Risk: `paint_order` field is read by a consumer the packet authoring scan missed.** Mitigation: Step 0's FACT dispatch is mandatory; if a reader is found, the field stays and the SDK API gains a `paint_order: u64` parameter on `push_paint_region`.
- **Risk: WIT bindgen surfaces `paint-value-input` with a kebab-cased Rust name that conflicts with existing `PaintValueView`.** Mitigation: `PaintValueInput` is the conventional bindgen output and is distinct. Step 0 confirms no name collision via Grep.
- **Risk: rebuilding `test-guests/prepass-guest.component.wasm` requires a wasm32 toolchain that isn't available on the implementer's Windows host.** Mitigation: Step 0 confirms toolchain availability; if not, the rebuild step is delegated to a sub-agent / CI; the implementer ships a "rebuild required" note and a CI handoff.
- **Risk: the canonical paint-segmentation guest's existing test suite has assertions on the `value` string that need migration.** Mitigation: Step 6 runs the guest's tests after migration and migrates any assertions that referenced string values; AC-6 catches a regression.
- **Tradeoff: keeping `world-prepass@1.0.0` violates the documented major-version rule.** Recorded as a Locked Assumption; the audit trail is the packet itself + DEV-025 mismatches 4 + 5.
- **Tradeoff: adding a `Custom(String)` variant to `PaintValue` (if Step 0 picks that path) is an IR shape change in a packet that otherwise is "out of scope of IR changes."** Justified because (a) it is additive (no consumer breaks); (b) it is the minimal IR change to faithfully represent the WIT `custom(string)` case; (c) without it, the harvest cannot uphold AC-5's typed round-trip for Custom values. The alternative (carry Custom only via `PaintSemantic::Custom`) is reasonable too but degrades the semantics: a `PaintSemantic::Material` paint with a non-numeric Custom value cannot be represented. Step 0 selects.
