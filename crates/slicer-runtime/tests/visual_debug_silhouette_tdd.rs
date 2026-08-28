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

use slicer_ir::{
    ExPolygon, Point2, Polygon, SliceIR, SlicedRegion, SupportGeometryIR, SupportGeometryKey,
    SupportPlanEntry, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
};
use slicer_runtime::{
    compute_silhouette_viewport_bounds, render_silhouette_composite, union_silhouette_intervals,
    CapturedIr, Projector, RenderError, SilhouetteScheduleSlab, SilhouetteSlabSchedule,
    SilhouetteView, StageCapture, ViewportBoundsMm,
};
use std::collections::HashMap;

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
