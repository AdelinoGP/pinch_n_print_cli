---
status: implemented
packet: 133_infill-linker-module
task_ids: [TASK-258]
---

# 133_infill-linker-module

## Goal

Ship `modules/core-modules/infill-linker/` — the single `Layer::InfillPostProcess` module (holding new non-fill claim `claim:infill-link`) that reads `prior-infill`, applies the OrcaSlicer overlap semantics, re-clips via `clip_polylines`, filters short segments, and connects raw infill into linked polylines per (region, role) and across wall-sharing groups, emitting the complete replacement `InfillIR`.

## Problem Statement

Architecture A (ADR-0025: modules emit raw; the linker connects) had no linker module: raw infill would ship as disjoint segments with maximum travel. The ported algorithms (`ExPolygonWithOffset`, `BoundaryInfillGraph`, `connect_infill`, `chain_or_connect_infill`, `remove_short_polylines`) live IN the module per ADR-0026, not in `slicer-core`. The module links whatever the current modules emit — including today's stub output — from the day it lands.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Orca constants ÷100 (`docs/08_coordinate_system.md`).
- Linking is the linker's sole responsibility (ADR-0025/0026 + amendments): overlap offset application (`INFILL_OVERLAP_OVER_SPACING = 0.45 × spacing`), re-clip, short-segment filter (< 0.8 × spacing), and connection all happen here. Modules emit pure geometry over the unoffset wall-inset polygon.
- The linker resolves a boundary PER (region, role) from that role's own host-partitioned polygon (ADR-0025 2026-07-24 amendment: per-role re-clip; `infill_areas` union is NOT a substitute); `InternalSolidInfill` maps to the union of the two solid-shell polygons. `for_role` returns `Option`: `None` = no boundary resolved → pass-through; `Some(empty)` = role clipped away.
- Connectors route along the boundary contour (`contour_connector` materialises ring vertices, lerping z/width; `BoundaryRing::directed_distance` gives walk direction); connectors never cross rings — endpoints not on the same ring stay unconnected.
- Cross-region connection restricted to wall-sharing groups (same object/tool/role, path-compatible): same-config siblings union-then-link with majority-length bucket ownership (tie → lower region-id); different-config siblings link per-region with NO overlap inset on wall-less shared arcs (a uniform inset would leave a `2 × 0.45 × spacing` unfilled ring).

## Data and Contract Notes

- Consumes `prior-infill` (packet 130) read-only; emits via `InfillOutputBuilder` (`begin_region` per bucket — packet-127 origin propagation applies); full re-emit: every input bucket appears in the output (transformed or passed through, e.g. ironing pass-through byte-identical).
- Determinism contract: candidate endpoints sorted by (arc-position, segment index), never HashMap iteration; identical inputs → identical `InfillIR` (packet design.md; codified in ADR-0025 2026-08-05 amendment).
- Spacing derivation per (region, role): `line_width / infill_density` for sparse, line-width-based for solid roles, read through the packet-131 accessor; the linker never guesses from path widths EXCEPT the cross-region compatibility predicate (endpoint widths), deliberately path-observable.
- Manifest: `claim:infill-link` (non-fill, first-winner dedup like `claim:ironing`; NOT in `FILL_CLAIM_IDS`); `infill_overlap` config schema in the linker's manifest; `infill_anchor` / `infill_anchor_max` float-or-percent keys resolved against extrusion-flow spacing (solid/bridge buckets force unlimited).
- Anchor-length rule ported: whole-arc-vs-stub branch merges when arc < `anchor_length_max`, else emits `anchor_length`-long lerped stubs; candidates consumed shortest-first (replaces the pre-fix lexicographic-by-endpoint ordering — recorded in DEVIATION_LOG).
- Parity residuals recorded in `docs/DEVIATION_LOG.md`: DEV-110 (ContourIntersectionPoint neighbour bookkeeping not ported — PnP clamps the stub and consumes both endpoints), DEV-112 (percent base uses module `line_width` rather than canonical per-role `frInfill` flow width), the percent-form transport finding, and the accepted `line_width` de-deadening behavior move. `infill_density` deliberately left an undeclared dead read (DEV-114) — declaring it would move spacing on every non-default-density slice.
- Orca ports carry the standard attribution header (`offset.rs`/`graph.rs`/`connect.rs` per `docs/ORCASLICER_ATTRIBUTION.md`).

## Locked Assumptions and Invariants

- The two linking branches and the predicate are locked (ADR-0025 §Amendment); do not re-open cross-wall connection.
- Wall-backed region boundaries are never crossed (AC-N1); different tools never connect (AC-N2).
- Roles and speed factors are never merged across buckets (AC-5).
- First-winner dedup covers the new claim (AC-N3, scheduler test).
- The linker is pipeline-transparent on 131/132/134/135 fixtures (AC-10 executor smoke).

## Risks and Tradeoffs

- The linker is REQUIRED infrastructure in the default dispatch graph — without it, raw disjoint segments (degraded-not-failed per ADR-0025; pinned by packet 136 AC-N1).
- Re-clipping already-clipped segments is not redundant — segments were never clipped to the offset boundary, only the wall-inset.
- Lightning-infill's transitional self-linking: paths carry no module identity, so pass-through detection is unreliable; the real fix is the lightning raw-emit conversion (packet 140).

## Implementation Deviations (recorded at close)

The two containment holes closed 2026-07-24 (per-role boundary resolution + contour routing of connectors) are recorded in ADR-0025's 2026-07-24 amendment, not as deviations. Doc Impact: `docs/03_wit_and_manifest.md` claim row + `docs/01_system_architecture.md` inventory mention (in-packet).
