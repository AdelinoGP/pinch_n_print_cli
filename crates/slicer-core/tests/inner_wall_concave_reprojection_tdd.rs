#![allow(missing_docs)]
// inner_wall_concave_reprojection_tdd.rs — concave-shape reprojection TDD test.
//
// Tests that build_wall_flags uses geometric reprojection for inner-wall vertices
// instead of index-based annotation lookup. On concave shapes the inset ring has
// a different vertex count/ordering than the original contour, so index-based
// sampling assigns the WRONG tool to inner-wall vertices near the concavity.
//
// Fixture geometry:
//   Original contour: a concave "notched rectangle" with 8 vertices.
//   The top edge has a rectangular notch cut into it (like a tooth missing from the
//   top-centre of a rectangle). The full shape looks like:
//
//   F---E   D---C
//   |   |   |   |
//   |   +---+   |  ← notch from x=100000 to x=200000 at y=50000..100000
//   |           |
//   G-----------B
//       (origin A at bottom-left, going clockwise)
//
//   Vertex order (CCW winding, scaled units where 1 unit = 100 nm):
//     0: A = (0,       0)       tool 1
//     1: B = (300000,  0)       tool 1
//     2: C = (300000,  100000)  tool 1
//     3: D = (200000,  100000)  tool 2  ← notch-right-top (MIDDLE of boundary)
//     4: E = (200000,  50000)   tool 2  ← notch-right-bottom
//     5: F = (100000,  50000)   tool 2  ← notch-left-bottom
//     6: G = (100000,  100000)  tool 2  ← notch-left-top
//     7: H = (0,       100000)  tool 1
//
//   Paint: vertices 0-2 and 7 → tool 1; vertices 3-6 → tool 2.
//   Material boundary runs between vertex 2→3 and vertex 7→0.
//
// Simulated inner ring (as if from a small inset, with an EXTRA vertex due to
// the Clipper2 offset rounding the concavity):
//   The inner ring has 9 vertices — one extra near the notch corner, causing a
//   ±1 index shift relative to the original.
//
//   Ring vertex 3 (near notch) is geometrically close to ORIGINAL vertex 3
//   (tool 2), but INDEX-based lookup for ring poly_idx=0 goes to
//   segment_annotations[0][3] which might land on the wrong entry when the
//   ring has a different vertex count or ordering.
//
// Specific assertion:
//   Ring vertex 3 is at position (210000, 90000) — geometrically closest to
//   original vertex 3 (200000, 100000) which is tool 2.
//   With reprojection → tool_index == Some(2).  CORRECT.
//   With index-based (None, None on a 9-point ring that still uses poly_idx=0
//   from a 8-entry annotation) → ring vertex 3 is looked up as
//   annotations[0][3] which is also tool 2 in this particular fixture — but
//   ring vertex 4 is at (195000, 45000), closest to original vertex 4 (200000,
//   50000) which is tool 2; index-based looks up annotations[0][4] = tool 2 too.
//
// To make the shortcut clearly WRONG we use a ring that has an EXTRA vertex
// inserted BEFORE vertex 3, shifting all subsequent indices by +1:
//   Ring vertices 0-2: match original 0-2 (tool 1 zone)
//   Ring vertex 3 (INSERTED): at position (250000, 90000) — extra Clipper2
//     artefact near notch; geometrically close to original vertex 2 → tool 1.
//     Index-based reads annotations[0][3] = tool 2. WRONG.
//   Ring vertex 4: at (200000, 90000), near original vertex 3 → tool 2.
//     Index-based reads annotations[0][4] = tool 2. OK here.
//
// The failing assertion: ring vertex 3 (position 250000,90000 near original
// vertex 2 = tool 1) must be tool 1 via reprojection, but index-based assigns
// tool 2. This is the concrete proof that the index shortcut is wrong.

use std::collections::HashMap;

use slicer_core::perimeter_utils::build_wall_flags;
use slicer_ir::{ExPolygon, PaintSemantic, PaintValue, Point2, Polygon, WallBoundaryType};

/// Build the 8-vertex concave notched-rectangle original polygon.
///
/// Shape (all coords in scaled units, 1 unit = 100 nm ≈ 0.1 µm):
/// - Large rectangle 300,000 × 100,000 units (30 mm × 10 mm)
/// - Rectangular notch at top: x=[100000..200000], y=[50000..100000]
fn original_polygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },      // 0: A — tool 1
                Point2 { x: 300000, y: 0 }, // 1: B — tool 1
                Point2 {
                    x: 300000,
                    y: 100000,
                }, // 2: C — tool 1
                Point2 {
                    x: 200000,
                    y: 100000,
                }, // 3: D — tool 2 (notch-right-top)
                Point2 {
                    x: 200000,
                    y: 50000,
                }, // 4: E — tool 2 (notch-right-bottom)
                Point2 {
                    x: 100000,
                    y: 50000,
                }, // 5: F — tool 2 (notch-left-bottom)
                Point2 {
                    x: 100000,
                    y: 100000,
                }, // 6: G — tool 2 (notch-left-top)
                Point2 { x: 0, y: 100000 }, // 7: H — tool 1
            ],
        },
        holes: vec![],
    }
}

/// Annotations for the 8-vertex original polygon:
/// vertices 0,1,2,7 → tool 1; vertices 3,4,5,6 → tool 2.
fn original_annotations() -> HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> {
    let per_vertex = vec![
        Some(PaintValue::ToolIndex(1)), // 0: A
        Some(PaintValue::ToolIndex(1)), // 1: B
        Some(PaintValue::ToolIndex(1)), // 2: C
        Some(PaintValue::ToolIndex(2)), // 3: D — notch-right-top
        Some(PaintValue::ToolIndex(2)), // 4: E
        Some(PaintValue::ToolIndex(2)), // 5: F
        Some(PaintValue::ToolIndex(2)), // 6: G
        Some(PaintValue::ToolIndex(1)), // 7: H
    ];
    let mut ann = HashMap::new();
    ann.insert(PaintSemantic::Material, vec![per_vertex]);
    ann
}

/// Simulated 9-vertex inner ring — produced by a small inset with an EXTRA
/// artefact vertex inserted at position 3 (near x=250000,y=90000).
///
/// The extra vertex is geometrically inside the tool-1 zone (near original
/// vertex C at (300000,100000) → nearest original vertex has tool 1).
///
/// Index-based lookup of ring vertex 3 hits annotations[0][3] = tool 2. WRONG.
/// Reprojection of ring vertex 3 at (250000,90000) finds nearest original
/// vertex 2 (C, 300000,100000) → tool 1. CORRECT.
fn inner_ring_points() -> Vec<Point2> {
    vec![
        Point2 { x: 2000, y: 2000 },   // ring 0 → near orig 0 (A) → tool 1
        Point2 { x: 298000, y: 2000 }, // ring 1 → near orig 1 (B) → tool 1
        Point2 {
            x: 298000,
            y: 98000,
        }, // ring 2 → near orig 2 (C) → tool 1
        // EXTRA vertex inserted by Clipper2 artefact near orig vertex 2 (C):
        Point2 {
            x: 250000,
            y: 90000,
        }, // ring 3 — geometrically tool-1 zone;
        //           index-based reads annotations[0][3] = tool 2 (WRONG)
        //           reprojection → nearest orig vertex = 2 (C, tool 1) (CORRECT)
        Point2 {
            x: 198000,
            y: 98000,
        }, // ring 4 → near orig 3 (D) → tool 2
        Point2 {
            x: 198000,
            y: 52000,
        }, // ring 5 → near orig 4 (E) → tool 2
        Point2 {
            x: 102000,
            y: 52000,
        }, // ring 6 → near orig 5 (F) → tool 2
        Point2 {
            x: 102000,
            y: 98000,
        }, // ring 7 → near orig 6 (G) → tool 2
        Point2 { x: 2000, y: 98000 }, // ring 8 → near orig 7 (H) → tool 1
    ]
}

/// Concave-shape reprojection test: ring vertex 3 must be tool 1 via reprojection.
///
/// Ring vertex 3 is at (250000, 90000) — geometrically in the tool-1 zone, closest
/// to original vertex 2 (C at 300000,100000). The index-based shortcut reads
/// annotations[0][3] = tool 2, which is WRONG for this inner-wall vertex.
///
/// This test MUST fail against the old index-based code path (None, None for
/// inset_ring_points / original_polygons) and PASS with reprojection.
#[test]
fn inner_wall_concave_vertex3_reprojection_gives_tool1() {
    let orig_poly = original_polygon();
    let annotations = original_annotations();
    let ring_pts = inner_ring_points();
    let num_ring_pts = ring_pts.len(); // 9 vertices
    let orig_polys = vec![orig_poly];

    // ── Reprojection path (the correct implementation) ──────────────────────
    let (flags_reproj, _) = build_wall_flags(
        num_ring_pts,
        0,
        &annotations,
        false, // is_outer = false (inner wall)
        Some(&ring_pts),
        Some(&orig_polys),
    );

    // Ring vertex 3 at (250000, 90000) is nearest to original vertex 2 (C, tool 1).
    // Reprojection must produce tool 1, not tool 2.
    assert_eq!(
        flags_reproj[3].tool_index,
        Some(1),
        "reprojection: ring vertex 3 at (250000,90000) must map to tool 1 \
         (nearest orig vertex is C=(300000,100000), tool 1); got {:?}",
        flags_reproj[3].tool_index
    );

    // Sanity: ring vertex 4 at (198000, 98000) is near orig vertex 3 (D, tool 2).
    assert_eq!(
        flags_reproj[4].tool_index,
        Some(2),
        "reprojection sanity: ring vertex 4 near orig D must be tool 2; got {:?}",
        flags_reproj[4].tool_index
    );

    // ── Index-based path (the OLD shortcut) — must give WRONG answer ────────
    // Pass None for inset_ring_points and original_polygons to force index-based
    // lookup. Ring has 9 vertices; annotations have 8 entries (orig polygon).
    // Ring vertex 3 reads annotations[0][3] = tool 2 (D, notch-right-top).
    // This is geometrically WRONG — that entry belongs to the notch, not (250000,90000).
    let (flags_idx_based, _) = build_wall_flags(
        num_ring_pts,
        0,
        &annotations,
        false, // is_outer = false
        None,  // no inset ring positions → index-based fallback
        None,  // no original polygons  → index-based fallback
    );

    // Index-based reads annotations[0][3] = Some(ToolIndex(2)), which is WRONG.
    // This assertion documents the old bug: the shortcut assigns tool 2 to a
    // vertex that is geometrically inside the tool-1 zone.
    assert_eq!(
        flags_idx_based[3].tool_index,
        Some(2),
        "index-based shortcut sanity: ring vertex 3 should still read annotations[0][3]=tool2 \
         (demonstrating the bug); got {:?} — if this is now None, the annotations map changed",
        flags_idx_based[3].tool_index
    );

    // The core assertion: reprojection produces a DIFFERENT (correct) result from
    // index-based for ring vertex 3. This is the TDD proof.
    assert_ne!(
        flags_reproj[3].tool_index, flags_idx_based[3].tool_index,
        "reprojection (tool 1) must differ from index-based (tool 2) for ring vertex 3; \
         both returned {:?} — the concave fixture is not triggering the divergence",
        flags_reproj[3].tool_index
    );
}

/// Regression: convex rectangle inner ring — reprojection must produce SAME result
/// as index-based (on a convex rectangle the vertex ordering is preserved by inset).
///
/// This confirms the reprojection path does not break the simple convex case.
#[test]
fn inner_wall_convex_reprojection_equals_index_based() {
    // 4-vertex convex rectangle (CCW), 10mm × 10mm.
    let orig_poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100000, y: 0 },
                Point2 {
                    x: 100000,
                    y: 100000,
                },
                Point2 { x: 0, y: 100000 },
            ],
        },
        holes: vec![],
    };
    let orig_polys = vec![orig_poly];

    // Annotations: first two vertices tool 1, last two tool 2.
    let per_vertex = vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
    ];
    let mut annotations = HashMap::new();
    annotations.insert(PaintSemantic::Material, vec![per_vertex]);

    // Small inset ring — 4 vertices matching the original's ordering.
    let ring_pts = vec![
        Point2 { x: 2000, y: 2000 },   // near orig 0 → tool 1
        Point2 { x: 98000, y: 2000 },  // near orig 1 → tool 1
        Point2 { x: 98000, y: 98000 }, // near orig 2 → tool 2
        Point2 { x: 2000, y: 98000 },  // near orig 3 → tool 2
    ];

    let (flags_reproj, bt_reproj) = build_wall_flags(
        4,
        0,
        &annotations,
        false,
        Some(&ring_pts),
        Some(&orig_polys),
    );
    let (flags_idx, bt_idx) = build_wall_flags(4, 0, &annotations, false, None, None);

    // Both paths must agree on tool assignments for a convex rectangle.
    for i in 0..4 {
        assert_eq!(
            flags_reproj[i].tool_index, flags_idx[i].tool_index,
            "convex regression: vertex {i} tool_index must agree between reprojection and \
             index-based; reproj={:?} idx={:?}",
            flags_reproj[i].tool_index, flags_idx[i].tool_index
        );
    }

    // Both must detect MaterialBoundary (two transitions).
    assert!(
        matches!(bt_reproj, WallBoundaryType::MaterialBoundary { .. }),
        "convex regression: reprojection path must detect MaterialBoundary; got {bt_reproj:?}"
    );
    assert!(
        matches!(bt_idx, WallBoundaryType::MaterialBoundary { .. }),
        "convex regression: index-based path must detect MaterialBoundary; got {bt_idx:?}"
    );
}
