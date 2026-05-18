# Design: macro-prepass-segmentation-output-drain

## Controlling Code Paths

- Primary code path: `crates/slicer-macros/src/lib.rs::build_prepass_world_glue` — the `"PrePass::PaintSegmentation"` arm at lines 1760-1788. After the trait call returns `Ok`, drain `sdk_output.regions()` and forward each entry via `_output.push_paint_region(&wit_entry)`. Mirror the existing `MeshSegmentation` arm drain at lib.rs:1733-1746.
- Adjacent code path (read-only): `crates/slicer-macros/src/lib.rs:1733-1746` — the canonical drain pattern this packet mirrors. Note the structure: trait call → loop over SDK output collection → push to WIT resource → ModuleError on Err → return Ok or `__slicer_error_out`.
- Test harness: `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs` — pre-built guest loading (`load_prepass_guest`), `make_compiled_module_with`, `Blackboard::new(Arc::new(mesh_ir), 0)`, `PrepassStageRunner::run_stage`. Reuse these helpers (or copy + adapt) for both new round-trip TDDs.
- Test harness fallback: `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — for `PaintRegionIR` shape assertions.
- Macro test guest: `test-guests/sdk-prepass-guest/` (existing per packet authoring scan: `sdk-prepass-guest.component.wasm` is 57,048 bytes). This is the macro-authored prepass guest. Step 0 confirms whether its current `run_paint_segmentation` exercises `push_paint_region` or whether it must be extended. Either way, it is the binary the new round-trip TDDs load.
- IR consumers (not edited; assertion targets only): `crates/slicer-ir/src/slice_ir.rs::PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `MeshSegmentationIR`.

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

## Code Change Surface

- Selected approach: mirror the MeshSegmentation drain inline in the PaintSegmentation arm. Extend the existing `sdk-prepass-guest` macro test guest (rather than fork) so its `run_paint_segmentation` and `run_mesh_segmentation` push the fixtures the new TDDs assert. Author two new TDDs that share helper functions copied from `macro_mesh_segmentation_geometry_tdd.rs`. Update the docs/07 TASK-130 cluster + DEV-025 to mark closure.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-macros/src/lib.rs::build_prepass_world_glue` (PaintSegmentation arm, lines ~1760-1788) — drain insertion + comment cleanup.
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (NEW)
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (NEW)
  - `test-guests/sdk-prepass-guest/src/lib.rs` (or whatever the macro-authored prepass guest source is; Step 0 confirms path) — extend `run_paint_segmentation` and `run_mesh_segmentation` to emit the fixtures the round-trip TDDs need.
  - `test-guests/sdk-prepass-guest.component.wasm` — rebuilt artifact.
  - `docs/07_implementation_status.md` — TASK-130, 130a, 130b checkboxes + blocker list.
  - `docs/DEVIATION_LOG.md` — DEV-025 mismatch 3 closure + status: closed.
  - `docs/14_deviation_audit_history.md` — DEV-025 row reference update.
- Rejected alternatives that were considered and why they were not chosen:
  - **Author a fresh macro test guest per round-trip TDD** (two guest crates, two .wasm). Rejected because `test-guests/sdk-prepass-guest/` already exists per the packet authoring scan, and a single guest can expose multiple fixtures via its config-key parsing pattern. Forking would duplicate scaffolding and compound the rebuild cost.
  - **Extract a generic drain helper across PaintSegmentation and MeshSegmentation arms.** Rejected because the SDK output collection types differ (`regions()` vs `triangle_paint_marks()`), the WIT push methods differ (`push_paint_region` vs `mark_triangle_paint`), and a generic helper would be a worse abstraction than two parallel ~10-line inline blocks. Match the MeshSegmentation arm's inline style.
  - **Add the round-trip tests to existing test files** (`macro_paint_region_roundtrip_tdd.rs`, `macro_mesh_segmentation_geometry_tdd.rs`). Rejected because the new tests are conceptually distinct: existing tests prove input/output **IR shape** is well-formed; the new tests prove the **macro-authored guest path** end-to-end. Separate files document the intent and isolate the failure mode.
  - **Bundle this packet's docs/07 + DEV-025 closure into Packet 42.** Rejected because Packet 42 closes mismatches 4 + 5 only; mismatch 3 stays open until the macro arm drain lands. Splitting closure across the two packets keeps the audit trail honest.

## Files in Scope (read + edit)

This packet's code change surface is small (~3 primary edits + 2 new test files + 1 guest extension). Each step touches at most 3 files.

- `crates/slicer-macros/src/lib.rs` — role: macro PrePass arm body; expected change: insert drain loop in PaintSegmentation arm, remove legacy rationalization comment.
- `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (NEW) — role: end-to-end PaintSegmentation round-trip TDD.
- `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (NEW) — role: end-to-end MeshSegmentation round-trip TDD.
- `test-guests/sdk-prepass-guest/src/lib.rs` (or equivalent; Step 0 confirms) — role: macro-authored guest emit; expected change: extend `run_paint_segmentation` to push hole-bearing + Custom fixtures, extend `run_mesh_segmentation` to push the symmetric MeshSegmentation marks.
- `test-guests/sdk-prepass-guest.component.wasm` — rebuilt artifact.
- `docs/07_implementation_status.md` — TASK-130 cluster checkboxes + blocker list.
- `docs/DEVIATION_LOG.md` — DEV-025 mismatch 3 closure + status: closed.
- `docs/14_deviation_audit_history.md` — DEV-025 row update.

## Read-Only Context

- `crates/slicer-macros/src/lib.rs:1733-1746` — read only this 14-line range. Purpose: the canonical drain pattern this packet mirrors. Do **not** read the rest of the file unless precisely targeted via Grep.
- `crates/slicer-macros/src/lib.rs:1297-1314` — inline-WIT block (post Packet 42 it carries `paint-value-input`). Purpose: confirm the WIT names the macro arm constructs (`PaintRegionEntry`, `PaintValueInput`, `ExPolygon`).
- `crates/slicer-sdk/src/prepass_builders.rs::PaintSegmentationOutput::regions()` — read only the accessor signature. Purpose: confirm the borrow shape of the loop iterator.
- `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs` — full file (≤ 500 lines per packet authoring scan). Purpose: harness patterns to reuse (`load_prepass_guest`, `make_compiled_module_with`, `Blackboard::new`, `PrepassStageRunner::run_stage`).
- `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — full file. Purpose: assertion patterns for `PaintRegionIR` shape.
- `crates/slicer-host/src/dispatch.rs:1954-2045` — read only `harvest_paint_segmentation_ir`. Purpose: confirm the assertion target (post Packet 42, the typed mapping).
- `crates/slicer-ir/src/slice_ir.rs` — read only the `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `PaintValue`, `PaintSemantic`, `MeshSegmentationIR` definitions. Purpose: confirm assertion shape.
- `.ralph/specs/06_macro-prepass-segmentation-bridge/packet.spec.md` — read the Goal + the deferred-work note. Do not load the rest of Packet 06.
- `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` — confirm `status: implemented` before activation.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- `vendor/` or equivalent vendored deps — never load.
- `crates/slicer-macros/src/lib.rs` outside lines 1700-1800 (and 1283-1314 for the inline-WIT post-42 confirmation) — never load. Use Grep to land precisely.
- `crates/slicer-host/src/wit_host.rs` — out of bounds entirely. The host validator was finalized in Packet 42; this packet does not modify it.
- `crates/slicer-host/src/dispatch.rs` outside `harvest_paint_segmentation_ir` (read-only) and the `dispatch_prepass_call` invocation site (read-only) — never load.
- `wit/world-prepass.wit`, `wit/deps/ir-types.wit` — out of bounds (Packet 42 finalized these).
- `modules/core-modules/paint-segmentation/` — entirely out of bounds. The canonical paint-segmentation module is unrelated to TASK-130's macro-authored bridge.
- The other 12 paint-related test files (per Packet 42's authoring scan) that are not the four named harness/assertion references above — auto-propagate; never load.
- `docs/07_implementation_status.md` outside the TASK-130 cluster (lines 65-72) and the blocker list (lines 175-185) — never load whole file.
- All other packets' files (other than Packets 06 and 42 named scopes) — never load.

## Expected Sub-Agent Dispatches

- "FACT-confirm: is `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` `status: implemented`? Return FACT." — purpose: activation gate.
- "FACT-confirm: under `test-guests/`, which directory contains the macro-authored prepass guest source (the one that produces `sdk-prepass-guest.component.wasm`)? Return LOCATIONS (path + brief contents listing)." — purpose: Step 0 lock on guest authoring location.
- "Show `crates/slicer-macros/src/lib.rs` lines 1700-1800; return SNIPPETS." — purpose: confirm line numbers post Packet 42's edits (Packet 42 only edited lines 1283-1314; the arm bodies should be unchanged but Step 0 verifies).
- "Show `crates/slicer-macros/src/lib.rs` lines 1283-1314; return SNIPPETS — the inline-WIT block post Packet 42." — purpose: confirm WIT type names (`PaintRegionEntry`, `PaintValueInput`, `ExPolygon`) the drain code will reference.
- "Run `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd <named test>`; return FACT pass/fail or SNIPPETS." — purpose: per-step validation.
- "Run `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd <named test>`; return FACT pass/fail or SNIPPETS." — purpose: per-step validation.
- "Run `./test-guests/build-test-guests.sh`; return FACT (success line + new size of `sdk-prepass-guest.component.wasm`)." — purpose: pre-built guest rebuild gate.
- "Update `docs/07_implementation_status.md` lines 68-70 to flip checkboxes for TASK-130, 130a, 130b. Update line ~180 to remove TASK-130a, TASK-130b from the blocker list. Return the diff (≤ 20 lines)." — purpose: precision doc edit via worker.
- "Update `docs/DEVIATION_LOG.md` DEV-025 entry to close mismatch 3 and set overall status to `closed`. Return the diff (≤ 20 lines)." — purpose: precision doc edit.
- "Update `docs/14_deviation_audit_history.md` DEV-025 row to reference TASK-130, 130a, 130b. Return the diff (≤ 20 lines)." — purpose: precision doc edit.

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

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 4 — author the PaintSegmentation round-trip TDD; multiple fixture cases + assertion logic).
- Highest-risk dispatch (whose return could blow budget if mis-shaped): the macro file lookup at lib.rs:1700-1800. Required return format: SNIPPETS (≤ 100 lines). If a SUMMARY of the whole file is returned, reject and re-dispatch. Second-riskiest: the `test-guests/` directory walk in Step 0 — required return format: LOCATIONS (≤ 10 entries with one-line context each).

## Open Questions

- **None blocking activation, conditional on Packet 42's `status: implemented`.** Step 0 verifies that condition. If Packet 42 is not yet `implemented` when this packet is reviewed for activation, the packet stays `draft` and the blocker is recorded.

The remaining "Step 0" decisions are not blockers — they are FACT-confirmations the implementation plan locks before any non-trivial edit:
- Which `test-guests/` subdir holds the macro-authored prepass guest source.
- Whether the existing guest's `run_paint_segmentation` already exercises `push_paint_region` (extend the call or add it).
- Whether macro `lib.rs` line numbers shifted post Packet 42 (Packet 42 only edited lines 1283-1314; arm bodies should be unchanged but Step 0 verifies).
