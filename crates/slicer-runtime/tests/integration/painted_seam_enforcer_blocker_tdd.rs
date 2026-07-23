//! Packet 108 (T-P98-SEAM, D-108-SEAM-CONSUMED): consume painted
//! `seam_enforcer` / `seam_blocker` semantics.
//!
//! AC-5: a `PaintSemantic::Custom("seam_enforcer")` region biases seam
//! selection toward itself (outweighing a higher raw geometric score
//! elsewhere on the same wall loop); a `Custom("seam_blocker")` region
//! excludes its covered corner from `seam_candidates` entirely.
//!
//! AC-N2 (`blocker_exhausts_candidates_preserves_walls_no_seam`): when a
//! blocker excludes the ONLY qualifying corner, `seam_candidates` is empty for
//! that region. Per D-109B-SEAM-FATAL-CORRECTED (superseding packet 108's
//! fatal-on-empty), `seam-placer` degrades GRACEFULLY — it preserves the
//! region's walls in the output and leaves `resolved_seam` unset — rather than
//! aborting the layer, honouring the HIGH-2 wall-preservation invariant and
//! OrcaSlicer's non-abort behaviour.

use std::collections::HashMap;

use classic_perimeters::ClassicPerimeters;
use seam_placer::SeamPlacer;
use slicer_core::perimeter_utils::{apply_seam_paint_bias, SeamCandidate as CoreSeamCandidate};
use slicer_ir::{
    mm_to_units, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, PaintSemantic, PaintValue,
    Point2, Point3, Point3WithWidth, Polygon, SeamCandidate, SeamReason, WallBoundaryType,
    WallFeatureFlags,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_support::fixtures::{
    square_polygon, ConfigViewBuilder, PerimeterRegionViewBuilder,
};
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a small axis-aligned square `ExPolygon` (paint "coverage box") in mm.
fn box_poly(cx_mm: f32, cy_mm: f32, half_mm: f32) -> ExPolygon {
    let half = mm_to_units(half_mm);
    let cx = mm_to_units(cx_mm);
    let cy = mm_to_units(cy_mm);
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: cx - half,
                    y: cy - half,
                },
                Point2 {
                    x: cx + half,
                    y: cy - half,
                },
                Point2 {
                    x: cx + half,
                    y: cy + half,
                },
                Point2 {
                    x: cx - half,
                    y: cy + half,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// A 4-point rectangular wall loop with explicit corners (mm), width `width`.
fn quad_path(corners: [(f32, f32); 4], z: f32, width: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: corners
            .iter()
            .map(|&(x, y)| Point3WithWidth {
                x,
                y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            })
            .collect(),
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    }
}

// ── AC-5 mechanics (slicer-core level): bias math + hard filter ───────────

#[test]
fn apply_seam_paint_bias_shrinks_enforced_score_and_removes_blocked_candidate() {
    let mut candidates = vec![
        CoreSeamCandidate {
            position: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            score: 0.5,
        },
        CoreSeamCandidate {
            position: Point3 {
                x: 5.0,
                y: 0.0,
                z: 0.0,
            },
            score: 0.5,
        },
        CoreSeamCandidate {
            position: Point3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            score: 0.5,
        },
    ];
    // Candidate at (5,0) is enforced; candidate at (10,0) is blocked.
    let enforcer_polys = vec![box_poly(5.0, 0.0, 1.0)];
    let blocker_polys = vec![box_poly(10.0, 0.0, 1.0)];

    apply_seam_paint_bias(&mut candidates, &enforcer_polys, &blocker_polys);

    assert_eq!(
        candidates.len(),
        2,
        "blocked candidate must be removed entirely"
    );
    assert!(
        candidates
            .iter()
            .all(|c| (c.position.x - 10.0).abs() > 0.01),
        "the blocked candidate at x=10.0 must not survive: {:?}",
        candidates.iter().map(|c| c.position.x).collect::<Vec<_>>()
    );
    let unpainted = candidates
        .iter()
        .find(|c| (c.position.x - 0.0).abs() < 0.01)
        .expect("unpainted candidate must survive untouched");
    assert!(
        (unpainted.score - 0.5).abs() < 1e-6,
        "unpainted candidate score must be untouched: {}",
        unpainted.score
    );
    let enforced = candidates
        .iter()
        .find(|c| (c.position.x - 5.0).abs() < 0.01)
        .expect("enforced candidate must survive");
    assert!(
        (enforced.score - 0.05).abs() < 1e-6,
        "enforced candidate score must be multiplied by 0.1: got {}",
        enforced.score
    );
}

// ── AC-5 selection (seam-placer level): bias outweighs geometric score ────

/// Without paint, the candidate at (0,0) naturally wins `SeamMode::Nearest`
/// (lowest `effective_score` — see `select_seam_candidate` /
/// `apply_seam_paint_bias`'s score-direction doc comment: classic-perimeters
/// always emits `SeamReason::Aligned`, zero bonus, so `min_by(score)` decides
/// directly). Applying an enforcer bias to the higher-scored candidate at
/// (5,0) shrinks it below (0,0)'s score, flipping the winner — this is the
/// "bias outweighs geometric score" half of AC-5.
#[test]
fn enforcer_bias_flips_seam_placer_selection() {
    let mut candidates = vec![
        CoreSeamCandidate {
            position: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            score: 0.287,
        },
        CoreSeamCandidate {
            position: Point3 {
                x: 5.0,
                y: 0.0,
                z: 0.0,
            },
            score: 0.492,
        },
    ];
    // Sanity: without bias, (0,0)'s lower score would already win min_by.
    assert!(candidates[0].score < candidates[1].score);

    let enforcer_polys = vec![box_poly(5.0, 0.0, 1.0)];
    apply_seam_paint_bias(&mut candidates, &enforcer_polys, &[]);
    assert!(
        candidates[1].score < candidates[0].score,
        "enforcer bias must flip the ranking: {:?}",
        candidates.iter().map(|c| c.score).collect::<Vec<_>>()
    );

    let path = quad_path([(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)], 0.0, 0.4);
    let ir_candidates: Vec<SeamCandidate> = candidates
        .iter()
        .map(|c| SeamCandidate {
            position: Point3WithWidth {
                x: c.position.x,
                y: c.position.y,
                z: c.position.z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            score: c.score,
            reason: SeamReason::Aligned,
        })
        .collect();

    let num_points = path.points.len();
    let mut region_builder = PerimeterRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .add_outer_wall_with_flags(
            path,
            vec![WallFeatureFlags::default(); num_points],
            WallBoundaryType::Interior,
        );
    for c in ir_candidates {
        region_builder = region_builder.add_seam_candidate(c);
    }
    let region = region_builder.build();

    let config = ConfigViewBuilder::new().build();
    let module = SeamPlacer::on_print_start(&config).expect("on_print_start");
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &[region], &mut output, &config)
        .expect("run_wall_postprocess must succeed");

    let resolved = output
        .resolved_seam()
        .expect("expected a resolved seam (enforcer candidate must be selected)");
    assert!(
        (resolved.point.x - 5.0).abs() < 0.01 && (resolved.point.y - 0.0).abs() < 0.01,
        "expected seam resolved INSIDE the enforcer region at (5,0), got ({}, {})",
        resolved.point.x,
        resolved.point.y
    );
}

// ── AC-5 end-to-end wiring: classic-perimeters consumes segment_annotations ─

/// A painted `seam_blocker` region over one corner of a square wall excludes
/// that corner from the emitted `seam_candidates` — proving the paint signal
/// is actually consumed at candidate-generation time (D-108-SEAM-CONSUMED),
/// not just plumbed through unread as before (D-98-SEAM-NO-CONSUMER).
#[test]
fn classic_perimeters_blocker_excludes_painted_corner() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).expect("on_print_start");
    let paint = PaintRegionLayerView::new(0);

    // Baseline: no annotations, expect 4 candidates (one per 90-degree corner).
    let mut baseline_region = SliceRegionView::default();
    baseline_region.set_object_id("obj-1");
    baseline_region.set_region_id(1);
    baseline_region.set_z(0.2);
    baseline_region.set_polygons(vec![square_polygon(0.0, 0.0, 10.0)]);
    baseline_region.set_infill_areas(vec![square_polygon(0.0, 0.0, 10.0)]);
    let mut baseline_output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &[baseline_region], &paint, &mut baseline_output, &config)
        .expect("baseline run_perimeters");
    let baseline_count = baseline_output.seam_candidates().len();
    assert_eq!(
        baseline_count, 4,
        "expected 4 unblocked seam candidates on an unpainted square, got {baseline_count}"
    );

    // Block vertex index 0 (square_polygon's first contour point).
    let mut ann: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> = HashMap::new();
    ann.insert(
        PaintSemantic::Custom("seam_blocker".to_string()),
        vec![vec![Some(PaintValue::Flag(true)), None, None, None]],
    );

    let mut blocked_region = SliceRegionView::default();
    blocked_region.set_object_id("obj-1");
    blocked_region.set_region_id(1);
    blocked_region.set_z(0.2);
    blocked_region.set_polygons(vec![square_polygon(0.0, 0.0, 10.0)]);
    blocked_region.set_infill_areas(vec![square_polygon(0.0, 0.0, 10.0)]);
    blocked_region.set_segment_annotations(ann);

    let mut blocked_output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &[blocked_region], &paint, &mut blocked_output, &config)
        .expect("blocked run_perimeters");

    assert_eq!(
        blocked_output.seam_candidates().len(),
        baseline_count - 1,
        "blocking one corner must remove exactly one candidate"
    );
}

// ── AC-N2 (corrected, P109 / D-109-SEAM-FATAL-CORRECTED): blocker exhausts
// every candidate → seam-placer degrades GRACEFULLY (walls preserved, no
// resolved seam), never aborts the layer ─────────────────────────────────────

#[test]
fn blocker_exhausts_candidates_preserves_walls_no_seam() {
    let path = quad_path([(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)], 0.0, 0.4);
    // No seam candidates added (all blocked at generation time) and no
    // resolved_seam — but real, non non-planar-shell wall loops exist.
    let region = PerimeterRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .add_outer_wall(path)
        .build();
    assert!(region
        .wall_loops()
        .iter()
        .all(|w| w.loop_type == LoopType::Outer));
    assert!(region.seam_candidates().is_empty());
    assert!(region.resolved_seam().is_none());

    let config = ConfigViewBuilder::new().build();
    let module = SeamPlacer::on_print_start(&config).expect("on_print_start");
    let mut output = PerimeterOutputBuilder::new();

    // A blocker that exhausts every candidate must NOT abort the layer: the
    // region has no usable seam, but its walls MUST still reach the output
    // (HIGH-2 wall-preservation invariant; OrcaSlicer degrades, never crashes).
    module
        .run_wall_postprocess(0, &[region], &mut output, &config)
        .expect("blocker-exhausted region must degrade gracefully, not fail the layer");

    assert!(
        output.resolved_seam().is_none(),
        "no surviving candidate => no resolved seam"
    );
    let rotated = output.rotated_wall_loops();
    assert_eq!(
        rotated.len(),
        1,
        "the region's wall must be preserved in the output even when no seam can be placed"
    );
    let (_, emitted_wall_index, _) = &rotated[0];
    assert_eq!(*emitted_wall_index, 0);
}
