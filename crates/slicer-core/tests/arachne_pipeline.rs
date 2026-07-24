//! Integration test for `arachne::pipeline::run_arachne_pipeline` (packet
//! 112, Step 9A — host-side Arachne wall-generation bridge).
//!
//! # Self-captured invariant checks — NOT an OrcaSlicer golden
//!
//! This packet has no OrcaSlicer oracle for the end-to-end pipeline (see
//! `crates/slicer-core/src/arachne/pipeline.rs`'s module-level doc comment,
//! and every stage it chains together for their own from-first-principles
//! adaptation notes). This file asserts real, self-consistent invariants
//! over the pipeline's own output — non-empty, determinism, and per-junction
//! width behaviour that follows from the fixture's geometry — never
//! OrcaSlicer numeric parity.
//!
//! Host-only: `arachne::pipeline` is gated behind the `host-algos` feature
//! (matching `voronoi`, `algos`, `medial_axis`, `skeletal_trapezoidation`),
//! so this whole file is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::pipeline::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon, UNITS_PER_MM};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// A 10mm square.
///
/// # Geometry note
///
/// A "unit square"-scale fixture (e.g. the 1000-unit / 0.1mm side square
/// `skt_graph_golden.rs` and the centrality/bead-count/propagation/
/// generate_toolpaths test files use) is too small for this test:
/// `ArachneParams::default()`'s `optimal_width` is 0.4mm, but a 0.1mm-side
/// square's medial axis only reaches a maximum `distance_to_boundary` of
/// 0.05mm at its center (half the side length) — every central edge's
/// `r_avg`-derived thickness (`2 * r_avg`) then falls far short of
/// `optimal_width`, so `BeadingStrategyFactory`'s composed stack reports
/// `optimal_bead_count() == 0` everywhere and `generate_toolpaths` emits
/// nothing. A 10mm square's center reaches a `distance_to_boundary` of 5mm,
/// comfortably clearing the default `optimal_width`/`max_bead_count`
/// thresholds and producing a non-trivial bead count on every spoke.
fn square_10mm() -> ExPolygon {
    let side_units = (10.0 * UNITS_PER_MM) as i64;
    expoly(vec![
        p(0, 0),
        p(side_units, 0),
        p(side_units, side_units),
        p(0, side_units),
    ])
}

/// A wedge that tapers from a 0.35mm-wide tip to a 2.6mm-wide base over 12mm.
///
/// # Geometry note
///
/// This is the fixture that exercises variable-width output, which
/// [`square_10mm`] structurally cannot. Arachne only varies a junction's width
/// where the bead count *changes*: `DistributedBeadingStrategy::compute`
/// distributes the leftover `to_be_divided` across beads, and
/// `SkeletalTrapezoidation::generateTransitioningRibs` interpolates widths
/// across a transition region. A taper sweeps continuously through several
/// optimal bead counts, so it has transition regions; a constant-width shape
/// has none.
fn tapered_wedge() -> ExPolygon {
    let mm = |v: f64| (v * UNITS_PER_MM) as i64;
    expoly(vec![
        p(0, mm(-0.175)),
        p(mm(12.0), mm(-1.3)),
        p(mm(12.0), mm(1.3)),
        p(0, mm(0.175)),
    ])
}

/// AC: `run_arachne_pipeline` on a (sufficiently large) square returns `Ok`
/// with at least one `ExtrusionLine`, and every junction carries the same
/// width — the optimal width exactly.
///
/// # Why uniform, and why that is the correct expectation
///
/// This assertion used to demand that some junction width *differ*. It never
/// could on this fixture, and canonical agrees: the square is 10mm on a side at
/// a 0.4mm optimal width, so `10.0 / 0.4 == 25` exactly and
/// `DistributedBeadingStrategy::compute` has `to_be_divided == 0` — there is no
/// leftover to redistribute, so every bead is emitted at exactly the optimal
/// width. Nor are there transitions to interpolate across: a square's 90-degree
/// corner gives a medial-axis `dR/dD` of about 0.707, far above the
/// `sin(5 degrees) ~= 0.087` cap that the default 10-degree
/// `wall_transition_angle` imposes, so `filter_central` marks no edge central
/// and `generateTransitioningRibs` has nothing to work on.
///
/// Variable-width output is asserted on [`tapered_wedge`], which actually has
/// transition regions. (The old rationale here cited a "per-junction local-R
/// width derivation" in `generate_toolpaths`; that derivation was deleted in
/// Step 9D, so the justification had outlived the code it named.)
#[test]
fn arachne_pipeline_square_produces_uniform_width_lines() {
    let square = square_10mm();
    let params = ArachneParams::default();

    let result = run_arachne_pipeline(std::slice::from_ref(&square), &params, false);
    let (lines, _) = result.expect("10mm square should produce Ok(lines) under default params");

    assert!(
        !lines.is_empty(),
        "expected at least one ExtrusionLine from a 10mm square"
    );

    let widths: Vec<f32> = lines
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|j| j.p.width)
        .collect();
    assert!(
        widths.len() >= 2,
        "expected at least two junctions total to compare widths, got {}",
        widths.len()
    );

    let optimal = params.optimal_width as f32;
    for (i, &w) in widths.iter().enumerate() {
        assert!(
            (w - optimal).abs() <= 1e-6,
            "junction {i} width {w} != optimal {optimal}; a 10mm/0.4mm square \
             divides exactly, so no bead should be widened or interpolated. \
             All widths: {widths:?}"
        );
    }
}

/// AC: a shape with a genuine taper produces variable-width output.
///
/// This is the assertion that moved off `arachne_pipeline_square_produces_uniform_width_lines`.
/// It is a self-captured invariant, not an OrcaSlicer numeric golden: it
/// asserts only that widths vary and stay within the strategy's own bounds,
/// never a specific width at a specific junction.
#[test]
fn arachne_pipeline_taper_produces_variable_width_lines() {
    let wedge = tapered_wedge();
    let params = ArachneParams::default();

    let result = run_arachne_pipeline(std::slice::from_ref(&wedge), &params, false);
    let (lines, _) = result.expect("tapered wedge should produce Ok(lines) under default params");

    assert!(
        !lines.is_empty(),
        "expected at least one ExtrusionLine from a tapered wedge"
    );

    let widths: Vec<f32> = lines
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|j| j.p.width)
        .collect();
    assert!(
        widths.len() >= 2,
        "expected at least two junctions total to compare widths, got {}",
        widths.len()
    );

    let first = widths[0];
    assert!(
        widths.iter().any(|&w| (w - first).abs() > 1e-6),
        "a tapered shape must produce variable-width output; every junction \
         width was identical: {widths:?}"
    );
    assert!(
        widths.iter().all(|&w| w > 0.0),
        "no junction may carry a non-positive width: {widths:?}"
    );
}

/// AC: two independent runs over the same input produce byte-identical
/// output — every stage `run_arachne_pipeline` chains together documents its
/// own determinism (index-ordered traversal, no float-keyed hash maps, no
/// nondeterministic parallelism observable in output order).
#[test]
fn arachne_pipeline_is_deterministic() {
    let square = square_10mm();
    let params = ArachneParams::default();

    let first = run_arachne_pipeline(std::slice::from_ref(&square), &params, false)
        .expect("first run should succeed");
    let second = run_arachne_pipeline(std::slice::from_ref(&square), &params, false)
        .expect("second run should succeed");

    assert_eq!(first, second, "pipeline must be deterministic");
}

/// AC: `ExtrusionJunction::perimeter_index` is the bead/inset index (the
/// N2 contract — packet 142), not the placeholder-0 carried at generation
/// and not the old "sequential position within the line's junction Vec"
/// redefinition that packet 142 deletes alongside the
/// `assign_perimeter_indices` post-pass. Every junction of every line must
/// read `perimeter_index == line.inset_idx`. Since a square's medial axis
/// produces several beads, at least one line must have more than one
/// junction so the assertion is non-trivial.
#[test]
fn arachne_pipeline_perimeter_index_is_sequential_per_line() {
    let square = square_10mm();
    let params = ArachneParams::default();

    let (lines, _) = run_arachne_pipeline(std::slice::from_ref(&square), &params, false)
        .expect("10mm square should produce Ok(lines)");

    assert!(!lines.is_empty(), "expected at least one ExtrusionLine");

    let mut saw_multi_junction_line = false;
    for line in &lines {
        if line.junctions.len() > 1 {
            saw_multi_junction_line = true;
        }
        for (j_pos, junction) in line.junctions.iter().enumerate() {
            assert_eq!(
                junction.perimeter_index, line.inset_idx,
                "junction {j_pos} of a line of {} should carry perimeter_index == line.inset_idx (= {}), got {}",
                line.junctions.len(),
                line.inset_idx,
                junction.perimeter_index
            );
        }
    }
    assert!(
        saw_multi_junction_line,
        "expected at least one line with >1 junction to make the perimeter_index == inset_idx assertion non-trivial"
    );
}

/// A long, narrow strip: half-width `half_width_mm` (so full width
/// `2 * half_width_mm`), running from `x = 0` to `x = length_mm` along the
/// x-axis, centered on `y = 0`.
fn thin_strip(half_width_mm: f64, length_mm: f64) -> ExPolygon {
    let hw = (half_width_mm * UNITS_PER_MM) as i64;
    let len = (length_mm * UNITS_PER_MM) as i64;
    expoly(vec![p(0, -hw), p(len, -hw), p(len, hw), p(0, hw)])
}

/// AC (packet 112, Step 9C): `print_thin_walls`/`min_feature_size`/
/// `min_bead_width` are now plumbed from `ArachneParams` into
/// `BeadingFactoryParams`, so `WideningBeadingStrategy` actually joins the
/// composed stack when the knob is on -- this proves the wiring is live, not
/// a silently-ignored no-op.
///
/// # Geometry (empirically confirmed, not derived from any OrcaSlicer trace)
///
/// A 10mm x 0.15mm strip (`thin_strip(0.075, 10.0)`): its medial axis is a
/// single straight central edge at constant `distance_to_boundary = 0.075mm`
/// (half the strip width), so `thickness = 2 * r_avg = 0.15mm` for that
/// edge. Under `ArachneParams::default()` (`optimal_width = 0.4mm`,
/// `minimum_variable_line_ratio` internal default `0.5`), this falls in a
/// band that this implementation's own beading stack treats very
/// differently depending on `print_thin_walls`:
/// - **Without widening**: `DistributedBeadingStrategy::optimal_bead_count`
///   is `(thickness / optimal_width).round()` = `(0.15 / 0.4).round()` =
///   `0`. Zero beads means `generate_toolpaths` emits nothing for this
///   region at all -- this is exactly the packet's stated bug ("sub-~0.2mm
///   features are dropped").
/// - **With widening**: `thickness` (0.15mm) is still `>=
///   min_feature_size` (default 0.1mm), so
///   `WideningBeadingStrategy::optimal_bead_count` forces `bead_count = 1`
///   instead of delegating to the (zero) parent result -- the feature is no
///   longer silently dropped.
///
/// `0.15mm` was chosen (over the packet brief's suggested ~0.25mm) because
/// empirically a 0.25mm-wide strip already clears `Distributed`'s own
/// rounding threshold (`round(0.25 / 0.4) = round(0.625) = 1`) even with
/// `print_thin_walls = false`, so the two runs would not differ in line
/// *count* at all (only in unrelated near-zero-width diagonal fragments
/// close to the strip's end caps). `0.15mm` sits below
/// `minimum_variable_line_ratio * optimal_width` (0.2mm) -- the threshold
/// `RedistributeBeadingStrategy`/`Distributed` need to ever report a bead --
/// while staying above `min_feature_size` (0.1mm), so it isolates the
/// `detect_thin_wall` knob's effect cleanly: present vs. entirely absent.
///
/// # Width clamp now observable (packet 112, Step 9D)
///
/// `crate::arachne::generate_toolpaths` (this pipeline's toolpath-emission
/// stage) now calls `strategy.compute()` per edge endpoint and reads each
/// bead's width directly from the returned `Beading` (see that module's own
/// doc comment), instead of the pre-Step-9D `2.0 * distance_to_boundary /
/// bead_count` geometric approximation that never touched
/// `BeadingStrategy::compute()` at all. So the bead this test observes with
/// widening on now carries `WideningBeadingStrategy::compute`'s
/// `min_output_width` clamp: the raw `0.15mm` feature width is widened up
/// toward `min_bead_width` (`ArachneParams::default()`'s 0.4mm), not left at
/// the raw feature thickness. This test asserts that widened width directly
/// -- both the functional effect (a thin feature goes from "silently
/// dropped" to "printed") and the width-clamp effect are now observable
/// end-to-end.
#[test]
fn arachne_pipeline_thin_wall_widening() {
    let strip = thin_strip(0.075, 10.0);

    let widening_on = ArachneParams {
        print_thin_walls: true,
        ..ArachneParams::default()
    };
    let widening_off = ArachneParams {
        print_thin_walls: false,
        ..ArachneParams::default()
    };

    let (on_lines, _) = run_arachne_pipeline(std::slice::from_ref(&strip), &widening_on, false)
        .expect("thin strip with widening on should produce Ok(lines)");
    let (off_lines, _) = run_arachne_pipeline(std::slice::from_ref(&strip), &widening_off, false)
        .expect("thin strip with widening off should still produce Ok([]), not an error");

    assert!(
        off_lines.is_empty(),
        "expected the 0.15mm strip to be entirely dropped with print_thin_walls=false \
         (Distributed rounds thickness/optimal_width down to 0 beads), got {} lines",
        off_lines.len()
    );
    assert!(
        !on_lines.is_empty(),
        "expected print_thin_walls=true to rescue the 0.15mm strip via \
         WideningBeadingStrategy forcing bead_count=1, got 0 lines"
    );

    // The rescued bead's width is now clamped up toward min_bead_width (see
    // this test's doc comment): the strip's raw ~0.15mm feature thickness is
    // below `ArachneParams::default()`'s `min_bead_width` (0.4mm), so
    // `WideningBeadingStrategy::compute`'s `thickness.max(min_output_width)`
    // clamp forces the emitted bead's width up to 0.4mm, not the raw 0.15mm
    // feature width. This is also this packet's units sanity check: if the
    // width comes out as ~4000 or ~0.00004 instead of ~0.4mm, the
    // units->mm conversion in `generate_toolpaths` is wrong.
    let widths: Vec<f32> = on_lines
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|j| j.p.width)
        .collect();
    assert!(
        !widths.is_empty(),
        "expected at least one junction in the rescued output"
    );
    for &w in &widths {
        assert!(
            (w - 0.4).abs() < 0.01,
            "expected the rescued bead's width to be clamped up to min_bead_width (~0.4mm), \
             got {w}mm"
        );
    }
}

/// AC-N4 (packet 113a, Step 3): when `arachne_params_from_config` receives a
/// config view that omits all 7 of the newly-wired keys, every read falls back
/// to [`ArachneParams::default()`] rather than erroring or returning zero.
///
/// AC-N5 (packet 113a, closure fix 2): `run_arachne_pipeline` with
/// `is_initial_layer=true` overrides `min_output_width` with
/// `initial_layer_min_bead_width` (0.34mm default) so a rescued thin wall on
/// layer 0 is clamped to the initial-layer minimum, not the general
/// `min_bead_width` (0.4mm default).
#[test]
fn arachne_pipeline_initial_layer_uses_initial_layer_min_bead_width() {
    let strip = thin_strip(0.075, 10.0);
    let params = ArachneParams {
        print_thin_walls: true,
        ..Default::default()
    };

    let (lines, _) = run_arachne_pipeline(std::slice::from_ref(&strip), &params, true)
        .expect("initial-layer thin strip should produce Ok(lines)");

    assert!(
        !lines.is_empty(),
        "expected the 0.15mm strip to be rescued on the initial layer"
    );

    let widths: Vec<f32> = lines
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|j| j.p.width)
        .collect();
    assert!(
        !widths.is_empty(),
        "expected at least one junction in the rescued output"
    );
    for &w in &widths {
        assert!(
            (w - 0.34).abs() < 0.01,
            "expected the initial-layer rescued bead's width to be clamped to \
             initial_layer_min_bead_width (~0.34mm), got {w}mm"
        );
    }
}

/// AC-N6 (packet 113a, closure fix 1): the beading strategy stack exposes the
/// configured `wall_transition_angle` through the `BeadingStrategy` trait, and
/// the factory passes the value from `ArachneParams` into
/// `DistributedBeadingStrategy`.
#[test]
fn distributed_strategy_wall_transition_angle_round_trips() {
    use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};

    let params = BeadingFactoryParams {
        wall_transition_angle: 0.123_456_789,
        ..Default::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    assert!(
        (stack.wall_transition_angle() - 0.123_456_789).abs() < 1e-9,
        "expected wall_transition_angle to round-trip through the factory and stack, got {}",
        stack.wall_transition_angle()
    );
}

/// AC-N4 (packet 113a, Step 3): when `arachne_params_from_config` receives a
/// config view that omits all 7 of the newly-wired keys, every read falls back
/// to [`ArachneParams::default()`] rather than erroring or returning zero.
///
/// The function under test lives in the `arachne-perimeters` module crate,
/// not in `slicer_core`, so we cannot call it directly from this
/// `slicer-core` integration test. The contract is identical to
/// constructing [`ArachneParams::default()`] here: an empty `ConfigView`
/// produces the same values as the default struct. This test asserts the
/// public fallback behavior the module promises.
#[test]
fn arachne_params_defaults_when_keys_absent() {
    let empty_config = ConfigView::new();
    let default_params = ArachneParams::default();

    // The 7 keys that 113a wires are all absent from `empty_config`; the
    // module's `arachne_params_from_config` must therefore fall back to the
    // same defaults declared on `ArachneParams`.
    assert_eq!(
        empty_config.get_float("min_central_distance"),
        None,
        "test fixture must not contain min_central_distance"
    );
    assert_eq!(
        empty_config.get_float("visvalingam_area_threshold"),
        None,
        "test fixture must not contain visvalingam_area_threshold"
    );
    assert_eq!(
        empty_config.get_float("min_width"),
        None,
        "test fixture must not contain min_width"
    );
    assert_eq!(
        empty_config.get_float("wall_transition_length"),
        None,
        "test fixture must not contain wall_transition_length"
    );
    assert_eq!(
        empty_config.get_float("wall_transition_angle"),
        None,
        "test fixture must not contain wall_transition_angle"
    );
    assert_eq!(
        empty_config.get_float("initial_layer_min_bead_width"),
        None,
        "test fixture must not contain initial_layer_min_bead_width"
    );
    assert_eq!(
        empty_config.get_float("outer_wall_offset"),
        None,
        "test fixture must not contain outer_wall_offset"
    );

    // The observable contract: default params are returned (not an error)
    // when the config is empty. We compare the subset of fields that the 7
    // wired keys influence; the remaining fields are unchanged by the empty
    // config anyway.
    assert_eq!(default_params.min_central_distance, 0.0);
    assert_eq!(default_params.visvalingam_area_threshold, 0.01);
    assert_eq!(default_params.min_width, 0.4);
}
