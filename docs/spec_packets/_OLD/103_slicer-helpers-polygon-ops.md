---
status: implemented
packet: 103_slicer-helpers-polygon-ops
task_ids:
  - T-040
  - T-041
  - T-042
  - T-043
  - T-044
  - T-045
---

# 103_slicer-helpers-polygon-ops

## Goal

Add six dual-use polygon-op primitives to `slicer-core` — `offset2_ex` / `opening_ex`, `medial_axis` (producing a new `ThickPolyline` IR type with a `variable_width` converter), a hole/contour containment tree builder, `keep_largest_contour_only`, and a promotion of the ray-cast helpers currently inlined in `arachne-perimeters` — so downstream Classic-perimeter (Phase 5/6) and Arachne (M2) work can consume them from one place.

## Problem Statement

Phase 5 (Classic spacing model) and Phase 6 (thin-walls + gap-fill) of the perimeter parity roadmap cannot start until five OrcaSlicer-canonical polygon primitives exist in `slicer-core`: `offset2_ex` (the open-close offset pair used to detect narrow channels and erode-then-dilate gap polygons), `opening_ex` (single-pass open), `medial_axis` (centerline extraction producing variable-width polylines from thin shapes), the hole/contour tree builder (so `process_classic`-style nested traversal can later happen in-module), and `keep_largest_contour_only` (spiral vase + narrow-island handling). All five are absent today. Additionally, the M2 Arachne port will need the same ray-cast helpers that `arachne-perimeters` currently inlines for its width-sampling approximation — those need to live in `slicer-core::geometry` so the M2 real-Arachne module can reuse them. Per `docs/13_slicer_helpers_crate.md` §Out of Scope, per-layer geometry operations belong in `slicer-core`, not `slicer-helpers`.

This packet adds all six primitives in one place. It is fully infrastructural — no perimeter module's wall-emission geometry changes here. The primitives are validated against analytic golden fixtures (a 10 mm square offset-then-expand, a 1×10 mm rectangle medial-axis, etc.) so the work is independently falsifiable without OrcaSlicer-recorded reference outputs.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Schema-version contract: the new types are additive. `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps by one minor (4.1.0 → 4.2.0 if this packet ships first, 4.2.0 → 4.3.0 if packet 100 ships first). `#[serde(default)]` is unnecessary on the new fields because no existing struct gains them; they appear as new top-level types only.
- WIT type identity: `ThickPolyline` and `Point2WithWidth` are new records in `crates/slicer-schema/wit/deps/ir-types.wit`. No existing record type changes shape in this packet.
- Purity invariant: every new `slicer-core` function added by this packet is a pure function over its inputs. No host-services calls, no logging, no global state. This keeps them safely callable from guest WASM later (M2).

## Data and Contract Notes

- IR or manifest contracts touched: `ThickPolyline` and `Point2WithWidth` added; `variable_width` converter introduced. Additive — no existing field shape changes.
- WIT boundary considerations: `thick-polyline` and `point2-with-width` records added. Per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass after the WIT edit before declaring the step done.
- Determinism or scheduler constraints: none. All primitives are pure functions over their inputs. Polygon-tree's child ordering must be deterministic — by ascending `polygon_index` within each parent's children list — to keep downstream consumers' iteration stable.

## Locked Assumptions and Invariants

- Every primitive in `slicer-core` is pure: same inputs → same outputs, no I/O, no logging, no global state. The polygon-tree's child ordering is deterministic by ascending source index.
- `1 unit = 100 nm` is the coordinate system across the workspace (per ADR / `docs/08`). Every mm↔unit boundary in this packet uses `Point2::from_mm` / `mm_to_units` (or the analogous typed helpers); raw `* 10_000.0` arithmetic is forbidden.
- `medial_axis`'s `min_width` and `max_width` are inclusive bounds on per-vertex `width` output; medial-axis paths in regions thinner than `min_width` are dropped, regions wider than `max_width` are not produced (they should be handled by full perimeters instead).
- `offset2_ex(polys, neg_delta, pos_delta, …)` parameter order is **negative first, positive second** — matching OrcaSlicer's `ClipperUtils.cpp` signature. Reversing the argument order is a contract break, not a stylistic choice.
- API matches OrcaSlicer `Geometry.hpp` shape. Future geometry helpers in `slicer-core::geometry` MUST follow this pattern: no positional flat-float args; no tuple returns for named geometric quantities; `Option<>` for ray miss; struct results carrying named fields. Established as the canonical pattern by this packet.
- `Vec2` lives in `crates/slicer-core/src/geometry.rs`, not in `slicer-ir`. It is not a serialized concern. Verify before Step 4: `rg -n 'pub struct Vec2' crates/slicer-ir/src/` must return empty.
- `D-ROADMAP-CRATE-PLACEMENT`: P103's `slicer-helpers`→`slicer-core` correction is one of five packets with the same pattern (P105, P110, P111, P112 also direct per-layer polygon math to `slicer-helpers`, against `docs/13`). Resolution: address per-packet at activation; do NOT batch-rename here. See `docs/13_slicer_helpers_crate.md` §Out of Scope.

## Risks and Tradeoffs

- `medial_axis` correctness vs OrcaSlicer parity: the Rust port targets the documented interface, not the exact C++ algorithm. The rectangle test (AC-2) catches gross-correctness issues; more nuanced parity (acute corners, near-degenerate shapes) will surface during Phase 6 thin-wall integration. Mitigation: AC-2 + AC-N1 catch the most common defect modes (centerline shift, width misreport, degenerate-input crash) early.
- Schema-bump race with packet 100: if packet 100 lands first (4.1.0 → 4.2.0 for `MaterialBoundary` widening), this packet's bump becomes 4.2.0 → 4.3.0. The doc-impact greps allow either bump (the regex matches `4\.[12]\.0.*MaterialBoundary` and `4\.[123]\.0.*ThickPolyline`). Document the ordering rationale in the closure log.
- `keep_largest_contour_only` corner case: ties (two polygons with equal area within float tolerance) must produce a deterministic outcome — pick the polygon with the lower index. Tested in AC-5's test file as a secondary assertion.
