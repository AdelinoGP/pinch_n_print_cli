//! Packet 247, Step 3 — silhouette composite renderer core
//! (`crates/slicer-runtime/src/visual_debug_render.rs`).
//!
//! Three contracts, asserted on decoded pixels wherever pixels can see them:
//!
//! - `region_slab_bottoms_follow_effective_layer_height` (AC-2): two
//!   `SlicedRegion`s on ONE capture with distinct `effective_layer_height`
//!   keep distinct rectangle bottoms — never merged to one uniform slab.
//! - `interval_union_holes_islands_and_touching_merge` (AC-3): a hole never
//!   splits a projection run; disjoint islands stay separated; touching
//!   islands merge. The touch-merge half asserts on
//!   `union_silhouette_intervals` directly — two abutting rectangles
//!   rasterize identically to one merged rectangle, so pixels cannot see the
//!   difference.
//! - `silhouette_composite_is_deterministic` (AC-6): same inputs twice →
//!   byte-identical PNG and element-for-element equal warnings.

use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, GCodeCommand, GCodeIR, LayerCollectionIR, Point2,
    Point3WithWidth, Polygon, PrintEntity, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SeamPlanEntry, SeamPlanIR, SeamPosition, SliceIR, SlicedRegion, SupportGeometryIR,
    SupportGeometryKey, SupportPlanEntry, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
    SurfaceClassificationIR,
};
use slicer_runtime::{
    build_silhouette_slice_height_index, compute_silhouette_viewport_bounds,
    gcode_emit_silhouette_segments, render_gcode_emit_silhouette,
    render_gcode_emit_silhouette_seamed, render_silhouette_composite,
    render_silhouette_composite_seamed, render_silhouette_composite_styled,
    render_silhouette_overhang_composite, render_silhouette_seam_overlay, silhouette_seam_events,
    union_silhouette_intervals, CapturedIr, ColorBy, DefaultGCodeEmitter, GCodeEmitter,
    OverlayEvent, Projector, RenderError, RenderStyle, SilhouetteScheduleSlab,
    SilhouetteSlabSchedule, SilhouetteView, StageCapture, ToolColors, ViewportBoundsMm,
};
use std::collections::{BTreeSet, HashMap};

const BACKGROUND: [u8; 3] = [255, 255, 255];

fn body_color() -> [u8; 3] {
    slicer_runtime::visual_debug_render::palette::SLICE_REGION
}

/// An axis-aligned rectangle contour in millimeters.
fn rect_polygon(x0: f32, x1: f32, y0: f32, y1: f32) -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(x0, y0),
            Point2::from_mm(x1, y0),
            Point2::from_mm(x1, y1),
            Point2::from_mm(x0, y1),
        ],
    }
}

fn rect_expolygon(x0: f32, x1: f32, y0: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: rect_polygon(x0, x1, y0, y1),
        holes: Vec::new(),
    }
}

fn region(effective_layer_height: f32, polygons: Vec<ExPolygon>) -> SlicedRegion {
    SlicedRegion {
        object_id: "obj-0".to_string(),
        region_id: 0,
        polygons,
        effective_layer_height,
        ..SlicedRegion::default()
    }
}

fn slice_capture(layer_index: u32, layer_z: f32, regions: Vec<SlicedRegion>) -> StageCapture {
    StageCapture {
        stage_id: "Layer::Slice".to_string(),
        layer_index,
        layer_z,
        ir: CapturedIr::Slice(SliceIR {
            global_layer_index: layer_index,
            z: layer_z,
            regions,
            ..SliceIR::default()
        }),
    }
}

fn schedule(slabs: &[(u32, f32, f32)]) -> SilhouetteSlabSchedule {
    SilhouetteSlabSchedule {
        slabs: slabs
            .iter()
            .map(|&(index, z_bottom, z_top)| SilhouetteScheduleSlab {
                index,
                z_bottom,
                z_top,
            })
            .collect(),
    }
}

fn decode_rgb(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder
        .read_info()
        .expect("render_silhouette_composite always encodes a valid PNG header");
    let mut buf = vec![
        0u8;
        reader.output_buffer_size().expect(
            "render_silhouette_composite always encodes a fixed-size RGB8 buffer"
        )
    ];
    let info = reader
        .next_frame(&mut buf)
        .expect("render_silhouette_composite always encodes a valid PNG frame");
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}

/// Sample the pixel a known (horizontal-mm, z-mm) world point lands on, using
/// the renderer's **real** `Projector` rather than a copy of its arithmetic.
fn sample(
    rgb: &[u8],
    bounds: ViewportBoundsMm,
    width: u32,
    height: u32,
    h_mm: f32,
    z_mm: f32,
) -> [u8; 3] {
    let (px, py) = Projector::new(bounds, width, height).project(f64::from(h_mm), f64::from(z_mm));
    let x = px.round().max(0.0) as usize;
    let y = py.round().max(0.0) as usize;
    let idx = (y * width as usize + x) * 3;
    [rgb[idx], rgb[idx + 1], rgb[idx + 2]]
}

/// AC-2. One capture, two regions, distinct `effective_layer_height`: each
/// rectangle's bottom row is its OWN `z − effective_layer_height`. The
/// catch-up-sized region's bottom lands strictly below the other's.
#[test]
fn region_slab_bottoms_follow_effective_layer_height() {
    // Layer top 1.0 mm. Region A: normal 0.2 mm layer -> bottom 0.8.
    // Region B: catch-up-sized 0.6 mm layer -> bottom 0.4, reaching down
    // past where the previous layer's top would be.
    let captures = vec![slice_capture(
        0,
        1.0,
        vec![
            region(0.2, vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)]),
            region(0.6, vec![rect_expolygon(20.0, 30.0, 0.0, 5.0)]),
        ],
    )];
    let sched = schedule(&[(0, 0.4, 1.0)]);
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("a populated slice capture group must render");
    assert!(
        warnings.is_empty(),
        "a single body class can never occlude anything: {warnings:?}"
    );
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    // Both slabs are painted at their shared top.
    assert_eq!(sample(&rgb, bounds, w, h, 5.0, 0.9), body_color());
    assert_eq!(sample(&rgb, bounds, w, h, 25.0, 0.9), body_color());

    // Region A stops at 0.8: below it is background.
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.6),
        BACKGROUND,
        "region A's 0.2 mm slab must not reach down to region B's bottom"
    );
    // Region B reaches to 0.4 at the same Z where A is already gone.
    assert_eq!(
        sample(&rgb, bounds, w, h, 25.0, 0.6),
        body_color(),
        "region B's 0.6 mm slab must still be painted below A's bottom"
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 25.0, 0.5),
        body_color(),
        "region B's slab bottom is 0.4, not 0.8"
    );
    // And B itself stops at 0.4 — the slab is exact, not unbounded.
    assert_eq!(sample(&rgb, bounds, w, h, 25.0, 0.2), BACKGROUND);
}

/// AC-3. (a) a contour with a hole projects to ONE unbroken run;
/// (b) two disjoint islands leave background between them;
/// (c) two touching-interval islands merge into one run.
#[test]
fn interval_union_holes_islands_and_touching_merge() {
    let sched = schedule(&[(0, 0.8, 1.0)]);

    // (a) One contour spanning [0, 10] with a hole over [4, 6]. Holes never
    // split a projection interval — the CONTOUR alone is projected.
    let holed = ExPolygon {
        contour: rect_polygon(0.0, 10.0, 0.0, 5.0),
        holes: vec![rect_polygon(4.0, 6.0, 1.0, 4.0)],
    };
    let captures = vec![slice_capture(0, 1.0, vec![region(0.2, vec![holed])])];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, _) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("a holed contour must render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    for x in [0.5_f32, 3.0, 5.0, 7.0, 9.5] {
        assert_eq!(
            sample(&rgb, bounds, w, h, x, 0.9),
            body_color(),
            "the hole at x={x} must NOT split the projection run"
        );
    }
    assert_eq!(sample(&rgb, bounds, w, h, -1.0, 0.9), BACKGROUND);
    assert_eq!(sample(&rgb, bounds, w, h, 11.0, 0.9), BACKGROUND);

    // (b) Two disjoint islands [0, 4] and [6, 10] -> two runs, background
    // between them.
    let captures = vec![slice_capture(
        0,
        1.0,
        vec![region(
            0.2,
            vec![
                rect_expolygon(0.0, 4.0, 0.0, 5.0),
                rect_expolygon(6.0, 10.0, 0.0, 5.0),
            ],
        )],
    )];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, _) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("two disjoint islands must render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(sample(&rgb, bounds, w, h, 2.0, 0.9), body_color());
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.9),
        BACKGROUND,
        "disjoint islands must stay separated by background"
    );
    assert_eq!(sample(&rgb, bounds, w, h, 8.0, 0.9), body_color());

    // (c) Two touching islands [0, 5] and [5, 10] -> ONE merged run. Pixels
    // cannot distinguish two abutting rectangles from one merged rectangle,
    // so the merge itself is asserted on the union helper directly; the
    // pixel assertions cover the resulting unbroken run.
    assert_eq!(
        union_silhouette_intervals(&[(0.0, 5.0), (5.0, 10.0)]),
        vec![(0.0, 10.0)],
        "touching intervals must merge (next.start <= current.end)"
    );
    assert_eq!(
        union_silhouette_intervals(&[(6.0, 10.0), (0.0, 4.0)]),
        vec![(0.0, 4.0), (6.0, 10.0)],
        "disjoint intervals must NOT merge, and the sweep must sort first"
    );
    let captures = vec![slice_capture(
        0,
        1.0,
        vec![region(
            0.2,
            vec![
                rect_expolygon(5.0, 10.0, 0.0, 5.0),
                rect_expolygon(0.0, 5.0, 0.0, 5.0),
            ],
        )],
    )];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, _) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("two touching islands must render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    for x in [0.5_f32, 2.0, 5.0, 8.0, 9.5] {
        assert_eq!(
            sample(&rgb, bounds, w, h, x, 0.9),
            body_color(),
            "touching islands must render as one unbroken run at x={x}"
        );
    }
}

/// AC-6. Same capture group, view, scale, schedule, and viewport rendered
/// twice → byte-identical PNG bytes and equal warning lists.
#[test]
fn silhouette_composite_is_deterministic() {
    let captures = vec![
        slice_capture(
            0,
            0.2,
            vec![region(
                0.2,
                vec![
                    rect_expolygon(0.0, 4.0, 0.0, 5.0),
                    rect_expolygon(6.0, 10.0, 0.0, 5.0),
                ],
            )],
        ),
        slice_capture(
            1,
            0.6,
            vec![
                region(0.4, vec![rect_expolygon(1.0, 9.0, 0.0, 5.0)]),
                region(0.2, vec![rect_expolygon(2.0, 3.0, 0.0, 5.0)]),
            ],
        ),
    ];
    let sched = schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.6)]);
    let bounds = compute_silhouette_viewport_bounds(
        &captures,
        SilhouetteView::Side,
        &sched,
        Some(ViewportBoundsMm {
            min_x: -1.0,
            min_y: 0.0,
            max_x: 12.0,
            max_y: 1.0,
        }),
    );
    let first = render_silhouette_composite(&captures, SilhouetteView::Side, 2, bounds, &sched)
        .expect("first render must succeed");
    let second = render_silhouette_composite(&captures, SilhouetteView::Side, 2, bounds, &sched)
        .expect("second render must succeed");
    assert_eq!(
        first.0.png_bytes, second.0.png_bytes,
        "the composite render must be byte-identical across runs"
    );
    assert_eq!(first.0.width, second.0.width);
    assert_eq!(first.0.height, second.0.height);
    assert_eq!(
        first.1, second.1,
        "warning lists must be equal element-for-element"
    );
}

/// Fail closed: a group that yields no rectangle at all is an error, never a
/// blank PNG.
#[test]
fn empty_capture_group_fails_closed() {
    let sched = schedule(&[(0, 0.0, 0.2)]);
    let err = render_silhouette_composite(
        &[],
        SilhouetteView::Front,
        1,
        ViewportBoundsMm {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        },
        &sched,
    )
    .expect_err("an empty capture group must fail closed");
    assert!(
        matches!(err, RenderError::MissingGeometryField { .. }),
        "expected MissingGeometryField, got {err:?}"
    );

    // A present-but-empty region set is the same failure, not a blank image.
    let captures = vec![slice_capture(0, 0.2, vec![region(0.2, Vec::new())])];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let err = render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
        .expect_err("a geometry-free capture group must fail closed");
    assert!(matches!(err, RenderError::MissingGeometryField { .. }));

    // Regression guard (review finding): a support-only group must name the
    // support field, not the `Slice` arm's `regions[].polygons`.
    let plan = SupportPlanIR {
        entries: vec![plan_entry(
            0,
            0,
            vec![plan_role(SupportPlanRole::SupportBody, Vec::new())],
        )],
        ..SupportPlanIR::default()
    };
    let captures = vec![support_capture(0, 0.2, SupportGeometryIR::default(), plan)];
    let err = render_silhouette_composite(
        &captures,
        SilhouetteView::Front,
        1,
        ViewportBoundsMm {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        },
        &sched,
    )
    .expect_err("a support-only group with no drawable region must fail closed");
    match err {
        RenderError::MissingGeometryField { field, .. } => assert_eq!(
            field, "plan.entries[].roles[].regions",
            "a support-only group must name the support field"
        ),
        other => panic!("expected MissingGeometryField, got {other:?}"),
    }
}

/// The request-facing spelling round-trips and unknown views are rejected.
#[test]
fn silhouette_view_names_round_trip() {
    assert_eq!(SilhouetteView::Front.name(), "front");
    assert_eq!(SilhouetteView::Side.name(), "side");
    assert_eq!(SilhouetteView::parse("front"), Some(SilhouetteView::Front));
    assert_eq!(SilhouetteView::parse("side"), Some(SilhouetteView::Side));
    assert_eq!(SilhouetteView::parse("top"), None);
    assert_eq!(SilhouetteView::parse("Front"), None);
}

// ============================================================================
// Step 4 - `CapturedIr::SupportGeometry` arm: per-role classes, paint order,
// W1 (raft) / W2 (coarse entries) / occlusion warnings.
// ============================================================================

fn support_color() -> [u8; 3] {
    slicer_runtime::visual_debug_render::palette::SUPPORT
}

fn support_interface_color() -> [u8; 3] {
    slicer_runtime::visual_debug_render::palette::SUPPORT_INTERFACE
}

/// One `SupportPlanEntry`. `SupportPlanEntry` has no `Default` impl, so the
/// literal is written out in full under the standard waiver.
fn plan_entry(
    global_layer_index: i32,
    region_id: u64,
    roles: Vec<SupportPlanRoleRegion>,
) -> SupportPlanEntry {
    // exhaustive: SupportPlanEntry has no Default impl; FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index,
        object_id: "obj-0".to_string(),
        region_id,
        family_id: "tree-support".to_string(),
        demand_ids: Vec::new(),
        body_ids: Vec::new(),
        anchor_layer_index: 0,
        anchor_z: 0,
        roles,
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec!["test".to_string()],
        decline_reason: None,
    }
}

fn plan_role(role: SupportPlanRole, regions: Vec<ExPolygon>) -> SupportPlanRoleRegion {
    SupportPlanRoleRegion { role, regions }
}

fn support_capture(
    layer_index: u32,
    layer_z: f32,
    geometry: SupportGeometryIR,
    plan: SupportPlanIR,
) -> StageCapture {
    StageCapture {
        stage_id: "PrePass::SupportGeometry".to_string(),
        layer_index,
        layer_z,
        ir: CapturedIr::SupportGeometry { geometry, plan },
    }
}

/// AC-4. `SupportBody` and `TopInterface` role regions overlapping in X on one
/// layer: the overlap paints `SUPPORT_INTERFACE` (interface classes paint
/// AFTER body in the pinned order), the body-only run paints `SUPPORT`, and
/// exactly ONE occlusion warning names the affected layer count.
#[test]
fn support_role_paint_order_and_occlusion_warning() {
    let sched = schedule(&[(0, 0.8, 1.0)]);
    let plan = SupportPlanIR {
        entries: vec![plan_entry(
            0,
            0,
            vec![
                plan_role(
                    SupportPlanRole::SupportBody,
                    vec![rect_expolygon(0.0, 20.0, 0.0, 5.0)],
                ),
                plan_role(
                    SupportPlanRole::TopInterface,
                    vec![rect_expolygon(10.0, 30.0, 0.0, 5.0)],
                ),
            ],
        )],
        ..SupportPlanIR::default()
    };
    let captures = vec![support_capture(0, 1.0, SupportGeometryIR::default(), plan)];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("a populated support-plan capture group must render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    // Body-only run [0, 10).
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.9),
        support_color(),
        "the non-overlapping body run must paint SUPPORT"
    );
    // Overlap [10, 20] - interface paints last and wins.
    assert_eq!(
        sample(&rgb, bounds, w, h, 15.0, 0.9),
        support_interface_color(),
        "the overlap must paint SUPPORT_INTERFACE: interface classes paint AFTER body"
    );
    // Interface-only run (20, 30].
    assert_eq!(
        sample(&rgb, bounds, w, h, 25.0, 0.9),
        support_interface_color()
    );
    // The slab is the caller's schedule slab, exactly.
    assert_eq!(sample(&rgb, bounds, w, h, 15.0, 0.5), BACKGROUND);

    let occlusion: Vec<&String> = warnings
        .iter()
        .filter(|w| w.contains("silhouette occlusion"))
        .collect();
    assert_eq!(
        occlusion.len(),
        1,
        "exactly one deduped occlusion warning expected, got {warnings:?}"
    );
    assert!(
        occlusion[0].contains("1 layer(s)"),
        "the occlusion warning must name the affected layer count: {}",
        occlusion[0]
    );
    assert_eq!(
        warnings.len(),
        1,
        "no raft or coarse-entry warning is due here: {warnings:?}"
    );
}

/// AC-5. Negative-index plan entries (raft) and coarse `SupportGeometryIR`
/// entries are skipped, each with a named warning, and contribute no pixels.
#[test]
fn raft_and_coarse_entries_skip_with_named_warnings() {
    let sched = schedule(&[(0, 0.8, 1.0)]);
    let mut entries = HashMap::new();
    for (i, region_id) in [1_u64, 2, 3].into_iter().enumerate() {
        let base = 60.0 + i as f32 * 2.0;
        entries.insert(
            SupportGeometryKey {
                global_support_layer_index: 0,
                object_id: "obj-0".to_string(),
                region_id,
            },
            vec![rect_expolygon(base, base + 1.0, 0.0, 5.0)],
        );
    }
    let geometry = SupportGeometryIR {
        support_layer_height_mm: 0.2,
        entries,
        ..SupportGeometryIR::default()
    };
    let plan = SupportPlanIR {
        entries: vec![
            plan_entry(
                -2,
                0,
                vec![plan_role(
                    SupportPlanRole::SupportBody,
                    vec![rect_expolygon(40.0, 50.0, 0.0, 5.0)],
                )],
            ),
            plan_entry(
                -1,
                0,
                vec![plan_role(
                    SupportPlanRole::SupportBody,
                    vec![rect_expolygon(40.0, 50.0, 0.0, 5.0)],
                )],
            ),
            // One drawable entry so the group does not fail closed; it is the
            // only thing that may contribute pixels.
            plan_entry(
                0,
                0,
                vec![plan_role(
                    SupportPlanRole::SupportBody,
                    vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)],
                )],
            ),
        ],
        ..SupportPlanIR::default()
    };
    let captures = vec![support_capture(0, 1.0, geometry, plan)];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("the layer-0 body entry keeps the group renderable");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.9),
        support_color(),
        "the non-negative-index body entry must still be drawn"
    );
    for x in [41.0_f32, 45.0, 49.0] {
        assert_eq!(
            sample(&rgb, bounds, w, h, x, 0.9),
            BACKGROUND,
            "raft (negative-index) geometry at x={x} must contribute no pixels"
        );
    }
    for x in [60.5_f32, 62.5, 64.5] {
        assert_eq!(
            sample(&rgb, bounds, w, h, x, 0.9),
            BACKGROUND,
            "coarse SupportGeometryIR geometry at x={x} must contribute no pixels"
        );
    }

    assert_eq!(
        warnings.len(),
        2,
        "exactly W1 then W2, deduped, no occlusion: {warnings:?}"
    );
    assert!(
        warnings[0].contains('2') && warnings[0].contains("-2..-1"),
        "W1 must name the count 2 and the dropped index range -2..-1: {}",
        warnings[0]
    );
    assert!(
        warnings[1].contains('3')
            && warnings[1].contains("coarse SupportGeometryIR entries skipped"),
        "W2 must name the count 3 and state the entries are skipped: {}",
        warnings[1]
    );
    // Regression guard (review finding): a dropped `\` line-continuation in
    // the source literal leaks runs of source indentation into the shipped
    // manifest.json string. `contains`-fragment assertions cannot see it.
    for w in &warnings {
        assert!(
            !w.contains("  "),
            "warning must render as a single, single-spaced sentence: {w:?}"
        );
    }
}

/// Binding check, asserted rather than performed by hand: negative-index
/// entries with an EMPTY coarse `SupportGeometryIR` yield W1 and only W1.
#[test]
fn raft_entries_without_coarse_geometry_emit_w1_but_not_w2() {
    let sched = schedule(&[(0, 0.8, 1.0)]);
    let plan = SupportPlanIR {
        entries: vec![
            plan_entry(
                -1,
                0,
                vec![plan_role(
                    SupportPlanRole::SupportBody,
                    vec![rect_expolygon(40.0, 50.0, 0.0, 5.0)],
                )],
            ),
            plan_entry(
                0,
                0,
                vec![plan_role(
                    SupportPlanRole::SupportBody,
                    vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)],
                )],
            ),
        ],
        ..SupportPlanIR::default()
    };
    let captures = vec![support_capture(0, 1.0, SupportGeometryIR::default(), plan)];
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (_, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("the layer-0 body entry keeps the group renderable");
    assert_eq!(warnings.len(), 1, "W1 only, no W2: {warnings:?}");
    assert!(warnings[0].contains("-1..-1"), "{}", warnings[0]);
    assert!(
        !warnings[0].contains("coarse SupportGeometryIR"),
        "an empty coarse entry map must not produce W2: {}",
        warnings[0]
    );
}

/// Every silhouette class color, plus the canvas background, must be pairwise
/// distinct - otherwise a paint-order assertion cannot see a class at all.
#[test]
fn silhouette_class_colors_are_pairwise_distinct() {
    use slicer_runtime::visual_debug_render::palette;
    let named: [(&str, [u8; 3]); 7] = [
        ("BACKGROUND", palette::BACKGROUND),
        ("SLICE_REGION", palette::SLICE_REGION),
        ("SUPPORT", palette::SUPPORT),
        ("SUPPORT_RAFT", palette::SUPPORT_RAFT),
        ("SUPPORT_BASE_INTERFACE", palette::SUPPORT_BASE_INTERFACE),
        (
            "SUPPORT_BOTTOM_INTERFACE",
            palette::SUPPORT_BOTTOM_INTERFACE,
        ),
        ("SUPPORT_INTERFACE", palette::SUPPORT_INTERFACE),
    ];
    for (i, (na, a)) in named.iter().enumerate() {
        for (nb, b) in named.iter().skip(i + 1) {
            assert_ne!(a, b, "{na} and {nb} must be distinct silhouette colors");
        }
    }
}

fn final_entity(
    id: u64,
    role: ExtrusionRole,
    tool_index: u32,
    x0: f32,
    x1: f32,
    width: f32,
) -> PrintEntity {
    // exhaustive: PrintEntity has no Default impl; FRU would let a new plan field default silently
    PrintEntity {
        entity_id: id,
        // exhaustive: ExtrusionPath3D has no Default impl; every path field is intentional in this fixture
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: x0,
                    y: 0.0,
                    z: 0.0,
                    width,
                    ..Point3WithWidth::default()
                },
                Point3WithWidth {
                    x: x1,
                    y: 0.0,
                    z: 0.0,
                    width,
                    ..Point3WithWidth::default()
                },
            ],
            role: role.clone(),
            speed_factor: 1.0,
            tool_index: Some(tool_index),
            order_lock: None,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 0,
            ..RegionKey::default()
        },
        topo_order: id as u32,
        tool_index,
    }
}

fn final_capture(layers: Vec<LayerCollectionIR>) -> StageCapture {
    StageCapture {
        stage_id: "Layer::Finalization".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::LayerFinalization(layers),
    }
}

fn final_layer(index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index: index,
        z,
        ordered_entities: entities,
        ..LayerCollectionIR::default()
    }
}

fn final_bounds() -> ViewportBoundsMm {
    ViewportBoundsMm {
        min_x: -1.0,
        min_y: -0.1,
        max_x: 11.0,
        max_y: 0.5,
    }
}

#[test]
fn gcode_emit_e_inversion_roundtrips_emitter_width() {
    let mut entity = final_entity(1, ExtrusionRole::SparseInfill, 0, 0.0, 10.0, 0.5);
    for point in &mut entity.path.points {
        point.flow_factor = 1.0;
        point.z = 0.2;
    }
    let layer = final_layer(0, 0.2, vec![entity]);
    let config = ResolvedConfig {
        filament_diameter: 2.85,
        ..Default::default()
    };
    let gcode = DefaultGCodeEmitter::new("test".into())
        .with_resolved_config(config)
        .emit_gcode(&[layer])
        .expect("emitter output");
    let (segments, warnings) = gcode_emit_silhouette_segments(
        &gcode,
        SilhouetteView::Front,
        &schedule(&[(0, 0.0, 0.2)]),
        2.85,
    );
    assert!(warnings.is_empty());
    assert!(!segments.is_empty());
    assert!(segments.iter().all(|s| (s.width_mm - 0.5).abs() <= 1e-3));
}

fn move_command(x: Option<f32>, z: Option<f32>, e: Option<f32>) -> GCodeCommand {
    GCodeCommand::Move {
        x,
        y: None,
        z,
        e,
        f: None,
        role: ExtrusionRole::SparseInfill,
    }
}

#[test]
fn gcode_emit_travel_carries_position_and_negative_delta_skipped() {
    let mut gcode = GCodeIR::default();
    gcode.commands.extend([
        move_command(Some(1.0), Some(0.1), Some(1.0)),
        move_command(Some(2.0), None, None),
        move_command(Some(3.0), None, Some(2.0)),
        move_command(Some(4.0), None, Some(1.2)),
    ]);
    let (segments, warnings) = gcode_emit_silhouette_segments(
        &gcode,
        SilhouetteView::Front,
        &schedule(&[(0, 0.0, 0.2)]),
        1.75,
    );
    assert!(warnings.is_empty());
    assert_eq!(segments.len(), 2);
    assert!((segments[1].h1_mm - segments[1].h0_mm - 1.0).abs() < 1e-6);
    gcode
        .commands
        .push(move_command(Some(5.0), None, Some(3.0)));
    let (segments, _) = gcode_emit_silhouette_segments(
        &gcode,
        SilhouetteView::Front,
        &schedule(&[(0, 0.0, 0.2)]),
        1.75,
    );
    assert_eq!(segments.len(), 3);
    assert!((segments[2].h1_mm - segments[2].h0_mm - 1.0).abs() < 1e-6);
}

#[test]
fn gcode_emit_top_edge_containment_has_no_warning() {
    let mut gcode = GCodeIR::default();
    gcode
        .commands
        .push(move_command(Some(1.0), Some(0.2), Some(1.0)));
    let (_, warnings) = gcode_emit_silhouette_segments(
        &gcode,
        SilhouetteView::Front,
        &schedule(&[(0, 0.0, 0.2)]),
        1.75,
    );
    assert!(warnings.is_empty());
}

#[test]
fn gcode_emit_nearest_slab_warns_in_ascending_order_and_caps() {
    let mut gcode = GCodeIR::default();
    for i in 0..10 {
        gcode.commands.push(move_command(
            Some(i as f32 + 1.0),
            Some(1.0 + i as f32 * 0.1),
            Some(i as f32 + 1.0),
        ));
    }
    let (segments, warnings) = gcode_emit_silhouette_segments(
        &gcode,
        SilhouetteView::Front,
        &schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.4)]),
        1.75,
    );
    assert_eq!(segments.len(), 10);
    assert_eq!(warnings.len(), 9);
    assert!(warnings[0].contains("z=1.000"));
    assert!(warnings.last().is_some_and(|w| w.contains("+2 more")));
}

#[test]
fn gcode_emit_z_containment_buckets_without_w4() {
    let mut gcode = GCodeIR::default();
    gcode.commands.extend([
        move_command(Some(2.0), Some(0.2), Some(0.1)),
        move_command(Some(4.0), Some(0.4), Some(0.2)),
    ]);
    let sched = schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.4)]);
    let bounds = final_bounds();
    let (image, warnings) = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0, 1],
    )
    .unwrap();
    assert!(warnings.is_empty());
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_ne!(sample(&rgb, bounds, w, h, 1.0, 0.1), BACKGROUND);
    assert_eq!(sample(&rgb, bounds, w, h, 1.0, 0.3), BACKGROUND);
    assert_ne!(sample(&rgb, bounds, w, h, 4.0, 0.3), BACKGROUND);
    assert_eq!(sample(&rgb, bounds, w, h, 4.0, 0.1), BACKGROUND);
}

#[test]
fn gcode_emit_seamed_draws_filtered_seam_glyphs() {
    let mut gcode = GCodeIR::default();
    gcode
        .commands
        .push(move_command(Some(2.0), Some(0.1), Some(0.1)));
    let sched = schedule(&[(0, 0.0, 0.2)]);
    let bounds = final_bounds();
    let seam_plan = SeamPlanIR {
        entries: vec![seam_entry(0, 2.0, 4.0, 0.1), seam_entry(1, 8.0, 4.0, 0.1)],
        ..SeamPlanIR::default()
    };
    let rendered_layers: BTreeSet<u32> = [0].into_iter().collect();
    let (image, events, warnings) = render_gcode_emit_silhouette_seamed(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0],
        Some((&seam_plan, &rendered_layers)),
    )
    .expect("a populated GCodeEmit silhouette with seams must render");
    assert!(warnings.is_empty());
    assert_eq!(
        events,
        vec![OverlayEvent::Seam {
            x: 2.0,
            y: 4.0,
            z: Some(0.1)
        }]
    );
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(sample(&rgb, bounds, w, h, 2.0, 0.1), seam_color());
    assert_ne!(sample(&rgb, bounds, w, h, 8.0, 0.1), seam_color());
}

#[test]
fn gcode_emit_unselected_slab_draws_nothing_without_warning() {
    let mut gcode = GCodeIR::default();
    gcode.commands.extend([
        move_command(Some(2.0), Some(0.1), Some(0.1)),
        move_command(Some(4.0), Some(0.3), Some(0.2)),
    ]);
    let sched = schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.4)]);
    let bounds = final_bounds();
    let (image, warnings) = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0],
    )
    .unwrap();
    assert!(warnings.is_empty());
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_ne!(sample(&rgb, bounds, w, h, 1.0, 0.1), BACKGROUND);
    assert_eq!(sample(&rgb, bounds, w, h, 3.0, 0.3), BACKGROUND);

    gcode
        .commands
        .push(move_command(Some(6.0), Some(0.7), Some(0.3)));
    let (_, warnings) = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0, 1],
    )
    .unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("z=0.700"));
}

#[test]
fn gcode_emit_out_of_slab_draws_nearest_with_w4() {
    let mut gcode = GCodeIR::default();
    gcode
        .commands
        .push(move_command(Some(4.0), Some(0.7), Some(0.1)));
    let sched = schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.4)]);
    let bounds = final_bounds();
    let (image, warnings) = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0, 1],
    )
    .unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("z=0.700") && warnings[0].contains("nearest slab"));
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_ne!(sample(&rgb, bounds, w, h, 4.0, 0.3), BACKGROUND);
}

#[test]
fn gcode_emit_tool_classes_track_toolchange() {
    let mut gcode = GCodeIR::default();
    gcode.commands.extend([
        move_command(Some(6.0), Some(0.2), Some(0.1)),
        GCodeCommand::ToolChange {
            after_entity_index: u32::MAX,
            from: 0,
            to: 1,
        },
        move_command(Some(2.0), None, None),
        move_command(Some(8.0), None, Some(0.2)),
    ]);
    let sched = schedule(&[(0, 0.0, 0.2)]);
    let style = RenderStyle {
        color_by: ColorBy::Tool,
        ..RenderStyle::default()
    };
    let bounds = final_bounds();
    let (image, _) = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
        1.75,
        &[0],
    )
    .unwrap();
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 4.0, 0.1),
        style.tool_colors.color(1)
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 1.0, 0.1),
        style.tool_colors.color(0)
    );
}

#[test]
fn gcode_emit_silhouette_is_deterministic() {
    let mut gcode = GCodeIR::default();
    gcode
        .commands
        .push(move_command(Some(4.0), Some(0.7), Some(0.1)));
    let sched = schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.4)]);
    let bounds = final_bounds();
    let first = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0, 1],
    )
    .unwrap();
    let second = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &RenderStyle::default(),
        1.75,
        &[0, 1],
    )
    .unwrap();
    assert_eq!(first.0.png_bytes, second.0.png_bytes);
    assert_eq!(first.1, second.1);
}

#[test]
fn gcode_emit_all_negative_deltas_fail_closed() {
    let mut gcode = GCodeIR::default();
    gcode
        .commands
        .push(move_command(Some(2.0), Some(0.2), Some(-1.0)));
    let err = render_gcode_emit_silhouette(
        &gcode,
        SilhouetteView::Front,
        1,
        final_bounds(),
        &schedule(&[(0, 0.0, 0.2)]),
        &RenderStyle::default(),
        1.75,
        &[0],
    )
    .unwrap_err();
    assert!(matches!(err, RenderError::MissingGeometryField { .. }));
}

#[test]
fn finalized_layer_slabs_and_half_width_inflation() {
    let captures = vec![final_capture(vec![
        final_layer(
            0,
            0.2,
            vec![final_entity(
                1,
                ExtrusionRole::SparseInfill,
                0,
                2.0,
                8.0,
                0.4,
            )],
        ),
        final_layer(
            1,
            0.4,
            vec![final_entity(
                2,
                ExtrusionRole::SparseInfill,
                0,
                2.0,
                8.0,
                0.4,
            )],
        ),
    ])];
    let sched = schedule(&[(0, 0.0, 0.2), (1, 0.2, 0.4)]);
    let bounds = final_bounds();
    let (image, _) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched).unwrap();
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.1),
        slicer_runtime::visual_debug_render::palette::SPARSE_INFILL
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.3),
        slicer_runtime::visual_debug_render::palette::SPARSE_INFILL
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.21),
        slicer_runtime::visual_debug_render::palette::SPARSE_INFILL
    );
    assert_eq!(sample(&rgb, bounds, w, h, 5.0, 0.41), BACKGROUND);
    let narrow = vec![final_capture(vec![final_layer(
        0,
        0.2,
        vec![final_entity(
            1,
            ExtrusionRole::SparseInfill,
            0,
            2.0,
            8.0,
            0.2,
        )],
    )])];
    let narrow_sched = schedule(&[(0, 0.0, 0.2)]);
    let (narrow_image, _) =
        render_silhouette_composite(&narrow, SilhouetteView::Front, 1, bounds, &narrow_sched)
            .unwrap();
    let (nw, nh, nrgb) = decode_rgb(&narrow_image.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 1.85, 0.1),
        slicer_runtime::visual_debug_render::palette::SPARSE_INFILL
    );
    assert_eq!(sample(&nrgb, bounds, nw, nh, 1.85, 0.1), BACKGROUND);
}

#[test]
fn schedule_filter_gates_whole_print_layers() {
    let capture = final_capture(vec![
        final_layer(
            0,
            0.2,
            vec![final_entity(
                1,
                ExtrusionRole::SparseInfill,
                0,
                0.0,
                2.0,
                0.2,
            )],
        ),
        final_layer(
            1,
            0.4,
            vec![final_entity(
                2,
                ExtrusionRole::SparseInfill,
                0,
                3.0,
                5.0,
                0.2,
            )],
        ),
        final_layer(
            2,
            0.6,
            vec![final_entity(
                3,
                ExtrusionRole::SparseInfill,
                0,
                6.0,
                8.0,
                0.2,
            )],
        ),
    ]);
    let sched = schedule(&[(1, 0.2, 0.4)]);
    let bounds = final_bounds();
    let (image, _) =
        render_silhouette_composite(&[capture], SilhouetteView::Front, 1, bounds, &sched).unwrap();
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 4.0, 0.3),
        slicer_runtime::visual_debug_render::palette::SPARSE_INFILL
    );
    assert_eq!(sample(&rgb, bounds, w, h, 1.0, 0.1), BACKGROUND);
    assert_eq!(sample(&rgb, bounds, w, h, 7.0, 0.5), BACKGROUND);
}

#[test]
fn finalization_role_paint_order_deterministic() {
    let entities = vec![
        final_entity(1, ExtrusionRole::SparseInfill, 0, 0.0, 8.0, 0.2),
        final_entity(2, ExtrusionRole::SupportMaterial, 0, 2.0, 10.0, 0.2),
        final_entity(3, ExtrusionRole::SupportInterface, 0, 4.0, 6.0, 0.2),
    ];
    let sched = schedule(&[(0, 0.0, 0.2)]);
    let bounds = final_bounds();
    let (image, _) = render_silhouette_composite(
        &[final_capture(vec![final_layer(0, 0.2, entities)])],
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
    )
    .unwrap();
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 1.0, 0.1),
        slicer_runtime::visual_debug_render::palette::SPARSE_INFILL
    );
    assert_eq!(sample(&rgb, bounds, w, h, 3.0, 0.1), support_color());
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.1),
        support_interface_color()
    );
}

#[test]
fn tool_classes_paint_ascending_tool_index() {
    let entities = vec![
        final_entity(1, ExtrusionRole::SparseInfill, 0, 0.0, 8.0, 0.2),
        final_entity(2, ExtrusionRole::SparseInfill, 1, 4.0, 10.0, 0.2),
    ];
    let sched = schedule(&[(0, 0.0, 0.2)]);
    let style = RenderStyle {
        color_by: ColorBy::Tool,
        tool_colors: ToolColors::default(),
    };
    let bounds = final_bounds();
    let (image, _) = render_silhouette_composite_styled(
        &[final_capture(vec![final_layer(0, 0.2, entities)])],
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
    )
    .unwrap();
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 2.0, 0.1),
        style.tool_colors.color(0)
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.1),
        style.tool_colors.color(1)
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 9.0, 0.1),
        style.tool_colors.color(1)
    );
    let err = render_silhouette_composite_styled(
        &[slice_capture(
            0,
            0.2,
            vec![region(0.2, vec![rect_expolygon(0.0, 1.0, 0.0, 1.0)])],
        )],
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
    )
    .unwrap_err();
    assert!(
        matches!(err, RenderError::ToolColorUnavailable { ref tap, layer_index: 0 } if tap == "Layer::Slice")
    );
}

#[test]
fn styled_composite_is_deterministic_and_default_equivalent() {
    let sched = schedule(&[(0, 0.0, 0.2)]);
    let captures = vec![
        final_capture(vec![final_layer(
            0,
            0.2,
            vec![final_entity(
                1,
                ExtrusionRole::SparseInfill,
                0,
                1.0,
                4.0,
                0.2,
            )],
        )]),
        slice_capture(
            0,
            0.2,
            vec![region(0.2, vec![rect_expolygon(5.0, 8.0, 0.0, 1.0)])],
        ),
    ];
    let bounds = final_bounds();
    let style = RenderStyle::default();
    let first = render_silhouette_composite_styled(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
    )
    .unwrap();
    let second = render_silhouette_composite_styled(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
    )
    .unwrap();
    let plain =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched).unwrap();
    assert_eq!(first.0.png_bytes, second.0.png_bytes);
    assert_eq!(first.1, second.1);
    assert_eq!(first.0.png_bytes, plain.0.png_bytes);
    assert_eq!(first.1, plain.1);
}

// ============================================================================
// Packet 251, Step 3 - seam glyphs on silhouettes.
//
// - `seam_glyphs_filter_by_rendered_layers_and_carry_source_coords` (AC-2):
//   only SeamPlanIR entries whose `region_key.global_layer_index` is in the
//   rendered-layer set produce an event/glyph, in entries source order, each
//   event carrying the source point's world coordinates verbatim.
// - `seam_overlay_render_is_deterministic` (AC-6): two isolated seam-overlay
//   renders are byte-identical with equal event lists, and the composited
//   form with `seams: None` is byte-equivalent to the frozen styled entry
//   point (delegation equivalence).
// ============================================================================

fn seam_color() -> [u8; 3] {
    slicer_runtime::visual_debug_style::overlay_palette::SEAM
}

fn faint_base_color() -> [u8; 3] {
    slicer_runtime::visual_debug_style::overlay_palette::FAINT_BASE
}

fn seam_entry(global_layer_index: u32, x: f32, y: f32, z: f32) -> SeamPlanEntry {
    SeamPlanEntry {
        region_key: RegionKey {
            global_layer_index,
            object_id: "obj-0".to_string(),
            ..RegionKey::default()
        },
        chosen_candidate: SeamPosition {
            point: Point3WithWidth {
                x,
                y,
                z,
                ..Point3WithWidth::default()
            },
            ..SeamPosition::default()
        },
        ..SeamPlanEntry::default()
    }
}

/// The AC-2 fixture: seams on layers 0, 1, 1, 2 in that source order; the
/// capture group and schedule cover only layer 1.
fn seam_fixture() -> (Vec<StageCapture>, SilhouetteSlabSchedule, SeamPlanIR) {
    let captures = vec![slice_capture(
        1,
        0.4,
        vec![region(0.2, vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)])],
    )];
    let sched = schedule(&[(1, 0.2, 0.4)]);
    let seam_plan = SeamPlanIR {
        entries: vec![
            seam_entry(0, 2.0, 1.0, 0.1),
            seam_entry(1, 5.0, 2.0, 0.3),
            seam_entry(1, 7.0, 1.5, 0.35),
            seam_entry(2, 8.0, 3.0, 0.5),
        ],
        ..SeamPlanIR::default()
    };
    (captures, sched, seam_plan)
}

/// AC-2. With `rendered_layers = {1}` only the two layer-1 entries survive,
/// in `SeamPlanIR.entries` source order, each event carrying the source
/// `chosen_candidate.point`'s world coordinates. The isolated render draws
/// exactly those two glyphs (per-view horizontal against z) over a faint
/// base; the filtered-out layers' seam positions stay glyph-free.
#[test]
fn seam_glyphs_filter_by_rendered_layers_and_carry_source_coords() {
    let (captures, sched, seam_plan) = seam_fixture();
    let rendered_layers: BTreeSet<u32> = [1].into_iter().collect();
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);

    let events = silhouette_seam_events(&seam_plan, SilhouetteView::Front, &rendered_layers);
    assert_eq!(
        events,
        vec![
            OverlayEvent::Seam {
                x: 5.0,
                y: 2.0,
                z: Some(0.3),
            },
            OverlayEvent::Seam {
                x: 7.0,
                y: 1.5,
                z: Some(0.35),
            },
        ],
        "only layer-1 entries, in entries source order, verbatim source coords"
    );

    let (image, events, warnings) = render_silhouette_seam_overlay(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &seam_plan,
        &rendered_layers,
    )
    .expect("a populated capture group with seams must render");
    assert!(warnings.is_empty(), "single body class: {warnings:?}");
    assert_eq!(
        events,
        vec![
            OverlayEvent::Seam {
                x: 5.0,
                y: 2.0,
                z: Some(0.3),
            },
            OverlayEvent::Seam {
                x: 7.0,
                y: 1.5,
                z: Some(0.35),
            },
        ],
        "the returned events are exactly the drawn events, in source order"
    );
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    // Front view: the glyph's in-image horizontal is the point's x.
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.3),
        seam_color(),
        "the first layer-1 seam must draw a seam glyph at its source (x, z)"
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 7.0, 0.35),
        seam_color(),
        "the second layer-1 seam must draw a seam glyph at its source (x, z)"
    );
    // Filtered-out layers' seam positions: no glyph. Both land outside the
    // layer-1 slab, so the canvas there is untouched background.
    assert_eq!(
        sample(&rgb, bounds, w, h, 2.0, 0.1),
        BACKGROUND,
        "the layer-0 seam is filtered out: no glyph at its (x, z)"
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 8.0, 0.5),
        BACKGROUND,
        "the layer-2 seam is filtered out: no glyph at its (x, z)"
    );
    // The base rectangles are the 247 role-mode rectangles recolored faint.
    assert_eq!(
        sample(&rgb, bounds, w, h, 1.0, 0.38),
        faint_base_color(),
        "base rectangles must be recolored FAINT_BASE under the isolated overlay"
    );
}

/// AC-6. Two isolated seam-overlay renders from the same inputs are
/// byte-identical with element-for-element equal event lists (in entries
/// source order) and equal warnings; the composited form is likewise
/// deterministic, and `seams: None` is byte-equivalent to the frozen styled
/// composite entry point.
#[test]
fn seam_overlay_render_is_deterministic() {
    let (captures, sched, seam_plan) = seam_fixture();
    let rendered_layers: BTreeSet<u32> = [1].into_iter().collect();
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);

    let first = render_silhouette_seam_overlay(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &seam_plan,
        &rendered_layers,
    )
    .expect("first isolated render must succeed");
    let second = render_silhouette_seam_overlay(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &seam_plan,
        &rendered_layers,
    )
    .expect("second isolated render must succeed");
    assert_eq!(
        first.0.png_bytes, second.0.png_bytes,
        "the isolated seam overlay must be byte-identical across runs"
    );
    assert_eq!(
        first.1, second.1,
        "event lists must be equal element-for-element, in source order"
    );
    assert_eq!(first.2, second.2, "warning lists must be equal");

    // Composited form: same seam glyphs onto the role-colored canvas.
    let style = RenderStyle::default();
    let composited_first = render_silhouette_composite_seamed(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
        Some((&seam_plan, &rendered_layers)),
    )
    .expect("first composited render must succeed");
    let composited_second = render_silhouette_composite_seamed(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
        Some((&seam_plan, &rendered_layers)),
    )
    .expect("second composited render must succeed");
    assert_eq!(
        composited_first.0.png_bytes, composited_second.0.png_bytes,
        "the composited seam render must be byte-identical across runs"
    );
    assert_eq!(composited_first.1, composited_second.1);
    assert_eq!(composited_first.2, composited_second.2);
    assert_eq!(
        composited_first.1, first.1,
        "both forms draw from the same filtered, source-ordered event list"
    );
    let (w, h, rgb) = decode_rgb(&composited_first.0.png_bytes);
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.3),
        seam_color(),
        "composited glyphs land at the same source coordinates"
    );

    // Delegation equivalence: `seams: None` must reproduce the frozen styled
    // composite byte-for-byte, with an empty event list.
    let styled = render_silhouette_composite_styled(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
    )
    .expect("the styled composite must render");
    let no_seams = render_silhouette_composite_seamed(
        &captures,
        SilhouetteView::Front,
        1,
        bounds,
        &sched,
        &style,
        None,
    )
    .expect("the seamed composite with seams: None must render");
    assert_eq!(
        styled.0.png_bytes, no_seams.0.png_bytes,
        "seams: None must be byte-equivalent to render_silhouette_composite_styled"
    );
    assert_eq!(styled.1, no_seams.2, "warnings pass through unchanged");
    assert!(
        no_seams.1.is_empty(),
        "seams: None yields an empty event list"
    );
}

// ---------------------------------------------------------------------------
// Packet 252, Step 1 — `PrePass::RegionMapping` silhouette extraction arm.
// ---------------------------------------------------------------------------

fn region_key(global_layer_index: u32, object_id: &str, region_id: u64) -> RegionKey {
    RegionKey {
        global_layer_index,
        object_id: object_id.to_string(),
        region_id,
        variant_chain: Vec::new(),
    }
}

fn mapping_region(
    object_id: &str,
    region_id: u64,
    effective_layer_height: f32,
    polygons: Vec<ExPolygon>,
) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        polygons,
        effective_layer_height,
        ..SlicedRegion::default()
    }
}

fn region_mapping_capture(
    layer_index: u32,
    layer_z: f32,
    region_map: RegionMapIR,
    slice_ir: Vec<SliceIR>,
) -> StageCapture {
    StageCapture {
        stage_id: "PrePass::RegionMapping".to_string(),
        layer_index,
        layer_z,
        ir: CapturedIr::RegionMapping {
            region_map,
            slice_ir,
        },
    }
}

fn overhang_capture(layer_z: f32, bands: Vec<QuartileBand>) -> StageCapture {
    let mut by_layer = HashMap::new();
    by_layer.insert(0, bands);
    let mut overhang_quartile_polygons = HashMap::new();
    overhang_quartile_polygons.insert("obj-0".to_string(), by_layer);
    StageCapture {
        stage_id: "PrePass::OverhangAnnotation".to_string(),
        layer_index: 0,
        layer_z,
        ir: CapturedIr::SurfaceClassification(SurfaceClassificationIR {
            overhang_quartile_polygons,
            ..SurfaceClassificationIR::default()
        }),
    }
}

fn overhang_index(classes: &[(f32, f32, f32, f32)]) -> slicer_runtime::SilhouetteSliceHeightIndex {
    build_silhouette_slice_height_index(&[SliceIR {
        global_layer_index: 0,
        regions: classes
            .iter()
            .map(|&(height, x0, x1, y1)| region(height, vec![rect_expolygon(x0, x1, 0.0, y1)]))
            .collect(),
        ..SliceIR::default()
    }])
}

fn overhang_bounds() -> ViewportBoundsMm {
    ViewportBoundsMm {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 10.0,
        max_y: 1.0,
    }
}

#[test]
fn overhang_bands_single_height_slabs_and_quartile_order() {
    let capture = overhang_capture(
        1.0,
        vec![
            QuartileBand {
                quartile: 1,
                polygons: vec![rect_expolygon(1.0, 9.0, 0.0, 1.0)],
            },
            QuartileBand {
                quartile: 4,
                polygons: vec![rect_expolygon(3.0, 7.0, 0.0, 1.0)],
            },
        ],
    );
    let index = overhang_index(&[(0.2, 0.0, 10.0, 1.0)]);
    let (image, _) = render_silhouette_overhang_composite(
        &[capture],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &index,
    )
    .expect("overhang bands render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!(
        sample(&rgb, overhang_bounds(), w, h, 5.0, 0.9),
        slicer_runtime::visual_debug_render::palette::OVERHANG_QUARTILE_4
    );
    assert_eq!(
        sample(&rgb, overhang_bounds(), w, h, 2.0, 0.9),
        slicer_runtime::visual_debug_render::palette::OVERHANG_QUARTILE_1
    );
    assert_eq!(sample(&rgb, overhang_bounds(), w, h, 2.0, 0.7), BACKGROUND);
}

#[test]
fn overhang_bands_partition_across_mixed_height_classes() {
    let capture = overhang_capture(
        1.0,
        vec![QuartileBand {
            quartile: 2,
            polygons: vec![
                rect_expolygon(0.0, 6.0, 0.0, 5.0),
                rect_expolygon(4.0, 10.0, 10.0, 15.0),
            ],
        }],
    );
    let index = build_silhouette_slice_height_index(&[SliceIR {
        global_layer_index: 0,
        regions: vec![
            region(0.2, vec![rect_expolygon(0.0, 6.0, 0.0, 5.0)]),
            region(0.6, vec![rect_expolygon(4.0, 10.0, 10.0, 15.0)]),
        ],
        ..SliceIR::default()
    }]);
    let (image, _) = render_silhouette_overhang_composite(
        &[capture],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &index,
    )
    .expect("mixed-height bands render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);
    let color = slicer_runtime::visual_debug_render::palette::OVERHANG_QUARTILE_2;
    assert_eq!(sample(&rgb, overhang_bounds(), w, h, 2.0, 0.9), color);
    assert_eq!(sample(&rgb, overhang_bounds(), w, h, 7.0, 0.5), color);
    assert_eq!(sample(&rgb, overhang_bounds(), w, h, 7.0, 0.3), BACKGROUND);
}

#[test]
fn overhang_composite_is_deterministic() {
    let capture = overhang_capture(
        1.0,
        vec![QuartileBand {
            quartile: 3,
            polygons: vec![rect_expolygon(1.0, 9.0, 0.0, 1.0)],
        }],
    );
    let index = overhang_index(&[(0.2, 0.0, 10.0, 1.0)]);
    let a = render_silhouette_overhang_composite(
        &[capture.clone()],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &index,
    )
    .unwrap();
    let b = render_silhouette_overhang_composite(
        &[capture],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &index,
    )
    .unwrap();
    assert_eq!(a.0.png_bytes, b.0.png_bytes);
    assert_eq!(a.1, b.1);
}

#[test]
fn overhang_invalid_quartile_fails_closed() {
    let capture = overhang_capture(
        1.0,
        vec![QuartileBand {
            quartile: 5,
            polygons: vec![rect_expolygon(0.0, 1.0, 0.0, 1.0)],
        }],
    );
    let err = render_silhouette_overhang_composite(
        &[capture],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &overhang_index(&[(0.2, 0.0, 10.0, 1.0)]),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidQuartile { quartile: 5, .. }
    ));
}

#[test]
fn overhang_empty_bands_fail_closed() {
    let capture = overhang_capture(1.0, Vec::new());
    let err = render_silhouette_overhang_composite(
        &[capture],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &overhang_index(&[(0.2, 0.0, 10.0, 1.0)]),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        RenderError::MissingGeometryField {
            field: "overhang_quartile_polygons",
            ..
        }
    ));
}

#[test]
fn overhang_missing_height_index_layer_fails_closed() {
    let capture = overhang_capture(
        1.0,
        vec![QuartileBand {
            quartile: 1,
            polygons: vec![rect_expolygon(0.0, 1.0, 0.0, 1.0)],
        }],
    );
    let err = render_silhouette_overhang_composite(
        &[capture],
        SilhouetteView::Front,
        1,
        overhang_bounds(),
        &Default::default(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        RenderError::MissingGeometryField {
            field: "silhouette_slice_height_index.layers",
            ..
        }
    ));
}

/// AC-1 (packet 252): a `CapturedIr::RegionMapping` capture joined against
/// its OWN retained `slice_ir` draws each joined region over its own slab
/// `[z - effective_layer_height, z]` — the catch-up-sized region's bottom
/// strictly below the other's, never one uniform slab — each painted its own
/// `config_tint` color.
#[test]
fn region_mapping_slabs_follow_joined_effective_layer_height() {
    // Layer top 1.0 mm. Region 0: normal 0.2 mm layer -> bottom 0.8.
    // Region 1: catch-up-sized 0.6 mm layer -> bottom 0.4.
    let mut region_map = RegionMapIR::default();
    let id_a = region_map.intern_config(ResolvedConfig {
        filament_diameter: 1.75,
        ..Default::default()
    });
    let id_b = region_map.intern_config(ResolvedConfig {
        filament_diameter: 2.85,
        ..Default::default()
    });
    region_map.entries.insert(
        region_key(0, "obj-0", 0),
        RegionPlan {
            config: id_a,
            ..RegionPlan::default()
        },
    );
    region_map.entries.insert(
        region_key(0, "obj-0", 1),
        RegionPlan {
            config: id_b,
            ..RegionPlan::default()
        },
    );
    // An entry on ANOTHER layer: filtered out of a layer-0 capture.
    region_map.entries.insert(
        region_key(1, "obj-0", 0),
        RegionPlan {
            config: id_a,
            ..RegionPlan::default()
        },
    );
    let slice_ir = vec![
        SliceIR {
            global_layer_index: 0,
            z: 1.0,
            regions: vec![
                mapping_region("obj-0", 0, 0.2, vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)]),
                mapping_region("obj-0", 1, 0.6, vec![rect_expolygon(20.0, 30.0, 0.0, 5.0)]),
            ],
            ..SliceIR::default()
        },
        SliceIR {
            global_layer_index: 1,
            z: 1.2,
            regions: vec![mapping_region(
                "obj-0",
                0,
                0.2,
                vec![rect_expolygon(40.0, 50.0, 0.0, 5.0)],
            )],
            ..SliceIR::default()
        },
    ];
    let captures = vec![region_mapping_capture(0, 1.0, region_map, slice_ir)];
    let sched = schedule(&[(0, 0.4, 1.0), (1, 1.0, 1.2)]);
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("a populated region-mapping capture group must render");
    assert!(
        warnings.is_empty(),
        "disjoint tint classes never occlude and every entry joins: {warnings:?}"
    );
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    // Each region paints its own `config_tint` color: a stable non-background
    // tint in the documented 60..=239 channel range, distinct per distinct
    // config content, and never the legacy Slice body palette color.
    let tint_a = sample(&rgb, bounds, w, h, 5.0, 0.9);
    let tint_b = sample(&rgb, bounds, w, h, 25.0, 0.9);
    for tint in [tint_a, tint_b] {
        assert_ne!(tint, BACKGROUND, "a joined region must be painted");
        assert_ne!(
            tint,
            body_color(),
            "region mapping paints config_tint, not the Slice body class"
        );
        assert!(
            tint.iter().all(|c| (60..=239).contains(c)),
            "config_tint channels stay in 60..=239, got {tint:?}"
        );
    }
    assert_ne!(
        tint_a, tint_b,
        "distinct ResolvedConfig contents must paint distinct tints"
    );

    // Region 0 stops at 0.8: below it is background.
    assert_eq!(
        sample(&rgb, bounds, w, h, 5.0, 0.6),
        BACKGROUND,
        "region 0's 0.2 mm slab must not reach down to region 1's bottom"
    );
    // Region 1 reaches to 0.4 at the same Z where region 0 is already gone.
    assert_eq!(
        sample(&rgb, bounds, w, h, 25.0, 0.6),
        tint_b,
        "region 1's 0.6 mm slab must still be painted below region 0's bottom"
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 25.0, 0.5),
        tint_b,
        "region 1's slab bottom is 0.4, not 0.8"
    );
    // And region 1 itself stops at 0.4 — the slab is exact, not unbounded.
    assert_eq!(sample(&rgb, bounds, w, h, 25.0, 0.2), BACKGROUND);

    // The other layer's entry is filtered out of this capture.
    assert_eq!(
        sample(&rgb, bounds, w, h, 45.0, 1.1),
        BACKGROUND,
        "a layer-1 RegionMapIR entry must not be drawn by the layer-0 capture"
    );
}

#[test]
fn region_mapping_nonpositive_height_fails_closed() {
    let mut region_map = RegionMapIR::default();
    let config = region_map.intern_config(ResolvedConfig::default());
    region_map.entries.insert(
        region_key(0, "obj-0", 0),
        RegionPlan {
            config,
            ..RegionPlan::default()
        },
    );
    let slice_ir = vec![SliceIR {
        global_layer_index: 0,
        regions: vec![mapping_region(
            "obj-0",
            0,
            0.0,
            vec![rect_expolygon(0.0, 1.0, 0.0, 1.0)],
        )],
        ..SliceIR::default()
    }];
    let capture = region_mapping_capture(0, 1.0, region_map, slice_ir);
    let schedule = schedule(&[(0, 0.8, 1.0)]);
    let bounds = compute_silhouette_viewport_bounds(
        std::slice::from_ref(&capture),
        SilhouetteView::Front,
        &schedule,
        None,
    );
    let err = render_silhouette_composite(&[capture], SilhouetteView::Front, 1, bounds, &schedule)
        .expect_err("nonpositive joined region height must fail closed");
    assert!(matches!(
        err,
        RenderError::MissingGeometryField {
            field: "slice_ir.regions.effective_layer_height",
            ..
        }
    ));
}

/// AC-2 (packet 252): two `RegionMapIR` entries on one layer with distinct
/// `ResolvedConfig` contents (distinct `config_tint` RGB triples) and
/// overlapping projected intervals — the overlap paints the
/// lexicographically-larger (r, g, b) tint (ascending-RGB class paint order,
/// later class wins); the occlusion warning fires naming the affected layer
/// count; rendering twice yields byte-identical PNG bytes.
#[test]
fn region_mapping_tint_class_order_and_determinism() {
    let mut region_map = RegionMapIR::default();
    let id_a = region_map.intern_config(ResolvedConfig {
        filament_diameter: 1.75,
        ..Default::default()
    });
    let id_b = region_map.intern_config(ResolvedConfig {
        filament_diameter: 2.85,
        ..Default::default()
    });
    region_map.entries.insert(
        region_key(0, "obj-0", 0),
        RegionPlan {
            config: id_a,
            ..RegionPlan::default()
        },
    );
    region_map.entries.insert(
        region_key(0, "obj-0", 1),
        RegionPlan {
            config: id_b,
            ..RegionPlan::default()
        },
    );
    let slice_ir = vec![SliceIR {
        global_layer_index: 0,
        z: 1.0,
        regions: vec![
            // Overlap on the projected X axis: [0, 10] and [5, 15].
            mapping_region("obj-0", 0, 0.2, vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)]),
            mapping_region("obj-0", 1, 0.2, vec![rect_expolygon(5.0, 15.0, 0.0, 5.0)]),
        ],
        ..SliceIR::default()
    }];
    let captures = vec![region_mapping_capture(0, 1.0, region_map, slice_ir)];
    let sched = schedule(&[(0, 0.8, 1.0)]);
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);

    let (image, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("overlapping tint classes must render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    // Learn each class' tint from its non-overlapping run, then the overlap
    // must paint the lexicographically-larger (r, g, b) triple.
    let tint_a = sample(&rgb, bounds, w, h, 2.0, 0.9);
    let tint_b = sample(&rgb, bounds, w, h, 13.0, 0.9);
    assert_ne!(tint_a, BACKGROUND);
    assert_ne!(tint_b, BACKGROUND);
    assert_ne!(
        tint_a, tint_b,
        "the fixture needs two distinct tints to witness paint order"
    );
    let winner = tint_a.max(tint_b);
    assert_eq!(
        sample(&rgb, bounds, w, h, 7.0, 0.9),
        winner,
        "the overlap must paint the lexicographically-larger tint: \
         classes paint in ascending (r, g, b) order, later class wins"
    );

    // 247's occlusion warning fires unchanged, naming the affected layer count.
    let occlusion: Vec<&String> = warnings
        .iter()
        .filter(|w| w.contains("silhouette occlusion"))
        .collect();
    assert_eq!(
        occlusion.len(),
        1,
        "exactly one deduped occlusion warning expected, got {warnings:?}"
    );
    assert!(
        occlusion[0].contains("1 layer(s)"),
        "the occlusion warning must name the affected layer count: {}",
        occlusion[0]
    );
    assert_eq!(
        warnings.len(),
        1,
        "every entry joined, so no region-mapping warning is due: {warnings:?}"
    );

    // Determinism: same inputs twice -> byte-identical PNG and warnings.
    let (image2, warnings2) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("the second render must succeed");
    assert_eq!(
        image.png_bytes, image2.png_bytes,
        "HashMap iteration order must never reach pixels: byte-identical re-render"
    );
    assert_eq!(warnings, warnings2);
}

/// AC-3 (packet 252): a `RegionMapIR` entry on a selected layer with no
/// matching `SlicedRegion` in the capture's retained `slice_ir` contributes
/// no pixels; the returned warnings contain ONE warning naming the
/// unjoined-entry count — never a silent drop.
#[test]
fn region_mapping_unjoined_entries_warn_and_skip() {
    let mut region_map = RegionMapIR::default();
    let id = region_map.intern_config(ResolvedConfig::default());
    let plan = || RegionPlan {
        config: id,
        ..RegionPlan::default()
    };
    // Joined: matches the layer-0 SliceIR row (obj-0, region 0).
    region_map.entries.insert(region_key(0, "obj-0", 0), plan());
    // Unjoined: region 99 has no SlicedRegion on layer 0.
    region_map
        .entries
        .insert(region_key(0, "obj-0", 99), plan());
    // Unjoined on ANOTHER layer: not counted by a layer-0 capture.
    region_map
        .entries
        .insert(region_key(1, "obj-0", 42), plan());
    let slice_ir = vec![SliceIR {
        global_layer_index: 0,
        z: 1.0,
        regions: vec![mapping_region(
            "obj-0",
            0,
            0.2,
            vec![rect_expolygon(0.0, 10.0, 0.0, 5.0)],
        )],
        ..SliceIR::default()
    }];
    let captures = vec![region_mapping_capture(0, 1.0, region_map, slice_ir)];
    let sched = schedule(&[(0, 0.8, 1.0)]);
    let bounds = compute_silhouette_viewport_bounds(&captures, SilhouetteView::Front, &sched, None);
    let (image, warnings) =
        render_silhouette_composite(&captures, SilhouetteView::Front, 1, bounds, &sched)
            .expect("the joined entry must still render");
    let (w, h, rgb) = decode_rgb(&image.png_bytes);

    // The joined entry paints; nothing else does.
    assert_ne!(
        sample(&rgb, bounds, w, h, 5.0, 0.9),
        BACKGROUND,
        "the joined entry must paint its interval"
    );
    assert_eq!(
        sample(&rgb, bounds, w, h, 11.0, 0.9),
        BACKGROUND,
        "the unjoined entry contributes no pixels"
    );

    // Exactly ONE warning, naming the unjoined-entry count for THIS layer
    // only — the layer-1 miss is not this capture's to report.
    assert_eq!(
        warnings,
        vec!["region mapping: 1 entries had no joined SliceIR region and were skipped".to_string()],
        "one deduped warning naming the unjoined-entry count, never a silent drop"
    );
}
