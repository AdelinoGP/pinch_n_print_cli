# Requirements: paint-region-transport-widening

## Packet Metadata

- Grouped task IDs:
  - `TASK-130c` (registered by Step 1 of this packet's implementation plan; not yet present in `docs/07_implementation_status.md` at packet authoring time)
- Backlog source: `docs/07_implementation_status.md` (TASK-130 cluster, lines 68-70 + blocker list at line 180)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`DEV-025` ("Prepass segmentation SDK↔WIT shapes are still misaligned") originally enumerated three mismatches. While planning the close of mismatch 3 (the macro-arm drain, deferred from Packet 06), an architectural review surfaced **two additional mismatches** that the original DEV-025 audit did not catch:

- **Mismatch 4 — paint value channel string-coerced.** The WIT `paint-region-entry.value: string` is parsed by the host (`crates/slicer-host/src/dispatch.rs:1975-1985 parse_value`) using a four-grammar guesser: `"true"`/`"false"` → `PaintValue::Flag`, parsable `u32` → `ToolIndex`, parsable `f32` → `Scalar`, otherwise → `ToolIndex(0)`. The fallback is silent data loss. The feature purpose explicitly lists `Custom(id) → passed through for the registering module to consume` — a `Custom` semantic with a structured value (`"profile_high"`, `"color:#ff0000"`) currently degrades to `ToolIndex(0)`. Meanwhile the SDK already exposes a structured `PaintValueView { kind, flag, scalar, tool_index }` and the WIT *read* side already has a typed `paint-value` view (`wit_host.rs:2303-2308 ir_to_wit_paint_value`, `2413-2418 ir_to_wit_paint_value_view`, `5589-5594 convert_paint_value`). The string-coerced *write* side is asymmetric debt.

- **Mismatch 5 — SDK paint-region polygons hole-blind.** The SDK `PaintRegionEntry::contour_points: Vec<[f64; 2]>` cannot represent a region with interior holes. The IR target (`SemanticRegion::polygons: Vec<ExPolygon>`) has held holes since inception, and OrcaSlicer's facet-painted layer regions natively produce `ExPolygons[layer][extruder]` (Clipper convention: CCW outer contour, CW inner holes; see `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` and `pseudocode_multimaterial_segmentation.md`). Real-world examples that hole-blindness silently corrupts: a SupportEnforcer ring around an unpainted pillar (models as a filled disc → spurious supports inside the pillar); a Material ring around an unpainted core (wrong tool deposited in the core); a FuzzySkin window-frame paint (fuzz applied to the window opening too).

This packet closes mismatches 4 and 5. It does not close mismatch 3 — that is the deferred macro-arm drain and is the subject of the follow-on Packet 43.

This packet is the architectural prerequisite for Packet 43: the macro `PrePass::PaintSegmentation` arm cannot losslessly drain `PaintSegmentationOutput::regions()` through `push-paint-region` while the WIT carries `value: string` and the SDK carries `contour_points` instead of polygons. Plan A (split into Packet 42 + 43) was chosen over Plan B (single packet) because Plan B would push aggregate cost into L-territory and would mix transport-shape-refactor concerns with macro-arm drain concerns, complicating both review and acceptance.

This packet does **not** reopen or supersede a prior packet. It extends DEV-025 with mismatches that a fresh audit identified.

## In Scope

- Widen `crates/slicer-sdk/src/prepass_builders.rs::PaintRegionEntry` to carry `polygons: Vec<ExPolygonView>` (with holes) instead of `contour_points: Vec<[f64; 2]>`.
- Widen the WIT `paint-region-entry.value` from `string` to a typed `paint-value-input` variant covering `flag(bool) | scalar(f32) | tool-index(u32) | custom(string)`. Update the WIT in `wit/world-prepass.wit` and the inline-WIT mirror in `crates/slicer-macros/src/lib.rs:1283-1314`.
- Reconcile the inline-WIT/canonical drift on `paint-region-entry.layer-index` (currently `u32` in inline vs `layer-idx`/`s32` in canonical).
- Update `crates/slicer-host/src/wit_host.rs::HostPaintSegmentationOutput::push_paint_region` validation for the new types.
- Drop the `parse_value` closure in `crates/slicer-host/src/dispatch.rs::harvest_paint_segmentation_ir` and replace with a 1:1 typed mapping. Lock the `Custom` mapping in dispatch.rs as a top-of-function doc comment.
- Migrate the canonical `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs::run_paint_segmentation` to construct the typed `paint-region-entry`.
- Migrate `crates/slicer-host/tests/dispatch_tdd.rs::paint_segmentation_output_rejects_invalid_entries` (lines 5349-5441; the only direct-wiring test) to the typed shape.
- Rebuild `test-guests/prepass-guest.component.wasm` via `test-guests/build-test-guests.sh`.
- Register `TASK-130c` in `docs/07_implementation_status.md` (sibling row to 130a/130b at lines 68-70 + add to blocker list at line 180).
- Extend `docs/DEVIATION_LOG.md` DEV-025 with mismatches 4 (paint value channel string-coerced) and 5 (SDK paint-region polygons hole-blind), and mark both closed-by-Packet-42 at acceptance.
- Update `docs/14_deviation_audit_history.md` DEV-025 row to reference TASK-130c closure of mismatches 4 + 5.
- Author a new TDD file `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` with the host-side acceptance criteria assertions (and a parallel `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs` for the SDK-side ones).

## Out of Scope

- Macro `PrePass::PaintSegmentation` arm drain (lib.rs:1770-1788) — Packet 43.
- Macro `PrePass::MeshSegmentation` arm — already drained (lib.rs:1733-1746).
- Round-trip TDDs through the macro-authored guest path — Packet 43.
- WIT world-prepass version bump (`@1.0.0` → `@2.0.0`). Per packet metadata decision, the version stays at `@1.0.0` because DEV-025 is openly registered and the contract is under remediation. See Locked Assumptions in `design.md`.
- `WIT_WORLD_ALLOWLIST` updates in `crates/slicer-host/src/manifest.rs:705-729`.
- Module manifest `wit_world` version updates for the 5 prepass-world modules (`layer-planner-default`, `mesh-segmentation`, `paint-segmentation`, `seam-planner-default`, `support-planner`).
- The `mesh-segmentation-output::mark-triangle-paint` transport. It is already symmetric and string-typed; a parallel widening can be its own future packet if motivated.
- Any change to `crates/slicer-ir/src/slice_ir.rs` to alter `PaintValue`, `PaintSemantic`, `ExPolygon`, `SemanticRegion`, or `PaintRegionIR` — except for one possible additive change: a `PaintValue::Custom(String)` variant. Step 0 of this packet's implementation plan locks whether the harvest needs that variant or whether `PaintSemantic::Custom(String)` already covers the channel. If `PaintValue::Custom(String)` is needed, that addition is in scope and treated as additive.
- gcode-emit changes. Confirmed at packet authoring that `crates/slicer-host/src/gcode_emit.rs` does not read `PaintValue`.
- Snapshot / golden file updates. None exist for paint regions.
- Python or CLI tool migration. Confirmed none exist that touch this transport.

## Authoritative Docs

- `docs/02_ir_schemas.md` — Direct read; narrow line ranges only. The PaintRegionIR section is the target the SDK and WIT widen *toward*. Do not load the whole file.
- `docs/03_wit_and_manifest.md` — Delegate SUMMARY ≤ 200 words for the WIT version-bumping rule. Locked assumption: stay at `@1.0.0` despite a non-additive change. The deviation rationale is recorded in `design.md`.
- `docs/05_module_sdk.md` — Delegate SUMMARY for the `PaintSegmentationOutput` section.
- `docs/07_implementation_status.md` — Direct read for TASK-130 cluster (lines 68-70) and blocker list (line 180); never load the whole file.
- `docs/14_deviation_audit_history.md` — Direct read; narrow (DEV-025 row only).
- `docs/DEVIATION_LOG.md` — Direct read; narrow (DEV-025 entry only).

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md` lines 211–225 — `ExPolygon` definition (Clipper convention). Parity anchor: paint regions natively support holes.
- `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — MMU segmentation produces `ExPolygons[layer][extruder]`. Borrow: data shape. Do not borrow: the Voronoi algorithm itself (out of scope).
- `OrcaSlicerDocumented/generated_documentation/01_system_architecture.md` lines 73–98 — slicing pipeline output `std::vector<ExPolygons>`.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases: see `packet.spec.md` Acceptance Criteria — SDK struct widening (AC-1), SDK round-trip with hole + typed value (AC-2), WIT typed-variant declaration (AC-3), host harvest 1:1 mapping (AC-4), end-to-end hole + typed value transport (AC-5, the substantive proof), canonical guest still passing (AC-6), migrated direct-wiring test (AC-7), pre-built test-guest rebuild OK (AC-8), backlog + DEV log updates (AC-9, AC-10).
- Negative cases: hole-fidelity proof (`fuzzy_skin_ring_with_hole_preserves_hole`); Custom-value silent-fallback elimination (`custom_value_does_not_coerce_to_tool_index_zero`); old-API genuine removal (`contour_points_api_is_fully_removed`); inline/canonical WIT byte-match (`inline_and_canonical_wit_match`).
- Measurable outcomes:
  - `crates/slicer-sdk/src/prepass_builders.rs` no longer contains `contour_points` (the substring) anywhere on `PaintRegionEntry` or `push_paint_region`.
  - `wit/world-prepass.wit::paint-region-entry.value` is a `paint-value-input` variant with exactly four cases.
  - `crates/slicer-host/src/dispatch.rs::harvest_paint_segmentation_ir` contains zero `parse_value` references and zero `parse::<u32>()`/`parse::<f32>()` calls inside its body.
  - `docs/07_implementation_status.md` carries a TASK-130c row near 130a/130b and a TASK-130c entry in the blocker list.
  - `docs/DEVIATION_LOG.md` DEV-025 carries mismatches 4 and 5 with closure annotations.
- Cross-packet impact: this packet **unblocks** Packet 43. It does **not** close DEV-025 entirely — DEV-025 closes when Packet 43 lands and mismatch 3 is resolved.

## Verification Commands

- `cargo build --workspace`
- `./test-guests/build-test-guests.sh` (delegate; small parseable output: success line + `prepass-guest.component.wasm` size)
- `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test paint_region_transport_widening_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test dispatch_tdd paint_segmentation_output_rejects_invalid_entries -- --exact --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p paint-segmentation -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test paint_segmentation_executor_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test paint_annotation_integration_tdd -- --nocapture` (FACT pass/fail)
- `cargo clippy --workspace -- -D warnings` (FACT pass/fail)

All commands above are delegation-friendly: a sub-agent dispatch returns FACT (pass/fail + exit code) or SNIPPETS (≤ 20 lines of failing assertion).

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: stated as a binary/observable check (e.g., "Packet 06 is implemented" or "Step 1 has produced TASK-130c row in docs/07").
- Postcondition: stated as a binary/observable check (e.g., "the file contains exactly one `polygons: Vec<ExPolygonView>` field on `PaintRegionEntry`").
- Falsifying check: the cheapest grep/test that would catch a regression in this step (named per step in the plan).
- Files allowed to read (with line-range hints when > 300 lines): named per step. Notably:
  - `crates/slicer-host/src/wit_host.rs` is large (>5000 lines) — read only the named line ranges via Grep + targeted Read; never load the whole file.
  - `crates/slicer-host/src/dispatch.rs` is large — same rule, lines 1954-2045 only for the harvest function.
  - `docs/07_implementation_status.md` is large — Grep first, then Read the targeted line range.
- Files allowed to edit (≤ 3 per step): named per step.
- Expected sub-agent dispatches: named per step (e.g., "Run `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd sdk_paint_region_entry_carries_expolygon_view`; return FACT pass/fail").
- Step context cost: stated per step (`S` or `M` only).

## Context Discipline Notes

Document context-budget hazards specific to this packet:

- **Large files in the read-only path that MUST be ranged or delegated**:
  - `crates/slicer-host/src/wit_host.rs` (~5800 lines) — read only the named line ranges (1371-1376 for the ctx field, 4074-4102 for the host trait impl, 2300-2330 for existing typed view helpers). Use Grep first.
  - `crates/slicer-host/src/dispatch.rs` (~2100 lines) — only `harvest_paint_segmentation_ir` (lines 1954-2045).
  - `docs/07_implementation_status.md` — only the TASK-130 cluster (lines 65-72) and the blocker list (lines 175-185). Never read the full doc.
  - `crates/slicer-macros/src/lib.rs` (~2000 lines) — only the inline-WIT block (lines 1283-1314). The PaintSegmentation arm at 1760-1788 is **out of bounds for this packet** — it is Packet 43's territory.
- **OrcaSlicer trees the implementer must NOT load directly**: the entire `OrcaSlicerDocumented/` tree. Delegate every parity check.
- **Likely temptation reads (skip)**:
  - `crates/slicer-ir/src/slice_ir.rs` — read only the `PaintValue`, `PaintSemantic`, `ExPolygon`, `SemanticRegion`, `PaintRegionIR` definitions (Step 0 confirms exact line ranges via Grep). Resist reading neighboring IR types.
  - The 14 paint-related test files in `crates/slicer-host/tests/` listed in the packet authoring scan — except `dispatch_tdd.rs` (1 hunk) and `macro_paint_region_roundtrip_tdd.rs` (run-only, no edit), do not load. They auto-propagate through the harvest path.
  - The macro `PrePass::PaintSegmentation` arm (lib.rs:1760-1788). Reading it is harmless but editing it is a packet boundary violation.
- **Sub-agent return-format hints for the heaviest dispatches**:
  - `wit_host.rs` lookups → LOCATIONS or SNIPPETS (≤ 30 lines each); never SUMMARY of the whole file.
  - Workspace test runs → FACT (pass/fail + exit code) or SNIPPETS (≤ 20 lines of failing assertion).
  - `build-test-guests.sh` → FACT (success line + new size of `prepass-guest.component.wasm`).
  - Doc edits to `docs/07_implementation_status.md` → dispatch a worker for the edit; receive a 3-line confirmation including the inserted line number.
