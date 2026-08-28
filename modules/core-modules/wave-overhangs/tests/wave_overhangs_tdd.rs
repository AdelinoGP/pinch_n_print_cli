#![allow(missing_docs)]
//! Packet 246 - wave-overhang bridge fill.
//!
//! Step 2 added `from_config` smoke coverage. Step 3 adds the acceptance
//! coverage for the ported generator and the region pipeline:
//!
//! - AC-4 `waves_emitted_anchor_first_order_locked`
//! - AC-5 `internal_bridge_areas_excluded_from_waves`
//! - AC-6 `fallback_rectilinear_no_silent_drop`
//! - AC-7 `speed_and_flow_factors_resolved`
//! - AC-9 `deterministic_double_run`

use slicer_ir::{ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;
use wave_overhangs::{WaveOverhangs, WavePattern};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

const NOZZLE_MM: f32 = 0.4;
const LAYER_HEIGHT_MM: f32 = 0.2;
const BRIDGE_SPEED: f32 = 25.0;
const WAVE_SPEED: f32 = 2.0;
const WAVE_FLOW_MM3_PER_MM: f32 = 0.15;

fn paint_view() -> PaintRegionLayerView {
    PaintRegionLayerView::new(0)
}

/// Axis-aligned rectangle in millimetres.
fn rect_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

/// Rectangular frame (`outer` minus `inner`) as a single hole-bearing polygon.
fn frame_mm(o0: f32, o1: f32, i0: f32, i1: f32) -> ExPolygon {
    ExPolygon {
        contour: rect_mm(o0, o0, o1, o1).contour,
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(i0, i0),
                Point2::from_mm(i0, i1),
                Point2::from_mm(i1, i1),
                Point2::from_mm(i1, i0),
            ],
        }],
    }
}

/// Module config with an explicit anchor depth deep enough that the anchor band
/// exceeds the canonical `anchors_size`, which is what lets the generator find
/// inset anchors (and therefore seeds) on this fixture.
fn wave_config(anchor_depth_mm: f32) -> slicer_ir::ConfigView {
    ConfigViewBuilder::new()
        .float("nozzle_diameter", f64::from(NOZZLE_MM))
        .float("layer_height", f64::from(LAYER_HEIGHT_MM))
        .float("bridge_speed", f64::from(BRIDGE_SPEED))
        .float("wave_overhang_print_speed", f64::from(WAVE_SPEED))
        .float(
            "wave_overhang_flow_mm3_per_mm",
            f64::from(WAVE_FLOW_MM3_PER_MM),
        )
        .float("wave_overhang_anchor_depth_mm", f64::from(anchor_depth_mm))
        .int("wall_count", 3)
        .build()
}

/// Region view carrying the `claim:bridge-fill` claim.
///
/// `should_emit` returns false for every fill role when `held_claims` is empty,
/// so this must be set or the module silently emits nothing.
fn bridge_region(
    bridge_areas: Vec<ExPolygon>,
    internal_bridge_areas: Vec<ExPolygon>,
    supported: Vec<ExPolygon>,
) -> SliceRegionView {
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj")
        .region_id(1)
        .z(LAYER_HEIGHT_MM)
        .effective_layer_height(LAYER_HEIGHT_MM)
        .is_bridge(true)
        .bridge_orientation_deg(0.0)
        .bridge_areas(bridge_areas)
        .internal_bridge_areas(internal_bridge_areas)
        .bottom_solid_fill(supported.clone())
        .previous_layer_boundary(supported)
        .build();
    region.set_held_claims(vec!["claim:bridge-fill".to_string()]);
    region
}

/// The canonical happy-path fixture: a 10x10 mm unsupported square surrounded
/// by a wide band of supported bottom-solid fill.
fn supported_square_fixture() -> SliceRegionView {
    bridge_region(
        vec![rect_mm(0.0, 0.0, 10.0, 10.0)],
        Vec::new(),
        vec![frame_mm(-6.0, 16.0, 0.0, 10.0)],
    )
}

fn run(module: &WaveOverhangs, regions: &[SliceRegionView]) -> InfillOutputBuilder {
    let mut output = InfillOutputBuilder::new();
    let config = wave_config(3.0);
    module
        .run_infill(1, regions, &paint_view(), &mut output, &config)
        .expect("run_infill must succeed");
    output
}

/// Same as [`wave_config`] but omits `wave_overhang_anchor_depth_mm` entirely,
/// so the module resolves the automatic anchor depth.
fn wave_config_auto_anchor() -> slicer_ir::ConfigView {
    ConfigViewBuilder::new()
        .float("nozzle_diameter", f64::from(NOZZLE_MM))
        .float("layer_height", f64::from(LAYER_HEIGHT_MM))
        .float("bridge_speed", f64::from(BRIDGE_SPEED))
        .float("wave_overhang_print_speed", f64::from(WAVE_SPEED))
        .float(
            "wave_overhang_flow_mm3_per_mm",
            f64::from(WAVE_FLOW_MM3_PER_MM),
        )
        .int("wall_count", 3)
        .build()
}

fn locked(paths: &[ExtrusionPath3D]) -> Vec<&ExtrusionPath3D> {
    paths.iter().filter(|p| p.order_lock.is_some()).collect()
}

fn unlocked(paths: &[ExtrusionPath3D]) -> Vec<&ExtrusionPath3D> {
    paths.iter().filter(|p| p.order_lock.is_none()).collect()
}

/// Distance in millimetres from the point `(px, py)` to the segment `a`-`b`.
fn point_to_segment_mm(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq <= f32::EPSILON {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0)
    };
    let (fx, fy) = (ax + t * dx, ay + t * dy);
    ((px - fx).powi(2) + (py - fy).powi(2)).sqrt()
}

/// Minimum geometric distance in millimetres between two paths.
///
/// This is a point-to-**segment** minimum in both directions, not a
/// vertex-to-vertex minimum. Wave fronts are Douglas-Peucker-simplified
/// polylines, so successive fronts that are exactly one wave spacing apart can
/// still have their *vertices* farther apart than that spacing wherever a
/// vertex was dropped or a round join re-sampled the contour. Comparing
/// vertices would therefore report a gap larger than the real front-to-front
/// distance and force a laxer bound than AC-4 specifies.
fn min_path_distance_mm(a: &ExtrusionPath3D, b: &ExtrusionPath3D) -> f32 {
    let mut best = f32::MAX;
    for (p, q) in [(a, b), (b, a)] {
        for pt in &p.points {
            for w in q.points.windows(2) {
                let d = point_to_segment_mm(pt.x, pt.y, w[0].x, w[0].y, w[1].x, w[1].y);
                if d < best {
                    best = d;
                }
            }
        }
    }
    best
}

/// One scaled coordinate unit expressed in millimetres (`slicer_ir::UNITS_PER_MM`
/// is 10 000). Used only to absorb f32 round-off on a bound that is met exactly;
/// it is never a slack allowance.
const UNIT_TOLERANCE_MM: f32 = 1.0e-4;

/// Strict interior containment in a rectangle, in millimetres.
fn point_in_rect_mm(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    x > x0 && x < x1 && y > y0 && y < y1
}

/// Does the segment `a`-`b` share any positive-length span with the OPEN
/// rectangle `(x0,y0)-(x1,y1)`? Liang-Barsky parametric clip.
///
/// Stricter than a per-vertex containment test: a wave front may straddle a
/// forbidden area with both endpoints outside it. No tolerance is applied —
/// AC-N2 requires the locked-vs-internal check to be exact.
fn segment_enters_rect_mm(
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> bool {
    let (dx, dy) = (bx - ax, by - ay);
    if dx == 0.0 && dy == 0.0 {
        return point_in_rect_mm(ax, ay, x0, y0, x1, y1);
    }
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-dx, ax - x0),
        (dx, x1 - ax),
        (-dy, ay - y0),
        (dy, y1 - ay),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            if r > t1 {
                return false;
            }
            if r > t0 {
                t0 = r;
            }
        } else {
            if r < t0 {
                return false;
            }
            if r < t1 {
                t1 = r;
            }
        }
    }
    t1 > t0
}

/// Does any segment of `path` enter the open rectangle?
fn path_enters_rect_mm(
    path: &ExtrusionPath3D,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> bool {
    path.points
        .windows(2)
        .any(|w| segment_enters_rect_mm(w[0].x, w[0].y, w[1].x, w[1].y, x0, y0, x1, y1))
        || path
            .points
            .iter()
            .any(|p| point_in_rect_mm(p.x, p.y, x0, y0, x1, y1))
}

/// Config with an explicit wave print speed, used to drive the speed-factor
/// clamp from both sides.
fn speed_config(wave_speed: f32) -> slicer_ir::ConfigView {
    ConfigViewBuilder::new()
        .float("nozzle_diameter", f64::from(NOZZLE_MM))
        .float("layer_height", f64::from(LAYER_HEIGHT_MM))
        .float("bridge_speed", f64::from(BRIDGE_SPEED))
        .float("wave_overhang_print_speed", f64::from(wave_speed))
        .float(
            "wave_overhang_flow_mm3_per_mm",
            f64::from(WAVE_FLOW_MM3_PER_MM),
        )
        .float("wave_overhang_anchor_depth_mm", 3.0)
        .int("wall_count", 3)
        .build()
}

/// Run the module with an explicit config, returning the raw result.
fn run_with(
    config: &slicer_ir::ConfigView,
    region: &SliceRegionView,
) -> Result<InfillOutputBuilder, slicer_sdk::error::ModuleError> {
    let module = WaveOverhangs::from_config(config).expect("config");
    let mut output = InfillOutputBuilder::new();
    module.run_infill(
        1,
        std::slice::from_ref(region),
        &paint_view(),
        &mut output,
        config,
    )?;
    Ok(output)
}

/// Clip-boundary tolerance, in millimetres.
///
/// `slicer_core::polygon_ops::clip_polylines` documents that reported boundary
/// coordinates land within +/-2 scaled units (2e-4 mm) of the exact boundary.
const CLIP_TOLERANCE_MM: f32 = 0.001;

/// Closed containment in a rectangle, in millimetres, within the clip tolerance.
///
/// Rectilinear scanlines are clipped to the fill boundary, so both endpoints of
/// every scanline sit essentially *on* the rectangle edge and the strict test
/// above would reject them.
fn point_on_or_in_rect_mm(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    x >= x0 - CLIP_TOLERANCE_MM
        && x <= x1 + CLIP_TOLERANCE_MM
        && y >= y0 - CLIP_TOLERANCE_MM
        && y <= y1 + CLIP_TOLERANCE_MM
}

// ---------------------------------------------------------------------------
// Step 2 smoke coverage (kept green)
// ---------------------------------------------------------------------------

#[test]
fn from_config_resolves_manifest_defaults() {
    let config = ConfigViewBuilder::new().build();
    let module = WaveOverhangs::from_config(&config).expect("defaults must resolve");

    assert_eq!(module.pattern(), WavePattern::Smart);
    assert!((module.line_spacing() - 0.35).abs() < 1e-6);
    assert!((module.print_speed() - 2.0).abs() < 1e-6);
    assert_eq!(module.max_iterations(), 0);
}

#[test]
fn from_config_reads_explicit_overrides() {
    let config = ConfigViewBuilder::new()
        .string("wave_overhang_pattern", "zigzag")
        .float("wave_overhang_line_spacing", 0.5)
        .int("wave_overhang_max_iterations", 12)
        .build();
    let module = WaveOverhangs::from_config(&config).expect("overrides must resolve");

    assert_eq!(module.pattern(), WavePattern::Zigzag);
    assert!((module.line_spacing() - 0.5).abs() < 1e-6);
    assert_eq!(module.max_iterations(), 12);
}

// ---------------------------------------------------------------------------
// AC-4
// ---------------------------------------------------------------------------

#[test]
fn waves_emitted_anchor_first_order_locked() {
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let region = supported_square_fixture();
    let output = run(&module, std::slice::from_ref(&region));

    let paths = output.solid_paths();
    assert!(!paths.is_empty(), "wave fill must emit paths");

    let wave_paths = locked(paths);
    assert!(
        wave_paths.len() > 1,
        "expected a multi-front wave, got {}",
        wave_paths.len()
    );

    // Role.
    for path in &wave_paths {
        assert_eq!(path.role, ExtrusionRole::BridgeInfill);
    }

    // One order-lock tag per connected wave domain: this fixture has exactly
    // one external bridge component, so exactly one tag.
    let mut tags: Vec<u64> = wave_paths
        .iter()
        .map(|p| p.order_lock.expect("locked"))
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 1, "one tag per connected wave domain");
    assert_ne!(tags[0], 0, "tag 0 is invalid");

    // Anchor-first: the first front must touch the supported material, which on
    // this fixture is everything outside the 0..10 mm square.
    let first = wave_paths[0];
    assert!(
        first
            .points
            .iter()
            .any(|p| !point_in_rect_mm(p.x, p.y, 0.0, 0.0, 10.0, 10.0)),
        "first wave front must start on supported material"
    );

    // Each subsequent front is within ONE wavelength of a predecessor (AC-4).
    //
    // The propagation loop offsets the accumulated region by exactly
    // `wave_spacing_mm` (== `wave_overhang_line_spacing`) per iteration, so the
    // bound is met *exactly*: the measured worst case on this fixture is
    // 0.35000038 mm against a 0.35 mm wavelength. The one-scaled-unit tolerance
    // absorbs that f32 round-off and nothing more.
    let wavelength = 0.35_f32; // manifest default wave_overhang_line_spacing
    for i in 1..wave_paths.len() {
        let nearest = (0..i)
            .map(|j| min_path_distance_mm(wave_paths[i], wave_paths[j]))
            .fold(f32::MAX, f32::min);
        assert!(
            nearest <= wavelength + UNIT_TOLERANCE_MM,
            "front {i} is {nearest} mm from its nearest predecessor \
             (> one wavelength of {wavelength} mm)"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-5
// ---------------------------------------------------------------------------

#[test]
fn internal_bridge_areas_excluded_from_waves() {
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let internal = rect_mm(3.0, 3.0, 7.0, 7.0);
    let region = bridge_region(
        vec![rect_mm(0.0, 0.0, 10.0, 10.0)],
        vec![internal],
        vec![frame_mm(-6.0, 16.0, 0.0, 10.0)],
    );
    let output = run(&module, std::slice::from_ref(&region));
    let paths = output.solid_paths();

    let wave_paths = locked(paths);
    assert!(!wave_paths.is_empty(), "waves must still be generated");

    // No locked footprint may enter the internal bridge area. STRICT: the
    // Liang-Barsky segment test, not per-vertex containment, so a front that
    // straddles the area with both endpoints outside it is still caught.
    for path in &wave_paths {
        assert!(
            !path_enters_rect_mm(path, 3.0, 3.0, 7.0, 7.0),
            "a locked wave footprint enters the internal bridge area"
        );
    }

    // The internal area is covered by unlocked rectilinear fallback instead.
    let internal_fallback: Vec<_> = unlocked(paths)
        .into_iter()
        .filter(|path| {
            path.points
                .iter()
                .all(|p| point_on_or_in_rect_mm(p.x, p.y, 3.0, 3.0, 7.0, 7.0))
        })
        .collect();
    assert!(
        !internal_fallback.is_empty(),
        "internal bridge area must receive unlocked rectilinear fallback"
    );
    for path in internal_fallback {
        assert!(path.order_lock.is_none(), "internal fallback must be unlocked");
    }
}

// ---------------------------------------------------------------------------
// AC-6
// ---------------------------------------------------------------------------

#[test]
fn fallback_rectilinear_no_silent_drop() {
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    // No previous-layer boundary and no solid fill => nothing to anchor on, so
    // waves are impossible for every component.
    let region = bridge_region(
        vec![rect_mm(0.0, 0.0, 6.0, 6.0), rect_mm(20.0, 0.0, 26.0, 6.0)],
        Vec::new(),
        Vec::new(),
    );
    let output = run(&module, std::slice::from_ref(&region));
    let paths = output.solid_paths();

    assert!(!paths.is_empty(), "fallback must emit conventional bridge fill");
    assert!(
        locked(paths).is_empty(),
        "fallback bridge fill must not be order-locked"
    );

    // Every nonempty external bridge component emits at least one path.
    let components = [(0.0, 0.0, 6.0, 6.0), (20.0, 0.0, 26.0, 6.0)];
    for (x0, y0, x1, y1) in components {
        let covered = paths.iter().any(|path| {
            path.points
                .iter()
                .any(|p| p.x >= x0 - 0.1 && p.x <= x1 + 0.1 && p.y >= y0 - 0.1 && p.y <= y1 + 0.1)
        });
        assert!(
            covered,
            "external bridge component ({x0},{y0})-({x1},{y1}) was silently dropped"
        );
    }

    // Conventional rectilinear scanlines. Clipping a scanline against a
    // non-convex fill boundary can yield more than two points, so the invariant
    // is a lower bound, not an exact count.
    assert!(
        paths.iter().all(|p| p.points.len() >= 2),
        "scanlines must have at least two points"
    );
    for path in paths {
        assert_eq!(path.role, ExtrusionRole::BridgeInfill);
        assert!((path.speed_factor - 1.0).abs() < 1e-6, "fallback speed factor is 1.0");
    }
}

// ---------------------------------------------------------------------------
// AC-7
// ---------------------------------------------------------------------------

#[test]
fn speed_and_flow_factors_resolved() {
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let region = supported_square_fixture();
    let output = run(&module, std::slice::from_ref(&region));

    let wave_paths = locked(output.solid_paths());
    assert!(!wave_paths.is_empty(), "expected wave paths");

    let expected_speed = WAVE_SPEED / BRIDGE_SPEED;
    let expected_flow = WAVE_FLOW_MM3_PER_MM / (NOZZLE_MM * LAYER_HEIGHT_MM);

    for path in &wave_paths {
        assert!(
            (path.speed_factor - expected_speed).abs() < 1e-6,
            "speed_factor {} != {expected_speed}",
            path.speed_factor
        );
        // width and flow_factor are PER-POINT on Point3WithWidth.
        for p in &path.points {
            assert!(
                (p.width - NOZZLE_MM).abs() < 1e-6,
                "bead width {} != nozzle diameter",
                p.width
            );
            assert!(
                (p.flow_factor - expected_flow).abs() < 1e-4,
                "flow_factor {} != {expected_flow}",
                p.flow_factor
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC-N1 (negative) - renamed/extended from the Step-3
// `unrepresentable_speed_factor_is_a_fatal_error` test, which only covered the
// upper bound.
// ---------------------------------------------------------------------------

#[test]
fn speed_factor_out_of_clamp_rejected() {
    let region = supported_square_fixture();

    // --- Above the clamp: 200 / 25 = 8.0 > 5.0. --------------------------
    let too_fast = 200.0_f32;
    let high_factor = too_fast / BRIDGE_SPEED;
    assert!(high_factor > 5.0, "fixture must exceed the clamp");
    let err = run_with(&speed_config(too_fast), &region)
        .expect_err("speed factor above the clamp must be fatal");
    assert!(err.fatal, "must be fatal, never a silent clamp");
    // Assert the whole formatted clause, not the bare factor: `8.0_f32`
    // stringifies to "8", which a message merely mentioning any 8 would
    // satisfy.
    let high_clause = format!(
        "speed factor {high_factor} (wave_overhang_print_speed {too_fast} mm/s / \
         bridge_speed {BRIDGE_SPEED} mm/s)"
    );
    assert!(
        err.message.contains(&high_clause),
        "error must name the unrepresentable speed factor as {high_clause:?}: {}",
        err.message
    );

    // --- Below the clamp: 0.5 / 25 = 0.02 < 0.05. ------------------------
    let too_slow = 0.5_f32;
    let low_factor = too_slow / BRIDGE_SPEED;
    assert!(low_factor < 0.05, "fixture must undershoot the clamp");
    let err = run_with(&speed_config(too_slow), &region)
        .expect_err("speed factor below the clamp must be fatal");
    assert!(err.fatal, "must be fatal, never a silent clamp");
    let low_clause = format!(
        "speed factor {low_factor} (wave_overhang_print_speed {too_slow} mm/s / \
         bridge_speed {BRIDGE_SPEED} mm/s)"
    );
    assert!(
        err.message.contains(&low_clause),
        "error must name the unrepresentable speed factor as {low_clause:?}: {}",
        err.message
    );

    // --- Inside the clamp still succeeds: rejection is selective. --------
    let ok_speed = WAVE_SPEED; // 2 / 25 = 0.08, inside [0.05, 5.0]
    let ok_factor = ok_speed / BRIDGE_SPEED;
    assert!(
        (0.05..=5.0).contains(&ok_factor),
        "control fixture must sit inside the clamp"
    );
    let output = run_with(&speed_config(ok_speed), &region)
        .expect("a representable speed factor must not be rejected");
    let paths = output.solid_paths();
    assert!(
        !paths.is_empty(),
        "control run must emit paths (otherwise the success assertion is vacuous)"
    );
    assert!(
        locked(paths)
            .iter()
            .all(|p| (p.speed_factor - ok_factor).abs() < 1e-6),
        "in-clamp factor must be carried through verbatim"
    );
}

// ---------------------------------------------------------------------------
// AC-N2 (negative)
// ---------------------------------------------------------------------------

#[test]
fn locked_footprint_disjoint_from_internal() {
    // Explicit anchor depth so waves actually engage; with the default (auto)
    // depth `inset_anchors` comes out empty and everything falls back.
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    const IX0: f32 = 3.0;
    const IY0: f32 = 3.0;
    const IX1: f32 = 7.0;
    const IY1: f32 = 7.0;
    let region = bridge_region(
        vec![rect_mm(0.0, 0.0, 10.0, 10.0)],
        vec![rect_mm(IX0, IY0, IX1, IY1)],
        vec![frame_mm(-6.0, 16.0, 0.0, 10.0)],
    );
    let output = run(&module, std::slice::from_ref(&region));
    let paths = output.solid_paths();
    assert!(!paths.is_empty(), "region must emit paths");

    // Guard against a vacuous pass: there must BE a locked footprint.
    let wave_paths = locked(paths);
    assert!(
        !wave_paths.is_empty(),
        "no locked wave footprint was produced; disjointness would be vacuous"
    );

    // STRICT: no clip tolerance. Not one locked segment may enter the internal
    // bridge area.
    for path in &wave_paths {
        assert!(
            !path_enters_rect_mm(path, IX0, IY0, IX1, IY1),
            "a locked wave footprint overlaps the internal bridge area"
        );
    }

    // The internal area still receives its own paths, and they are unlocked.
    // (The host `InternalBridgeInfill` constructor is untouched; from inside the
    // module the observable behaviour is the unlocked fallback.)
    let internal_paths: Vec<_> = unlocked(paths)
        .into_iter()
        .filter(|path| {
            path.points
                .iter()
                .all(|p| point_on_or_in_rect_mm(p.x, p.y, IX0, IY0, IX1, IY1))
        })
        .collect();
    assert!(
        !internal_paths.is_empty(),
        "internal-qualified polygons must still receive their own paths"
    );
    for path in internal_paths {
        assert!(
            path.order_lock.is_none(),
            "internal-area paths must never be order-locked"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-N3 (negative) - fork issue #84 analog: missing and narrow anchors.
// ---------------------------------------------------------------------------

#[test]
fn missing_and_narrow_anchor_no_holes() {
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let components = [(0.0_f32, 0.0_f32, 6.0_f32, 6.0_f32), (20.0, 0.0, 26.0, 6.0)];
    let bridges = || {
        components
            .iter()
            .map(|&(x0, y0, x1, y1)| rect_mm(x0, y0, x1, y1))
            .collect::<Vec<_>>()
    };

    // (a) No supported material at all => no anchors can be found.
    // (b) A supported band far too narrow to survive the anchor inset.
    let cases: [(&str, Vec<ExPolygon>); 2] = [
        ("missing anchor", Vec::new()),
        (
            "narrow anchor band",
            vec![
                frame_mm(-0.02, 6.02, 0.0, 6.0),
                frame_mm(19.98, 26.02, 20.0, 26.0),
            ],
        ),
    ];

    for (label, supported) in cases {
        let region = bridge_region(bridges(), Vec::new(), supported);
        let output = run(&module, std::slice::from_ref(&region));
        let paths = output.solid_paths();
        assert!(
            !paths.is_empty(),
            "{label}: fallback must emit bridge fill, not nothing"
        );

        // AC-N3 names the FALLBACK path, so prove the fallback actually ran.
        // Coverage alone would pass even if waves had engaged, which would
        // leave the case this test exists for unexercised. Waves are always
        // order-locked and always carry the wave speed factor; conventional
        // fallback is unlocked at speed factor 1.0.
        assert!(
            locked(paths).is_empty(),
            "{label}: waves engaged; the fallback path was never exercised"
        );
        for path in paths {
            assert!(
                (path.speed_factor - 1.0).abs() < 1e-6,
                "{label}: fallback speed factor must be 1.0, got {}",
                path.speed_factor
            );
        }

        for (x0, y0, x1, y1) in components {
            let covered = paths.iter().any(|path| {
                path.points.iter().any(|p| {
                    p.x >= x0 - 0.1 && p.x <= x1 + 0.1 && p.y >= y0 - 0.1 && p.y <= y1 + 0.1
                })
            });
            assert!(
                covered,
                "{label}: external bridge component ({x0},{y0})-({x1},{y1}) left a hole"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC-9
// ---------------------------------------------------------------------------

#[test]
fn deterministic_double_run() {
    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let region = supported_square_fixture();

    let first = run(&module, std::slice::from_ref(&region));
    let second = run(&module, std::slice::from_ref(&region));

    assert!(!first.solid_paths().is_empty(), "expected output to compare");
    assert_eq!(
        first.solid_paths(),
        second.solid_paths(),
        "two runs on identical input must be identical"
    );
    assert_eq!(first.sparse_paths(), second.sparse_paths());
}

/// Regression: the automatic anchor depth must be deep enough for waves to
/// engage with no explicit `wave_overhang_anchor_depth_mm`. Before the floor at
/// `anchors_size + base_spacing`, the auto depth equalled the generator's own
/// `anchors_size`, `inset_anchors` came out empty, and every component fell
/// back to conventional rectilinear bridge fill.
#[test]
fn waves_engage_with_default_anchor_depth() {
    let config = wave_config_auto_anchor();
    let module = WaveOverhangs::from_config(&config).expect("config");
    let region = supported_square_fixture();

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(1, std::slice::from_ref(&region), &paint_view(), &mut output, &config)
        .expect("run_infill must succeed");

    let paths = output.solid_paths();
    assert!(!paths.is_empty(), "expected bridge fill output");
    let wave_paths = locked(paths);
    assert!(
        !wave_paths.is_empty(),
        "waves must engage under the default (auto) anchor depth; got {} paths, all fallback",
        paths.len()
    );
    let expected_speed = WAVE_SPEED / BRIDGE_SPEED;
    for path in &wave_paths {
        assert!(
            (path.speed_factor - expected_speed).abs() < 1e-6,
            "wave paths must carry the wave speed factor {expected_speed}, got {}",
            path.speed_factor
        );
    }
}


// ---------------------------------------------------------------------------
// Regression: front-merge seam gap.
//
// Human G-code inspection of `resources/A_upsidedown.obj` found a missing
// bridge line at z30. Root cause: the propagation loop emits each front as the
// *contour* of the growing accumulated region. When two opposing fronts merge,
// the merged region's contour no longer runs through the seam between them, so
// the seam line is never emitted. Whether that shows up as a visible void
// depends on how far apart the two neighbouring full-height fronts ended up --
// which is layer-dependent, so path counts stay stable and every existing test
// stayed green while the defect was live.
//
// This test pins coverage directly: the union of the *swept footprints* (built
// from per-point widths, not centrelines) of the emitted wave paths must leave
// no INTERIOR void wider than half a flow width inside the external bridge
// domain. Voids that touch the domain boundary are harmless slivers -- real
// layers carry ~2 mm^2 of them and print correctly -- so they are excluded by
// construction rather than by weakening the width threshold.
// ---------------------------------------------------------------------------

/// Width of the seam fixture's bridge, in mm. Tuned so the two opposing fronts
/// merge with a wide residual: the pre-fix generator left a 0.29 mm x 4.76 mm
/// interior void here (1.26 mm^2), the same shape and width the human found in
/// the real `A_upsidedown.obj` G-code at z30.
const SEAM_W_MM: f32 = 6.0;
/// Height of the seam fixture's bridge, in mm.
const SEAM_H_MM: f32 = 6.0;

/// A bridge anchored on its LEFT and RIGHT edges only. Wave fronts therefore
/// advance towards each other and merge along a vertical seam at mid-x --
/// exactly the front-merge geometry that drops the seam line.
fn seam_fixture() -> SliceRegionView {
    bridge_region(
        vec![rect_mm(0.0, 0.0, SEAM_W_MM, SEAM_H_MM)],
        Vec::new(),
        vec![
            rect_mm(-6.0, -6.0, 0.0, SEAM_H_MM + 6.0),
            rect_mm(SEAM_W_MM, -6.0, SEAM_W_MM + 6.0, SEAM_H_MM + 6.0),
        ],
    )
}

/// Half the flow width, in mm: the widest interior void the closing pass is
/// allowed to leave behind.
const MAX_INTERIOR_VOID_WIDTH_MM: f32 = 0.5 * NOZZLE_MM;
/// Boundary band thickness used to classify a void as interior, in mm.
const BOUNDARY_BAND_MM: f32 = 0.02;

/// Regular polygon approximating a disc, in scaled units.
fn disc_units(cx: f64, cy: f64, r: f64) -> ExPolygon {
    const SEGS: usize = 32;
    ExPolygon {
        contour: Polygon {
            points: (0..SEGS)
                .map(|i| {
                    let a = std::f64::consts::TAU * (i as f64) / (SEGS as f64);
                    Point2 {
                        x: (cx + r * a.cos()).round() as i64,
                        y: (cy + r * a.sin()).round() as i64,
                    }
                })
                .collect(),
        },
        holes: Vec::new(),
    }
}

/// Union of the round-capped swept footprints of `paths`, honouring each
/// point's own `width` (never a nominal constant, never the centreline).
fn swept_footprint(paths: &[&ExtrusionPath3D]) -> Vec<ExPolygon> {
    swept_footprint_with(paths, |p| p.width)
}

/// Union of the round-capped swept footprints of `paths`, using the
/// **effective deposited** bead width (`width * flow_factor`) -- the width the
/// G-code emitter's volumetric-E path actually lays down, which for wave beads
/// is `nozzle_diameter * wave_flow / (nozzle_diameter * layer_height)`, i.e.
/// far wider than the nominal `width` stamped on each point.
fn swept_footprint_effective(paths: &[&ExtrusionPath3D]) -> Vec<ExPolygon> {
    swept_footprint_with(paths, |p| p.width * p.flow_factor)
}

/// Union of the round-capped swept footprints of `paths`, taking each point's
/// bead width from `width_of`.
fn swept_footprint_with(
    paths: &[&ExtrusionPath3D],
    width_of: impl Fn(&slicer_ir::Point3WithWidth) -> f32,
) -> Vec<ExPolygon> {
    let upm: f64 = slicer_ir::UNITS_PER_MM;
    let mut parts: Vec<ExPolygon> = Vec::new();
    for path in paths {
        for p in &path.points {
            parts.push(disc_units(
                f64::from(p.x) * upm,
                f64::from(p.y) * upm,
                f64::from(width_of(p)) * upm / 2.0,
            ));
        }
        for w in path.points.windows(2) {
            let (ax, ay) = (f64::from(w[0].x) * upm, f64::from(w[0].y) * upm);
            let (bx, by) = (f64::from(w[1].x) * upm, f64::from(w[1].y) * upm);
            let (dx, dy) = (bx - ax, by - ay);
            let len = (dx * dx + dy * dy).sqrt();
            if len <= f64::EPSILON {
                continue;
            }
            let ra = f64::from(width_of(&w[0])) * upm / 2.0;
            let rb = f64::from(width_of(&w[1])) * upm / 2.0;
            let (ux, uy) = (-dy / len, dx / len);
            parts.push(ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: (ax + ux * ra).round() as i64,
                            y: (ay + uy * ra).round() as i64,
                        },
                        Point2 {
                            x: (bx + ux * rb).round() as i64,
                            y: (by + uy * rb).round() as i64,
                        },
                        Point2 {
                            x: (bx - ux * rb).round() as i64,
                            y: (by - uy * rb).round() as i64,
                        },
                        Point2 {
                            x: (ax - ux * ra).round() as i64,
                            y: (ay - uy * ra).round() as i64,
                        },
                    ],
                },
                holes: Vec::new(),
            });
        }
    }
    slicer_core::polygon_ops::union_ex(&parts)
}

/// Area of an `ExPolygon` in mm^2.
fn area_mm2(exp: &ExPolygon) -> f64 {
    let ring = |poly: &Polygon| -> f64 {
        let pts = &poly.points;
        if pts.len() < 3 {
            return 0.0;
        }
        let mut acc = 0.0;
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            acc += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
        }
        acc.abs() / 2.0
    };
    let upm: f64 = slicer_ir::UNITS_PER_MM;
    (ring(&exp.contour) - exp.holes.iter().map(ring).sum::<f64>()) / (upm * upm)
}

/// Bounding box of an `ExPolygon`, in mm.
fn bbox_mm(exp: &ExPolygon) -> (f32, f32, f32, f32) {
    let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &exp.contour.points {
        let (x, y) = p.to_mm();
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }
    b
}

#[test]
fn waves_cover_domain_without_seam_gap() {
    use slicer_core::polygon_ops::{difference, intersection, offset, union_ex, OffsetJoinType};

    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let region = seam_fixture();
    let output = run(&module, std::slice::from_ref(&region));
    let paths = output.solid_paths();

    // Non-vacuity: waves must actually have engaged, otherwise this asserts
    // nothing about the wave generator at all.
    let wave_paths = locked(paths);
    assert!(
        !wave_paths.is_empty(),
        "waves did not engage; the coverage assertion would be vacuous"
    );

    let domain = vec![rect_mm(0.0, 0.0, SEAM_W_MM, SEAM_H_MM)];
    let swept = swept_footprint(&wave_paths);
    assert!(!swept.is_empty(), "swept footprint must be non-empty");

    // Voids touching the domain boundary are harmless slivers; only interior
    // voids indicate a missing line.
    let boundary_band = difference(
        &domain,
        &offset(&domain, -BOUNDARY_BAND_MM, OffsetJoinType::Round, 0.0),
    );
    let residual = union_ex(&difference(&domain, &swept));
    let mut offenders: Vec<(f64, (f32, f32, f32, f32))> = Vec::new();
    for comp in &residual {
        let comp_slice = std::slice::from_ref(comp);
        if !intersection(comp_slice, &boundary_band).is_empty() {
            continue; // boundary sliver
        }
        let eroded = offset(
            comp_slice,
            -MAX_INTERIOR_VOID_WIDTH_MM / 2.0,
            OffsetJoinType::Round,
            0.0,
        );
        if !eroded.is_empty() {
            offenders.push((area_mm2(comp), bbox_mm(comp)));
        }
    }
    assert!(
        offenders.is_empty(),
        "interior void(s) wider than {MAX_INTERIOR_VOID_WIDTH_MM} mm left inside the wave \
         domain (area mm^2, bbox mm): {offenders:?}"
    );
}


// ---------------------------------------------------------------------------
// Deposited-bead containment (packet 246 follow-up)
//
// `run_infill` stamps the NOMINAL nozzle diameter as each point's `width` and
// pairs it with `flow_factor = wave_flow / (width * layer_height)`. The
// emitter's volumetric-E path multiplies the two, so the bead actually laid
// down is `width * flow_factor` wide -- 0.75 mm for the 0.15 mm^3/mm default,
// not 0.4 mm. `generate` must therefore inset its trim boundary by half the
// EFFECTIVE width, or the bead physically overruns the fillable region and
// encroaches on the adjacent wall.
// ---------------------------------------------------------------------------

/// Manifest default `wave_overhang_perimeter_overlap`, in mm. The generator
/// grows each wave domain by this before filling, so it is the outer bound of
/// the fillable area.
const PERIMETER_OVERLAP_MM: f32 = 0.1;

/// Overflow area tolerated outside the fillable region, in mm^2.
///
/// Not slack for a real overrun: the test's swept footprint and the
/// generator's Clipper offsets approximate arcs with different polygon
/// resolutions, so a hair of disagreement along the boundary is expected. The
/// pre-fix overrun on this fixture is two orders of magnitude larger.
const MAX_BEAD_OVERFLOW_MM2: f64 = 0.01;

#[test]
fn wave_bead_footprint_stays_inside_trim_boundary() {
    use slicer_core::polygon_ops::{difference, intersection, offset, union, union_ex,
        OffsetJoinType};

    let module = WaveOverhangs::from_config(&wave_config(3.0)).expect("config");
    let region = seam_fixture();
    let output = run(&module, std::slice::from_ref(&region));
    let wave_paths = locked(output.solid_paths());
    assert!(
        !wave_paths.is_empty(),
        "waves did not engage; the containment assertion would be vacuous"
    );

    // Non-vacuity on the width itself: the emitted bead must really be wider
    // than the nominal width, otherwise this test proves nothing.
    let (nominal, effective) = wave_paths
        .iter()
        .flat_map(|p| p.points.iter())
        .map(|p| (p.width, p.width * p.flow_factor))
        .next()
        .expect("wave paths carry points");
    assert!(
        effective > nominal * 1.5,
        "expected the deposited bead ({effective} mm) to be much wider than the \
         nominal width ({nominal} mm)"
    );

    // The fillable area, rebuilt exactly as `run_infill` + `generate` do:
    // wave domain = external bridge U anchor band, grown by the perimeter
    // overlap into the neighbouring wall.
    let external = vec![rect_mm(0.0, 0.0, SEAM_W_MM, SEAM_H_MM)];
    let supported = vec![
        rect_mm(-6.0, -6.0, 0.0, SEAM_H_MM + 6.0),
        rect_mm(SEAM_W_MM, -6.0, SEAM_W_MM + 6.0, SEAM_H_MM + 6.0),
    ];
    let band = intersection(
        &supported,
        &offset(&external, 3.0, OffsetJoinType::Round, 0.0),
    );
    let fillable = offset(
        &union(&external, &band),
        PERIMETER_OVERLAP_MM,
        OffsetJoinType::Round,
        0.0,
    );

    let swept = swept_footprint_effective(&wave_paths);
    assert!(!swept.is_empty(), "swept footprint must be non-empty");
    let overflow = union_ex(&difference(&swept, &fillable));
    let total: f64 = overflow.iter().map(area_mm2).sum();
    let worst: Vec<(f64, (f32, f32, f32, f32))> = {
        let mut v: Vec<_> = overflow
            .iter()
            .map(|c| (area_mm2(c), bbox_mm(c)))
            .filter(|(a, _)| *a > 0.0)
            .collect();
        v.sort_by(|l, r| r.0.total_cmp(&l.0));
        v.truncate(3);
        v
    };
    assert!(
        total <= MAX_BEAD_OVERFLOW_MM2,
        "deposited wave bead (width {effective} mm) overruns the fillable region by \
         {total} mm^2 (limit {MAX_BEAD_OVERFLOW_MM2} mm^2); worst components \
         (area mm^2, bbox mm): {worst:?}"
    );
}
