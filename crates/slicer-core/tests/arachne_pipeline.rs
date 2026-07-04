//! Integration test for `arachne::pipeline::run_arachne_pipeline` (packet
//! 112, Step 9A — host-side Arachne wall-generation bridge).
//!
//! # Self-captured invariant checks — NOT an OrcaSlicer golden
//!
//! This packet has no OrcaSlicer oracle for the end-to-end pipeline (see
//! `crates/slicer-core/src/arachne/pipeline.rs`'s module-level doc comment,
//! and every stage it chains together for their own from-first-principles
//! adaptation notes). This file asserts real, self-consistent invariants
//! over the pipeline's own output — non-empty, observable per-junction width
//! variation, determinism — never OrcaSlicer numeric parity.
//!
//! Host-only: `arachne::pipeline` is gated behind the `host-algos` feature
//! (matching `voronoi`, `algos`, `medial_axis`, `skeletal_trapezoidation`),
//! so this whole file is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::pipeline::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

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

/// AC: `run_arachne_pipeline` on a (sufficiently large) square returns
/// `Ok` with at least one `ExtrusionLine`, and per-junction widths are not
/// all identical — the medial axis of a square runs from each corner
/// (`distance_to_boundary == 0`, sitting exactly on the boundary) to the
/// center (`distance_to_boundary == side / 2`), so every emitted line's own
/// two junctions carry genuinely different local-R-derived widths (see
/// `crate::arachne::generate_toolpaths`'s module-level doc comment for the
/// per-junction local-R width derivation this relies on).
#[test]
fn arachne_pipeline_square_produces_lines() {
    let square = square_10mm();
    let params = ArachneParams::default();

    let result = run_arachne_pipeline(std::slice::from_ref(&square), &params);
    let lines = result.expect("10mm square should produce Ok(lines) under default params");

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
    let first = widths[0];
    assert!(
        widths.iter().any(|&w| (w - first).abs() > 1e-6),
        "expected variable-width output (not every junction width identical): {widths:?}"
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

    let first = run_arachne_pipeline(std::slice::from_ref(&square), &params)
        .expect("first run should succeed");
    let second = run_arachne_pipeline(std::slice::from_ref(&square), &params)
        .expect("second run should succeed");

    assert_eq!(first, second, "pipeline must be deterministic");
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

    let on_lines = run_arachne_pipeline(std::slice::from_ref(&strip), &widening_on)
        .expect("thin strip with widening on should produce Ok(lines)");
    let off_lines = run_arachne_pipeline(std::slice::from_ref(&strip), &widening_off)
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
