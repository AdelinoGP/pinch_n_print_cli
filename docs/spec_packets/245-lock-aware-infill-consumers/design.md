# Design: 245-lock-aware-infill-consumers

## Controlling Code Paths

- Primary code path: `process_bucket_role` (`modules/core-modules/infill-linker/src/orchestrate.rs`),
  `group_then_nearest_neighbor` (`modules/core-modules/path-optimization-default/src/lib.rs`), and
  `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`).
- Neighboring tests/fixtures: `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs` (uses
  `InfillLinker`, `InfillOutputBuilder`, `PerimeterRegionViewBuilder`, and the
  `slicer_sdk::test_support::fixtures::extrusion_path3d_base` fixture);
  `modules/core-modules/path-optimization-default/src/lib.rs` `#[cfg(test)] mod tests` (lines
  ~404-700); `crates/slicer-gcode/tests/gcode_emit_tdd.rs` and
  `crates/slicer-gcode/tests/gcode_emit_per_role_tolerance_tdd.rs`.
- OrcaSlicer comparison: none — this packet has no OrcaSlicer parity surface (it changes PnP's own
  consumer behavior, not a ported algorithm).

## Architecture Constraints

- The carve pass is module-local to `infill-linker` per ADR-0026 (single caller, single home). No new
  shared geometry helper is extracted to `slicer-core` or `slicer-sdk`; the swept-footprint builder
  lives in the linker and is the only caller of the carve.
- Locked paths are self-clipping: the producer guarantees the entire swept footprint lies inside its
  legal domain, so the linker neither clips nor links them. This is the ADR-0063 exception to the
  four-canonical-fill-polygons invariant.
- The optimizer treats a locked block as one non-reversible candidate, mirroring ADR-0011's
  wall-subsequence precedent (walls are committed in final print order and never reordered within a
  region).
- No schema/version constant is bumped by this packet; the change is purely behavioral and gated on
  `order_lock.is_some()`.

## Code Change Surface

- Selected approach: three independent consumer edits, each gated on `order_lock.is_some()`, plus a
  shared structural-parity test per consumer. No shared abstraction between the three consumers —
  each reads the field directly from `ExtrusionPath3D` / `OrderedEntityView`.

### C1 — infill-linker (`modules/core-modules/infill-linker/src/orchestrate.rs`)

- In `process_bucket_role`, before the boundary/link path, partition each region's selected paths
  into `locked` (any `path.order_lock.is_some()`) and `unlocked`. Append `locked` verbatim to
  `buckets[record.prior_index]` in emission order (reusing the existing `append_paths` helper), and
  feed only `unlocked` into the existing `active`/`wall_groups`/`link_*` flow.
- Add a module-local `swept_footprint_polygons(paths: &[ExtrusionPath3D]) -> Vec<ExPolygon>` that
  builds, per segment, one endpoint-width trapezoid (the `swept_fill_shape` quad shape from
  `crates/slicer-runtime/src/visual_debug_render.rs`, lines ~654-673) plus a round disk (polygon
  approximation of a circle of radius `width/2`) at every vertex, then unions them.
- Add a module-local carve step that, after all buckets are filled, differences
  `swept_footprint_polygons(locked_paths)` out of every untagged role bucket of the same region via
  `slicer_core::polygon_ops::difference_ex`. Locked paths are never carved.
- Rejected alternative: host-carved fifth partition polygon (`bridge_anchor_area`) — rejected in the
  plan (D4) because it encodes producer-config-derived geometry into the generic partition.

### C2 — path-optimization-default (`modules/core-modules/path-optimization-default/src/lib.rs`)

- In `group_then_nearest_neighbor`, before `nearest_neighbor_permutation` is applied to a role group,
  coalesce consecutive entities sharing a non-`None` `order_lock` into a single candidate whose
  start is the block's first entity start and whose end is the block's last entity end. The
  permutation then treats the block as one non-reversible unit; the block's internal entities are
  emitted in authored order with `reversal = false`.
- Rejected alternative: a dedicated `ExtrusionRole` variant or `Custom("…")` string convention —
  rejected in the plan (D2) as role proliferation / scattered string matches.

### C3 — G-code emission (`crates/slicer-gcode/src/emit.rs`)

- In `emit_gcode`, at the simplification site (lines ~504-547), when the entity's path carries
  `order_lock: Some(_)`, skip both `simplify_polyline_mm` (Douglas-Peucker at `tolerance_for_role`)
  and `drop_short_segments_mm` (`min_segment_length`), emitting every authored point. The per-point
  speed-profile and E-computation loop (lines ~549-599) is unchanged.
- Rejected alternative: a new `ExtrusionRole` to signal "no simplify" — same D2 rejection.

## Files in Scope (read + edit)

- `modules/core-modules/infill-linker/src/orchestrate.rs` - role: primary; expected change: locked
  passthrough branch + `swept_footprint_polygons` + carve step.
- `modules/core-modules/path-optimization-default/src/lib.rs` - role: primary; expected change:
  locked-block coalescing in `group_then_nearest_neighbor` + inline tests.
- `crates/slicer-gcode/src/emit.rs` - role: primary; expected change: locked bypass at the
  simplification site.
- `modules/core-modules/infill-linker/tests/orchestrate_tdd.rs` - role: tests; expected change: new
  locked-passthrough, carve, cross-domain, and neutrality tests.
- `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - role: tests; expected change: new locked-bypass and
  neutrality tests.
- `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` - role: docs; new ADR.
- `docs/02_ir_schemas.md` - role: docs; amend the four-canonical-fill-polygons invariant.
- `CONTEXT.md` - role: docs; amend the Infill entry.

## Read-Only Context

- `crates/slicer-runtime/src/visual_debug_render.rs` - lines `639-687` only - purpose: the
  `swept_fill_shape` segment-quad precedent (no round caps; the linker adds them).
- `crates/slicer-gcode/src/serialize.rs` - lines `26-58` only - purpose: `tolerance_for_role`
  resolves `BridgeInfill` to `infill_resolution`.
- `crates/slicer-core/src/polygon_ops.rs` - `difference_ex`/`union_ex` signatures only - purpose: the
  carve differencing primitive.
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` - single-caller rule.
- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` - wall-subsequence precedent.

## Out-of-Bounds Files

- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/**`, `crates/slicer-sdk/src/views.rs`,
  `crates/slicer-runtime/src/layer_executor.rs` - packet 244 owns the carrier/projection/enforcement;
  do not edit.
- `crates/slicer-runtime/src/visual_debug_render.rs` - read-only precedent; do not edit.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `OrcaSlicerDocumented/` - not applicable to this packet.

## Expected Sub-Agent Dispatches

- Question: which `append_paths`/bucket helpers in `orchestrate.rs` already append verbatim without
  clipping, and what is their exact signature? scope: `modules/core-modules/infill-linker/src/orchestrate.rs`; return: `LOCATIONS`; purpose: Step 1.
- Question: does `slicer_core::polygon_ops` expose a circle/round-disk polygon builder, or must the
  linker approximate vertex disks itself? scope: `crates/slicer-core/src/polygon_ops.rs`; return: `FACT`; purpose: Step 1.
- Question: how does `nearest_neighbor_permutation` consume its `&[&OrderedEntityView]` input (does
  it read `original_index` and a start/end point per entity)? scope: `modules/core-modules/path-optimization-default/src/lib.rs`; return: `SNIPPETS` (≤30 lines); purpose: Step 2.

## Data and Contract Notes

- IR/manifest contracts: none changed. `order_lock` is read-only here; the field's semantics are
  fixed by ADR-0062 (packet 244).
- WIT boundary: none changed.
- Determinism/scheduler constraints: the carve and locked-passthrough must be deterministic
  (discovery order); the optimizer's block coalescing must not change the tool-cluster ordering.

## Locked Assumptions and Invariants

- Locked paths are self-clipping (ADR-0063): the producer guarantees the swept footprint lies inside
  its legal domain; the linker differences untagged fill by that footprint and never clips the locked
  path itself.
- A locked block is atomic and contiguous within one `(layer, object, region)`; the optimizer may
  move the block as a unit but never split, reverse, or internally reorder it.
- Speed/flow side mutations of locked paths remain legal; only sequence and geometry (points,
  widths) are protected.

## Risks and Tradeoffs

- The round-disk vertex approximation adds polygon vertices; if the disk is too coarse, the carve
  leaves slivers of untagged fill under the caps. Mitigation: a fixed segment count per disk (e.g.
  16) and a test asserting no untagged fill overlaps the swept area.
- The carve runs after bucket fill, so it must not reorder buckets or disturb the locked passthrough
  already appended. Mitigation: carve operates on the untagged buckets only, keyed by region.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1, the linker passthrough + carve)
- Highest-risk dispatch and required return format: the `polygon_ops` round-disk `FACT` (determines
  whether the linker writes its own disk builder).

## Open Questions

- `[FWD]` Does packet 246's wave module need the linker's carve to also difference the swept
  footprint out of *other regions'* buckets, or only the same region? (Plan D4 says "same region";
  confirm at packet 246 authoring.)
- None `[BLOCK]`.
