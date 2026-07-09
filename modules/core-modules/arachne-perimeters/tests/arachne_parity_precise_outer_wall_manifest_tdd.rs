//! TDD test for arachne per-vertex parity packet 148, AC-7.
//!
//! Guards that `arachne-perimeters.toml` declares the wall-sequencing /
//! seam-parity config keys added in packet 148 Step 1:
//! `precise_outer_wall`, `wall_sequence`, and
//! `seam_candidate_angle_threshold_deg`. Parses the manifest via the `toml`
//! crate (mirroring part-cooling's `cooling_config_schema_tdd.rs`) and
//! asserts the exact type/default/min/max values the AC demands, not just
//! section presence.
//!
//! Expected initial state: GREEN immediately, since Step 1 of this packet
//! already landed all three manifest sections before this test was written.

#![allow(missing_docs)]

use toml::Value;

fn manifest() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("arachne-perimeters.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "arachne-perimeters.toml must be readable at {}: {e}",
            path.display()
        )
    });
    text.parse::<Value>()
        .expect("arachne-perimeters.toml must parse as TOML")
}

fn schema_entry<'a>(manifest: &'a Value, key: &str) -> &'a Value {
    manifest
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.get(key))
        .unwrap_or_else(|| panic!("arachne-perimeters.toml is missing [config.schema.{key}]"))
}

#[test]
fn precise_outer_wall_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "precise_outer_wall");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("bool"),
        "expected bool type for precise_outer_wall"
    );
    assert_eq!(
        entry.get("default").and_then(Value::as_bool),
        Some(false),
        "expected default false for precise_outer_wall"
    );
}

#[test]
fn wall_sequence_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "wall_sequence");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("string"),
        "expected string type for wall_sequence"
    );
    assert_eq!(
        entry.get("default").and_then(Value::as_str),
        Some("InnerOuter"),
        "expected default \"InnerOuter\" for wall_sequence"
    );
}

#[test]
fn seam_candidate_angle_threshold_deg_section_declared() {
    let m = manifest();
    let entry = schema_entry(&m, "seam_candidate_angle_threshold_deg");
    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("float"),
        "expected float type for seam_candidate_angle_threshold_deg"
    );
    let default = entry
        .get("default")
        .and_then(Value::as_float)
        .unwrap_or_else(|| {
            panic!("missing/non-float default for seam_candidate_angle_threshold_deg")
        });
    assert!(
        (default - 30.0).abs() < f64::EPSILON,
        "expected default 30.0 for seam_candidate_angle_threshold_deg, got {default}"
    );
    let min = entry
        .get("min")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float min for seam_candidate_angle_threshold_deg"));
    assert!(
        (min - 0.0).abs() < f64::EPSILON,
        "expected min 0.0 for seam_candidate_angle_threshold_deg, got {min}"
    );
    let max = entry
        .get("max")
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("missing/non-float max for seam_candidate_angle_threshold_deg"));
    assert!(
        (max - 180.0).abs() < f64::EPSILON,
        "expected max 180.0 for seam_candidate_angle_threshold_deg, got {max}"
    );
}
