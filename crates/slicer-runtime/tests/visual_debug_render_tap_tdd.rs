//! Contract coverage for the packet 161 Step 6 renderer extension:
//! `crates/slicer-runtime/src/visual_debug_render.rs`'s `geometry_points_mm`
//! and `shapes_for` handling of the seven new `CapturedIr` variants
//! introduced in Steps 3-5 (`Slice`, `SurfaceClassification`, `SeamPlan`,
//! `SupportGeometry`, `RegionMapping`, `LayerFinalization`, `GCodeEmit`).
//!
//! Two contracts, one test each:
//! - `mixed_unit_shared_viewport`: a bundle mixing a `Point2`/`ExPolygon`
//!   (100 nm) source with three `f32`-millimeter sources
//!   (`SeamPlanIR.entries[].chosen_candidate.point`,
//!   `SupportPlanIR.entries[].branch_segments`, `GCodeIR::Move`) must
//!   produce ONE correct shared model-wide XY viewport — proving every
//!   source is projected in its own correct basis, not a mixed/rescaled one.
//! - `regionmapping_join_and_layerplanning_overlay`: the `RegionMapping`
//!   join half only (Step 6's scope) — real `SliceIR` region polygons,
//!   joined by the full `(global_layer_index, object_id, region_id,
//!   variant_chain)` tuple, tinted per the matched `RegionPlan`'s resolved
//!   config. The LayerPlanning-overlay + no-synthetic-mode assertions are
//!   Step 7's scope and are NOT asserted here — see the marked slot at the
//!   bottom of that test.
//!
//! Fixture style mirrors `visual_debug_blackboard_tap_tdd.rs`'s
//! `triangle_expolygon`/`seeded_*` helpers (small, deterministic, arbitrary
//! coordinates) — no new geometry generator is authored here.

use slicer_ir::{
    ActiveRegion, ExPolygon, ExtrusionPath3D, ExtrusionRole, GCodeCommand, GCodeIR, GlobalLayer,
    Point2, Point3WithWidth, Polygon, PrintMetadata, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SeamPlanEntry, SeamPlanIR, SeamPosition, SliceIR, SlicedRegion,
    SupportGeometryIR, SupportPlanEntry, SupportPlanIR, CURRENT_GCODE_IR_SCHEMA_VERSION,
    CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION, CURRENT_SLICE_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
};
use slicer_runtime::{
    compute_viewport_bounds, render_stage_capture, CapturedIr, GeometryView, Projector, RenderView,
    StageCapture, ViewportBoundsMm, BASE_DIMENSION_PX,
};

/// One populated `ExPolygon` triangle at arbitrary but deterministic
/// millimeter coordinates. Mirrors `visual_debug_blackboard_tap_tdd.rs`'s
/// `triangle_expolygon`, parameterized so each test can size the triangle to
/// its own bounds-pinning needs.
fn triangle_mm(x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x2, y2),
            ],
        },
        holes: Vec::new(),
    }
}

fn point3(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.0,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn approx_eq(a: f32, b: f32, eps: f32) {
    assert!(
        (a - b).abs() < eps,
        "expected ~{b}, got {a} (diff {})",
        (a - b).abs()
    );
}

/// A bundle mixing one `Point2`/`ExPolygon` (100 nm) source with three
/// `f32`-millimeter sources, each engineered to dominate a distinct edge of
/// the combined bounding box, so a units bug in ANY ONE source's conversion
/// is independently visible as a wrong bound on that specific edge:
///
/// - `Slice` (100 nm, needs `Point2::to_mm`) dominates `max_x` (500 mm). If
///   projected in raw 100-nm units instead of mm, `max_x` would jump from
///   ~545 to ~5,000,000 (a ~10000x blow-up — `UNITS_PER_MM` is 10 000).
/// - `SeamPlan` (already mm) dominates `min_x` (-400 mm). If wrongly passed
///   through a second 100-nm-to-mm conversion, it would collapse toward 0.
/// - `SupportGeometry.plan.branch_segments` (already mm) dominates `max_y`
///   (600 mm), same collapse-toward-0 risk if double-converted.
/// - `GCodeEmit` (already mm) dominates `min_y` (-700 mm), same risk.
#[test]
fn mixed_unit_shared_viewport() {
    let slice_capture = StageCapture {
        stage_id: "Layer::Slice".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::Slice(SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.0,
            regions: vec![SlicedRegion {
                object_id: "obj-0".to_string(),
                region_id: 7,
                polygons: vec![triangle_mm(0.0, 0.0, 500.0, 0.0, 0.0, 5.0)],
                ..SlicedRegion::default()
            }],
        }),
    };

    let seam_capture = StageCapture {
        stage_id: "PrePass::SeamPlanning".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::SeamPlan(SeamPlanIR {
            schema_version: CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
            entries: vec![SeamPlanEntry {
                region_key: RegionKey::default(),
                chosen_candidate: SeamPosition {
                    point: point3(-400.0, 100.0),
                    wall_index: 0,
                },
                scored_candidates: Vec::new(),
            }],
        }),
    };

    let support_capture = StageCapture {
        stage_id: "PrePass::SupportGeometry".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::SupportGeometry {
            geometry: SupportGeometryIR::default(),
            plan: SupportPlanIR {
                schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
                entries: vec![SupportPlanEntry {
                    global_layer_index: 0,
                    object_id: "obj-0".to_string(),
                    region_id: 7,
                    branch_segments: vec![ExtrusionPath3D {
                        points: vec![point3(50.0, 600.0), point3(60.0, 600.0)],
                        role: ExtrusionRole::SupportMaterial,
                        speed_factor: 1.0,
                    }],
                }],
                raft_plan: None,
            },
        },
    };

    let gcode_capture = StageCapture {
        stage_id: "PostPass::GCodeEmit".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::GCodeEmit(GCodeIR {
            schema_version: CURRENT_GCODE_IR_SCHEMA_VERSION,
            commands: vec![GCodeCommand::Move {
                x: Some(10.0),
                y: Some(-700.0),
                z: None,
                e: None,
                f: None,
                role: ExtrusionRole::OuterWall,
            }],
            metadata: PrintMetadata::default(),
        }),
    };

    let captures = vec![slice_capture, seam_capture, support_capture, gcode_capture];
    let bounds = compute_viewport_bounds(&captures);

    // True combined extent (pre-margin): x in [-400, 500], y in [-700, 600].
    // AC-4's fixed margin is an absolute millimeter distance, identical on
    // both axes. It was previously 5% of each axis' own extent — which made
    // the margin itself anisotropic (45 mm in x vs 65 mm here in y), skewing
    // a non-square viewport before projection even began.
    let margin = slicer_runtime::VIEWPORT_MARGIN_MM;
    let expected = ViewportBoundsMm {
        min_x: -400.0 - margin,
        max_x: 500.0 + margin,
        min_y: -700.0 - margin,
        max_y: 600.0 + margin,
    };

    approx_eq(bounds.min_x, expected.min_x, 0.1);
    approx_eq(bounds.max_x, expected.max_x, 0.1);
    approx_eq(bounds.min_y, expected.min_y, 0.1);
    approx_eq(bounds.max_y, expected.max_y, 0.1);

    // Pin against the ~10000x blow-up a Point2/ExPolygon source would cause
    // if projected in raw 100-nm units instead of mm (UNITS_PER_MM = 10_000):
    // the Slice triangle's 500 mm point would read back as ~5,000,000.
    assert!(
        bounds.max_x < 1000.0,
        "max_x blew up to {} — Point2 100 nm source likely projected without the mm-conversion helper",
        bounds.max_x
    );
    // Pin against an f32-mm source silently collapsing toward 0 if it were
    // wrongly passed through a second (100 nm -> mm) conversion: each of
    // min_x/max_y/min_y is dominated by a value with |mm| >= 400, so a
    // collapse would pull that bound back toward the other sources' range
    // (roughly [-5, 500]), clearly outside this tolerance.
    assert!(
        bounds.min_x < -300.0,
        "min_x collapsed to {} — SeamPlan's already-mm point likely double-converted",
        bounds.min_x
    );
    assert!(
        bounds.max_y > 500.0,
        "max_y collapsed to {} — SupportPlan branch_segments' already-mm point likely double-converted",
        bounds.max_y
    );
    assert!(
        bounds.min_y < -600.0,
        "min_y collapsed to {} — GCodeEmit's already-mm Move point likely double-converted",
        bounds.min_y
    );
}

/// Decode an encoded RGB8 PNG (as produced by `render_stage_capture`) back
/// into `(width, height, rgb_bytes)`. `png` is already a direct dependency
/// of `slicer-runtime` (it is what `visual_debug_render.rs` encodes with),
/// so this is available to the integration test without a new dependency.
fn decode_rgb(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder
        .read_info()
        .expect("render_stage_capture always encodes a valid PNG header");
    let mut buf = vec![
        0u8;
        reader
            .output_buffer_size()
            .expect("render_stage_capture always encodes a fixed-size RGB8 buffer")
    ];
    let info = reader
        .next_frame(&mut buf)
        .expect("render_stage_capture always encodes a valid PNG frame");
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}

/// Sample the pixel a known mm point lands on, using the renderer's **real**
/// `Projector` rather than a copy of its arithmetic.
///
/// This previously reimplemented `Canvas::to_px`'s mapping by hand. That copy
/// was the reason the renderer's aspect-ratio bug (independent per-axis
/// scaling onto a square canvas) was invisible to this suite for two packets:
/// the test and the code were wrong in exactly the same way, so they agreed.
/// Always project through `Projector` — never restate the transform here.
fn mm_to_px(bounds: ViewportBoundsMm, width: u32, height: u32, x: f32, y: f32) -> (usize, usize) {
    let (px, py) = Projector::new(bounds, width, height).project(f64::from(x), f64::from(y));
    (px.round().max(0.0) as usize, py.round().max(0.0) as usize)
}

fn pixel_at(rgb: &[u8], width: u32, x: usize, y: usize) -> [u8; 3] {
    let idx = (y * width as usize + x) * 3;
    [rgb[idx], rgb[idx + 1], rgb[idx + 2]]
}

const BACKGROUND: [u8; 3] = [255, 255, 255];

/// `RegionMapping` join half (packet 161, Step 6's scope): a `RegionMapIR`
/// entry keyed `(global_layer_index: 0, object_id: "obj-0", region_id: 7,
/// variant_chain: [])` joined against a `SliceIR` carrying that exact
/// `SlicedRegion`, drawn tinted by the matched `RegionPlan`'s resolved
/// config — proven three ways:
/// 1. The render succeeds at all (previously `Err(MissingGeometryField)`
///    with a "packet 161 Step 6" placeholder message — a regression pin on
///    the fail-closed placeholder being replaced).
/// 2. The fill lands exactly on the joined polygon: non-background inside
///    the triangle, background just outside it.
/// 3. The fill color is a genuine function of the joined `RegionPlan`'s
///    resolved config: identical config -> identical color across repeated
///    render calls (AC-5 purity); different config -> different color.
#[test]
fn regionmapping_join_and_layerplanning_overlay() {
    let region_key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-0".to_string(),
        region_id: 7,
        variant_chain: Vec::new(),
    };
    let slice_ir = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.0,
        regions: vec![SlicedRegion {
            object_id: "obj-0".to_string(),
            region_id: 7,
            polygons: vec![triangle_mm(0.0, 0.0, 20.0, 0.0, 0.0, 20.0)],
            ..SlicedRegion::default()
        }],
    };

    let mut region_map_a = RegionMapIR::default();
    let config_a = ResolvedConfig::default();
    let config_id_a = region_map_a.intern_config(config_a);
    region_map_a.entries.insert(
        region_key.clone(),
        RegionPlan {
            config: config_id_a,
            ..RegionPlan::default()
        },
    );

    let mut region_map_b = RegionMapIR::default();
    let mut config_b = ResolvedConfig {
        layer_height: 0.9,
        ..ResolvedConfig::default()
    };
    config_b.extensions.insert(
        "test_marker".to_string(),
        slicer_ir::ConfigValue::String("b".to_string()),
    );
    let config_id_b = region_map_b.intern_config(config_b);
    region_map_b.entries.insert(
        region_key.clone(),
        RegionPlan {
            config: config_id_b,
            ..RegionPlan::default()
        },
    );

    let capture_a = StageCapture {
        stage_id: "PrePass::RegionMapping".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::RegionMapping {
            region_map: region_map_a,
            slice_ir: vec![slice_ir.clone()],
        },
    };
    let capture_b = StageCapture {
        stage_id: "PrePass::RegionMapping".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::RegionMapping {
            region_map: region_map_b,
            slice_ir: vec![slice_ir],
        },
    };

    let viewport = compute_viewport_bounds(std::slice::from_ref(&capture_a));

    let rendered_a1 = render_stage_capture(
        &capture_a,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        viewport,
    )
    .expect("RegionMapping join must render, not fail closed on the Step-6 placeholder");
    let rendered_a2 = render_stage_capture(
        &capture_a,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        viewport,
    )
    .expect("RegionMapping join must render on a repeated call");
    let rendered_b = render_stage_capture(
        &capture_b,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        viewport,
    )
    .expect("RegionMapping join must render for a differently-configured RegionPlan");

    assert_eq!(
        rendered_a1.png_bytes, rendered_a2.png_bytes,
        "pure function: identical (capture, view, scale, viewport) must render byte-identical PNGs (AC-5)"
    );

    let width = BASE_DIMENSION_PX;
    let height = BASE_DIMENSION_PX;
    assert_eq!(rendered_a1.width, width);
    assert_eq!(rendered_a1.height, height);

    let (_, _, rgb_a1) = decode_rgb(&rendered_a1.png_bytes);
    let (_, _, rgb_b) = decode_rgb(&rendered_b.png_bytes);

    // Interior point: (5, 5) is inside the (0,0)-(20,0)-(0,20) triangle
    // (5 + 5 = 10 <= 20). Exterior point: (15, 15) is outside it
    // (15 + 15 = 30 > 20), but still inside the shared viewport.
    let (ix, iy) = mm_to_px(viewport, width, height, 5.0, 5.0);
    let (ex, ey) = mm_to_px(viewport, width, height, 15.0, 15.0);

    let interior_a1 = pixel_at(&rgb_a1, width, ix, iy);
    let exterior_a1 = pixel_at(&rgb_a1, width, ex, ey);
    let interior_b = pixel_at(&rgb_b, width, ix, iy);

    assert_ne!(
        interior_a1, BACKGROUND,
        "expected the joined SliceIR region's polygon fill inside the triangle; got background — join is missing/not drawing"
    );
    assert_eq!(
        exterior_a1, BACKGROUND,
        "expected background outside the joined polygon; got a fill — join drew geometry that isn't the real SlicedRegion shape"
    );
    assert_ne!(
        interior_a1, interior_b,
        "expected the fill color to depend on the matched RegionPlan's resolved config; two different configs produced the same tint"
    );

    // ─── Step 7: LayerPlanIR flags render ONLY as a diagnostic_overlay
    // annotation on a geometry tap (never a standalone tap/CapturedIr
    // variant) ────────────────────────────────────────────────────────────
    // `LayerPlanning` never appears in any of the three disjoint tap-id
    // sets `pnp-cli`'s `run_model_source` partitions requested taps into.
    for &tap in slicer_runtime::SUPPORTED_TAP_STAGE_IDS
        .iter()
        .chain(slicer_runtime::layer_executor::BLACKBOARD_TAP_STAGE_IDS.iter())
        .chain(slicer_runtime::layer_executor::POSTPASS_TAP_STAGE_IDS.iter())
    {
        assert!(
            !tap.contains("LayerPlanning") && !tap.contains("LayerPlan::"),
            "LayerPlanning must not be a standalone tap; found {tap} in a tap-id set"
        );
    }

    let global_layer = GlobalLayer {
        index: 0,
        active_regions: vec![ActiveRegion::default(), ActiveRegion::default()],
        has_nonplanar: true,
        is_sync_layer: true,
        ..GlobalLayer::default()
    };

    // Composed onto the SAME `RegionMapping` geometry tap used for the join
    // assertions above — proving the flags are an annotation on a geometry
    // tap's `diagnostic_overlay` view, not a separate render path.
    let overlay_with_layerplan =
        slicer_runtime::visual_debug_render::render_stage_capture_with_layer_plan(
            &capture_a,
            RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
            1,
            viewport,
            Some(&global_layer),
        )
        .expect("diagnostic_overlay + LayerPlanIR annotation must render on a geometry tap");
    let overlay_without_layerplan =
        slicer_runtime::visual_debug_render::render_stage_capture_with_layer_plan(
            &capture_a,
            RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
            1,
            viewport,
            None,
        )
        .expect("diagnostic_overlay is still valid with no LayerPlanIR threaded in (opt-in)");
    // Same capture/view, but a plain `Geometry` request: the flags must
    // never leak onto a non-overlay render even if a `LayerPlanIR` was
    // passed in — proving there is no separate standalone LayerPlanning
    // rendering path, only the opt-in `DiagnosticOverlay` annotation.
    let geometry_with_layerplan =
        slicer_runtime::visual_debug_render::render_stage_capture_with_layer_plan(
            &capture_a,
            RenderView::Geometry(GeometryView::FilledAreas),
            1,
            viewport,
            Some(&global_layer),
        )
        .expect("plain Geometry view must still render even if a LayerPlanIR was passed");

    let (_, _, rgb_with) = decode_rgb(&overlay_with_layerplan.png_bytes);
    let (_, _, rgb_without) = decode_rgb(&overlay_without_layerplan.png_bytes);
    let (_, _, rgb_geometry_with) = decode_rgb(&geometry_with_layerplan.png_bytes);

    let sync_marker = slicer_runtime::visual_debug_render::palette::OVERLAY_LAYERPLAN_SYNC;
    let nonplanar_marker =
        slicer_runtime::visual_debug_render::palette::OVERLAY_LAYERPLAN_NONPLANAR;
    let active_region_marker =
        slicer_runtime::visual_debug_render::palette::OVERLAY_LAYERPLAN_ACTIVE_REGION;

    assert_eq!(
        pixel_at(&rgb_with, width, 10, 10),
        sync_marker,
        "is_sync_layer=true must draw the sync marker on the diagnostic_overlay"
    );
    assert_eq!(
        pixel_at(&rgb_with, width, 10, 30),
        nonplanar_marker,
        "has_nonplanar=true must draw the non-planar marker on the diagnostic_overlay"
    );
    assert_eq!(
        pixel_at(&rgb_with, width, 10, 50),
        active_region_marker,
        "active_regions[0] must draw an active-region marker on the diagnostic_overlay"
    );
    assert_eq!(
        pixel_at(&rgb_with, width, 30, 50),
        active_region_marker,
        "active_regions[1] must draw a second active-region marker on the diagnostic_overlay"
    );

    assert_ne!(
        pixel_at(&rgb_without, width, 10, 10),
        sync_marker,
        "opt-in: no LayerPlanIR threaded in must mean no sync marker is drawn"
    );
    assert_ne!(
        pixel_at(&rgb_geometry_with, width, 10, 10),
        sync_marker,
        "LayerPlanIR flags must render only on the diagnostic_overlay view, never on plain Geometry"
    );

    // ─── Step 7: no synthetic-diagram render mode ──────────────────────
    // Every implemented `CapturedIr` variant must route through the real
    // geometry/overlay renderer. Proven for three structurally distinct
    // variant kinds beyond `RegionMapping` (already exercised above): a
    // simple Blackboard-read tap (`Slice`), a PostPass whole-print tap
    // (`GCodeEmit`), and the point-only `SeamPlan` tap, whose base-geometry
    // emptiness is real (documented, AC-correct) rather than an error or a
    // placeholder image — each renders geometry that actually depends on
    // its own input, which a synthetic/generic placeholder renderer would
    // not do.
    let slice_capture = StageCapture {
        stage_id: "Layer::Slice".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::Slice(SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.0,
            regions: vec![SlicedRegion {
                object_id: "obj-9".to_string(),
                region_id: 3,
                polygons: vec![triangle_mm(100.0, 100.0, 140.0, 100.0, 100.0, 140.0)],
                ..SlicedRegion::default()
            }],
        }),
    };
    let slice_viewport = compute_viewport_bounds(std::slice::from_ref(&slice_capture));
    let slice_rendered = render_stage_capture(
        &slice_capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        slice_viewport,
    )
    .expect("Slice must render through the real per-variant renderer, not a synthetic fallback");
    let (_, _, slice_rgb) = decode_rgb(&slice_rendered.png_bytes);
    let (six, siy) = mm_to_px(slice_viewport, width, height, 110.0, 110.0);
    assert_ne!(
        pixel_at(&slice_rgb, width, six, siy),
        BACKGROUND,
        "Slice's own triangle must actually be drawn — a synthetic/placeholder renderer would not vary with input geometry"
    );

    let gcode_capture = StageCapture {
        stage_id: "PostPass::GCodeEmit".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::GCodeEmit(GCodeIR {
            schema_version: CURRENT_GCODE_IR_SCHEMA_VERSION,
            commands: vec![
                GCodeCommand::Move {
                    x: Some(-50.0),
                    y: Some(-50.0),
                    z: None,
                    e: None,
                    f: None,
                    role: ExtrusionRole::OuterWall,
                },
                GCodeCommand::Move {
                    x: Some(-40.0),
                    y: Some(-50.0),
                    z: None,
                    e: None,
                    f: None,
                    role: ExtrusionRole::OuterWall,
                },
            ],
            metadata: PrintMetadata::default(),
        }),
    };
    let gcode_viewport = compute_viewport_bounds(std::slice::from_ref(&gcode_capture));
    let gcode_rendered = render_stage_capture(
        &gcode_capture,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        gcode_viewport,
    )
    .expect(
        "GCodeEmit must render through the real per-variant renderer, not a synthetic fallback",
    );
    let (_, _, gcode_rgb) = decode_rgb(&gcode_rendered.png_bytes);
    let (gx, gy) = mm_to_px(gcode_viewport, width, height, -45.0, -50.0);
    assert_ne!(
        pixel_at(&gcode_rgb, width, gx, gy),
        BACKGROUND,
        "GCodeEmit's own move line must actually be drawn — a synthetic/placeholder renderer would not vary with input geometry"
    );

    let seam_capture = StageCapture {
        stage_id: "PrePass::SeamPlanning".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::SeamPlan(SeamPlanIR {
            schema_version: CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
            entries: vec![SeamPlanEntry {
                region_key: RegionKey::default(),
                chosen_candidate: SeamPosition {
                    point: point3(-30.0, 30.0),
                    wall_index: 0,
                },
                scored_candidates: Vec::new(),
            }],
        }),
    };
    let seam_viewport = compute_viewport_bounds(std::slice::from_ref(&seam_capture));
    let seam_base = render_stage_capture(
        &seam_capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        seam_viewport,
    )
    .expect("SeamPlan base geometry is legitimately empty, not an error");
    let (_, _, seam_base_rgb) = decode_rgb(&seam_base.png_bytes);
    assert!(
        seam_base_rgb.chunks_exact(3).all(|px| px == BACKGROUND),
        "SeamPlan's base geometry view has no area/path to draw; a non-background pixel here would mean a synthetic placeholder shape was drawn instead"
    );
    let seam_overlay = slicer_runtime::visual_debug_render::render_stage_capture_with_layer_plan(
        &seam_capture,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        seam_viewport,
        None,
    )
    .expect("SeamPlan diagnostic overlay must render the real seam marker");
    let (_, _, seam_overlay_rgb) = decode_rgb(&seam_overlay.png_bytes);
    let (spx, spy) = mm_to_px(seam_viewport, width, height, -30.0, 30.0);
    assert_eq!(
        pixel_at(&seam_overlay_rgb, width, spx, spy),
        slicer_runtime::visual_debug_render::palette::OVERLAY_SEAM,
        "SeamPlan's diagnostic overlay must draw the real chosen_candidate.point seam marker, mirroring Perimeter's arm — not a synthetic generic marker"
    );

    // Exhaustive match, no wildcard arm: if a `LayerPlanning` variant were
    // ever added to `CapturedIr`, this stops compiling instead of silently
    // falling through to a synthetic catch-all — a compile-time pin on both
    // "LayerPlanning has no standalone CapturedIr variant" and "every
    // variant is handled explicitly, none via a synthetic fallback arm".
    fn assert_every_variant_handled_explicitly(ir: &CapturedIr) {
        match ir {
            CapturedIr::Perimeter(_)
            | CapturedIr::Infill(_)
            | CapturedIr::Support(_)
            | CapturedIr::LayerCollection(_)
            | CapturedIr::Slice(_)
            | CapturedIr::SurfaceClassification(_)
            | CapturedIr::SeamPlan(_)
            | CapturedIr::SupportGeometry { .. }
            | CapturedIr::RegionMapping { .. }
            | CapturedIr::LayerFinalization(_)
            | CapturedIr::GCodeEmit(_) => {}
        }
    }
    assert_every_variant_handled_explicitly(&capture_a.ir);
    assert_every_variant_handled_explicitly(&slice_capture.ir);
    assert_every_variant_handled_explicitly(&gcode_capture.ir);
    assert_every_variant_handled_explicitly(&seam_capture.ir);
}
