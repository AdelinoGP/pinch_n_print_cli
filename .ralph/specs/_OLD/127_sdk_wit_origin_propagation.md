---
status: implemented
packet: 127_sdk_wit_origin_propagation
task_ids:
  - TASK-252
---

# 127_sdk_wit_origin_propagation

## Goal

Add an explicit `set-current-origin` method to the WIT `perimeter-output-builder` resource and a matching `begin_region` context method on the SDK `PerimeterOutputBuilder`, so per-region perimeter output pushes (walls, infill areas, seam candidates, reordered wall loops) carry the origin of the region the guest is currently iterating rather than the last-touched WIT view's stale LIFO origin — restoring per-tool sparse-infill distribution on the painted-cube fixture to OrcaSlicer parity.

## Problem Statement

Slicing `resources/cube_4color.3mf` produces gcode that is mostly correct but missing infill across internal painted regions. The per-tool `;TYPE:Sparse infill` segment count shows T1 = 30 (just unretract priming moves, no actual extrusion) where OrcaSlicer's golden has T1 = 1243, and T3 = 2425 (absorbing T1/T2 interior infill) where OrcaSlicer has T3 = 992.

**Root cause:** The SDK `PerimeterOutputBuilder` buffers every `set_infill_areas` call. The macro-generated `__slicer_drain_perimeter` forwards the buffered calls to the WIT builder **after** the guest's `run_perimeters` returns. The WIT-level `set_infill_areas` captures `effective_perimeter_origin()` at the moment of the drain call — by which point the host's `current_slice_region` is the **LAST** `(object_id, region_id)` that any WIT `SliceRegionView` accessor touched during `__slicer_adapt_slice_regions`. The guest's `run_perimeters` iterates **SDK** `SliceRegionView`s (plain-data structs with no host callback), so `current_slice_region` is never re-touched during the guest's loop. Every per-region `set_infill_areas` call collapses to one bucket — only the last painted region's `infill_areas` survives, and `sync_perimeter_infill_areas_into_slice` populates `sparse_infill_area` for exactly one region.

The same LIFO-touch bug affects `Layer::PerimetersPostProcess` (seam-placer, fuzzy-skin): after `__slicer_adapt_perimeter_regions`, `current_perimeter_region` is the last region, and all `push_reordered_wall_loop` / `push_wall_loop` calls are tagged with that origin.

**Why this is a coherent slice:** the fix is one WIT method + one SDK method + one `begin_region` call per guest loop + one `.or_else()` line in `effective_perimeter_origin`. The marshal is unchanged. The four guest modules share the same `for region in regions` loop shape. The infill stage has the same bug but is a separate fix surface (different WIT resource, different SDK builder, different modules) — deferring it keeps this packet's blast radius to 4 crates + 4 modules.

This packet supersedes the prior `127_sdk_wit_origin_propagation/packet.spec.md` (authored during the diagnose session), which recommended Option A (forward-through SDK). The grilling session proved Option A does not fix the bug: forwarding at SDK push time still captures the stale `current_slice_region` because the SDK `SliceRegionView` has no host callback to re-touch it. The prior spec's three options (A/B/C) all share this flaw. This packet replaces them with Shape 2 (single builder + explicit `set-current-origin` WIT method + `begin_region` SDK method), which captures origin at SDK push time from the guest's loop context, not from the host's stale touch state.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Builder backing structs are stateless tags.** `PerimeterOutputBuilderData`, `InfillOutputBuilderData`, `SupportOutputBuilderData` are empty structs (host.rs:195-238). Every push ignores the resource handle and writes to one per-stage collector on `HostExecutionContext` keyed by origin tag. This packet adds a new field (`explicit_perimeter_origin`) to `HostExecutionContext`, not to the backing struct — the single-builder WIT contract is preserved. A `list<builder>` approach (Shape 1, rejected) would have forced these structs to become stateful and the collector to become a `Vec` — a marshal/dispatch architecture change this packet avoids.
- **Origin machinery is shared by perimeter, infill, and support.** `effective_perimeter_origin()` and `OriginBucket` are used by `HostPerimeterOutputBuilder` (6 sites), `HostInfillOutputBuilder` (3 sites via `current_slice_region.clone()`), and `HostSupportOutputBuilder` (3 sites via `current_slice_region.clone()`). This packet's additive `.or_else()` in `effective_perimeter_origin` affects only the perimeter path (infill/support read `current_slice_region` directly, not via `effective_perimeter_origin`). A perimeter-only migration leaves the origin machinery standing for infill/support — no parallel mechanisms.
- **Support IR is flat.** `SupportIR` has no per-region identity; support prints as T0 (layer_executor.rs:816-839). Per-region builders buy support nothing until its IR gains tool semantics (a schema change, not a builder change). This packet does not touch support.

## Data and Contract Notes

- **IR contracts touched:** none. `PerimeterIR`, `PerimeterRegion`, `PerimeterOutputCollected` are unchanged. The `*_origins` Vecs already exist (from the marshal precondition).
- **WIT boundary considerations:** one new method on `perimeter-output-builder` resource. This regenerates every guest's bindgen output — `cargo xtask build-guests` is mandatory. The method is `set-current-origin: func(object-id: string, region-id: string) -> result<_, string>;` — takes string-typed identity (matching the existing `region-key` pattern in the WIT), returns `result` for consistency with other builder methods.
- **SDK contract:** `begin_region(&mut self, object_id: &str, region_id: u64)` — takes `&str` + `u64` (matching `ObjectId = String` + `RegionId = u64` in slicer-ir). Sets `self.current_origin = Some(OriginId { object_id: object_id.to_string(), region_id })`. Does NOT return `Result` — it's a pure setter with no capacity check.
- **Determinism or scheduler constraints:** none. The origin is set synchronously in the guest's loop; the drain is synchronous after the guest returns. No reordering.

## Locked Assumptions and Invariants

- **Invariant: additive origin chain.** `effective_perimeter_origin()` must remain `explicit_perimeter_origin.or(current_perimeter_region).or(current_slice_region)`. The `touch_*` fallback must not be removed — it's defence-in-depth for guests that forget `begin_region` and it's the only origin source for infill/support.
- **Invariant: marshal unchanged.** `convert_perimeter_output` and `OriginBucket` must not be modified by this packet. The origins are just correct now; the bucketing logic is the same.
- **Invariant: `begin_region` is convention-based, not structural.** A guest that forgets `begin_region` falls through to the stale `touch_*` fallback (no hard error). The new host test (AC-4) pins the explicit path; the gcode test (AC-1/AC-3) pins the end-to-end behaviour. The fallback test (AC-5) pins the defence-in-depth path.
- **Invariant: single builder per dispatch.** The WIT `run-perimeters` and `run-wall-postprocess` signatures are unchanged. One `perimeter-output-builder` resource per dispatch call.
- **Invariant: `resolved_seam` drain gap stays.** The macro drain does NOT call `wit.push_resolved_seam(...)`. `backfill_resolved_seam` in `layer_executor.rs:1020-1037` fills from `SeamPlanIR`. This packet does not fix the drain gap. Seam-placer's `set_resolved_seam` calls continue to have no effect on the output IR via the drain path.

## Risks and Tradeoffs

- **WIT change regenerates every guest's bindgen.** `cargo xtask build-guests` mandatory. Stale guests surface as test failures that look unrelated to the edit (typed instantiation mismatches). The `--check` gate must pass before attributing any test failure to the packet's changes.
- **`set-current-origin` is convention-based.** A future guest that forgets `begin_region` gets the stale fallback (same bug as today for that guest). No hard error. This is the trade-off of Shape 2 over Shape 1 (which makes mis-assignment structurally impossible). Accepted because Shape 1's cost (stateful builders + Vec collector + per-builder commit) is disproportionate for a perimeter-only fix.
- **`PerimetersPostProcess` fix changes wall tool attribution for seam-placer/fuzzy-skin output.** Today the spatial fallback in `layer_executor` recovers wall tools (walls sit on their region's perimeter). Post-fix, the origin tag is correct from the source, so the fallback is redundant but not removed. No behaviour change expected for walls (fallback already worked for them). The gcode test (AC-1/AC-3) covers this.
- **The uncommitted marshal precondition (11 files) must land with this packet.** It's the foundation: per-call `infill_areas` accumulation + `OriginBucket` per-origin drain. Without it, the explicit origins have nothing to bucket into. Folded into Step 1.
