# Requirements: 245-lock-aware-infill-consumers

## Packet Metadata

- Grouped task IDs: `TASK-355`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 244 (draft) introduces the `order_lock: Option<u64>` carrier and the host enforcement
contract, but no consumer honors the field yet. Three downstream stages still destroy locked sequences: the infill
linker re-clips, chains, and reverses bridge-role paths; path optimization nearest-neighbor permutes
role groups and may reverse entities; G-code emission runs Douglas-Peucker and `min_segment_length`
pruning that drops authored interior points. Until these three consumers honor locks, a producer
that mints locks (packet 246's wave-overhangs module) would have its physically load-bearing print
order silently destroyed. This packet closes that gap with three consumer changes plus structural
parity proof that all-`None` slices are unchanged.

## In Scope

- **C1 — infill-linker locked passthrough + carve.** In `process_bucket_role`
  (`modules/core-modules/infill-linker/src/orchestrate.rs`): locked paths bypass boundary lookup,
  linking, overlap-offset trimming, and clipping, and are appended verbatim per region in emission
  order. A new module-local carve pass (ADR-0026 single-caller rule) differences the swept footprint
  of locked paths — one endpoint-width trapezoid per segment plus a round disk at every vertex —
  out of every untagged role bucket of the same region. The host precedent
  `swept_fill_shape` (`crates/slicer-runtime/src/visual_debug_render.rs`) has no round caps; the
  linker's guest-side equivalent adds them.
- **C2 — path-optimization-default locked blocks.** In `group_then_nearest_neighbor`
  (`modules/core-modules/path-optimization-default/src/lib.rs`): each locked block is one
  nearest-neighbor candidate (authored first start, last end), never reversed, never split; blocks
  keep internal order. Mirrors the wall-subsequence precedent of ADR-0011.
- **C3 — G-code emission locked bypass.** In `DefaultGCodeEmitter::emit_gcode`
  (`crates/slicer-gcode/src/emit.rs`): locked paths bypass both Douglas-Peucker
  (`simplify_polyline_mm` at `tolerance_for_role`, which resolves `BridgeInfill` to
  `infill_resolution`) and `min_segment_length` pruning (`drop_short_segments_mm`). Coordinate
  formatting at serialization still applies.
- **C4 — structural parity.** Representative linker/optimizer/emitter fixtures with all-`None` locks
  produce identical path/entity/G-code structures through the new branches. No new golden files.
- **Docs.** Land ADR-0063 (sequence-locked paths may occupy neighboring fill domains); amend the
  four-canonical-fill-polygons invariant in `docs/02_ir_schemas.md` and the `CONTEXT.md` Infill
  entry.

## Out of Scope

- Any new producer of `order_lock` (packet 246's wave-overhangs module is the first).
- Changes to the `ExtrusionPath3D`/WIT/`OrderedEntityView` surface or the host enforcement points
  (packet 244 owns those).
- The `InternalBridgeInfill` host constructor and internal-bridge handling (packet 246).
- Speed/flow side mutations of locked paths — these remain legal per the ADR-0062 semantics
  contract; only sequence and geometry (points, widths) are protected.
- New golden G-code files.

## Authoritative Docs

- `docs/specs/wave-overhangs-bridge-fill-plan.md` - 473 lines; §"Packet 3 — Lock-aware consumers"
  (C1–C4) and Appendix A (ADR draft) read directly; the rest is context.
- `docs/02_ir_schemas.md` - over 300 lines; only §"Post-`Layer::Perimeters` invariant" (lines
  ~596-626) read directly.
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` - single-caller rule.
- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` - wall-subsequence precedent.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (linker passthrough), `AC-2` (linker carve), `AC-3` (optimizer block), `AC-4`
  (emitter bypass), `AC-5` (all-`None` neutrality).
- Negative: `AC-N1` (cross-domain locked path not clipped), `AC-N2` (optimizer never splits/reverses
  a block).
- Cross-packet impact: packet 246 depends on the behavior this packet lands — the wave module emits
  order-locked `BridgeInfill` paths and relies on the linker/optimizer/emitter to preserve them
  verbatim and carve untagged fill around their swept footprint.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p infill-linker --test orchestrate_tdd` | linker passthrough + carve + neutrality | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p path-optimization-default --lib` | optimizer locked-block handling | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd` | emitter bypass + neutrality | FACT pass/fail |
| `cargo check --workspace --all-targets` | all targets compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT pass/fail |

## Step Completion Expectations

- The linker carve pass must be module-local (ADR-0026): no new shared `slicer-core` or `slicer-sdk`
  surface, no extraction of the swept-footprint builder out of the linker.
- The optimizer's locked-block grouping must be computed before `nearest_neighbor_permutation` is
  applied to a role group, so a block is never split across the permutation.
- The emitter's locked bypass must skip both simplification stages but still run the per-point
  speed-profile and E-computation loop unchanged.

## Context Discipline Notes

- `modules/core-modules/infill-linker/src/orchestrate.rs` is ~600+ lines; read only the
  `RoleBoundaries`/`process_bucket_role`/`link_*` ranges, never the whole file.
- `crates/slicer-gcode/src/emit.rs` is large; read only `emit_gcode` (lines ~255-600) and
  `resolve_feedrate` (lines ~144-187). The D-P tolerance lives in
  `crates/slicer-gcode/src/serialize.rs::tolerance_for_role` (lines ~26-58).
- `crates/slicer-runtime/src/visual_debug_render.rs::swept_fill_shape` (lines ~639-687) is read-only
  precedent; do not edit it.
