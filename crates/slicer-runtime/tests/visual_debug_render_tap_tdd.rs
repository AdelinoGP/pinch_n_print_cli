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
    ActiveRegion, ExPolygon, ExtrusionRole, GCodeCommand, GCodeIR, GlobalLayer, Point2, Point3,
    Point3WithWidth, Polygon, PrintMetadata, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SeamPlanEntry, SeamPlanIR, SeamPosition, SliceIR, SlicedRegion, SupportGeometryIR,
    SupportPlanEntry, SupportPlanIR, CURRENT_GCODE_IR_SCHEMA_VERSION,
    CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION, CURRENT_SLICE_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
};
use slicer_runtime::{
    compute_viewport_bounds, render_stage_capture, CapturedIr, GeometryView, Projector, RenderView,
    StageCapture, ViewportBoundsMm, BASE_DIMENSION_PX,
};
use slicer_wasm_host::preparation::{
    freeze_native, CommittedPreparationInputs, CommittedPreparationProjection,
    NativePreparationArtifact, PreparationInputData, PreparationOwner,
    PreparationProjectionRequest, PreparationSelectionContext, PreparedOwnerStore,
};
use std::sync::Arc;

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
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
        ..Default::default()
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
                ..Default::default()
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
                // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
                entries: vec![SupportPlanEntry {
                    global_layer_index: 0,
                    object_id: "obj-0".to_string(),
                    region_id: 7,
                    skeleton: Some(slicer_ir::SupportPlanSkeleton {
                        points: vec![
                            Point3 {
                                x: 50.0,
                                y: 600.0,
                                z: 0.0,
                            },
                            Point3 {
                                x: 60.0,
                                y: 600.0,
                                z: 0.0,
                            },
                        ],
                        wall_counts: vec![0, 0],
                    }),
                    family_id: "tree".into(),
                    demand_ids: vec![],
                    body_ids: vec![],
                    anchor_layer_index: 0,
                    anchor_z: 0,
                    roles: vec![],
                    capabilities: vec![],
                    provenance: vec![],
                    decline_reason: None,
                    variant_chain: Vec::new(), // genuinely chain-empty source: this fixture seeds no paint chain
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

#[test]
fn degenerate_support_branch_renders_as_visible_disk() {
    let capture = StageCapture {
        stage_id: "Layer::Support".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::SupportGeometry {
            geometry: SupportGeometryIR::default(),
            plan: SupportPlanIR {
                schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
                // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
                entries: vec![SupportPlanEntry {
                    global_layer_index: 0,
                    object_id: "obj-0".to_string(),
                    region_id: 0,
                    skeleton: Some(slicer_ir::SupportPlanSkeleton {
                        points: vec![
                            Point3 {
                                x: 0.0,
                                y: 0.0,
                                z: 0.0,
                            },
                            Point3 {
                                x: 0.0,
                                y: 0.0,
                                z: 0.0,
                            },
                        ],
                        wall_counts: vec![0, 0],
                    }),
                    family_id: "tree".into(),
                    demand_ids: vec![],
                    body_ids: vec![],
                    anchor_layer_index: 0,
                    anchor_z: 0,
                    roles: vec![],
                    capabilities: vec![],
                    provenance: vec![],
                    decline_reason: None,
                    variant_chain: Vec::new(), // genuinely chain-empty source: this fixture seeds no paint chain
                }],
                raft_plan: None,
            },
        },
    };
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));
    let rendered = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .expect("degenerate support branch should render");
    let (_, _, rgb) = decode_rgb(&rendered.png_bytes);
    let painted = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .count();
    assert!(
        painted > 1,
        "degenerate branch rendered only {painted} pixel"
    );
}

#[test]
fn degenerate_segment_at_path_end_renders_as_visible_disk() {
    let capture = StageCapture {
        stage_id: "Layer::Support".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::SupportGeometry {
            geometry: SupportGeometryIR::default(),
            plan: SupportPlanIR {
                schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
                // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
                entries: vec![SupportPlanEntry {
                    global_layer_index: 0,
                    object_id: "obj-0".to_string(),
                    region_id: 0,
                    skeleton: Some(slicer_ir::SupportPlanSkeleton {
                        points: vec![
                            Point3 {
                                x: 3.0,
                                y: 9.557,
                                z: 0.0,
                            },
                            Point3 {
                                x: 3.0,
                                y: 9.808,
                                z: 0.0,
                            },
                            Point3 {
                                x: 3.0,
                                y: 9.808,
                                z: 0.0,
                            },
                        ],
                        wall_counts: vec![0, 0, 0],
                    }),
                    family_id: "tree".into(),
                    demand_ids: vec![],
                    body_ids: vec![],
                    anchor_layer_index: 0,
                    anchor_z: 0,
                    roles: vec![],
                    capabilities: vec![],
                    provenance: vec![],
                    decline_reason: None,
                    variant_chain: Vec::new(), // genuinely chain-empty source: this fixture seeds no paint chain
                }],
                raft_plan: None,
            },
        },
    };
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));
    let rendered = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .expect("mixed support branch should render");
    let (_, _, rgb) = decode_rgb(&rendered.png_bytes);
    let painted = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .count();
    assert!(
        painted > 1,
        "degenerate segment rendered only {painted} pixel"
    );
}

#[test]
fn nearly_equal_path_endpoints_render_as_visible_disk() {
    let capture = StageCapture {
        stage_id: "Layer::Support".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::SupportGeometry {
            geometry: SupportGeometryIR::default(),
            plan: SupportPlanIR {
                schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
                // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
                entries: vec![SupportPlanEntry {
                    global_layer_index: 0,
                    object_id: "obj-0".to_string(),
                    region_id: 0,
                    skeleton: Some(slicer_ir::SupportPlanSkeleton {
                        points: vec![
                            Point3 {
                                x: 3.0,
                                y: 9.808,
                                z: 0.0,
                            },
                            Point3 {
                                x: 3.0 + 1e-9,
                                y: 9.808,
                                z: 0.0,
                            },
                        ],
                        wall_counts: vec![0, 0],
                    }),
                    family_id: "tree".into(),
                    demand_ids: vec![],
                    body_ids: vec![],
                    anchor_layer_index: 0,
                    anchor_z: 0,
                    roles: vec![],
                    capabilities: vec![],
                    provenance: vec![],
                    decline_reason: None,
                    variant_chain: Vec::new(), // genuinely chain-empty source: this fixture seeds no paint chain
                }],
                raft_plan: None,
            },
        },
    };
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));
    let rendered = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .expect("nearly degenerate support branch should render");
    let (_, _, rgb) = decode_rgb(&rendered.png_bytes);
    let painted = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .count();
    assert!(
        painted > 1,
        "nearly degenerate branch rendered only {painted} pixel"
    );
}

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
                ..Default::default()
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
        seam_base_rgb
            .as_chunks::<3>()
            .0
            .iter()
            .all(|px| *px == BACKGROUND),
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
            CapturedIr::PreparationProjection(_) => {}
        }
    }
    assert_every_variant_handled_explicitly(&capture_a.ir);
    assert_every_variant_handled_explicitly(&slice_capture.ir);
    assert_every_variant_handled_explicitly(&gcode_capture.ir);
    assert_every_variant_handled_explicitly(&seam_capture.ir);
}

/// Execute the real native fixture against production committed-input,
/// selection, transaction and publication adapters. No ready plan is injected.
fn prepared_projection(
    mode: &str,
    two_layers: bool,
) -> (
    Arc<CommittedPreparationProjection>,
    PreparedOwnerStore,
    PreparationOwner,
) {
    use preparation_visual_guest::PreparationVisualGuest;
    use slicer_ir::{ConfigValue, ConfigView, LayerPlanIR, MeshIR};
    let binding = freeze_native(
        "test.preparation-visual".into(),
        Some(slicer_schema::preparation::PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec!["layer_plan".into(), "selected_targets".into()],
            vec!["probe_xy".into()],
        )),
        NativePreparationArtifact::Prepared {
            entry: PreparationVisualGuest::__slicer_prepared_native_entry(),
            schema: PreparationVisualGuest::__SLICER_PREPARATION_SCHEMA,
        },
    )
    .expect("freeze production native fixture");
    let store = PreparedOwnerStore::new();
    let owner = store.create_owner(&binding).unwrap();
    let mut resolved = ResolvedConfig::default();
    resolved.extensions.insert(
        "preparation_visual_mode".into(),
        ConfigValue::String(mode.into()),
    );
    let mut regions = RegionMapIR::default();
    let config = regions.intern_config(resolved);
    let mut layers = Vec::new();
    let mut slices = Vec::new();
    for &(index, z) in if two_layers {
        &[(0, 0.2), (1, 0.4)][..]
    } else {
        &[(0, 0.2)][..]
    } {
        let region = SlicedRegion {
            object_id: "probe-object".into(),
            region_id: 7,
            effective_layer_height: 0.2,
            ..Default::default()
        };
        regions.entries.insert(
            slicer_wasm_host::selection::region_map_key_for_sliced_region(index, &region),
            RegionPlan {
                config,
                ..Default::default()
            },
        );
        layers.push(GlobalLayer {
            index,
            z,
            ..Default::default()
        });
        slices.push(SliceIR {
            global_layer_index: index,
            z,
            regions: vec![region],
            ..Default::default()
        });
    }
    let committed = CommittedPreparationInputs::new(
        Arc::new(MeshIR::default()),
        Arc::new(LayerPlanIR {
            global_layers: layers,
            ..Default::default()
        }),
        Arc::new(slices),
        Arc::new(regions),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let input = PreparationInputData::from_committed(
        &owner,
        Arc::new(committed),
        PreparationSelectionContext::new(
            "Layer::PathOptimization".into(),
            "test.preparation-visual".into(),
            vec!["path-optimizer".into()],
            Default::default(),
            Arc::new(ConfigView::from_map(
                [(
                    "preparation_visual_mode".into(),
                    ConfigValue::String(mode.into()),
                )]
                .into_iter()
                .collect(),
            )),
        ),
    )
    .unwrap();
    let targets = input.host_selected_targets(&owner, 0, 0, u64::MAX).unwrap();
    assert_eq!(targets.len(), 1);
    assert!(targets[0].invoke);
    assert_eq!(
        targets[0].config.get_string("preparation_visual_mode"),
        Some(mode)
    );
    binding
        .prepare_with_diagnostics(
            &owner,
            input,
            PreparationProjectionRequest::new(vec!["probe_xy".into()]).unwrap(),
            |_| Ok(()),
        )
        .expect("real fixture must finish Ready");
    let snapshot = store
        .current(&owner)
        .expect("Ready committed plan")
        .projection("probe_xy")
        .unwrap()
        .expect("explicitly completed scene");
    (snapshot, store, owner)
}

fn projection_capture(
    snapshot: &Arc<CommittedPreparationProjection>,
    layer_index: u32,
) -> StageCapture {
    // The fixture commits this exact schedule. Empty completed scenes have no
    // primitive Z attachments; capture Z still comes from the committed schedule.
    let layer_z = match layer_index {
        0 => 0.2,
        1 => 0.4,
        _ => panic!("layer is absent from fixture schedule"),
    };
    if let Some(attached_z) = snapshot.layer_z(layer_index) {
        assert_eq!(attached_z, layer_z);
    }
    StageCapture {
        stage_id: "Layer::PathOptimization".into(),
        layer_index,
        layer_z,
        ir: CapturedIr::PreparationProjection(Arc::clone(snapshot)),
    }
}

#[test]
fn preparation_projection_serializes_only_scene() {
    let (snapshot, store, owner) = prepared_projection("valid", false);
    let first = projection_capture(&snapshot, 0);
    let second = first.clone();
    let (CapturedIr::PreparationProjection(a), CapturedIr::PreparationProjection(b)) =
        (&first.ir, &second.ir)
    else {
        panic!("captures must retain the shared typed snapshot");
    };
    assert!(Arc::ptr_eq(a, b));
    assert_eq!(first.ir.schema_version_string(), "1.0.0");
    // Independent AC-1 literal, not serialized from the fixture's scene. Exact
    // equality rejects ANY private field, including future unanticipated ones.
    let expected = serde_json::json!({
        "kind": "PreparationProjection", "value": {
            "scene_id": "final", "view": "probe_xy", "classes": ["anchors", "domains", "paths"],
            "primitives": [
                {"id":"anchor_a", "global_layer_index":0, "class":"anchors", "label":"anchor",
                 "geometry":{"Point":{"x":10000,"y":20000}}},
                {"id":"path_a", "global_layer_index":0, "class":"paths", "label":"route",
                 "geometry":{"Polyline":[{"x":0,"y":0},{"x":20000,"y":0}]}},
                {"id":"domain_a", "global_layer_index":0, "class":"domains", "label":"area",
                 "geometry":{"Polygon":{"filled":true,"polygon":{
                     "contour":{"points":[{"x":0,"y":0},{"x":40000,"y":0},{"x":40000,"y":40000},{"x":0,"y":40000}]},
                     "holes":[{"points":[{"x":10000,"y":10000},{"x":10000,"y":30000},{"x":30000,"y":30000},{"x":30000,"y":10000}]}]
                 }}}},
                {"id":"outline_a", "global_layer_index":0, "class":"domains", "label":"outline",
                 "geometry":{"Polygon":{"filled":false,"polygon":{
                     "contour":{"points":[{"x":60000,"y":0},{"x":80000,"y":0},{"x":80000,"y":20000},{"x":60000,"y":20000}]},
                     "holes":[]
                 }}}}
            ]
        }
    });
    let actual = serde_json::to_value(&first.ir).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(serde_json::to_value(&second.ir).unwrap(), expected);
    // test-quality: pinned DTO wire-format keys, not a hand-maintained implementation roster.
    assert_eq!(
        actual["value"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["classes", "primitives", "scene_id", "view"]
    );
    store.dispose_owner(&owner).unwrap();
    assert_eq!(
        store.diagnostic_accounting(&owner).unwrap().retained_bytes,
        0
    );
    assert_eq!(
        serde_json::to_value(&first.ir).unwrap(),
        expected,
        "public diagnostics survive private-store disposal"
    );
    // The serialized public DTO itself is usable independently of the host.
    let scene: slicer_sdk::preparation::PreparationProjectionScene =
        serde_json::from_value(actual["value"].clone()).unwrap();
    assert_eq!(
        scene.primitives[0].geometry,
        slicer_sdk::preparation::PreparationProjectionGeometry::Point(Point2 {
            x: 10000,
            y: 20000
        })
    );

    let (empty, empty_store, empty_owner) = prepared_projection("empty", false);
    empty_store.dispose_owner(&empty_owner).unwrap();
    let empty_json = serde_json::to_value(CapturedIr::PreparationProjection(empty)).unwrap();
    assert_eq!(
        empty_json,
        serde_json::json!({"kind":"PreparationProjection", "value":{
            "scene_id":"final", "view":"probe_xy", "classes":["anchors","domains","paths"], "primitives":[]
        }})
    );
    // test-quality: pinned DTO wire-format keys for explicit empty completion.
    assert_eq!(
        empty_json["value"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["classes", "primitives", "scene_id", "view"]
    );
}

#[test]
fn preparation_projection_renders_typed_geometry_and_refuses_ordinary_styles() {
    use slicer_runtime::visual_debug_render::{
        render_silhouette_composite, render_stage_capture_styled, RenderError, RenderStyle,
        SilhouetteSlabSchedule, SilhouetteView,
    };
    use slicer_runtime::visual_debug_style::{ColorBy, OverlayKind};
    let (snapshot, store, _) = prepared_projection("valid", false);
    let capture = projection_capture(&snapshot, 0);
    store.dispose().unwrap();
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));
    assert_eq!(
        bounds,
        ViewportBoundsMm {
            min_x: -2.0,
            min_y: -2.0,
            max_x: 10.0,
            max_y: 6.0
        }
    );
    let (image, events) = render_stage_capture_styled(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
        None,
        &RenderStyle::default(),
    )
    .unwrap();
    assert!(
        events.is_empty(),
        "diagnostic geometry must not invent extrusion events"
    );
    let (width, height, rgb) = decode_rgb(&image.png_bytes);
    assert_eq!((width, height), (1024, 1024));
    // Independent mm/Projector literals: scale 1024/12 with vertical letterbox.
    // Rank anchors/domains/paths selects literal blue/orange/green RGBs.
    assert_eq!(
        pixel_at(&rgb, width, 256, 512),
        [31, 119, 180],
        "point at (1,2), painted above fill"
    );
    assert_eq!(
        pixel_at(&rgb, width, 259, 515),
        BACKGROUND,
        "circle corner is not a square glyph"
    );
    assert_eq!(
        pixel_at(&rgb, width, 213, 512),
        [255, 127, 14],
        "filled shell (0.5,2)"
    );
    assert_eq!(pixel_at(&rgb, width, 341, 512), BACKGROUND, "hole (2,2)");
    assert_eq!(
        pixel_at(&rgb, width, 171, 683),
        [44, 160, 44],
        "open-line start (0,0)"
    );
    assert_eq!(
        pixel_at(&rgb, width, 341, 683),
        [44, 160, 44],
        "open-line end (2,0)"
    );
    assert_eq!(
        pixel_at(&rgb, width, 360, 683),
        BACKGROUND,
        "open line must not extend past endpoint"
    );
    assert_eq!(
        pixel_at(&rgb, width, 683, 597),
        [255, 127, 14],
        "outline closing edge (6,1)"
    );
    assert_eq!(
        pixel_at(&rgb, width, 768, 597),
        BACKGROUND,
        "outline interior (7,1) is unfilled"
    );
    assert_eq!(
        pixel_at(&rgb, width, 265, 500),
        [31, 119, 180],
        "label A top row is painted last"
    );
    let alternate = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .unwrap();
    // test-quality: determinism/dispatch pin — GeometryView cannot reinterpret diagnostic shapes as extrusion.
    assert_eq!(alternate.png_bytes, image.png_bytes);
    let enlarged = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        2,
        bounds,
    )
    .unwrap();
    let (large_width, _, large_rgb) = decode_rgb(&enlarged.png_bytes);
    assert_eq!(
        pixel_at(&large_rgb, large_width, 519, 1024),
        [31, 119, 180],
        "point radius scales in screen pixels"
    );
    assert_eq!(
        pixel_at(&large_rgb, large_width, 521, 1024),
        BACKGROUND,
        "scaled circle remains bounded"
    );

    let (empty, empty_store, _) = prepared_projection("empty", false);
    empty_store.dispose().unwrap();
    for capture in [&capture, &projection_capture(&empty, 0)] {
        for view in [
            RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
            RenderView::OverlayIsolated(GeometryView::FilamentLines, OverlayKind::Seams),
        ] {
            assert!(matches!(
                render_stage_capture_styled(
                    capture,
                    view,
                    1,
                    bounds,
                    None,
                    &RenderStyle::default()
                ),
                Err(RenderError::UnsupportedPreparationOption {
                    option: "ordinary overlays",
                    ..
                })
            ));
        }
        let tool_style = RenderStyle {
            color_by: ColorBy::Tool,
            ..Default::default()
        };
        assert!(matches!(
            render_stage_capture_styled(
                capture,
                RenderView::Geometry(GeometryView::FilledAreas),
                1,
                bounds,
                None,
                &tool_style
            ),
            Err(RenderError::UnsupportedPreparationOption {
                option: "tool/extrusion styles",
                ..
            })
        ));
        for view in [SilhouetteView::Front, SilhouetteView::Side] {
            assert!(
                matches!(render_silhouette_composite(std::slice::from_ref(capture), view, 1, bounds,
                &SilhouetteSlabSchedule::default()), Err(RenderError::UnsupportedPreparationOption { option, .. }) if option == view.name())
            );
        }
    }
    let empty_image = render_stage_capture(
        &projection_capture(&empty, 0),
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .unwrap();
    let (_, _, empty_rgb) = decode_rgb(&empty_image.png_bytes);
    assert!(empty_rgb
        .as_chunks::<3>()
        .0
        .iter()
        .all(|pixel| *pixel == BACKGROUND));
}

#[test]
fn preparation_projection_full_scene_bounds_and_class_ranks_survive_layer_subset() {
    let (snapshot, store, _) = prepared_projection("valid", true);
    let selected = projection_capture(&snapshot, 0);
    let remote = projection_capture(&snapshot, 1);
    store.dispose().unwrap();
    let subset = compute_viewport_bounds(std::slice::from_ref(&selected));
    let all = compute_viewport_bounds(&[selected.clone(), remote.clone()]);
    assert_eq!(
        subset,
        ViewportBoundsMm {
            min_x: -2.0,
            min_y: -2.0,
            max_x: 202.0,
            max_y: 152.0
        }
    );
    assert_eq!(all, subset);
    // Explicit plate bounds are passed through without a scene-derived margin.
    let plate = ViewportBoundsMm {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 256.0,
        max_y: 256.0,
    };
    let remote_image = render_stage_capture(
        &remote,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        plate,
    )
    .unwrap();
    let (width, _, rgb) = decode_rgb(&remote_image.png_bytes);
    assert_eq!(
        pixel_at(&rgb, width, 800, 424),
        [214, 39, 40],
        "remote class is full-scene rank 3, not rendered-layer rank 0"
    );
    assert_eq!(
        pixel_at(&rgb, width, 4, 1016),
        BACKGROUND,
        "unselected layer point must not leak into remote image"
    );
    let ordinary = StageCapture {
        stage_id: "Layer::Slice".into(),
        layer_index: 0,
        layer_z: 0.2,
        ir: CapturedIr::Slice(SliceIR {
            regions: vec![SlicedRegion {
                polygons: vec![triangle_mm(-10.0, -20.0, 0.0, 0.0, 10.0, 0.0)],
                ..Default::default()
            }],
            ..Default::default()
        }),
    };
    assert_eq!(
        compute_viewport_bounds(&[selected, ordinary]),
        ViewportBoundsMm {
            min_x: -12.0,
            min_y: -22.0,
            max_x: 202.0,
            max_y: 152.0
        }
    );
}
