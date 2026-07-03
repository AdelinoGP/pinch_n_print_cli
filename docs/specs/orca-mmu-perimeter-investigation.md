# OrcaSlicer MMU Perimeter Investigation — Packet 105 (T-P96-A0)

## Status

Verified 2026-06-22; supersedes prior draft (which incorrectly stated overlap-and-trace-both). Authoritative for packet 105 design.

**Post-P105 note (2026-07-02):** the `bisector_edge_skip_mask` this document describes as "structural metadata" was a P105 *draft* design that never shipped — the mask, its host populator, and its WIT/SDK accessors were fully removed at P105 close because Model A (ADR-0013 as rewritten) needs no skip data and no consumer existed (removed 2026-06-23, Packet 105; see git history). The partition/both-sides-trace findings below remain authoritative; read the mask paragraphs as historical design context only.

## Files inspected

- `MultiMaterialSegmentation.cpp:138-161` — NON_BORDER arc creation (color=-1, shared between colors)
- `MultiMaterialSegmentation.cpp:214-219` — BORDER arc color assignment
- `MultiMaterialSegmentation.cpp:396-406` — `get_all_next_arcs()` filters BORDER arcs by color; NON_BORDER arcs (color=-1) pass through for both traversals
- `MultiMaterialSegmentation.cpp:509-534` — per-color ExPolygon construction via leftmost-arc walk
- `PerimeterGenerator.cpp:1630-1631` — per-region offset by `ext_perimeter_width/2`

## Per-color ExPolygon construction

Per-color ExPolygons form a Voronoi **PARTITION** (non-overlapping). The leftmost-arc walk starts from BORDER arcs for a given color and follows `BORDER → NON_BORDER → … → BORDER`, where NON_BORDER arcs (color=-1) pass through for both color traversals. This partitions the interior into non-overlapping cells that meet at shared bisectors.

## Bisector emission

Each per-region ExPolygon includes the bisector as part of its boundary. `PerimeterGenerator.cpp:1630-1631` offsets the entire boundary inward by `ext_perimeter_width/2`. Both regions do this independently, producing two perimeters spaced one full extrusion width apart — touching but not overlapping geometrically.

## Skip-mask concept

OrcaSlicer has **no skip-mask concept** (`rg` for `skip_mask|edge_skip|bisector_mask|shared_edge|perimeter_mask`: zero matches). The packet's *drafted* `bisector_edge_skip_mask` was framed as structural metadata, NOT a skip predicate — and was subsequently dropped entirely at P105 close (D-105-BISECTOR-MASK-DROPPED): nothing consumed it, so no mask exists in the shipped tree.

## Default for this packet

At a shared bisector, BOTH cells trace their respective outer walls (OrcaSlicer parity). No tie-break is needed; OrcaSlicer parity is partition-based, both sides trace. *(Historical draft, not shipped:)* the mask was to be set `true` for edges that are shared bisectors (in BOTH cells' per-cell views), `false` for non-bisector edges, for downstream consumers only (seam placement, role distinction) — never suppressing emission. As shipped, P105 dropped the mask entirely (D-105-BISECTOR-MASK-DROPPED); the both-sides-trace behavior stands without it.

## Single-color baseline

When there is no MMU paint, all per-cell masks are all-false (no bisectors to mark), and each cell's outer wall is emitted exactly once. Matches the unpainted baseline.
