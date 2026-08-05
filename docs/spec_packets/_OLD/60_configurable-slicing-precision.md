---
status: implemented
packet: 60_configurable-slicing-precision
task_ids:
  - TASK-201
---

# 60_configurable-slicing-precision

## Goal

Add seven OrcaSlicer-style numeric precision knobs to `ResolvedConfig`, implement a real Douglas-Peucker simplifier in `slicer-helpers`, wire D-P + min-segment sweep into the host G-code emit by `ExtrusionRole`, parameterize XYZ decimal output, plumb a configurable Clipper2 arc tolerance through `slicer_core::polygon_ops::offset`, and apply a `slice_closing_radius` inflate/deflate round-trip at mesh slice — all with zero-cost legacy behavior when each key is set to `0.0` / `4`.

## Problem Statement

Three sites in the slicing pipeline are pinned to the finest-possible precision with no user knob, making slicing slower than necessary and producing G-code that is finer than printer hardware can resolve:

1. **G-code XY format is hardcoded `{:.4}`** in `crates/slicer-host/src/gcode_emit.rs:1304` — emits 0.0001 mm resolution. OrcaSlicer uses 3 decimals (`XYZF_EXPORT_DIGITS = 3`, 1 µm). Printers step in ~10 µm increments, so the extra digit is wasted precision plus extra file size.
2. **No polyline simplification exists at G-code emit.** `simplify_polygon_points` in `crates/slicer-core/src/triangle_mesh_slicer.rs:349` removes only *exactly* collinear vertices (integer `cross == 0` test). OrcaSlicer additionally runs a real Douglas-Peucker pass with a tunable `resolution` (default 0.01 mm) right before emit. We have neither the algorithm nor the knob.
3. **Clipper2 offset arc tolerance is hardcoded `0.0`** in `crates/slicer-core/src/polygon_ops.rs:207`. Clipper2 treats `0.0` as "use default", which is `delta/500` — extremely fine. Every rounded perimeter corner gets dense vertices, multiplying downstream work.

Additionally, OrcaSlicer's `slice_closing_radius` (default `0.049 mm`) fuses hairline cracks at slice time via an inflate→deflate round-trip. We have no equivalent.

This packet adds seven OrcaSlicer-style numeric knobs to `ResolvedConfig` and wires them through the slice and emit paths. All seven default to OrcaSlicer-aligned values for out-of-the-box parity; setting any to its "legacy" value (`0.0` for the tolerances and radius; `4` for the decimals) reproduces current behavior exactly. No preset enum, no G2/G3 arc fitting.

This is a fresh backlog item — no existing TASK-### in `docs/07_implementation_status.md` covers Douglas-Peucker simplification, Clipper2 arc tolerance, min-segment-length filtering, G-code XY decimals, or slice closing radius. The closest precedents — TASK-153 (per-role feedrate, +26 keys), TASK-154-cooling (+8 keys), TASK-182 (overhang speed, +4 keys), TASK-181 (paint_config namespace) — establish the additive `declare_resolved_config!` pattern this packet replicates.

## Architecture Constraints

- 1 unit = 100 nm (`docs/08_coordinate_system.md`). All mm-to-units conversions go via `* 10_000.0`. The D-P pass operates in mm-space `f32` because G-code emit is already mm-space — no integer trap.
- `ResolvedConfig` extensions are additive and must not change existing field order or visibility (per the macro-driven generator at `slicer-ir/src/resolved_config.rs:67-162` comments). The 7 new fields go inside the existing block, alongside their semantic neighbors (geometry/precision section).
- Manifest schema additions (`[config.schema.<key>]`) follow snake_case headers per `CLAUDE.md` "Config Key Naming Convention".
- Module guest WASM is NOT rebuilt by `cargo build` or `cargo test`. After editing perimeter modules / `slicer-ir` / `slicer-sdk` / `slicer-helpers`, the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and rebuild if STALE (per `CLAUDE.md` "Guest WASM Staleness").
- The slice path (`slicer-core::triangle_mesh_slicer`) runs at host built-in stage; `ResolvedConfig` is already in scope there. No new plumbing.
- The host emit pass already receives `ResolvedConfig` via host-context plumbing (precedent: TASK-153 per-role feedrate, TASK-182 overhang speed, both at `gcode_emit.rs`). No new plumbing.
- `slicer_core::polygon_ops::offset` is called from both host code AND guest WASM modules. The signature change must remain ABI-stable on the Rust side (it is — Rust doesn't have ABI compat constraints between guest and host modules; each compiles its own copy).

## Data and Contract Notes

- **IR contract**: `ResolvedConfig` gains 7 fields; all `Default`-derived; no schema-version bump needed because additive `ResolvedConfig` extensions are within the same IR version family per existing precedent (TASK-153 added 26 keys without a bump; TASK-182 added 4).
- **WIT boundary**: no WIT changes. The new `arc_tolerance_mm` parameter on `polygon_ops::offset` is a Rust-internal signature change; the WIT-level `offset_polygons` host service keeps its current signature (the SDK wrapper just hardcodes `0.0` for now).
- **Manifest contract**: `[config.schema.perimeter_arc_tolerance]` follows the established `wall_count` / `line_width` shape (`type`, `default`, `min`, `max`, `display`, `group`). `min = 0.0` lets legacy mode set `0.0`.
- **Determinism / scheduler**: no scheduler impact. Slice and emit stages already in the pipeline; this packet adds intra-stage simplification, not reordering.

## Locked Assumptions and Invariants

- **Zero-cost legacy path**: when each new key is set to its legacy value (`0.0` for tolerances and radius; `4` for decimals), the produced G-code MUST be byte-identical to pre-packet output. Enforced by NEG-2 (golden test).
- **F and temperature formatting unchanged**: `format_coord(value: f32) -> String` keeps current `{:.4}` behavior. Only the 5 XYZ call sites adopt the new `format_xyz(value, decimals)`. F adoption is a future packet.
- **`simplify_polyline_mm(pts, 0.0)` is identity**: returns the exact input. No allocation strategy change; same `Vec` length and element values.
- **`drop_short_segments_mm` preserves first AND last point**: critical for closed loops (where the first and last point are intentionally equal) and open paths (where the last point is a real endpoint).
- **`slice_closing_radius` round-trip uses `OffsetJoinType::Round`**: matches OrcaSlicer's `PrintObjectSlice.cpp` semantics. Arc tolerance on the round-trip is `0.0` (Clipper2 default) — closing-radius is a topology operation, not a finishing offset.
- **Per-`ExtrusionRole` dispatch table**: travel (`ExtrusionRole::Travel` or equivalent non-extrusion variant) gets tolerance `0.0` (no simplification). The exact `ExtrusionRole` variants the implementer maps must come from the variant list at `slicer-ir/src/slice_ir.rs:1318` — do not invent variants.
- **OrcaSlicer defaults adopted verbatim**: `RESOLUTION`, `SPARSE_INFILL_RESOLUTION`, `SUPPORT_RESOLUTION`, `slice_closing_radius`, `XYZF_EXPORT_DIGITS`. Source citations in `requirements.md`.

## Risks and Tradeoffs

- **Risk: golden bit-rot.** The legacy-mode golden file pinned in `tests/fixtures/golden/` will break on any unrelated G-code emit change (e.g. comment-string change, extruder-temp formatting, header alteration). Mitigation: the golden is small (one fixture STL, small mesh), the legacy mode disables every new feature, so any breakage signals a non-additive change elsewhere — which we want to catch. Reviewer note: when this golden breaks in a future packet, the breaking packet must either restore byte-identity for legacy mode or update the golden with a justification.
- **Risk: per-role dispatch miscategorizes a new `ExtrusionRole` variant.** The `tolerance_for_role` helper must be exhaustive (no wildcard arm that silently swallows new variants). Mitigation: use a `match` with explicit arms and a `// All variants must be enumerated above; new variants intentionally fail compile here.` comment (which IS a non-obvious WHY).
- **Risk: arc-tolerance regressions in non-perimeter callers.** Host call sites and the SDK wrapper currently pass `0.0` (Clipper2 default = `delta/500`, very fine). Threading `0.0` keeps current behavior at those sites. The follow-up packet can introduce per-call-site tolerances.
- **Risk: closing-radius bridges geometry users care about.** A `0.049 mm` radius can fuse intentionally-separate features that are within `0.098 mm`. Mitigation: matches OrcaSlicer's default exactly; legacy mode (`0.0`) opts out.
- **Tradeoff: D-P operates in mm-space f32, not int64 units.** OrcaSlicer's port uses int64 (`coord_t`). We choose mm `f32` because G-code emit is already mm-space. Risk: `f32` precision could mis-order interior vertices for very long polylines (`> 100 m` accumulated length). Acceptable for printer-scale geometry; out-of-scope to switch to f64.
- **Tradeoff: `format_xyz` sibling instead of changing `format_coord` signature.** Two functions instead of one is mildly more surface, but avoids touching F / E / temperature emit, which is what the user explicitly excluded. Net: less risk.
