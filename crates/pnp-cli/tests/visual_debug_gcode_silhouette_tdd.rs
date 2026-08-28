//! Packet 248, Steps 1-2 — parser groundwork and the two pure silhouette
//! derivation helpers for the standalone `.gcode` source.
//!
//! Scope is deliberately narrow: these tests pin the *helper* contracts
//! (`parse_gcode`'s `G92 E` synchronization, `gcode_silhouette_slabs`,
//! `silhouette_segment_width_mm`), not the renderer, validation, or bundle
//! assembly built on top of them by later steps.
//!
//! All fixtures are authored inline as small strings — no real `.gcode`
//! artifact is ever loaded from disk.

use pnp_cli::visual_debug::visual_debug_gcode::{
    gcode_silhouette_slabs, parse_gcode, silhouette_segment_width_mm,
};

/// Filament cross-sectional area in mm² for the given diameter.
fn filament_area_mm2(diameter_mm: f64) -> f64 {
    std::f64::consts::PI * (diameter_mm / 2.0).powi(2)
}

// ───────────────────────────────── AC-7 ─────────────────────────────────────

/// AC-7: a mid-file `G92 E0` in an ABSOLUTE-extrusion file must synchronize
/// the parser's carried E position.
///
/// Before this fix the parser carried `last_e` across the reset, so the first
/// post-reset extruding move computed `e_delta = 0.4 - 5.0`, a large NEGATIVE
/// delta, and was misclassified as travel (`is_extrusion == false`). A `G92`
/// line carrying an `E` token is now an understood construct and must not
/// produce an unsupported-construct warning.
#[test]
fn g92_e_reset_synchronizes_e_position() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M82
G1 X10 Y10 F3000
G1 X20 Y10 E5.0
G92 E0
G1 X30 Y10 E0.4
";
    let parsed = parse_gcode(gcode);

    let segs: Vec<_> = parsed
        .layers
        .iter()
        .flat_map(|l| l.segments.iter())
        .collect();
    // Two segments, not three: the file's opening `G1 X10 Y10` has no known
    // `from` (the parser never fabricates a start position), so it emits no
    // segment of its own.
    assert_eq!(
        segs.len(),
        2,
        "expected two XY segments (the two extrusions), got {segs:?}"
    );

    // The pre-reset extrusion: absolute 0.0 -> 5.0.
    assert!(segs[0].is_extrusion, "pre-reset move must be an extrusion");
    assert!(
        (segs[0].e_delta_mm - 5.0).abs() < 1e-12,
        "pre-reset e_delta_mm = {}, expected 5.0",
        segs[0].e_delta_mm
    );

    // The post-reset extrusion: `G92 E0` re-zeroed the axis, so this is
    // +0.4, NOT 0.4 - 5.0 = -4.6.
    assert!(
        segs[1].is_extrusion,
        "post-`G92 E0` extruding move was misclassified as travel \
         (e_delta_mm = {})",
        segs[1].e_delta_mm
    );
    assert!(
        segs[1].e_delta_mm > 0.0,
        "post-reset e_delta_mm must be POSITIVE, got {}",
        segs[1].e_delta_mm
    );
    assert!(
        (segs[1].e_delta_mm - 0.4).abs() < 1e-12,
        "post-reset e_delta_mm = {}, expected 0.4",
        segs[1].e_delta_mm
    );

    // A `G92` carrying an `E` token is understood — no warning about it.
    assert!(
        !parsed.warnings.iter().any(|w| w.contains("G92")),
        "`G92 E0` must not warn; warnings = {:?}",
        parsed.warnings
    );
}

/// A `G92` with no `E` token (X/Y/Z offsets) remains an unsupported
/// construct — the fix must not silently swallow position offsets the parser
/// does not model.
#[test]
fn g92_without_e_still_warns() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
G1 X10 Y10 F3000
G92 X0 Y0
G1 X20 Y10 F3000
";
    let parsed = parse_gcode(gcode);
    assert!(
        parsed
            .warnings
            .iter()
            .any(|w| w.contains("G92") && w.contains("unsupported")),
        "`G92 X0 Y0` must still warn; warnings = {:?}",
        parsed.warnings
    );
}

/// `M200` is recorded as a volumetric-extrusion poison marker rather than
/// warned about: flow-derived widths are meaningless once E is volumetric.
#[test]
fn m200_recorded_not_warned() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
M200 D1.75
G1 X10 Y10 F3000
G1 X20 Y10 E1.0
";
    let parsed = parse_gcode(gcode);
    assert_eq!(
        parsed.volumetric_extrusion_line,
        Some(3),
        "expected the 1-indexed source line of the first M200"
    );
    assert!(
        !parsed.warnings.iter().any(|w| w.contains("M200")),
        "M200 is understood and must not warn; warnings = {:?}",
        parsed.warnings
    );

    let no_m200 = parse_gcode(";LAYER_CHANGE\n;Z:0.2\nG1 X10 Y10 F3000\n");
    assert_eq!(no_m200.volumetric_extrusion_line, None);
}

// ───────────────────────────────── AC-2 ─────────────────────────────────────

/// AC-2 (closed-form half): a width authored via
/// `Δe = L × w × h / A_filament` must round-trip back through
/// [`silhouette_segment_width_mm`] to exactly the authored `w`, in BOTH
/// `M82` (absolute) and `M83` (relative) extrusion modes.
#[test]
fn silhouette_width_formula_closed_form() {
    const W: f64 = 0.5; // authored extrusion width, mm
    const H: f64 = 0.2; // slab height, mm
    const D: f64 = 1.75; // filament diameter, mm
    const L: f64 = 10.0; // move length, mm

    let e = L * W * H / filament_area_mm2(D);

    // Absolute mode: E is a cumulative axis position. Starting from 0, the
    // single extruding move states `E{e}`, so Δe == e.
    let absolute = format!(
        "; filament_diameter = {D}\n\
         ;LAYER_CHANGE\n\
         ;Z:{H}\n\
         ;TYPE:Perimeter\n\
         M82\n\
         G1 X0 Y0 F3000\n\
         G1 X{L} Y0 E{e:.17}\n"
    );
    // Relative mode: the E token IS the delta.
    let relative = format!(
        "; filament_diameter = {D}\n\
         ;LAYER_CHANGE\n\
         ;Z:{H}\n\
         ;TYPE:Perimeter\n\
         M83\n\
         G1 X0 Y0 F3000\n\
         G1 X{L} Y0 E{e:.17}\n"
    );

    for (label, gcode) in [("M82 absolute", &absolute), ("M83 relative", &relative)] {
        let parsed = parse_gcode(gcode);

        assert_eq!(
            parsed.filament_diameters_mm,
            vec![D],
            "{label}: the filament_diameter comment must be parsed"
        );

        let seg = parsed
            .layers
            .iter()
            .flat_map(|l| l.segments.iter())
            .find(|s| s.is_extrusion)
            .unwrap_or_else(|| panic!("{label}: no extrusion segment parsed"));

        assert!(
            (seg.e_delta_mm - e).abs() < 1e-12,
            "{label}: e_delta_mm = {}, expected {e}",
            seg.e_delta_mm
        );

        let width = silhouette_segment_width_mm(seg.e_delta_mm, L, H, D);
        assert!(
            (width - W).abs() < 1e-9,
            "{label}: width round-tripped to {width}, expected {W}"
        );
    }
}

// ───────────────────────────────── AC-6 ─────────────────────────────────────

/// AC-6 (helper half): [`gcode_silhouette_slabs`] emits exactly one W3
/// warning per layer with a duplicate, non-monotonic, or absent `;Z:` marker,
/// and emits NO slab for those layers. A skipped layer must not advance the
/// carried marker, so the next good layer's slab bottom is the last ACCEPTED
/// Z. The first accepted marker's slab always starts at 0.0.
#[test]
fn slab_derivation_w3_cases() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
G1 X0 Y0 F3000
G1 X10 Y0 E1.0
;LAYER_CHANGE
;Z:0.2
G1 X10 Y10 E2.0
;LAYER_CHANGE
;Z:0.1
G1 X0 Y10 E3.0
;LAYER_CHANGE
G1 X0 Y0 E4.0
;LAYER_CHANGE
;Z:0.4
G1 X10 Y0 E5.0
";
    let parsed = parse_gcode(gcode);
    assert_eq!(parsed.layers.len(), 5, "fixture should parse five layers");

    let (slabs, warnings) = gcode_silhouette_slabs(&parsed);

    assert_eq!(
        warnings.len(),
        3,
        "expected exactly three W3 warnings, got {warnings:?}"
    );

    // Warning 0: layer 1, duplicate Z (0.2 <= 0.2).
    assert!(
        warnings[0].contains('1') && warnings[0].contains("0.2"),
        "warning 0 must name layer 1 and Z 0.2: {}",
        warnings[0]
    );
    // Warning 1: layer 2, non-monotonic Z (0.1 <= 0.2).
    assert!(
        warnings[1].contains('2') && warnings[1].contains("0.1"),
        "warning 1 must name layer 2 and Z 0.1: {}",
        warnings[1]
    );
    // Warning 2: layer 3, absent marker.
    assert!(
        warnings[2].contains('3') && warnings[2].contains("no ;Z: marker"),
        "warning 2 must name layer 3 and the absent marker: {}",
        warnings[2]
    );

    // No slab for any warned layer.
    for skipped in [1_i64, 2, 3] {
        assert!(
            !slabs.contains_key(&skipped),
            "layer {skipped} was warned and must have NO slab; slabs = {slabs:?}"
        );
    }

    // The first accepted marker's slab bottom is always 0.0, never a
    // marker-delta guess; the skipped layers did not advance the carried
    // marker, so layer 4's slab starts at 0.2.
    assert_eq!(slabs.len(), 2, "slabs = {slabs:?}");
    let (b0, t0) = slabs[&0];
    assert!(
        (b0 - 0.0).abs() < 1e-12 && (t0 - 0.2).abs() < 1e-12,
        "layer 0 slab = ({b0}, {t0}), expected (0.0, 0.2)"
    );
    let (b4, t4) = slabs[&4];
    assert!(
        (b4 - 0.2).abs() < 1e-12 && (t4 - 0.4).abs() < 1e-12,
        "layer 4 slab = ({b4}, {t4}), expected (0.2, 0.4)"
    );
}

/// Closure-review fix: the FIRST `;Z:` marker must clear the same
/// monotonicity bar as every subsequent one — for it, `prev` is the bed, so
/// the marker must be strictly `> 0.0` and finite.
///
/// Before this fix the first marker was special-cased straight into
/// `(0.0, z)` with no comparison at all, so `;Z:0` or `;Z:-0.1` produced a
/// degenerate or inverted slab. `silhouette_segment_width_mm` divides by the
/// slab height, so every move on that layer silently rendered at width 0 —
/// fail-OPEN, with no W3 warning and the layer still counted in
/// `layers_rendered`. A layer that renders nothing must say so.
#[test]
fn first_z_marker_must_be_positive() {
    for (bad_z, label) in [("0", "zero"), ("-0.1", "negative")] {
        let gcode = format!(
            "\
;LAYER_CHANGE
;Z:{bad_z}
;TYPE:Perimeter
M83
G1 X0 Y10 F3000
G1 X20 Y10 E1.0
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
G1 X5 Y10 F3000
G1 X15 Y10 E0.5
"
        );

        // ── helper half ────────────────────────────────────────────────────
        let parsed = parse_gcode(&gcode);
        let (slabs, warnings) = gcode_silhouette_slabs(&parsed);

        assert_eq!(
            warnings.len(),
            1,
            "{label}: expected exactly one W3 warning, got {warnings:?}"
        );
        assert!(
            warnings[0].starts_with("W3:")
                && warnings[0].contains('0')
                && warnings[0].contains(bad_z),
            "{label}: the W3 warning must name layer 0 and Z {bad_z}: {}",
            warnings[0]
        );
        assert!(
            !slabs.contains_key(&0),
            "{label}: layer 0 was warned and must have NO slab; slabs = {slabs:?}"
        );
        // The rejected marker must NOT advance the carried value, so the next
        // good layer's slab is still measured from the bed.
        assert_eq!(slabs.len(), 1, "{label}: slabs = {slabs:?}");
        let (b1, t1) = slabs[&1];
        assert!(
            (b1 - 0.0).abs() < 1e-12 && (t1 - 0.2).abs() < 1e-12,
            "{label}: layer 1 slab = ({b1}, {t1}), expected (0.0, 0.2)"
        );

        // ── renderer half ──────────────────────────────────────────────────
        let out = render_gcode_silhouette(
            &gcode,
            &[0, 1],
            SilhouetteView::Front,
            CANVAS,
            CANVAS,
            Some(0.4),
            ColorBy::Role,
        )
        .expect("silhouette render must succeed");

        assert_eq!(
            out.layers_rendered,
            vec![1],
            "{label}: the rejected layer must be EXCLUDED from layers_rendered"
        );
        assert!(
            out.warnings
                .iter()
                .any(|w| w.starts_with("W3:") && w.contains('0') && w.contains(bad_z)),
            "{label}: the render output must carry the W3 warning: {:?}",
            out.warnings
        );
        // Layer 0 contributed no pixels: the only painted geometry is the
        // second layer's move, 10 mm wide inflated by 0.4/2 at each end.
        let tol = 1.5 * axes(&out).px_mm();
        let (lo, hi) = row_extent_mm(&out, 0.1)
            .unwrap_or_else(|| panic!("{label}: the accepted slab must paint a row at z = 0.1"));
        assert_close(lo, 4.8, tol, &format!("{label}: accepted slab left edge"));
        assert_close(hi, 15.2, tol, &format!("{label}: accepted slab right edge"));
    }
}

// ═══════════════════════════ Step 4: renderer ═══════════════════════════════
//
// Decoded-pixel assertions for `render_gcode_silhouette` — the composite
// silhouette entry point. Every fixture below is authored inline; none is
// loaded from disk.

use pnp_cli::visual_debug::visual_debug_gcode::{render_gcode_silhouette, GcodeSilhouetteOutput};
use slicer_runtime::visual_debug_style::{gcode_role_color, ColorBy, ToolColors};
use slicer_runtime::{Projector, SilhouetteView};

const CANVAS: u32 = 400;
const WHITE: [u8; 3] = [255, 255, 255];

fn decode_rgb(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().expect("decodable PNG");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("known buffer size")];
    let info = reader.next_frame(&mut buf).expect("decodable frame");
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// The affine (mm -> px) coefficients of the output's own projector,
/// recovered from two probe points so the test never re-derives the
/// transform by hand.
struct Axes {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

fn axes(out: &GcodeSilhouetteOutput) -> Axes {
    axes_from(out.world_bounds_mm, out.width, out.height)
}

/// The same recovery, from a raster's bounds/dimensions alone — used by the
/// bundle-level tests, which only ever see the manifest's `world_bounds_mm`
/// and a PNG on disk.
fn axes_from(bounds: slicer_runtime::ViewportBoundsMm, width: u32, height: u32) -> Axes {
    let p = Projector::new(bounds, width, height);
    let (x0, y0) = p.project(0.0, 0.0);
    let (x1, _) = p.project(1.0, 0.0);
    let (_, y1) = p.project(0.0, 1.0);
    Axes {
        a: x1 - x0,
        b: x0,
        c: y1 - y0,
        d: y0,
    }
}

impl Axes {
    fn h_px(&self, h_mm: f64) -> f64 {
        self.a * h_mm + self.b
    }
    fn v_px(&self, z_mm: f64) -> f64 {
        self.c * z_mm + self.d
    }
    fn h_mm(&self, px: f64) -> f64 {
        (px - self.b) / self.a
    }
    /// One pixel expressed in mm on the horizontal axis.
    fn px_mm(&self) -> f64 {
        1.0 / self.a
    }
}

/// The painted (non-white) horizontal extent, in mm, of the raster row that
/// the world-space height `z_mm` falls on. `None` when the row is blank.
fn row_extent_mm(out: &GcodeSilhouetteOutput, z_mm: f64) -> Option<(f64, f64)> {
    let ax = axes(out);
    let (w, h, rgb) = decode_rgb(&out.png_bytes);
    let row = ax.v_px(z_mm).round() as i64;
    if row < 0 || row >= h as i64 {
        return None;
    }
    let mut lo: Option<u32> = None;
    let mut hi: Option<u32> = None;
    for x in 0..w {
        let idx = (row as u32 * w + x) as usize * 3;
        if [rgb[idx], rgb[idx + 1], rgb[idx + 2]] != WHITE {
            lo.get_or_insert(x);
            hi = Some(x);
        }
    }
    let (lo, hi) = (lo?, hi?);
    Some((ax.h_mm(f64::from(lo)), ax.h_mm(f64::from(hi))))
}

fn pixel_at_mm(out: &GcodeSilhouetteOutput, h_mm: f64, z_mm: f64) -> [u8; 3] {
    let ax = axes(out);
    let (w, _h, rgb) = decode_rgb(&out.png_bytes);
    let x = ax.h_px(h_mm).round() as u32;
    let y = ax.v_px(z_mm).round() as u32;
    let idx = (y * w + x) as usize * 3;
    [rgb[idx], rgb[idx + 1], rgb[idx + 2]]
}

fn assert_close(actual: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: expected {expected} +/- {tol}, got {actual}"
    );
}

// ───────────────────────────────── AC-2 ─────────────────────────────────────

/// AC-2: the closed-form flow width round-trips to the authored 0.5 mm in
/// BOTH absolute (`M82`) and relative (`M83`) extrusion modes, and the
/// rendered silhouette interval for a horizontal move is that move's own X
/// extent inflated by `w/2` at EACH end.
#[test]
fn flow_width_roundtrip_absolute_and_relative_modes() {
    const DIAMETER: f64 = 1.75;
    const LENGTH: f64 = 10.0;
    const SLAB: f64 = 0.2;
    const WIDTH: f64 = 0.5;
    let e = WIDTH * LENGTH * SLAB / filament_area_mm2(DIAMETER);

    assert_close(
        silhouette_segment_width_mm(e, LENGTH, SLAB, DIAMETER),
        WIDTH,
        1e-9,
        "closed-form round-trip",
    );

    let absolute = format!(
        "; filament_diameter = {DIAMETER}\n\
         ;LAYER_CHANGE\n\
         ;Z:0.2\n\
         ;TYPE:Perimeter\n\
         M82\n\
         G1 X10 Y10 F3000\n\
         G1 X20 Y10 E{e:.12}\n"
    );
    let relative = format!(
        "; filament_diameter = {DIAMETER}\n\
         ;LAYER_CHANGE\n\
         ;Z:0.2\n\
         ;TYPE:Perimeter\n\
         M83\n\
         G1 X10 Y10 F3000\n\
         G1 X20 Y10 E{e:.12}\n"
    );

    for (label, gcode) in [("M82", &absolute), ("M83", &relative)] {
        let out = render_gcode_silhouette(
            gcode,
            &[0],
            SilhouetteView::Front,
            CANVAS,
            CANVAS,
            None,
            ColorBy::Role,
        )
        .unwrap_or_else(|e| panic!("{label}: silhouette render must succeed: {e:?}"));

        assert_eq!(out.layers_rendered, vec![0], "{label}: layer 0 rendered");
        let (lo, hi) = row_extent_mm(&out, 0.1).unwrap_or_else(|| {
            panic!("{label}: the layer-0 slab must paint a non-blank row at z = 0.1")
        });
        let tol = 1.5 * axes(&out).px_mm();
        assert_close(lo, 10.0 - WIDTH / 2.0, tol, &format!("{label}: left edge"));
        assert_close(hi, 20.0 + WIDTH / 2.0, tol, &format!("{label}: right edge"));
    }
}

// ───────────────────────────────── AC-3 ─────────────────────────────────────

/// AC-3: adaptive `;Z:` markers 0.2 / 0.5 / 0.65 derive per-layer slabs
/// `[0, 0.2]`, `[0.2, 0.5]`, `[0.5, 0.65]` — the first slab's bottom is 0,
/// and each later slab starts at the previous accepted marker. Verified by
/// decoded-pixel row extents at three distinct slab heights.
#[test]
fn adaptive_z_markers_derive_per_layer_slabs() {
    let gcode = "\
G1 X10 Y10 F3000
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X20 Y10 E0.5
;LAYER_CHANGE
;Z:0.5
G1 X12 Y10 F3000
G1 X18 Y10 E0.5
;LAYER_CHANGE
;Z:0.65
G1 X14 Y10 F3000
G1 X16 Y10 E0.5
";
    let out = render_gcode_silhouette(
        gcode,
        &[0, 1, 2],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.4),
        ColorBy::Role,
    )
    .expect("silhouette render must succeed with an explicit fallback width");

    assert_eq!(out.layers_rendered, vec![0, 1, 2]);
    let tol = 1.5 * axes(&out).px_mm();

    for (z, lo_mm, hi_mm, label) in [
        (0.1, 9.8, 20.2, "slab [0, 0.2]"),
        (0.35, 11.8, 18.2, "slab [0.2, 0.5]"),
        (0.575, 13.8, 16.2, "slab [0.5, 0.65]"),
    ] {
        let (lo, hi) =
            row_extent_mm(&out, z).unwrap_or_else(|| panic!("{label} must paint a row at z={z}"));
        assert_close(lo, lo_mm, tol, &format!("{label} left edge"));
        assert_close(hi, hi_mm, tol, &format!("{label} right edge"));
    }

    assert!(
        row_extent_mm(&out, 0.01).is_some(),
        "the first slab must reach down to z = 0"
    );
}

// ───────────────────────────────── AC-4 ─────────────────────────────────────

/// AC-4: with no `; filament_diameter` comment the flow derivation is
/// impossible, so an explicit `gcode_line_width_mm` is used — and it is the
/// width actually rendered (a 0.42 render is narrower than a 0.84 control).
#[test]
fn fallback_width_used_when_underivable() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.5
";
    let render = |w: f64| {
        render_gcode_silhouette(
            gcode,
            &[0],
            SilhouetteView::Front,
            CANVAS,
            CANVAS,
            Some(w),
            ColorBy::Role,
        )
        .expect("an explicit fallback width must make an underivable source renderable")
    };

    let narrow = render(0.42);
    let wide = render(0.84);
    let tol = 1.5 * axes(&narrow).px_mm();

    let (n_lo, n_hi) = row_extent_mm(&narrow, 0.1).expect("narrow render paints a row");
    let (w_lo, w_hi) = row_extent_mm(&wide, 0.1).expect("wide render paints a row");
    assert_close(n_lo, 10.0 - 0.21, tol, "0.42 left edge");
    assert_close(n_hi, 20.0 + 0.21, tol, "0.42 right edge");
    assert_close(w_lo, 10.0 - 0.42, tol, "0.84 left edge");
    assert_close(w_hi, 20.0 + 0.42, tol, "0.84 right edge");
    assert!(
        w_hi - w_lo > n_hi - n_lo,
        "the 0.84 extent ({}) must exceed the 0.42 extent ({})",
        w_hi - w_lo,
        n_hi - n_lo
    );
}

// ───────────────────────────────── AC-5 ─────────────────────────────────────

/// AC-5: extrusion before any `;TYPE:` marker is retained as its own
/// `unclassified` class, painted FIRST so every role class occludes it, and
/// reported in the warnings.
#[test]
fn unclassified_class_paints_first_and_warns() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
M83
G1 X10 Y10 F3000
G1 X30 Y10 E1.0
;TYPE:Perimeter
G1 X20 Y10 F3000
G1 X30 Y10 E0.5
";
    let out = render_gcode_silhouette(
        gcode,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.4),
        ColorBy::Role,
    )
    .expect("silhouette render must succeed");

    let unclassified = [128u8, 128, 128];
    let perimeter = gcode_role_color("Perimeter", "unclassified");
    assert_ne!(
        perimeter, unclassified,
        "fixture is only meaningful if the role color differs from gray"
    );

    assert_eq!(
        pixel_at_mm(&out, 15.0, 0.1),
        unclassified,
        "unclassified extrusion must survive where nothing overlaps it"
    );
    assert_eq!(
        pixel_at_mm(&out, 25.0, 0.1),
        perimeter,
        "a role class must occlude unclassified in the overlap"
    );

    assert!(
        out.warnings.iter().any(|w| w.contains("unclassified")),
        "expected an unclassified-extrusion warning, got {:?}",
        out.warnings
    );
}

// ───────────────────────────────── AC-8 ─────────────────────────────────────

/// AC-8: two independent renders of the same request are byte-identical.
#[test]
fn gcode_silhouette_is_deterministic() {
    let gcode = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
;TYPE:Solid infill
G1 X20 Y12 E0.1
G1 X12 Y12 E0.3
;LAYER_CHANGE
G1 X12 Y10 F3000
G1 X18 Y10 E0.3
";
    let render = || {
        render_gcode_silhouette(
            gcode,
            &[0, 1],
            SilhouetteView::Front,
            CANVAS,
            CANVAS,
            None,
            ColorBy::Role,
        )
        .expect("silhouette render must succeed")
    };
    let a = render();
    let b = render();

    assert_eq!(a.png_bytes, b.png_bytes, "PNG bytes must be identical");
    assert_eq!(a.warnings.len(), b.warnings.len(), "warning count");
    for (i, (x, y)) in a.warnings.iter().zip(b.warnings.iter()).enumerate() {
        assert_eq!(x, y, "warning {i} must be identical across renders");
    }
    assert_eq!(a.world_bounds_mm, b.world_bounds_mm);
    assert_eq!(a.layers_rendered, b.layers_rendered);
    assert_eq!(a.parser_version, b.parser_version);
    // The second layer has no ;Z: marker of its own, so it is W3-skipped and
    // must not appear in `layers_rendered`.
    assert_eq!(a.layers_rendered, vec![0]);
}

// ──────────────────────────────── AC-N1 ─────────────────────────────────────

/// AC-N1 (renderer half): no `filament_diameter` comment and no fallback
/// width — the render fails closed, naming the missing datum and the remedy.
#[test]
fn width_underivable_without_diameter_fails_closed() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.5
";
    let err = render_gcode_silhouette(
        gcode,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        None,
        ColorBy::Role,
    )
    .expect_err("an underivable width with no fallback must fail closed");
    let text = format!("{err:?}");
    assert!(
        text.contains("SilhouetteWidthUnderivable"),
        "expected the named variant, got {text}"
    );
    assert!(
        text.contains("filament_diameter"),
        "the error must name the missing datum, got {text}"
    );
    assert!(
        text.contains("gcode_line_width_mm"),
        "the error must name the remedy, got {text}"
    );
}

// ──────────────────────────────── AC-N2 ─────────────────────────────────────

/// AC-N2: `M200` makes E a volume, poisoning every flow-derived width from
/// its line onward. With no fallback the render fails closed naming `M200`;
/// the same fixture with an explicit width succeeds.
#[test]
fn m200_volumetric_poisons_flow_derivation() {
    let gcode = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
M200 D1.75
G1 X10 Y10 F3000
G1 X20 Y10 E0.5
";
    let err = render_gcode_silhouette(
        gcode,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        None,
        ColorBy::Role,
    )
    .expect_err("M200 with no fallback width must fail closed");
    let text = format!("{err:?}");
    assert!(
        text.contains("SilhouetteWidthUnderivable"),
        "expected the named variant, got {text}"
    );
    assert!(
        text.contains("M200"),
        "the error must name M200 as the poisoning construct, got {text}"
    );
    assert!(
        text.contains("gcode_line_width_mm"),
        "the error must name the remedy, got {text}"
    );

    let out = render_gcode_silhouette(
        gcode,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.42),
        ColorBy::Role,
    )
    .expect("the same fixture must render once a fallback width is supplied");
    let tol = 1.5 * axes(&out).px_mm();
    let (lo, hi) = row_extent_mm(&out, 0.1).expect("the fallback render paints a row");
    assert_close(lo, 10.0 - 0.21, tol, "fallback left edge");
    assert_close(hi, 20.0 + 0.21, tol, "fallback right edge");
}

// ──────────────────────────────── AC-10 ─────────────────────────────────────

/// AC-10 (renderer half): `ColorBy::Tool` paints tool classes in ascending
/// tool index from the fixed palette, so tool 1 occludes tool 0 on overlap.
/// A standalone `.gcode` resolves no config, so the palette is the only
/// color source.
#[test]
fn gcode_silhouette_tool_coloring_renderer_half() {
    let gcode = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X30 Y10 E1.0
T1
G1 X20 Y10 F3000
G1 X30 Y10 E0.5
";
    let out = render_gcode_silhouette(
        gcode,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.4),
        ColorBy::Tool,
    )
    .expect("silhouette render must succeed");

    let palette = ToolColors::default();
    let t0 = palette.color(0);
    let t1 = palette.color(1);
    assert_ne!(t0, t1, "fixture needs two distinct palette entries");

    assert_eq!(
        pixel_at_mm(&out, 15.0, 0.1),
        t0,
        "tool 0 must paint where nothing overlaps it"
    );
    assert_eq!(
        pixel_at_mm(&out, 25.0, 0.1),
        t1,
        "tool 1 must occlude tool 0 in the overlap (ascending tool paint order)"
    );
}

// ═════════════════════ Step 6: bundle assembly (end to end) ═════════════════
//
// These drive the whole `run_visual_debug` command over a standalone
// `.gcode` source and assert against the written bundle — `manifest.json`
// plus the PNGs on disk — never against the renderer's return value.

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, VisualDebugError, VisualDebugRequest,
    VisualDebugSource, VisualizationSpec,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Two layers, both with `;Z:` markers and `;TYPE:` roles, and a
/// `filament_diameter` comment so every width is flow-derivable with no
/// `gcode_line_width_mm` fallback.
const TWO_LAYER_GCODE: &str = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
;LAYER_CHANGE
;Z:0.4
;TYPE:Solid infill
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
";

fn write_fixture(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).expect("write gcode fixture");
    path
}

/// A schema 1.2.0 standalone-G-code silhouette request. `taps` is empty —
/// the gcode source has no captured stage to name, and R8 rejects a
/// non-empty `taps` on a silhouette outright.
fn silhouette_bundle_request(
    gcode_path: PathBuf,
    layers: Vec<LayerSelector>,
    visualizations: Vec<VisualizationSpec>,
) -> VisualDebugRequest {
    // exhaustive: standalone gcode silhouette request boundary fixture
    VisualDebugRequest {
        schema_version: "1.2.0".to_string(),
        source: VisualDebugSource::Gcode {
            path: Some(gcode_path),
            model: None,
        },
        layers,
        taps: Vec::new(),
        visualizations,
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn detail_viz(kind: &str, options: serde_json::Value) -> VisualizationSpec {
    VisualizationSpec::Detail {
        kind: kind.to_string(),
        options,
    }
}

fn read_manifest(manifest_path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(manifest_path).expect("manifest.json must exist"))
        .expect("manifest.json must be valid JSON")
}

/// Every file directly under the bundle's `images/` directory, sorted.
fn image_file_names(manifest_path: &Path) -> Vec<String> {
    let dir = manifest_path.parent().expect("bundle dir").join("images");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("images/ must exist")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn bundle_png(manifest_path: &Path, file_name: &str) -> Vec<u8> {
    let path = manifest_path
        .parent()
        .expect("bundle dir")
        .join("images")
        .join(file_name);
    fs::read(&path).unwrap_or_else(|e| panic!("expected {} on disk: {e}", path.display()))
}

fn entry_bounds(entry: &serde_json::Value) -> slicer_runtime::ViewportBoundsMm {
    let b = &entry["world_bounds_mm"];
    let f = |k: &str| {
        b[k].as_f64()
            .unwrap_or_else(|| panic!("world_bounds_mm.{k}")) as f32
    };
    slicer_runtime::ViewportBoundsMm {
        min_x: f("min_x"),
        min_y: f("min_y"),
        max_x: f("max_x"),
        max_y: f("max_y"),
    }
}

/// The rightmost horizontal position, in mm, at which the raster is painted.
/// `None` when the whole image is blank.
fn max_painted_h_mm(png_bytes: &[u8], bounds: slicer_runtime::ViewportBoundsMm) -> Option<f64> {
    let (w, h, rgb) = decode_rgb(png_bytes);
    let ax = axes_from(bounds, w, h);
    let mut max_px: Option<u32> = None;
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize * 3;
            if [rgb[idx], rgb[idx + 1], rgb[idx + 2]] != WHITE {
                max_px = Some(max_px.map_or(x, |m| m.max(x)));
            }
        }
    }
    max_px.map(|px| ax.h_mm(f64::from(px)))
}

fn pixel_at_mm_in(
    png_bytes: &[u8],
    bounds: slicer_runtime::ViewportBoundsMm,
    h_mm: f64,
    z_mm: f64,
) -> [u8; 3] {
    let (w, h, rgb) = decode_rgb(png_bytes);
    let ax = axes_from(bounds, w, h);
    let x = ax.h_px(h_mm).round() as u32;
    let y = ax.v_px(z_mm).round() as u32;
    let idx = (y * w + x) as usize * 3;
    [rgb[idx], rgb[idx + 1], rgb[idx + 2]]
}

/// The bundle's single silhouette entry: one image per (view, color mode)
/// group is the whole point, so "exactly one" is itself an assertion.
fn sole_image_entry(manifest: &serde_json::Value) -> serde_json::Value {
    let images = manifest["images"].as_array().expect("images array");
    assert_eq!(
        images.len(),
        1,
        "a silhouette bundle emits exactly one entry per (view, color mode) group; \
         got {images:#?}"
    );
    images[0].clone()
}

// ───────────────────────────────── AC-1 ─────────────────────────────────────

/// AC-1: the manifest entry a gcode-source silhouette produces, field for
/// field — including the two keys that must be ABSENT rather than null.
#[test]
fn gcode_silhouette_bundle_entry_shape() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "two-layer.gcode", TWO_LAYER_GCODE);
    let output = tmp.path().join("bundle");
    let req = silhouette_bundle_request(
        gcode,
        vec![LayerSelector::Index(0), LayerSelector::Index(1)],
        vec![VisualizationSpec::Name("silhouette".to_string())],
    );

    let manifest_path = run_visual_debug(req, &output, false)
        .expect("a 1.2.0 gcode-source silhouette must render end to end");
    let manifest = read_manifest(&manifest_path);

    assert_eq!(
        image_file_names(&manifest_path),
        vec!["gcode_silhouette_front.png".to_string()],
        "exactly one silhouette PNG, named for the source and the resolved view"
    );

    let entry = sole_image_entry(&manifest);
    assert_eq!(entry["source"], serde_json::json!("gcode"));
    assert_eq!(
        entry["tap"],
        serde_json::json!(""),
        "the standalone gcode bundle's empty-tap convention, never a pseudo-tap"
    );
    assert_eq!(entry["visualization"], serde_json::json!("silhouette"));
    assert_eq!(entry["view"], serde_json::json!("front"));
    assert_eq!(
        entry["png_path"],
        serde_json::json!("images/gcode_silhouette_front.png")
    );
    assert_eq!(
        entry["layers_rendered"],
        serde_json::json!([{ "start": 0, "end": 1 }]),
        "both selected layers derived a slab, so they compress to one inclusive range"
    );
    assert!(
        entry["gcode_parser_version"].as_str().is_some(),
        "a gcode-source entry must carry the parser version, got {:?}",
        entry["gcode_parser_version"]
    );
    assert!(
        entry["world_bounds_mm"].is_object(),
        "the shared mm viewport must be recorded, got {:?}",
        entry["world_bounds_mm"]
    );
    assert!(
        entry["typed_capture"].is_null(),
        "a gcode source produces no typed capture, got {:?}",
        entry["typed_capture"]
    );

    // Key ABSENCE, not null-ness: a composite spans many layers, so neither
    // key has a truthful value and both must be omitted from the JSON.
    let object = entry.as_object().expect("entry object");
    assert!(
        !object.contains_key("layer_index"),
        "layer_index must be ABSENT on a silhouette entry, found {:?}",
        object.get("layer_index")
    );
    assert!(
        !object.contains_key("layer_z"),
        "layer_z must be ABSENT on a silhouette entry, found {:?}",
        object.get("layer_z")
    );
}

/// Duplicate silhouette specs collapse: two silhouette specs asking for the
/// same (view, color mode) group must still yield ONE image and ONE entry.
#[test]
fn duplicate_silhouette_specs_collapse_to_one_image() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "two-layer.gcode", TWO_LAYER_GCODE);
    let output = tmp.path().join("bundle");
    let req = silhouette_bundle_request(
        gcode,
        vec![LayerSelector::Index(0), LayerSelector::Index(1)],
        vec![
            VisualizationSpec::Name("silhouette".to_string()),
            detail_viz("silhouette", serde_json::json!({ "color_by": "role" })),
        ],
    );

    let manifest_path = run_visual_debug(req, &output, false).expect("render must succeed");
    let manifest = read_manifest(&manifest_path);
    assert_eq!(
        image_file_names(&manifest_path),
        vec!["gcode_silhouette_front.png".to_string()]
    );
    let _ = sole_image_entry(&manifest);
}

// ───────────────────────────────── AC-6 ─────────────────────────────────────

/// AC-6 (bundle level): a repeated Z, a decreasing Z, and a missing `;Z:`
/// marker each cost their layer its slab. The bundle must say so by layer
/// index, paint none of those layers, and exclude them from
/// `layers_rendered`.
#[test]
fn w3_nonmonotonic_duplicate_and_markerless_layers_skip_with_warning() {
    let gcode_text = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
G1 X30 Y10 F3000
G1 X40 Y10 E0.4
;LAYER_CHANGE
;Z:0.1
;TYPE:Perimeter
G1 X50 Y10 F3000
G1 X60 Y10 E0.4
;LAYER_CHANGE
;TYPE:Perimeter
G1 X70 Y10 F3000
G1 X80 Y10 E0.4
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "w3.gcode", gcode_text);
    let output = tmp.path().join("bundle");
    let req = silhouette_bundle_request(
        gcode,
        vec![LayerSelector::Range { start: 0, end: 3 }],
        vec![VisualizationSpec::Name("silhouette".to_string())],
    );

    let manifest_path = run_visual_debug(req, &output, false).expect("render must succeed");
    let manifest = read_manifest(&manifest_path);
    let entry = sole_image_entry(&manifest);

    let warnings: Vec<String> = entry["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .map(|w| w.as_str().expect("warning string").to_string())
        .collect();

    // Layer 1: repeats layer 0's Z. Names the layer and the repeated value.
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("layer 1") && w.contains("0.2")),
        "expected a W3 warning naming layer 1 and its repeated Z; got {warnings:?}"
    );
    // Layer 2: decreases. Names the layer, its Z, and the previous accepted Z.
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("layer 2") && w.contains("0.1") && w.contains("0.2")),
        "expected a W3 warning naming layer 2, its Z and the previous accepted Z; \
         got {warnings:?}"
    );
    // Layer 3: no marker at all. Names the layer and the marker's absence.
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("layer 3") && w.contains("no ;Z: marker")),
        "expected a W3 warning naming layer 3 and the absent marker; got {warnings:?}"
    );

    assert_eq!(
        entry["layers_rendered"],
        serde_json::json!([{ "start": 0, "end": 0 }]),
        "only layer 0 derived a slab, so only it may appear in layers_rendered"
    );

    // The three skipped layers all sit at X >= 30; layer 0 spans X 10..20
    // plus a sub-millimeter bead. Nothing may be painted out there.
    let png = bundle_png(&manifest_path, "gcode_silhouette_front.png");
    let bounds = entry_bounds(&entry);
    let max_h = max_painted_h_mm(&png, bounds).expect("layer 0 must paint something");
    assert!(
        max_h < 25.0,
        "the W3-skipped layers (X 30..80) must contribute no pixels; rightmost paint at \
         {max_h} mm"
    );
}

// ───────────────────────────────── AC-9 ─────────────────────────────────────

/// AC-9: framing is whole-file, so a layer-subset request and an all-layers
/// request over the same fixture frame identically.
#[test]
fn gcode_silhouette_framing_is_selection_independent() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "two-layer.gcode", TWO_LAYER_GCODE);

    let subset_out = tmp.path().join("subset");
    let subset = run_visual_debug(
        silhouette_bundle_request(
            gcode.clone(),
            vec![LayerSelector::Index(0)],
            vec![VisualizationSpec::Name("silhouette".to_string())],
        ),
        &subset_out,
        false,
    )
    .expect("subset render must succeed");

    let all_out = tmp.path().join("all");
    let all = run_visual_debug(
        silhouette_bundle_request(
            gcode,
            vec![LayerSelector::Range { start: 0, end: 1 }],
            vec![VisualizationSpec::Name("silhouette".to_string())],
        ),
        &all_out,
        false,
    )
    .expect("all-layers render must succeed");

    let subset_entry = sole_image_entry(&read_manifest(&subset));
    let all_entry = sole_image_entry(&read_manifest(&all));

    assert_eq!(
        subset_entry["world_bounds_mm"], all_entry["world_bounds_mm"],
        "framing must not move with the selection"
    );
    // Sanity: the two runs really did select different layer sets.
    assert_ne!(
        subset_entry["layers_rendered"], all_entry["layers_rendered"],
        "fixture error: both requests resolved the same layers"
    );
}

// ───────────────────────────────── AC-8 ─────────────────────────────────────

/// AC-8 (bundle level): two full bundle renders of the same request produce
/// identical PNG bytes, identical manifests, and element-for-element equal
/// warning lists.
#[test]
fn gcode_silhouette_bundle_is_deterministic() {
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "two-layer.gcode", TWO_LAYER_GCODE);

    let run = |dir: PathBuf| {
        let path = run_visual_debug(
            silhouette_bundle_request(
                gcode.clone(),
                vec![LayerSelector::Range { start: 0, end: 1 }],
                vec![VisualizationSpec::Name("silhouette".to_string())],
            ),
            &dir,
            false,
        )
        .expect("render must succeed");
        let raw = fs::read(&path).expect("read manifest.json");
        let png = bundle_png(&path, "gcode_silhouette_front.png");
        let warnings: Vec<String> = read_manifest(&path)["images"][0]["warnings"]
            .as_array()
            .expect("warnings array")
            .iter()
            .map(|w| w.as_str().expect("warning string").to_string())
            .collect();
        (raw, png, warnings)
    };

    let (raw_a, png_a, warn_a) = run(tmp.path().join("a"));
    let (raw_b, png_b, warn_b) = run(tmp.path().join("b"));

    assert_eq!(png_a, png_b, "PNG bytes must be identical across two runs");
    assert_eq!(
        raw_a, raw_b,
        "manifest.json bytes must be identical across two runs"
    );
    assert_eq!(warn_a.len(), warn_b.len(), "warning count");
    for (i, (x, y)) in warn_a.iter().zip(warn_b.iter()).enumerate() {
        assert_eq!(x, y, "warning {i} must be identical across runs");
    }
}

// ──────────────────────────────── AC-10 ─────────────────────────────────────

/// AC-10 (bundle half): `color_by: "tool"` on a gcode silhouette renders to
/// the `_tool`-suffixed filename, paints from the fixed palette in ascending
/// tool order, and records `tool_color_source: "palette"` plus the manifest's
/// `tool_palette` table — a standalone `.gcode` resolves no config, so the
/// palette is the only color source available.
#[test]
fn gcode_silhouette_tool_coloring_palette_only() {
    let gcode_text = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X30 Y10 E1.0
T1
G1 X20 Y10 F3000
G1 X30 Y10 E0.5
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "tools.gcode", gcode_text);
    let output = tmp.path().join("bundle");
    let mut req = silhouette_bundle_request(
        gcode,
        vec![LayerSelector::Index(0)],
        vec![detail_viz(
            "silhouette",
            serde_json::json!({ "color_by": "tool" }),
        )],
    );
    // No `filament_diameter` comment in this fixture: the explicit fallback
    // width is what makes every bead derivable.
    req.gcode_line_width_mm = Some(0.4);

    let manifest_path = run_visual_debug(req, &output, false).expect("render must succeed");
    let manifest = read_manifest(&manifest_path);

    assert_eq!(
        image_file_names(&manifest_path),
        vec!["gcode_silhouette_front_tool.png".to_string()],
        "a tool-colored silhouette gets the _tool-suffixed filename"
    );

    let entry = sole_image_entry(&manifest);
    assert_eq!(entry["color_by"], serde_json::json!("tool"));
    assert_eq!(entry["tool_color_source"], serde_json::json!("palette"));
    assert_eq!(
        entry["png_path"],
        serde_json::json!("images/gcode_silhouette_front_tool.png")
    );

    let palette_table = manifest["tool_palette"]
        .as_array()
        .expect("the manifest must carry the per-tool color table");
    assert!(
        !palette_table.is_empty(),
        "the tool_palette table must not be empty"
    );

    let png = bundle_png(&manifest_path, "gcode_silhouette_front_tool.png");
    let bounds = entry_bounds(&entry);
    let palette = ToolColors::default();
    let t0 = palette.color(0);
    let t1 = palette.color(1);
    assert_ne!(t0, t1, "fixture needs two distinct palette entries");
    assert_eq!(
        pixel_at_mm_in(&png, bounds, 15.0, 0.1),
        t0,
        "tool 0 must paint where nothing overlaps it"
    );
    assert_eq!(
        pixel_at_mm_in(&png, bounds, 25.0, 0.1),
        t1,
        "tool 1 must occlude tool 0 in the overlap (ascending tool paint order)"
    );
}

// ──────────────────── AC-N1 / AC-N2 at the command level ────────────────────

/// The bundle directory must hold nothing after a failed run — a rejected
/// request may never leave a half-written bundle a consumer could mistake
/// for a result.
fn assert_no_bundle_written(output: &Path) {
    if !output.exists() {
        return;
    }
    let entries: Vec<_> = fs::read_dir(output)
        .expect("read output dir")
        .map(|e| e.expect("dir entry").path())
        .collect();
    assert!(
        entries.is_empty(),
        "a failed silhouette run must leave no bundle content; found {entries:?}"
    );
}

/// AC-N1 (command level): no `filament_diameter` and no
/// `gcode_line_width_mm` fails the whole command with
/// `SilhouetteWidthUnderivable`, and writes no bundle.
#[test]
fn width_underivable_without_diameter_fails_command_and_writes_no_bundle() {
    let gcode_text = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.5
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "nodiameter.gcode", gcode_text);
    let output = tmp.path().join("bundle");
    let req = silhouette_bundle_request(
        gcode,
        vec![LayerSelector::Index(0)],
        vec![VisualizationSpec::Name("silhouette".to_string())],
    );

    let err = run_visual_debug(req, &output, false)
        .expect_err("an underivable width with no fallback must fail the command");
    match &err {
        VisualDebugError::SilhouetteWidthUnderivable(message) => {
            assert!(
                message.contains("filament_diameter"),
                "the error must name the missing datum, got {message}"
            );
            assert!(
                message.contains("gcode_line_width_mm"),
                "the error must name the remedy, got {message}"
            );
        }
        other => panic!("expected SilhouetteWidthUnderivable, got {other:?}"),
    }
    assert_no_bundle_written(&output);
}

/// AC-N2 (command level): `M200` poisons flow derivation, so the command
/// fails closed naming `M200` and writes no bundle; supplying the explicit
/// width the error names makes the same request render.
#[test]
fn m200_volumetric_fails_command_and_writes_no_bundle() {
    let gcode_text = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
M200 D1.75
G1 X10 Y10 F3000
G1 X20 Y10 E0.5
";
    let tmp = TempDir::new().expect("tempdir");
    let gcode = write_fixture(tmp.path(), "m200.gcode", gcode_text);
    let output = tmp.path().join("bundle");
    let req = silhouette_bundle_request(
        gcode.clone(),
        vec![LayerSelector::Index(0)],
        vec![VisualizationSpec::Name("silhouette".to_string())],
    );

    let err = run_visual_debug(req, &output, false)
        .expect_err("M200 with no fallback width must fail the command");
    match &err {
        VisualDebugError::SilhouetteWidthUnderivable(message) => {
            assert!(
                message.contains("M200"),
                "the error must name M200 as the poisoning construct, got {message}"
            );
            assert!(
                message.contains("gcode_line_width_mm"),
                "the error must name the remedy, got {message}"
            );
        }
        other => panic!("expected SilhouetteWidthUnderivable, got {other:?}"),
    }
    assert_no_bundle_written(&output);

    // The same request renders once the remedy the error named is applied.
    let recovered = tmp.path().join("recovered");
    let mut req = silhouette_bundle_request(
        gcode,
        vec![LayerSelector::Index(0)],
        vec![VisualizationSpec::Name("silhouette".to_string())],
    );
    req.gcode_line_width_mm = Some(0.42);
    let manifest_path =
        run_visual_debug(req, &recovered, false).expect("an explicit width must make it render");
    assert_eq!(
        image_file_names(&manifest_path),
        vec!["gcode_silhouette_front.png".to_string()]
    );
}

// ───────────────── packet 248 DOGFOOD findings 1-4 regressions ─────────────
//
// Four defects found by running the real `pnp_cli visual-debug` CLI against
// real `.gcode` files after the packet shipped. All four fixes are additive
// (warnings and error text only) — none may alter framing, geometry, or
// pixel output.

/// Finding 1: a silhouette whose widths ALL came from the caller's
/// `gcode_line_width_mm` used to render with `warnings: []`, looking exactly
/// as authoritative as a flow-derived one. Measured on a real OrcaSlicer
/// file that carries no `; filament_diameter` comment.
#[test]
fn fallback_width_use_is_warned() {
    // No `filament_diameter` comment anywhere: every width must fall back.
    let no_diameter = "\
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
G1 X20 Y20 E0.4
";
    let out = render_gcode_silhouette(
        no_diameter,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.42),
        ColorBy::Role,
    )
    .expect("an explicit fallback width must still render");

    let fallback: Vec<&String> = out
        .warnings
        .iter()
        .filter(|w| w.contains("gcode_line_width_mm fallback"))
        .collect();
    assert_eq!(
        fallback.len(),
        1,
        "exactly ONE bundle-level fallback warning, not one per segment; got {:?}",
        out.warnings
    );
    let w = fallback[0];
    assert!(
        w.contains("2 of 2"),
        "the warning must state how many of how many rendered extruding moves used the \
         fallback; got {w}"
    );
    assert!(
        w.contains("0.42"),
        "the warning must state the fallback width it used; got {w}"
    );
    assert!(
        w.contains("filament_diameter"),
        "the warning must name the cause (no filament_diameter comment); got {w}"
    );
    assert!(
        w.contains("NOT derived from the file"),
        "the warning must state the consequence; got {w}"
    );

    // Control: the SAME geometry with a derivable diameter must carry no
    // such warning even though a fallback was offered.
    let with_diameter = format!("; filament_diameter = 1.75\n{no_diameter}");
    let control = render_gcode_silhouette(
        &with_diameter,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.42),
        ColorBy::Role,
    )
    .expect("the control fixture must render");
    assert!(
        !control
            .warnings
            .iter()
            .any(|w| w.contains("gcode_line_width_mm fallback")),
        "a flow-derivable source must NOT warn about the fallback; got {:?}",
        control.warnings
    );

    // The other modelled cause must be named distinctly.
    let volumetric = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
M200 D1.75
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
";
    let out = render_gcode_silhouette(
        volumetric,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        Some(0.42),
        ColorBy::Role,
    )
    .expect("an explicit fallback width must still render past M200");
    assert!(
        out.warnings
            .iter()
            .any(|w| w.contains("gcode_line_width_mm fallback") && w.contains("M200")),
        "the M200 cause must be distinguished from the missing-comment cause; got {:?}",
        out.warnings
    );
}

/// Finding 2: measured on a real 13,662-line source, the bundle carried 234
/// warnings — all 234 of them `M73` progress commands — which would bury a
/// genuine W3 skip-with-warning entirely.
#[test]
fn repeated_unsupported_constructs_collapse() {
    let mut gcode =
        String::from("; filament_diameter = 1.75\n;LAYER_CHANGE\n;Z:0.2\n;TYPE:Perimeter\nM83\n");
    // Five `M73` progress commands (unsupported), interleaved with motion.
    for i in 0..5 {
        gcode.push_str(&format!("M73 P{i} R10\n"));
        gcode.push_str(&format!("G1 X{} Y10 E0.4 F3000\n", 10 + i));
    }
    let parsed = parse_gcode(&gcode);

    let m73: Vec<&String> = parsed
        .warnings
        .iter()
        .filter(|w| w.contains("M73"))
        .collect();
    assert_eq!(
        m73.len(),
        2,
        "five M73 lines must collapse to the first occurrence plus ONE summary, \
         not five warnings; got {:?}",
        parsed.warnings
    );
    assert!(
        m73[0].starts_with("line 6:")
            && m73[0].contains("unsupported G-code construct outside the documented"),
        "the first occurrence must keep its existing line-numbered text; got {}",
        m73[0]
    );
    let summary = m73[1];
    assert!(
        summary.contains("unsupported G-code construct outside the documented"),
        "the summary must keep the greppable prefix; got {summary}"
    );
    assert!(
        summary.contains("first at line 6"),
        "the summary must name the first occurrence's line; got {summary}"
    );
    assert!(
        summary.contains("5 occurrences total") && summary.contains("4 suppressed"),
        "the summary must preserve the true total and the suppressed count; got {summary}"
    );

    // A construct seen exactly once still emits exactly one warning and no
    // redundant summary.
    let once = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G2 X10 Y0 I5 J0 E1.0
G1 X10 Y10 F3000
G1 X20 Y10 E0.4
";
    let parsed = parse_gcode(once);
    assert_eq!(
        parsed.warnings.iter().filter(|w| w.contains("G2")).count(),
        1,
        "a single occurrence must not gain a summary; got {:?}",
        parsed.warnings
    );
}

/// Finding 3: a 239 mm wide x 0.8 mm tall source rendered as a ~1-pixel line
/// on a 2048x2048 canvas. The image is correct; the fix is to SAY it is
/// unreadable, never to distort the axes or change the framing.
#[test]
fn extreme_aspect_ratio_is_warned() {
    let wide_and_short = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X0 Y10 F3000
G1 X240 Y10 E9.0
";
    let out = render_gcode_silhouette(
        wide_and_short,
        &[0],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        None,
        ColorBy::Role,
    )
    .expect("the wide fixture must still render");
    let flat: Vec<&String> = out
        .warnings
        .iter()
        .filter(|w| w.contains("extremely flat"))
        .collect();
    assert_eq!(
        flat.len(),
        1,
        "exactly one aspect-ratio warning; got {:?}",
        out.warnings
    );
    assert!(
        flat[0].contains("side") && flat[0].contains("fewer layers"),
        "the warning must suggest the actionable remedies; got {}",
        flat[0]
    );

    // Control: normally-proportioned geometry must not warn.
    let normal = "\
; filament_diameter = 1.75
;LAYER_CHANGE
;Z:0.2
;TYPE:Perimeter
M83
G1 X0 Y10 F3000
G1 X10 Y10 E0.4
;LAYER_CHANGE
;Z:0.4
;TYPE:Perimeter
G1 X0 Y10 F3000
G1 X10 Y10 E0.4
;LAYER_CHANGE
;Z:0.6
;TYPE:Perimeter
G1 X0 Y10 F3000
G1 X10 Y10 E0.4
";
    let out = render_gcode_silhouette(
        normal,
        &[0, 1, 2],
        SilhouetteView::Front,
        CANVAS,
        CANVAS,
        None,
        ColorBy::Role,
    )
    .expect("the control fixture must render");
    assert!(
        !out.warnings.iter().any(|w| w.contains("extremely flat")),
        "normally-proportioned geometry must NOT warn; got {:?}",
        out.warnings
    );
}

/// Finding 4: `source` is tagged by `kind` while `visualizations` entries are
/// tagged by `type`. The bare serde failure named neither the expected values
/// nor the sibling key that spells the same idea differently. The wire format
/// is unchanged; only the error text is.
#[test]
fn request_source_tag_error_names_kind_and_type() {
    let tmp = TempDir::new().expect("tempdir");
    let req_path = tmp.path().join("request.json");
    fs::write(
        &req_path,
        br#"{
  "schema_version": "1.2.0",
  "source": {"type": "gcode", "path": "nope.gcode"},
  "layers": [0],
  "visualizations": [{"type": "silhouette"}]
}"#,
    )
    .expect("write request");

    let err = pnp_cli::visual_debug::run_cli(&req_path, &tmp.path().join("bundle"), false)
        .expect_err("a `type`-tagged source must fail");
    let message = err.to_string();
    assert!(
        message.contains("\"kind\": \"model\"") && message.contains("\"kind\": \"gcode\""),
        "the error must name the expected source.kind values; got {message}"
    );
    assert!(
        message.contains("visualizations") && message.contains("`type`"),
        "the error must note that visualizations entries use a different key; got {message}"
    );
}
