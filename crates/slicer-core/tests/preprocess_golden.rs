#![allow(missing_docs)]

//! Golden/black-box tests for `slicer_core::arachne::preprocess` (packet 110
//! step 5: T-204's 9-stage Arachne input-outline pipeline and T-P96-E's
//! per-color validated pass-through).
//!
//! Exercises only the public API (`preprocess_input_outline`,
//! `preprocess_per_color_inputs`, and the pure diagnostic-detection helpers
//! `detect_dropped_features`/`detect_color_overlaps`) — the crate's own
//! `#[cfg(test)]` unit tests inside `arachne/preprocess.rs` cover the
//! private per-stage helpers white-box.
//!
//! The dropped-feature/overlap tests below assert on the pure detection
//! helpers rather than on `log::warn!` output: no log-capture crate exists
//! anywhere in this workspace, and this exercises the real detection logic
//! without depending on the global `log` facade (fragile under parallel test
//! execution) — see `arachne/preprocess.rs`'s "Diagnostics" module doc for
//! the full rationale.

use slicer_core::arachne::preprocess::{
    detect_color_overlaps, detect_dropped_features, preprocess_input_outline_with_stages,
};
use slicer_core::arachne::{
    preprocess_input_outline, preprocess_per_color_inputs, PreprocessParams, ToolIndex,
};
use slicer_core::polygon_ops::{intersection_ex, validate_polygon_simplicity};
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

fn to_units(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM).round() as i64,
        y: (y_mm * UNITS_PER_MM).round() as i64,
    }
}

fn square(x0_mm: f64, y0_mm: f64, side_mm: f64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                to_units(x0_mm, y0_mm),
                to_units(x0_mm + side_mm, y0_mm),
                to_units(x0_mm + side_mm, y0_mm + side_mm),
                to_units(x0_mm, y0_mm + side_mm),
            ],
        },
        holes: Vec::new(),
    }
}

fn polygon_area_mm2(poly: &Polygon) -> f64 {
    let pts = &poly.points;
    if pts.len() < 3 {
        return 0.0;
    }
    let n = pts.len();
    let mut area = 0.0f64;
    for i in 0..n {
        let j = (i + 1) % n;
        area += (pts[i].x as f64) * (pts[j].y as f64);
        area -= (pts[j].x as f64) * (pts[i].y as f64);
    }
    (area.abs() * 0.5) / (UNITS_PER_MM * UNITS_PER_MM)
}

/// Returns `true` if any two consecutive points (ring wrap-around included)
/// in `poly` are exactly coincident, i.e. a zero-length edge. Mirrors
/// `preprocess.rs`'s own `dedup_ring`/stage-4/7 post-condition, but is
/// re-derived independently here (not imported) so the golden test doesn't
/// just call back into the code under test.
fn ring_has_degenerate_edge(poly: &Polygon) -> bool {
    let pts = &poly.points;
    let n = pts.len();
    if n < 2 {
        return false;
    }
    (0..n).any(|i| pts[i] == pts[(i + 1) % n])
}

fn expolygon_has_degenerate_edge(exp: &ExPolygon) -> bool {
    ring_has_degenerate_edge(&exp.contour) || exp.holes.iter().any(ring_has_degenerate_edge)
}

/// Returns `true` if any 3 consecutive vertices (ring wrap-around included)
/// in `poly` are colinear within `angle_tolerance_rad` of the incoming vs.
/// outgoing edge vectors' angle. Independently re-derived from stage 5's
/// `remove_colinear_ring` angle math (same formula) rather than imported, so
/// this checks the actual geometric post-condition rather than trusting the
/// production helper's own bookkeeping.
fn ring_has_colinear_triple(poly: &Polygon, angle_tolerance_rad: f64) -> bool {
    let pts = &poly.points;
    let n = pts.len();
    if n < 4 {
        return false;
    }
    for i in 0..n {
        let prev = pts[(i + n - 1) % n];
        let cur = pts[i];
        let next = pts[(i + 1) % n];
        let v1x = (cur.x - prev.x) as f64;
        let v1y = (cur.y - prev.y) as f64;
        let v2x = (next.x - cur.x) as f64;
        let v2y = (next.y - cur.y) as f64;
        let len1 = (v1x * v1x + v1y * v1y).sqrt();
        let len2 = (v2x * v2x + v2y * v2y).sqrt();
        if len1 <= f64::EPSILON || len2 <= f64::EPSILON {
            continue;
        }
        let cos_theta = ((v1x * v2x + v1y * v2y) / (len1 * len2)).clamp(-1.0, 1.0);
        if cos_theta.acos().abs() <= angle_tolerance_rad {
            return true;
        }
    }
    false
}

fn expolygon_has_colinear_triple(exp: &ExPolygon, angle_tolerance_rad: f64) -> bool {
    ring_has_colinear_triple(&exp.contour, angle_tolerance_rad)
        || exp
            .holes
            .iter()
            .any(|h| ring_has_colinear_triple(h, angle_tolerance_rad))
}

/// Total absolute intersection area (mm^2) between two `ExPolygon` sets,
/// used to assert stage 9's final union leaves no overlapping members.
fn overlap_area_mm2(a: &[ExPolygon], b: &[ExPolygon]) -> f64 {
    intersection_ex(a, b)
        .iter()
        .map(|p| polygon_area_mm2(&p.contour))
        .sum()
}

/// AC-6: the 9-stage pipeline runs to completion on a square with
/// intentionally-redundant vertices (a colinear midpoint on one edge, a
/// duplicate closing point on another), stays a simple (non-self-
/// intersecting) polygon, roughly preserves the original area (the
/// triple-offset in stage 1 is net-zero for features far larger than
/// `epsilon_offset`), and comes out with no more vertices than it started
/// with (colinear-edge and degenerate-vertex removal only ever remove
/// points).
#[test]
fn preprocess_nine_stage_pipeline() {
    // 10mm x 10mm square with a colinear midpoint on the bottom edge
    // (5,0) and a duplicated corner point right after (10,0).
    let input = vec![ExPolygon {
        contour: Polygon {
            points: vec![
                to_units(0.0, 0.0),
                to_units(5.0, 0.0), // colinear -> stage 5 should drop it
                to_units(10.0, 0.0),
                to_units(10.0, 0.0), // duplicate -> stage 4/7 should drop it
                to_units(10.0, 10.0),
                to_units(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }];
    let input_vertex_count = input[0].contour.points.len();
    let input_area_mm2 = polygon_area_mm2(&input[0].contour);

    let params = PreprocessParams::default();
    let output = preprocess_input_outline(&input, &params);

    assert!(!output.is_empty(), "10mm square must survive the pipeline");

    for poly in &output {
        assert!(
            validate_polygon_simplicity(poly).is_ok(),
            "pipeline output must be self-intersection-free: {poly:?}"
        );
    }

    let output_area_mm2: f64 = output.iter().map(|p| polygon_area_mm2(&p.contour)).sum();
    assert!(
        (output_area_mm2 - input_area_mm2).abs() < 0.05,
        "net-zero pipeline should roughly preserve area: input={input_area_mm2} output={output_area_mm2}"
    );

    let output_vertex_count: usize = output.iter().map(|p| p.contour.points.len()).sum();
    assert!(
        output_vertex_count <= input_vertex_count,
        "colinear/degenerate-vertex removal must not increase vertex count: \
         input={input_vertex_count} output={output_vertex_count}"
    );

    // --- AC-6 per-stage strengthening ---------------------------------
    //
    // AC-6 literally requires "the output matches the recorded reference
    // polygons within tolerance per stage", not just at the end-to-end
    // boundary asserted above. `preprocess_input_outline_with_stages`
    // exposes all 9 intermediate stage outputs (test-introspection only —
    // production code keeps calling `preprocess_input_outline`) so each
    // stage's own post-condition can be asserted directly against this same
    // fixture, catching a stage being skipped, reordered, or misapplied
    // even when the final output alone would look plausible.
    let stages = preprocess_input_outline_with_stages(&input, &params);
    assert_eq!(
        stages.len(),
        9,
        "the pipeline must expose exactly 9 stage outputs"
    );
    assert_eq!(
        stages.last(),
        Some(&output),
        "with_stages' final (9th) entry must match preprocess_input_outline's return value"
    );

    // Stage 1 (triple offset, stages[0]): net-zero shrink/grow/shrink should
    // roughly preserve area for a feature this far above epsilon_offset.
    let stage1_area_mm2: f64 = stages[0].iter().map(|p| polygon_area_mm2(&p.contour)).sum();
    assert!(
        (stage1_area_mm2 - input_area_mm2).abs() < 0.05,
        "stage 1 (triple offset) should roughly preserve area: \
         input={input_area_mm2} stage1={stage1_area_mm2}"
    );

    // Stage 2 (simplify, stages[1]): merge-short-segments + RDP only ever
    // remove points, so vertex count must not increase versus stage 1.
    let stage1_vertex_count: usize = stages[0].iter().map(|p| p.contour.points.len()).sum();
    let stage2_vertex_count: usize = stages[1].iter().map(|p| p.contour.points.len()).sum();
    assert!(
        stage2_vertex_count <= stage1_vertex_count,
        "stage 2 (simplify) must not increase vertex count: \
         stage1={stage1_vertex_count} stage2={stage2_vertex_count}"
    );

    // Stages 3 and 6 (fix self-intersections, stages[2] and stages[5]):
    // output must be self-intersection-free after each pass.
    for (stage_idx, stage_polys) in [(3, &stages[2]), (6, &stages[5])] {
        for poly in stage_polys {
            assert!(
                validate_polygon_simplicity(poly).is_ok(),
                "stage {stage_idx} (fix self-intersections) output must be simple: {poly:?}"
            );
        }
    }

    // Stages 4 and 7 (remove degenerate vertices, stages[3] and stages[6]):
    // no zero-length edges / coincident consecutive vertices may remain.
    // The fixture's intentional duplicate point at (10,0) must not survive
    // past either pass.
    for (stage_idx, stage_polys) in [(4, &stages[3]), (7, &stages[6])] {
        for poly in stage_polys {
            assert!(
                !expolygon_has_degenerate_edge(poly),
                "stage {stage_idx} (remove degenerate verts) must leave no zero-length \
                 edges: {poly:?}"
            );
        }
    }

    // Stage 5 (remove colinear edges, stages[4]): no 3 consecutive vertices
    // may remain colinear within `colinear_angle_tolerance_rad`. The
    // fixture's intentional colinear midpoint at (5,0) is the case this
    // guards against.
    for poly in &stages[4] {
        assert!(
            !expolygon_has_colinear_triple(poly, params.colinear_angle_tolerance_rad),
            "stage 5 (remove colinear edges) must leave no colinear vertex triples: {poly:?}"
        );
    }

    // Stage 8 (remove small areas, stages[7]): every surviving contour must
    // meet the configured `small_area_length_mm^2` threshold. With this
    // fixture's 10mm square (100 mm^2) far above the ~0.25 mm^2 default
    // threshold, a broken units conversion that inflated the threshold
    // would show up here as the square itself getting removed.
    let small_area_threshold_mm2 = params.small_area_length_mm * params.small_area_length_mm;
    for poly in &stages[7] {
        let area_mm2 = polygon_area_mm2(&poly.contour);
        assert!(
            area_mm2 + 1.0e-9 >= small_area_threshold_mm2,
            "stage 8 (remove small areas) left a contour below its own threshold: \
             area={area_mm2} threshold={small_area_threshold_mm2}"
        );
    }

    // Stage 9 (final union, stages[8]): a union's output members must not
    // overlap each other.
    for i in 0..stages[8].len() {
        for j in (i + 1)..stages[8].len() {
            let overlap = overlap_area_mm2(&stages[8][i..=i], &stages[8][j..=j]);
            assert!(
                overlap < 1.0e-6,
                "stage 9 (final union) must not leave overlapping members: \
                 poly {i} and poly {j} overlap by {overlap} mm^2"
            );
        }
    }
}

/// AC-6 per-stage strengthening, stage 9 specifically: two separate input
/// squares that overlap by 5mm x 10mm must come out of the final union
/// (stage 9) merged into non-overlapping output, with total area close to
/// the expected union area. The single-polygon fixture used by
/// `preprocess_nine_stage_pipeline` above never exercises this — it only
/// ever has one contour to union, so a union that silently failed to merge
/// overlapping members would go unnoticed there. This fixture exists
/// specifically to make stage 9's "no overlapping members" property
/// falsifiable.
#[test]
fn preprocess_stage_nine_union_merges_overlapping_squares() {
    let input = vec![square(0.0, 0.0, 10.0), square(5.0, 0.0, 10.0)];
    // 10x10 + 10x10 squares overlapping in a 5x10 strip: 100 + 100 - 50 = 150 mm^2.
    let expected_union_area_mm2 = 150.0;

    let params = PreprocessParams::default();
    let stages = preprocess_input_outline_with_stages(&input, &params);
    assert_eq!(stages.len(), 9);
    let stage9 = &stages[8];

    assert!(
        !stage9.is_empty(),
        "overlapping squares must survive the pipeline"
    );

    for i in 0..stage9.len() {
        for j in (i + 1)..stage9.len() {
            let overlap = overlap_area_mm2(&stage9[i..=i], &stage9[j..=j]);
            assert!(
                overlap < 1.0e-6,
                "stage 9 must merge overlapping input into non-overlapping output: \
                 poly {i} and poly {j} overlap by {overlap} mm^2"
            );
        }
    }

    let stage9_area_mm2: f64 = stage9.iter().map(|p| polygon_area_mm2(&p.contour)).sum();
    assert!(
        (stage9_area_mm2 - expected_union_area_mm2).abs() < 0.5,
        "stage 9 union area should be close to the expected merged area: \
         expected={expected_union_area_mm2} actual={stage9_area_mm2}"
    );
}

/// AC-N3: a feature far smaller than `epsilon_offset` is dropped by the
/// pipeline, and a diagnostic naming its centroid is emitted.
#[test]
fn preprocess_drops_tiny_features_with_warn() {
    let big = square(0.0, 0.0, 10.0);
    // epsilon_offset defaults to ~0.0125mm; this loop is 0.001mm wide,
    // more than an order of magnitude below threshold.
    let tiny = square(50.0, 50.0, 0.001);
    let input = vec![big, tiny];

    let params = PreprocessParams::default();
    let output = preprocess_input_outline(&input, &params);
    let dropped = detect_dropped_features(&input, &output, params.epsilon_offset_mm);

    // The 10mm square must survive with roughly its original area; the
    // tiny square must not contribute any surviving area near (50, 50).
    let survives_near_tiny = output.iter().any(|p| {
        let (min_x, max_x) = p
            .contour
            .points
            .iter()
            .fold((i64::MAX, i64::MIN), |(mn, mx), pt| {
                (mn.min(pt.x), mx.max(pt.x))
            });
        let tiny_x_units = (50.0 * UNITS_PER_MM) as i64;
        min_x <= tiny_x_units && tiny_x_units <= max_x
    });
    assert!(
        !survives_near_tiny,
        "sub-epsilon feature must not survive preprocessing; output={output:?}"
    );

    assert!(
        !dropped.is_empty(),
        "dropping a sub-epsilon feature must be reported by detect_dropped_features"
    );
    // tiny = square(50.0, 50.0, 0.001) -> centroid (50.0005, 50.0005) mm.
    assert!(
        dropped
            .iter()
            .any(|&(cx, cy)| (cx - 50.0005).abs() < 1.0e-3 && (cy - 50.0005).abs() < 1.0e-3),
        "expected the tiny feature's centroid near (50.0005, 50.0005) mm, got: {dropped:?}"
    );
}

/// AC-7 (T-P96-E, corrected to a validated pass-through — see
/// `preprocess_per_color_inputs`'s doc-comment): a 2x2 grid of unit squares,
/// each a different `ToolIndex`, sharing edges at the grid lines.
#[test]
fn preprocess_per_color_mmu_dedup() {
    let cells: Vec<(ToolIndex, Vec<ExPolygon>)> = vec![
        (0u32, vec![square(0.0, 0.0, 1.0)]),
        (1u32, vec![square(1.0, 0.0, 1.0)]),
        (2u32, vec![square(0.0, 1.0, 1.0)]),
        (3u32, vec![square(1.0, 1.0, 1.0)]),
    ];

    let out1 = preprocess_per_color_inputs(&cells);

    // (a) cell count preserved.
    assert_eq!(out1.len(), cells.len());

    // (b) each output cell's polygon is unchanged from its input (exact,
    // since this is a pass-through — no floating point ever enters).
    for (expected, actual) in cells.iter().zip(out1.iter()) {
        assert_eq!(
            expected.0, actual.0,
            "tool index must be preserved in order"
        );
        assert_eq!(
            expected.1, actual.1,
            "cell geometry must pass through unmodified for color {}",
            expected.0
        );
    }

    // (c) union of all output cells covers the union of all input cells
    // within epsilon: since output == input exactly, total areas match
    // exactly (well within any epsilon).
    let input_area: f64 = cells
        .iter()
        .flat_map(|(_, polys)| polys.iter())
        .map(|p| polygon_area_mm2(&p.contour))
        .sum();
    let output_area: f64 = out1
        .iter()
        .flat_map(|(_, polys)| polys.iter())
        .map(|p| polygon_area_mm2(&p.contour))
        .sum();
    assert!(
        (input_area - output_area).abs() < 1.0e-9,
        "pass-through must preserve total area: input={input_area} output={output_area}"
    );
    assert!((input_area - 4.0).abs() < 1.0e-9, "4 unit squares == 4mm^2");

    // (d) determinism: running twice produces identical output ordering and
    // geometry.
    let out2 = preprocess_per_color_inputs(&cells);
    assert_eq!(
        out1, out2,
        "preprocess_per_color_inputs must be deterministic"
    );

    // No overlap beyond epsilon between these cells (they only touch at
    // grid-line edges), so no overlap should be detected for this call.
    let overlaps = detect_color_overlaps(&cells);
    assert!(
        overlaps.is_empty(),
        "non-overlapping grid must not trigger the overlap diagnostic: {overlaps:?}"
    );
}

/// AC-7 negative case: cells that genuinely overlap beyond epsilon trigger
/// the diagnostic but are still passed through unmodified (no silent
/// contraction/repair).
#[test]
fn preprocess_per_color_inputs_warns_on_overlap_but_passes_through() {
    let cells: Vec<(ToolIndex, Vec<ExPolygon>)> = vec![
        (0u32, vec![square(0.0, 0.0, 2.0)]),
        // Deliberately overlaps color 0's cell by 1mm x 2mm (upstream
        // partition bug simulation).
        (1u32, vec![square(1.0, 0.0, 2.0)]),
    ];

    let out = preprocess_per_color_inputs(&cells);
    let overlaps = detect_color_overlaps(&cells);

    assert_eq!(
        out, cells,
        "overlap must not change the pass-through output"
    );
    assert!(
        overlaps
            .iter()
            .any(|&(a, b, area)| a == 0 && b == 1 && area > 0.0),
        "overlapping cells must be reported by detect_color_overlaps: {overlaps:?}"
    );
    // Overlap of two 2mm x 2mm squares offset by 1mm in x: 1mm x 2mm = 2mm^2.
    assert!(
        overlaps
            .iter()
            .any(|&(a, b, area)| a == 0 && b == 1 && (area - 2.0).abs() < 1.0e-6),
        "expected ~2.0 mm^2 overlap area, got: {overlaps:?}"
    );
}
