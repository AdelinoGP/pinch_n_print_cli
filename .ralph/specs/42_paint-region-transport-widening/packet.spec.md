---
status: implemented
packet: paint-region-transport-widening
task_ids:
  - TASK-130c
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: paint-region-transport-widening

## Goal

Widen the paint-region transport between the SDK builder, the WIT `paint-segmentation-output::push-paint-region` resource, and the host harvest into `PaintRegionIR` so paint regions carry **(a) full `ExPolygon` polygons with holes** (matching OrcaSlicer's `ExPolygons[layer][extruder]` shape and the IR's existing `SemanticRegion::polygons: Vec<ExPolygon>` field) and **(b) typed `PaintValue` variants** (replacing the lossy `value: string` channel that the host currently re-parses with a four-grammar guesser falling back to `ToolIndex(0)`). Migrate the canonical `paint-segmentation` core module's `wit-guest/` emit code to the new shape, and migrate the one direct-wiring test (`dispatch_tdd.rs::paint_segmentation_output_rejects_invalid_entries`). Result: the paint-region transport stops silently corrupting hole-bearing regions and stops silently coercing `Custom`-semantic / non-numeric `PaintValue`s to `ToolIndex(0)`. This packet is the architectural prerequisite for Packet 43, which will then drain `PaintSegmentationOutput` from the macro-authored prepass arm.

## Scope Boundaries

- In scope:
  - **SDK widening** (`crates/slicer-sdk/src/prepass_builders.rs`):
    - Replace `PaintRegionEntry::contour_points: Vec<[f64; 2]>` with `polygons: Vec<ExPolygonView>` where `ExPolygonView { contour: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>> }`.
    - Replace `PaintSegmentationOutput::push_paint_region` signature so it accepts `polygons: Vec<ExPolygonView>` instead of `contour_points: Vec<[f64; 2]>`. Drop the `paint_order` parameter (the host harvest derives it from enumeration order — see `dispatch.rs:2024 idx as u64`); confirm at Step 0 whether the field is read anywhere else; if not, also drop `paint_order` from `PaintRegionEntry`.
    - Re-export `ExPolygonView` (or use `slicer_ir::ExPolygon` directly — Step 0 selects).
  - **WIT widening** (`wit/world-prepass.wit` + `wit/deps/ir-types.wit` + the inline-WIT mirror in `crates/slicer-macros/src/lib.rs:1295-1306`):
    - Add a new `paint-value-input` variant: `variant paint-value-input { flag(bool), scalar(f32), tool-index(u32), custom(string) }` (or extend `wit/deps/ir-types.wit`'s existing `paint-value-view` if it is exactly this shape — Step 0 confirms).
    - Replace `paint-region-entry.value: string` with `value: paint-value-input`.
    - Reconcile the inline-WIT/canonical drift on `paint-region-entry.layer-index` (inline declares `u32`; canonical declares `layer-idx`/`s32`). Step 0 picks one; both files end up identical.
  - **Host widening** (`crates/slicer-host/src/wit_host.rs` + `crates/slicer-host/src/dispatch.rs`):
    - `HostExecutionContext::paint_region_entries` field type updates with the WIT bindgen.
    - `HostPaintSegmentationOutput::push_paint_region` validation handles the new variant + polygon list shape.
    - `harvest_paint_segmentation_ir` (dispatch.rs:1954-2045) drops the `parse_value` closure (line 1975-1985) and the `parse_semantic` closure stays (semantic side is already typed correctly). The function becomes a near-trivial conversion of typed WIT entries to `SemanticRegion`.
  - **Canonical guest migration** (`modules/core-modules/paint-segmentation/wit-guest/src/lib.rs`):
    - Update `run_paint_segmentation` to construct `paint-region-entry` with the new variant `value` and a `polygons: Vec<ExPolygon>` (currently constructs `polygons: vec![ExPolygon { contour, holes: Vec::new() }]` — i.e., it already wraps in the right shell, only the `value` serialization changes).
  - **Test migration** (`crates/slicer-host/tests/dispatch_tdd.rs`):
    - Migrate `paint_segmentation_output_rejects_invalid_entries` (lines 5349-5441) — the only test that constructs `pm::PaintRegionEntry` directly and asserts on the WIT-side struct. Replace `value: "1".to_string()` with `value: PaintValueInput::ToolIndex(1)` (or equivalent), and replace single-`Polygon`-as-vec values with `vec![ExPolygon { contour, holes: vec![] }]`.
  - **Pre-built test-guest rebuild**:
    - Run `test-guests/build-test-guests.sh` to rebuild `prepass-guest.component.wasm` so subsequent macro-arm tests in Packet 43 link against the new WIT shape. Confirm `test-guests/prepass-guest.component.wasm` size changes and that the existing `macro_paint_region_roundtrip_tdd` (which uses the canonical guest, not a macro guest) still passes against the new shape.
  - **Inline-WIT update only** in `crates/slicer-macros/src/lib.rs` (the WIT block at lines 1283-1314). The macro PaintSegmentation arm itself (lines 1760-1788) is **not modified in this packet** — that's Packet 43.
  - **Backlog + deviation registration**:
    - Add `TASK-130c` row in `docs/07_implementation_status.md` near the existing 130/130a/130b cluster: "Widen paint-region transport (SDK ExPolygon-bearing, WIT typed paint-value-input variant, host harvest 1:1) so paint regions carry hole-bearing polygons and Custom/typed values without coercion. Covers DEV-025."
    - Add `TASK-130c` to the blocking-tasks list at `docs/07_implementation_status.md:180` (the Architecture Acceptance Gate blocker).
    - Extend `docs/DEVIATION_LOG.md` DEV-025 with mismatches **4 (paint value channel string-coerced)** and **5 (SDK paint-region polygons hole-blind)**. Mark 4 + 5 closed by this packet at acceptance.
    - Update `docs/14_deviation_audit_history.md` DEV-025 row to reference TASK-130c closure of mismatches 4 + 5.
- Out of scope:
  - Macro `PrePass::PaintSegmentation` arm drain in `crates/slicer-macros/src/lib.rs:1770-1788`. Closed in Packet 43.
  - Macro `PrePass::MeshSegmentation` arm. Already drained at lib.rs:1733-1746; not touched by this packet.
  - Round-trip TDDs through the macro path. Authored in Packet 43.
  - Host-side string-keyed `PaintSemantic::Custom(String)` parsing. The semantic side is already typed correctly through `parse_semantic` (dispatch.rs:1961) — Custom round-trips. No change needed.
  - WIT world-prepass version bump. Per packet metadata decision: stay at `slicer:world-prepass@1.0.0` since DEV-025 is openly registered and the contract is under active remediation (see Locked Assumptions in `design.md`).
  - WIT_WORLD_ALLOWLIST changes in `crates/slicer-host/src/manifest.rs:705-729`. None needed because the world version is unchanged.
  - Module manifest `wit_world` field updates for the 5 prepass-world modules. None needed for the same reason.
  - The MeshSegmentation transport (`mesh-segmentation-output::mark-triangle-paint`). It is already symmetric and string-typed only for `semantic` and `value`, and a parallel widening can be its own future packet if motivated. Not coupled to TASK-130 closure.
  - Any change to `slicer-ir` (`PaintValue`, `PaintSemantic`, `ExPolygon`, `SemanticRegion`, `PaintRegionIR`). The IR is already correctly shaped; this packet aligns the SDK + WIT to it.
  - Any change to gcode-emit. The earlier scan confirmed gcode_emit does not read `PaintValue`.
  - Snapshot / golden file updates. None exist for paint regions.

## Prerequisites and Blockers

- Depends on:
  - Packet `06_macro-prepass-segmentation-bridge` — `implemented`. The host converters `object_mesh_to_wit_paint_segmentation_view` and the `paint_region_entries` collection point already exist and continue to work; this packet only changes the value/polygon types those entries carry.
- Unblocks:
  - Packet `43_macro-prepass-segmentation-output-drain` — drains `sdk_output.regions()` through the now-typed `push-paint-region` from the macro `PrePass::PaintSegmentation` arm. Cannot land before this packet because the SDK builder regions cannot be losslessly serialized through `value: string`.
- Activation blockers:
  - Step 0 must FACT-confirm: (a) whether `wit/deps/ir-types.wit` already declares a `paint-value-input` (or equivalently shaped) variant or whether this packet must add one; (b) whether `PaintRegionEntry::paint_order` is read anywhere besides the host-harvest enumeration index; (c) whether `slicer-ir::ExPolygon` can be re-exported in the SDK or whether a new `ExPolygonView` is required (mirrors the existing `MeshObjectView` / `PaintSegmentationObjectView` view-type convention).
  - Step 0 must locate the exact insertion line in `docs/07_implementation_status.md` for TASK-130c (sibling row to 130a/130b at lines 68-70).
  - Step 0 must confirm `test-guests/build-test-guests.sh` is runnable on the host's Windows toolchain via WSL or has a Windows-equivalent path. If not runnable locally, Step 0 records that the rebuild is delegated to a sub-agent / CI.

## Acceptance Criteria

- **Given** the SDK `PaintRegionEntry` struct definition in `crates/slicer-sdk/src/prepass_builders.rs`, **when** the file is grepped for `contour_points`, **then** zero matches are found AND the struct declares a field `polygons: Vec<ExPolygonView>` where `ExPolygonView` is defined in the same crate with `contour: Vec<[f64; 2]>` and `holes: Vec<Vec<[f64; 2]>>` fields. | `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd sdk_paint_region_entry_carries_expolygon_view -- --exact --nocapture`
- **Given** an SDK `PaintSegmentationOutput` builder, **when** a caller invokes `builder.push_paint_region(layer_index=0, semantic="material".into(), object_id="o1".into(), value=PaintValueView::tool_index(2), polygons=vec![ExPolygonView { contour: <triangle>, holes: vec![<inner triangle>] }])`, **then** `builder.regions()[0].polygons[0].holes.len() == 1` AND `regions()[0].value` round-trips a typed `tool_index(2)` (no string coercion). | `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd sdk_push_paint_region_preserves_holes_and_typed_value -- --exact --nocapture`
- **Given** the WIT file `wit/world-prepass.wit`, **when** the file is grepped for the `paint-region-entry` record, **then** the `value` field's type is the literal token `paint-value-input` (a variant) AND the variant declares the four cases `flag(bool)`, `scalar(f32)`, `tool-index(u32)`, `custom(string)` AND the field `value: string` is no longer present in the record. | `cargo test -p slicer-host --test paint_region_transport_widening_tdd wit_paint_region_entry_value_is_typed_variant -- --exact --nocapture`
- **Given** the host harvest function `harvest_paint_segmentation_ir` in `crates/slicer-host/src/dispatch.rs`, **when** the file is grepped within that function for the literal token `parse_value` or `parse::<u32>()` or `parse::<f32>()`, **then** zero matches are found AND the function maps `paint-value-input::tool-index(n)` to `PaintValue::ToolIndex(n)`, `flag(b)` to `PaintValue::Flag(b)`, `scalar(f)` to `PaintValue::Scalar(f)`, and `custom(s)` to a deterministic IR representation (Step 0 selects: either `PaintValue` gains a `Custom(String)` variant in this packet, OR the harvest preserves Custom via the `PaintSemantic::Custom` channel + a fallback enum case — the implementation step records the chosen mapping as a doc comment in dispatch.rs). | `cargo test -p slicer-host --test paint_region_transport_widening_tdd host_harvest_drops_string_parsing -- --exact --nocapture`
- **Given** a guest module that pushes `paint-region-entry { semantic: "material", polygons: [ExPolygon { contour: <square>, holes: [<inner square>] }], value: tool-index(7), object_id: "obj-a", layer_index: 3 }` via `push-paint-region`, **when** the host runs `harvest_paint_segmentation_ir`, **then** the resulting `PaintRegionIR.per_layer[3].semantic_regions[PaintSemantic::Material][0]` has `polygons.len() == 1` AND `polygons[0].holes.len() == 1` (hole geometry preserved end-to-end) AND `value == PaintValue::ToolIndex(7)` (no string coercion) AND `object_id == "obj-a"`. **This is the substantive transport-fidelity validation that DEV-025 mismatches 4 and 5 demanded.** | `cargo test -p slicer-host --test paint_region_transport_widening_tdd hole_bearing_region_round_trips_through_typed_value -- --exact --nocapture`
- **Given** the canonical `paint-segmentation` core module's `wit-guest/src/lib.rs`, **when** the workspace builds and the module's existing acceptance test suite runs (`cargo test -p paint-segmentation -- --nocapture`), **then** every existing test PASSES against the new shape AND the file no longer contains the literal substring `value: entry.value.clone()` (which was the `String`-typed assignment). | `cargo test -p paint-segmentation -- --nocapture`
- **Given** the migrated `dispatch_tdd.rs::paint_segmentation_output_rejects_invalid_entries`, **when** the test runs after this packet's refactor, **then** it PASSES with the typed `value: PaintValueInput::ToolIndex(_)` fixture and asserts on `polygons` (not `contour_points`). | `cargo test -p slicer-host --test dispatch_tdd paint_segmentation_output_rejects_invalid_entries -- --exact --nocapture`
- **Given** the pre-built `test-guests/prepass-guest.component.wasm` after `build-test-guests.sh` is re-run, **when** the existing `macro_paint_region_roundtrip_tdd` runs (this test exercises the IR shape, not the macro arm), **then** it PASSES against the new typed transport. | `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd -- --nocapture`
- **Given** `docs/07_implementation_status.md` after this packet, **when** the file is read, **then** TASK-130c appears as a sibling row to TASK-130a/130b AND TASK-130c appears in the Architecture Acceptance Gate blocker list at line ~180. | `cargo test -p slicer-host --test paint_region_transport_widening_tdd docs_07_registers_task_130c -- --exact --nocapture`
- **Given** `docs/DEVIATION_LOG.md` after this packet, **when** the DEV-025 entry is read, **then** mismatches 4 (paint value channel string-coerced) and 5 (SDK paint-region polygons hole-blind) are present AND both are marked closed-by-Packet-42 with the closure date AND mismatch 3 remains open (closes in Packet 43). | `cargo test -p slicer-host --test paint_region_transport_widening_tdd dev_log_extends_dev025_with_4_and_5 -- --exact --nocapture`

## Negative Test Cases

- **Given** a guest module that pushes `paint-region-entry { semantic: "fuzzy_skin", polygons: [<ring with one hole>], value: flag(true) }`, **when** the host harvests, **then** the resulting `SemanticRegion.polygons[0].holes.len() == 1` (the hole survives) AND a parallel old-shape fixture with `polygons: [<ring with no holes>]` would have produced a *different* `SemanticRegion` (proves the hole-fidelity is real, not vestigial). | `cargo test -p slicer-host --test paint_region_transport_widening_tdd fuzzy_skin_ring_with_hole_preserves_hole -- --exact --nocapture`
- **Given** a guest module that pushes `paint-region-entry { semantic: "custom:my_temp", value: custom("profile_high") }`, **when** the host harvests, **then** the resulting `SemanticRegion.value` is **not** `PaintValue::ToolIndex(0)` (the silent-fallback failure mode of the old string parser) AND the Custom payload `"profile_high"` is preserved verbatim in the chosen IR representation (Step 0 locks the mapping). | `cargo test -p slicer-host --test paint_region_transport_widening_tdd custom_value_does_not_coerce_to_tool_index_zero -- --exact --nocapture`
- **Given** the SDK `PaintSegmentationOutput::push_paint_region` source, **when** the file is grepped for the parameter name `contour_points`, **then** zero matches are found (the old single-contour API is genuinely removed, not deprecated-and-still-callable). | `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd contour_points_api_is_fully_removed -- --exact --nocapture`
- **Given** the WIT files `wit/world-prepass.wit` and the inline-WIT block in `crates/slicer-macros/src/lib.rs`, **when** both `paint-region-entry` records are extracted and compared, **then** they are byte-identical modulo whitespace (the inline-WIT/canonical drift on `layer-index` field type is reconciled to one form). | `cargo test -p slicer-host --test paint_region_transport_widening_tdd inline_and_canonical_wit_match -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./test-guests/build-test-guests.sh` (rebuild `prepass-guest.component.wasm` against new WIT shape)
- `cargo test -p slicer-sdk --test paint_region_transport_widening_tdd -- --nocapture`
- `cargo test -p slicer-host --test paint_region_transport_widening_tdd -- --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd paint_segmentation_output_rejects_invalid_entries -- --exact --nocapture`
- `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd -- --nocapture`
- `cargo test -p paint-segmentation -- --nocapture`
- `cargo test -p slicer-host --test paint_segmentation_executor_tdd -- --nocapture`
- `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd -- --nocapture`
- `cargo test -p slicer-host --test paint_annotation_integration_tdd -- --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintRegionIR`, `SemanticRegion`, `LayerPaintMap`, `PaintValue`, `PaintSemantic`, `ExPolygon`. Direct read; narrow line ranges only — these structs already define the target shape.
- `docs/03_wit_and_manifest.md` — WIT version-bumping rule and host-boundary contract. Delegate SUMMARY ≤ 200 words for the version-bumping rule (locked assumption: this packet stays at `@1.0.0` despite a non-additive change; the rationale is recorded in `design.md`'s Locked Assumptions).
- `docs/05_module_sdk.md` — `PaintSegmentationOutput` builder API description. Delegate SUMMARY for the section that names `push_paint_region`'s signature; this packet replaces it.
- `docs/14_deviation_audit_history.md` — DEV-025 audit row to be extended with mismatches 4 + 5. Direct read; narrow.
- `docs/DEVIATION_LOG.md` — DEV-025 entry to be extended. Direct read; narrow.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md` lines 211–225 — `ExPolygon` data type definition (Clipper convention: CCW outer contour, CW inner holes). This is the parity anchor for "paint regions are ExPolygon-with-holes."
- `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — facet-painted segmentation produces `ExPolygons[layer][extruder]`; cite as evidence that real OrcaSlicer paint regions routinely have holes (e.g., a Material ring around an unpainted core).
- `OrcaSlicerDocumented/generated_documentation/01_system_architecture.md` lines 73–98 — slicing pipeline output of `std::vector<ExPolygons>` consumed by MMU segmentation.

All OrcaSlicer reads MUST be delegated; never load this tree into the implementer's own context.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was authored under the spec-packet-generator's context_discipline preamble. Downstream agents must:

- Treat `design.md`'s code change surface as authoritative; touch nothing outside it.
- Honor `design.md`'s out-of-bounds list (no IR shape changes; no macro-arm changes; no module manifest version bumps; no WIT_WORLD_ALLOWLIST changes; no MeshSegmentation transport changes).
- Delegate every cargo run, every workspace search, and every authoritative-doc fact-check.
- Stop reading at 60% context; hand off at 85%.

This is a **transport-shape-widening + canonical-guest-migration** packet. The biggest implementation risks are (a) the `PaintValue::Custom` mapping decision (Step 0 is the critical gate — either the IR enum gets a `Custom` variant or the harvest carries Custom in the `PaintSemantic` channel) and (b) the `prepass-guest.component.wasm` rebuild step needing a Windows-toolchain workaround. AC-5 (`hole_bearing_region_round_trips_through_typed_value`) is the substantive validation — without it, the packet ships a contract that nobody proves works end-to-end.
