//! Packet 109 (Step 3 / T-102, AC-3): perimeter-generation edge-case verification.
//!
//! Seven distinct edge cases called out by the M1 audit, each asserting a
//! specific, falsifiable property of the classic-perimeters output — no panics,
//! no silent data loss, correct flag / classification propagation.
//!
//! Harness: the lightest one that meaningfully exercises each case — a direct
//! `ClassicPerimeters::run_perimeters` call over synthetic `SliceRegionView`
//! input (the idiomatic module-unit style used by `thin_wall_emission_tdd`,
//! `gap_fill_emission_tdd`, `extra_perimeters_on_overhangs_tdd`,
//! `mmu_per_color_fragmentation_tdd`, etc.). Per-region output identity is read
//! back via the builder's parallel `wall_loop_origins()` tags (the SDK-side
//! `begin_region` origin), which is how the host distinguishes per-color /
//! per-material `PerimeterRegion`s (ADR-0013 Model A).
//!
//! Coordinate note: `Point2`/`ExPolygon` are integer units (1 unit = 100 nm);
//! `Point3WithWidth` wall vertices are f32 millimeters — the two are never mixed.

use std::collections::{BTreeMap, HashMap};

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{
    mm_to_units, ExPolygon, ExtrusionRole, LoopType, PaintSemantic, PaintValue, Point2, Polygon,
    WallBoundaryType, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};

/// Collect the `Outer`/`Inner` wall loops (the standard perimeter bands, i.e.
/// excluding `ThinWall`/`GapFill`/`NonPlanarShell`).
fn outer_inner_walls(output: &PerimeterOutputBuilder) -> Vec<&WallLoop> {
    output
        .wall_loops()
        .iter()
        .filter(|w| matches!(w.loop_type, LoopType::Outer | LoopType::Inner))
        .collect()
}

/// Mean X (mm) of a wall loop's path points.
fn mean_x_mm(w: &WallLoop) -> f32 {
    let pts = &w.path.points;
    pts.iter().map(|p| p.x).sum::<f32>() / pts.len() as f32
}

// ---------------------------------------------------------------------------
// Case 1 — a convex polygon painted with 3 tools fragments into 3 independent
// PerimeterRegions (ADR-0013 Model A per-color fragmentation).
// ---------------------------------------------------------------------------

/// Model A: a convex 9 mm × 9 mm square painted with 3 tools reaches the
/// perimeter stage already fragmented into 3 adjacent per-color regions
/// (region_id 1/2/3, whose union is the original convex silhouette). Each
/// fragment must trace its OWN wall set, tagged with its own `(object_id,
/// region_id)` origin — never merged into a single shared trace.
#[test]
fn three_tool_polygon_fragments_into_three_regions() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    // Three adjacent 3 mm-wide vertical strips whose union is a convex 9×9 mm
    // square; region_id encodes the per-color fragment (Model A key).
    let regions = vec![
        SliceRegionViewBuilder::new()
            .object_id("obj-3c")
            .region_id(1)
            .z(0.2)
            .add_polygon(rect_polygon(-3.0, 0.0, 3.0, 9.0))
            .build(),
        SliceRegionViewBuilder::new()
            .object_id("obj-3c")
            .region_id(2)
            .z(0.2)
            .add_polygon(rect_polygon(0.0, 0.0, 3.0, 9.0))
            .build(),
        SliceRegionViewBuilder::new()
            .object_id("obj-3c")
            .region_id(3)
            .z(0.2)
            .add_polygon(rect_polygon(3.0, 0.0, 3.0, 9.0))
            .build(),
    ];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    // Group emitted walls by their per-region origin tag.
    let mut walls_per_region: BTreeMap<u64, usize> = BTreeMap::new();
    for (_wall, origin) in output.wall_loops().iter().zip(output.wall_loop_origins()) {
        let (_obj, region_id) = origin
            .as_ref()
            .expect("every emitted wall must carry a (object_id, region_id) origin");
        *walls_per_region.entry(*region_id).or_default() += 1;
    }

    assert_eq!(
        walls_per_region.len(),
        3,
        "expected 3 independent PerimeterRegions (one per tool/color fragment), got {walls_per_region:?}"
    );
    assert_eq!(
        walls_per_region.keys().copied().collect::<Vec<_>>(),
        vec![1, 2, 3],
        "the 3 regions must carry the distinct per-color region_ids 1/2/3"
    );
    for (region_id, count) in &walls_per_region {
        assert!(
            *count >= 1,
            "region {region_id} must have a non-empty wall set, got {count} walls"
        );
    }
}

// ---------------------------------------------------------------------------
// Case 2 — a material boundary through an island's INTERIOR partitions the
// inner walls: each material's inner-wall vertices stay on its own side.
// ---------------------------------------------------------------------------

/// A single 10 mm × 10 mm island whose interior is split by a material boundary
/// at x = 0: the two left contour corners are tool 0, the two right corners
/// tool 1. `build_wall_flags` reprojects every inner-wall vertex to its nearest
/// original corner, so inner walls must (a) be classified as a
/// `WallBoundaryType::MaterialBoundary`, and (b) carry the tool of the side they
/// physically lie on — no vertex crosses the boundary into the other material.
#[test]
fn inner_wall_respects_material_boundary() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    // `square_polygon` emits corners CCW: BL, BR, TR, TL.
    // Material paint: BL(x<0)=tool0, BR(x>0)=tool1, TR(x>0)=tool1, TL(x<0)=tool0.
    // => a single vertical material transition running through x = 0.
    let mut segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
        HashMap::new();
    segment_annotations.insert(
        PaintSemantic::Material,
        vec![vec![
            Some(PaintValue::ToolIndex(0)), // BL
            Some(PaintValue::ToolIndex(1)), // BR
            Some(PaintValue::ToolIndex(1)), // TR
            Some(PaintValue::ToolIndex(0)), // TL
        ]],
    );

    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj-mat")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .build();
    region.set_segment_annotations(segment_annotations);

    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            0,
            std::slice::from_ref(&region),
            &paint,
            &mut output,
            &config,
        )
        .unwrap();

    let inner_walls: Vec<&WallLoop> = output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .collect();
    assert!(
        !inner_walls.is_empty(),
        "wall_count=3 must produce inner walls to partition"
    );

    // (a) Inner walls straddling the transition are flagged as a MaterialBoundary.
    let material_boundary_inner = inner_walls
        .iter()
        .filter(|w| matches!(w.boundary_type, WallBoundaryType::MaterialBoundary { .. }))
        .count();
    assert!(
        material_boundary_inner >= 1,
        "at least one inner wall must be a MaterialBoundary; got boundary_types {:?}",
        inner_walls
            .iter()
            .map(|w| &w.boundary_type)
            .collect::<Vec<_>>()
    );

    // (b) Each material's inner-wall vertices stay on its own side of x = 0:
    // left-of-boundary vertices carry tool 0, right-of-boundary vertices tool 1.
    // (|x| > 1 mm margin avoids the near-boundary reprojection tie.)
    for w in &inner_walls {
        for (pt, flag) in w.path.points.iter().zip(w.feature_flags.iter()) {
            if pt.x < -1.0 {
                assert_eq!(
                    flag.tool_index,
                    Some(0),
                    "left-of-boundary inner-wall vertex at x={} must belong to tool 0",
                    pt.x
                );
            } else if pt.x > 1.0 {
                assert_eq!(
                    flag.tool_index,
                    Some(1),
                    "right-of-boundary inner-wall vertex at x={} must belong to tool 1",
                    pt.x
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Case 3 — degenerate polygons must be skipped gracefully (no panic, no
// spurious walls).
// ---------------------------------------------------------------------------

/// A region carrying only degenerate contours — a 0-vertex polygon and a
/// 2-vertex (collinear, zero-area) polygon — must flow through the perimeter
/// path without panicking, and must emit no walls for the degenerate input.
#[test]
fn degenerate_polygon_no_panic() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    let empty_poly = ExPolygon {
        contour: Polygon { points: Vec::new() },
        holes: Vec::new(),
    };
    let two_point_poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 {
                    x: mm_to_units(5.0),
                    y: 0,
                },
            ],
        },
        holes: Vec::new(),
    };

    let region = SliceRegionViewBuilder::new()
        .object_id("obj-degen")
        .region_id(1)
        .z(0.2)
        .add_polygon(empty_poly)
        .add_polygon(two_point_poly)
        .build();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    // Must not panic and must not surface as a module error.
    module
        .run_perimeters(
            0,
            std::slice::from_ref(&region),
            &paint,
            &mut output,
            &config,
        )
        .expect("degenerate region must be skipped gracefully, not error out");

    let wall_count = output.wall_loops().len();
    assert_eq!(
        wall_count,
        0,
        "degenerate (0- and 2-vertex) polygons must emit zero walls, got {wall_count}: {:?}",
        output
            .wall_loops()
            .iter()
            .map(|w| (w.loop_type, w.path.role.clone(), w.path.points.len()))
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Case 4 — a thin ring between a hole and the outer boundary is captured as a
// ThinWall, not silently dropped.
// ---------------------------------------------------------------------------

/// A 10 mm square island with an off-center square hole: the hole's right edge
/// sits 0.22 mm from the outer right boundary — narrower than 2 × (nozzle/3) =
/// 0.267 mm (the same erode-to-thin threshold proven by
/// `thin_wall_emission_tdd`) — while the other three margins are a thick 1 mm.
/// The thin right gap between hole and boundary must be captured as a
/// `ThinWall` (an open medial-axis spine near the right edge), not silently
/// dropped.
#[test]
fn hole_with_thin_wall_emits_thin_wall() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .float("nozzle_diameter", 0.4)
        .bool("detect_thin_wall", true)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    // Outer 10 mm square (CCW, corners at ±5 mm). Hole (CW): x ∈ [-4.0, 4.78],
    // y ∈ [-4.0, 4.0] — so the RIGHT gap is 5.0 - 4.78 = 0.22 mm (thin), while
    // the left/top/bottom margins are 1.0 mm (thick).
    let outer = square_polygon(0.0, 0.0, 10.0);
    let hx_l = mm_to_units(-4.0);
    let hx_r = mm_to_units(4.78);
    let hy_b = mm_to_units(-4.0);
    let hy_t = mm_to_units(4.0);
    // CW winding (opposite of the CCW contour) so the ExPolygon is a true frame.
    let hole = Polygon {
        points: vec![
            Point2 { x: hx_l, y: hy_b },
            Point2 { x: hx_l, y: hy_t },
            Point2 { x: hx_r, y: hy_t },
            Point2 { x: hx_r, y: hy_b },
        ],
    };
    let frame = ExPolygon {
        contour: outer.contour,
        holes: vec![hole],
    };

    let region = SliceRegionViewBuilder::new()
        .object_id("obj-frame")
        .region_id(1)
        .z(0.2)
        .add_polygon(frame)
        .build();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            0,
            std::slice::from_ref(&region),
            &paint,
            &mut output,
            &config,
        )
        .unwrap();

    let thin: Vec<&WallLoop> = output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::ThinWall)
        .collect();

    assert!(
        !thin.is_empty(),
        "the 0.22 mm gap between hole and boundary must emit a ThinWall; got loop types {:?}",
        output
            .wall_loops()
            .iter()
            .map(|w| w.loop_type)
            .collect::<Vec<_>>()
    );

    // Every thin-wall loop must carry the ThinWall role + per-vertex flag.
    for tl in &thin {
        assert_eq!(
            tl.path.role,
            ExtrusionRole::ThinWall,
            "thin-wall loop must carry ExtrusionRole::ThinWall"
        );
        assert!(
            tl.feature_flags.iter().all(|f| f.is_thin_wall),
            "every thin-wall vertex must have is_thin_wall = true"
        );
    }
    // At least one thin wall must trace the thin right gap (x ≈ 4.89 mm), i.e.
    // the feature between the hole and the outer boundary — not dropped.
    assert!(
        thin.iter()
            .any(|tl| tl.path.points.iter().any(|p| p.x > 4.0)),
        "a ThinWall must trace the thin right gap near x ≈ 4.89 mm"
    );
}

// ---------------------------------------------------------------------------
// Case 5 — gap-fill inside an overhang region coexists with the overhang
// extra-perimeter classification (both are emitted, neither suppresses the
// other).
// ---------------------------------------------------------------------------

/// A composite region entirely inside an overhang footprint: a wide 10 mm lobe
/// (which gains the `extra_perimeters_on_overhangs` bonus wall) plus a thin
/// 1.9 mm arm (which yields gap-fill). With the overhang bonus enabled, the
/// overhang branch must BOTH add the bonus wall to the lobe AND still emit the
/// arm's gap-fill; with it disabled the lobe stays at the base wall count.
#[test]
fn gap_fill_in_overhang_region() {
    let make_config = |overhang_bonus: bool| {
        ConfigViewBuilder::new()
            .int("wall_count", 1)
            .float("outer_wall_line_width", 0.4)
            .float("inner_wall_line_width", 0.4)
            .float("gap_infill_speed", 30.0)
            .float("filter_out_gap_fill", 0.5)
            .bool("extra_perimeters_on_overhangs", overhang_bonus)
            .build()
    };

    // Wide lobe at x∈[-15,-5]; thin gap-fill arm at x≈[9.05,10.95]; overhang
    // footprint (x∈[-20,20]) covers both.
    let make_region = || {
        SliceRegionViewBuilder::new()
            .object_id("obj-ovh")
            .region_id(1)
            .z(0.2)
            .add_polygon(square_polygon(-10.0, 0.0, 10.0))
            .add_polygon(rect_polygon(10.0, 0.0, 1.9, 8.0))
            .overhang_areas(vec![rect_polygon(0.0, 0.0, 40.0, 20.0)])
            .build()
    };

    let paint = PaintRegionLayerView::new(0);

    // Overhang bonus ENABLED.
    let cfg_on = make_config(true);
    let module_on = ClassicPerimeters::on_print_start(&cfg_on).unwrap();
    let region_on = make_region();
    let mut out_on = PerimeterOutputBuilder::new();
    module_on
        .run_perimeters(
            0,
            std::slice::from_ref(&region_on),
            &paint,
            &mut out_on,
            &cfg_on,
        )
        .unwrap();

    // Overhang bonus DISABLED (contrast).
    let cfg_off = make_config(false);
    let module_off = ClassicPerimeters::on_print_start(&cfg_off).unwrap();
    let region_off = make_region();
    let mut out_off = PerimeterOutputBuilder::new();
    module_off
        .run_perimeters(
            0,
            std::slice::from_ref(&region_off),
            &paint,
            &mut out_off,
            &cfg_off,
        )
        .unwrap();

    // (1) Gap-fill is emitted, on the thin-arm side (x > 0), inside the overhang.
    let gap_loops: Vec<&WallLoop> = out_on
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::GapFill)
        .collect();
    assert!(
        !gap_loops.is_empty(),
        "gap-fill must be emitted inside the overhang region; got loop types {:?}",
        out_on
            .wall_loops()
            .iter()
            .map(|w| w.loop_type)
            .collect::<Vec<_>>()
    );
    assert!(
        gap_loops.iter().any(|w| mean_x_mm(w) > 5.0),
        "gap-fill must lie on the thin-arm side (x>0) within the overhang footprint"
    );

    // (2) Overhang classification is preserved and coexists with gap-fill: the
    // wide lobe (x<0) gains the extra overhang wall (base 1 -> 2) with the bonus
    // enabled, while it stays at the base count (1) with the bonus disabled.
    let lobe_on = outer_inner_walls(&out_on)
        .into_iter()
        .filter(|w| mean_x_mm(w) < 0.0)
        .count();
    let lobe_off = outer_inner_walls(&out_off)
        .into_iter()
        .filter(|w| mean_x_mm(w) < 0.0)
        .count();
    assert_eq!(
        lobe_off, 1,
        "without the overhang bonus the wide lobe keeps base wall_count=1; got {lobe_off}"
    );
    assert_eq!(
        lobe_on, 2,
        "the overhang bonus must add one wall to the lobe (=2) even while gap-fill is emitted; got {lobe_on}"
    );
}

// ---------------------------------------------------------------------------
// Case 6 — a top-surface-flagged region propagates that classification into
// the perimeter output.
// ---------------------------------------------------------------------------

/// A region flagged as an exposed top surface (`top_shell_index == Some(0)`)
/// must have that classification propagate into perimeter generation: under
/// `only_one_wall_top` the top region collapses to a single wall, while an
/// otherwise-identical non-top region keeps the full `wall_count`. (This module
/// carries no per-vertex "top" flag; the observable propagation of the
/// top-surface classification is this wall-count collapse.)
#[test]
fn top_flagged_region_propagates_flag() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .bool("only_one_wall_top", true)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);

    let top_region = SliceRegionViewBuilder::new()
        .object_id("obj-top")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .top_shell_index(Some(0))
        .build();
    let non_top_region = SliceRegionViewBuilder::new()
        .object_id("obj-top")
        .region_id(2)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .top_shell_index(None)
        .build();

    let mut out_top = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            0,
            std::slice::from_ref(&top_region),
            &paint,
            &mut out_top,
            &config,
        )
        .unwrap();
    let mut out_non_top = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            0,
            std::slice::from_ref(&non_top_region),
            &paint,
            &mut out_non_top,
            &config,
        )
        .unwrap();

    let top_walls = outer_inner_walls(&out_top).len();
    let non_top_walls = outer_inner_walls(&out_non_top).len();

    assert_eq!(
        top_walls, 1,
        "top-flagged region (top_shell_index=Some(0)) must collapse to a single wall under only_one_wall_top; got {top_walls}"
    );
    assert_eq!(
        non_top_walls, 3,
        "non-top region must keep the full wall_count=3; got {non_top_walls}"
    );
}

// ---------------------------------------------------------------------------
// Case 7 — a first-layer config override applies to layer 0 only.
// ---------------------------------------------------------------------------

/// `only_one_wall_first_layer` overrides the perimeter count on the first layer
/// (index 0) only: layer 0's output collapses to a single wall while an
/// interior layer keeps the default `wall_count`.
#[test]
fn first_layer_override_applies() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .bool("only_one_wall_first_layer", true)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);

    let make_region = |z: f32| {
        SliceRegionViewBuilder::new()
            .object_id("obj-fl")
            .region_id(1)
            .z(z)
            .add_polygon(square_polygon(0.0, 0.0, 10.0))
            .build()
    };

    // Layer 0 — override active.
    let region_first = make_region(0.2);
    let mut out_first = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            0,
            std::slice::from_ref(&region_first),
            &paint,
            &mut out_first,
            &config,
        )
        .unwrap();

    // Interior layer 5 — override inactive.
    let region_interior = make_region(1.2);
    let mut out_interior = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            5,
            std::slice::from_ref(&region_interior),
            &paint,
            &mut out_interior,
            &config,
        )
        .unwrap();

    let first_layer_walls = outer_inner_walls(&out_first).len();
    let interior_layer_walls = outer_inner_walls(&out_interior).len();

    assert_eq!(
        first_layer_walls, 1,
        "first layer (index 0) must reflect the only_one_wall_first_layer override => 1 wall; got {first_layer_walls}"
    );
    assert_eq!(
        interior_layer_walls, 2,
        "interior layer must use the default wall_count=2; got {interior_layer_walls}"
    );
}
