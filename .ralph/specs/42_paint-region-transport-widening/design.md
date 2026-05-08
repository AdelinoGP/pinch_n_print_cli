# Design: paint-region-transport-widening

## Controlling Code Paths

- Primary code path: SDK builder (`crates/slicer-sdk/src/prepass_builders.rs::PaintRegionEntry` + `PaintSegmentationOutput::push_paint_region`) → WIT resource (`wit/world-prepass.wit::paint-segmentation-output::push-paint-region` + the `paint-region-entry` record) → host validator (`crates/slicer-host/src/wit_host.rs::HostPaintSegmentationOutput::push_paint_region`, lines 4074-4102) → host execution context (`HostExecutionContext::paint_region_entries`, wit_host.rs:1371-1376) → host harvest (`crates/slicer-host/src/dispatch.rs::harvest_paint_segmentation_ir`, lines 1954-2045) → IR (`crates/slicer-ir/src/slice_ir.rs::PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`).
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/dispatch_tdd.rs` lines 5349-5441 — direct-wiring test that constructs `pm::PaintRegionEntry` against the WIT-generated struct. Migrates with the WIT shape.
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — exercises `PaintRegionIR` via the harvest path; auto-propagates if the harvest stays correct.
  - `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs::run_paint_segmentation` — the canonical guest emit; updates with the WIT shape.
  - `crates/slicer-host/tests/paint_segmentation_executor_tdd.rs`, `paint_annotation_integration_tdd.rs`, `slice_postprocess_paint_annotation_tdd.rs` — IR-level tests; auto-propagate.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` (ExPolygon = contour + holes, Clipper convention) and `pseudocode_multimaterial_segmentation.md` (per-layer per-extruder ExPolygon arrays as the canonical paint-region shape). Parity claim: paint regions natively support holes and structured per-region values; the new SDK + WIT shapes mirror this.

## Architecture Constraints

- **The IR is the contract anchor.** `crates/slicer-ir/src/slice_ir.rs` defines `SemanticRegion::polygons: Vec<ExPolygon>` (with `ExPolygon { contour, holes }`) and `PaintValue` (enum `Flag(bool) | Scalar(f32) | ToolIndex(u32) | ...`). The IR has been correctly shaped since Phase A; this packet aligns the SDK and WIT to match. **No IR shape change is in scope** except possibly the additive `PaintValue::Custom(String)` variant — Step 0 locks whether that addition is needed.
- **WIT view side already typed.** `wit_host.rs:2303-2308 ir_to_wit_paint_value`, `2413-2418 ir_to_wit_paint_value_view`, `5589-5594 convert_paint_value` already use a typed `paint-value` (or `paint-value-view`) variant for IR→guest reads. Widening the *push* side to a parallel typed variant is consistent, not novel. Step 0 confirms the exact name (`paint-value` vs `paint-value-input`) the new push variant should use; if a suitable type already exists in `wit/deps/ir-types.wit`, reuse it.
- **WIT version gate.** `crates/slicer-host/src/manifest.rs:705-729 WIT_WORLD_ALLOWLIST` fatally rejects unrecognized world versions. Per packet decision, this packet **does not bump the world-prepass version**. The architecture-rule deviation is recorded as a Locked Assumption below, with rationale and audit trail.
- **Determinism.** `harvest_paint_segmentation_ir` derives `paint_order` from `enumerate()` index (`idx as u64`, dispatch.rs:2024). This is order-deterministic per dispatch and does not need to change. SDK `PaintRegionEntry::paint_order` is therefore redundant and can be dropped (Step 0 confirms).
- **Cross-packet boundary.** The macro `PrePass::PaintSegmentation` arm at `crates/slicer-macros/src/lib.rs:1760-1788` is **Packet 43's territory**. This packet edits the **inline-WIT block** in the same file (lines 1283-1314) but does **not** edit the arm itself. Implementers must respect the line boundary.

## Code Change Surface

- Selected approach: typed `paint-value-input` variant on the WIT push side, mirroring the existing typed `paint-value-view` on the read side. SDK `PaintRegionEntry` carries a new `ExPolygonView` re-exporting (or wrapping) the IR's `ExPolygon` shape. Host harvest becomes a direct typed conversion. Canonical guest migrates to typed emit. One direct-wiring test migrates to the typed shape. World-prepass version stays at `@1.0.0` per packet decision.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-sdk/src/prepass_builders.rs::PaintRegionEntry` (struct fields)
  - `crates/slicer-sdk/src/prepass_builders.rs::PaintSegmentationOutput::push_paint_region` (signature + body)
  - `crates/slicer-sdk/src/prepass_builders.rs::ExPolygonView` (new type, or `pub use slicer_ir::ExPolygon as ExPolygonView`)
  - `wit/world-prepass.wit::paint-region-entry` (replace `value: string` with `value: paint-value-input`; reconcile `layer-index` type)
  - `wit/deps/ir-types.wit` — add `paint-value-input` variant if not already present (Step 0 confirms)
  - `crates/slicer-macros/src/lib.rs::build_prepass_world_glue` inline-WIT block at lines 1283-1314 (mirror canonical exactly)
  - `crates/slicer-host/src/wit_host.rs::HostPaintSegmentationOutput::push_paint_region` (lines 4074-4102) — validation update
  - `crates/slicer-host/src/wit_host.rs::HostExecutionContext::paint_region_entries` (line 1371-1376) — type update via WIT bindgen
  - `crates/slicer-host/src/dispatch.rs::harvest_paint_segmentation_ir` (lines 1954-2045) — drop `parse_value`, replace with typed match
  - `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs::run_paint_segmentation` (lines ~356-419) — typed emit
  - `crates/slicer-host/tests/dispatch_tdd.rs::paint_segmentation_output_rejects_invalid_entries` (lines 5349-5441) — fixture migration
  - `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` (NEW) — host-side acceptance assertions
  - `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs` (NEW) — SDK-side acceptance assertions
  - `docs/07_implementation_status.md` — TASK-130c row + blocker-list entry
  - `docs/DEVIATION_LOG.md` — DEV-025 mismatches 4 + 5
  - `docs/14_deviation_audit_history.md` — DEV-025 row update
- Rejected alternatives that were considered and why they were not chosen:
  - **Single-contour polygons + value: string serialized from PaintValueView at the macro boundary** (Option (a) in the design discussion). Rejected because: (1) hole-blindness silently corrupts SupportEnforcer/Material/FuzzySkin regions on objects with interior unpainted areas; (2) string coercion silently degrades `Custom`-semantic values to `ToolIndex(0)` per `parse_value`'s fallback. The feature purpose explicitly demands `Custom(id) → passed through`; (a) cannot deliver this.
  - **SDK widening only, leaving WIT `value: string` as a deprecated-but-functional channel** (the additive option). Rejected because: it doubles the contract surface, demands a parallel parser, and leaves a known-bad channel callable indefinitely. The contract is internal; pre-1.0 we can break it cleanly.
  - **Bumping `world-prepass@1.0.0` → `@2.0.0`** (the strict-versioning option). Rejected per packet decision because DEV-025 is openly registered, no prepass module is shipped externally, and the entire WIT contract is under remediation per `docs/07_implementation_status.md`. Recorded as a Locked Assumption deviation from the documented version-bumping rule, with mitigation: this packet's existence and DEV-025 mismatches 4 + 5 are the audit trail.
  - **Splitting the canonical guest migration into Packet 42b before Packet 43**. Rejected because the WIT shape change forces the guest to update; there is no compile-clean intermediate state. Migrating it inside Packet 42 keeps `cargo build --workspace` green at every step exit.

## Files in Scope (read + edit)

This packet edits more than 3 primary files because the transport widening crosses three layers (SDK / WIT / host) plus the canonical guest. Each layer has exactly one or two file edits; the implementation plan splits them into atomic per-layer steps so no single step touches more than 3 files.

- `crates/slicer-sdk/src/prepass_builders.rs` — role: SDK builder + types; expected change: drop `contour_points`, add `polygons: Vec<ExPolygonView>`, add `ExPolygonView` type, drop or update `paint_order` per Step 0.
- `wit/world-prepass.wit` — role: canonical WIT for prepass world; expected change: `paint-region-entry.value: paint-value-input`; reconcile `layer-index` type.
- `wit/deps/ir-types.wit` — role: shared WIT types; expected change: add `paint-value-input` variant if not already present.
- `crates/slicer-macros/src/lib.rs` lines 1283-1314 — role: inline-WIT mirror; expected change: byte-identical to canonical (modulo whitespace).
- `crates/slicer-host/src/wit_host.rs` (narrow line ranges only) — role: host validator + execution context; expected change: validate the new `paint-value-input` variant + the `polygons` list shape.
- `crates/slicer-host/src/dispatch.rs` lines 1954-2045 — role: host harvest; expected change: drop `parse_value`, replace with typed mapping.
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — role: canonical guest; expected change: typed-variant emit.
- `crates/slicer-host/tests/dispatch_tdd.rs` lines 5349-5441 — role: direct-wiring test; expected change: fixture migration.
- `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` (NEW) — role: host-side AC tests.
- `crates/slicer-sdk/tests/paint_region_transport_widening_tdd.rs` (NEW) — role: SDK-side AC tests.
- `docs/07_implementation_status.md` — TASK-130c row + blocker list entry.
- `docs/DEVIATION_LOG.md` — DEV-025 mismatches 4 + 5.
- `docs/14_deviation_audit_history.md` — DEV-025 row update.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — read only the lines defining `PaintValue`, `PaintSemantic`, `ExPolygon`, `Polygon`, `Point2`, `SemanticRegion`, `LayerPaintMap`, `PaintRegionIR` (Grep first; lines ~890-955 region per packet authoring scan). Purpose: confirm the IR target shape this packet aligns the SDK + WIT to.
- `docs/02_ir_schemas.md` — read only the PaintRegionIR section. Purpose: confirm IR contract for paint regions.
- `docs/03_wit_and_manifest.md` — delegate SUMMARY ≤ 200 words for the version-bumping rule. Purpose: document the deviation in the Locked Assumptions.
- `crates/slicer-host/src/manifest.rs:705-729 WIT_WORLD_ALLOWLIST` — read these lines only. Purpose: confirm no allowlist change is required (we keep `@1.0.0`).
- `crates/slicer-host/src/wit_host.rs:2303-2418` — read only the existing `ir_to_wit_paint_value*` helpers. Purpose: locate the typed-variant naming convention to mirror on the push side.
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` — delegate; never load. Purpose: cite ExPolygon parity in DEVIATION_LOG entry.
- `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — delegate; never load. Purpose: cite paint-region-with-holes parity.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate every parity check; never load.
- `target/`, `Cargo.lock`, generated bindgen output under `target/` — never load.
- `vendor/` or equivalent vendored deps — never load.
- `crates/slicer-macros/src/lib.rs` lines 1730-1788 (the MeshSegmentation drain block + the PaintSegmentation arm body) — out of bounds for this packet. Reading them to navigate is fine; **editing them is a packet boundary violation** (Packet 43's territory).
- `crates/slicer-host/src/gcode_emit.rs` — confirmed not to read PaintValue. Do not load.
- The 13 paint-related test files in `crates/slicer-host/tests/` that are not `dispatch_tdd.rs`, `macro_paint_region_roundtrip_tdd.rs`, `paint_segmentation_executor_tdd.rs`, `slice_postprocess_paint_annotation_tdd.rs`, or `paint_annotation_integration_tdd.rs` — auto-propagate; no edit needed; do not load.
- Module manifests under `modules/core-modules/*/` — none need version-bump edits (allowlist unchanged); do not load.
- The full `crates/slicer-host/src/wit_host.rs` (~5800 lines) — only narrow line-ranges per the change surface. Do not load whole file.
- The full `crates/slicer-host/src/dispatch.rs` (~2100 lines) — only `harvest_paint_segmentation_ir` (lines 1954-2045). Do not load whole file.
- `docs/07_implementation_status.md` outside the TASK-130 cluster (lines 65-72) and the blocker list (lines 175-185). Do not load whole file.

## Expected Sub-Agent Dispatches

- "FACT-confirm: does `wit/deps/ir-types.wit` already declare a `paint-value-input` variant or equivalently shaped type? If yes, return its full record/variant declaration; if no, confirm absence." — purpose: Step 0 lock on whether to add or reuse.
- "FACT-confirm: is `PaintRegionEntry::paint_order` read anywhere besides the host-harvest enumeration index? Search `crates/slicer-sdk/`, `crates/slicer-host/`, `modules/core-modules/`. Return LOCATIONS." — purpose: Step 0 lock on whether to drop the field.
- "FACT-confirm: does `slicer_ir::ExPolygon` have public fields and `Clone`/`Debug` derives compatible with SDK re-export? Return the struct definition + derives." — purpose: Step 0 lock on `ExPolygonView` strategy.
- "Run `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd <named test>`; return FACT pass/fail or SNIPPETS (≤ 20 lines of failing assertion)." — purpose: per-step validation.
- "Run `cargo test -p slicer-host --test paint_region_transport_widening_tdd <named test>`; return FACT pass/fail or SNIPPETS." — purpose: per-step validation.
- "Run `./test-guests/build-test-guests.sh`; return FACT (success line + new size of `prepass-guest.component.wasm`)." — purpose: pre-built guest rebuild gate.
- "Find all references to `paint-region-entry` in the workspace (excluding the canonical and inline-WIT files); return LOCATIONS." — purpose: confirm no orphan call sites.
- "Find the precise insertion line in `docs/07_implementation_status.md` for TASK-130c (sibling of TASK-130a/130b at lines 68-70). Return file:line of TASK-130b plus the blocker-list line. Return SNIPPETS." — purpose: doc edit precision.
- "Update `docs/07_implementation_status.md` to insert TASK-130c row at line 71 and append `TASK-130c` to the blocker list at line 180. Return the diff (≤ 20 lines)." — purpose: precision doc edit via worker.

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

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 5 — host harvest typed mapping + dispatch_tdd.rs migration; lots of read-only Grep-and-confirm but bounded edits in one file)
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): the `wit/deps/ir-types.wit` "does it already declare paint-value-input" FACT (Step 0). Required return format: SNIPPETS ≤ 30 lines; if a SUMMARY of the whole file is returned, reject and re-dispatch. Second-riskiest: any `cargo test --workspace` accidental fire — explicitly forbidden during implementation iterations; the acceptance ceremony is the only place workspace test runs.

## Open Questions

- **None blocking activation.** The two design questions answered by user choice prior to packet generation:
  - Drain shape: Option (b), typed + hole-bearing.
  - WIT version: stay at `@1.0.0`.
  - Canonical guest migration: in this packet.
  - DEV log: extend DEV-025 with mismatches 4 + 5.

The remaining "Step 0" decisions are not blockers — they are FACT-confirmations the implementation plan locks before any non-trivial edit. None of them can change scope, interfaces, or verification strategy.
