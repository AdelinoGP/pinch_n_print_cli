---
status: implemented
packet: 57_overhang-speed
task_ids:
  - TASK-182
---

# 57_overhang-speed

## Goal

Wire OrcaSlicer-parity overhang quartile speed end-to-end on the live G-code path: extend the `point3-with-width` WIT record with an `overhang-quartile: option<u8>` field, add a host-side `overhang_classifier` prepass that buckets wall-family vertices by signed distance to the previous-layer support polygons, and extend `resolve_feedrate` to dispatch `OuterWall | InnerWall | ThinWall` points to the four `overhang_{1,2,3,4}_4_speed` keys registered by packet 52. First remediation against DEV-009 for *quality-modulated* feedrate beyond the bare per-role tokens.

## Problem Statement

Packet 52 registered all four `overhang_N_4_speed` config keys in `FeedrateConfig` and in the wider speed schema, but those keys are dead code today:

1. `resolve_feedrate` in `crates/slicer-host/src/gcode_emit.rs:154` has no dispatch arm for them; it only chooses among the per-role base speeds.
2. No upstream stage produces quartile data on `Point3WithWidth`; the IR carries no `overhang_quartile` field.
3. The `point3-with-width` WIT record at `wit/deps/types.wit:7-11` carries no overhang field either, so even if a host stage produced one, it would not survive a WIT roundtrip into modules that consume layer IR.

OrcaSlicer slows down wall extrusions when they print over insufficiently-supported previous-layer geometry, using the `overhang_{1,2,3,4}_4_speed` schedule (1/4 = least supported / slowest, 4/4 = most supported / fastest). The four keys are wired in OrcaSlicer through `ExtrusionProcessor.hpp::estimate_points_properties` and `GCode.cpp::estimate_extrusion_quality`, which bucketize each vertex by signed distance to the previous-layer support polygons.

This packet closes the gap end-to-end: WIT field + Rust mirror, classifier prepass, `resolve_feedrate` dispatch, pipeline wire-in, regression tests, and remediation notes.

This packet does **not** reopen or supersede packet 52; it consumes packet 52's registered keys without modifying them.

## Architecture Constraints

- WIT boundary integrity: any field added to the `point3-with-width` record propagates to every host-binding and guest-binding site. Per `CLAUDE.md` *WIT/Type Changes Checklist*: search `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules; verify type identity matches across component boundaries; run `cargo build --tests` after the change.
- IR schema evolution: bumping the schema minor version constant is mandatory (per `docs/02_ir_schemas.md` versioning rules). New field on a host-serialized struct without a version bump is a contract regression.
- Coordinate convention: `Point3WithWidth.x/y/z` are `f32` mm at the emitter layer (per usage at `gcode_emit.rs:380` and `docs/08_coordinate_system.md`). The classifier MUST work in mm and use `Point2::from_mm` (per `docs/13_slicer_helpers_crate.md`).
- Determinism: classification iterates `layer_irs.windows(2)`; for each layer, paths and points are visited in their existing IR order. No hashing, no parallel iteration that could reorder writes.
- Backpressure gate (per `CLAUDE.md`): `cargo build`, narrow tests, and `cargo clippy` must pass before any packet-close motion. `cargo test --workspace` runs only at the close ceremony.

## Data and Contract Notes

- IR contracts touched: `Point3WithWidth` gains an optional field; `#[serde(default)]` preserves backward-compat for older JSON producers (AC-6). Schema minor version bump signals the change to any downstream consumer.
- WIT boundary considerations: `point3-with-width` record adds `overhang-quartile: option<u8>`. WIT `option<u8>` maps to Rust `Option<u8>` directly via `wit-bindgen`; no manual wrapping required. The conversion sites enumerated by Step 0's LOCATIONS dispatch must each propagate the new field — no silent drops, no `Default::default()` shortcuts that erase classifier output.
- Determinism / scheduler constraints: classifier mutates `LayerCollectionIR` in place. It MUST run after layer finalization (the existing `LayerCollectionIR` is fully built) and before per-layer G-code emission. Implementation runs it from inside `DefaultGCodeEmitter::emit_gcode` on the emitter's cloned layer set, so the upstream `LayerCollectionIR` observed by other postpass modules is not mutated. No new scheduling phase; piggybacks on emit.
- The classifier short-circuits when all four `overhang_N_4_speed` keys are exactly `0.0` (the packet 52 default). This preserves AC-2's byte-identical zero-config no-op.
- Synthetic `Point3WithWidth` construction sites in seam-candidate code paths (`crates/slicer-host/src/wit_host.rs` at three sites: the seam-candidate position rehydration paths) deliberately set `overhang_quartile: None`. These sites build temporary points from seam position data alone (no classification context), strictly upstream of the classifier and not on a path that emits G-code without re-classification. The `None` value is the correct construction default; the classifier overwrites it on wall-family roles when emit runs.

## Locked Assumptions and Invariants

- `Point3WithWidth.x/y/z` are `f32` mm at the time the classifier runs (per `gcode_emit.rs:380` usage). The classifier works in mm; no unit conversion.
- `Point3WithWidth.width` is `f32` mm (consistent with the `width` arg of the OrcaSlicer estimator). The thresholds `[0, 0.25w, 0.5w, 0.75w]` use this `width` field per point — not a global default.
- `overhang_quartile` value space: `None | Some(1) | Some(2) | Some(3) | Some(4)`. `Some(0)` is reserved and treated as an invariant violation (AC-N1).
- `classify_layers` only mutates entries whose role is in `{OuterWall, InnerWall, ThinWall}`. Other roles' points are left with `overhang_quartile = None`.
- First layer (no previous layer) leaves every quartile as `None` regardless of config (AC-4).
- The classifier consumes the previous layer's `OuterWall | InnerWall | ThinWall` polylines, joined into closed-loop polygons using the existing IR's loop convention. Interior holes (inner contours) flip the inside-test sign — the classifier respects loop winding.

## Risks and Tradeoffs

1. **WIT-binding fan-out blast radius.** `Point3WithWidth` is a foundational record. The conversion sites are mechanical but enumeration must be exhaustive — a missed site silently drops `overhang_quartile` on the floor. Mitigation: Step 0 begins with a LOCATIONS dispatch; the implementer audits the list against the resulting build errors from a deliberate `unimplemented!()` placeholder before filling in correct conversions.
2. **Quartile threshold endpoint off-by-one.** OrcaSlicer's `< / <=` convention at quartile boundaries is critical for AC-5. Mitigation: Step 3 dispatches a SNIPPETS read of `ExtrusionProcessor.hpp:397` and `:535`; the implementer mirrors the convention verbatim.
3. **Polygon-inside-test for interior holes.** A naïve line-distance approach would mis-classify points above an interior hole as "supported." Mitigation: Up-front polygon-winding inside-test (already chosen as the selected approach). Trade-off: more code than line-distance, but only marginally — and avoids a guaranteed deviation against Orca.
4. **`LinesDistancer2D` performance.** Linear scan with bbox prefilter is O(N·M) for N points × M segments per layer-transition. Mitigation: profile-only-if-needed; defer BVH. If profiling later shows dominance, a follow-up packet adds the BVH.
5. **Byte-identical zero-config baseline (AC-2).** A subtle bug — e.g., the classifier writing `Some(_)` despite short-circuit — would silently break AC-2 even though `resolve_feedrate` ignores it. Mitigation: the AC-2 test asserts BOTH the G-code bytes AND that every wall point's `overhang_quartile == None` after the pipeline runs.
6. **Schema-version bump coordination.** Any consumer of the IR with a pinned older schema fails roundtrip. Mitigation: `#[serde(default)]` + the AC-6 missing-field branch. The bump is "minor" — additive compatible field.
