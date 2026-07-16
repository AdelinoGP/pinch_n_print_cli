//! Framing contract for the visual-debug renderers: **aspect ratio survives
//! projection**, and one `Projector` owns the world(mm)→pixel transform for
//! every render path.
//!
//! Why this file exists. Packets 159 and 160 each wrote their own transform.
//! The standalone-G-code renderer scaled uniformly and letterboxed; the typed
//! stage renderer normalized X and Y independently against an always-square
//! 1024x1024 canvas, stretching any non-square model — a Benchy footprint
//! (~60x31 mm, ~2:1) rendered visibly squashed, worst at the prepass slice
//! tap. Nothing caught it, because the stage renderer's own test helper
//! reimplemented the same broken arithmetic instead of calling the renderer.
//!
//! These tests assert the *user-visible symptom* — a square in millimeters
//! must render square in pixels — rather than restating the transform's
//! algebra, so they stay honest if the implementation is rewritten again.

use slicer_ir::{
    ExPolygon, Point2, Polygon, SliceIR, SlicedRegion, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::{
    compute_viewport_bounds, render_stage_capture, CapturedIr, GeometryView, Projector, RenderView,
    StageCapture, ViewportBoundsMm, BASE_DIMENSION_PX,
};

const BACKGROUND: [u8; 3] = [255, 255, 255];

/// An axis-aligned mm-space rectangle as a filled `ExPolygon`.
fn rect_mm(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: vec![],
    }
}

/// A `Layer::Slice` capture (the prepass slice tap named in the bug report)
/// carrying `polygons` as one region's geometry.
fn slice_capture(polygons: Vec<ExPolygon>) -> StageCapture {
    StageCapture {
        stage_id: "Layer::Slice".to_string(),
        layer_index: 0,
        layer_z: 0.2,
        ir: CapturedIr::Slice(SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            regions: vec![SlicedRegion {
                polygons,
                ..Default::default()
            }],
            ..Default::default()
        }),
    }
}

fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().expect("valid PNG header");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("fixed-size RGB8 buffer")];
    let info = reader.next_frame(&mut buf).expect("valid PNG frame");
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}

fn pixel(rgb: &[u8], width: u32, x: usize, y: usize) -> [u8; 3] {
    let i = (y * width as usize + x) * 3;
    [rgb[i], rgb[i + 1], rgb[i + 2]]
}

/// Pixel-space bbox of everything that isn't background.
fn drawn_bbox(rgb: &[u8], width: u32, height: u32) -> (usize, usize, usize, usize) {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (usize::MAX, usize::MAX, 0usize, 0usize);
    for y in 0..height as usize {
        for x in 0..width as usize {
            if pixel(rgb, width, x, y) != BACKGROUND {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(min_x != usize::MAX, "nothing was drawn");
    (min_x, min_y, max_x, max_y)
}

/// **The reported bug.** A 10x10 mm square, inside a viewport whose extent is
/// ~2:1 (a Benchy-like footprint), must render as a square: equal pixel width
/// and height.
///
/// Before the shared `Projector` landed, the stage renderer mapped X against
/// the viewport's width and Y against its height independently onto a square
/// canvas, so this square came out ~2:1 — stretched along X by exactly the
/// viewport's aspect ratio.
#[test]
fn mm_square_renders_square_in_a_non_square_viewport() {
    // Wide geometry (60x30 mm) fixes a ~2:1 viewport; the 10x10 mm square
    // sits inside it and is what we measure.
    let square = rect_mm(10.0, 10.0, 20.0, 20.0);
    let bounds = compute_viewport_bounds(&[slice_capture(vec![rect_mm(0.0, 0.0, 60.0, 30.0)])]);

    let aspect = (bounds.max_x - bounds.min_x) / (bounds.max_y - bounds.min_y);
    assert!(
        aspect > 1.5,
        "fixture must pin a non-square viewport or it cannot detect stretching; got {aspect}"
    );

    let img = render_stage_capture(
        &slice_capture(vec![square]),
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("render");
    let (w, h, rgb) = decode(&img.png_bytes);
    let (min_x, min_y, max_x, max_y) = drawn_bbox(&rgb, w, h);

    let px_w = (max_x - min_x) as f32;
    let px_h = (max_y - min_y) as f32;
    let ratio = px_w / px_h;
    assert!(
        (ratio - 1.0).abs() < 0.02,
        "a 10x10 mm square must render square: got {px_w}x{px_h} px (ratio {ratio:.3}). \
         A ratio near the viewport's own aspect ({aspect:.3}) means the renderer is \
         scaling X and Y independently again."
    );
}

/// The uniform scale is the *same* number on both axes, so any mm length
/// projects to the same pixel length regardless of orientation.
#[test]
fn projector_scale_is_uniform_across_axes() {
    let bounds = ViewportBoundsMm {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 60.0,
        max_y: 30.0,
    };
    let p = Projector::new(bounds, 1024, 1024);

    let (x0, y0) = p.project(10.0, 10.0);
    let (x1, y1) = p.project(20.0, 20.0);

    let dx = x1 - x0;
    let dy = y0 - y1; // Y is flipped
    assert!(
        (dx - dy).abs() < 1e-6,
        "10 mm must project to the same pixel length on both axes; got dx={dx}, dy={dy}"
    );
    assert!(
        (p.scale_mm(10.0) - dx).abs() < 1e-6,
        "scale_mm must agree with project()"
    );
}

/// Aspect-preserving fit means the geometry is centered, with the slack on the
/// short axis split evenly into letterbox bands — not stretched to fill.
#[test]
fn non_square_viewport_is_letterboxed_and_centered() {
    let bounds = compute_viewport_bounds(&[slice_capture(vec![rect_mm(0.0, 0.0, 60.0, 30.0)])]);
    let img = render_stage_capture(
        &slice_capture(vec![rect_mm(0.0, 0.0, 60.0, 30.0)]),
        RenderView::Geometry(GeometryView::FilledAreas),
        1,
        bounds,
    )
    .expect("render");
    let (w, h, rgb) = decode(&img.png_bytes);
    let (min_x, min_y, max_x, max_y) = drawn_bbox(&rgb, w, h);

    // A ~2:1 viewport in a square canvas fills the width and leaves ~a
    // quarter of the height blank above and below.
    let top = min_y;
    let bottom = h as usize - 1 - max_y;
    assert!(
        (top as i64 - bottom as i64).abs() <= 2,
        "letterbox bands must be even: {top} px above vs {bottom} px below"
    );
    assert!(
        top > 100,
        "a 2:1 viewport must letterbox in a square canvas, not fill it; top band was {top} px"
    );
    // And it must be wider than it is tall, in the same proportion as the mm.
    let drawn_aspect = (max_x - min_x) as f32 / (max_y - min_y) as f32;
    assert!(
        (drawn_aspect - 2.0).abs() < 0.05,
        "a 60x30 mm rect must render ~2:1; got {drawn_aspect:.3}"
    );
}

/// The viewport is *model-wide*: it must not depend on which layers/captures a
/// request happened to select, or two bundles over one model can't be compared
/// stage-to-stage. `union` is how `pnp-cli` combines the mesh extent with the
/// captured geometry, so it must be order-independent and only ever grow.
#[test]
fn union_grows_to_cover_both_and_is_order_independent() {
    let a = ViewportBoundsMm {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 60.0,
        max_y: 30.0,
    };
    // Skirt-like geometry outside the mesh footprint, on all four sides.
    let b = ViewportBoundsMm {
        min_x: -5.0,
        min_y: -5.0,
        max_x: 65.0,
        max_y: 35.0,
    };

    let ab = a.union(b);
    assert_eq!(ab, b.union(a), "union must be order-independent");
    assert_eq!(
        ab,
        ViewportBoundsMm {
            min_x: -5.0,
            min_y: -5.0,
            max_x: 65.0,
            max_y: 35.0,
        },
        "union must cover geometry outside the mesh footprint (brim/skirt/support), \
         which a mesh-only viewport would silently clip"
    );
    assert_eq!(a.union(a), a, "union with self is identity");
}

/// The fixed margin is an absolute mm distance applied equally to both axes.
/// A margin expressed as a fraction of each axis' own extent is itself
/// anisotropic — it distorts the viewport before projection even begins, which
/// is what this renderer used to do (`MARGIN_FRACTION = 0.05`).
#[test]
fn margin_is_equal_absolute_mm_on_both_axes() {
    let bounds = compute_viewport_bounds(&[slice_capture(vec![rect_mm(0.0, 0.0, 60.0, 30.0)])]);

    let left = 0.0 - bounds.min_x;
    let right = bounds.max_x - 60.0;
    let bottom = 0.0 - bounds.min_y;
    let top = bounds.max_y - 30.0;

    for (name, got) in [
        ("left", left),
        ("right", right),
        ("bottom", bottom),
        ("top", top),
    ] {
        assert!(
            (got - slicer_runtime::VIEWPORT_MARGIN_MM).abs() < 1e-4,
            "{name} margin must equal VIEWPORT_MARGIN_MM ({} mm); got {got} mm. \
             Unequal X/Y margins mean a proportional (anisotropic) margin is back.",
            slicer_runtime::VIEWPORT_MARGIN_MM
        );
    }
}

/// Every rendered pixel of a shape at the viewport's extreme corners must land
/// inside the raster — the projection must not push geometry off-canvas.
#[test]
fn geometry_at_viewport_corners_stays_on_canvas() {
    let bounds = compute_viewport_bounds(&[slice_capture(vec![rect_mm(0.0, 0.0, 60.0, 30.0)])]);
    let p = Projector::new(bounds, BASE_DIMENSION_PX, BASE_DIMENSION_PX);

    for (x, y) in [
        (bounds.min_x, bounds.min_y),
        (bounds.max_x, bounds.max_y),
        (bounds.min_x, bounds.max_y),
        (bounds.max_x, bounds.min_y),
    ] {
        let (px, py) = p.project(f64::from(x), f64::from(y));
        assert!(
            px >= -0.5 && px <= f64::from(BASE_DIMENSION_PX) + 0.5,
            "({x},{y}) mm projected to px={px}, outside the raster"
        );
        assert!(
            py >= -0.5 && py <= f64::from(BASE_DIMENSION_PX) + 0.5,
            "({x},{y}) mm projected to py={py}, outside the raster"
        );
    }
}

/// Y is flipped: larger mm-Y renders toward the top of the image (smaller row
/// index), matching the print-bed convention.
#[test]
fn larger_mm_y_renders_toward_the_top() {
    let bounds = ViewportBoundsMm {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 60.0,
        max_y: 30.0,
    };
    let p = Projector::new(bounds, 1024, 1024);
    let (_, low) = p.project(30.0, 5.0);
    let (_, high) = p.project(30.0, 25.0);
    assert!(
        high < low,
        "mm-Y 25 must render above mm-Y 5 (smaller row index); got {high} vs {low}"
    );
}
