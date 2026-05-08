# Requirements: macro-prepass-segmentation-output-drain

## Packet Metadata

- Grouped task IDs:
  - `TASK-130` — Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules.
  - `TASK-130a` — Drain `PaintSegmentationOutput` back through WIT `push-paint-region` so macro-authored modules can emit paint regions without hand-written `wit-guest` glue.
  - `TASK-130b` — Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT.
- Backlog source: `docs/07_implementation_status.md` lines 68-70 (TASK-130 cluster) + line ~180 (Architecture Acceptance Gate blocker list).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`DEV-025` ("Prepass segmentation SDK↔WIT shapes are still misaligned") is being closed in two packets. Packet 42 (`paint-region-transport-widening`) addresses mismatches 4 (paint value channel string-coerced) and 5 (SDK paint-region polygons hole-blind). This packet (Packet 43) addresses **mismatch 3** — the macro `PrePass::PaintSegmentation` arm in `crates/slicer-macros/src/lib.rs:1760-1788` does not drain `PaintSegmentationOutput::regions()` back through the WIT `paint-segmentation-output::push-paint-region` resource. The arm currently lifecycles the SDK module and calls `run_paint_segmentation`, but discards the builder's output. Macro-authored prepass modules can therefore call `sdk_output.push_paint_region(...)` and have nothing happen.

Packet 06 (`06_macro-prepass-segmentation-bridge`) closed DEV-025 mismatches 1 and 2 and laid the macro-arm scaffolding (input bridges, host harvest, `PaintRegionIR` structure). It explicitly DEFERRED mismatch 3 because the SDK and WIT shapes were not aligned: the SDK's `PaintRegionEntry` carried `contour_points: Vec<[f64; 2]>` (no holes) and `value: PaintValueView` (typed), while the WIT carried `polygons: list<ex-polygon>` (with holes) and `value: string` (lossy). Packet 42 corrects that misalignment. With Packet 42 implemented, the drain is a near-mechanical mirror of the existing `MeshSegmentation` arm drain at lib.rs:1733-1746.

This packet also adds the **end-to-end macro-path round-trip tests** that TASK-130b demands. Two new TDDs (`macro_paint_segmentation_output_roundtrip_tdd.rs` and `macro_mesh_segmentation_output_roundtrip_tdd.rs`) prove that a macro-authored guest WASM round-trips paint regions and mesh segmentation marks through the WIT contract end-to-end, including the hole-bearing polygon and Custom-semantic Custom-value cases (which were architecturally impossible before Packet 42).

This packet does not reopen any prior packet. It completes the work Packet 06 deferred.

## In Scope

- Edit the `"PrePass::PaintSegmentation"` arm in `crates/slicer-macros/src/lib.rs::build_prepass_world_glue` (~lines 1760-1788) to add a drain loop iterating `sdk_output.regions()` and forwarding each entry via `_output.push_paint_region(&wit_entry)`. Mirror the MeshSegmentation arm's `ModuleError { code: 10, fatal: true }` shape on push failure. Replace or remove the legacy comment block at lines 1760-1769 (which currently rationalizes the missing drain).
- Author `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` with at least the named tests in `packet.spec.md` Acceptance Criteria + Negative Test Cases.
- Author `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` with at least the named test in `packet.spec.md` AC-4.
- Author or extend a macro-authored prepass test guest under `test-guests/sdk-prepass-guest/` whose `run_paint_segmentation` and `run_mesh_segmentation` exercise the `sdk_output.push_paint_region` and `sdk_output.mark_triangle_paint` paths with hole-bearing polygons + Custom-semantic-and-value fixtures. Reuse the existing `sdk-prepass-guest.component.wasm` if it can be extended; fork only if not.
- Run `test-guests/build-test-guests.sh` to rebuild the affected guest .wasm.
- Update `docs/07_implementation_status.md`:
  - Flip TASK-130 from `[~]` to `[x]`, TASK-130a from `[ ]` to `[x]`, TASK-130b from `[ ]` to `[x]`.
  - Remove TASK-130a and TASK-130b from the Architecture Acceptance Gate blocker list at line ~180. (TASK-130c was registered + removed by Packet 42.)
- Update `docs/DEVIATION_LOG.md`:
  - Mark DEV-025 mismatch 3 closed-by-Packet-43 with the closure date.
  - Confirm mismatches 1 + 2 already closed-by-Packet-06 (TASK-128a/128b).
  - Confirm mismatches 4 + 5 already closed-by-Packet-42 (TASK-130c).
  - Set DEV-025 overall status to `closed`.
- Update `docs/14_deviation_audit_history.md` DEV-025 row to reference TASK-130, TASK-130a, TASK-130b closure.

## Out of Scope

- SDK API widening, WIT shape changes, host harvest changes — Packet 42's territory.
- The `MeshSegmentation` arm production drain at lib.rs:1733-1746. This packet only ADDS a round-trip test; the production drain is unmodified.
- Any other prepass arm: `Slicing`, `LayerPlanning`, `MeshAnalysis`, `SeamPlanning`, `SupportPlanning`.
- Module manifest edits.
- WIT version bumps.
- Any change to IR schemas, host scheduler, or progress events.
- New helper functions or test harness scaffolding in `crates/slicer-host/tests/` beyond what the two new round-trip TDDs need. Reuse helpers from `macro_mesh_segmentation_geometry_tdd.rs` and `macro_paint_region_roundtrip_tdd.rs` (loaders, fixture builders, dispatch invocation).
- Forks of the canonical `paint-segmentation` core module — that is a non-macro module and is unrelated to TASK-130's macro-authored bridge.

## Authoritative Docs

- `docs/05_module_sdk.md` — Delegate SUMMARY ≤ 200 words for the `run_paint_segmentation` / `PaintSegmentationOutput` section.
- `docs/02_ir_schemas.md` — Direct read; narrow line ranges only. The `PaintRegionIR` and `MeshSegmentationIR` definitions are the round-trip assertion targets.
- `docs/03_wit_and_manifest.md` — Delegate SUMMARY ≤ 150 words for prepass world surface.
- `docs/14_deviation_audit_history.md` — Direct read; narrow (DEV-025 row).
- `docs/DEVIATION_LOG.md` — Direct read; narrow (DEV-025 entry).
- `.ralph/specs/06_macro-prepass-segmentation-bridge/packet.spec.md` — predecessor; direct read for the deferred-work note. **Do not load `06`'s `requirements.md`/`design.md`/`implementation-plan.md`/`task-map.md`**; they are out of scope.
- `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` — co-prerequisite; FACT-confirm `status: implemented` before activation.

## OrcaSlicer Reference Obligations

- None directly required. Parity is established by Packet 42. This packet wires the macro path through the corrected transport.
- If a parity question arises while authoring round-trip fixtures, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` for ExPolygon shape context only.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases: see `packet.spec.md` Acceptance Criteria — drain exists in macro arm (AC-1), hole-bearing typed value round-trips (AC-2, the substantive proof), Custom semantic + Custom value round-trip (AC-3, fulfills the original feature purpose), MeshSegmentation marks round-trip (AC-4), push-failure surfaces fatally (AC-5), legacy rationalization comment removed (AC-6), docs/07 marked done (AC-7), DEV-025 fully closed (AC-8), audit history complete (AC-9).
- Negative cases: empty-polygons rejection at host validator surfaces fatally; no early-return bypass of the drain.
- Measurable outcomes:
  - `crates/slicer-macros/src/lib.rs` PaintSegmentation arm contains at least one `sdk_output.regions()` iteration AND at least one `_output.push_paint_region` call AND a `ModuleError { code: 10, fatal: true }` on error.
  - The legacy rationalization comment ("Same disconnect as MeshSegmentation" / "the SDK PaintSegmentationOutput builder operates on an in-Rust tree") no longer appears in the file.
  - Two new round-trip TDDs exist with the named tests, all GREEN.
  - `docs/07` TASK-130 cluster marked done; blocker list shrunk by two entries.
  - DEV-025 overall status `closed`.
- Cross-packet impact: this packet **closes** DEV-025 (assuming Packets 06 and 42 have already closed mismatches 1, 2, 4, 5). It removes TASK-130a and TASK-130b from the Architecture Acceptance Gate blocker list.

## Verification Commands

- `cargo build --workspace`
- `./test-guests/build-test-guests.sh` (FACT: success line + size delta of any rebuilt guest .wasm)
- `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd -- --nocapture` (FACT pass/fail)
- `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd -- --nocapture` (FACT pass/fail; regression sweep)
- `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd -- --nocapture` (FACT pass/fail; regression sweep)
- `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd -- --nocapture` (FACT pass/fail; regression sweep)
- `cargo test -p slicer-host --test macro_mesh_raycast_z_down_tdd -- --nocapture` (FACT pass/fail; regression sweep)
- `cargo clippy --workspace -- -D warnings` (FACT pass/fail)

All commands above are delegation-friendly: a sub-agent dispatch returns FACT (pass/fail + exit code) or SNIPPETS (≤ 20 lines of failing assertion).

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: stated as a binary/observable check (most steps' precondition is "Step N-1 GREEN" plus "Packet 42 implemented" for Step 0).
- Postcondition: stated as a binary/observable check.
- Falsifying check: the cheapest grep/test that catches a regression in this step.
- Files allowed to read: named per step. The macro file (`crates/slicer-macros/src/lib.rs`) is large (~2000 lines) — read only the PaintSegmentation arm region (lib.rs:1760-1788) and the surrounding MeshSegmentation drain (lib.rs:1733-1746) for pattern reference.
- Files allowed to edit (≤ 3 per step): named per step.
- Expected sub-agent dispatches: named per step.
- Step context cost: stated per step (`S` or `M` only).

## Context Discipline Notes

Document context-budget hazards specific to this packet:

- **Large files in the read-only path that MUST be ranged or delegated**:
  - `crates/slicer-macros/src/lib.rs` (~2000 lines) — only lines 1700-1800 (the two prepass arms) and lines 1283-1314 (the inline-WIT block, post Packet 42; Step 0 confirms unchanged). Use Grep to land precisely.
  - `crates/slicer-host/src/wit_host.rs` (~5800 lines) — only the typed-variant helpers (lines 2300-2330 per Packet 42's edits) and the `HostExecutionContext::paint_region_entries` accessor.
  - `crates/slicer-host/src/dispatch.rs` (~2100 lines) — only `dispatch_prepass_call` for `PrePass::PaintSegmentation` / `MeshSegmentation` and `harvest_paint_segmentation_ir` (read-only context for assertion targets).
  - `docs/07_implementation_status.md` — only the TASK-130 cluster (lines 65-72) and the blocker list (lines 175-185). Never load whole file.
- **OrcaSlicer trees the implementer must NOT load directly**: the entire `OrcaSlicerDocumented/` tree.
- **Likely temptation reads (skip)**:
  - The other 13 paint-related test files in `crates/slicer-host/tests/` (full list in Packet 42's authoring scan). Reuse only `macro_mesh_segmentation_geometry_tdd.rs` and `macro_paint_region_roundtrip_tdd.rs` for harness patterns.
  - The full `crates/slicer-sdk/src/prepass_builders.rs` (~600 lines post Packet 42). Read only the `PaintSegmentationOutput::regions()` accessor signature.
  - The full canonical paint-segmentation core module (`modules/core-modules/paint-segmentation/`). Out of scope. The macro test-guest is a separate macro-authored fixture under `test-guests/sdk-prepass-guest/`.
- **Sub-agent return-format hints for the heaviest dispatches**:
  - `crates/slicer-macros/src/lib.rs` lookups → SNIPPETS (≤ 50 lines) of just the PaintSegmentation arm; never SUMMARY of the whole file.
  - Workspace test runs → FACT (pass/fail + exit code) or SNIPPETS (≤ 20 lines of failing assertion).
  - `build-test-guests.sh` → FACT (success line + new size of any rebuilt guest .wasm).
  - Doc edits → dispatch a worker for the edit; receive a 3-line confirmation including the inserted/modified line numbers.
