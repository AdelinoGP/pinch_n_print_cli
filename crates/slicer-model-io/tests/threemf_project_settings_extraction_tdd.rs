//! TDD suite for the generic 3MF `project_settings.config` extractor
//! (packet-XXX: generalise config ingestion).
//!
//! Replaces the prior narrow `read_3mf_filament_colours` (filament colours
//! only) with `read_3mf_project_settings`, which returns every key in the
//! JSON sidecar with authored strings preserved. Verifies:
//! - Array-valued keys (`filament_colour`, `chamber_temperature`) come
//!   through as `ConfigValue::List` with string elements still authored strings.
//! - String-valued numeric keys (`"4"`, `"25"`) remain exact `String` values.
//! - String-valued boolean keys (`"0"`, `"1"`, `"true"`, `"false"`) remain
//!   exact `String` values rather than becoming `Bool`.
//! - Truly string keys stay as `ConfigValue::String`.
//! - The `cube_4color.3mf` fixture has the documented `filament_colour`
//!   palette, the `thumbnails` size/format list, and the
//!   `extruder_colour` array.

use std::io::Cursor;
use zip::write::SimpleFileOptions;

// Helper: write a 3MF zip with a `Metadata/project_settings.config` entry
// to a temp file on disk; returns the temp file (caller can `.path()` it).
// Used to exercise the path-based `read_3mf_project_settings` API end-to-end.
fn write_zip_to_temp(project_settings_json: &str) -> tempfile::NamedTempFile {
    let tmp = tempfile::Builder::new()
        .suffix(".3mf")
        .tempfile()
        .expect("tempfile create failed");
    let buf = Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(buf);
    let opts = SimpleFileOptions::default();
    writer
        .start_file("Metadata/project_settings.config", opts)
        .unwrap();
    writer.write_all(project_settings_json.as_bytes()).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    use std::io::Write as _;
    let mut file_ref = std::fs::File::create(tmp.path()).expect("file create failed");
    file_ref.write_all(&bytes).expect("file write failed");
    drop(file_ref);
    tmp
}

#[test]
fn extracts_filament_colour_as_list_of_strings() {
    // The cube_4color-style fixture: `filament_colour` is a JSON array of
    // hex strings; the generic extractor must surface it as
    // `ConfigValue::List` of `ConfigValue::String`s.
    let json = r##"{
        "filament_colour": ["#FF9B00", "#02BF06", "#1800F2", "#EC0006"],
        "default_filament_colour": ["", "", "", ""],
        "filament_colour_type": ["1", "1", "1", "1"],
        "string_array": ["0", "1", "50%"]
    }"##;
    let tmp = write_zip_to_temp(json);
    let parsed = slicer_model_io::read_3mf_project_settings(tmp.path())
        .expect("project_settings.config present");

    match parsed.get("filament_colour") {
        Some(slicer_ir::ConfigValue::List(items)) => {
            let colours: Vec<String> = items
                .iter()
                .map(|v| match v {
                    slicer_ir::ConfigValue::String(s) => s.clone(),
                    other => panic!("expected String, got {other:?}"),
                })
                .collect();
            assert_eq!(colours, vec!["#FF9B00", "#02BF06", "#1800F2", "#EC0006"]);
        }
        other => panic!("expected List, got {other:?}"),
    }

    assert_eq!(
        parsed.get("string_array"),
        Some(&slicer_ir::ConfigValue::List(vec![
            slicer_ir::ConfigValue::String("0".to_owned()),
            slicer_ir::ConfigValue::String("1".to_owned()),
            slicer_ir::ConfigValue::String("50%".to_owned()),
        ])),
        "string-authored array elements must remain strings"
    );
}

#[test]
fn extract_extruder_colour_independently() {
    // `extruder_colour` must come through as a `ConfigValue::List` of its
    // own values — not synthesised from `filament_colour`. The 4-color
    // fixture authors them with different lengths (filament has 4,
    // extruder has 1) to verify the independent path.
    let json = r##"{
        "filament_colour": ["#FF9B00", "#02BF06", "#1800F2", "#EC0006"],
        "extruder_colour": ["#018001"]
    }"##;
    let tmp = write_zip_to_temp(json);
    let parsed = slicer_model_io::read_3mf_project_settings(tmp.path())
        .expect("project_settings.config present");

    let fc = parsed.get("filament_colour");
    let ec = parsed.get("extruder_colour");
    assert!(fc.is_some(), "filament_colour missing");
    assert!(ec.is_some(), "extruder_colour missing");
    match (fc, ec) {
        (
            Some(slicer_ir::ConfigValue::List(fc_items)),
            Some(slicer_ir::ConfigValue::List(ec_items)),
        ) => {
            assert_eq!(fc_items.len(), 4);
            assert_eq!(ec_items.len(), 1);
        }
        _ => panic!("expected both to be List"),
    }
}

#[test]
fn extracts_thumbnails_key_as_string() {
    // `thumbnails` is a scalar `WxH/EXT` spec; must come through as
    // `ConfigValue::String`. Historically this key only reached
    // `config_source` via `--config` JSON, not via the 3MF.
    let json = r##"{
        "thumbnails": "160x120/PNG",
        "thumbnails_format": "PNG"
    }"##;
    let tmp = write_zip_to_temp(json);
    let parsed = slicer_model_io::read_3mf_project_settings(tmp.path())
        .expect("project_settings.config present");

    match parsed.get("thumbnails") {
        Some(slicer_ir::ConfigValue::String(s)) => {
            assert_eq!(s, "160x120/PNG");
        }
        other => panic!("expected String, got {other:?}"),
    }
    match parsed.get("thumbnails_format") {
        Some(slicer_ir::ConfigValue::String(s)) => {
            assert_eq!(s, "PNG");
        }
        other => panic!("expected String, got {other:?}"),
    }
}

#[test]
fn preserves_string_authored_numbers_verbatim() {
    // OrcaSlicer stores these values in `project_settings.config` as strings.
    // The syntax-only adapter must preserve the exact spelling for registry
    // ingestion rather than guessing `Int` from numeric-looking text.
    let json = r##"{
        "wall_loops": "4",
        "layer_height": "0.5",
        "support_threshold_overlap": "50%",
        "bridge_line_width": "100%",
        "support_style": "normal(auto)",
        "bottom_shell_layers": "4",
        "bridge_speed": "25",
        "support_threshold_angle": "40"
    }"##;
    let tmp = write_zip_to_temp(json);
    let parsed = slicer_model_io::read_3mf_project_settings(tmp.path())
        .expect("project_settings.config present");

    assert_eq!(
        parsed.get("wall_loops"),
        Some(&slicer_ir::ConfigValue::String("4".to_owned()))
    );
    assert_eq!(
        parsed.get("layer_height"),
        Some(&slicer_ir::ConfigValue::String("0.5".to_owned()))
    );
    assert_eq!(
        parsed.get("support_threshold_overlap"),
        Some(&slicer_ir::ConfigValue::String("50%".to_owned()))
    );
    assert_eq!(
        parsed.get("bridge_line_width"),
        Some(&slicer_ir::ConfigValue::String("100%".to_owned()))
    );
    assert_eq!(
        parsed.get("support_style"),
        Some(&slicer_ir::ConfigValue::String("normal(auto)".to_owned()))
    );
    assert_eq!(
        parsed.get("bottom_shell_layers"),
        Some(&slicer_ir::ConfigValue::String("4".to_owned()))
    );
    assert_eq!(
        parsed.get("bridge_speed"),
        Some(&slicer_ir::ConfigValue::String("25".to_owned()))
    );
    assert_eq!(
        parsed.get("support_threshold_angle"),
        Some(&slicer_ir::ConfigValue::String("40".to_owned()))
    );
}

#[test]
fn preserves_string_authored_booleans_verbatim() {
    // OrcaSlicer uses `"0"` and `"1"` (sometimes `"true"`/`"false"`) for
    // boolean-valued keys. The syntax-only adapter must not decide that
    // declared type; in particular, these values must not become `Bool`.
    let json = r##"{
        "enable_support": "0",
        "enable_arc_fitting": "1",
        "enable_prime_tower": "false",
        "enable_overhang_bridge_fan_boost": "true"
    }"##;
    let tmp = write_zip_to_temp(json);
    let parsed = slicer_model_io::read_3mf_project_settings(tmp.path())
        .expect("project_settings.config present");

    assert_eq!(
        parsed.get("enable_support"),
        Some(&slicer_ir::ConfigValue::String("0".to_owned()))
    );
    assert_eq!(
        parsed.get("enable_arc_fitting"),
        Some(&slicer_ir::ConfigValue::String("1".to_owned()))
    );
    assert_eq!(
        parsed.get("enable_prime_tower"),
        Some(&slicer_ir::ConfigValue::String("false".to_owned()))
    );
    assert_eq!(
        parsed.get("enable_overhang_bridge_fan_boost"),
        Some(&slicer_ir::ConfigValue::String("true".to_owned()))
    );
}

#[test]
fn keeps_unparseable_strings_as_string() {
    // Free-form strings (e.g. enums, mixed-content values) remain exact
    // `ConfigValue::String` values at the loader boundary.
    let json = r##"{
        "support_type": "normal(auto)",
        "seam_position": "rear",
        "sparse_infill_pattern": "gyroid"
    }"##;
    let tmp = write_zip_to_temp(json);
    let parsed = slicer_model_io::read_3mf_project_settings(tmp.path())
        .expect("project_settings.config present");

    assert_eq!(
        parsed.get("support_type"),
        Some(&slicer_ir::ConfigValue::String("normal(auto)".to_string()))
    );
    assert_eq!(
        parsed.get("seam_position"),
        Some(&slicer_ir::ConfigValue::String("rear".to_string()))
    );
    assert_eq!(
        parsed.get("sparse_infill_pattern"),
        Some(&slicer_ir::ConfigValue::String("gyroid".to_string()))
    );
}

#[test]
fn cube_4color_fixture_extracts_full_palette_thumbnails_and_extruder_colour() {
    // Integration test against the real cube_4color.3mf fixture. Must
    // extract the documented palette, the `thumbnails` spec, and the
    // `extruder_colour` (which the old extractor never saw).
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_4color.3mf"
    ));
    assert!(
        path.exists(),
        "cube_4color.3mf fixture missing: {}",
        path.display()
    );
    let sidecar = slicer_model_io::read_3mf_project_settings(path)
        .expect("project_settings.config present in cube_4color.3mf");

    // filament_colour: List of 4 strings.
    match sidecar.get("filament_colour") {
        Some(slicer_ir::ConfigValue::List(items)) => {
            let values: Vec<String> = items
                .iter()
                .map(|v| match v {
                    slicer_ir::ConfigValue::String(s) => s.clone(),
                    other => panic!("expected String, got {other:?}"),
                })
                .collect();
            assert_eq!(values, vec!["#FF9B00", "#02BF06", "#1800F2", "#EC0006"]);
        }
        other => panic!("filament_colour: expected List, got {other:?}"),
    }

    // extruder_colour: independently present, List of 1 string.
    match sidecar.get("extruder_colour") {
        Some(slicer_ir::ConfigValue::List(items)) => {
            assert_eq!(items.len(), 1, "extruder_colour should be 1 entry");
        }
        other => panic!("extruder_colour: expected List, got {other:?}"),
    }

    // thumbnails: scalar string with WxH/EXT spec.
    match sidecar.get("thumbnails") {
        Some(slicer_ir::ConfigValue::String(s)) => {
            assert!(s.contains("x"), "thumbnails should contain a WxH spec");
        }
        other => panic!("thumbnails: expected String, got {other:?}"),
    }
}

#[test]
fn missing_sidecar_returns_none() {
    // `read_3mf_project_settings` is the path-based API; for a 3MF zip
    // without `Metadata/project_settings.config`, it must return `None`
    // (the contract callers like `main.rs` rely on to detect "not a 3MF
    // or 3MF without project config").
    let tmp = tempfile::Builder::new()
        .suffix(".3mf")
        .tempfile()
        .expect("tempfile create failed");
    let result = slicer_model_io::read_3mf_project_settings(tmp.path());
    assert!(result.is_none(), "no project_settings.config → None");
}
