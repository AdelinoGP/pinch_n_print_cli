//! Ticket 128 / 131 — a JSON integer in the `--config` sidecar must reach a
//! module exactly like the equivalent JSON float.
//!
//! `serde_json` turns `90` into `ConfigValue::Int` and `90.0` into
//! `ConfigValue::Float` (`json_to_config_value`), and modules read through the
//! RAW source map — `bind_module_config_view` builds their view with
//! `ConfigView::from_declared(source, ...)`, not from `ResolvedConfig`. So the
//! JSON spelling of a number is visible all the way to the module's read
//! accessor, and `Int` used to fall into `get_abs_value` / `get_float`'s
//! `_ => None` arm: the module silently kept its fallback.
//!
//! Measured before the fix, `pnp_cli slice` on `resources/regression_wedge.stl`
//! with every core-module dir, `gcode_filament_length_mm`:
//!
//!   {"sparse_infill_density": 90}    -> 10927.99609375   (inert: the 20% default)
//!   {"sparse_infill_density": 90.0}  -> 33215.7109375
//!   {"sparse_infill_density": "90%"} -> 33215.7109375
//!
//! This test pins the chain at the seam where it breaks — the real call site,
//! sidecar JSON through to the accessor a module calls.
//!
//! Authoritative pipe command:
//!   `cargo test -p slicer-runtime --test integration -- sidecar_integer`

#![allow(missing_docs)]

use slicer_ir::ConfigView;
use slicer_scheduler::execution_plan::parse_cli_config_source;

/// Build the view a module declaring `sparse_infill_density` would receive,
/// the same way `bind_module_config_view` does.
fn module_view(json: &str) -> ConfigView {
    let source = parse_cli_config_source(json).expect("sidecar JSON must parse");
    ConfigView::from_declared(&source, ["sparse_infill_density"])
}

/// `resolve_percent_float`'s read: `get_abs_value(key, 100.0) / 100.0`, which
/// is what `rectilinear-infill` and `gyroid-infill` call for this key.
fn density_fraction(json: &str) -> Option<f64> {
    module_view(json)
        .get_abs_value("sparse_infill_density", 100.0)
        .map(|v| v / 100.0)
}

#[test]
fn sidecar_integer_reaches_the_module_like_the_float_spelling() {
    let by_int = density_fraction(r#"{"sparse_infill_density": 90}"#);
    let by_float = density_fraction(r#"{"sparse_infill_density": 90.0}"#);
    let by_percent = density_fraction(r#"{"sparse_infill_density": "90%"}"#);

    assert_eq!(
        by_float,
        Some(0.9),
        "the float spelling is the reference and must resolve to 0.9"
    );
    assert_eq!(
        by_percent,
        Some(0.9),
        "the percent-string spelling must resolve to 0.9"
    );
    assert_eq!(
        by_int, by_float,
        "a JSON integer must reach the module as the same value as the equivalent \
         JSON float. `None` here is the ticket-128 sidecar defect: the module falls \
         back and the user's config silently does nothing."
    );
}

#[test]
fn distinct_sidecar_integers_produce_distinct_module_values() {
    // The user-visible symptom was that these were indistinguishable: three
    // different profiles sliced to a byte-identical filament length.
    let five = density_fraction(r#"{"sparse_infill_density": 5}"#);
    let twenty_five = density_fraction(r#"{"sparse_infill_density": 25}"#);
    let ninety = density_fraction(r#"{"sparse_infill_density": 90}"#);

    assert_eq!(five, Some(0.05));
    assert_eq!(twenty_five, Some(0.25));
    assert_eq!(ninety, Some(0.9));
    assert!(
        five != twenty_five && twenty_five != ninety,
        "distinct integer densities must produce distinct module values; equal \
         values mean every integer profile collapses onto the module fallback"
    );
}

#[test]
fn sidecar_integer_reaches_a_plain_float_read_too() {
    // `get_float` had the same `_ => None` gap, so this is not specific to
    // percent keys: any float-typed key spelled without a decimal point was
    // invisible to the module.
    let source = parse_cli_config_source(r#"{"line_width": 1}"#).expect("parse");
    let view = ConfigView::from_declared(&source, ["line_width"]);
    assert_eq!(
        view.get_float("line_width"),
        Some(1.0),
        "a JSON integer must satisfy a float read; `None` means the module keeps \
         its fallback and the user's value is silently dropped"
    );
}

#[test]
fn integer_reads_still_see_integers() {
    // Widening the float accessors must not disturb `get_int`.
    let source = parse_cli_config_source(r#"{"top_shell_layers": 3}"#).expect("parse");
    let view = ConfigView::from_declared(&source, ["top_shell_layers"]);
    assert_eq!(view.get_int("top_shell_layers"), Some(3));
}
