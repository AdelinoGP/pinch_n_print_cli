# Task Map — Packet 126: MMU Painted-Cube OrcaSlicer Parity

This packet spans multiple `docs/07` entries, reopens the Phase-4f port left non-faithful by packet 95, and retires the cell-decomposition / `external_contour` line from packet 96. It also records parity fixes that have no `docs/07` entry yet.

## docs/07 crosswalk

| docs/07 ref | status in docs/07 | Relationship to this packet | Steps |
|---|---|---|---|
| TASK-245 (paint-seg parity ph.1–4,6,7, packet 95) | closed | Completes the Phase-4f `extract_colored_segments` port that packet 95 shipped non-faithful (colour-ignoring walk, per-distinct-colour emission). | 3, 4 |
| TASK-246 (Phase-5 width-limit, packet 95/96) | closed | Same paint-seg parity workstream; decomposition backend swap. | 3, 4, 9, 10 |
| TASK-246-BISECTOR / packet 96 (`external_contour`) | closed (retired by ADR-0013) | The boostvoronoi cell shortcut introduced alongside `external_contour` is retired here; mark packet 96's union-trace superseded. | 1, 4, 10 |
| DEV-009 (live-path quality bar) | open | Colour parity / top-bottom fill on the live path — this packet's G1/G2/G3/G7/G8 close painted-face colour gaps under it. | 5, 6, 7, 8 |
| Roadmap P110 (Voronoi/SKT, draft) | draft | Related but separate (SKT/arachne); this packet is the colour-decomposition slice only — NOT P110. | — |

## Untracked items recorded by this packet (no docs/07 entry)

| Item | Where landed | This packet |
|---|---|---|
| G5/RC5 volumetric flow (`emit.rs`, `resolved_config.rs`) | commit `17bb59bd` | verify (AC-V-G5), record in docs/07 at Step 10 |
| G4/RC4 internal solid infill (`slice_ir.rs`, macros, `leaf.rs`, infill modules) | commit `17bb59bd` | verify (AC-V-G4), record at Step 10 |
| G6 shell inset (`top_bottom.rs`) | commit `17bb59bd` | verify (AC-V-G6) |
| RC1 palette (`loader.rs`, `serialize.rs`) | S1 | verify (AC-V-RC1) |
| RC2 default tool (`layer_executor.rs`) | resolved via RC3 (S1) | verified via AC-V-RC3 / AC-2 |
| RC3 side colours (`painted_line_collection.rs`) | S1/S2 | verify (AC-V-RC3) |
| Corner-displacement removal + confinement-test correction | this session (uncommitted) | commit at Step 1; guarded by AC-N1/AC-N3 |

## Authoritative-doc / OrcaSlicer ref divergence by step

| Step | Primary doc | OrcaSlicer ref |
|---|---|---|
| 3 | docs/02 (graph types) | MultiMaterialSegmentation.cpp:390-456, :1714 |
| 4 | ADR-0013 | MultiMaterialSegmentation.cpp:494-566 |
| 5 (G1) | docs/02 (`top_solid_fill`) | PrintObject.cpp / Fill/ |
| 7 (G3) | docs/01 (PrePass order) | PrintObject.cpp (bottom-shell perimeter colour) |
| 9 | docs/02 (`ExtrusionRole`, `ResolvedConfig`) | GCode.cpp (flow), PrintObject.cpp (solid) |
