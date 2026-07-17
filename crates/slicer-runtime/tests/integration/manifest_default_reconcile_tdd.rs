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
//! Two layers of guard:
//!
//! 1. **Behavioral** (`classic_perimeters_defaults_match_manifest`): drives
//!    `run_perimeters` with an empty config and reads 3 fallbacks back out of
//!    the emitted walls. Strongest form, but only reaches keys with an
//!    observable output path — which is exactly how classic's
//!    `outer_wall_line_width` manifest default sat at a false `0.5` for months
//!    (the code fallback was `0.4`; D-160's audit caught it by hand).
//!
//! 2. **Exhaustive by enumeration** (`*_manifest_defaults_are_the_code_fallbacks`):
//!    every `[config.schema.*]` key in the classic + arachne manifests must
//!    appear in a per-module table pinning its code fallback (transcribed from
//!    the read site, cited per entry), and the manifest default must equal it.
//!    Set-equality runs both ways, so adding a manifest key without
//!    classifying it here fails, as does listing a key the manifest dropped.
//!    Declared-but-never-read keys are classified `Unread` explicitly — there
//!    is no opt-out that skips a key silently.
//!
//! The table is still a transcription (the code side is quoted, not consumed
//! by the module at runtime). The by-construction version — a per-module
//! DEFAULTS table that the read sites themselves consume, making drift
//! impossible rather than detectable — is split out as a follow-up packet;
//! see D-160's reconcile follow-up in docs/DEVIATION_LOG.md. It is blocked on
//! a real design question this test surfaced: several classic fallbacks are
//! *derived* (`nozzle_diameter` falls back to `inner_wall_line_width`, which
//! falls back to `line_width`), so a constants table cannot represent every
//! key.
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

// ═══════════════════════════════════════════════════════════════════════════
// Exhaustive manifest-key reconcile (D-160 follow-through)
// ═══════════════════════════════════════════════════════════════════════════

const ARACHNE_MANIFEST: &str =
    include_str!("../../../../modules/core-modules/arachne-perimeters/arachne-perimeters.toml");

/// The module's code fallback for one manifest key, expressed in the
/// MANIFEST's own domain (so `Float(4000.0)` for a `unit = "units"` key whose
/// code fallback is `0.4mm`). Each table entry cites the read site it was
/// transcribed from; auditing that citation is what keeps this table honest
/// until the by-construction DEFAULTS refactor lands.
enum CodeFallback {
    Float(f64),
    Int(i64),
    Bool(bool),
    Str(&'static str),
    /// Declared in the manifest (schema/UI surface) but never read by the
    /// module's code — there is no code fallback to reconcile. Presence-only.
    Unread,
}
use CodeFallback::*;

/// Transcribed from `classic-perimeters/src/lib.rs` — `on_print_start` and
/// the R2 per-invocation block at the top of `run_perimeters`.
const CLASSIC_FALLBACKS: &[(&str, CodeFallback)] = &[
    ("wall_count", Int(3)),       // on_print_start `_ => 3`
    ("extra_perimeters", Int(0)), // unwrap_or(0)
    ("extra_perimeters_on_overhangs", Bool(false)),
    ("line_width", Float(0.4)), // legacy_line_width `_ => 0.4`
    // Derived: falls back to legacy_line_width (`line_width`), itself 0.4 —
    // the effective empty-config fallback is 0.4. THIS row is the one that
    // read a false 0.5 in the manifest until D-160's audit.
    ("outer_wall_line_width", Float(0.4)),
    ("inner_wall_line_width", Float(0.4)), // derived like outer; effective 0.4
    ("precise_outer_wall", Bool(false)),
    ("wall_sequence", Str("InnerOuter")),
    ("detect_thin_wall", Bool(true)),
    ("seam_candidate_angle_threshold_deg", Float(30.0)),
    ("gap_infill_speed", Float(30.0)),
    ("filter_out_gap_fill", Float(0.5)),
    ("gap_fill_medial_axis_on_painted", Bool(false)),
    ("slice_has_paint", Bool(false)),
    ("outer_wall_speed", Float(30.0)), // on_print_start `_ => 30.0`
    ("inner_wall_speed", Float(45.0)), // on_print_start `_ => 45.0`
    ("perimeter_arc_tolerance", Float(0.0125)),
    ("only_one_wall_top", Bool(false)),
    ("only_one_wall_first_layer", Bool(false)),
    ("smaller_perimeter_line_width", Float(0.25)),
    ("smaller_perimeter_threshold_mm", Float(0.8)),
    ("narrow_loop_length_threshold_mm", Float(10.0)),
    ("detect_overhang_wall", Unread), // declared; classic never reads it
    ("overhang_reverse", Unread),
    ("overhang_reverse_internal_only", Unread),
    ("min_width_top_surface", Float(1.2)),
    ("alternate_extra_wall", Unread),
    ("bridge_flow", Float(1.0)),
    ("thick_bridges", Bool(false)),
    // Derived: nozzle_diameter falls back to inner_wall_line_width ->
    // line_width -> 0.4; effective empty-config fallback is 0.4.
    ("nozzle_diameter", Float(0.4)),
    ("layer_height", Float(0.2)),
];

/// Transcribed from `arachne-perimeters/src/lib.rs::arachne_params_from_config`
/// and `run_perimeters`. Fallbacks routed through `ArachneParams::default()`
/// (mm domain) are stated here in the manifest's domain — e.g. `min_bead_width`
/// is read via `units_to_mm(v).unwrap_or(defaults.min_bead_width = 0.4mm)`, so
/// its `unit = "units"` manifest default must be `mm_to_units(0.4) = 4000`.
const ARACHNE_FALLBACKS: &[(&str, CodeFallback)] = &[
    ("layer_height", Float(0.2)),
    ("nozzle_diameter", Float(0.4)),
    // percent key resolved via get_abs_value against nozzle_diameter; code
    // fallback defaults.min_feature_size = 0.1mm = 25% of the 0.4mm nozzle.
    ("min_feature_size", Str("25%")),
    ("min_bead_width", Float(4000.0)), // units; defaults.min_bead_width 0.4mm
    ("wall_transition_filter_deviation", Float(1000.0)), // units; defaults.transition_filter_dist 0.1mm
    // percent; code fallback defaults.wall_transition_length = 0.4mm = 100% of nozzle.
    ("wall_transition_length", Str("100%")),
    // degrees in the manifest; the read maps .to_radians() and its fallback is
    // defaults.wall_transition_angle = 10deg-as-radians.
    ("wall_transition_angle", Float(10.0)),
    ("wall_distribution_count", Int(1)), // defaults.distribution_count
    ("min_length_factor", Float(0.5)),
    ("initial_layer_min_bead_width", Float(3400.0)), // units; defaults 0.34mm
    ("outer_wall_offset", Float(0.0)),               // units; defaults 0.0
    ("min_central_distance", Float(0.0)),            // units; defaults 0.0
    ("visvalingam_area_threshold", Float(100.0)),    // units; defaults 0.01mm
    ("min_width", Float(4000.0)),                    // units; defaults 0.4mm
    // 0 is the "auto: 2 * wall_count" sentinel; an absent key takes the same
    // branch, so 0 IS the code fallback.
    ("max_bead_count", Int(0)),
    ("wall_count", Int(3)),
    ("wall_direction", Str("counter_clockwise")),
    // D-160 Bug A: bead_width_x, read in plain mm; fallback
    // defaults.optimal_width = 0.4mm.
    ("inner_wall_line_width", Float(0.4)),
    ("detect_thin_wall", Bool(false)), // defaults.print_thin_walls
    // D-160 Bug A: bead_width_0, plain mm; fallback
    // defaults.preferred_bead_width_outer = 0.4mm.
    ("outer_wall_line_width", Float(0.4)),
    ("precise_outer_wall", Bool(false)),
    ("wall_sequence", Str("InnerOuter")),
    ("seam_candidate_angle_threshold_deg", Float(30.0)),
    ("detect_overhang_wall", Bool(true)),
    ("overhang_reverse", Bool(false)),
    ("overhang_reverse_internal_only", Bool(false)),
    ("overhang_reverse_threshold", Str("0.0")), // float_or_percent; unwrap_or(0.0)
    ("extra_perimeters_on_overhangs", Unread),  // declared; arachne never reads it
    // float_or_percent; code fallback is 0.0 = "filter disabled"
    // (get_abs_value(..).unwrap_or(0.0)). Upstream's default is 300%, but the
    // manifest states the CODE fallback because manifest defaults are never
    // injected; this row was a "300%" lie until the exhaustive guard landed.
    ("min_width_top_surface", Str("0.0")),
    ("alternate_extra_wall", Bool(false)),
    ("bridge_flow", Float(1.0)),
    ("thick_bridges", Bool(false)),
    ("spiral_vase", Bool(false)),
    ("sparse_infill_density", Float(20.0)),
    ("only_one_wall_top", Bool(false)),
    ("only_one_wall_first_layer", Bool(false)),
    // mm; squared at the read site; fallback defaults.smallest_line_segment_squared
    // = 0.0025 mm^2 = (0.05mm)^2. Was a 0.5 (10x) lie until this guard landed.
    ("wall_maximum_resolution", Float(0.05)),
    // mm; squared at the read site; fallback defaults.allowed_error_distance_squared
    // = 0.000025 mm^2 = (0.005mm)^2. Was a 0.025 (5x) lie until this guard landed.
    ("wall_maximum_deviation", Float(0.005)),
];

fn schema_table(manifest: &str) -> toml::value::Table {
    let parsed: toml::Value = toml::from_str(manifest).expect("manifest parses as TOML");
    parsed["config"]["schema"]
        .as_table()
        .expect("[config.schema] is a table")
        .clone()
}

fn assert_exhaustive_reconcile(module: &str, manifest: &str, table: &[(&str, CodeFallback)]) {
    let schema = schema_table(manifest);

    // Set equality, both directions: no manifest key may be unclassified, and
    // no table row may outlive its manifest key.
    let manifest_keys: std::collections::BTreeSet<&str> =
        schema.keys().map(String::as_str).collect();
    let table_keys: std::collections::BTreeSet<&str> = table.iter().map(|(k, _)| *k).collect();
    let unclassified: Vec<&&str> = manifest_keys.difference(&table_keys).collect();
    assert!(
        unclassified.is_empty(),
        "{module}: manifest keys with no code-fallback classification (add them \
         to the table with their read-site fallback, or `Unread` if the module \
         never reads them): {unclassified:?}"
    );
    let stale: Vec<&&str> = table_keys.difference(&manifest_keys).collect();
    assert!(
        stale.is_empty(),
        "{module}: table rows whose manifest key no longer exists: {stale:?}"
    );
    assert_eq!(
        table.len(),
        table_keys.len(),
        "{module}: duplicate table rows"
    );

    // Value equality per key.
    for (key, expected) in table {
        let default = &schema[*key]["default"];
        match expected {
            Float(v) => {
                let got = default
                    .as_float()
                    .or_else(|| default.as_integer().map(|i| i as f64))
                    .unwrap_or_else(|| panic!("{module}: `{key}` default is not numeric"));
                assert!(
                    (got - v).abs() < 1e-9,
                    "{module}: `{key}` manifest default {got} != code fallback {v}"
                );
            }
            Int(v) => assert_eq!(
                default.as_integer(),
                Some(*v),
                "{module}: `{key}` manifest default != code fallback {v}"
            ),
            Bool(v) => assert_eq!(
                default.as_bool(),
                Some(*v),
                "{module}: `{key}` manifest default != code fallback {v}"
            ),
            Str(v) => assert_eq!(
                default.as_str(),
                Some(*v),
                "{module}: `{key}` manifest default != code fallback {v:?}"
            ),
            Unread => {} // presence already proven by set-equality above
        }
    }
}

#[test]
fn classic_manifest_defaults_are_the_code_fallbacks() {
    assert_exhaustive_reconcile("classic-perimeters", CLASSIC_MANIFEST, CLASSIC_FALLBACKS);
}

#[test]
fn arachne_manifest_defaults_are_the_code_fallbacks() {
    assert_exhaustive_reconcile("arachne-perimeters", ARACHNE_MANIFEST, ARACHNE_FALLBACKS);
}
