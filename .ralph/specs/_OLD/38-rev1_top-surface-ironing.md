---
status: implemented
packet: top-surface-ironing-rev1
task_ids:
  - TASK-169
supersedes: 38_top-surface-ironing
---

# 38-rev1_top-surface-ironing

## Goal

Ship a new core module `top-surface-ironing` running at `PostPass::LayerFinalization` (object-scope, sequential, full `Vec<LayerCollectionIR>` visibility) that emits a low-flow zigzag pass over `TopSolidInfill` polygons on the topmost top-solid layer per region, tagged with `ExtrusionRole::Ironing` and producing `;TYPE:Ironing` G-code blocks. Mirrors Orca's `posIroning` phase (`PrintObject::ironing()` at OrcaSlicer `PrintObject.cpp:838`) which runs strictly after all per-layer infill is committed. Configurable via `ironing: bool`, `ironing_speed`, `ironing_flow`, `ironing_spacing`, `ironing_pattern`. Defaults align with OrcaSlicer (`ironing_spacing = 0.1 mm`, `ironing_flow = 0.10`, `ironing_speed = 20 mm/s`).

## Problem Statement

The live path lacks an ironing pass over top surfaces. Orca emits a low-flow zigzag pass over the topmost top-solid layer's `TopSolidInfill` polygons (`PrintObject::ironing()` at `PrintObject.cpp:838` — runs as a distinct phase strictly after all per-layer infill is committed; analog of our `PostPass::LayerFinalization`). Without ironing, printed top surfaces show extrusion lines and inter-line gaps.

Predecessor packet `38_top-surface-ironing` attempted this at `Layer::InfillPostProcess` — wrong stage. `Layer::InfillPostProcess` is rayon-parallel per layer with no cross-layer look-ahead, and the `is_top_surface` flag set on `SliceRegionView` at slice time does not propagate to `PerimeterRegionView`. The predecessor implementation fell back to a structurally incorrect `infill_areas.is_empty()` proxy that could not distinguish topmost-of-stack from interior top-solid layers; the AC-TSI-3 test was vacuous (an empty-region fixture); and the Benchy E2E never emitted `;TYPE:Ironing`. The architectural fix — chosen by the user after spec-review — is to relocate to `PostPass::LayerFinalization` (object-scope, sequential, full `Vec<LayerCollectionIR>` visibility), where topmost-layer detection is a direct scan rather than a proxy.

This packet redesigns at the correct stage, mirroring the `skirt-brim` module (the existing object-scope reference) for skeleton, callback shape, and output mechanism. Defaults align with OrcaSlicer (`0.1 mm` / `0.10` / `20 mm/s`), eliminating the divergence flagged by the predecessor's spec-review.

## Architecture Constraints

- **Stage**: `PostPass::LayerFinalization`. Sequential, single-threaded, runs after all per-layer stages drain via the rayon join (`docs/04_host_scheduler.md:680-717`). The module sees the FULL `Vec<LayerCollectionIR>` for all objects in the print; cross-layer look-ahead is therefore native, not synthesized.
- **Trait**: `FinalizationModule` with `run_finalization(&self, layers: &[LayerCollectionView], output: &mut FinalizationOutputBuilder, _config: &ConfigView) -> Result<(), ModuleError>` (signature from `skirt-brim/src/lib.rs:300-305`). NOT `LayerModule::run_infill_postprocess` — that was the predecessor's mistake.
- **Output channel**: `output.push_entity_to_layer(layer_index, path, region_key)` (precedent `skirt-brim/src/lib.rs:347-349`). Module does NOT mutate the input `&[LayerCollectionView]`. Host's dispatch code at `crates/slicer-host/src/dispatch.rs:2877` collects pushes and merges them into `layer.ordered_entities` via `splice(0..0, ...)` (prepend). **The prepend behavior is a known concern** — see "Risks and Tradeoffs" — Step 0 must verify whether the splice actually targets the front or whether the index is computed per push, AND whether the host has any role-based ordering that would correctly place Ironing entities after fill entities at G-code emit time. If pure prepend is the only behavior, Step 0a extends the SDK to support an APPEND mode before Step 3 implementation.
- **IR-access transform chain**: `reads = ["LayerCollectionIR"]`, `writes = ["LayerCollectionIR.ironing"]`. The kebab-case sub-field write target is the canonical pattern from `skirt-brim` (`"LayerCollectionIR.skirt-brim"`); Step 0 confirms the exact ironing field name from the IR schema.
- **Detection mechanism**: object-scope direct lookup. For each `(object_id, region_key)` derivable from the `LayerCollectionView` slice, scan `0..layers.len()` from highest index downward; the first index whose region carries any `TopSolidInfill` paths is "the topmost top-solid layer" for that region. Emit ironing only on that layer. This requires no `is_top_surface` flag, no SDK extension to per-region views, and no proxy.
- **Coordinate system**: 1 unit = 100 nm (`docs/08_coordinate_system.md`). All zigzag generation must use `Point2::from_mm` / `mm_to_units()`; never assume Orca's 1 unit = 1 nm.
- **Append-only contract**: ironing entities are emitted as ADDITIONAL entities on the topmost layer; existing `TopSolidInfill` paths in input layers are not touched (the module reads `&[LayerCollectionView]` — read-only by signature; mutations are scoped to the output builder).
- **Determinism**: PostPass is sequential with pool size 1 per `docs/04_host_scheduler.md:680-717`. No parallelism inside the module is needed or allowed.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR` — read (full input visibility) + write (ironing strokes appended to a designated sub-field, exact kebab-case path TBD by Step 0).
  - `InfillRegion.solid_infill` and `InfillRegion.ironing` — both exist (predecessor's Step 0 dispatch confirmed `slicer-ir/src/slice_ir.rs:1354-1365`). The `ironing` sub-field is the natural target; whether the host merges `ironing` paths into G-code or whether the module's emitted entities flow through `ordered_entities` is the open question Step 0 resolves.
- WIT boundary considerations: none new — the `FinalizationModule` trait is already host-supported (precedent: `skirt-brim`).
- Determinism / scheduler constraints:
  - Transform-chain edge fill→ironing established by `reads/writes` declarations.
  - PostPass is sequential; no internal parallelism allowed.

## Locked Assumptions and Invariants

- `ExtrusionRole::Ironing` enum variant exists (predecessor confirmed at `crates/slicer-host/src/wit_host.rs:2572`).
- `ExtrusionRole::Ironing => ";TYPE:Ironing"` mapping exists at `crates/slicer-host/src/gcode_emit.rs:91` (predecessor confirmed).
- `skirt-brim` module is a working `PostPass::LayerFinalization` module and its skeleton is the canonical template.
- `FinalizationOutputBuilder::push_entity_to_layer(layer_index, path, region_key)` is the only emission API for finalization modules (per `skirt-brim/src/lib.rs:347-349`).
- The host's per-layer entity merge at `dispatch.rs:2877` integrates finalization pushes into `ordered_entities` so that G-code emit picks up the role marker. Pending confirmation of insertion order — see Risks.

## Risks and Tradeoffs

- **Insertion-order risk (highest implementation risk).** If the host's `splice(0..0, ...)` at `dispatch.rs:2877` literally prepends, ironing entities will appear BEFORE fill entities in G-code, which is wrong (ironing must follow fill within a layer). Mitigations in priority order:
  1. Step 0 FACT confirms whether the index parameter is actually `0` or computed per push. If computed, the issue is moot.
  2. If the splice is purely prepend, Step 0a extends the SDK / host to provide an APPEND or AFTER-region insertion mode. Scope expansion stays inside `crates/slicer-sdk/` and `crates/slicer-host/src/` finalization paths; no stage-graph changes.
  3. Worst case: if (1) and (2) are both blocked, the packet stays `draft` and a follow-up packet is opened for the SDK extension before reactivating this one. The module-level tests CAN still verify the entity-push contract (assertion is on `output.entity_pushes()`, which records pushes regardless of host merge order); only the Benchy E2E AC-6 is sensitive to the merge order.
- **`claim_transition_matrix_tdd` regression**. May or may not be packet-attributable. Step 0 SUMMARY determines whether the fix is mechanical or substantive; a substantive fix may exceed packet scope and would be carved out into a separate packet.
- **`placeholder_wasm` convention**. Predecessor's Step 0 found NO existing core-module manifest declares the field. Either every existing manifest is non-compliant (unlikely) or the test fixture is wrong. Step 0 FACT determines the correct fix.
- **Bounding ExPolygon vs union**. For complex top surfaces (donuts, multi-island regions), a bounding-box approach over-irons empty space. Orca uses union (`Fill.cpp:1719`). The packet's algorithm calls for union via `slicer-helpers`; Step 0 FACT confirms helper availability. If absent, fall back to per-island bounding boxes.
