---
status: superseded
superseded_by: 43-rev1_macro-prepass-segmentation-output-drain
packet: macro-prepass-segmentation-output-drain
task_ids:
  - TASK-130
  - TASK-130a
  - TASK-130b
---

# 43_macro-prepass-segmentation-output-drain

## Goal

Close `TASK-130`, `TASK-130a`, and `TASK-130b` (DEV-025 mismatch 3) by draining the SDK `PaintSegmentationOutput` builder back through the WIT `paint-segmentation-output::push-paint-region` resource from the `#[slicer_module]` macro's `PrePass::PaintSegmentation` arm — mirroring the `MeshSegmentation` arm's existing drain pattern at `crates/slicer-macros/src/lib.rs:1733-1746`. Add two end-to-end macro-path round-trip TDDs that load a macro-authored guest WASM and assert the harvested `PaintRegionIR` / `MeshSegmentationIR` faithfully reflects what the macro-authored module emitted, including hole-bearing polygons and Custom-semantic structured values (made possible by Packet 42's transport widening). Update `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, and `docs/14_deviation_audit_history.md` to reflect TASK-130/130a/130b completion and DEV-025 final closure (assuming Packet 42's mismatches 4 + 5 closure has landed).

## Problem Statement

`DEV-025` ("Prepass segmentation SDK↔WIT shapes are still misaligned") is being closed in two packets. Packet 42 (`paint-region-transport-widening`) addresses mismatches 4 (paint value channel string-coerced) and 5 (SDK paint-region polygons hole-blind). This packet (Packet 43) addresses **mismatch 3** — the macro `PrePass::PaintSegmentation` arm in `crates/slicer-macros/src/lib.rs:1760-1788` does not drain `PaintSegmentationOutput::regions()` back through the WIT `paint-segmentation-output::push-paint-region` resource. The arm currently lifecycles the SDK module and calls `run_paint_segmentation`, but discards the builder's output. Macro-authored prepass modules can therefore call `sdk_output.push_paint_region(...)` and have nothing happen.

Packet 06 (`06_macro-prepass-segmentation-bridge`) closed DEV-025 mismatches 1 and 2 and laid the macro-arm scaffolding (input bridges, host harvest, `PaintRegionIR` structure). It explicitly DEFERRED mismatch 3 because the SDK and WIT shapes were not aligned: the SDK's `PaintRegionEntry` carried `contour_points: Vec<[f64; 2]>` (no holes) and `value: PaintValueView` (typed), while the WIT carried `polygons: list<ex-polygon>` (with holes) and `value: string` (lossy). Packet 42 corrects that misalignment. With Packet 42 implemented, the drain is a near-mechanical mirror of the existing `MeshSegmentation` arm drain at lib.rs:1733-1746.

This packet also adds the **end-to-end macro-path round-trip tests** that TASK-130b demands. Two new TDDs (`macro_paint_segmentation_output_roundtrip_tdd.rs` and `macro_mesh_segmentation_output_roundtrip_tdd.rs`) prove that a macro-authored guest WASM round-trips paint regions and mesh segmentation marks through the WIT contract end-to-end, including the hole-bearing polygon and Custom-semantic Custom-value cases (which were architecturally impossible before Packet 42).

This packet does not reopen any prior packet. It completes the work Packet 06 deferred.

## Architecture Constraints

- **Mirror the MeshSegmentation drain shape exactly.** The MeshSegmentation arm's drain pattern is the "canonical post-Ok-trait-return drain" template. Deviating from it would create churn for future drain authors. The structure:
  ```
  let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_paint_segmentation(...);
  for entry in sdk_output.regions() {
      let wit_entry = /* construct WIT paint-region-entry from SDK PaintRegionEntry */;
      if let Err(e) = _output.push_paint_region(&wit_entry) {
          return Err(ModuleError { code: 10, message: e, fatal: true });
      }
  }
  match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
  ```
- **Packet 42 has aligned the SDK and WIT shapes.** Post Packet 42, `sdk_output.regions()` returns `&[PaintRegionEntry]` where each entry's `polygons: Vec<ExPolygonView>` and `value: PaintValueView` map 1:1 to the WIT `paint-region-entry { polygons: list<ex-polygon>, value: paint-value-input }`. The drain is now a near-mechanical conversion: clone the SDK polygons into WIT `ExPolygon`s (or use the bindgen's borrow path if compatible), map `PaintValueView` to the WIT `paint-value-input` variant case-by-case, copy `object_id` / `layer_index` / `semantic` directly.
- **No new helper functions in slicer-macros.** Inline the conversion within the arm to keep the macro's emitted code self-contained, matching the MeshSegmentation arm's inline style. If the conversion grows beyond ~25 lines, extract it as a `#[doc(hidden)]` helper at the `__slicer_*` prefix convention used elsewhere in the file (see `__slicer_paint_segmentation_object_from_wit` at lib.rs:1509 for the naming pattern).
- **Determinism.** `PaintSegmentationOutput::regions()` returns entries in push order. The macro drain iterates in this order. The host harvest (post Packet 42) assigns `paint_order` from enumeration index. Therefore the macro path produces a deterministic ordering equivalent to a hand-written `wit-guest`.
- **Pre-built test-guest dependency.** The new round-trip TDDs load `test-guests/sdk-prepass-guest.component.wasm`. The guest's `run_paint_segmentation` must be authored to push the fixtures the tests assert against. If a single guest exposes multiple fixtures (driven by config keys, like the canonical `paint-segmentation` does), one guest .wasm covers both round-trip TDDs. Otherwise two separate guests are needed; Step 0 picks.

## Data and Contract Notes

- IR or manifest contracts touched: none modified. Assertion targets only: `PaintRegionIR.per_layer[layer_index].semantic_regions[PaintSemantic][SemanticRegion]` for the paint round-trip; `MeshSegmentationIR` (object → marks) for the mesh round-trip.
- WIT boundary considerations: post Packet 42, `paint-region-entry { object-id, layer-index, semantic, polygons: list<ex-polygon>, value: paint-value-input }`. The macro arm constructs this record from SDK types and pushes via `paint-segmentation-output::push-paint-region`. No new WIT.
- Determinism or scheduler constraints:
  - Drain order = SDK push order (`PaintSegmentationOutput::regions()` returns push order).
  - Host harvest assigns `paint_order` from enumeration index, so a macro path round-trip produces the same `paint_order` sequence as a hand-written `wit-guest` does.
  - The macro arm's drain happens inside the same dispatch turn as the trait call; no scheduler-visible state change.

## Locked Assumptions and Invariants

- **Packet 42 is `implemented` before this packet activates.** Step 0 verifies. If Packet 42 is still `draft` or `active`, this packet stays `draft`.
- **The MeshSegmentation drain at lib.rs:1733-1746 is the canonical pattern.** Any deviation in this packet's PaintSegmentation drain must be justified by a structural difference (e.g., `regions()` vs `triangle_paint_marks()` accessor names, `push_paint_region` vs `mark_triangle_paint` method names, the typed `paint-value-input` variant construction). Stylistic deviation is not allowed.
- **The macro test guest under `test-guests/sdk-prepass-guest/` is the round-trip fixture vehicle.** No new test guest is authored unless Step 0 finds the existing guest cannot be extended.
- **No production code outside the macro arm is modified.** The host validator, host harvest, SDK types, WIT, and IR are all post-Packet-42 and stay put.

## Risks and Tradeoffs

- **Risk: macro arm borrow lifetimes for `_output` differ from MeshSegmentation's `_output` because the resource is consumed by the trait call by mutable borrow.** Mitigation: the MeshSegmentation arm is the live precedent; copy its pattern verbatim. If a borrow-checker error surfaces, dispatch a sub-agent for the exact error + the matching helper from `__slicer_*` prefix convention; do not attempt to redesign the arm structure.
- **Risk: the macro test guest's bindgen output for the new `paint-value-input` variant uses Rust names that conflict with the trait's `PaintValueView` (SDK side).** Mitigation: in the guest, use the bindgen-generated `PaintValueInput` directly; do not import `slicer_sdk::prepass_types::PaintValueView` in the guest crate. If the guest currently imports SDK types, refactor at Step 3 to use bindgen types only.
- **Risk: `test-guests/build-test-guests.sh` requires a wasm32 toolchain not available locally.** Mitigation: same as Packet 42 Step 0 — Step 0 records toolchain status and either runs locally or delegates to CI.
- **Risk: an existing test that currently relies on `sdk-prepass-guest.component.wasm`'s pre-Packet-42 emit shape breaks when the guest is extended.** Mitigation: Step 0 enumerates every test that loads `sdk-prepass-guest.component.wasm` (Grep for the exact path); Step 6's regression sweep runs all of them.
- **Tradeoff: extending the existing macro test guest vs forking.** Chose extending for scaffolding economy; the cost is that the guest now serves two test files. Acceptable because both files exercise the same guest fixtures from different angles.
