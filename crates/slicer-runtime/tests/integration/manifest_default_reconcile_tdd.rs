//! AC-6 reconcile: each manifest `[config.schema.<key>].default` must equal the
//! module's *code fallback* — the value the module uses when no config override
//! is present. This is the "single source of truth" guard: if a manifest default
//! and its code fallback ever diverge, manifest validation being bypassed would
//! silently change behavior.
//!
//! To avoid a vacuous literal-against-literal assertion, this test reads BOTH
//! sides from their real sources. The manifest side is parsed from the committed
//! `*.toml` at compile time; the code side is observed by driving
//! `run_perimeters` with an EMPTY config and reading the fallback values back out
//! of the emitted wall loops. A divergence on either side (e.g. someone edits the
//! `unwrap_or(30.0)` arm but forgets the manifest, or vice-versa) fails the
//! assertion.
//!
//! Covers all three reconciled keys: `wall_count`, `outer_wall_speed`,
//! `inner_wall_speed`, for `classic-perimeters` (the sole perimeter generator;
//! the fake `arachne-perimeters` module was deleted in P108).
//!
//! Exit condition: `cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd`

use std::collections::HashMap;

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Outer-wall speed normalisation reference (mm/s); mirrors
/// `slicer_core::perimeter_utils::BASE_SPEED`. Both modules store/compute speeds
/// as `speed / BASE_SPEED` factors, so the observed factor is rescaled here.
const BASE_SPEED: f32 = 50.0;

const CLASSIC_MANIFEST: &str =
    include_str!("../../../../modules/core-modules/classic-perimeters/classic-perimeters.toml");

/// Read a numeric `[config.schema.<key>].default` value from a manifest TOML.
/// Panics (failing the test) if the key or its default is absent / non-numeric.
fn manifest_default(manifest: &str, key: &str) -> f64 {
    let parsed: toml::Value = toml::from_str(manifest).expect("manifest parses as TOML");
    let default = &parsed["config"]["schema"][key]["default"];
    default
        .as_float()
        .or_else(|| default.as_integer().map(|i| i as f64))
        .unwrap_or_else(|| panic!("default for `{key}` is missing or non-numeric in manifest"))
}

fn square_region(z: f32) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_z(z);
    region.set_polygons(vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    }]);
    region
}

/// Drive `run_perimeters` for module `M` with an empty config so every value is
/// supplied by the code fallback, then recover those fallbacks from the emitted
/// wall loops. Returns `(wall_count, outer_wall_speed, inner_wall_speed)`.
///
/// A 10mm square at the default 0.4mm line width fits the default 3 walls, so the
/// emitted loop count equals the `wall_count` code fallback, and the outer
/// (perimeter_index 0) / inner (perimeter_index >= 1) loops carry the speed
/// fallbacks as `speed_factor`.
fn observed_code_fallbacks<M: LayerModule>() -> (usize, f32, f32) {
    let empty = ConfigView::from_map(HashMap::new());
    let module = M::on_print_start(&empty).expect("on_print_start should succeed");
    let region = square_region(0.2);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(
            0,
            &[region],
            &PaintRegionLayerView::new(0),
            &mut output,
            &empty,
        )
        .expect("run_perimeters with empty config should succeed");

    let walls = output.wall_loops();
    let outer = walls
        .iter()
        .find(|w| w.perimeter_index == 0)
        .expect("an outer wall (perimeter_index 0) should be emitted");
    let inner = walls
        .iter()
        .find(|w| w.perimeter_index >= 1)
        .expect("an inner wall (perimeter_index >= 1) should be emitted");
    (
        walls.len(),
        outer.path.speed_factor * BASE_SPEED,
        inner.path.speed_factor * BASE_SPEED,
    )
}

fn assert_reconciled(manifest: &str, wall_count: usize, outer: f32, inner: f32) {
    assert_eq!(
        wall_count as f64,
        manifest_default(manifest, "wall_count"),
        "wall_count code fallback must equal the manifest default"
    );
    let expected_outer = manifest_default(manifest, "outer_wall_speed");
    assert!(
        (outer as f64 - expected_outer).abs() < 0.001,
        "outer_wall_speed code fallback {outer} must equal manifest default {expected_outer}"
    );
    let expected_inner = manifest_default(manifest, "inner_wall_speed");
    assert!(
        (inner as f64 - expected_inner).abs() < 0.001,
        "inner_wall_speed code fallback {inner} must equal manifest default {expected_inner}"
    );
}

#[test]
fn classic_perimeters_defaults_match_manifest() {
    let (wall_count, outer, inner) = observed_code_fallbacks::<ClassicPerimeters>();
    assert_reconciled(CLASSIC_MANIFEST, wall_count, outer, inner);
}
