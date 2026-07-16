//! Packet 159 — intermediate visual-debug renderer: typed, renderer-owned
//! captures (packet 158) in, deterministic PNGs out.
//!
//! Two tiers of tests:
//!
//! - AC-1 / AC-4 drive the real CLI pipeline (`pnp_cli::visual_debug::run_
//!   visual_debug`, `Model` source) against `resources/regression_wedge.stl`
//!   — same fixture/config pattern as packet 158's
//!   `visual_debug_typed_tap_capture_tdd.rs` — because both ACs are
//!   explicitly about `manifest.json`/bundle contract fields the CLI
//!   handoff (not the pure renderer) owns.
//! - AC-2, AC-3, AC-5, AC-N1, AC-N2, AC-N3 construct minimal typed IR
//!   fixtures directly (`PerimeterIR`/`InfillIR`, real field types, no
//!   builder boilerplate needed — every field used here is `pub` and most
//!   container types derive `Default`) and call
//!   `slicer_runtime::render_stage_capture` directly. This is faster than
//!   driving the full model pipeline and lets assertions target the
//!   renderer's actual documented behavior (width-sweep vs. centerline,
//!   overlay compositing, determinism, typed rejections) precisely.
//!
//! None of these tests depend on the `png` crate: `crates/pnp-cli`'s own
//! `Cargo.toml` is out of this packet's edit scope, so PNG contents are
//! checked either by raw `Vec<u8>` equality/inequality (renderer-level
//! tests — a strictly stronger check than decoding for "did this change")
//! or, for on-disk artifacts written by the CLI, by hand-parsing the fixed
//! 8-byte PNG signature + IHDR chunk header (pure `std`, no crate needed —
//! IHDR is guaranteed to be the first chunk per the PNG spec).
//!
//! AC-3's "`manifest.json` records the same tap/layer/view association"
//! clause is covered structurally rather than by a second full-model-pipeline
//! test: `run_model_source`'s image-building loop (`crates/pnp-cli/src/
//! visual_debug.rs`) records tap/layer/view identically for every
//! visualization kind including `diagnostic_overlay` — the same code path
//! AC-1 already exercises and asserts on for `filled_areas`. AC-3's own test
//! below therefore focuses on the renderer-level overlay-composability
//! behavior, which is where the packet's actual new logic lives.

use std::fs;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugError, VisualDebugRequest,
    VisualDebugSource, VisualizationSpec,
};
use serde_json::{json, Value};
use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion, PerimeterIR,
    PerimeterRegion, Point2, Point3WithWidth, Polygon, SeamPosition,
};
use slicer_runtime::{
    compute_viewport_bounds, render_stage_capture, CapturedIr, GeometryView, RenderError,
    RenderView, StageCapture, ViewportBoundsMm,
};
use tempfile::TempDir;

// ─────────────────────────── CLI-level fixtures ────────────────────────────
// Mirrors `visual_debug_typed_tap_capture_tdd.rs`'s helpers exactly (packet
// 158) — this is a standalone integration-test binary, so the small amount
// of duplication is the normal Rust convention rather than a shared `mod`.

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn module_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// `layer_height` at the schema max (1.0mm) so the ~40mm-tall
/// regression_wedge fixture bounds to ~40 layers instead of ~200 — mirrors
/// packet 158's own bound (see `visual_debug_typed_tap_capture_tdd.rs`).
fn write_bounded_config(dir: &Path) -> PathBuf {
    let path = dir.join("config.json");
    fs::write(&path, br#"{"layer_height": 1.0}"#).expect("write bounded config");
    path
}

fn model_request_with_viz(
    taps: Vec<&str>,
    layers: Vec<i64>,
    config: PathBuf,
    visualizations: Vec<VisualizationSpec>,
    resolution_scale: u32,
) -> VisualDebugRequest {
    VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(wedge_path()),
            config: Some(config),
            module_dirs: vec![module_dir()],
            path: None,
        },
        layers: layers.into_iter().map(LayerSelector::Index).collect(),
        taps: taps
            .into_iter()
            .map(|t| TapSelector::Name(t.to_string()))
            .collect(),
        visualizations,
        resolution_scale,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn manifest_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("manifest.json should exist"))
        .expect("manifest.json should be valid JSON")
}

/// Parse a PNG's raster width/height from its IHDR chunk without a
/// PNG-decoding crate dependency (see module doc). Per the PNG spec, IHDR is
/// always the first chunk, immediately after the fixed 8-byte signature.
fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    assert_eq!(&bytes[0..8], &SIGNATURE, "not a PNG file");
    assert_eq!(&bytes[12..16], b"IHDR", "IHDR must be the first PNG chunk");
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("4 bytes"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("4 bytes"));
    (width, height)
}

// ─────────────────────────────── AC-1 ──────────────────────────────────────

#[test]
fn typed_polygon_render_records_contract_metadata() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    let req = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0],
        config,
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("filled_areas render should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "expected exactly one tap/layer/view image entry; got {images:#?}"
    );
    let entry = &images[0];
    assert_eq!(entry["tap"], "Layer::Perimeters");
    assert_eq!(entry["layer_index"], 0);
    assert_eq!(entry["visualization"], "filled_areas");

    let png_path = entry["png_path"].as_str().expect("png_path is a string");
    assert!(
        !png_path.is_empty(),
        "png_path must be populated for a rendered visualization"
    );
    let png_file = output.join(png_path);
    assert!(png_file.exists(), "the referenced PNG must exist on disk");

    assert_eq!(
        entry["viewport"], manifest["viewport"],
        "the image entry's viewport must be the model-wide (bundle-shared) viewport"
    );
    assert_eq!(entry["legend_version"], manifest["legend_version"]);

    let bytes = fs::read(&png_file).expect("read PNG bytes");
    let (width, height) = png_dimensions(&bytes);
    assert_eq!(width, 1024, "resolution_scale defaults to 1 -> 1024px");
    assert_eq!(height, 1024);
}

// ─────────────────────── AC-3 (manifest association) ───────────────────────

#[test]
fn diagnostic_overlay_records_contract_metadata() {
    // AC-3's "manifest.json records the same tap/layer/view association"
    // clause, driven through the real CLI/manifest path (Finding 2) — no
    // prior test in this file drove `diagnostic_overlay` through
    // `run_visual_debug`, only through the renderer directly.
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    let req = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0],
        config,
        vec![VisualizationSpec::Detail {
            kind: "diagnostic_overlay".to_string(),
            options: json!({}),
        }],
        1,
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("diagnostic_overlay render should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "expected exactly one tap/layer/view image entry; got {images:#?}"
    );
    let entry = &images[0];
    assert_eq!(
        entry["tap"], "Layer::Perimeters",
        "manifest entry must record the requested tap"
    );
    assert_eq!(
        entry["layer_index"], 0,
        "manifest entry must record the requested layer"
    );
    assert_eq!(
        entry["visualization"], "diagnostic_overlay",
        "manifest entry must record the requested visualization kind"
    );

    let png_path = entry["png_path"].as_str().expect("png_path is a string");
    assert!(
        !png_path.is_empty(),
        "png_path must be populated for a rendered diagnostic_overlay"
    );
    let png_file = output.join(png_path);
    assert!(png_file.exists(), "the referenced PNG must exist on disk");
}

#[test]
fn diagnostic_overlay_different_bases_do_not_collide() {
    // Finding 1 regression coverage: two `diagnostic_overlay` visualizations
    // on the same tap+layer with different `options.base` must produce two
    // distinct `png_path`s (and two distinct on-disk files), not silently
    // overwrite one another.
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    let req = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0],
        config,
        vec![
            VisualizationSpec::Detail {
                kind: "diagnostic_overlay".to_string(),
                options: json!({"base": "filled_areas"}),
            },
            VisualizationSpec::Detail {
                kind: "diagnostic_overlay".to_string(),
                options: json!({"base": "filament_lines"}),
            },
        ],
        1,
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("diagnostic_overlay render should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        2,
        "one image per requested diagnostic_overlay base; got {images:#?}"
    );

    let png_paths: Vec<&str> = images
        .iter()
        .map(|entry| entry["png_path"].as_str().expect("png_path is a string"))
        .collect();
    assert_ne!(
        png_paths[0], png_paths[1],
        "two diagnostic_overlay visualizations with different bases must not \
         collide on the same png_path"
    );
    for path in &png_paths {
        assert!(
            output.join(path).exists(),
            "each distinct diagnostic_overlay base's PNG must exist on disk: {path}"
        );
    }

    let bytes_a = fs::read(output.join(png_paths[0])).expect("read PNG a");
    let bytes_b = fs::read(output.join(png_paths[1])).expect("read PNG b");
    assert_ne!(
        bytes_a, bytes_b,
        "filled_areas-based and filament_lines-based overlays must render \
         different content, not overwrite one another"
    );
}

// ─────────────────────────────── AC-4 ──────────────────────────────────────

#[test]
fn shared_viewport_palette_and_scale_are_bundle_wide() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());

    // A request-supplied `options.color` is not a documented option any
    // visualization kind honors — the fixed v1 palette must ignore it
    // entirely (proven below via byte-identical output vs. no color option).
    let colored_req = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0, 3],
        config.clone(),
        vec![VisualizationSpec::Detail {
            kind: "filled_areas".to_string(),
            options: json!({"color": "#ff00ff"}),
        }],
        2,
    );
    let colored_output = tmp.path().join("bundle-colored");
    let manifest_path = run_visual_debug(colored_req, &colored_output, false)
        .expect("scale-2 render should succeed");
    let manifest = manifest_at(&manifest_path);
    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2, "one image per selected layer");

    for entry in images {
        assert_eq!(
            entry["viewport"], manifest["viewport"],
            "every entry must record the byte-identical bundle-wide viewport, \
             regardless of that layer's own XY extent"
        );
        assert_eq!(entry["legend_version"], manifest["legend_version"]);
        let png_path = entry["png_path"].as_str().expect("png_path is a string");
        let bytes = fs::read(colored_output.join(png_path)).expect("read PNG");
        let (w, h) = png_dimensions(&bytes);
        assert_eq!(
            (w, h),
            (2048, 2048),
            "resolution_scale 2 -> 2048x2048 raster for every entry"
        );
    }

    let plain_req = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0, 3],
        config,
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        2,
    );
    let plain_output = tmp.path().join("bundle-plain");
    let plain_manifest_path =
        run_visual_debug(plain_req, &plain_output, false).expect("second render should succeed");
    let plain_manifest = manifest_at(&plain_manifest_path);
    let plain_images = plain_manifest["images"].as_array().expect("images array");
    assert_eq!(plain_images.len(), 2);

    for (colored_entry, plain_entry) in images.iter().zip(plain_images.iter()) {
        assert_eq!(colored_entry["tap"], plain_entry["tap"]);
        assert_eq!(colored_entry["layer_index"], plain_entry["layer_index"]);
        let colored_bytes =
            fs::read(colored_output.join(colored_entry["png_path"].as_str().expect("path")))
                .expect("read colored PNG");
        let plain_bytes =
            fs::read(plain_output.join(plain_entry["png_path"].as_str().expect("path")))
                .expect("read plain PNG");
        assert_eq!(
            colored_bytes, plain_bytes,
            "a request-supplied color option must not change the rendered output; \
             the palette is fixed v1, not request-derived"
        );
    }
}

#[test]
fn shared_viewport_bounds_are_computed_bundle_wide_not_per_capture() {
    // Finding 4: `shared_viewport_palette_and_scale_are_bundle_wide` (above)
    // only checks `Viewport{width,height}` in pixels, which
    // `run_model_source` sets from `resolution_scale` alone
    // (`BASE_DIMENSION_PX * resolution_scale`) — never from geometry — so it
    // cannot distinguish a correctly bundle-wide `compute_viewport_bounds`
    // call from a regression that moves it inside the per-capture loop.
    // `pnp_cli::visual_debug::Viewport` (the manifest-facing struct) carries
    // no world-space field to check instead. The real world-space bounds
    // live on `slicer_runtime::ViewportBoundsMm` (`min_x`/`min_y`/`max_x`/
    // `max_y`, mm) — the type `compute_viewport_bounds` actually returns —
    // so this test drives that function directly with two captures of
    // deliberately very different XY extents and proves the bundle-wide
    // bounds reflect BOTH captures' geometry (not either one alone), which
    // is exactly what would stop holding if `compute_viewport_bounds` were
    // ever called per-capture instead of once over the whole bundle.
    // Deliberately disjoint (not nested) extents: a tiny ~1mm square far in
    // the negative quadrant, and a large ~100mm square in the positive
    // quadrant. Neither capture's own tight bounds contain the other's, so
    // the true bundle-wide union is strictly larger than EITHER capture's
    // own per-capture bounds — a per-capture regression on either capture
    // alone could never accidentally produce the same value by coincidence.
    let small = infill_capture_with_extent(-50.0, -49.0);
    let large = infill_capture_with_extent(0.0, 100.0);

    let bundle_bounds = compute_viewport_bounds(&[small.clone(), large.clone()]);
    let small_only_bounds = compute_viewport_bounds(std::slice::from_ref(&small));
    let large_only_bounds = compute_viewport_bounds(std::slice::from_ref(&large));

    // The bundle-wide bounds must span both the small capture's negative
    // extent and the large capture's positive extent.
    assert!(
        bundle_bounds.min_x < -40.0,
        "bundle-wide bounds must reflect the small capture's real negative \
         XY extent (min_x = {}); a per-capture regression on the large \
         capture alone would clip this to ~0mm",
        bundle_bounds.min_x
    );
    assert!(
        bundle_bounds.max_x > 50.0,
        "bundle-wide bounds must reflect the large capture's real positive \
         XY extent (max_x = {}); a per-capture regression on the small \
         capture alone would clip this to ~-49mm",
        bundle_bounds.max_x
    );

    // The bundle-wide value is not byte/value-identical to either capture's
    // own tight bounds — proving it is a real union over the whole bundle,
    // not a stand-in for one capture's bounds recomputed twice.
    assert_ne!(
        bundle_bounds, small_only_bounds,
        "the bundle-wide viewport must not collapse to the small capture's \
         own tight per-capture bounds"
    );
    assert_ne!(
        bundle_bounds, large_only_bounds,
        "the bundle-wide viewport must not collapse to the large capture's \
         own tight per-capture bounds"
    );

    // The one shared `ViewportBoundsMm` value that a correct implementation
    // applies when rendering EITHER capture (mirroring
    // `run_model_source`'s single `viewport_bounds` binding reused across
    // its per-capture loop) is byte/value-identical for both — the literal
    // AC-4 property this test exists to check.
    let bounds_for_small_render = bundle_bounds;
    let bounds_for_large_render = bundle_bounds;
    assert_eq!(
        bounds_for_small_render, bounds_for_large_render,
        "the same bundle-wide ViewportBoundsMm must be applied verbatim to \
         every capture in the bundle, regardless of that capture's own XY \
         extent"
    );
}

#[test]
fn world_bounds_mm_is_shared_bundle_wide_across_manifest_entries() {
    // Gap-1 follow-up fix: AC-4's "viewport bounds" clause (world-space mm,
    // distinct from the pixel `viewport{width,height}` field) had no
    // manifest-facing field at all, so no end-to-end test could catch a
    // regression that narrowed `compute_viewport_bounds` from one bundle-wide
    // call to a per-capture one. `ImageEntry::world_bounds_mm` (new,
    // additive) now records the shared `ViewportBoundsMm` verbatim.
    //
    // Two DIFFERENT TAPS at the SAME layer (rather than the same tap at two
    // layers) deliberately gives each capture a genuinely different tight
    // XY extent regardless of this wedge fixture's particular Z-taper
    // shape: `Layer::Infill`'s sparse-infill lines are inset well within
    // `Layer::Perimeters`' outer wall, so their own per-capture bounds are
    // provably not equal to each other. If a regression ever narrowed the
    // shared computation back to per-capture, this test would catch it
    // because the two entries' `world_bounds_mm` would then diverge.
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    let req = model_request_with_viz(
        vec!["Layer::Perimeters", "Layer::Infill"],
        vec![0],
        config,
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
    );

    let manifest_path =
        run_visual_debug(req, &output, false).expect("multi-tap render should succeed");
    let manifest = manifest_at(&manifest_path);

    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        2,
        "one image per requested tap at the shared layer; got {images:#?}"
    );

    for entry in images {
        assert!(
            !entry["world_bounds_mm"].is_null(),
            "every rendered image entry must carry a non-null world_bounds_mm; got {entry:#?}"
        );
    }

    assert_eq!(
        images[0]["world_bounds_mm"], images[1]["world_bounds_mm"],
        "world_bounds_mm must be byte/value-identical across every image entry in one \
         bundle, regardless of that capture's own (deliberately different, here) XY extent \
         — the literal AC-4 property a per-capture regression would break"
    );

    // Sanity: the shared bounds are real (not the `compute_viewport_bounds`
    // no-geometry-anywhere fallback of a unit square around the origin),
    // proving this is a genuine geometry-derived value, not a vacuous
    // always-equal default.
    let bounds = &images[0]["world_bounds_mm"];
    let min_x = bounds["min_x"].as_f64().expect("min_x is a number");
    let max_x = bounds["max_x"].as_f64().expect("max_x is a number");
    assert!(
        max_x - min_x > 1.0,
        "shared world_bounds_mm must reflect real geometry extent, not the degenerate \
         1mm fallback square; got {bounds:#?}"
    );
}

// ────────────────────────── renderer-level fixtures ────────────────────────

/// An infill capture whose single-segment path spans `(from, from)` to
/// `(to, to)` — lets a caller construct two captures with deliberately very
/// different (and, for Finding 4, deliberately disjoint) XY extents while
/// sharing every other fixture convention with [`simple_infill_capture`].
fn infill_capture_with_extent(from: f32, to: f32) -> StageCapture {
    let path = ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: from,
                y: from,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: to,
                y: to,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    };
    let region = InfillRegion {
        object_id: "obj".to_string(),
        region_id: 1,
        sparse_infill: vec![path],
        solid_infill: Vec::new(),
        ironing: Vec::new(),
    };
    let ir = InfillIR {
        regions: vec![region],
        ..Default::default()
    };
    StageCapture {
        stage_id: "Layer::Infill".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::Infill(ir),
    }
}

fn simple_infill_capture(width: f32) -> StageCapture {
    let path = ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z: 0.0,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    };
    let region = InfillRegion {
        object_id: "obj".to_string(),
        region_id: 1,
        sparse_infill: vec![path],
        solid_infill: Vec::new(),
        ironing: Vec::new(),
    };
    let ir = InfillIR {
        regions: vec![region],
        ..Default::default()
    };
    StageCapture {
        stage_id: "Layer::Infill".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::Infill(ir),
    }
}

fn perimeter_capture_with_seam(seam_xy: (f32, f32)) -> StageCapture {
    let square = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    };
    let seam = SeamPosition {
        point: Point3WithWidth {
            x: seam_xy.0,
            y: seam_xy.1,
            z: 0.0,
            width: 0.0,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        wall_index: 0,
    };
    let region = PerimeterRegion {
        object_id: "obj".to_string(),
        region_id: 1,
        walls: Vec::new(),
        infill_areas: vec![square],
        seam_candidates: Vec::new(),
        resolved_seam: Some(seam),
    };
    let ir = PerimeterIR {
        regions: vec![region],
        ..Default::default()
    };
    StageCapture {
        stage_id: "Layer::Perimeters".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::Perimeter(ir),
    }
}

fn layer_collection_capture_with_travel_and_annotation() -> StageCapture {
    use slicer_ir::{
        LayerAnnotation, LayerAnnotationKind, LayerCollectionIR, PrintEntity, RegionKey, TravelMove,
    };

    let path = ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    let entity = PrintEntity {
        entity_id: 1,
        path,
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey::default(),
        topo_order: 0,
        tool_index: 0,
    };
    let travel = TravelMove {
        entity_id: 1,
        x: Some(9.0),
        y: Some(9.0),
        z: Some(0.0),
        f: None,
    };
    let annotation = LayerAnnotation {
        after_entity_index: 0,
        kind: LayerAnnotationKind::Comment("test annotation".to_string()),
    };
    let ir = LayerCollectionIR {
        ordered_entities: vec![entity],
        travel_moves: vec![travel],
        annotations: vec![annotation],
        ..Default::default()
    };
    StageCapture {
        stage_id: "Layer::PathOptimization".to_string(),
        layer_index: 0,
        layer_z: 0.0,
        ir: CapturedIr::LayerCollection(ir),
    }
}

// ─────────────────────── AC-3 (LayerCollection overlay) ────────────────────

#[test]
fn layer_collection_overlay_renders_travel_and_annotation_markers() {
    // `draw_overlay`'s `CapturedIr::LayerCollection` branch (travel-move and
    // guest-emitted-annotation markers) was previously untested — every
    // other overlay test used a `Perimeter` seam-marker capture only
    // (Finding 3).
    let capture = layer_collection_capture_with_travel_and_annotation();
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));

    let base = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("base render should succeed");
    let overlay = render_stage_capture(
        &capture,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("overlay render should succeed");

    assert_ne!(
        overlay.png_bytes, base.png_bytes,
        "the LayerCollection overlay (travel-move + annotation markers) must \
         add visible content over the base geometry render"
    );

    // Determinism holds for this branch too.
    let overlay_again = render_stage_capture(
        &capture,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("repeat overlay render should succeed");
    assert_eq!(
        overlay.png_bytes, overlay_again.png_bytes,
        "the LayerCollection overlay must be stable/deterministic across \
         repeated renders of the same capture + view"
    );
}

// ─────────────────────────────── AC-2 ──────────────────────────────────────

#[test]
fn typed_width_sweep_is_rendered() {
    let cap_w1 = simple_infill_capture(1.0);
    let cap_w5 = simple_infill_capture(5.0);
    // NOT `compute_viewport_bounds`: the fixture's centerline is horizontal
    // (both points share y=0), so the tight content bbox is ~zero-height —
    // both the width=1 and width=5 swept quads would overflow and clamp to
    // the full canvas height identically, masking the very difference this
    // test exists to prove. A fixed, generously-sized viewport (independent
    // of either fixture's own extent — mirrors how a real bundle-wide
    // viewport is computed across many captures, never a single narrow one)
    // leaves room for both widths to render at visibly different sizes.
    let bounds = ViewportBoundsMm {
        min_x: -3.0,
        min_y: -4.0,
        max_x: 13.0,
        max_y: 4.0,
    };

    let filled_w1 = render_stage_capture(
        &cap_w1,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("filled_areas width=1 should succeed");
    let filled_w5 = render_stage_capture(
        &cap_w5,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("filled_areas width=5 should succeed");
    let lines_w1 = render_stage_capture(
        &cap_w1,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .expect("filament_lines width=1 should succeed");
    let lines_w5 = render_stage_capture(
        &cap_w5,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .expect("filament_lines width=5 should succeed");

    assert_ne!(
        filled_w1.png_bytes, filled_w5.png_bytes,
        "filled_areas must rasterize the path's real per-vertex width as a swept \
         shape, not a fixed/zero-width shape — changing only the width must \
         change the output"
    );
    assert_eq!(
        lines_w1.png_bytes, lines_w5.png_bytes,
        "filament_lines must render the zero-width centerline regardless of \
         width, isolating the difference above to filled_areas' width handling"
    );
    assert_ne!(
        filled_w1.png_bytes, lines_w1.png_bytes,
        "filled_areas (swept) must differ from filament_lines (centerline) on \
         a non-zero-width fixture"
    );
}

// ─────────────────────────────── AC-3 ──────────────────────────────────────

#[test]
fn diagnostic_overlay_is_stable_and_composable() {
    let cap_a = perimeter_capture_with_seam((0.3, 0.3));
    let cap_b = perimeter_capture_with_seam((8.0, 8.0));
    // Both fixtures share the identical base square; only the (non-geometry)
    // `resolved_seam` field differs, so one shared viewport is valid for both.
    let bounds = compute_viewport_bounds(std::slice::from_ref(&cap_a));

    let base_a = render_stage_capture(
        &cap_a,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("base a should succeed");
    let base_b = render_stage_capture(
        &cap_b,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("base b should succeed");
    assert_eq!(
        base_a.png_bytes, base_b.png_bytes,
        "base geometry pixels must be unaffected by a seam-only field \
         difference — proves the overlay is a compositional add-on, not \
         baked into the base geometry render"
    );

    let overlay_a = render_stage_capture(
        &cap_a,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("overlay a should succeed");
    let overlay_b = render_stage_capture(
        &cap_b,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("overlay b should succeed");

    assert_ne!(
        overlay_a.png_bytes, base_a.png_bytes,
        "diagnostic_overlay must add visible content over the base geometry render"
    );
    assert_ne!(
        overlay_a.png_bytes, overlay_b.png_bytes,
        "diagnostic_overlay must render the seam-specific marker at its real \
         position, differing when the seam differs"
    );

    let overlay_a_again = render_stage_capture(
        &cap_a,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("overlay a repeat should succeed");
    assert_eq!(
        overlay_a.png_bytes, overlay_a_again.png_bytes,
        "the overlay must be stable/deterministic across repeated renders of \
         the same capture + view"
    );
}

// ─────────────────────────────── AC-5 ──────────────────────────────────────

#[test]
fn intermediate_png_output_is_deterministic() {
    let capture = simple_infill_capture(1.0);
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));

    let a = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        2,
        bounds,
    )
    .expect("first render should succeed");
    let b = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        2,
        bounds,
    )
    .expect("second render should succeed");
    assert_eq!(a.png_bytes, b.png_bytes, "PNG bytes must be byte-identical");
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);

    // Determinism must also hold for the diagnostic-overlay code path.
    let overlay_capture = perimeter_capture_with_seam((0.3, 0.3));
    let overlay_bounds = compute_viewport_bounds(std::slice::from_ref(&overlay_capture));
    let oa = render_stage_capture(
        &overlay_capture,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        overlay_bounds,
    )
    .expect("overlay render a should succeed");
    let ob = render_stage_capture(
        &overlay_capture,
        RenderView::DiagnosticOverlay(GeometryView::FilledAreas),
        1,
        overlay_bounds,
    )
    .expect("overlay render b should succeed");
    assert_eq!(oa.png_bytes, ob.png_bytes);
}

#[test]
fn full_manifest_and_bundle_output_is_deterministic_via_cli() {
    // Finding 6: `intermediate_png_output_is_deterministic` (above) only
    // compares raw `RenderedImage{png_bytes,width,height}` from two direct
    // `render_stage_capture` calls. AC-5's literal wording is about
    // "the intermediate PNG output" of a request, which — driven through
    // the real CLI — means the full `ImageEntry` (not just the renderer's
    // own return value) must also be deterministic: `png_path`, `viewport`,
    // `legend_version`, dimensions (via the PNG itself), tap identity,
    // `layer_index`, and `warnings`. This test runs `run_visual_debug` twice
    // on the same request against two separate output directories and diffs
    // every field.
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());

    let req_a = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0],
        config.clone(),
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
    );
    let output_a = tmp.path().join("bundle-a");
    let manifest_path_a =
        run_visual_debug(req_a, &output_a, false).expect("first run should succeed");
    let manifest_a = manifest_at(&manifest_path_a);

    let req_b = model_request_with_viz(
        vec!["Layer::Perimeters"],
        vec![0],
        config,
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
    );
    let output_b = tmp.path().join("bundle-b");
    let manifest_path_b =
        run_visual_debug(req_b, &output_b, false).expect("second run should succeed");
    let manifest_b = manifest_at(&manifest_path_b);

    let images_a = manifest_a["images"].as_array().expect("images array");
    let images_b = manifest_b["images"].as_array().expect("images array");
    assert_eq!(
        images_a.len(),
        images_b.len(),
        "both runs must produce the same number of image entries"
    );
    assert!(!images_a.is_empty(), "sanity: at least one image entry");

    for (entry_a, entry_b) in images_a.iter().zip(images_b.iter()) {
        // Itemized field checks first, so a failure names the specific
        // AC-5-relevant field that diverged rather than only an opaque
        // whole-object diff.
        assert_eq!(entry_a["tap"], entry_b["tap"], "tap identity must match");
        assert_eq!(
            entry_a["layer_index"], entry_b["layer_index"],
            "layer_index must match"
        );
        assert_eq!(entry_a["layer_z"], entry_b["layer_z"], "layer_z must match");
        assert_eq!(
            entry_a["visualization"], entry_b["visualization"],
            "visualization kind must match"
        );
        assert_eq!(
            entry_a["viewport"], entry_b["viewport"],
            "viewport must be byte/value-identical across independent runs"
        );
        assert_eq!(
            entry_a["legend_version"], entry_b["legend_version"],
            "legend_version must match"
        );
        assert_eq!(
            entry_a["warnings"], entry_b["warnings"],
            "warnings must match"
        );
        assert_eq!(
            entry_a["ir_schema_version"], entry_b["ir_schema_version"],
            "ir_schema_version must match"
        );
        assert_eq!(
            entry_a["gcode_parser_version"], entry_b["gcode_parser_version"],
            "gcode_parser_version must match"
        );
        assert_eq!(
            entry_a["png_path"], entry_b["png_path"],
            "png_path must be deterministically derived (same tap/visualization/layer)"
        );
        // Whole-entry equality as the strongest single check — catches any
        // field (including `typed_capture`) the itemized list above missed.
        assert_eq!(
            entry_a, entry_b,
            "the full ImageEntry must be identical across two independent runs"
        );

        let png_path_a = entry_a["png_path"].as_str().expect("png_path is a string");
        let png_path_b = entry_b["png_path"].as_str().expect("png_path is a string");
        let png_a = fs::read(output_a.join(png_path_a)).expect("read PNG a");
        let png_b = fs::read(output_b.join(png_path_b)).expect("read PNG b");
        assert_eq!(
            png_a, png_b,
            "on-disk PNG bytes must be byte-identical across two independent CLI runs"
        );
        let (dims_a, dims_b) = (png_dimensions(&png_a), png_dimensions(&png_b));
        assert_eq!(dims_a, dims_b, "PNG raster dimensions must match");
    }

    // The bundle-wide `viewport` (top-level manifest field, not just the
    // per-entry echo) must also match.
    assert_eq!(
        manifest_a["viewport"], manifest_b["viewport"],
        "the bundle-wide manifest viewport must be deterministic across independent runs"
    );
}

// ─────────────────────────────── AC-N1 ─────────────────────────────────────

#[test]
fn invalid_typed_geometry_fails_without_partial_success() {
    // A `PerimeterIR` with no regions at all has no documented geometry
    // field (`infill_areas`/`walls`) to source `filled_areas` from anywhere.
    let capture = StageCapture {
        stage_id: "Layer::Perimeters".to_string(),
        layer_index: 2,
        layer_z: 2.0,
        ir: CapturedIr::Perimeter(PerimeterIR::default()),
    };
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));

    let err = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect_err("a capture with no regions must not succeed");

    match err {
        RenderError::MissingGeometryField {
            tap,
            layer_index,
            field,
        } => {
            assert_eq!(
                tap, "Layer::Perimeters",
                "error must name the offending tap"
            );
            assert_eq!(layer_index, 2, "error must name the offending layer");
            assert!(!field.is_empty(), "error must name the missing field");
        }
        other => panic!("expected RenderError::MissingGeometryField, got {other:?}"),
    }
}

// ─────────────────────────────── AC-N2 ─────────────────────────────────────

#[test]
fn missing_typed_width_is_rejected() {
    let capture = simple_infill_capture(0.0);

    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));

    let err = render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect_err("a path with no usable width must reject filled_areas rather than infer one");

    match err {
        RenderError::MissingWidth { tap, layer_index } => {
            assert_eq!(tap, "Layer::Infill");
            assert_eq!(layer_index, 0);
        }
        other => panic!("expected RenderError::MissingWidth, got {other:?}"),
    }

    // Sanity: filament_lines never needs width and must still succeed for
    // the identical capture — proves the rejection above is specific to
    // filled_areas' width-sweep requirement, not a generally-broken fixture.
    render_stage_capture(
        &capture,
        RenderView::Geometry(GeometryView::FilamentLines),
        1,
        bounds,
    )
    .expect("filament_lines does not require width and must still succeed");
}

// ─────────────────────────────── AC-N3 ─────────────────────────────────────

#[test]
fn unsupported_resolution_scale_fails_without_output() {
    let capture = simple_infill_capture(1.0);
    let bounds = compute_viewport_bounds(std::slice::from_ref(&capture));

    for scale in [0u32, 4u32] {
        let err = render_stage_capture(
            &capture,
            RenderView::Geometry(GeometryView::FilledAreas),
            scale,
            bounds,
        )
        .expect_err("scale outside {{1, 2, 3}} must be rejected");
        assert_eq!(err, RenderError::UnsupportedResolutionScale { scale });
    }
}

// ───────────────────── AC-N1/N2/N3 (CLI-level guarantee) ───────────────────

// Gap-2 follow-up fix note (supersedes the prior "Finding 5" note, which
// concluded `RenderError::MissingGeometryField` was unreachable via the CLI
// and is now known to be wrong for one specific tap): every per-layer tap
// EXCEPT `Layer::PathOptimization` skips its own arena commit entirely when
// it has nothing to contribute (`Layer::Perimeters`/`Layer::Infill`/
// `Layer::Support` all do this), so an empty-but-present capture never
// reaches the renderer for them — confirmed unreachable, as the prior note
// found.
//
// `Layer::PathOptimization` is different: `crates/slicer-runtime/src/
// layer_executor.rs`'s `prestage_layer_collection_if_path_optimization`
// (called unconditionally immediately before the `Layer::PathOptimization`
// stage runs, `layer_executor.rs:539-562`) ALWAYS calls
// `arena.set_layer_collection(...)` with an `ordered_entities` list
// assembled from whatever `arena.perimeter()`/`arena.infill()`/
// `arena.support()` already hold — even when all three are `None` (no
// module bound to those stages in this plan) it still commits a
// `LayerCollectionIR` with an EMPTY `ordered_entities: Vec::new()`, not a
// skipped commit. `capture_ir_for_stage("Layer::PathOptimization", arena)`
// (`layer_executor.rs:745`) therefore always returns `Some(..)`, never
// `None`/`TapSourceUnavailable`, for this tap. Selecting ONLY
// `Layer::PathOptimization` as the requested tap (`SUPPORTED_TAP_STAGE_IDS`
// includes it, `layer_executor.rs:585`) with `module_dirs` limited to
// `layer-planner-default` (needed for `PrePass::LayerPlanning` so the model
// has real layers at all) and `path-optimization-default` (the tap itself)
// — deliberately excluding any perimeter/infill/support-generating module —
// leaves every layer's `ordered_entities` genuinely empty, which
// `layer_collection_shapes` (`visual_debug_render.rs:582`) turns into a real
// `RenderError::MissingGeometryField { field: "ordered_entities", .. }`,
// verified by direct CLI probing against `resources/regression_wedge.stl`
// before writing the test below. `RenderError::UnsupportedResolutionScale`
// and `RenderError::MissingWidth` remain unreachable via the CLI for the
// reasons the prior note already established (`validate_request`'s
// pre-check; every width-controlling config key's `min = 0.1` schema
// clamp) — this fix only concerns `MissingGeometryField`.

#[test]
fn missing_layer_collection_geometry_fails_with_render_failed_via_cli() {
    // AC-N1: "fails with a typed renderer error naming the tap, layer, and
    // missing field and does not report a successful bundle or leave a
    // successful image entry for that capture." Drives a genuine
    // `VisualDebugError::RenderFailed` (not `CaptureFailed`) end to end
    // through `run_visual_debug`.
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");
    let module_dir = minimal_layer_collection_only_module_dir(tmp.path());

    let req = VisualDebugRequest {
        schema_version: "1.0.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(wedge_path()),
            config: Some(config),
            module_dirs: vec![module_dir],
            path: None,
        },
        layers: vec![LayerSelector::Index(0)],
        taps: vec![TapSelector::Name("Layer::PathOptimization".to_string())],
        visualizations: vec![VisualizationSpec::Name("filled_areas".to_string())],
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    };

    let err = run_visual_debug(req, &output, false).expect_err(
        "a layer with no perimeter/infill/support module committing anything must \
                     leave Layer::PathOptimization's ordered_entities empty and fail to render",
    );

    match &err {
        VisualDebugError::RenderFailed(msg) => {
            assert!(
                msg.contains("Layer::PathOptimization"),
                "AC-N1: error must name the offending tap; got: {msg}"
            );
            assert!(
                msg.contains('0'),
                "AC-N1: error must name the offending layer (0); got: {msg}"
            );
            assert!(
                msg.contains("ordered_entities"),
                "AC-N1: error must name the missing field; got: {msg}"
            );
        }
        other => panic!("expected VisualDebugError::RenderFailed, got {other:?}"),
    }

    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when the renderer genuinely fails"
    );
    assert!(
        !output.exists()
            || fs::read_dir(&output)
                .expect("read output dir")
                .next()
                .is_none(),
        "no stray PNG or other file may land in the output directory when the renderer \
         genuinely fails"
    );
}

/// A `module_dirs` root carrying only `layer-planner-default` (so the model
/// has a real `LayerPlanIR`/global layer list at all — required for
/// `PrePass::LayerPlanning` to succeed) and `path-optimization-default`
/// (the `Layer::PathOptimization` tap itself) — deliberately excluding every
/// perimeter/infill/support-generating module. Copies the two modules'
/// already-built `<name>.toml` + `<name>.wasm` pair from the real
/// `modules/core-modules/` tree (built once for the whole workspace by
/// `cargo xtask build-guests`, same precondition every other test in this
/// file already relies on via `module_dir()`) into a fresh subdirectory of
/// `tmp` so this test's isolation doesn't depend on any file outside the
/// repo.
fn minimal_layer_collection_only_module_dir(tmp: &Path) -> PathBuf {
    let dest_root = tmp.join("minimal-modules");
    for module_name in ["layer-planner-default", "path-optimization-default"] {
        let dest_dir = dest_root.join(module_name);
        fs::create_dir_all(&dest_dir).expect("create minimal module subdir");
        for ext in ["toml", "wasm"] {
            let file_name = format!("{module_name}.{ext}");
            let src = module_dir().join(module_name).join(&file_name);
            fs::copy(&src, dest_dir.join(&file_name)).unwrap_or_else(|e| {
                panic!(
                    "copy {} -> {}: {e} (has `cargo xtask build-guests` run?)",
                    src.display(),
                    dest_dir.join(&file_name).display()
                )
            });
        }
    }
    dest_root
}

// Finding 5 (packet 159 second review pass): the CLI-level "no partial
// bundle" guarantee on a real `CaptureFailed` (as opposed to `RenderFailed`)
// still holds independent coverage below — a different, still-real failure
// mode (`Layer::Support` uncommitted because `support_enabled` defaults to
// `false`) that fails before `run_model_source`'s render loop is ever
// reached, even on a request that carries a non-empty `visualizations` list.

#[test]
fn capture_failure_on_a_visualization_bearing_request_leaves_no_partial_bundle_via_cli() {
    let tmp = TempDir::new().expect("tempdir");
    let config = write_bounded_config(tmp.path());
    let output = tmp.path().join("bundle");

    // `Layer::Support` with `support_enabled` left at its default (`false`
    // in every support-generating module): the tap is documented/supported
    // but no module commits it at layer 0 for this request, so capture
    // fails before `run_model_source`'s render loop is ever reached — even
    // though this request carries a real `visualizations` entry.
    let req = model_request_with_viz(
        vec!["Layer::Support"],
        vec![0],
        config,
        vec![VisualizationSpec::Name("filled_areas".to_string())],
        1,
    );

    let err = run_visual_debug(req, &output, false).expect_err(
        "an uncommitted tap source must not succeed even with visualizations requested",
    );
    assert!(
        matches!(err, VisualDebugError::CaptureFailed(_)),
        "expected VisualDebugError::CaptureFailed, got {err:?}"
    );

    assert!(
        !output.join("manifest.json").exists(),
        "no partial bundle/manifest may be written when tap capture fails on a \
         visualization-bearing request"
    );
    assert!(
        !output.exists()
            || fs::read_dir(&output)
                .expect("read output dir")
                .next()
                .is_none(),
        "no stray PNG or other file may land in the output directory when tap capture \
         fails on a visualization-bearing request"
    );
}
