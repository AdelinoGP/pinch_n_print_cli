---
status: implemented
packet: 129_clip-polylines
task_ids:
  - TASK-254
---

# 129_clip-polylines

## Goal

Add `clip_polylines` — a generic Clipper2 open-path intersection of polylines against an
`ExPolygon` set — to `crates/slicer-core/src/polygon_ops.rs`, using `clipper2-rust 1.0.3`'s
native `Clipper64::add_open_subject` + `execute(…, Some(&mut solution_open))` API.

## Problem Statement

The infill-linker (packet 133) must re-clip raw infill segments against overlap-inset
boundaries, and today the only polyline-vs-polygon clipping in the workspace is gyroid's
per-vertex ray-casting `clip_polyline_to_expolygon`
(`modules/core-modules/gyroid-infill/src/lib.rs:611-636`), which misclassifies any segment
whose boundary crossing falls between sample points. The workspace has no generic open-path
clip primitive, although the `clipper2-rust 1.0.3` dependency already exposes one
(`Clipper64::add_open_subject` + `execute` with `solution_open`). The API was recorded from
the crate source `engine_public.rs:296,335` inside the cargo registry — this is a
`clipper2-rust` crate file, NOT a repo path, and is OUT-OF-BOUNDS for reading (verified
2026-07-01; pure Rust, wasm32-clean). Without this primitive, every downstream infill packet
would have to invent its own clipping.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- `polygon_ops` is available on wasm32 and NOT `host-algos`-gated
  (`crates/slicer-core/src/lib.rs:30`); the new function must not introduce a cfg gate or a
  non-wasm32-clean dependency. `clipper2-rust` is pure Rust — no build script, no FFI.

## Data and Contract Notes

- IR or manifest contracts touched: none.
- WIT boundary considerations: none (pure Rust helper), but slicer-core is baked into every
  guest — the freshness gate applies (see Architecture Constraints).
- Determinism: Clipper2 output ordering is deterministic for identical input; tests must not
  assume a specific polyline output ORDER across the result Vec — assert on set membership /
  counts / geometry, not index positions, except where a single output makes index-0 safe.

## Locked Assumptions and Invariants

- `clip_polylines` is generic geometry — it must NOT gain infill-specific parameters (spacing,
  roles, overlap). ADR-0026 locks linking/domain logic in the infill-linker module;
  `slicer-core` gains only this primitive.
- On-edge spans count as inside (Clipper2 boundary rule) — AC-5 pins this; downstream linker
  behavior depends on it.
- The function stays wasm32-clean and un-gated.

## Risks and Tradeoffs

- Clipper2 may merge collinear vertices or split at clip-path vertices, making exact
  point-equality assertions brittle — tests assert geometry within ±2 units tolerance and
  count/coverage properties instead of exact vertex lists (except AC-1's strictly-interior
  case, where no clipping occurs).
- First `Clipper64` builder use in the workspace: if the builder's path-type conversions
  differ from the `engine_fns` layer, the conversion helpers absorb it — keep them private to
  `polygon_ops.rs`.

## Implementation Deviations (recorded at close)

Clipper2's inclusion of open segments lying exactly on a closed clip boundary is side-dependent, so `clip_polylines` pre-inflates the clip universe by 1 unit (100 nm, Miter join, Polygon end type offset) before the single Clipper64 boolean run so that AC-5 (on-boundary segments are kept) holds on all edges. Consequence: hole edges shrink by 1 unit, so returned points may lie up to 1 unit inside an original hole — within the packet's ±2-unit tolerance. Packet 133's author should be aware of this behavior.
