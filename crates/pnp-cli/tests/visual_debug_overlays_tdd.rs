//! Schema 1.1.0 visual-debug overlays + tool coloring — request validation,
//! isolated per-overlay rendering on the standalone G-code source, manifest
//! `overlay_events` mirroring, `tool_palette`, and determinism.
//!
//! Uses the gcode source throughout: it exercises the full request → bundle
//! → manifest pipeline without model/module loading, so the suite stays
//! fast. The typed-IR overlay path shares the same shared-style glyph/event
//! code (`slicer_runtime::visual_debug_style`) unit-tested in
//! `slicer-runtime`.

use std::fs;
use std::path::PathBuf;

use pnp_cli::visual_debug::{run_visual_debug, ValidationError, VisualDebugError};

/// Two tools, retractions (inline-E and firmware), a z-hop, and travels
/// across one layer.
const MULTI_TOOL_GCODE: &str = "\
;LAYER_CHANGE
;Z:0.2
G1 Z0.2 F600
;TYPE:Outer wall
G1 X0 Y0 F3000
G1 X10 Y0 E1.0 F1200
G1 X10 Y10 E2.0
G1 E1.2 F1800
G0 X20 Y20 F9000
G1 E2.0 F1800
T1
G1 X30 Y20 E3.0
G1 Z0.6 F600
G1 Z0.2 F600
G10
G0 X0 Y0 F9000
G11
G1 X5 Y0 E4.0
; printable_area = 0x0,220x0,220x200,0x200
";

fn write_fixture(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    fs::write(&path, MULTI_TOOL_GCODE).expect("write gcode fixture");
    (dir, path)
}

fn request_json(
    gcode_path: &std::path::Path,
    schema: &str,
    visualizations: &str,
) -> serde_json::Value {
    serde_json::from_str(&format!(
        r#"{{
            "schema_version": "{schema}",
            "source": {{ "kind": "gcode", "path": {path:?} }},
            "layers": [0],
            "taps": [],
            "visualizations": {visualizations}
        }}"#,
        path = gcode_path.to_string_lossy(),
    ))
    .expect("request json parses")
}

fn run(request: serde_json::Value, out: &std::path::Path) -> Result<PathBuf, VisualDebugError> {
    let req = serde_json::from_value(request).expect("request deserializes");
    run_visual_debug(req, out, true)
}

// ───────────────────────── validation gating ──────────────────────────────

#[test]
fn v1_0_requests_reject_the_new_options_rather_than_ignoring_them() {
    let (_dir, gcode) = write_fixture("gate.gcode");
    for options in [
        r#"{"color_by": "tool"}"#,
        r#"{"color_by": "tool", "tool_color_source": "filament"}"#,
        r#"{"overlays": ["travel"]}"#,
    ] {
        let viz = format!(r#"[{{"type": "filament_lines", "options": {options}}}]"#);
        let out = tempfile::tempdir().expect("tempdir");
        let err = run(request_json(&gcode, "1.0.0", &viz), out.path())
            .expect_err("a 1.0.0 request with 1.1-only options must fail closed");
        assert!(
            matches!(
                err,
                VisualDebugError::Validation(ValidationError::OptionRequiresSchema11 { .. })
            ),
            "options {options}: expected OptionRequiresSchema11, got {err:?}"
        );
    }
}

#[test]
fn unknown_overlay_names_and_bad_color_by_fail_closed() {
    let (_dir, gcode) = write_fixture("badopts.gcode");
    let cases: [(&str, fn(&VisualDebugError) -> bool); 3] = [
        (
            r#"[{"type": "diagnostic_overlay", "options": {"overlays": ["wipe"]}}]"#,
            |e| {
                matches!(
                    e,
                    VisualDebugError::Validation(ValidationError::InvalidOverlays { .. })
                )
            },
        ),
        (
            r#"[{"type": "filament_lines", "options": {"color_by": "extruder"}}]"#,
            |e| {
                matches!(
                    e,
                    VisualDebugError::Validation(ValidationError::InvalidColorBy { .. })
                )
            },
        ),
        (
            r#"[{"type": "diagnostic_overlay", "options": {"overlays": ["seams"]}}]"#,
            |e| {
                matches!(
                    e,
                    VisualDebugError::Validation(ValidationError::OverlayUnsupportedOnGcode { .. })
                )
            },
        ),
    ];
    for (viz, matches_expected) in cases {
        let out = tempfile::tempdir().expect("tempdir");
        let err = run(request_json(&gcode, "1.1.0", viz), out.path())
            .expect_err(&format!("{viz} must be rejected"));
        assert!(matches_expected(&err), "{viz}: got {err:?}");
    }
}

// ───────────────────── overlays + tool coloring bundle ────────────────────

fn overlay_bundle_request(gcode: &std::path::Path) -> serde_json::Value {
    request_json(
        gcode,
        "1.1.0",
        r#"[
            {"type": "filament_lines", "options": {"color_by": "tool"}},
            {"type": "diagnostic_overlay",
             "options": {"overlays": ["travel", "retractions", "z_hops", "tool_changes"]}}
        ]"#,
    )
}

#[test]
fn gcode_overlay_bundle_mirrors_events_into_the_manifest() {
    let (_dir, gcode) = write_fixture("bundle.gcode");
    let out = tempfile::tempdir().expect("tempdir");
    let manifest_path =
        run(overlay_bundle_request(&gcode), out.path()).expect("1.1.0 overlay bundle succeeds");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("read manifest"))
            .expect("manifest parses");

    assert_eq!(manifest["schema_version"], "1.1.0");
    assert_eq!(manifest["legend_version"], "1.1.0");
    assert!(
        manifest["tool_palette"].is_array(),
        "color_by tool must emit the tool_palette table"
    );

    let images = manifest["images"].as_array().expect("images array");
    // 1 tool-colored filament_lines + 4 isolated overlays.
    assert_eq!(images.len(), 5, "one image per view: {images:#?}");

    // Every overlay image carries its events; every PNG exists on disk.
    let mut overlays_seen = Vec::new();
    for image in images {
        let png = out
            .path()
            .join(image["png_path"].as_str().expect("png_path"));
        assert!(png.is_file(), "missing PNG {png:?}");
        if let Some(overlay) = image["overlay"].as_str() {
            overlays_seen.push(overlay.to_string());
            assert!(
                image["overlay_events"].is_array(),
                "overlay '{overlay}' must mirror its events"
            );
        }
    }
    overlays_seen.sort();
    assert_eq!(
        overlays_seen,
        ["retractions", "tool_changes", "travel", "z_hops"]
    );

    let events_for = |name: &str| -> Vec<serde_json::Value> {
        images
            .iter()
            .find(|i| i["overlay"] == name)
            .expect("overlay image present")["overlay_events"]
            .as_array()
            .expect("events array")
            .clone()
    };

    // Retractions: inline-E retract at (10,10) + firmware G10, and the two
    // matching unretractions.
    let retractions = events_for("retractions");
    let kinds: Vec<&str> = retractions
        .iter()
        .map(|e| e["event"].as_str().expect("event tag"))
        .collect();
    assert_eq!(
        kinds,
        ["retraction", "unretraction", "retraction", "unretraction"],
        "retraction stream order must follow source order"
    );
    assert!((retractions[0]["x"].as_f64().expect("x") - 10.0).abs() < 1e-6);
    assert!((retractions[0]["length_mm"].as_f64().expect("len") - 0.8).abs() < 1e-6);

    // Z-hop: the Z0.2 -> Z0.6 lift, height 0.4.
    let z_hops = events_for("z_hops");
    assert_eq!(z_hops.len(), 1, "one z-hop: {z_hops:#?}");
    assert!((z_hops[0]["height_mm"].as_f64().expect("h") - 0.4).abs() < 1e-4);

    // Tool change: T0 -> T1 at (20,20).
    let tool_changes = events_for("tool_changes");
    assert_eq!(tool_changes.len(), 1);
    assert_eq!(tool_changes[0]["from_tool"], 0);
    assert_eq!(tool_changes[0]["to_tool"], 1);
    assert!((tool_changes[0]["x"].as_f64().expect("x") - 20.0).abs() < 1e-6);

    // Travel: the G0 moves produce polylines with real lengths.
    let travels = events_for("travel");
    assert!(!travels.is_empty(), "travel overlay must carry polylines");
    for t in &travels {
        assert!(t["points"].as_array().expect("points").len() >= 2);
        assert!(t["length_mm"].as_f64().expect("len") > 0.0);
    }

    // The tool-colored geometry entry records its color mode.
    let tool_colored = images
        .iter()
        .find(|i| i["visualization"] == "filament_lines")
        .expect("filament_lines entry");
    assert_eq!(tool_colored["color_by"], "tool");
    assert_eq!(tool_colored["tool_color_source"], "palette");
}

// ───────────────── typed-IR path: events + tool coloring ──────────────────

mod typed_ir {
    use slicer_ir::{
        ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey,
        ToolChange, TravelMove, TravelRetract, ZHop,
    };
    use slicer_runtime::layer_executor::{CapturedIr, StageCapture};
    use slicer_runtime::{
        collect_overlay_events, render_stage_capture_styled, ColorBy, GeometryView, OverlayEvent,
        OverlayKind, RenderError, RenderStyle, RenderView, ToolColors, ViewportBoundsMm,
    };

    fn entity(entity_id: u64, tool_index: u32, from: (f32, f32), to: (f32, f32)) -> PrintEntity {
        let point = |x, y| Point3WithWidth {
            x,
            y,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        };
        PrintEntity {
            entity_id,
            path: ExtrusionPath3D {
                points: vec![point(from.0, from.1), point(to.0, to.1)],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            role: ExtrusionRole::OuterWall,
            region_key: RegionKey::default(),
            topo_order: 0,
            tool_index,
        }
    }

    /// Two entities on different tools; a retract, unretract, z-hop, tool
    /// change, and travel all anchored at real entities.
    fn capture() -> StageCapture {
        let ir = LayerCollectionIR {
            ordered_entities: vec![
                entity(1, 0, (0.0, 0.0), (10.0, 0.0)),
                entity(2, 1, (10.0, 10.0), (0.0, 10.0)),
            ],
            tool_changes: vec![ToolChange {
                after_entity_index: 0,
                from_tool: 0,
                to_tool: 1,
            }],
            z_hops: vec![ZHop {
                after_entity_index: 0,
                hop_height: 0.4,
            }],
            retracts: vec![
                TravelRetract {
                    after_entity_index: 0,
                    length: 0.8,
                    speed: 30.0,
                    is_unretract: false,
                    ..Default::default()
                },
                TravelRetract {
                    after_entity_index: 1,
                    length: 0.8,
                    speed: 30.0,
                    is_unretract: true,
                    ..Default::default()
                },
            ],
            travel_moves: vec![TravelMove {
                entity_id: 1,
                x: Some(10.0),
                y: Some(10.0),
                z: Some(0.2),
                f: None,
            }],
            ..Default::default()
        };
        StageCapture {
            stage_id: "Layer::PathOptimization".to_string(),
            layer_index: 0,
            layer_z: 0.2,
            ir: CapturedIr::LayerCollection(ir),
        }
    }

    const BOUNDS: ViewportBoundsMm = ViewportBoundsMm {
        min_x: -2.0,
        min_y: -2.0,
        max_x: 12.0,
        max_y: 12.0,
    };

    #[test]
    fn layer_collection_events_are_anchored_at_their_entities() {
        let c = capture();
        let retracts = collect_overlay_events(&c.ir, OverlayKind::Retractions, &c.stage_id, 0)
            .expect("retractions supported on LayerCollection");
        assert_eq!(retracts.len(), 2);
        assert!(matches!(
            &retracts[0],
            // Anchored at entity 0's last point (10, 0).
            OverlayEvent::Retraction { x, y, length_mm }
                if *x == 10.0 && *y == 0.0 && (*length_mm - 0.8).abs() < 1e-6
        ));
        assert!(matches!(&retracts[1], OverlayEvent::Unretraction { .. }));

        let hops = collect_overlay_events(&c.ir, OverlayKind::ZHops, &c.stage_id, 0)
            .expect("z_hops supported");
        assert!(
            matches!(&hops[0], OverlayEvent::ZHop { height_mm, .. } if (*height_mm - 0.4).abs() < 1e-6)
        );

        let tools = collect_overlay_events(&c.ir, OverlayKind::ToolChanges, &c.stage_id, 0)
            .expect("tool_changes supported");
        assert!(matches!(
            &tools[0],
            OverlayEvent::ToolChange {
                from_tool: Some(0),
                to_tool: 1,
                ..
            }
        ));

        let travels = collect_overlay_events(&c.ir, OverlayKind::Travel, &c.stage_id, 0)
            .expect("travel supported");
        assert!(matches!(
            &travels[0],
            // From entity 1's last point (10, 0) to the destination (10, 10).
            OverlayEvent::Travel { points, length_mm }
                if points.as_slice() == [[10.0, 0.0], [10.0, 10.0]] && (*length_mm - 10.0).abs() < 1e-6
        ));
    }

    #[test]
    fn unsupported_overlay_tap_pairings_fail_closed() {
        let c = capture();
        // Seams have no source field on a LayerCollection capture.
        let err = collect_overlay_events(&c.ir, OverlayKind::Seams, &c.stage_id, 0)
            .expect_err("seams must be unsupported on LayerCollection");
        assert!(matches!(err, RenderError::OverlayUnsupportedForTap { .. }));
    }

    #[test]
    fn isolated_overlay_render_returns_the_drawn_events_and_differs_from_base() {
        let c = capture();
        let base = render_stage_capture_styled(
            &c,
            RenderView::Geometry(GeometryView::FilamentLines),
            1,
            BOUNDS,
            None,
            &RenderStyle::default(),
        )
        .expect("base render succeeds");
        let (overlay, events) = render_stage_capture_styled(
            &c,
            RenderView::OverlayIsolated(GeometryView::FilamentLines, OverlayKind::Retractions),
            1,
            BOUNDS,
            None,
            &RenderStyle::default(),
        )
        .expect("isolated overlay render succeeds");
        assert_eq!(events.len(), 2, "returned events mirror the drawn glyphs");
        assert_ne!(
            base.0.png_bytes, overlay.png_bytes,
            "overlay image must differ from the plain geometry render"
        );
    }

    #[test]
    fn tool_coloring_changes_pixels_and_is_rejected_where_no_tool_exists() {
        let c = capture();
        let role = render_stage_capture_styled(
            &c,
            RenderView::Geometry(GeometryView::FilamentLines),
            1,
            BOUNDS,
            None,
            &RenderStyle::default(),
        )
        .expect("role render succeeds");
        let tool = render_stage_capture_styled(
            &c,
            RenderView::Geometry(GeometryView::FilamentLines),
            1,
            BOUNDS,
            None,
            &RenderStyle {
                color_by: ColorBy::Tool,
                tool_colors: ToolColors::default(),
            },
        )
        .expect("tool render succeeds");
        assert_ne!(
            role.0.png_bytes, tool.0.png_bytes,
            "two entities on different tools must recolor under color_by tool"
        );

        // A Perimeter capture carries no tool assignment: fail closed.
        let perimeter = StageCapture {
            stage_id: "Layer::Perimeters".to_string(),
            layer_index: 0,
            layer_z: 0.2,
            ir: CapturedIr::Perimeter(slicer_ir::PerimeterIR::default()),
        };
        let err = render_stage_capture_styled(
            &perimeter,
            RenderView::Geometry(GeometryView::FilamentLines),
            1,
            BOUNDS,
            None,
            &RenderStyle {
                color_by: ColorBy::Tool,
                tool_colors: ToolColors::default(),
            },
        )
        .expect_err("color_by tool must be rejected on a tool-less capture");
        assert!(matches!(err, RenderError::ToolColorUnavailable { .. }));
    }
}

#[test]
fn overlay_bundles_are_deterministic_across_runs() {
    let (_dir, gcode) = write_fixture("det.gcode");
    let out_a = tempfile::tempdir().expect("tempdir");
    let out_b = tempfile::tempdir().expect("tempdir");
    let manifest_a = run(overlay_bundle_request(&gcode), out_a.path()).expect("run A succeeds");
    let manifest_b = run(overlay_bundle_request(&gcode), out_b.path()).expect("run B succeeds");
    assert_eq!(
        fs::read_to_string(manifest_a).expect("read A"),
        fs::read_to_string(manifest_b).expect("read B"),
        "manifests must be byte-identical"
    );
    for entry in fs::read_dir(out_a.path().join("images")).expect("images dir") {
        let path_a = entry.expect("entry").path();
        let path_b = out_b
            .path()
            .join("images")
            .join(path_a.file_name().expect("file name"));
        assert_eq!(
            fs::read(&path_a).expect("read A png"),
            fs::read(&path_b).expect("read B png"),
            "PNG {path_a:?} must be byte-identical across runs"
        );
    }
}
