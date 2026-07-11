#![allow(missing_docs)]

// ── AC-N1 (packet 150): live manifest validation of percent/float_or_percent
// config-schema defaults ─────────────────────────────────────────────────

/// Builds a minimal valid module manifest whose `[config.schema.<key>]`
/// entry is `type = "percent"` with the given raw TOML `default` literal
/// (caller supplies quoting, e.g. `"\"25%\""` or `"\"abc%\""`).
fn percent_type_manifest_toml(id: &str, key: &str, default_literal: &str) -> String {
    format!(
        r#"
[module]
id = "{id}"
version = "1.2.0"
display-name = "Fixture Module"
description = "fixture manifest"
author = "community"
license = "MIT"
homepage = "https://example.invalid/{id}"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = ["SliceIR.regions.infill_areas", "RegionMapIR"]
writes = ["InfillIR.regions.sparse_infill"]

[claims]
holds = ["infill-generator"]
requires = ["region-map"]

[compatibility]
incompatible-with = ["com.community.lines-*"]
requires = ["com.community.support-prep"]
min-host-version = "0.5.0"
min-ir-schema = "1.2.0"
max-ir-schema = "2.0.0"

[config.schema.{key}]
type = "percent"
default = {default_literal}

[config.overridable-per-region]
keys = ["density"]

[config.overridable-per-layer]
keys = ["density"]

[hints]
layer-parallel-safe = true
"#
    )
}

fn write_percent_fixture(dir: &std::path::Path, stem: &str, manifest: &str) -> std::path::PathBuf {
    let manifest_path = dir.join(format!("{stem}.toml"));
    std::fs::write(&manifest_path, manifest).expect("write manifest fixture");
    std::fs::write(manifest_path.with_extension("wasm"), b"placeholder wasm")
        .expect("write wasm fixture");
    manifest_path
}

#[test]
fn config_percent_type_rejects_malformed_percent_default_naming_the_key() {
    let temp_dir = tempfile::Builder::new()
        .prefix("config-percent-type-reject-")
        .tempdir()
        .expect("create temp fixture dir");
    let manifest_path = write_percent_fixture(
        temp_dir.path(),
        "bad-percent",
        &percent_type_manifest_toml("com.community.bad-percent", "min_feature_size", "\"abc%\""),
    );

    let error = slicer_scheduler::load_module_from_paths(
        &manifest_path,
        &manifest_path.with_extension("wasm"),
    )
    .expect_err("malformed percent default must be rejected, not silently coerced to 0");

    assert!(
        error.message.contains("min_feature_size"),
        "error should name the offending key: {error:?}"
    );
    assert!(
        error.message.contains("abc%"),
        "error should surface the malformed value: {error:?}"
    );
}

#[test]
fn config_percent_type_accepts_well_formed_percent_default() {
    let temp_dir = tempfile::Builder::new()
        .prefix("config-percent-type-accept-")
        .tempdir()
        .expect("create temp fixture dir");
    let manifest_path = write_percent_fixture(
        temp_dir.path(),
        "good-percent",
        &percent_type_manifest_toml("com.community.good-percent", "min_feature_size", "\"25%\""),
    );

    let module = slicer_scheduler::load_module_from_paths(
        &manifest_path,
        &manifest_path.with_extension("wasm"),
    )
    .expect("well-formed \"25%\" percent default must load");

    let entry = module
        .config_schema()
        .entries
        .get("min_feature_size")
        .expect("min_feature_size entry present");
    assert_eq!(entry.field_type, "percent");

    // Directly exercise the parser `parse_config_field_entry` defers to,
    // proving the default actually parses into `ConfigValue::Percent(25.0)`
    // rather than merely being accepted as an opaque string.
    let parsed = slicer_scheduler::manifest::parse_percent_default(
        "min_feature_size",
        "percent",
        Some(&toml::Value::String("25%".to_string())),
        &manifest_path,
    )
    .expect("well-formed 25% default parses");
    match parsed {
        slicer_ir::ConfigValue::Percent(n) => assert_eq!(n, 25.0),
        other => panic!("expected ConfigValue::Percent(25.0), got {other:?}"),
    }
}

#[test]
fn perimeter_modules_declare_arc_tolerance() {
    let path = "../../modules/core-modules/classic-perimeters/classic-perimeters.toml";
    let abs_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let manifest_text = std::fs::read_to_string(&abs_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", abs_path.display(), e));
    let parsed: toml::Value = toml::from_str(&manifest_text)
        .unwrap_or_else(|e| panic!("toml parse error in {}: {}", abs_path.display(), e));
    let schema = &parsed["config"]["schema"]["perimeter_arc_tolerance"];
    assert_eq!(schema["type"].as_str(), Some("float"), "type in {}", path);
    assert_eq!(
        schema["default"].as_float(),
        Some(0.0125),
        "default in {}",
        path
    );
    assert_eq!(schema["min"].as_float(), Some(0.0), "min in {}", path);
    assert_eq!(schema["max"].as_float(), Some(1.0), "max in {}", path);
}
