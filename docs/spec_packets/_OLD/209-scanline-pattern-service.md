---
status: implemented
packet: 209-scanline-pattern-service
task_ids:
  - TASK-325
---

# 209-scanline-pattern-service

## Goal

Make the three independent scan-line fill copies — `scan_expolygon`
(`modules/core-modules/rectilinear-infill/src/lib.rs`), `TraditionalSupport::fill_expolygon`
(`modules/core-modules/traditional-support/src/lib.rs`) and `SupportSurfaceIroning::fill_expolygon`
(`modules/core-modules/support-surface-ironing/src/lib.rs`) — **agree on one canonical scan-line
behaviour**, each keeping its own implementation, so the correctness divergence between them is
removed while the duplication itself is left to a future WIT pattern-services packet.

## Problem Statement

There are **three** live copies of the same scan-line fill skeleton — flatten edges to
`(i64,i64,i64,i64)`, take a y-bbox, walk `scan_y`, sort x-intersections, pair them, emit
`ExtrusionPath3D`:

- `scan_expolygon` (`modules/core-modules/rectilinear-infill/src/lib.rs`) — rotates by −angle about a
  per-ExPolygon unrotated-bbox centre, applies an `x_shift`, uses `adjust_solid_spacing` for solid
  roles, a half-open vertex test (`scan_y < lo || scan_y >= hi`), an **inclusive** grid
  (`scan_y = rmin_y; while scan_y <= rmax_y`), a sub-spacing bail, a zero-length-span drop, and a
  top-boundary post-pass over `rotated_contour`.
- `TraditionalSupport::fill_expolygon` (`modules/core-modules/traditional-support/src/lib.rs`) —
  rotates about the world origin, uses a **strictly-between** test
  (`scan_y > edge_min_y && scan_y < edge_max_y`), starts at `min_y + line_spacing`, has **no**
  zero-length-span drop, and has a centroid fallback with no canonical analog.
- `SupportSurfaceIroning::fill_expolygon`
  (`modules/core-modules/support-surface-ironing/src/lib.rs`) — a real third copy with its own
  `collect_edges`, the same strictly-between test and the same `min_y + line_spacing` start, no
  zero-length-span drop, no rotation at all (axis-aligned scan) and no centroid fallback. DEV-127's
  row names only the first two. It is **in scope now**: reconciling without it leaves the divergence
  half-fixed.

**The real correctness bug is the strictly-between test.** At a vertex lying exactly on a scan line
with one neighbour above and one below — a true crossing — it drops **both** incident edges' events,
losing a crossing. The row's intersection list then pairs incorrectly and inside/outside inverts for
the remainder of that scan row. Support and ironing both carry it. Canonical
`slice_region_by_vertical_lines` (`FillRectilinear.cpp`) keeps a true crossing **exactly once** (two
raw events, collapsed to one by the post-sort compaction) and drops a tangential touch entirely.

The half-open test in `rectilinear-infill` already reproduces the canonical **crossing** count
exactly (the lower edge's event is included at `lo`, the upper edge's is excluded at `hi`). It
differs from canonical only at a tangential touch, where it yields two coincident events that its
zero-length-span rule annihilates — coverage-identical to canonical, but splitting one span into two
abutting spans when the touch lies inside a span. That residual is filed, not fixed
(`D-209-TANGENTIAL-TOUCH-SPAN-SPLIT`).

Two further axes diverge without either side being canonical. The **rotation reference** (bbox centre
vs world origin) is geometrically inert here, because both anchor the scan grid at the rotated-space
bbox extreme; the difference is that bbox-centre rotation is *exactly* translation-equivariant while
world-origin rotation drifts by up to ~1 unit (100 nm) with plate position through `rotate_point`'s
`round()`. Canonical rotates about the origin but derives its phase from `align_to_grid` against
`_infill_direction`'s object-bbox anchor, which PnP ports in neither copy. The **scan grid** is
worse: `rectilinear-infill` is inclusive (`floor(h/s)+1` lines), support and ironing are exclusive
from `min + s` (`scan_y = min_y + line_spacing; while scan_y < max_y`, which emits `ceil(h/s) - 1`
lines — *not* `floor((h-s)/s)+1`; that expression reduces to `floor(h/s)` and is off by one at exact
multiples, e.g. it predicts 5 lines for the 10 mm / 2 mm fixture case where the code emits 4), and
canonical is half-open from the bbox min (`ceil(h/s)`) — **no copy matches canonical.**

Existing coverage is stale and thin. `traditional-support` and `support-surface-ironing` have **zero**
geometric-invariant tests: nothing covers the vertex test, the scan start, translation invariance or
zero-length spans. `docs/adr/0009-raft-as-layer-infill-role.md` acknowledges the duplication and
defers it, but all three of its pointers have rotted (see §In Scope).

This is one coherent slice because the three copies cannot be made to agree one axis at a time
without leaving intermediate states where two of them disagree with the third.

## Architecture Constraints

- **No file may be created under `crates/slicer-core/`, and no existing `slicer-core` file may be
  edited.** This is the ADR-0026 boundary and the reason this packet exists in its current form. If an
  implementation step starts to want a shared helper, stop and re-read ADR-0026 §Future-Reviewer
  Notes rather than negotiating with it.
- No WIT, IR, manifest or schema-version change. Every edit is inside a module's private geometry, or
  in a module's own test file, or in docs. No struct-literal blast radius and no version-constant
  fallout follows from this packet.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Unit-boundary rule specific to this packet: every scan-line computation stays in **integer units**.
  `slicer_ir::ExPolygon` points are `slicer_ir::Point2 { x: i64, y: i64 }` in units (note: **not**
  `slicer_core::arachne::sparse_point_grid::Point2`, an unrelated same-named type in a crate all three
  modules depend on — keep both names crate-qualified in prose and code). The `mm_to_units` conversion
  of `line_width / density` and the `units_to_mm` conversion into `ExtrusionPath3D` points already sit
  at each module's boundary and stay there. No mm value may be introduced into the scan loop; that is
  the single largest source of porting error in a scan-line routine.

## Data and Contract Notes

- IR/manifest contracts: none changed. `ExtrusionPath3D`, `ExtrusionRole`, `Point3WithWidth`,
  `InfillOutputBuilder`, `SupportOutputBuilder` are used exactly as today.
- WIT boundary: untouched. All three modules are guests; only the `ExtrusionPath3D`s they already emit
  cross the boundary.
- Determinism/scheduler constraints: each `fill_expolygon` / `scan_expolygon` must remain a pure
  function of its inputs — no interior mutability, no floating-point accumulation across scan rows
  (`scan_y` is an `i64` accumulator and stays one), no iteration over a `HashMap`. Sort x-intersections
  with a total order on `i64`. No stage, claim or dependency edge changes.

## Locked Assumptions and Invariants

- **Locked: no `slicer-core` file is created or edited.** ADR-0026. AC-2 enforces it. This is the
  single constraint that defines the packet's shape; a step that violates it has failed regardless of
  its test results.
- **Locked: three copies remain.** DEV-127 stays Open (AC-13). No artifact, deviation row, commit
  message or report may state or imply the duplication is removed.
- **Locked: the vertex contract is `true crossing → exactly 1 intersection`, `tangential touch → no
  net span`,** realised as the half-open predicate on non-scan-parallel edges plus the
  zero-length-span drop, in all three copies. Reintroducing a strictly-between test anywhere is a
  regression; AC-N3 and AC-N4 catch it.
- **Locked: the scan grid is canonical half-open** — first line at the rotated-space bbox min, bound
  `scan_y < rmax_y`, `ceil(h/s)` lines, **no** sub-spacing bail. This moves shipped infill geometry,
  by design (`D-209-HALF-OPEN-SCAN-GRID-ADOPTED`). The `line_spacing <= 0` guard and the degenerate
  bbox guard are *different* conditions and must survive (AC-N1, AC-N2).
- **Locked: `adjust_solid_spacing`'s arithmetic is byte-for-byte preserved and stays private to
  `rectilinear-infill`.** Only its attribution changes. Its divergence from
  `Fill::_adjust_solid_spacing` (`FillBase.cpp`) is **exactly three axes** — `(width - EPSILON)` as
  canonical's **numerator** in both expressions (never describe it as a divisor; that inverts it),
  truncation vs `.round()`, and `floor(distance * 1.2 + 0.5)` vs the original `distance` on the
  over-cap branch. Canonical's `number_of_intervals == 0 → return distance` guard is **not** a fourth
  axis; PnP's `if count < 1 { return distance; }` is that same guard.
- **Locked: `support-surface-ironing` gains no rotation.** AC-9.
- **Not locked:** whether `TraditionalSupport::fill_expolygon` keeps its current signature or gains a
  `refpt` helper; whether the half-open predicate is written as `scan_y >= lo && scan_y < hi` or as
  `scan_y < lo || scan_y >= hi { continue }`. Either satisfies the ACs.

## Risks and Tradeoffs

- **Shipped infill geometry moves, and it is the highest-blast-radius module.**
  `rectilinear-infill` feeds sparse, top-solid, bottom-solid, internal-solid and bridge roles. Two
  changes compound: the grid loses its top line, and sub-spacing regions gain one. Expect exactly four
  fixture count re-baselines: `square_10mm_density_20_emits_n_raw_segments` 6 → 5,
  `polygon_with_hole_segments_split_around_hole` 8 → 7,
  `solid_spacing_adjusted_for_solid_role` 5 → 4 with its y-set losing the 10.0 mm top-boundary line,
  and `very_small_polygon_emits_no_paths_without_panic` → `very_small_polygon_emits_one_scan_row_without_panic` 0 → 1. **A count change in any of `rectilinear_infill_tdd`,
  `top_bottom_fill_tdd` or `bridge_infill_emission_tdd` is a defect, not a re-baseline candidate** —
  those three assert relations, not absolute counts, and must pass untouched (AC-12).
- **Support and ironing output moves more.** Each gains a scan line at the bbox min and shifts every
  other line by one spacing, and support loses its centroid fallback. Both are intended and both are
  filed. Neither module has any self-captured baseline, so nothing will detect further drift —
  Steps 2 and 3 author the first geometric coverage either has ever had.
- **The strictly-between → half-open swap is a real correctness fix with a real output change.**
  Every support/ironing region whose contour has a vertex on a scan line currently loses a crossing
  and inverts the remainder of that row. Fixing it changes those rows visibly. That is the point.
- **The zero-length-span drop is easy to forget** in support and ironing, and omitting it is silent:
  the half-open test then emits a 2-point path with identical endpoints at every tangential local
  minimum. AC-N3 exists to catch exactly that.
- **Deleting the top-boundary post-pass and the sub-spacing bail in the same step as the grid change
  makes attribution hard if something breaks.** Change the bound first, run
  `rectilinear_raw_emit_tdd`, capture the counts, then delete. `target/test-output.log` is overwritten
  on every run.
- **The forbidden design will reassert itself.** Three copies making the same four changes is exactly
  the situation that motivates a shared kernel. It is forbidden. Any implementer who finds themselves
  drafting `crates/slicer-core/src/scanline_fill.rs` has re-derived the rejected proposal and must
  stop.
- **All three modules are guests.** `cargo xtask build-guests --check` after each of Steps 1-3, and a
  rebuild if `STALE:`, before attributing any failure to the reconciliation.
