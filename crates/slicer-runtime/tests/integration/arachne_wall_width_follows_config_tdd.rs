//! Arachne's emitted wall width must equal the user's configured wall line
//! width — the same contract `outer_inner_width_and_spacing_tdd` pins for
//! `classic-perimeters`.
//!
//! This is the discriminating fixture for two defects that hid each other
//! (`D-160-ARACHNE-IGNORES-WALL-LINE-WIDTH`). It is parameterised over the
//! configured width `W` rather than asserting a constant, because a fixture
//! that cannot vary the quantity under test is not a test of it — the lesson
//! `tapered_wedge` learned the hard way by asserting `0.3571` (the defect)
//! and calling it "the Flow-spacing value".
//!
//! The two defects, and how each `W` case separates them:
//!
//! - **Emission.** Canonical treats a beading junction's width as a *spacing*
//!   and converts it back to an extrusion width before emitting:
//!   `VariableWidth.cpp::thick_polyline_to_multi_path` does
//!   `flow.with_width(unscale(w) + height * (1 - PI/4))`, reached from
//!   `PerimeterGenerator.cpp::traverse_extrusions` via
//!   `extrusion_paths_append(dst, ExtrusionLine, role, flow)`. PnP emitted the
//!   raw spacing, so every arachne wall was ~10.7% narrow at default config.
//!   Caught by `W = 0.4`.
//!
//! - **Wiring.** Canonical derives Arachne's bead widths from the user's wall
//!   flows — `PerimeterGenerator` builds
//!   `WallToolPaths(last_p, bead_width_0, perimeter_spacing, ...)` with
//!   `bead_width_0 = ext_perimeter_spacing = ext_perimeter_flow.scaled_spacing()`
//!   and `bead_width_x = perimeter_spacing = perimeter_flow.scaled_spacing()`.
//!   PnP instead read two Arachne-internal knobs and never connected them, so
//!   output was *invariant* to the user's setting. Caught by `W = 0.8`.
//!
//! Because the internal knobs defaulted to 0.4mm, the wiring defect is silent
//! at `W = 0.4` and only `W = 0.8` can observe it — which is exactly why it
//! survived: no fixture in the suite sets either wall-width key.

use arachne_perimeters::ArachnePerimeters;
use slicer_core::flow::{flow_to_width, line_width_to_spacing};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Layer height and nozzle used by every case here. 0.2mm/0.4mm is the
/// canonical sanity pair `crates/slicer-core/src/flow.rs` documents.
const LAYER_HEIGHT_MM: f32 = 0.2;
const NOZZLE_MM: f32 = 0.4;

/// A square is deliberately chosen: it is a *uniform-thickness* region, so the
/// beading strategy has no surplus gradient to redistribute and every bead
/// lands at its target width. That makes "emitted width == configured width" a
/// clean equality rather than a statistical claim.
const SQUARE_SIDE_MM: f32 = 10.0;

fn make_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, SQUARE_SIDE_MM))
        .build()
}

/// Drive `ArachnePerimeters` with `outer_wall_line_width == inner_wall_line_width == w_mm`
/// and return every emitted wall-vertex width.
///
/// Both keys are set to the same value on purpose: with outer == inner, every
/// bead shares one target, so a single expected value covers the whole loop
/// set and the assertion needs no per-wall bookkeeping.
fn emitted_widths_for(w_mm: f32) -> Vec<f32> {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", w_mm as f64)
        .float("inner_wall_line_width", w_mm as f64)
        .float("layer_height", LAYER_HEIGHT_MM as f64)
        .float("nozzle_diameter", NOZZLE_MM as f64)
        .build();

    let module = ArachnePerimeters::on_print_start(&config).expect("on_print_start");
    let regions = vec![make_region(LAYER_HEIGHT_MM)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .expect("run_perimeters");

    let widths: Vec<f32> = output
        .wall_loops()
        .iter()
        .flat_map(|w| w.path.points.iter().map(|p| p.width))
        .collect();
    assert!(
        !widths.is_empty(),
        "no wall vertices emitted for w={w_mm}mm — the fixture must not vacuously pass"
    );
    widths
}

/// Assert every emitted width equals `expected` within `tol`, reporting the
/// worst offender so a failure names the actual number rather than "false".
fn assert_all_widths(widths: &[f32], expected: f32, tol: f32, what: &str) {
    let worst = widths
        .iter()
        .copied()
        .max_by(|a, b| {
            (a - expected)
                .abs()
                .partial_cmp(&(b - expected).abs())
                .expect("finite widths")
        })
        .expect("non-empty");
    assert!(
        (worst - expected).abs() < tol,
        "{what}: every emitted arachne wall width must be {expected}mm (+/- {tol}), \
         worst was {worst}mm over {} vertices (deviation {})",
        widths.len(),
        (worst - expected).abs()
    );
}

/// The arithmetic identity the whole contract rests on: converting a width to
/// a Flow spacing and back must recover the width. If this ever fails, the two
/// behavioural tests below are measuring the wrong thing, so pin it first and
/// derive their expectation from it rather than from a literal.
#[test]
fn spacing_round_trip_recovers_the_configured_width() {
    for w in [0.4_f32, 0.8] {
        let spacing = line_width_to_spacing(w, LAYER_HEIGHT_MM, NOZZLE_MM);
        let recovered = flow_to_width(spacing, LAYER_HEIGHT_MM);
        assert!(
            (recovered - w).abs() < 1e-4,
            "flow_to_width(line_width_to_spacing({w})) = {recovered}, must recover {w}"
        );
        assert!(
            spacing < w,
            "spacing {spacing} must be strictly narrower than width {w} — \
             otherwise these tests cannot tell the two apart"
        );
    }
}

/// Emission defect (`W = 0.4`). At the default width the wiring defect is
/// invisible (the internal knobs default to 0.4mm too), so this case isolates
/// the missing spacing -> width back-conversion.
///
/// Verified to FAIL before the emission fix: emitted 0.3571mm, which is
/// `line_width_to_spacing(0.4)` — the raw beading spacing, never converted.
#[test]
fn emitted_width_equals_configured_width_at_default() {
    let w = 0.4_f32;
    assert_all_widths(&emitted_widths_for(w), w, 0.005, "default width");
}

/// Wiring defect (`W = 0.8`). Arachne must track the configured width rather
/// than its own internal default.
///
/// Verified to FAIL before the wiring fix: emitted the same value as the 0.4
/// case regardless of this setting — output *invariant* to the user's key.
#[test]
#[ignore = "RED until the Bug A wiring fix lands (D-160): emits 0.4mm regardless of the key"]
fn emitted_width_tracks_a_non_default_configured_width() {
    let w = 0.8_f32;
    assert_all_widths(&emitted_widths_for(w), w, 0.005, "non-default width");
}

/// The invariance check stated directly: two different configured widths must
/// produce two different emitted widths. This is the assertion `D-160` was
/// discovered by hand, and the one no fixture in the suite could make.
#[test]
#[ignore = "RED until the Bug A wiring fix lands (D-160): emits 0.4mm regardless of the key"]
fn emitted_width_is_not_invariant_to_the_configured_key() {
    let narrow = emitted_widths_for(0.4);
    let wide = emitted_widths_for(0.8);
    let (n, w) = (narrow[0], wide[0]);
    assert!(
        (w - n).abs() > 0.1,
        "arachne emitted {n}mm at outer/inner_wall_line_width=0.4 and {w}mm at 0.8 — \
         output is invariant to the user's wall width (D-160)"
    );
}
