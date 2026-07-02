# ADR-0022: Explicit per-region origin for perimeter output builders

## Status

Accepted (packet 127 implementation session, 2026-06-30).

## Context

`PerimeterOutputBuilder` pushes (walls, infill areas, seam candidates, reordered
wall loops) are buffered in the SDK and drained to the WIT boundary after the
guest's `run_perimeters` / `run_wall_postprocess` returns. The host's
`effective_perimeter_origin()` is read at drain time to tag each push with its
source `(object_id, region_id)`.

The pre-fix origin came exclusively from `current_slice_region` /
`current_perimeter_region` — host-side fields set by `touch_*` callbacks fired
when the guest accesses a WIT `SliceRegionView` / `PerimeterRegionView` accessor.
But the guest's `for region in regions` loop iterates **SDK** view structs
(plain-data copies with no host callback), so `current_slice_region` is never
re-touched during the loop. It holds the LAST region touched during
`__slicer_adapt_slice_regions` (the macro's WIT-view construction pass before
the guest body runs). Every per-region `set_infill_areas` call collapses to
that one stale origin — only the last painted region's infill survives the
marshal's `OriginBucket` grouping, and `sync_perimeter_infill_areas_into_slice`
populates `sparse_infill_area` for exactly one region.

On `resources/cube_4color.3mf` this produced the visible symptom: T1 sparse
infill = 30 moves (unretract priming only, no real infill) where OrcaSlicer's
golden is 1243; T3 = 2425 (absorbing T1/T2 interior infill) where the golden is
992. The same LIFO-touch bug affects `Layer::PerimetersPostProcess` (seam-placer,
fuzzy-skin): all `push_reordered_wall_loop` / `push_wall_loop` calls are tagged
with the last region.

A grilling session evaluated three shapes:
- **Shape 1 (`list<perimeter-output-builder>`):** one builder per region. Rejected
  because builder backing structs (`PerimeterOutputBuilderData` etc.) are stateless
  tags — pushes write to one per-stage collector on `HostExecutionContext`. Shape 1
  forces builders to become stateful, the collector to become a `Vec`, and commit to
  go per-builder — a marshal/dispatch architecture change. The "kills `OriginBucket`
  complexity" pro was false for perimeter-only: `OriginBucket` / `OriginId` /
  `effective_perimeter_origin` are shared by infill and support, so a perimeter-only
  Shape 1 leaves all of it standing, creating two parallel output mechanisms.
- **Sub-shape 2B (explicit origin parameter on every push method):** rejected because
  it requires editing every push call site (classic-perimeters ~7, seam-placer ~3,
  fuzzy-skin ~1, arachne ~N) vs one `begin_region` call per loop.
- **Option A (forward-through SDK, from the prior spec):** rejected because it does
  not fix the bug. Forwarding at SDK push time still captures the stale
  `current_slice_region` because the SDK `SliceRegionView` has no host callback to
  re-touch it.

## Decision

Add an explicit `set-current-origin: func(object-id: string, region-id: string)
-> result<_, string>;` method to the WIT `perimeter-output-builder` resource. The
host stores it in a new `explicit_perimeter_origin: Option<OriginId>` field on
`HostExecutionContext`. `effective_perimeter_origin()` becomes a three-level
additive chain: `explicit_perimeter_origin.or(current_perimeter_region)
.or(current_slice_region)`. The existing `touch_*` fallback stays as
defence-in-depth.

On the SDK side, `PerimeterOutputBuilder` gains a `current_origin` field (set via
`begin_region(&mut self, object_id: &str, region_id: u64)`) and per-item
`*_origins: Vec<Option<(String, u64)>>` Vecs parallel to each collection. The
macro's `__slicer_drain_perimeter` calls `wit.set_current_origin(...)` before
each WIT push when the SDK item's origin is `Some`, and skips it for `None` (the
host fallback chain handles the `None` case).

Guests call `output.begin_region(region.object_id(), *region.region_id());` at
the top of their `for region in regions` loop. Four modules at the time of this
decision: classic-perimeters, arachne-perimeters, seam-placer, fuzzy-skin.
(The fake `arachne-perimeters` module was deleted in P108; `classic-perimeters`
is the sole perimeter generator until real Arachne lands under P110+P112.)

The marshal (`convert_perimeter_output`, `OriginBucket`) is unchanged. The
origins are just correct now; the bucketing logic is the same. The
`resolved_seam` drain gap (the macro drain never calls
`wit.push_resolved_seam(...)`, masked by `backfill_resolved_seam`) is a separate
bug class and deferred.

## Consequences

- Per-region perimeter output pushes carry the correct origin, restoring
  per-tool sparse-infill distribution on multi-region prints. The
  `cube_4color.3mf` T1 sparse-infill count went from 30 (no real infill) to
  14906 (real infill attributed to T1's region); sparse-infill blocks went
  from 194 (1 region) to 470 (all 4 regions).
- `begin_region` is convention-based, not structural. A guest that forgets it
  falls through to the stale `touch_*` fallback (same bug as today for that
  guest). No hard error. This is the trade-off of Shape 2 over Shape 1.
- The infill stage (`HostInfillOutputBuilder`) has the same LIFO-touch bug via
  `current_slice_region.clone()` but is a separate WIT resource, separate SDK
  builder, and separate modules. The `begin_region` SDK pattern this packet
  establishes is reusable for the infill sequel.
- The support stage is out of scope: `SupportIR` is flat (no per-region tool
  semantics). Per-region builders buy support nothing until its IR gains tool
  semantics.
- WIT change regenerates every guest's bindgen; `cargo xtask build-guests` is
  mandatory after editing the canonical WIT source.

## Alternatives considered

- **Shape 1 (`list<perimeter-output-builder>`):** rejected — stateless backing
  structs, disproportionate marshal/dispatch change, leaves `OriginBucket`
  standing for infill/support anyway. See Context.
- **Sub-shape 2B (origin parameter on every push):** rejected — more call sites
  to get wrong, same risk class as forgetting `begin_region`. See Context.
- **Option A (forward-through SDK):** rejected — does not fix the bug; the stale
  `current_slice_region` is captured regardless of where in the SDK push path
  the origin is read. See Context.
- **Option C (bindgen `self`):** rejected — breaks every existing WASM guest at
  once, and even with `self` the drain still has no per-item origin info.

## Verification

- `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket`
  — AC-4: explicit origin routes to the correct `PerimeterRegion`.
- `cargo test -p slicer-wasm-host --test contract -- layer_perimeters_origin_falls_back_to_slice_region_through_host_trait`
  — AC-5: fallback path preserved (additive, not replacement).
- `cargo test -p slicer-wasm-host --test contract -- effective_perimeter_origin_is_none_when_neither_set`
  — AC-N1: anonymous mode preserved when no origin is set.
- `cargo test -p slicer-runtime --test executor -- cube_4color_sparse_infill_per_painted_region`
  — AC-3: all four tools T0-T3 appear in sparse-infill output.
- `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face`
  — AC-2: no wall-colour regression.
- `cargo clippy --workspace --all-targets -- -D warnings` — AC-6: clean.

## Cross-references

- ADR-0021 (marshal boundary flat functions over `OriginBucket`) — the
  `OriginBucket` all-or-none attribution rule this packet preserves; the
  marshal is unchanged.
- Packet 126 (TASK-250, MMU painted-cube parity) — introduced the multi-region
  `variant_chain` that creates the dispatch scenario this bug surfaces on.
- Packet 95 (TASK-245/246, paint-segmentation OrcaSlicer parity port) —
  introduced per-color region splitting.
